//! Complete static meaning for the TLCS-900/H instruction profile.

use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlBoundary, ControlTarget, LocationAccess,
    ReferenceKind, StaticReference, StorageRelation,
};

use crate::{
    ArithmeticOperation, BitOperation, Condition, CurrentRegister, DestinationMemoryInstruction,
    DestinationMemoryOperation, EncodeError, GeneralRegister, Instruction, MainInstruction,
    MemoryAddress as OperandAddress, OperandSize, RegisterEncoding, RegisterInstruction,
    RegisterOperation, ShiftOperation, SourceMemoryInstruction, SourceMemoryOperation,
};

/// Statically known context at one TLCS-900/H instruction address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnalysisPoint {
    /// 24-bit architectural program address.
    pub location: crate::CodeLocation,
    /// Current register-file bank when known by the analysis.
    pub current_bank: CurrentBank,
}

impl AnalysisPoint {
    /// Constructs a placed instruction with an unknown current register bank.
    pub const fn new(location: crate::CodeLocation) -> Self {
        Self {
            location: location & 0x00ff_ffff,
            current_bank: CurrentBank::Unknown,
        }
    }

    /// Constructs a placed instruction with a known current register bank.
    pub const fn with_current_bank(location: crate::CodeLocation, bank: u8) -> Self {
        Self {
            location: location & 0x00ff_ffff,
            current_bank: CurrentBank::Known(bank & 7),
        }
    }
}

/// Current-register bank selected by the register-file pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CurrentBank {
    /// Statically known three-bit bank number.
    Known(u8),
    /// Bank cannot be recovered statically.
    Unknown,
}

/// Runtime-derived TLCS-900/H control destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndirectTarget {
    /// Sixteen-bit target interpreted in the current program bank.
    ProgramBankOffset(u16),
    /// Target formed from an effective-address operand.
    EffectiveAddress(OperandAddress),
    /// Return address loaded from the system stack.
    StackReturn,
    /// Return address and status loaded from an interrupt frame.
    InterruptFrame,
    /// Handler selected by a software-interrupt vector.
    SoftwareInterruptVector(u8),
}

/// Synchronous TLCS-900/H execution boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Boundary {
    /// HALT waits for an accepted interrupt or reset.
    Halt,
    /// Software interrupt selected by the embedded vector number.
    SoftwareInterrupt(u8),
}

/// Explicit register storage selected by an operand prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegisterLocation {
    /// Register in the current bank selected by the register-file pointer.
    Current {
        /// Current-bank knowledge at the instruction point.
        bank: CurrentBank,
        /// Size-specific current-register selector.
        register: CurrentRegister,
        /// Access width.
        size: OperandSize,
    },
    /// Extended register-map byte range.
    Extended {
        /// First register-map byte code.
        register: GeneralRegister,
        /// Access width.
        size: OperandSize,
    },
}

/// Stable implicit architectural state used by instruction families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImplicitState {
    /// System stack pointer and its addressed stack storage.
    StackPointer,
    /// Primary accumulator used by dedicated accumulator forms.
    Accumulator,
    /// Register-file pointer selecting current-register storage.
    RegisterFilePointer,
    /// Interrupt-priority mask.
    InterruptMask,
    /// Source pointer used by block/string operations.
    StringSource,
    /// Destination pointer used by block/string operations.
    StringDestination,
    /// Counter used by block/string and repeat operations.
    StringCounter,
    /// Accumulator pair used by multiply-accumulate.
    MultiplyAccumulator,
}

/// TLCS-900/H architectural storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Location {
    /// Explicit current or extended register storage.
    Register(RegisterLocation),
    /// Whole status register.
    Status,
    /// Status flag set considered as one unit.
    Flags,
    /// Carry flag addressed independently by carry instructions.
    CarryFlag,
    /// One encoded control register.
    ControlRegister(u8),
    /// Stable implicit state used by an instruction family.
    Implicit(ImplicitState),
    /// Symbolic memory storage.
    Memory(MemoryLocation),
}

/// One symbolic TLCS-900/H memory access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryLocation {
    /// Typed effective-address rule.
    pub address: EffectiveAddress,
    /// Access width in bytes.
    pub byte_len: u8,
}

/// Address source of a semantic memory access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectiveAddress {
    /// Address explicitly encoded by an instruction operand.
    Operand(OperandAddress),
    /// System-stack storage addressed through the architectural stack pointer.
    Stack,
    /// Destination storage selected by the block/string destination pointer.
    StringDestination,
}

/// Statically resolved TLCS-900/H address reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reference {
    /// Contiguous memory range.
    Memory {
        /// First referenced byte.
        address: u32,
        /// Referenced byte count.
        byte_len: u8,
    },
    /// Address produced or used without claiming a memory read.
    Address(u32),
}

/// Failure to form sound TLCS-900/H static facts.
#[derive(Debug, thiserror::Error)]
pub enum SemanticError {
    /// The typed instruction is not encodable in the exact profile.
    #[error(transparent)]
    Encode(#[from] EncodeError),
}

/// Shared control-flow result specialized with TLCS-900/H vocabulary.
pub type ControlFlow = typed_isa_core::ControlFlow<AnalysisPoint, IndirectTarget, Boundary>;

/// Shared instruction-semantics result specialized with TLCS-900/H vocabulary.
pub type InstructionSemantics = typed_isa_core::InstructionSemantics<
    AnalysisPoint,
    IndirectTarget,
    Boundary,
    Location,
    Reference,
>;

pub(crate) fn semantics(
    instruction: &Instruction,
    at: AnalysisPoint,
) -> Result<InstructionSemantics, SemanticError> {
    let bytes = crate::encode_bytes(instruction)?;
    let next = AnalysisPoint {
        location: at.location.wrapping_add(bytes.len() as u32) & 0x00ff_ffff,
        ..at
    };
    let mut accesses = Vec::new();
    let mut references = Vec::new();
    instruction_accesses(instruction, at, next, &mut accesses, &mut references);
    Ok(InstructionSemantics {
        control_flow: control_flow(instruction, at, next),
        location_accesses: accesses,
        static_references: references,
    })
}

fn control_flow(instruction: &Instruction, at: AnalysisPoint, next: AnalysisPoint) -> ControlFlow {
    use MainInstruction as M;
    match *instruction {
        Instruction::Main(M::Halt) => ControlFlow {
            action: ControlAction::Boundary(ControlBoundary::Wait {
                kind: Boundary::Halt,
                resume: next,
            }),
            fallthrough: None,
            delay_slot: None,
        },
        Instruction::Main(M::Return | M::ReturnAndDeallocate(_)) => return_flow(None, next),
        Instruction::Main(M::ReturnFromInterrupt) => ControlFlow {
            action: ControlAction::ExceptionReturn {
                target: IndirectTarget::InterruptFrame,
            },
            fallthrough: None,
            delay_slot: None,
        },
        Instruction::Main(M::JumpDirect16(target)) => transfer(
            ControlTarget::Indirect(IndirectTarget::ProgramBankOffset(target)),
            None,
        ),
        Instruction::Main(M::JumpDirect24(target)) => transfer(
            ControlTarget::Direct(AnalysisPoint {
                location: target & 0x00ff_ffff,
                ..at
            }),
            None,
        ),
        Instruction::Main(M::CallDirect16(target)) => linked_transfer(
            ControlTarget::Indirect(IndirectTarget::ProgramBankOffset(target)),
            next,
            None,
        ),
        Instruction::Main(M::CallDirect24(target)) => linked_transfer(
            ControlTarget::Direct(AnalysisPoint {
                location: target & 0x00ff_ffff,
                ..at
            }),
            next,
            None,
        ),
        Instruction::Main(M::CallRelative(displacement)) => linked_transfer(
            ControlTarget::Direct(relative(next, i32::from(displacement))),
            next,
            None,
        ),
        Instruction::Main(M::JumpRelative {
            condition,
            displacement,
        }) => conditional_transfer(condition, relative(next, i32::from(displacement)), next),
        Instruction::Main(M::JumpRelativeLong {
            condition,
            displacement,
        }) => conditional_transfer(condition, relative(next, i32::from(displacement)), next),
        Instruction::Main(M::SoftwareInterrupt(vector)) => ControlFlow {
            action: ControlAction::Boundary(ControlBoundary::Trap {
                kind: Boundary::SoftwareInterrupt(vector),
                target: Some(ControlTarget::Indirect(
                    IndirectTarget::SoftwareInterruptVector(vector),
                )),
                resume: Some(next),
            }),
            fallthrough: None,
            delay_slot: None,
        },
        Instruction::Register(RegisterOperation {
            instruction: RegisterInstruction::DecrementJump(displacement),
            ..
        }) => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Direct(relative(next, i32::from(displacement))),
            },
            fallthrough: Some(next),
            delay_slot: None,
        },
        Instruction::DestinationMemory(DestinationMemoryOperation {
            destination,
            instruction: DestinationMemoryInstruction::Jump(condition),
        }) => conditional_indirect_transfer(condition, destination, next, false),
        Instruction::DestinationMemory(DestinationMemoryOperation {
            destination,
            instruction: DestinationMemoryInstruction::Call(condition),
        }) => conditional_indirect_transfer(condition, destination, next, true),
        Instruction::DestinationMemory(DestinationMemoryOperation {
            instruction: DestinationMemoryInstruction::Return(condition),
            ..
        }) => return_flow(condition_fallthrough(condition, next), next),
        _ => ControlFlow {
            action: ControlAction::Continue,
            fallthrough: Some(next),
            delay_slot: None,
        },
    }
}

fn transfer(
    target: ControlTarget<AnalysisPoint, IndirectTarget>,
    fallthrough: Option<AnalysisPoint>,
) -> ControlFlow {
    ControlFlow {
        action: ControlAction::Transfer { target },
        fallthrough,
        delay_slot: None,
    }
}

fn linked_transfer(
    target: ControlTarget<AnalysisPoint, IndirectTarget>,
    return_site: AnalysisPoint,
    fallthrough: Option<AnalysisPoint>,
) -> ControlFlow {
    ControlFlow {
        action: ControlAction::LinkedTransfer {
            target,
            return_site,
        },
        fallthrough,
        delay_slot: None,
    }
}

fn return_flow(fallthrough: Option<AnalysisPoint>, _next: AnalysisPoint) -> ControlFlow {
    ControlFlow {
        action: ControlAction::Return {
            target: IndirectTarget::StackReturn,
        },
        fallthrough,
        delay_slot: None,
    }
}

fn conditional_transfer(
    condition: Condition,
    target: AnalysisPoint,
    next: AnalysisPoint,
) -> ControlFlow {
    match condition {
        Condition::False => ControlFlow {
            action: ControlAction::Continue,
            fallthrough: Some(next),
            delay_slot: None,
        },
        Condition::True => transfer(ControlTarget::Direct(target), None),
        _ => transfer(ControlTarget::Direct(target), Some(next)),
    }
}

fn conditional_indirect_transfer(
    condition: Condition,
    address: OperandAddress,
    next: AnalysisPoint,
    link: bool,
) -> ControlFlow {
    if condition == Condition::False {
        return ControlFlow {
            action: ControlAction::Continue,
            fallthrough: Some(next),
            delay_slot: None,
        };
    }
    let target = resolved_address(address, next)
        .map(|location| ControlTarget::Direct(AnalysisPoint { location, ..next }))
        .unwrap_or(ControlTarget::Indirect(IndirectTarget::EffectiveAddress(
            address,
        )));
    let fallthrough = (condition != Condition::True).then_some(next);
    if link {
        linked_transfer(target, next, fallthrough)
    } else {
        transfer(target, fallthrough)
    }
}

fn condition_fallthrough(condition: Condition, next: AnalysisPoint) -> Option<AnalysisPoint> {
    (condition != Condition::True).then_some(next)
}

fn relative(next: AnalysisPoint, displacement: i32) -> AnalysisPoint {
    AnalysisPoint {
        location: next.location.wrapping_add_signed(displacement) & 0x00ff_ffff,
        ..next
    }
}

fn instruction_accesses(
    instruction: &Instruction,
    at: AnalysisPoint,
    next: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    match *instruction {
        Instruction::Main(main) => main_accesses(main, at, accesses),
        Instruction::Register(operation) => register_accesses(operation, at, accesses),
        Instruction::SourceMemory(operation) => {
            source_memory_accesses(operation, at, next, accesses, references)
        }
        Instruction::DestinationMemory(operation) => {
            destination_memory_accesses(operation, at, next, accesses, references)
        }
    }
}

fn main_accesses(
    instruction: MainInstruction,
    at: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    use MainInstruction::*;
    match instruction {
        Nop | Halt | JumpDirect16(_) | JumpDirect24(_) | CallRelative(_) => {}
        PushStatus => {
            read(accesses, Location::Status, AccessOccurrence::Always);
            stack_write(accesses, 2);
        }
        PopStatus => {
            stack_read(accesses, 2);
            write(accesses, Location::Status, AccessOccurrence::Always);
        }
        EnableInterrupt(_) | DisableInterrupt => write(
            accesses,
            Location::Implicit(ImplicitState::InterruptMask),
            AccessOccurrence::Always,
        ),
        ReturnFromInterrupt => {
            stack_read(accesses, 6);
            write(accesses, Location::Status, AccessOccurrence::Always);
        }
        LoadDirect8Byte { address, .. } => memory_write(
            accesses,
            OperandAddress::Direct8(address),
            1,
            AccessOccurrence::Always,
        ),
        PushImmediateByte(_) => stack_write(accesses, 1),
        LoadDirect8Word { address, .. } => memory_write(
            accesses,
            OperandAddress::Direct8(address),
            2,
            AccessOccurrence::Always,
        ),
        PushImmediateWord(_) => stack_write(accesses, 2),
        IncrementRegisterFilePointer
        | DecrementRegisterFilePointer
        | LoadRegisterFilePointer(_) => write(
            accesses,
            Location::Implicit(ImplicitState::RegisterFilePointer),
            AccessOccurrence::Always,
        ),
        Return => stack_read(accesses, 4),
        ReturnAndDeallocate(_) => {
            stack_read(accesses, 4);
            read_write_implicit(accesses, ImplicitState::StackPointer);
        }
        ResetCarry | SetCarry => write(accesses, Location::CarryFlag, AccessOccurrence::Always),
        ComplementCarry => read_write(accesses, Location::CarryFlag),
        ZeroToCarry => {
            read(accesses, Location::Flags, AccessOccurrence::Always);
            write(accesses, Location::CarryFlag, AccessOccurrence::Always);
        }
        PushAccumulator => {
            read(
                accesses,
                Location::Implicit(ImplicitState::Accumulator),
                AccessOccurrence::Always,
            );
            stack_write(accesses, 4);
        }
        PopAccumulator => {
            stack_read(accesses, 4);
            write(
                accesses,
                Location::Implicit(ImplicitState::Accumulator),
                AccessOccurrence::Always,
            );
        }
        ExchangeFlags => read_write(accesses, Location::Flags),
        PushFlags => {
            read(accesses, Location::Flags, AccessOccurrence::Always);
            stack_write(accesses, 2);
        }
        PopFlags => {
            stack_read(accesses, 2);
            write(accesses, Location::Flags, AccessOccurrence::Always);
        }
        CallDirect16(_) | CallDirect24(_) => stack_write(accesses, 4),
        LoadCurrentImmediate {
            destination, size, ..
        } => write_current(accesses, at, destination, size, AccessOccurrence::Always),
        PushCurrent { source, size } => {
            read_current(accesses, at, source, size, AccessOccurrence::Always);
            stack_write(accesses, size.byte_len() as u8);
        }
        PopCurrent { destination, size } => {
            stack_read(accesses, size.byte_len() as u8);
            write_current(accesses, at, destination, size, AccessOccurrence::Always);
        }
        JumpRelative { condition, .. } | JumpRelativeLong { condition, .. } => {
            read_condition(accesses, condition)
        }
        LoadExtract { address, .. } => memory_write(
            accesses,
            OperandAddress::Direct8(address),
            1,
            AccessOccurrence::Always,
        ),
        SoftwareInterrupt(_) => {
            read(accesses, Location::Status, AccessOccurrence::Always);
            stack_write(accesses, 6);
            write(accesses, Location::Status, AccessOccurrence::Always);
        }
    }
}

fn register_accesses(
    operation: RegisterOperation,
    at: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    use RegisterInstruction::*;
    let target = register_location(at, operation.register, operation.size);
    match operation.instruction {
        LoadImmediate(_) | LoadSmallImmediate(_) | SetCondition(_) => {
            write(accesses, target, AccessOccurrence::Always)
        }
        Push => {
            read(accesses, target, AccessOccurrence::Always);
            stack_write(accesses, operation.size.byte_len() as u8);
        }
        Pop => {
            stack_read(accesses, operation.size.byte_len() as u8);
            write(accesses, target, AccessOccurrence::Always);
        }
        Complement
        | Negate
        | DecimalAdjust
        | ExtendZero
        | ExtendSign
        | PointerAdjust
        | Mirror
        | Increment(_)
        | Decrement(_)
        | ModuloIncrement { .. }
        | ModuloDecrement { .. } => {
            read_write(accesses, target);
            write(accesses, Location::Flags, AccessOccurrence::Always);
        }
        MultiplyImmediate { .. } | DivideImmediate { .. } => {
            read_write(accesses, target);
            write(accesses, Location::Flags, AccessOccurrence::Always);
        }
        Link(_) | Unlink => {
            read_write(accesses, target);
            read_write_implicit(accesses, ImplicitState::StackPointer);
        }
        BitSearchForward | BitSearchBackward => {
            read(accesses, target, AccessOccurrence::Always);
            write(accesses, Location::Flags, AccessOccurrence::Always);
        }
        MultiplyAccumulate => {
            read(accesses, target, AccessOccurrence::Always);
            read_write_implicit(accesses, ImplicitState::MultiplyAccumulator);
            write(accesses, Location::Flags, AccessOccurrence::Always);
        }
        DecrementJump(_) => {
            read_write(accesses, target);
            write(accesses, Location::Flags, AccessOccurrence::Always);
        }
        CarryImmediate { operation, .. } | BitImmediate { operation, .. } => {
            bit_operation(accesses, target, operation)
        }
        CarryAccumulator(operation) => {
            read(
                accesses,
                Location::Implicit(ImplicitState::Accumulator),
                AccessOccurrence::Always,
            );
            bit_operation(accesses, target, operation);
        }
        LoadControl { control_code } => {
            read(accesses, target, AccessOccurrence::Always);
            write(
                accesses,
                Location::ControlRegister(control_code),
                AccessOccurrence::Always,
            );
        }
        StoreControl { control_code } => {
            read(
                accesses,
                Location::ControlRegister(control_code),
                AccessOccurrence::Always,
            );
            write(accesses, target, AccessOccurrence::Always);
        }
        MultiplyCurrent { source, .. } | DivideCurrent { source, .. } => {
            read_write(accesses, target);
            read_current(
                accesses,
                at,
                source,
                operation.size,
                AccessOccurrence::Always,
            );
            write(accesses, Location::Flags, AccessOccurrence::Always);
        }
        ArithmeticCurrent {
            operation: arithmetic,
            other,
        } => {
            arithmetic_inputs(accesses, arithmetic);
            read_current(
                accesses,
                at,
                other,
                operation.size,
                AccessOccurrence::Always,
            );
            if arithmetic == ArithmeticOperation::Compare {
                read(accesses, target, AccessOccurrence::Always);
            } else {
                read_write(accesses, target);
            }
            write(accesses, Location::Flags, AccessOccurrence::Always);
        }
        LoadFromCurrent(source) => {
            read_current(
                accesses,
                at,
                source,
                operation.size,
                AccessOccurrence::Always,
            );
            write(accesses, target, AccessOccurrence::Always);
        }
        LoadToCurrent(destination) => {
            read(accesses, target, AccessOccurrence::Always);
            write_current(
                accesses,
                at,
                destination,
                operation.size,
                AccessOccurrence::Always,
            );
        }
        ExchangeCurrent(other) => {
            read_write(accesses, target);
            read_write(accesses, current_location(at, other, operation.size));
        }
        ArithmeticImmediate {
            operation: arithmetic,
            ..
        } => {
            arithmetic_inputs(accesses, arithmetic);
            if arithmetic == ArithmeticOperation::Compare {
                read(accesses, target, AccessOccurrence::Always);
            } else {
                read_write(accesses, target);
            }
            write(accesses, Location::Flags, AccessOccurrence::Always);
        }
        CompareSmallImmediate(_) => {
            read(accesses, target, AccessOccurrence::Always);
            write(accesses, Location::Flags, AccessOccurrence::Always);
        }
        ShiftImmediate {
            operation: shift, ..
        }
        | ShiftAccumulator(shift) => {
            shift_inputs(accesses, shift);
            read_write(accesses, target);
            write(accesses, Location::Flags, AccessOccurrence::Always);
        }
    }
}

fn source_memory_accesses(
    operation: SourceMemoryOperation,
    at: AnalysisPoint,
    next: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    use SourceMemoryInstruction::*;
    let width = operation.size.byte_len() as u8;
    address_inputs(accesses, at, operation.source);
    let memory = Location::Memory(MemoryLocation {
        address: EffectiveAddress::Operand(operation.source),
        byte_len: width,
    });
    match operation.instruction {
        Push => {
            read(accesses, memory, AccessOccurrence::Always);
            stack_write(accesses, width);
        }
        RotateLeftDigit | RotateRightDigit => {
            read_write(accesses, memory);
            read_write_implicit(accesses, ImplicitState::Accumulator);
            write(accesses, Location::Flags, AccessOccurrence::Always);
        }
        LoadIncrement
        | LoadIncrementRepeat
        | LoadDecrement
        | LoadDecrementRepeat
        | CompareIncrement
        | CompareIncrementRepeat
        | CompareDecrement
        | CompareDecrementRepeat => {
            read(accesses, memory, AccessOccurrence::Always);
            read_write_implicit(accesses, ImplicitState::StringSource);
            read_write_implicit(accesses, ImplicitState::StringDestination);
            read_write_implicit(accesses, ImplicitState::StringCounter);
            if matches!(
                operation.instruction,
                CompareIncrement
                    | CompareIncrementRepeat
                    | CompareDecrement
                    | CompareDecrementRepeat
            ) {
                write(accesses, Location::Flags, AccessOccurrence::Always);
            } else {
                write(
                    accesses,
                    Location::Memory(MemoryLocation {
                        address: EffectiveAddress::StringDestination,
                        byte_len: width,
                    }),
                    AccessOccurrence::Always,
                );
            }
        }
        LoadToDirect16(address) => {
            read(accesses, memory, AccessOccurrence::Always);
            memory_write(
                accesses,
                OperandAddress::Direct16(address),
                width,
                AccessOccurrence::Always,
            );
            add_memory_reference(
                references,
                u32::from(address),
                width,
                ReferenceKind::Write,
                AccessOccurrence::Always,
            );
        }
        LoadToCurrent(destination) => {
            read(accesses, memory, AccessOccurrence::Always);
            write_current(
                accesses,
                at,
                destination,
                operation.size,
                AccessOccurrence::Always,
            );
        }
        ExchangeCurrent(register) => {
            read_write(accesses, memory);
            read_write(accesses, current_location(at, register, operation.size));
        }
        ArithmeticImmediate {
            operation: arithmetic,
            ..
        } => {
            arithmetic_inputs(accesses, arithmetic);
            if arithmetic == ArithmeticOperation::Compare {
                read(accesses, memory, AccessOccurrence::Always);
            } else {
                read_write(accesses, memory);
            }
            write(accesses, Location::Flags, AccessOccurrence::Always);
        }
        MultiplyCurrent { destination, .. } | DivideCurrent { destination, .. } => {
            read(accesses, memory, AccessOccurrence::Always);
            read_write(accesses, current_location(at, destination, operation.size));
            write(accesses, Location::Flags, AccessOccurrence::Always);
        }
        Increment(_) | Decrement(_) => {
            read_write(accesses, memory);
            write(accesses, Location::Flags, AccessOccurrence::Always);
        }
        Shift(shift) => {
            shift_inputs(accesses, shift);
            read_write(accesses, memory);
            write(accesses, Location::Flags, AccessOccurrence::Always);
        }
        ArithmeticCurrent {
            operation: arithmetic,
            current,
            destination_is_memory,
        } => {
            arithmetic_inputs(accesses, arithmetic);
            read_current(
                accesses,
                at,
                current,
                operation.size,
                AccessOccurrence::Always,
            );
            if destination_is_memory && arithmetic != ArithmeticOperation::Compare {
                read_write(accesses, memory);
            } else {
                read(accesses, memory, AccessOccurrence::Always);
                if arithmetic != ArithmeticOperation::Compare {
                    write_current(
                        accesses,
                        at,
                        current,
                        operation.size,
                        AccessOccurrence::Always,
                    );
                }
            }
            write(accesses, Location::Flags, AccessOccurrence::Always);
        }
    }
    reference_for_memory(
        references,
        operation.source,
        next,
        width,
        ReferenceKind::Read,
        AccessOccurrence::Always,
    );
}

fn destination_memory_accesses(
    operation: DestinationMemoryOperation,
    at: AnalysisPoint,
    next: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    use DestinationMemoryInstruction::*;
    address_inputs(accesses, at, operation.destination);
    match operation.instruction {
        LoadImmediateByte(_) => {
            memory_write(accesses, operation.destination, 1, AccessOccurrence::Always)
        }
        LoadImmediateWord(_) => {
            memory_write(accesses, operation.destination, 2, AccessOccurrence::Always)
        }
        PopByte => {
            stack_read(accesses, 1);
            memory_write(accesses, operation.destination, 1, AccessOccurrence::Always);
        }
        PopWord => {
            stack_read(accesses, 2);
            memory_write(accesses, operation.destination, 2, AccessOccurrence::Always);
        }
        LoadFromDirect16Byte(source) => {
            memory_read(
                accesses,
                OperandAddress::Direct16(source),
                1,
                AccessOccurrence::Always,
            );
            memory_write(accesses, operation.destination, 1, AccessOccurrence::Always);
        }
        LoadFromDirect16Word(source) => {
            memory_read(
                accesses,
                OperandAddress::Direct16(source),
                2,
                AccessOccurrence::Always,
            );
            memory_write(accesses, operation.destination, 2, AccessOccurrence::Always);
        }
        LoadAddressWord(destination) | LoadAddressLong(destination) => write_current(
            accesses,
            at,
            destination,
            if matches!(operation.instruction, LoadAddressWord(_)) {
                OperandSize::Word
            } else {
                OperandSize::Long
            },
            AccessOccurrence::Always,
        ),
        CarryAccumulator(bit) | BitImmediate { operation: bit, .. } => bit_operation(
            accesses,
            Location::Memory(MemoryLocation {
                address: EffectiveAddress::Operand(operation.destination),
                byte_len: 1,
            }),
            bit,
        ),
        LoadFromCurrent { size, source } => {
            read_current(accesses, at, source, size, AccessOccurrence::Always);
            memory_write(
                accesses,
                operation.destination,
                size.byte_len() as u8,
                AccessOccurrence::Always,
            );
        }
        Jump(condition) | Call(condition) => {
            read_condition(accesses, condition);
            if matches!(operation.instruction, Call(_)) {
                stack_write(accesses, 4);
            }
        }
        Return(condition) => {
            read_condition(accesses, condition);
            stack_read_conditional(accesses, 4, condition_occurrence(condition));
        }
    }

    match operation.instruction {
        LoadImmediateByte(_) | PopByte | LoadFromDirect16Byte(_) => reference_for_memory(
            references,
            operation.destination,
            next,
            1,
            ReferenceKind::Write,
            AccessOccurrence::Always,
        ),
        LoadImmediateWord(_) | PopWord | LoadFromDirect16Word(_) => reference_for_memory(
            references,
            operation.destination,
            next,
            2,
            ReferenceKind::Write,
            AccessOccurrence::Always,
        ),
        LoadAddressWord(_) | LoadAddressLong(_) => {
            if let Some(address) = resolved_address(operation.destination, next) {
                add_reference(
                    references,
                    StaticReference {
                        target: Reference::Address(address),
                        kind: ReferenceKind::Address,
                        occurrence: AccessOccurrence::Always,
                    },
                );
            }
        }
        _ => {}
    }
}

fn register_location(at: AnalysisPoint, register: RegisterEncoding, size: OperandSize) -> Location {
    Location::Register(match register {
        RegisterEncoding::Current(register) => RegisterLocation::Current {
            bank: at.current_bank,
            register,
            size,
        },
        RegisterEncoding::Extended(register) => RegisterLocation::Extended { register, size },
    })
}

fn current_location(at: AnalysisPoint, register: CurrentRegister, size: OperandSize) -> Location {
    register_location(at, RegisterEncoding::Current(register), size)
}

fn read_current(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    register: CurrentRegister,
    size: OperandSize,
    occurrence: AccessOccurrence,
) {
    read(accesses, current_location(at, register, size), occurrence);
}

fn write_current(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    register: CurrentRegister,
    size: OperandSize,
    occurrence: AccessOccurrence,
) {
    write(accesses, current_location(at, register, size), occurrence);
}

fn address_inputs(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    address: OperandAddress,
) {
    match address {
        OperandAddress::CurrentIndirect(register)
        | OperandAddress::CurrentDisplacement { base: register, .. } => {
            read_current(
                accesses,
                at,
                register,
                OperandSize::Long,
                AccessOccurrence::Always,
            );
        }
        OperandAddress::Register(register)
        | OperandAddress::RegisterDisplacement { base: register, .. } => read(
            accesses,
            Location::Register(RegisterLocation::Extended {
                register,
                size: OperandSize::Long,
            }),
            AccessOccurrence::Always,
        ),
        OperandAddress::IndexedByte { base, index }
        | OperandAddress::IndexedWord { base, index } => {
            for register in [base, index] {
                read(
                    accesses,
                    Location::Register(RegisterLocation::Extended {
                        register,
                        size: OperandSize::Long,
                    }),
                    AccessOccurrence::Always,
                );
            }
        }
        OperandAddress::PreDecrement { register, .. }
        | OperandAddress::PostIncrement { register, .. } => read_write(
            accesses,
            Location::Register(RegisterLocation::Extended {
                register,
                size: OperandSize::Long,
            }),
        ),
        OperandAddress::Direct8(_)
        | OperandAddress::Direct16(_)
        | OperandAddress::Direct24(_)
        | OperandAddress::PcRelative(_) => {}
    }
}

fn arithmetic_inputs(accesses: &mut Vec<LocationAccess<Location>>, operation: ArithmeticOperation) {
    if matches!(
        operation,
        ArithmeticOperation::AddCarry | ArithmeticOperation::SubtractCarry
    ) {
        read(accesses, Location::CarryFlag, AccessOccurrence::Always);
    }
}

fn shift_inputs(accesses: &mut Vec<LocationAccess<Location>>, operation: ShiftOperation) {
    if matches!(
        operation,
        ShiftOperation::RotateLeftCarry | ShiftOperation::RotateRightCarry
    ) {
        read(accesses, Location::CarryFlag, AccessOccurrence::Always);
    }
}

fn bit_operation(
    accesses: &mut Vec<LocationAccess<Location>>,
    target: Location,
    operation: BitOperation,
) {
    match operation {
        BitOperation::AndCarry
        | BitOperation::OrCarry
        | BitOperation::XorCarry
        | BitOperation::LoadCarry => {
            read(accesses, target, AccessOccurrence::Always);
            if operation != BitOperation::LoadCarry {
                read(accesses, Location::CarryFlag, AccessOccurrence::Always);
            }
            write(accesses, Location::CarryFlag, AccessOccurrence::Always);
        }
        BitOperation::StoreCarry => {
            read(accesses, Location::CarryFlag, AccessOccurrence::Always);
            read_write(accesses, target);
        }
        BitOperation::Reset
        | BitOperation::Set
        | BitOperation::Change
        | BitOperation::TestAndSet => read_write(accesses, target),
        BitOperation::Test => {
            read(accesses, target, AccessOccurrence::Always);
            write(accesses, Location::Flags, AccessOccurrence::Always);
        }
    }
}

fn read_condition(accesses: &mut Vec<LocationAccess<Location>>, condition: Condition) {
    if !matches!(condition, Condition::False | Condition::True) {
        read(accesses, Location::Flags, AccessOccurrence::Always);
    }
}

fn condition_occurrence(condition: Condition) -> AccessOccurrence {
    if condition == Condition::True {
        AccessOccurrence::Always
    } else {
        AccessOccurrence::Conditional
    }
}

fn memory_read(
    accesses: &mut Vec<LocationAccess<Location>>,
    address: OperandAddress,
    byte_len: u8,
    occurrence: AccessOccurrence,
) {
    read(
        accesses,
        Location::Memory(MemoryLocation {
            address: EffectiveAddress::Operand(address),
            byte_len,
        }),
        occurrence,
    );
}

fn memory_write(
    accesses: &mut Vec<LocationAccess<Location>>,
    address: OperandAddress,
    byte_len: u8,
    occurrence: AccessOccurrence,
) {
    write(
        accesses,
        Location::Memory(MemoryLocation {
            address: EffectiveAddress::Operand(address),
            byte_len,
        }),
        occurrence,
    );
}

fn stack_read(accesses: &mut Vec<LocationAccess<Location>>, byte_len: u8) {
    stack_read_conditional(accesses, byte_len, AccessOccurrence::Always);
}

fn stack_read_conditional(
    accesses: &mut Vec<LocationAccess<Location>>,
    byte_len: u8,
    occurrence: AccessOccurrence,
) {
    read(
        accesses,
        Location::Implicit(ImplicitState::StackPointer),
        occurrence,
    );
    read(
        accesses,
        Location::Memory(MemoryLocation {
            address: EffectiveAddress::Stack,
            byte_len,
        }),
        occurrence,
    );
    write(
        accesses,
        Location::Implicit(ImplicitState::StackPointer),
        occurrence,
    );
}

fn stack_write(accesses: &mut Vec<LocationAccess<Location>>, byte_len: u8) {
    read_write_implicit(accesses, ImplicitState::StackPointer);
    write(
        accesses,
        Location::Memory(MemoryLocation {
            address: EffectiveAddress::Stack,
            byte_len,
        }),
        AccessOccurrence::Always,
    );
}

fn read_write_implicit(accesses: &mut Vec<LocationAccess<Location>>, state: ImplicitState) {
    read_write(accesses, Location::Implicit(state));
}

fn read_write(accesses: &mut Vec<LocationAccess<Location>>, location: Location) {
    read(accesses, location, AccessOccurrence::Always);
    write(accesses, location, AccessOccurrence::Always);
}

fn resolved_address(address: OperandAddress, next: AnalysisPoint) -> Option<u32> {
    match address {
        OperandAddress::Direct16(address) => Some(u32::from(address)),
        OperandAddress::Direct24(address) => Some(address & 0x00ff_ffff),
        OperandAddress::PcRelative(displacement) => {
            Some(next.location.wrapping_add_signed(i32::from(displacement)) & 0x00ff_ffff)
        }
        _ => None,
    }
}

fn reference_for_memory(
    references: &mut Vec<StaticReference<Reference>>,
    address: OperandAddress,
    next: AnalysisPoint,
    byte_len: u8,
    kind: ReferenceKind,
    occurrence: AccessOccurrence,
) {
    if let Some(address) = resolved_address(address, next) {
        add_memory_reference(references, address, byte_len, kind, occurrence);
    }
}

fn add_memory_reference(
    references: &mut Vec<StaticReference<Reference>>,
    address: u32,
    byte_len: u8,
    kind: ReferenceKind,
    occurrence: AccessOccurrence,
) {
    add_reference(
        references,
        StaticReference {
            target: Reference::Memory { address, byte_len },
            kind,
            occurrence,
        },
    );
}

fn read(
    accesses: &mut Vec<LocationAccess<Location>>,
    location: Location,
    occurrence: AccessOccurrence,
) {
    add_access(accesses, location, AccessKind::Read, occurrence);
}

fn write(
    accesses: &mut Vec<LocationAccess<Location>>,
    location: Location,
    occurrence: AccessOccurrence,
) {
    add_access(accesses, location, AccessKind::Write, occurrence);
}

fn add_access(
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
        (Location::Status, Location::Flags | Location::CarryFlag)
        | (Location::Flags | Location::CarryFlag, Location::Status)
        | (Location::Flags, Location::CarryFlag)
        | (Location::CarryFlag, Location::Flags) => Overlapping,
        (Location::Register(left), Location::Register(right)) => register_relation(*left, *right),
        (Location::Memory(left), Location::Memory(right)) => memory_relation(*left, *right),
        _ => Disjoint,
    }
}

fn register_relation(left: RegisterLocation, right: RegisterLocation) -> StorageRelation {
    match (left, right) {
        (
            RegisterLocation::Extended {
                register: left,
                size: left_size,
            },
            RegisterLocation::Extended {
                register: right,
                size: right_size,
            },
        ) => range_relation(
            u32::from(left.code()),
            left_size.byte_len() as u8,
            u32::from(right.code()),
            right_size.byte_len() as u8,
        ),
        (
            RegisterLocation::Current {
                bank: left_bank,
                register: left,
                size: left_size,
            },
            RegisterLocation::Current {
                bank: right_bank,
                register: right,
                size: right_size,
            },
        ) if left_bank == right_bank && left == right && left_size == right_size => {
            StorageRelation::Same
        }
        (RegisterLocation::Current { .. }, RegisterLocation::Current { .. })
        | (RegisterLocation::Current { .. }, RegisterLocation::Extended { .. })
        | (RegisterLocation::Extended { .. }, RegisterLocation::Current { .. }) => {
            StorageRelation::Unknown
        }
    }
}

fn memory_relation(left: MemoryLocation, right: MemoryLocation) -> StorageRelation {
    if left.address == right.address {
        return StorageRelation::Overlapping;
    }
    let (
        EffectiveAddress::Operand(left_address_mode),
        EffectiveAddress::Operand(right_address_mode),
    ) = (left.address, right.address)
    else {
        return StorageRelation::Unknown;
    };
    match (
        resolved_address(left_address_mode, AnalysisPoint::new(0)),
        resolved_address(right_address_mode, AnalysisPoint::new(0)),
    ) {
        (Some(left_address), Some(right_address))
            if !matches!(left_address_mode, OperandAddress::PcRelative(_))
                && !matches!(right_address_mode, OperandAddress::PcRelative(_)) =>
        {
            range_relation(left_address, left.byte_len, right_address, right.byte_len)
        }
        _ => StorageRelation::Unknown,
    }
}

fn range_relation(
    left_address: u32,
    left_len: u8,
    right_address: u32,
    right_len: u8,
) -> StorageRelation {
    let overlaps = (0..u32::from(left_len)).any(|left_offset| {
        (0..u32::from(right_len)).any(|right_offset| {
            left_address.wrapping_add(left_offset) == right_address.wrapping_add(right_offset)
        })
    });
    if overlaps {
        StorageRelation::Overlapping
    } else {
        StorageRelation::Disjoint
    }
}
