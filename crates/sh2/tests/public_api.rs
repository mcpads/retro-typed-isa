use sh2::{Boundary, Instruction, PROFILE_ID, Register, Sh2};
use typed_isa_core::{ControlAction, ControlBoundary, ControlTarget, StaticSemantics, TypedIsa};

#[test]
fn protocol_decode_requires_an_even_location() {
    for location in [1, 0x1001, u32::MAX] {
        assert_eq!(
            Sh2::decode_one(&[0x00, 0x09], &location),
            Err(sh2::DecodeError::UnalignedCodeLocation { location }),
        );
    }
    for location in [0, 2, 0x1000] {
        let decoded = Sh2::decode_one(&[0x00, 0x09, 0xff], &location).unwrap();
        assert_eq!(decoded.instruction(), &Instruction::Nop);
        assert_eq!(decoded.source().as_slice(), [0x00, 0x09]);
    }
}

#[test]
fn protocol_encode_requires_an_even_location() {
    for location in [1, 0x1001, u32::MAX] {
        assert_eq!(
            Sh2::encode_one(&Instruction::Nop, &location),
            Err(sh2::EncodeError::UnalignedCodeLocation { location }),
        );
    }
    for location in [0, 2, 0x1000] {
        assert_eq!(
            Sh2::encode_one(&Instruction::Nop, &location)
                .unwrap()
                .as_slice(),
            [0x00, 0x09]
        );
    }
}

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
