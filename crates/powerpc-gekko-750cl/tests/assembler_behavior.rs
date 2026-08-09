use powerpc_gekko_750cl::{
    Assembler, AssemblerError, GeneralRegister, Instruction, InstructionForm, Operand,
};

fn addi() -> Instruction {
    Instruction::new(
        InstructionForm::Addi,
        [
            Operand::GeneralRegister(GeneralRegister::new(0).unwrap()),
            Operand::GeneralRegister(GeneralRegister::new(1).unwrap()),
            Operand::SignedImmediate(0x140),
        ],
    )
    .unwrap()
}

#[test]
fn labels_lower_to_checked_relative_branches() {
    let mut assembler = Assembler::new();
    assembler
        .label("entry")
        .branch_to_label(false, "exit")
        .emit(addi())
        .label("exit")
        .branch_to_label(true, "entry");

    let program = assembler.assemble(0x8000_0000).unwrap();

    assert_eq!(program.label_location("entry"), Some(0x8000_0000));
    assert_eq!(program.label_location("exit"), Some(0x8000_0008));
    assert_eq!(
        program.bytes(),
        [
            0x48, 0x00, 0x00, 0x08, 0x38, 0x01, 0x01, 0x40, 0x4b, 0xff, 0xff, 0xf9,
        ]
    );
    assert_eq!(program.instruction_spans().len(), 3);
}

#[test]
fn data_is_explicit_and_cannot_misalign_following_code() {
    let mut assembler = Assembler::new();
    assembler.data([0xde, 0xad]).emit(addi());

    assert_eq!(
        assembler.assemble(0x8000_0000),
        Err(AssemblerError::UnalignedInstruction { offset: 2 })
    );
}

#[test]
fn placement_failures_are_reported_before_bytes_are_returned() {
    let mut duplicate = Assembler::new();
    duplicate.label("same").emit(addi()).label("same");
    assert_eq!(
        duplicate.assemble(0x8000_0000),
        Err(AssemblerError::DuplicateLabel {
            label: "same".to_owned(),
        })
    );

    let mut missing = Assembler::new();
    missing.branch_to_label(false, "missing");
    assert_eq!(
        missing.assemble(0x8000_0000),
        Err(AssemblerError::UndefinedLabel {
            label: "missing".to_owned(),
        })
    );

    let mut unaligned = Assembler::new();
    unaligned.emit(addi());
    assert_eq!(
        unaligned.assemble(0x8000_0002),
        Err(AssemblerError::UnalignedOrigin {
            origin: 0x8000_0002,
        })
    );
}
