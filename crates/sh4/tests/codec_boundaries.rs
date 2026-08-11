use sh4::{
    DecodedWord, DoubleRegister, ExtendedDoubleRegister, FloatRegister, FpscrState,
    FpuPairRegister, FpuPrecision, FpuTransferSize, Instruction, VectorRegister, classify_word,
    decode_bytes, decode_word, encode_bytes, encode_word,
};

const SINGLE: FpscrState = FpscrState::new(FpuPrecision::Single, FpuTransferSize::Single);
const SINGLE_PAIR: FpscrState = FpscrState::new(FpuPrecision::Single, FpuTransferSize::Pair);
const DOUBLE: FpscrState = FpscrState::new(FpuPrecision::Double, FpuTransferSize::Single);

#[test]
fn byte_lowering_is_little_endian_and_fails_closed() {
    assert_eq!(encode_bytes(&Instruction::Nop), Ok([0x09, 0x00]));
    assert_eq!(
        decode_bytes(&[0x20, 0xF0], SINGLE).unwrap().to_string(),
        "FADD    FR2,FR0"
    );
    assert!(decode_bytes(&[0x09], SINGLE).is_err());
    assert!(decode_word(0xFFFF, SINGLE).is_err());
}

#[test]
fn precision_state_changes_arithmetic_identity() {
    assert_eq!(
        decode_word(0xF020, SINGLE),
        Ok(Instruction::Fadd {
            frm: FloatRegister::new(2).unwrap(),
            frn: FloatRegister::new(0).unwrap(),
        })
    );
    assert_eq!(
        decode_word(0xF020, DOUBLE),
        Ok(Instruction::FaddDouble {
            drm: DoubleRegister::new(2).unwrap(),
            drn: DoubleRegister::new(0).unwrap(),
        })
    );
    assert!(matches!(
        classify_word(0xF120, DOUBLE),
        DecodedWord::Unknown { word: 0xF120 }
    ));
}

#[test]
fn renesas_fpu_transfer_and_conversion_selectors_match_hardware_names() {
    // Renesas SH-4 Software Manual, table 7.10 and sections 9.34, 9.35,
    // 9.44, and 9.46: https://www.renesas.com/en/document/mas/sh-4
    let fr3 = FloatRegister::new(3).unwrap();
    let single_cases = [
        (0xF30D, Instruction::Fsts { frn: fr3 }),
        (0xF31D, Instruction::Flds { frm: fr3 }),
        (0xF32D, Instruction::Float { frn: fr3 }),
        (0xF33D, Instruction::Ftrc { frm: fr3 }),
    ];
    for (word, instruction) in single_cases {
        assert_eq!(decode_word(word, SINGLE), Ok(instruction));
        assert_eq!(encode_word(&instruction), Ok(word));
    }

    let fr2 = FloatRegister::new(2).unwrap();
    let dr2 = DoubleRegister::new(2).unwrap();
    let double_cases = [
        (0xF20D, Instruction::Fsts { frn: fr2 }),
        (0xF21D, Instruction::Flds { frm: fr2 }),
        (0xF22D, Instruction::FloatDouble { drn: dr2 }),
        (0xF23D, Instruction::FtrcDouble { drm: dr2 }),
    ];
    for (word, instruction) in double_cases {
        assert_eq!(decode_word(word, DOUBLE), Ok(instruction));
        assert_eq!(encode_word(&instruction), Ok(word));
    }

    assert!(matches!(
        classify_word(0xF12D, DOUBLE),
        DecodedWord::Unknown { word: 0xF12D }
    ));
    assert!(matches!(
        classify_word(0xF13D, DOUBLE),
        DecodedWord::Unknown { word: 0xF13D }
    ));
}

#[test]
fn transfer_size_preserves_dr_and_xd_selectors() {
    assert_eq!(
        decode_word(0xF12C, SINGLE_PAIR),
        Ok(Instruction::FmovPair {
            source: FpuPairRegister::Double(DoubleRegister::new(2).unwrap()),
            destination: FpuPairRegister::Extended(ExtendedDoubleRegister::new(0).unwrap()),
        })
    );
    assert!(matches!(
        decode_word(0xF12C, SINGLE),
        Ok(Instruction::Fmov { .. })
    ));
}

#[test]
fn graphics_selectors_are_not_collapsed() {
    assert_eq!(
        decode_word(0xF9ED, SINGLE),
        Ok(Instruction::Fipr {
            fvm: VectorRegister::new(4).unwrap(),
            fvn: VectorRegister::new(8).unwrap(),
        })
    );
    assert!(decode_word(0xF9ED, DOUBLE).is_err());
    assert_eq!(
        decode_word(0xFDFD, SINGLE),
        Ok(Instruction::Ftrv {
            fvn: VectorRegister::new(12).unwrap(),
        })
    );
}

#[test]
fn register_constructors_reject_unencodable_values() {
    assert!(FloatRegister::new(16).is_none());
    assert!(DoubleRegister::new(1).is_none());
    assert!(DoubleRegister::new(16).is_none());
    assert!(ExtendedDoubleRegister::new(3).is_none());
    assert!(VectorRegister::new(2).is_none());
    assert!(VectorRegister::new(16).is_none());
}
