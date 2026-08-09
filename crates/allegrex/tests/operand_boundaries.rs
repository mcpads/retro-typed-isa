use allegrex::{
    BitFieldOperation, DecodeError, DestinationPrefixLane, DestinationSaturation, EncodeError,
    FloatingRegister, GeneralRegister, Immediate5, Immediate8, Immediate16, Instruction,
    MatrixRegister, SourcePrefixConstant, SourcePrefixLane, SourcePrefixValue,
    SystemControlRegister, VectorComponent, VectorRegister, VectorSize, VfpuConditionBit,
    VfpuConstant, VfpuConstructionError, VfpuControlRegister, VfpuForm, VfpuInstruction,
    VfpuMemoryOffset, VfpuOperands, VfpuOperation, decode_word, encode_bytes, encode_word,
};

const ADDRESS: allegrex::CodeLocation = 0x0880_0000;

#[test]
fn public_field_types_reject_values_outside_the_manual_bit_widths() {
    assert_eq!(GeneralRegister::new(31).unwrap().index(), 31);
    assert!(GeneralRegister::new(32).is_none());
    assert_eq!(SystemControlRegister::new(31).unwrap().index(), 31);
    assert!(SystemControlRegister::new(32).is_none());
    assert_eq!(FloatingRegister::new(31).unwrap().index(), 31);
    assert!(FloatingRegister::new(32).is_none());

    assert_eq!(VectorRegister::new(127).unwrap().index(), 127);
    assert!(VectorRegister::new(128).is_none());
    assert_eq!(MatrixRegister::new(127).unwrap().index(), 127);
    assert!(MatrixRegister::new(128).is_none());
    assert_eq!(VfpuConditionBit::new(7).unwrap().index(), 7);
    assert!(VfpuConditionBit::new(8).is_none());

    assert!(VfpuConstant::new(0).is_none());
    assert_eq!(VfpuConstant::new(1).unwrap().code(), 1);
    assert_eq!(VfpuConstant::new(19).unwrap().code(), 19);
    assert!(VfpuConstant::new(20).is_none());
    assert_eq!(Immediate5::new(0x1f).unwrap().value(), 0x1f);
    assert!(Immediate5::new(0x20).is_none());
    assert_eq!(Immediate8::new(0xff).unwrap().value(), 0xff);
    assert!(Immediate8::new(0x100).is_none());
    assert_eq!(Immediate16::new(0xffff).unwrap().value(), 0xffff);
    assert!(Immediate16::new(0x1_0000).is_none());

    assert_eq!(VfpuMemoryOffset::new(i16::MIN).unwrap().value(), i16::MIN);
    assert_eq!(VfpuMemoryOffset::new(32_764).unwrap().value(), 32_764);
    assert!(VfpuMemoryOffset::new(-2).is_none());
    assert!(VfpuMemoryOffset::new(i16::MAX).is_none());
}

#[test]
fn semantic_cpu_and_vfpu_instructions_lower_to_little_endian_bytes() {
    let add = Instruction::ThreeRegister {
        operation: allegrex::ThreeRegisterOperation::AddUnsigned,
        destination: GeneralRegister::T0,
        left: GeneralRegister::T1,
        right: GeneralRegister::T2,
    };
    assert_eq!(
        encode_bytes(&add, ADDRESS).unwrap(),
        [0x21, 0x40, 0x2a, 0x01]
    );

    let form = VfpuForm::new(VfpuOperation::Add, Some(VectorSize::Quad)).unwrap();
    let zero = VectorRegister::new(0).unwrap();
    let add_vectors = Instruction::Vfpu(
        VfpuInstruction::new(
            form,
            VfpuOperands::ThreeVectors {
                destination: zero,
                left: zero,
                right: zero,
            },
        )
        .unwrap(),
    );
    assert_eq!(
        encode_bytes(&add_vectors, ADDRESS).unwrap(),
        [0x80, 0x80, 0x00, 0x60]
    );
}

#[test]
fn prefix_fields_round_trip_as_semantic_lanes_and_reject_reserved_bits() {
    let source_lanes = [
        SourcePrefixLane {
            value: SourcePrefixValue::Component {
                component: VectorComponent::X,
                absolute: false,
            },
            negate: false,
        },
        SourcePrefixLane {
            value: SourcePrefixValue::Component {
                component: VectorComponent::Z,
                absolute: true,
            },
            negate: true,
        },
        SourcePrefixLane {
            value: SourcePrefixValue::Constant(SourcePrefixConstant::One),
            negate: false,
        },
        SourcePrefixLane {
            value: SourcePrefixValue::Constant(SourcePrefixConstant::OneSixth),
            negate: true,
        },
    ];
    let source_prefix = vfpu_instruction(
        VfpuOperation::SourcePrefix,
        VfpuOperands::SourcePrefix {
            lanes: source_lanes,
        },
    );
    assert_eq!(encode_word(&source_prefix, ADDRESS).unwrap(), 0xdc0a_cad8);
    assert_eq!(decode_word(0xdc0a_cad8, ADDRESS).unwrap(), source_prefix);

    let destination_lanes = [
        DestinationPrefixLane {
            saturation: DestinationSaturation::None,
            mask: false,
        },
        DestinationPrefixLane {
            saturation: DestinationSaturation::ZeroToOne,
            mask: true,
        },
        DestinationPrefixLane {
            saturation: DestinationSaturation::NegativeOneToOne,
            mask: false,
        },
        DestinationPrefixLane {
            saturation: DestinationSaturation::None,
            mask: true,
        },
    ];
    let destination_prefix = vfpu_instruction(
        VfpuOperation::DestinationPrefix,
        VfpuOperands::DestinationPrefix {
            lanes: destination_lanes,
        },
    );
    assert_eq!(
        encode_word(&destination_prefix, ADDRESS).unwrap(),
        0xde00_0a34
    );
    assert_eq!(
        decode_word(0xde00_0a34, ADDRESS).unwrap(),
        destination_prefix
    );

    for word in [
        0xdc10_0000, // VPFXS upper field is reserved.
        0xdd10_0000, // VPFXT upper field is reserved.
        0xde00_1000, // VPFXD upper field is reserved.
        0xde00_0002, // Destination saturation selector two is reserved.
    ] {
        assert!(matches!(
            decode_word(word, ADDRESS),
            Err(DecodeError::ReservedWord { .. })
        ));
    }
}

#[test]
fn vfpu_control_and_memory_forms_enforce_architecture_restrictions() {
    assert!(VfpuControlRegister::new(133).is_none());
    assert!(VfpuControlRegister::new(134).is_none());
    assert!(!VfpuControlRegister::Revision.is_writable());

    let write_control = VfpuForm::new(VfpuOperation::MoveToControl, None).unwrap();
    assert!(matches!(
        VfpuInstruction::new(
            write_control,
            VfpuOperands::GeneralAndControl {
                general: GeneralRegister::T0,
                control: VfpuControlRegister::Revision,
            },
        ),
        Err(VfpuConstructionError::ReadOnlyControlRegister { .. })
    ));

    let load_quad = Instruction::Vfpu(
        VfpuInstruction::new(
            VfpuForm::new(VfpuOperation::Load, Some(VectorSize::Quad)).unwrap(),
            VfpuOperands::MemoryVector {
                vector: VectorRegister::new(64).unwrap(),
                base: GeneralRegister::SP,
                offset: VfpuMemoryOffset::new(0).unwrap(),
            },
        )
        .unwrap(),
    );
    assert!(matches!(
        encode_word(&load_quad, ADDRESS),
        Err(EncodeError::FieldOutOfRange {
            field: "quad VFPU register",
            value: 64,
            maximum: 63,
        })
    ));
}

#[test]
fn core_encoder_rejects_unrepresentable_operands_and_targets() {
    let bit_field = |position, size| Instruction::BitField {
        operation: BitFieldOperation::Extract,
        destination: GeneralRegister::T0,
        source: GeneralRegister::T1,
        position,
        size,
    };
    assert!(encode_word(&bit_field(31, 1), ADDRESS).is_ok());
    assert!(matches!(
        encode_word(&bit_field(0, 0), ADDRESS),
        Err(EncodeError::InvalidInstructionForm { .. })
    ));
    assert!(matches!(
        encode_word(&bit_field(31, 2), ADDRESS),
        Err(EncodeError::InvalidInstructionForm { .. })
    ));
    assert!(matches!(
        encode_word(&bit_field(32, 1), ADDRESS),
        Err(EncodeError::FieldOutOfRange { .. })
    ));

    let branch = |target| Instruction::EqualityBranch {
        condition: allegrex::EqualityCondition::Equal,
        likely: false,
        left: GeneralRegister::ZERO,
        right: GeneralRegister::ZERO,
        target,
    };
    assert!(encode_word(&branch(0x0882_0000), ADDRESS).is_ok());
    assert!(matches!(
        encode_word(&branch(0x0882_0004), ADDRESS),
        Err(EncodeError::BranchOutOfRange { .. })
    ));
    assert!(matches!(
        encode_word(&branch(0x0880_0002), ADDRESS),
        Err(EncodeError::UnalignedTarget { .. })
    ));

    let jump = |target| Instruction::Jump {
        link: false,
        target,
    };
    assert!(encode_word(&jump(0x0fff_fffc), ADDRESS).is_ok());
    assert!(matches!(
        encode_word(&jump(0x1000_0000), ADDRESS),
        Err(EncodeError::JumpRegionMismatch { .. })
    ));
}

fn vfpu_instruction(operation: VfpuOperation, operands: VfpuOperands) -> Instruction {
    Instruction::Vfpu(
        VfpuInstruction::new(VfpuForm::new(operation, None).unwrap(), operands).unwrap(),
    )
}
