use std::collections::BTreeSet;

use arm7tdmi::{
    ArmInstructionForm, DataOperation, DecodeError, HalfwordTransferKind, StatusRegister,
    ThumbAluOperation, ThumbHighOperation, ThumbImmediateOperation, ThumbInstructionForm,
    ThumbTransferOperation, ThumbTransferWidth, TransferWidth, decode_arm_bytes,
    decode_thumb_bytes, encode_arm_bytes, encode_thumb_bytes,
};

#[test]
fn ddi_0210c_arm_summary_accounts_for_forty_nine_semantic_forms() {
    let mut expected = BTreeSet::new();
    for operation in [
        DataOperation::And,
        DataOperation::ExclusiveOr,
        DataOperation::Subtract,
        DataOperation::ReverseSubtract,
        DataOperation::Add,
        DataOperation::AddWithCarry,
        DataOperation::SubtractWithCarry,
        DataOperation::ReverseSubtractWithCarry,
        DataOperation::Test,
        DataOperation::TestEquivalence,
        DataOperation::Compare,
        DataOperation::CompareNegative,
        DataOperation::Or,
        DataOperation::Move,
        DataOperation::BitClear,
        DataOperation::MoveNot,
    ] {
        expected.insert(ArmInstructionForm::DataProcessing(operation));
    }
    expected.extend([
        ArmInstructionForm::Multiply,
        ArmInstructionForm::MultiplyAccumulate,
        ArmInstructionForm::MultiplyLong {
            signed: false,
            accumulate: false,
        },
        ArmInstructionForm::MultiplyLong {
            signed: false,
            accumulate: true,
        },
        ArmInstructionForm::MultiplyLong {
            signed: true,
            accumulate: false,
        },
        ArmInstructionForm::MultiplyLong {
            signed: true,
            accumulate: true,
        },
        ArmInstructionForm::BranchExchange,
        ArmInstructionForm::Branch,
        ArmInstructionForm::BranchLink,
        ArmInstructionForm::SingleTransfer {
            load: false,
            width: TransferWidth::Word,
        },
        ArmInstructionForm::SingleTransfer {
            load: false,
            width: TransferWidth::Byte,
        },
        ArmInstructionForm::SingleTransfer {
            load: true,
            width: TransferWidth::Word,
        },
        ArmInstructionForm::SingleTransfer {
            load: true,
            width: TransferWidth::Byte,
        },
        ArmInstructionForm::HalfwordTransfer {
            load: false,
            kind: HalfwordTransferKind::UnsignedHalfword,
        },
        ArmInstructionForm::HalfwordTransfer {
            load: true,
            kind: HalfwordTransferKind::UnsignedHalfword,
        },
        ArmInstructionForm::HalfwordTransfer {
            load: true,
            kind: HalfwordTransferKind::SignedByte,
        },
        ArmInstructionForm::HalfwordTransfer {
            load: true,
            kind: HalfwordTransferKind::SignedHalfword,
        },
        ArmInstructionForm::BlockTransfer { load: false },
        ArmInstructionForm::BlockTransfer { load: true },
        ArmInstructionForm::Swap(TransferWidth::Word),
        ArmInstructionForm::Swap(TransferWidth::Byte),
        ArmInstructionForm::MoveStatusToRegister(StatusRegister::Current),
        ArmInstructionForm::MoveStatusToRegister(StatusRegister::Saved),
        ArmInstructionForm::MoveToStatusRegister {
            destination: StatusRegister::Current,
            immediate: false,
        },
        ArmInstructionForm::MoveToStatusRegister {
            destination: StatusRegister::Saved,
            immediate: false,
        },
        ArmInstructionForm::MoveToStatusRegister {
            destination: StatusRegister::Current,
            immediate: true,
        },
        ArmInstructionForm::MoveToStatusRegister {
            destination: StatusRegister::Saved,
            immediate: true,
        },
        ArmInstructionForm::CoprocessorDataOperation,
        ArmInstructionForm::CoprocessorRegisterTransfer { load: false },
        ArmInstructionForm::CoprocessorRegisterTransfer { load: true },
        ArmInstructionForm::CoprocessorDataTransfer { load: false },
        ArmInstructionForm::CoprocessorDataTransfer { load: true },
        ArmInstructionForm::SoftwareInterrupt,
    ]);
    assert_eq!(expected.len(), 49);
}

#[test]
fn ddi_0210c_thumb_summary_accounts_for_sixty_two_semantic_forms() {
    let mut forms = BTreeSet::new();
    for kind in [
        arm7tdmi::ShiftKind::LogicalLeft,
        arm7tdmi::ShiftKind::LogicalRight,
        arm7tdmi::ShiftKind::ArithmeticRight,
    ] {
        forms.insert(ThumbInstructionForm::ShiftImmediate(kind));
    }
    for subtract in [false, true] {
        for immediate in [false, true] {
            forms.insert(ThumbInstructionForm::AddSubtract {
                subtract,
                immediate,
            });
        }
    }
    for operation in [
        ThumbImmediateOperation::Move,
        ThumbImmediateOperation::Compare,
        ThumbImmediateOperation::Add,
        ThumbImmediateOperation::Subtract,
    ] {
        forms.insert(ThumbInstructionForm::Immediate(operation));
    }
    for operation in [
        ThumbAluOperation::And,
        ThumbAluOperation::ExclusiveOr,
        ThumbAluOperation::LogicalShiftLeft,
        ThumbAluOperation::LogicalShiftRight,
        ThumbAluOperation::ArithmeticShiftRight,
        ThumbAluOperation::AddWithCarry,
        ThumbAluOperation::SubtractWithCarry,
        ThumbAluOperation::RotateRight,
        ThumbAluOperation::Test,
        ThumbAluOperation::Negate,
        ThumbAluOperation::Compare,
        ThumbAluOperation::CompareNegative,
        ThumbAluOperation::Or,
        ThumbAluOperation::Multiply,
        ThumbAluOperation::BitClear,
        ThumbAluOperation::MoveNot,
    ] {
        forms.insert(ThumbInstructionForm::Alu(operation));
    }
    for operation in [
        ThumbHighOperation::Add,
        ThumbHighOperation::Compare,
        ThumbHighOperation::Move,
    ] {
        forms.insert(ThumbInstructionForm::HighRegister(operation));
    }
    forms.extend([
        ThumbInstructionForm::BranchExchange,
        ThumbInstructionForm::PcRelativeLoad,
    ]);
    for operation in [
        ThumbTransferOperation::StoreWord,
        ThumbTransferOperation::StoreHalfword,
        ThumbTransferOperation::StoreByte,
        ThumbTransferOperation::LoadSignedByte,
        ThumbTransferOperation::LoadWord,
        ThumbTransferOperation::LoadHalfword,
        ThumbTransferOperation::LoadByte,
        ThumbTransferOperation::LoadSignedHalfword,
    ] {
        forms.insert(ThumbInstructionForm::RegisterTransfer(operation));
    }
    for load in [false, true] {
        for width in [
            ThumbTransferWidth::Word,
            ThumbTransferWidth::Halfword,
            ThumbTransferWidth::Byte,
        ] {
            forms.insert(ThumbInstructionForm::ImmediateTransfer { load, width });
        }
        forms.insert(ThumbInstructionForm::SpRelativeTransfer { load });
        forms.insert(ThumbInstructionForm::MultipleTransfer { load });
    }
    for from_sp in [false, true] {
        forms.insert(ThumbInstructionForm::LoadAddress { from_sp });
    }
    for subtract in [false, true] {
        forms.insert(ThumbInstructionForm::AddStackPointer { subtract });
    }
    for link in [false, true] {
        forms.insert(ThumbInstructionForm::Push { link });
    }
    for program_counter in [false, true] {
        forms.insert(ThumbInstructionForm::Pop { program_counter });
    }
    forms.extend([
        ThumbInstructionForm::ConditionalBranch,
        ThumbInstructionForm::UnconditionalBranch,
        ThumbInstructionForm::BranchLink,
        ThumbInstructionForm::SoftwareInterrupt,
    ]);
    assert_eq!(forms.len(), 62);
}

#[test]
fn every_thumb_selector_is_fail_closed_or_round_trips_exactly() {
    for word in 0_u16..=u16::MAX {
        let mut source = Vec::from(word.to_le_bytes());
        if word >> 11 == 0b11110 {
            source.extend_from_slice(&0xf800_u16.to_le_bytes());
        }
        match decode_thumb_bytes(&source) {
            Ok(instruction) => assert_eq!(encode_thumb_bytes(&instruction).unwrap(), source),
            Err(
                DecodeError::ReservedEncoding { .. } | DecodeError::UnpredictableEncoding { .. },
            ) => {}
            Err(error) => panic!("selector {word:#06x} produced unexpected error: {error}"),
        }
    }
}

#[test]
fn all_documented_arm_condition_selectors_round_trip_and_nv_is_reserved() {
    for selector in 0_u32..=14 {
        let word = (selector << 28) | 0x03a0_0001;
        let bytes = word.to_le_bytes();
        let instruction = decode_arm_bytes(&bytes).unwrap();
        assert_eq!(encode_arm_bytes(&instruction).unwrap(), bytes);
    }
    assert!(matches!(
        decode_arm_bytes(&0xf3a0_0001_u32.to_le_bytes()),
        Err(DecodeError::ReservedEncoding { .. })
    ));
}

#[test]
fn arm_structural_selector_lattice_is_fail_closed_or_round_trips_exactly() {
    for upper_selector in 0_u32..=0xff {
        for lower_selector in 0_u32..=0xf {
            let word = 0xe001_2304 | (upper_selector << 20) | (lower_selector << 4);
            match decode_arm_bytes(&word.to_le_bytes()) {
                Ok(instruction) => {
                    assert_eq!(encode_arm_bytes(&instruction).unwrap(), word.to_le_bytes())
                }
                Err(
                    DecodeError::ReservedEncoding { .. }
                    | DecodeError::UnpredictableEncoding { .. },
                ) => {}
                Err(error) => {
                    panic!("selector word {word:#010x} produced unexpected error: {error}")
                }
            }
        }
    }
}

#[test]
fn source_format_representatives_decode_to_each_arm_family() {
    let mut representatives = Vec::new();
    for operation in [
        DataOperation::And,
        DataOperation::ExclusiveOr,
        DataOperation::Subtract,
        DataOperation::ReverseSubtract,
        DataOperation::Add,
        DataOperation::AddWithCarry,
        DataOperation::SubtractWithCarry,
        DataOperation::ReverseSubtractWithCarry,
        DataOperation::Test,
        DataOperation::TestEquivalence,
        DataOperation::Compare,
        DataOperation::CompareNegative,
        DataOperation::Or,
        DataOperation::Move,
        DataOperation::BitClear,
        DataOperation::MoveNot,
    ] {
        let test = operation.is_test();
        let first = if operation.ignores_first_register() {
            0
        } else {
            1
        };
        representatives.push((
            0xe000_0002 | ((operation as u32) << 21) | (u32::from(test) << 20) | (first << 16),
            ArmInstructionForm::DataProcessing(operation),
        ));
    }
    representatives.extend([
        (0xe000_0291, ArmInstructionForm::Multiply),
        (0xe020_3291, ArmInstructionForm::MultiplyAccumulate),
        (
            0xe081_0392,
            ArmInstructionForm::MultiplyLong {
                signed: false,
                accumulate: false,
            },
        ),
        (
            0xe0a1_0392,
            ArmInstructionForm::MultiplyLong {
                signed: false,
                accumulate: true,
            },
        ),
        (
            0xe0c1_0392,
            ArmInstructionForm::MultiplyLong {
                signed: true,
                accumulate: false,
            },
        ),
        (
            0xe0e1_0392,
            ArmInstructionForm::MultiplyLong {
                signed: true,
                accumulate: true,
            },
        ),
        (0xe12f_ff1e, ArmInstructionForm::BranchExchange),
        (0xea00_0000, ArmInstructionForm::Branch),
        (0xeb00_0000, ArmInstructionForm::BranchLink),
        (
            0xe581_0004,
            ArmInstructionForm::SingleTransfer {
                load: false,
                width: TransferWidth::Word,
            },
        ),
        (
            0xe5c1_0004,
            ArmInstructionForm::SingleTransfer {
                load: false,
                width: TransferWidth::Byte,
            },
        ),
        (
            0xe591_0004,
            ArmInstructionForm::SingleTransfer {
                load: true,
                width: TransferWidth::Word,
            },
        ),
        (
            0xe5d1_0004,
            ArmInstructionForm::SingleTransfer {
                load: true,
                width: TransferWidth::Byte,
            },
        ),
        (
            0xe1c1_00b2,
            ArmInstructionForm::HalfwordTransfer {
                load: false,
                kind: HalfwordTransferKind::UnsignedHalfword,
            },
        ),
        (
            0xe1d1_00b2,
            ArmInstructionForm::HalfwordTransfer {
                load: true,
                kind: HalfwordTransferKind::UnsignedHalfword,
            },
        ),
        (
            0xe1d1_00d2,
            ArmInstructionForm::HalfwordTransfer {
                load: true,
                kind: HalfwordTransferKind::SignedByte,
            },
        ),
        (
            0xe1d1_00f2,
            ArmInstructionForm::HalfwordTransfer {
                load: true,
                kind: HalfwordTransferKind::SignedHalfword,
            },
        ),
        (
            0xe881_0003,
            ArmInstructionForm::BlockTransfer { load: false },
        ),
        (
            0xe891_0003,
            ArmInstructionForm::BlockTransfer { load: true },
        ),
        (0xe101_0092, ArmInstructionForm::Swap(TransferWidth::Word)),
        (0xe141_0092, ArmInstructionForm::Swap(TransferWidth::Byte)),
        (
            0xe10f_0000,
            ArmInstructionForm::MoveStatusToRegister(StatusRegister::Current),
        ),
        (
            0xe14f_0000,
            ArmInstructionForm::MoveStatusToRegister(StatusRegister::Saved),
        ),
        (
            0xe121_f000,
            ArmInstructionForm::MoveToStatusRegister {
                destination: StatusRegister::Current,
                immediate: false,
            },
        ),
        (
            0xe161_f000,
            ArmInstructionForm::MoveToStatusRegister {
                destination: StatusRegister::Saved,
                immediate: false,
            },
        ),
        (
            0xe328_f001,
            ArmInstructionForm::MoveToStatusRegister {
                destination: StatusRegister::Current,
                immediate: true,
            },
        ),
        (
            0xe368_f001,
            ArmInstructionForm::MoveToStatusRegister {
                destination: StatusRegister::Saved,
                immediate: true,
            },
        ),
        (0xee00_0100, ArmInstructionForm::CoprocessorDataOperation),
        (
            0xee01_0110,
            ArmInstructionForm::CoprocessorRegisterTransfer { load: false },
        ),
        (
            0xee11_0110,
            ArmInstructionForm::CoprocessorRegisterTransfer { load: true },
        ),
        (
            0xed81_0101,
            ArmInstructionForm::CoprocessorDataTransfer { load: false },
        ),
        (
            0xed91_0101,
            ArmInstructionForm::CoprocessorDataTransfer { load: true },
        ),
        (0xef00_0001, ArmInstructionForm::SoftwareInterrupt),
    ]);

    let mut decoded_forms = BTreeSet::new();
    for (word, expected_form) in representatives {
        let instruction = decode_arm_bytes(&word.to_le_bytes()).unwrap();
        assert_eq!(instruction.form(), expected_form, "word {word:#010x}");
        assert_eq!(encode_arm_bytes(&instruction).unwrap(), word.to_le_bytes());
        decoded_forms.insert(instruction.form());
    }
    assert_eq!(decoded_forms.len(), 49);
}

#[test]
fn source_format_representatives_decode_to_each_thumb_form() {
    let mut representatives = Vec::new();
    for (selector, kind) in [
        (0_u16, arm7tdmi::ShiftKind::LogicalLeft),
        (1, arm7tdmi::ShiftKind::LogicalRight),
        (2, arm7tdmi::ShiftKind::ArithmeticRight),
    ] {
        representatives.push((selector << 11, ThumbInstructionForm::ShiftImmediate(kind)));
    }
    for subtract in [false, true] {
        for immediate in [false, true] {
            representatives.push((
                0x1800 | (u16::from(immediate) << 10) | (u16::from(subtract) << 9),
                ThumbInstructionForm::AddSubtract {
                    subtract,
                    immediate,
                },
            ));
        }
    }
    for operation in [
        ThumbImmediateOperation::Move,
        ThumbImmediateOperation::Compare,
        ThumbImmediateOperation::Add,
        ThumbImmediateOperation::Subtract,
    ] {
        representatives.push((
            0x2000 | ((operation as u16) << 11),
            ThumbInstructionForm::Immediate(operation),
        ));
    }
    for operation in [
        ThumbAluOperation::And,
        ThumbAluOperation::ExclusiveOr,
        ThumbAluOperation::LogicalShiftLeft,
        ThumbAluOperation::LogicalShiftRight,
        ThumbAluOperation::ArithmeticShiftRight,
        ThumbAluOperation::AddWithCarry,
        ThumbAluOperation::SubtractWithCarry,
        ThumbAluOperation::RotateRight,
        ThumbAluOperation::Test,
        ThumbAluOperation::Negate,
        ThumbAluOperation::Compare,
        ThumbAluOperation::CompareNegative,
        ThumbAluOperation::Or,
        ThumbAluOperation::Multiply,
        ThumbAluOperation::BitClear,
        ThumbAluOperation::MoveNot,
    ] {
        representatives.push((
            0x4000 | ((operation as u16) << 6),
            ThumbInstructionForm::Alu(operation),
        ));
    }
    for operation in [
        ThumbHighOperation::Add,
        ThumbHighOperation::Compare,
        ThumbHighOperation::Move,
    ] {
        representatives.push((
            0x4480 | ((operation as u16) << 8),
            ThumbInstructionForm::HighRegister(operation),
        ));
    }
    representatives.extend([
        (0x4700, ThumbInstructionForm::BranchExchange),
        (0x4800, ThumbInstructionForm::PcRelativeLoad),
    ]);
    for (selector, operation) in [
        (0_u16, ThumbTransferOperation::StoreWord),
        (1, ThumbTransferOperation::StoreHalfword),
        (2, ThumbTransferOperation::StoreByte),
        (3, ThumbTransferOperation::LoadSignedByte),
        (4, ThumbTransferOperation::LoadWord),
        (5, ThumbTransferOperation::LoadHalfword),
        (6, ThumbTransferOperation::LoadByte),
        (7, ThumbTransferOperation::LoadSignedHalfword),
    ] {
        representatives.push((
            0x5000 | (selector << 9),
            ThumbInstructionForm::RegisterTransfer(operation),
        ));
    }
    for load in [false, true] {
        for (base, width) in [
            (0x6000, ThumbTransferWidth::Word),
            (0x8000, ThumbTransferWidth::Halfword),
            (0x7000, ThumbTransferWidth::Byte),
        ] {
            representatives.push((
                base | (u16::from(load) << 11),
                ThumbInstructionForm::ImmediateTransfer { load, width },
            ));
        }
        representatives.push((
            0x9000 | (u16::from(load) << 11),
            ThumbInstructionForm::SpRelativeTransfer { load },
        ));
        representatives.push((
            0xc001 | (u16::from(load) << 11),
            ThumbInstructionForm::MultipleTransfer { load },
        ));
    }
    for from_sp in [false, true] {
        representatives.push((
            0xa000 | (u16::from(from_sp) << 11),
            ThumbInstructionForm::LoadAddress { from_sp },
        ));
    }
    for subtract in [false, true] {
        representatives.push((
            0xb000 | (u16::from(subtract) << 7),
            ThumbInstructionForm::AddStackPointer { subtract },
        ));
    }
    for link in [false, true] {
        representatives.push((
            0xb401 | (u16::from(link) << 8),
            ThumbInstructionForm::Push { link },
        ));
    }
    for program_counter in [false, true] {
        representatives.push((
            0xbc01 | (u16::from(program_counter) << 8),
            ThumbInstructionForm::Pop { program_counter },
        ));
    }
    representatives.extend([
        (0xd000, ThumbInstructionForm::ConditionalBranch),
        (0xe000, ThumbInstructionForm::UnconditionalBranch),
        (0xdf00, ThumbInstructionForm::SoftwareInterrupt),
    ]);

    let mut decoded_forms = BTreeSet::new();
    for (word, expected_form) in representatives {
        let bytes = word.to_le_bytes();
        let instruction = decode_thumb_bytes(&bytes).unwrap();
        assert_eq!(instruction.form(), expected_form, "halfword {word:#06x}");
        assert_eq!(encode_thumb_bytes(&instruction).unwrap(), bytes);
        decoded_forms.insert(instruction.form());
    }
    let branch_link = decode_thumb_bytes(&[0x00, 0xf0, 0x00, 0xf8]).unwrap();
    assert_eq!(branch_link.form(), ThumbInstructionForm::BranchLink);
    assert_eq!(
        encode_thumb_bytes(&branch_link).unwrap(),
        [0x00, 0xf0, 0x00, 0xf8]
    );
    decoded_forms.insert(branch_link.form());
    assert_eq!(decoded_forms.len(), 62);
}
