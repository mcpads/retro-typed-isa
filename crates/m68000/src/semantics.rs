//! Complete static meaning for the Motorola MC68000 profile.

use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlBoundary, ControlTarget, LocationAccess,
    ReferenceKind, StaticReference, StorageRelation,
};

use crate::{
    AddressRegister, AddressSize, BitNumber, BitOperation, BranchDisplacement, BranchKind,
    CodeLocation, Condition, DataDirection, DataRegister, EffectiveAddress, EncodeError,
    ExchangeRegisters, ExtendSize, IndexRegister, Instruction, MoveMultipleDirection,
    MovePeripheralDirection, OperandSize, Register, RegisterMask, RegisterPair, ShiftCount,
    ShiftOperation, UserStackPointerDirection,
};

/// Privilege state that selects the physical A7 stack pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrivilegeMode {
    /// User mode selects USP as A7.
    User,
    /// Supervisor mode selects SSP as A7.
    Supervisor,
    /// SR.S is not statically known.
    Unknown,
}

/// Placed MC68000 instruction together with the A7-selecting privilege state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnalysisPoint {
    /// Even logical program-counter address.
    pub location: CodeLocation,
    /// Known privilege mode, or `Unknown` after an unresolved SR restore.
    pub privilege: PrivilegeMode,
}

impl AnalysisPoint {
    /// Constructs an MC68000 static-analysis point.
    pub const fn new(location: CodeLocation, privilege: PrivilegeMode) -> Self {
        Self {
            location,
            privilege,
        }
    }
}

/// Runtime-derived MC68000 control destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndirectTarget {
    /// Destination computed by an effective-address operand.
    EffectiveAddress(EffectiveAddress),
    /// Long return address popped from the active stack.
    StackReturn,
    /// Status and program counter restored from an exception frame.
    ExceptionReturn,
    /// Handler address loaded through one vector-table entry.
    ExceptionVector {
        /// Vector number.
        vector: u8,
        /// Address of the four-byte vector-table entry.
        table_address: u32,
    },
}

/// MC68000 synchronous boundary or wait reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Boundary {
    /// Illegal-instruction exception, vector 4.
    IllegalInstruction,
    /// Divide-by-zero exception, vector 5.
    DivideByZero,
    /// CHK exception, vector 6.
    Check,
    /// TRAPV exception, vector 7.
    TrapOnOverflow,
    /// Encoded TRAP number and its vector 32-47.
    Trap(u8),
    /// STOP wait state.
    Stop,
}

/// One MC68000 condition-code bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConditionCode {
    /// Extend bit X.
    Extend,
    /// Negative bit N.
    Negative,
    /// Zero bit Z.
    Zero,
    /// Overflow bit V.
    Overflow,
    /// Carry bit C.
    Carry,
}

/// Physical or unresolved storage selected by an address-register name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AddressRegisterLocation {
    /// Fixed A0-A6 register storage.
    Register(AddressRegister),
    /// User stack pointer selected by A7 in user mode.
    UserStackPointer,
    /// Supervisor stack pointer selected by A7 in supervisor mode.
    SupervisorStackPointer,
    /// A7 when SR.S is unavailable.
    ActiveStackPointerUnknown,
}

/// Symbolic address of an MC68000 memory access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryAddress {
    /// Address held in one address register.
    AddressRegister(AddressRegisterLocation),
    /// Address before postincrementing its register.
    Postincrement(AddressRegisterLocation),
    /// Address after predecrementing its register.
    Predecrement(AddressRegisterLocation),
    /// Address register plus a signed displacement.
    Displacement {
        /// Base register storage.
        base: AddressRegisterLocation,
        /// Signed byte displacement.
        displacement: i16,
    },
    /// Address register plus a brief index and displacement.
    Indexed {
        /// Base register storage.
        base: AddressRegisterLocation,
        /// Index-register selector.
        index: IndexRegister,
        /// Signed byte displacement.
        displacement: i8,
    },
    /// Statically resolved absolute address.
    Absolute(u32),
    /// Statically resolved PC-relative address.
    ProgramCounterRelative(u32),
    /// PC-relative address with a runtime index component.
    ProgramCounterIndexed {
        /// Address of the brief extension word used as the PC base.
        base: u32,
        /// Index-register selector.
        index: IndexRegister,
        /// Signed byte displacement.
        displacement: i8,
    },
    /// Exception or return frame at the selected stack pointer.
    StackFrame(AddressRegisterLocation),
}

/// One contiguous or strided MC68000 memory access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryLocation {
    /// Symbolic starting address.
    pub address: MemoryAddress,
    /// Bytes in one transferred element.
    pub element_byte_len: u8,
    /// Number of transferred elements.
    pub element_count: u8,
    /// Byte distance between successive element starts.
    pub stride: u8,
}

impl MemoryLocation {
    fn contiguous(address: MemoryAddress, byte_len: u8) -> Self {
        Self {
            address,
            element_byte_len: byte_len,
            element_count: 1,
            stride: byte_len,
        }
    }
}

/// Architectural MC68000 storage location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Location {
    /// Low one, two, or four bytes of a data register.
    DataRegister {
        /// Selected data register.
        register: DataRegister,
        /// Accessed low-byte width.
        byte_len: u8,
    },
    /// Address-register or physical stack-pointer storage.
    AddressRegister(AddressRegisterLocation),
    /// Entire condition-code register.
    ConditionCodeRegister,
    /// One condition-code bit.
    ConditionCode(ConditionCode),
    /// Entire status register.
    StatusRegister,
    /// Symbolic memory storage.
    Memory(MemoryLocation),
    /// External reset control asserted by RESET.
    ExternalReset,
}

/// Statically resolved non-control MC68000 reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reference {
    /// Contiguous memory range.
    Memory {
        /// First byte address.
        address: u32,
        /// Number of bytes.
        byte_len: u8,
    },
    /// Non-contiguous MOVEP memory bytes.
    StridedMemory {
        /// First byte address.
        address: u32,
        /// Number of one-byte elements.
        element_count: u8,
        /// Byte stride between elements.
        stride: u8,
    },
    /// Address produced without claiming a memory access.
    Address(u32),
}

/// Failure to form sound MC68000 static facts.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SemanticError {
    /// Instruction location is not word aligned.
    #[error("MC68000 instruction location 0x{location:08X} is not word aligned")]
    UnalignedLocation {
        /// Rejected logical address.
        location: u32,
    },
    /// The typed instruction is not encodable in the MC68000 profile.
    #[error(transparent)]
    Encode(#[from] EncodeError),
}

/// Shared control-flow result specialized with MC68000 vocabulary.
pub type ControlFlow = typed_isa_core::ControlFlow<AnalysisPoint, IndirectTarget, Boundary>;

/// Shared instruction-semantics result specialized with MC68000 vocabulary.
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
    let address = at.location.logical_address();
    if address & 1 != 0 {
        return Err(SemanticError::UnalignedLocation { location: address });
    }
    let byte_len = u32::try_from(crate::encode_bytes(instruction)?.len())
        .expect("an MC68000 instruction length fits in u32");
    let next = successor(instruction, at, address.wrapping_add(byte_len));
    let mut accesses = Vec::new();
    let mut references = Vec::new();
    instruction_accesses(instruction, at, &mut accesses, &mut references);
    Ok(InstructionSemantics {
        control_flow: control_flow(instruction, at, next),
        location_accesses: accesses,
        static_references: references,
    })
}

fn successor(instruction: &Instruction, mut at: AnalysisPoint, address: u32) -> AnalysisPoint {
    at.location = CodeLocation::new(address);
    at.privilege = match *instruction {
        Instruction::AndImmediateToStatus(mask) if mask & 0x2000 == 0 => PrivilegeMode::User,
        Instruction::OrImmediateToStatus(mask) if mask & 0x2000 != 0 => PrivilegeMode::Supervisor,
        Instruction::ExclusiveOrImmediateToStatus(mask) if mask & 0x2000 != 0 => {
            match at.privilege {
                PrivilegeMode::User => PrivilegeMode::Supervisor,
                PrivilegeMode::Supervisor => PrivilegeMode::User,
                PrivilegeMode::Unknown => PrivilegeMode::Unknown,
            }
        }
        Instruction::MoveToStatus { .. } | Instruction::ReturnFromException => {
            PrivilegeMode::Unknown
        }
        Instruction::Stop { status } => {
            if status & 0x2000 == 0 {
                PrivilegeMode::User
            } else {
                PrivilegeMode::Supervisor
            }
        }
        _ => at.privilege,
    };
    at
}

fn direct_point(at: AnalysisPoint, address: u32) -> AnalysisPoint {
    AnalysisPoint {
        location: CodeLocation::new(address),
        ..at
    }
}

fn trap_flow(kind: Boundary, vector: u8, next: AnalysisPoint, conditional: bool) -> ControlFlow {
    ControlFlow {
        action: ControlAction::Boundary(ControlBoundary::Trap {
            kind,
            target: Some(ControlTarget::Indirect(IndirectTarget::ExceptionVector {
                vector,
                table_address: u32::from(vector) * 4,
            })),
            resume: Some(next),
        }),
        fallthrough: conditional.then_some(next),
        delay_slot: None,
    }
}

fn control_flow(instruction: &Instruction, at: AnalysisPoint, next: AnalysisPoint) -> ControlFlow {
    let address = at.location.logical_address();
    let branch_base = address.wrapping_add(2);
    match *instruction {
        Instruction::Branch { kind, displacement } => {
            let displacement = match displacement {
                BranchDisplacement::Byte(value) => i32::from(value),
                BranchDisplacement::Word(value) => i32::from(value),
            };
            let target = direct_point(at, branch_base.wrapping_add_signed(displacement));
            let action = match kind {
                BranchKind::Subroutine => ControlAction::LinkedTransfer {
                    target: ControlTarget::Direct(target),
                    return_site: next,
                },
                BranchKind::Always | BranchKind::Conditional(_) => ControlAction::Transfer {
                    target: ControlTarget::Direct(target),
                },
            };
            ControlFlow {
                action,
                fallthrough: matches!(kind, BranchKind::Conditional(_)).then_some(next),
                delay_slot: None,
            }
        }
        Instruction::DecrementBranch { displacement, .. } => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Direct(direct_point(
                    at,
                    branch_base.wrapping_add_signed(i32::from(displacement)),
                )),
            },
            fallthrough: Some(next),
            delay_slot: None,
        },
        Instruction::Jump { target } | Instruction::JumpSubroutine { target } => {
            let target = control_target(target, at, branch_base);
            let action = if matches!(instruction, Instruction::JumpSubroutine { .. }) {
                ControlAction::LinkedTransfer {
                    target,
                    return_site: next,
                }
            } else {
                ControlAction::Transfer { target }
            };
            ControlFlow {
                action,
                fallthrough: None,
                delay_slot: None,
            }
        }
        Instruction::ReturnFromException => ControlFlow {
            action: ControlAction::ExceptionReturn {
                target: IndirectTarget::ExceptionReturn,
            },
            fallthrough: None,
            delay_slot: None,
        },
        Instruction::ReturnAndRestore | Instruction::ReturnFromSubroutine => ControlFlow {
            action: ControlAction::Return {
                target: IndirectTarget::StackReturn,
            },
            fallthrough: None,
            delay_slot: None,
        },
        Instruction::Illegal => trap_flow(Boundary::IllegalInstruction, 4, next, false),
        Instruction::Divide { .. } => trap_flow(Boundary::DivideByZero, 5, next, true),
        Instruction::Check { .. } => trap_flow(Boundary::Check, 6, next, true),
        Instruction::TrapOnOverflow => trap_flow(Boundary::TrapOnOverflow, 7, next, true),
        Instruction::Trap { vector } => trap_flow(Boundary::Trap(vector), 32 + vector, next, false),
        Instruction::Stop { .. } => ControlFlow {
            action: ControlAction::Boundary(ControlBoundary::Wait {
                kind: Boundary::Stop,
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

fn control_target(
    target: EffectiveAddress,
    at: AnalysisPoint,
    pc_base: u32,
) -> ControlTarget<AnalysisPoint, IndirectTarget> {
    let direct = match target {
        EffectiveAddress::AbsoluteWord(address) => Some(i32::from(address as i16) as u32),
        EffectiveAddress::AbsoluteLong(address) => Some(address),
        EffectiveAddress::ProgramCounterDisplacement(displacement) => {
            Some(pc_base.wrapping_add_signed(i32::from(displacement)))
        }
        _ => None,
    };
    direct.map_or_else(
        || ControlTarget::Indirect(IndirectTarget::EffectiveAddress(target)),
        |address| ControlTarget::Direct(direct_point(at, address)),
    )
}

fn instruction_accesses(
    instruction: &Instruction,
    at: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    use Instruction::*;
    let pc_base = pc_extension_base(instruction, at);
    match *instruction {
        AddDecimal { operands } | SubtractDecimal { operands } => {
            register_pair_read_write(operands, at, 1, accesses, references);
            read_flag(accesses, ConditionCode::Extend);
            write_arithmetic_flags(accesses, true);
        }
        Add {
            size,
            direction,
            data_register,
            effective_address,
        }
        | Subtract {
            size,
            direction,
            data_register,
            effective_address,
        } => {
            binary_data_operation(
                direction,
                data_register,
                effective_address,
                size,
                at,
                pc_base,
                (accesses, references),
            );
            write_arithmetic_flags(accesses, true);
        }
        And {
            size,
            direction,
            data_register,
            effective_address,
        }
        | Or {
            size,
            direction,
            data_register,
            effective_address,
        } => {
            binary_data_operation(
                direction,
                data_register,
                effective_address,
                size,
                at,
                pc_base,
                (accesses, references),
            );
            write_logic_flags(accesses);
        }
        AddAddress {
            size,
            source,
            destination,
        }
        | SubtractAddress {
            size,
            source,
            destination,
        }
        | CompareAddress {
            size,
            source,
            destination,
        } => {
            read_ea(source, size.into(), at, pc_base, accesses, references);
            read_address(accesses, at, destination, AccessOccurrence::Always);
            if matches!(instruction, CompareAddress { .. }) {
                write_arithmetic_flags(accesses, false);
            } else {
                write_address(accesses, at, destination, AccessOccurrence::Always);
            }
        }
        AddImmediate {
            size, destination, ..
        }
        | SubtractImmediate {
            size, destination, ..
        } => {
            read_write_ea(destination, size, at, pc_base, accesses, references);
            write_arithmetic_flags(accesses, true);
        }
        AndImmediate {
            size, destination, ..
        }
        | OrImmediate {
            size, destination, ..
        }
        | ExclusiveOrImmediate {
            size, destination, ..
        } => {
            read_write_ea(destination, size, at, pc_base, accesses, references);
            write_logic_flags(accesses);
        }
        CompareImmediate {
            size, destination, ..
        } => {
            read_ea(destination, size, at, pc_base, accesses, references);
            write_arithmetic_flags(accesses, false);
        }
        AddQuick {
            size, destination, ..
        }
        | SubtractQuick {
            size, destination, ..
        } => {
            read_write_ea(destination, size, at, pc_base, accesses, references);
            if !matches!(destination, EffectiveAddress::AddressRegister(_)) {
                write_arithmetic_flags(accesses, true);
            }
        }
        AddExtend { size, operands } | SubtractExtend { size, operands } => {
            register_pair_read_write(operands, at, size_bytes(size), accesses, references);
            read_flag(accesses, ConditionCode::Extend);
            write_arithmetic_flags(accesses, true);
        }
        AndImmediateToConditionCode(_)
        | ExclusiveOrImmediateToConditionCode(_)
        | OrImmediateToConditionCode(_) => read_write(accesses, Location::ConditionCodeRegister),
        AndImmediateToStatus(_) | ExclusiveOrImmediateToStatus(_) | OrImmediateToStatus(_) => {
            read_write(accesses, Location::StatusRegister)
        }
        ShiftRegister {
            operation,
            count,
            destination,
            size,
            ..
        } => {
            if let ShiftCount::DataRegister(register) = count {
                read_data(accesses, register, 4, AccessOccurrence::Always);
            }
            read_write_data(accesses, destination, size_bytes(size));
            if matches!(operation, ShiftOperation::RotateWithExtend) {
                read_flag(accesses, ConditionCode::Extend);
            }
            write_shift_flags(accesses, operation);
        }
        ShiftMemory {
            operation,
            destination,
            ..
        } => {
            read_write_ea(
                destination,
                OperandSize::Word,
                at,
                pc_base,
                accesses,
                references,
            );
            if matches!(operation, ShiftOperation::RotateWithExtend) {
                read_flag(accesses, ConditionCode::Extend);
            }
            write_shift_flags(accesses, operation);
        }
        Branch { kind, .. } => {
            if let BranchKind::Conditional(condition) = kind {
                read_condition(accesses, condition);
            }
            if matches!(kind, BranchKind::Subroutine) {
                push_stack(at, 4, AccessOccurrence::Always, accesses);
            }
        }
        Bit {
            operation,
            bit_number,
            destination,
        } => {
            if let BitNumber::DataRegister(register) = bit_number {
                read_data(accesses, register, 4, AccessOccurrence::Always);
            }
            let size = if matches!(destination, EffectiveAddress::DataRegister(_)) {
                OperandSize::Long
            } else {
                OperandSize::Byte
            };
            read_ea(destination, size, at, pc_base, accesses, references);
            if !matches!(operation, BitOperation::Test) {
                write_ea(destination, size, at, pc_base, accesses, references);
            }
            write_flag(accesses, ConditionCode::Zero);
        }
        Check {
            source,
            destination,
        } => {
            read_ea(source, OperandSize::Word, at, pc_base, accesses, references);
            read_data(accesses, destination, 2, AccessOccurrence::Always);
            write_with_occurrence(
                accesses,
                Location::ConditionCode(ConditionCode::Negative),
                AccessOccurrence::Conditional,
            );
            exception_accesses(at, 6, AccessOccurrence::Conditional, accesses);
        }
        Clear { size, destination } => {
            write_ea(destination, size, at, pc_base, accesses, references);
            write_logic_flags(accesses);
        }
        Compare {
            size,
            source,
            destination,
        } => {
            read_ea(source, size, at, pc_base, accesses, references);
            read_data(
                accesses,
                destination,
                size_bytes(size),
                AccessOccurrence::Always,
            );
            write_arithmetic_flags(accesses, false);
        }
        CompareMemory {
            size,
            source,
            destination,
        } => {
            for register in [source, destination] {
                read_ea(
                    EffectiveAddress::AddressPostincrement(register),
                    size,
                    at,
                    pc_base,
                    accesses,
                    references,
                );
            }
            write_arithmetic_flags(accesses, false);
        }
        DecrementBranch {
            condition,
            register,
            ..
        } => {
            read_condition(accesses, condition);
            read_data(accesses, register, 2, AccessOccurrence::Conditional);
            write_data(accesses, register, 2, AccessOccurrence::Conditional);
        }
        Divide {
            source,
            destination,
            ..
        } => {
            read_ea(source, OperandSize::Word, at, pc_base, accesses, references);
            read_write_data(accesses, destination, 4);
            write_arithmetic_flags(accesses, false);
            exception_accesses(at, 5, AccessOccurrence::Conditional, accesses);
        }
        ExclusiveOr {
            size,
            source,
            destination,
        } => {
            read_data(accesses, source, size_bytes(size), AccessOccurrence::Always);
            read_write_ea(destination, size, at, pc_base, accesses, references);
            write_logic_flags(accesses);
        }
        Exchange { registers } => exchange(registers, at, accesses),
        Extend { size, register } => {
            let width = match size {
                ExtendSize::ByteToWord => 2,
                ExtendSize::WordToLong => 4,
            };
            read_write_data(accesses, register, width);
            write_logic_flags(accesses);
        }
        Illegal => exception_accesses(at, 4, AccessOccurrence::Always, accesses),
        Jump { target } => address_only(target, at, pc_base, accesses, references, false),
        JumpSubroutine { target } => {
            address_only(target, at, pc_base, accesses, references, false);
            push_stack(at, 4, AccessOccurrence::Always, accesses);
        }
        LoadEffectiveAddress {
            source,
            destination,
        } => {
            address_only(source, at, pc_base, accesses, references, true);
            write_address(accesses, at, destination, AccessOccurrence::Always);
        }
        Link {
            register,
            displacement: _,
        } => {
            read_address(accesses, at, register, AccessOccurrence::Always);
            push_stack(at, 4, AccessOccurrence::Always, accesses);
            write_address(accesses, at, register, AccessOccurrence::Always);
            read_write_address(accesses, at, AddressRegister::A7);
        }
        Move {
            size,
            source,
            destination,
        } => {
            read_ea(source, size, at, pc_base, accesses, references);
            write_ea(destination, size, at, pc_base, accesses, references);
            write_logic_flags(accesses);
        }
        MoveAddress {
            size,
            source,
            destination,
        } => {
            read_ea(source, size.into(), at, pc_base, accesses, references);
            write_address(accesses, at, destination, AccessOccurrence::Always);
        }
        MoveToConditionCode { source } => {
            read_ea(source, OperandSize::Word, at, pc_base, accesses, references);
            write(accesses, Location::ConditionCodeRegister);
        }
        MoveFromStatus { destination } => {
            read(accesses, Location::StatusRegister);
            write_ea(
                destination,
                OperandSize::Word,
                at,
                pc_base,
                accesses,
                references,
            );
        }
        MoveToStatus { source } => {
            read_ea(source, OperandSize::Word, at, pc_base, accesses, references);
            write(accesses, Location::StatusRegister);
        }
        MoveUserStackPointer {
            direction,
            register,
        } => match direction {
            UserStackPointerDirection::AddressRegisterToUserStackPointer => {
                read_address(accesses, at, register, AccessOccurrence::Always);
                write(
                    accesses,
                    Location::AddressRegister(AddressRegisterLocation::UserStackPointer),
                );
            }
            UserStackPointerDirection::UserStackPointerToAddressRegister => {
                read(
                    accesses,
                    Location::AddressRegister(AddressRegisterLocation::UserStackPointer),
                );
                write_address(accesses, at, register, AccessOccurrence::Always);
            }
        },
        MoveMultiple {
            size,
            direction,
            registers,
            effective_address,
        } => move_multiple(
            size,
            direction,
            registers,
            effective_address,
            at,
            pc_base,
            (accesses, references),
        ),
        MovePeripheral {
            size,
            direction,
            data_register,
            address_register,
            displacement,
        } => move_peripheral(
            size,
            direction,
            data_register,
            address_register,
            displacement,
            at,
            (accesses, references),
        ),
        MoveQuick { destination, .. } => {
            write_data(accesses, destination, 4, AccessOccurrence::Always);
            write_logic_flags(accesses);
        }
        Multiply {
            source,
            destination,
            ..
        } => {
            read_ea(source, OperandSize::Word, at, pc_base, accesses, references);
            read_write_data(accesses, destination, 4);
            write_logic_flags(accesses);
        }
        NegateDecimal { destination } => {
            read_write_ea(
                destination,
                OperandSize::Byte,
                at,
                pc_base,
                accesses,
                references,
            );
            read_flag(accesses, ConditionCode::Extend);
            write_arithmetic_flags(accesses, true);
        }
        Negate { size, destination } | NegateExtend { size, destination } => {
            read_write_ea(destination, size, at, pc_base, accesses, references);
            if matches!(instruction, NegateExtend { .. }) {
                read_flag(accesses, ConditionCode::Extend);
            }
            write_arithmetic_flags(accesses, true);
        }
        Nop => {}
        Not { size, destination } => {
            read_write_ea(destination, size, at, pc_base, accesses, references);
            write_logic_flags(accesses);
        }
        PushEffectiveAddress { source } => {
            address_only(source, at, pc_base, accesses, references, true);
            push_stack(at, 4, AccessOccurrence::Always, accesses);
        }
        Reset => write(accesses, Location::ExternalReset),
        ReturnFromException => {
            pop_stack(at, 6, AccessOccurrence::Always, accesses);
            write(accesses, Location::StatusRegister);
        }
        ReturnAndRestore => {
            pop_stack(at, 6, AccessOccurrence::Always, accesses);
            write(accesses, Location::ConditionCodeRegister);
        }
        ReturnFromSubroutine => pop_stack(at, 4, AccessOccurrence::Always, accesses),
        SetCondition {
            condition,
            destination,
        } => {
            read_condition(accesses, condition);
            write_ea(
                destination,
                OperandSize::Byte,
                at,
                pc_base,
                accesses,
                references,
            );
        }
        Stop { .. } => write(accesses, Location::StatusRegister),
        Swap { register } => {
            read_write_data(accesses, register, 4);
            write_logic_flags(accesses);
        }
        TestAndSet { destination } => {
            read_write_ea(
                destination,
                OperandSize::Byte,
                at,
                pc_base,
                accesses,
                references,
            );
            write_logic_flags(accesses);
        }
        Trap { vector } => exception_accesses(at, 32 + vector, AccessOccurrence::Always, accesses),
        TrapOnOverflow => {
            read_flag(accesses, ConditionCode::Overflow);
            exception_accesses(at, 7, AccessOccurrence::Conditional, accesses);
        }
        Test { size, source } => {
            read_ea(source, size, at, pc_base, accesses, references);
            write_logic_flags(accesses);
        }
        Unlink { register } => {
            read_address(accesses, at, register, AccessOccurrence::Always);
            write_address(accesses, at, AddressRegister::A7, AccessOccurrence::Always);
            pop_stack(at, 4, AccessOccurrence::Always, accesses);
            write_address(accesses, at, register, AccessOccurrence::Always);
        }
    }
}

fn pc_extension_base(instruction: &Instruction, at: AnalysisPoint) -> u32 {
    at.location.logical_address().wrapping_add(
        if matches!(
            instruction,
            Instruction::Bit {
                bit_number: BitNumber::Immediate(_),
                ..
            }
        ) {
            4
        } else {
            2
        },
    )
}

fn size_bytes(size: OperandSize) -> u8 {
    match size {
        OperandSize::Byte => 1,
        OperandSize::Word => 2,
        OperandSize::Long => 4,
    }
}

fn address_size_bytes(size: AddressSize) -> u8 {
    match size {
        AddressSize::Word => 2,
        AddressSize::Long => 4,
    }
}

fn address_location(at: AnalysisPoint, register: AddressRegister) -> AddressRegisterLocation {
    if register != AddressRegister::A7 {
        return AddressRegisterLocation::Register(register);
    }
    match at.privilege {
        PrivilegeMode::User => AddressRegisterLocation::UserStackPointer,
        PrivilegeMode::Supervisor => AddressRegisterLocation::SupervisorStackPointer,
        PrivilegeMode::Unknown => AddressRegisterLocation::ActiveStackPointerUnknown,
    }
}

fn read_data(
    accesses: &mut Vec<LocationAccess<Location>>,
    register: DataRegister,
    byte_len: u8,
    occurrence: AccessOccurrence,
) {
    access(
        accesses,
        Location::DataRegister { register, byte_len },
        AccessKind::Read,
        occurrence,
    );
}

fn write_data(
    accesses: &mut Vec<LocationAccess<Location>>,
    register: DataRegister,
    byte_len: u8,
    occurrence: AccessOccurrence,
) {
    access(
        accesses,
        Location::DataRegister { register, byte_len },
        AccessKind::Write,
        occurrence,
    );
}

fn read_write_data(
    accesses: &mut Vec<LocationAccess<Location>>,
    register: DataRegister,
    byte_len: u8,
) {
    read_data(accesses, register, byte_len, AccessOccurrence::Always);
    write_data(accesses, register, byte_len, AccessOccurrence::Always);
}

fn read_address(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    register: AddressRegister,
    occurrence: AccessOccurrence,
) {
    access(
        accesses,
        Location::AddressRegister(address_location(at, register)),
        AccessKind::Read,
        occurrence,
    );
}

fn write_address(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    register: AddressRegister,
    occurrence: AccessOccurrence,
) {
    access(
        accesses,
        Location::AddressRegister(address_location(at, register)),
        AccessKind::Write,
        occurrence,
    );
}

fn read_write_address(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    register: AddressRegister,
) {
    read_address(accesses, at, register, AccessOccurrence::Always);
    write_address(accesses, at, register, AccessOccurrence::Always);
}

fn read_register(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    register: Register,
    byte_len: u8,
) {
    match register {
        Register::Data(register) => {
            read_data(accesses, register, byte_len, AccessOccurrence::Always)
        }
        Register::Address(register) => {
            read_address(accesses, at, register, AccessOccurrence::Always)
        }
    }
}

fn write_register(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    register: Register,
    byte_len: u8,
) {
    match register {
        Register::Data(register) => {
            write_data(accesses, register, byte_len, AccessOccurrence::Always)
        }
        Register::Address(register) => {
            write_address(accesses, at, register, AccessOccurrence::Always)
        }
    }
}

fn read_flag(accesses: &mut Vec<LocationAccess<Location>>, flag: ConditionCode) {
    read(accesses, Location::ConditionCode(flag));
}

fn write_flag(accesses: &mut Vec<LocationAccess<Location>>, flag: ConditionCode) {
    write(accesses, Location::ConditionCode(flag));
}

fn read_condition(accesses: &mut Vec<LocationAccess<Location>>, condition: Condition) {
    use Condition::*;
    let flags: &[ConditionCode] = match condition {
        True | False => &[],
        High | LowOrSame => &[ConditionCode::Carry, ConditionCode::Zero],
        CarryClear | CarrySet => &[ConditionCode::Carry],
        NotEqual | Equal => &[ConditionCode::Zero],
        OverflowClear | OverflowSet => &[ConditionCode::Overflow],
        Plus | Minus => &[ConditionCode::Negative],
        GreaterOrEqual | LessThan => &[ConditionCode::Negative, ConditionCode::Overflow],
        GreaterThan | LessOrEqual => &[
            ConditionCode::Zero,
            ConditionCode::Negative,
            ConditionCode::Overflow,
        ],
    };
    for flag in flags {
        read_flag(accesses, *flag);
    }
}

fn write_arithmetic_flags(accesses: &mut Vec<LocationAccess<Location>>, extend: bool) {
    if extend {
        write_flag(accesses, ConditionCode::Extend);
    }
    for flag in [
        ConditionCode::Negative,
        ConditionCode::Zero,
        ConditionCode::Overflow,
        ConditionCode::Carry,
    ] {
        write_flag(accesses, flag);
    }
}

fn write_logic_flags(accesses: &mut Vec<LocationAccess<Location>>) {
    write_arithmetic_flags(accesses, false);
}

fn write_shift_flags(accesses: &mut Vec<LocationAccess<Location>>, operation: ShiftOperation) {
    write_arithmetic_flags(accesses, !matches!(operation, ShiftOperation::Rotate));
}

type SemanticSinks<'a> = (
    &'a mut Vec<LocationAccess<Location>>,
    &'a mut Vec<StaticReference<Reference>>,
);

fn binary_data_operation(
    direction: DataDirection,
    data_register: DataRegister,
    effective_address: EffectiveAddress,
    size: OperandSize,
    at: AnalysisPoint,
    pc_base: u32,
    sinks: SemanticSinks<'_>,
) {
    let (accesses, references) = sinks;
    let width = size_bytes(size);
    match direction {
        DataDirection::ToDataRegister => {
            read_ea(effective_address, size, at, pc_base, accesses, references);
            read_write_data(accesses, data_register, width);
        }
        DataDirection::ToEffectiveAddress => {
            read_data(accesses, data_register, width, AccessOccurrence::Always);
            read_write_ea(effective_address, size, at, pc_base, accesses, references);
        }
    }
}

fn register_pair_read_write(
    pair: RegisterPair,
    at: AnalysisPoint,
    byte_len: u8,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    match pair {
        RegisterPair::Data {
            source,
            destination,
        } => {
            read_data(accesses, source, byte_len, AccessOccurrence::Always);
            read_write_data(accesses, destination, byte_len);
        }
        RegisterPair::AddressPredecrement {
            source,
            destination,
        } => {
            read_ea(
                EffectiveAddress::AddressPredecrement(source),
                width_size(byte_len),
                at,
                at.location.logical_address().wrapping_add(2),
                accesses,
                references,
            );
            read_write_ea(
                EffectiveAddress::AddressPredecrement(destination),
                width_size(byte_len),
                at,
                at.location.logical_address().wrapping_add(2),
                accesses,
                references,
            );
        }
    }
}

fn width_size(byte_len: u8) -> OperandSize {
    match byte_len {
        1 => OperandSize::Byte,
        2 => OperandSize::Word,
        4 => OperandSize::Long,
        _ => unreachable!("MC68000 scalar widths are one, two, or four bytes"),
    }
}

fn exchange(
    registers: ExchangeRegisters,
    at: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    match registers {
        ExchangeRegisters::Data {
            source,
            destination,
        } => {
            for register in [source, destination] {
                read_write_data(accesses, register, 4);
            }
        }
        ExchangeRegisters::Address {
            source,
            destination,
        } => {
            for register in [source, destination] {
                read_write_address(accesses, at, register);
            }
        }
        ExchangeRegisters::DataAddress { data, address } => {
            read_write_data(accesses, data, 4);
            read_write_address(accesses, at, address);
        }
    }
}

fn ea_memory(
    effective_address: EffectiveAddress,
    at: AnalysisPoint,
    pc_base: u32,
    byte_len: u8,
) -> Option<MemoryLocation> {
    let address = match effective_address {
        EffectiveAddress::AddressIndirect(register) => {
            MemoryAddress::AddressRegister(address_location(at, register))
        }
        EffectiveAddress::AddressPostincrement(register) => {
            MemoryAddress::Postincrement(address_location(at, register))
        }
        EffectiveAddress::AddressPredecrement(register) => {
            MemoryAddress::Predecrement(address_location(at, register))
        }
        EffectiveAddress::AddressDisplacement {
            register,
            displacement,
        } => MemoryAddress::Displacement {
            base: address_location(at, register),
            displacement,
        },
        EffectiveAddress::AddressIndex {
            register,
            index,
            displacement,
        } => MemoryAddress::Indexed {
            base: address_location(at, register),
            index,
            displacement,
        },
        EffectiveAddress::AbsoluteWord(address) => {
            MemoryAddress::Absolute(i32::from(address as i16) as u32)
        }
        EffectiveAddress::AbsoluteLong(address) => MemoryAddress::Absolute(address),
        EffectiveAddress::ProgramCounterDisplacement(displacement) => {
            MemoryAddress::ProgramCounterRelative(
                pc_base.wrapping_add_signed(i32::from(displacement)),
            )
        }
        EffectiveAddress::ProgramCounterIndex {
            index,
            displacement,
        } => MemoryAddress::ProgramCounterIndexed {
            base: pc_base,
            index,
            displacement,
        },
        EffectiveAddress::DataRegister(_)
        | EffectiveAddress::AddressRegister(_)
        | EffectiveAddress::Immediate(_) => return None,
    };
    Some(MemoryLocation::contiguous(address, byte_len))
}

fn address_inputs(
    effective_address: EffectiveAddress,
    at: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    match effective_address {
        EffectiveAddress::AddressIndirect(register)
        | EffectiveAddress::AddressPostincrement(register)
        | EffectiveAddress::AddressPredecrement(register)
        | EffectiveAddress::AddressDisplacement { register, .. } => {
            read_address(accesses, at, register, AccessOccurrence::Always)
        }
        EffectiveAddress::AddressIndex {
            register, index, ..
        } => {
            read_address(accesses, at, register, AccessOccurrence::Always);
            read_register(accesses, at, index.register, 4);
        }
        EffectiveAddress::ProgramCounterIndex { index, .. } => {
            read_register(accesses, at, index.register, 4)
        }
        _ => {}
    }
    match effective_address {
        EffectiveAddress::AddressPostincrement(register)
        | EffectiveAddress::AddressPredecrement(register) => {
            write_address(accesses, at, register, AccessOccurrence::Always)
        }
        _ => {}
    }
}

fn read_ea(
    effective_address: EffectiveAddress,
    size: OperandSize,
    at: AnalysisPoint,
    pc_base: u32,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    let byte_len = size_bytes(size);
    match effective_address {
        EffectiveAddress::DataRegister(register) => {
            read_data(accesses, register, byte_len, AccessOccurrence::Always)
        }
        EffectiveAddress::AddressRegister(register) => {
            read_address(accesses, at, register, AccessOccurrence::Always)
        }
        EffectiveAddress::Immediate(_) => {}
        _ => {
            address_inputs(effective_address, at, accesses);
            let memory = ea_memory(effective_address, at, pc_base, byte_len)
                .expect("memory effective address has memory storage");
            read(accesses, Location::Memory(memory));
            add_memory_reference(references, memory, ReferenceKind::Read);
        }
    }
}

fn write_ea(
    effective_address: EffectiveAddress,
    size: OperandSize,
    at: AnalysisPoint,
    pc_base: u32,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    let byte_len = size_bytes(size);
    match effective_address {
        EffectiveAddress::DataRegister(register) => {
            write_data(accesses, register, byte_len, AccessOccurrence::Always)
        }
        EffectiveAddress::AddressRegister(register) => {
            write_address(accesses, at, register, AccessOccurrence::Always)
        }
        EffectiveAddress::Immediate(_) => {}
        _ => {
            address_inputs(effective_address, at, accesses);
            let memory = ea_memory(effective_address, at, pc_base, byte_len)
                .expect("memory effective address has memory storage");
            write(accesses, Location::Memory(memory));
            add_memory_reference(references, memory, ReferenceKind::Write);
        }
    }
}

fn read_write_ea(
    effective_address: EffectiveAddress,
    size: OperandSize,
    at: AnalysisPoint,
    pc_base: u32,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    read_ea(effective_address, size, at, pc_base, accesses, references);
    write_ea(effective_address, size, at, pc_base, accesses, references);
}

fn address_only(
    effective_address: EffectiveAddress,
    at: AnalysisPoint,
    pc_base: u32,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
    emit_reference: bool,
) {
    address_inputs(effective_address, at, accesses);
    if !emit_reference {
        return;
    }
    let address = match effective_address {
        EffectiveAddress::AbsoluteWord(address) => Some(i32::from(address as i16) as u32),
        EffectiveAddress::AbsoluteLong(address) => Some(address),
        EffectiveAddress::ProgramCounterDisplacement(displacement) => {
            Some(pc_base.wrapping_add_signed(i32::from(displacement)))
        }
        _ => None,
    };
    if let Some(address) = address {
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

fn move_multiple(
    size: AddressSize,
    direction: MoveMultipleDirection,
    registers: RegisterMask,
    effective_address: EffectiveAddress,
    at: AnalysisPoint,
    pc_base: u32,
    sinks: SemanticSinks<'_>,
) {
    let (accesses, references) = sinks;
    let width = address_size_bytes(size);
    let selected: Vec<_> = DataRegister::ALL
        .into_iter()
        .map(Register::Data)
        .chain(AddressRegister::ALL.into_iter().map(Register::Address))
        .filter(|register| registers.contains(*register))
        .collect();
    match direction {
        MoveMultipleDirection::RegistersToMemory => {
            for register in &selected {
                read_register(accesses, at, *register, width);
            }
        }
        MoveMultipleDirection::MemoryToRegisters => {}
    }
    address_inputs(effective_address, at, accesses);
    let mut memory = ea_memory(effective_address, at, pc_base, width)
        .expect("MOVEM uses a memory effective address");
    memory.element_count = u8::try_from(selected.len()).expect("MOVEM has at most 16 registers");
    let kind = match direction {
        MoveMultipleDirection::RegistersToMemory => AccessKind::Write,
        MoveMultipleDirection::MemoryToRegisters => AccessKind::Read,
    };
    access(
        accesses,
        Location::Memory(memory),
        kind,
        AccessOccurrence::Always,
    );
    add_memory_reference(
        references,
        memory,
        if kind == AccessKind::Read {
            ReferenceKind::Read
        } else {
            ReferenceKind::Write
        },
    );
    if matches!(direction, MoveMultipleDirection::MemoryToRegisters) {
        for register in selected {
            write_register(accesses, at, register, width);
        }
    }
}

fn move_peripheral(
    size: AddressSize,
    direction: MovePeripheralDirection,
    data_register: DataRegister,
    address_register: AddressRegister,
    displacement: i16,
    at: AnalysisPoint,
    sinks: SemanticSinks<'_>,
) {
    let (accesses, references) = sinks;
    let element_count = match size {
        AddressSize::Word => 2,
        AddressSize::Long => 4,
    };
    read_address(accesses, at, address_register, AccessOccurrence::Always);
    let memory = MemoryLocation {
        address: MemoryAddress::Displacement {
            base: address_location(at, address_register),
            displacement,
        },
        element_byte_len: 1,
        element_count,
        stride: 2,
    };
    match direction {
        MovePeripheralDirection::MemoryToRegister => {
            read(accesses, Location::Memory(memory));
            write_data(
                accesses,
                data_register,
                element_count,
                AccessOccurrence::Always,
            );
        }
        MovePeripheralDirection::RegisterToMemory => {
            read_data(
                accesses,
                data_register,
                element_count,
                AccessOccurrence::Always,
            );
            write(accesses, Location::Memory(memory));
        }
    }
    let _ = references;
}

fn push_stack(
    at: AnalysisPoint,
    byte_len: u8,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    let stack = address_location(at, AddressRegister::A7);
    access(
        accesses,
        Location::AddressRegister(stack),
        AccessKind::Read,
        occurrence,
    );
    access(
        accesses,
        Location::AddressRegister(stack),
        AccessKind::Write,
        occurrence,
    );
    access(
        accesses,
        Location::Memory(MemoryLocation::contiguous(
            MemoryAddress::Predecrement(stack),
            byte_len,
        )),
        AccessKind::Write,
        occurrence,
    );
}

fn pop_stack(
    at: AnalysisPoint,
    byte_len: u8,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    let stack = address_location(at, AddressRegister::A7);
    access(
        accesses,
        Location::AddressRegister(stack),
        AccessKind::Read,
        occurrence,
    );
    access(
        accesses,
        Location::Memory(MemoryLocation::contiguous(
            MemoryAddress::StackFrame(stack),
            byte_len,
        )),
        AccessKind::Read,
        occurrence,
    );
    access(
        accesses,
        Location::AddressRegister(stack),
        AccessKind::Write,
        occurrence,
    );
}

fn exception_accesses(
    at: AnalysisPoint,
    vector: u8,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    access(
        accesses,
        Location::StatusRegister,
        AccessKind::Read,
        occurrence,
    );
    access(
        accesses,
        Location::StatusRegister,
        AccessKind::Write,
        occurrence,
    );
    let stack = AddressRegisterLocation::SupervisorStackPointer;
    for kind in [AccessKind::Read, AccessKind::Write] {
        access(accesses, Location::AddressRegister(stack), kind, occurrence);
    }
    access(
        accesses,
        Location::Memory(MemoryLocation::contiguous(
            MemoryAddress::StackFrame(stack),
            6,
        )),
        AccessKind::Write,
        occurrence,
    );
    access(
        accesses,
        Location::Memory(MemoryLocation::contiguous(
            MemoryAddress::Absolute(u32::from(vector) * 4),
            4,
        )),
        AccessKind::Read,
        occurrence,
    );
    let _ = at;
}

fn add_memory_reference(
    references: &mut Vec<StaticReference<Reference>>,
    memory: MemoryLocation,
    kind: ReferenceKind,
) {
    let address = match memory.address {
        MemoryAddress::Absolute(address) | MemoryAddress::ProgramCounterRelative(address) => {
            address
        }
        _ => return,
    };
    let target = if memory.stride == memory.element_byte_len {
        Reference::Memory {
            address,
            byte_len: memory.element_byte_len.saturating_mul(memory.element_count),
        }
    } else {
        Reference::StridedMemory {
            address,
            element_count: memory.element_count,
            stride: memory.stride,
        }
    };
    add_reference(
        references,
        StaticReference {
            target,
            kind,
            occurrence: AccessOccurrence::Always,
        },
    );
}

fn read(accesses: &mut Vec<LocationAccess<Location>>, location: Location) {
    access(
        accesses,
        location,
        AccessKind::Read,
        AccessOccurrence::Always,
    );
}

fn write(accesses: &mut Vec<LocationAccess<Location>>, location: Location) {
    access(
        accesses,
        location,
        AccessKind::Write,
        AccessOccurrence::Always,
    );
}

fn read_write(accesses: &mut Vec<LocationAccess<Location>>, location: Location) {
    read(accesses, location);
    write(accesses, location);
}

fn write_with_occurrence(
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
        (
            Location::DataRegister { register: left, .. },
            Location::DataRegister {
                register: right, ..
            },
        ) if left == right => Overlapping,
        (Location::StatusRegister, Location::ConditionCodeRegister)
        | (Location::ConditionCodeRegister, Location::StatusRegister)
        | (Location::StatusRegister, Location::ConditionCode(_))
        | (Location::ConditionCode(_), Location::StatusRegister)
        | (Location::ConditionCodeRegister, Location::ConditionCode(_))
        | (Location::ConditionCode(_), Location::ConditionCodeRegister) => Overlapping,
        (
            Location::AddressRegister(AddressRegisterLocation::ActiveStackPointerUnknown),
            Location::AddressRegister(
                AddressRegisterLocation::UserStackPointer
                | AddressRegisterLocation::SupervisorStackPointer,
            ),
        )
        | (
            Location::AddressRegister(
                AddressRegisterLocation::UserStackPointer
                | AddressRegisterLocation::SupervisorStackPointer,
            ),
            Location::AddressRegister(AddressRegisterLocation::ActiveStackPointerUnknown),
        ) => Unknown,
        (Location::Memory(left), Location::Memory(right)) => memory_relation(*left, *right),
        (Location::Memory(_), _) | (_, Location::Memory(_)) => Unknown,
        _ => Disjoint,
    }
}

fn memory_relation(left: MemoryLocation, right: MemoryLocation) -> StorageRelation {
    let left_address = resolved_address(left.address);
    let right_address = resolved_address(right.address);
    let (Some(left_address), Some(right_address)) = (left_address, right_address) else {
        return StorageRelation::Unknown;
    };
    let left_bytes = memory_bytes(left_address, left);
    let right_bytes = memory_bytes(right_address, right);
    if left_bytes.iter().any(|byte| right_bytes.contains(byte)) {
        StorageRelation::Overlapping
    } else {
        StorageRelation::Disjoint
    }
}

fn resolved_address(address: MemoryAddress) -> Option<u32> {
    match address {
        MemoryAddress::Absolute(address) | MemoryAddress::ProgramCounterRelative(address) => {
            Some(address)
        }
        _ => None,
    }
}

fn memory_bytes(start: u32, memory: MemoryLocation) -> Vec<u32> {
    (0..u32::from(memory.element_count))
        .flat_map(|element| {
            let base = start.wrapping_add(element * u32::from(memory.stride));
            (0..u32::from(memory.element_byte_len)).map(move |offset| base.wrapping_add(offset))
        })
        .collect()
}
