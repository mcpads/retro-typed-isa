//! Complete static meaning for the RP2A03 selector profile.

use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlBoundary, ControlTarget, LocationAccess,
    ReferenceKind, StaticReference, StorageRelation,
};

use crate::{AddressingMode, CodeLocation, Instruction, Mnemonic, Operand};

/// Statically known rule for a runtime-derived RP2A03 control destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndirectTarget {
    /// JMP target loaded through the NMOS page-wrapping pointer rule.
    JumpPointer {
        /// Address of the pointer's low byte.
        address: u16,
    },
    /// Destination restored by a dedicated stack-return instruction.
    StackReturn(ReturnAddressRule),
    /// Handler loaded from the maskable-interrupt vector.
    InterruptVector {
        /// Address of the vector's low byte.
        address: u16,
    },
}

/// Stack-supplied destination rule used by a return instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReturnAddressRule {
    /// RTS restores a saved address and adds one.
    Subroutine,
    /// RTI restores status and the interrupted program counter.
    Interrupt,
}

/// RP2A03 synchronous boundary or stop kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Boundary {
    /// BRK software interrupt.
    Break,
    /// Unofficial KIL instruction-fetch stop.
    JammedUntilReset,
}

/// RP2A03 register storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegisterLocation {
    /// Accumulator A.
    Accumulator,
    /// Index register X.
    X,
    /// Index register Y.
    Y,
    /// Stack pointer S.
    StackPointer,
}

/// Individually meaningful RP2A03 processor-status storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusLocation {
    /// Negative flag N.
    Negative,
    /// Overflow flag V.
    Overflow,
    /// Decimal flag D, retained as storage even though RP2A03 arithmetic ignores it.
    Decimal,
    /// Interrupt-disable flag I.
    InterruptDisable,
    /// Zero flag Z.
    Zero,
    /// Carry flag C.
    Carry,
}

/// Symbolic RP2A03 memory address established by typed operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryAddress {
    /// Fully resolved flat CPU address.
    Direct(u16),
    /// Runtime-indexed or runtime-dereferenced data address.
    Effective {
        /// Addressing form selecting the address rule.
        mode: AddressingMode,
        /// Encoded base or pointer operand.
        operand: Operand,
    },
    /// Pointer bytes read while forming an indirect target.
    Pointer {
        /// Addressing form selecting the pointer rule.
        mode: AddressingMode,
        /// Encoded pointer base.
        operand: Operand,
    },
    /// Runtime stack storage in page one.
    Stack,
    /// Maskable-interrupt vector bytes.
    InterruptVector,
}

/// Architectural RP2A03 storage location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Location {
    /// Register storage.
    Register(RegisterLocation),
    /// Individually meaningful status storage.
    Status(StatusLocation),
    /// One byte of symbolic memory storage.
    Memory(MemoryAddress),
}

/// Statically resolved non-control CPU address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Reference {
    /// First referenced byte.
    pub address: u16,
    /// Number of bytes in the reference.
    pub byte_len: u8,
}

/// Shared control-flow result specialized with RP2A03 vocabulary.
pub type ControlFlow = typed_isa_core::ControlFlow<CodeLocation, IndirectTarget, Boundary>;

/// Shared instruction-semantics result specialized with RP2A03 vocabulary.
pub type InstructionSemantics = typed_isa_core::InstructionSemantics<
    CodeLocation,
    IndirectTarget,
    Boundary,
    Location,
    Reference,
>;

pub(crate) fn semantics(instruction: &Instruction, at: CodeLocation) -> InstructionSemantics {
    let next = instruction.fallthrough_location(at);
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
    use Mnemonic::*;
    match instruction.mnemonic() {
        Bcc | Bcs | Beq | Bmi | Bne | Bpl | Bvc | Bvs => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Direct(relative_target(instruction, next)),
            },
            fallthrough: Some(next),
            delay_slot: None,
        },
        Jmp if instruction.addressing_mode() == AddressingMode::Absolute => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Direct(absolute_target(instruction)),
            },
            fallthrough: None,
            delay_slot: None,
        },
        Jmp => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Indirect(IndirectTarget::JumpPointer {
                    address: absolute_target(instruction),
                }),
            },
            fallthrough: None,
            delay_slot: None,
        },
        Jsr => ControlFlow {
            action: ControlAction::LinkedTransfer {
                target: ControlTarget::Direct(absolute_target(instruction)),
                return_site: next,
            },
            fallthrough: None,
            delay_slot: None,
        },
        Rts => ControlFlow {
            action: ControlAction::Return {
                target: IndirectTarget::StackReturn(ReturnAddressRule::Subroutine),
            },
            fallthrough: None,
            delay_slot: None,
        },
        Rti => ControlFlow {
            action: ControlAction::ExceptionReturn {
                target: IndirectTarget::StackReturn(ReturnAddressRule::Interrupt),
            },
            fallthrough: None,
            delay_slot: None,
        },
        Brk => ControlFlow {
            action: ControlAction::Boundary(ControlBoundary::Trap {
                kind: Boundary::Break,
                target: Some(ControlTarget::Indirect(IndirectTarget::InterruptVector {
                    address: 0xfffe,
                })),
                resume: Some(next),
            }),
            fallthrough: None,
            delay_slot: None,
        },
        Kil => ControlFlow {
            action: ControlAction::Boundary(ControlBoundary::Stop {
                kind: Boundary::JammedUntilReset,
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

fn relative_target(instruction: &Instruction, next: u16) -> u16 {
    let Operand::Relative(displacement) = instruction.operand() else {
        unreachable!("validated branch has a relative operand")
    };
    next.wrapping_add(displacement as i16 as u16)
}

fn absolute_target(instruction: &Instruction) -> u16 {
    let Operand::Word(target) = instruction.operand() else {
        unreachable!("validated absolute control instruction has a word operand")
    };
    target
}

fn add_instruction_accesses(
    instruction: &Instruction,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    use Mnemonic::*;
    use StatusLocation::{Carry, Decimal, InterruptDisable, Negative, Overflow, Zero};

    match instruction.mnemonic() {
        Adc | Sbc => {
            read_register(accesses, RegisterLocation::Accumulator);
            add_operand_access(instruction, AccessKind::Read, accesses, references);
            read_status(accesses, Carry);
            write_register(accesses, RegisterLocation::Accumulator);
            write_statuses(accesses, &[Negative, Overflow, Zero, Carry]);
        }
        And | Eor | Ora => {
            read_register(accesses, RegisterLocation::Accumulator);
            add_operand_access(instruction, AccessKind::Read, accesses, references);
            write_register(accesses, RegisterLocation::Accumulator);
            write_statuses(accesses, &[Negative, Zero]);
        }
        Cmp | Cpx | Cpy => {
            read_register(
                accesses,
                match instruction.mnemonic() {
                    Cmp => RegisterLocation::Accumulator,
                    Cpx => RegisterLocation::X,
                    Cpy => RegisterLocation::Y,
                    _ => unreachable!(),
                },
            );
            add_operand_access(instruction, AccessKind::Read, accesses, references);
            write_statuses(accesses, &[Negative, Zero, Carry]);
        }
        Bit => {
            read_register(accesses, RegisterLocation::Accumulator);
            add_operand_access(instruction, AccessKind::Read, accesses, references);
            write_statuses(accesses, &[Negative, Overflow, Zero]);
        }
        Asl | Lsr | Rol | Ror | Dec | Inc => {
            add_target_access(instruction, AccessKind::Read, accesses, references);
            if matches!(instruction.mnemonic(), Rol | Ror) {
                read_status(accesses, Carry);
            }
            add_target_access(instruction, AccessKind::Write, accesses, references);
            if matches!(instruction.mnemonic(), Asl | Lsr | Rol | Ror) {
                write_statuses(accesses, &[Negative, Zero, Carry]);
            } else {
                write_statuses(accesses, &[Negative, Zero]);
            }
        }
        Lda | Ldx | Ldy => {
            add_operand_access(instruction, AccessKind::Read, accesses, references);
            write_register(
                accesses,
                match instruction.mnemonic() {
                    Lda => RegisterLocation::Accumulator,
                    Ldx => RegisterLocation::X,
                    Ldy => RegisterLocation::Y,
                    _ => unreachable!(),
                },
            );
            write_statuses(accesses, &[Negative, Zero]);
        }
        Sta | Stx | Sty => {
            read_register(
                accesses,
                match instruction.mnemonic() {
                    Sta => RegisterLocation::Accumulator,
                    Stx => RegisterLocation::X,
                    Sty => RegisterLocation::Y,
                    _ => unreachable!(),
                },
            );
            add_operand_access(instruction, AccessKind::Write, accesses, references);
        }
        Bcc | Bcs => read_status(accesses, Carry),
        Beq | Bne => read_status(accesses, Zero),
        Bmi | Bpl => read_status(accesses, Negative),
        Bvc | Bvs => read_status(accesses, Overflow),
        Brk => {
            add_stack_write(accesses, 3);
            read_all_status(accesses);
            write_statuses(accesses, &[InterruptDisable]);
            read_memory(accesses, MemoryAddress::InterruptVector);
        }
        Clc | Sec => write_statuses(accesses, &[Carry]),
        Cld | Sed => write_statuses(accesses, &[Decimal]),
        Cli | Sei => write_statuses(accesses, &[InterruptDisable]),
        Clv => write_statuses(accesses, &[Overflow]),
        Dex | Inx => {
            read_register(accesses, RegisterLocation::X);
            write_register(accesses, RegisterLocation::X);
            write_statuses(accesses, &[Negative, Zero]);
        }
        Dey | Iny => {
            read_register(accesses, RegisterLocation::Y);
            write_register(accesses, RegisterLocation::Y);
            write_statuses(accesses, &[Negative, Zero]);
        }
        Jmp => {
            if instruction.addressing_mode() == AddressingMode::AbsoluteIndirect {
                add_pointer_read(instruction, accesses, references);
            }
        }
        Jsr => add_stack_write(accesses, 2),
        Rts => add_stack_read(accesses, 2),
        Rti => {
            add_stack_read(accesses, 3);
            write_all_status(accesses);
        }
        Pha => {
            read_register(accesses, RegisterLocation::Accumulator);
            add_stack_write(accesses, 1);
        }
        Php => {
            read_all_status(accesses);
            add_stack_write(accesses, 1);
        }
        Pla => {
            add_stack_read(accesses, 1);
            write_register(accesses, RegisterLocation::Accumulator);
            write_statuses(accesses, &[Negative, Zero]);
        }
        Plp => {
            add_stack_read(accesses, 1);
            write_all_status(accesses);
        }
        Tax | Tay | Tsx | Txa | Tya => add_transfer_access(instruction, accesses),
        Txs => {
            read_register(accesses, RegisterLocation::X);
            write_register(accesses, RegisterLocation::StackPointer);
        }
        Dcp => {
            add_operand_access(instruction, AccessKind::Read, accesses, references);
            add_operand_access(instruction, AccessKind::Write, accesses, references);
            read_register(accesses, RegisterLocation::Accumulator);
            write_statuses(accesses, &[Negative, Zero, Carry]);
        }
        Isc => {
            add_operand_access(instruction, AccessKind::Read, accesses, references);
            add_operand_access(instruction, AccessKind::Write, accesses, references);
            read_register(accesses, RegisterLocation::Accumulator);
            read_status(accesses, Carry);
            write_register(accesses, RegisterLocation::Accumulator);
            write_statuses(accesses, &[Negative, Overflow, Zero, Carry]);
        }
        Rla | Rra | Slo | Sre => add_combined_rmw_access(instruction, accesses, references),
        Lax => {
            add_operand_access(instruction, AccessKind::Read, accesses, references);
            write_register(accesses, RegisterLocation::Accumulator);
            write_register(accesses, RegisterLocation::X);
            write_statuses(accesses, &[Negative, Zero]);
        }
        Sax => {
            read_register(accesses, RegisterLocation::Accumulator);
            read_register(accesses, RegisterLocation::X);
            add_operand_access(instruction, AccessKind::Write, accesses, references);
        }
        Las => {
            add_operand_access(instruction, AccessKind::Read, accesses, references);
            read_register(accesses, RegisterLocation::StackPointer);
            for register in [
                RegisterLocation::Accumulator,
                RegisterLocation::X,
                RegisterLocation::StackPointer,
            ] {
                write_register(accesses, register);
            }
            write_statuses(accesses, &[Negative, Zero]);
        }
        Ahx => {
            read_register(accesses, RegisterLocation::Accumulator);
            read_register(accesses, RegisterLocation::X);
            add_operand_access(instruction, AccessKind::Write, accesses, references);
        }
        Shx => {
            read_register(accesses, RegisterLocation::X);
            add_operand_access(instruction, AccessKind::Write, accesses, references);
        }
        Shy => {
            read_register(accesses, RegisterLocation::Y);
            add_operand_access(instruction, AccessKind::Write, accesses, references);
        }
        Tas => {
            read_register(accesses, RegisterLocation::Accumulator);
            read_register(accesses, RegisterLocation::X);
            write_register(accesses, RegisterLocation::StackPointer);
            add_operand_access(instruction, AccessKind::Write, accesses, references);
        }
        Alr | Anc | Arr | Xaa => {
            read_register(accesses, RegisterLocation::Accumulator);
            if instruction.mnemonic() == Xaa {
                read_register(accesses, RegisterLocation::X);
            }
            write_register(accesses, RegisterLocation::Accumulator);
            write_statuses(
                accesses,
                match instruction.mnemonic() {
                    Arr => &[Negative, Overflow, Zero, Carry],
                    _ => &[Negative, Zero, Carry],
                },
            );
        }
        Axs => {
            read_register(accesses, RegisterLocation::Accumulator);
            read_register(accesses, RegisterLocation::X);
            write_register(accesses, RegisterLocation::X);
            write_statuses(accesses, &[Negative, Zero, Carry]);
        }
        Nop => add_operand_access(instruction, AccessKind::Read, accesses, references),
        Kil => {}
    }
}

fn add_combined_rmw_access(
    instruction: &Instruction,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    add_operand_access(instruction, AccessKind::Read, accesses, references);
    add_operand_access(instruction, AccessKind::Write, accesses, references);
    read_register(accesses, RegisterLocation::Accumulator);
    if matches!(instruction.mnemonic(), Mnemonic::Rla | Mnemonic::Rra) {
        read_status(accesses, StatusLocation::Carry);
    }
    write_register(accesses, RegisterLocation::Accumulator);
    let flags = if instruction.mnemonic() == Mnemonic::Rra {
        &[
            StatusLocation::Negative,
            StatusLocation::Overflow,
            StatusLocation::Zero,
            StatusLocation::Carry,
        ][..]
    } else {
        &[
            StatusLocation::Negative,
            StatusLocation::Zero,
            StatusLocation::Carry,
        ][..]
    };
    write_statuses(accesses, flags);
}

fn add_transfer_access(instruction: &Instruction, accesses: &mut Vec<LocationAccess<Location>>) {
    let (source, destination) = match instruction.mnemonic() {
        Mnemonic::Tax => (RegisterLocation::Accumulator, RegisterLocation::X),
        Mnemonic::Tay => (RegisterLocation::Accumulator, RegisterLocation::Y),
        Mnemonic::Tsx => (RegisterLocation::StackPointer, RegisterLocation::X),
        Mnemonic::Txa => (RegisterLocation::X, RegisterLocation::Accumulator),
        Mnemonic::Tya => (RegisterLocation::Y, RegisterLocation::Accumulator),
        _ => unreachable!(),
    };
    read_register(accesses, source);
    write_register(accesses, destination);
    write_statuses(accesses, &[StatusLocation::Negative, StatusLocation::Zero]);
}

fn add_target_access(
    instruction: &Instruction,
    kind: AccessKind,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    if instruction.addressing_mode() == AddressingMode::Accumulator {
        match kind {
            AccessKind::Read => read_register(accesses, RegisterLocation::Accumulator),
            AccessKind::Write => write_register(accesses, RegisterLocation::Accumulator),
        }
    } else {
        add_operand_access(instruction, kind, accesses, references);
    }
}

fn add_operand_access(
    instruction: &Instruction,
    kind: AccessKind,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    use AddressingMode::*;
    let mode = instruction.addressing_mode();
    match mode {
        Implied | Accumulator | Immediate | Relative => return,
        ZeroPageX | AbsoluteX | ZeroPageIndexedIndirectX => {
            read_register(accesses, RegisterLocation::X)
        }
        ZeroPageY | AbsoluteY | ZeroPageIndirectIndexedY => {
            read_register(accesses, RegisterLocation::Y)
        }
        _ => {}
    }
    if matches!(mode, ZeroPageIndexedIndirectX | ZeroPageIndirectIndexedY) {
        add_pointer_read(instruction, accesses, references);
    }

    let address = match (mode, instruction.operand()) {
        (ZeroPage, Operand::Byte(address)) => MemoryAddress::Direct(u16::from(address)),
        (Absolute, Operand::Word(address)) => MemoryAddress::Direct(address),
        _ => MemoryAddress::Effective {
            mode,
            operand: instruction.operand(),
        },
    };
    access(
        accesses,
        Location::Memory(address),
        kind,
        AccessOccurrence::Always,
    );
    if let MemoryAddress::Direct(address) = address {
        references.push(StaticReference {
            target: Reference {
                address,
                byte_len: 1,
            },
            kind: match kind {
                AccessKind::Read => ReferenceKind::Read,
                AccessKind::Write => ReferenceKind::Write,
            },
            occurrence: AccessOccurrence::Always,
        });
    }
}

fn add_pointer_read(
    instruction: &Instruction,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    let mode = instruction.addressing_mode();
    let operand = instruction.operand();
    access(
        accesses,
        Location::Memory(MemoryAddress::Pointer { mode, operand }),
        AccessKind::Read,
        AccessOccurrence::Always,
    );
    let static_address = match (mode, operand) {
        (AddressingMode::ZeroPageIndirectIndexedY, Operand::Byte(address)) => {
            Some(u16::from(address))
        }
        (AddressingMode::AbsoluteIndirect, Operand::Word(address)) => Some(address),
        _ => None,
    };
    if let Some(address) = static_address {
        references.push(StaticReference {
            target: Reference {
                address,
                byte_len: 2,
            },
            kind: ReferenceKind::Read,
            occurrence: AccessOccurrence::Always,
        });
    }
}

fn add_stack_read(accesses: &mut Vec<LocationAccess<Location>>, byte_len: u8) {
    read_register(accesses, RegisterLocation::StackPointer);
    write_register(accesses, RegisterLocation::StackPointer);
    for _ in 0..byte_len {
        read_memory(accesses, MemoryAddress::Stack);
    }
}

fn add_stack_write(accesses: &mut Vec<LocationAccess<Location>>, byte_len: u8) {
    read_register(accesses, RegisterLocation::StackPointer);
    write_register(accesses, RegisterLocation::StackPointer);
    for _ in 0..byte_len {
        write_memory(accesses, MemoryAddress::Stack);
    }
}

fn read_register(accesses: &mut Vec<LocationAccess<Location>>, register: RegisterLocation) {
    access(
        accesses,
        Location::Register(register),
        AccessKind::Read,
        AccessOccurrence::Always,
    );
}

fn write_register(accesses: &mut Vec<LocationAccess<Location>>, register: RegisterLocation) {
    access(
        accesses,
        Location::Register(register),
        AccessKind::Write,
        AccessOccurrence::Always,
    );
}

fn read_status(accesses: &mut Vec<LocationAccess<Location>>, status: StatusLocation) {
    access(
        accesses,
        Location::Status(status),
        AccessKind::Read,
        AccessOccurrence::Always,
    );
}

fn write_statuses(accesses: &mut Vec<LocationAccess<Location>>, statuses: &[StatusLocation]) {
    for status in statuses {
        access(
            accesses,
            Location::Status(*status),
            AccessKind::Write,
            AccessOccurrence::Always,
        );
    }
}

fn read_all_status(accesses: &mut Vec<LocationAccess<Location>>) {
    for status in status_locations() {
        read_status(accesses, status);
    }
}

fn write_all_status(accesses: &mut Vec<LocationAccess<Location>>) {
    write_statuses(accesses, &status_locations());
}

const fn status_locations() -> [StatusLocation; 6] {
    use StatusLocation::*;
    [Negative, Overflow, Decimal, InterruptDisable, Zero, Carry]
}

fn read_memory(accesses: &mut Vec<LocationAccess<Location>>, address: MemoryAddress) {
    access(
        accesses,
        Location::Memory(address),
        AccessKind::Read,
        AccessOccurrence::Always,
    );
}

fn write_memory(accesses: &mut Vec<LocationAccess<Location>>, address: MemoryAddress) {
    access(
        accesses,
        Location::Memory(address),
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
        (Location::Memory(left), Location::Memory(right)) => memory_relation(*left, *right),
        (Location::Memory(_), _) | (_, Location::Memory(_)) => StorageRelation::Disjoint,
        _ => StorageRelation::Disjoint,
    }
}

fn memory_relation(left: MemoryAddress, right: MemoryAddress) -> StorageRelation {
    match (left, right) {
        (MemoryAddress::Direct(left), MemoryAddress::Direct(right)) => {
            if left == right {
                StorageRelation::Same
            } else {
                StorageRelation::Disjoint
            }
        }
        _ => StorageRelation::Unknown,
    }
}
