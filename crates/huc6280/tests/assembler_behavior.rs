use huc6280::{
    AddressingMode, AnalysisPoint, Assembler, AssemblerError, EncodeError, HuC6280, Instruction,
    Mnemonic, Operand, TState, decode_bytes,
};
use typed_isa_core::{ControlAction, ControlTarget, StaticSemantics};

fn instruction(mnemonic: Mnemonic, mode: AddressingMode, operand: Operand) -> Instruction {
    Instruction::new(mnemonic, mode, operand).unwrap()
}

#[test]
fn resolves_forward_and_backward_typed_code_references() {
    let mut assembler = Assembler::new();
    assembler
        .emit(instruction(
            Mnemonic::Lda,
            AddressingMode::ZeroPage,
            Operand::Byte(0x41),
        ))
        .emit_label_ref(Mnemonic::Sta, AddressingMode::Absolute, "variable")
        .emit_label_ref(Mnemonic::Beq, AddressingMode::Relative, "done")
        .emit_label_ref(Mnemonic::Jmp, AddressingMode::Absolute, "variable")
        .label("done")
        .emit(instruction(
            Mnemonic::Rts,
            AddressingMode::Implied,
            Operand::None,
        ))
        .label("variable")
        .emit(instruction(
            Mnemonic::Nop,
            AddressingMode::Implied,
            Operand::None,
        ));

    let program = assembler.assemble(0x5000).unwrap();
    assert_eq!(program.origin(), 0x5000);
    assert_eq!(program.label_location("done"), Some(0x500A));
    assert_eq!(program.label_location("variable"), Some(0x500B));
    assert_eq!(
        program.bytes(),
        [
            0xA5, 0x41, 0x8D, 0x0B, 0x50, 0xF0, 0x03, 0x4C, 0x0B, 0x50, 0x60, 0xEA,
        ]
    );
    assert_eq!(program.instruction_spans().len(), 6);

    let branch = decode_bytes(&program.bytes()[5..]).unwrap();
    let flow = HuC6280::semantics(&branch, &AnalysisPoint::new(0x5005, TState::Unknown))
        .unwrap()
        .control_flow;
    assert_eq!(
        flow.action,
        ControlAction::Transfer {
            target: ControlTarget::Direct(AnalysisPoint::new(0x500A, TState::Clear)),
        }
    );
    assert_eq!(
        flow.fallthrough,
        Some(AnalysisPoint::new(0x5007, TState::Clear))
    );

    let mut backward = Assembler::new();
    backward
        .label("again")
        .emit(instruction(
            Mnemonic::Nop,
            AddressingMode::Implied,
            Operand::None,
        ))
        .emit_label_ref(Mnemonic::Bra, AddressingMode::Relative, "again");
    assert_eq!(
        backward.assemble(0x6000).unwrap().bytes(),
        [0xEA, 0x80, 0xFD]
    );
}

#[test]
fn zero_page_branch_and_block_transfer_labels_are_checked() {
    let mut branch = Assembler::new();
    branch
        .emit_zero_page_branch(Mnemonic::Bbr { bit: 0 }, 0x42, "clear")
        .emit(instruction(
            Mnemonic::Nop,
            AddressingMode::Implied,
            Operand::None,
        ))
        .label("clear");
    assert_eq!(
        branch.assemble(0x1000).unwrap().bytes(),
        [0x0F, 0x42, 0x01, 0xEA]
    );

    let mut transfer = Assembler::new();
    transfer
        .label("source")
        .data([0x11, 0x22, 0x33, 0x44].to_vec())
        .emit_block_transfer_from_label(Mnemonic::Tii, "source", 0x2DA5, 4);
    assert_eq!(
        transfer.assemble(0x56F6).unwrap().bytes(),
        [
            0x11, 0x22, 0x33, 0x44, 0x73, 0xF6, 0x56, 0xA5, 0x2D, 0x04, 0x00,
        ]
    );
}

#[test]
fn declared_data_is_separate_from_verified_instruction_spans() {
    let mut assembler = Assembler::new();
    assembler
        .data([0xDE, 0xAD].to_vec())
        .word(0x1234)
        .label_word("after")
        .label("after")
        .emit(Instruction::from_opcode(0xEA, Operand::None).unwrap());
    let program = assembler.assemble(0x4000).unwrap();

    assert_eq!(program.bytes(), [0xDE, 0xAD, 0x34, 0x12, 0x06, 0x40, 0xEA]);
    assert_eq!(program.instruction_spans().len(), 1);
    assert_eq!(program.instruction_spans()[0].offset, 6);
    assert_eq!(program.instruction_spans()[0].location, 0x4006);
    assert_eq!(program.instruction_spans()[0].bytes.as_slice(), [0xEA]);
    assert_eq!(program.instruction_spans()[0].instruction.opcode(), 0xEA);
}

#[test]
fn label_and_address_failures_do_not_wrap_or_truncate() {
    let mut duplicate = Assembler::new();
    duplicate.label("same").label("same");
    assert_eq!(
        duplicate.assemble(0),
        Err(AssemblerError::DuplicateLabel {
            label: "same".to_owned(),
        })
    );

    let mut undefined = Assembler::new();
    undefined.emit_label_ref(Mnemonic::Jsr, AddressingMode::Absolute, "missing");
    assert_eq!(
        undefined.assemble(0x8000),
        Err(AssemblerError::UndefinedLabel {
            label: "missing".to_owned(),
        })
    );

    let mut out_of_range = Assembler::new();
    out_of_range
        .label("start")
        .data(vec![0; 200])
        .emit_label_ref(Mnemonic::Bne, AddressingMode::Relative, "start");
    assert!(matches!(
        out_of_range.assemble(0x2000),
        Err(AssemblerError::RelativeOutOfRange { .. })
    ));

    let mut addend = Assembler::new();
    addend
        .label("zero")
        .emit_label_ref_offset(Mnemonic::Lda, AddressingMode::Absolute, "zero", -1);
    assert_eq!(
        addend.assemble(0),
        Err(AssemblerError::LabelAddendOutOfRange {
            label: "zero".to_owned(),
            addend: -1,
        })
    );

    let mut overflow = Assembler::new();
    overflow.emit(instruction(
        Mnemonic::Lda,
        AddressingMode::Immediate,
        Operand::Byte(0),
    ));
    assert_eq!(
        overflow.assemble(0xFFFF),
        Err(AssemblerError::AddressOverflow {
            origin: 0xFFFF,
            byte_len: 2,
        })
    );
}

#[test]
fn label_helpers_reject_non_address_forms_and_wrong_instruction_families() {
    let mut unsupported = Assembler::new();
    unsupported.emit_label_ref(Mnemonic::Lda, AddressingMode::Immediate, "value");
    assert_eq!(
        unsupported.assemble(0),
        Err(AssemblerError::UnsupportedLabelMode {
            mode: AddressingMode::Immediate,
        })
    );

    let mut wrong_branch = Assembler::new();
    wrong_branch
        .emit_zero_page_branch(Mnemonic::Lda, 0x10, "target")
        .label("target");
    assert_eq!(
        wrong_branch.assemble(0),
        Err(AssemblerError::Encode(EncodeError::UnknownEncoding {
            mnemonic: Mnemonic::Lda,
            mode: AddressingMode::ZeroPageRelative,
        }))
    );
}
