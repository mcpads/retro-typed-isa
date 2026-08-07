//! Independent W65C816 opcode coverage derived from the architecture source.
//!
//! Normative source: The Western Design Center, *W65C816S Datasheet*,
//! March 13, 2024, Table 5-4 "Opcode Matrix", datasheet page 31.
//! https://www.westerndesigncenter.com/wdc/documentation/w65c816s.pdf
//!
//! This fixture is deliberately not generated from `OPCODE_TABLE`,
//! `InstructionForm::all()`, or the production decoder.

use w65c816::{AddressingMode, InstructionForm, Mnemonic, WidthState, decode_bytes, encode_bytes};

const fn form(mnemonic: Mnemonic, mode: AddressingMode) -> InstructionForm {
    InstructionForm::new(mnemonic, mode)
}

use AddressingMode::*;
use Mnemonic::*;

const WDC_OPCODE_MATRIX: [InstructionForm; 256] = [
    // $00-$0F
    form(Brk, ImmediateByte),
    form(Ora, DirectPageIndexedIndirectX),
    form(Cop, ImmediateByte),
    form(Ora, StackRelative),
    form(Tsb, DirectPage),
    form(Ora, DirectPage),
    form(Asl, DirectPage),
    form(Ora, DirectPageIndirectLong),
    form(Php, Implied),
    form(Ora, Immediate),
    form(Asl, Accumulator),
    form(Phd, Implied),
    form(Tsb, Absolute),
    form(Ora, Absolute),
    form(Asl, Absolute),
    form(Ora, AbsoluteLong),
    // $10-$1F
    form(Bpl, Relative8),
    form(Ora, DirectPageIndirectIndexedY),
    form(Ora, DirectPageIndirect),
    form(Ora, StackRelativeIndirectIndexedY),
    form(Trb, DirectPage),
    form(Ora, DirectPageX),
    form(Asl, DirectPageX),
    form(Ora, DirectPageIndirectLongIndexedY),
    form(Clc, Implied),
    form(Ora, AbsoluteY),
    form(Inc, Accumulator),
    form(Tcs, Implied),
    form(Trb, Absolute),
    form(Ora, AbsoluteX),
    form(Asl, AbsoluteX),
    form(Ora, AbsoluteLongX),
    // $20-$2F
    form(Jsr, Absolute),
    form(And, DirectPageIndexedIndirectX),
    form(Jsl, AbsoluteLong),
    form(And, StackRelative),
    form(Bit, DirectPage),
    form(And, DirectPage),
    form(Rol, DirectPage),
    form(And, DirectPageIndirectLong),
    form(Plp, Implied),
    form(And, Immediate),
    form(Rol, Accumulator),
    form(Pld, Implied),
    form(Bit, Absolute),
    form(And, Absolute),
    form(Rol, Absolute),
    form(And, AbsoluteLong),
    // $30-$3F
    form(Bmi, Relative8),
    form(And, DirectPageIndirectIndexedY),
    form(And, DirectPageIndirect),
    form(And, StackRelativeIndirectIndexedY),
    form(Bit, DirectPageX),
    form(And, DirectPageX),
    form(Rol, DirectPageX),
    form(And, DirectPageIndirectLongIndexedY),
    form(Sec, Implied),
    form(And, AbsoluteY),
    form(Dec, Accumulator),
    form(Tsc, Implied),
    form(Bit, AbsoluteX),
    form(And, AbsoluteX),
    form(Rol, AbsoluteX),
    form(And, AbsoluteLongX),
    // $40-$4F
    form(Rti, Implied),
    form(Eor, DirectPageIndexedIndirectX),
    form(Wdm, ImmediateByte),
    form(Eor, StackRelative),
    form(Mvp, BlockMove),
    form(Eor, DirectPage),
    form(Lsr, DirectPage),
    form(Eor, DirectPageIndirectLong),
    form(Pha, Implied),
    form(Eor, Immediate),
    form(Lsr, Accumulator),
    form(Phk, Implied),
    form(Jmp, Absolute),
    form(Eor, Absolute),
    form(Lsr, Absolute),
    form(Eor, AbsoluteLong),
    // $50-$5F
    form(Bvc, Relative8),
    form(Eor, DirectPageIndirectIndexedY),
    form(Eor, DirectPageIndirect),
    form(Eor, StackRelativeIndirectIndexedY),
    form(Mvn, BlockMove),
    form(Eor, DirectPageX),
    form(Lsr, DirectPageX),
    form(Eor, DirectPageIndirectLongIndexedY),
    form(Cli, Implied),
    form(Eor, AbsoluteY),
    form(Phy, Implied),
    form(Tcd, Implied),
    form(Jml, AbsoluteLong),
    form(Eor, AbsoluteX),
    form(Lsr, AbsoluteX),
    form(Eor, AbsoluteLongX),
    // $60-$6F
    form(Rts, Implied),
    form(Adc, DirectPageIndexedIndirectX),
    form(Per, Relative16),
    form(Adc, StackRelative),
    form(Stz, DirectPage),
    form(Adc, DirectPage),
    form(Ror, DirectPage),
    form(Adc, DirectPageIndirectLong),
    form(Pla, Implied),
    form(Adc, Immediate),
    form(Ror, Accumulator),
    form(Rtl, Implied),
    form(Jmp, AbsoluteIndirect),
    form(Adc, Absolute),
    form(Ror, Absolute),
    form(Adc, AbsoluteLong),
    // $70-$7F
    form(Bvs, Relative8),
    form(Adc, DirectPageIndirectIndexedY),
    form(Adc, DirectPageIndirect),
    form(Adc, StackRelativeIndirectIndexedY),
    form(Stz, DirectPageX),
    form(Adc, DirectPageX),
    form(Ror, DirectPageX),
    form(Adc, DirectPageIndirectLongIndexedY),
    form(Sei, Implied),
    form(Adc, AbsoluteY),
    form(Ply, Implied),
    form(Tdc, Implied),
    form(Jmp, AbsoluteIndexedIndirectX),
    form(Adc, AbsoluteX),
    form(Ror, AbsoluteX),
    form(Adc, AbsoluteLongX),
    // $80-$8F
    form(Bra, Relative8),
    form(Sta, DirectPageIndexedIndirectX),
    form(Brl, Relative16),
    form(Sta, StackRelative),
    form(Sty, DirectPage),
    form(Sta, DirectPage),
    form(Stx, DirectPage),
    form(Sta, DirectPageIndirectLong),
    form(Dey, Implied),
    form(Bit, Immediate),
    form(Txa, Implied),
    form(Phb, Implied),
    form(Sty, Absolute),
    form(Sta, Absolute),
    form(Stx, Absolute),
    form(Sta, AbsoluteLong),
    // $90-$9F
    form(Bcc, Relative8),
    form(Sta, DirectPageIndirectIndexedY),
    form(Sta, DirectPageIndirect),
    form(Sta, StackRelativeIndirectIndexedY),
    form(Sty, DirectPageX),
    form(Sta, DirectPageX),
    form(Stx, DirectPageY),
    form(Sta, DirectPageIndirectLongIndexedY),
    form(Tya, Implied),
    form(Sta, AbsoluteY),
    form(Txs, Implied),
    form(Txy, Implied),
    form(Stz, Absolute),
    form(Sta, AbsoluteX),
    form(Stz, AbsoluteX),
    form(Sta, AbsoluteLongX),
    // $A0-$AF
    form(Ldy, Immediate),
    form(Lda, DirectPageIndexedIndirectX),
    form(Ldx, Immediate),
    form(Lda, StackRelative),
    form(Ldy, DirectPage),
    form(Lda, DirectPage),
    form(Ldx, DirectPage),
    form(Lda, DirectPageIndirectLong),
    form(Tay, Implied),
    form(Lda, Immediate),
    form(Tax, Implied),
    form(Plb, Implied),
    form(Ldy, Absolute),
    form(Lda, Absolute),
    form(Ldx, Absolute),
    form(Lda, AbsoluteLong),
    // $B0-$BF
    form(Bcs, Relative8),
    form(Lda, DirectPageIndirectIndexedY),
    form(Lda, DirectPageIndirect),
    form(Lda, StackRelativeIndirectIndexedY),
    form(Ldy, DirectPageX),
    form(Lda, DirectPageX),
    form(Ldx, DirectPageY),
    form(Lda, DirectPageIndirectLongIndexedY),
    form(Clv, Implied),
    form(Lda, AbsoluteY),
    form(Tsx, Implied),
    form(Tyx, Implied),
    form(Ldy, AbsoluteX),
    form(Lda, AbsoluteX),
    form(Ldx, AbsoluteY),
    form(Lda, AbsoluteLongX),
    // $C0-$CF
    form(Cpy, Immediate),
    form(Cmp, DirectPageIndexedIndirectX),
    form(Rep, ImmediateByte),
    form(Cmp, StackRelative),
    form(Cpy, DirectPage),
    form(Cmp, DirectPage),
    form(Dec, DirectPage),
    form(Cmp, DirectPageIndirectLong),
    form(Iny, Implied),
    form(Cmp, Immediate),
    form(Dex, Implied),
    form(Wai, Implied),
    form(Cpy, Absolute),
    form(Cmp, Absolute),
    form(Dec, Absolute),
    form(Cmp, AbsoluteLong),
    // $D0-$DF
    form(Bne, Relative8),
    form(Cmp, DirectPageIndirectIndexedY),
    form(Cmp, DirectPageIndirect),
    form(Cmp, StackRelativeIndirectIndexedY),
    form(Pei, DirectPage),
    form(Cmp, DirectPageX),
    form(Dec, DirectPageX),
    form(Cmp, DirectPageIndirectLongIndexedY),
    form(Cld, Implied),
    form(Cmp, AbsoluteY),
    form(Phx, Implied),
    form(Stp, Implied),
    form(Jml, AbsoluteIndirectLong),
    form(Cmp, AbsoluteX),
    form(Dec, AbsoluteX),
    form(Cmp, AbsoluteLongX),
    // $E0-$EF
    form(Cpx, Immediate),
    form(Sbc, DirectPageIndexedIndirectX),
    form(Sep, ImmediateByte),
    form(Sbc, StackRelative),
    form(Cpx, DirectPage),
    form(Sbc, DirectPage),
    form(Inc, DirectPage),
    form(Sbc, DirectPageIndirectLong),
    form(Inx, Implied),
    form(Sbc, Immediate),
    form(Nop, Implied),
    form(Xba, Implied),
    form(Cpx, Absolute),
    form(Sbc, Absolute),
    form(Inc, Absolute),
    form(Sbc, AbsoluteLong),
    // $F0-$FF
    form(Beq, Relative8),
    form(Sbc, DirectPageIndirectIndexedY),
    form(Sbc, DirectPageIndirect),
    form(Sbc, StackRelativeIndirectIndexedY),
    form(Pea, Absolute),
    form(Sbc, DirectPageX),
    form(Inc, DirectPageX),
    form(Sbc, DirectPageIndirectLongIndexedY),
    form(Sed, Implied),
    form(Sbc, AbsoluteY),
    form(Plx, Implied),
    form(Xce, Implied),
    form(Jsr, AbsoluteIndexedIndirectX),
    form(Sbc, AbsoluteX),
    form(Inc, AbsoluteX),
    form(Sbc, AbsoluteLongX),
];

// Program-sequence byte counts printed in Table 5-4 for M=1 and X=1.
const WDC_M8_X8_LENGTHS: [usize; 256] = [
    2, 2, 2, 2, 2, 2, 2, 2, 1, 2, 1, 1, 3, 3, 3, 4, // $00-$0F
    2, 2, 2, 2, 2, 2, 2, 2, 1, 3, 1, 1, 3, 3, 3, 4, // $10-$1F
    3, 2, 4, 2, 2, 2, 2, 2, 1, 2, 1, 1, 3, 3, 3, 4, // $20-$2F
    2, 2, 2, 2, 2, 2, 2, 2, 1, 3, 1, 1, 3, 3, 3, 4, // $30-$3F
    1, 2, 2, 2, 3, 2, 2, 2, 1, 2, 1, 1, 3, 3, 3, 4, // $40-$4F
    2, 2, 2, 2, 3, 2, 2, 2, 1, 3, 1, 1, 4, 3, 3, 4, // $50-$5F
    1, 2, 3, 2, 2, 2, 2, 2, 1, 2, 1, 1, 3, 3, 3, 4, // $60-$6F
    2, 2, 2, 2, 2, 2, 2, 2, 1, 3, 1, 1, 3, 3, 3, 4, // $70-$7F
    2, 2, 3, 2, 2, 2, 2, 2, 1, 2, 1, 1, 3, 3, 3, 4, // $80-$8F
    2, 2, 2, 2, 2, 2, 2, 2, 1, 3, 1, 1, 3, 3, 3, 4, // $90-$9F
    2, 2, 2, 2, 2, 2, 2, 2, 1, 2, 1, 1, 3, 3, 3, 4, // $A0-$AF
    2, 2, 2, 2, 2, 2, 2, 2, 1, 3, 1, 1, 3, 3, 3, 4, // $B0-$BF
    2, 2, 2, 2, 2, 2, 2, 2, 1, 2, 1, 1, 3, 3, 3, 4, // $C0-$CF
    2, 2, 2, 2, 2, 2, 2, 2, 1, 3, 1, 1, 3, 3, 3, 4, // $D0-$DF
    2, 2, 2, 2, 2, 2, 2, 2, 1, 2, 1, 1, 3, 3, 3, 4, // $E0-$EF
    2, 2, 2, 2, 3, 2, 2, 2, 1, 3, 1, 1, 3, 3, 3, 4, // $F0-$FF
];

#[test]
fn every_wdc_opcode_has_the_manual_form_length_and_exact_lowering() {
    for opcode in 0_u8..=u8::MAX {
        let expected_form = WDC_OPCODE_MATRIX[usize::from(opcode)];
        let expected_len = WDC_M8_X8_LENGTHS[usize::from(opcode)];
        let source = [opcode, 0x41, 0x42, 0x43];

        let decoded = decode_bytes(&source, WidthState::M8_X8)
            .unwrap_or_else(|error| panic!("WDC opcode ${opcode:02X} did not decode: {error}"));

        assert_eq!(
            decoded.instruction.form(),
            expected_form,
            "WDC opcode ${opcode:02X} has the wrong instruction form"
        );
        assert_eq!(
            decoded.len, expected_len,
            "WDC opcode ${opcode:02X} has the wrong M=1/X=1 length"
        );
        assert_eq!(
            encode_bytes(&decoded.instruction, WidthState::M8_X8).unwrap(),
            source[..expected_len],
            "WDC opcode ${opcode:02X} did not lower to its exact selector"
        );
    }
}

#[test]
fn manual_immediate_width_classes_follow_m_and_x() {
    // WDC section 6.3.3.3 requires separate accumulator and index width state.
    const ACCUMULATOR_IMMEDIATE_OPCODES: [u8; 7] = [0x09, 0x29, 0x49, 0x69, 0x89, 0xA9, 0xE9];
    const INDEX_IMMEDIATE_OPCODES: [u8; 4] = [0xA0, 0xA2, 0xC0, 0xE0];

    for opcode in ACCUMULATOR_IMMEDIATE_OPCODES {
        assert_eq!(
            decode_bytes(&[opcode, 1], WidthState::M8_X16).unwrap().len,
            2
        );
        assert_eq!(
            decode_bytes(&[opcode, 1, 2], WidthState::M16_X8)
                .unwrap()
                .len,
            3
        );
    }
    for opcode in INDEX_IMMEDIATE_OPCODES {
        assert_eq!(
            decode_bytes(&[opcode, 1], WidthState::M16_X8).unwrap().len,
            2
        );
        assert_eq!(
            decode_bytes(&[opcode, 1, 2], WidthState::M8_X16)
                .unwrap()
                .len,
            3
        );
    }
}
