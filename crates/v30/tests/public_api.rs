use v30::{DecodeError, Instruction, PROFILE_ID, Prefix, Register16, decode_bytes, encode_bytes};

#[test]
fn profile_and_common_byte_api_are_stable() {
    assert_eq!(PROFILE_ID, "v30");

    let instruction = Instruction::Mov {
        dest: v30::Operand::Reg16(Register16::BX),
        src: v30::Operand::Reg16(Register16::AX),
    };
    let bytes = encode_bytes(&instruction).unwrap();
    assert_eq!(bytes, [0x89, 0xc3]);
    assert_eq!(decode_bytes(&bytes).unwrap().instruction, instruction);
}

#[test]
fn static_decode_preserves_prefix_order_but_raw_bytes_are_not_executable() {
    let decoded = decode_bytes(&[0x26, 0xf3, 0xa4]).unwrap();
    assert_eq!(
        decoded.prefixes.as_slice(),
        [Prefix::Segment(v30::SegmentRegister::ES), Prefix::Rep]
    );
    assert!(matches!(
        encode_bytes(&Instruction::Unknown { raw: vec![0x66] }),
        Err(v30::EncodeError::RawBytesNotExecutable)
    ));
    assert_eq!(
        decode_bytes(&[0x66]),
        Err(DecodeError::Truncated { needed: 2, got: 1 })
    );
    assert_eq!(
        decode_bytes(&[0x0f]),
        Err(DecodeError::Truncated { needed: 2, got: 1 })
    );
    assert_eq!(
        decode_bytes(&[0xf3]),
        Err(DecodeError::Truncated { needed: 2, got: 1 })
    );
    assert!(matches!(
        decode_bytes(&[0x0f, 0x00]),
        Err(DecodeError::ReservedSelector { opcode: 0x0f })
    ));
}

#[test]
fn brkem_is_decoded_as_a_native_static_boundary() {
    let decoded = decode_bytes(&[0x0f, 0xff, 0x80]).unwrap();
    assert_eq!(decoded.byte_len, 3);
    assert_eq!(decoded.instruction, Instruction::Brkem { vector: 0x80 });
    assert_eq!(
        encode_bytes(&decoded.instruction).unwrap(),
        [0x0f, 0xff, 0x80]
    );
}

#[test]
fn raw_bytes_cannot_hide_inside_a_typed_prefix() {
    let instruction = Instruction::BusLock(Box::new(Instruction::Unknown { raw: vec![0x90] }));
    assert_eq!(
        encode_bytes(&instruction),
        Err(v30::EncodeError::RawBytesNotExecutable)
    );
}

#[test]
fn typed_prefixes_must_use_canonical_semantic_order() {
    let reversed = Instruction::Rep(Box::new(Instruction::BusLock(Box::new(Instruction::Movsb))));
    assert_eq!(
        encode_bytes(&reversed),
        Err(v30::EncodeError::InvalidInstruction)
    );

    let duplicate =
        Instruction::BusLock(Box::new(Instruction::BusLock(Box::new(Instruction::Nop))));
    assert_eq!(
        encode_bytes(&duplicate),
        Err(v30::EncodeError::InvalidInstruction)
    );
}
