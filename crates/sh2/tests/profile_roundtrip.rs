use std::collections::BTreeSet;

use sh2::{DecodedWord, InstructionForm, classify_word, encode_word};

#[test]
fn every_word_is_classified_and_known_words_round_trip_exactly() {
    let mut known = 0usize;
    let mut unknown = 0usize;
    let mut forms = BTreeSet::new();

    for word in 0..=u16::MAX {
        match classify_word(word) {
            DecodedWord::Instruction(instruction) => {
                known += 1;
                forms.insert(instruction.form());
                assert_eq!(
                    encode_word(&instruction).unwrap(),
                    word,
                    "word 0x{word:04X}"
                );
            }
            DecodedWord::Unknown { word: classified } => {
                unknown += 1;
                assert_eq!(classified, word);
            }
        }
    }

    assert_eq!(known, 53_752);
    assert_eq!(unknown, 11_784);
    assert_eq!(forms, InstructionForm::all().collect());
    assert_eq!(forms.len(), 142);
}

#[test]
fn form_denominator_has_no_duplicates() {
    let forms: Vec<_> = InstructionForm::all().collect();
    assert_eq!(forms.len(), 142);
    assert_eq!(forms.iter().copied().collect::<BTreeSet<_>>().len(), 142);
}
