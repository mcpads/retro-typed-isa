use sh4::{ControlFlow, Instruction, PROFILE_ID, Register};

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
