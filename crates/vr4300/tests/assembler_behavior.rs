use vr4300::{
    Assembler, AssemblerError, EqualityCondition, GeneralRegister, ImmediateArithmeticOperation,
    Instruction,
};

fn addiu() -> Instruction {
    Instruction::ImmediateArithmetic {
        operation: ImmediateArithmeticOperation::AddUnsigned,
        destination: GeneralRegister::SP,
        source: GeneralRegister::SP,
        immediate: -0x10,
    }
}

#[test]
fn labels_lower_to_checked_branch_and_jump_targets() {
    let mut assembler = Assembler::new();
    assembler
        .label("entry")
        .equality_branch_to_label(
            EqualityCondition::NotEqual,
            false,
            GeneralRegister::T0,
            GeneralRegister::ZERO,
            "exit",
        )
        .emit(addiu())
        .label("exit")
        .jump_to_label(false, "entry");

    let program = assembler.assemble(0x8000_0000).unwrap();

    assert_eq!(program.label_address("entry"), Some(0x8000_0000));
    assert_eq!(program.label_address("exit"), Some(0x8000_0008));
    assert_eq!(
        program.bytes(),
        [
            0x15, 0x00, 0x00, 0x01, 0x27, 0xbd, 0xff, 0xf0, 0x08, 0x00, 0x00, 0x00,
        ]
    );
    assert_eq!(program.instruction_spans().len(), 3);
    assert_eq!(
        program.instruction_spans()[0].bytes.as_slice(),
        [0x15, 0x00, 0x00, 0x01]
    );
}

#[test]
fn explicit_data_cannot_silently_misalign_code() {
    let mut assembler = Assembler::new();
    assembler.data([0xde, 0xad]).emit(addiu());

    assert_eq!(
        assembler.assemble(0x8000_0000),
        Err(AssemblerError::UnalignedInstruction { offset: 2 })
    );
}

#[test]
fn undefined_and_duplicate_labels_are_rejected() {
    let mut missing = Assembler::new();
    missing.jump_to_label(false, "missing");
    assert_eq!(
        missing.assemble(0x8000_0000),
        Err(AssemblerError::UndefinedLabel {
            label: "missing".to_owned()
        })
    );

    let mut duplicate = Assembler::new();
    duplicate.label("same").emit(addiu()).label("same");
    assert_eq!(
        duplicate.assemble(0x8000_0000),
        Err(AssemblerError::DuplicateLabel {
            label: "same".to_owned()
        })
    );
}
