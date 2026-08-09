//! Complete static meaning for the PlayStation R3000A and GTE profile.

use core::fmt;

use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlBoundary, ControlTarget, DelaySlot,
    DelaySlotExecution, LocationAccess, ReferenceKind, StaticReference, StorageRelation,
};

use crate::{CodeLocation, GteCommand, GteOpcode, Instruction, Register};

/// Runtime-derived PlayStation R3000A control destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndirectTarget {
    /// Destination read from the named general-purpose register.
    Register(Register),
    /// General exception vector selected by the COP0 Status.BEV bit.
    GeneralExceptionVector,
}

/// Synchronous boundary raised explicitly by an R3000A instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Boundary {
    /// System-call exception carrying the encoded diagnostic code.
    Syscall(u32),
    /// Breakpoint exception carrying the encoded diagnostic code.
    Breakpoint(u32),
}

/// One encoded R3000A system-control coprocessor register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cop0Register(u8);

impl Cop0Register {
    /// Bad virtual address register.
    pub const BAD_VADDR: Self = Self(8);
    /// Processor status register.
    pub const STATUS: Self = Self(12);
    /// Exception cause register.
    pub const CAUSE: Self = Self(13);
    /// Exception program counter register.
    pub const EPC: Self = Self(14);
    /// Processor revision identifier register.
    pub const PRID: Self = Self(15);

    /// Constructs a register from a five-bit COP0 register field.
    pub const fn new(index: u8) -> Option<Self> {
        if index < 32 { Some(Self(index)) } else { None }
    }

    /// Returns the encoded COP0 register index.
    pub const fn index(self) -> u8 {
        self.0
    }
}

/// Individually meaningful view of the COP0 Status register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cop0StatusField {
    /// Current, previous, and old kernel/user and interrupt-enable stack.
    ModeStack,
    /// Boot-exception-vector selection bit.
    BootExceptionVector,
}

/// One encoded GTE data register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GteDataRegister(u8);

impl GteDataRegister {
    /// Constructs a register from a five-bit GTE data-register field.
    pub const fn new(index: u8) -> Option<Self> {
        if index < 32 { Some(Self(index)) } else { None }
    }

    /// Returns the encoded data-register index.
    pub const fn index(self) -> u8 {
        self.0
    }
}

/// One encoded GTE control register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GteControlRegister(u8);

impl GteControlRegister {
    /// GTE error and saturation flag register.
    pub const FLAG: Self = Self(31);

    /// Constructs a register from a five-bit GTE control-register field.
    pub const fn new(index: u8) -> Option<Self> {
        if index < 32 { Some(Self(index)) } else { None }
    }

    /// Returns the encoded control-register index.
    pub const fn index(self) -> u8 {
        self.0
    }
}

/// Symbolic base-plus-displacement R3000A memory location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryLocation {
    /// General-purpose register supplying the base address.
    pub base: Register,
    /// Signed byte displacement from `base`.
    pub displacement: i16,
    /// Number of bytes participating in the architectural access.
    pub byte_len: u8,
    /// Whether the access uses a left/right unaligned merge operation.
    pub merge: bool,
}

/// Architectural storage location in the PlayStation R3000A profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Location {
    /// General-purpose register storage.
    Gpr(Register),
    /// Multiply/divide high-result register.
    Hi,
    /// Multiply/divide low-result register.
    Lo,
    /// Whole COP0 register selected by an encoded transfer.
    Cop0(Cop0Register),
    /// Individually meaningful field within COP0 Status.
    Cop0Status(Cop0StatusField),
    /// GTE data-register storage or architectural transfer port.
    GteData(GteDataRegister),
    /// GTE control-register storage.
    GteControl(GteControlRegister),
    /// Symbolic memory storage.
    Memory(MemoryLocation),
}

/// Statically resolved non-control R3000A memory reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Reference {
    /// First byte of the referenced range.
    pub address: u32,
    /// Number of referenced bytes.
    pub byte_len: u8,
}

/// Structural failure to form sound PlayStation R3000A static facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticError {
    /// The instruction location is not word aligned.
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
                "PlayStation R3000A instruction location {location:#010x} is not word aligned"
            ),
        }
    }
}

impl std::error::Error for SemanticError {}

/// Shared control-flow result specialized with R3000A vocabulary.
pub type ControlFlow = typed_isa_core::ControlFlow<CodeLocation, IndirectTarget, Boundary>;

/// Shared instruction-semantics result specialized with R3000A vocabulary.
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
    if at & 3 != 0 {
        return Err(SemanticError::UnalignedLocation { location: at });
    }
    let mut location_accesses = Vec::new();
    let mut static_references = Vec::new();
    add_instruction_accesses(instruction, &mut location_accesses, &mut static_references);
    Ok(InstructionSemantics {
        control_flow: control_flow(instruction, at),
        location_accesses,
        static_references,
    })
}

fn control_flow(instruction: &Instruction, at: CodeLocation) -> ControlFlow {
    let slot = at.wrapping_add(4);
    let after_slot = at.wrapping_add(8);
    let delay_slot = || {
        Some(DelaySlot {
            point: slot,
            execution: DelaySlotExecution::Always,
        })
    };
    match *instruction {
        Instruction::Beq { rs, rt, target } if rs == Register::ZERO && rt == Register::ZERO => {
            ControlFlow {
                action: ControlAction::Transfer {
                    target: ControlTarget::Direct(target),
                },
                fallthrough: None,
                delay_slot: delay_slot(),
            }
        }
        Instruction::Beq { target, .. }
        | Instruction::Bne { target, .. }
        | Instruction::Blez { target, .. }
        | Instruction::Bgtz { target, .. }
        | Instruction::Bltz { target, .. }
        | Instruction::Bgez { target, .. } => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Direct(target),
            },
            fallthrough: Some(after_slot),
            delay_slot: delay_slot(),
        },
        Instruction::Bltzal { target, .. } | Instruction::Bgezal { target, .. } => ControlFlow {
            action: ControlAction::LinkedTransfer {
                target: ControlTarget::Direct(target),
                return_site: after_slot,
            },
            fallthrough: Some(after_slot),
            delay_slot: delay_slot(),
        },
        Instruction::J { target } => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Direct(target),
            },
            fallthrough: None,
            delay_slot: delay_slot(),
        },
        Instruction::Jal { target } => ControlFlow {
            action: ControlAction::LinkedTransfer {
                target: ControlTarget::Direct(target),
                return_site: after_slot,
            },
            fallthrough: None,
            delay_slot: delay_slot(),
        },
        Instruction::Jr { rs } => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Indirect(IndirectTarget::Register(rs)),
            },
            fallthrough: None,
            delay_slot: delay_slot(),
        },
        Instruction::Jalr { rs, .. } => ControlFlow {
            action: ControlAction::LinkedTransfer {
                target: ControlTarget::Indirect(IndirectTarget::Register(rs)),
                return_site: after_slot,
            },
            fallthrough: None,
            delay_slot: delay_slot(),
        },
        Instruction::Syscall { code } | Instruction::Break { code } => ControlFlow {
            action: ControlAction::Boundary(ControlBoundary::Trap {
                kind: if matches!(instruction, Instruction::Syscall { .. }) {
                    Boundary::Syscall(code)
                } else {
                    Boundary::Breakpoint(code)
                },
                target: Some(ControlTarget::Indirect(
                    IndirectTarget::GeneralExceptionVector,
                )),
                resume: None,
            }),
            fallthrough: None,
            delay_slot: None,
        },
        _ => ControlFlow {
            action: ControlAction::Continue,
            fallthrough: Some(slot),
            delay_slot: None,
        },
    }
}

fn add_instruction_accesses(
    instruction: &Instruction,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    use Instruction::*;
    match *instruction {
        Add { rd, rs, rt }
        | Addu { rd, rs, rt }
        | Sub { rd, rs, rt }
        | Subu { rd, rs, rt }
        | And { rd, rs, rt }
        | Or { rd, rs, rt }
        | Xor { rd, rs, rt }
        | Nor { rd, rs, rt }
        | Slt { rd, rs, rt }
        | Sltu { rd, rs, rt }
        | Sllv { rd, rs, rt }
        | Srlv { rd, rs, rt }
        | Srav { rd, rs, rt } => {
            read_gpr(accesses, rs);
            read_gpr(accesses, rt);
            write_gpr(accesses, rd, AccessOccurrence::Always);
        }
        Addi { rt, rs, .. }
        | Addiu { rt, rs, .. }
        | Andi { rt, rs, .. }
        | Ori { rt, rs, .. }
        | Xori { rt, rs, .. }
        | Slti { rt, rs, .. }
        | Sltiu { rt, rs, .. } => {
            read_gpr(accesses, rs);
            write_gpr(accesses, rt, AccessOccurrence::Always);
        }
        Lui { rt, .. } => write_gpr(accesses, rt, AccessOccurrence::Always),
        Sll { rd, rt, .. } | Srl { rd, rt, .. } | Sra { rd, rt, .. } => {
            read_gpr(accesses, rt);
            write_gpr(accesses, rd, AccessOccurrence::Always);
        }
        Mult { rs, rt } | Multu { rs, rt } | Div { rs, rt } | Divu { rs, rt } => {
            read_gpr(accesses, rs);
            read_gpr(accesses, rt);
            write(accesses, Location::Hi, AccessOccurrence::Always);
            write(accesses, Location::Lo, AccessOccurrence::Always);
        }
        Mfhi { rd } => {
            read(accesses, Location::Hi, AccessOccurrence::Always);
            write_gpr(accesses, rd, AccessOccurrence::Always);
        }
        Mthi { rs } => {
            read_gpr(accesses, rs);
            write(accesses, Location::Hi, AccessOccurrence::Always);
        }
        Mflo { rd } => {
            read(accesses, Location::Lo, AccessOccurrence::Always);
            write_gpr(accesses, rd, AccessOccurrence::Always);
        }
        Mtlo { rs } => {
            read_gpr(accesses, rs);
            write(accesses, Location::Lo, AccessOccurrence::Always);
        }
        Lb { rt, base, offset } | Lbu { rt, base, offset } => {
            add_load(accesses, references, rt, base, offset, 1, false)
        }
        Lh { rt, base, offset } | Lhu { rt, base, offset } => {
            add_load(accesses, references, rt, base, offset, 2, false)
        }
        Lw { rt, base, offset } => add_load(accesses, references, rt, base, offset, 4, false),
        Lwl { rt, base, offset } | Lwr { rt, base, offset } => {
            add_load(accesses, references, rt, base, offset, 4, true)
        }
        Sb { rt, base, offset } => add_store(accesses, references, rt, base, offset, 1, false),
        Sh { rt, base, offset } => add_store(accesses, references, rt, base, offset, 2, false),
        Sw { rt, base, offset } => add_store(accesses, references, rt, base, offset, 4, false),
        Swl { rt, base, offset } | Swr { rt, base, offset } => {
            add_store(accesses, references, rt, base, offset, 4, true)
        }
        Beq { rs, rt, .. } | Bne { rs, rt, .. } => {
            read_gpr(accesses, rs);
            read_gpr(accesses, rt);
        }
        Blez { rs, .. }
        | Bgtz { rs, .. }
        | Bltz { rs, .. }
        | Bgez { rs, .. }
        | Bltzal { rs, .. }
        | Bgezal { rs, .. } => {
            read_gpr(accesses, rs);
            if matches!(instruction, Bltzal { .. } | Bgezal { .. }) {
                write_gpr(accesses, Register::RA, AccessOccurrence::Conditional);
            }
        }
        J { .. } => {}
        Jal { .. } => write_gpr(accesses, Register::RA, AccessOccurrence::Always),
        Jr { rs } => read_gpr(accesses, rs),
        Jalr { rd, rs } => {
            read_gpr(accesses, rs);
            write_gpr(accesses, rd, AccessOccurrence::Always);
        }
        Syscall { .. } | Break { .. } => add_exception_accesses(accesses),
        Mfc0 { rt, rd } => {
            read(accesses, Location::Cop0(cop0(rd)), AccessOccurrence::Always);
            write_gpr(accesses, rt, AccessOccurrence::Always);
        }
        Mtc0 { rt, rd } => {
            read_gpr(accesses, rt);
            write(accesses, Location::Cop0(cop0(rd)), AccessOccurrence::Always);
        }
        Rfe => {
            read(
                accesses,
                Location::Cop0Status(Cop0StatusField::ModeStack),
                AccessOccurrence::Always,
            );
            write(
                accesses,
                Location::Cop0Status(Cop0StatusField::ModeStack),
                AccessOccurrence::Always,
            );
        }
        Mfc2 { rt, rd } => {
            add_gte_data_read(accesses, rd);
            write_gpr(accesses, rt, AccessOccurrence::Always);
        }
        Mtc2 { rt, rd } => {
            read_gpr(accesses, rt);
            add_gte_data_write(accesses, rd);
        }
        Cfc2 { rt, rd } => {
            read_gte_control(accesses, rd);
            write_gpr(accesses, rt, AccessOccurrence::Always);
        }
        Ctc2 { rt, rd } => {
            read_gpr(accesses, rt);
            write_gte_control(accesses, rd);
        }
        Lwc2 { rt, base, offset } => {
            add_memory(
                accesses,
                references,
                base,
                offset,
                4,
                false,
                AccessKind::Read,
            );
            add_gte_data_write(accesses, rt);
        }
        Swc2 { rt, base, offset } => {
            add_gte_data_read(accesses, rt);
            add_memory(
                accesses,
                references,
                base,
                offset,
                4,
                false,
                AccessKind::Write,
            );
        }
        Gte(command) => add_gte_command_accesses(command, accesses),
    }
}

fn add_exception_accesses(accesses: &mut Vec<LocationAccess<Location>>) {
    read(
        accesses,
        Location::Cop0Status(Cop0StatusField::BootExceptionVector),
        AccessOccurrence::Always,
    );
    read(
        accesses,
        Location::Cop0Status(Cop0StatusField::ModeStack),
        AccessOccurrence::Always,
    );
    write(
        accesses,
        Location::Cop0Status(Cop0StatusField::ModeStack),
        AccessOccurrence::Always,
    );
    write(
        accesses,
        Location::Cop0(Cop0Register::CAUSE),
        AccessOccurrence::Always,
    );
    write(
        accesses,
        Location::Cop0(Cop0Register::EPC),
        AccessOccurrence::Always,
    );
}

fn add_load(
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
    destination: Register,
    base: Register,
    displacement: i16,
    byte_len: u8,
    merge: bool,
) {
    add_memory(
        accesses,
        references,
        base,
        displacement,
        byte_len,
        merge,
        AccessKind::Read,
    );
    if merge {
        read_gpr(accesses, destination);
    }
    write_gpr(accesses, destination, AccessOccurrence::Always);
}

fn add_store(
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
    source: Register,
    base: Register,
    displacement: i16,
    byte_len: u8,
    merge: bool,
) {
    read_gpr(accesses, source);
    if merge {
        add_memory(
            accesses,
            references,
            base,
            displacement,
            byte_len,
            merge,
            AccessKind::Read,
        );
    }
    add_memory(
        accesses,
        references,
        base,
        displacement,
        byte_len,
        merge,
        AccessKind::Write,
    );
}

fn add_memory(
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
    base: Register,
    displacement: i16,
    byte_len: u8,
    merge: bool,
    kind: AccessKind,
) {
    read_gpr(accesses, base);
    let location = Location::Memory(MemoryLocation {
        base,
        displacement,
        byte_len,
        merge,
    });
    access(accesses, location, kind, AccessOccurrence::Always);
    if base == Register::ZERO {
        let mut address = displacement as i32 as u32;
        if merge {
            address &= !3;
        }
        references.push(StaticReference {
            target: Reference { address, byte_len },
            kind: match kind {
                AccessKind::Read => ReferenceKind::Read,
                AccessKind::Write => ReferenceKind::Write,
            },
            occurrence: AccessOccurrence::Always,
        });
    }
}

fn add_gte_data_read(accesses: &mut Vec<LocationAccess<Location>>, index: u8) {
    read_gte_data(accesses, index);
    match index {
        29 => read_gte_data_many(accesses, &[9, 10, 11]),
        31 => read_gte_data(accesses, 30),
        _ => {}
    }
}

fn add_gte_data_write(accesses: &mut Vec<LocationAccess<Location>>, index: u8) {
    write_gte_data(accesses, index);
    match index {
        15 => {
            read_gte_data_many(accesses, &[13, 14]);
            write_gte_data_many(accesses, &[12, 13, 14]);
        }
        28 => write_gte_data_many(accesses, &[9, 10, 11]),
        _ => {}
    }
}

fn add_gte_command_accesses(command: GteCommand, accesses: &mut Vec<LocationAccess<Location>>) {
    use GteOpcode::*;
    match command.opcode {
        Rtps => {
            read_gte_data_many(accesses, &[0, 1, 13, 14, 17, 18, 19]);
            read_gte_control_many(accesses, &[0, 1, 2, 3, 4, 5, 6, 7, 24, 25, 26, 27, 28]);
            write_gte_data_many(
                accesses,
                &[8, 9, 10, 11, 12, 13, 14, 16, 17, 18, 19, 24, 25, 26, 27],
            );
        }
        Rtpt => {
            read_gte_data_many(accesses, &[0, 1, 2, 3, 4, 5, 19]);
            read_gte_control_many(accesses, &[0, 1, 2, 3, 4, 5, 6, 7, 24, 25, 26, 27, 28]);
            write_gte_data_many(
                accesses,
                &[8, 9, 10, 11, 12, 13, 14, 16, 17, 18, 19, 24, 25, 26, 27],
            );
        }
        Ncds | Ncdt | Nccs | Ncct | Ncs | Nct => {
            let triple = matches!(command.opcode, Ncdt | Ncct | Nct);
            read_gte_data_many(accesses, if triple { &[0, 1, 2, 3, 4, 5] } else { &[0, 1] });
            if matches!(command.opcode, Ncds | Ncdt | Nccs | Ncct) {
                read_gte_data(accesses, 6);
            }
            if matches!(command.opcode, Ncds | Ncdt) {
                read_gte_data(accesses, 8);
                read_gte_control_many(accesses, &[21, 22, 23]);
            }
            read_gte_control_many(
                accesses,
                &[8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
            );
            add_color_fifo_accesses(accesses, triple);
            write_gte_data_many(accesses, &[9, 10, 11, 25, 26, 27]);
        }
        Cdp | Cc => {
            read_gte_data_many(accesses, &[6, 9, 10, 11]);
            read_gte_control_many(accesses, &[13, 14, 15, 16, 17, 18, 19, 20]);
            if command.opcode == Cdp {
                read_gte_data(accesses, 8);
                read_gte_control_many(accesses, &[21, 22, 23]);
            }
            add_color_fifo_accesses(accesses, false);
            write_gte_data_many(accesses, &[9, 10, 11, 25, 26, 27]);
        }
        Mvmva => {
            read_gte_data_many(
                accesses,
                match command.vector & 3 {
                    0 => &[0, 1],
                    1 => &[2, 3],
                    2 => &[4, 5],
                    _ => &[9, 10, 11],
                },
            );
            read_gte_control_many(
                accesses,
                match command.matrix & 3 {
                    0 => &[0, 1, 2, 3, 4],
                    1 => &[8, 9, 10, 11, 12],
                    2 => &[16, 17, 18, 19, 20],
                    _ => &[],
                },
            );
            read_gte_control_many(
                accesses,
                match command.control_vector & 3 {
                    0 => &[5, 6, 7],
                    1 => &[13, 14, 15],
                    2 => &[21, 22, 23],
                    _ => &[],
                },
            );
            write_gte_data_many(accesses, &[9, 10, 11, 25, 26, 27]);
        }
        Intpl => {
            read_gte_data_many(accesses, &[6, 8, 9, 10, 11]);
            read_gte_control_many(accesses, &[21, 22, 23]);
            add_color_fifo_accesses(accesses, false);
            write_gte_data_many(accesses, &[9, 10, 11, 25, 26, 27]);
        }
        Dcpl => {
            read_gte_data_many(accesses, &[6, 8, 9, 10, 11]);
            read_gte_control_many(accesses, &[21, 22, 23]);
            add_color_fifo_accesses(accesses, false);
            write_gte_data_many(accesses, &[9, 10, 11, 25, 26, 27]);
        }
        Dpcs => {
            read_gte_data_many(accesses, &[6, 8]);
            read_gte_control_many(accesses, &[21, 22, 23]);
            add_color_fifo_accesses(accesses, false);
            write_gte_data_many(accesses, &[9, 10, 11, 25, 26, 27]);
        }
        Dpct => {
            read_gte_data_many(accesses, &[6, 8, 20, 21, 22]);
            read_gte_control_many(accesses, &[21, 22, 23]);
            write_gte_data_many(accesses, &[9, 10, 11, 20, 21, 22, 25, 26, 27]);
        }
        Sqr => {
            read_gte_data_many(accesses, &[9, 10, 11]);
            write_gte_data_many(accesses, &[9, 10, 11, 25, 26, 27]);
        }
        Avsz3 => {
            read_gte_data_many(accesses, &[17, 18, 19]);
            read_gte_control(accesses, 29);
            write_gte_data_many(accesses, &[7, 24]);
        }
        Avsz4 => {
            read_gte_data_many(accesses, &[16, 17, 18, 19]);
            read_gte_control(accesses, 30);
            write_gte_data_many(accesses, &[7, 24]);
        }
        Nclip => {
            read_gte_data_many(accesses, &[12, 13, 14]);
            write_gte_data(accesses, 24);
        }
        Op => {
            read_gte_data_many(accesses, &[9, 10, 11]);
            read_gte_control_many(accesses, &[0, 2, 4]);
            write_gte_data_many(accesses, &[9, 10, 11, 25, 26, 27]);
        }
        Gpf | Gpl => {
            read_gte_data_many(accesses, &[6, 8, 9, 10, 11]);
            if command.opcode == Gpl {
                read_gte_data_many(accesses, &[25, 26, 27]);
            }
            add_color_fifo_accesses(accesses, false);
            write_gte_data_many(accesses, &[9, 10, 11, 25, 26, 27]);
        }
    }
    write_gte_control(accesses, GteControlRegister::FLAG.index());
}

fn add_color_fifo_accesses(accesses: &mut Vec<LocationAccess<Location>>, replaces_all: bool) {
    if !replaces_all {
        read_gte_data_many(accesses, &[21, 22]);
    }
    write_gte_data_many(accesses, &[20, 21, 22]);
}

fn cop0(index: u8) -> Cop0Register {
    Cop0Register::new(index).expect("decoded COP0 register fields are five bits")
}

fn gte_data(index: u8) -> GteDataRegister {
    GteDataRegister::new(index).expect("decoded GTE register fields are five bits")
}

fn gte_control(index: u8) -> GteControlRegister {
    GteControlRegister::new(index).expect("decoded GTE control fields are five bits")
}

fn read_gpr(accesses: &mut Vec<LocationAccess<Location>>, register: Register) {
    read(accesses, Location::Gpr(register), AccessOccurrence::Always);
}

fn write_gpr(
    accesses: &mut Vec<LocationAccess<Location>>,
    register: Register,
    occurrence: AccessOccurrence,
) {
    if register != Register::ZERO {
        write(accesses, Location::Gpr(register), occurrence);
    }
}

fn read_gte_data(accesses: &mut Vec<LocationAccess<Location>>, index: u8) {
    read(
        accesses,
        Location::GteData(gte_data(index)),
        AccessOccurrence::Always,
    );
}

fn write_gte_data(accesses: &mut Vec<LocationAccess<Location>>, index: u8) {
    write(
        accesses,
        Location::GteData(gte_data(index)),
        AccessOccurrence::Always,
    );
}

fn read_gte_data_many(accesses: &mut Vec<LocationAccess<Location>>, indexes: &[u8]) {
    for index in indexes {
        read_gte_data(accesses, *index);
    }
}

fn write_gte_data_many(accesses: &mut Vec<LocationAccess<Location>>, indexes: &[u8]) {
    for index in indexes {
        write_gte_data(accesses, *index);
    }
}

fn read_gte_control(accesses: &mut Vec<LocationAccess<Location>>, index: u8) {
    read(
        accesses,
        Location::GteControl(gte_control(index)),
        AccessOccurrence::Always,
    );
}

fn write_gte_control(accesses: &mut Vec<LocationAccess<Location>>, index: u8) {
    write(
        accesses,
        Location::GteControl(gte_control(index)),
        AccessOccurrence::Always,
    );
}

fn read_gte_control_many(accesses: &mut Vec<LocationAccess<Location>>, indexes: &[u8]) {
    for index in indexes {
        read_gte_control(accesses, *index);
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
        (Location::Cop0(register), Location::Cop0Status(_))
        | (Location::Cop0Status(_), Location::Cop0(register))
            if *register == Cop0Register::STATUS =>
        {
            Overlapping
        }
        (Location::Memory(left), Location::Memory(right)) if left.base == right.base => {
            let left_start = i32::from(left.displacement);
            let left_end = left_start + i32::from(left.byte_len);
            let right_start = i32::from(right.displacement);
            let right_end = right_start + i32::from(right.byte_len);
            if left_start < right_end && right_start < left_end {
                Overlapping
            } else {
                Disjoint
            }
        }
        (Location::Memory(_), Location::Memory(_)) => Unknown,
        (Location::Memory(_), _) | (_, Location::Memory(_)) => Disjoint,
        _ => Disjoint,
    }
}
