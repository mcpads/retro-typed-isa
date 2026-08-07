//! Independent RP2A03 selector coverage derived from architecture sources.
//!
//! Documented selectors come from MOS Technology's *MCS6500 Microcomputer
//! Family Programming Manual* (1976), Appendix D, "Operation Code Instruction
//! Listing Hexadecimal Sequence". The remaining physical NMOS selectors and
//! their addressing modes come from NESdev's hardware-research matrix,
//! revision 23975:
//! https://www.nesdev.org/w/index.php?title=CPU_unofficial_opcodes&oldid=23975
//!
//! NESdev calls the halt family `STP`; this profile uses the established `KIL`
//! spelling so it cannot be confused with a documented later-CPU instruction.
//! The fixture is not generated from `OPCODE_TABLE`, `InstructionForm::all()`,
//! `opcode_is_documented`, or the production decoder.

use rp2a03::{decode_bytes, encode_bytes};

// D/U records whether MOS documents the selector. Each row is one hexadecimal
// row of the architecture matrix, in low-nibble order.
const SOURCE_ROWS: [&str; 16] = [
    "D:Brk:Implied D:Ora:ZeroPageIndexedIndirectX U:Kil:Implied U:Slo:ZeroPageIndexedIndirectX U:Nop:ZeroPage D:Ora:ZeroPage D:Asl:ZeroPage U:Slo:ZeroPage D:Php:Implied D:Ora:Immediate D:Asl:Accumulator U:Anc:Immediate U:Nop:Absolute D:Ora:Absolute D:Asl:Absolute U:Slo:Absolute",
    "D:Bpl:Relative D:Ora:ZeroPageIndirectIndexedY U:Kil:Implied U:Slo:ZeroPageIndirectIndexedY U:Nop:ZeroPageX D:Ora:ZeroPageX D:Asl:ZeroPageX U:Slo:ZeroPageX D:Clc:Implied D:Ora:AbsoluteY U:Nop:Implied U:Slo:AbsoluteY U:Nop:AbsoluteX D:Ora:AbsoluteX D:Asl:AbsoluteX U:Slo:AbsoluteX",
    "D:Jsr:Absolute D:And:ZeroPageIndexedIndirectX U:Kil:Implied U:Rla:ZeroPageIndexedIndirectX D:Bit:ZeroPage D:And:ZeroPage D:Rol:ZeroPage U:Rla:ZeroPage D:Plp:Implied D:And:Immediate D:Rol:Accumulator U:Anc:Immediate D:Bit:Absolute D:And:Absolute D:Rol:Absolute U:Rla:Absolute",
    "D:Bmi:Relative D:And:ZeroPageIndirectIndexedY U:Kil:Implied U:Rla:ZeroPageIndirectIndexedY U:Nop:ZeroPageX D:And:ZeroPageX D:Rol:ZeroPageX U:Rla:ZeroPageX D:Sec:Implied D:And:AbsoluteY U:Nop:Implied U:Rla:AbsoluteY U:Nop:AbsoluteX D:And:AbsoluteX D:Rol:AbsoluteX U:Rla:AbsoluteX",
    "D:Rti:Implied D:Eor:ZeroPageIndexedIndirectX U:Kil:Implied U:Sre:ZeroPageIndexedIndirectX U:Nop:ZeroPage D:Eor:ZeroPage D:Lsr:ZeroPage U:Sre:ZeroPage D:Pha:Implied D:Eor:Immediate D:Lsr:Accumulator U:Alr:Immediate D:Jmp:Absolute D:Eor:Absolute D:Lsr:Absolute U:Sre:Absolute",
    "D:Bvc:Relative D:Eor:ZeroPageIndirectIndexedY U:Kil:Implied U:Sre:ZeroPageIndirectIndexedY U:Nop:ZeroPageX D:Eor:ZeroPageX D:Lsr:ZeroPageX U:Sre:ZeroPageX D:Cli:Implied D:Eor:AbsoluteY U:Nop:Implied U:Sre:AbsoluteY U:Nop:AbsoluteX D:Eor:AbsoluteX D:Lsr:AbsoluteX U:Sre:AbsoluteX",
    "D:Rts:Implied D:Adc:ZeroPageIndexedIndirectX U:Kil:Implied U:Rra:ZeroPageIndexedIndirectX U:Nop:ZeroPage D:Adc:ZeroPage D:Ror:ZeroPage U:Rra:ZeroPage D:Pla:Implied D:Adc:Immediate D:Ror:Accumulator U:Arr:Immediate D:Jmp:AbsoluteIndirect D:Adc:Absolute D:Ror:Absolute U:Rra:Absolute",
    "D:Bvs:Relative D:Adc:ZeroPageIndirectIndexedY U:Kil:Implied U:Rra:ZeroPageIndirectIndexedY U:Nop:ZeroPageX D:Adc:ZeroPageX D:Ror:ZeroPageX U:Rra:ZeroPageX D:Sei:Implied D:Adc:AbsoluteY U:Nop:Implied U:Rra:AbsoluteY U:Nop:AbsoluteX D:Adc:AbsoluteX D:Ror:AbsoluteX U:Rra:AbsoluteX",
    "U:Nop:Immediate D:Sta:ZeroPageIndexedIndirectX U:Nop:Immediate U:Sax:ZeroPageIndexedIndirectX D:Sty:ZeroPage D:Sta:ZeroPage D:Stx:ZeroPage U:Sax:ZeroPage D:Dey:Implied U:Nop:Immediate D:Txa:Implied U:Xaa:Immediate D:Sty:Absolute D:Sta:Absolute D:Stx:Absolute U:Sax:Absolute",
    "D:Bcc:Relative D:Sta:ZeroPageIndirectIndexedY U:Kil:Implied U:Ahx:ZeroPageIndirectIndexedY D:Sty:ZeroPageX D:Sta:ZeroPageX D:Stx:ZeroPageY U:Sax:ZeroPageY D:Tya:Implied D:Sta:AbsoluteY D:Txs:Implied U:Tas:AbsoluteY U:Shy:AbsoluteX D:Sta:AbsoluteX U:Shx:AbsoluteY U:Ahx:AbsoluteY",
    "D:Ldy:Immediate D:Lda:ZeroPageIndexedIndirectX D:Ldx:Immediate U:Lax:ZeroPageIndexedIndirectX D:Ldy:ZeroPage D:Lda:ZeroPage D:Ldx:ZeroPage U:Lax:ZeroPage D:Tay:Implied D:Lda:Immediate D:Tax:Implied U:Lax:Immediate D:Ldy:Absolute D:Lda:Absolute D:Ldx:Absolute U:Lax:Absolute",
    "D:Bcs:Relative D:Lda:ZeroPageIndirectIndexedY U:Kil:Implied U:Lax:ZeroPageIndirectIndexedY D:Ldy:ZeroPageX D:Lda:ZeroPageX D:Ldx:ZeroPageY U:Lax:ZeroPageY D:Clv:Implied D:Lda:AbsoluteY D:Tsx:Implied U:Las:AbsoluteY D:Ldy:AbsoluteX D:Lda:AbsoluteX D:Ldx:AbsoluteY U:Lax:AbsoluteY",
    "D:Cpy:Immediate D:Cmp:ZeroPageIndexedIndirectX U:Nop:Immediate U:Dcp:ZeroPageIndexedIndirectX D:Cpy:ZeroPage D:Cmp:ZeroPage D:Dec:ZeroPage U:Dcp:ZeroPage D:Iny:Implied D:Cmp:Immediate D:Dex:Implied U:Axs:Immediate D:Cpy:Absolute D:Cmp:Absolute D:Dec:Absolute U:Dcp:Absolute",
    "D:Bne:Relative D:Cmp:ZeroPageIndirectIndexedY U:Kil:Implied U:Dcp:ZeroPageIndirectIndexedY U:Nop:ZeroPageX D:Cmp:ZeroPageX D:Dec:ZeroPageX U:Dcp:ZeroPageX D:Cld:Implied D:Cmp:AbsoluteY U:Nop:Implied U:Dcp:AbsoluteY U:Nop:AbsoluteX D:Cmp:AbsoluteX D:Dec:AbsoluteX U:Dcp:AbsoluteX",
    "D:Cpx:Immediate D:Sbc:ZeroPageIndexedIndirectX U:Nop:Immediate U:Isc:ZeroPageIndexedIndirectX D:Cpx:ZeroPage D:Sbc:ZeroPage D:Inc:ZeroPage U:Isc:ZeroPage D:Inx:Implied D:Sbc:Immediate D:Nop:Implied U:Sbc:Immediate D:Cpx:Absolute D:Sbc:Absolute D:Inc:Absolute U:Isc:Absolute",
    "D:Beq:Relative D:Sbc:ZeroPageIndirectIndexedY U:Kil:Implied U:Isc:ZeroPageIndirectIndexedY U:Nop:ZeroPageX D:Sbc:ZeroPageX D:Inc:ZeroPageX U:Isc:ZeroPageX D:Sed:Implied D:Sbc:AbsoluteY U:Nop:Implied U:Isc:AbsoluteY U:Nop:AbsoluteX D:Sbc:AbsoluteX D:Inc:AbsoluteX U:Isc:AbsoluteX",
];

#[test]
fn every_source_selector_has_the_expected_form_status_and_exact_lowering() {
    for (high_nibble, row) in SOURCE_ROWS.iter().enumerate() {
        let entries: Vec<_> = row.split_ascii_whitespace().collect();
        assert_eq!(entries.len(), 16, "source row {high_nibble:X}");

        for (low_nibble, entry) in entries.into_iter().enumerate() {
            let opcode = ((high_nibble << 4) | low_nibble) as u8;
            let (status, expected_form) = entry.split_once(':').unwrap();
            let source = [opcode, 0x41, 0x42];
            let decoded = decode_bytes(&source)
                .unwrap_or_else(|error| panic!("source opcode ${opcode:02X}: {error}"));
            let actual_form = format!("{:?}:{:?}", decoded.mnemonic(), decoded.addressing_mode());

            assert_eq!(
                actual_form, expected_form,
                "source opcode ${opcode:02X} has the wrong form"
            );
            assert_eq!(
                decoded.opcode_is_documented(),
                status == "D",
                "source opcode ${opcode:02X} has the wrong documentation status"
            );

            let expected_len = source_form_len(expected_form);
            assert_eq!(
                decoded.encoded_len(),
                expected_len,
                "source opcode ${opcode:02X} has the wrong length"
            );
            assert_eq!(
                encode_bytes(&decoded).unwrap(),
                source[..expected_len],
                "source opcode ${opcode:02X} did not lower to its exact selector"
            );
        }
    }
}

fn source_form_len(form: &str) -> usize {
    let mode = form.rsplit_once(':').unwrap().1;
    match mode {
        "Implied" | "Accumulator" => 1,
        "Immediate"
        | "ZeroPage"
        | "ZeroPageX"
        | "ZeroPageY"
        | "Relative"
        | "ZeroPageIndexedIndirectX"
        | "ZeroPageIndirectIndexedY" => 2,
        "Absolute" | "AbsoluteX" | "AbsoluteY" | "AbsoluteIndirect" => 3,
        _ => panic!("unknown source addressing mode {mode}"),
    }
}
