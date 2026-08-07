//! Independent HuC6280 coverage derived from the processor software manual.
//!
//! Normative source: Hudson Soft, *HuC6280 CMOS 8-bit Microprocessor Software
//! Manual*, Tables 2-1-1, 2-2-1 through 2-2-3, and the instruction encoding
//! tables on pages S8-17 through S8-108. A preserved scan is available at:
//! https://github.com/MiSTer-devel/TurboGrafx16_MiSTer/blob/master/docs/HuC6280%20-%20CMOS%208-bit%20Microprocessor%20Software%20Manual.pdf
//!
//! The manual defines 234 selector slots and leaves 22 slots undefined. Those
//! undefined slots are expected to fail closed.
//!
//! This fixture is not generated from the production selector table, form
//! catalog, documentation predicate, or decoder.

use huc6280::{DecodeError, decode_bytes, encode_bytes};

// `U` is an undefined manual slot. `D:MNEMONIC:MODE` is a documented selector.
const MANUAL_ROWS: [&str; 16] = [
    "D:BRK:Implied D:ORA:ZeroPageIndexedIndirectX D:SXY:Implied D:ST0:Immediate D:TSB:ZeroPage D:ORA:ZeroPage D:ASL:ZeroPage D:RMB0:ZeroPage D:PHP:Implied D:ORA:Immediate D:ASL:Accumulator U D:TSB:Absolute D:ORA:Absolute D:ASL:Absolute D:BBR0:ZeroPageRelative",
    "D:BPL:Relative D:ORA:ZeroPageIndirectIndexedY D:ORA:ZeroPageIndirect D:ST1:Immediate D:TRB:ZeroPage D:ORA:ZeroPageX D:ASL:ZeroPageX D:RMB1:ZeroPage D:CLC:Implied D:ORA:AbsoluteY D:INC:Accumulator U D:TRB:Absolute D:ORA:AbsoluteX D:ASL:AbsoluteX D:BBR1:ZeroPageRelative",
    "D:JSR:Absolute D:AND:ZeroPageIndexedIndirectX D:SAX:Implied D:ST2:Immediate D:BIT:ZeroPage D:AND:ZeroPage D:ROL:ZeroPage D:RMB2:ZeroPage D:PLP:Implied D:AND:Immediate D:ROL:Accumulator U D:BIT:Absolute D:AND:Absolute D:ROL:Absolute D:BBR2:ZeroPageRelative",
    "D:BMI:Relative D:AND:ZeroPageIndirectIndexedY D:AND:ZeroPageIndirect U D:BIT:ZeroPageX D:AND:ZeroPageX D:ROL:ZeroPageX D:RMB3:ZeroPage D:SEC:Implied D:AND:AbsoluteY D:DEC:Accumulator U D:BIT:AbsoluteX D:AND:AbsoluteX D:ROL:AbsoluteX D:BBR3:ZeroPageRelative",
    "D:RTI:Implied D:EOR:ZeroPageIndexedIndirectX D:SAY:Implied D:TMA:Immediate D:BSR:Relative D:EOR:ZeroPage D:LSR:ZeroPage D:RMB4:ZeroPage D:PHA:Implied D:EOR:Immediate D:LSR:Accumulator U D:JMP:Absolute D:EOR:Absolute D:LSR:Absolute D:BBR4:ZeroPageRelative",
    "D:BVC:Relative D:EOR:ZeroPageIndirectIndexedY D:EOR:ZeroPageIndirect D:TAM:Immediate D:CSL:Implied D:EOR:ZeroPageX D:LSR:ZeroPageX D:RMB5:ZeroPage D:CLI:Implied D:EOR:AbsoluteY D:PHY:Implied U U D:EOR:AbsoluteX D:LSR:AbsoluteX D:BBR5:ZeroPageRelative",
    "D:RTS:Implied D:ADC:ZeroPageIndexedIndirectX D:CLA:Implied U D:STZ:ZeroPage D:ADC:ZeroPage D:ROR:ZeroPage D:RMB6:ZeroPage D:PLA:Implied D:ADC:Immediate D:ROR:Accumulator U D:JMP:AbsoluteIndirect D:ADC:Absolute D:ROR:Absolute D:BBR6:ZeroPageRelative",
    "D:BVS:Relative D:ADC:ZeroPageIndirectIndexedY D:ADC:ZeroPageIndirect D:TII:BlockTransfer D:STZ:ZeroPageX D:ADC:ZeroPageX D:ROR:ZeroPageX D:RMB7:ZeroPage D:SEI:Implied D:ADC:AbsoluteY D:PLY:Implied U D:JMP:AbsoluteIndexedIndirectX D:ADC:AbsoluteX D:ROR:AbsoluteX D:BBR7:ZeroPageRelative",
    "D:BRA:Relative D:STA:ZeroPageIndexedIndirectX D:CLX:Implied D:TST:ImmediateZeroPage D:STY:ZeroPage D:STA:ZeroPage D:STX:ZeroPage D:SMB0:ZeroPage D:DEY:Implied D:BIT:Immediate D:TXA:Implied U D:STY:Absolute D:STA:Absolute D:STX:Absolute D:BBS0:ZeroPageRelative",
    "D:BCC:Relative D:STA:ZeroPageIndirectIndexedY D:STA:ZeroPageIndirect D:TST:ImmediateAbsolute D:STY:ZeroPageX D:STA:ZeroPageX D:STX:ZeroPageY D:SMB1:ZeroPage D:TYA:Implied D:STA:AbsoluteY D:TXS:Implied U D:STZ:Absolute D:STA:AbsoluteX D:STZ:AbsoluteX D:BBS1:ZeroPageRelative",
    "D:LDY:Immediate D:LDA:ZeroPageIndexedIndirectX D:LDX:Immediate D:TST:ImmediateZeroPageX D:LDY:ZeroPage D:LDA:ZeroPage D:LDX:ZeroPage D:SMB2:ZeroPage D:TAY:Implied D:LDA:Immediate D:TAX:Implied U D:LDY:Absolute D:LDA:Absolute D:LDX:Absolute D:BBS2:ZeroPageRelative",
    "D:BCS:Relative D:LDA:ZeroPageIndirectIndexedY D:LDA:ZeroPageIndirect D:TST:ImmediateAbsoluteX D:LDY:ZeroPageX D:LDA:ZeroPageX D:LDX:ZeroPageY D:SMB3:ZeroPage D:CLV:Implied D:LDA:AbsoluteY D:TSX:Implied U D:LDY:AbsoluteX D:LDA:AbsoluteX D:LDX:AbsoluteY D:BBS3:ZeroPageRelative",
    "D:CPY:Immediate D:CMP:ZeroPageIndexedIndirectX D:CLY:Implied D:TDD:BlockTransfer D:CPY:ZeroPage D:CMP:ZeroPage D:DEC:ZeroPage D:SMB4:ZeroPage D:INY:Implied D:CMP:Immediate D:DEX:Implied U D:CPY:Absolute D:CMP:Absolute D:DEC:Absolute D:BBS4:ZeroPageRelative",
    "D:BNE:Relative D:CMP:ZeroPageIndirectIndexedY D:CMP:ZeroPageIndirect D:TIN:BlockTransfer D:CSH:Implied D:CMP:ZeroPageX D:DEC:ZeroPageX D:SMB5:ZeroPage D:CLD:Implied D:CMP:AbsoluteY D:PHX:Implied U U D:CMP:AbsoluteX D:DEC:AbsoluteX D:BBS5:ZeroPageRelative",
    "D:CPX:Immediate D:SBC:ZeroPageIndexedIndirectX U D:TIA:BlockTransfer D:CPX:ZeroPage D:SBC:ZeroPage D:INC:ZeroPage D:SMB6:ZeroPage D:INX:Implied D:SBC:Immediate D:NOP:Implied U D:CPX:Absolute D:SBC:Absolute D:INC:Absolute D:BBS6:ZeroPageRelative",
    "D:BEQ:Relative D:SBC:ZeroPageIndirectIndexedY D:SBC:ZeroPageIndirect D:TAI:BlockTransfer D:SET:Implied D:SBC:ZeroPageX D:INC:ZeroPageX D:SMB7:ZeroPage D:SED:Implied D:SBC:AbsoluteY D:PLX:Implied U U D:SBC:AbsoluteX D:INC:AbsoluteX D:BBS7:ZeroPageRelative",
];

#[test]
fn every_manual_selector_has_the_documented_form_and_exact_lowering() {
    for (high_nibble, row) in MANUAL_ROWS.iter().enumerate() {
        let entries: Vec<_> = row.split_ascii_whitespace().collect();
        assert_eq!(entries.len(), 16, "manual row {high_nibble:X}");

        for (low_nibble, entry) in entries.into_iter().enumerate() {
            let opcode = ((high_nibble << 4) | low_nibble) as u8;
            let source = [opcode, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC];

            if entry == "U" {
                assert_eq!(
                    decode_bytes(&source),
                    Err(DecodeError::UndocumentedOpcode { opcode }),
                    "manual-undefined selector ${opcode:02X} must fail closed"
                );
                continue;
            }

            let expected_form = entry.strip_prefix("D:").unwrap();
            let decoded = decode_bytes(&source)
                .unwrap_or_else(|error| panic!("manual opcode ${opcode:02X}: {error}"));
            let actual_form = format!("{}:{:?}", decoded.mnemonic(), decoded.addressing_mode());
            assert_eq!(
                actual_form, expected_form,
                "manual opcode ${opcode:02X} has the wrong form"
            );
            assert!(decoded.opcode_is_documented());

            let expected_len = manual_form_len(expected_form);
            assert_eq!(decoded.encoded_len(), expected_len);
            assert_eq!(
                encode_bytes(&decoded).unwrap(),
                source[..expected_len],
                "manual opcode ${opcode:02X} did not lower exactly"
            );
        }
    }
}

fn manual_form_len(form: &str) -> usize {
    let mode = form.rsplit_once(':').unwrap().1;
    match mode {
        "Implied" | "Accumulator" => 1,
        "Immediate"
        | "ZeroPage"
        | "ZeroPageX"
        | "ZeroPageY"
        | "ZeroPageIndexedIndirectX"
        | "ZeroPageIndirectIndexedY"
        | "ZeroPageIndirect"
        | "Relative" => 2,
        "Absolute"
        | "AbsoluteX"
        | "AbsoluteY"
        | "AbsoluteIndirect"
        | "AbsoluteIndexedIndirectX"
        | "ZeroPageRelative"
        | "ImmediateZeroPage"
        | "ImmediateZeroPageX" => 3,
        "ImmediateAbsolute" | "ImmediateAbsoluteX" => 4,
        "BlockTransfer" => 7,
        _ => panic!("unknown manual addressing mode {mode}"),
    }
}
