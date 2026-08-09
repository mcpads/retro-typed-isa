use std::collections::BTreeSet;

use sh4::{
    AnalysisPoint, DecodedWord, FloatBank, FpscrState, FpuPrecision, FpuTransferSize,
    GeneralRegisterBank, Instruction, InstructionForm, Sh4Pr0Sz0, Sh4Pr0Sz1, Sh4Pr1Sz0, Sh4Pr1Sz1,
    classify_word, encode_word,
};
use typed_isa_core::StaticSemantics;

// Normative source: STMicroelectronics and Hitachi, SH-4 32-bit CPU Core
// Architecture, ADCS 7182230F, section 7.2 "Instruction Set". The predicates
// below are a direct transcription of the instruction-code columns, including
// the FPSCR.PR and FPSCR.SZ restrictions in the FPU tables. They do not use the
// production decoder or its form catalog to decide whether a word exists.
//
// https://www.st.com/resource/en/user_manual/cd00147165-sh-4-32-bit-cpu-core-architecture-stmicroelectronics.pdf

const STATES: [FpscrState; 4] = [
    FpscrState::new(FpuPrecision::Single, FpuTransferSize::Single),
    FpscrState::new(FpuPrecision::Single, FpuTransferSize::Pair),
    FpscrState::new(FpuPrecision::Double, FpuTransferSize::Single),
    FpscrState::new(FpuPrecision::Double, FpuTransferSize::Pair),
];

const GROUP_4_FIXED_LOW_BYTES: [u8; 54] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0E, 0x10, 0x11, 0x12,
    0x13, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1E, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26,
    0x27, 0x28, 0x29, 0x2A, 0x2B, 0x2E, 0x32, 0x33, 0x37, 0x3E, 0x43, 0x47, 0x4E, 0x52, 0x56, 0x5A,
    0x62, 0x66, 0x6A, 0xF2, 0xF6, 0xFA,
];

fn group_0_is_documented(word: u16) -> bool {
    let m = ((word >> 4) & 0xF) as u8;
    match word & 0xF {
        0x2 => matches!(m, 0..=4 | 8..=15),
        0x3 => matches!(m, 0 | 2 | 8..=12),
        0x4..=0x7 | 0xC..=0xF => true,
        0x8 => matches!(word, 0x0008 | 0x0018 | 0x0028 | 0x0038 | 0x0048 | 0x0058),
        0x9 => matches!(word, 0x0009 | 0x0019) || m == 2,
        0xA => matches!(m, 0 | 1 | 2 | 3 | 5 | 6 | 15),
        0xB => matches!(word, 0x000B | 0x001B | 0x002B),
        _ => false,
    }
}

fn group_4_is_documented(word: u16) -> bool {
    let low = word as u8;
    let m = (low >> 4) & 0xF;
    let low_nibble = low & 0xF;

    GROUP_4_FIXED_LOW_BYTES.contains(&low)
        || matches!(low_nibble, 0xC | 0xD | 0xF)
        || (matches!(low_nibble, 0x3 | 0x7 | 0xE) && m >= 8)
}

fn group_f_is_documented(word: u16, state: FpscrState) -> bool {
    let n = ((word >> 8) & 0xF) as u8;
    let m = ((word >> 4) & 0xF) as u8;
    let low = (word & 0xF) as u8;
    let single = state.precision() == FpuPrecision::Single;
    let double_registers = n & 1 == 0 && m & 1 == 0;

    match low {
        0x0..=0x5 => single || double_registers,
        0x6..=0xC => true,
        0xD => match m {
            0x0 | 0x1 | 0x4 | 0x5 | 0x6 => single || n & 1 == 0,
            0x2 | 0x3 => true,
            0x8 | 0x9 | 0xE => single,
            0xA | 0xB => !single && n & 1 == 0,
            0xF => single && (matches!(word, 0xF3FD | 0xFBFD) || word & 0xF3FF == 0xF1FD),
            _ => false,
        },
        0xE => single,
        _ => false,
    }
}

fn manual_word_is_documented(word: u16, state: FpscrState) -> bool {
    let low_nibble = (word & 0xF) as u8;
    match word >> 12 {
        0x0 => group_0_is_documented(word),
        0x1 | 0x5 | 0x6 | 0x7 | 0x9..=0xE => true,
        0x2 => low_nibble != 0x3,
        0x3 => !matches!(low_nibble, 0x1 | 0x9),
        0x4 => group_4_is_documented(word),
        0x8 => matches!((word >> 8) & 0xF, 0 | 1 | 4 | 5 | 8 | 9 | 0xB | 0xD | 0xF),
        0xF => group_f_is_documented(word, state),
        _ => false,
    }
}

fn assert_semantics(instruction: &Instruction, state: FpscrState) {
    let point = AnalysisPoint::new(
        0x1000,
        GeneralRegisterBank::Primary,
        FloatBank::Primary,
        Some(state),
    );
    match (state.precision(), state.transfer_size()) {
        (FpuPrecision::Single, FpuTransferSize::Single) => {
            Sh4Pr0Sz0::semantics(instruction, &point).unwrap();
        }
        (FpuPrecision::Single, FpuTransferSize::Pair) => {
            Sh4Pr0Sz1::semantics(instruction, &point).unwrap();
        }
        (FpuPrecision::Double, FpuTransferSize::Single) => {
            Sh4Pr1Sz0::semantics(instruction, &point).unwrap();
        }
        (FpuPrecision::Double, FpuTransferSize::Pair) => {
            Sh4Pr1Sz1::semantics(instruction, &point).unwrap();
        }
    }
}

#[test]
fn every_fpscr_profile_matches_the_manual_opcode_space() {
    let mut all_decoded_forms = BTreeSet::new();
    let mut source_counts = Vec::new();

    for state in STATES {
        let mut source_count = 0usize;
        for word in u16::MIN..=u16::MAX {
            let source_defined = manual_word_is_documented(word, state);
            if source_defined {
                source_count += 1;
            }

            match (source_defined, classify_word(word, state)) {
                (true, DecodedWord::Instruction(instruction)) => {
                    all_decoded_forms.insert(instruction.form());
                    assert_eq!(encode_word(&instruction), Ok(word), "state={state:?}");
                    assert_semantics(&instruction, state);
                }
                (false, DecodedWord::Unknown { word: actual }) => assert_eq!(actual, word),
                (true, DecodedWord::Unknown { .. }) => {
                    panic!("manual word 0x{word:04X} rejected for state {state:?}")
                }
                (false, DecodedWord::Instruction(instruction)) => panic!(
                    "reserved manual word 0x{word:04X} decoded as {instruction:?} for {state:?}"
                ),
            }
        }
        source_counts.push(source_count);
    }

    assert_eq!(source_counts, [58_961, 58_961, 57_475, 57_475]);
    assert_eq!(all_decoded_forms, InstructionForm::all().collect());
}
