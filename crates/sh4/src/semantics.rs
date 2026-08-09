//! Complete static meaning for the four explicit SH-4 FPSCR profiles.

use core::fmt;

use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlBoundary, ControlTarget, DelaySlot,
    DelaySlotExecution, LocationAccess, ReferenceKind, StaticReference, StorageRelation,
};

use crate::{
    BankedRegister, CodeLocation, DoubleRegister, FloatRegister, FpscrState, FpuPairRegister,
    FpuTransferSize, Instruction, Register, Sh4Profile, VectorRegister,
};

/// Physical bank selected for encoded R0-R7 operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GeneralRegisterBank {
    /// Ordinary R0-R7 storage.
    Primary,
    /// R0_BANK-R7_BANK storage.
    Banked,
    /// SR.MD/RB is not statically known.
    Unknown,
}

/// Physical floating-register bank selected by FPSCR.FR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloatBank {
    /// FR names the primary floating bank and XF the extended bank.
    Primary,
    /// FR names the extended floating bank and XF the primary bank.
    Alternate,
    /// FPSCR.FR is not statically known.
    Unknown,
}

impl FloatBank {
    fn toggled(self) -> Self {
        match self {
            Self::Primary => Self::Alternate,
            Self::Alternate => Self::Primary,
            Self::Unknown => Self::Unknown,
        }
    }
}

/// Placed SH-4 instruction together with storage- and decode-selecting state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnalysisPoint {
    /// Halfword-aligned architectural instruction address.
    pub location: CodeLocation,
    /// Physical bank selected for encoded R0-R7 operands.
    pub general_bank: GeneralRegisterBank,
    /// Physical bank selected for FR names.
    pub float_bank: FloatBank,
    /// Known FPSCR.PR/SZ decode state, or `None` after an unknown FPSCR write.
    pub fpscr: Option<FpscrState>,
}

impl AnalysisPoint {
    /// Constructs a point with all state required for SH-4 static meaning.
    pub const fn new(
        location: CodeLocation,
        general_bank: GeneralRegisterBank,
        float_bank: FloatBank,
        fpscr: Option<FpscrState>,
    ) -> Self {
        Self {
            location,
            general_bank,
            float_bank,
            fpscr,
        }
    }

    /// Constructs a point whose decode state matches an exact SH-4 profile.
    pub fn for_profile<P: Sh4Profile>(
        location: CodeLocation,
        general_bank: GeneralRegisterBank,
        float_bank: FloatBank,
    ) -> Self {
        Self::new(location, general_bank, float_bank, Some(P::FPSCR))
    }
}

/// Runtime-derived SH-4 control destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndirectTarget {
    /// Destination read from an encoded general register.
    Register(GeneralRegisterLocation),
    /// Destination formed as `PC + 4 + Rm`.
    PcRelativeRegister(GeneralRegisterLocation),
    /// Destination read from PR.
    ProcedureRegister,
    /// Destination restored from SPC while SR is restored from SSR.
    ExceptionReturn,
    /// Trap vector formed from VBR and the encoded immediate.
    TrapVector(u8),
}

/// SH-4 synchronous boundary or wait kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Boundary {
    /// Software trap.
    Trap(u8),
    /// SLEEP wait state.
    Sleep,
}

/// Individually meaningful SH-4 SR field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SrField {
    /// Condition bit T.
    T,
    /// Saturation bit S.
    S,
    /// Division quotient bit Q.
    Q,
    /// Division sign bit M.
    M,
    /// Privileged register-bank selector RB.
    RegisterBank,
    /// Privileged/user mode selector MD.
    PrivilegedMode,
    /// Interrupt mask field.
    InterruptMask,
    /// Exception-blocking bit BL.
    ExceptionBlock,
}

/// Individually meaningful FPSCR field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FpscrField {
    /// Exception cause fields.
    Cause,
    /// Sticky exception flag fields.
    Flag,
    /// Precision selector PR.
    Precision,
    /// Transfer-size selector SZ.
    TransferSize,
    /// Floating-register bank selector FR.
    RegisterBank,
}

/// Physical or conservatively unresolved general-register storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GeneralRegisterLocation {
    /// Ordinary R0-R15 storage.
    Primary(Register),
    /// Banked R0_BANK-R7_BANK storage.
    Banked(BankedRegister),
    /// Encoded R0-R7 whose current physical bank is unavailable.
    CurrentUnknown(Register),
    /// Encoded Rn_BANK whose opposite physical bank is unavailable.
    OppositeUnknown(Register),
}

/// Physical or conservatively unresolved floating-register lane bank.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloatLaneBank {
    /// Primary physical bank.
    Primary,
    /// Extended physical bank.
    Extended,
    /// Current bank when FPSCR.FR is unavailable.
    CurrentUnknown,
    /// Opposite bank when FPSCR.FR is unavailable.
    OppositeUnknown,
}

/// SH-4 symbolic memory address rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Addressing {
    /// Address held in one encoded general register.
    Register(GeneralRegisterLocation),
    /// Address after pre-decrementing a general register.
    PreDecrement(GeneralRegisterLocation),
    /// Address before post-incrementing a general register.
    PostIncrement(GeneralRegisterLocation),
    /// Sum of R0 and another general register.
    IndexedR0(GeneralRegisterLocation),
    /// Base register plus an already scaled byte displacement.
    Displacement {
        /// Base register.
        base: GeneralRegisterLocation,
        /// Byte displacement.
        displacement: u16,
    },
    /// GBR plus an already scaled byte displacement.
    GbrDisplacement(u16),
    /// GBR plus R0.
    GbrIndexedR0,
    /// Placed PC-relative address.
    PcRelative(u32),
    /// Exception stack frame at R15.
    ExceptionFrame,
}

/// Symbolic SH-4 memory storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryLocation {
    /// Addressing rule.
    pub addressing: Addressing,
    /// Access width in bytes.
    pub byte_len: u8,
}

/// Architectural SH-4 storage location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Location {
    /// General-register storage.
    General(GeneralRegisterLocation),
    /// Whole status register.
    Sr,
    /// Individually meaningful status field.
    SrField(SrField),
    /// Global base register.
    Gbr,
    /// Vector base register.
    Vbr,
    /// Saved status register.
    Ssr,
    /// Saved program counter.
    Spc,
    /// Saved general register.
    Sgr,
    /// Debug base register.
    Dbr,
    /// Multiply-accumulate high register.
    Mach,
    /// Multiply-accumulate low register.
    Macl,
    /// Procedure register.
    Pr,
    /// Floating-point communication register.
    Fpul,
    /// Whole floating-point status/control register.
    Fpscr,
    /// Individually meaningful FPSCR field.
    FpscrField(FpscrField),
    /// One physical or unresolved 32-bit floating lane.
    FloatLane {
        /// Lane bank.
        bank: FloatLaneBank,
        /// Lane number 0-15.
        index: u8,
    },
    /// Symbolic memory storage.
    Memory(MemoryLocation),
    /// Translation lookaside buffer state touched by LDTLB.
    Tlb,
    /// Cache line selected by an address register.
    CacheLine(GeneralRegisterLocation),
    /// Page-table entry high register PTEH.
    Pteh,
    /// Page-table entry low register PTEL.
    Ptel,
    /// Page-table assistance register PTEA.
    Ptea,
    /// Memory-management control register MMUCR.
    Mmucr,
}

/// Statically resolved non-control SH-4 reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reference {
    /// Literal-pool bytes.
    Memory {
        /// First referenced byte.
        address: u32,
        /// Referenced width in bytes.
        byte_len: u8,
    },
    /// Address formed by MOVA.
    Address(u32),
}

/// Structural failure to form sound SH-4 static facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticError {
    /// Instruction location is not halfword aligned.
    UnalignedLocation {
        /// Rejected location.
        location: CodeLocation,
    },
    /// Analysis point lacks the FPSCR.PR/SZ state needed to select a profile.
    UnavailableDecodeState,
    /// Analysis point decode state belongs to another exact SH-4 profile.
    ProfileStateMismatch {
        /// State required by the selected profile.
        expected: FpscrState,
        /// State supplied by the point.
        actual: FpscrState,
    },
}

impl fmt::Display for SemanticError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnalignedLocation { location } => write!(
                formatter,
                "SH-4 instruction location {location:#010x} is not halfword aligned"
            ),
            Self::UnavailableDecodeState => {
                formatter.write_str("SH-4 FPSCR.PR/SZ decode state is unavailable")
            }
            Self::ProfileStateMismatch { expected, actual } => write!(
                formatter,
                "SH-4 point state {actual:?} does not match selected profile {expected:?}"
            ),
        }
    }
}

impl std::error::Error for SemanticError {}

/// Shared control-flow result specialized with SH-4 vocabulary.
pub type ControlFlow = typed_isa_core::ControlFlow<AnalysisPoint, IndirectTarget, Boundary>;

/// Shared instruction-semantics result specialized with SH-4 vocabulary.
pub type InstructionSemantics = typed_isa_core::InstructionSemantics<
    AnalysisPoint,
    IndirectTarget,
    Boundary,
    Location,
    Reference,
>;

pub(crate) fn semantics<P: Sh4Profile>(
    instruction: &Instruction,
    at: AnalysisPoint,
) -> Result<InstructionSemantics, SemanticError> {
    if at.location & 1 != 0 {
        return Err(SemanticError::UnalignedLocation {
            location: at.location,
        });
    }
    match at.fpscr {
        None => return Err(SemanticError::UnavailableDecodeState),
        Some(actual) if actual != P::FPSCR => {
            return Err(SemanticError::ProfileStateMismatch {
                expected: P::FPSCR,
                actual,
            });
        }
        Some(_) => {}
    }
    let mut accesses = Vec::new();
    let mut references = Vec::new();
    add_instruction_accesses(instruction, at, &mut accesses, &mut references);
    Ok(InstructionSemantics {
        control_flow: control_flow(instruction, at),
        location_accesses: accesses,
        static_references: references,
    })
}

fn successor(instruction: &Instruction, at: AnalysisPoint, location: u32) -> AnalysisPoint {
    let mut point = AnalysisPoint { location, ..at };
    match instruction {
        Instruction::LdcSr { .. } | Instruction::LdcLSr { .. } | Instruction::Rte => {
            point.general_bank = GeneralRegisterBank::Unknown
        }
        Instruction::LdsFpscr { .. } | Instruction::LdsLFpscr { .. } => {
            point.fpscr = None;
            point.float_bank = FloatBank::Unknown;
        }
        Instruction::Frchg => point.float_bank = point.float_bank.toggled(),
        Instruction::Fschg => {
            if let Some(state) = point.fpscr {
                point.fpscr = Some(FpscrState::new(
                    state.precision(),
                    match state.transfer_size() {
                        FpuTransferSize::Single => FpuTransferSize::Pair,
                        FpuTransferSize::Pair => FpuTransferSize::Single,
                    },
                ));
            }
        }
        _ => {}
    }
    point
}

fn control_flow(instruction: &Instruction, at: AnalysisPoint) -> ControlFlow {
    let next = successor(instruction, at, at.location.wrapping_add(2));
    let mut after_slot = successor(instruction, at, at.location.wrapping_add(4));
    // An allowed delay-slot instruction can change FPSCR or FR. Until a
    // sequence consumer composes the actual slot, the post-slot decode and
    // physical floating-bank context are therefore unavailable.
    after_slot.fpscr = None;
    after_slot.float_bank = FloatBank::Unknown;
    let slot_general_bank = if matches!(instruction, Instruction::Rte) {
        // RTE restores SR before executing its delay-slot instruction.
        GeneralRegisterBank::Unknown
    } else {
        at.general_bank
    };
    let slot = Some(DelaySlot {
        point: AnalysisPoint {
            location: at.location.wrapping_add(2),
            general_bank: slot_general_bank,
            ..at
        },
        execution: DelaySlotExecution::Always,
    });
    let direct = |target| {
        ControlTarget::Direct(AnalysisPoint {
            location: target,
            ..after_slot
        })
    };
    match *instruction {
        Instruction::Bt { .. } | Instruction::Bf { .. } => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Direct(AnalysisPoint {
                    location: instruction.branch_target(at.location).unwrap(),
                    ..at
                }),
            },
            fallthrough: Some(next),
            delay_slot: None,
        },
        Instruction::Bts { .. } | Instruction::Bfs { .. } => ControlFlow {
            action: ControlAction::Transfer {
                target: direct(instruction.branch_target(at.location).unwrap()),
            },
            fallthrough: Some(after_slot),
            delay_slot: slot,
        },
        Instruction::Bra { .. } => ControlFlow {
            action: ControlAction::Transfer {
                target: direct(instruction.branch_target(at.location).unwrap()),
            },
            fallthrough: None,
            delay_slot: slot,
        },
        Instruction::Bsr { .. } => ControlFlow {
            action: ControlAction::LinkedTransfer {
                target: direct(instruction.branch_target(at.location).unwrap()),
                return_site: after_slot,
            },
            fallthrough: None,
            delay_slot: slot,
        },
        Instruction::Braf { rm } => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Indirect(IndirectTarget::PcRelativeRegister(
                    general_location(at.general_bank, rm),
                )),
            },
            fallthrough: None,
            delay_slot: slot,
        },
        Instruction::Bsrf { rm } => ControlFlow {
            action: ControlAction::LinkedTransfer {
                target: ControlTarget::Indirect(IndirectTarget::PcRelativeRegister(
                    general_location(at.general_bank, rm),
                )),
                return_site: after_slot,
            },
            fallthrough: None,
            delay_slot: slot,
        },
        Instruction::Jmp { rm } => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Indirect(IndirectTarget::Register(general_location(
                    at.general_bank,
                    rm,
                ))),
            },
            fallthrough: None,
            delay_slot: slot,
        },
        Instruction::Jsr { rm } => ControlFlow {
            action: ControlAction::LinkedTransfer {
                target: ControlTarget::Indirect(IndirectTarget::Register(general_location(
                    at.general_bank,
                    rm,
                ))),
                return_site: after_slot,
            },
            fallthrough: None,
            delay_slot: slot,
        },
        Instruction::Rts => ControlFlow {
            action: ControlAction::Return {
                target: IndirectTarget::ProcedureRegister,
            },
            fallthrough: None,
            delay_slot: slot,
        },
        Instruction::Rte => ControlFlow {
            action: ControlAction::ExceptionReturn {
                target: IndirectTarget::ExceptionReturn,
            },
            fallthrough: None,
            delay_slot: slot,
        },
        Instruction::Trapa { imm } => ControlFlow {
            action: ControlAction::Boundary(ControlBoundary::Trap {
                kind: Boundary::Trap(imm),
                target: Some(ControlTarget::Indirect(IndirectTarget::TrapVector(imm))),
                resume: Some(next),
            }),
            fallthrough: None,
            delay_slot: None,
        },
        Instruction::Sleep => ControlFlow {
            action: ControlAction::Boundary(ControlBoundary::Wait {
                kind: Boundary::Sleep,
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

pub(crate) fn has_delay_slot(instruction: &Instruction) -> bool {
    control_flow(
        instruction,
        AnalysisPoint::for_profile::<crate::Sh4Pr0Sz0>(
            0,
            GeneralRegisterBank::Primary,
            FloatBank::Primary,
        ),
    )
    .delay_slot
    .is_some()
}

fn add_instruction_accesses(
    instruction: &Instruction,
    at: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    use Instruction::*;
    match *instruction {
        MovImm { rn, .. } => write_general(accesses, at, rn),
        MovWPcRel { rn, .. } => pc_relative_load(instruction, at, rn, 2, accesses, references),
        MovLPcRel { rn, .. } => pc_relative_load(instruction, at, rn, 4, accesses, references),
        MovReg { rm, rn } => {
            read_general(accesses, at, rm);
            write_general(accesses, at, rn);
        }
        MovBStore { rm, rn } => register_store(at, rm, rn, 1, accesses),
        MovWStore { rm, rn } => register_store(at, rm, rn, 2, accesses),
        MovLStore { rm, rn } => register_store(at, rm, rn, 4, accesses),
        MovBLoad { rm, rn } => register_load(at, rm, rn, 1, accesses),
        MovWLoad { rm, rn } => register_load(at, rm, rn, 2, accesses),
        MovLLoad { rm, rn } => register_load(at, rm, rn, 4, accesses),
        MovBStorePreDec { rm, rn } => predecrement_store(at, rm, rn, 1, accesses),
        MovWStorePreDec { rm, rn } => predecrement_store(at, rm, rn, 2, accesses),
        MovLStorePreDec { rm, rn } => predecrement_store(at, rm, rn, 4, accesses),
        MovBLoadPostInc { rm, rn } => postincrement_load(at, rm, rn, 1, accesses),
        MovWLoadPostInc { rm, rn } => postincrement_load(at, rm, rn, 2, accesses),
        MovLLoadPostInc { rm, rn } => postincrement_load(at, rm, rn, 4, accesses),
        MovBR0LoadIndexed { rm, rn } => indexed_load(at, rm, rn, 1, accesses),
        MovWR0LoadIndexed { rm, rn } => indexed_load(at, rm, rn, 2, accesses),
        MovLR0LoadIndexed { rm, rn } => indexed_load(at, rm, rn, 4, accesses),
        MovBR0StoreIndexed { rm, rn } => indexed_store(at, rm, rn, 1, accesses),
        MovWR0StoreIndexed { rm, rn } => indexed_store(at, rm, rn, 2, accesses),
        MovLR0StoreIndexed { rm, rn } => indexed_store(at, rm, rn, 4, accesses),
        MovBDispStore { disp, rn } => displacement_store(at, Register::R0, rn, disp, 1, accesses),
        MovWDispStore { disp, rn } => displacement_store(at, Register::R0, rn, disp, 2, accesses),
        MovLDispStore { disp, rm, rn } => displacement_store(at, rm, rn, disp, 4, accesses),
        MovBDispLoad { disp, rm } => displacement_load(at, rm, Register::R0, disp, 1, accesses),
        MovWDispLoad { disp, rm } => displacement_load(at, rm, Register::R0, disp, 2, accesses),
        MovLDispLoad { disp, rm, rn } => displacement_load(at, rm, rn, disp, 4, accesses),
        MovBGbrStore { disp } => gbr_transfer(at, disp, 1, AccessKind::Write, accesses),
        MovWGbrStore { disp } => gbr_transfer(at, disp, 2, AccessKind::Write, accesses),
        MovLGbrStore { disp } => gbr_transfer(at, disp, 4, AccessKind::Write, accesses),
        MovBGbrLoad { disp } => gbr_transfer(at, disp, 1, AccessKind::Read, accesses),
        MovWGbrLoad { disp } => gbr_transfer(at, disp, 2, AccessKind::Read, accesses),
        MovLGbrLoad { disp } => gbr_transfer(at, disp, 4, AccessKind::Read, accesses),
        Mova { .. } => {
            write_general(accesses, at, Register::R0);
            references.push(StaticReference {
                target: Reference::Address(literal_address(instruction, at.location)),
                kind: ReferenceKind::Address,
                occurrence: AccessOccurrence::Always,
            });
        }
        MovT { rn } => {
            read_sr(accesses, SrField::T);
            write_general(accesses, at, rn);
        }
        Add { rm, rn }
        | Sub { rm, rn }
        | And { rm, rn }
        | Or { rm, rn }
        | Xor { rm, rn }
        | Xtrct { rm, rn }
        | Shad { rm, rn }
        | Shld { rm, rn } => {
            read_general(accesses, at, rm);
            read_write_general(accesses, at, rn);
        }
        AddImm { rn, .. } => read_write_general(accesses, at, rn),
        Addc { rm, rn } | Subc { rm, rn } => {
            read_general(accesses, at, rm);
            read_write_general(accesses, at, rn);
            read_sr(accesses, SrField::T);
            write_sr(accesses, SrField::T);
        }
        Addv { rm, rn } | Subv { rm, rn } => {
            read_general(accesses, at, rm);
            read_write_general(accesses, at, rn);
            write_sr(accesses, SrField::T);
        }
        Neg { rm, rn } => {
            read_general(accesses, at, rm);
            write_general(accesses, at, rn);
        }
        Negc { rm, rn } => {
            read_general(accesses, at, rm);
            read_sr(accesses, SrField::T);
            write_general(accesses, at, rn);
            write_sr(accesses, SrField::T);
        }
        Dt { rn } => {
            read_write_general(accesses, at, rn);
            write_sr(accesses, SrField::T);
        }
        MulL { rm, rn } | MulS { rm, rn } | MulU { rm, rn } => {
            read_general(accesses, at, rm);
            read_general(accesses, at, rn);
            write(accesses, Location::Macl);
        }
        DmulS { rm, rn } | DmulU { rm, rn } => {
            read_general(accesses, at, rm);
            read_general(accesses, at, rn);
            write(accesses, Location::Mach);
            write(accesses, Location::Macl);
        }
        Div0S { rm, rn } => {
            read_general(accesses, at, rm);
            read_general(accesses, at, rn);
            write_sr_many(accesses, &[SrField::Q, SrField::M, SrField::T]);
        }
        Div0U => write_sr_many(accesses, &[SrField::Q, SrField::M, SrField::T]),
        Div1 { rm, rn } => {
            read_general(accesses, at, rm);
            read_write_general(accesses, at, rn);
            for field in [SrField::Q, SrField::M, SrField::T] {
                read_sr(accesses, field);
            }
            write_sr_many(accesses, &[SrField::Q, SrField::T]);
        }
        MacL { rm, rn } => mac(at, rm, rn, 4, accesses),
        MacW { rm, rn } => mac(at, rm, rn, 2, accesses),
        AndImm { .. } | OrImm { .. } | XorImm { .. } => {
            read_write_general(accesses, at, Register::R0)
        }
        TstImm { .. } => {
            read_general(accesses, at, Register::R0);
            write_sr(accesses, SrField::T);
        }
        AndB { .. } | OrB { .. } | XorB { .. } | TstB { .. } => {
            read(accesses, Location::Gbr);
            read_general(accesses, at, Register::R0);
            let memory = Location::Memory(MemoryLocation {
                addressing: Addressing::GbrIndexedR0,
                byte_len: 1,
            });
            read(accesses, memory);
            if matches!(instruction, TstB { .. }) {
                write_sr(accesses, SrField::T);
            } else {
                write(accesses, memory);
            }
        }
        Not { rm, rn }
        | ExtsB { rm, rn }
        | ExtsW { rm, rn }
        | ExtuB { rm, rn }
        | ExtuW { rm, rn }
        | SwapB { rm, rn }
        | SwapW { rm, rn } => {
            read_general(accesses, at, rm);
            write_general(accesses, at, rn);
        }
        Tst { rm, rn }
        | CmpEq { rm, rn }
        | CmpGe { rm, rn }
        | CmpGt { rm, rn }
        | CmpHi { rm, rn }
        | CmpHs { rm, rn }
        | CmpStr { rm, rn } => {
            read_general(accesses, at, rm);
            read_general(accesses, at, rn);
            write_sr(accesses, SrField::T);
        }
        Shll { rn } | Shlr { rn } | Shal { rn } | Shar { rn } | Rotl { rn } | Rotr { rn } => {
            read_write_general(accesses, at, rn);
            write_sr(accesses, SrField::T);
        }
        Rotcl { rn } | Rotcr { rn } => {
            read_write_general(accesses, at, rn);
            read_sr(accesses, SrField::T);
            write_sr(accesses, SrField::T);
        }
        Shll2 { rn }
        | Shlr2 { rn }
        | Shll8 { rn }
        | Shlr8 { rn }
        | Shll16 { rn }
        | Shlr16 { rn } => read_write_general(accesses, at, rn),
        CmpEqImm { .. } => {
            read_general(accesses, at, Register::R0);
            write_sr(accesses, SrField::T);
        }
        CmpPl { rn } | CmpPz { rn } => {
            read_general(accesses, at, rn);
            write_sr(accesses, SrField::T);
        }
        Bt { .. } | Bf { .. } | Bts { .. } | Bfs { .. } => read_sr(accesses, SrField::T),
        Bra { .. } => {}
        Bsr { .. } => write(accesses, Location::Pr),
        Jmp { rm } | Braf { rm } => read_general(accesses, at, rm),
        Jsr { rm } | Bsrf { rm } => {
            read_general(accesses, at, rm);
            write(accesses, Location::Pr);
        }
        Rts => read(accesses, Location::Pr),
        Rte => {
            read(accesses, Location::Spc);
            read(accesses, Location::Ssr);
            write(accesses, Location::Sr);
        }
        LdcGbr { rm } => register_to_special(at, rm, Location::Gbr, accesses),
        LdcVbr { rm } => register_to_special(at, rm, Location::Vbr, accesses),
        LdcSr { rm } => register_to_special(at, rm, Location::Sr, accesses),
        LdcSsr { rm } => register_to_special(at, rm, Location::Ssr, accesses),
        LdcSpc { rm } => register_to_special(at, rm, Location::Spc, accesses),
        LdcDbr { rm } => register_to_special(at, rm, Location::Dbr, accesses),
        LdcBank { rm, bank } => {
            read_general(accesses, at, rm);
            write(
                accesses,
                Location::General(opposite_general_location(at.general_bank, bank)),
            );
        }
        LdcLGbr { rm } => load_special(at, rm, Location::Gbr, accesses),
        LdcLVbr { rm } => load_special(at, rm, Location::Vbr, accesses),
        LdcLSr { rm } => load_special(at, rm, Location::Sr, accesses),
        LdcLSsr { rm } => load_special(at, rm, Location::Ssr, accesses),
        LdcLSpc { rm } => load_special(at, rm, Location::Spc, accesses),
        LdcLDbr { rm } => load_special(at, rm, Location::Dbr, accesses),
        LdcLBank { rm, bank } => load_special(
            at,
            rm,
            Location::General(opposite_general_location(at.general_bank, bank)),
            accesses,
        ),
        StcGbr { rn } => special_to_register(at, Location::Gbr, rn, accesses),
        StcVbr { rn } => special_to_register(at, Location::Vbr, rn, accesses),
        StcSr { rn } => special_to_register(at, Location::Sr, rn, accesses),
        StcSsr { rn } => special_to_register(at, Location::Ssr, rn, accesses),
        StcSpc { rn } => special_to_register(at, Location::Spc, rn, accesses),
        StcSgr { rn } => special_to_register(at, Location::Sgr, rn, accesses),
        StcDbr { rn } => special_to_register(at, Location::Dbr, rn, accesses),
        StcBank { bank, rn } => {
            read(
                accesses,
                Location::General(opposite_general_location(at.general_bank, bank)),
            );
            write_general(accesses, at, rn);
        }
        StcLGbr { rn } => store_special(at, Location::Gbr, rn, accesses),
        StcLVbr { rn } => store_special(at, Location::Vbr, rn, accesses),
        StcLSr { rn } => store_special(at, Location::Sr, rn, accesses),
        StcLSsr { rn } => store_special(at, Location::Ssr, rn, accesses),
        StcLSpc { rn } => store_special(at, Location::Spc, rn, accesses),
        StcLSgr { rn } => store_special(at, Location::Sgr, rn, accesses),
        StcLDbr { rn } => store_special(at, Location::Dbr, rn, accesses),
        StcLBank { bank, rn } => store_special(
            at,
            Location::General(opposite_general_location(at.general_bank, bank)),
            rn,
            accesses,
        ),
        LdsMach { rm } => register_to_special(at, rm, Location::Mach, accesses),
        LdsMacl { rm } => register_to_special(at, rm, Location::Macl, accesses),
        LdsPr { rm } => register_to_special(at, rm, Location::Pr, accesses),
        LdsFpul { rm } => register_to_special(at, rm, Location::Fpul, accesses),
        LdsFpscr { rm } => register_to_special(at, rm, Location::Fpscr, accesses),
        LdsLMach { rm } => load_special(at, rm, Location::Mach, accesses),
        LdsLMacl { rm } => load_special(at, rm, Location::Macl, accesses),
        LdsLPr { rm } => load_special(at, rm, Location::Pr, accesses),
        LdsLFpul { rm } => load_special(at, rm, Location::Fpul, accesses),
        LdsLFpscr { rm } => load_special(at, rm, Location::Fpscr, accesses),
        StsMach { rn } => special_to_register(at, Location::Mach, rn, accesses),
        StsMacl { rn } => special_to_register(at, Location::Macl, rn, accesses),
        StsPr { rn } => special_to_register(at, Location::Pr, rn, accesses),
        StsFpul { rn } => special_to_register(at, Location::Fpul, rn, accesses),
        StsFpscr { rn } => special_to_register(at, Location::Fpscr, rn, accesses),
        StsLMach { rn } => store_special(at, Location::Mach, rn, accesses),
        StsLMacl { rn } => store_special(at, Location::Macl, rn, accesses),
        StsLPr { rn } => store_special(at, Location::Pr, rn, accesses),
        StsLFpul { rn } => store_special(at, Location::Fpul, rn, accesses),
        StsLFpscr { rn } => store_special(at, Location::Fpscr, rn, accesses),
        Nop | Sleep => {}
        SetT | ClrT => write_sr(accesses, SrField::T),
        SetS | ClrS => write_sr(accesses, SrField::S),
        ClrMac => {
            write(accesses, Location::Mach);
            write(accesses, Location::Macl);
        }
        Ldtlb => {
            for location in [
                Location::Pteh,
                Location::Ptel,
                Location::Ptea,
                Location::Mmucr,
            ] {
                read(accesses, location);
            }
            write(accesses, Location::Tlb);
        }
        Trapa { .. } => {
            read(accesses, Location::Vbr);
            read(accesses, Location::Sr);
            write(accesses, Location::Ssr);
            write(accesses, Location::Spc);
            write(accesses, Location::Sgr);
            write_sr_many(
                accesses,
                &[
                    SrField::PrivilegedMode,
                    SrField::RegisterBank,
                    SrField::ExceptionBlock,
                ],
            );
        }
        Tas { rn } => {
            read_general(accesses, at, rn);
            let memory = memory_register(at, rn, 1);
            read(accesses, memory);
            write(accesses, memory);
            write_sr(accesses, SrField::T);
        }
        Pref { rn } | Ocbi { rn } | Ocbp { rn } | Ocbwb { rn } => {
            read_general(accesses, at, rn);
            let register = general_location(at.general_bank, rn);
            access(
                accesses,
                Location::CacheLine(register),
                if matches!(instruction, Pref { .. }) {
                    AccessKind::Read
                } else {
                    AccessKind::Write
                },
            );
            if matches!(instruction, Ocbp { .. } | Ocbwb { .. }) {
                write(
                    accesses,
                    Location::Memory(MemoryLocation {
                        addressing: Addressing::Register(register),
                        byte_len: 32,
                    }),
                );
            }
            if matches!(instruction, Pref { .. }) {
                access_with_occurrence(
                    accesses,
                    Location::Memory(MemoryLocation {
                        addressing: Addressing::Register(register),
                        byte_len: 32,
                    }),
                    AccessKind::Write,
                    AccessOccurrence::Conditional,
                );
            }
        }
        MovcaL { rn } => {
            read_general(accesses, at, Register::R0);
            read_general(accesses, at, rn);
            write(accesses, memory_register(at, rn, 4));
            write(
                accesses,
                Location::CacheLine(general_location(at.general_bank, rn)),
            );
        }
        Fldi0 { frn } | Fldi1 { frn } => write_float(accesses, at, frn),
        Fmov { frm, frn } => {
            read_float(accesses, at, frm);
            write_float(accesses, at, frn);
        }
        FmovLoad { rm, frn } => float_load(
            at,
            rm,
            FloatView::Single(frn),
            4,
            AddressKind::Register,
            accesses,
        ),
        FmovLoadIndexed { rm, frn } => float_load(
            at,
            rm,
            FloatView::Single(frn),
            4,
            AddressKind::Indexed,
            accesses,
        ),
        FmovLoadPostInc { rm, frn } => float_load(
            at,
            rm,
            FloatView::Single(frn),
            4,
            AddressKind::PostIncrement,
            accesses,
        ),
        FmovStore { frm, rn } => float_store(
            at,
            FloatView::Single(frm),
            rn,
            4,
            AddressKind::Register,
            accesses,
        ),
        FmovStoreIndexed { frm, rn } => float_store(
            at,
            FloatView::Single(frm),
            rn,
            4,
            AddressKind::Indexed,
            accesses,
        ),
        FmovStorePreDec { frm, rn } => float_store(
            at,
            FloatView::Single(frm),
            rn,
            4,
            AddressKind::PreDecrement,
            accesses,
        ),
        Flds { frm } => {
            read_float(accesses, at, frm);
            write(accesses, Location::Fpul);
        }
        Fsts { frn } => {
            read(accesses, Location::Fpul);
            write_float(accesses, at, frn);
        }
        Fabs { frn } | Fneg { frn } => read_write_float(accesses, at, frn),
        Fadd { frm, frn } | Fdiv { frm, frn } | Fmul { frm, frn } | Fsub { frm, frn } => {
            read_float(accesses, at, frm);
            read_write_float(accesses, at, frn);
            write_fpu_status(accesses);
        }
        FcmpEq { frm, frn } | FcmpGt { frm, frn } => {
            read_float(accesses, at, frm);
            read_float(accesses, at, frn);
            write_sr(accesses, SrField::T);
            write_fpu_status(accesses);
        }
        Float { frn } => {
            read(accesses, Location::Fpul);
            write_float(accesses, at, frn);
            write_fpu_status(accesses);
        }
        Fmac { frm, frn } => {
            read_float(accesses, at, FloatRegister::new(0).unwrap());
            read_float(accesses, at, frm);
            read_write_float(accesses, at, frn);
            write_fpu_status(accesses);
        }
        Fsqrt { frn } => {
            read_write_float(accesses, at, frn);
            write_fpu_status(accesses);
        }
        Ftrc { frm } => {
            read_float(accesses, at, frm);
            write(accesses, Location::Fpul);
            write_fpu_status(accesses);
        }
        FmovPair {
            source,
            destination,
        } => {
            read_pair_float(accesses, at, source);
            write_pair_float(accesses, at, destination);
        }
        FmovPairLoad { rm, destination } => float_load(
            at,
            rm,
            FloatView::Pair(destination),
            8,
            AddressKind::Register,
            accesses,
        ),
        FmovPairLoadIndexed { rm, destination } => float_load(
            at,
            rm,
            FloatView::Pair(destination),
            8,
            AddressKind::Indexed,
            accesses,
        ),
        FmovPairLoadPostInc { rm, destination } => float_load(
            at,
            rm,
            FloatView::Pair(destination),
            8,
            AddressKind::PostIncrement,
            accesses,
        ),
        FmovPairStore { source, rn } => float_store(
            at,
            FloatView::Pair(source),
            rn,
            8,
            AddressKind::Register,
            accesses,
        ),
        FmovPairStoreIndexed { source, rn } => float_store(
            at,
            FloatView::Pair(source),
            rn,
            8,
            AddressKind::Indexed,
            accesses,
        ),
        FmovPairStorePreDec { source, rn } => float_store(
            at,
            FloatView::Pair(source),
            rn,
            8,
            AddressKind::PreDecrement,
            accesses,
        ),
        FabsDouble { drn } | FnegDouble { drn } => read_write_double(accesses, at, drn),
        FaddDouble { drm, drn }
        | FdivDouble { drm, drn }
        | FmulDouble { drm, drn }
        | FsubDouble { drm, drn } => {
            read_double(accesses, at, drm);
            read_write_double(accesses, at, drn);
            write_fpu_status(accesses);
        }
        FcmpEqDouble { drm, drn } | FcmpGtDouble { drm, drn } => {
            read_double(accesses, at, drm);
            read_double(accesses, at, drn);
            write_sr(accesses, SrField::T);
            write_fpu_status(accesses);
        }
        Fcnvds { drm } => {
            read_double(accesses, at, drm);
            write(accesses, Location::Fpul);
            write_fpu_status(accesses);
        }
        Fcnvsd { drn } | FloatDouble { drn } => {
            read(accesses, Location::Fpul);
            write_double(accesses, at, drn);
            write_fpu_status(accesses);
        }
        FsqrtDouble { drn } => {
            read_write_double(accesses, at, drn);
            write_fpu_status(accesses);
        }
        FtrcDouble { drm } => {
            read_double(accesses, at, drm);
            write(accesses, Location::Fpul);
            write_fpu_status(accesses);
        }
        Fipr { fvm, fvn } => {
            read_vector(accesses, at, fvm);
            read_vector(accesses, at, fvn);
            write_float_lane(accesses, at, fvn.index() + 3, false);
            write_fpu_status(accesses);
        }
        Ftrv { fvn } => {
            read_vector(accesses, at, fvn);
            for index in 0..16 {
                read_float_lane(accesses, at, index, true);
            }
            write_vector(accesses, at, fvn);
            write_fpu_status(accesses);
        }
        Frchg => {
            read(accesses, Location::FpscrField(FpscrField::RegisterBank));
            write(accesses, Location::FpscrField(FpscrField::RegisterBank));
        }
        Fschg => {
            read(accesses, Location::FpscrField(FpscrField::TransferSize));
            write(accesses, Location::FpscrField(FpscrField::TransferSize));
        }
    }
}

fn general_location(bank: GeneralRegisterBank, register: Register) -> GeneralRegisterLocation {
    if register.index() >= 8 {
        return GeneralRegisterLocation::Primary(register);
    }
    match bank {
        GeneralRegisterBank::Primary => GeneralRegisterLocation::Primary(register),
        GeneralRegisterBank::Banked => {
            GeneralRegisterLocation::Banked(BankedRegister::new(register.index()).unwrap())
        }
        GeneralRegisterBank::Unknown => GeneralRegisterLocation::CurrentUnknown(register),
    }
}

fn opposite_general_location(
    bank: GeneralRegisterBank,
    register: BankedRegister,
) -> GeneralRegisterLocation {
    match bank {
        GeneralRegisterBank::Primary => GeneralRegisterLocation::Banked(register),
        GeneralRegisterBank::Banked => GeneralRegisterLocation::Primary(
            Register::new(register.index()).expect("banked register index is below eight"),
        ),
        GeneralRegisterBank::Unknown => GeneralRegisterLocation::OppositeUnknown(
            Register::new(register.index()).expect("banked register index is below eight"),
        ),
    }
}

fn read_general(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    register: Register,
) {
    read(
        accesses,
        Location::General(general_location(at.general_bank, register)),
    );
}
fn write_general(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    register: Register,
) {
    write(
        accesses,
        Location::General(general_location(at.general_bank, register)),
    );
}
fn read_write_general(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    register: Register,
) {
    read_general(accesses, at, register);
    write_general(accesses, at, register);
}
fn read_sr(accesses: &mut Vec<LocationAccess<Location>>, field: SrField) {
    read(accesses, Location::SrField(field));
}
fn write_sr(accesses: &mut Vec<LocationAccess<Location>>, field: SrField) {
    write(accesses, Location::SrField(field));
}
fn write_sr_many(accesses: &mut Vec<LocationAccess<Location>>, fields: &[SrField]) {
    for field in fields {
        write_sr(accesses, *field);
    }
}

fn memory_register(at: AnalysisPoint, register: Register, byte_len: u8) -> Location {
    Location::Memory(MemoryLocation {
        addressing: Addressing::Register(general_location(at.general_bank, register)),
        byte_len,
    })
}

fn pc_relative_load(
    instruction: &Instruction,
    at: AnalysisPoint,
    register: Register,
    byte_len: u8,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    let address = literal_address(instruction, at.location);
    read(
        accesses,
        Location::Memory(MemoryLocation {
            addressing: Addressing::PcRelative(address),
            byte_len,
        }),
    );
    write_general(accesses, at, register);
    references.push(StaticReference {
        target: Reference::Memory { address, byte_len },
        kind: ReferenceKind::Read,
        occurrence: AccessOccurrence::Always,
    });
}

fn literal_address(instruction: &Instruction, location: u32) -> u32 {
    match *instruction {
        Instruction::MovWPcRel { disp, .. } => {
            location.wrapping_add(4).wrapping_add(u32::from(disp) * 2)
        }
        Instruction::MovLPcRel { disp, .. } | Instruction::Mova { disp } => (location & !3)
            .wrapping_add(4)
            .wrapping_add(u32::from(disp) * 4),
        _ => unreachable!(),
    }
}

fn register_store(
    at: AnalysisPoint,
    source: Register,
    base: Register,
    byte_len: u8,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_general(accesses, at, source);
    read_general(accesses, at, base);
    write(accesses, memory_register(at, base, byte_len));
}
fn register_load(
    at: AnalysisPoint,
    base: Register,
    destination: Register,
    byte_len: u8,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_general(accesses, at, base);
    read(accesses, memory_register(at, base, byte_len));
    write_general(accesses, at, destination);
}
fn predecrement_store(
    at: AnalysisPoint,
    source: Register,
    base: Register,
    byte_len: u8,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_general(accesses, at, source);
    read_write_general(accesses, at, base);
    write(
        accesses,
        Location::Memory(MemoryLocation {
            addressing: Addressing::PreDecrement(general_location(at.general_bank, base)),
            byte_len,
        }),
    );
}
fn postincrement_load(
    at: AnalysisPoint,
    base: Register,
    destination: Register,
    byte_len: u8,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_general(accesses, at, base);
    read(
        accesses,
        Location::Memory(MemoryLocation {
            addressing: Addressing::PostIncrement(general_location(at.general_bank, base)),
            byte_len,
        }),
    );
    write_general(accesses, at, destination);
    if base != destination {
        write_general(accesses, at, base);
    }
}
fn indexed_load(
    at: AnalysisPoint,
    base: Register,
    destination: Register,
    byte_len: u8,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_general(accesses, at, Register::R0);
    read_general(accesses, at, base);
    read(
        accesses,
        Location::Memory(MemoryLocation {
            addressing: Addressing::IndexedR0(general_location(at.general_bank, base)),
            byte_len,
        }),
    );
    write_general(accesses, at, destination);
}
fn indexed_store(
    at: AnalysisPoint,
    source: Register,
    base: Register,
    byte_len: u8,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_general(accesses, at, Register::R0);
    read_general(accesses, at, source);
    read_general(accesses, at, base);
    write(
        accesses,
        Location::Memory(MemoryLocation {
            addressing: Addressing::IndexedR0(general_location(at.general_bank, base)),
            byte_len,
        }),
    );
}
fn displacement_store(
    at: AnalysisPoint,
    source: Register,
    base: Register,
    displacement: u8,
    byte_len: u8,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_general(accesses, at, source);
    read_general(accesses, at, base);
    write(
        accesses,
        Location::Memory(MemoryLocation {
            addressing: Addressing::Displacement {
                base: general_location(at.general_bank, base),
                displacement: u16::from(displacement) * u16::from(byte_len),
            },
            byte_len,
        }),
    );
}
fn displacement_load(
    at: AnalysisPoint,
    base: Register,
    destination: Register,
    displacement: u8,
    byte_len: u8,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_general(accesses, at, base);
    read(
        accesses,
        Location::Memory(MemoryLocation {
            addressing: Addressing::Displacement {
                base: general_location(at.general_bank, base),
                displacement: u16::from(displacement) * u16::from(byte_len),
            },
            byte_len,
        }),
    );
    write_general(accesses, at, destination);
}
fn gbr_transfer(
    at: AnalysisPoint,
    displacement: u8,
    byte_len: u8,
    kind: AccessKind,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read(accesses, Location::Gbr);
    if kind == AccessKind::Write {
        read_general(accesses, at, Register::R0);
    }
    access(
        accesses,
        Location::Memory(MemoryLocation {
            addressing: Addressing::GbrDisplacement(u16::from(displacement) * u16::from(byte_len)),
            byte_len,
        }),
        kind,
    );
    if kind == AccessKind::Read {
        write_general(accesses, at, Register::R0);
    }
}

fn mac(
    at: AnalysisPoint,
    left: Register,
    right: Register,
    byte_len: u8,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    for register in [left, right] {
        read_write_general(accesses, at, register);
        read(
            accesses,
            Location::Memory(MemoryLocation {
                addressing: Addressing::PostIncrement(general_location(at.general_bank, register)),
                byte_len,
            }),
        );
    }
    read(accesses, Location::Mach);
    read(accesses, Location::Macl);
    read_sr(accesses, SrField::S);
    write(accesses, Location::Mach);
    write(accesses, Location::Macl);
}

fn register_to_special(
    at: AnalysisPoint,
    register: Register,
    special: Location,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_general(accesses, at, register);
    write(accesses, special);
}
fn special_to_register(
    at: AnalysisPoint,
    special: Location,
    register: Register,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read(accesses, special);
    write_general(accesses, at, register);
}
fn load_special(
    at: AnalysisPoint,
    register: Register,
    special: Location,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_write_general(accesses, at, register);
    read(
        accesses,
        Location::Memory(MemoryLocation {
            addressing: Addressing::PostIncrement(general_location(at.general_bank, register)),
            byte_len: 4,
        }),
    );
    write(accesses, special);
}
fn store_special(
    at: AnalysisPoint,
    special: Location,
    register: Register,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read(accesses, special);
    read_write_general(accesses, at, register);
    write(
        accesses,
        Location::Memory(MemoryLocation {
            addressing: Addressing::PreDecrement(general_location(at.general_bank, register)),
            byte_len: 4,
        }),
    );
}

#[derive(Clone, Copy)]
enum FloatView {
    Single(FloatRegister),
    Pair(FpuPairRegister),
}
#[derive(Clone, Copy)]
enum AddressKind {
    Register,
    Indexed,
    PostIncrement,
    PreDecrement,
}

fn float_load(
    at: AnalysisPoint,
    base: Register,
    destination: FloatView,
    byte_len: u8,
    address_kind: AddressKind,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_general(accesses, at, base);
    if matches!(address_kind, AddressKind::Indexed) {
        read_general(accesses, at, Register::R0);
    }
    let base_location = general_location(at.general_bank, base);
    let addressing = match address_kind {
        AddressKind::Register => Addressing::Register(base_location),
        AddressKind::Indexed => Addressing::IndexedR0(base_location),
        AddressKind::PostIncrement => Addressing::PostIncrement(base_location),
        AddressKind::PreDecrement => unreachable!(),
    };
    read(
        accesses,
        Location::Memory(MemoryLocation {
            addressing,
            byte_len,
        }),
    );
    match destination {
        FloatView::Single(register) => write_float(accesses, at, register),
        FloatView::Pair(register) => write_pair_float(accesses, at, register),
    }
    if matches!(address_kind, AddressKind::PostIncrement) {
        write_general(accesses, at, base);
    }
}

fn float_store(
    at: AnalysisPoint,
    source: FloatView,
    base: Register,
    byte_len: u8,
    address_kind: AddressKind,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    match source {
        FloatView::Single(register) => read_float(accesses, at, register),
        FloatView::Pair(register) => read_pair_float(accesses, at, register),
    }
    read_general(accesses, at, base);
    if matches!(address_kind, AddressKind::Indexed) {
        read_general(accesses, at, Register::R0);
    }
    let base_location = general_location(at.general_bank, base);
    let addressing = match address_kind {
        AddressKind::Register => Addressing::Register(base_location),
        AddressKind::Indexed => Addressing::IndexedR0(base_location),
        AddressKind::PreDecrement => {
            write_general(accesses, at, base);
            Addressing::PreDecrement(base_location)
        }
        AddressKind::PostIncrement => unreachable!(),
    };
    write(
        accesses,
        Location::Memory(MemoryLocation {
            addressing,
            byte_len,
        }),
    );
}

fn current_float_bank(bank: FloatBank, opposite: bool) -> FloatLaneBank {
    match (bank, opposite) {
        (FloatBank::Primary, false) | (FloatBank::Alternate, true) => FloatLaneBank::Primary,
        (FloatBank::Primary, true) | (FloatBank::Alternate, false) => FloatLaneBank::Extended,
        (FloatBank::Unknown, false) => FloatLaneBank::CurrentUnknown,
        (FloatBank::Unknown, true) => FloatLaneBank::OppositeUnknown,
    }
}

fn float_lane(at: AnalysisPoint, index: u8, opposite: bool) -> Location {
    Location::FloatLane {
        bank: current_float_bank(at.float_bank, opposite),
        index,
    }
}
fn read_float_lane(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    index: u8,
    opposite: bool,
) {
    read(accesses, float_lane(at, index, opposite));
}
fn write_float_lane(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    index: u8,
    opposite: bool,
) {
    write(accesses, float_lane(at, index, opposite));
}
fn read_float(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    register: FloatRegister,
) {
    read_float_lane(accesses, at, register.index(), false);
}
fn write_float(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    register: FloatRegister,
) {
    write_float_lane(accesses, at, register.index(), false);
}
fn read_write_float(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    register: FloatRegister,
) {
    read_float(accesses, at, register);
    write_float(accesses, at, register);
}
fn read_double(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    register: DoubleRegister,
) {
    for index in register.index()..register.index() + 2 {
        read_float_lane(accesses, at, index, false);
    }
}
fn write_double(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    register: DoubleRegister,
) {
    for index in register.index()..register.index() + 2 {
        write_float_lane(accesses, at, index, false);
    }
}
fn read_write_double(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    register: DoubleRegister,
) {
    read_double(accesses, at, register);
    write_double(accesses, at, register);
}
fn pair_parts(register: FpuPairRegister) -> (u8, bool) {
    match register {
        FpuPairRegister::Double(register) => (register.index(), false),
        FpuPairRegister::Extended(register) => (register.index(), true),
    }
}
fn read_pair_float(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    register: FpuPairRegister,
) {
    let (first, opposite) = pair_parts(register);
    for index in first..first + 2 {
        read_float_lane(accesses, at, index, opposite);
    }
}
fn write_pair_float(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    register: FpuPairRegister,
) {
    let (first, opposite) = pair_parts(register);
    for index in first..first + 2 {
        write_float_lane(accesses, at, index, opposite);
    }
}
fn read_vector(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    register: VectorRegister,
) {
    for index in register.index()..register.index() + 4 {
        read_float_lane(accesses, at, index, false);
    }
}
fn write_vector(
    accesses: &mut Vec<LocationAccess<Location>>,
    at: AnalysisPoint,
    register: VectorRegister,
) {
    for index in register.index()..register.index() + 4 {
        write_float_lane(accesses, at, index, false);
    }
}
fn write_fpu_status(accesses: &mut Vec<LocationAccess<Location>>) {
    write(accesses, Location::FpscrField(FpscrField::Cause));
    write(accesses, Location::FpscrField(FpscrField::Flag));
}

fn read(accesses: &mut Vec<LocationAccess<Location>>, location: Location) {
    access(accesses, location, AccessKind::Read);
}
fn write(accesses: &mut Vec<LocationAccess<Location>>, location: Location) {
    access(accesses, location, AccessKind::Write);
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
        (Location::Sr, Location::SrField(_))
        | (Location::SrField(_), Location::Sr)
        | (Location::Fpscr, Location::FpscrField(_))
        | (Location::FpscrField(_), Location::Fpscr) => Overlapping,
        (
            Location::General(GeneralRegisterLocation::CurrentUnknown(left)),
            Location::General(GeneralRegisterLocation::Primary(right)),
        )
        | (
            Location::General(GeneralRegisterLocation::Primary(right)),
            Location::General(GeneralRegisterLocation::CurrentUnknown(left)),
        ) if left == right && left.index() < 8 => Unknown,
        (
            Location::General(GeneralRegisterLocation::CurrentUnknown(left)),
            Location::General(GeneralRegisterLocation::Banked(right)),
        )
        | (
            Location::General(GeneralRegisterLocation::Banked(right)),
            Location::General(GeneralRegisterLocation::CurrentUnknown(left)),
        ) if left.index() == right.index() => Unknown,
        (
            Location::General(GeneralRegisterLocation::OppositeUnknown(left)),
            Location::General(GeneralRegisterLocation::Primary(right)),
        )
        | (
            Location::General(GeneralRegisterLocation::Primary(right)),
            Location::General(GeneralRegisterLocation::OppositeUnknown(left)),
        ) if left == right && left.index() < 8 => Unknown,
        (
            Location::General(GeneralRegisterLocation::OppositeUnknown(left)),
            Location::General(GeneralRegisterLocation::Banked(right)),
        )
        | (
            Location::General(GeneralRegisterLocation::Banked(right)),
            Location::General(GeneralRegisterLocation::OppositeUnknown(left)),
        ) if left.index() == right.index() => Unknown,
        (
            Location::FloatLane {
                bank: FloatLaneBank::CurrentUnknown,
                index: left,
            },
            Location::FloatLane {
                bank: FloatLaneBank::Primary | FloatLaneBank::Extended,
                index: right,
            },
        )
        | (
            Location::FloatLane {
                bank: FloatLaneBank::Primary | FloatLaneBank::Extended,
                index: right,
            },
            Location::FloatLane {
                bank: FloatLaneBank::CurrentUnknown,
                index: left,
            },
        ) if left == right => Unknown,
        (
            Location::FloatLane {
                bank: FloatLaneBank::OppositeUnknown,
                index: left,
            },
            Location::FloatLane {
                bank: FloatLaneBank::Primary | FloatLaneBank::Extended,
                index: right,
            },
        )
        | (
            Location::FloatLane {
                bank: FloatLaneBank::Primary | FloatLaneBank::Extended,
                index: right,
            },
            Location::FloatLane {
                bank: FloatLaneBank::OppositeUnknown,
                index: left,
            },
        ) if left == right => Unknown,
        (Location::Memory(left), Location::Memory(right)) => {
            match (left.addressing, right.addressing) {
                (Addressing::PcRelative(left_address), Addressing::PcRelative(right_address)) => {
                    if ranges_overlap(left_address, left.byte_len, right_address, right.byte_len) {
                        Overlapping
                    } else {
                        Disjoint
                    }
                }
                _ => Unknown,
            }
        }
        (Location::Memory(_), _)
        | (_, Location::Memory(_))
        | (Location::CacheLine(_), Location::CacheLine(_)) => Unknown,
        (Location::CacheLine(_), _) | (_, Location::CacheLine(_)) => Disjoint,
        _ => Disjoint,
    }
}

fn ranges_overlap(left: u32, left_len: u8, right: u32, right_len: u8) -> bool {
    (0..u32::from(left_len)).any(|offset| {
        let address = left.wrapping_add(offset);
        (0..u32::from(right_len)).any(|other| address == right.wrapping_add(other))
    })
}
