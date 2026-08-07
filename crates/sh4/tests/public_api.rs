use sh4::{ControlFlow, FloatRegister, Instruction, PROFILE_ID, Register};
use typed_isa_core::TypedIsa;

#[test]
fn profile_and_common_names_are_stable() {
    assert_eq!(PROFILE_ID, "sh4");
    assert_eq!(Register::SP, Register::R15);

    let branch = Instruction::Bra { disp: 2 };
    assert_eq!(
        branch.control_flow(0x1000),
        ControlFlow::Branch {
            target: 0x1008,
            fallthrough: None,
            delay_slot: Some(0x1002),
        }
    );
    assert_eq!(
        Instruction::MovReg {
            rm: Register::R0,
            rn: Register::R1,
        }
        .to_string(),
        "MOV     R0,R1"
    );
}

#[test]
fn shared_protocol_keeps_fpscr_decode_state_in_the_profile() {
    let instruction = Instruction::Fadd {
        frm: FloatRegister::new(0).unwrap(),
        frn: FloatRegister::new(1).unwrap(),
    };

    let encoded = sh4::Sh4Pr0Sz0::encode_one(&instruction, 0).unwrap();
    assert_eq!(
        sh4::Sh4Pr0Sz0::decode_one(encoded.as_slice(), 0)
            .unwrap()
            .instruction(),
        &instruction
    );
    assert!(sh4::Sh4Pr1Sz0::encode_one(&instruction, 0).is_err());
}
