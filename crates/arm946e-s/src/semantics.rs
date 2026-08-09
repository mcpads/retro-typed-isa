//! Complete static meaning for ARMv5TE ARM and Thumb execution states.

use arm7tdmi::{
    AddressOffset, AddressingMode2, AddressingMode3, AnalysisPoint, Condition, ControlBit,
    Coprocessor, CoprocessorOpcode, CoprocessorRegister, ExecutionState, Flag, HalfwordOffset,
    HalfwordOffsetLocation, IndexMode, IndirectTarget, Location, MemoryAddress, MemoryLocation,
    MemoryWidth, Reference, Register,
};
use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlBoundary, ControlTarget, LocationAccess,
    ReferenceKind, StaticReference,
};

use crate::{ArmInstruction, EncodeError, ThumbInstruction};

/// Failure to form sound ARM946E-S static facts.
#[derive(Debug, thiserror::Error)]
pub enum SemanticError {
    /// Analysis-point state does not match the selected exact profile.
    #[error("analysis point is in {actual:?}, but {expected:?} is required")]
    ProfileStateMismatch {
        /// Required execution state.
        expected: ExecutionState,
        /// Supplied execution state.
        actual: ExecutionState,
    },
    /// The typed instruction is not encodable in ARMv5TE.
    #[error(transparent)]
    Encode(#[from] EncodeError),
    /// An inherited ARMv4T instruction failed semantic validation.
    #[error(transparent)]
    Armv4T(#[from] arm7tdmi::SemanticError),
}

/// Shared control-flow result specialized with the ARM-family vocabulary.
pub type ControlFlow = arm7tdmi::ControlFlow;

/// Shared instruction-semantics result specialized with the ARM-family vocabulary.
pub type InstructionSemantics = arm7tdmi::InstructionSemantics;

pub(crate) fn arm_semantics(
    instruction: &ArmInstruction,
    at: AnalysisPoint,
) -> Result<InstructionSemantics, SemanticError> {
    require_state(at, ExecutionState::Arm)?;
    crate::encode_arm_bytes(instruction)?;
    if at.location & 3 != 0 {
        return Err(SemanticError::Encode(EncodeError::UnalignedAddress {
            location: at.location,
            alignment: 4,
        }));
    }
    if let ArmInstruction::Armv4T(inherited) = instruction {
        let mut semantics =
            <arm7tdmi::Arm7TdmiArm as typed_isa_core::StaticSemantics>::semantics(inherited, &at)?;
        adapt_armv5t_loaded_pc(inherited, &mut semantics);
        return Ok(semantics);
    }

    let next = AnalysisPoint::arm(at.location.wrapping_add(4), at.mode);
    let mut accesses = Vec::new();
    let mut references = Vec::new();
    arm_extension_accesses(instruction, at, &mut accesses, &mut references);
    Ok(InstructionSemantics {
        control_flow: arm_extension_control_flow(instruction, at, next),
        location_accesses: accesses,
        static_references: references,
    })
}

pub(crate) fn thumb_semantics(
    instruction: &ThumbInstruction,
    at: AnalysisPoint,
) -> Result<InstructionSemantics, SemanticError> {
    require_state(at, ExecutionState::Thumb)?;
    crate::encode_thumb_bytes(instruction)?;
    if at.location & 1 != 0 {
        return Err(SemanticError::Encode(EncodeError::UnalignedAddress {
            location: at.location,
            alignment: 2,
        }));
    }
    if let ThumbInstruction::Armv4T(inherited) = instruction {
        let mut semantics =
            <arm7tdmi::Arm7TdmiThumb as typed_isa_core::StaticSemantics>::semantics(
                inherited, &at,
            )?;
        if matches!(
            inherited,
            arm7tdmi::ThumbInstruction::Pop {
                program_counter: true,
                ..
            }
        ) {
            semantics.control_flow.action = ControlAction::Transfer {
                target: ControlTarget::Indirect(IndirectTarget::InterworkingLoadedProgramCounter),
            };
            write(
                &mut semantics.location_accesses,
                Location::ControlBit(ControlBit::ThumbState),
                AccessOccurrence::Always,
            );
        }
        return Ok(semantics);
    }

    let next = AnalysisPoint::thumb(
        at.location
            .wrapping_add(u32::try_from(instruction.encoded_len()).unwrap()),
        at.mode,
    );
    let mut accesses = Vec::new();
    thumb_extension_accesses(instruction, at, &mut accesses);
    Ok(InstructionSemantics {
        control_flow: thumb_extension_control_flow(instruction, at, next),
        location_accesses: accesses,
        static_references: Vec::new(),
    })
}

fn require_state(at: AnalysisPoint, expected: ExecutionState) -> Result<(), SemanticError> {
    if at.state == expected {
        Ok(())
    } else {
        Err(SemanticError::ProfileStateMismatch {
            expected,
            actual: at.state,
        })
    }
}

fn adapt_armv5t_loaded_pc(
    instruction: &arm7tdmi::ArmInstruction,
    semantics: &mut InstructionSemantics,
) {
    let exchanges = match instruction {
        arm7tdmi::ArmInstruction::SingleTransfer {
            load: true,
            register: Register::PC,
            ..
        } => true,
        arm7tdmi::ArmInstruction::BlockTransfer {
            load: true,
            user_registers: false,
            registers,
            ..
        } => registers.bits() & (1 << Register::PC.index()) != 0,
        _ => false,
    };
    if exchanges {
        semantics.control_flow.action = ControlAction::Transfer {
            target: ControlTarget::Indirect(IndirectTarget::InterworkingLoadedProgramCounter),
        };
        let occurrence = arm_occurrence(arm7_condition(instruction));
        write(
            &mut semantics.location_accesses,
            Location::ControlBit(ControlBit::ThumbState),
            occurrence,
        );
    }
}

fn arm_extension_control_flow(
    instruction: &ArmInstruction,
    at: AnalysisPoint,
    next: AnalysisPoint,
) -> ControlFlow {
    match *instruction {
        ArmInstruction::Armv4T(_) => unreachable!(),
        ArmInstruction::Breakpoint { comment } => ControlFlow {
            action: ControlAction::Boundary(ControlBoundary::Trap {
                kind: arm7tdmi::Boundary::Breakpoint(comment),
                target: None,
                resume: None,
            }),
            fallthrough: None,
            delay_slot: None,
        },
        ArmInstruction::BranchLinkExchangeImmediate { displacement } => ControlFlow {
            action: ControlAction::LinkedTransfer {
                target: ControlTarget::Direct(AnalysisPoint::thumb(
                    at.location
                        .wrapping_add(8)
                        .wrapping_add_signed(displacement),
                    at.mode,
                )),
                return_site: next,
            },
            fallthrough: None,
            delay_slot: None,
        },
        ArmInstruction::BranchLinkExchangeRegister {
            condition, target, ..
        } => ControlFlow {
            action: ControlAction::LinkedTransfer {
                target: ControlTarget::Indirect(IndirectTarget::ExchangeRegister(
                    at.register_location(target),
                )),
                return_site: next,
            },
            fallthrough: (condition != Condition::Always).then_some(next),
            delay_slot: None,
        },
        _ => ControlFlow {
            action: ControlAction::Continue,
            fallthrough: Some(next),
            delay_slot: None,
        },
    }
}

fn thumb_extension_control_flow(
    instruction: &ThumbInstruction,
    at: AnalysisPoint,
    next: AnalysisPoint,
) -> ControlFlow {
    match *instruction {
        ThumbInstruction::Armv4T(_) => unreachable!(),
        ThumbInstruction::Breakpoint { comment } => ControlFlow {
            action: ControlAction::Boundary(ControlBoundary::Trap {
                kind: arm7tdmi::Boundary::Breakpoint(u16::from(comment)),
                target: None,
                resume: None,
            }),
            fallthrough: None,
            delay_slot: None,
        },
        ThumbInstruction::BranchLinkExchangeRegister { target } => ControlFlow {
            action: ControlAction::LinkedTransfer {
                target: ControlTarget::Indirect(IndirectTarget::ExchangeRegister(
                    at.register_location(target),
                )),
                return_site: next,
            },
            fallthrough: None,
            delay_slot: None,
        },
        ThumbInstruction::BranchLinkExchangeImmediate { displacement } => {
            let base = at.location.wrapping_add(4) & !3_u32;
            ControlFlow {
                action: ControlAction::LinkedTransfer {
                    target: ControlTarget::Direct(AnalysisPoint::arm(
                        base.wrapping_add_signed(displacement),
                        at.mode,
                    )),
                    return_site: next,
                },
                fallthrough: None,
                delay_slot: None,
            }
        }
    }
}

fn arm_extension_accesses(
    instruction: &ArmInstruction,
    at: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    use ArmInstruction::*;
    let condition = extension_condition(instruction);
    let occurrence = condition.map_or(AccessOccurrence::Always, arm_occurrence);
    if let Some(condition) = condition {
        read_condition(accesses, condition);
    }
    match *instruction {
        Armv4T(_) | Breakpoint { .. } => {}
        BranchLinkExchangeImmediate { .. } => {
            write_register(accesses, at, Register::LINK, occurrence);
            write(
                accesses,
                Location::ControlBit(ControlBit::ThumbState),
                occurrence,
            );
        }
        BranchLinkExchangeRegister { target, .. } => {
            read_register(accesses, at, target, occurrence);
            write_register(accesses, at, Register::LINK, occurrence);
            write(
                accesses,
                Location::ControlBit(ControlBit::ThumbState),
                occurrence,
            );
        }
        CountLeadingZeros {
            destination,
            source,
            ..
        } => {
            read_register(accesses, at, source, occurrence);
            write_register(accesses, at, destination, occurrence);
        }
        DoublewordTransfer {
            load,
            first_register,
            address,
            ..
        } => doubleword_transfer(
            load,
            first_register,
            address,
            at,
            occurrence,
            accesses,
            references,
        ),
        CoprocessorDataOperation2 {
            coprocessor,
            opcode,
            info,
            destination,
            first,
            second,
        } => coprocessor_data_operation(
            coprocessor,
            opcode,
            info,
            destination,
            first,
            second,
            occurrence,
            accesses,
        ),
        CoprocessorRegisterTransfer2 {
            load,
            coprocessor,
            opcode,
            info,
            arm_register,
            first,
            second,
        } => {
            let opcode = CoprocessorOpcode::new(opcode).unwrap();
            coprocessor_command_inputs(
                coprocessor,
                opcode,
                info,
                first,
                second,
                occurrence,
                accesses,
            );
            if load {
                if arm_register == Register::PC {
                    write_nzcv(accesses, occurrence);
                } else {
                    write_register(accesses, at, arm_register, occurrence);
                }
            } else {
                read_register(accesses, at, arm_register, occurrence);
            }
        }
        CoprocessorDataTransfer2 {
            load,
            long,
            coprocessor,
            register,
            base,
            offset_words,
            add,
            index,
        } => coprocessor_transfer(
            load,
            long,
            coprocessor,
            register,
            base,
            offset_words,
            add,
            index,
            at,
            occurrence,
            accesses,
        ),
        CoprocessorDoubleRegisterTransfer {
            load,
            coprocessor,
            opcode,
            first_arm_register,
            second_arm_register,
            coprocessor_register,
            ..
        } => {
            let coprocessor_location = Location::CoprocessorRegister {
                coprocessor,
                register: coprocessor_register,
            };
            write(
                accesses,
                Location::CoprocessorCommand {
                    coprocessor,
                    opcode,
                    info: 0,
                },
                occurrence,
            );
            if load {
                read(accesses, coprocessor_location, occurrence);
                for register in [first_arm_register, second_arm_register] {
                    write_register(accesses, at, register, occurrence);
                }
            } else {
                for register in [first_arm_register, second_arm_register] {
                    read_register(accesses, at, register, occurrence);
                }
                write(accesses, coprocessor_location, occurrence);
            }
        }
        PreloadData { address } => {
            address_inputs_mode2(accesses, at, address, occurrence);
            if let Some(address) = pc_relative_mode2(at, address) {
                add_reference(
                    references,
                    StaticReference {
                        target: Reference::Address(address),
                        kind: ReferenceKind::Address,
                        occurrence,
                    },
                );
            }
        }
        SaturatingArithmetic {
            destination,
            first,
            second,
            ..
        } => {
            for register in [first, second] {
                read_register(accesses, at, register, occurrence);
            }
            write_register(accesses, at, destination, occurrence);
            write(
                accesses,
                Location::Flag(Flag::Saturation),
                AccessOccurrence::Conditional,
            );
        }
        SignedHalfwordMultiplyAccumulate {
            destination,
            first,
            second,
            addend,
            ..
        }
        | SignedWordHalfwordMultiplyAccumulate {
            destination,
            word: first,
            halfword: second,
            addend,
            ..
        } => {
            for register in [first, second, addend] {
                read_register(accesses, at, register, occurrence);
            }
            write_register(accesses, at, destination, occurrence);
            write(
                accesses,
                Location::Flag(Flag::Saturation),
                AccessOccurrence::Conditional,
            );
        }
        SignedHalfwordMultiplyAccumulateLong {
            destination_low,
            destination_high,
            first,
            second,
            ..
        } => {
            for register in [destination_low, destination_high, first, second] {
                read_register(accesses, at, register, occurrence);
            }
            for register in [destination_low, destination_high] {
                write_register(accesses, at, register, occurrence);
            }
        }
        SignedHalfwordMultiply {
            destination,
            first,
            second,
            ..
        }
        | SignedWordHalfwordMultiply {
            destination,
            word: first,
            halfword: second,
            ..
        } => {
            for register in [first, second] {
                read_register(accesses, at, register, occurrence);
            }
            write_register(accesses, at, destination, occurrence);
        }
    }
}

fn thumb_extension_accesses(
    instruction: &ThumbInstruction,
    at: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    match *instruction {
        ThumbInstruction::Armv4T(_) | ThumbInstruction::Breakpoint { .. } => {}
        ThumbInstruction::BranchLinkExchangeRegister { target } => {
            read_register(accesses, at, target, AccessOccurrence::Always);
            write_register(accesses, at, Register::LINK, AccessOccurrence::Always);
            write(
                accesses,
                Location::ControlBit(ControlBit::ThumbState),
                AccessOccurrence::Always,
            );
        }
        ThumbInstruction::BranchLinkExchangeImmediate { .. } => {
            write_register(accesses, at, Register::LINK, AccessOccurrence::Always);
            write(
                accesses,
                Location::ControlBit(ControlBit::ThumbState),
                AccessOccurrence::Always,
            );
        }
    }
}

fn extension_condition(instruction: &ArmInstruction) -> Option<Condition> {
    match *instruction {
        ArmInstruction::Armv4T(_)
        | ArmInstruction::Breakpoint { .. }
        | ArmInstruction::BranchLinkExchangeImmediate { .. }
        | ArmInstruction::CoprocessorDataOperation2 { .. }
        | ArmInstruction::CoprocessorRegisterTransfer2 { .. }
        | ArmInstruction::CoprocessorDataTransfer2 { .. }
        | ArmInstruction::PreloadData { .. } => None,
        ArmInstruction::BranchLinkExchangeRegister { condition, .. }
        | ArmInstruction::CountLeadingZeros { condition, .. }
        | ArmInstruction::DoublewordTransfer { condition, .. }
        | ArmInstruction::CoprocessorDoubleRegisterTransfer { condition, .. }
        | ArmInstruction::SaturatingArithmetic { condition, .. }
        | ArmInstruction::SignedHalfwordMultiplyAccumulate { condition, .. }
        | ArmInstruction::SignedHalfwordMultiplyAccumulateLong { condition, .. }
        | ArmInstruction::SignedWordHalfwordMultiplyAccumulate { condition, .. }
        | ArmInstruction::SignedHalfwordMultiply { condition, .. }
        | ArmInstruction::SignedWordHalfwordMultiply { condition, .. } => Some(condition),
    }
}

fn arm7_condition(instruction: &arm7tdmi::ArmInstruction) -> Condition {
    use arm7tdmi::ArmInstruction::*;
    match *instruction {
        DataProcessing { condition, .. }
        | Multiply { condition, .. }
        | MultiplyLong { condition, .. }
        | BranchExchange { condition, .. }
        | Branch { condition, .. }
        | SingleTransfer { condition, .. }
        | HalfwordTransfer { condition, .. }
        | BlockTransfer { condition, .. }
        | Swap { condition, .. }
        | MoveStatusToRegister { condition, .. }
        | MoveToStatusRegister { condition, .. }
        | CoprocessorDataOperation { condition, .. }
        | CoprocessorRegisterTransfer { condition, .. }
        | CoprocessorDataTransfer { condition, .. }
        | SoftwareInterrupt { condition, .. } => condition,
    }
}

fn arm_occurrence(condition: Condition) -> AccessOccurrence {
    if condition == Condition::Always {
        AccessOccurrence::Always
    } else {
        AccessOccurrence::Conditional
    }
}

fn read_condition(accesses: &mut Vec<LocationAccess<Location>>, condition: Condition) {
    use Condition::*;
    let flags: &[Flag] = match condition {
        Equal | NotEqual => &[Flag::Zero],
        CarrySet | CarryClear => &[Flag::Carry],
        Negative | PositiveOrZero => &[Flag::Negative],
        Overflow | NoOverflow => &[Flag::Overflow],
        UnsignedHigher | UnsignedLowerOrSame => &[Flag::Carry, Flag::Zero],
        SignedGreaterOrEqual | SignedLessThan => &[Flag::Negative, Flag::Overflow],
        SignedGreaterThan | SignedLessOrEqual => &[Flag::Zero, Flag::Negative, Flag::Overflow],
        Always => &[],
    };
    for flag in flags {
        read(accesses, Location::Flag(*flag), AccessOccurrence::Always);
    }
}

fn read_register(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    register: Register,
    occurrence: AccessOccurrence,
) {
    read(
        accesses,
        Location::Register(at.register_location(register)),
        occurrence,
    );
}

fn write_register(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    register: Register,
    occurrence: AccessOccurrence,
) {
    write(
        accesses,
        Location::Register(at.register_location(register)),
        occurrence,
    );
}

fn address_inputs_mode2(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    address: AddressingMode2,
    occurrence: AccessOccurrence,
) {
    read_register(accesses, at, address.base, occurrence);
    if let AddressOffset::Register { register, shift } = address.offset {
        read_register(accesses, at, register, occurrence);
        if matches!(shift, arm7tdmi::ImmediateShift::RotateRightExtended) {
            read(accesses, Location::Flag(Flag::Carry), occurrence);
        }
    }
    if address.index != IndexMode::Offset {
        write_register(accesses, at, address.base, occurrence);
    }
}

fn address_inputs_mode3(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    address: AddressingMode3,
    occurrence: AccessOccurrence,
) {
    read_register(accesses, at, address.base, occurrence);
    if let HalfwordOffset::Register(register) = address.offset {
        read_register(accesses, at, register, occurrence);
    }
    if address.index != IndexMode::Offset {
        write_register(accesses, at, address.base, occurrence);
    }
}

fn mode3_address(at: AnalysisPoint, address: AddressingMode3) -> MemoryAddress {
    MemoryAddress::ArmMode3 {
        base: at.register_location(address.base),
        offset: match address.offset {
            HalfwordOffset::Immediate(offset) => HalfwordOffsetLocation::Immediate(offset),
            HalfwordOffset::Register(register) => {
                HalfwordOffsetLocation::Register(at.register_location(register))
            }
        },
        add: address.add,
        index: address.index,
    }
}

fn pc_relative_mode2(at: AnalysisPoint, address: AddressingMode2) -> Option<u32> {
    if address.base != Register::PC
        || matches!(
            address.index,
            IndexMode::PostIndexed | IndexMode::PostIndexedUser
        )
    {
        return None;
    }
    let AddressOffset::Immediate(offset) = address.offset else {
        return None;
    };
    let base = at.location.wrapping_add(8);
    Some(if address.add {
        base.wrapping_add(u32::from(offset))
    } else {
        base.wrapping_sub(u32::from(offset))
    })
}

fn pc_relative_mode3(at: AnalysisPoint, address: AddressingMode3) -> Option<u32> {
    if address.base != Register::PC
        || matches!(
            address.index,
            IndexMode::PostIndexed | IndexMode::PostIndexedUser
        )
    {
        return None;
    }
    let HalfwordOffset::Immediate(offset) = address.offset else {
        return None;
    };
    let base = at.location.wrapping_add(8);
    Some(if address.add {
        base.wrapping_add(u32::from(offset))
    } else {
        base.wrapping_sub(u32::from(offset))
    })
}

#[allow(clippy::too_many_arguments)]
fn doubleword_transfer(
    load: bool,
    first_register: Register,
    address: AddressingMode3,
    at: AnalysisPoint,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    address_inputs_mode3(accesses, at, address, occurrence);
    let resolved = pc_relative_mode3(at, address);
    let memory = Location::Memory(MemoryLocation {
        address: resolved.map_or_else(|| mode3_address(at, address), MemoryAddress::Literal),
        width: MemoryWidth::Known(8),
    });
    let second_register = Register::new(first_register.index() + 1).unwrap();
    if load {
        read(accesses, memory, occurrence);
        for register in [first_register, second_register] {
            write_register(accesses, at, register, occurrence);
        }
    } else {
        for register in [first_register, second_register] {
            read_register(accesses, at, register, occurrence);
        }
        write(accesses, memory, occurrence);
    }
    if let Some(address) = resolved {
        add_reference(
            references,
            StaticReference {
                target: Reference::Memory {
                    address,
                    byte_len: 8,
                },
                kind: if load {
                    ReferenceKind::Read
                } else {
                    ReferenceKind::Write
                },
                occurrence,
            },
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn coprocessor_data_operation(
    coprocessor: Coprocessor,
    opcode: CoprocessorOpcode,
    info: u8,
    destination: CoprocessorRegister,
    first: CoprocessorRegister,
    second: CoprocessorRegister,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    coprocessor_command_inputs(
        coprocessor,
        opcode,
        info,
        first,
        second,
        occurrence,
        accesses,
    );
    write(
        accesses,
        Location::CoprocessorRegister {
            coprocessor,
            register: destination,
        },
        occurrence,
    );
}

#[allow(clippy::too_many_arguments)]
fn coprocessor_command_inputs(
    coprocessor: Coprocessor,
    opcode: CoprocessorOpcode,
    info: u8,
    first: CoprocessorRegister,
    second: CoprocessorRegister,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    for register in [first, second] {
        read(
            accesses,
            Location::CoprocessorRegister {
                coprocessor,
                register,
            },
            occurrence,
        );
    }
    write(
        accesses,
        Location::CoprocessorCommand {
            coprocessor,
            opcode,
            info,
        },
        occurrence,
    );
}

#[allow(clippy::too_many_arguments)]
fn coprocessor_transfer(
    load: bool,
    long: bool,
    coprocessor: Coprocessor,
    register: CoprocessorRegister,
    base: Register,
    offset_words: u8,
    add: bool,
    index: IndexMode,
    at: AnalysisPoint,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_register(accesses, at, base, occurrence);
    if index != IndexMode::Offset {
        write_register(accesses, at, base, occurrence);
    }
    let memory = Location::Memory(MemoryLocation {
        address: MemoryAddress::Coprocessor {
            base: at.register_location(base),
            offset: u16::from(offset_words) * 4,
            add,
            index,
        },
        width: MemoryWidth::CoprocessorDefined { long },
    });
    let coprocessor_register = Location::CoprocessorRegister {
        coprocessor,
        register,
    };
    if load {
        read(accesses, memory, occurrence);
        write(accesses, coprocessor_register, occurrence);
    } else {
        read(accesses, coprocessor_register, occurrence);
        write(accesses, memory, occurrence);
    }
}

fn write_nzcv(accesses: &mut Vec<LocationAccess<Location>>, occurrence: AccessOccurrence) {
    for flag in [Flag::Negative, Flag::Zero, Flag::Carry, Flag::Overflow] {
        write(accesses, Location::Flag(flag), occurrence);
    }
}

fn read(
    accesses: &mut Vec<LocationAccess<Location>>,
    location: Location,
    occurrence: AccessOccurrence,
) {
    access(accesses, location, AccessKind::Read, occurrence);
}

fn write(
    accesses: &mut Vec<LocationAccess<Location>>,
    location: Location,
    occurrence: AccessOccurrence,
) {
    access(accesses, location, AccessKind::Write, occurrence);
}

fn access(
    accesses: &mut Vec<LocationAccess<Location>>,
    location: Location,
    kind: AccessKind,
    occurrence: AccessOccurrence,
) {
    let candidate = LocationAccess {
        location,
        kind,
        occurrence,
    };
    if !accesses.contains(&candidate) {
        accesses.push(candidate);
    }
}

fn add_reference(
    references: &mut Vec<StaticReference<Reference>>,
    candidate: StaticReference<Reference>,
) {
    if !references.contains(&candidate) {
        references.push(candidate);
    }
}
