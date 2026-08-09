//! Source-derived SM83 encoding and semantic admission checks.
//!
//! Normative source: Nintendo, *Game Boy Programming Manual*, version 1.1,
//! Appendix 2 "Register and Instruction Set Summaries", pages 248-264.
//! https://archive.org/download/GameBoyProgManVer1.1/GameBoyProgManVer1.1.pdf
//!
//! Corrections to the official selector presentation are cross-checked against
//! the Pan Docs CPU instruction tables already identified by the crate API
//! documentation. The fixed illegal-selector set below is not derived from the
//! production classifier or canonical instruction catalog.

use std::collections::BTreeSet;

use sm83::{
    BaseOpcodeClass, Boundary, DecodeError, Sm83, canonical_instructions, classify_base_opcode,
    decode_bytes, encode_bytes,
};
use typed_isa_core::{ControlAction, ControlBoundary, StaticSemantics};

const SOURCE_ILLEGAL_BASE_SELECTORS: [u8; 11] = [
    0xd3, 0xdb, 0xdd, 0xe3, 0xe4, 0xeb, 0xec, 0xed, 0xf4, 0xfc, 0xfd,
];

#[test]
fn every_base_selector_is_explicitly_classified() {
    let mut instructions = 0;
    let mut prefixes = 0;
    let mut illegal = 0;

    for opcode in u8::MIN..=u8::MAX {
        let source_class = if opcode == 0xcb {
            BaseOpcodeClass::Prefix
        } else if SOURCE_ILLEGAL_BASE_SELECTORS.contains(&opcode) {
            BaseOpcodeClass::Illegal
        } else {
            BaseOpcodeClass::Instruction
        };
        assert_eq!(
            classify_base_opcode(opcode),
            source_class,
            "source selector ${opcode:02X} has the wrong class"
        );
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceFlowClass {
    Continue,
    ConditionalTransfer,
    Transfer,
    ConditionalLink,
    Link,
    ConditionalReturn,
    Return,
    ExceptionReturn,
    Wait,
    Stop,
}

fn source_flow_class(opcode: u8) -> SourceFlowClass {
    match opcode {
        0x20 | 0x28 | 0x30 | 0x38 | 0xc2 | 0xca | 0xd2 | 0xda => {
            SourceFlowClass::ConditionalTransfer
        }
        0x18 | 0xc3 | 0xe9 => SourceFlowClass::Transfer,
        0xc4 | 0xcc | 0xd4 | 0xdc => SourceFlowClass::ConditionalLink,
        0xcd | 0xc7 | 0xcf | 0xd7 | 0xdf | 0xe7 | 0xef | 0xf7 | 0xff => SourceFlowClass::Link,
        0xc0 | 0xc8 | 0xd0 | 0xd8 => SourceFlowClass::ConditionalReturn,
        0xc9 => SourceFlowClass::Return,
        0xd9 => SourceFlowClass::ExceptionReturn,
        0x76 => SourceFlowClass::Wait,
        0x10 => SourceFlowClass::Stop,
        _ => SourceFlowClass::Continue,
    }
}

#[test]
fn every_source_instruction_selector_has_total_static_semantics() {
    for opcode in u8::MIN..=u8::MAX {
        if opcode == 0xcb || SOURCE_ILLEGAL_BASE_SELECTORS.contains(&opcode) {
            continue;
        }
        let decoded = decode_bytes(&[opcode, 0, 0]).unwrap();
        let semantics = Sm83::semantics(decoded.instruction(), &0x4000).unwrap();
        let actual = source_flow(&semantics.control_flow, opcode);
        assert_eq!(
            actual,
            source_flow_class(opcode),
            "source base selector ${opcode:02X} has the wrong flow class"
        );
    }

    for opcode in u8::MIN..=u8::MAX {
        let decoded = decode_bytes(&[0xcb, opcode]).unwrap();
        let semantics = Sm83::semantics(decoded.instruction(), &0x4000).unwrap();
        assert_eq!(
            source_flow(&semantics.control_flow, opcode),
            SourceFlowClass::Continue,
            "source CB selector ${opcode:02X} must continue sequentially"
        );
    }
}

fn source_flow(flow: &sm83::ControlFlow, opcode: u8) -> SourceFlowClass {
    match &flow.action {
        ControlAction::Continue => SourceFlowClass::Continue,
        ControlAction::Transfer { .. } if flow.fallthrough.is_some() => {
            SourceFlowClass::ConditionalTransfer
        }
        ControlAction::Transfer { .. } => SourceFlowClass::Transfer,
        ControlAction::LinkedTransfer { .. } if flow.fallthrough.is_some() => {
            SourceFlowClass::ConditionalLink
        }
        ControlAction::LinkedTransfer { .. } => SourceFlowClass::Link,
        ControlAction::Return { .. } if flow.fallthrough.is_some() => {
            SourceFlowClass::ConditionalReturn
        }
        ControlAction::Return { .. } => SourceFlowClass::Return,
        ControlAction::ExceptionReturn { .. } => SourceFlowClass::ExceptionReturn,
        ControlAction::Boundary(ControlBoundary::Wait {
            kind: Boundary::Halt,
            ..
        }) => SourceFlowClass::Wait,
        ControlAction::Boundary(ControlBoundary::Stop {
            kind: Boundary::Stop,
        }) => SourceFlowClass::Stop,
        ControlAction::Boundary(_) => {
            panic!("source selector ${opcode:02X} has an impossible SM83 boundary")
        }
    }
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
