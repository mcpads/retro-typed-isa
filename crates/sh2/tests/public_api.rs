use sh2::{Boundary, Instruction, PROFILE_ID, Register, Sh2};
use typed_isa_core::{ControlAction, ControlBoundary, ControlTarget, StaticSemantics};

#[test]
fn profile_and_common_names_are_stable() {
    assert_eq!(PROFILE_ID, "sh2");
    assert_eq!(Register::SP, Register::R15);

    let branch = Instruction::Bra { disp: 2 };
    let semantics = Sh2::semantics(&branch, &0x1000).unwrap();
    assert_eq!(
        semantics.control_flow.action,
        ControlAction::Transfer {
            target: ControlTarget::Direct(0x1008)
        }
    );
    assert_eq!(semantics.control_flow.fallthrough, None);
    assert_eq!(semantics.control_flow.delay_slot.unwrap().point, 0x1002);
    assert!(branch.has_delay_slot());

    let trap = Sh2::semantics(&Instruction::Trapa { imm: 7 }, &0).unwrap();
    assert!(matches!(
        trap.control_flow.action,
        ControlAction::Boundary(ControlBoundary::Trap {
            kind: Boundary::Trap(7),
            ..
        })
    ));
    assert_eq!(
        Instruction::MovReg {
            rm: Register::R0,
            rn: Register::R1
        }
        .to_string(),
        "MOV     R0,R1"
    );
}
