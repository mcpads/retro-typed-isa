use powerpc_750cl::{
    DecodeError, EncodeError, FloatingRegister, GeneralRegister, Instruction, InstructionForm,
    InstructionModifiers, Operand, OperandKind, PairedSingleWidth, QuantizationRegister,
    SpecialPurposeRegister, decode_bytes, decode_word, encode_bytes, encode_word,
};

fn gpr(index: u8) -> GeneralRegister {
    GeneralRegister::new(index).unwrap()
}

fn fpr(index: u8) -> FloatingRegister {
    FloatingRegister::new(index).unwrap()
}

#[test]
fn typed_instruction_lowers_to_big_endian_bytes() {
    let instruction = Instruction::new(
        InstructionForm::Addi,
        [
            Operand::GeneralRegister(gpr(0)),
            Operand::GeneralRegister(gpr(1)),
            Operand::SignedImmediate(0x140),
        ],
    )
    .unwrap();

    assert_eq!(encode_word(&instruction, 0x8000_0000), Ok(0x3801_0140));
    assert_eq!(
        encode_bytes(&instruction, 0x8000_0000),
        Ok([0x38, 0x01, 0x01, 0x40])
    );
    assert_eq!(
        decode_bytes(&[0x38, 0x01, 0x01, 0x40], 0x8000_0000).unwrap(),
        instruction
    );
}

#[test]
fn record_and_overflow_are_explicit_typed_modifiers() {
    let instruction = Instruction::with_modifiers(
        InstructionForm::Add,
        InstructionModifiers {
            record: true,
            overflow: true,
            ..InstructionModifiers::default()
        },
        [
            Operand::GeneralRegister(gpr(28)),
            Operand::GeneralRegister(gpr(29)),
            Operand::GeneralRegister(gpr(30)),
        ],
    )
    .unwrap();

    assert_eq!(encode_word(&instruction, 0), Ok(0x7f9d_f615));
    assert_eq!(decode_word(0x7f9d_f615, 0).unwrap(), instruction);

    assert_eq!(
        Instruction::with_modifiers(
            InstructionForm::Addi,
            InstructionModifiers {
                record: true,
                ..InstructionModifiers::default()
            },
            [
                Operand::GeneralRegister(gpr(2)),
                Operand::GeneralRegister(gpr(3)),
                Operand::SignedImmediate(4),
            ],
        ),
        Err(EncodeError::InvalidModifiers {
            form: InstructionForm::Addi,
            modifiers: InstructionModifiers {
                record: true,
                ..InstructionModifiers::default()
            },
        })
    );
}

#[test]
fn paired_single_fields_have_semantic_types() {
    let instruction = Instruction::new(
        InstructionForm::PsqL,
        [
            Operand::FloatingRegister(fpr(3)),
            Operand::SignedImmediate(-16),
            Operand::GeneralRegister(gpr(4)),
            Operand::PairedSingleWidth(PairedSingleWidth::Single),
            Operand::QuantizationRegister(QuantizationRegister::new(7).unwrap()),
        ],
    )
    .unwrap();

    assert_eq!(encode_word(&instruction, 0), Ok(0xe064_fff0));
    assert_eq!(decode_word(0xe064_fff0, 0).unwrap(), instruction);
}

#[test]
fn addis_uses_the_manuals_signed_immediate_semantics() {
    let instruction = Instruction::new(
        InstructionForm::Addis,
        [
            Operand::GeneralRegister(gpr(3)),
            Operand::GeneralRegister(gpr(4)),
            Operand::SignedImmediate(-1),
        ],
    )
    .unwrap();
    assert_eq!(encode_word(&instruction, 0), Ok(0x3c64_ffff));
    assert_eq!(decode_word(0x3c64_ffff, 0).unwrap(), instruction);

    assert_eq!(
        Instruction::new(
            InstructionForm::Addis,
            [
                Operand::GeneralRegister(gpr(3)),
                Operand::GeneralRegister(gpr(4)),
                Operand::UnsignedImmediate(0xffff),
            ],
        ),
        Err(EncodeError::OperandKind {
            index: 2,
            expected: OperandKind::SignedImmediate,
            actual: OperandKind::UnsignedImmediate,
        })
    );
}

#[test]
fn special_purpose_register_access_follows_manual_directions() {
    let read_pvr = Instruction::new(
        InstructionForm::Mfspr,
        [
            Operand::GeneralRegister(gpr(3)),
            Operand::SpecialPurposeRegister(SpecialPurposeRegister::ProcessorVersion),
        ],
    )
    .unwrap();
    assert_eq!(encode_word(&read_pvr, 0), Ok(0x7c7f_42a6));

    assert_eq!(
        Instruction::new(
            InstructionForm::Mtspr,
            [
                Operand::SpecialPurposeRegister(SpecialPurposeRegister::ProcessorVersion),
                Operand::GeneralRegister(gpr(3)),
            ],
        ),
        Err(EncodeError::InvalidSpecialPurposeAccess {
            form: InstructionForm::Mtspr,
            register: SpecialPurposeRegister::ProcessorVersion,
        })
    );

    let write_compatibility_sda = Instruction::new(
        InstructionForm::Mtspr,
        [
            Operand::SpecialPurposeRegister(SpecialPurposeRegister::SampledDataAddress),
            Operand::GeneralRegister(gpr(4)),
        ],
    )
    .unwrap();
    assert_eq!(encode_word(&write_compatibility_sda, 0), Ok(0x7c9f_eba6));
    assert_eq!(
        decode_word(0x7c9f_eba6, 0).unwrap(),
        write_compatibility_sda
    );

    assert_eq!(
        Instruction::new(
            InstructionForm::Mfspr,
            [
                Operand::GeneralRegister(gpr(4)),
                Operand::SpecialPurposeRegister(SpecialPurposeRegister::SampledDataAddress),
            ],
        ),
        Err(EncodeError::InvalidSpecialPurposeAccess {
            form: InstructionForm::Mfspr,
            register: SpecialPurposeRegister::SampledDataAddress,
        })
    );

    let unlisted_selector = 0x7c00_02a6;
    assert_eq!(
        decode_word(unlisted_selector, 0),
        Err(DecodeError::ReservedWord {
            word: unlisted_selector,
        })
    );
}

#[test]
fn manual_invalid_operand_combinations_fail_before_lowering() {
    assert_eq!(
        Instruction::new(
            InstructionForm::Lfsu,
            [
                Operand::FloatingRegister(fpr(2)),
                Operand::MemoryOffset(0),
                Operand::GeneralRegister(gpr(0)),
            ],
        ),
        Err(EncodeError::ZeroUpdateBase {
            form: InstructionForm::Lfsu,
        })
    );

    assert_eq!(
        Instruction::new(
            InstructionForm::Lwzu,
            [
                Operand::GeneralRegister(gpr(3)),
                Operand::MemoryOffset(0),
                Operand::GeneralRegister(gpr(3)),
            ],
        ),
        Err(EncodeError::UpdateBaseAliasesDestination {
            form: InstructionForm::Lwzu,
            register: gpr(3),
        })
    );

    assert_eq!(
        Instruction::new(
            InstructionForm::Lmw,
            [
                Operand::GeneralRegister(gpr(20)),
                Operand::MemoryOffset(0),
                Operand::GeneralRegister(gpr(24)),
            ],
        ),
        Err(EncodeError::LoadRangeIncludesAddress {
            form: InstructionForm::Lmw,
            register: gpr(24),
        })
    );

    assert_eq!(
        Instruction::new(
            InstructionForm::Lswi,
            [
                Operand::GeneralRegister(gpr(30)),
                Operand::GeneralRegister(gpr(0)),
                Operand::StringByteCount(powerpc_750cl::StringByteCount::new(12).unwrap()),
            ],
        ),
        Err(EncodeError::LoadRangeIncludesAddress {
            form: InstructionForm::Lswi,
            register: gpr(0),
        })
    );

    assert_eq!(
        Instruction::new(
            InstructionForm::Lswx,
            [
                Operand::GeneralRegister(gpr(4)),
                Operand::GeneralRegister(gpr(4)),
                Operand::GeneralRegister(gpr(5)),
            ],
        ),
        Err(EncodeError::StringLoadDestinationAliasesAddress { register: gpr(4) })
    );

    let decrement_count = powerpc_750cl::BranchOptions::new(0).unwrap();
    assert_eq!(
        Instruction::new(
            InstructionForm::Bcctr,
            [
                Operand::BranchOptions(decrement_count),
                Operand::ConditionRegisterBit(powerpc_750cl::ConditionRegisterBit::new(0).unwrap(),),
            ],
        ),
        Err(EncodeError::CountRegisterBranchDecrementsCount {
            options: decrement_count,
        })
    );

    let invalid_lwzu = 0x8440_0000;
    assert_eq!(
        decode_word(invalid_lwzu, 0),
        Err(DecodeError::ReservedWord { word: invalid_lwzu })
    );
}

#[test]
fn branch_targets_are_checked_at_placement() {
    let relative =
        Instruction::new(InstructionForm::B, [Operand::BranchTarget(0x8000_0100)]).unwrap();
    assert_eq!(encode_word(&relative, 0x8000_0000), Ok(0x4800_0100));
    assert_eq!(decode_word(0x4800_0100, 0x8000_0000).unwrap(), relative);

    let too_far = Instruction::new(
        InstructionForm::Bc,
        [
            Operand::BranchOptions(powerpc_750cl::BranchOptions::new(20).unwrap()),
            Operand::ConditionRegisterBit(powerpc_750cl::ConditionRegisterBit::new(0).unwrap()),
            Operand::BranchTarget(0x8001_0000),
        ],
    )
    .unwrap();
    assert_eq!(
        encode_word(&too_far, 0x8000_0000),
        Err(EncodeError::BranchOutOfRange {
            address: 0x8000_0000,
            target: 0x8001_0000,
        })
    );
}

#[test]
fn invalid_words_and_operand_shapes_fail_closed() {
    assert_eq!(
        decode_bytes(&[0x38, 0x01, 0x01], 0),
        Err(DecodeError::UnexpectedEnd { got: 3 })
    );
    assert_eq!(
        decode_word(0, 0),
        Err(DecodeError::ReservedWord { word: 0 })
    );
    assert_eq!(GeneralRegister::new(32), None);
    assert_eq!(QuantizationRegister::new(8), None);
    assert_eq!(
        Instruction::new(
            InstructionForm::Add,
            [
                Operand::FloatingRegister(fpr(0)),
                Operand::GeneralRegister(gpr(0)),
                Operand::GeneralRegister(gpr(0)),
            ],
        ),
        Err(EncodeError::OperandKind {
            index: 0,
            expected: OperandKind::GeneralRegister,
            actual: OperandKind::FloatingRegister,
        })
    );
}
