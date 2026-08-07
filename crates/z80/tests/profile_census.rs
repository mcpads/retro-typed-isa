use std::collections::BTreeSet;

use z80::{DecodeError, InstructionForm, decode_bytes, encode_bytes};

// Normative source: Zilog, Z80 CPU User Manual, UM008011-0816,
// "Z80 Instruction Set" (pages 67-291). The functions below encode the
// selector families and instruction widths printed in those instruction
// tables. They deliberately do not ask the production decoder which byte
// strings it accepts.
//
// https://www.zilog.com/docs/z80/um0080.pdf

const PREFIXES: [u8; 4] = [0xCB, 0xDD, 0xED, 0xFD];

fn primary_length(opcode: u8) -> usize {
    let x = opcode >> 6;
    let y = (opcode >> 3) & 7;
    let z = opcode & 7;
    let p = y >> 1;
    let q = y & 1;

    match x {
        0 => match z {
            0 if y >= 2 => 2,
            1 if q == 0 => 3,
            2 if p >= 2 => 3,
            6 => 2,
            _ => 1,
        },
        1 | 2 => 1,
        3 => match z {
            2 | 4 => 3,
            3 if y == 0 => 3,
            3 if matches!(y, 2 | 3) => 2,
            5 if q == 1 && p == 0 => 3,
            6 => 2,
            _ => 1,
        },
        _ => unreachable!(),
    }
}

fn cb_is_documented(selector: u8) -> bool {
    let operation_class = selector >> 6;
    let operation = (selector >> 3) & 7;
    operation_class != 0 || operation != 6
}

const ED_DOCUMENTED_SINGLE_BYTE_OPERANDS: [u8; 28] = [
    0x44, 0x45, 0x46, 0x47, 0x4D, 0x4F, 0x56, 0x57, 0x5E, 0x5F, 0x67, 0x6F, 0xA0, 0xA1, 0xA2, 0xA3,
    0xA8, 0xA9, 0xAA, 0xAB, 0xB0, 0xB1, 0xB2, 0xB3, 0xB8, 0xB9, 0xBA, 0xBB,
];

fn ed_is_documented(selector: u8) -> bool {
    let y = (selector >> 3) & 7;
    let z = selector & 7;
    (matches!(z, 0 | 1) && (0x40..=0x7F).contains(&selector) && y != 6)
        || (matches!(z, 2 | 3) && (0x40..=0x7F).contains(&selector))
        || ED_DOCUMENTED_SINGLE_BYTE_OPERANDS.contains(&selector)
}

fn ed_length(selector: u8) -> usize {
    if (0x40..=0x7F).contains(&selector) && selector & 7 == 3 {
        4
    } else {
        2
    }
}

const INDEX_DOCUMENTED_SELECTORS: [u8; 39] = [
    0x09, 0x19, 0x21, 0x22, 0x23, 0x29, 0x2A, 0x2B, 0x34, 0x35, 0x36, 0x39, 0x46, 0x4E, 0x56, 0x5E,
    0x66, 0x6E, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x77, 0x7E, 0x86, 0x8E, 0x96, 0x9E, 0xA6, 0xAE,
    0xB6, 0xBE, 0xE1, 0xE3, 0xE5, 0xE9, 0xF9,
];

fn index_is_documented(selector: u8) -> bool {
    INDEX_DOCUMENTED_SELECTORS.contains(&selector)
}

fn index_length(selector: u8) -> usize {
    match selector {
        0x21 | 0x22 | 0x2A | 0x36 => 4,
        0x34 | 0x35 | 0x46 | 0x4E | 0x56 | 0x5E | 0x66 | 0x6E | 0x70 | 0x71 | 0x72 | 0x73
        | 0x74 | 0x75 | 0x77 | 0x7E | 0x86 | 0x8E | 0x96 | 0x9E | 0xA6 | 0xAE | 0xB6 | 0xBE => 3,
        _ => 2,
    }
}

fn indexed_cb_is_documented(selector: u8) -> bool {
    selector & 7 == 6 && cb_is_documented(selector)
}

fn zero_filled_encoding(prefix: &[u8], selector: u8, length: usize) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(length);
    bytes.extend_from_slice(prefix);
    bytes.push(selector);
    bytes.resize(length, 0);
    bytes
}

fn source_defined_encodings() -> Vec<Vec<u8>> {
    let mut encodings = Vec::new();

    for opcode in 0u8..=u8::MAX {
        if PREFIXES.contains(&opcode) {
            continue;
        }
        let length = primary_length(opcode);
        let mut bytes = vec![opcode];
        bytes.resize(length, 0);
        encodings.push(bytes);
    }

    for selector in 0u8..=u8::MAX {
        if cb_is_documented(selector) {
            encodings.push(vec![0xCB, selector]);
        }
        if ed_is_documented(selector) {
            encodings.push(zero_filled_encoding(&[0xED], selector, ed_length(selector)));
        }
        for prefix in [0xDD, 0xFD] {
            if index_is_documented(selector) {
                encodings.push(zero_filled_encoding(
                    &[prefix],
                    selector,
                    index_length(selector),
                ));
            }
            if indexed_cb_is_documented(selector) {
                encodings.push(vec![prefix, 0xCB, 0, selector]);
            }
        }
    }

    encodings
}

#[test]
fn source_defined_selector_families_are_complete_and_exact() {
    let encodings = source_defined_encodings();
    assert_eq!(encodings.len(), 698);

    let mut forms = BTreeSet::new();
    for bytes in &encodings {
        let decoded = decode_bytes(bytes).unwrap_or_else(|error| {
            panic!("source-defined encoding {bytes:02X?} was rejected: {error}")
        });
        assert_eq!(decoded.length(), bytes.len(), "encoding {bytes:02X?}");
        let instruction = decoded.into_instruction();
        assert_eq!(
            encode_bytes(&instruction).unwrap(),
            *bytes,
            "encoding {bytes:02X?}"
        );
        forms.insert(instruction.form());
    }

    assert_eq!(forms, InstructionForm::all().collect());
}

#[test]
fn every_selector_namespace_matches_the_manual_tables() {
    for selector in 0u8..=u8::MAX {
        if !PREFIXES.contains(&selector) {
            let mut bytes = vec![selector];
            bytes.resize(primary_length(selector), 0);
            assert!(decode_bytes(&bytes).is_ok(), "primary {selector:02X}");
        }

        assert_eq!(
            decode_bytes(&[0xCB, selector]).is_ok(),
            cb_is_documented(selector),
            "CB {selector:02X}"
        );
        assert_eq!(
            decode_bytes(&[0xED, selector, 0, 0]).is_ok(),
            ed_is_documented(selector),
            "ED {selector:02X}"
        );

        for prefix in [0xDD, 0xFD] {
            if selector != 0xCB {
                assert_eq!(
                    decode_bytes(&[prefix, selector, 0, 0]).is_ok(),
                    index_is_documented(selector),
                    "{prefix:02X} {selector:02X}"
                );
            }
            assert_eq!(
                decode_bytes(&[prefix, 0xCB, 0, selector]).is_ok(),
                indexed_cb_is_documented(selector),
                "{prefix:02X} CB d {selector:02X}"
            );
        }
    }
}

#[test]
fn source_defined_family_denominators_are_fixed() {
    assert_eq!(
        (
            256 - PREFIXES.len(),
            (0u8..=u8::MAX).filter(|&b| cb_is_documented(b)).count(),
            (0u8..=u8::MAX).filter(|&b| ed_is_documented(b)).count(),
            2 * INDEX_DOCUMENTED_SELECTORS.len(),
            2 * (0u8..=u8::MAX)
                .filter(|&b| indexed_cb_is_documented(b))
                .count(),
        ),
        (252, 248, 58, 78, 62)
    );
}

#[test]
fn every_source_defined_encoding_is_rejected_when_truncated() {
    for bytes in source_defined_encodings() {
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
