use arm7tdmi::{
    Arm7TdmiArm, Arm7TdmiThumb, ArmInstruction, Condition, DataOperation, ImmediateShift,
    LowRegister, Operand2, Register, RotatedImmediate, ThumbInstruction, decode_arm_bytes,
    decode_thumb_bytes, encode_arm_bytes, encode_thumb_bytes,
};
use typed_isa_core::TypedIsa;

fn register(index: u8) -> Register {
    Register::new(index).unwrap()
}

fn low(index: u8) -> LowRegister {
    LowRegister::new(index).unwrap()
}

#[test]
fn typed_arm_instructions_lower_to_little_endian_words() {
    let move_one = ArmInstruction::DataProcessing {
        condition: Condition::Always,
        operation: DataOperation::Move,
        set_flags: false,
        destination: register(0),
        first: register(0),
        second: Operand2::Immediate(RotatedImmediate::new(1, 0).unwrap()),
    };
    assert_eq!(
        encode_arm_bytes(&move_one).unwrap(),
        [0x01, 0x00, 0xa0, 0xe3]
    );
    assert_eq!(
        decode_arm_bytes(&[0x01, 0x00, 0xa0, 0xe3]).unwrap(),
        move_one
    );

    let add_shifted = ArmInstruction::DataProcessing {
        condition: Condition::Always,
        operation: DataOperation::Add,
        set_flags: false,
        destination: register(2),
        first: register(1),
        second: Operand2::Register {
            register: register(0),
            shift: ImmediateShift::LogicalLeft(2),
        },
    };
    assert_eq!(
        encode_arm_bytes(&add_shifted).unwrap(),
        [0x00, 0x21, 0x81, 0xe0]
    );
}

#[test]
fn typed_thumb_instructions_lower_to_little_endian_halfwords() {
    let move_one = ThumbInstruction::Immediate {
        operation: arm7tdmi::ThumbImmediateOperation::Move,
        register: low(0),
        immediate: 1,
    };
    assert_eq!(encode_thumb_bytes(&move_one).unwrap(), [0x01, 0x20]);
    assert_eq!(decode_thumb_bytes(&[0x01, 0x20]).unwrap(), move_one);

    let branch_exchange = ThumbInstruction::BranchExchange {
        source: Register::LINK,
    };
    assert_eq!(encode_thumb_bytes(&branch_exchange).unwrap(), [0x70, 0x47]);
}

#[test]
fn protocol_profiles_consume_only_the_leading_instruction() {
    let arm = Arm7TdmiArm::decode_one(&[0x01, 0x00, 0xa0, 0xe3, 0xaa], 0x0800_0000).unwrap();
    assert_eq!(arm.source().as_slice(), [0x01, 0x00, 0xa0, 0xe3]);

    let thumb = Arm7TdmiThumb::decode_one(&[0x01, 0x20, 0xaa], 0x0800_0000).unwrap();
    assert_eq!(thumb.source().as_slice(), [0x01, 0x20]);
}

#[test]
fn arm_and_thumb_profile_identity_is_explicit() {
    assert_eq!(Arm7TdmiArm::PROFILE_ID, "arm7tdmi-arm-le");
    assert_eq!(Arm7TdmiThumb::PROFILE_ID, "arm7tdmi-thumb-le");
}
