use huc6280::{
    AddressingMode, Assembler, ControlFlow, Instruction, InstructionForm, Mnemonic, Operand,
    PROFILE_ID, decode_bytes, encode_bytes,
};

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
    assert_eq!(
        decoded.control_flow(program.origin()),
        ControlFlow::FallThrough { next: 0x8002 }
    );
}
