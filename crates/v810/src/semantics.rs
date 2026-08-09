//! Complete static meaning for the NEC V810 profile.

use core::fmt;

use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlBoundary, ControlTarget, LocationAccess,
    ReferenceKind, StaticReference, StorageRelation,
};

use crate::{
    BitStringOperation, CodeLocation, Condition, FloatingOperation, Instruction, MemoryOperation,
    Register, RegisterOperation, SystemRegister,
};

/// Runtime-derived V810 control destination described statically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndirectTarget {
    /// Destination read from a general register by JMP, with bit zero cleared.
    Register(Register),
    /// Destination restored by RETI according to the exception-level state.
    ExceptionReturn,
}

/// V810 synchronous boundary or wait kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Boundary {
    /// Encoded software trap.
    Trap(u8),
    /// HALT wait state.
    Halt,
}

/// Individually meaningful V810 program-status-word storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PswField {
    /// Zero flag Z.
    Zero,
    /// Sign flag S.
    Sign,
    /// Overflow flag OV.
    Overflow,
    /// Carry flag CY.
    Carry,
    /// Saturation flag SAT.
    Saturation,
    /// Floating reserved-operand flag FRO.
    FloatingReservedOperand,
    /// Floating invalid-operation flag FIV.
    FloatingInvalidOperation,
    /// Floating divide-by-zero flag FZD.
    FloatingDivideByZero,
    /// Floating overflow flag FOV.
    FloatingOverflow,
    /// Floating underflow flag FUD.
    FloatingUnderflow,
    /// Floating precision-degradation flag FPR.
    FloatingPrecision,
    /// Interrupt-disable flag ID.
    InterruptDisable,
    /// Address-trap enable flag AE.
    AddressTrapEnable,
    /// Exception-pending flag EP.
    ExceptionPending,
    /// Duplexed-exception-pending flag NP.
    DuplexedExceptionPending,
}

/// V810 memory or I/O address space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AddressSpace {
    /// Ordinary memory space.
    Memory,
    /// Distinct input/output space.
    Io,
}

/// Symbolic V810 data location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryLocation {
    /// Address space selected by the instruction.
    pub space: AddressSpace,
    /// Base general register.
    pub base: Register,
    /// Signed byte displacement.
    pub displacement: i16,
    /// Access width in bytes.
    pub byte_len: u8,
}

/// Implicit bit-string source or destination memory stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BitStringMemory {
    /// Stream addressed by R30 with bit offset R27.
    Source,
    /// Stream addressed by R29 with bit offset R26.
    Destination,
}

/// Architectural V810 storage location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Location {
    /// General-register storage. R0 may be read but is never reported written.
    Register(Register),
    /// Whole architecturally implemented system register.
    SystemRegister(SystemRegister),
    /// Individually meaningful view into PSW storage.
    Psw(PswField),
    /// Explicit memory or I/O transfer location.
    Memory(MemoryLocation),
    /// Implicit bit-string memory stream.
    BitStringMemory(BitStringMemory),
}

/// Statically resolved non-control V810 address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reference {
    /// Bytes read or written at an address known without register values.
    Memory {
        /// Address space selected by the instruction.
        space: AddressSpace,
        /// First referenced byte after the instruction's alignment mask.
        address: u32,
        /// Referenced width in bytes.
        byte_len: u8,
    },
    /// Address value formed without claiming a memory access or extent.
    Address {
        /// Address space containing the formed address.
        space: AddressSpace,
        /// Formed address value.
        address: u32,
    },
}

/// Structural failure to form sound V810 static facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticError {
    /// The instruction location is not halfword aligned.
    UnalignedLocation {
        /// Rejected instruction location.
        location: CodeLocation,
    },
}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnalignedLocation { location } => {
                write!(
                    f,
                    "V810 instruction location {location:#010x} is not halfword aligned"
                )
            }
        }
    }
}

impl std::error::Error for SemanticError {}

/// Shared control-flow result specialized with V810 vocabulary.
pub type ControlFlow = typed_isa_core::ControlFlow<CodeLocation, IndirectTarget, Boundary>;

/// Shared instruction-semantics result specialized with V810 vocabulary.
pub type InstructionSemantics = typed_isa_core::InstructionSemantics<
    CodeLocation,
    IndirectTarget,
    Boundary,
    Location,
    Reference,
>;

pub(crate) fn semantics(
    instruction: &Instruction,
    at: CodeLocation,
) -> Result<InstructionSemantics, SemanticError> {
    if at & 1 != 0 {
        return Err(SemanticError::UnalignedLocation { location: at });
    }
    let next = at.wrapping_add(instruction.encoded_len() as u32);
    let mut accesses = Vec::new();
    let mut references = Vec::new();
    add_instruction_accesses(instruction, &mut accesses, &mut references);
    Ok(InstructionSemantics {
        control_flow: control_flow(instruction, at, next),
        location_accesses: accesses,
        static_references: references,
    })
}

fn control_flow(instruction: &Instruction, at: u32, next: u32) -> ControlFlow {
    match *instruction {
        Instruction::JumpRegister(register) => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Indirect(IndirectTarget::Register(register)),
            },
            fallthrough: None,
            delay_slot: None,
        },
        Instruction::Branch {
            condition,
            displacement,
        } => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Direct(at.wrapping_add_signed(i32::from(displacement))),
            },
            fallthrough: (condition != Condition::Always).then_some(next),
            delay_slot: None,
        },
        Instruction::JumpRelative(displacement) => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Direct(at.wrapping_add_signed(displacement)),
            },
            fallthrough: None,
            delay_slot: None,
        },
        Instruction::JumpAndLink(displacement) => ControlFlow {
            action: ControlAction::LinkedTransfer {
                target: ControlTarget::Direct(at.wrapping_add_signed(displacement)),
                return_site: next,
            },
            fallthrough: None,
            delay_slot: None,
        },
        Instruction::Trap(vector) => ControlFlow {
            action: ControlAction::Boundary(ControlBoundary::Trap {
                kind: Boundary::Trap(vector),
                target: Some(ControlTarget::Direct(
                    0xffff_ffa0_u32.wrapping_add(u32::from(vector & 0x10)),
                )),
                resume: Some(next),
            }),
            fallthrough: None,
            delay_slot: None,
        },
        Instruction::ReturnFromInterrupt => ControlFlow {
            action: ControlAction::ExceptionReturn {
                target: IndirectTarget::ExceptionReturn,
            },
            fallthrough: None,
            delay_slot: None,
        },
        Instruction::Halt => ControlFlow {
            action: ControlAction::Boundary(ControlBoundary::Wait {
                kind: Boundary::Halt,
                resume: next,
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

fn add_instruction_accesses(
    instruction: &Instruction,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    match *instruction {
        Instruction::Register {
            operation,
            source,
            destination,
        } => add_register_operation(operation, source, destination, accesses),
        Instruction::JumpRegister(register) => read_register(accesses, register),
        Instruction::Immediate {
            operation,
            destination,
            ..
        } => {
            if operation != crate::ImmediateOperation::Move {
                read_register(accesses, destination);
            }
            if operation != crate::ImmediateOperation::Compare {
                write_register(accesses, destination);
            }
            if operation != crate::ImmediateOperation::Move {
                write_integer_flags(accesses);
            }
        }
        Instruction::ShiftImmediate { destination, .. } => {
            read_register(accesses, destination);
            write_register(accesses, destination);
            write_integer_flags(accesses);
        }
        Instruction::SetFlag {
            condition,
            destination,
        } => {
            read_condition(accesses, condition);
            write_register(accesses, destination);
        }
        Instruction::Trap(_) => add_trap_accesses(accesses),
        Instruction::ReturnFromInterrupt => {
            for register in [
                SystemRegister::Eipc,
                SystemRegister::Eipsw,
                SystemRegister::Fepc,
                SystemRegister::Fepsw,
            ] {
                access(
                    accesses,
                    Location::SystemRegister(register),
                    AccessKind::Read,
                    AccessOccurrence::Conditional,
                );
            }
            read_psw(accesses, PswField::DuplexedExceptionPending);
            write_system(accesses, SystemRegister::Psw);
        }
        Instruction::Halt => read_psw(accesses, PswField::InterruptDisable),
        Instruction::LoadSystemRegister {
            source,
            destination,
        } => {
            read_register(accesses, source);
            write_system(accesses, destination);
        }
        Instruction::StoreSystemRegister {
            source,
            destination,
        } => {
            read_system(accesses, source);
            write_register(accesses, destination);
        }
        Instruction::BitString(operation) => add_bit_string_accesses(operation, accesses),
        Instruction::Nop | Instruction::JumpRelative(_) => {}
        Instruction::JumpAndLink(_) => {
            write_register(
                accesses,
                Register::new(31).expect("R31 is the V810 link register"),
            );
        }
        Instruction::Branch { condition, .. } => read_condition(accesses, condition),
        Instruction::AddImmediate {
            source,
            destination,
            ..
        } => {
            read_register(accesses, source);
            write_register(accesses, destination);
            write_integer_flags(accesses);
        }
        Instruction::MoveAddress {
            source,
            destination,
            immediate,
        } => {
            read_register(accesses, source);
            write_register(accesses, destination);
            if source == Register::ZERO {
                references.push(StaticReference {
                    target: Reference::Address {
                        space: AddressSpace::Memory,
                        address: immediate as i32 as u32,
                    },
                    kind: ReferenceKind::Address,
                    occurrence: AccessOccurrence::Always,
                });
            }
        }
        Instruction::LogicalImmediate {
            operation,
            source,
            destination,
            ..
        } => {
            read_register(accesses, source);
            write_register(accesses, destination);
            if operation != crate::LogicalImmediateOperation::MoveHigh {
                write_logical_flags(accesses);
            }
        }
        Instruction::Memory {
            operation,
            displacement,
            base,
            register,
        } => add_memory_accesses(
            operation,
            displacement,
            base,
            register,
            accesses,
            references,
        ),
        Instruction::Floating {
            operation,
            source,
            destination,
        } => add_floating_accesses(operation, source, destination, accesses),
    }
}

fn add_register_operation(
    operation: RegisterOperation,
    source: Register,
    destination: Register,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_register(accesses, source);
    if !matches!(operation, RegisterOperation::Move | RegisterOperation::Not) {
        read_register(accesses, destination);
    }
    if operation != RegisterOperation::Compare {
        write_register(accesses, destination);
    }
    match operation {
        RegisterOperation::Move => {}
        RegisterOperation::Multiply
        | RegisterOperation::MultiplyUnsigned
        | RegisterOperation::Divide
        | RegisterOperation::DivideUnsigned => {
            write_register(accesses, Register::new(30).expect("R30 is valid"));
            write_logical_flags(accesses);
        }
        RegisterOperation::Or
        | RegisterOperation::And
        | RegisterOperation::Xor
        | RegisterOperation::Not => write_logical_flags(accesses),
        _ => write_integer_flags(accesses),
    }
}

fn add_memory_accesses(
    operation: MemoryOperation,
    displacement: i16,
    base: Register,
    register: Register,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    use MemoryOperation::*;
    read_register(accesses, base);
    let (space, byte_len, kind) = match operation {
        LoadByte => (AddressSpace::Memory, 1, AccessKind::Read),
        LoadHalfword => (AddressSpace::Memory, 2, AccessKind::Read),
        LoadWord | CompareExchange => (AddressSpace::Memory, 4, AccessKind::Read),
        StoreByte => (AddressSpace::Memory, 1, AccessKind::Write),
        StoreHalfword => (AddressSpace::Memory, 2, AccessKind::Write),
        StoreWord => (AddressSpace::Memory, 4, AccessKind::Write),
        InputByte => (AddressSpace::Io, 1, AccessKind::Read),
        InputHalfword => (AddressSpace::Io, 2, AccessKind::Read),
        InputWord => (AddressSpace::Io, 4, AccessKind::Read),
        OutputByte => (AddressSpace::Io, 1, AccessKind::Write),
        OutputHalfword => (AddressSpace::Io, 2, AccessKind::Write),
        OutputWord => (AddressSpace::Io, 4, AccessKind::Write),
    };
    let memory = Location::Memory(MemoryLocation {
        space,
        base,
        displacement,
        byte_len,
    });
    access(accesses, memory, kind, AccessOccurrence::Always);
    match operation {
        LoadByte | LoadHalfword | LoadWord | InputByte | InputHalfword | InputWord => {
            write_register(accesses, register)
        }
        StoreByte | StoreHalfword | StoreWord | OutputByte | OutputHalfword | OutputWord => {
            read_register(accesses, register)
        }
        CompareExchange => {
            read_register(accesses, register);
            access(
                accesses,
                Location::Register(Register::new(30).expect("R30 is valid")),
                AccessKind::Read,
                AccessOccurrence::Conditional,
            );
            write_register(accesses, register);
            access(
                accesses,
                memory,
                AccessKind::Write,
                AccessOccurrence::Always,
            );
            write_integer_flags(accesses);
        }
    }
    if base == Register::ZERO {
        let target = Reference::Memory {
            space,
            address: aligned_address(displacement, byte_len),
            byte_len,
        };
        references.push(StaticReference {
            target,
            kind: match kind {
                AccessKind::Read => ReferenceKind::Read,
                AccessKind::Write => ReferenceKind::Write,
            },
            occurrence: AccessOccurrence::Always,
        });
        if operation == CompareExchange {
            references.push(StaticReference {
                target,
                kind: ReferenceKind::Write,
                occurrence: AccessOccurrence::Always,
            });
        }
    }
}

fn aligned_address(displacement: i16, byte_len: u8) -> u32 {
    (displacement as i32 as u32) & !u32::from(byte_len - 1)
}

fn add_bit_string_accesses(
    operation: BitStringOperation,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    let first_register = if matches!(
        operation,
        BitStringOperation::SearchZeroUp
            | BitStringOperation::SearchZeroDown
            | BitStringOperation::SearchOneUp
            | BitStringOperation::SearchOneDown
    ) {
        27
    } else {
        26
    };
    for index in first_register..=30 {
        let register = Register::new(index).expect("implicit bit-string register is valid");
        read_register(accesses, register);
        write_register(accesses, register);
    }
    access(
        accesses,
        Location::BitStringMemory(BitStringMemory::Source),
        AccessKind::Read,
        AccessOccurrence::Always,
    );
    if !matches!(
        operation,
        BitStringOperation::SearchZeroUp
            | BitStringOperation::SearchZeroDown
            | BitStringOperation::SearchOneUp
            | BitStringOperation::SearchOneDown
    ) {
        access(
            accesses,
            Location::BitStringMemory(BitStringMemory::Destination),
            AccessKind::Read,
            AccessOccurrence::Always,
        );
        access(
            accesses,
            Location::BitStringMemory(BitStringMemory::Destination),
            AccessKind::Write,
            AccessOccurrence::Always,
        );
    }
    if matches!(
        operation,
        BitStringOperation::SearchZeroUp
            | BitStringOperation::SearchZeroDown
            | BitStringOperation::SearchOneUp
            | BitStringOperation::SearchOneDown
    ) {
        write_psw(accesses, PswField::Zero);
    }
}

fn add_floating_accesses(
    operation: FloatingOperation,
    source: Register,
    destination: Register,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_register(accesses, source);
    if matches!(
        operation,
        FloatingOperation::Compare
            | FloatingOperation::Add
            | FloatingOperation::Subtract
            | FloatingOperation::Multiply
            | FloatingOperation::Divide
    ) {
        read_register(accesses, destination);
    }
    if operation != FloatingOperation::Compare {
        write_register(accesses, destination);
    }
    let integer_fields: &[PswField] = match operation {
        FloatingOperation::ConvertFloatToWord | FloatingOperation::TruncateFloatToWord => {
            &[PswField::Overflow, PswField::Sign, PswField::Zero]
        }
        _ => &[
            PswField::Carry,
            PswField::Overflow,
            PswField::Sign,
            PswField::Zero,
        ],
    };
    for field in integer_fields {
        write_psw(accesses, *field);
    }
    let conditional_fields: &[PswField] = match operation {
        FloatingOperation::Compare => &[PswField::FloatingReservedOperand],
        FloatingOperation::ConvertWordToFloat => &[PswField::FloatingPrecision],
        FloatingOperation::ConvertFloatToWord | FloatingOperation::TruncateFloatToWord => &[
            PswField::FloatingReservedOperand,
            PswField::FloatingInvalidOperation,
            PswField::FloatingPrecision,
        ],
        FloatingOperation::Add | FloatingOperation::Subtract | FloatingOperation::Multiply => &[
            PswField::FloatingReservedOperand,
            PswField::FloatingOverflow,
            PswField::FloatingUnderflow,
            PswField::FloatingPrecision,
        ],
        FloatingOperation::Divide => &[
            PswField::FloatingReservedOperand,
            PswField::FloatingInvalidOperation,
            PswField::FloatingDivideByZero,
            PswField::FloatingOverflow,
            PswField::FloatingUnderflow,
            PswField::FloatingPrecision,
        ],
    };
    for field in conditional_fields {
        access(
            accesses,
            Location::Psw(*field),
            AccessKind::Write,
            AccessOccurrence::Conditional,
        );
    }
}

fn add_trap_accesses(accesses: &mut Vec<LocationAccess<Location>>) {
    read_psw(accesses, PswField::ExceptionPending);
    for register in [
        SystemRegister::Eipc,
        SystemRegister::Eipsw,
        SystemRegister::Fepc,
        SystemRegister::Fepsw,
    ] {
        access(
            accesses,
            Location::SystemRegister(register),
            AccessKind::Write,
            AccessOccurrence::Conditional,
        );
    }
    write_system(accesses, SystemRegister::Ecr);
    for field in [PswField::InterruptDisable, PswField::AddressTrapEnable] {
        write_psw(accesses, field);
    }
    for field in [
        PswField::ExceptionPending,
        PswField::DuplexedExceptionPending,
    ] {
        access(
            accesses,
            Location::Psw(field),
            AccessKind::Write,
            AccessOccurrence::Conditional,
        );
    }
}

fn read_condition(accesses: &mut Vec<LocationAccess<Location>>, condition: Condition) {
    use Condition::*;
    let fields: &[PswField] = match condition {
        Always | Never => &[],
        Overflow | NoOverflow => &[PswField::Overflow],
        CarryOrLower | NoCarryOrNotLower => &[PswField::Carry],
        ZeroOrEqual | NotZeroOrNotEqual => &[PswField::Zero],
        NotHigher | Higher => &[PswField::Carry, PswField::Zero],
        Negative | Positive => &[PswField::Sign],
        LessThan | GreaterOrEqual => &[PswField::Sign, PswField::Overflow],
        LessOrEqual | GreaterThan => &[PswField::Zero, PswField::Sign, PswField::Overflow],
    };
    for field in fields {
        read_psw(accesses, *field);
    }
}

fn write_integer_flags(accesses: &mut Vec<LocationAccess<Location>>) {
    for field in [
        PswField::Zero,
        PswField::Sign,
        PswField::Overflow,
        PswField::Carry,
    ] {
        write_psw(accesses, field);
    }
}

fn write_logical_flags(accesses: &mut Vec<LocationAccess<Location>>) {
    for field in [PswField::Zero, PswField::Sign, PswField::Overflow] {
        write_psw(accesses, field);
    }
}

fn read_register(accesses: &mut Vec<LocationAccess<Location>>, register: Register) {
    access(
        accesses,
        Location::Register(register),
        AccessKind::Read,
        AccessOccurrence::Always,
    );
}

fn write_register(accesses: &mut Vec<LocationAccess<Location>>, register: Register) {
    if register != Register::ZERO {
        access(
            accesses,
            Location::Register(register),
            AccessKind::Write,
            AccessOccurrence::Always,
        );
    }
}

fn read_system(accesses: &mut Vec<LocationAccess<Location>>, register: SystemRegister) {
    access(
        accesses,
        Location::SystemRegister(register),
        AccessKind::Read,
        AccessOccurrence::Always,
    );
}

fn write_system(accesses: &mut Vec<LocationAccess<Location>>, register: SystemRegister) {
    access(
        accesses,
        Location::SystemRegister(register),
        AccessKind::Write,
        AccessOccurrence::Always,
    );
}

fn read_psw(accesses: &mut Vec<LocationAccess<Location>>, field: PswField) {
    access(
        accesses,
        Location::Psw(field),
        AccessKind::Read,
        AccessOccurrence::Always,
    );
}

fn write_psw(accesses: &mut Vec<LocationAccess<Location>>, field: PswField) {
    access(
        accesses,
        Location::Psw(field),
        AccessKind::Write,
        AccessOccurrence::Always,
    );
}

fn access(
    accesses: &mut Vec<LocationAccess<Location>>,
    location: Location,
    kind: AccessKind,
    occurrence: AccessOccurrence,
) {
    accesses.push(LocationAccess {
        location,
        kind,
        occurrence,
    });
}

pub(crate) fn storage_relation(left: &Location, right: &Location) -> StorageRelation {
    if left == right {
        return StorageRelation::Same;
    }
    match (left, right) {
        (Location::SystemRegister(SystemRegister::Psw), Location::Psw(_))
        | (Location::Psw(_), Location::SystemRegister(SystemRegister::Psw)) => {
            StorageRelation::Overlapping
        }
        (Location::Memory(left), Location::Memory(right)) => memory_relation(*left, *right),
        (Location::Memory(_), Location::BitStringMemory(_))
        | (Location::BitStringMemory(_), Location::Memory(_))
        | (Location::BitStringMemory(_), Location::BitStringMemory(_)) => StorageRelation::Unknown,
        (Location::Memory(_) | Location::BitStringMemory(_), _)
        | (_, Location::Memory(_) | Location::BitStringMemory(_)) => StorageRelation::Disjoint,
        _ => StorageRelation::Disjoint,
    }
}

fn memory_relation(left: MemoryLocation, right: MemoryLocation) -> StorageRelation {
    if left.space != right.space {
        return StorageRelation::Disjoint;
    }
    if left.base == Register::ZERO && right.base == Register::ZERO {
        range_relation(
            left.displacement as i32 as u32,
            left.byte_len,
            right.displacement as i32 as u32,
            right.byte_len,
        )
    } else {
        StorageRelation::Unknown
    }
}

fn range_relation(left: u32, left_len: u8, right: u32, right_len: u8) -> StorageRelation {
    if left == right && left_len == right_len {
        StorageRelation::Same
    } else if (0..u32::from(left_len)).any(|left_offset| {
        (0..u32::from(right_len))
            .any(|right_offset| left.wrapping_add(left_offset) == right.wrapping_add(right_offset))
    }) {
        StorageRelation::Overlapping
    } else {
        StorageRelation::Disjoint
    }
}
