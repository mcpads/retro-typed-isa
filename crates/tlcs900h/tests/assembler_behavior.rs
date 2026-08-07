use tlcs900h::{Assembler, Condition, Instruction, MainInstruction};

#[test]
fn labels_use_the_post_instruction_program_counter() {
    let mut assembler = Assembler::new();
    assembler
        .label("start")
        .jump_relative_to_label(Condition::True, "end")
        .emit(Instruction::Main(MainInstruction::Nop))
        .label("end")
        .call_relative_to_label("start")
        .jump_to_label("end");

    let program = assembler.assemble(0x010000).unwrap();
    assert_eq!(
        program.bytes(),
        [0x68, 0x01, 0x00, 0x1e, 0xfa, 0xff, 0x1b, 0x03, 0x00, 0x01]
    );
    assert_eq!(program.label_address("end"), Some(0x010003));
}

#[test]
fn data_is_explicit_and_not_recorded_as_an_instruction() {
    let mut assembler = Assembler::new();
    assembler
        .emit(Instruction::Main(MainInstruction::Nop))
        .data([0x01, 0x04])
        .emit(Instruction::Main(MainInstruction::Halt));

    let program = assembler.assemble(0).unwrap();
    assert_eq!(program.bytes(), [0x00, 0x01, 0x04, 0x05]);
    assert_eq!(program.instruction_spans().len(), 2);
}
