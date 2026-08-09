use m68000::{
    AnalysisPoint, Assembler, DecodedInstruction, Instruction, InstructionForm, Mc68000,
    PROFILE_ID, PrivilegeMode, decode_bytes, encode_bytes, verify_placed_program,
};
use typed_isa_core::{ControlAction, StaticSemantics};

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
