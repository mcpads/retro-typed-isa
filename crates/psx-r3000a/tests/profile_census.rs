use std::collections::BTreeSet;

use psx_r3000a::{DecodeError, GteOpcode, Instruction, decode_word};

const PC: u32 = 0x8001_0000;

fn assert_selector_census(
    selectors: impl IntoIterator<Item = u8>,
    supported: &[u8],
    word_for: impl Fn(u8) -> u32,
) {
    for selector in selectors {
        assert_eq!(
            decode_word(word_for(selector), PC).is_ok(),
            supported.contains(&selector),
            "unexpected classification for selector {selector:#04x}"
        );
    }
}

#[test]
fn every_primary_opcode_is_classified_by_the_profile() {
    const SUPPORTED: &[u8] = &[
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
        0x0f, 0x10, 0x12, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x28, 0x29, 0x2a, 0x2b, 0x2e,
        0x32, 0x3a,
    ];
    assert_selector_census(0..64, SUPPORTED, |opcode| u32::from(opcode) << 26);
}

#[test]
fn every_special_regimm_and_coprocessor_selector_is_classified() {
    const SPECIAL: &[u8] = &[
        0x00, 0x02, 0x03, 0x04, 0x06, 0x07, 0x08, 0x09, 0x0c, 0x0d, 0x10, 0x11, 0x12, 0x13, 0x18,
        0x19, 0x1a, 0x1b, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x2a, 0x2b,
    ];
    assert_selector_census(0..64, SPECIAL, u32::from);

    const REGIMM: &[u8] = &[0x00, 0x01, 0x10, 0x11];
    assert_selector_census(0..32, REGIMM, |selector| {
        0x0400_0000 | u32::from(selector) << 16
    });

    const COP0_TRANSFERS: &[u8] = &[0x00, 0x04];
    assert_selector_census(0..32, COP0_TRANSFERS, |selector| {
        0x4000_0000 | u32::from(selector) << 21
    });
    assert_eq!(decode_word(0x4200_0010, PC).unwrap(), Instruction::Rfe);

    const COP2_TRANSFERS: &[u8] = &[0x00, 0x02, 0x04, 0x06];
    assert_selector_census(0..32, COP2_TRANSFERS, |selector| {
        0x4800_0000 | u32::from(selector) << 21
    });
}

#[test]
fn every_gte_function_is_classified_without_aliases() {
    let declared: BTreeSet<_> = GteOpcode::ALL
        .iter()
        .map(|opcode| opcode.function())
        .collect();
    assert_eq!(declared.len(), GteOpcode::ALL.len());

    for function in 0..64_u8 {
        assert_eq!(
            decode_word(0x4a00_0000 | u32::from(function), PC).is_ok(),
            declared.contains(&function),
            "unexpected GTE function classification for {function:#04x}"
        );
    }
}

#[test]
fn each_reserved_field_class_fails_closed() {
    let reserved_words = [
        0x0020_0000, // SLL with non-zero rs
        0x0000_0044, // variable shift with non-zero sa
        0x0001_0008, // JR with non-zero rt
        0x0001_0049, // JALR with non-zero rt and sa
        0x0020_0810, // MFHI with non-zero rs and rt
        0x0020_0811, // MTHI with non-zero rt and rd
        0x0000_0818, // MULT with non-zero rd
        0x0000_0060, // ADD with non-zero sa
        0x18_010000, // BLEZ with non-zero rt
        0x3c20_0000, // LUI with non-zero rs
        0x4000_0001, // MFC0 with non-zero low field
        0x4800_0001, // MFC2 with non-zero low field
        0x4a00_0041, // GTE command with non-zero padding bit
    ];

    for word in reserved_words {
        assert!(
            matches!(
                decode_word(word, PC),
                Err(DecodeError::ReservedEncoding { .. })
            ),
            "word {word:#010x} did not fail as a reserved encoding"
        );
    }
}
