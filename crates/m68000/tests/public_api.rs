use m68000::{
    AnalysisPoint, Assembler, DecodedInstruction, Instruction, InstructionForm, Mc68000,
    PROFILE_ID, PrivilegeMode, decode_bytes, encode_bytes, verify_placed_program,
};
use typed_isa_core::{ControlAction, StaticSemantics, TypedIsa};

#[test]
fn protocol_decode_requires_an_even_logical_location() {
    for address in [1, 0x1001, 0xab00_1001, u32::MAX] {
        assert_eq!(
            Mc68000::decode_one(&[0x4e, 0x71], &m68000::CodeLocation::new(address)),
            Err(m68000::DecodeError::UnalignedCodeLocation { location: address }),
        );
    }
    for address in [0, 2, 0x1000, 0xab00_1000] {
        let decoded =
            Mc68000::decode_one(&[0x4e, 0x71, 0xff], &m68000::CodeLocation::new(address)).unwrap();
        assert_eq!(decoded.instruction(), &Instruction::Nop);
        assert_eq!(decoded.source().as_slice(), [0x4e, 0x71]);
    }
}

#[test]
fn protocol_encode_requires_an_even_logical_location() {
    for address in [1, 0x1001, 0xab00_1001, u32::MAX] {
        assert_eq!(
            Mc68000::encode_one(&Instruction::Nop, &m68000::CodeLocation::new(address)),
            Err(m68000::EncodeError::UnalignedCodeLocation { location: address }),
        );
    }
    for address in [0, 2, 0x1000, 0xab00_1000] {
        assert_eq!(
            Mc68000::encode_one(&Instruction::Nop, &m68000::CodeLocation::new(address))
                .unwrap()
                .as_slice(),
            [0x4e, 0x71]
        );
    }
}

#[test]
fn profile_and_common_names_are_stable() {
    assert_eq!(PROFILE_ID, "m68000");
    assert_eq!(InstructionForm::all().len(), 1_365);

    let mut assembler = Assembler::new();
    assembler.emit(Instruction::Nop);
    let origin = m68000::CodeLocation::new(0x1000);
    assert_eq!(
        m68000::CodeLocation::new(0xab10_0000).bus_address(),
        0x0010_0000
    );
    let program = assembler.assemble(origin).unwrap();
    assert_eq!(program.origin(), origin);
    assert_eq!(program.bytes(), [0x4e, 0x71]);
    assert_eq!(program.instructions().len(), 1);
    assert_eq!(program.instructions()[0].location, origin);
    assert_eq!(program.instruction_spans()[0].offset, 0);
    assert_eq!(
        program.instruction_spans()[0].bytes.as_slice(),
        [0x4e, 0x71]
    );
    assert_eq!(program.clone().into_bytes(), [0x4e, 0x71]);

    let DecodedInstruction {
        instruction,
        byte_len,
    } = decode_bytes(program.bytes()).unwrap();
    assert_eq!(instruction, Instruction::Nop);
    assert_eq!(byte_len, 2);
    assert_eq!(encode_bytes(&instruction).unwrap(), program.bytes());
    assert_eq!(
        verify_placed_program(program.bytes(), origin)
            .unwrap()
            .len(),
        1
    );
    let point = AnalysisPoint::new(m68000::CodeLocation::new(0x1000), PrivilegeMode::Supervisor);
    let semantics = Mc68000::semantics(&instruction, &point).unwrap();
    assert_eq!(semantics.control_flow.action, ControlAction::Continue);
    assert_eq!(
        semantics.control_flow.fallthrough.unwrap().location,
        m68000::CodeLocation::new(0x1002),
    );
}
