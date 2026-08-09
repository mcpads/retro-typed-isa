//! Complete static meaning for the SM83 instruction profile.

use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlBoundary, ControlTarget, LocationAccess,
    ReferenceKind, StaticReference, StorageRelation,
};

use crate::{
    AluOperation, ByteOperand, CodeLocation, Condition, IndirectAddress, Instruction, Register16,
    RotateShiftOperation, StackRegister,
};

/// Runtime-derived SM83 control destination described statically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndirectTarget {
    /// Destination read from the HL register pair.
    Hl,
    /// Destination restored by a dedicated stack-return instruction.
    StackReturn(ReturnAddressRule),
}

/// Stack-supplied destination rule used by a return instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReturnAddressRule {
    /// RET restores a subroutine return address.
    Subroutine,
    /// RETI restores an interrupted address and enables interrupts.
    Interrupt,
}

/// SM83 wait or stop kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Boundary {
    /// HALT waits for an interrupt condition.
    Halt,
    /// STOP enters the architecture-defined stopped state.
    Stop,
}

/// Physical SM83 byte-register storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ByteRegister {
    /// Accumulator A.
    A,
    /// General register B.
    B,
    /// General register C.
    C,
    /// General register D.
    D,
    /// General register E.
    E,
    /// General register H.
    H,
    /// General register L.
    L,
}

/// Individually meaningful SM83 status storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusLocation {
    /// Zero flag Z.
    Zero,
    /// Subtract flag N.
    Subtract,
    /// Half-carry flag H.
    HalfCarry,
    /// Carry flag C.
    Carry,
}

/// Symbolic SM83 memory address established by typed operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryAddress {
    /// Fully resolved CPU address.
    Direct(u16),
    /// Address held in a register pair.
    RegisterIndirect(Register16),
    /// High-page address `0xFF00 + C`.
    HighPageC,
    /// Runtime stack storage.
    Stack,
}

/// Architectural SM83 storage location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Location {
    /// Physical byte-register storage.
    Register(ByteRegister),
    /// Stack pointer SP.
    StackPointer,
    /// Individually meaningful status storage.
    Status(StatusLocation),
    /// Interrupt master enable state.
    InterruptMasterEnable,
    /// Delayed enable state established by EI.
    InterruptEnablePending,
    /// Symbolic memory byte.
    Memory(MemoryAddress),
}

/// Statically resolved non-control SM83 memory reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Reference {
    /// First referenced byte.
    pub address: u16,
    /// Number of bytes in the reference.
    pub byte_len: u8,
}

/// Shared control-flow result specialized with SM83 vocabulary.
pub type ControlFlow = typed_isa_core::ControlFlow<CodeLocation, IndirectTarget, Boundary>;

/// Shared instruction-semantics result specialized with SM83 vocabulary.
pub type InstructionSemantics = typed_isa_core::InstructionSemantics<
    CodeLocation,
    IndirectTarget,
    Boundary,
    Location,
    Reference,
>;

pub(crate) fn semantics(instruction: &Instruction, at: CodeLocation) -> InstructionSemantics {
    let next = at.wrapping_add(instruction.encoded_len() as u16);
    let mut accesses = Vec::new();
    let mut references = Vec::new();
    add_instruction_accesses(instruction, &mut accesses, &mut references);
    InstructionSemantics {
        control_flow: control_flow(instruction, next),
        location_accesses: accesses,
        static_references: references,
    }
}

fn control_flow(instruction: &Instruction, next: CodeLocation) -> ControlFlow {
    match *instruction {
        Instruction::JumpRelative(displacement) => transfer(relative_target(next, displacement)),
        Instruction::JumpRelativeIf(_, displacement) => {
            conditional_transfer(relative_target(next, displacement), next)
        }
        Instruction::JumpIf(_, target) => conditional_transfer(target, next),
        Instruction::Jump(target) => transfer(target),
        Instruction::JumpHl => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Indirect(IndirectTarget::Hl),
            },
            fallthrough: None,
            delay_slot: None,
        },
        Instruction::CallIf(_, target) => conditional_link(target, next, true),
        Instruction::Call(target) => conditional_link(target, next, false),
        Instruction::Restart(vector) => conditional_link(u16::from(vector.value()), next, false),
        Instruction::ReturnIf(_) => ControlFlow {
            action: ControlAction::Return {
                target: IndirectTarget::StackReturn(ReturnAddressRule::Subroutine),
            },
            fallthrough: Some(next),
            delay_slot: None,
        },
        Instruction::Return => ControlFlow {
            action: ControlAction::Return {
                target: IndirectTarget::StackReturn(ReturnAddressRule::Subroutine),
            },
            fallthrough: None,
            delay_slot: None,
        },
        Instruction::ReturnFromInterrupt => ControlFlow {
            action: ControlAction::ExceptionReturn {
                target: IndirectTarget::StackReturn(ReturnAddressRule::Interrupt),
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
        Instruction::Stop => ControlFlow {
            action: ControlAction::Boundary(ControlBoundary::Stop {
                kind: Boundary::Stop,
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

fn transfer(target: u16) -> ControlFlow {
    ControlFlow {
        action: ControlAction::Transfer {
            target: ControlTarget::Direct(target),
        },
        fallthrough: None,
        delay_slot: None,
    }
}

fn conditional_transfer(target: u16, next: u16) -> ControlFlow {
    ControlFlow {
        action: ControlAction::Transfer {
            target: ControlTarget::Direct(target),
        },
        fallthrough: Some(next),
        delay_slot: None,
    }
}

fn conditional_link(target: u16, next: u16, conditional: bool) -> ControlFlow {
    ControlFlow {
        action: ControlAction::LinkedTransfer {
            target: ControlTarget::Direct(target),
            return_site: next,
        },
        fallthrough: conditional.then_some(next),
        delay_slot: None,
    }
}

fn relative_target(next: u16, displacement: i8) -> u16 {
    next.wrapping_add(displacement as i16 as u16)
}

fn add_instruction_accesses(
    instruction: &Instruction,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    use Instruction::*;
    use StatusLocation::{Carry, HalfCarry, Subtract, Zero};

    match *instruction {
        Nop | Stop | JumpRelative(_) | Jump(_) => {}
        Halt => read_location(accesses, Location::InterruptMasterEnable),
        Load16Immediate(register, _) => write_pair(accesses, register),
        LoadIndirectFromA(address) => {
            read_register(accesses, ByteRegister::A);
            access_indirect(address, AccessKind::Write, accesses);
        }
        LoadAFromIndirect(address) => {
            access_indirect(address, AccessKind::Read, accesses);
            write_register(accesses, ByteRegister::A);
        }
        Increment16(register) | Decrement16(register) => {
            read_pair(accesses, register);
            write_pair(accesses, register);
        }
        Increment8(operand) | Decrement8(operand) => {
            access_byte_operand(operand, AccessKind::Read, accesses);
            access_byte_operand(operand, AccessKind::Write, accesses);
            write_statuses(accesses, &[Zero, Subtract, HalfCarry]);
        }
        Load8Immediate(destination, _) => {
            access_byte_operand(destination, AccessKind::Write, accesses)
        }
        Load8(destination, source) => {
            access_byte_operand(source, AccessKind::Read, accesses);
            access_byte_operand(destination, AccessKind::Write, accesses);
        }
        LoadAddressFromSp(address) => {
            read_stack_pointer(accesses);
            write_direct_memory(address, 2, accesses, references);
        }
        AddHl(register) => {
            read_pair(accesses, Register16::Hl);
            read_pair(accesses, register);
            write_pair(accesses, Register16::Hl);
            write_statuses(accesses, &[Subtract, HalfCarry, Carry]);
        }
        RotateLeftCircularA | RotateRightCircularA => {
            read_register(accesses, ByteRegister::A);
            write_register(accesses, ByteRegister::A);
            write_statuses(accesses, &[Zero, Subtract, HalfCarry, Carry]);
        }
        RotateLeftA | RotateRightA => {
            read_register(accesses, ByteRegister::A);
            read_status(accesses, Carry);
            write_register(accesses, ByteRegister::A);
            write_statuses(accesses, &[Zero, Subtract, HalfCarry, Carry]);
        }
        JumpRelativeIf(condition, _) | JumpIf(condition, _) => read_condition(accesses, condition),
        CallIf(condition, _) => {
            read_condition(accesses, condition);
            add_conditional_stack_write(accesses, 2);
        }
        ReturnIf(condition) => {
            read_condition(accesses, condition);
            add_conditional_stack_read(accesses, 2);
        }
        DecimalAdjustA => {
            read_register(accesses, ByteRegister::A);
            for status in [Subtract, HalfCarry, Carry] {
                read_status(accesses, status);
            }
            write_register(accesses, ByteRegister::A);
            write_statuses(accesses, &[Zero, HalfCarry, Carry]);
        }
        ComplementA => {
            read_register(accesses, ByteRegister::A);
            write_register(accesses, ByteRegister::A);
            write_statuses(accesses, &[Subtract, HalfCarry]);
        }
        SetCarryFlag => write_statuses(accesses, &[Subtract, HalfCarry, Carry]),
        ComplementCarryFlag => {
            read_status(accesses, Carry);
            write_statuses(accesses, &[Subtract, HalfCarry, Carry]);
        }
        Alu8(operation, operand) => add_alu_access(operation, Some(operand), accesses),
        Pop(register) => {
            add_stack_read(accesses, 2);
            write_stack_register(accesses, register);
        }
        Push(register) => {
            read_stack_register(accesses, register);
            add_stack_write(accesses, 2);
        }
        AluImmediate(operation, _) => add_alu_access(operation, None, accesses),
        Call(_) | Restart(_) => add_stack_write(accesses, 2),
        Return => add_stack_read(accesses, 2),
        ReturnFromInterrupt => {
            add_stack_read(accesses, 2);
            write_location(accesses, Location::InterruptMasterEnable);
            write_location(accesses, Location::InterruptEnablePending);
        }
        LoadHighAddressFromA(offset) => {
            read_register(accesses, ByteRegister::A);
            write_direct_memory(0xff00 | u16::from(offset), 1, accesses, references);
        }
        LoadHighAddressCFromA => {
            read_register(accesses, ByteRegister::A);
            read_register(accesses, ByteRegister::C);
            write_memory(accesses, MemoryAddress::HighPageC);
        }
        AddSp(_) => {
            read_stack_pointer(accesses);
            write_stack_pointer(accesses);
            write_statuses(accesses, &[Zero, Subtract, HalfCarry, Carry]);
        }
        JumpHl => read_pair(accesses, Register16::Hl),
        LoadAddressFromA(address) => {
            read_register(accesses, ByteRegister::A);
            write_direct_memory(address, 1, accesses, references);
        }
        LoadAFromHighAddress(offset) => {
            read_direct_memory(0xff00 | u16::from(offset), 1, accesses, references);
            write_register(accesses, ByteRegister::A);
        }
        LoadAFromHighAddressC => {
            read_register(accesses, ByteRegister::C);
            read_memory(accesses, MemoryAddress::HighPageC);
            write_register(accesses, ByteRegister::A);
        }
        DisableInterrupts => {
            write_location(accesses, Location::InterruptMasterEnable);
            write_location(accesses, Location::InterruptEnablePending);
        }
        LoadHlFromSpPlus(_) => {
            read_stack_pointer(accesses);
            write_pair(accesses, Register16::Hl);
            write_statuses(accesses, &[Zero, Subtract, HalfCarry, Carry]);
        }
        LoadSpFromHl => {
            read_pair(accesses, Register16::Hl);
            write_stack_pointer(accesses);
        }
        LoadAFromAddress(address) => {
            read_direct_memory(address, 1, accesses, references);
            write_register(accesses, ByteRegister::A);
        }
        EnableInterrupts => write_location(accesses, Location::InterruptEnablePending),
        RotateShift(operation, operand) => {
            access_byte_operand(operand, AccessKind::Read, accesses);
            if matches!(
                operation,
                RotateShiftOperation::Rl | RotateShiftOperation::Rr
            ) {
                read_status(accesses, Carry);
            }
            access_byte_operand(operand, AccessKind::Write, accesses);
            write_statuses(accesses, &[Zero, Subtract, HalfCarry, Carry]);
        }
        Bit(_, operand) => {
            access_byte_operand(operand, AccessKind::Read, accesses);
            write_statuses(accesses, &[Zero, Subtract, HalfCarry]);
        }
        ResetBit(_, operand) | SetBit(_, operand) => {
            access_byte_operand(operand, AccessKind::Read, accesses);
            access_byte_operand(operand, AccessKind::Write, accesses);
        }
    }
}

fn add_alu_access(
    operation: AluOperation,
    operand: Option<ByteOperand>,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_register(accesses, ByteRegister::A);
    if let Some(operand) = operand {
        access_byte_operand(operand, AccessKind::Read, accesses);
    }
    if matches!(operation, AluOperation::Adc | AluOperation::Sbc) {
        read_status(accesses, StatusLocation::Carry);
    }
    if operation != AluOperation::Cp {
        write_register(accesses, ByteRegister::A);
    }
    write_statuses(
        accesses,
        &[
            StatusLocation::Zero,
            StatusLocation::Subtract,
            StatusLocation::HalfCarry,
            StatusLocation::Carry,
        ],
    );
}

fn access_indirect(
    address: IndirectAddress,
    kind: AccessKind,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    let register = match address {
        IndirectAddress::Bc => Register16::Bc,
        IndirectAddress::De => Register16::De,
        IndirectAddress::HlIncrement | IndirectAddress::HlDecrement => Register16::Hl,
    };
    read_pair(accesses, register);
    access(
        accesses,
        Location::Memory(MemoryAddress::RegisterIndirect(register)),
        kind,
    );
    if matches!(
        address,
        IndirectAddress::HlIncrement | IndirectAddress::HlDecrement
    ) {
        write_pair(accesses, Register16::Hl);
    }
}

fn access_byte_operand(
    operand: ByteOperand,
    kind: AccessKind,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    let location = match operand {
        ByteOperand::A => Location::Register(ByteRegister::A),
        ByteOperand::B => Location::Register(ByteRegister::B),
        ByteOperand::C => Location::Register(ByteRegister::C),
        ByteOperand::D => Location::Register(ByteRegister::D),
        ByteOperand::E => Location::Register(ByteRegister::E),
        ByteOperand::H => Location::Register(ByteRegister::H),
        ByteOperand::L => Location::Register(ByteRegister::L),
        ByteOperand::IndirectHl => {
            read_pair(accesses, Register16::Hl);
            Location::Memory(MemoryAddress::RegisterIndirect(Register16::Hl))
        }
    };
    access(accesses, location, kind);
}

fn read_condition(accesses: &mut Vec<LocationAccess<Location>>, condition: Condition) {
    read_status(
        accesses,
        match condition {
            Condition::Nz | Condition::Z => StatusLocation::Zero,
            Condition::Nc | Condition::C => StatusLocation::Carry,
        },
    );
}

fn read_pair(accesses: &mut Vec<LocationAccess<Location>>, register: Register16) {
    match register {
        Register16::Bc => read_register_pair(accesses, ByteRegister::B, ByteRegister::C),
        Register16::De => read_register_pair(accesses, ByteRegister::D, ByteRegister::E),
        Register16::Hl => read_register_pair(accesses, ByteRegister::H, ByteRegister::L),
        Register16::Sp => read_stack_pointer(accesses),
    }
}

fn write_pair(accesses: &mut Vec<LocationAccess<Location>>, register: Register16) {
    match register {
        Register16::Bc => write_register_pair(accesses, ByteRegister::B, ByteRegister::C),
        Register16::De => write_register_pair(accesses, ByteRegister::D, ByteRegister::E),
        Register16::Hl => write_register_pair(accesses, ByteRegister::H, ByteRegister::L),
        Register16::Sp => write_stack_pointer(accesses),
    }
}

fn read_stack_register(accesses: &mut Vec<LocationAccess<Location>>, register: StackRegister) {
    match register {
        StackRegister::Bc => read_pair(accesses, Register16::Bc),
        StackRegister::De => read_pair(accesses, Register16::De),
        StackRegister::Hl => read_pair(accesses, Register16::Hl),
        StackRegister::Af => {
            read_register(accesses, ByteRegister::A);
            for status in status_locations() {
                read_status(accesses, status);
            }
        }
    }
}

fn write_stack_register(accesses: &mut Vec<LocationAccess<Location>>, register: StackRegister) {
    match register {
        StackRegister::Bc => write_pair(accesses, Register16::Bc),
        StackRegister::De => write_pair(accesses, Register16::De),
        StackRegister::Hl => write_pair(accesses, Register16::Hl),
        StackRegister::Af => {
            write_register(accesses, ByteRegister::A);
            write_statuses(accesses, &status_locations());
        }
    }
}

fn read_register_pair(
    accesses: &mut Vec<LocationAccess<Location>>,
    high: ByteRegister,
    low: ByteRegister,
) {
    read_register(accesses, high);
    read_register(accesses, low);
}

fn write_register_pair(
    accesses: &mut Vec<LocationAccess<Location>>,
    high: ByteRegister,
    low: ByteRegister,
) {
    write_register(accesses, high);
    write_register(accesses, low);
}

fn read_register(accesses: &mut Vec<LocationAccess<Location>>, register: ByteRegister) {
    access(accesses, Location::Register(register), AccessKind::Read);
}

fn write_register(accesses: &mut Vec<LocationAccess<Location>>, register: ByteRegister) {
    access(accesses, Location::Register(register), AccessKind::Write);
}

fn read_stack_pointer(accesses: &mut Vec<LocationAccess<Location>>) {
    access(accesses, Location::StackPointer, AccessKind::Read);
}

fn write_stack_pointer(accesses: &mut Vec<LocationAccess<Location>>) {
    access(accesses, Location::StackPointer, AccessKind::Write);
}

fn read_status(accesses: &mut Vec<LocationAccess<Location>>, status: StatusLocation) {
    access(accesses, Location::Status(status), AccessKind::Read);
}

fn write_statuses(accesses: &mut Vec<LocationAccess<Location>>, statuses: &[StatusLocation]) {
    for status in statuses {
        access(accesses, Location::Status(*status), AccessKind::Write);
    }
}

const fn status_locations() -> [StatusLocation; 4] {
    use StatusLocation::*;
    [Zero, Subtract, HalfCarry, Carry]
}

fn add_stack_read(accesses: &mut Vec<LocationAccess<Location>>, byte_len: u8) {
    read_stack_pointer(accesses);
    write_stack_pointer(accesses);
    for _ in 0..byte_len {
        read_memory(accesses, MemoryAddress::Stack);
    }
}

fn add_stack_write(accesses: &mut Vec<LocationAccess<Location>>, byte_len: u8) {
    read_stack_pointer(accesses);
    write_stack_pointer(accesses);
    for _ in 0..byte_len {
        write_memory(accesses, MemoryAddress::Stack);
    }
}

fn add_conditional_stack_read(accesses: &mut Vec<LocationAccess<Location>>, byte_len: u8) {
    access_with_occurrence(
        accesses,
        Location::StackPointer,
        AccessKind::Read,
        AccessOccurrence::Conditional,
    );
    access_with_occurrence(
        accesses,
        Location::StackPointer,
        AccessKind::Write,
        AccessOccurrence::Conditional,
    );
    for _ in 0..byte_len {
        access_with_occurrence(
            accesses,
            Location::Memory(MemoryAddress::Stack),
            AccessKind::Read,
            AccessOccurrence::Conditional,
        );
    }
}

fn add_conditional_stack_write(accesses: &mut Vec<LocationAccess<Location>>, byte_len: u8) {
    access_with_occurrence(
        accesses,
        Location::StackPointer,
        AccessKind::Read,
        AccessOccurrence::Conditional,
    );
    access_with_occurrence(
        accesses,
        Location::StackPointer,
        AccessKind::Write,
        AccessOccurrence::Conditional,
    );
    for _ in 0..byte_len {
        access_with_occurrence(
            accesses,
            Location::Memory(MemoryAddress::Stack),
            AccessKind::Write,
            AccessOccurrence::Conditional,
        );
    }
}

fn read_direct_memory(
    address: u16,
    byte_len: u8,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    for offset in 0..byte_len {
        read_memory(
            accesses,
            MemoryAddress::Direct(address.wrapping_add(u16::from(offset))),
        );
    }
    references.push(StaticReference {
        target: Reference { address, byte_len },
        kind: ReferenceKind::Read,
        occurrence: AccessOccurrence::Always,
    });
}

fn write_direct_memory(
    address: u16,
    byte_len: u8,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    for offset in 0..byte_len {
        write_memory(
            accesses,
            MemoryAddress::Direct(address.wrapping_add(u16::from(offset))),
        );
    }
    references.push(StaticReference {
        target: Reference { address, byte_len },
        kind: ReferenceKind::Write,
        occurrence: AccessOccurrence::Always,
    });
}

fn read_memory(accesses: &mut Vec<LocationAccess<Location>>, address: MemoryAddress) {
    access(accesses, Location::Memory(address), AccessKind::Read);
}

fn write_memory(accesses: &mut Vec<LocationAccess<Location>>, address: MemoryAddress) {
    access(accesses, Location::Memory(address), AccessKind::Write);
}

fn write_location(accesses: &mut Vec<LocationAccess<Location>>, location: Location) {
    access(accesses, location, AccessKind::Write);
}

fn read_location(accesses: &mut Vec<LocationAccess<Location>>, location: Location) {
    access(accesses, location, AccessKind::Read);
}

fn access(accesses: &mut Vec<LocationAccess<Location>>, location: Location, kind: AccessKind) {
    access_with_occurrence(accesses, location, kind, AccessOccurrence::Always);
}

fn access_with_occurrence(
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
        (
            Location::Memory(MemoryAddress::Direct(left)),
            Location::Memory(MemoryAddress::Direct(right)),
        ) => {
            if left == right {
                StorageRelation::Same
            } else {
                StorageRelation::Disjoint
            }
        }
        (Location::Memory(_), Location::Memory(_)) => StorageRelation::Unknown,
        (Location::Memory(_), _) | (_, Location::Memory(_)) => StorageRelation::Disjoint,
        _ => StorageRelation::Disjoint,
    }
}
