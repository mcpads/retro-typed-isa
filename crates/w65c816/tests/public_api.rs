use w65c816::{
    AddressingMode, Assembler, CpuMode, Instruction, Mnemonic, Operand, PROFILE_ID, WidthState,
    disassemble_linear, encode_bytes,
};

#[test]
fn consumer_can_verify_width_sensitive_program() {
    assert_eq!(PROFILE_ID, "w65c816");

    let mut program = Assembler::new();
    program
        .emit(Instruction::new(
            Mnemonic::Rep,
            AddressingMode::ImmediateByte,
            Operand::Byte(0x20),
        ))
        .emit(Instruction::new(
            Mnemonic::Lda,
            AddressingMode::Immediate,
            Operand::Word(0x1234),
        ))
        .emit(Instruction::new(
            Mnemonic::Sep,
            AddressingMode::ImmediateByte,
            Operand::Byte(0x20),
        ))
        .emit(Instruction::new(
            Mnemonic::Lda,
            AddressingMode::Immediate,
            Operand::Byte(0x56),
        ));

    let origin = 0x80_8000;
    let assembled = program
        .assemble(origin, CpuMode::NATIVE_M8_X8)
        .expect("typed program should assemble and verify");
    assert_eq!(
        assembled.instruction_spans()[1].instruction.form(),
        w65c816::InstructionForm::new(Mnemonic::Lda, AddressingMode::Immediate)
    );
    assert_eq!(
        assembled.bytes(),
        [0xC2, 0x20, 0xA9, 0x34, 0x12, 0xE2, 0x20, 0xA9, 0x56]
    );
    assert_eq!(
        encode_bytes(
            &assembled.instruction_spans()[1].instruction,
            WidthState::M16_X8
        )
        .unwrap(),
        [0xA9, 0x34, 0x12]
    );

    let disassembly = disassemble_linear(assembled.bytes(), origin, CpuMode::NATIVE_M8_X8)
        .expect("assembled bytes should decode under the same CPU mode");
    assert_eq!(disassembly.final_mode, Some(CpuMode::NATIVE_M8_X8));
    assert_eq!(
        disassembly.instructions[1].mode_before.widths(),
        WidthState::M16_X8
    );
    assert_eq!(
        disassembly.instructions[3].mode_before.widths(),
        WidthState::M8_X8
    );
}
