use z80::{
    ByteOperand, Condition, EncodeError, IndexRegister, Instruction, RotateShiftOperation,
    decode_bytes, encode_bytes,
};

#[test]
fn little_endian_addresses_and_index_displacements_lower_to_bytes() {
    assert_eq!(
        encode_bytes(&Instruction::Call(0x9234)).unwrap(),
        [0xCD, 0x34, 0x92]
    );
    assert_eq!(
        encode_bytes(&Instruction::LdIxDispImm(IndexRegister::Iy, -2, 0x7F)).unwrap(),
        [0xFD, 0x36, 0xFE, 0x7F]
    );
    assert_eq!(
        decode_bytes(&[0xFD, 0x36, 0xFE, 0x7F])
            .unwrap()
            .into_instruction(),
        Instruction::LdIxDispImm(IndexRegister::Iy, -2, 0x7F)
    );
}

#[test]
fn invalid_typed_operand_combinations_do_not_alias_other_opcodes() {
    assert!(matches!(
        encode_bytes(&Instruction::LdRR(
            ByteOperand::IndirectHl,
            ByteOperand::IndirectHl,
        )),
        Err(EncodeError::InvalidByteOperand { .. })
    ));
    assert!(matches!(
        encode_bytes(&Instruction::LdRIxDisp(
            ByteOperand::IndirectHl,
            IndexRegister::Ix,
            0,
        )),
        Err(EncodeError::InvalidByteOperand { .. })
    ));
    assert_eq!(
        encode_bytes(&Instruction::JrCond(Condition::Pe, 0)),
        Err(EncodeError::InvalidRelativeCondition {
            condition: Condition::Pe,
        })
    );
    assert_eq!(
        encode_bytes(&Instruction::Bit(8, ByteOperand::A)),
        Err(EncodeError::BitIndexOutOfRange { bit: 8 })
    );
    assert_eq!(
        encode_bytes(&Instruction::Rst(7)),
        Err(EncodeError::InvalidRestartTarget { target: 7 })
    );
}

#[test]
fn documented_rotate_set_has_no_sll_value() {
    let encoded: Vec<_> = RotateShiftOperation::ALL
        .into_iter()
        .map(|operation| encode_bytes(&Instruction::Rot(operation, ByteOperand::A)).unwrap())
        .collect();
    assert_eq!(encoded.len(), 7);
    assert!(!encoded.iter().any(|bytes| bytes == &[0xCB, 0x37]));
}
