//! Complete native V-series static meaning shared by V30-family exact profiles.

use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlBoundary, ControlTarget, LocationAccess,
    ReferenceKind, StaticReference, StorageRelation,
};

use crate::{
    BitIndex, CallTarget, CodeLocation, Condition, EffectiveAddress, EffectiveAddressBase,
    EncodeError, FloatingPointOperand, Instruction, JmpTarget, LoopCondition, Operand, OperandSize,
    PortAddress, Register8, Register16, SegmentRegister, ShiftCount,
};

/// Interrupt-acceptance state relevant to the successor of STI and SS loads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InterruptState {
    /// Maskable interrupts are accepted at ordinary instruction boundaries.
    Enabled,
    /// IF is clear.
    Disabled,
    /// STI has set IF, but recognition begins after the following instruction.
    EnableAfterNextInstruction,
    /// Loading SS inhibits recognition through the following instruction.
    InhibitedAfterStackSegmentLoad,
    /// A FLAGS restore made the state unavailable.
    Unknown,
}

/// Placed native V-series instruction and minimal successor state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnalysisPoint {
    /// Segmented architectural code location.
    pub location: CodeLocation,
    /// Known interrupt-acceptance state.
    pub interrupt_state: InterruptState,
}

impl AnalysisPoint {
    /// Constructs a native V-series static-analysis point.
    pub const fn new(location: CodeLocation, interrupt_state: InterruptState) -> Self {
        Self {
            location,
            interrupt_state,
        }
    }
}

/// Runtime-derived native V-series control destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndirectTarget {
    /// Near destination read through a register or memory operand.
    Near(Operand),
    /// Far destination read from an m16:16 pointer.
    Far(EffectiveAddress),
    /// Near return offset popped from SS:SP.
    NearReturn,
    /// Far return CS:IP popped from SS:SP.
    FarReturn,
    /// Interrupt return FLAGS and CS:IP popped from SS:SP.
    InterruptReturn,
    /// Handler CS:IP read from one interrupt-vector-table entry.
    InterruptVector {
        /// Interrupt number.
        vector: u8,
        /// First byte of the four-byte vector entry.
        table_offset: u16,
    },
    /// BRKEM vector interpreted by the 8080-emulation profile.
    EmulationVector(u8),
}

/// Native V-series synchronous boundary, wait, or profile-exit reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Boundary {
    /// Divide error, interrupt 0.
    DivideError,
    /// BOUND range exception, interrupt 5.
    BoundRange,
    /// Software interrupt.
    Interrupt(u8),
    /// Conditional overflow interrupt 4.
    Overflow,
    /// Halt until an external interrupt or reset.
    Halt,
    /// WAIT synchronization with the external numeric processor.
    NumericProcessorWait,
    /// Exit from native V30 decoding through BRKEM.
    EmulationMode(u8),
}

/// Individually meaningful FLAGS bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Flag {
    /// Carry flag CF.
    Carry,
    /// Parity flag PF.
    Parity,
    /// Auxiliary-carry flag AF.
    AuxiliaryCarry,
    /// Zero flag ZF.
    Zero,
    /// Sign flag SF.
    Sign,
    /// Trap flag TF.
    Trap,
    /// Interrupt-enable flag IF.
    InterruptEnable,
    /// Direction flag DF.
    Direction,
    /// Overflow flag OF.
    Overflow,
}

/// Segment storage selected for a symbolic memory location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SegmentSelection {
    /// Runtime value of a segment register.
    Register(SegmentRegister),
    /// Current code segment, known from placement.
    CurrentCode(u16),
    /// Fixed zero segment used by the interrupt vector table.
    Zero,
}

/// Symbolic 16-bit offset within a native V-series segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetAddress {
    /// Absolute sixteen-bit displacement-only offset.
    Direct(u16),
    /// 8086 ModR/M base expression plus signed displacement.
    Effective {
        /// Typed base expression.
        base: EffectiveAddressBase,
        /// Signed displacement.
        displacement: i16,
    },
    /// Offset held in one register, used by strings.
    Register(Register16),
    /// XLAT offset formed from BX and unsigned AL.
    Xlat,
    /// Current stack top at SP.
    Stack,
}

/// Symbolic native V-series memory storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryLocation {
    /// Selected segment source.
    pub segment: SegmentSelection,
    /// Offset expression within the segment.
    pub offset: OffsetAddress,
    /// Contiguous transfer width in bytes.
    pub byte_len: u8,
}

/// Symbolic I/O port storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortLocation {
    /// Immediate eight-bit port number.
    Immediate(u8),
    /// Runtime sixteen-bit port number in DX.
    DataRegisterDx,
}

/// External numeric-processor command family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExternalProcessorFamily {
    /// 8087-compatible ESC command.
    Escape,
    /// NEC FPO2 command.
    Fpo2,
}

/// Architectural native V-series storage or externally visible resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Location {
    /// Entire sixteen-bit general register.
    Register16(Register16),
    /// Low or high byte register view.
    Register8(Register8),
    /// Segment register storage.
    SegmentRegister(SegmentRegister),
    /// Entire FLAGS word.
    Flags,
    /// One FLAGS bit.
    Flag(Flag),
    /// Symbolic segmented memory storage.
    Memory(MemoryLocation),
    /// Symbolic I/O port storage and width.
    Port {
        /// Port-number rule.
        port: PortLocation,
        /// Transfer width in bytes.
        byte_len: u8,
    },
    /// Command handed to an external numeric processor.
    ExternalProcessorCommand {
        /// Command namespace.
        family: ExternalProcessorFamily,
        /// Checked operation selector.
        operation: u8,
        /// External register or memory operand descriptor.
        operand: FloatingPointOperand,
    },
    /// Bus-lock state asserted by the LOCK prefix.
    BusLock,
}

/// Statically resolved non-control native V-series reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reference {
    /// Segmented direct memory range.
    Memory {
        /// Segment source.
        segment: SegmentSelection,
        /// First offset.
        offset: u16,
        /// Contiguous byte count.
        byte_len: u8,
    },
    /// Sixteen-bit offset produced by LEA without a segment or memory access.
    Offset(u16),
}

/// Failure to form sound native V-series static facts.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SemanticError {
    /// The typed instruction is not executable in the selected base profile.
    #[error(transparent)]
    Encode(#[from] EncodeError),
}

/// Shared control-flow result specialized with native V-series vocabulary.
pub type ControlFlow = typed_isa_core::ControlFlow<AnalysisPoint, IndirectTarget, Boundary>;

/// Shared instruction-semantics result specialized with native V-series vocabulary.
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
    let byte_len = crate::encode_bytes(instruction)?.len();
    let next = successor(instruction, at, advance(at.location, byte_len));
    let mut accesses = Vec::new();
    let mut references = Vec::new();
    instruction_accesses(
        instruction,
        at,
        None,
        AccessOccurrence::Always,
        &mut accesses,
        &mut references,
    );
    Ok(InstructionSemantics {
        control_flow: control_flow(instruction, at, next),
        location_accesses: accesses,
        static_references: references,
    })
}

fn successor(
    instruction: &Instruction,
    mut at: AnalysisPoint,
    location: CodeLocation,
) -> AnalysisPoint {
    let mut state = match at.interrupt_state {
        InterruptState::EnableAfterNextInstruction
        | InterruptState::InhibitedAfterStackSegmentLoad => InterruptState::Enabled,
        state => state,
    };
    let instruction = unwrap_prefixes(instruction);
    state = match instruction {
        Instruction::Cli => InterruptState::Disabled,
        Instruction::Sti => InterruptState::EnableAfterNextInstruction,
        Instruction::Pop {
            dest: Operand::Sreg(SegmentRegister::SS),
        }
        | Instruction::Mov {
            dest: Operand::Sreg(SegmentRegister::SS),
            ..
        } => InterruptState::InhibitedAfterStackSegmentLoad,
        Instruction::Popf | Instruction::Iret => InterruptState::Unknown,
        _ => state,
    };
    at.location = location;
    at.interrupt_state = state;
    at
}

fn unwrap_prefixes(mut instruction: &Instruction) -> &Instruction {
    loop {
        instruction = match instruction {
            Instruction::Rep(inner)
            | Instruction::Repne(inner)
            | Instruction::Repc(inner)
            | Instruction::Repnc(inner)
            | Instruction::BusLock(inner)
            | Instruction::SegmentOverride {
                instruction: inner, ..
            } => inner,
            _ => return instruction,
        };
    }
}

fn direct(at: AnalysisPoint, location: CodeLocation) -> AnalysisPoint {
    AnalysisPoint { location, ..at }
}

fn trap_flow(kind: Boundary, vector: u8, next: AnalysisPoint, conditional: bool) -> ControlFlow {
    ControlFlow {
        action: ControlAction::Boundary(ControlBoundary::Trap {
            kind,
            target: Some(ControlTarget::Indirect(IndirectTarget::InterruptVector {
                vector,
                table_offset: u16::from(vector) * 4,
            })),
            resume: Some(next),
        }),
        fallthrough: conditional.then_some(next),
        delay_slot: None,
    }
}

fn control_flow(instruction: &Instruction, at: AnalysisPoint, next: AnalysisPoint) -> ControlFlow {
    match instruction {
        Instruction::Rep(_)
        | Instruction::Repne(_)
        | Instruction::Repc(_)
        | Instruction::Repnc(_) => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Direct(direct(next, at.location)),
            },
            fallthrough: Some(next),
            delay_slot: None,
        },
        Instruction::BusLock(inner)
        | Instruction::SegmentOverride {
            instruction: inner, ..
        } => control_flow(inner, at, next),
        Instruction::Jcc { target, .. } | Instruction::Loop { target, .. } => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Direct(direct(
                    next,
                    relative_target(next.location, i32::from(*target)),
                )),
            },
            fallthrough: Some(next),
            delay_slot: None,
        },
        Instruction::Jmp { target } => ControlFlow {
            action: ControlAction::Transfer {
                target: jump_target(target, next),
            },
            fallthrough: None,
            delay_slot: None,
        },
        Instruction::Call { target } => ControlFlow {
            action: ControlAction::LinkedTransfer {
                target: call_target(target, next),
                return_site: next,
            },
            fallthrough: None,
            delay_slot: None,
        },
        Instruction::Ret { .. } => ControlFlow {
            action: ControlAction::Return {
                target: IndirectTarget::NearReturn,
            },
            fallthrough: None,
            delay_slot: None,
        },
        Instruction::Retf { .. } => ControlFlow {
            action: ControlAction::Return {
                target: IndirectTarget::FarReturn,
            },
            fallthrough: None,
            delay_slot: None,
        },
        Instruction::Iret => ControlFlow {
            action: ControlAction::ExceptionReturn {
                target: IndirectTarget::InterruptReturn,
            },
            fallthrough: None,
            delay_slot: None,
        },
        Instruction::Int { vector } => {
            trap_flow(Boundary::Interrupt(*vector), *vector, next, false)
        }
        Instruction::Into => trap_flow(Boundary::Overflow, 4, next, true),
        Instruction::Bound { .. } => trap_flow(Boundary::BoundRange, 5, next, true),
        Instruction::Div { .. } | Instruction::Idiv { .. } => {
            trap_flow(Boundary::DivideError, 0, next, true)
        }
        Instruction::Aam { base } if *base == 0 => trap_flow(Boundary::DivideError, 0, next, false),
        Instruction::Hlt => ControlFlow {
            action: ControlAction::Boundary(ControlBoundary::Wait {
                kind: Boundary::Halt,
                resume: next,
            }),
            fallthrough: None,
            delay_slot: None,
        },
        Instruction::Wait => ControlFlow {
            action: ControlAction::Boundary(ControlBoundary::Wait {
                kind: Boundary::NumericProcessorWait,
                resume: next,
            }),
            fallthrough: None,
            delay_slot: None,
        },
        Instruction::Brkem { vector } => ControlFlow {
            action: ControlAction::Boundary(ControlBoundary::ProfileExit {
                kind: Boundary::EmulationMode(*vector),
                target: Some(ControlTarget::Indirect(IndirectTarget::EmulationVector(
                    *vector,
                ))),
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

fn jump_target(
    target: &JmpTarget,
    next: AnalysisPoint,
) -> ControlTarget<AnalysisPoint, IndirectTarget> {
    match target {
        JmpTarget::Rel8(displacement) => ControlTarget::Direct(direct(
            next,
            relative_target(next.location, i32::from(*displacement)),
        )),
        JmpTarget::Rel16(displacement) => ControlTarget::Direct(direct(
            next,
            relative_target(next.location, i32::from(*displacement)),
        )),
        JmpTarget::Far { seg, off } => ControlTarget::Direct(direct(
            next,
            CodeLocation {
                seg: *seg,
                off: *off,
            },
        )),
        JmpTarget::IndirectNear(operand) => {
            ControlTarget::Indirect(IndirectTarget::Near(operand.clone()))
        }
        JmpTarget::IndirectFar(memory) => ControlTarget::Indirect(IndirectTarget::Far(*memory)),
    }
}

fn call_target(
    target: &CallTarget,
    next: AnalysisPoint,
) -> ControlTarget<AnalysisPoint, IndirectTarget> {
    match target {
        CallTarget::Rel16(displacement) => ControlTarget::Direct(direct(
            next,
            relative_target(next.location, i32::from(*displacement)),
        )),
        CallTarget::Far { seg, off } => ControlTarget::Direct(direct(
            next,
            CodeLocation {
                seg: *seg,
                off: *off,
            },
        )),
        CallTarget::IndirectNear(operand) => {
            ControlTarget::Indirect(IndirectTarget::Near(operand.clone()))
        }
        CallTarget::IndirectFar(memory) => ControlTarget::Indirect(IndirectTarget::Far(*memory)),
    }
}

#[allow(clippy::too_many_arguments)]
fn instruction_accesses(
    instruction: &Instruction,
    at: AnalysisPoint,
    segment_override: Option<SegmentRegister>,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    use Instruction::*;
    match instruction {
        SegmentOverride {
            segment,
            instruction,
        } => instruction_accesses(
            instruction,
            at,
            Some(*segment),
            occurrence,
            accesses,
            references,
        ),
        BusLock(instruction) => {
            access(accesses, Location::BusLock, AccessKind::Write, occurrence);
            instruction_accesses(
                instruction,
                at,
                segment_override,
                occurrence,
                accesses,
                references,
            );
        }
        Rep(instruction) => repeat_accesses(
            instruction,
            RepeatCondition::ZeroSetForCompare,
            at,
            segment_override,
            accesses,
            references,
        ),
        Repne(instruction) => repeat_accesses(
            instruction,
            RepeatCondition::ZeroClear,
            at,
            segment_override,
            accesses,
            references,
        ),
        Repc(instruction) => repeat_accesses(
            instruction,
            RepeatCondition::CarrySet,
            at,
            segment_override,
            accesses,
            references,
        ),
        Repnc(instruction) => repeat_accesses(
            instruction,
            RepeatCondition::CarryClear,
            at,
            segment_override,
            accesses,
            references,
        ),
        Mov { dest, src } => {
            read_operand(src, at, segment_override, occurrence, accesses, references);
            write_operand(dest, at, segment_override, occurrence, accesses, references);
        }
        Lea { dest, src } => {
            offset_inputs(*src, occurrence, accesses);
            add_direct_reference(
                src,
                at,
                segment_override,
                ReferenceKind::Address,
                references,
            );
            write_register16(accesses, *dest, occurrence);
        }
        Les { dest, src } | Lds { dest, src } => {
            read_memory_width(
                *src,
                4,
                at,
                segment_override,
                occurrence,
                accesses,
                references,
            );
            write_register16(accesses, *dest, occurrence);
            write(
                accesses,
                Location::SegmentRegister(if matches!(instruction, Les { .. }) {
                    SegmentRegister::ES
                } else {
                    SegmentRegister::DS
                }),
                occurrence,
            );
        }
        Xchg { a, b } => {
            read_write_operand(a, at, segment_override, occurrence, accesses, references);
            read_write_operand(b, at, segment_override, occurrence, accesses, references);
        }
        Push { src } => {
            read_operand(src, at, segment_override, occurrence, accesses, references);
            push_stack(2, occurrence, accesses);
        }
        Pop { dest } => {
            pop_stack(2, occurrence, accesses);
            write_operand(dest, at, segment_override, occurrence, accesses, references);
        }
        Pusha => {
            for register in all_register16() {
                read_register16(accesses, register, occurrence);
            }
            push_stack(16, occurrence, accesses);
        }
        Popa => {
            pop_stack(16, occurrence, accesses);
            for register in [
                Register16::AX,
                Register16::CX,
                Register16::DX,
                Register16::BX,
                Register16::BP,
                Register16::SI,
                Register16::DI,
            ] {
                write_register16(accesses, register, occurrence);
            }
        }
        Bound { index, bounds } => {
            read_register16(accesses, *index, occurrence);
            read_memory_width(
                *bounds,
                4,
                at,
                segment_override,
                occurrence,
                accesses,
                references,
            );
            exception_accesses(5, AccessOccurrence::Conditional, accesses, references);
        }
        Add { dest, src }
        | Sub { dest, src }
        | And { dest, src }
        | Or { dest, src }
        | Xor { dest, src } => {
            read_operand(src, at, segment_override, occurrence, accesses, references);
            read_write_operand(dest, at, segment_override, occurrence, accesses, references);
            if matches!(instruction, And { .. } | Or { .. } | Xor { .. }) {
                write_logic_flags(accesses, occurrence);
            } else {
                write_arithmetic_flags(accesses, occurrence);
            }
        }
        Adc { dest, src } | Sbb { dest, src } => {
            read_flag(accesses, Flag::Carry, occurrence);
            read_operand(src, at, segment_override, occurrence, accesses, references);
            read_write_operand(dest, at, segment_override, occurrence, accesses, references);
            write_arithmetic_flags(accesses, occurrence);
        }
        Cmp { a, b } => {
            read_operand(a, at, segment_override, occurrence, accesses, references);
            read_operand(b, at, segment_override, occurrence, accesses, references);
            write_arithmetic_flags(accesses, occurrence);
        }
        Test { a, b } => {
            read_operand(a, at, segment_override, occurrence, accesses, references);
            read_operand(b, at, segment_override, occurrence, accesses, references);
            write_logic_flags(accesses, occurrence);
        }
        Inc { dest } | Dec { dest } => {
            read_write_operand(dest, at, segment_override, occurrence, accesses, references);
            write_inc_dec_flags(accesses, occurrence);
        }
        Neg { dest } => {
            read_write_operand(dest, at, segment_override, occurrence, accesses, references);
            write_arithmetic_flags(accesses, occurrence);
        }
        Not { dest } => {
            read_write_operand(dest, at, segment_override, occurrence, accesses, references)
        }
        Mul { src } | Imul { src } => {
            read_operand(src, at, segment_override, occurrence, accesses, references);
            implicit_multiply(src, occurrence, accesses);
            write_arithmetic_flags(accesses, occurrence);
        }
        ImulImmediate { dest, src, .. } => {
            read_operand(src, at, segment_override, occurrence, accesses, references);
            write_register16(accesses, *dest, occurrence);
            write_arithmetic_flags(accesses, occurrence);
        }
        Div { src } | Idiv { src } => {
            read_operand(src, at, segment_override, occurrence, accesses, references);
            implicit_divide(src, occurrence, accesses);
            write_arithmetic_flags(accesses, AccessOccurrence::Conditional);
            exception_accesses(0, AccessOccurrence::Conditional, accesses, references);
        }
        Shl { dest, count }
        | Shr { dest, count }
        | Sar { dest, count }
        | Rol { dest, count }
        | Ror { dest, count }
        | Rcl { dest, count }
        | Rcr { dest, count } => {
            let effect = shift_occurrence(*count, occurrence);
            if matches!(count, ShiftCount::Cl) {
                read_register8(accesses, Register8::CL, occurrence);
            }
            if matches!(instruction, Rcl { .. } | Rcr { .. }) {
                read_flag(accesses, Flag::Carry, effect);
            }
            read_operand(dest, at, segment_override, occurrence, accesses, references);
            write_operand(dest, at, segment_override, effect, accesses, references);
            write_shift_flags(accesses, effect);
        }
        Jcc { cond, .. } => read_condition(accesses, *cond, occurrence),
        Loop { condition, .. } => {
            read_register16(accesses, Register16::CX, occurrence);
            write_register16(accesses, Register16::CX, occurrence);
            if matches!(condition, LoopCondition::Equal | LoopCondition::NotEqual) {
                read_flag(accesses, Flag::Zero, occurrence);
            }
        }
        Jmp { target } => read_jump_target(
            target,
            at,
            segment_override,
            occurrence,
            accesses,
            references,
        ),
        Call { target } => {
            read_call_target(
                target,
                at,
                segment_override,
                occurrence,
                accesses,
                references,
            );
            push_stack(
                if matches!(target, CallTarget::Far { .. } | CallTarget::IndirectFar(_)) {
                    4
                } else {
                    2
                },
                occurrence,
                accesses,
            );
        }
        Ret { .. } => pop_stack(2, occurrence, accesses),
        Retf { .. } => pop_stack(4, occurrence, accesses),
        Int { vector } => exception_accesses(*vector, occurrence, accesses, references),
        Into => {
            read_flag(accesses, Flag::Overflow, occurrence);
            exception_accesses(4, AccessOccurrence::Conditional, accesses, references);
        }
        Iret => {
            pop_stack(6, occurrence, accesses);
            write(
                accesses,
                Location::SegmentRegister(SegmentRegister::CS),
                occurrence,
            );
            write(accesses, Location::Flags, occurrence);
        }
        InAl { port } => {
            read_port(*port, 1, occurrence, accesses);
            write_register8(accesses, Register8::AL, occurrence);
        }
        InAx { port } => {
            read_port(*port, 2, occurrence, accesses);
            write_register16(accesses, Register16::AX, occurrence);
        }
        OutAl { port } => {
            read_register8(accesses, Register8::AL, occurrence);
            write_port(*port, 1, occurrence, accesses);
        }
        OutAx { port } => {
            read_register16(accesses, Register16::AX, occurrence);
            write_port(*port, 2, occurrence, accesses);
        }
        Insb => string_input(1, occurrence, accesses),
        Insw => string_input(2, occurrence, accesses),
        Outsb => string_output(1, at, segment_override, occurrence, accesses, references),
        Outsw => string_output(2, at, segment_override, occurrence, accesses, references),
        Movsb => string_move(1, at, segment_override, occurrence, accesses, references),
        Movsw => string_move(2, at, segment_override, occurrence, accesses, references),
        Lodsb => string_load(1, at, segment_override, occurrence, accesses, references),
        Lodsw => string_load(2, at, segment_override, occurrence, accesses, references),
        Stosb => string_store(1, occurrence, accesses),
        Stosw => string_store(2, occurrence, accesses),
        Cmpsb => string_compare(1, at, segment_override, occurrence, accesses, references),
        Cmpsw => string_compare(2, at, segment_override, occurrence, accesses, references),
        Scasb => string_scan(1, occurrence, accesses),
        Scasw => string_scan(2, occurrence, accesses),
        Nop | Hlt | Wait | Brkem { .. } => {}
        Xlat => {
            read_register16(accesses, Register16::BX, occurrence);
            read_register8(accesses, Register8::AL, occurrence);
            read(
                accesses,
                Location::SegmentRegister(segment_override.unwrap_or(SegmentRegister::DS)),
                occurrence,
            );
            read(
                accesses,
                Location::Memory(MemoryLocation {
                    segment: segment_selection(segment_override.unwrap_or(SegmentRegister::DS), at),
                    offset: OffsetAddress::Xlat,
                    byte_len: 1,
                }),
                occurrence,
            );
            write_register8(accesses, Register8::AL, occurrence);
        }
        Pushf => {
            read(accesses, Location::Flags, occurrence);
            push_stack(2, occurrence, accesses);
        }
        Popf => {
            pop_stack(2, occurrence, accesses);
            write(accesses, Location::Flags, occurrence);
        }
        Sahf => {
            read_register8(accesses, Register8::AH, occurrence);
            for flag in [
                Flag::Sign,
                Flag::Zero,
                Flag::AuxiliaryCarry,
                Flag::Parity,
                Flag::Carry,
            ] {
                write_flag(accesses, flag, occurrence);
            }
        }
        Lahf => {
            for flag in [
                Flag::Sign,
                Flag::Zero,
                Flag::AuxiliaryCarry,
                Flag::Parity,
                Flag::Carry,
            ] {
                read_flag(accesses, flag, occurrence);
            }
            write_register8(accesses, Register8::AH, occurrence);
        }
        Cmc => {
            read_flag(accesses, Flag::Carry, occurrence);
            write_flag(accesses, Flag::Carry, occurrence);
        }
        Clc | Stc => write_flag(accesses, Flag::Carry, occurrence),
        Cli | Sti => write_flag(accesses, Flag::InterruptEnable, occurrence),
        Cld | Std => write_flag(accesses, Flag::Direction, occurrence),
        Cbw => {
            read_register8(accesses, Register8::AL, occurrence);
            write_register16(accesses, Register16::AX, occurrence);
        }
        Cwd => {
            read_register16(accesses, Register16::AX, occurrence);
            write_register16(accesses, Register16::DX, occurrence);
        }
        Aaa | Aas => {
            read_register16(accesses, Register16::AX, occurrence);
            write_register16(accesses, Register16::AX, occurrence);
            read_flag(accesses, Flag::AuxiliaryCarry, occurrence);
            read_flag(accesses, Flag::Carry, occurrence);
            write_arithmetic_flags(accesses, occurrence);
        }
        Daa | Das => {
            read_write_register8(accesses, Register8::AL, occurrence);
            read_flag(accesses, Flag::AuxiliaryCarry, occurrence);
            read_flag(accesses, Flag::Carry, occurrence);
            write_arithmetic_flags(accesses, occurrence);
        }
        Aam { base } => {
            read_register8(accesses, Register8::AL, occurrence);
            if *base == 0 {
                exception_accesses(0, occurrence, accesses, references);
            } else {
                write_register16(accesses, Register16::AX, occurrence);
                write_logic_flags(accesses, occurrence);
            }
        }
        Aad { .. } => {
            read_register16(accesses, Register16::AX, occurrence);
            write_register16(accesses, Register16::AX, occurrence);
            write_logic_flags(accesses, occurrence);
        }
        Enter {
            frame_size: _,
            nesting_level,
        } => enter(*nesting_level, occurrence, accesses),
        Leave => {
            read_register16(accesses, Register16::BP, occurrence);
            write_register16(accesses, Register16::SP, occurrence);
            pop_stack(2, occurrence, accesses);
            write_register16(accesses, Register16::BP, occurrence);
        }
        Esc { operation, operand } => external_processor(
            ExternalProcessorFamily::Escape,
            operation.value(),
            operand,
            at,
            segment_override,
            occurrence,
            accesses,
        ),
        Fpo2 { operation, operand } => external_processor(
            ExternalProcessorFamily::Fpo2,
            operation.value(),
            operand,
            at,
            segment_override,
            occurrence,
            accesses,
        ),
        Add4s | Sub4s | Cmp4s => {
            packed_bcd_string(
                !matches!(instruction, Cmp4s),
                at,
                segment_override,
                occurrence,
                accesses,
            );
            write_arithmetic_flags(accesses, occurrence);
        }
        Rol4 { rm } | Ror4 { rm } => {
            read_write_register8(accesses, Register8::AL, occurrence);
            read_write_operand(rm, at, segment_override, occurrence, accesses, references);
        }
        Ins { bit_offset, length } | Ext { bit_offset, length } => {
            for register in [*bit_offset, *length] {
                read_write_register8(accesses, register, occurrence);
            }
            read_write_register16(accesses, Register16::AX, occurrence);
        }
        InsImm { bit_offset, .. } | ExtImm { bit_offset, .. } => {
            read_write_register8(accesses, *bit_offset, occurrence);
            read_write_register16(accesses, Register16::AX, occurrence);
        }
        Test1 { target, bit }
        | Clr1 { target, bit }
        | Set1 { target, bit }
        | Not1 { target, bit } => {
            if matches!(bit, BitIndex::Cl) {
                read_register8(accesses, Register8::CL, occurrence);
            }
            read_operand(
                target,
                at,
                segment_override,
                occurrence,
                accesses,
                references,
            );
            if !matches!(instruction, Test1 { .. }) {
                write_operand(
                    target,
                    at,
                    segment_override,
                    occurrence,
                    accesses,
                    references,
                );
            }
            write_flag(accesses, Flag::Zero, occurrence);
        }
        Unknown { .. } => unreachable!("public static semantics rejects raw bytes during encoding"),
    }
}

#[derive(Clone, Copy)]
enum RepeatCondition {
    ZeroSetForCompare,
    ZeroClear,
    CarrySet,
    CarryClear,
}

fn repeat_accesses(
    instruction: &Instruction,
    condition: RepeatCondition,
    at: AnalysisPoint,
    segment_override: Option<SegmentRegister>,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    read_register16(accesses, Register16::CX, AccessOccurrence::Always);
    write_register16(accesses, Register16::CX, AccessOccurrence::Conditional);
    match condition {
        RepeatCondition::ZeroSetForCompare
            if matches!(
                instruction,
                Instruction::Cmpsb | Instruction::Cmpsw | Instruction::Scasb | Instruction::Scasw
            ) =>
        {
            read_flag(accesses, Flag::Zero, AccessOccurrence::Always);
        }
        RepeatCondition::ZeroClear => {
            read_flag(accesses, Flag::Zero, AccessOccurrence::Always);
        }
        RepeatCondition::CarrySet | RepeatCondition::CarryClear => {
            read_flag(accesses, Flag::Carry, AccessOccurrence::Always);
        }
        RepeatCondition::ZeroSetForCompare => {}
    }
    instruction_accesses(
        instruction,
        at,
        segment_override,
        AccessOccurrence::Conditional,
        accesses,
        references,
    );
}

fn operand_size(operand: &Operand) -> Option<u8> {
    match operand {
        Operand::Reg8(_) | Operand::Imm8(_) => Some(1),
        Operand::Reg16(_) | Operand::Sreg(_) | Operand::Imm16(_) => Some(2),
        Operand::Mem(memory) => Some(size_bytes(memory.size())),
        Operand::Rel8(_) | Operand::Rel16(_) | Operand::Far { .. } => None,
    }
}

fn size_bytes(size: OperandSize) -> u8 {
    match size {
        OperandSize::Byte => 1,
        OperandSize::Word => 2,
    }
}

fn read_operand(
    operand: &Operand,
    at: AnalysisPoint,
    segment_override: Option<SegmentRegister>,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    match operand {
        Operand::Reg8(register) => read_register8(accesses, *register, occurrence),
        Operand::Reg16(register) => read_register16(accesses, *register, occurrence),
        Operand::Sreg(register) => read(accesses, Location::SegmentRegister(*register), occurrence),
        Operand::Mem(memory) => read_memory_width(
            *memory,
            size_bytes(memory.size()),
            at,
            segment_override,
            occurrence,
            accesses,
            references,
        ),
        Operand::Imm8(_)
        | Operand::Imm16(_)
        | Operand::Rel8(_)
        | Operand::Rel16(_)
        | Operand::Far { .. } => {}
    }
}

fn write_operand(
    operand: &Operand,
    at: AnalysisPoint,
    segment_override: Option<SegmentRegister>,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    match operand {
        Operand::Reg8(register) => write_register8(accesses, *register, occurrence),
        Operand::Reg16(register) => write_register16(accesses, *register, occurrence),
        Operand::Sreg(register) => {
            write(accesses, Location::SegmentRegister(*register), occurrence)
        }
        Operand::Mem(memory) => write_memory_width(
            *memory,
            size_bytes(memory.size()),
            at,
            segment_override,
            occurrence,
            accesses,
            references,
        ),
        Operand::Imm8(_)
        | Operand::Imm16(_)
        | Operand::Rel8(_)
        | Operand::Rel16(_)
        | Operand::Far { .. } => {}
    }
}

fn read_write_operand(
    operand: &Operand,
    at: AnalysisPoint,
    segment_override: Option<SegmentRegister>,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    read_operand(
        operand,
        at,
        segment_override,
        occurrence,
        accesses,
        references,
    );
    write_operand(
        operand,
        at,
        segment_override,
        occurrence,
        accesses,
        references,
    );
}

fn default_segment(base: EffectiveAddressBase) -> SegmentRegister {
    match base {
        EffectiveAddressBase::BpSi | EffectiveAddressBase::BpDi | EffectiveAddressBase::Bp => {
            SegmentRegister::SS
        }
        _ => SegmentRegister::DS,
    }
}

fn segment_selection(segment: SegmentRegister, at: AnalysisPoint) -> SegmentSelection {
    if segment == SegmentRegister::CS {
        SegmentSelection::CurrentCode(at.location.seg)
    } else {
        SegmentSelection::Register(segment)
    }
}

fn memory_location(
    memory: EffectiveAddress,
    byte_len: u8,
    at: AnalysisPoint,
    segment_override: Option<SegmentRegister>,
) -> MemoryLocation {
    let segment = memory
        .segment()
        .or(segment_override)
        .unwrap_or_else(|| default_segment(memory.base()));
    let offset = match memory.displacement() {
        crate::EffectiveAddressDisplacement::Absolute(address) => OffsetAddress::Direct(address),
        crate::EffectiveAddressDisplacement::Signed(displacement) => OffsetAddress::Effective {
            base: memory.base(),
            displacement,
        },
    };
    MemoryLocation {
        segment: segment_selection(segment, at),
        offset,
        byte_len,
    }
}

fn address_inputs(
    memory: EffectiveAddress,
    at: AnalysisPoint,
    segment_override: Option<SegmentRegister>,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    let segment = memory
        .segment()
        .or(segment_override)
        .unwrap_or_else(|| default_segment(memory.base()));
    read(accesses, Location::SegmentRegister(segment), occurrence);
    for register in base_registers(memory.base()) {
        read_register16(accesses, *register, occurrence);
    }
    let _ = at;
}

fn offset_inputs(
    memory: EffectiveAddress,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    for register in base_registers(memory.base()) {
        read_register16(accesses, *register, occurrence);
    }
}

fn base_registers(base: EffectiveAddressBase) -> &'static [Register16] {
    match base {
        EffectiveAddressBase::BxSi => &[Register16::BX, Register16::SI],
        EffectiveAddressBase::BxDi => &[Register16::BX, Register16::DI],
        EffectiveAddressBase::BpSi => &[Register16::BP, Register16::SI],
        EffectiveAddressBase::BpDi => &[Register16::BP, Register16::DI],
        EffectiveAddressBase::Si => &[Register16::SI],
        EffectiveAddressBase::Di => &[Register16::DI],
        EffectiveAddressBase::Bp => &[Register16::BP],
        EffectiveAddressBase::Bx => &[Register16::BX],
        EffectiveAddressBase::Direct => &[],
    }
}

#[allow(clippy::too_many_arguments)]
fn read_memory_width(
    memory: EffectiveAddress,
    byte_len: u8,
    at: AnalysisPoint,
    segment_override: Option<SegmentRegister>,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    address_inputs(memory, at, segment_override, occurrence, accesses);
    let location = memory_location(memory, byte_len, at, segment_override);
    read(accesses, Location::Memory(location), occurrence);
    add_memory_reference(location, ReferenceKind::Read, occurrence, references);
}

#[allow(clippy::too_many_arguments)]
fn write_memory_width(
    memory: EffectiveAddress,
    byte_len: u8,
    at: AnalysisPoint,
    segment_override: Option<SegmentRegister>,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    address_inputs(memory, at, segment_override, occurrence, accesses);
    let location = memory_location(memory, byte_len, at, segment_override);
    write(accesses, Location::Memory(location), occurrence);
    add_memory_reference(location, ReferenceKind::Write, occurrence, references);
}

fn add_direct_reference(
    memory: &EffectiveAddress,
    at: AnalysisPoint,
    segment_override: Option<SegmentRegister>,
    kind: ReferenceKind,
    references: &mut Vec<StaticReference<Reference>>,
) {
    let location = memory_location(*memory, size_bytes(memory.size()), at, segment_override);
    let OffsetAddress::Direct(offset) = location.offset else {
        return;
    };
    let target = if kind == ReferenceKind::Address {
        Reference::Offset(offset)
    } else {
        Reference::Memory {
            segment: location.segment,
            offset,
            byte_len: location.byte_len,
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

fn add_memory_reference(
    location: MemoryLocation,
    kind: ReferenceKind,
    occurrence: AccessOccurrence,
    references: &mut Vec<StaticReference<Reference>>,
) {
    let OffsetAddress::Direct(offset) = location.offset else {
        return;
    };
    add_reference(
        references,
        StaticReference {
            target: Reference::Memory {
                segment: location.segment,
                offset,
                byte_len: location.byte_len,
            },
            kind,
            occurrence,
        },
    );
}

fn implicit_multiply(
    operand: &Operand,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    if operand_size(operand) == Some(1) {
        read_register8(accesses, Register8::AL, occurrence);
        write_register16(accesses, Register16::AX, occurrence);
    } else {
        read_register16(accesses, Register16::AX, occurrence);
        write_register16(accesses, Register16::AX, occurrence);
        write_register16(accesses, Register16::DX, occurrence);
    }
}

fn implicit_divide(
    operand: &Operand,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_register16(accesses, Register16::AX, occurrence);
    if operand_size(operand) == Some(2) {
        read_register16(accesses, Register16::DX, occurrence);
        write_register16(accesses, Register16::DX, AccessOccurrence::Conditional);
    }
    write_register16(accesses, Register16::AX, AccessOccurrence::Conditional);
}

fn shift_occurrence(count: ShiftCount, parent: AccessOccurrence) -> AccessOccurrence {
    if parent == AccessOccurrence::Conditional {
        return parent;
    }
    match count {
        ShiftCount::One => AccessOccurrence::Always,
        ShiftCount::Imm8(value) if value != 0 => AccessOccurrence::Always,
        ShiftCount::Imm8(_) | ShiftCount::Cl => AccessOccurrence::Conditional,
    }
}

fn read_jump_target(
    target: &JmpTarget,
    at: AnalysisPoint,
    segment_override: Option<SegmentRegister>,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    match target {
        JmpTarget::IndirectNear(operand) => read_operand(
            operand,
            at,
            segment_override,
            occurrence,
            accesses,
            references,
        ),
        JmpTarget::IndirectFar(memory) => read_memory_width(
            *memory,
            4,
            at,
            segment_override,
            occurrence,
            accesses,
            references,
        ),
        _ => {}
    }
}

fn read_call_target(
    target: &CallTarget,
    at: AnalysisPoint,
    segment_override: Option<SegmentRegister>,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    match target {
        CallTarget::IndirectNear(operand) => read_operand(
            operand,
            at,
            segment_override,
            occurrence,
            accesses,
            references,
        ),
        CallTarget::IndirectFar(memory) => read_memory_width(
            *memory,
            4,
            at,
            segment_override,
            occurrence,
            accesses,
            references,
        ),
        _ => {}
    }
}

fn stack_memory(byte_len: u8) -> MemoryLocation {
    MemoryLocation {
        segment: SegmentSelection::Register(SegmentRegister::SS),
        offset: OffsetAddress::Stack,
        byte_len,
    }
}

fn push_stack(
    byte_len: u8,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read(
        accesses,
        Location::SegmentRegister(SegmentRegister::SS),
        occurrence,
    );
    read_register16(accesses, Register16::SP, occurrence);
    write_register16(accesses, Register16::SP, occurrence);
    write(
        accesses,
        Location::Memory(stack_memory(byte_len)),
        occurrence,
    );
}

fn pop_stack(
    byte_len: u8,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read(
        accesses,
        Location::SegmentRegister(SegmentRegister::SS),
        occurrence,
    );
    read_register16(accesses, Register16::SP, occurrence);
    read(
        accesses,
        Location::Memory(stack_memory(byte_len)),
        occurrence,
    );
    write_register16(accesses, Register16::SP, occurrence);
}

fn exception_accesses(
    vector: u8,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    read(accesses, Location::Flags, occurrence);
    read(
        accesses,
        Location::SegmentRegister(SegmentRegister::CS),
        occurrence,
    );
    push_stack(6, occurrence, accesses);
    for flag in [Flag::Trap, Flag::InterruptEnable] {
        write_flag(accesses, flag, occurrence);
    }
    let location = MemoryLocation {
        segment: SegmentSelection::Zero,
        offset: OffsetAddress::Direct(u16::from(vector) * 4),
        byte_len: 4,
    };
    read(accesses, Location::Memory(location), occurrence);
    add_memory_reference(location, ReferenceKind::Read, occurrence, references);
}

fn read_port(
    port: PortAddress,
    byte_len: u8,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    let port = port_location(port, occurrence, accesses);
    read(accesses, Location::Port { port, byte_len }, occurrence);
}

fn write_port(
    port: PortAddress,
    byte_len: u8,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    let port = port_location(port, occurrence, accesses);
    write(accesses, Location::Port { port, byte_len }, occurrence);
}

fn port_location(
    port: PortAddress,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) -> PortLocation {
    match port {
        PortAddress::Imm8(port) => PortLocation::Immediate(port),
        PortAddress::Dx => {
            read_register16(accesses, Register16::DX, occurrence);
            PortLocation::DataRegisterDx
        }
    }
}

fn string_memory(segment: SegmentSelection, index: Register16, byte_len: u8) -> MemoryLocation {
    MemoryLocation {
        segment,
        offset: OffsetAddress::Register(index),
        byte_len,
    }
}

fn string_source_segment(
    at: AnalysisPoint,
    segment_override: Option<SegmentRegister>,
) -> SegmentSelection {
    segment_selection(segment_override.unwrap_or(SegmentRegister::DS), at)
}

fn string_index(
    register: Register16,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_register16(accesses, register, occurrence);
    write_register16(accesses, register, occurrence);
    read_flag(accesses, Flag::Direction, occurrence);
}

fn string_input(
    byte_len: u8,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_register16(accesses, Register16::DX, occurrence);
    read(
        accesses,
        Location::Port {
            port: PortLocation::DataRegisterDx,
            byte_len,
        },
        occurrence,
    );
    read(
        accesses,
        Location::SegmentRegister(SegmentRegister::ES),
        occurrence,
    );
    write(
        accesses,
        Location::Memory(string_memory(
            SegmentSelection::Register(SegmentRegister::ES),
            Register16::DI,
            byte_len,
        )),
        occurrence,
    );
    string_index(Register16::DI, occurrence, accesses);
}

fn string_output(
    byte_len: u8,
    at: AnalysisPoint,
    segment_override: Option<SegmentRegister>,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
    _references: &mut Vec<StaticReference<Reference>>,
) {
    let segment = segment_override.unwrap_or(SegmentRegister::DS);
    read(accesses, Location::SegmentRegister(segment), occurrence);
    read(
        accesses,
        Location::Memory(string_memory(
            string_source_segment(at, segment_override),
            Register16::SI,
            byte_len,
        )),
        occurrence,
    );
    read_register16(accesses, Register16::DX, occurrence);
    write(
        accesses,
        Location::Port {
            port: PortLocation::DataRegisterDx,
            byte_len,
        },
        occurrence,
    );
    string_index(Register16::SI, occurrence, accesses);
}

fn string_move(
    byte_len: u8,
    at: AnalysisPoint,
    segment_override: Option<SegmentRegister>,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
    _references: &mut Vec<StaticReference<Reference>>,
) {
    let source_segment = segment_override.unwrap_or(SegmentRegister::DS);
    read(
        accesses,
        Location::SegmentRegister(source_segment),
        occurrence,
    );
    read(
        accesses,
        Location::SegmentRegister(SegmentRegister::ES),
        occurrence,
    );
    read(
        accesses,
        Location::Memory(string_memory(
            string_source_segment(at, segment_override),
            Register16::SI,
            byte_len,
        )),
        occurrence,
    );
    write(
        accesses,
        Location::Memory(string_memory(
            SegmentSelection::Register(SegmentRegister::ES),
            Register16::DI,
            byte_len,
        )),
        occurrence,
    );
    string_index(Register16::SI, occurrence, accesses);
    string_index(Register16::DI, occurrence, accesses);
}

fn string_load(
    byte_len: u8,
    at: AnalysisPoint,
    segment_override: Option<SegmentRegister>,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
    _references: &mut Vec<StaticReference<Reference>>,
) {
    let segment = segment_override.unwrap_or(SegmentRegister::DS);
    read(accesses, Location::SegmentRegister(segment), occurrence);
    read(
        accesses,
        Location::Memory(string_memory(
            string_source_segment(at, segment_override),
            Register16::SI,
            byte_len,
        )),
        occurrence,
    );
    if byte_len == 1 {
        write_register8(accesses, Register8::AL, occurrence);
    } else {
        write_register16(accesses, Register16::AX, occurrence);
    }
    string_index(Register16::SI, occurrence, accesses);
}

fn string_store(
    byte_len: u8,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    if byte_len == 1 {
        read_register8(accesses, Register8::AL, occurrence);
    } else {
        read_register16(accesses, Register16::AX, occurrence);
    }
    read(
        accesses,
        Location::SegmentRegister(SegmentRegister::ES),
        occurrence,
    );
    write(
        accesses,
        Location::Memory(string_memory(
            SegmentSelection::Register(SegmentRegister::ES),
            Register16::DI,
            byte_len,
        )),
        occurrence,
    );
    string_index(Register16::DI, occurrence, accesses);
}

fn string_compare(
    byte_len: u8,
    at: AnalysisPoint,
    segment_override: Option<SegmentRegister>,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
    _references: &mut Vec<StaticReference<Reference>>,
) {
    let source_segment = segment_override.unwrap_or(SegmentRegister::DS);
    read(
        accesses,
        Location::SegmentRegister(source_segment),
        occurrence,
    );
    read(
        accesses,
        Location::SegmentRegister(SegmentRegister::ES),
        occurrence,
    );
    read(
        accesses,
        Location::Memory(string_memory(
            string_source_segment(at, segment_override),
            Register16::SI,
            byte_len,
        )),
        occurrence,
    );
    read(
        accesses,
        Location::Memory(string_memory(
            SegmentSelection::Register(SegmentRegister::ES),
            Register16::DI,
            byte_len,
        )),
        occurrence,
    );
    string_index(Register16::SI, occurrence, accesses);
    string_index(Register16::DI, occurrence, accesses);
    write_arithmetic_flags(accesses, occurrence);
}

fn string_scan(
    byte_len: u8,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    if byte_len == 1 {
        read_register8(accesses, Register8::AL, occurrence);
    } else {
        read_register16(accesses, Register16::AX, occurrence);
    }
    read(
        accesses,
        Location::SegmentRegister(SegmentRegister::ES),
        occurrence,
    );
    read(
        accesses,
        Location::Memory(string_memory(
            SegmentSelection::Register(SegmentRegister::ES),
            Register16::DI,
            byte_len,
        )),
        occurrence,
    );
    string_index(Register16::DI, occurrence, accesses);
    write_arithmetic_flags(accesses, occurrence);
}

fn enter(
    nesting_level: u8,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    let level = nesting_level & 31;
    read_register16(accesses, Register16::BP, occurrence);
    push_stack(2, occurrence, accesses);
    if level > 1 {
        read(
            accesses,
            Location::Memory(MemoryLocation {
                segment: SegmentSelection::Register(SegmentRegister::SS),
                offset: OffsetAddress::Effective {
                    base: EffectiveAddressBase::Bp,
                    displacement: -2,
                },
                byte_len: (level - 1).saturating_mul(2),
            }),
            occurrence,
        );
        push_stack((level - 1).saturating_mul(2), occurrence, accesses);
    }
    if level > 0 {
        push_stack(2, occurrence, accesses);
    }
    write_register16(accesses, Register16::BP, occurrence);
    read_register16(accesses, Register16::SP, occurrence);
    write_register16(accesses, Register16::SP, occurrence);
}

fn external_processor(
    family: ExternalProcessorFamily,
    operation: u8,
    operand: &FloatingPointOperand,
    at: AnalysisPoint,
    segment_override: Option<SegmentRegister>,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    if let FloatingPointOperand::Memory(memory) = operand {
        address_inputs(*memory, at, segment_override, occurrence, accesses);
    }
    write(
        accesses,
        Location::ExternalProcessorCommand {
            family,
            operation,
            operand: operand.clone(),
        },
        occurrence,
    );
}

fn packed_bcd_string(
    writes_source: bool,
    at: AnalysisPoint,
    segment_override: Option<SegmentRegister>,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    let source_segment = segment_override.unwrap_or(SegmentRegister::DS);
    read(
        accesses,
        Location::SegmentRegister(source_segment),
        occurrence,
    );
    read(
        accesses,
        Location::SegmentRegister(SegmentRegister::ES),
        occurrence,
    );
    let source = Location::Memory(string_memory(
        string_source_segment(at, segment_override),
        Register16::SI,
        1,
    ));
    let destination = Location::Memory(string_memory(
        SegmentSelection::Register(SegmentRegister::ES),
        Register16::DI,
        1,
    ));
    read(accesses, source.clone(), occurrence);
    read(accesses, destination, occurrence);
    if writes_source {
        write(accesses, source, occurrence);
    }
    for register in [Register16::SI, Register16::DI] {
        string_index(register, occurrence, accesses);
    }
    read_register8(accesses, Register8::CL, occurrence);
}

fn read_condition(
    accesses: &mut Vec<LocationAccess<Location>>,
    condition: Condition,
    occurrence: AccessOccurrence,
) {
    use Condition::*;
    let flags: &[Flag] = match condition {
        O | No => &[Flag::Overflow],
        B | Nb => &[Flag::Carry],
        E | Ne => &[Flag::Zero],
        Be | A => &[Flag::Carry, Flag::Zero],
        S | Ns => &[Flag::Sign],
        P | Np => &[Flag::Parity],
        L | Ge => &[Flag::Sign, Flag::Overflow],
        Le | G => &[Flag::Zero, Flag::Sign, Flag::Overflow],
        Cxz => {
            read_register16(accesses, Register16::CX, occurrence);
            &[]
        }
    };
    for flag in flags {
        read_flag(accesses, *flag, occurrence);
    }
}

fn write_arithmetic_flags(
    accesses: &mut Vec<LocationAccess<Location>>,
    occurrence: AccessOccurrence,
) {
    for flag in [
        Flag::Carry,
        Flag::Parity,
        Flag::AuxiliaryCarry,
        Flag::Zero,
        Flag::Sign,
        Flag::Overflow,
    ] {
        write_flag(accesses, flag, occurrence);
    }
}

fn write_logic_flags(accesses: &mut Vec<LocationAccess<Location>>, occurrence: AccessOccurrence) {
    for flag in [
        Flag::Carry,
        Flag::Parity,
        Flag::Zero,
        Flag::Sign,
        Flag::Overflow,
    ] {
        write_flag(accesses, flag, occurrence);
    }
}

fn write_inc_dec_flags(accesses: &mut Vec<LocationAccess<Location>>, occurrence: AccessOccurrence) {
    for flag in [
        Flag::Parity,
        Flag::AuxiliaryCarry,
        Flag::Zero,
        Flag::Sign,
        Flag::Overflow,
    ] {
        write_flag(accesses, flag, occurrence);
    }
}

fn write_shift_flags(accesses: &mut Vec<LocationAccess<Location>>, occurrence: AccessOccurrence) {
    write_arithmetic_flags(accesses, occurrence);
}

fn all_register16() -> [Register16; 8] {
    [
        Register16::AX,
        Register16::CX,
        Register16::DX,
        Register16::BX,
        Register16::SP,
        Register16::BP,
        Register16::SI,
        Register16::DI,
    ]
}

fn read_register8(
    accesses: &mut Vec<LocationAccess<Location>>,
    register: Register8,
    occurrence: AccessOccurrence,
) {
    read(accesses, Location::Register8(register), occurrence);
}

fn write_register8(
    accesses: &mut Vec<LocationAccess<Location>>,
    register: Register8,
    occurrence: AccessOccurrence,
) {
    write(accesses, Location::Register8(register), occurrence);
}

fn read_write_register8(
    accesses: &mut Vec<LocationAccess<Location>>,
    register: Register8,
    occurrence: AccessOccurrence,
) {
    read_register8(accesses, register, occurrence);
    write_register8(accesses, register, occurrence);
}

fn read_register16(
    accesses: &mut Vec<LocationAccess<Location>>,
    register: Register16,
    occurrence: AccessOccurrence,
) {
    read(accesses, Location::Register16(register), occurrence);
}

fn write_register16(
    accesses: &mut Vec<LocationAccess<Location>>,
    register: Register16,
    occurrence: AccessOccurrence,
) {
    write(accesses, Location::Register16(register), occurrence);
}

fn read_write_register16(
    accesses: &mut Vec<LocationAccess<Location>>,
    register: Register16,
    occurrence: AccessOccurrence,
) {
    read_register16(accesses, register, occurrence);
    write_register16(accesses, register, occurrence);
}

fn read_flag(
    accesses: &mut Vec<LocationAccess<Location>>,
    flag: Flag,
    occurrence: AccessOccurrence,
) {
    read(accesses, Location::Flag(flag), occurrence);
}

fn write_flag(
    accesses: &mut Vec<LocationAccess<Location>>,
    flag: Flag,
    occurrence: AccessOccurrence,
) {
    write(accesses, Location::Flag(flag), occurrence);
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

fn advance(location: CodeLocation, byte_len: usize) -> CodeLocation {
    CodeLocation {
        seg: location.seg,
        off: location.off.wrapping_add(byte_len as u16),
    }
}

fn relative_target(next: CodeLocation, displacement: i32) -> CodeLocation {
    CodeLocation {
        seg: next.seg,
        off: (i32::from(next.off) + displacement) as u16,
    }
}

pub(crate) fn storage_relation(left: &Location, right: &Location) -> StorageRelation {
    use StorageRelation::*;
    if left == right {
        return Same;
    }
    match (left, right) {
        (Location::Register16(word), Location::Register8(byte))
        | (Location::Register8(byte), Location::Register16(word))
            if byte_parent(*byte) == *word =>
        {
            Overlapping
        }
        (Location::Register8(left), Location::Register8(right))
            if byte_parent(*left) == byte_parent(*right) =>
        {
            Disjoint
        }
        (Location::Flags, Location::Flag(_)) | (Location::Flag(_), Location::Flags) => Overlapping,
        (Location::Memory(left), Location::Memory(right)) => memory_relation(*left, *right),
        (Location::Memory(_), _) | (_, Location::Memory(_)) => Unknown,
        (
            Location::Port {
                port: PortLocation::DataRegisterDx,
                ..
            },
            Location::Port { .. },
        )
        | (
            Location::Port { .. },
            Location::Port {
                port: PortLocation::DataRegisterDx,
                ..
            },
        ) => Unknown,
        (Location::Port { .. }, Location::Port { .. }) => Disjoint,
        _ => Disjoint,
    }
}

fn byte_parent(register: Register8) -> Register16 {
    match register.index() & 3 {
        0 => Register16::AX,
        1 => Register16::CX,
        2 => Register16::DX,
        3 => Register16::BX,
        _ => unreachable!(),
    }
}

fn memory_relation(left: MemoryLocation, right: MemoryLocation) -> StorageRelation {
    if left.segment != right.segment {
        return StorageRelation::Unknown;
    }
    let (OffsetAddress::Direct(left_offset), OffsetAddress::Direct(right_offset)) =
        (left.offset, right.offset)
    else {
        return StorageRelation::Unknown;
    };
    let overlaps = (0..u16::from(left.byte_len)).any(|offset| {
        let byte = left_offset.wrapping_add(offset);
        (0..u16::from(right.byte_len)).any(|other| byte == right_offset.wrapping_add(other))
    });
    if overlaps {
        StorageRelation::Overlapping
    } else {
        StorageRelation::Disjoint
    }
}
