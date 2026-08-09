use r5900::{
    DecodeError, GeneralRegister, Instruction, InstructionForm, Operands, VectorControlRegister,
    VectorDestinationMask, VectorFloatingRegister, VectorImmediate5, VectorIntegerRegister,
    VectorInterlock, VectorLane, VectorMicroAddress, decode_word, encode_bytes, encode_word,
};

const ADDRESS: r5900::CodeLocation = 0x0010_0000;

#[test]
fn public_vector_fields_enforce_the_vu_manual_ranges() {
    assert_eq!(VectorFloatingRegister::new(31).unwrap().index(), 31);
    assert!(VectorFloatingRegister::new(32).is_none());
    assert_eq!(VectorIntegerRegister::new(15).unwrap().index(), 15);
    assert!(VectorIntegerRegister::new(16).is_none());
    assert_eq!(VectorDestinationMask::new(0x0f).unwrap().bits(), 0x0f);
    assert!(VectorDestinationMask::new(0x10).is_none());

    assert_eq!(VectorImmediate5::new(-16).unwrap().value(), -16);
    assert_eq!(VectorImmediate5::new(15).unwrap().value(), 15);
    assert!(VectorImmediate5::new(-17).is_none());
    assert!(VectorImmediate5::new(16).is_none());

    assert_eq!(
        VectorMicroAddress::new(0x0003_fff8).unwrap().byte_address(),
        0x0003_fff8
    );
    assert!(VectorMicroAddress::new(2).is_none());
    assert!(VectorMicroAddress::new(0x0004_0000).is_none());

    for index in [19, 23, 24, 25, 30, 32] {
        assert!(VectorControlRegister::new(index).is_none(), "CCR {index}");
    }
    assert_eq!(VectorControlRegister::CALL_ADDRESS_ONE.index(), 31);
}

#[test]
fn typed_vu_operands_lower_to_the_manual_words_and_little_endian_bytes() {
    let vector_add = Instruction::new(
        InstructionForm::Vadd,
        Operands::VectorThree {
            destination_mask: VectorDestinationMask::XYZW,
            destination: VectorFloatingRegister::new(3).unwrap(),
            left: VectorFloatingRegister::new(2).unwrap(),
            right: VectorFloatingRegister::new(1).unwrap(),
        },
    )
    .unwrap();
    assert_eq!(encode_word(&vector_add, ADDRESS).unwrap(), 0x4be1_10e8);
    assert_eq!(
        encode_bytes(&vector_add, ADDRESS).unwrap(),
        [0xe8, 0x10, 0xe1, 0x4b]
    );
    assert_eq!(decode_word(0x4be1_10e8, ADDRESS).unwrap(), vector_add);

    let add_immediate = Instruction::new(
        InstructionForm::VaddI,
        Operands::VectorTwo {
            destination_mask: VectorDestinationMask::XYZW,
            destination: VectorFloatingRegister::new(3).unwrap(),
            source: VectorFloatingRegister::new(2).unwrap(),
        },
    )
    .unwrap();
    assert_eq!(encode_word(&add_immediate, ADDRESS).unwrap(), 0x4be0_10e2);
    assert_eq!(decode_word(0x4be0_10e2, ADDRESS).unwrap(), add_immediate);

    let transfer = Instruction::new(
        InstructionForm::Qmfc2,
        Operands::VectorFloatingTransfer {
            general: GeneralRegister::T0,
            vector: VectorFloatingRegister::new(31).unwrap(),
            interlock: VectorInterlock::With,
        },
    )
    .unwrap();
    assert_eq!(encode_word(&transfer, ADDRESS).unwrap(), 0x4828_f801);
    assert_eq!(decode_word(0x4828_f801, ADDRESS).unwrap(), transfer);

    let integer_add = Instruction::new(
        InstructionForm::Viaddi,
        Operands::VectorIntegerImmediate {
            destination: VectorIntegerRegister::new(15).unwrap(),
            source: VectorIntegerRegister::new(14).unwrap(),
            immediate: VectorImmediate5::new(-16).unwrap(),
        },
    )
    .unwrap();
    assert_eq!(encode_word(&integer_add, ADDRESS).unwrap(), 0x4a0f_7432);
    assert_eq!(decode_word(0x4a0f_7432, ADDRESS).unwrap(), integer_add);

    let call = Instruction::new(
        InstructionForm::Vcallms,
        Operands::VectorMicroCall {
            target: VectorMicroAddress::new(0x0003_fff8).unwrap(),
        },
    )
    .unwrap();
    assert_eq!(encode_word(&call, ADDRESS).unwrap(), 0x4a1f_fff8);
    assert_eq!(decode_word(0x4a1f_fff8, ADDRESS).unwrap(), call);

    let square_root = Instruction::new(
        InstructionForm::Vsqrt,
        Operands::VectorFieldSource {
            source: VectorFloatingRegister::new(31).unwrap(),
            lane: VectorLane::W,
        },
    )
    .unwrap();
    assert_eq!(encode_word(&square_root, ADDRESS).unwrap(), 0x4b9f_03bd);
    assert_eq!(decode_word(0x4b9f_03bd, ADDRESS).unwrap(), square_root);
}

#[test]
fn reserved_vu_registers_and_undefined_field_combinations_fail_closed() {
    for word in [
        0x4840_9800, // CFC2 reserved CCR19.
        0x48c0_b800, // CTC2 reserved CCR23.
        0x4a10_0030, // VIADD with out-of-range VI[it] 16.
        0x4a00_8030, // VIADD with out-of-range VI[is] 16.
        0x4a00_0430, // VIADD with out-of-range VI[id] 16.
        0x4a00_03fe, // VILWR requires exactly one destination lane.
        0x4bc0_03fe, // VILWR cannot select multiple lanes.
    ] {
        assert!(matches!(
            decode_word(word, ADDRESS),
            Err(DecodeError::ReservedWord { .. })
        ));
    }

    let lane_load = Instruction::new(
        InstructionForm::Vilwr,
        Operands::VectorIntegerLoad {
            lane: VectorLane::W,
            destination: VectorIntegerRegister::new(1).unwrap(),
            base: VectorIntegerRegister::new(2).unwrap(),
        },
    )
    .unwrap();
    let word = encode_word(&lane_load, ADDRESS).unwrap();
    assert_eq!((word >> 21) & 0x0f, 1);
    assert_eq!(decode_word(word, ADDRESS).unwrap(), lane_load);
}
