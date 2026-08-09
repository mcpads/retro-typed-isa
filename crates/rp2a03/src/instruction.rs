//! Declarative RP2A03 instruction-selector profile.

use core::fmt;

/// Official and physically decoded RP2A03 instruction mnemonics.
///
/// Mnemonics absent from the original MOS programming model describe
/// unofficial selector behavior. They are modeled so existing Famicom code is
/// lossless, not as a recommendation to use unstable hardware behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mnemonic {
    /// Add memory and carry to the accumulator.
    Adc,
    /// Unofficial masked store of the accumulator/X intersection.
    Ahx,
    /// Unofficial immediate AND followed by accumulator logical shift right.
    Alr,
    /// Unofficial immediate AND that copies accumulator bit 7 to carry.
    Anc,
    /// Bitwise AND memory with the accumulator.
    And,
    /// Unofficial immediate AND followed by accumulator rotate right.
    Arr,
    /// Arithmetic shift left.
    Asl,
    /// Unofficial subtract immediate from the accumulator/X intersection into X.
    Axs,
    /// Branch if carry is clear.
    Bcc,
    /// Branch if carry is set.
    Bcs,
    /// Branch if equal, when the zero flag is set.
    Beq,
    /// Test memory bits against the accumulator and status flags.
    Bit,
    /// Branch if the negative flag is set.
    Bmi,
    /// Branch if not equal, when the zero flag is clear.
    Bne,
    /// Branch if the negative flag is clear.
    Bpl,
    /// Enter the maskable-interrupt vector through a software interrupt.
    Brk,
    /// Branch if overflow is clear.
    Bvc,
    /// Branch if overflow is set.
    Bvs,
    /// Clear the carry flag.
    Clc,
    /// Clear the decimal-mode flag; RP2A03 arithmetic itself has no decimal mode.
    Cld,
    /// Clear the interrupt-disable flag.
    Cli,
    /// Clear the overflow flag.
    Clv,
    /// Compare memory with the accumulator.
    Cmp,
    /// Compare memory with X.
    Cpx,
    /// Compare memory with Y.
    Cpy,
    /// Unofficial decrement memory followed by accumulator compare.
    Dcp,
    /// Decrement memory.
    Dec,
    /// Decrement X.
    Dex,
    /// Decrement Y.
    Dey,
    /// Bitwise exclusive OR memory with the accumulator.
    Eor,
    /// Increment memory.
    Inc,
    /// Increment X.
    Inx,
    /// Increment Y.
    Iny,
    /// Unofficial increment memory followed by subtract with carry.
    Isc,
    /// Jump to an absolute or indirect address.
    Jmp,
    /// Jump to a subroutine.
    Jsr,
    /// Unofficial selector that jams instruction fetch until reset.
    Kil,
    /// Unofficial load of accumulator, X, and stack pointer through the stack mask.
    Las,
    /// Unofficial load of both the accumulator and X.
    Lax,
    /// Load the accumulator.
    Lda,
    /// Load X.
    Ldx,
    /// Load Y.
    Ldy,
    /// Logical shift right.
    Lsr,
    /// Perform no architecturally visible operation.
    Nop,
    /// Bitwise OR memory with the accumulator.
    Ora,
    /// Push the accumulator onto the stack.
    Pha,
    /// Push processor status onto the stack.
    Php,
    /// Pull the accumulator from the stack.
    Pla,
    /// Pull processor status from the stack.
    Plp,
    /// Unofficial rotate memory left followed by accumulator AND.
    Rla,
    /// Rotate left through carry.
    Rol,
    /// Rotate right through carry.
    Ror,
    /// Unofficial rotate memory right followed by add with carry.
    Rra,
    /// Return from an interrupt.
    Rti,
    /// Return from a subroutine.
    Rts,
    /// Unofficial store of the accumulator/X intersection.
    Sax,
    /// Subtract memory and inverted carry from the accumulator.
    Sbc,
    /// Set the carry flag.
    Sec,
    /// Set the decimal-mode flag; RP2A03 arithmetic itself has no decimal mode.
    Sed,
    /// Set the interrupt-disable flag.
    Sei,
    /// Unofficial masked store derived from X and the destination high byte.
    Shx,
    /// Unofficial masked store derived from Y and the destination high byte.
    Shy,
    /// Unofficial arithmetic shift left followed by accumulator OR.
    Slo,
    /// Unofficial logical shift right followed by accumulator exclusive OR.
    Sre,
    /// Store the accumulator.
    Sta,
    /// Store X.
    Stx,
    /// Store Y.
    Sty,
    /// Unofficial transfer of accumulator/X intersection to the stack pointer and memory.
    Tas,
    /// Transfer the accumulator to X.
    Tax,
    /// Transfer the accumulator to Y.
    Tay,
    /// Transfer the stack pointer to X.
    Tsx,
    /// Transfer X to the accumulator.
    Txa,
    /// Transfer X to the stack pointer.
    Txs,
    /// Transfer Y to the accumulator.
    Tya,
    /// Unofficial, hardware-unstable immediate operation involving A and X.
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
    /// Operand is implicit in the instruction identity.
    Implied,
    /// Operation targets the accumulator without an encoded operand byte.
    Accumulator,
    /// One literal byte immediately follows the opcode.
    Immediate,
    /// One byte selects an address in zero page.
    ZeroPage,
    /// One zero-page base byte is indexed by X.
    ZeroPageX,
    /// One zero-page base byte is indexed by Y.
    ZeroPageY,
    /// One signed byte is relative to the post-instruction program counter.
    Relative,
    /// One little-endian word supplies an absolute address.
    Absolute,
    /// One absolute base address is indexed by X.
    AbsoluteX,
    /// One absolute base address is indexed by Y.
    AbsoluteY,
    /// One absolute address names the pointer used by indirect `JMP`.
    AbsoluteIndirect,
    /// One zero-page pointer base is first indexed by X, then dereferenced.
    ZeroPageIndexedIndirectX,
    /// One zero-page pointer is dereferenced, then indexed by Y.
    ZeroPageIndirectIndexedY,
}

impl AddressingMode {
    /// Return the number of bytes encoded after the opcode for this mode.
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

    /// Complete, duplicate-free semantic-form denominator for this profile.
    pub fn all() -> impl Iterator<Item = Self> {
        (0..OPCODE_TABLE.len()).filter_map(|index| {
            let form = OPCODE_TABLE[index];
            (!OPCODE_TABLE[..index].contains(&form)).then_some(form)
        })
    }

    /// Return the complete instruction length, including the opcode byte.
    pub const fn encoded_len(self) -> usize {
        1 + self.mode.operand_len()
    }
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
