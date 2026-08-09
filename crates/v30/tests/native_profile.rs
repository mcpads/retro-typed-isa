use v30::{
    BitFieldLength, BitIndex, CodeLocation, DecodeError, EffectiveAddress, EffectiveAddressBase,
    EffectiveAddressDisplacement, EffectiveAddressError, Instruction, Operand, OperandSize, Prefix,
    Register8, Register16, SignedImmediate, decode_bytes, encode_bytes, verify_placed_program,
};

fn assert_canonical(bytes: &[u8], expected: Instruction) {
    let decoded = decode_bytes(bytes).unwrap();
    assert_eq!(decoded.byte_len, bytes.len());
    assert_eq!(decoded.instruction, expected);
    assert_eq!(encode_bytes(&decoded.instruction).unwrap(), bytes);
}

#[test]
fn v30_and_80186_native_additions_lower_to_bytes() {
    assert_canonical(&[0x60], Instruction::Pusha);
    assert_canonical(&[0x61], Instruction::Popa);
    assert_canonical(
        &[0x6a, 0xfe],
        Instruction::Push {
            src: Operand::Imm8(0xfe),
        },
    );
    assert_canonical(
        &[0x6b, 0xc3, 0xfe],
        Instruction::ImulImmediate {
            dest: Register16::AX,
            src: Operand::Reg16(Register16::BX),
            immediate: SignedImmediate::Byte(-2),
        },
    );
    assert_canonical(
        &[0xc0, 0xe0, 0x04],
        Instruction::Shl {
            dest: Operand::Reg8(Register8::AL),
            count: v30::ShiftCount::Imm8(4),
        },
    );
    assert_canonical(&[0xce], Instruction::Into);
    assert_canonical(&[0x6c], Instruction::Insb);
    assert_canonical(&[0x6f], Instruction::Outsw);
}

#[test]
fn v30_register_bit_fields_use_only_the_documented_register_forms() {
    assert_canonical(
        &[0x0f, 0x31, 0xd4],
        Instruction::Ins {
            bit_offset: Register8::AH,
            length: Register8::DL,
        },
    );
    assert_canonical(
        &[0x0f, 0x3b, 0xc1, 0x0f],
        Instruction::ExtImm {
            bit_offset: Register8::CL,
            length: BitFieldLength::try_from(15).unwrap(),
        },
    );

    for reserved in [
        &[0x0f, 0x31, 0x00][..],
        &[0x0f, 0x39, 0xc8, 0x07],
        &[0x0f, 0x3b, 0xc1, 0x10],
    ] {
        assert!(matches!(
            decode_bytes(reserved),
            Err(DecodeError::ReservedSelector { .. })
        ));
    }
}

#[test]
fn v30_bit_operations_preserve_width_and_index_source() {
    assert_canonical(
        &[0x0f, 0x10, 0xc0],
        Instruction::Test1 {
            target: Operand::Reg8(Register8::AL),
            bit: BitIndex::Cl,
        },
    );
    assert_canonical(
        &[0x0f, 0x1f, 0xc3, 0x0f],
        Instruction::Not1 {
            target: Operand::Reg16(Register16::BX),
            bit: BitIndex::Immediate(15),
        },
    );

    for reserved in [
        &[0x0f, 0x10, 0xc8][..],
        &[0x0f, 0x18, 0xc0, 0x08],
        &[0x0f, 0x19, 0xc0, 0x10],
        &[0x0f, 0x28, 0xc8],
        &[0x8e, 0xc8],
    ] {
        assert!(matches!(
            decode_bytes(reserved),
            Err(DecodeError::ReservedSelector { .. })
        ));
    }
}

#[test]
fn public_selector_types_reject_values_outside_the_encoding_field() {
    assert!(Register8::try_from(8).is_err());
    assert!(Register16::try_from(8).is_err());
    assert!(BitFieldLength::try_from(16).is_err());
}

#[test]
fn effective_address_construction_keeps_absolute_and_signed_offsets_distinct() {
    let direct = EffectiveAddress::new(
        None,
        EffectiveAddressBase::Direct,
        EffectiveAddressDisplacement::Absolute(0xf000),
        OperandSize::Byte,
    )
    .unwrap();
    assert_eq!(
        direct.displacement(),
        EffectiveAddressDisplacement::Absolute(0xf000)
    );

    let based = EffectiveAddress::new(
        None,
        EffectiveAddressBase::Bp,
        EffectiveAddressDisplacement::Signed(-4),
        OperandSize::Word,
    )
    .unwrap();
    assert_eq!(
        based.displacement(),
        EffectiveAddressDisplacement::Signed(-4)
    );
    assert_eq!(
        EffectiveAddress::new(
            None,
            EffectiveAddressBase::Direct,
            EffectiveAddressDisplacement::Signed(0),
            OperandSize::Byte,
        ),
        Err(EffectiveAddressError::DirectRequiresAbsolute)
    );
}

#[test]
fn repeat_prefixes_reject_instruction_families_the_manual_does_not_define() {
    assert_eq!(
        decode_bytes(&[0x65, 0xa4]),
        Err(DecodeError::InvalidEncoding)
    );
    assert!(v30::InstructionForm::of(&Instruction::Repc(Box::new(Instruction::Movsb))).is_none());
    assert!(decode_bytes(&[0x65, 0xa6]).is_ok());
    assert!(decode_bytes(&[0x64, 0xae]).is_ok());
}

#[test]
fn segment_override_is_typed_for_implicit_source_strings() {
    let decoded = decode_bytes(&[0x26, 0xa4]).unwrap();
    assert_eq!(
        decoded.instruction,
        Instruction::SegmentOverride {
            segment: v30::SegmentRegister::ES,
            instruction: Box::new(Instruction::Movsb),
        }
    );
    assert_eq!(encode_bytes(&decoded.instruction).unwrap(), [0x26, 0xa4]);

    let repeated = decode_bytes(&[0x2e, 0x65, 0xa6]).unwrap();
    assert_eq!(
        encode_bytes(&repeated.instruction).unwrap(),
        [0x2e, 0x65, 0xa6]
    );
    assert_eq!(
        decode_bytes(&[0x26, 0xaa]),
        Err(DecodeError::InvalidEncoding)
    );

    let translated = decode_bytes(&[0x2e, 0xd7]).unwrap();
    assert_eq!(
        translated.instruction,
        Instruction::SegmentOverride {
            segment: v30::SegmentRegister::CS,
            instruction: Box::new(Instruction::Xlat),
        }
    );
    assert_eq!(encode_bytes(&translated.instruction).unwrap(), [0x2e, 0xd7]);
}

#[test]
fn bus_lock_applies_to_the_following_native_instruction() {
    assert_canonical(
        &[0xf0, 0x90],
        Instruction::BusLock(Box::new(Instruction::Nop)),
    );

    let combined = decode_bytes(&[0xf0, 0x26, 0xf3, 0xa4]).unwrap();
    assert_eq!(
        combined.prefixes.as_slice(),
        [
            Prefix::BusLock,
            Prefix::Segment(v30::SegmentRegister::ES),
            Prefix::Rep,
        ]
    );
    assert_eq!(
        encode_bytes(&combined.instruction).unwrap(),
        [0xf0, 0x26, 0xf3, 0xa4]
    );

    let placed = verify_placed_program(
        &[0xf0, 0xeb, 0xfe],
        CodeLocation {
            seg: 0x1000,
            off: 0,
        },
    )
    .unwrap();
    assert!(matches!(
        &placed[0].control_flow.action,
        typed_isa_core::ControlAction::Transfer {
            target: typed_isa_core::ControlTarget::Direct(target)
        } if target.location == CodeLocation {
            seg: 0x1000,
            off: 1,
        }
    ));
}
