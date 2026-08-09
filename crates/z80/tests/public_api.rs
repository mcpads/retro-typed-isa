use typed_isa_core::{ControlAction, ControlTarget, StaticSemantics};
use z80::{Instruction, PROFILE_ID, Z80};

#[test]
fn profile_and_common_names_are_stable() {
    assert_eq!(PROFILE_ID, "z80");
    let semantics = Z80::semantics(&Instruction::Jr(-4), &0x1000).unwrap();
    assert_eq!(
        semantics.control_flow.action,
        ControlAction::Transfer {
            target: ControlTarget::Direct(0x0ffe)
        }
    );
    assert_eq!(Instruction::ExDEHL.to_string(), "EX DE, HL");
}
