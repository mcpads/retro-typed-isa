use rp2a03::{
    AddressingMode, Assembler, ControlFlow, Instruction, InstructionForm, Mnemonic, Operand,
    PROFILE_ID, decode_bytes, encode_bytes,
};

#[test]
fn public_profile_builds_checked_famicom_code() {
    assert_eq!(PROFILE_ID, "rp2a03");
    assert_eq!(InstructionForm::all().count(), 221);

    let lda = Instruction::new(
        Mnemonic::Lda,
        AddressingMode::Immediate,
        Operand::Byte(0x2A),
    )
    .unwrap();
    assert_eq!(encode_bytes(&lda).unwrap(), [0xA9, 0x2A]);
    assert_eq!(decode_bytes(&[0xA9, 0x2A]).unwrap(), lda);

    let mut assembler = Assembler::new();
    assembler.label("loop").emit(lda).emit_label_ref(
        Mnemonic::Bne,
        AddressingMode::Relative,
        "loop",
    );
    let program = assembler.assemble(0x8000).unwrap();
    assert_eq!(program.bytes(), [0xA9, 0x2A, 0xD0, 0xFC]);
    assert_eq!(
        program.instruction_spans()[1]
            .instruction
            .control_flow(0x8002),
        ControlFlow::Branch {
            target: 0x8000,
            fallthrough: Some(0x8004),
        }
    );
}
