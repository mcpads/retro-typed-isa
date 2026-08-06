use std::collections::BTreeSet;

use sh4::{
    DecodedWord, FpscrState, FpuPrecision, FpuTransferSize, InstructionForm, classify_word,
    encode_word,
};

const STATES: [FpscrState; 4] = [
    FpscrState::new(FpuPrecision::Single, FpuTransferSize::Single),
    FpscrState::new(FpuPrecision::Single, FpuTransferSize::Pair),
    FpscrState::new(FpuPrecision::Double, FpuTransferSize::Single),
    FpscrState::new(FpuPrecision::Double, FpuTransferSize::Pair),
];

#[test]
fn every_state_dependent_word_round_trips_exactly() {
    for state in STATES {
        for word in 0u16..=u16::MAX {
            if let DecodedWord::Instruction(instruction) = classify_word(word, state) {
                assert_eq!(encode_word(&instruction), Ok(word), "state={state:?}");
            }
        }
    }
}

#[test]
fn official_profile_census_is_stable() {
    let counts = STATES.map(|state| {
        (0u16..=u16::MAX)
            .filter(|word| matches!(classify_word(*word, state), DecodedWord::Instruction(_)))
            .count()
    });
    assert_eq!(counts, [58_961, 58_961, 57_475, 57_475]);

    let forms: Vec<_> = InstructionForm::all().collect();
    assert_eq!(forms.len(), 225);
    assert_eq!(forms.iter().copied().collect::<BTreeSet<_>>().len(), 225);

    let decoded_forms: BTreeSet<_> = STATES
        .into_iter()
        .flat_map(|state| {
            (0u16..=u16::MAX).filter_map(move |word| match classify_word(word, state) {
                DecodedWord::Instruction(instruction) => Some(instruction.form()),
                DecodedWord::Unknown { .. } => None,
            })
        })
        .collect();
    assert_eq!(decoded_forms, forms.into_iter().collect());
}

#[test]
fn reserved_words_remain_separate_from_typed_instructions() {
    for state in STATES {
        let valid = (0u16..=u16::MAX)
            .filter(|word| matches!(classify_word(*word, state), DecodedWord::Instruction(_)))
            .count();
        let unknown = 65_536 - valid;
        assert!(unknown > 0);
        assert_eq!(valid + unknown, 65_536);
    }
}
