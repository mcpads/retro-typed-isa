//! Complete static meaning for ARMv4T ARM and Thumb execution states.

use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlBoundary, ControlTarget, LocationAccess,
    ReferenceKind, StaticReference, StorageRelation,
};

use crate::{
    AddressOffset, AddressingMode2, AddressingMode3, ArmInstruction, BlockAddressing, Condition,
    Coprocessor, CoprocessorOpcode, CoprocessorRegister, DataOperation, EncodeError,
    HalfwordOffset, HalfwordTransferKind, ImmediateShift, IndexMode, LowRegister, Operand2,
    Register, RegisterList, ShiftKind, StatusMask, StatusRegister, StatusSource, ThumbAddOperand,
    ThumbAluOperation, ThumbHighOperation, ThumbImmediateOperation, ThumbInstruction,
    ThumbRegisterList, ThumbTransferOperation, ThumbTransferWidth, TransferWidth,
};

/// Instruction-set state at an ARM-family analysis point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutionState {
    /// 32-bit ARM instruction state.
    Arm,
    /// 16/32-bit Thumb instruction state.
    Thumb,
    /// A status restore made the state unavailable.
    Unknown,
}

/// ARM processor mode relevant to banked register selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcessorMode {
    /// User mode.
    User,
    /// System mode, sharing the user register bank.
    System,
    /// Fast interrupt mode.
    Fiq,
    /// Interrupt mode.
    Irq,
    /// Supervisor mode.
    Supervisor,
    /// Abort mode.
    Abort,
    /// Undefined-instruction mode.
    Undefined,
    /// CPSR mode bits are unavailable.
    Unknown,
}

/// Placed ARM-family instruction together with state selecting physical registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnalysisPoint {
    /// Architectural instruction address.
    pub location: crate::CodeLocation,
    /// Current instruction-set state.
    pub state: ExecutionState,
    /// Current processor mode.
    pub mode: ProcessorMode,
}

impl AnalysisPoint {
    /// Constructs an ARM-family static-analysis point.
    pub const fn new(
        location: crate::CodeLocation,
        state: ExecutionState,
        mode: ProcessorMode,
    ) -> Self {
        Self {
            location,
            state,
            mode,
        }
    }

    /// Constructs an ARM-state point.
    pub const fn arm(location: crate::CodeLocation, mode: ProcessorMode) -> Self {
        Self::new(location, ExecutionState::Arm, mode)
    }

    /// Constructs a Thumb-state point.
    pub const fn thumb(location: crate::CodeLocation, mode: ProcessorMode) -> Self {
        Self::new(location, ExecutionState::Thumb, mode)
    }

    /// Resolves an architectural register to the physical bank selected here.
    pub const fn register_location(self, register: Register) -> RegisterLocation {
        register_location(self.mode, register)
    }
}

/// Runtime-derived ARM-family control destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndirectTarget {
    /// Target and destination state selected from bit zero of a register.
    ExchangeRegister(RegisterLocation),
    /// Target written to the program counter without exchanging state.
    WrittenProgramCounter,
    /// Target loaded from memory with ARMv5T interworking semantics.
    InterworkingLoadedProgramCounter,
    /// Program counter and CPSR restored through an exception-return form.
    ExceptionReturn,
    /// Exception-vector base plus a fixed offset.
    ExceptionVector {
        /// Vector-table byte offset.
        offset: u8,
        /// Mode entered by the exception.
        mode: ProcessorMode,
    },
}

/// ARM-family synchronous boundary kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Boundary {
    /// Software interrupt exception.
    SoftwareInterrupt(u32),
    /// Debug breakpoint exception introduced by ARMv5.
    Breakpoint(u16),
}

/// One CPSR condition or sticky arithmetic bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Flag {
    /// Negative flag N.
    Negative,
    /// Zero flag Z.
    Zero,
    /// Carry flag C.
    Carry,
    /// Overflow flag V.
    Overflow,
    /// Saturation flag Q introduced by ARMv5TE.
    Saturation,
}

/// One individually addressable CPSR control bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlBit {
    /// T bit selecting ARM or Thumb instruction state.
    ThumbState,
}

/// Physical bank used by one banked ARM register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegisterBank {
    /// User and System shared bank.
    UserSystem,
    /// FIQ bank.
    Fiq,
    /// IRQ bank.
    Irq,
    /// Supervisor bank.
    Supervisor,
    /// Abort bank.
    Abort,
    /// Undefined-instruction bank.
    Undefined,
}

/// Physical or conservatively unresolved ARM register storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegisterLocation {
    /// Unbanked R0-R7 storage.
    Unbanked(Register),
    /// R8-R12 storage with its FIQ/non-FIQ bank made explicit.
    High {
        /// Architectural register R8-R12.
        register: Register,
        /// Selected high-register bank.
        bank: RegisterBank,
    },
    /// R13 or R14 storage in one physical mode bank.
    Banked {
        /// Architectural register R13 or R14.
        register: Register,
        /// Selected physical bank.
        bank: RegisterBank,
    },
    /// Banked register whose current physical storage is unavailable.
    CurrentUnknown(Register),
    /// Architectural program counter R15.
    ProgramCounter,
}

/// Physical current or saved program-status storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusLocation {
    /// Current program status register CPSR.
    Current,
    /// Saved program status register for a known exception mode.
    Saved(ProcessorMode),
    /// Saved program status register when the mode is unavailable.
    SavedUnknown,
}

/// One byte field selected by an MSR field mask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusField {
    /// Control byte, including mode and execution-state bits.
    Control,
    /// Extension byte.
    Extension,
    /// Status byte.
    Status,
    /// Condition-flags byte.
    Flags,
}

/// Resolved immediate or physical register address offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AddressOffsetLocation {
    /// Immediate byte offset.
    Immediate(u16),
    /// Shifted register offset.
    Register {
        /// Physical register storage.
        register: RegisterLocation,
        /// Immediate barrel-shift rule.
        shift: ImmediateShift,
    },
}

/// Resolved halfword-transfer offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HalfwordOffsetLocation {
    /// Immediate byte offset.
    Immediate(u8),
    /// Register offset.
    Register(RegisterLocation),
}

/// Symbolic ARM-family memory address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryAddress {
    /// ARM addressing mode 2.
    ArmMode2 {
        /// Base register storage.
        base: RegisterLocation,
        /// Immediate or shifted-register offset.
        offset: AddressOffsetLocation,
        /// Whether the offset is added.
        add: bool,
        /// Pre/post-index and write-back rule.
        index: IndexMode,
    },
    /// ARM addressing mode 3.
    ArmMode3 {
        /// Base register storage.
        base: RegisterLocation,
        /// Immediate or register offset.
        offset: HalfwordOffsetLocation,
        /// Whether the offset is added.
        add: bool,
        /// Pre/post-index and write-back rule.
        index: IndexMode,
    },
    /// ARM block-transfer address range.
    ArmBlock {
        /// Base register storage.
        base: RegisterLocation,
        /// Increment/decrement and before/after rule.
        addressing: BlockAddressing,
    },
    /// Thumb register-offset transfer.
    ThumbRegisterOffset {
        /// Base register storage.
        base: RegisterLocation,
        /// Offset register storage.
        offset: RegisterLocation,
    },
    /// Thumb immediate-offset transfer.
    ThumbImmediateOffset {
        /// Base register storage.
        base: RegisterLocation,
        /// Already scaled byte offset.
        offset: u16,
    },
    /// Stack range selected by SP.
    Stack,
    /// Statically resolved PC-relative literal address.
    Literal(u32),
    /// Coprocessor-defined transfer at a base plus word-scaled offset.
    Coprocessor {
        /// Base register storage.
        base: RegisterLocation,
        /// Byte offset magnitude.
        offset: u16,
        /// Whether the offset is added.
        add: bool,
        /// Pre/post-index and write-back rule.
        index: IndexMode,
    },
}

/// Width of an ARM-family memory transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryWidth {
    /// Architecturally fixed byte count.
    Known(u8),
    /// Coprocessor-defined word count with the N/long selector preserved.
    CoprocessorDefined {
        /// Whether the long-transfer selector is set.
        long: bool,
    },
}

/// Symbolic ARM-family memory storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryLocation {
    /// Symbolic address rule.
    pub address: MemoryAddress,
    /// Transfer width.
    pub width: MemoryWidth,
}

/// Architectural ARM-family storage or coprocessor state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Location {
    /// Physical or unresolved general-register storage.
    Register(RegisterLocation),
    /// Whole current or saved program status register.
    Status(StatusLocation),
    /// One byte of current or saved program status.
    StatusField {
        /// Selected status register.
        status: StatusLocation,
        /// Selected byte field.
        field: StatusField,
    },
    /// One CPSR arithmetic flag.
    Flag(Flag),
    /// One CPSR control bit.
    ControlBit(ControlBit),
    /// Symbolic memory storage.
    Memory(MemoryLocation),
    /// One coprocessor register selector.
    CoprocessorRegister {
        /// Selected coprocessor.
        coprocessor: Coprocessor,
        /// Selected register.
        register: CoprocessorRegister,
    },
    /// Opaque coprocessor data-operation command.
    CoprocessorCommand {
        /// Selected coprocessor.
        coprocessor: Coprocessor,
        /// Primary operation selector.
        opcode: CoprocessorOpcode,
        /// Secondary information selector.
        info: u8,
    },
}

/// Statically resolved non-control ARM-family reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reference {
    /// PC-relative memory range.
    Memory {
        /// First referenced byte.
        address: u32,
        /// Referenced byte count.
        byte_len: u8,
    },
    /// Address produced without claiming a memory transfer.
    Address(u32),
}

/// Failure to form sound ARMv4T static facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SemanticError {
    /// Analysis-point state does not match the selected exact profile.
    #[error("analysis point is in {actual:?}, but {expected:?} is required")]
    ProfileStateMismatch {
        /// Required execution state.
        expected: ExecutionState,
        /// Supplied execution state.
        actual: ExecutionState,
    },
    /// The typed instruction is not encodable in ARMv4T.
    #[error(transparent)]
    Encode(#[from] EncodeError),
}

/// Shared control-flow result specialized with ARM-family vocabulary.
pub type ControlFlow = typed_isa_core::ControlFlow<AnalysisPoint, IndirectTarget, Boundary>;

/// Shared instruction-semantics result specialized with ARM-family vocabulary.
pub type InstructionSemantics = typed_isa_core::InstructionSemantics<
    AnalysisPoint,
    IndirectTarget,
    Boundary,
    Location,
    Reference,
>;

pub(crate) fn arm_semantics(
    instruction: &ArmInstruction,
    at: AnalysisPoint,
) -> Result<InstructionSemantics, SemanticError> {
    if at.state != ExecutionState::Arm {
        return Err(SemanticError::ProfileStateMismatch {
            expected: ExecutionState::Arm,
            actual: at.state,
        });
    }
    crate::encode_arm_bytes(instruction)?;
    if at.location & 3 != 0 {
        return Err(SemanticError::Encode(EncodeError::UnalignedAddress {
            location: at.location,
            alignment: 4,
        }));
    }
    let next = arm_successor(instruction, at);
    let mut accesses = Vec::new();
    let mut references = Vec::new();
    arm_accesses(instruction, at, &mut accesses, &mut references);
    Ok(InstructionSemantics {
        control_flow: arm_control_flow(instruction, at, next),
        location_accesses: accesses,
        static_references: references,
    })
}

pub(crate) fn thumb_semantics(
    instruction: &ThumbInstruction,
    at: AnalysisPoint,
) -> Result<InstructionSemantics, SemanticError> {
    if at.state != ExecutionState::Thumb {
        return Err(SemanticError::ProfileStateMismatch {
            expected: ExecutionState::Thumb,
            actual: at.state,
        });
    }
    crate::encode_thumb_bytes(instruction)?;
    if at.location & 1 != 0 {
        return Err(SemanticError::Encode(EncodeError::UnalignedAddress {
            location: at.location,
            alignment: 2,
        }));
    }
    let next = AnalysisPoint {
        location: at
            .location
            .wrapping_add(u32::try_from(instruction.encoded_len()).unwrap()),
        ..at
    };
    let mut accesses = Vec::new();
    let mut references = Vec::new();
    thumb_accesses(instruction, at, &mut accesses, &mut references);
    Ok(InstructionSemantics {
        control_flow: thumb_control_flow(instruction, at, next),
        location_accesses: accesses,
        static_references: references,
    })
}

fn arm_successor(instruction: &ArmInstruction, at: AnalysisPoint) -> AnalysisPoint {
    let mut next = AnalysisPoint {
        location: at.location.wrapping_add(4),
        ..at
    };
    if matches!(
        instruction,
        ArmInstruction::MoveToStatusRegister {
            destination: StatusRegister::Current,
            mask,
            ..
        } if mask.bits() & StatusMask::CONTROL.bits() != 0
    ) {
        next.state = ExecutionState::Unknown;
        next.mode = ProcessorMode::Unknown;
    }
    next
}

fn arm_control_flow(
    instruction: &ArmInstruction,
    at: AnalysisPoint,
    next: AnalysisPoint,
) -> ControlFlow {
    let condition = arm_condition(instruction);
    let conditional_fallthrough = (condition != Condition::Always).then_some(next);
    match *instruction {
        ArmInstruction::BranchExchange { target, .. } => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Indirect(IndirectTarget::ExchangeRegister(
                    register_location(at.mode, target),
                )),
            },
            fallthrough: conditional_fallthrough,
            delay_slot: None,
        },
        ArmInstruction::Branch {
            link, displacement, ..
        } => {
            let target = ControlTarget::Direct(AnalysisPoint {
                location: at
                    .location
                    .wrapping_add(8)
                    .wrapping_add_signed(displacement),
                ..at
            });
            ControlFlow {
                action: if link {
                    ControlAction::LinkedTransfer {
                        target,
                        return_site: next,
                    }
                } else {
                    ControlAction::Transfer { target }
                },
                fallthrough: conditional_fallthrough,
                delay_slot: None,
            }
        }
        ArmInstruction::DataProcessing {
            destination,
            set_flags,
            operation,
            ..
        } if destination == Register::PC && !operation.is_test() => ControlFlow {
            action: if set_flags {
                ControlAction::ExceptionReturn {
                    target: IndirectTarget::ExceptionReturn,
                }
            } else {
                ControlAction::Transfer {
                    target: ControlTarget::Indirect(IndirectTarget::WrittenProgramCounter),
                }
            },
            fallthrough: conditional_fallthrough,
            delay_slot: None,
        },
        ArmInstruction::SingleTransfer {
            load: true,
            register,
            ..
        } if register == Register::PC => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Indirect(IndirectTarget::WrittenProgramCounter),
            },
            fallthrough: conditional_fallthrough,
            delay_slot: None,
        },
        ArmInstruction::BlockTransfer {
            load: true,
            user_registers,
            registers,
            ..
        } if registers.bits() & (1 << Register::PC.index()) != 0 => ControlFlow {
            action: if user_registers {
                ControlAction::ExceptionReturn {
                    target: IndirectTarget::ExceptionReturn,
                }
            } else {
                ControlAction::Transfer {
                    target: ControlTarget::Indirect(IndirectTarget::WrittenProgramCounter),
                }
            },
            fallthrough: conditional_fallthrough,
            delay_slot: None,
        },
        ArmInstruction::SoftwareInterrupt { comment, .. } => ControlFlow {
            action: ControlAction::Boundary(ControlBoundary::Trap {
                kind: Boundary::SoftwareInterrupt(comment),
                target: Some(ControlTarget::Indirect(IndirectTarget::ExceptionVector {
                    offset: 0x08,
                    mode: ProcessorMode::Supervisor,
                })),
                resume: Some(next),
            }),
            fallthrough: conditional_fallthrough,
            delay_slot: None,
        },
        _ => ControlFlow {
            action: ControlAction::Continue,
            fallthrough: Some(next),
            delay_slot: None,
        },
    }
}

fn thumb_control_flow(
    instruction: &ThumbInstruction,
    at: AnalysisPoint,
    next: AnalysisPoint,
) -> ControlFlow {
    match *instruction {
        ThumbInstruction::BranchExchange { source } => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Indirect(IndirectTarget::ExchangeRegister(
                    register_location(at.mode, source),
                )),
            },
            fallthrough: None,
            delay_slot: None,
        },
        ThumbInstruction::HighRegister {
            operation: ThumbHighOperation::Add | ThumbHighOperation::Move,
            destination,
            ..
        } if destination == Register::PC => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Indirect(IndirectTarget::WrittenProgramCounter),
            },
            fallthrough: None,
            delay_slot: None,
        },
        ThumbInstruction::Pop {
            program_counter: true,
            ..
        } => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Indirect(IndirectTarget::WrittenProgramCounter),
            },
            fallthrough: None,
            delay_slot: None,
        },
        ThumbInstruction::ConditionalBranch { displacement, .. } => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Direct(AnalysisPoint {
                    location: at
                        .location
                        .wrapping_add(4)
                        .wrapping_add_signed(i32::from(displacement)),
                    ..at
                }),
            },
            fallthrough: Some(next),
            delay_slot: None,
        },
        ThumbInstruction::UnconditionalBranch { displacement } => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Direct(AnalysisPoint {
                    location: at
                        .location
                        .wrapping_add(4)
                        .wrapping_add_signed(i32::from(displacement)),
                    ..at
                }),
            },
            fallthrough: None,
            delay_slot: None,
        },
        ThumbInstruction::BranchLink { displacement } => ControlFlow {
            action: ControlAction::LinkedTransfer {
                target: ControlTarget::Direct(AnalysisPoint {
                    location: at
                        .location
                        .wrapping_add(4)
                        .wrapping_add_signed(displacement),
                    ..at
                }),
                return_site: next,
            },
            fallthrough: None,
            delay_slot: None,
        },
        ThumbInstruction::SoftwareInterrupt { comment } => ControlFlow {
            action: ControlAction::Boundary(ControlBoundary::Trap {
                kind: Boundary::SoftwareInterrupt(u32::from(comment)),
                target: Some(ControlTarget::Indirect(IndirectTarget::ExceptionVector {
                    offset: 0x08,
                    mode: ProcessorMode::Supervisor,
                })),
                resume: Some(next),
            }),
            fallthrough: None,
            delay_slot: None,
        },
        _ => ControlFlow {
            action: ControlAction::Continue,
            fallthrough: Some(next),
            delay_slot: None,
        },
    }
}

fn arm_condition(instruction: &ArmInstruction) -> Condition {
    match *instruction {
        ArmInstruction::DataProcessing { condition, .. }
        | ArmInstruction::Multiply { condition, .. }
        | ArmInstruction::MultiplyLong { condition, .. }
        | ArmInstruction::BranchExchange { condition, .. }
        | ArmInstruction::Branch { condition, .. }
        | ArmInstruction::SingleTransfer { condition, .. }
        | ArmInstruction::HalfwordTransfer { condition, .. }
        | ArmInstruction::BlockTransfer { condition, .. }
        | ArmInstruction::Swap { condition, .. }
        | ArmInstruction::MoveStatusToRegister { condition, .. }
        | ArmInstruction::MoveToStatusRegister { condition, .. }
        | ArmInstruction::CoprocessorDataOperation { condition, .. }
        | ArmInstruction::CoprocessorRegisterTransfer { condition, .. }
        | ArmInstruction::CoprocessorDataTransfer { condition, .. }
        | ArmInstruction::SoftwareInterrupt { condition, .. } => condition,
    }
}

fn arm_accesses(
    instruction: &ArmInstruction,
    at: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    use ArmInstruction::*;
    let condition = arm_condition(instruction);
    let occurrence = predicate_occurrence(condition);
    read_condition(accesses, condition);
    match *instruction {
        DataProcessing {
            operation,
            set_flags,
            destination,
            first,
            second,
            ..
        } => {
            if !operation.ignores_first_register() {
                read_register(accesses, at, first, occurrence);
            }
            read_operand2(accesses, at, second, occurrence);
            if matches!(
                operation,
                DataOperation::AddWithCarry
                    | DataOperation::SubtractWithCarry
                    | DataOperation::ReverseSubtractWithCarry
            ) {
                read(accesses, Location::Flag(Flag::Carry), occurrence);
            }
            if !operation.is_test() {
                write_register(accesses, at, destination, occurrence);
            }
            if set_flags || operation.is_test() {
                write_data_processing_flags(accesses, operation, occurrence);
                if data_operation_is_logical(operation) {
                    read_preserved_shifter_carry(accesses, second, occurrence);
                }
            }
            if destination == Register::PC && set_flags && !operation.is_test() {
                read(
                    accesses,
                    Location::Status(saved_status(at.mode)),
                    occurrence,
                );
                write(
                    accesses,
                    Location::Status(StatusLocation::Current),
                    occurrence,
                );
            }
        }
        Multiply {
            accumulate,
            set_flags,
            destination,
            multiplicand,
            multiplier,
            addend,
            ..
        } => {
            for register in [multiplicand, multiplier] {
                read_register(accesses, at, register, occurrence);
            }
            if accumulate {
                read_register(accesses, at, addend, occurrence);
            }
            write_register(accesses, at, destination, occurrence);
            if set_flags {
                write_nz(accesses, occurrence);
            }
        }
        MultiplyLong {
            accumulate,
            set_flags,
            destination_low,
            destination_high,
            multiplicand,
            multiplier,
            ..
        } => {
            for register in [multiplicand, multiplier] {
                read_register(accesses, at, register, occurrence);
            }
            if accumulate {
                for register in [destination_low, destination_high] {
                    read_register(accesses, at, register, occurrence);
                }
            }
            for register in [destination_low, destination_high] {
                write_register(accesses, at, register, occurrence);
            }
            if set_flags {
                write_nz(accesses, occurrence);
            }
        }
        BranchExchange { target, .. } => {
            read_register(accesses, at, target, occurrence);
            write(
                accesses,
                Location::ControlBit(ControlBit::ThumbState),
                occurrence,
            );
        }
        Branch { link, .. } => {
            if link {
                write_register(accesses, at, Register::LINK, occurrence);
            }
        }
        SingleTransfer {
            load,
            width,
            register,
            address,
            ..
        } => transfer_mode2(
            load,
            transfer_width(width),
            register,
            address,
            at,
            occurrence,
            accesses,
            references,
        ),
        HalfwordTransfer {
            load,
            kind,
            register,
            address,
            ..
        } => transfer_mode3(
            load,
            halfword_width(kind),
            register,
            address,
            at,
            occurrence,
            accesses,
            references,
        ),
        BlockTransfer {
            load,
            addressing,
            write_back,
            user_registers,
            base,
            registers,
            ..
        } => block_transfer(
            load,
            addressing,
            write_back,
            user_registers,
            base,
            registers,
            at,
            occurrence,
            accesses,
        ),
        Swap {
            width,
            destination,
            source,
            base,
            ..
        } => {
            read_register(accesses, at, base, occurrence);
            read_register(accesses, at, source, occurrence);
            let memory = Location::Memory(MemoryLocation {
                address: MemoryAddress::ArmMode2 {
                    base: register_location(at.mode, base),
                    offset: AddressOffsetLocation::Immediate(0),
                    add: true,
                    index: IndexMode::Offset,
                },
                width: MemoryWidth::Known(transfer_width(width)),
            });
            read(accesses, memory, occurrence);
            write(accesses, memory, occurrence);
            write_register(accesses, at, destination, occurrence);
        }
        MoveStatusToRegister {
            source,
            destination,
            ..
        } => {
            read(
                accesses,
                Location::Status(status_location(source, at.mode)),
                occurrence,
            );
            write_register(accesses, at, destination, occurrence);
        }
        MoveToStatusRegister {
            destination,
            mask,
            source,
            ..
        } => {
            if let StatusSource::Register(register) = source {
                read_register(accesses, at, register, occurrence);
            }
            let status = status_location(destination, at.mode);
            for (bit, field) in [
                (StatusMask::CONTROL.bits(), StatusField::Control),
                (StatusMask::EXTENSION.bits(), StatusField::Extension),
                (StatusMask::STATUS.bits(), StatusField::Status),
                (StatusMask::FLAGS.bits(), StatusField::Flags),
            ] {
                if mask.bits() & bit != 0 {
                    write(
                        accesses,
                        Location::StatusField { status, field },
                        occurrence,
                    );
                }
            }
        }
        CoprocessorDataOperation {
            coprocessor,
            opcode,
            info,
            destination,
            first,
            second,
            ..
        } => {
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
                Location::CoprocessorRegister {
                    coprocessor,
                    register: destination,
                },
                occurrence,
            );
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
        CoprocessorRegisterTransfer {
            load,
            coprocessor,
            arm_register,
            first,
            second,
            ..
        } => {
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
        CoprocessorDataTransfer {
            load,
            long,
            coprocessor,
            register,
            base,
            offset_words,
            add,
            index,
            ..
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
        SoftwareInterrupt { .. } => {
            exception_accesses(at, ProcessorMode::Supervisor, occurrence, accesses)
        }
    }
}

fn thumb_accesses(
    instruction: &ThumbInstruction,
    at: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    use ThumbInstruction::*;
    let occurrence = AccessOccurrence::Always;
    match *instruction {
        ShiftImmediate {
            kind,
            destination,
            source,
            amount,
        } => {
            read_low(accesses, at, source, occurrence);
            if kind == ShiftKind::LogicalLeft && amount == 0 {
                read(accesses, Location::Flag(Flag::Carry), occurrence);
            }
            write_low(accesses, at, destination, occurrence);
            write_nzc(accesses, occurrence);
        }
        AddSubtract {
            destination,
            source,
            operand,
            ..
        } => {
            read_low(accesses, at, source, occurrence);
            if let ThumbAddOperand::Register(register) = operand {
                read_low(accesses, at, register, occurrence);
            }
            write_low(accesses, at, destination, occurrence);
            write_nzcv(accesses, occurrence);
        }
        Immediate {
            operation,
            register,
            ..
        } => {
            if !matches!(operation, ThumbImmediateOperation::Move) {
                read_low(accesses, at, register, occurrence);
            }
            if !matches!(operation, ThumbImmediateOperation::Compare) {
                write_low(accesses, at, register, occurrence);
            }
            if operation == ThumbImmediateOperation::Move {
                write_nz(accesses, occurrence);
            } else {
                write_nzcv(accesses, occurrence);
            }
        }
        Alu {
            operation,
            destination,
            source,
        } => {
            if !matches!(
                operation,
                ThumbAluOperation::Negate | ThumbAluOperation::MoveNot
            ) {
                read_low(accesses, at, destination, occurrence);
            }
            read_low(accesses, at, source, occurrence);
            if matches!(
                operation,
                ThumbAluOperation::AddWithCarry | ThumbAluOperation::SubtractWithCarry
            ) {
                read(accesses, Location::Flag(Flag::Carry), occurrence);
            }
            if matches!(
                operation,
                ThumbAluOperation::LogicalShiftLeft
                    | ThumbAluOperation::LogicalShiftRight
                    | ThumbAluOperation::ArithmeticShiftRight
                    | ThumbAluOperation::RotateRight
            ) {
                read(
                    accesses,
                    Location::Flag(Flag::Carry),
                    AccessOccurrence::Conditional,
                );
            }
            if !matches!(
                operation,
                ThumbAluOperation::Test
                    | ThumbAluOperation::Compare
                    | ThumbAluOperation::CompareNegative
            ) {
                write_low(accesses, at, destination, occurrence);
            }
            match operation {
                ThumbAluOperation::LogicalShiftLeft
                | ThumbAluOperation::LogicalShiftRight
                | ThumbAluOperation::ArithmeticShiftRight
                | ThumbAluOperation::RotateRight => write_nzc(accesses, occurrence),
                ThumbAluOperation::AddWithCarry
                | ThumbAluOperation::SubtractWithCarry
                | ThumbAluOperation::Negate
                | ThumbAluOperation::Compare
                | ThumbAluOperation::CompareNegative => write_nzcv(accesses, occurrence),
                ThumbAluOperation::And
                | ThumbAluOperation::ExclusiveOr
                | ThumbAluOperation::Test
                | ThumbAluOperation::Or
                | ThumbAluOperation::Multiply
                | ThumbAluOperation::BitClear
                | ThumbAluOperation::MoveNot => write_nz(accesses, occurrence),
            }
        }
        HighRegister {
            operation,
            destination,
            source,
        } => {
            read_register(accesses, at, source, occurrence);
            if !matches!(operation, ThumbHighOperation::Move) {
                read_register(accesses, at, destination, occurrence);
            }
            if matches!(operation, ThumbHighOperation::Compare) {
                write_nzcv(accesses, occurrence);
            } else {
                write_register(accesses, at, destination, occurrence);
            }
        }
        BranchExchange { source } => {
            read_register(accesses, at, source, occurrence);
            write(
                accesses,
                Location::ControlBit(ControlBit::ThumbState),
                occurrence,
            );
        }
        PcRelativeLoad {
            destination,
            offset,
        } => {
            let address = at.location.wrapping_add(4) & !3_u32;
            let address = address.wrapping_add(u32::from(offset));
            read(
                accesses,
                Location::Memory(MemoryLocation {
                    address: MemoryAddress::Literal(address),
                    width: MemoryWidth::Known(4),
                }),
                occurrence,
            );
            write_low(accesses, at, destination, occurrence);
            add_reference(
                references,
                StaticReference {
                    target: Reference::Memory {
                        address,
                        byte_len: 4,
                    },
                    kind: ReferenceKind::Read,
                    occurrence,
                },
            );
        }
        RegisterTransfer {
            operation,
            destination,
            base,
            offset,
        } => {
            let (load, width) = thumb_register_transfer(operation);
            read_low(accesses, at, base, occurrence);
            read_low(accesses, at, offset, occurrence);
            let memory = Location::Memory(MemoryLocation {
                address: MemoryAddress::ThumbRegisterOffset {
                    base: register_location(at.mode, base.as_register()),
                    offset: register_location(at.mode, offset.as_register()),
                },
                width: MemoryWidth::Known(width),
            });
            if load {
                read(accesses, memory, occurrence);
                write_low(accesses, at, destination, occurrence);
            } else {
                read_low(accesses, at, destination, occurrence);
                write(accesses, memory, occurrence);
            }
        }
        ImmediateTransfer {
            load,
            width,
            destination,
            base,
            offset,
        } => {
            let width = thumb_width(width);
            read_low(accesses, at, base, occurrence);
            let memory = Location::Memory(MemoryLocation {
                address: MemoryAddress::ThumbImmediateOffset {
                    base: register_location(at.mode, base.as_register()),
                    offset: u16::from(offset),
                },
                width: MemoryWidth::Known(width),
            });
            if load {
                read(accesses, memory, occurrence);
                write_low(accesses, at, destination, occurrence);
            } else {
                read_low(accesses, at, destination, occurrence);
                write(accesses, memory, occurrence);
            }
        }
        SpRelativeTransfer {
            load,
            register,
            offset,
        } => {
            read_register(accesses, at, Register::SP, occurrence);
            let memory = Location::Memory(MemoryLocation {
                address: MemoryAddress::ThumbImmediateOffset {
                    base: register_location(at.mode, Register::SP),
                    offset,
                },
                width: MemoryWidth::Known(4),
            });
            if load {
                read(accesses, memory, occurrence);
                write_low(accesses, at, register, occurrence);
            } else {
                read_low(accesses, at, register, occurrence);
                write(accesses, memory, occurrence);
            }
        }
        LoadAddress {
            from_sp,
            destination,
            offset,
        } => {
            if from_sp {
                read_register(accesses, at, Register::SP, occurrence);
            } else {
                let address =
                    (at.location.wrapping_add(4) & !3_u32).wrapping_add(u32::from(offset));
                add_reference(
                    references,
                    StaticReference {
                        target: Reference::Address(address),
                        kind: ReferenceKind::Address,
                        occurrence,
                    },
                );
            }
            write_low(accesses, at, destination, occurrence);
        }
        AddStackPointer { .. } => {
            read_register(accesses, at, Register::SP, occurrence);
            write_register(accesses, at, Register::SP, occurrence);
        }
        Push { registers, link } => {
            for register in thumb_registers(registers) {
                read_register(accesses, at, register, occurrence);
            }
            if link {
                read_register(accesses, at, Register::LINK, occurrence);
            }
            stack_transfer(
                false,
                thumb_count(registers) + u8::from(link),
                at,
                occurrence,
                accesses,
            );
        }
        Pop {
            registers,
            program_counter,
        } => {
            stack_transfer(
                true,
                thumb_count(registers) + u8::from(program_counter),
                at,
                occurrence,
                accesses,
            );
            for register in thumb_registers(registers) {
                write_register(accesses, at, register, occurrence);
            }
            if program_counter {
                write_register(accesses, at, Register::PC, occurrence);
            }
        }
        MultipleTransfer {
            load,
            base,
            registers,
        } => {
            read_low(accesses, at, base, occurrence);
            let memory = Location::Memory(MemoryLocation {
                address: MemoryAddress::ArmBlock {
                    base: register_location(at.mode, base.as_register()),
                    addressing: BlockAddressing::IncrementAfter,
                },
                width: MemoryWidth::Known(thumb_count(registers) * 4),
            });
            if load {
                read(accesses, memory, occurrence);
                for register in thumb_registers(registers) {
                    write_register(accesses, at, register, occurrence);
                }
            } else {
                for register in thumb_registers(registers) {
                    read_register(accesses, at, register, occurrence);
                }
                write(accesses, memory, occurrence);
            }
            write_low(accesses, at, base, occurrence);
        }
        ConditionalBranch { condition, .. } => read_condition(accesses, condition),
        UnconditionalBranch { .. } => {}
        BranchLink { .. } => write_register(accesses, at, Register::LINK, occurrence),
        SoftwareInterrupt { .. } => {
            exception_accesses(at, ProcessorMode::Supervisor, occurrence, accesses)
        }
    }
}

fn predicate_occurrence(condition: Condition) -> AccessOccurrence {
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

const fn register_location(mode: ProcessorMode, register: Register) -> RegisterLocation {
    match register.index() {
        0..=7 => RegisterLocation::Unbanked(register),
        8..=12 => match mode {
            ProcessorMode::Fiq => RegisterLocation::High {
                register,
                bank: RegisterBank::Fiq,
            },
            ProcessorMode::Unknown => RegisterLocation::CurrentUnknown(register),
            _ => RegisterLocation::High {
                register,
                bank: RegisterBank::UserSystem,
            },
        },
        13 | 14 => match mode_bank(mode) {
            Some(bank) => RegisterLocation::Banked { register, bank },
            None => RegisterLocation::CurrentUnknown(register),
        },
        15 => RegisterLocation::ProgramCounter,
        _ => unreachable!(),
    }
}

const fn mode_bank(mode: ProcessorMode) -> Option<RegisterBank> {
    match mode {
        ProcessorMode::User | ProcessorMode::System => Some(RegisterBank::UserSystem),
        ProcessorMode::Fiq => Some(RegisterBank::Fiq),
        ProcessorMode::Irq => Some(RegisterBank::Irq),
        ProcessorMode::Supervisor => Some(RegisterBank::Supervisor),
        ProcessorMode::Abort => Some(RegisterBank::Abort),
        ProcessorMode::Undefined => Some(RegisterBank::Undefined),
        ProcessorMode::Unknown => None,
    }
}

fn saved_status(mode: ProcessorMode) -> StatusLocation {
    if matches!(
        mode,
        ProcessorMode::User | ProcessorMode::System | ProcessorMode::Unknown
    ) {
        StatusLocation::SavedUnknown
    } else {
        StatusLocation::Saved(mode)
    }
}

fn status_location(status: StatusRegister, mode: ProcessorMode) -> StatusLocation {
    match status {
        StatusRegister::Current => StatusLocation::Current,
        StatusRegister::Saved => saved_status(mode),
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
        Location::Register(register_location(at.mode, register)),
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
        Location::Register(register_location(at.mode, register)),
        occurrence,
    );
}

fn read_low(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    register: LowRegister,
    occurrence: AccessOccurrence,
) {
    read_register(accesses, at, register.as_register(), occurrence);
}

fn write_low(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    register: LowRegister,
    occurrence: AccessOccurrence,
) {
    write_register(accesses, at, register.as_register(), occurrence);
}

fn read_operand2(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    operand: Operand2,
    occurrence: AccessOccurrence,
) {
    match operand {
        Operand2::Immediate(_) => {}
        Operand2::Register { register, shift } => {
            read_register(accesses, at, register, occurrence);
            if matches!(shift, ImmediateShift::RotateRightExtended) {
                read(accesses, Location::Flag(Flag::Carry), occurrence);
            }
        }
        Operand2::RegisterShift {
            register, amount, ..
        } => {
            read_register(accesses, at, register, occurrence);
            read_register(accesses, at, amount, occurrence);
        }
    }
}

fn read_preserved_shifter_carry(
    accesses: &mut Vec<LocationAccess<Location>>,
    operand: Operand2,
    occurrence: AccessOccurrence,
) {
    match operand {
        Operand2::Immediate(immediate) if immediate.rotate_right() == 0 => {
            read(accesses, Location::Flag(Flag::Carry), occurrence);
        }
        Operand2::Register {
            shift: ImmediateShift::LogicalLeft(0),
            ..
        } => read(accesses, Location::Flag(Flag::Carry), occurrence),
        Operand2::RegisterShift { .. } => read(
            accesses,
            Location::Flag(Flag::Carry),
            AccessOccurrence::Conditional,
        ),
        _ => {}
    }
}

fn mode2_address(at: AnalysisPoint, address: AddressingMode2) -> MemoryAddress {
    MemoryAddress::ArmMode2 {
        base: register_location(at.mode, address.base),
        offset: match address.offset {
            AddressOffset::Immediate(offset) => AddressOffsetLocation::Immediate(offset),
            AddressOffset::Register { register, shift } => AddressOffsetLocation::Register {
                register: register_location(at.mode, register),
                shift,
            },
        },
        add: address.add,
        index: address.index,
    }
}

fn mode3_address(at: AnalysisPoint, address: AddressingMode3) -> MemoryAddress {
    MemoryAddress::ArmMode3 {
        base: register_location(at.mode, address.base),
        offset: match address.offset {
            HalfwordOffset::Immediate(offset) => HalfwordOffsetLocation::Immediate(offset),
            HalfwordOffset::Register(register) => {
                HalfwordOffsetLocation::Register(register_location(at.mode, register))
            }
        },
        add: address.add,
        index: address.index,
    }
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
        if matches!(shift, ImmediateShift::RotateRightExtended) {
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

#[allow(clippy::too_many_arguments)]
fn transfer_mode2(
    load: bool,
    byte_len: u8,
    register: Register,
    address: AddressingMode2,
    at: AnalysisPoint,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    address_inputs_mode2(accesses, at, address, occurrence);
    let resolved = pc_relative_mode2(at, address);
    let memory = Location::Memory(MemoryLocation {
        address: resolved.map_or_else(|| mode2_address(at, address), MemoryAddress::Literal),
        width: MemoryWidth::Known(byte_len),
    });
    if load {
        read(accesses, memory, occurrence);
        write_register(accesses, at, register, occurrence);
    } else {
        read_register(accesses, at, register, occurrence);
        write(accesses, memory, occurrence);
    }
    if let Some(address) = resolved {
        add_reference(
            references,
            StaticReference {
                target: Reference::Memory { address, byte_len },
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
fn transfer_mode3(
    load: bool,
    byte_len: u8,
    register: Register,
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
        width: MemoryWidth::Known(byte_len),
    });
    if load {
        read(accesses, memory, occurrence);
        write_register(accesses, at, register, occurrence);
    } else {
        read_register(accesses, at, register, occurrence);
        write(accesses, memory, occurrence);
    }
    if let Some(address) = resolved {
        add_reference(
            references,
            StaticReference {
                target: Reference::Memory { address, byte_len },
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

fn pc_relative_mode2(at: AnalysisPoint, address: AddressingMode2) -> Option<u32> {
    if address.base != Register::PC
        || address.index == IndexMode::PostIndexed
        || address.index == IndexMode::PostIndexedUser
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
        || address.index == IndexMode::PostIndexed
        || address.index == IndexMode::PostIndexedUser
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
fn block_transfer(
    load: bool,
    addressing: BlockAddressing,
    write_back: bool,
    user_registers: bool,
    base: Register,
    registers: RegisterList,
    at: AnalysisPoint,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_register(accesses, at, base, occurrence);
    let count = registers.bits().count_ones() as u8;
    let memory = Location::Memory(MemoryLocation {
        address: MemoryAddress::ArmBlock {
            base: register_location(at.mode, base),
            addressing,
        },
        width: MemoryWidth::Known(count * 4),
    });
    let exception_return =
        load && user_registers && registers.bits() & (1 << Register::PC.index()) != 0;
    let transfer_mode = if user_registers && !exception_return {
        ProcessorMode::User
    } else {
        at.mode
    };
    if load {
        read(accesses, memory, occurrence);
        for register in registers_from_bits(registers.bits()) {
            write(
                accesses,
                Location::Register(register_location(transfer_mode, register)),
                occurrence,
            );
        }
    } else {
        for register in registers_from_bits(registers.bits()) {
            read(
                accesses,
                Location::Register(register_location(transfer_mode, register)),
                occurrence,
            );
        }
        write(accesses, memory, occurrence);
    }
    if write_back {
        write_register(accesses, at, base, occurrence);
    }
    if exception_return {
        read(
            accesses,
            Location::Status(saved_status(at.mode)),
            occurrence,
        );
        write(
            accesses,
            Location::Status(StatusLocation::Current),
            occurrence,
        );
    }
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
            base: register_location(at.mode, base),
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

fn exception_accesses(
    at: AnalysisPoint,
    target_mode: ProcessorMode,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read(
        accesses,
        Location::Status(StatusLocation::Current),
        occurrence,
    );
    write(
        accesses,
        Location::Status(StatusLocation::Saved(target_mode)),
        occurrence,
    );
    write(
        accesses,
        Location::Status(StatusLocation::Current),
        occurrence,
    );
    write(
        accesses,
        Location::Register(register_location(target_mode, Register::LINK)),
        occurrence,
    );
    let _ = at;
}

fn stack_transfer(
    load: bool,
    count: u8,
    at: AnalysisPoint,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_register(accesses, at, Register::SP, occurrence);
    let memory = Location::Memory(MemoryLocation {
        address: MemoryAddress::Stack,
        width: MemoryWidth::Known(count * 4),
    });
    if load {
        read(accesses, memory, occurrence);
    } else {
        write(accesses, memory, occurrence);
    }
    write_register(accesses, at, Register::SP, occurrence);
}

fn transfer_width(width: TransferWidth) -> u8 {
    match width {
        TransferWidth::Word => 4,
        TransferWidth::Byte => 1,
    }
}

fn halfword_width(kind: HalfwordTransferKind) -> u8 {
    match kind {
        HalfwordTransferKind::SignedByte => 1,
        HalfwordTransferKind::UnsignedHalfword | HalfwordTransferKind::SignedHalfword => 2,
    }
}

fn thumb_width(width: ThumbTransferWidth) -> u8 {
    match width {
        ThumbTransferWidth::Word => 4,
        ThumbTransferWidth::Halfword => 2,
        ThumbTransferWidth::Byte => 1,
    }
}

fn thumb_register_transfer(operation: ThumbTransferOperation) -> (bool, u8) {
    match operation {
        ThumbTransferOperation::StoreWord => (false, 4),
        ThumbTransferOperation::StoreHalfword => (false, 2),
        ThumbTransferOperation::StoreByte => (false, 1),
        ThumbTransferOperation::LoadSignedByte | ThumbTransferOperation::LoadByte => (true, 1),
        ThumbTransferOperation::LoadWord => (true, 4),
        ThumbTransferOperation::LoadHalfword | ThumbTransferOperation::LoadSignedHalfword => {
            (true, 2)
        }
    }
}

fn registers_from_bits(bits: u16) -> impl Iterator<Item = Register> {
    (0_u8..16)
        .filter(move |index| bits & (1_u16 << index) != 0)
        .map(|index| Register::new(index).unwrap())
}

fn thumb_registers(list: ThumbRegisterList) -> impl Iterator<Item = Register> {
    registers_from_bits(u16::from(list.bits()))
}

fn thumb_count(list: ThumbRegisterList) -> u8 {
    list.bits().count_ones() as u8
}

fn write_nz(accesses: &mut Vec<LocationAccess<Location>>, occurrence: AccessOccurrence) {
    for flag in [Flag::Negative, Flag::Zero] {
        write(accesses, Location::Flag(flag), occurrence);
    }
}

fn write_nzc(accesses: &mut Vec<LocationAccess<Location>>, occurrence: AccessOccurrence) {
    for flag in [Flag::Negative, Flag::Zero, Flag::Carry] {
        write(accesses, Location::Flag(flag), occurrence);
    }
}

fn write_data_processing_flags(
    accesses: &mut Vec<LocationAccess<Location>>,
    operation: DataOperation,
    occurrence: AccessOccurrence,
) {
    if data_operation_is_logical(operation) {
        write_nzc(accesses, occurrence);
    } else {
        write_nzcv(accesses, occurrence);
    }
}

fn data_operation_is_logical(operation: DataOperation) -> bool {
    matches!(
        operation,
        DataOperation::And
            | DataOperation::ExclusiveOr
            | DataOperation::Test
            | DataOperation::TestEquivalence
            | DataOperation::Or
            | DataOperation::Move
            | DataOperation::BitClear
            | DataOperation::MoveNot
    )
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

pub(crate) fn storage_relation(left: &Location, right: &Location) -> StorageRelation {
    use StorageRelation::*;
    if left == right {
        return Same;
    }
    match (left, right) {
        (Location::Status(StatusLocation::Current), Location::Flag(_))
        | (Location::Flag(_), Location::Status(StatusLocation::Current))
        | (Location::Status(StatusLocation::Current), Location::ControlBit(_))
        | (Location::ControlBit(_), Location::Status(StatusLocation::Current)) => Overlapping,
        (
            Location::Status(left_status),
            Location::StatusField {
                status: right_status,
                ..
            },
        )
        | (
            Location::StatusField {
                status: right_status,
                ..
            },
            Location::Status(left_status),
        ) if left_status == right_status => Overlapping,
        (
            Location::StatusField {
                status: StatusLocation::Current,
                field: StatusField::Flags,
            },
            Location::Flag(_),
        )
        | (
            Location::Flag(_),
            Location::StatusField {
                status: StatusLocation::Current,
                field: StatusField::Flags,
            },
        )
        | (
            Location::StatusField {
                status: StatusLocation::Current,
                field: StatusField::Control,
            },
            Location::ControlBit(_),
        )
        | (
            Location::ControlBit(_),
            Location::StatusField {
                status: StatusLocation::Current,
                field: StatusField::Control,
            },
        ) => Overlapping,
        (Location::Register(RegisterLocation::CurrentUnknown(left)), Location::Register(right))
        | (Location::Register(right), Location::Register(RegisterLocation::CurrentUnknown(left)))
            if register_identity(*right) == Some(*left) =>
        {
            Unknown
        }
        (Location::Memory(left), Location::Memory(right)) => memory_relation(*left, *right),
        (Location::Memory(_), _) | (_, Location::Memory(_)) => Disjoint,
        _ => Disjoint,
    }
}

fn register_identity(location: RegisterLocation) -> Option<Register> {
    match location {
        RegisterLocation::Unbanked(register)
        | RegisterLocation::High { register, .. }
        | RegisterLocation::Banked { register, .. }
        | RegisterLocation::CurrentUnknown(register) => Some(register),
        RegisterLocation::ProgramCounter => Some(Register::PC),
    }
}

fn memory_relation(left: MemoryLocation, right: MemoryLocation) -> StorageRelation {
    let (MemoryAddress::Literal(left_address), MemoryAddress::Literal(right_address)) =
        (left.address, right.address)
    else {
        return StorageRelation::Unknown;
    };
    let (MemoryWidth::Known(left_len), MemoryWidth::Known(right_len)) = (left.width, right.width)
    else {
        return StorageRelation::Unknown;
    };
    let overlaps = (0..u32::from(left_len)).any(|offset| {
        let byte = left_address.wrapping_add(offset);
        (0..u32::from(right_len)).any(|other| byte == right_address.wrapping_add(other))
    });
    if overlaps {
        StorageRelation::Overlapping
    } else {
        StorageRelation::Disjoint
    }
}
