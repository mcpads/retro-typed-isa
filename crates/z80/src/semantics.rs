//! Complete static meaning for the documented Zilog Z80 profile.

use core::fmt;

use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlBoundary, ControlTarget, LocationAccess,
    ReferenceKind, StaticReference, StorageRelation,
};

use crate::{
    AluOperation, ByteOperand, CodeLocation, Condition, EncodeError, IndexRegister, Instruction,
    Register16, RotateShiftOperation, StackRegister,
};

/// Runtime-derived Z80 control destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndirectTarget {
    /// Destination read through HL.
    Hl,
    /// Destination read through IX or IY.
    Index(IndexRegister),
    /// Destination popped from the memory stack.
    Stack,
}

/// Z80 synchronous wait-state kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Boundary {
    /// HALT wait state.
    Halt,
}

/// Physical byte-register storage, including alternate banks and index halves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ByteRegister {
    /// Accumulator A.
    A,
    /// Flags byte F.
    F,
    /// B register.
    B,
    /// C register.
    C,
    /// D register.
    D,
    /// E register.
    E,
    /// H register.
    H,
    /// L register.
    L,
    /// Alternate accumulator A'.
    AlternateA,
    /// Alternate flags byte F'.
    AlternateF,
    /// Alternate B'.
    AlternateB,
    /// Alternate C'.
    AlternateC,
    /// Alternate D'.
    AlternateD,
    /// Alternate E'.
    AlternateE,
    /// Alternate H'.
    AlternateH,
    /// Alternate L'.
    AlternateL,
    /// High byte of IX.
    IxHigh,
    /// Low byte of IX.
    IxLow,
    /// High byte of IY.
    IyHigh,
    /// Low byte of IY.
    IyLow,
}

/// Individually meaningful documented Z80 flag bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Flag {
    /// Sign flag S.
    Sign,
    /// Zero flag Z.
    Zero,
    /// Half-carry flag H.
    HalfCarry,
    /// Parity or overflow flag P/V.
    ParityOverflow,
    /// Add/subtract flag N.
    Subtract,
    /// Carry flag C.
    Carry,
}

/// Z80 symbolic memory-address rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryAddress {
    /// Immediate 16-bit address.
    Absolute(u16),
    /// Address held in BC.
    Bc,
    /// Address held in DE.
    De,
    /// Address held in HL.
    Hl,
    /// Address held in SP.
    Sp,
    /// Address held in IX or IY plus a signed displacement.
    IndexDisplacement {
        /// Selected index register.
        index: IndexRegister,
        /// Signed byte displacement.
        displacement: i8,
    },
}

/// Z80 symbolic I/O-port addressing rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortAddress {
    /// Immediate low byte with A supplying the high byte.
    AccumulatorHighImmediateLow(u8),
    /// Full port address held in BC.
    Bc,
}

/// Architectural Z80 storage location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Location {
    /// Physical byte-register storage.
    ByteRegister(ByteRegister),
    /// Stack pointer storage.
    StackPointer,
    /// Interrupt-vector-base register I.
    InterruptVector,
    /// Memory-refresh register R.
    Refresh,
    /// First interrupt flip-flop.
    Iff1,
    /// Second interrupt flip-flop.
    Iff2,
    /// Delayed interrupt-enable state established by EI.
    PendingInterruptEnable,
    /// Selected interrupt mode.
    InterruptMode,
    /// Individually meaningful flag storage.
    Flag(Flag),
    /// Symbolic memory byte or word.
    Memory {
        /// Addressing rule.
        address: MemoryAddress,
        /// Access width in bytes.
        byte_len: u8,
    },
    /// Symbolic I/O port byte.
    Port(PortAddress),
}

/// Statically resolved non-control Z80 reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reference {
    /// Absolute memory byte or word.
    Memory {
        /// First referenced byte.
        address: u16,
        /// Referenced width in bytes.
        byte_len: u8,
    },
    /// Immediate port low byte; A supplies the high byte at runtime.
    PortImmediateLow(u8),
}

/// Structural failure to form sound Z80 static facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticError {
    /// The public instruction value does not denote an encodable documented form.
    UnencodableInstruction(EncodeError),
}

impl fmt::Display for SemanticError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnencodableInstruction(error) => {
                write!(formatter, "unencodable Z80 instruction: {error}")
            }
        }
    }
}

impl std::error::Error for SemanticError {}

/// Shared control-flow result specialized with Z80 vocabulary.
pub type ControlFlow = typed_isa_core::ControlFlow<CodeLocation, IndirectTarget, Boundary>;

/// Shared instruction-semantics result specialized with Z80 vocabulary.
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
    let encoded_len = crate::encode_bytes(instruction)
        .map_err(SemanticError::UnencodableInstruction)?
        .len() as u16;
    let next = at.wrapping_add(encoded_len);
    let mut location_accesses = Vec::new();
    let mut static_references = Vec::new();
    add_instruction_accesses(instruction, &mut location_accesses, &mut static_references);
    Ok(InstructionSemantics {
        control_flow: control_flow(instruction, at, next),
        location_accesses,
        static_references,
    })
}

fn control_flow(instruction: &Instruction, at: u16, next: u16) -> ControlFlow {
    let direct_transfer = |target, fallthrough| ControlFlow {
        action: ControlAction::Transfer {
            target: ControlTarget::Direct(target),
        },
        fallthrough,
        delay_slot: None,
    };
    let direct_call = |target, fallthrough| ControlFlow {
        action: ControlAction::LinkedTransfer {
            target: ControlTarget::Direct(target),
            return_site: next,
        },
        fallthrough,
        delay_slot: None,
    };
    match *instruction {
        Instruction::Jp(target) => direct_transfer(target, None),
        Instruction::JpCond(_, target) => direct_transfer(target, Some(next)),
        Instruction::JpHL => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Indirect(IndirectTarget::Hl),
            },
            fallthrough: None,
            delay_slot: None,
        },
        Instruction::JpIx(index) => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Indirect(IndirectTarget::Index(index)),
            },
            fallthrough: None,
            delay_slot: None,
        },
        Instruction::Jr(displacement) => {
            direct_transfer(next.wrapping_add_signed(i16::from(displacement)), None)
        }
        Instruction::JrCond(_, displacement) | Instruction::Djnz(displacement) => direct_transfer(
            next.wrapping_add_signed(i16::from(displacement)),
            Some(next),
        ),
        Instruction::Call(target) => direct_call(target, None),
        Instruction::CallCond(_, target) => direct_call(target, Some(next)),
        Instruction::Rst(target) => direct_call(u16::from(target), None),
        Instruction::Ret => stack_return(false, next),
        Instruction::RetCond(_) => stack_return(true, next),
        Instruction::Reti | Instruction::Retn => ControlFlow {
            action: ControlAction::ExceptionReturn {
                target: IndirectTarget::Stack,
            },
            fallthrough: None,
            delay_slot: None,
        },
        Instruction::Inir
        | Instruction::Indr
        | Instruction::Otir
        | Instruction::Otdr
        | Instruction::Ldir
        | Instruction::Lddr
        | Instruction::Cpir
        | Instruction::Cpdr => direct_transfer(at, Some(next)),
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

fn stack_return(conditional: bool, next: u16) -> ControlFlow {
    ControlFlow {
        action: ControlAction::Return {
            target: IndirectTarget::Stack,
        },
        fallthrough: conditional.then_some(next),
        delay_slot: None,
    }
}

fn add_instruction_accesses(
    instruction: &Instruction,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    use Instruction::*;
    match *instruction {
        Nop | Halt => {}
        Di => {
            write(accesses, Location::Iff1, AccessOccurrence::Always);
            write(accesses, Location::Iff2, AccessOccurrence::Always);
            write(
                accesses,
                Location::PendingInterruptEnable,
                AccessOccurrence::Always,
            );
        }
        Ei => write(
            accesses,
            Location::PendingInterruptEnable,
            AccessOccurrence::Always,
        ),
        LdRR(destination, source) => {
            read_byte_operand(accesses, source);
            write_byte_operand(accesses, destination);
        }
        LdRImm(destination, _) => write_byte_operand(accesses, destination),
        LdAddrA(address) => {
            read_byte(accesses, ByteRegister::A);
            absolute_memory(accesses, references, address, 1, AccessKind::Write);
        }
        LdAAddr(address) => {
            absolute_memory(accesses, references, address, 1, AccessKind::Read);
            write_byte(accesses, ByteRegister::A);
        }
        LdBCA => register_pair_memory(accesses, Register16::Bc, 1, AccessKind::Write, true),
        LdDEA => register_pair_memory(accesses, Register16::De, 1, AccessKind::Write, true),
        LdABC => register_pair_memory(accesses, Register16::Bc, 1, AccessKind::Read, true),
        LdADE => register_pair_memory(accesses, Register16::De, 1, AccessKind::Read, true),
        LdAddrHL(address) => {
            read_pair(accesses, Register16::Hl);
            absolute_memory(accesses, references, address, 2, AccessKind::Write);
        }
        LdHLAddr(address) => {
            absolute_memory(accesses, references, address, 2, AccessKind::Read);
            write_pair(accesses, Register16::Hl);
        }
        LdRRImm(pair, _) => write_pair(accesses, pair),
        LdSPHL => {
            read_pair(accesses, Register16::Hl);
            write(accesses, Location::StackPointer, AccessOccurrence::Always);
        }
        Push(pair) => add_push(accesses, stack_pair_bytes(pair)),
        Pop(pair) => add_pop(accesses, stack_pair_bytes(pair)),
        AluR(operation, operand) => add_alu(operation, Some(operand), accesses),
        AluImm(operation, _) => add_alu(operation, None, accesses),
        IncR(operand) | DecR(operand) => {
            read_byte_operand(accesses, operand);
            write_byte_operand(accesses, operand);
            write_flags(accesses, ALU_FLAGS_WITHOUT_CARRY);
        }
        IncRR(pair) | DecRR(pair) => read_write_pair(accesses, pair),
        AddHLRR(pair) => {
            read_pair(accesses, Register16::Hl);
            read_pair(accesses, pair);
            write_pair(accesses, Register16::Hl);
            write_flags(accesses, &[Flag::HalfCarry, Flag::Subtract, Flag::Carry]);
        }
        Jp(_) | Jr(_) => {}
        JpCond(condition, _) | JrCond(condition, _) => read_condition(accesses, condition),
        JpHL => read_pair(accesses, Register16::Hl),
        JpIx(index) => read_index(accesses, index),
        Djnz(_) => {
            read_byte(accesses, ByteRegister::B);
            write_byte(accesses, ByteRegister::B);
        }
        Call(_) | Rst(_) => add_call_stack(accesses, AccessOccurrence::Always),
        CallCond(condition, _) => {
            read_condition(accesses, condition);
            add_call_stack(accesses, AccessOccurrence::Conditional);
        }
        Ret => add_return_stack(accesses, AccessOccurrence::Always),
        RetCond(condition) => {
            read_condition(accesses, condition);
            add_return_stack(accesses, AccessOccurrence::Conditional);
        }
        Reti | Retn => {
            add_return_stack(accesses, AccessOccurrence::Always);
            read(accesses, Location::Iff2, AccessOccurrence::Always);
            write(accesses, Location::Iff1, AccessOccurrence::Always);
        }
        Rlca | Rrca | Rla | Rra => {
            read_byte(accesses, ByteRegister::A);
            if matches!(instruction, Rla | Rra) {
                read_flag(accesses, Flag::Carry);
            }
            write_byte(accesses, ByteRegister::A);
            write_flags(accesses, &[Flag::HalfCarry, Flag::Subtract, Flag::Carry]);
        }
        ExAFAF => exchange_bytes(
            accesses,
            &[
                ByteRegister::A,
                ByteRegister::F,
                ByteRegister::AlternateA,
                ByteRegister::AlternateF,
            ],
        ),
        Exx => exchange_bytes(
            accesses,
            &[
                ByteRegister::B,
                ByteRegister::C,
                ByteRegister::D,
                ByteRegister::E,
                ByteRegister::H,
                ByteRegister::L,
                ByteRegister::AlternateB,
                ByteRegister::AlternateC,
                ByteRegister::AlternateD,
                ByteRegister::AlternateE,
                ByteRegister::AlternateH,
                ByteRegister::AlternateL,
            ],
        ),
        ExDEHL => exchange_bytes(
            accesses,
            &[
                ByteRegister::D,
                ByteRegister::E,
                ByteRegister::H,
                ByteRegister::L,
            ],
        ),
        ExSPHL => exchange_stack_pair(accesses, PairView::Register(Register16::Hl)),
        Daa => {
            read_byte(accesses, ByteRegister::A);
            for flag in [Flag::HalfCarry, Flag::Subtract, Flag::Carry] {
                read_flag(accesses, flag);
            }
            write_byte(accesses, ByteRegister::A);
            write_flags(accesses, ALU_FLAGS);
        }
        Cpl => {
            read_byte(accesses, ByteRegister::A);
            write_byte(accesses, ByteRegister::A);
            write_flags(accesses, &[Flag::HalfCarry, Flag::Subtract]);
        }
        Neg => {
            read_byte(accesses, ByteRegister::A);
            write_byte(accesses, ByteRegister::A);
            write_flags(accesses, ALU_FLAGS);
        }
        Scf => write_flags(accesses, &[Flag::HalfCarry, Flag::Subtract, Flag::Carry]),
        Ccf => {
            read_flag(accesses, Flag::Carry);
            write_flags(accesses, &[Flag::HalfCarry, Flag::Subtract, Flag::Carry]);
        }
        InAPort(port) => {
            read_byte(accesses, ByteRegister::A);
            port_access(
                accesses,
                references,
                PortAddress::AccumulatorHighImmediateLow(port),
                AccessKind::Read,
            );
            write_byte(accesses, ByteRegister::A);
        }
        OutPortA(port) => {
            read_byte(accesses, ByteRegister::A);
            port_access(
                accesses,
                references,
                PortAddress::AccumulatorHighImmediateLow(port),
                AccessKind::Write,
            );
        }
        InRC(operand) => {
            read_pair(accesses, Register16::Bc);
            access(
                accesses,
                Location::Port(PortAddress::Bc),
                AccessKind::Read,
                AccessOccurrence::Always,
            );
            write_byte_operand(accesses, operand);
            write_flags(accesses, ALU_FLAGS_WITHOUT_CARRY);
        }
        OutCR(operand) => {
            read_pair(accesses, Register16::Bc);
            read_byte_operand(accesses, operand);
            access(
                accesses,
                Location::Port(PortAddress::Bc),
                AccessKind::Write,
                AccessOccurrence::Always,
            );
        }
        Ini | Ind | Inir | Indr => add_block_input(accesses),
        Outi | Outd | Otir | Otdr => add_block_output(accesses),
        Ldi | Ldd | Ldir | Lddr => add_block_transfer(accesses),
        Cpi | Cpd | Cpir | Cpdr => add_block_compare(accesses),
        Rot(operation, operand) => add_rotate(operation, operand, accesses),
        Bit(_, operand) => {
            read_byte_operand(accesses, operand);
            read_flag(accesses, Flag::Carry);
            write_flags(
                accesses,
                &[
                    Flag::Sign,
                    Flag::Zero,
                    Flag::HalfCarry,
                    Flag::ParityOverflow,
                    Flag::Subtract,
                ],
            );
        }
        Res(_, operand) | Set(_, operand) => {
            read_byte_operand(accesses, operand);
            write_byte_operand(accesses, operand);
        }
        SbcHLRR(pair) | AdcHLRR(pair) => {
            read_pair(accesses, Register16::Hl);
            read_pair(accesses, pair);
            read_flag(accesses, Flag::Carry);
            write_pair(accesses, Register16::Hl);
            write_flags(accesses, ALU_FLAGS);
        }
        LdAddrRR(address, pair) => {
            read_pair(accesses, pair);
            absolute_memory(accesses, references, address, 2, AccessKind::Write);
        }
        LdRRAddr(pair, address) => {
            absolute_memory(accesses, references, address, 2, AccessKind::Read);
            write_pair(accesses, pair);
        }
        LdIA => {
            read_byte(accesses, ByteRegister::A);
            write(
                accesses,
                Location::InterruptVector,
                AccessOccurrence::Always,
            );
        }
        LdRA => {
            read_byte(accesses, ByteRegister::A);
            write(accesses, Location::Refresh, AccessOccurrence::Always);
        }
        LdAI | LdAR => {
            read(
                accesses,
                if matches!(instruction, LdAI) {
                    Location::InterruptVector
                } else {
                    Location::Refresh
                },
                AccessOccurrence::Always,
            );
            read(accesses, Location::Iff2, AccessOccurrence::Always);
            write_byte(accesses, ByteRegister::A);
            write_flags(accesses, ALU_FLAGS_WITHOUT_CARRY);
        }
        Rrd | Rld => {
            read_pair(accesses, Register16::Hl);
            read_byte(accesses, ByteRegister::A);
            let memory = Location::Memory {
                address: MemoryAddress::Hl,
                byte_len: 1,
            };
            read(accesses, memory, AccessOccurrence::Always);
            write(accesses, memory, AccessOccurrence::Always);
            write_byte(accesses, ByteRegister::A);
            write_flags(accesses, ALU_FLAGS_WITHOUT_CARRY);
        }
        Im(_) => write(accesses, Location::InterruptMode, AccessOccurrence::Always),
        LdIxImm(index, _) => write_index(accesses, index),
        LdIxAddr(index, address) => {
            absolute_memory(accesses, references, address, 2, AccessKind::Read);
            write_index(accesses, index);
        }
        LdAddrIx(address, index) => {
            read_index(accesses, index);
            absolute_memory(accesses, references, address, 2, AccessKind::Write);
        }
        PushIx(index) => add_push(accesses, index_bytes(index)),
        PopIx(index) => add_pop(accesses, index_bytes(index)),
        IncIx(index) | DecIx(index) => read_write_index(accesses, index),
        AddIxRR(index, pair) => {
            read_index(accesses, index);
            if pair == Register16::Hl {
                read_index(accesses, index);
            } else {
                read_pair(accesses, pair);
            }
            write_index(accesses, index);
            write_flags(accesses, &[Flag::HalfCarry, Flag::Subtract, Flag::Carry]);
        }
        LdSpIx(index) => {
            read_index(accesses, index);
            write(accesses, Location::StackPointer, AccessOccurrence::Always);
        }
        ExSpIx(index) => exchange_stack_pair(accesses, PairView::Index(index)),
        LdRIxDisp(destination, index, displacement) => {
            read_index(accesses, index);
            read_index_memory(accesses, index, displacement);
            write_byte_operand(accesses, destination);
        }
        LdIxDispR(index, displacement, source) => {
            read_index(accesses, index);
            read_byte_operand(accesses, source);
            write_index_memory(accesses, index, displacement);
        }
        LdIxDispImm(index, displacement, _) => {
            read_index(accesses, index);
            write_index_memory(accesses, index, displacement);
        }
        IncIxDisp(index, displacement) | DecIxDisp(index, displacement) => {
            read_index(accesses, index);
            read_index_memory(accesses, index, displacement);
            write_index_memory(accesses, index, displacement);
            write_flags(accesses, ALU_FLAGS_WITHOUT_CARRY);
        }
        AluIxDisp(operation, index, displacement) => {
            read_index(accesses, index);
            read_index_memory(accesses, index, displacement);
            add_alu(operation, None, accesses);
        }
        RotIxDisp(operation, index, displacement) => {
            read_index(accesses, index);
            read_index_memory(accesses, index, displacement);
            if matches!(
                operation,
                RotateShiftOperation::Rl | RotateShiftOperation::Rr
            ) {
                read_flag(accesses, Flag::Carry);
            }
            write_index_memory(accesses, index, displacement);
            write_flags(accesses, ALU_FLAGS);
        }
        BitIxDisp(_, index, displacement) => {
            read_index(accesses, index);
            read_index_memory(accesses, index, displacement);
            read_flag(accesses, Flag::Carry);
            write_flags(
                accesses,
                &[
                    Flag::Sign,
                    Flag::Zero,
                    Flag::HalfCarry,
                    Flag::ParityOverflow,
                    Flag::Subtract,
                ],
            );
        }
        ResIxDisp(_, index, displacement) | SetIxDisp(_, index, displacement) => {
            read_index(accesses, index);
            read_index_memory(accesses, index, displacement);
            write_index_memory(accesses, index, displacement);
        }
    }
}

const ALU_FLAGS: &[Flag] = &[
    Flag::Sign,
    Flag::Zero,
    Flag::HalfCarry,
    Flag::ParityOverflow,
    Flag::Subtract,
    Flag::Carry,
];
const ALU_FLAGS_WITHOUT_CARRY: &[Flag] = &[
    Flag::Sign,
    Flag::Zero,
    Flag::HalfCarry,
    Flag::ParityOverflow,
    Flag::Subtract,
];

fn add_alu(
    operation: AluOperation,
    operand: Option<ByteOperand>,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_byte(accesses, ByteRegister::A);
    if let Some(operand) = operand {
        read_byte_operand(accesses, operand);
    }
    if matches!(operation, AluOperation::Adc | AluOperation::Sbc) {
        read_flag(accesses, Flag::Carry);
    }
    if operation != AluOperation::Cp {
        write_byte(accesses, ByteRegister::A);
    }
    write_flags(accesses, ALU_FLAGS);
}

fn add_rotate(
    operation: RotateShiftOperation,
    operand: ByteOperand,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_byte_operand(accesses, operand);
    if matches!(
        operation,
        RotateShiftOperation::Rl | RotateShiftOperation::Rr
    ) {
        read_flag(accesses, Flag::Carry);
    }
    write_byte_operand(accesses, operand);
    write_flags(accesses, ALU_FLAGS);
}

fn add_block_input(accesses: &mut Vec<LocationAccess<Location>>) {
    read_pair(accesses, Register16::Bc);
    read_pair(accesses, Register16::Hl);
    access(
        accesses,
        Location::Port(PortAddress::Bc),
        AccessKind::Read,
        AccessOccurrence::Always,
    );
    access(
        accesses,
        Location::Memory {
            address: MemoryAddress::Hl,
            byte_len: 1,
        },
        AccessKind::Write,
        AccessOccurrence::Always,
    );
    read_write_byte(accesses, ByteRegister::B);
    read_write_pair(accesses, Register16::Hl);
    write_flags(accesses, ALU_FLAGS);
}

fn add_block_output(accesses: &mut Vec<LocationAccess<Location>>) {
    read_pair(accesses, Register16::Bc);
    read_pair(accesses, Register16::Hl);
    access(
        accesses,
        Location::Memory {
            address: MemoryAddress::Hl,
            byte_len: 1,
        },
        AccessKind::Read,
        AccessOccurrence::Always,
    );
    access(
        accesses,
        Location::Port(PortAddress::Bc),
        AccessKind::Write,
        AccessOccurrence::Always,
    );
    read_write_byte(accesses, ByteRegister::B);
    read_write_pair(accesses, Register16::Hl);
    write_flags(accesses, ALU_FLAGS);
}

fn add_block_transfer(accesses: &mut Vec<LocationAccess<Location>>) {
    read_pair(accesses, Register16::Hl);
    read_pair(accesses, Register16::De);
    access(
        accesses,
        Location::Memory {
            address: MemoryAddress::Hl,
            byte_len: 1,
        },
        AccessKind::Read,
        AccessOccurrence::Always,
    );
    access(
        accesses,
        Location::Memory {
            address: MemoryAddress::De,
            byte_len: 1,
        },
        AccessKind::Write,
        AccessOccurrence::Always,
    );
    read_write_pair(accesses, Register16::Hl);
    read_write_pair(accesses, Register16::De);
    read_write_pair(accesses, Register16::Bc);
    write_flags(
        accesses,
        &[Flag::HalfCarry, Flag::ParityOverflow, Flag::Subtract],
    );
}

fn add_block_compare(accesses: &mut Vec<LocationAccess<Location>>) {
    read_byte(accesses, ByteRegister::A);
    read_pair(accesses, Register16::Hl);
    access(
        accesses,
        Location::Memory {
            address: MemoryAddress::Hl,
            byte_len: 1,
        },
        AccessKind::Read,
        AccessOccurrence::Always,
    );
    read_write_pair(accesses, Register16::Hl);
    read_write_pair(accesses, Register16::Bc);
    write_flags(accesses, ALU_FLAGS_WITHOUT_CARRY);
}

fn register_pair_memory(
    accesses: &mut Vec<LocationAccess<Location>>,
    pair: Register16,
    byte_len: u8,
    kind: AccessKind,
    accumulator: bool,
) {
    read_pair(accesses, pair);
    if accumulator && kind == AccessKind::Write {
        read_byte(accesses, ByteRegister::A);
    }
    let address = match pair {
        Register16::Bc => MemoryAddress::Bc,
        Register16::De => MemoryAddress::De,
        Register16::Hl => MemoryAddress::Hl,
        Register16::Sp => MemoryAddress::Sp,
    };
    access(
        accesses,
        Location::Memory { address, byte_len },
        kind,
        AccessOccurrence::Always,
    );
    if accumulator && kind == AccessKind::Read {
        write_byte(accesses, ByteRegister::A);
    }
}

fn absolute_memory(
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
    address: u16,
    byte_len: u8,
    kind: AccessKind,
) {
    access(
        accesses,
        Location::Memory {
            address: MemoryAddress::Absolute(address),
            byte_len,
        },
        kind,
        AccessOccurrence::Always,
    );
    references.push(StaticReference {
        target: Reference::Memory { address, byte_len },
        kind: if kind == AccessKind::Read {
            ReferenceKind::Read
        } else {
            ReferenceKind::Write
        },
        occurrence: AccessOccurrence::Always,
    });
}

fn port_access(
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
    address: PortAddress,
    kind: AccessKind,
) {
    access(
        accesses,
        Location::Port(address),
        kind,
        AccessOccurrence::Always,
    );
    if let PortAddress::AccumulatorHighImmediateLow(low) = address {
        references.push(StaticReference {
            target: Reference::PortImmediateLow(low),
            kind: if kind == AccessKind::Read {
                ReferenceKind::Read
            } else {
                ReferenceKind::Write
            },
            occurrence: AccessOccurrence::Always,
        });
    }
}

fn add_call_stack(accesses: &mut Vec<LocationAccess<Location>>, occurrence: AccessOccurrence) {
    read(accesses, Location::StackPointer, occurrence);
    write(accesses, Location::StackPointer, occurrence);
    access(
        accesses,
        Location::Memory {
            address: MemoryAddress::Sp,
            byte_len: 2,
        },
        AccessKind::Write,
        occurrence,
    );
}

fn add_return_stack(accesses: &mut Vec<LocationAccess<Location>>, occurrence: AccessOccurrence) {
    read(accesses, Location::StackPointer, occurrence);
    access(
        accesses,
        Location::Memory {
            address: MemoryAddress::Sp,
            byte_len: 2,
        },
        AccessKind::Read,
        occurrence,
    );
    write(accesses, Location::StackPointer, occurrence);
}

fn add_push(accesses: &mut Vec<LocationAccess<Location>>, bytes: &[ByteRegister]) {
    for byte in bytes {
        read_byte(accesses, *byte);
    }
    add_call_stack(accesses, AccessOccurrence::Always);
}
fn add_pop(accesses: &mut Vec<LocationAccess<Location>>, bytes: &[ByteRegister]) {
    add_return_stack(accesses, AccessOccurrence::Always);
    for byte in bytes {
        write_byte(accesses, *byte);
    }
}

enum PairView {
    Register(Register16),
    Index(IndexRegister),
}
fn exchange_stack_pair(accesses: &mut Vec<LocationAccess<Location>>, pair: PairView) {
    read(accesses, Location::StackPointer, AccessOccurrence::Always);
    access(
        accesses,
        Location::Memory {
            address: MemoryAddress::Sp,
            byte_len: 2,
        },
        AccessKind::Read,
        AccessOccurrence::Always,
    );
    access(
        accesses,
        Location::Memory {
            address: MemoryAddress::Sp,
            byte_len: 2,
        },
        AccessKind::Write,
        AccessOccurrence::Always,
    );
    match pair {
        PairView::Register(pair) => read_write_pair(accesses, pair),
        PairView::Index(index) => read_write_index(accesses, index),
    }
}

fn read_byte_operand(accesses: &mut Vec<LocationAccess<Location>>, operand: ByteOperand) {
    if operand == ByteOperand::IndirectHl {
        read_pair(accesses, Register16::Hl);
        read(
            accesses,
            Location::Memory {
                address: MemoryAddress::Hl,
                byte_len: 1,
            },
            AccessOccurrence::Always,
        );
    } else {
        read_byte(accesses, byte_operand_register(operand));
    }
}
fn write_byte_operand(accesses: &mut Vec<LocationAccess<Location>>, operand: ByteOperand) {
    if operand == ByteOperand::IndirectHl {
        read_pair(accesses, Register16::Hl);
        write(
            accesses,
            Location::Memory {
                address: MemoryAddress::Hl,
                byte_len: 1,
            },
            AccessOccurrence::Always,
        );
    } else {
        write_byte(accesses, byte_operand_register(operand));
    }
}

fn byte_operand_register(operand: ByteOperand) -> ByteRegister {
    match operand {
        ByteOperand::A => ByteRegister::A,
        ByteOperand::B => ByteRegister::B,
        ByteOperand::C => ByteRegister::C,
        ByteOperand::D => ByteRegister::D,
        ByteOperand::E => ByteRegister::E,
        ByteOperand::H => ByteRegister::H,
        ByteOperand::L => ByteRegister::L,
        ByteOperand::IndirectHl => unreachable!(),
    }
}
fn pair_bytes(pair: Register16) -> &'static [ByteRegister] {
    match pair {
        Register16::Bc => &[ByteRegister::B, ByteRegister::C],
        Register16::De => &[ByteRegister::D, ByteRegister::E],
        Register16::Hl => &[ByteRegister::H, ByteRegister::L],
        Register16::Sp => &[],
    }
}
fn stack_pair_bytes(pair: StackRegister) -> &'static [ByteRegister] {
    match pair {
        StackRegister::Bc => pair_bytes(Register16::Bc),
        StackRegister::De => pair_bytes(Register16::De),
        StackRegister::Hl => pair_bytes(Register16::Hl),
        StackRegister::Af => &[ByteRegister::A, ByteRegister::F],
    }
}
fn index_bytes(index: IndexRegister) -> &'static [ByteRegister] {
    match index {
        IndexRegister::Ix => &[ByteRegister::IxHigh, ByteRegister::IxLow],
        IndexRegister::Iy => &[ByteRegister::IyHigh, ByteRegister::IyLow],
    }
}

fn read_pair(accesses: &mut Vec<LocationAccess<Location>>, pair: Register16) {
    if pair == Register16::Sp {
        read(accesses, Location::StackPointer, AccessOccurrence::Always);
    } else {
        for byte in pair_bytes(pair) {
            read_byte(accesses, *byte);
        }
    }
}
fn write_pair(accesses: &mut Vec<LocationAccess<Location>>, pair: Register16) {
    if pair == Register16::Sp {
        write(accesses, Location::StackPointer, AccessOccurrence::Always);
    } else {
        for byte in pair_bytes(pair) {
            write_byte(accesses, *byte);
        }
    }
}
fn read_write_pair(accesses: &mut Vec<LocationAccess<Location>>, pair: Register16) {
    read_pair(accesses, pair);
    write_pair(accesses, pair);
}
fn read_index(accesses: &mut Vec<LocationAccess<Location>>, index: IndexRegister) {
    for byte in index_bytes(index) {
        read_byte(accesses, *byte);
    }
}
fn write_index(accesses: &mut Vec<LocationAccess<Location>>, index: IndexRegister) {
    for byte in index_bytes(index) {
        write_byte(accesses, *byte);
    }
}
fn read_write_index(accesses: &mut Vec<LocationAccess<Location>>, index: IndexRegister) {
    read_index(accesses, index);
    write_index(accesses, index);
}
fn read_index_memory(
    accesses: &mut Vec<LocationAccess<Location>>,
    index: IndexRegister,
    displacement: i8,
) {
    read(
        accesses,
        Location::Memory {
            address: MemoryAddress::IndexDisplacement {
                index,
                displacement,
            },
            byte_len: 1,
        },
        AccessOccurrence::Always,
    );
}
fn write_index_memory(
    accesses: &mut Vec<LocationAccess<Location>>,
    index: IndexRegister,
    displacement: i8,
) {
    write(
        accesses,
        Location::Memory {
            address: MemoryAddress::IndexDisplacement {
                index,
                displacement,
            },
            byte_len: 1,
        },
        AccessOccurrence::Always,
    );
}

fn exchange_bytes(accesses: &mut Vec<LocationAccess<Location>>, bytes: &[ByteRegister]) {
    for byte in bytes {
        read_byte(accesses, *byte);
        write_byte(accesses, *byte);
    }
}
fn read_write_byte(accesses: &mut Vec<LocationAccess<Location>>, byte: ByteRegister) {
    read_byte(accesses, byte);
    write_byte(accesses, byte);
}
fn read_byte(accesses: &mut Vec<LocationAccess<Location>>, byte: ByteRegister) {
    read(
        accesses,
        Location::ByteRegister(byte),
        AccessOccurrence::Always,
    );
}
fn write_byte(accesses: &mut Vec<LocationAccess<Location>>, byte: ByteRegister) {
    write(
        accesses,
        Location::ByteRegister(byte),
        AccessOccurrence::Always,
    );
}
fn read_condition(accesses: &mut Vec<LocationAccess<Location>>, condition: Condition) {
    read_flag(
        accesses,
        match condition {
            Condition::Nz | Condition::Z => Flag::Zero,
            Condition::Nc | Condition::C => Flag::Carry,
            Condition::Po | Condition::Pe => Flag::ParityOverflow,
            Condition::P | Condition::M => Flag::Sign,
        },
    );
}
fn read_flag(accesses: &mut Vec<LocationAccess<Location>>, flag: Flag) {
    read(accesses, Location::Flag(flag), AccessOccurrence::Always);
}
fn write_flags(accesses: &mut Vec<LocationAccess<Location>>, flags: &[Flag]) {
    for flag in flags {
        write(accesses, Location::Flag(*flag), AccessOccurrence::Always);
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

pub(crate) fn storage_relation(left: &Location, right: &Location) -> StorageRelation {
    use StorageRelation::*;
    if left == right {
        return Same;
    }
    match (left, right) {
        (Location::ByteRegister(ByteRegister::F), Location::Flag(_))
        | (Location::Flag(_), Location::ByteRegister(ByteRegister::F)) => Overlapping,
        (
            Location::Memory {
                address: MemoryAddress::Absolute(left),
                byte_len: left_len,
            },
            Location::Memory {
                address: MemoryAddress::Absolute(right),
                byte_len: right_len,
            },
        ) => {
            if ranges_overlap(*left, *left_len, *right, *right_len) {
                Overlapping
            } else {
                Disjoint
            }
        }
        (Location::Memory { .. }, Location::Memory { .. })
        | (Location::Port(_), Location::Port(_)) => Unknown,
        (Location::Memory { .. }, _)
        | (_, Location::Memory { .. })
        | (Location::Port(_), _)
        | (_, Location::Port(_)) => Disjoint,
        _ => Disjoint,
    }
}

fn ranges_overlap(left: u16, left_len: u8, right: u16, right_len: u8) -> bool {
    (0..u16::from(left_len)).any(|offset| {
        let address = left.wrapping_add(offset);
        (0..u16::from(right_len)).any(|other| address == right.wrapping_add(other))
    })
}
