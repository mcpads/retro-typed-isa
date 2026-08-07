use std::collections::BTreeSet;

use v810::{DecodeError, InstructionForm, decode_halfwords, encode_halfwords};

const RESERVED_PRIMARY_OPCODES: [u8; 5] = [0x16, 0x1b, 0x1e, 0x32, 0x36];
const BIT_STRING_SUBOPCODES: [u8; 12] = [
    0x00, 0x01, 0x02, 0x03, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
];
const FLOATING_SUBOPCODES: [u8; 8] = [0x00, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x0b];

fn opcode_uses_second_halfword(opcode: u8) -> bool {
    matches!(opcode, 0x28..=0x3f) && !RESERVED_PRIMARY_OPCODES.contains(&opcode)
}

fn representative(opcode: u8) -> (u16, Option<u16>) {
    let first = u16::from(opcode) << 10;
    let second = opcode_uses_second_halfword(opcode).then_some(0);
    (first, second)
}

#[test]
fn appendix_b_primary_opcode_map_is_classified_directly() {
    for opcode in 0_u8..64 {
        let (first, second) = representative(opcode);
        let decoded = decode_halfwords(first, second);

        if RESERVED_PRIMARY_OPCODES.contains(&opcode) {
            assert!(
                matches!(decoded, Err(DecodeError::ReservedOpcode { .. })),
                "primary opcode {opcode:06b} was not reserved"
            );
        } else {
            let instruction = decoded
                .unwrap_or_else(|error| panic!("primary opcode {opcode:06b}: {error}"))
                .into_instruction();
            assert_eq!(encode_halfwords(&instruction).unwrap(), (first, second));
        }
    }
}

#[test]
fn appendix_c_bit_string_subopcode_map_is_classified_directly() {
    for subopcode in 0_u8..32 {
        let first = (0x1f_u16 << 10) | u16::from(subopcode);
        let decoded = decode_halfwords(first, None);
        if BIT_STRING_SUBOPCODES.contains(&subopcode) {
            let instruction = decoded.unwrap().into_instruction();
            assert_eq!(encode_halfwords(&instruction).unwrap(), (first, None));
        } else {
            assert_eq!(
                decoded,
                Err(DecodeError::ReservedBitStringSubopcode { subopcode })
            );
        }
    }
}

#[test]
fn appendix_c_floating_subopcode_map_and_rfu_bits_are_classified_directly() {
    for subopcode in 0_u8..64 {
        let first = 0x3e_u16 << 10;
        let second = u16::from(subopcode) << 10;
        let decoded = decode_halfwords(first, Some(second));
        if FLOATING_SUBOPCODES.contains(&subopcode) {
            let instruction = decoded.unwrap().into_instruction();
            assert_eq!(
                encode_halfwords(&instruction).unwrap(),
                (first, Some(second))
            );
        } else {
            assert_eq!(
                decoded,
                Err(DecodeError::ReservedFloatingSubopcode { subopcode })
            );
        }
    }

    for reserved in 1_u16..=0x03ff {
        assert!(matches!(
            decode_halfwords(0x3e_u16 << 10, Some(reserved)),
            Err(DecodeError::ReservedField { .. })
        ));
    }
}

#[test]
fn source_tables_account_for_eighty_five_typed_forms() {
    let mut forms = BTreeSet::<InstructionForm>::new();

    for opcode in 0_u8..64 {
        if RESERVED_PRIMARY_OPCODES.contains(&opcode) {
            continue;
        }
        let (first, second) = representative(opcode);
        forms.insert(
            decode_halfwords(first, second)
                .unwrap()
                .instruction()
                .form(),
        );
    }
    for condition in 0_u16..16 {
        let first = if condition == 0x0d {
            0x8000 | (condition << 9)
        } else {
            0x8000 | (condition << 9) | 2
        };
        forms.insert(decode_halfwords(first, None).unwrap().instruction().form());
    }
    for subopcode in BIT_STRING_SUBOPCODES {
        let first = (0x1f_u16 << 10) | u16::from(subopcode);
        forms.insert(decode_halfwords(first, None).unwrap().instruction().form());
    }
    for subopcode in FLOATING_SUBOPCODES {
        forms.insert(
            decode_halfwords(0x3e_u16 << 10, Some(u16::from(subopcode) << 10))
                .unwrap()
                .instruction()
                .form(),
        );
    }

    assert_eq!(forms.len(), 85);
}

#[test]
fn every_first_halfword_is_fail_closed_or_round_trips_exactly() {
    for first in u16::MIN..=u16::MAX {
        if let Ok(decoded) = decode_halfwords(first, Some(0)) {
            let expected_second = (decoded.byte_len() == 4).then_some(0);
            assert_eq!(
                encode_halfwords(decoded.instruction()).unwrap(),
                (first, expected_second),
                "first halfword {first:#06x} changed"
            );
        }
    }
}

#[test]
fn every_second_halfword_is_fail_closed_or_round_trips_exactly() {
    for opcode in 0x28_u8..=0x3f {
        if RESERVED_PRIMARY_OPCODES.contains(&opcode) {
            continue;
        }
        let first = u16::from(opcode) << 10;
        for second in u16::MIN..=u16::MAX {
            if let Ok(decoded) = decode_halfwords(first, Some(second)) {
                assert_eq!(
                    encode_halfwords(decoded.instruction()).unwrap(),
                    (first, Some(second)),
                    "instruction {first:#06x} {second:#06x} changed"
                );
            }
        }
    }
}

#[test]
fn reserved_and_write_disabled_system_registers_fail_closed() {
    for id in 0_u16..32 {
        let load = (0x1c_u16 << 10) | id;
        let store = (0x1d_u16 << 10) | id;
        let load_valid = matches!(id, 0..=3 | 5 | 24 | 25);
        let store_valid = matches!(id, 0..=7 | 24 | 25);

        assert_eq!(decode_halfwords(load, None).is_ok(), load_valid);
        assert_eq!(decode_halfwords(store, None).is_ok(), store_valid);
    }
}
