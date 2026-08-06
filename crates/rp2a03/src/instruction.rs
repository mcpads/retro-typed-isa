//! Declarative RP2A03 instruction-selector profile.

use core::fmt;

/// Official and physically decoded RP2A03 instruction mnemonics.
///
/// Mnemonics absent from the original MOS programming model describe
/// unofficial selector behavior. They are modeled so existing Famicom code is
/// lossless, not as a recommendation to use unstable hardware behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mnemonic {
    Adc,
    Ahx,
    Alr,
    Anc,
    And,
    Arr,
    Asl,
    Axs,
    Bcc,
    Bcs,
    Beq,
    Bit,
    Bmi,
    Bne,
    Bpl,
    Brk,
    Bvc,
    Bvs,
    Clc,
    Cld,
    Cli,
    Clv,
    Cmp,
    Cpx,
    Cpy,
    Dcp,
    Dec,
    Dex,
    Dey,
    Eor,
    Inc,
    Inx,
    Iny,
    Isc,
    Jmp,
    Jsr,
    Kil,
    Las,
    Lax,
    Lda,
    Ldx,
    Ldy,
    Lsr,
    Nop,
    Ora,
    Pha,
    Php,
    Pla,
    Plp,
    Rla,
    Rol,
    Ror,
    Rra,
    Rti,
    Rts,
    Sax,
    Sbc,
    Sec,
    Sed,
    Sei,
    Shx,
    Shy,
    Slo,
    Sre,
    Sta,
    Stx,
    Sty,
    Tas,
    Tax,
    Tay,
    Tsx,
    Txa,
    Txs,
    Tya,
    Xaa,
}

impl fmt::Display for Mnemonic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", format!("{self:?}").to_ascii_uppercase())
    }
}

/// Operand interpretation selected by an RP2A03 opcode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AddressingMode {
    Implied,
    Accumulator,
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Relative,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    AbsoluteIndirect,
    ZeroPageIndexedIndirectX,
    ZeroPageIndirectIndexedY,
}

impl AddressingMode {
    pub const fn operand_len(self) -> usize {
        match self {
            Self::Implied | Self::Accumulator => 0,
            Self::Immediate
            | Self::ZeroPage
            | Self::ZeroPageX
            | Self::ZeroPageY
            | Self::Relative
            | Self::ZeroPageIndexedIndirectX
            | Self::ZeroPageIndirectIndexedY => 1,
            Self::Absolute | Self::AbsoluteX | Self::AbsoluteY | Self::AbsoluteIndirect => 2,
        }
    }
}

/// A semantic instruction form with operand values removed.
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
            let form = OPCODE_TABLE[index];
            (!OPCODE_TABLE[..index].contains(&form)).then_some(form)
        })
    }

    pub const fn encoded_len(self) -> usize {
        1 + self.mode.operand_len()
    }
}

/// Static control-transfer classification for one placed instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlFlow {
    FallThrough {
        next: u16,
    },
    Branch {
        target: u16,
        fallthrough: Option<u16>,
    },
    Jump {
        target: Option<u16>,
    },
    Call {
        target: u16,
        return_address: u16,
    },
    Return,
    Interrupt,
    Stop,
}

const fn op(mnemonic: Mnemonic, mode: AddressingMode) -> InstructionForm {
    InstructionForm::new(mnemonic, mode)
}

use AddressingMode::*;
use Mnemonic::*;

/// Complete RP2A03 selector profile, indexed by opcode byte.
///
/// This includes all unofficial NMOS 6502-family selectors physically decoded
/// by the RP2A03. [`opcode_is_documented`] distinguishes the original 151
/// documented selectors from the remaining 105 selectors.
pub const OPCODE_TABLE: [InstructionForm; 256] = [
    // $00-$0F
    op(Brk, Implied),
    op(Ora, ZeroPageIndexedIndirectX),
    op(Kil, Implied),
    op(Slo, ZeroPageIndexedIndirectX),
    op(Nop, ZeroPage),
    op(Ora, ZeroPage),
    op(Asl, ZeroPage),
    op(Slo, ZeroPage),
    op(Php, Implied),
    op(Ora, Immediate),
    op(Asl, Accumulator),
    op(Anc, Immediate),
    op(Nop, Absolute),
    op(Ora, Absolute),
    op(Asl, Absolute),
    op(Slo, Absolute),
    // $10-$1F
    op(Bpl, Relative),
    op(Ora, ZeroPageIndirectIndexedY),
    op(Kil, Implied),
    op(Slo, ZeroPageIndirectIndexedY),
    op(Nop, ZeroPageX),
    op(Ora, ZeroPageX),
    op(Asl, ZeroPageX),
    op(Slo, ZeroPageX),
    op(Clc, Implied),
    op(Ora, AbsoluteY),
    op(Nop, Implied),
    op(Slo, AbsoluteY),
    op(Nop, AbsoluteX),
    op(Ora, AbsoluteX),
    op(Asl, AbsoluteX),
    op(Slo, AbsoluteX),
    // $20-$2F
    op(Jsr, Absolute),
    op(And, ZeroPageIndexedIndirectX),
    op(Kil, Implied),
    op(Rla, ZeroPageIndexedIndirectX),
    op(Bit, ZeroPage),
    op(And, ZeroPage),
    op(Rol, ZeroPage),
    op(Rla, ZeroPage),
    op(Plp, Implied),
    op(And, Immediate),
    op(Rol, Accumulator),
    op(Anc, Immediate),
    op(Bit, Absolute),
    op(And, Absolute),
    op(Rol, Absolute),
    op(Rla, Absolute),
    // $30-$3F
    op(Bmi, Relative),
    op(And, ZeroPageIndirectIndexedY),
    op(Kil, Implied),
    op(Rla, ZeroPageIndirectIndexedY),
    op(Nop, ZeroPageX),
    op(And, ZeroPageX),
    op(Rol, ZeroPageX),
    op(Rla, ZeroPageX),
    op(Sec, Implied),
    op(And, AbsoluteY),
    op(Nop, Implied),
    op(Rla, AbsoluteY),
    op(Nop, AbsoluteX),
    op(And, AbsoluteX),
    op(Rol, AbsoluteX),
    op(Rla, AbsoluteX),
    // $40-$4F
    op(Rti, Implied),
    op(Eor, ZeroPageIndexedIndirectX),
    op(Kil, Implied),
    op(Sre, ZeroPageIndexedIndirectX),
    op(Nop, ZeroPage),
    op(Eor, ZeroPage),
    op(Lsr, ZeroPage),
    op(Sre, ZeroPage),
    op(Pha, Implied),
    op(Eor, Immediate),
    op(Lsr, Accumulator),
    op(Alr, Immediate),
    op(Jmp, Absolute),
    op(Eor, Absolute),
    op(Lsr, Absolute),
    op(Sre, Absolute),
    // $50-$5F
    op(Bvc, Relative),
    op(Eor, ZeroPageIndirectIndexedY),
    op(Kil, Implied),
    op(Sre, ZeroPageIndirectIndexedY),
    op(Nop, ZeroPageX),
    op(Eor, ZeroPageX),
    op(Lsr, ZeroPageX),
    op(Sre, ZeroPageX),
    op(Cli, Implied),
    op(Eor, AbsoluteY),
    op(Nop, Implied),
    op(Sre, AbsoluteY),
    op(Nop, AbsoluteX),
    op(Eor, AbsoluteX),
    op(Lsr, AbsoluteX),
    op(Sre, AbsoluteX),
    // $60-$6F
    op(Rts, Implied),
    op(Adc, ZeroPageIndexedIndirectX),
    op(Kil, Implied),
    op(Rra, ZeroPageIndexedIndirectX),
    op(Nop, ZeroPage),
    op(Adc, ZeroPage),
    op(Ror, ZeroPage),
    op(Rra, ZeroPage),
    op(Pla, Implied),
    op(Adc, Immediate),
    op(Ror, Accumulator),
    op(Arr, Immediate),
    op(Jmp, AbsoluteIndirect),
    op(Adc, Absolute),
    op(Ror, Absolute),
    op(Rra, Absolute),
    // $70-$7F
    op(Bvs, Relative),
    op(Adc, ZeroPageIndirectIndexedY),
    op(Kil, Implied),
    op(Rra, ZeroPageIndirectIndexedY),
    op(Nop, ZeroPageX),
    op(Adc, ZeroPageX),
    op(Ror, ZeroPageX),
    op(Rra, ZeroPageX),
    op(Sei, Implied),
    op(Adc, AbsoluteY),
    op(Nop, Implied),
    op(Rra, AbsoluteY),
    op(Nop, AbsoluteX),
    op(Adc, AbsoluteX),
    op(Ror, AbsoluteX),
    op(Rra, AbsoluteX),
    // $80-$8F
    op(Nop, Immediate),
    op(Sta, ZeroPageIndexedIndirectX),
    op(Nop, Immediate),
    op(Sax, ZeroPageIndexedIndirectX),
    op(Sty, ZeroPage),
    op(Sta, ZeroPage),
    op(Stx, ZeroPage),
    op(Sax, ZeroPage),
    op(Dey, Implied),
    op(Nop, Immediate),
    op(Txa, Implied),
    op(Xaa, Immediate),
    op(Sty, Absolute),
    op(Sta, Absolute),
    op(Stx, Absolute),
    op(Sax, Absolute),
    // $90-$9F
    op(Bcc, Relative),
    op(Sta, ZeroPageIndirectIndexedY),
    op(Kil, Implied),
    op(Ahx, ZeroPageIndirectIndexedY),
    op(Sty, ZeroPageX),
    op(Sta, ZeroPageX),
    op(Stx, ZeroPageY),
    op(Sax, ZeroPageY),
    op(Tya, Implied),
    op(Sta, AbsoluteY),
    op(Txs, Implied),
    op(Tas, AbsoluteY),
    op(Shy, AbsoluteX),
    op(Sta, AbsoluteX),
    op(Shx, AbsoluteY),
    op(Ahx, AbsoluteY),
    // $A0-$AF
    op(Ldy, Immediate),
    op(Lda, ZeroPageIndexedIndirectX),
    op(Ldx, Immediate),
    op(Lax, ZeroPageIndexedIndirectX),
    op(Ldy, ZeroPage),
    op(Lda, ZeroPage),
    op(Ldx, ZeroPage),
    op(Lax, ZeroPage),
    op(Tay, Implied),
    op(Lda, Immediate),
    op(Tax, Implied),
    op(Lax, Immediate),
    op(Ldy, Absolute),
    op(Lda, Absolute),
    op(Ldx, Absolute),
    op(Lax, Absolute),
    // $B0-$BF
    op(Bcs, Relative),
    op(Lda, ZeroPageIndirectIndexedY),
    op(Kil, Implied),
    op(Lax, ZeroPageIndirectIndexedY),
    op(Ldy, ZeroPageX),
    op(Lda, ZeroPageX),
    op(Ldx, ZeroPageY),
    op(Lax, ZeroPageY),
    op(Clv, Implied),
    op(Lda, AbsoluteY),
    op(Tsx, Implied),
    op(Las, AbsoluteY),
    op(Ldy, AbsoluteX),
    op(Lda, AbsoluteX),
    op(Ldx, AbsoluteY),
    op(Lax, AbsoluteY),
    // $C0-$CF
    op(Cpy, Immediate),
    op(Cmp, ZeroPageIndexedIndirectX),
    op(Nop, Immediate),
    op(Dcp, ZeroPageIndexedIndirectX),
    op(Cpy, ZeroPage),
    op(Cmp, ZeroPage),
    op(Dec, ZeroPage),
    op(Dcp, ZeroPage),
    op(Iny, Implied),
    op(Cmp, Immediate),
    op(Dex, Implied),
    op(Axs, Immediate),
    op(Cpy, Absolute),
    op(Cmp, Absolute),
    op(Dec, Absolute),
    op(Dcp, Absolute),
    // $D0-$DF
    op(Bne, Relative),
    op(Cmp, ZeroPageIndirectIndexedY),
    op(Kil, Implied),
    op(Dcp, ZeroPageIndirectIndexedY),
    op(Nop, ZeroPageX),
    op(Cmp, ZeroPageX),
    op(Dec, ZeroPageX),
    op(Dcp, ZeroPageX),
    op(Cld, Implied),
    op(Cmp, AbsoluteY),
    op(Nop, Implied),
    op(Dcp, AbsoluteY),
    op(Nop, AbsoluteX),
    op(Cmp, AbsoluteX),
    op(Dec, AbsoluteX),
    op(Dcp, AbsoluteX),
    // $E0-$EF
    op(Cpx, Immediate),
    op(Sbc, ZeroPageIndexedIndirectX),
    op(Nop, Immediate),
    op(Isc, ZeroPageIndexedIndirectX),
    op(Cpx, ZeroPage),
    op(Sbc, ZeroPage),
    op(Inc, ZeroPage),
    op(Isc, ZeroPage),
    op(Inx, Implied),
    op(Sbc, Immediate),
    op(Nop, Implied),
    op(Sbc, Immediate),
    op(Cpx, Absolute),
    op(Sbc, Absolute),
    op(Inc, Absolute),
    op(Isc, Absolute),
    // $F0-$FF
    op(Beq, Relative),
    op(Sbc, ZeroPageIndirectIndexedY),
    op(Kil, Implied),
    op(Isc, ZeroPageIndirectIndexedY),
    op(Nop, ZeroPageX),
    op(Sbc, ZeroPageX),
    op(Inc, ZeroPageX),
    op(Isc, ZeroPageX),
    op(Sed, Implied),
    op(Sbc, AbsoluteY),
    op(Nop, Implied),
    op(Isc, AbsoluteY),
    op(Nop, AbsoluteX),
    op(Sbc, AbsoluteX),
    op(Inc, AbsoluteX),
    op(Isc, AbsoluteX),
];

/// Whether an opcode belongs to the original documented 151-selector set.
pub const fn opcode_is_documented(opcode: u8) -> bool {
    matches!(
        opcode,
        0x00 | 0x01
            | 0x05
            | 0x06
            | 0x08
            | 0x09
            | 0x0A
            | 0x0D
            | 0x0E
            | 0x10
            | 0x11
            | 0x15
            | 0x16
            | 0x18
            | 0x19
            | 0x1D
            | 0x1E
            | 0x20
            | 0x21
            | 0x24
            | 0x25
            | 0x26
            | 0x28
            | 0x29
            | 0x2A
            | 0x2C
            | 0x2D
            | 0x2E
            | 0x30
            | 0x31
            | 0x35
            | 0x36
            | 0x38
            | 0x39
            | 0x3D
            | 0x3E
            | 0x40
            | 0x41
            | 0x45
            | 0x46
            | 0x48
            | 0x49
            | 0x4A
            | 0x4C
            | 0x4D
            | 0x4E
            | 0x50
            | 0x51
            | 0x55
            | 0x56
            | 0x58
            | 0x59
            | 0x5D
            | 0x5E
            | 0x60
            | 0x61
            | 0x65
            | 0x66
            | 0x68
            | 0x69
            | 0x6A
            | 0x6C
            | 0x6D
            | 0x6E
            | 0x70
            | 0x71
            | 0x75
            | 0x76
            | 0x78
            | 0x79
            | 0x7D
            | 0x7E
            | 0x81
            | 0x84
            | 0x85
            | 0x86
            | 0x88
            | 0x8A
            | 0x8C
            | 0x8D
            | 0x8E
            | 0x90
            | 0x91
            | 0x94
            | 0x95
            | 0x96
            | 0x98
            | 0x99
            | 0x9A
            | 0x9D
            | 0xA0
            | 0xA1
            | 0xA2
            | 0xA4
            | 0xA5
            | 0xA6
            | 0xA8
            | 0xA9
            | 0xAA
            | 0xAC
            | 0xAD
            | 0xAE
            | 0xB0
            | 0xB1
            | 0xB4
            | 0xB5
            | 0xB6
            | 0xB8
            | 0xB9
            | 0xBA
            | 0xBC
            | 0xBD
            | 0xBE
            | 0xC0
            | 0xC1
            | 0xC4
            | 0xC5
            | 0xC6
            | 0xC8
            | 0xC9
            | 0xCA
            | 0xCC
            | 0xCD
            | 0xCE
            | 0xD0
            | 0xD1
            | 0xD5
            | 0xD6
            | 0xD8
            | 0xD9
            | 0xDD
            | 0xDE
            | 0xE0
            | 0xE1
            | 0xE4
            | 0xE5
            | 0xE6
            | 0xE8
            | 0xE9
            | 0xEA
            | 0xEC
            | 0xED
            | 0xEE
            | 0xF0
            | 0xF1
            | 0xF5
            | 0xF6
            | 0xF8
            | 0xF9
            | 0xFD
            | 0xFE
    )
}

/// Canonical selector for a semantic form.
///
/// The documented selector wins when both documented and unofficial aliases
/// exist. Otherwise the lowest selector is stable.
pub(crate) fn canonical_opcode(form: InstructionForm) -> Option<u8> {
    OPCODE_TABLE
        .iter()
        .enumerate()
        .filter(|(_, candidate)| **candidate == form)
        .min_by_key(|(index, _)| (!opcode_is_documented(*index as u8), *index))
        .map(|(index, _)| index as u8)
}
