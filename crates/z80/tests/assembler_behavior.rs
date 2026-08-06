use z80::{Assembler, AssemblerError, Condition, Instruction};

#[test]
fn resolves_relative_absolute_and_call_labels() {
    let mut assembler = Assembler::new();
    assembler
        .label("start")
        .jr_cond(Condition::Nz, "done")
        .emit(Instruction::Nop)
        .call("start")
        .label("done")
        .jp("start");

    let program = assembler.assemble(0x4000).unwrap();
    assert_eq!(
        program.bytes(),
        &[0x20, 0x04, 0x00, 0xCD, 0x00, 0x40, 0xC3, 0x00, 0x40]
    );
    assert_eq!(program.origin(), 0x4000);
    assert_eq!(program.instructions().len(), 4);
}

#[test]
fn rejects_label_and_address_failures() {
    let mut duplicate = Assembler::new();
    duplicate.label("x").label("x");
    assert!(matches!(
        duplicate.assemble(0),
        Err(AssemblerError::DuplicateLabel { .. })
    ));

    let mut undefined = Assembler::new();
    undefined.jp("missing");
    assert!(matches!(
        undefined.assemble(0),
        Err(AssemblerError::UndefinedLabel { .. })
    ));

    let mut overflow = Assembler::new();
    overflow.emit(Instruction::Nop).emit(Instruction::Nop);
    assert!(matches!(
        overflow.assemble(0xFFFF),
        Err(AssemblerError::AddressOverflow { .. })
    ));
}

#[test]
fn rejects_relative_labels_outside_signed_byte_range() {
    let mut assembler = Assembler::new();
    assembler.jr("far");
    assembler.emit_all(std::iter::repeat_n(Instruction::Nop, 128));
    assembler.label("far");
    assert!(matches!(
        assembler.assemble(0),
        Err(AssemblerError::RelativeBranchOutOfRange { .. })
    ));
}
