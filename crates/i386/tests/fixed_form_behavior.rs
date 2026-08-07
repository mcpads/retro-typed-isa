use i386::{
    AddressSize, CodeMode, Condition, Displacement, EffectiveAddress, FarPointer, GeneralRegister,
    Immediate, MemoryOperand, MemoryWidth, Operand, Operation, Prefix, Register16, Register32,
    RelativeDisplacement, decode_bytes, encode_bytes,
};

fn assert_round_trip(bytes: &[u8], mode: CodeMode) -> i386::Instruction {
    let decoded = decode_bytes(bytes, mode).unwrap();
    assert_eq!(decoded.source_bytes(), bytes);
    assert_eq!(encode_bytes(decoded.instruction()).unwrap(), bytes);
    decoded.into_instruction()
}

#[test]
fn operand_override_changes_mov_immediate_width_without_changing_code_mode() {
    let instruction = assert_round_trip(&[0x66, 0xb8, 0x78, 0x56, 0x34, 0x12], CodeMode::Code16);
    assert_eq!(instruction.code_mode(), CodeMode::Code16);
    assert_eq!(
        instruction.prefixes().as_slice(),
        [Prefix::OperandSizeOverride]
    );
    assert_eq!(instruction.operation(), Operation::Move);
    assert_eq!(
        instruction.operands(),
        [
            Operand::GeneralRegister(GeneralRegister::Doubleword(Register32::Eax)),
            Operand::Immediate(Immediate::Doubleword(0x1234_5678)),
        ]
    );
}

#[test]
fn duplicate_overrides_preserve_source_bytes_and_keep_doubleword_semantics() {
    let instruction = assert_round_trip(
        &[0x66, 0x66, 0xb9, 0xef, 0xcd, 0xab, 0x89],
        CodeMode::Code16,
    );
    assert_eq!(
        instruction.operands()[0],
        Operand::GeneralRegister(GeneralRegister::Doubleword(Register32::Ecx))
    );
}

#[test]
fn address_override_types_a_32_bit_moffs_in_a_code16_segment() {
    let instruction = assert_round_trip(&[0x67, 0xa1, 0x78, 0x56, 0x34, 0x12], CodeMode::Code16);
    assert_eq!(
        instruction.operands(),
        [
            Operand::GeneralRegister(GeneralRegister::Word(Register16::Ax)),
            Operand::Memory(MemoryOperand::new(
                MemoryWidth::Word,
                None,
                EffectiveAddress::bits32(
                    None,
                    None,
                    Displacement::AbsoluteDoubleword(0x1234_5678),
                    false,
                )
                .unwrap(),
            )),
        ]
    );
    assert_eq!(
        match instruction.operands()[1] {
            Operand::Memory(memory) => memory.address().address_size(),
            _ => unreachable!(),
        },
        AddressSize::Bits32
    );
}

#[test]
fn code32_near_condition_uses_a_32_bit_relative_displacement() {
    let instruction = assert_round_trip(&[0x0f, 0x85, 0x78, 0x56, 0x34, 0x12], CodeMode::Code32);
    assert_eq!(
        instruction.operation(),
        Operation::ConditionalJump(Condition::NotEqual)
    );
    assert_eq!(
        instruction.operands(),
        [Operand::Relative(RelativeDisplacement::Doubleword(
            0x1234_5678
        ))]
    );
}

#[test]
fn far_call_width_follows_the_effective_operand_size() {
    let word = assert_round_trip(&[0x9a, 0x34, 0x12, 0x78, 0x56], CodeMode::Code16);
    assert_eq!(
        word.operands(),
        [Operand::FarPointer(FarPointer::Word {
            offset: 0x1234,
            segment: 0x5678,
        })]
    );

    let dword = assert_round_trip(
        &[0x66, 0x9a, 0x78, 0x56, 0x34, 0x12, 0xbc, 0x9a],
        CodeMode::Code16,
    );
    assert_eq!(
        dword.operands(),
        [Operand::FarPointer(FarPointer::Doubleword {
            offset: 0x1234_5678,
            segment: 0x9abc,
        })]
    );
}

#[test]
fn push_byte_keeps_its_sign_extended_semantic_value() {
    let instruction = assert_round_trip(&[0x6a, 0xfe], CodeMode::Code16);
    assert_eq!(instruction.operation(), Operation::Push);
    assert_eq!(
        instruction.operands(),
        [Operand::Immediate(Immediate::SignExtendedByte(-2))]
    );
}

#[test]
fn undefined_primary_opcode_fails_closed() {
    assert!(matches!(
        decode_bytes(&[0x82, 0xc0, 0], CodeMode::Code16),
        Err(i386::DecodeError::UndefinedPrimaryOpcode { opcode: 0x82 })
    ));
}

#[test]
fn modrm_register_and_memory_forms_preserve_direction_and_address_encoding() {
    let registers = assert_round_trip(&[0x89, 0xd8], CodeMode::Code16);
    assert_eq!(registers.operation(), Operation::Move);
    assert_eq!(
        registers.operands(),
        [
            Operand::GeneralRegister(GeneralRegister::Word(Register16::Ax)),
            Operand::GeneralRegister(GeneralRegister::Word(Register16::Bx)),
        ]
    );

    let sib = assert_round_trip(&[0x67, 0x01, 0x54, 0x88, 0x7f], CodeMode::Code16);
    let Operand::Memory(memory) = sib.operands()[0] else {
        panic!("destination must be memory");
    };
    assert_eq!(memory.width(), MemoryWidth::Word);
    assert_eq!(memory.address().address_size(), AddressSize::Bits32);
    assert_eq!(
        memory.address(),
        EffectiveAddress::bits32(
            Some(Register32::Eax),
            Some(i386::ScaledIndex::new(Register32::Ecx, i386::Scale::Four).unwrap()),
            Displacement::Byte(0x7f),
            true,
        )
        .unwrap()
    );
    assert_eq!(
        sib.operands()[1],
        Operand::GeneralRegister(GeneralRegister::Word(Register16::Dx))
    );
}

#[test]
fn source_defined_secondary_fields_are_not_invented_as_opcode_extensions() {
    let set_with_nonzero_reg_field = assert_round_trip(&[0x0f, 0x95, 0xf8], CodeMode::Code16);
    assert_eq!(
        set_with_nonzero_reg_field.operation(),
        Operation::SetCondition(Condition::NotEqual)
    );
    assert!(matches!(
        decode_bytes(&[0x0f, 0x24, 0xd8], CodeMode::Code16),
        Err(i386::DecodeError::InvalidOperand)
    ));
    assert_round_trip(&[0x0f, 0x24, 0xf0], CodeMode::Code16);
    assert!(matches!(
        decode_bytes(&[0x0f, 0xb7, 0xc0], CodeMode::Code16),
        Err(i386::DecodeError::InvalidOperand)
    ));
    assert_round_trip(&[0x66, 0x0f, 0xb7, 0xc0], CodeMode::Code16);
}

#[test]
fn lock_and_repeat_prefixes_follow_the_manual_instruction_lists() {
    assert!(matches!(
        decode_bytes(&[0xf0, 0x90], CodeMode::Code16),
        Err(i386::DecodeError::InvalidPrefix)
    ));
    assert!(matches!(
        decode_bytes(&[0xf3, 0x90], CodeMode::Code16),
        Err(i386::DecodeError::InvalidPrefix)
    ));
    assert!(matches!(
        decode_bytes(&[0xf2, 0xa4], CodeMode::Code16),
        Err(i386::DecodeError::InvalidPrefix)
    ));

    assert_round_trip(&[0xf0, 0x01, 0x00], CodeMode::Code16);
    assert_round_trip(&[0xf3, 0xa4], CodeMode::Code16);
    assert_round_trip(&[0xf2, 0xa6], CodeMode::Code16);
}
