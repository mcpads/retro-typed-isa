//! Declarative W65C816 opcode profile.

use core::fmt;

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
    Brl,
    Bvc,
    Bvs,
    Clc,
    Cld,
    Cli,
    Clv,
    Cmp,
    Cop,
    Cpx,
    Cpy,
    Dec,
    Dex,
    Dey,
    Eor,
    Inc,
    Inx,
    Iny,
    Jml,
    Jmp,
    Jsl,
    Jsr,
    Lda,
    Ldx,
    Ldy,
    Lsr,
    Mvn,
    Mvp,
    Nop,
    Ora,
    Pea,
    Pei,
    Per,
    Pha,
    Phb,
    Phd,
    Phk,
    Php,
    Phx,
    Phy,
    Pla,
    Plb,
    Pld,
    Plp,
    Plx,
    Ply,
    Rep,
    Rol,
    Ror,
    Rti,
    Rtl,
    Rts,
    Sbc,
    Sec,
    Sed,
    Sei,
    Sep,
    Sta,
    Stp,
    Stx,
    Sty,
    Stz,
    Tax,
    Tay,
    Tcd,
    Tcs,
    Tdc,
    Trb,
    Tsb,
    Tsc,
    Tsx,
    Txa,
    Txs,
    Txy,
    Tya,
    Tyx,
    Wai,
    Wdm,
    Xba,
    Xce,
}

impl fmt::Display for Mnemonic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self).map(|_| ())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AddressingMode {
    Implied,
    Accumulator,
    Immediate,
    ImmediateByte,
    DirectPage,
    DirectPageX,
    DirectPageY,
    DirectPageIndirect,
    DirectPageIndirectLong,
    DirectPageIndexedIndirectX,
    DirectPageIndirectIndexedY,
    DirectPageIndirectLongIndexedY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    AbsoluteLong,
    AbsoluteLongX,
    AbsoluteIndirect,
    AbsoluteIndexedIndirectX,
    AbsoluteIndirectLong,
    Relative8,
    Relative16,
    StackRelative,
    StackRelativeIndirectIndexedY,
    BlockMove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Width {
    Eight,
    Sixteen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidthState {
    pub accumulator: Width,
    pub index: Width,
}

impl WidthState {
    pub const M8_X8: Self = Self::new(Width::Eight, Width::Eight);
    pub const M8_X16: Self = Self::new(Width::Eight, Width::Sixteen);
    pub const M16_X8: Self = Self::new(Width::Sixteen, Width::Eight);
    pub const M16_X16: Self = Self::new(Width::Sixteen, Width::Sixteen);

    pub const fn new(accumulator: Width, index: Width) -> Self {
        Self { accumulator, index }
    }

    pub(crate) fn apply_rep(&mut self, mask: u8) {
        if mask & 0x20 != 0 {
            self.accumulator = Width::Sixteen;
        }
        if mask & 0x10 != 0 {
            self.index = Width::Sixteen;
        }
    }

    pub(crate) fn apply_sep(&mut self, mask: u8) {
        if mask & 0x20 != 0 {
            self.accumulator = Width::Eight;
        }
        if mask & 0x10 != 0 {
            self.index = Width::Eight;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InstructionForm {
    pub mnemonic: Mnemonic,
    pub mode: AddressingMode,
}

impl InstructionForm {
    pub const fn new(mnemonic: Mnemonic, mode: AddressingMode) -> Self {
        Self { mnemonic, mode }
    }

    /// Complete semantic-form denominator for profile-level checks.
    pub fn all() -> impl Iterator<Item = Self> {
        OPCODE_TABLE.into_iter()
    }

    pub fn operand_len(self, widths: WidthState) -> usize {
        use AddressingMode::*;
        match self.mode {
            Implied | Accumulator => 0,
            ImmediateByte => 1,
            Immediate => {
                let width = if matches!(
                    self.mnemonic,
                    Mnemonic::Ldx | Mnemonic::Ldy | Mnemonic::Cpx | Mnemonic::Cpy
                ) {
                    widths.index
                } else {
                    widths.accumulator
                };
                match width {
                    Width::Eight => 1,
                    Width::Sixteen => 2,
                }
            }
            DirectPage
            | DirectPageX
            | DirectPageY
            | DirectPageIndirect
            | DirectPageIndirectLong
            | DirectPageIndexedIndirectX
            | DirectPageIndirectIndexedY
            | DirectPageIndirectLongIndexedY
            | Relative8
            | StackRelative
            | StackRelativeIndirectIndexedY => 1,
            Absolute
            | AbsoluteX
            | AbsoluteY
            | AbsoluteIndirect
            | AbsoluteIndexedIndirectX
            | AbsoluteIndirectLong
            | Relative16
            | BlockMove => 2,
            AbsoluteLong | AbsoluteLongX => 3,
        }
    }
}

const fn op(mnemonic: Mnemonic, mode: AddressingMode) -> InstructionForm {
    InstructionForm::new(mnemonic, mode)
}

use AddressingMode::*;
use Mnemonic::*;

/// The complete, official W65C816 opcode profile. Every byte value is valid.
pub const OPCODE_TABLE: [InstructionForm; 256] = [
    // $00-$0F
    op(Brk, ImmediateByte),
    op(Ora, DirectPageIndexedIndirectX),
    op(Cop, ImmediateByte),
    op(Ora, StackRelative),
    op(Tsb, DirectPage),
    op(Ora, DirectPage),
    op(Asl, DirectPage),
    op(Ora, DirectPageIndirectLong),
    op(Php, Implied),
    op(Ora, Immediate),
    op(Asl, Accumulator),
    op(Phd, Implied),
    op(Tsb, Absolute),
    op(Ora, Absolute),
    op(Asl, Absolute),
    op(Ora, AbsoluteLong),
    // $10-$1F
    op(Bpl, Relative8),
    op(Ora, DirectPageIndirectIndexedY),
    op(Ora, DirectPageIndirect),
    op(Ora, StackRelativeIndirectIndexedY),
    op(Trb, DirectPage),
    op(Ora, DirectPageX),
    op(Asl, DirectPageX),
    op(Ora, DirectPageIndirectLongIndexedY),
    op(Clc, Implied),
    op(Ora, AbsoluteY),
    op(Inc, Accumulator),
    op(Tcs, Implied),
    op(Trb, Absolute),
    op(Ora, AbsoluteX),
    op(Asl, AbsoluteX),
    op(Ora, AbsoluteLongX),
    // $20-$2F
    op(Jsr, Absolute),
    op(And, DirectPageIndexedIndirectX),
    op(Jsl, AbsoluteLong),
    op(And, StackRelative),
    op(Bit, DirectPage),
    op(And, DirectPage),
    op(Rol, DirectPage),
    op(And, DirectPageIndirectLong),
    op(Plp, Implied),
    op(And, Immediate),
    op(Rol, Accumulator),
    op(Pld, Implied),
    op(Bit, Absolute),
    op(And, Absolute),
    op(Rol, Absolute),
    op(And, AbsoluteLong),
    // $30-$3F
    op(Bmi, Relative8),
    op(And, DirectPageIndirectIndexedY),
    op(And, DirectPageIndirect),
    op(And, StackRelativeIndirectIndexedY),
    op(Bit, DirectPageX),
    op(And, DirectPageX),
    op(Rol, DirectPageX),
    op(And, DirectPageIndirectLongIndexedY),
    op(Sec, Implied),
    op(And, AbsoluteY),
    op(Dec, Accumulator),
    op(Tsc, Implied),
    op(Bit, AbsoluteX),
    op(And, AbsoluteX),
    op(Rol, AbsoluteX),
    op(And, AbsoluteLongX),
    // $40-$4F
    op(Rti, Implied),
    op(Eor, DirectPageIndexedIndirectX),
    op(Wdm, ImmediateByte),
    op(Eor, StackRelative),
    op(Mvp, BlockMove),
    op(Eor, DirectPage),
    op(Lsr, DirectPage),
    op(Eor, DirectPageIndirectLong),
    op(Pha, Implied),
    op(Eor, Immediate),
    op(Lsr, Accumulator),
    op(Phk, Implied),
    op(Jmp, Absolute),
    op(Eor, Absolute),
    op(Lsr, Absolute),
    op(Eor, AbsoluteLong),
    // $50-$5F
    op(Bvc, Relative8),
    op(Eor, DirectPageIndirectIndexedY),
    op(Eor, DirectPageIndirect),
    op(Eor, StackRelativeIndirectIndexedY),
    op(Mvn, BlockMove),
    op(Eor, DirectPageX),
    op(Lsr, DirectPageX),
    op(Eor, DirectPageIndirectLongIndexedY),
    op(Cli, Implied),
    op(Eor, AbsoluteY),
    op(Phy, Implied),
    op(Tcd, Implied),
    op(Jml, AbsoluteLong),
    op(Eor, AbsoluteX),
    op(Lsr, AbsoluteX),
    op(Eor, AbsoluteLongX),
    // $60-$6F
    op(Rts, Implied),
    op(Adc, DirectPageIndexedIndirectX),
    op(Per, Relative16),
    op(Adc, StackRelative),
    op(Stz, DirectPage),
    op(Adc, DirectPage),
    op(Ror, DirectPage),
    op(Adc, DirectPageIndirectLong),
    op(Pla, Implied),
    op(Adc, Immediate),
    op(Ror, Accumulator),
    op(Rtl, Implied),
    op(Jmp, AbsoluteIndirect),
    op(Adc, Absolute),
    op(Ror, Absolute),
    op(Adc, AbsoluteLong),
    // $70-$7F
    op(Bvs, Relative8),
    op(Adc, DirectPageIndirectIndexedY),
    op(Adc, DirectPageIndirect),
    op(Adc, StackRelativeIndirectIndexedY),
    op(Stz, DirectPageX),
    op(Adc, DirectPageX),
    op(Ror, DirectPageX),
    op(Adc, DirectPageIndirectLongIndexedY),
    op(Sei, Implied),
    op(Adc, AbsoluteY),
    op(Ply, Implied),
    op(Tdc, Implied),
    op(Jmp, AbsoluteIndexedIndirectX),
    op(Adc, AbsoluteX),
    op(Ror, AbsoluteX),
    op(Adc, AbsoluteLongX),
    // $80-$8F
    op(Bra, Relative8),
    op(Sta, DirectPageIndexedIndirectX),
    op(Brl, Relative16),
    op(Sta, StackRelative),
    op(Sty, DirectPage),
    op(Sta, DirectPage),
    op(Stx, DirectPage),
    op(Sta, DirectPageIndirectLong),
    op(Dey, Implied),
    op(Bit, Immediate),
    op(Txa, Implied),
    op(Phb, Implied),
    op(Sty, Absolute),
    op(Sta, Absolute),
    op(Stx, Absolute),
    op(Sta, AbsoluteLong),
    // $90-$9F
    op(Bcc, Relative8),
    op(Sta, DirectPageIndirectIndexedY),
    op(Sta, DirectPageIndirect),
    op(Sta, StackRelativeIndirectIndexedY),
    op(Sty, DirectPageX),
    op(Sta, DirectPageX),
    op(Stx, DirectPageY),
    op(Sta, DirectPageIndirectLongIndexedY),
    op(Tya, Implied),
    op(Sta, AbsoluteY),
    op(Txs, Implied),
    op(Txy, Implied),
    op(Stz, Absolute),
    op(Sta, AbsoluteX),
    op(Stz, AbsoluteX),
    op(Sta, AbsoluteLongX),
    // $A0-$AF
    op(Ldy, Immediate),
    op(Lda, DirectPageIndexedIndirectX),
    op(Ldx, Immediate),
    op(Lda, StackRelative),
    op(Ldy, DirectPage),
    op(Lda, DirectPage),
    op(Ldx, DirectPage),
    op(Lda, DirectPageIndirectLong),
    op(Tay, Implied),
    op(Lda, Immediate),
    op(Tax, Implied),
    op(Plb, Implied),
    op(Ldy, Absolute),
    op(Lda, Absolute),
    op(Ldx, Absolute),
    op(Lda, AbsoluteLong),
    // $B0-$BF
    op(Bcs, Relative8),
    op(Lda, DirectPageIndirectIndexedY),
    op(Lda, DirectPageIndirect),
    op(Lda, StackRelativeIndirectIndexedY),
    op(Ldy, DirectPageX),
    op(Lda, DirectPageX),
    op(Ldx, DirectPageY),
    op(Lda, DirectPageIndirectLongIndexedY),
    op(Clv, Implied),
    op(Lda, AbsoluteY),
    op(Tsx, Implied),
    op(Tyx, Implied),
    op(Ldy, AbsoluteX),
    op(Lda, AbsoluteX),
    op(Ldx, AbsoluteY),
    op(Lda, AbsoluteLongX),
    // $C0-$CF
    op(Cpy, Immediate),
    op(Cmp, DirectPageIndexedIndirectX),
    op(Rep, ImmediateByte),
    op(Cmp, StackRelative),
    op(Cpy, DirectPage),
    op(Cmp, DirectPage),
    op(Dec, DirectPage),
    op(Cmp, DirectPageIndirectLong),
    op(Iny, Implied),
    op(Cmp, Immediate),
    op(Dex, Implied),
    op(Wai, Implied),
    op(Cpy, Absolute),
    op(Cmp, Absolute),
    op(Dec, Absolute),
    op(Cmp, AbsoluteLong),
    // $D0-$DF
    op(Bne, Relative8),
    op(Cmp, DirectPageIndirectIndexedY),
    op(Cmp, DirectPageIndirect),
    op(Cmp, StackRelativeIndirectIndexedY),
    op(Pei, DirectPage),
    op(Cmp, DirectPageX),
    op(Dec, DirectPageX),
    op(Cmp, DirectPageIndirectLongIndexedY),
    op(Cld, Implied),
    op(Cmp, AbsoluteY),
    op(Phx, Implied),
    op(Stp, Implied),
    op(Jml, AbsoluteIndirectLong),
    op(Cmp, AbsoluteX),
    op(Dec, AbsoluteX),
    op(Cmp, AbsoluteLongX),
    // $E0-$EF
    op(Cpx, Immediate),
    op(Sbc, DirectPageIndexedIndirectX),
    op(Sep, ImmediateByte),
    op(Sbc, StackRelative),
    op(Cpx, DirectPage),
    op(Sbc, DirectPage),
    op(Inc, DirectPage),
    op(Sbc, DirectPageIndirectLong),
    op(Inx, Implied),
    op(Sbc, Immediate),
    op(Nop, Implied),
    op(Xba, Implied),
    op(Cpx, Absolute),
    op(Sbc, Absolute),
    op(Inc, Absolute),
    op(Sbc, AbsoluteLong),
    // $F0-$FF
    op(Beq, Relative8),
    op(Sbc, DirectPageIndirectIndexedY),
    op(Sbc, DirectPageIndirect),
    op(Sbc, StackRelativeIndirectIndexedY),
    op(Pea, Absolute),
    op(Sbc, DirectPageX),
    op(Inc, DirectPageX),
    op(Sbc, DirectPageIndirectLongIndexedY),
    op(Sed, Implied),
    op(Sbc, AbsoluteY),
    op(Plx, Implied),
    op(Xce, Implied),
    op(Jsr, AbsoluteIndexedIndirectX),
    op(Sbc, AbsoluteX),
    op(Inc, AbsoluteX),
    op(Sbc, AbsoluteLongX),
];

pub fn opcode_for(mnemonic: Mnemonic, mode: AddressingMode) -> Option<u8> {
    OPCODE_TABLE
        .iter()
        .position(|info| info.mnemonic == mnemonic && info.mode == mode)
        .map(|opcode| opcode as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_has_one_encoding_per_mnemonic_mode_pair() {
        assert_eq!(InstructionForm::all().count(), 256);
        for (left_opcode, left) in OPCODE_TABLE.iter().enumerate() {
            assert_eq!(
                opcode_for(left.mnemonic, left.mode),
                Some(left_opcode as u8)
            );
            for right in &OPCODE_TABLE[left_opcode + 1..] {
                assert_ne!(
                    (left.mnemonic, left.mode),
                    (right.mnemonic, right.mode),
                    "duplicate {:?} {:?}",
                    left.mnemonic,
                    left.mode
                );
            }
        }
    }

    #[test]
    fn width_dependent_immediates_use_the_right_register_class() {
        assert_eq!(OPCODE_TABLE[0xA9].operand_len(WidthState::M8_X16), 1);
        assert_eq!(OPCODE_TABLE[0xA9].operand_len(WidthState::M16_X8), 2);
        assert_eq!(OPCODE_TABLE[0xA2].operand_len(WidthState::M8_X16), 2);
        assert_eq!(OPCODE_TABLE[0xA2].operand_len(WidthState::M16_X8), 1);
    }

    #[test]
    fn jml_indirect_long_has_a_sixteen_bit_pointer_operand() {
        assert_eq!(OPCODE_TABLE[0xDC].mode, AbsoluteIndirectLong);
        assert_eq!(OPCODE_TABLE[0xDC].operand_len(WidthState::M8_X8), 2);
    }
}
