use std::collections::BTreeSet;

use z80::{DecodeError, InstructionForm, decode_bytes, encode_bytes};

fn canonical_encodings() -> Vec<Vec<u8>> {
    let mut encodings = Vec::new();

    for opcode in 0u8..=u8::MAX {
        if matches!(opcode, 0xCB | 0xDD | 0xED | 0xFD) {
            continue;
        }
        collect_if_documented(&[opcode, 0, 0, 0], &mut encodings);
    }
    for selector in 0u8..=u8::MAX {
        collect_if_documented(&[0xCB, selector], &mut encodings);
        collect_if_documented(&[0xED, selector, 0, 0], &mut encodings);
        for prefix in [0xDD, 0xFD] {
            if selector != 0xCB {
                collect_if_documented(&[prefix, selector, 0, 0], &mut encodings);
            }
            collect_if_documented(&[prefix, 0xCB, 0, selector], &mut encodings);
        }
    }
    encodings
}

fn collect_if_documented(candidate: &[u8], encodings: &mut Vec<Vec<u8>>) {
    if let Ok(decoded) = decode_bytes(candidate) {
        encodings.push(candidate[..decoded.length()].to_vec());
    }
}

#[test]
fn canonical_selector_families_are_complete_and_exact() {
    let encodings = canonical_encodings();
    assert_eq!(encodings.len(), 698);

    let mut forms = BTreeSet::new();
    for bytes in &encodings {
        let decoded = decode_bytes(bytes).unwrap();
        assert_eq!(decoded.length(), bytes.len());
        let instruction = decoded.into_instruction();
        assert_eq!(encode_bytes(&instruction).unwrap(), *bytes);
        forms.insert(instruction.form());
    }

    assert_eq!(forms, InstructionForm::all().collect());
    assert_eq!(forms.len(), 142);
}

#[test]
fn documented_selector_family_denominators_are_fixed() {
    let primary = (0u8..=u8::MAX)
        .filter(|opcode| !matches!(opcode, 0xCB | 0xDD | 0xED | 0xFD))
        .filter(|opcode| decode_bytes(&[*opcode, 0, 0, 0]).is_ok())
        .count();
    let cb = (0u8..=u8::MAX)
        .filter(|selector| decode_bytes(&[0xCB, *selector]).is_ok())
        .count();
    let ed = (0u8..=u8::MAX)
        .filter(|selector| decode_bytes(&[0xED, *selector, 0, 0]).is_ok())
        .count();
    let indexed = [0xDD, 0xFD]
        .into_iter()
        .map(|prefix| {
            (0u8..=u8::MAX)
                .filter(|selector| *selector != 0xCB)
                .filter(|selector| decode_bytes(&[prefix, *selector, 0, 0]).is_ok())
                .count()
        })
        .sum::<usize>();
    let indexed_cb = [0xDD, 0xFD]
        .into_iter()
        .map(|prefix| {
            (0u8..=u8::MAX)
                .filter(|selector| decode_bytes(&[prefix, 0xCB, 0, *selector]).is_ok())
                .count()
        })
        .sum::<usize>();

    assert_eq!(
        (primary, cb, ed, indexed, indexed_cb),
        (252, 248, 58, 78, 62)
    );
}

#[test]
fn every_canonical_prefix_is_rejected_when_truncated() {
    for bytes in canonical_encodings() {
        for length in 0..bytes.len() {
            assert!(
                matches!(
                    decode_bytes(&bytes[..length]),
                    Err(DecodeError::UnexpectedEnd(_))
                ),
                "prefix {:02X?}",
                &bytes[..length]
            );
        }
    }
}

#[test]
fn undocumented_selector_classes_fail_closed() {
    for bytes in [
        &[0xCB, 0x30][..],
        &[0xED, 0x4C][..],
        &[0xED, 0x55][..],
        &[0xED, 0x70][..],
        &[0xDD, 0x24][..],
        &[0xFD, 0x84][..],
        &[0xDD, 0xCB, 0, 0x00][..],
        &[0xFD, 0xCB, 0, 0x30][..],
        &[0xDD, 0x00][..],
        &[0xDD, 0xDD][..],
    ] {
        assert!(matches!(
            decode_bytes(bytes),
            Err(DecodeError::UnsupportedEncoding { .. })
        ));
    }
}
