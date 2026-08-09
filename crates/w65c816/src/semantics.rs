//! Complete static meaning for the exact W65C816 width profiles.

use core::fmt;

use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlBoundary, ControlTarget, LocationAccess,
    ReferenceKind, StaticReference, StorageRelation,
};

use crate::{
    AddressingMode, CodeLocation, CpuMode, EncodeError, Instruction, Mnemonic, Operand, Width,
    WidthState,
};

/// One placed W65C816 instruction point and its semantic mode context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnalysisPoint {
    location: CodeLocation,
    mode: Option<CpuMode>,
}

impl AnalysisPoint {
    /// Construct a point whose entry mode is statically known.
    pub const fn new(location: CodeLocation, mode: CpuMode) -> Self {
        Self {
            location,
            mode: Some(mode),
        }
    }

    /// Return the architectural code location.
    pub const fn location(self) -> CodeLocation {
        self.location
    }

    /// Return the entry mode, or `None` after a transition whose width state is
    /// not statically recoverable.
    pub const fn mode(self) -> Option<CpuMode> {
        self.mode
    }

    const fn with_mode(self, mode: Option<CpuMode>) -> Self {
        Self {
            location: self.location,
            mode,
        }
    }
}

/// Statically known rule for a runtime-derived W65C816 control destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndirectTarget {
    /// Target loaded through an instruction operand naming a pointer.
    Pointer {
        /// Pointer addressing form.
        mode: AddressingMode,
        /// Encoded pointer-table address.
        pointer: u16,
        /// Whether the loaded target preserves or replaces the program bank.
        bank: IndirectBankRule,
    },
    /// Target restored by a dedicated return instruction.
    StackReturn(ReturnAddressRule),
    /// Handler loaded through a software-interrupt vector.
    Vector {
        /// Software boundary selecting the vector.
        kind: Boundary,
        /// Vector address in bank zero.
        vector: u16,
    },
}

/// Program-bank rule carried by an indirect target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndirectBankRule {
    /// Preserve the current program bank and load a 16-bit program counter.
    PreserveProgramBank,
    /// Load a 24-bit program bank and program counter from the pointer.
    LoadProgramBank,
}

/// Stack-supplied address and mode rule used by a return instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnAddressRule {
    /// RTS restores a 16-bit address in the current program bank and adds one.
    Subroutine,
    /// RTL restores a 24-bit address and adds one.
    LongSubroutine,
    /// RTI restores status and a 16- or 24-bit interrupted address.
    Interrupt {
        /// Entry mode determining whether RTI restores a program-bank byte.
        mode: CpuMode,
    },
}

/// W65C816 synchronous boundary, wait, or stop kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Boundary {
    /// BRK software interrupt.
    Break,
    /// COP software interrupt.
    Coprocessor,
    /// WAI low-power wait state.
    WaitForInterrupt,
    /// STP clock-stop state.
    StopUntilReset,
}

/// Physical accumulator storage selected by an architectural view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccumulatorView {
    /// Low accumulator byte A.
    Low,
    /// High accumulator byte B.
    High,
    /// Full sixteen-bit C accumulator.
    Full,
}

/// Physical register storage or architectural register view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegisterLocation {
    /// Accumulator storage.
    Accumulator(AccumulatorView),
    /// X index register at the selected architectural width.
    X(Width),
    /// Y index register at the selected architectural width.
    Y(Width),
    /// Direct-page register D.
    DirectPage,
    /// Stack pointer S.
    StackPointer,
    /// Data-bank register DBR.
    DataBank,
    /// Program-bank register PBR when explicitly read as data.
    ProgramBank,
}

/// Individually meaningful processor-status storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusLocation {
    /// Negative flag N.
    Negative,
    /// Overflow flag V.
    Overflow,
    /// Accumulator-width selector M.
    AccumulatorWidth,
    /// Index-width selector X.
    IndexWidth,
    /// Decimal-mode flag D.
    Decimal,
    /// Interrupt-disable flag I.
    InterruptDisable,
    /// Zero flag Z.
    Zero,
    /// Carry flag C.
    Carry,
    /// Emulation-mode state E.
    Emulation,
}

/// Symbolic memory address established without general register values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryAddress {
    /// Effective data operand described by its typed addressing form.
    Effective {
        /// Addressing mode used to form the effective address.
        mode: AddressingMode,
        /// Encoded operand contributing to the address.
        operand: Operand,
    },
    /// Pointer table read performed by an indirect addressing form.
    Pointer {
        /// Addressing mode selecting the pointer rule.
        mode: AddressingMode,
        /// Encoded operand locating the pointer table.
        operand: Operand,
    },
    /// Runtime stack storage in bank zero.
    Stack,
    /// Source byte addressed by a block-move operation.
    BlockSource {
        /// Immediate source bank.
        bank: u8,
    },
    /// Destination byte addressed by a block-move operation.
    BlockDestination {
        /// Immediate destination bank.
        bank: u8,
    },
    /// Software-interrupt vector bytes in bank zero.
    Vector {
        /// Low byte address of the two-byte vector.
        address: u16,
    },
}

/// One symbolic W65C816 memory location and its access width.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryLocation {
    address: MemoryAddress,
    byte_len: u8,
}

impl MemoryLocation {
    /// Borrow the symbolic address.
    pub const fn address(&self) -> &MemoryAddress {
        &self.address
    }

    /// Return the non-zero number of bytes accessed.
    pub const fn byte_len(&self) -> u8 {
        self.byte_len
    }
}

/// Architectural W65C816 storage location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Location {
    /// Register storage or view.
    Register(RegisterLocation),
    /// Individually meaningful status storage.
    Status(StatusLocation),
    /// Symbolic memory storage.
    Memory(MemoryLocation),
}

/// Statically resolved non-control W65C816 address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reference {
    /// Fully resolved 24-bit data address and access width.
    Data {
        /// First referenced byte.
        address: u32,
        /// Non-zero referenced byte length.
        byte_len: u8,
    },
    /// Banked code-relative address produced as a value.
    CodeAddress(CodeLocation),
}

/// Structural failure to form sound W65C816 static facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticError {
    /// The point has no width state from which this exact profile can operate.
    UnknownMode,
    /// The point's width state belongs to a different exact profile.
    ProfileWidthMismatch {
        /// Widths fixed by the profile receiving the query.
        profile: WidthState,
        /// Widths carried by the analysis point.
        point: WidthState,
    },
    /// The typed instruction is not executable in the selected exact profile.
    InvalidInstruction(EncodeError),
    /// Sequential placement would cross the current program-bank boundary.
    CrossesProgramBank {
        /// Instruction's architectural location.
        location: CodeLocation,
        /// Complete encoded instruction length.
        byte_len: usize,
    },
}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownMode => write!(f, "W65C816 mode is unknown at the analysis point"),
            Self::ProfileWidthMismatch { profile, point } => write!(
                f,
                "analysis-point widths {point:?} do not match exact profile widths {profile:?}"
            ),
            Self::InvalidInstruction(error) => write!(f, "invalid W65C816 instruction: {error}"),
            Self::CrossesProgramBank { location, byte_len } => write!(
                f,
                "instruction at {:02X}:{:04X} with length {byte_len} crosses the program bank",
                location.bank(),
                location.program_counter()
            ),
        }
    }
}

impl std::error::Error for SemanticError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidInstruction(error) => Some(error),
            _ => None,
        }
    }
}

/// Shared control-flow result specialized with W65C816 profile vocabulary.
pub type ControlFlow = typed_isa_core::ControlFlow<AnalysisPoint, IndirectTarget, Boundary>;

/// Shared instruction-semantics result specialized with W65C816 profile vocabulary.
pub type InstructionSemantics = typed_isa_core::InstructionSemantics<
    AnalysisPoint,
    IndirectTarget,
    Boundary,
    Location,
    Reference,
>;

pub(crate) fn semantics_for<P: crate::W65C816Profile>(
    instruction: &Instruction,
    at: &AnalysisPoint,
) -> Result<InstructionSemantics, SemanticError> {
    let mode = at.mode.ok_or(SemanticError::UnknownMode)?;
    if mode.widths() != P::WIDTHS {
        return Err(SemanticError::ProfileWidthMismatch {
            profile: P::WIDTHS,
            point: mode.widths(),
        });
    }
    let byte_len = instruction
        .encoded_len(P::WIDTHS)
        .map_err(SemanticError::InvalidInstruction)?;
    let next = successor(*at, byte_len)?;

    let mut location_accesses = Vec::new();
    let mut static_references = Vec::new();
    add_instruction_accesses::<P>(
        instruction,
        mode,
        next,
        &mut location_accesses,
        &mut static_references,
    );

    Ok(typed_isa_core::InstructionSemantics {
        control_flow: control_flow(instruction, *at, next, mode),
        location_accesses,
        static_references,
    })
}

fn successor(at: AnalysisPoint, byte_len: usize) -> Result<AnalysisPoint, SemanticError> {
    let Some(program_counter) = at
        .location
        .program_counter()
        .checked_add(u16::try_from(byte_len).expect("W65C816 instruction length fits in u16"))
    else {
        return Err(SemanticError::CrossesProgramBank {
            location: at.location,
            byte_len,
        });
    };
    Ok(AnalysisPoint {
        location: CodeLocation::new(at.location.bank(), program_counter),
        mode: at.mode,
    })
}

fn control_flow(
    instruction: &Instruction,
    at: AnalysisPoint,
    next: AnalysisPoint,
    mode: CpuMode,
) -> ControlFlow {
    use Mnemonic::*;

    let conditional_target = |target| ControlFlow {
        action: ControlAction::Transfer {
            target: ControlTarget::Direct(target),
        },
        fallthrough: Some(next),
        delay_slot: None,
    };
    let transfer = |target| ControlFlow {
        action: ControlAction::Transfer { target },
        fallthrough: None,
        delay_slot: None,
    };
    let linked_transfer = |target| ControlFlow {
        action: ControlAction::LinkedTransfer {
            target,
            return_site: next,
        },
        fallthrough: None,
        delay_slot: None,
    };

    match instruction.mnemonic {
        Bcc | Bcs | Beq | Bmi | Bne | Bpl | Bvc | Bvs => conditional_target(
            relative_target(at, next, &instruction.operand)
                .expect("a validated relative branch has a relative operand"),
        ),
        Bra | Brl => transfer(ControlTarget::Direct(
            relative_target(at, next, &instruction.operand)
                .expect("a validated relative branch has a relative operand"),
        )),
        Jmp | Jml => transfer(control_target(instruction, at)),
        Jsr | Jsl => linked_transfer(control_target(instruction, at)),
        Rts => return_flow(ReturnAddressRule::Subroutine),
        Rtl => return_flow(ReturnAddressRule::LongSubroutine),
        Rti => ControlFlow {
            action: ControlAction::ExceptionReturn {
                target: IndirectTarget::StackReturn(ReturnAddressRule::Interrupt { mode }),
            },
            fallthrough: None,
            delay_slot: None,
        },
        Brk | Cop => {
            let kind = if instruction.mnemonic == Brk {
                Boundary::Break
            } else {
                Boundary::Coprocessor
            };
            let vector = software_vector(kind, mode);
            ControlFlow {
                action: ControlAction::Boundary(ControlBoundary::Trap {
                    kind,
                    target: Some(ControlTarget::Indirect(IndirectTarget::Vector {
                        kind,
                        vector,
                    })),
                    resume: Some(next),
                }),
                fallthrough: None,
                delay_slot: None,
            }
        }
        Wai => ControlFlow {
            action: ControlAction::Boundary(ControlBoundary::Wait {
                kind: Boundary::WaitForInterrupt,
                resume: next,
            }),
            fallthrough: None,
            delay_slot: None,
        },
        Stp => ControlFlow {
            action: ControlAction::Boundary(ControlBoundary::Stop {
                kind: Boundary::StopUntilReset,
            }),
            fallthrough: None,
            delay_slot: None,
        },
        _ => ControlFlow {
            action: ControlAction::Continue,
            fallthrough: Some(next.with_mode(successor_mode(mode, instruction))),
            delay_slot: None,
        },
    }
}

fn return_flow(rule: ReturnAddressRule) -> ControlFlow {
    ControlFlow {
        action: ControlAction::Return {
            target: IndirectTarget::StackReturn(rule),
        },
        fallthrough: None,
        delay_slot: None,
    }
}

fn relative_target(
    at: AnalysisPoint,
    next: AnalysisPoint,
    operand: &Operand,
) -> Option<AnalysisPoint> {
    let displacement = match operand {
        Operand::Relative8(value) => i32::from(*value),
        Operand::Relative16(value) => i32::from(*value),
        _ => return None,
    };
    let target = (i32::from(next.location.program_counter()) + displacement) as u16;
    Some(AnalysisPoint {
        location: CodeLocation::new(at.location.bank(), target),
        mode: at.mode,
    })
}

fn control_target(
    instruction: &Instruction,
    at: AnalysisPoint,
) -> ControlTarget<AnalysisPoint, IndirectTarget> {
    match (&instruction.mode, &instruction.operand) {
        (AddressingMode::Absolute, Operand::Word(target)) => ControlTarget::Direct(AnalysisPoint {
            location: CodeLocation::new(at.location.bank(), *target),
            mode: at.mode,
        }),
        (AddressingMode::AbsoluteLong, Operand::Long(target)) => {
            ControlTarget::Direct(AnalysisPoint {
                location: CodeLocation::new((*target >> 16) as u8, *target as u16),
                mode: at.mode,
            })
        }
        (mode, Operand::Word(pointer)) => ControlTarget::Indirect(IndirectTarget::Pointer {
            mode: *mode,
            pointer: *pointer,
            bank: if *mode == AddressingMode::AbsoluteIndirectLong {
                IndirectBankRule::LoadProgramBank
            } else {
                IndirectBankRule::PreserveProgramBank
            },
        }),
        _ => unreachable!("validated W65C816 control form has a target operand"),
    }
}

fn successor_mode(mode: CpuMode, instruction: &Instruction) -> Option<CpuMode> {
    match (mode, instruction.mnemonic, &instruction.operand) {
        (CpuMode::Native(mut widths), Mnemonic::Rep, Operand::Byte(mask)) => {
            if mask & 0x20 != 0 {
                widths.accumulator = Width::Sixteen;
            }
            if mask & 0x10 != 0 {
                widths.index = Width::Sixteen;
            }
            Some(CpuMode::Native(widths))
        }
        (CpuMode::Native(mut widths), Mnemonic::Sep, Operand::Byte(mask)) => {
            if mask & 0x20 != 0 {
                widths.accumulator = Width::Eight;
            }
            if mask & 0x10 != 0 {
                widths.index = Width::Eight;
            }
            Some(CpuMode::Native(widths))
        }
        (CpuMode::Native(_), Mnemonic::Plp, _) | (_, Mnemonic::Xce, _) => None,
        _ => Some(mode),
    }
}

fn software_vector(kind: Boundary, mode: CpuMode) -> u16 {
    match (kind, mode) {
        (Boundary::Break, CpuMode::Emulation) => 0xfffe,
        (Boundary::Coprocessor, CpuMode::Emulation) => 0xfff4,
        (Boundary::Break, CpuMode::Native(_)) => 0xffe6,
        (Boundary::Coprocessor, CpuMode::Native(_)) => 0xffe4,
        _ => unreachable!("only software trap kinds select vectors"),
    }
}

fn add_instruction_accesses<P: crate::W65C816Profile>(
    instruction: &Instruction,
    mode: CpuMode,
    next: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    use Mnemonic::*;
    use StatusLocation::{Carry, Decimal, Emulation, InterruptDisable, Negative, Overflow, Zero};

    match instruction.mnemonic {
        Adc | Sbc => {
            read_accumulator::<P>(accesses);
            add_operand_access::<P>(instruction, AccessKind::Read, accesses, references);
            read_status(accesses, StatusLocation::Carry);
            read_status(accesses, StatusLocation::Decimal);
            write_accumulator::<P>(accesses);
            write_statuses(accesses, &[Negative, Overflow, Zero, Carry]);
        }
        And | Eor | Ora => {
            read_accumulator::<P>(accesses);
            add_operand_access::<P>(instruction, AccessKind::Read, accesses, references);
            write_accumulator::<P>(accesses);
            write_statuses(accesses, &[Negative, Zero]);
        }
        Cmp => {
            read_accumulator::<P>(accesses);
            add_operand_access::<P>(instruction, AccessKind::Read, accesses, references);
            write_statuses(accesses, &[Negative, Zero, Carry]);
        }
        Cpx | Cpy => {
            read_register(
                accesses,
                if instruction.mnemonic == Cpx {
                    RegisterLocation::X(P::WIDTHS.index)
                } else {
                    RegisterLocation::Y(P::WIDTHS.index)
                },
            );
            add_operand_access::<P>(instruction, AccessKind::Read, accesses, references);
            write_statuses(accesses, &[Negative, Zero, Carry]);
        }
        Bit => {
            read_accumulator::<P>(accesses);
            add_operand_access::<P>(instruction, AccessKind::Read, accesses, references);
            if instruction.mode == AddressingMode::Immediate {
                write_statuses(accesses, &[Zero]);
            } else {
                write_statuses(accesses, &[Negative, Overflow, Zero]);
            }
        }
        Asl | Lsr | Rol | Ror | Dec | Inc => {
            add_target_access::<P>(instruction, AccessKind::Read, accesses, references);
            if matches!(instruction.mnemonic, Rol | Ror) {
                read_status(accesses, StatusLocation::Carry);
            }
            add_target_access::<P>(instruction, AccessKind::Write, accesses, references);
            if matches!(instruction.mnemonic, Asl | Lsr | Rol | Ror) {
                write_statuses(accesses, &[Negative, Zero, Carry]);
            } else {
                write_statuses(accesses, &[Negative, Zero]);
            }
        }
        Lda | Ldx | Ldy => {
            add_operand_access::<P>(instruction, AccessKind::Read, accesses, references);
            let register = match instruction.mnemonic {
                Lda => accumulator_register(P::WIDTHS.accumulator),
                Ldx => RegisterLocation::X(P::WIDTHS.index),
                Ldy => RegisterLocation::Y(P::WIDTHS.index),
                _ => unreachable!(),
            };
            write_register(accesses, register);
            write_statuses(accesses, &[Negative, Zero]);
        }
        Sta | Stx | Sty | Stz => {
            if instruction.mnemonic != Stz {
                read_register(
                    accesses,
                    match instruction.mnemonic {
                        Sta => accumulator_register(P::WIDTHS.accumulator),
                        Stx => RegisterLocation::X(P::WIDTHS.index),
                        Sty => RegisterLocation::Y(P::WIDTHS.index),
                        _ => unreachable!(),
                    },
                );
            }
            add_operand_access::<P>(instruction, AccessKind::Write, accesses, references);
        }
        Trb | Tsb => {
            read_accumulator::<P>(accesses);
            add_operand_access::<P>(instruction, AccessKind::Read, accesses, references);
            add_operand_access::<P>(instruction, AccessKind::Write, accesses, references);
            write_statuses(accesses, &[Zero]);
        }
        Bcc | Bcs => read_status(accesses, StatusLocation::Carry),
        Beq | Bne => read_status(accesses, StatusLocation::Zero),
        Bmi | Bpl => read_status(accesses, StatusLocation::Negative),
        Bvc | Bvs => read_status(accesses, StatusLocation::Overflow),
        Bra | Brl | Nop | Wdm | Wai | Stp => {}
        Jmp | Jml => add_indirect_control_access::<P>(instruction, accesses),
        Jsr | Jsl => {
            add_indirect_control_access::<P>(instruction, accesses);
            add_stack_write(accesses, if instruction.mnemonic == Jsl { 3 } else { 2 });
        }
        Rts => add_stack_read(accesses, 2),
        Rtl => add_stack_read(accesses, 3),
        Rti => {
            add_stack_read(accesses, if mode == CpuMode::Emulation { 3 } else { 4 });
            write_pulled_status(accesses, mode);
        }
        Brk | Cop => {
            add_stack_write(accesses, if mode == CpuMode::Emulation { 3 } else { 4 });
            read_all_status(accesses);
            write_statuses(accesses, &[InterruptDisable, Decimal]);
            let kind = if instruction.mnemonic == Brk {
                Boundary::Break
            } else {
                Boundary::Coprocessor
            };
            read_memory(
                accesses,
                MemoryAddress::Vector {
                    address: software_vector(kind, mode),
                },
                2,
            );
        }
        Clc | Sec => write_statuses(accesses, &[Carry]),
        Cld | Sed => write_statuses(accesses, &[Decimal]),
        Cli | Sei => write_statuses(accesses, &[InterruptDisable]),
        Clv => write_statuses(accesses, &[Overflow]),
        Rep | Sep => add_immediate_status_writes(instruction, mode, accesses),
        Dex | Inx => {
            read_register(accesses, RegisterLocation::X(P::WIDTHS.index));
            write_register(accesses, RegisterLocation::X(P::WIDTHS.index));
            write_statuses(accesses, &[Negative, Zero]);
        }
        Dey | Iny => {
            read_register(accesses, RegisterLocation::Y(P::WIDTHS.index));
            write_register(accesses, RegisterLocation::Y(P::WIDTHS.index));
            write_statuses(accesses, &[Negative, Zero]);
        }
        Mvn | Mvp => add_block_move_accesses(instruction, accesses),
        Pea | Pei | Per => {
            if instruction.mnemonic == Pei {
                read_register(accesses, RegisterLocation::DirectPage);
                add_pointer_read(instruction, 2, accesses);
            }
            if let (Per, Operand::Relative16(displacement)) =
                (instruction.mnemonic, &instruction.operand)
            {
                references.push(StaticReference {
                    target: Reference::CodeAddress(CodeLocation::new(
                        next.location.bank(),
                        (i32::from(next.location.program_counter()) + i32::from(*displacement))
                            as u16,
                    )),
                    kind: ReferenceKind::Address,
                    occurrence: AccessOccurrence::Always,
                });
            }
            add_stack_write(accesses, 2);
        }
        Pha | Phb | Phd | Phk | Php | Phx | Phy => add_push_access::<P>(instruction, accesses),
        Pla | Plb | Pld | Plp | Plx | Ply => add_pull_access::<P>(instruction, mode, accesses),
        Tax | Tay | Txa | Tya | Tsx | Txy | Tyx => add_transfer_access::<P>(instruction, accesses),
        Tcd | Tcs | Tdc | Tsc | Txs => add_wide_transfer_access(instruction, mode, accesses),
        Xba => {
            read_register(
                accesses,
                RegisterLocation::Accumulator(AccumulatorView::Low),
            );
            read_register(
                accesses,
                RegisterLocation::Accumulator(AccumulatorView::High),
            );
            write_register(
                accesses,
                RegisterLocation::Accumulator(AccumulatorView::Low),
            );
            write_register(
                accesses,
                RegisterLocation::Accumulator(AccumulatorView::High),
            );
            write_statuses(accesses, &[Negative, Zero]);
        }
        Xce => {
            read_status(accesses, StatusLocation::Carry);
            read_status(accesses, StatusLocation::Emulation);
            write_statuses(accesses, &[Carry, Emulation]);
            conditional_write_register(accesses, RegisterLocation::X(Width::Sixteen));
            conditional_write_register(accesses, RegisterLocation::Y(Width::Sixteen));
            conditional_write_register(accesses, RegisterLocation::StackPointer);
            conditional_write_status(accesses, StatusLocation::AccumulatorWidth);
            conditional_write_status(accesses, StatusLocation::IndexWidth);
        }
    }
}

fn add_operand_access<P: crate::W65C816Profile>(
    instruction: &Instruction,
    kind: AccessKind,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    if matches!(
        instruction.mode,
        AddressingMode::Immediate
            | AddressingMode::ImmediateByte
            | AddressingMode::Implied
            | AddressingMode::Accumulator
            | AddressingMode::Relative8
            | AddressingMode::Relative16
            | AddressingMode::BlockMove
    ) {
        return;
    }

    add_address_register_reads::<P>(instruction.mode, accesses);
    add_pointer_read(instruction, pointer_width(instruction.mode), accesses);
    let byte_len = operand_width::<P>(instruction.mnemonic);
    access_memory(
        accesses,
        MemoryAddress::Effective {
            mode: instruction.mode,
            operand: instruction.operand.clone(),
        },
        byte_len,
        kind,
        AccessOccurrence::Always,
    );
    if let (AddressingMode::AbsoluteLong, Operand::Long(address)) =
        (instruction.mode, &instruction.operand)
    {
        references.push(StaticReference {
            target: Reference::Data {
                address: *address,
                byte_len,
            },
            kind: match kind {
                AccessKind::Read => ReferenceKind::Read,
                AccessKind::Write => ReferenceKind::Write,
            },
            occurrence: AccessOccurrence::Always,
        });
    }
}

fn add_target_access<P: crate::W65C816Profile>(
    instruction: &Instruction,
    kind: AccessKind,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    if instruction.mode == AddressingMode::Accumulator {
        match kind {
            AccessKind::Read => read_accumulator::<P>(accesses),
            AccessKind::Write => write_accumulator::<P>(accesses),
        }
    } else {
        add_operand_access::<P>(instruction, kind, accesses, references);
    }
}

fn add_address_register_reads<P: crate::W65C816Profile>(
    mode: AddressingMode,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    use AddressingMode::*;
    match mode {
        DirectPage
        | DirectPageIndirect
        | DirectPageIndirectLong
        | DirectPageIndexedIndirectX
        | DirectPageIndirectIndexedY
        | DirectPageIndirectLongIndexedY
        | DirectPageX
        | DirectPageY => read_register(accesses, RegisterLocation::DirectPage),
        Absolute | AbsoluteX | AbsoluteY => read_register(accesses, RegisterLocation::DataBank),
        StackRelative | StackRelativeIndirectIndexedY => {
            read_register(accesses, RegisterLocation::StackPointer)
        }
        _ => {}
    }
    match mode {
        DirectPageX | DirectPageIndexedIndirectX | AbsoluteX | AbsoluteLongX => {
            read_register(accesses, RegisterLocation::X(P::WIDTHS.index));
        }
        DirectPageY
        | DirectPageIndirectIndexedY
        | DirectPageIndirectLongIndexedY
        | AbsoluteY
        | StackRelativeIndirectIndexedY => {
            read_register(accesses, RegisterLocation::Y(P::WIDTHS.index));
        }
        _ => {}
    }
}

fn pointer_width(mode: AddressingMode) -> u8 {
    use AddressingMode::*;
    match mode {
        DirectPageIndirect
        | DirectPageIndexedIndirectX
        | DirectPageIndirectIndexedY
        | StackRelativeIndirectIndexedY
        | AbsoluteIndirect
        | AbsoluteIndexedIndirectX => 2,
        DirectPageIndirectLong | DirectPageIndirectLongIndexedY | AbsoluteIndirectLong => 3,
        _ => 0,
    }
}

fn add_pointer_read(
    instruction: &Instruction,
    byte_len: u8,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    if byte_len != 0 {
        read_memory(
            accesses,
            MemoryAddress::Pointer {
                mode: instruction.mode,
                operand: instruction.operand.clone(),
            },
            byte_len,
        );
    }
}

fn add_indirect_control_access<P: crate::W65C816Profile>(
    instruction: &Instruction,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    match instruction.mode {
        AddressingMode::AbsoluteIndirect
        | AddressingMode::AbsoluteIndexedIndirectX
        | AddressingMode::AbsoluteIndirectLong => {
            if instruction.mode == AddressingMode::AbsoluteIndexedIndirectX {
                read_register(accesses, RegisterLocation::X(P::WIDTHS.index));
            }
            add_pointer_read(instruction, pointer_width(instruction.mode), accesses);
        }
        _ => {}
    }
}

fn operand_width<P: crate::W65C816Profile>(mnemonic: Mnemonic) -> u8 {
    if matches!(
        mnemonic,
        Mnemonic::Ldx
            | Mnemonic::Ldy
            | Mnemonic::Cpx
            | Mnemonic::Cpy
            | Mnemonic::Stx
            | Mnemonic::Sty
    ) {
        width_bytes(P::WIDTHS.index)
    } else {
        width_bytes(P::WIDTHS.accumulator)
    }
}

const fn width_bytes(width: Width) -> u8 {
    match width {
        Width::Eight => 1,
        Width::Sixteen => 2,
    }
}

const fn accumulator_register(width: Width) -> RegisterLocation {
    RegisterLocation::Accumulator(match width {
        Width::Eight => AccumulatorView::Low,
        Width::Sixteen => AccumulatorView::Full,
    })
}

fn read_accumulator<P: crate::W65C816Profile>(accesses: &mut Vec<LocationAccess<Location>>) {
    read_register(accesses, accumulator_register(P::WIDTHS.accumulator));
}

fn write_accumulator<P: crate::W65C816Profile>(accesses: &mut Vec<LocationAccess<Location>>) {
    write_register(accesses, accumulator_register(P::WIDTHS.accumulator));
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

fn conditional_write_register(
    accesses: &mut Vec<LocationAccess<Location>>,
    register: RegisterLocation,
) {
    access(
        accesses,
        Location::Register(register),
        AccessKind::Write,
        AccessOccurrence::Conditional,
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

fn conditional_write_status(accesses: &mut Vec<LocationAccess<Location>>, status: StatusLocation) {
    access(
        accesses,
        Location::Status(status),
        AccessKind::Write,
        AccessOccurrence::Conditional,
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
    for status in processor_statuses() {
        read_status(accesses, status);
    }
}

fn write_pulled_status(accesses: &mut Vec<LocationAccess<Location>>, mode: CpuMode) {
    for status in processor_statuses() {
        if mode != CpuMode::Emulation
            || !matches!(
                status,
                StatusLocation::AccumulatorWidth | StatusLocation::IndexWidth
            )
        {
            write_statuses(accesses, &[status]);
        }
    }
}

const fn processor_statuses() -> [StatusLocation; 8] {
    use StatusLocation::*;
    [
        Negative,
        Overflow,
        AccumulatorWidth,
        IndexWidth,
        Decimal,
        InterruptDisable,
        Zero,
        Carry,
    ]
}

fn add_immediate_status_writes(
    instruction: &Instruction,
    mode: CpuMode,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    let Operand::Byte(mask) = instruction.operand else {
        return;
    };
    for (bit, status) in processor_statuses().into_iter().enumerate() {
        let source_bit = 7 - bit;
        if mask & (1 << source_bit) != 0
            && (mode != CpuMode::Emulation
                || !matches!(
                    status,
                    StatusLocation::AccumulatorWidth | StatusLocation::IndexWidth
                ))
        {
            write_statuses(accesses, &[status]);
        }
    }
}

fn add_stack_read(accesses: &mut Vec<LocationAccess<Location>>, byte_len: u8) {
    read_register(accesses, RegisterLocation::StackPointer);
    write_register(accesses, RegisterLocation::StackPointer);
    read_memory(accesses, MemoryAddress::Stack, byte_len);
}

fn add_stack_write(accesses: &mut Vec<LocationAccess<Location>>, byte_len: u8) {
    read_register(accesses, RegisterLocation::StackPointer);
    write_register(accesses, RegisterLocation::StackPointer);
    write_memory(accesses, MemoryAddress::Stack, byte_len);
}

fn add_push_access<P: crate::W65C816Profile>(
    instruction: &Instruction,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    let (register, byte_len) = match instruction.mnemonic {
        Mnemonic::Pha => (
            accumulator_register(P::WIDTHS.accumulator),
            width_bytes(P::WIDTHS.accumulator),
        ),
        Mnemonic::Phb => (RegisterLocation::DataBank, 1),
        Mnemonic::Phd => (RegisterLocation::DirectPage, 2),
        Mnemonic::Phk => (RegisterLocation::ProgramBank, 1),
        Mnemonic::Phx => (
            RegisterLocation::X(P::WIDTHS.index),
            width_bytes(P::WIDTHS.index),
        ),
        Mnemonic::Phy => (
            RegisterLocation::Y(P::WIDTHS.index),
            width_bytes(P::WIDTHS.index),
        ),
        Mnemonic::Php => {
            read_all_status(accesses);
            add_stack_write(accesses, 1);
            return;
        }
        _ => unreachable!(),
    };
    read_register(accesses, register);
    add_stack_write(accesses, byte_len);
}

fn add_pull_access<P: crate::W65C816Profile>(
    instruction: &Instruction,
    mode: CpuMode,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    let (register, byte_len, flags) = match instruction.mnemonic {
        Mnemonic::Pla => (
            accumulator_register(P::WIDTHS.accumulator),
            width_bytes(P::WIDTHS.accumulator),
            true,
        ),
        Mnemonic::Plb => (RegisterLocation::DataBank, 1, true),
        Mnemonic::Pld => (RegisterLocation::DirectPage, 2, true),
        Mnemonic::Plx => (
            RegisterLocation::X(P::WIDTHS.index),
            width_bytes(P::WIDTHS.index),
            true,
        ),
        Mnemonic::Ply => (
            RegisterLocation::Y(P::WIDTHS.index),
            width_bytes(P::WIDTHS.index),
            true,
        ),
        Mnemonic::Plp => {
            add_stack_read(accesses, 1);
            write_pulled_status(accesses, mode);
            return;
        }
        _ => unreachable!(),
    };
    add_stack_read(accesses, byte_len);
    write_register(accesses, register);
    if flags {
        write_statuses(accesses, &[StatusLocation::Negative, StatusLocation::Zero]);
    }
}

fn add_transfer_access<P: crate::W65C816Profile>(
    instruction: &Instruction,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    let (source, destination) = match instruction.mnemonic {
        Mnemonic::Tax => (
            accumulator_register(P::WIDTHS.accumulator),
            RegisterLocation::X(P::WIDTHS.index),
        ),
        Mnemonic::Tay => (
            accumulator_register(P::WIDTHS.accumulator),
            RegisterLocation::Y(P::WIDTHS.index),
        ),
        Mnemonic::Txa => (
            RegisterLocation::X(P::WIDTHS.index),
            accumulator_register(P::WIDTHS.accumulator),
        ),
        Mnemonic::Tya => (
            RegisterLocation::Y(P::WIDTHS.index),
            accumulator_register(P::WIDTHS.accumulator),
        ),
        Mnemonic::Tsx => (
            RegisterLocation::StackPointer,
            RegisterLocation::X(P::WIDTHS.index),
        ),
        Mnemonic::Txy => (
            RegisterLocation::X(P::WIDTHS.index),
            RegisterLocation::Y(P::WIDTHS.index),
        ),
        Mnemonic::Tyx => (
            RegisterLocation::Y(P::WIDTHS.index),
            RegisterLocation::X(P::WIDTHS.index),
        ),
        _ => unreachable!(),
    };
    read_register(accesses, source);
    write_register(accesses, destination);
    write_statuses(accesses, &[StatusLocation::Negative, StatusLocation::Zero]);
}

fn add_wide_transfer_access(
    instruction: &Instruction,
    mode: CpuMode,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    let full_accumulator = RegisterLocation::Accumulator(AccumulatorView::Full);
    let (source, destination, writes_flags) = match instruction.mnemonic {
        Mnemonic::Tcd => (full_accumulator, RegisterLocation::DirectPage, true),
        Mnemonic::Tcs => (full_accumulator, RegisterLocation::StackPointer, false),
        Mnemonic::Tdc => (RegisterLocation::DirectPage, full_accumulator, true),
        Mnemonic::Tsc => (RegisterLocation::StackPointer, full_accumulator, true),
        Mnemonic::Txs => (
            RegisterLocation::X(mode.widths().index),
            RegisterLocation::StackPointer,
            false,
        ),
        _ => unreachable!(),
    };
    read_register(accesses, source);
    write_register(accesses, destination);
    if writes_flags {
        write_statuses(accesses, &[StatusLocation::Negative, StatusLocation::Zero]);
    }
}

fn add_block_move_accesses(
    instruction: &Instruction,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    let Operand::BlockMove {
        destination_bank,
        source_bank,
    } = instruction.operand
    else {
        return;
    };
    for register in [
        RegisterLocation::Accumulator(AccumulatorView::Full),
        RegisterLocation::X(Width::Sixteen),
        RegisterLocation::Y(Width::Sixteen),
    ] {
        read_register(accesses, register);
        write_register(accesses, register);
    }
    write_register(accesses, RegisterLocation::DataBank);
    read_memory(
        accesses,
        MemoryAddress::BlockSource { bank: source_bank },
        1,
    );
    write_memory(
        accesses,
        MemoryAddress::BlockDestination {
            bank: destination_bank,
        },
        1,
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

fn access_memory(
    accesses: &mut Vec<LocationAccess<Location>>,
    address: MemoryAddress,
    byte_len: u8,
    kind: AccessKind,
    occurrence: AccessOccurrence,
) {
    debug_assert_ne!(byte_len, 0);
    access(
        accesses,
        Location::Memory(MemoryLocation { address, byte_len }),
        kind,
        occurrence,
    );
}

fn read_memory(accesses: &mut Vec<LocationAccess<Location>>, address: MemoryAddress, byte_len: u8) {
    access_memory(
        accesses,
        address,
        byte_len,
        AccessKind::Read,
        AccessOccurrence::Always,
    );
}

fn write_memory(
    accesses: &mut Vec<LocationAccess<Location>>,
    address: MemoryAddress,
    byte_len: u8,
) {
    access_memory(
        accesses,
        address,
        byte_len,
        AccessKind::Write,
        AccessOccurrence::Always,
    );
}

pub(crate) fn storage_relation(left: &Location, right: &Location) -> StorageRelation {
    if left == right {
        return StorageRelation::Same;
    }
    match (left, right) {
        (Location::Register(left), Location::Register(right)) => register_relation(*left, *right),
        (Location::Status(_), Location::Status(_)) => StorageRelation::Disjoint,
        (Location::Memory(left), Location::Memory(right)) => memory_relation(left, right),
        (Location::Memory(_), _) | (_, Location::Memory(_)) => StorageRelation::Disjoint,
        _ => StorageRelation::Disjoint,
    }
}

fn register_relation(left: RegisterLocation, right: RegisterLocation) -> StorageRelation {
    use AccumulatorView::*;
    use RegisterLocation::*;
    match (left, right) {
        (Accumulator(Full), Accumulator(Low | High))
        | (Accumulator(Low | High), Accumulator(Full)) => StorageRelation::Overlapping,
        (X(_), X(_)) | (Y(_), Y(_)) => StorageRelation::Overlapping,
        _ => StorageRelation::Disjoint,
    }
}

fn memory_relation(left: &MemoryLocation, right: &MemoryLocation) -> StorageRelation {
    match (&left.address, &right.address) {
        (
            MemoryAddress::Effective {
                mode: AddressingMode::AbsoluteLong,
                operand: Operand::Long(left_address),
            },
            MemoryAddress::Effective {
                mode: AddressingMode::AbsoluteLong,
                operand: Operand::Long(right_address),
            },
        ) => range_relation(*left_address, left.byte_len, *right_address, right.byte_len),
        (left_address, right_address) if left_address == right_address => {
            StorageRelation::Overlapping
        }
        _ => StorageRelation::Unknown,
    }
}

fn range_relation(left: u32, left_len: u8, right: u32, right_len: u8) -> StorageRelation {
    let left_end = left + u32::from(left_len);
    let right_end = right + u32::from(right_len);
    if left == right && left_len == right_len {
        StorageRelation::Same
    } else if left < right_end && right < left_end {
        StorageRelation::Overlapping
    } else {
        StorageRelation::Disjoint
    }
}
