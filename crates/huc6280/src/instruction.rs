//! Declarative HuC6280 instruction and opcode profile.

use core::fmt;

/// HuC6280 assembly mnemonic.
///
/// The bit number is part of the printed mnemonic for RMB, SMB, BBR, and BBS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mnemonic {
    Adc,
    And,
    Asl,
    Bcc,
    Bcs,
    Beq,
    Bit,
    Bmi,
    Bne,
    Bpl,
    Bra,
    Brk,
    Bsr,
    Bvc,
    Bvs,
    Cla,
    Clc,
    Cld,
    Cli,
    Clv,
    Clx,
    Cly,
    Cmp,
    Cpx,
    Cpy,
    Csh,
    Csl,
    Dec,
    Dex,
    Dey,
    Eor,
    Inc,
    Inx,
    Iny,
    Jmp,
    Jsr,
    Lda,
    Ldx,
    Ldy,
    Lsr,
    Nop,
    Ora,
    Pha,
    Php,
    Phx,
    Phy,
    Pla,
    Plp,
    Plx,
    Ply,
    Rmb { bit: u8 },
    Rol,
    Ror,
    Rti,
    Rts,
    Sax,
    Say,
    Sbc,
    Sec,
    Sed,
    Sei,
    Set,
    Smb { bit: u8 },
    St0,
    St1,
    St2,
    Sta,
    Stx,
    Sty,
    Stz,
    Sxy,
    Tai,
    Tam,
    Tax,
    Tay,
    Tdd,
    Tia,
    Tii,
    Tin,
    Tma,
    Trb,
    Tsb,
    Tst,
    Tsx,
    Txa,
    Txs,
    Tya,
    Bbr { bit: u8 },
    Bbs { bit: u8 },
}

impl fmt::Display for Mnemonic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rmb { bit } => write!(f, "RMB{bit}"),
            Self::Smb { bit } => write!(f, "SMB{bit}"),
            Self::Bbr { bit } => write!(f, "BBR{bit}"),
            Self::Bbs { bit } => write!(f, "BBS{bit}"),
            other => write!(f, "{}", format!("{other:?}").to_ascii_uppercase()),
        }
    }
}

/// Operand interpretation selected by an opcode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AddressingMode {
    Implied,
    Accumulator,
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    AbsoluteIndirect,
    ZeroPageIndexedIndirectX,
    ZeroPageIndirectIndexedY,
    ZeroPageIndirect,
    AbsoluteIndexedIndirectX,
    Relative,
    ZeroPageRelative,
    BlockTransfer,
    ImmediateZeroPage,
    ImmediateZeroPageX,
    ImmediateAbsolute,
    ImmediateAbsoluteX,
}

impl AddressingMode {
    pub const fn operand_len(self) -> usize {
        match self {
            Self::Implied | Self::Accumulator => 0,
            Self::Immediate
            | Self::ZeroPage
            | Self::ZeroPageX
            | Self::ZeroPageY
            | Self::ZeroPageIndexedIndirectX
            | Self::ZeroPageIndirectIndexedY
            | Self::ZeroPageIndirect
            | Self::Relative => 1,
            Self::Absolute
            | Self::AbsoluteX
            | Self::AbsoluteY
            | Self::AbsoluteIndirect
            | Self::AbsoluteIndexedIndirectX
            | Self::ZeroPageRelative
            | Self::ImmediateZeroPage
            | Self::ImmediateZeroPageX => 2,
            Self::ImmediateAbsolute | Self::ImmediateAbsoluteX => 3,
            Self::BlockTransfer => 6,
        }
    }
}

/// A semantic instruction form with operand values removed.
///
/// Only forms assigned by Hudson's software manual belong to this denominator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InstructionForm {
    pub mnemonic: Mnemonic,
    pub mode: AddressingMode,
}

impl InstructionForm {
    pub const fn new(mnemonic: Mnemonic, mode: AddressingMode) -> Self {
        Self { mnemonic, mode }
    }

    /// Complete, duplicate-free semantic-form denominator for this profile.
    pub fn all() -> impl Iterator<Item = Self> {
        (0..OPCODE_TABLE.len()).filter_map(|index| {
            if !opcode_is_documented(index as u8) {
                return None;
            }
            let form = OPCODE_TABLE[index];
            let seen = (0..index).any(|previous| {
                opcode_is_documented(previous as u8) && OPCODE_TABLE[previous] == form
            });
            (!seen).then_some(form)
        })
    }

    pub const fn encoded_len(self) -> usize {
        1 + self.mode.operand_len()
    }
}

const fn op(mnemonic: Mnemonic, mode: AddressingMode) -> InstructionForm {
    InstructionForm::new(mnemonic, mode)
}

use AddressingMode::*;
use Mnemonic::*;

// Full selector layout used internally to keep the manual matrix legible. The
// placeholder forms at the 22 undefined slots are never exposed as supported
// instructions; `instruction_form` filters them out.
pub(crate) const OPCODE_TABLE: [InstructionForm; 256] = [
    // $00-$0F
    op(Brk, Implied),
    op(Ora, ZeroPageIndexedIndirectX),
    op(Sxy, Implied),
    op(St0, Immediate),
    op(Tsb, ZeroPage),
    op(Ora, ZeroPage),
    op(Asl, ZeroPage),
    op(Rmb { bit: 0 }, ZeroPage),
    op(Php, Implied),
    op(Ora, Immediate),
    op(Asl, Accumulator),
    op(Nop, Implied),
    op(Tsb, Absolute),
    op(Ora, Absolute),
    op(Asl, Absolute),
    op(Bbr { bit: 0 }, ZeroPageRelative),
    // $10-$1F
    op(Bpl, Relative),
    op(Ora, ZeroPageIndirectIndexedY),
    op(Ora, ZeroPageIndirect),
    op(St1, Immediate),
    op(Trb, ZeroPage),
    op(Ora, ZeroPageX),
    op(Asl, ZeroPageX),
    op(Rmb { bit: 1 }, ZeroPage),
    op(Clc, Implied),
    op(Ora, AbsoluteY),
    op(Inc, Accumulator),
    op(Nop, Implied),
    op(Trb, Absolute),
    op(Ora, AbsoluteX),
    op(Asl, AbsoluteX),
    op(Bbr { bit: 1 }, ZeroPageRelative),
    // $20-$2F
    op(Jsr, Absolute),
    op(And, ZeroPageIndexedIndirectX),
    op(Sax, Implied),
    op(St2, Immediate),
    op(Bit, ZeroPage),
    op(And, ZeroPage),
    op(Rol, ZeroPage),
    op(Rmb { bit: 2 }, ZeroPage),
    op(Plp, Implied),
    op(And, Immediate),
    op(Rol, Accumulator),
    op(Nop, Implied),
    op(Bit, Absolute),
    op(And, Absolute),
    op(Rol, Absolute),
    op(Bbr { bit: 2 }, ZeroPageRelative),
    // $30-$3F
    op(Bmi, Relative),
    op(And, ZeroPageIndirectIndexedY),
    op(And, ZeroPageIndirect),
    op(Nop, Implied),
    op(Bit, ZeroPageX),
    op(And, ZeroPageX),
    op(Rol, ZeroPageX),
    op(Rmb { bit: 3 }, ZeroPage),
    op(Sec, Implied),
    op(And, AbsoluteY),
    op(Dec, Accumulator),
    op(Nop, Implied),
    op(Bit, AbsoluteX),
    op(And, AbsoluteX),
    op(Rol, AbsoluteX),
    op(Bbr { bit: 3 }, ZeroPageRelative),
    // $40-$4F
    op(Rti, Implied),
    op(Eor, ZeroPageIndexedIndirectX),
    op(Say, Implied),
    op(Tma, Immediate),
    op(Bsr, Relative),
    op(Eor, ZeroPage),
    op(Lsr, ZeroPage),
    op(Rmb { bit: 4 }, ZeroPage),
    op(Pha, Implied),
    op(Eor, Immediate),
    op(Lsr, Accumulator),
    op(Nop, Implied),
    op(Jmp, Absolute),
    op(Eor, Absolute),
    op(Lsr, Absolute),
    op(Bbr { bit: 4 }, ZeroPageRelative),
    // $50-$5F
    op(Bvc, Relative),
    op(Eor, ZeroPageIndirectIndexedY),
    op(Eor, ZeroPageIndirect),
    op(Tam, Immediate),
    op(Csl, Implied),
    op(Eor, ZeroPageX),
    op(Lsr, ZeroPageX),
    op(Rmb { bit: 5 }, ZeroPage),
    op(Cli, Implied),
    op(Eor, AbsoluteY),
    op(Phy, Implied),
    op(Nop, Implied),
    op(Nop, Implied),
    op(Eor, AbsoluteX),
    op(Lsr, AbsoluteX),
    op(Bbr { bit: 5 }, ZeroPageRelative),
    // $60-$6F
    op(Rts, Implied),
    op(Adc, ZeroPageIndexedIndirectX),
    op(Cla, Implied),
    op(Nop, Implied),
    op(Stz, ZeroPage),
    op(Adc, ZeroPage),
    op(Ror, ZeroPage),
    op(Rmb { bit: 6 }, ZeroPage),
    op(Pla, Implied),
    op(Adc, Immediate),
    op(Ror, Accumulator),
    op(Nop, Implied),
    op(Jmp, AbsoluteIndirect),
    op(Adc, Absolute),
    op(Ror, Absolute),
    op(Bbr { bit: 6 }, ZeroPageRelative),
    // $70-$7F
    op(Bvs, Relative),
    op(Adc, ZeroPageIndirectIndexedY),
    op(Adc, ZeroPageIndirect),
    op(Tii, BlockTransfer),
    op(Stz, ZeroPageX),
    op(Adc, ZeroPageX),
    op(Ror, ZeroPageX),
    op(Rmb { bit: 7 }, ZeroPage),
    op(Sei, Implied),
    op(Adc, AbsoluteY),
    op(Ply, Implied),
    op(Nop, Implied),
    op(Jmp, AbsoluteIndexedIndirectX),
    op(Adc, AbsoluteX),
    op(Ror, AbsoluteX),
    op(Bbr { bit: 7 }, ZeroPageRelative),
    // $80-$8F
    op(Bra, Relative),
    op(Sta, ZeroPageIndexedIndirectX),
    op(Clx, Implied),
    op(Tst, ImmediateZeroPage),
    op(Sty, ZeroPage),
    op(Sta, ZeroPage),
    op(Stx, ZeroPage),
    op(Smb { bit: 0 }, ZeroPage),
    op(Dey, Implied),
    op(Bit, Immediate),
    op(Txa, Implied),
    op(Nop, Implied),
    op(Sty, Absolute),
    op(Sta, Absolute),
    op(Stx, Absolute),
    op(Bbs { bit: 0 }, ZeroPageRelative),
    // $90-$9F
    op(Bcc, Relative),
    op(Sta, ZeroPageIndirectIndexedY),
    op(Sta, ZeroPageIndirect),
    op(Tst, ImmediateAbsolute),
    op(Sty, ZeroPageX),
    op(Sta, ZeroPageX),
    op(Stx, ZeroPageY),
    op(Smb { bit: 1 }, ZeroPage),
    op(Tya, Implied),
    op(Sta, AbsoluteY),
    op(Txs, Implied),
    op(Nop, Implied),
    op(Stz, Absolute),
    op(Sta, AbsoluteX),
    op(Stz, AbsoluteX),
    op(Bbs { bit: 1 }, ZeroPageRelative),
    // $A0-$AF
    op(Ldy, Immediate),
    op(Lda, ZeroPageIndexedIndirectX),
    op(Ldx, Immediate),
    op(Tst, ImmediateZeroPageX),
    op(Ldy, ZeroPage),
    op(Lda, ZeroPage),
    op(Ldx, ZeroPage),
    op(Smb { bit: 2 }, ZeroPage),
    op(Tay, Implied),
    op(Lda, Immediate),
    op(Tax, Implied),
    op(Nop, Implied),
    op(Ldy, Absolute),
    op(Lda, Absolute),
    op(Ldx, Absolute),
    op(Bbs { bit: 2 }, ZeroPageRelative),
    // $B0-$BF
    op(Bcs, Relative),
    op(Lda, ZeroPageIndirectIndexedY),
    op(Lda, ZeroPageIndirect),
    op(Tst, ImmediateAbsoluteX),
    op(Ldy, ZeroPageX),
    op(Lda, ZeroPageX),
    op(Ldx, ZeroPageY),
    op(Smb { bit: 3 }, ZeroPage),
    op(Clv, Implied),
    op(Lda, AbsoluteY),
    op(Tsx, Implied),
    op(Nop, Implied),
    op(Ldy, AbsoluteX),
    op(Lda, AbsoluteX),
    op(Ldx, AbsoluteY),
    op(Bbs { bit: 3 }, ZeroPageRelative),
    // $C0-$CF
    op(Cpy, Immediate),
    op(Cmp, ZeroPageIndexedIndirectX),
    op(Cly, Implied),
    op(Tdd, BlockTransfer),
    op(Cpy, ZeroPage),
    op(Cmp, ZeroPage),
    op(Dec, ZeroPage),
    op(Smb { bit: 4 }, ZeroPage),
    op(Iny, Implied),
    op(Cmp, Immediate),
    op(Dex, Implied),
    op(Nop, Implied),
    op(Cpy, Absolute),
    op(Cmp, Absolute),
    op(Dec, Absolute),
    op(Bbs { bit: 4 }, ZeroPageRelative),
    // $D0-$DF
    op(Bne, Relative),
    op(Cmp, ZeroPageIndirectIndexedY),
    op(Cmp, ZeroPageIndirect),
    op(Tin, BlockTransfer),
    op(Csh, Implied),
    op(Cmp, ZeroPageX),
    op(Dec, ZeroPageX),
    op(Smb { bit: 5 }, ZeroPage),
    op(Cld, Implied),
    op(Cmp, AbsoluteY),
    op(Phx, Implied),
    op(Nop, Implied),
    op(Nop, Implied),
    op(Cmp, AbsoluteX),
    op(Dec, AbsoluteX),
    op(Bbs { bit: 5 }, ZeroPageRelative),
    // $E0-$EF
    op(Cpx, Immediate),
    op(Sbc, ZeroPageIndexedIndirectX),
    op(Nop, Implied),
    op(Tia, BlockTransfer),
    op(Cpx, ZeroPage),
    op(Sbc, ZeroPage),
    op(Inc, ZeroPage),
    op(Smb { bit: 6 }, ZeroPage),
    op(Inx, Implied),
    op(Sbc, Immediate),
    op(Nop, Implied),
    op(Nop, Implied),
    op(Cpx, Absolute),
    op(Sbc, Absolute),
    op(Inc, Absolute),
    op(Bbs { bit: 6 }, ZeroPageRelative),
    // $F0-$FF
    op(Beq, Relative),
    op(Sbc, ZeroPageIndirectIndexedY),
    op(Sbc, ZeroPageIndirect),
    op(Tai, BlockTransfer),
    op(Set, Implied),
    op(Sbc, ZeroPageX),
    op(Inc, ZeroPageX),
    op(Smb { bit: 7 }, ZeroPage),
    op(Sed, Implied),
    op(Sbc, AbsoluteY),
    op(Plx, Implied),
    op(Nop, Implied),
    op(Nop, Implied),
    op(Sbc, AbsoluteX),
    op(Inc, AbsoluteX),
    op(Bbs { bit: 7 }, ZeroPageRelative),
];

/// Whether an opcode is part of Hudson's documented instruction table.
pub const fn opcode_is_documented(opcode: u8) -> bool {
    !matches!(
        opcode,
        0x0B | 0x1B
            | 0x2B
            | 0x33
            | 0x3B
            | 0x4B
            | 0x5B
            | 0x5C
            | 0x63
            | 0x6B
            | 0x7B
            | 0x8B
            | 0x9B
            | 0xAB
            | 0xBB
            | 0xCB
            | 0xDB
            | 0xDC
            | 0xE2
            | 0xEB
            | 0xFB
            | 0xFC
    )
}

/// Return the manual-defined form for an opcode selector.
///
/// The 22 slots left undefined by Hudson's software manual return `None`.
pub const fn instruction_form(opcode: u8) -> Option<InstructionForm> {
    if opcode_is_documented(opcode) {
        Some(OPCODE_TABLE[opcode as usize])
    } else {
        None
    }
}

pub(crate) fn canonical_opcode(form: InstructionForm) -> Option<u8> {
    OPCODE_TABLE
        .iter()
        .enumerate()
        .find(|(opcode, candidate)| opcode_is_documented(*opcode as u8) && **candidate == form)
        .map(|(opcode, _)| opcode as u8)
}

/// Placement-aware static control-flow classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlFlow {
    FallThrough {
        next: u16,
    },
    Branch {
        target: u16,
        fallthrough: Option<u16>,
    },
    Call {
        target: u16,
        return_address: u16,
    },
    Jump {
        target: Option<u16>,
    },
    Return,
    Interrupt,
}
