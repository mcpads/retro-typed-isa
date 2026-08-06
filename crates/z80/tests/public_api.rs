use z80::{ControlFlow, Instruction, PROFILE_ID};

#[test]
fn profile_and_common_names_are_stable() {
    assert_eq!(PROFILE_ID, "z80");
    assert_eq!(
        Instruction::Jr(-4).control_flow(0x1000),
        ControlFlow::Jump { target: 0x0FFE }
    );
    assert_eq!(Instruction::ExDEHL.to_string(), "EX DE, HL");
}
