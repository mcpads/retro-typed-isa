use typed_isa_core::TypedIsa;
use vr4300::{
    DecodeError, EncodeError, FloatingBinaryOperation, FloatingFormat, FloatingRegister,
    GeneralRegister, ImmediateArithmeticOperation, Instruction, ShiftOperation, Vr4300,
    decode_bytes, decode_word, encode_bytes, encode_word,
};

#[test]
fn typed_integer_instruction_lowers_to_big_endian_bytes() {
    let instruction = Instruction::ImmediateArithmetic {
        operation: ImmediateArithmeticOperation::AddUnsigned,
        destination: GeneralRegister::SP,
        source: GeneralRegister::SP,
        immediate: -0x740,
    };

    assert_eq!(encode_word(&instruction, 0x8000_0000).unwrap(), 0x27bd_f8c0);
    assert_eq!(
        encode_bytes(&instruction, 0x8000_0000).unwrap(),
        [0x27, 0xbd, 0xf8, 0xc0]
    );
    assert_eq!(
        decode_bytes(&[0x27, 0xbd, 0xf8, 0xc0], 0x8000_0000).unwrap(),
        instruction
    );
}

#[test]
fn fixed_fpu_vector_uses_the_manual_field_order() {
    let instruction = Instruction::FloatingBinary {
        operation: FloatingBinaryOperation::Add,
        format: FloatingFormat::Single,
        destination: FloatingRegister::new(0).unwrap(),
        left: FloatingRegister::new(2).unwrap(),
        right: FloatingRegister::new(4).unwrap(),
    };

    assert_eq!(encode_word(&instruction, 0x8000_0000).unwrap(), 0x4604_1000);
    assert_eq!(decode_word(0x4604_1000, 0x8000_0000).unwrap(), instruction);
}

#[test]
fn control_flow_targets_are_address_aware() {
    let jump = Instruction::Jump {
        link: false,
        target: 0x8000_1000,
    };
    assert_eq!(encode_word(&jump, 0x8000_0000).unwrap(), 0x0800_0400);
    assert_eq!(decode_word(0x0800_0400, 0x8000_0000).unwrap(), jump);

    let branch = Instruction::EqualityBranch {
        condition: vr4300::EqualityCondition::NotEqual,
        likely: false,
        left: GeneralRegister::T1,
        right: GeneralRegister::ZERO,
        target: 0x7fff_fff0,
    };
    assert_eq!(encode_word(&branch, 0x8000_0000).unwrap(), 0x1520_fffb);
    assert_eq!(decode_word(0x1520_fffb, 0x8000_0000).unwrap(), branch);
}

#[test]
fn reserved_fields_and_undefined_cp2_operations_fail_closed() {
    assert_eq!(
        decode_bytes(&[0; 3], 0x8000_0000),
        Err(DecodeError::UnexpectedEnd { got: 3 })
    );
    assert_eq!(
        decode_word(0x4c00_0000, 0x8000_0000),
        Err(DecodeError::ReservedWord { word: 0x4c00_0000 })
    );
    assert_eq!(
        decode_word(0x0020_0000, 0x8000_0000),
        Err(DecodeError::ReservedWord { word: 0x0020_0000 })
    );
    assert_eq!(
        decode_word(0x4a00_0000, 0x8000_0000),
        Err(DecodeError::ReservedWord { word: 0x4a00_0000 })
    );

    let word_shift = Instruction::ShiftImmediate {
        operation: ShiftOperation::LeftLogical,
        destination: GeneralRegister::T0,
        source: GeneralRegister::T1,
        amount: 32,
    };
    assert_eq!(
        encode_word(&word_shift, 0x8000_0000),
        Err(EncodeError::ShiftAmountOutOfRange {
            amount: 32,
            maximum: 5
        })
    );
}

#[test]
fn protocol_decodes_only_the_leading_instruction() {
    let decoded = Vr4300::decode_one(&[0x27, 0xbd, 0xf8, 0xc0, 0xaa], 0x8000_0000).unwrap();
    assert_eq!(decoded.source().as_slice(), [0x27, 0xbd, 0xf8, 0xc0]);
    assert_eq!(
        Vr4300::encode_one(decoded.instruction(), 0x8000_0000)
            .unwrap()
            .as_slice(),
        decoded.source().as_slice()
    );
}

#[test]
fn jump_region_mismatch_is_rejected() {
    let jump = Instruction::Jump {
        link: false,
        target: 0x9000_0000,
    };
    assert_eq!(
        encode_word(&jump, 0x8000_0000),
        Err(EncodeError::JumpRegionMismatch {
            address: 0x8000_0000,
            target: 0x9000_0000,
        })
    );
}
