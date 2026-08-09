use typed_isa_core::StaticSemantics;
use v30::{
    AnalysisPoint, BitIndex, DecodeError, Instruction, InstructionForm, InterruptState, Mnemonic,
    ModRmEncodingClass, Operand, OperandForm, Prefix, PrimaryOpcodeClass, Register8,
    SecondaryOpcodeClass, SecondaryOperandEncoding, SemanticPrefix, classify_primary_modrm,
    classify_primary_opcode, classify_secondary_modrm, classify_secondary_opcode, decode_bytes,
    encode_bytes,
};

fn assert_semantics(instruction: &Instruction) {
    v30::V30::semantics(
        instruction,
        &AnalysisPoint::new(
            v30::CodeLocation {
                seg: 0x1000,
                off: 0x0100,
            },
            InterruptState::Unknown,
        ),
    )
    .unwrap();
}

#[test]
fn semantic_form_denominator_is_unique_and_stable() {
    let forms = InstructionForm::all();
    assert!(!forms.is_empty());
    for (index, form) in forms.iter().enumerate() {
        assert!(!forms[..index].contains(form), "duplicate form: {form:?}");
    }
    assert_eq!(forms.len(), 21_558);
}

#[test]
fn every_semantic_form_has_a_canonical_typed_byte_vector() {
    let forms = InstructionForm::all();
    let canonical = InstructionForm::canonical_instructions();
    assert_eq!(canonical.len(), forms.len());

    for (form, vector) in forms.iter().zip(canonical) {
        assert_eq!(vector.form(), form);
        assert_eq!(encode_bytes(vector.instruction()).unwrap(), vector.bytes());
        assert_semantics(vector.instruction());
        let decoded = decode_bytes(vector.bytes()).unwrap();
        assert_eq!(
            InstructionForm::of(&decoded.instruction).as_ref(),
            Some(form)
        );
    }
}

#[test]
fn every_canonical_instruction_prefix_is_reported_as_truncated() {
    for vector in InstructionForm::canonical_instructions() {
        for cut in 0..vector.bytes().len() {
            assert!(
                matches!(
                    decode_bytes(&vector.bytes()[..cut]),
                    Err(DecodeError::Truncated { .. })
                ),
                "cut {cut} of {:?} was not truncated",
                vector.bytes()
            );
        }
    }
}

#[test]
fn form_identity_preserves_semantic_selectors_not_operand_values() {
    let low = InstructionForm::of(&Instruction::Test1 {
        target: Operand::Reg8(Register8::AL),
        bit: BitIndex::Immediate(1),
    })
    .unwrap();
    let high = InstructionForm::of(&Instruction::Test1 {
        target: Operand::Reg8(Register8::BH),
        bit: BitIndex::Immediate(7),
    })
    .unwrap();
    assert_eq!(low, high);
    assert_eq!(low.mnemonic(), Mnemonic::Test1);
    assert_eq!(
        low.operands(),
        [OperandForm::Register8, OperandForm::BitIndexImmediate8]
    );

    let repeated = InstructionForm::of(&Instruction::Repc(Box::new(Instruction::Cmpsb))).unwrap();
    assert_eq!(repeated.prefixes(), [SemanticPrefix::Repc]);
    assert_eq!(repeated.mnemonic(), Mnemonic::Cmpsb);
}

#[test]
fn memory_form_identity_preserves_address_base_and_segment_selection() {
    let es_bx_si =
        InstructionForm::of(&decode_bytes(&[0x26, 0x8b, 0x00]).unwrap().instruction).unwrap();
    assert_eq!(
        es_bx_si.operands(),
        [
            OperandForm::Register16,
            OperandForm::Memory {
                size: v30::OperandSize::Word,
                segment: Some(v30::SegmentRegister::ES),
                base: v30::EffectiveAddressBase::BxSi,
            },
        ]
    );

    let cs_bx_si =
        InstructionForm::of(&decode_bytes(&[0x2e, 0x8b, 0x00]).unwrap().instruction).unwrap();
    let es_direct =
        InstructionForm::of(&decode_bytes(&[0x26, 0x8b, 0x06, 0, 0]).unwrap().instruction).unwrap();
    assert_ne!(es_bx_si, cs_bx_si);
    assert_ne!(es_bx_si, es_direct);
}

#[test]
fn raw_bytes_have_no_executable_instruction_form() {
    assert!(InstructionForm::of(&Instruction::Unknown { raw: vec![0xff] }).is_none());
}

#[test]
fn all_primary_opcode_bytes_have_a_manual_class() {
    let mut instruction = 0;
    let mut prefix = 0;
    let mut escape = 0;
    let mut reserved = 0;

    for opcode in u8::MIN..=u8::MAX {
        match classify_primary_opcode(opcode) {
            PrimaryOpcodeClass::Instruction => {
                instruction += 1;
                let mut bytes = [0_u8; 15];
                bytes[0] = opcode;
                assert!(
                    decode_bytes(&bytes).is_ok(),
                    "classified primary opcode {opcode:#04x} did not decode"
                );
                assert_semantics(&decode_bytes(&bytes).unwrap().instruction);
            }
            PrimaryOpcodeClass::Prefix(kind) => {
                prefix += 1;
                let bytes: &[u8] = match kind {
                    Prefix::BusLock => &[0xf0, 0x01, 0x00],
                    Prefix::Rep => &[0xf3, 0xa4],
                    Prefix::Repne => &[0xf2, 0xa6],
                    Prefix::Repc => &[0x65, 0xa6],
                    Prefix::Repnc => &[0x64, 0xa6],
                    Prefix::Segment(_) => &[opcode, 0xa4],
                };
                let decoded = decode_bytes(bytes).unwrap();
                assert_semantics(&decoded.instruction);
            }
            PrimaryOpcodeClass::SecondaryEscape => escape += 1,
            PrimaryOpcodeClass::Reserved => {
                reserved += 1;
                let mut bytes = [0_u8; 15];
                bytes[0] = opcode;
                assert!(decode_bytes(&bytes).is_err());
            }
        }
    }

    assert_eq!((instruction, prefix, escape, reserved), (243, 9, 1, 3));
}

#[test]
fn all_secondary_opcode_bytes_match_the_manual_partition() {
    let mut known = 0;
    let mut reserved = 0;

    for secondary in u8::MIN..=u8::MAX {
        match classify_secondary_opcode(secondary) {
            SecondaryOpcodeClass::Instruction { mnemonic, operands } => {
                known += 1;
                let mut bytes = vec![0x0f, secondary];
                match operands {
                    SecondaryOperandEncoding::None => {}
                    SecondaryOperandEncoding::RegisterOrMemory8
                    | SecondaryOperandEncoding::RegisterPair8 => bytes.push(0xc0),
                    SecondaryOperandEncoding::RegisterOrMemoryBit {
                        immediate_index, ..
                    } => {
                        bytes.push(0xc0);
                        if immediate_index {
                            bytes.push(0);
                        }
                    }
                    SecondaryOperandEncoding::RegisterImmediate4 => {
                        bytes.extend([0xc0, 0]);
                    }
                    SecondaryOperandEncoding::Immediate8 => bytes.push(0),
                }
                let decoded = decode_bytes(&bytes).unwrap();
                assert_semantics(&decoded.instruction);
                assert_eq!(
                    InstructionForm::of(&decoded.instruction)
                        .unwrap()
                        .mnemonic(),
                    mnemonic
                );
            }
            SecondaryOpcodeClass::Reserved => {
                reserved += 1;
                let bytes = [0x0f, secondary, 0, 0, 0, 0];
                assert!(decode_bytes(&bytes).is_err());
            }
        }
    }

    assert_eq!((known, reserved), (26, 230));
}

#[test]
fn every_primary_modrm_selector_matches_the_decoder() {
    for opcode in u8::MIN..=u8::MAX {
        for modrm in u8::MIN..=u8::MAX {
            let class = classify_primary_modrm(opcode, modrm);
            if class == ModRmEncodingClass::NotUsed {
                continue;
            }
            let mut bytes = [0_u8; 15];
            bytes[0] = opcode;
            bytes[1] = modrm;
            let decoded = decode_bytes(&bytes);
            assert_eq!(
                decoded.is_ok(),
                class == ModRmEncodingClass::Valid,
                "primary={opcode:#04x}, modrm={modrm:#04x}, class={class:?}"
            );
            if let Ok(decoded) = decoded {
                assert_semantics(&decoded.instruction);
            }
        }
    }
}

#[test]
fn every_secondary_modrm_selector_matches_the_decoder() {
    for secondary in u8::MIN..=u8::MAX {
        for modrm in u8::MIN..=u8::MAX {
            let class = classify_secondary_modrm(secondary, modrm);
            if class == ModRmEncodingClass::NotUsed {
                continue;
            }
            let bytes = [0x0f, secondary, modrm, 0, 0, 0, 0];
            let decoded = decode_bytes(&bytes);
            assert_eq!(
                decoded.is_ok(),
                class == ModRmEncodingClass::Valid,
                "secondary={secondary:#04x}, modrm={modrm:#04x}, class={class:?}"
            );
            if let Ok(decoded) = decoded {
                assert_semantics(&decoded.instruction);
            }
        }
    }
}

#[test]
fn every_selector_probe_reaches_a_stable_canonical_semantic_value() {
    let mut probe = [0_u8; 15];

    for opcode in u8::MIN..=u8::MAX {
        for following in u8::MIN..=u8::MAX {
            probe.fill(0);
            probe[0] = opcode;
            probe[1] = following;
            assert_canonical_fixed_point(&probe);
        }
    }
    for secondary in u8::MIN..=u8::MAX {
        for modrm in u8::MIN..=u8::MAX {
            probe.fill(0);
            probe[0] = 0x0f;
            probe[1] = secondary;
            probe[2] = modrm;
            assert_canonical_fixed_point(&probe);
        }
    }
}

#[test]
fn documented_encoding_alias_families_preserve_exact_source_bytes() {
    let fixtures: &[(&[u8], &[u8])] = &[
        (&[0x8b, 0xe0], &[0x89, 0xc4]),
        (&[0x8f, 0xc3], &[0x5b]),
        (&[0xff, 0xf3], &[0x53]),
        (&[0xff, 0xc3], &[0x43]),
        (&[0xff, 0xcb], &[0x4b]),
        (&[0xc7, 0xc3, 0x34, 0x12], &[0xbb, 0x34, 0x12]),
        (&[0x8b, 0x06, 0x34, 0x12], &[0xa1, 0x34, 0x12]),
        (&[0x81, 0xc0, 0x34, 0x12], &[0x05, 0x34, 0x12]),
        (&[0x81, 0xc3, 0xff, 0xff], &[0x83, 0xc3, 0xff]),
        (&[0x82, 0xc3, 0x7f], &[0x80, 0xc3, 0x7f]),
        (&[0xf7, 0xc0, 0x34, 0x12], &[0xa9, 0x34, 0x12]),
        (&[0xcd, 0x03], &[0xcc]),
        (&[0xc2, 0x00, 0x00], &[0xc3]),
        (&[0xca, 0x00, 0x00], &[0xcb]),
        (&[0x8b, 0x40, 0x00], &[0x8b, 0x00]),
        (&[0x8b, 0x80, 0x01, 0x00], &[0x8b, 0x40, 0x01]),
        (&[0x87, 0xd8], &[0x93]),
        (&[0x26, 0xf0, 0x01, 0x00], &[0xf0, 0x26, 0x01, 0x00]),
        (&[0x26, 0x26, 0xa4], &[0x26, 0xa4]),
        (&[0xf2, 0xf3, 0xa4], &[0xf3, 0xa4]),
        (&[0xf0, 0xf0, 0x90], &[0xf0, 0x90]),
    ];

    for &(source, canonical) in fixtures {
        let decoded = decode_bytes(source).unwrap();
        assert_eq!(decoded.source_bytes(), source);
        assert_eq!(encode_bytes(&decoded.instruction).unwrap(), canonical);
        assert_eq!(
            decode_bytes(canonical).unwrap().instruction,
            decoded.instruction
        );
    }
}

fn assert_canonical_fixed_point(probe: &[u8]) {
    let Ok(decoded) = decode_bytes(probe) else {
        return;
    };
    let canonical = encode_bytes(&decoded.instruction).unwrap();
    let canonical_decoded = decode_bytes(&canonical).unwrap();
    assert_eq!(canonical_decoded.instruction, decoded.instruction);
    assert_eq!(
        encode_bytes(&canonical_decoded.instruction).unwrap(),
        canonical
    );
}
