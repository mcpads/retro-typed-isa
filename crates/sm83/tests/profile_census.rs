use std::collections::BTreeSet;

use sm83::{
    BaseOpcodeClass, DecodeError, canonical_instructions, classify_base_opcode, decode_bytes,
    encode_bytes,
};

#[test]
fn every_base_selector_is_explicitly_classified() {
    let mut instructions = 0;
    let mut prefixes = 0;
    let mut illegal = 0;

    for opcode in u8::MIN..=u8::MAX {
        match classify_base_opcode(opcode) {
            BaseOpcodeClass::Instruction => {
                instructions += 1;
                let probe = [opcode, 0, 0];
                let decoded = decode_bytes(&probe).expect("classified selector must decode");
                let encoded = encode_bytes(decoded.instruction());
                assert_eq!(encoded, probe[..decoded.length()]);
            }
            BaseOpcodeClass::Prefix => {
                prefixes += 1;
                assert_eq!(opcode, 0xcb);
                assert!(matches!(
                    decode_bytes(&[opcode]),
                    Err(DecodeError::UnexpectedEnd { .. })
                ));
            }
            BaseOpcodeClass::Illegal => {
                illegal += 1;
                assert_eq!(
                    decode_bytes(&[opcode, 0, 0]),
                    Err(DecodeError::IllegalOpcode { opcode })
                );
            }
        }
    }

    assert_eq!((instructions, prefixes, illegal), (244, 1, 11));
}

#[test]
fn every_cb_selector_round_trips_exactly() {
    for opcode in u8::MIN..=u8::MAX {
        let source = [0xcb, opcode];
        let decoded = decode_bytes(&source).expect("every CB selector must decode");
        assert_eq!(decoded.length(), 2);
        assert_eq!(encode_bytes(decoded.instruction()), source);
    }
}

#[test]
fn canonical_vectors_cover_each_semantic_form_once() {
    let vectors = canonical_instructions();
    assert_eq!(vectors.len(), 500);

    let forms: BTreeSet<_> = vectors.iter().map(|vector| vector.form()).collect();
    assert_eq!(forms.len(), vectors.len());

    for vector in vectors {
        let decoded = decode_bytes(vector.bytes()).unwrap();
        assert_eq!(decoded.instruction(), vector.instruction());
        assert_eq!(encode_bytes(vector.instruction()), vector.bytes());
    }
}

#[test]
fn every_canonical_instruction_prefix_is_truncated() {
    for vector in canonical_instructions() {
        for cut in 0..vector.bytes().len() {
            assert!(
                matches!(
                    decode_bytes(&vector.bytes()[..cut]),
                    Err(DecodeError::UnexpectedEnd { .. })
                ),
                "cut {cut} of {:02X?} was not reported as truncated",
                vector.bytes()
            );
        }
    }
}

#[test]
fn stop_rejects_nonzero_padding() {
    assert_eq!(
        decode_bytes(&[0x10, 0x01]),
        Err(DecodeError::InvalidStopPadding { padding: 0x01 })
    );
}
