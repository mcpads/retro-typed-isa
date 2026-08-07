//! Declarative W65C816 opcode profile.

use core::fmt;

/// W65C816 instruction mnemonic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mnemonic {
    /// Add memory and carry to the accumulator.
    Adc,
    /// Bitwise AND memory with the accumulator.
    And,
    /// Arithmetic shift left.
    Asl,
    /// Branch if carry is clear.
    Bcc,
    /// Branch if carry is set.
    Bcs,
    /// Branch if the zero flag is set.
    Beq,
    /// Test memory bits against the accumulator and status flags.
    Bit,
    /// Branch if the negative flag is set.
    Bmi,
    /// Branch if the zero flag is clear.
    Bne,
    /// Branch if the negative flag is clear.
    Bpl,
    /// Branch unconditionally with an 8-bit displacement.
    Bra,
    /// Enter the maskable-interrupt vector through a software interrupt.
    Brk,
    /// Branch unconditionally with a 16-bit displacement.
    Brl,
    /// Branch if overflow is clear.
    Bvc,
    /// Branch if overflow is set.
    Bvs,
    /// Clear the carry flag.
    Clc,
    /// Clear the decimal-mode flag.
    Cld,
    /// Clear the interrupt-disable flag.
    Cli,
    /// Clear the overflow flag.
    Clv,
    /// Compare memory with the accumulator.
    Cmp,
    /// Enter the native COP software-interrupt vector.
    Cop,
    /// Compare memory with X.
    Cpx,
    /// Compare memory with Y.
    Cpy,
    /// Decrement memory or the accumulator.
    Dec,
    /// Decrement X.
    Dex,
    /// Decrement Y.
    Dey,
    /// Bitwise exclusive OR memory with the accumulator.
    Eor,
    /// Increment memory or the accumulator.
    Inc,
    /// Increment X.
    Inx,
    /// Increment Y.
    Iny,
    /// Jump long to a 24-bit address or through a long pointer.
    Jml,
    /// Jump within the current program bank or through a pointer.
    Jmp,
    /// Jump long to a subroutine.
    Jsl,
    /// Jump to a subroutine within the current program bank.
    Jsr,
    /// Load the accumulator.
    Lda,
    /// Load X.
    Ldx,
    /// Load Y.
    Ldy,
    /// Logical shift right.
    Lsr,
    /// Move a block while incrementing source and destination offsets.
    Mvn,
    /// Move a block while decrementing source and destination offsets.
    Mvp,
    /// Perform no operation.
    Nop,
    /// Bitwise OR memory with the accumulator.
    Ora,
    /// Push an immediate absolute address.
    Pea,
    /// Push the word addressed indirectly through direct page.
    Pei,
    /// Push a PC-relative effective address.
    Per,
    /// Push the accumulator.
    Pha,
    /// Push the data-bank register.
    Phb,
    /// Push the direct-page register.
    Phd,
    /// Push the program-bank register.
    Phk,
    /// Push processor status.
    Php,
    /// Push X.
    Phx,
    /// Push Y.
    Phy,
    /// Pull the accumulator.
    Pla,
    /// Pull the data-bank register.
    Plb,
    /// Pull the direct-page register.
    Pld,
    /// Pull processor status.
    Plp,
    /// Pull X.
    Plx,
    /// Pull Y.
    Ply,
    /// Reset selected processor-status bits.
    Rep,
    /// Rotate left through carry.
    Rol,
    /// Rotate right through carry.
    Ror,
    /// Return from an interrupt.
    Rti,
    /// Return from a long subroutine call.
    Rtl,
    /// Return from a same-bank subroutine call.
    Rts,
    /// Subtract memory and inverted carry from the accumulator.
    Sbc,
    /// Set the carry flag.
    Sec,
    /// Set the decimal-mode flag.
    Sed,
    /// Set the interrupt-disable flag.
    Sei,
    /// Set selected processor-status bits.
    Sep,
    /// Store the accumulator.
    Sta,
    /// Stop the clock until reset.
    Stp,
    /// Store X.
    Stx,
    /// Store Y.
    Sty,
    /// Store zero.
    Stz,
    /// Transfer the accumulator to X.
    Tax,
    /// Transfer the accumulator to Y.
    Tay,
    /// Transfer the 16-bit accumulator to the direct-page register.
    Tcd,
    /// Transfer the 16-bit accumulator to the stack pointer.
    Tcs,
    /// Transfer the direct-page register to the 16-bit accumulator.
    Tdc,
    /// Test memory bits and reset those selected by the accumulator.
    Trb,
    /// Test memory bits and set those selected by the accumulator.
    Tsb,
    /// Transfer the stack pointer to the 16-bit accumulator.
    Tsc,
    /// Transfer the stack pointer to X.
    Tsx,
    /// Transfer X to the accumulator.
    Txa,
    /// Transfer X to the stack pointer.
    Txs,
    /// Transfer X to Y.
    Txy,
    /// Transfer Y to the accumulator.
    Tya,
    /// Transfer Y to X.
    Tyx,
    /// Wait for an interrupt.
    Wai,
    /// Reserved two-byte instruction for system-defined use.
    Wdm,
    /// Exchange the accumulator's high and low bytes.
    Xba,
    /// Exchange carry with the emulation-mode flag.
    Xce,
}

impl fmt::Display for Mnemonic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self).map(|_| ())
    }
}

/// Operand interpretation selected by an opcode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AddressingMode {
    /// Operand is implicit in the instruction identity.
    Implied,
    /// Operation targets the accumulator without encoded operand bytes.
    Accumulator,
    /// Immediate width follows accumulator or index width state.
    Immediate,
    /// One fixed-width immediate byte follows the opcode.
    ImmediateByte,
    /// One byte selects an address relative to the direct-page register.
    DirectPage,
    /// Direct-page address indexed by X.
    DirectPageX,
    /// Direct-page address indexed by Y.
    DirectPageY,
    /// Direct-page address names a 16-bit pointer.
    DirectPageIndirect,
    /// Direct-page address names a 24-bit long pointer.
    DirectPageIndirectLong,
    /// Direct-page pointer base indexed by X before dereferencing.
    DirectPageIndexedIndirectX,
    /// Direct-page pointer dereferenced before indexing by Y.
    DirectPageIndirectIndexedY,
    /// Direct-page long pointer dereferenced before indexing by Y.
    DirectPageIndirectLongIndexedY,
    /// One little-endian word supplies an absolute address.
    Absolute,
    /// Absolute address indexed by X.
    AbsoluteX,
    /// Absolute address indexed by Y.
    AbsoluteY,
    /// Three-byte address supplies an absolute long target.
    AbsoluteLong,
    /// Absolute long address indexed by X.
    AbsoluteLongX,
    /// Absolute address names a 16-bit indirect pointer.
    AbsoluteIndirect,
    /// Absolute pointer-table base indexed by X before dereferencing.
    AbsoluteIndexedIndirectX,
    /// Absolute address names a 24-bit indirect pointer.
    AbsoluteIndirectLong,
    /// Signed 8-bit displacement from the post-instruction PC.
    Relative8,
    /// Signed 16-bit displacement from the post-instruction PC.
    Relative16,
    /// Stack-relative address selected by one unsigned byte.
    StackRelative,
    /// Stack-relative pointer dereferenced before indexing by Y.
    StackRelativeIndirectIndexedY,
    /// Source-bank and destination-bank bytes for block movement.
    BlockMove,
}

/// Effective width of an accumulator or index immediate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Width {
    /// Eight-bit register/immediate width.
    Eight,
    /// Sixteen-bit register/immediate width.
    Sixteen,
}

/// Accumulator and index widths required to decode width-dependent immediates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidthState {
    /// Effective accumulator width controlled by the M status bit.
    pub accumulator: Width,
    /// Effective X/Y index width controlled by the X status bit.
    pub index: Width,
}

impl WidthState {
    /// Eight-bit accumulator and eight-bit index registers.
    pub const M8_X8: Self = Self::new(Width::Eight, Width::Eight);
    /// Eight-bit accumulator and sixteen-bit index registers.
    pub const M8_X16: Self = Self::new(Width::Eight, Width::Sixteen);
    /// Sixteen-bit accumulator and eight-bit index registers.
    pub const M16_X8: Self = Self::new(Width::Sixteen, Width::Eight);
    /// Sixteen-bit accumulator and sixteen-bit index registers.
    pub const M16_X16: Self = Self::new(Width::Sixteen, Width::Sixteen);

    /// Construct an explicit native-mode width state.
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

/// Semantic instruction form with operand values removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InstructionForm {
    /// Operation selected by the form.
    pub mnemonic: Mnemonic,
    /// Operand interpretation selected by the form.
    pub mode: AddressingMode,
}

impl InstructionForm {
    /// Construct a semantic form from its mnemonic and addressing mode.
    pub const fn new(mnemonic: Mnemonic, mode: AddressingMode) -> Self {
        Self { mnemonic, mode }
    }

    /// Complete semantic-form denominator for profile-level checks.
    pub fn all() -> impl Iterator<Item = Self> {
        OPCODE_TABLE.into_iter()
    }

    /// Return the operand byte length under an explicit width state.
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

/// Return the unique opcode assigned to a mnemonic/addressing-mode pair.
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
