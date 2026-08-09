//! Complete static meaning for the Hitachi SH-2 profile.

use core::fmt;

use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlBoundary, ControlTarget, DelaySlot,
    DelaySlotExecution, LocationAccess, ReferenceKind, StaticReference, StorageRelation,
};

use crate::{CodeLocation, Instruction, Register};

/// Runtime-derived SH-2 control destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndirectTarget {
    /// Destination read directly from a general register.
    Register(Register),
    /// Destination formed as `PC + 4 + Rm`.
    PcRelativeRegister(Register),
    /// Destination read from the procedure register.
    ProcedureRegister,
    /// Destination and status restored from the exception frame at R15.
    ExceptionFrame,
    /// Trap vector formed from VBR and the encoded immediate.
    TrapVector(u8),
}

/// SH-2 synchronous control boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Boundary {
    /// Software trap with the encoded vector number.
    Trap(u8),
    /// SLEEP wait state.
    Sleep,
}

/// Individually meaningful SH-2 status-register field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SrField {
    /// Trace or condition bit T.
    T,
    /// Division quotient bit Q.
    Q,
    /// Division sign bit M.
    M,
    /// Multiply-accumulate saturation bit S.
    S,
    /// Interrupt mask field I3-I0.
    InterruptMask,
}

/// Addressing rule for an SH-2 symbolic memory access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Addressing {
    /// Address held in one general register.
    Register(Register),
    /// Address after decrementing the named register by the access width.
    PreDecrement(Register),
    /// Address held in the register before its post-increment.
    PostIncrement(Register),
    /// Sum of R0 and the named general register.
    IndexedR0(Register),
    /// Named register plus the already scaled byte displacement.
    Displacement {
        /// Base register.
        base: Register,
        /// Unsigned byte displacement.
        displacement: u16,
    },
    /// GBR plus the already scaled byte displacement.
    GbrDisplacement(u16),
    /// Sum of GBR and R0.
    GbrIndexedR0,
    /// Placed PC-relative absolute address.
    PcRelative(u32),
    /// Two-longword exception frame addressed through R15.
    ExceptionFrame,
}

/// Symbolic SH-2 memory storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryLocation {
    /// Addressing rule established by the instruction.
    pub addressing: Addressing,
    /// Access width in bytes.
    pub byte_len: u8,
}

/// Architectural SH-2 storage location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Location {
    /// General-purpose register storage.
    Register(Register),
    /// Whole status register.
    Sr,
    /// Individually meaningful status-register field.
    SrField(SrField),
    /// Global base register.
    Gbr,
    /// Vector base register.
    Vbr,
    /// Multiply-accumulate high register.
    Mach,
    /// Multiply-accumulate low register.
    Macl,
    /// Procedure register.
    Pr,
    /// Symbolic memory storage.
    Memory(MemoryLocation),
}

/// Statically resolved non-control SH-2 reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reference {
    /// Literal-pool bytes at a placed PC-relative address.
    Memory {
        /// First referenced byte.
        address: u32,
        /// Referenced width in bytes.
        byte_len: u8,
    },
    /// Address value formed by MOVA without claiming a memory access.
    Address(u32),
}

/// Structural failure to form sound SH-2 static facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticError {
    /// The instruction location is not halfword aligned.
    UnalignedLocation {
        /// Rejected code location.
        location: CodeLocation,
    },
}

impl fmt::Display for SemanticError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnalignedLocation { location } => write!(
                formatter,
                "SH-2 instruction location {location:#010x} is not halfword aligned"
            ),
        }
    }
}

impl std::error::Error for SemanticError {}

/// Shared control-flow result specialized with SH-2 vocabulary.
pub type ControlFlow = typed_isa_core::ControlFlow<CodeLocation, IndirectTarget, Boundary>;

/// Shared instruction-semantics result specialized with SH-2 vocabulary.
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
    let mut location_accesses = Vec::new();
    let mut static_references = Vec::new();
    add_instruction_accesses(
        instruction,
        at,
        &mut location_accesses,
        &mut static_references,
    );
    Ok(InstructionSemantics {
        control_flow: control_flow(instruction, at),
        location_accesses,
        static_references,
    })
}

fn control_flow(instruction: &Instruction, at: CodeLocation) -> ControlFlow {
    let next = at.wrapping_add(2);
    let after_slot = at.wrapping_add(4);
    let slot = || {
        Some(DelaySlot {
            point: next,
            execution: DelaySlotExecution::Always,
        })
    };
    match *instruction {
        Instruction::Bt { .. } | Instruction::Bf { .. } => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Direct(
                    instruction.branch_target(at).expect("direct SH-2 branch"),
                ),
            },
            fallthrough: Some(next),
            delay_slot: None,
        },
        Instruction::Bts { .. } | Instruction::Bfs { .. } => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Direct(
                    instruction.branch_target(at).expect("direct SH-2 branch"),
                ),
            },
            fallthrough: Some(after_slot),
            delay_slot: slot(),
        },
        Instruction::Bra { .. } => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Direct(
                    instruction.branch_target(at).expect("direct SH-2 branch"),
                ),
            },
            fallthrough: None,
            delay_slot: slot(),
        },
        Instruction::Bsr { .. } => ControlFlow {
            action: ControlAction::LinkedTransfer {
                target: ControlTarget::Direct(
                    instruction.branch_target(at).expect("direct SH-2 branch"),
                ),
                return_site: after_slot,
            },
            fallthrough: None,
            delay_slot: slot(),
        },
        Instruction::Braf { rm } => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Indirect(IndirectTarget::PcRelativeRegister(rm)),
            },
            fallthrough: None,
            delay_slot: slot(),
        },
        Instruction::Bsrf { rm } => ControlFlow {
            action: ControlAction::LinkedTransfer {
                target: ControlTarget::Indirect(IndirectTarget::PcRelativeRegister(rm)),
                return_site: after_slot,
            },
            fallthrough: None,
            delay_slot: slot(),
        },
        Instruction::Jmp { rm } => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Indirect(IndirectTarget::Register(rm)),
            },
            fallthrough: None,
            delay_slot: slot(),
        },
        Instruction::Jsr { rm } => ControlFlow {
            action: ControlAction::LinkedTransfer {
                target: ControlTarget::Indirect(IndirectTarget::Register(rm)),
                return_site: after_slot,
            },
            fallthrough: None,
            delay_slot: slot(),
        },
        Instruction::Rts => ControlFlow {
            action: ControlAction::Return {
                target: IndirectTarget::ProcedureRegister,
            },
            fallthrough: None,
            delay_slot: slot(),
        },
        Instruction::Rte => ControlFlow {
            action: ControlAction::ExceptionReturn {
                target: IndirectTarget::ExceptionFrame,
            },
            fallthrough: None,
            delay_slot: slot(),
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

fn add_instruction_accesses(
    instruction: &Instruction,
    at: CodeLocation,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    use Instruction::*;
    match *instruction {
        MovImm { rn, .. } => write_register(accesses, rn),
        MovWPcRel { rn, .. } => add_pc_relative_load(instruction, at, rn, 2, accesses, references),
        MovLPcRel { rn, .. } => add_pc_relative_load(instruction, at, rn, 4, accesses, references),
        MovReg { rm, rn } => {
            read_register(accesses, rm);
            write_register(accesses, rn);
        }
        MovBStore { rm, rn } => add_register_store(rm, rn, 1, accesses),
        MovWStore { rm, rn } => add_register_store(rm, rn, 2, accesses),
        MovLStore { rm, rn } => add_register_store(rm, rn, 4, accesses),
        MovBLoad { rm, rn } => add_register_load(rm, rn, 1, accesses),
        MovWLoad { rm, rn } => add_register_load(rm, rn, 2, accesses),
        MovLLoad { rm, rn } => add_register_load(rm, rn, 4, accesses),
        MovBStorePreDec { rm, rn } => add_predecrement_store(rm, rn, 1, accesses),
        MovWStorePreDec { rm, rn } => add_predecrement_store(rm, rn, 2, accesses),
        MovLStorePreDec { rm, rn } => add_predecrement_store(rm, rn, 4, accesses),
        MovBLoadPostInc { rm, rn } => add_postincrement_load(rm, rn, 1, accesses),
        MovWLoadPostInc { rm, rn } => add_postincrement_load(rm, rn, 2, accesses),
        MovLLoadPostInc { rm, rn } => add_postincrement_load(rm, rn, 4, accesses),
        MovBR0LoadIndexed { rm, rn } => add_r0_indexed_load(rm, rn, 1, accesses),
        MovWR0LoadIndexed { rm, rn } => add_r0_indexed_load(rm, rn, 2, accesses),
        MovLR0LoadIndexed { rm, rn } => add_r0_indexed_load(rm, rn, 4, accesses),
        MovBR0StoreIndexed { rm, rn } => add_r0_indexed_store(rm, rn, 1, accesses),
        MovWR0StoreIndexed { rm, rn } => add_r0_indexed_store(rm, rn, 2, accesses),
        MovLR0StoreIndexed { rm, rn } => add_r0_indexed_store(rm, rn, 4, accesses),
        MovBDispStore { disp, rn } => add_displacement_store(Register::R0, rn, disp, 1, accesses),
        MovWDispStore { disp, rn } => add_displacement_store(Register::R0, rn, disp, 2, accesses),
        MovLDispStore { disp, rm, rn } => add_displacement_store(rm, rn, disp, 4, accesses),
        MovBDispLoad { disp, rm } => add_displacement_load(rm, Register::R0, disp, 1, accesses),
        MovWDispLoad { disp, rm } => add_displacement_load(rm, Register::R0, disp, 2, accesses),
        MovLDispLoad { disp, rm, rn } => add_displacement_load(rm, rn, disp, 4, accesses),
        MovBGbrStore { disp } => add_gbr_transfer(disp, 1, AccessKind::Write, accesses),
        MovWGbrStore { disp } => add_gbr_transfer(disp, 2, AccessKind::Write, accesses),
        MovLGbrStore { disp } => add_gbr_transfer(disp, 4, AccessKind::Write, accesses),
        MovBGbrLoad { disp } => add_gbr_transfer(disp, 1, AccessKind::Read, accesses),
        MovWGbrLoad { disp } => add_gbr_transfer(disp, 2, AccessKind::Read, accesses),
        MovLGbrLoad { disp } => add_gbr_transfer(disp, 4, AccessKind::Read, accesses),
        Mova { .. } => {
            write_register(accesses, Register::R0);
            references.push(StaticReference {
                target: Reference::Address(
                    instruction.literal_pool_address(at).expect("MOVA address"),
                ),
                kind: ReferenceKind::Address,
                occurrence: AccessOccurrence::Always,
            });
        }
        MovT { rn } => {
            read_sr_field(accesses, SrField::T);
            write_register(accesses, rn);
        }
        Add { rm, rn }
        | Sub { rm, rn }
        | And { rm, rn }
        | Or { rm, rn }
        | Xor { rm, rn }
        | Xtrct { rm, rn } => {
            read_register(accesses, rm);
            read_register(accesses, rn);
            write_register(accesses, rn);
        }
        AddImm { rn, .. } => read_write_register(accesses, rn),
        Addc { rm, rn } | Subc { rm, rn } => {
            read_register(accesses, rm);
            read_write_register(accesses, rn);
            read_sr_field(accesses, SrField::T);
            write_sr_field(accesses, SrField::T);
        }
        Addv { rm, rn } | Subv { rm, rn } => {
            read_register(accesses, rm);
            read_write_register(accesses, rn);
            write_sr_field(accesses, SrField::T);
        }
        Neg { rm, rn } => {
            read_register(accesses, rm);
            write_register(accesses, rn);
        }
        Negc { rm, rn } => {
            read_register(accesses, rm);
            read_sr_field(accesses, SrField::T);
            write_register(accesses, rn);
            write_sr_field(accesses, SrField::T);
        }
        Dt { rn } => {
            read_write_register(accesses, rn);
            write_sr_field(accesses, SrField::T);
        }
        MulL { rm, rn } | MulS { rm, rn } | MulU { rm, rn } => {
            read_register(accesses, rm);
            read_register(accesses, rn);
            write(accesses, Location::Macl);
        }
        DmulS { rm, rn } | DmulU { rm, rn } => {
            read_register(accesses, rm);
            read_register(accesses, rn);
            write(accesses, Location::Mach);
            write(accesses, Location::Macl);
        }
        Div0S { rm, rn } => {
            read_register(accesses, rm);
            read_register(accesses, rn);
            write_sr_fields(accesses, &[SrField::Q, SrField::M, SrField::T]);
        }
        Div0U => write_sr_fields(accesses, &[SrField::Q, SrField::M, SrField::T]),
        Div1 { rm, rn } => {
            read_register(accesses, rm);
            read_write_register(accesses, rn);
            for field in [SrField::Q, SrField::M, SrField::T] {
                read_sr_field(accesses, field);
            }
            write_sr_fields(accesses, &[SrField::Q, SrField::T]);
        }
        MacL { rm, rn } => add_mac(rm, rn, 4, accesses),
        MacW { rm, rn } => add_mac(rm, rn, 2, accesses),
        AndImm { .. } | OrImm { .. } | XorImm { .. } => read_write_register(accesses, Register::R0),
        TstImm { .. } => {
            read_register(accesses, Register::R0);
            write_sr_field(accesses, SrField::T);
        }
        AndB { .. } | OrB { .. } | XorB { .. } | TstB { .. } => {
            read(accesses, Location::Gbr);
            read_register(accesses, Register::R0);
            let memory = Location::Memory(MemoryLocation {
                addressing: Addressing::GbrIndexedR0,
                byte_len: 1,
            });
            read(accesses, memory);
            if !matches!(instruction, TstB { .. }) {
                write(accesses, memory);
            } else {
                write_sr_field(accesses, SrField::T);
            }
        }
        Not { rm, rn }
        | ExtsB { rm, rn }
        | ExtsW { rm, rn }
        | ExtuB { rm, rn }
        | ExtuW { rm, rn }
        | SwapB { rm, rn }
        | SwapW { rm, rn } => {
            read_register(accesses, rm);
            write_register(accesses, rn);
        }
        Tst { rm, rn }
        | CmpEq { rm, rn }
        | CmpGe { rm, rn }
        | CmpGt { rm, rn }
        | CmpHi { rm, rn }
        | CmpHs { rm, rn }
        | CmpStr { rm, rn } => {
            read_register(accesses, rm);
            read_register(accesses, rn);
            write_sr_field(accesses, SrField::T);
        }
        Shll { rn } | Shlr { rn } | Shal { rn } | Shar { rn } | Rotl { rn } | Rotr { rn } => {
            read_write_register(accesses, rn);
            write_sr_field(accesses, SrField::T);
        }
        Rotcl { rn } | Rotcr { rn } => {
            read_write_register(accesses, rn);
            read_sr_field(accesses, SrField::T);
            write_sr_field(accesses, SrField::T);
        }
        Shll2 { rn }
        | Shlr2 { rn }
        | Shll8 { rn }
        | Shlr8 { rn }
        | Shll16 { rn }
        | Shlr16 { rn } => read_write_register(accesses, rn),
        CmpEqImm { .. } => {
            read_register(accesses, Register::R0);
            write_sr_field(accesses, SrField::T);
        }
        CmpPl { rn } | CmpPz { rn } => {
            read_register(accesses, rn);
            write_sr_field(accesses, SrField::T);
        }
        Bt { .. } | Bf { .. } | Bts { .. } | Bfs { .. } => read_sr_field(accesses, SrField::T),
        Bra { .. } => {}
        Bsr { .. } => write(accesses, Location::Pr),
        Jmp { rm } | Braf { rm } => read_register(accesses, rm),
        Jsr { rm } | Bsrf { rm } => {
            read_register(accesses, rm);
            write(accesses, Location::Pr);
        }
        Rts => read(accesses, Location::Pr),
        Rte => {
            read_register(accesses, Register::SP);
            read(
                accesses,
                Location::Memory(MemoryLocation {
                    addressing: Addressing::ExceptionFrame,
                    byte_len: 8,
                }),
            );
            write_register(accesses, Register::SP);
            write(accesses, Location::Sr);
        }
        LdcGbr { rm } => transfer_register_to_special(rm, Location::Gbr, accesses),
        LdcVbr { rm } => transfer_register_to_special(rm, Location::Vbr, accesses),
        LdcSr { rm } => transfer_register_to_special(rm, Location::Sr, accesses),
        LdcLGbr { rm } => load_special_postincrement(rm, Location::Gbr, accesses),
        LdcLVbr { rm } => load_special_postincrement(rm, Location::Vbr, accesses),
        LdcLSr { rm } => load_special_postincrement(rm, Location::Sr, accesses),
        StcGbr { rn } => transfer_special_to_register(Location::Gbr, rn, accesses),
        StcVbr { rn } => transfer_special_to_register(Location::Vbr, rn, accesses),
        StcSr { rn } => transfer_special_to_register(Location::Sr, rn, accesses),
        StcLGbr { rn } => store_special_predecrement(Location::Gbr, rn, accesses),
        StcLVbr { rn } => store_special_predecrement(Location::Vbr, rn, accesses),
        StcLSr { rn } => store_special_predecrement(Location::Sr, rn, accesses),
        LdsMach { rm } => transfer_register_to_special(rm, Location::Mach, accesses),
        LdsMacl { rm } => transfer_register_to_special(rm, Location::Macl, accesses),
        LdsPr { rm } => transfer_register_to_special(rm, Location::Pr, accesses),
        LdsLMach { rm } => load_special_postincrement(rm, Location::Mach, accesses),
        LdsLMacl { rm } => load_special_postincrement(rm, Location::Macl, accesses),
        LdsLPr { rm } => load_special_postincrement(rm, Location::Pr, accesses),
        StsMach { rn } => transfer_special_to_register(Location::Mach, rn, accesses),
        StsMacl { rn } => transfer_special_to_register(Location::Macl, rn, accesses),
        StsPr { rn } => transfer_special_to_register(Location::Pr, rn, accesses),
        StsLMach { rn } => store_special_predecrement(Location::Mach, rn, accesses),
        StsLMacl { rn } => store_special_predecrement(Location::Macl, rn, accesses),
        StsLPr { rn } => store_special_predecrement(Location::Pr, rn, accesses),
        Nop | Sleep => {}
        SetT | ClrT => write_sr_field(accesses, SrField::T),
        ClrMac => {
            write(accesses, Location::Mach);
            write(accesses, Location::Macl);
        }
        Trapa { .. } => {
            read(accesses, Location::Vbr);
            read(accesses, Location::Sr);
            read_register(accesses, Register::SP);
            write_register(accesses, Register::SP);
            write(
                accesses,
                Location::Memory(MemoryLocation {
                    addressing: Addressing::ExceptionFrame,
                    byte_len: 8,
                }),
            );
        }
        Tas { rn } => {
            read_register(accesses, rn);
            let memory = Location::Memory(MemoryLocation {
                addressing: Addressing::Register(rn),
                byte_len: 1,
            });
            read(accesses, memory);
            write(accesses, memory);
            write_sr_field(accesses, SrField::T);
        }
    }
}

fn add_pc_relative_load(
    instruction: &Instruction,
    at: u32,
    rn: Register,
    byte_len: u8,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    let address = instruction
        .literal_pool_address(at)
        .expect("PC-relative load");
    read(
        accesses,
        Location::Memory(MemoryLocation {
            addressing: Addressing::PcRelative(address),
            byte_len,
        }),
    );
    write_register(accesses, rn);
    references.push(StaticReference {
        target: Reference::Memory { address, byte_len },
        kind: ReferenceKind::Read,
        occurrence: AccessOccurrence::Always,
    });
}

fn add_register_store(
    rm: Register,
    rn: Register,
    byte_len: u8,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_register(accesses, rm);
    read_register(accesses, rn);
    write(
        accesses,
        Location::Memory(MemoryLocation {
            addressing: Addressing::Register(rn),
            byte_len,
        }),
    );
}

fn add_register_load(
    rm: Register,
    rn: Register,
    byte_len: u8,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_register(accesses, rm);
    read(
        accesses,
        Location::Memory(MemoryLocation {
            addressing: Addressing::Register(rm),
            byte_len,
        }),
    );
    write_register(accesses, rn);
}

fn add_predecrement_store(
    rm: Register,
    rn: Register,
    byte_len: u8,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_register(accesses, rm);
    read_write_register(accesses, rn);
    write(
        accesses,
        Location::Memory(MemoryLocation {
            addressing: Addressing::PreDecrement(rn),
            byte_len,
        }),
    );
}

fn add_postincrement_load(
    rm: Register,
    rn: Register,
    byte_len: u8,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_register(accesses, rm);
    read(
        accesses,
        Location::Memory(MemoryLocation {
            addressing: Addressing::PostIncrement(rm),
            byte_len,
        }),
    );
    write_register(accesses, rn);
    if rm != rn {
        write_register(accesses, rm);
    }
}

fn add_r0_indexed_load(
    rm: Register,
    rn: Register,
    byte_len: u8,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_register(accesses, Register::R0);
    read_register(accesses, rm);
    read(
        accesses,
        Location::Memory(MemoryLocation {
            addressing: Addressing::IndexedR0(rm),
            byte_len,
        }),
    );
    write_register(accesses, rn);
}

fn add_r0_indexed_store(
    rm: Register,
    rn: Register,
    byte_len: u8,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_register(accesses, Register::R0);
    read_register(accesses, rm);
    read_register(accesses, rn);
    write(
        accesses,
        Location::Memory(MemoryLocation {
            addressing: Addressing::IndexedR0(rn),
            byte_len,
        }),
    );
}

fn add_displacement_store(
    source: Register,
    base: Register,
    disp: u8,
    byte_len: u8,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_register(accesses, source);
    read_register(accesses, base);
    write(
        accesses,
        Location::Memory(MemoryLocation {
            addressing: Addressing::Displacement {
                base,
                displacement: u16::from(disp) * u16::from(byte_len),
            },
            byte_len,
        }),
    );
}

fn add_displacement_load(
    base: Register,
    destination: Register,
    disp: u8,
    byte_len: u8,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_register(accesses, base);
    read(
        accesses,
        Location::Memory(MemoryLocation {
            addressing: Addressing::Displacement {
                base,
                displacement: u16::from(disp) * u16::from(byte_len),
            },
            byte_len,
        }),
    );
    write_register(accesses, destination);
}

fn add_gbr_transfer(
    disp: u8,
    byte_len: u8,
    kind: AccessKind,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read(accesses, Location::Gbr);
    if kind == AccessKind::Write {
        read_register(accesses, Register::R0);
    }
    access(
        accesses,
        Location::Memory(MemoryLocation {
            addressing: Addressing::GbrDisplacement(u16::from(disp) * u16::from(byte_len)),
            byte_len,
        }),
        kind,
    );
    if kind == AccessKind::Read {
        write_register(accesses, Register::R0);
    }
}

fn add_mac(rm: Register, rn: Register, byte_len: u8, accesses: &mut Vec<LocationAccess<Location>>) {
    for register in [rm, rn] {
        read_write_register(accesses, register);
        read(
            accesses,
            Location::Memory(MemoryLocation {
                addressing: Addressing::PostIncrement(register),
                byte_len,
            }),
        );
    }
    read(accesses, Location::Mach);
    read(accesses, Location::Macl);
    read_sr_field(accesses, SrField::S);
    write(accesses, Location::Mach);
    write(accesses, Location::Macl);
}

fn transfer_register_to_special(
    register: Register,
    special: Location,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_register(accesses, register);
    write(accesses, special);
}
fn transfer_special_to_register(
    special: Location,
    register: Register,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read(accesses, special);
    write_register(accesses, register);
}
fn load_special_postincrement(
    register: Register,
    special: Location,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_write_register(accesses, register);
    read(
        accesses,
        Location::Memory(MemoryLocation {
            addressing: Addressing::PostIncrement(register),
            byte_len: 4,
        }),
    );
    write(accesses, special);
}
fn store_special_predecrement(
    special: Location,
    register: Register,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read(accesses, special);
    read_write_register(accesses, register);
    write(
        accesses,
        Location::Memory(MemoryLocation {
            addressing: Addressing::PreDecrement(register),
            byte_len: 4,
        }),
    );
}

fn read_write_register(accesses: &mut Vec<LocationAccess<Location>>, register: Register) {
    read_register(accesses, register);
    write_register(accesses, register);
}
fn read_register(accesses: &mut Vec<LocationAccess<Location>>, register: Register) {
    read(accesses, Location::Register(register));
}
fn write_register(accesses: &mut Vec<LocationAccess<Location>>, register: Register) {
    write(accesses, Location::Register(register));
}
fn read_sr_field(accesses: &mut Vec<LocationAccess<Location>>, field: SrField) {
    read(accesses, Location::SrField(field));
}
fn write_sr_field(accesses: &mut Vec<LocationAccess<Location>>, field: SrField) {
    write(accesses, Location::SrField(field));
}
fn write_sr_fields(accesses: &mut Vec<LocationAccess<Location>>, fields: &[SrField]) {
    for field in fields {
        write_sr_field(accesses, *field);
    }
}
fn read(accesses: &mut Vec<LocationAccess<Location>>, location: Location) {
    access(accesses, location, AccessKind::Read);
}
fn write(accesses: &mut Vec<LocationAccess<Location>>, location: Location) {
    access(accesses, location, AccessKind::Write);
}
fn access(accesses: &mut Vec<LocationAccess<Location>>, location: Location, kind: AccessKind) {
    let candidate = LocationAccess {
        location,
        kind,
        occurrence: AccessOccurrence::Always,
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
        (Location::Sr, Location::SrField(_)) | (Location::SrField(_), Location::Sr) => Overlapping,
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
        (Location::Memory(_), _) | (_, Location::Memory(_)) => Disjoint,
        _ => Disjoint,
    }
}

fn ranges_overlap(left: u32, left_len: u8, right: u32, right_len: u8) -> bool {
    (0..u32::from(left_len)).any(|offset| {
        let address = left.wrapping_add(offset);
        (0..u32::from(right_len)).any(|other| address == right.wrapping_add(other))
    })
}
