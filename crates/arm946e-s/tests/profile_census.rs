use std::collections::BTreeSet;

use arm946e_s::{
    ArmInstructionForm, DecodeError, HalfwordSelection, SaturatingOperation, ThumbInstructionForm,
    decode_arm_bytes, decode_thumb_bytes, encode_arm_bytes, encode_thumb_bytes,
};

#[test]
fn ddi_0100i_armv5te_table_has_thirty_four_extension_forms() {
    let mut forms = BTreeSet::from([
        ArmInstructionForm::Breakpoint,
        ArmInstructionForm::BranchLinkExchangeImmediate,
        ArmInstructionForm::BranchLinkExchangeRegister,
        ArmInstructionForm::CountLeadingZeros,
        ArmInstructionForm::DoublewordTransfer { load: false },
        ArmInstructionForm::DoublewordTransfer { load: true },
        ArmInstructionForm::CoprocessorDataOperation2,
        ArmInstructionForm::CoprocessorRegisterTransfer2 { load: false },
        ArmInstructionForm::CoprocessorRegisterTransfer2 { load: true },
        ArmInstructionForm::CoprocessorDataTransfer2 { load: false },
        ArmInstructionForm::CoprocessorDataTransfer2 { load: true },
        ArmInstructionForm::CoprocessorDoubleRegisterTransfer { load: false },
        ArmInstructionForm::CoprocessorDoubleRegisterTransfer { load: true },
        ArmInstructionForm::PreloadData,
    ]);
    for operation in [
        SaturatingOperation::Add,
        SaturatingOperation::Subtract,
        SaturatingOperation::DoubleAndAdd,
        SaturatingOperation::DoubleAndSubtract,
    ] {
        forms.insert(ArmInstructionForm::SaturatingArithmetic(operation));
    }
    for first_half in [HalfwordSelection::Bottom, HalfwordSelection::Top] {
        for second_half in [HalfwordSelection::Bottom, HalfwordSelection::Top] {
            forms.insert(ArmInstructionForm::SignedHalfwordMultiplyAccumulate {
                first_half,
                second_half,
            });
            forms.insert(ArmInstructionForm::SignedHalfwordMultiplyAccumulateLong {
                first_half,
                second_half,
            });
            forms.insert(ArmInstructionForm::SignedHalfwordMultiply {
                first_half,
                second_half,
            });
        }
    }
    for half in [HalfwordSelection::Bottom, HalfwordSelection::Top] {
        forms.insert(ArmInstructionForm::SignedWordHalfwordMultiplyAccumulate(
            half,
        ));
        forms.insert(ArmInstructionForm::SignedWordHalfwordMultiply(half));
    }
    assert_eq!(forms.len(), 34);
}

#[test]
fn ddi_0100i_fixed_vectors_reach_every_armv5te_extension_form() {
    let vectors: [(u32, ArmInstructionForm); 34] = [
        (0xe121_2374, ArmInstructionForm::Breakpoint),
        (0xfaff_fffe, ArmInstructionForm::BranchLinkExchangeImmediate),
        (0xe12f_ff31, ArmInstructionForm::BranchLinkExchangeRegister),
        (0xe16f_0f11, ArmInstructionForm::CountLeadingZeros),
        (
            0xe1c2_00d4,
            ArmInstructionForm::DoublewordTransfer { load: true },
        ),
        (
            0xe1c2_00f4,
            ArmInstructionForm::DoublewordTransfer { load: false },
        ),
        (0xfe00_0100, ArmInstructionForm::CoprocessorDataOperation2),
        (
            0xfe01_0110,
            ArmInstructionForm::CoprocessorRegisterTransfer2 { load: false },
        ),
        (
            0xfe11_0110,
            ArmInstructionForm::CoprocessorRegisterTransfer2 { load: true },
        ),
        (
            0xfd81_0101,
            ArmInstructionForm::CoprocessorDataTransfer2 { load: false },
        ),
        (
            0xfd91_0101,
            ArmInstructionForm::CoprocessorDataTransfer2 { load: true },
        ),
        (
            0xec41_0123,
            ArmInstructionForm::CoprocessorDoubleRegisterTransfer { load: false },
        ),
        (
            0xec51_0123,
            ArmInstructionForm::CoprocessorDoubleRegisterTransfer { load: true },
        ),
        (0xf5d2_f004, ArmInstructionForm::PreloadData),
        (
            0xe102_0051,
            ArmInstructionForm::SaturatingArithmetic(SaturatingOperation::Add),
        ),
        (
            0xe122_0051,
            ArmInstructionForm::SaturatingArithmetic(SaturatingOperation::Subtract),
        ),
        (
            0xe142_0051,
            ArmInstructionForm::SaturatingArithmetic(SaturatingOperation::DoubleAndAdd),
        ),
        (
            0xe162_0051,
            ArmInstructionForm::SaturatingArithmetic(SaturatingOperation::DoubleAndSubtract),
        ),
        (
            0xe100_3281,
            ArmInstructionForm::SignedHalfwordMultiplyAccumulate {
                first_half: HalfwordSelection::Bottom,
                second_half: HalfwordSelection::Bottom,
            },
        ),
        (
            0xe100_32c1,
            ArmInstructionForm::SignedHalfwordMultiplyAccumulate {
                first_half: HalfwordSelection::Bottom,
                second_half: HalfwordSelection::Top,
            },
        ),
        (
            0xe100_32a1,
            ArmInstructionForm::SignedHalfwordMultiplyAccumulate {
                first_half: HalfwordSelection::Top,
                second_half: HalfwordSelection::Bottom,
            },
        ),
        (
            0xe100_32e1,
            ArmInstructionForm::SignedHalfwordMultiplyAccumulate {
                first_half: HalfwordSelection::Top,
                second_half: HalfwordSelection::Top,
            },
        ),
        (
            0xe141_0382,
            ArmInstructionForm::SignedHalfwordMultiplyAccumulateLong {
                first_half: HalfwordSelection::Bottom,
                second_half: HalfwordSelection::Bottom,
            },
        ),
        (
            0xe141_03c2,
            ArmInstructionForm::SignedHalfwordMultiplyAccumulateLong {
                first_half: HalfwordSelection::Bottom,
                second_half: HalfwordSelection::Top,
            },
        ),
        (
            0xe141_03a2,
            ArmInstructionForm::SignedHalfwordMultiplyAccumulateLong {
                first_half: HalfwordSelection::Top,
                second_half: HalfwordSelection::Bottom,
            },
        ),
        (
            0xe141_03e2,
            ArmInstructionForm::SignedHalfwordMultiplyAccumulateLong {
                first_half: HalfwordSelection::Top,
                second_half: HalfwordSelection::Top,
            },
        ),
        (
            0xe120_3281,
            ArmInstructionForm::SignedWordHalfwordMultiplyAccumulate(HalfwordSelection::Bottom),
        ),
        (
            0xe120_32c1,
            ArmInstructionForm::SignedWordHalfwordMultiplyAccumulate(HalfwordSelection::Top),
        ),
        (
            0xe160_0281,
            ArmInstructionForm::SignedHalfwordMultiply {
                first_half: HalfwordSelection::Bottom,
                second_half: HalfwordSelection::Bottom,
            },
        ),
        (
            0xe160_02c1,
            ArmInstructionForm::SignedHalfwordMultiply {
                first_half: HalfwordSelection::Bottom,
                second_half: HalfwordSelection::Top,
            },
        ),
        (
            0xe160_02a1,
            ArmInstructionForm::SignedHalfwordMultiply {
                first_half: HalfwordSelection::Top,
                second_half: HalfwordSelection::Bottom,
            },
        ),
        (
            0xe160_02e1,
            ArmInstructionForm::SignedHalfwordMultiply {
                first_half: HalfwordSelection::Top,
                second_half: HalfwordSelection::Top,
            },
        ),
        (
            0xe120_02a1,
            ArmInstructionForm::SignedWordHalfwordMultiply(HalfwordSelection::Bottom),
        ),
        (
            0xe120_02e1,
            ArmInstructionForm::SignedWordHalfwordMultiply(HalfwordSelection::Top),
        ),
    ];
    let mut forms = BTreeSet::new();
    for (word, expected_form) in vectors {
        let instruction = decode_arm_bytes(&word.to_le_bytes()).unwrap();
        assert_eq!(instruction.form(), expected_form, "word {word:#010x}");
        assert_eq!(encode_arm_bytes(&instruction).unwrap(), word.to_le_bytes());
        forms.insert(instruction.form());
    }
    assert_eq!(forms.len(), 34);
}

#[test]
fn thumb_v5_additions_are_independent_of_the_armv4t_denominator() {
    let vectors: [(&[u8], ThumbInstructionForm); 3] = [
        (&[0x12, 0xbe], ThumbInstructionForm::Breakpoint),
        (
            &[0x88, 0x47],
            ThumbInstructionForm::BranchLinkExchangeRegister,
        ),
        (
            &[0xff, 0xf7, 0xfe, 0xef],
            ThumbInstructionForm::BranchLinkExchangeImmediate,
        ),
    ];
    for (bytes, expected_form) in vectors {
        let instruction = decode_thumb_bytes(bytes).unwrap();
        assert_eq!(instruction.form(), expected_form);
        assert_eq!(encode_thumb_bytes(&instruction).unwrap(), bytes);
    }
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
                DecodeError::ReservedEncoding { .. }
                | DecodeError::UnpredictableEncoding { .. }
                | DecodeError::Armv4T(
                    arm7tdmi::DecodeError::ReservedEncoding { .. }
                    | arm7tdmi::DecodeError::UnpredictableEncoding { .. },
                ),
            ) => {}
            Err(error) => panic!("selector {word:#06x} produced unexpected error: {error}"),
        }
    }

    for prefix in 0xf000_u16..=0xf7ff {
        let source = [prefix.to_le_bytes()[0], prefix.to_le_bytes()[1], 0x00, 0xe8];
        let instruction = decode_thumb_bytes(&source).unwrap();
        assert_eq!(encode_thumb_bytes(&instruction).unwrap(), source);
    }
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
                    | DecodeError::UnpredictableEncoding { .. }
                    | DecodeError::Armv4T(
                        arm7tdmi::DecodeError::ReservedEncoding { .. }
                        | arm7tdmi::DecodeError::UnpredictableEncoding { .. },
                    ),
                ) => {}
                Err(error) => {
                    panic!("selector word {word:#010x} produced unexpected error: {error}")
                }
            }
        }
    }
}
