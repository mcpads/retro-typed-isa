/// Z80 byte-sized register or the `(HL)` operand in an encoded byte-operand field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ByteOperand {
    B,
    C,
    D,
    E,
    H,
    L,
    /// Indirect through Hl — `(Hl)`
    IndirectHl,
    A,
}

impl ByteOperand {
    /// Decode from the Z80 `r[y]` table (y = 0..7).
    pub(crate) fn from_field(idx: u8) -> Self {
        match idx {
            0 => Self::B,
            1 => Self::C,
            2 => Self::D,
            3 => Self::E,
            4 => Self::H,
            5 => Self::L,
            6 => Self::IndirectHl,
            7 => Self::A,
            _ => unreachable!(),
        }
    }

    /// Encode back to the Z80 `r[y]` index.
    pub const fn field(self) -> u8 {
        match self {
            Self::B => 0,
            Self::C => 1,
            Self::D => 2,
            Self::E => 3,
            Self::H => 4,
            Self::L => 5,
            Self::IndirectHl => 6,
            Self::A => 7,
        }
    }
}

/// Z80 16-bit register pair (rp table, indices 0-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Register16 {
    Bc,
    De,
    Hl,
    Sp,
}

impl Register16 {
    /// Decode from the Z80 `rp[p]` table (p = 0..3).
    pub(crate) fn from_field(idx: u8) -> Self {
        match idx {
            0 => Self::Bc,
            1 => Self::De,
            2 => Self::Hl,
            3 => Self::Sp,
            _ => unreachable!(),
        }
    }

    /// Encode back to the Z80 `rp[p]` index.
    pub const fn field(self) -> u8 {
        match self {
            Self::Bc => 0,
            Self::De => 1,
            Self::Hl => 2,
            Self::Sp => 3,
        }
    }
}

/// Z80 16-bit register pair for PUSH/POP (rp2 table, indices 0-3).
/// Af replaces Sp compared to [`Register16`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StackRegister {
    Bc,
    De,
    Hl,
    Af,
}

impl StackRegister {
    /// Decode from the Z80 `rp2[p]` table (p = 0..3).
    pub(crate) fn from_field(idx: u8) -> Self {
        match idx {
            0 => Self::Bc,
            1 => Self::De,
            2 => Self::Hl,
            3 => Self::Af,
            _ => unreachable!(),
        }
    }

    /// Encode back to the Z80 `rp2[p]` index.
    pub const fn field(self) -> u8 {
        match self {
            Self::Bc => 0,
            Self::De => 1,
            Self::Hl => 2,
            Self::Af => 3,
        }
    }
}

/// Ix or Iy index register (used by DD/FD-prefixed instructions).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IndexRegister {
    Ix,
    Iy,
}

impl IndexRegister {
    /// Returns the DD/FD prefix byte for this index register.
    pub const fn prefix(self) -> u8 {
        match self {
            IndexRegister::Ix => 0xDD,
            IndexRegister::Iy => 0xFD,
        }
    }
}

/// Z80 condition code (cc table, indices 0-7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Condition {
    Nz,
    Z,
    Nc,
    C,
    Po,
    Pe,
    P,
    M,
}

impl Condition {
    /// Decode from the Z80 `cc[y]` table (y = 0..7).
    pub(crate) fn from_field(idx: u8) -> Self {
        match idx {
            0 => Self::Nz,
            1 => Self::Z,
            2 => Self::Nc,
            3 => Self::C,
            4 => Self::Po,
            5 => Self::Pe,
            6 => Self::P,
            7 => Self::M,
            _ => unreachable!(),
        }
    }

    /// Encode back to the Z80 `cc[y]` index.
    pub const fn field(self) -> u8 {
        match self {
            Self::Nz => 0,
            Self::Z => 1,
            Self::Nc => 2,
            Self::C => 3,
            Self::Po => 4,
            Self::Pe => 5,
            Self::P => 6,
            Self::M => 7,
        }
    }
}

/// Z80 ALU operation (alu table, indices 0-7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AluOperation {
    Add,
    Adc,
    Sub,
    Sbc,
    And,
    Xor,
    Or,
    Cp,
}

impl AluOperation {
    pub const ALL: [Self; 8] = [
        Self::Add,
        Self::Adc,
        Self::Sub,
        Self::Sbc,
        Self::And,
        Self::Xor,
        Self::Or,
        Self::Cp,
    ];

    /// Decode from the Z80 `alu[y]` table (y = 0..7).
    pub(crate) fn from_field(idx: u8) -> Self {
        match idx {
            0 => Self::Add,
            1 => Self::Adc,
            2 => Self::Sub,
            3 => Self::Sbc,
            4 => Self::And,
            5 => Self::Xor,
            6 => Self::Or,
            7 => Self::Cp,
            _ => unreachable!(),
        }
    }

    /// Encode back to the Z80 `alu[y]` index.
    pub const fn field(self) -> u8 {
        match self {
            Self::Add => 0,
            Self::Adc => 1,
            Self::Sub => 2,
            Self::Sbc => 3,
            Self::And => 4,
            Self::Xor => 5,
            Self::Or => 6,
            Self::Cp => 7,
        }
    }
}

/// Documented Z80 rotation/shift operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RotateShiftOperation {
    Rlc,
    Rrc,
    Rl,
    Rr,
    Sla,
    Sra,
    Srl,
}

impl RotateShiftOperation {
    pub const ALL: [Self; 7] = [
        Self::Rlc,
        Self::Rrc,
        Self::Rl,
        Self::Rr,
        Self::Sla,
        Self::Sra,
        Self::Srl,
    ];

    /// Decode from the Z80 `rot[y]` table (y = 0..7).
    pub(crate) fn from_field(idx: u8) -> Option<Self> {
        match idx {
            0 => Some(Self::Rlc),
            1 => Some(Self::Rrc),
            2 => Some(Self::Rl),
            3 => Some(Self::Rr),
            4 => Some(Self::Sla),
            5 => Some(Self::Sra),
            6 => None,
            7 => Some(Self::Srl),
            _ => unreachable!(),
        }
    }

    /// Encode back to the Z80 `rot[y]` index.
    pub const fn field(self) -> u8 {
        match self {
            Self::Rlc => 0,
            Self::Rrc => 1,
            Self::Rl => 2,
            Self::Rr => 3,
            Self::Sla => 4,
            Self::Sra => 5,
            Self::Srl => 7,
        }
    }
}

/// Z80 interrupt mode (IM instruction).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InterruptMode {
    Im0,
    Im1,
    Im2,
}

impl InterruptMode {
    pub const ALL: [Self; 3] = [Self::Im0, Self::Im1, Self::Im2];
}

/// Complete instruction set documented by the Zilog Z80 CPU User Manual.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    // -----------------------------------------------------------------------
    // Misc / control
    // -----------------------------------------------------------------------
    Nop,
    Halt,
    Di,
    Ei,

    // -----------------------------------------------------------------------
    // 8-bit loads
    // -----------------------------------------------------------------------
    /// `LD r, r'`
    LdRR(ByteOperand, ByteOperand),
    /// `LD r, n`
    LdRImm(ByteOperand, u8),
    /// `LD (nn), A`
    LdAddrA(u16),
    /// `LD A, (nn)`
    LdAAddr(u16),
    /// `LD (Bc), A`
    LdBCA,
    /// `LD (De), A`
    LdDEA,
    /// `LD A, (Bc)`
    LdABC,
    /// `LD A, (De)`
    LdADE,
    /// `LD (nn), Hl`
    LdAddrHL(u16),
    /// `LD Hl, (nn)`
    LdHLAddr(u16),

    // -----------------------------------------------------------------------
    // 16-bit loads
    // -----------------------------------------------------------------------
    /// `LD rr, nn`
    LdRRImm(Register16, u16),
    /// `LD Sp, Hl`
    LdSPHL,
    /// `PUSH rr`
    Push(StackRegister),
    /// `POP rr`
    Pop(StackRegister),

    // -----------------------------------------------------------------------
    // 8-bit arithmetic / logic
    // -----------------------------------------------------------------------
    /// `ADD/ADC/SUB/SBC/AND/XOR/OR/CP A, r`
    AluR(AluOperation, ByteOperand),
    /// `ADD/ADC/SUB/SBC/AND/XOR/OR/CP A, n`
    AluImm(AluOperation, u8),
    /// `INC r`
    IncR(ByteOperand),
    /// `DEC r`
    DecR(ByteOperand),

    // -----------------------------------------------------------------------
    // 16-bit arithmetic
    // -----------------------------------------------------------------------
    /// `INC rr`
    IncRR(Register16),
    /// `DEC rr`
    DecRR(Register16),
    /// `ADD Hl, rr`
    AddHLRR(Register16),

    // -----------------------------------------------------------------------
    // Jumps
    // -----------------------------------------------------------------------
    /// `JP nn`
    Jp(u16),
    /// `JP cc, nn`
    JpCond(Condition, u16),
    /// `JP (Hl)`
    JpHL,
    /// `JR e`
    Jr(i8),
    /// `JR cc, e`
    JrCond(Condition, i8),
    /// `DJNZ e`
    Djnz(i8),

    // -----------------------------------------------------------------------
    // Calls / returns
    // -----------------------------------------------------------------------
    /// `CALL nn`
    Call(u16),
    /// `CALL cc, nn`
    CallCond(Condition, u16),
    /// `RET`
    Ret,
    /// `RET cc`
    RetCond(Condition),
    /// `RETI`
    Reti,
    /// `RETN`
    Retn,
    /// `RST p`  (p is 0, 8, 16, 24, 32, 40, 48, or 56)
    Rst(u8),

    // -----------------------------------------------------------------------
    // Accumulator rotates (unprefixed)
    // -----------------------------------------------------------------------
    Rlca,
    Rrca,
    Rla,
    Rra,

    // -----------------------------------------------------------------------
    // Misc accumulator / flag ops
    // -----------------------------------------------------------------------
    ExAFAF,
    Exx,
    ExDEHL,
    ExSPHL,
    Daa,
    Cpl,
    Neg,
    Scf,
    Ccf,

    // -----------------------------------------------------------------------
    // I/O
    // -----------------------------------------------------------------------
    /// `IN A, (n)`
    InAPort(u8),
    /// `OUT (n), A`
    OutPortA(u8),
    /// `IN r, (C)`
    InRC(ByteOperand),
    /// `OUT (C), r`
    OutCR(ByteOperand),

    // -----------------------------------------------------------------------
    // Block I/O (ED-prefixed)
    // -----------------------------------------------------------------------
    Ini,
    Ind,
    Inir,
    Indr,
    Outi,
    Outd,
    Otir,
    Otdr,

    // -----------------------------------------------------------------------
    // Block transfer / search (ED-prefixed)
    // -----------------------------------------------------------------------
    Ldi,
    Ldd,
    Ldir,
    Lddr,
    Cpi,
    Cpd,
    Cpir,
    Cpdr,

    // -----------------------------------------------------------------------
    // CB-prefix: bit operations on registers / (Hl)
    // -----------------------------------------------------------------------
    /// `RLC/RRC/RL/RR/SLA/SRA/SLL/SRL r`
    Rot(RotateShiftOperation, ByteOperand),
    /// `BIT b, r`
    Bit(u8, ByteOperand),
    /// `RES b, r`
    Res(u8, ByteOperand),
    /// `SET b, r`
    Set(u8, ByteOperand),

    // -----------------------------------------------------------------------
    // ED-prefix: 16-bit arithmetic with Hl
    // -----------------------------------------------------------------------
    /// `SBC Hl, rr`
    SbcHLRR(Register16),
    /// `ADC Hl, rr`
    AdcHLRR(Register16),
    /// `LD (nn), rr`  (ED-prefixed 16-bit store)
    LdAddrRR(u16, Register16),
    /// `LD rr, (nn)`  (ED-prefixed 16-bit load)
    LdRRAddr(Register16, u16),

    // -----------------------------------------------------------------------
    // ED-prefix: I/R register transfers
    // -----------------------------------------------------------------------
    /// `LD I, A`
    LdIA,
    /// `LD R, A`
    LdRA,
    /// `LD A, I`
    LdAI,
    /// `LD A, R`
    LdAR,

    // -----------------------------------------------------------------------
    // ED-prefix: rotate digit
    // -----------------------------------------------------------------------
    Rrd,
    Rld,

    // -----------------------------------------------------------------------
    // ED-prefix: interrupt mode
    // -----------------------------------------------------------------------
    Im(InterruptMode),

    // -----------------------------------------------------------------------
    // DD/FD prefix: Ix/Iy whole-register operations
    // -----------------------------------------------------------------------
    /// `LD Ix/Iy, nn`
    LdIxImm(IndexRegister, u16),
    /// `LD Ix/Iy, (nn)`
    LdIxAddr(IndexRegister, u16),
    /// `LD (nn), Ix/Iy`
    LdAddrIx(u16, IndexRegister),
    /// `PUSH Ix/Iy`
    PushIx(IndexRegister),
    /// `POP Ix/Iy`
    PopIx(IndexRegister),
    /// `INC Ix/Iy`
    IncIx(IndexRegister),
    /// `DEC Ix/Iy`
    DecIx(IndexRegister),
    /// `ADD Ix/Iy, rr`  (rr is Bc, De, Ix/Iy, or Sp)
    AddIxRR(IndexRegister, Register16),
    /// `LD Sp, Ix/Iy`
    LdSpIx(IndexRegister),
    /// `JP (Ix/Iy)`
    JpIx(IndexRegister),
    /// `EX (Sp), Ix/Iy`
    ExSpIx(IndexRegister),

    // -----------------------------------------------------------------------
    // DD/FD prefix: Ix/Iy + displacement operands
    // -----------------------------------------------------------------------
    /// `LD r, (Ix/Iy+d)`
    LdRIxDisp(ByteOperand, IndexRegister, i8),
    /// `LD (Ix/Iy+d), r`
    LdIxDispR(IndexRegister, i8, ByteOperand),
    /// `LD (Ix/Iy+d), n`
    LdIxDispImm(IndexRegister, i8, u8),
    /// `INC (Ix/Iy+d)`
    IncIxDisp(IndexRegister, i8),
    /// `DEC (Ix/Iy+d)`
    DecIxDisp(IndexRegister, i8),
    /// `ADD/ADC/…/CP A, (Ix/Iy+d)`
    AluIxDisp(AluOperation, IndexRegister, i8),

    // -----------------------------------------------------------------------
    // DDCB/FDCB prefix: indexed bit operations
    // -----------------------------------------------------------------------
    /// `RLC/RRC/… (Ix/Iy+d)`
    RotIxDisp(RotateShiftOperation, IndexRegister, i8),
    /// `BIT b, (Ix/Iy+d)`
    BitIxDisp(u8, IndexRegister, i8),
    /// `RES b, (Ix/Iy+d)`
    ResIxDisp(u8, IndexRegister, i8),
    /// `SET b, (Ix/Iy+d)`
    SetIxDisp(u8, IndexRegister, i8),
}

/// Operand-free semantic forms in the documented Z80 profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InstructionForm {
    Nop,
    Halt,
    Di,
    Ei,
    LdRR,
    LdRImm,
    LdAddrA,
    LdAAddr,
    LdBCA,
    LdDEA,
    LdABC,
    LdADE,
    LdAddrHL,
    LdHLAddr,
    LdRRImm,
    LdSPHL,
    Push,
    Pop,
    AluR(AluOperation),
    AluImm(AluOperation),
    IncR,
    DecR,
    IncRR,
    DecRR,
    AddHLRR,
    Jp,
    JpCond,
    JpHL,
    Jr,
    JrCond,
    Djnz,
    Call,
    CallCond,
    Ret,
    RetCond,
    Reti,
    Retn,
    Rst,
    Rlca,
    Rrca,
    Rla,
    Rra,
    ExAFAF,
    Exx,
    ExDEHL,
    ExSPHL,
    Daa,
    Cpl,
    Neg,
    Scf,
    Ccf,
    InAPort,
    OutPortA,
    InRC,
    OutCR,
    Ini,
    Ind,
    Inir,
    Indr,
    Outi,
    Outd,
    Otir,
    Otdr,
    Ldi,
    Ldd,
    Ldir,
    Lddr,
    Cpi,
    Cpd,
    Cpir,
    Cpdr,
    Rot(RotateShiftOperation),
    Bit,
    Res,
    Set,
    SbcHLRR,
    AdcHLRR,
    LdAddrRR,
    LdRRAddr,
    LdIA,
    LdRA,
    LdAI,
    LdAR,
    Rrd,
    Rld,
    Im(InterruptMode),
    LdIxImm,
    LdIxAddr,
    LdAddrIx,
    PushIx,
    PopIx,
    IncIx,
    DecIx,
    AddIxRR,
    LdSpIx,
    JpIx,
    ExSpIx,
    LdRIxDisp,
    LdIxDispR,
    LdIxDispImm,
    IncIxDisp,
    DecIxDisp,
    AluIxDisp(AluOperation),
    RotIxDisp(RotateShiftOperation),
    BitIxDisp,
    ResIxDisp,
    SetIxDisp,
}

impl InstructionForm {
    const BASE: &'static [Self] = &[
        Self::Nop,
        Self::Halt,
        Self::Di,
        Self::Ei,
        Self::LdRR,
        Self::LdRImm,
        Self::LdAddrA,
        Self::LdAAddr,
        Self::LdBCA,
        Self::LdDEA,
        Self::LdABC,
        Self::LdADE,
        Self::LdAddrHL,
        Self::LdHLAddr,
        Self::LdRRImm,
        Self::LdSPHL,
        Self::Push,
        Self::Pop,
        Self::IncR,
        Self::DecR,
        Self::IncRR,
        Self::DecRR,
        Self::AddHLRR,
        Self::Jp,
        Self::JpCond,
        Self::JpHL,
        Self::Jr,
        Self::JrCond,
        Self::Djnz,
        Self::Call,
        Self::CallCond,
        Self::Ret,
        Self::RetCond,
        Self::Reti,
        Self::Retn,
        Self::Rst,
        Self::Rlca,
        Self::Rrca,
        Self::Rla,
        Self::Rra,
        Self::ExAFAF,
        Self::Exx,
        Self::ExDEHL,
        Self::ExSPHL,
        Self::Daa,
        Self::Cpl,
        Self::Neg,
        Self::Scf,
        Self::Ccf,
        Self::InAPort,
        Self::OutPortA,
        Self::InRC,
        Self::OutCR,
        Self::Ini,
        Self::Ind,
        Self::Inir,
        Self::Indr,
        Self::Outi,
        Self::Outd,
        Self::Otir,
        Self::Otdr,
        Self::Ldi,
        Self::Ldd,
        Self::Ldir,
        Self::Lddr,
        Self::Cpi,
        Self::Cpd,
        Self::Cpir,
        Self::Cpdr,
        Self::Bit,
        Self::Res,
        Self::Set,
        Self::SbcHLRR,
        Self::AdcHLRR,
        Self::LdAddrRR,
        Self::LdRRAddr,
        Self::LdIA,
        Self::LdRA,
        Self::LdAI,
        Self::LdAR,
        Self::Rrd,
        Self::Rld,
        Self::LdIxImm,
        Self::LdIxAddr,
        Self::LdAddrIx,
        Self::PushIx,
        Self::PopIx,
        Self::IncIx,
        Self::DecIx,
        Self::AddIxRR,
        Self::LdSpIx,
        Self::JpIx,
        Self::ExSpIx,
        Self::LdRIxDisp,
        Self::LdIxDispR,
        Self::LdIxDispImm,
        Self::IncIxDisp,
        Self::DecIxDisp,
        Self::BitIxDisp,
        Self::ResIxDisp,
        Self::SetIxDisp,
    ];

    pub fn all() -> impl Iterator<Item = Self> {
        Self::BASE
            .iter()
            .copied()
            .chain(AluOperation::ALL.into_iter().flat_map(|operation| {
                [
                    Self::AluR(operation),
                    Self::AluImm(operation),
                    Self::AluIxDisp(operation),
                ]
            }))
            .chain(
                RotateShiftOperation::ALL
                    .into_iter()
                    .flat_map(|operation| [Self::Rot(operation), Self::RotIxDisp(operation)]),
            )
            .chain(InterruptMode::ALL.into_iter().map(Self::Im))
    }
}

impl Instruction {
    pub fn form(&self) -> InstructionForm {
        match self {
            Self::Nop => InstructionForm::Nop,
            Self::Halt => InstructionForm::Halt,
            Self::Di => InstructionForm::Di,
            Self::Ei => InstructionForm::Ei,
            Self::LdRR(..) => InstructionForm::LdRR,
            Self::LdRImm(..) => InstructionForm::LdRImm,
            Self::LdAddrA(..) => InstructionForm::LdAddrA,
            Self::LdAAddr(..) => InstructionForm::LdAAddr,
            Self::LdBCA => InstructionForm::LdBCA,
            Self::LdDEA => InstructionForm::LdDEA,
            Self::LdABC => InstructionForm::LdABC,
            Self::LdADE => InstructionForm::LdADE,
            Self::LdAddrHL(..) => InstructionForm::LdAddrHL,
            Self::LdHLAddr(..) => InstructionForm::LdHLAddr,
            Self::LdRRImm(..) => InstructionForm::LdRRImm,
            Self::LdSPHL => InstructionForm::LdSPHL,
            Self::Push(..) => InstructionForm::Push,
            Self::Pop(..) => InstructionForm::Pop,
            Self::AluR(operation, ..) => InstructionForm::AluR(*operation),
            Self::AluImm(operation, ..) => InstructionForm::AluImm(*operation),
            Self::IncR(..) => InstructionForm::IncR,
            Self::DecR(..) => InstructionForm::DecR,
            Self::IncRR(..) => InstructionForm::IncRR,
            Self::DecRR(..) => InstructionForm::DecRR,
            Self::AddHLRR(..) => InstructionForm::AddHLRR,
            Self::Jp(..) => InstructionForm::Jp,
            Self::JpCond(..) => InstructionForm::JpCond,
            Self::JpHL => InstructionForm::JpHL,
            Self::Jr(..) => InstructionForm::Jr,
            Self::JrCond(..) => InstructionForm::JrCond,
            Self::Djnz(..) => InstructionForm::Djnz,
            Self::Call(..) => InstructionForm::Call,
            Self::CallCond(..) => InstructionForm::CallCond,
            Self::Ret => InstructionForm::Ret,
            Self::RetCond(..) => InstructionForm::RetCond,
            Self::Reti => InstructionForm::Reti,
            Self::Retn => InstructionForm::Retn,
            Self::Rst(..) => InstructionForm::Rst,
            Self::Rlca => InstructionForm::Rlca,
            Self::Rrca => InstructionForm::Rrca,
            Self::Rla => InstructionForm::Rla,
            Self::Rra => InstructionForm::Rra,
            Self::ExAFAF => InstructionForm::ExAFAF,
            Self::Exx => InstructionForm::Exx,
            Self::ExDEHL => InstructionForm::ExDEHL,
            Self::ExSPHL => InstructionForm::ExSPHL,
            Self::Daa => InstructionForm::Daa,
            Self::Cpl => InstructionForm::Cpl,
            Self::Neg => InstructionForm::Neg,
            Self::Scf => InstructionForm::Scf,
            Self::Ccf => InstructionForm::Ccf,
            Self::InAPort(..) => InstructionForm::InAPort,
            Self::OutPortA(..) => InstructionForm::OutPortA,
            Self::InRC(..) => InstructionForm::InRC,
            Self::OutCR(..) => InstructionForm::OutCR,
            Self::Ini => InstructionForm::Ini,
            Self::Ind => InstructionForm::Ind,
            Self::Inir => InstructionForm::Inir,
            Self::Indr => InstructionForm::Indr,
            Self::Outi => InstructionForm::Outi,
            Self::Outd => InstructionForm::Outd,
            Self::Otir => InstructionForm::Otir,
            Self::Otdr => InstructionForm::Otdr,
            Self::Ldi => InstructionForm::Ldi,
            Self::Ldd => InstructionForm::Ldd,
            Self::Ldir => InstructionForm::Ldir,
            Self::Lddr => InstructionForm::Lddr,
            Self::Cpi => InstructionForm::Cpi,
            Self::Cpd => InstructionForm::Cpd,
            Self::Cpir => InstructionForm::Cpir,
            Self::Cpdr => InstructionForm::Cpdr,
            Self::Rot(operation, ..) => InstructionForm::Rot(*operation),
            Self::Bit(..) => InstructionForm::Bit,
            Self::Res(..) => InstructionForm::Res,
            Self::Set(..) => InstructionForm::Set,
            Self::SbcHLRR(..) => InstructionForm::SbcHLRR,
            Self::AdcHLRR(..) => InstructionForm::AdcHLRR,
            Self::LdAddrRR(..) => InstructionForm::LdAddrRR,
            Self::LdRRAddr(..) => InstructionForm::LdRRAddr,
            Self::LdIA => InstructionForm::LdIA,
            Self::LdRA => InstructionForm::LdRA,
            Self::LdAI => InstructionForm::LdAI,
            Self::LdAR => InstructionForm::LdAR,
            Self::Rrd => InstructionForm::Rrd,
            Self::Rld => InstructionForm::Rld,
            Self::Im(mode) => InstructionForm::Im(*mode),
            Self::LdIxImm(..) => InstructionForm::LdIxImm,
            Self::LdIxAddr(..) => InstructionForm::LdIxAddr,
            Self::LdAddrIx(..) => InstructionForm::LdAddrIx,
            Self::PushIx(..) => InstructionForm::PushIx,
            Self::PopIx(..) => InstructionForm::PopIx,
            Self::IncIx(..) => InstructionForm::IncIx,
            Self::DecIx(..) => InstructionForm::DecIx,
            Self::AddIxRR(..) => InstructionForm::AddIxRR,
            Self::LdSpIx(..) => InstructionForm::LdSpIx,
            Self::JpIx(..) => InstructionForm::JpIx,
            Self::ExSpIx(..) => InstructionForm::ExSpIx,
            Self::LdRIxDisp(..) => InstructionForm::LdRIxDisp,
            Self::LdIxDispR(..) => InstructionForm::LdIxDispR,
            Self::LdIxDispImm(..) => InstructionForm::LdIxDispImm,
            Self::IncIxDisp(..) => InstructionForm::IncIxDisp,
            Self::DecIxDisp(..) => InstructionForm::DecIxDisp,
            Self::AluIxDisp(operation, ..) => InstructionForm::AluIxDisp(*operation),
            Self::RotIxDisp(operation, ..) => InstructionForm::RotIxDisp(*operation),
            Self::BitIxDisp(..) => InstructionForm::BitIxDisp,
            Self::ResIxDisp(..) => InstructionForm::ResIxDisp,
            Self::SetIxDisp(..) => InstructionForm::SetIxDisp,
        }
    }

    pub fn control_flow(&self, pc: u16) -> ControlFlow {
        match self {
            Self::Jp(target) => ControlFlow::Jump { target: *target },
            Self::JpCond(_, target) => ControlFlow::Branch {
                target: *target,
                conditional: true,
            },
            Self::JpHL | Self::JpIx(_) => ControlFlow::IndirectJump,
            Self::Jr(displacement) => ControlFlow::Jump {
                target: relative_target(pc, *displacement),
            },
            Self::JrCond(_, displacement) | Self::Djnz(displacement) => ControlFlow::Branch {
                target: relative_target(pc, *displacement),
                conditional: true,
            },
            Self::Call(target) => ControlFlow::Call {
                target: *target,
                conditional: false,
            },
            Self::CallCond(_, target) => ControlFlow::Call {
                target: *target,
                conditional: true,
            },
            Self::Ret | Self::Reti | Self::Retn => ControlFlow::Return { conditional: false },
            Self::RetCond(_) => ControlFlow::Return { conditional: true },
            Self::Rst(target) => ControlFlow::Call {
                target: u16::from(*target),
                conditional: false,
            },
            _ => ControlFlow::FallThrough,
        }
    }
}

fn relative_target(pc: u16, displacement: i8) -> u16 {
    pc.wrapping_add(2)
        .wrapping_add_signed(i16::from(displacement))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlFlow {
    FallThrough,
    Branch { target: u16, conditional: bool },
    Jump { target: u16 },
    IndirectJump,
    Call { target: u16, conditional: bool },
    Return { conditional: bool },
}
