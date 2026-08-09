use huc6280::{
    AddressingMode, AnalysisPoint, Assembler, HuC6280, Instruction, InstructionForm, Mnemonic,
    Operand, PROFILE_ID, TState, decode_bytes, encode_bytes,
};
use typed_isa_core::{ControlAction, StaticSemantics};

#[test]
fn public_api_builds_places_and_lowers_typed_code_to_bytes() {
    assert_eq!(PROFILE_ID, "huc6280");
    assert_eq!(InstructionForm::all().count(), 234);

    let load = Instruction::new(
        Mnemonic::Lda,
        AddressingMode::Immediate,
        Operand::Byte(0x2A),
    )
    .unwrap();
    let ret = Instruction::new(Mnemonic::Rts, AddressingMode::Implied, Operand::None).unwrap();

    let mut assembler = Assembler::new();
    assembler
        .emit(load)
        .emit_label_ref(Mnemonic::Bne, AddressingMode::Relative, "done")
        .emit(Instruction::new(Mnemonic::Nop, AddressingMode::Implied, Operand::None).unwrap())
        .label("done")
        .emit(ret);
    let program = assembler.assemble(0x8000).unwrap();

    assert_eq!(program.bytes(), [0xA9, 0x2A, 0xD0, 0x01, 0xEA, 0x60]);
    assert_eq!(program.origin(), 0x8000);
    assert_eq!(program.instruction_spans().len(), 4);

    let decoded = decode_bytes(program.bytes()).unwrap();
    assert_eq!(decoded, load);
    assert_eq!(encode_bytes(&decoded).unwrap(), [0xA9, 0x2A]);
    let flow = HuC6280::semantics(
        &decoded,
        &AnalysisPoint::new(program.origin(), TState::Clear),
    )
    .unwrap()
    .control_flow;
    assert_eq!(flow.action, ControlAction::Continue);
    assert_eq!(
        flow.fallthrough,
        Some(AnalysisPoint::new(0x8002, TState::Clear))
    );
}
