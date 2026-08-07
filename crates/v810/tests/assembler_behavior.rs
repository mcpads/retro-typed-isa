use v810::{Assembler, AssemblerError, Condition, Instruction, Register};

fn register(index: u8) -> Register {
    Register::new(index).unwrap()
}

#[test]
fn labels_lower_to_current_pc_relative_displacements() {
    let mut assembler = Assembler::new();
    assembler
        .label("loop")
        .emit(Instruction::JumpRegister(register(31)))
        .branch_to_label(Condition::NotZeroOrNotEqual, "loop")
        .jump_and_link("loop");

    let program = assembler.assemble(0x1000).unwrap();
    assert_eq!(program.label_address("loop"), Some(0x1000));
    assert_eq!(
        program.bytes(),
        [0x1f, 0x18, 0xfe, 0x95, 0xff, 0xaf, 0xfc, 0xff]
    );
}

#[test]
fn declared_data_cannot_silently_misalign_following_code() {
    let mut assembler = Assembler::new();
    assembler
        .emit(Instruction::Nop)
        .data([0xff])
        .emit(Instruction::Halt);

    assert_eq!(
        assembler.assemble(0),
        Err(AssemblerError::UnalignedInstruction { offset: 3 })
    );
}

#[test]
fn duplicate_and_undefined_labels_are_rejected() {
    let mut duplicate = Assembler::new();
    duplicate.label("same").label("same");
    assert!(matches!(
        duplicate.assemble(0),
        Err(AssemblerError::DuplicateLabel { .. })
    ));

    let mut undefined = Assembler::new();
    undefined.jump_relative("missing");
    assert!(matches!(
        undefined.assemble(0),
        Err(AssemblerError::UndefinedLabel { .. })
    ));
}
