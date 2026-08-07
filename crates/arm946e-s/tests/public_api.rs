use arm7tdmi::{Condition, Register};
use arm946e_s::{
    Arm946EsArm, Arm946EsThumb, ArmInstruction, SaturatingOperation, ThumbInstruction,
    decode_arm_bytes, encode_arm_bytes, encode_thumb_bytes,
};
use typed_isa_core::TypedIsa;

fn register(index: u8) -> Register {
    Register::new(index).unwrap()
}

#[test]
fn typed_enhanced_dsp_instruction_lowers_to_reference_bytes() {
    let instruction = ArmInstruction::SaturatingArithmetic {
        condition: Condition::Always,
        operation: SaturatingOperation::Add,
        destination: register(0),
        first: register(1),
        second: register(2),
    };
    assert_eq!(
        encode_arm_bytes(&instruction).unwrap(),
        [0x51, 0x00, 0x02, 0xe1]
    );
    assert_eq!(
        decode_arm_bytes(&[0x51, 0x00, 0x02, 0xe1]).unwrap(),
        instruction
    );
}

#[test]
fn typed_thumb_blx_forms_lower_to_reference_bytes() {
    let register_call = ThumbInstruction::BranchLinkExchangeRegister {
        target: register(1),
    };
    assert_eq!(encode_thumb_bytes(&register_call).unwrap(), [0x88, 0x47]);

    let immediate_call = ThumbInstruction::BranchLinkExchangeImmediate { displacement: -4 };
    assert_eq!(
        encode_thumb_bytes(&immediate_call).unwrap(),
        [0xff, 0xf7, 0xfe, 0xef]
    );
}

#[test]
fn protocol_profiles_consume_only_the_leading_instruction() {
    let arm = Arm946EsArm::decode_one(&[0x51, 0x00, 0x02, 0xe1, 0xaa], 0x0200_0000).unwrap();
    assert_eq!(arm.source().as_slice(), [0x51, 0x00, 0x02, 0xe1]);

    let thumb = Arm946EsThumb::decode_one(&[0x12, 0xbe, 0xaa], 0x0200_0000).unwrap();
    assert_eq!(thumb.source().as_slice(), [0x12, 0xbe]);
}

#[test]
fn profile_identity_names_the_exact_core_and_state() {
    assert_eq!(Arm946EsArm::PROFILE_ID, "arm946e-s-arm-le");
    assert_eq!(Arm946EsThumb::PROFILE_ID, "arm946e-s-thumb-le");
}
