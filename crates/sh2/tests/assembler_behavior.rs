use sh2::{Assembler, AssemblerError, Instruction, Register};

#[test]
fn resolves_pc_relative_labels_and_emits_big_endian_words() {
    let mut assembler = Assembler::new();
    assembler
        .bra("target")
        .emit(Instruction::Nop)
        .label("target")
        .emit(Instruction::SetT);

    let program = assembler.assemble(0x0600_0000).unwrap();
    assert_eq!(program.words(), &[0xA000, 0x0009, 0x0018]);
    assert_eq!(program.bytes(), &[0xA0, 0x00, 0x00, 0x09, 0x00, 0x18]);
    assert_eq!(program.instructions()[0], Instruction::Bra { disp: 0 });
    assert_eq!(program.instruction_spans()[0].offset, 0);
    assert_eq!(program.instruction_spans()[0].location, 0x0600_0000);
    assert_eq!(
        program.instruction_spans()[0].bytes.as_slice(),
        [0xA0, 0x00]
    );
}

#[test]
fn requires_and_validates_delay_slots() {
    let mut missing = Assembler::new();
    missing.bra("end").label("end");
    assert!(matches!(
        missing.assemble(0),
        Err(AssemblerError::MissingDelaySlot { .. })
    ));

    let mut illegal = Assembler::new();
    illegal
        .bra("end")
        .bt("end")
        .label("end")
        .emit(Instruction::Nop);
    assert!(matches!(
        illegal.assemble(0),
        Err(AssemblerError::IllegalDelaySlot {
            instruction: Instruction::Bt { .. },
            ..
        })
    ));

    let mut trap = Assembler::new();
    trap.bra("end")
        .emit(Instruction::Trapa { imm: 1 })
        .label("end")
        .emit(Instruction::Nop);
    assert!(matches!(
        trap.assemble(0),
        Err(AssemblerError::IllegalDelaySlot {
            instruction: Instruction::Trapa { .. },
            ..
        })
    ));
}

#[test]
fn checks_builder_and_address_boundaries() {
    let mut duplicate = Assembler::new();
    duplicate.label("same").label("same");
    assert!(matches!(
        duplicate.assemble(0),
        Err(AssemblerError::DuplicateLabel { .. })
    ));

    let mut undefined = Assembler::new();
    undefined.bt("missing");
    assert!(matches!(
        undefined.assemble(0),
        Err(AssemblerError::UndefinedLabel { .. })
    ));

    assert!(matches!(
        Assembler::new().assemble(1),
        Err(AssemblerError::UnalignedOrigin { origin: 1 })
    ));

    let mut overflow = Assembler::new();
    overflow.emit(Instruction::Nop);
    assert!(matches!(
        overflow.assemble(u32::MAX - 1),
        Err(AssemblerError::AddressOverflow { .. })
    ));
}

#[test]
fn rejects_eight_and_twelve_bit_label_displacements_out_of_range() {
    let mut short = Assembler::new();
    short.bt("far");
    short.emit_all(std::iter::repeat_n(Instruction::Nop, 129));
    short.label("far");
    assert!(matches!(
        short.assemble(0),
        Err(AssemblerError::BranchOutOfRange { bits: 8, .. })
    ));

    let mut long = Assembler::new();
    long.bra("far").emit(Instruction::Nop);
    long.emit_all(std::iter::repeat_n(Instruction::Nop, 2048));
    long.label("far");
    assert!(matches!(
        long.assemble(0),
        Err(AssemblerError::BranchOutOfRange { bits: 12, .. })
    ));
}

#[test]
fn official_illegal_slot_instruction_set_is_explicit() {
    let illegal = [
        Instruction::Bf { disp: 0 },
        Instruction::Bt { disp: 0 },
        Instruction::Bra { disp: 0 },
        Instruction::Bsr { disp: 0 },
        Instruction::Jmp { rm: Register::R0 },
        Instruction::Jsr { rm: Register::R0 },
        Instruction::Rts,
        Instruction::Rte,
        Instruction::Trapa { imm: 0 },
        Instruction::Bfs { disp: 0 },
        Instruction::Bts { disp: 0 },
        Instruction::Braf { rm: Register::R0 },
        Instruction::Bsrf { rm: Register::R0 },
    ];

    assert!(illegal.iter().all(Instruction::is_illegal_delay_slot));
    assert!(!Instruction::Nop.is_illegal_delay_slot());
}
