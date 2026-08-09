use sm83::{Assembler, AssemblerError, ByteOperand, Condition, Instruction};

#[test]
fn labels_resolve_to_checked_relative_and_absolute_operands() {
    let mut assembler = Assembler::new();
    assembler
        .jump("body")
        .label("loop")
        .emit(Instruction::Increment8(ByteOperand::B))
        .jump_relative_if(Condition::Nz, "loop")
        .label("body")
        .call("loop");

    let program = assembler.assemble(0x4000).unwrap();
    assert_eq!(
        program.bytes(),
        [0xc3, 0x06, 0x40, 0x04, 0x20, 0xfd, 0xcd, 0x03, 0x40]
    );
    assert_eq!(program.label_location("loop"), Some(0x4003));
    assert_eq!(program.label_location("body"), Some(0x4006));
}

#[test]
fn relative_branch_outside_signed_byte_range_fails() {
    let mut assembler = Assembler::new();
    assembler
        .jump_relative("far")
        .data(vec![0; 128])
        .label("far");

    assert!(matches!(
        assembler.assemble(0),
        Err(AssemblerError::RelativeBranchOutOfRange { .. })
    ));
}

#[test]
fn code_and_declared_data_remain_separate() {
    let mut assembler = Assembler::new();
    assembler
        .emit(Instruction::Nop)
        .data([0xd3, 0xdb].to_vec())
        .label("pointer")
        .label_word("pointer");

    let program = assembler.assemble(0x2000).unwrap();
    assert_eq!(program.bytes(), [0x00, 0xd3, 0xdb, 0x03, 0x20]);
    assert_eq!(program.instruction_spans().len(), 1);
    assert_eq!(program.instruction_spans()[0].offset, 0);
    assert_eq!(program.instruction_spans()[0].location, 0x2000);
    assert_eq!(program.instruction_spans()[0].bytes.as_slice(), [0x00]);
}
