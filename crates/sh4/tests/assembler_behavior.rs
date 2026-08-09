use sh4::{
    Assembler, AssemblerError, DoubleRegister, FloatRegister, FpscrState, FpuPairRegister,
    FpuPrecision, FpuTransferSize, Instruction, LiteralValue, Register,
};

const SINGLE: FpscrState = FpscrState::new(FpuPrecision::Single, FpuTransferSize::Single);
const PAIR: FpscrState = FpscrState::new(FpuPrecision::Single, FpuTransferSize::Pair);

#[test]
fn resolves_labels_and_emits_little_endian_words() {
    let mut assembler = Assembler::new();
    assembler
        .bra("target")
        .emit(Instruction::Nop)
        .label("target")
        .emit(Instruction::SetT);

    let program = assembler.assemble(0x8C01_0000, SINGLE).unwrap();
    assert_eq!(program.words(), &[0xA000, 0x0009, 0x0018]);
    assert_eq!(program.bytes(), &[0x00, 0xA0, 0x09, 0x00, 0x18, 0x00]);
    assert_eq!(program.instructions()[0], Instruction::Bra { disp: 0 });
    assert_eq!(program.instruction_spans()[0].offset, 0);
    assert_eq!(program.instruction_spans()[0].location, 0x8C01_0000);
    assert_eq!(
        program.instruction_spans()[0].bytes.as_slice(),
        [0x00, 0xA0]
    );
}

#[test]
fn explicit_fpscr_annotations_guard_semantic_round_trips() {
    let fr0 = FloatRegister::new(0).unwrap();
    let dr0 = FpuPairRegister::Double(DoubleRegister::new(0).unwrap());
    let mut assembler = Assembler::new();
    assembler
        .emit(Instruction::Fmov { frm: fr0, frn: fr0 })
        .assume_fpscr(PAIR)
        .emit(Instruction::FmovPair {
            source: dr0,
            destination: dr0,
        });
    let program = assembler.assemble(0, SINGLE).unwrap();
    assert_eq!(program.placed_instructions()[0].fpscr(), SINGLE);
    assert_eq!(program.placed_instructions()[1].fpscr(), PAIR);

    let mut wrong = Assembler::new();
    wrong.emit(Instruction::FmovPair {
        source: dr0,
        destination: dr0,
    });
    assert!(matches!(
        wrong.assemble(0, SINGLE),
        Err(AssemblerError::SemanticRoundTripMismatch { .. })
    ));
}

#[test]
fn validates_labels_ranges_and_delay_slots() {
    let mut duplicate = Assembler::new();
    duplicate.label("same").label("same");
    assert!(matches!(
        duplicate.assemble(0, SINGLE),
        Err(AssemblerError::DuplicateLabel { .. })
    ));

    let mut undefined = Assembler::new();
    undefined.bt("missing");
    assert!(matches!(
        undefined.assemble(0, SINGLE),
        Err(AssemblerError::UndefinedLabel { .. })
    ));

    let mut illegal = Assembler::new();
    illegal
        .bra("end")
        .emit(Instruction::MovWPcRel {
            disp: 0,
            rn: Register::R0,
        })
        .label("end")
        .emit(Instruction::Nop);
    assert!(matches!(
        illegal.assemble(0, SINGLE),
        Err(AssemblerError::IllegalDelaySlot { .. })
    ));

    let mut short = Assembler::new();
    short.bt("far");
    short.emit_all(std::iter::repeat_n(Instruction::Nop, 129));
    short.label("far");
    assert!(matches!(
        short.assemble(0, SINGLE),
        Err(AssemblerError::BranchOutOfRange { bits: 8, .. })
    ));
}

#[test]
fn places_typed_literal_pools_and_resolves_scaled_displacements() {
    let mut assembler = Assembler::new();
    assembler
        .mov_l_literal(0xDEAD_BEEF, Register::R1)
        .mov_w_literal(0x4321, Register::R2)
        .emit(Instruction::Rts)
        .emit(Instruction::Nop);

    let program = assembler.assemble(0x8C01_0002, SINGLE).unwrap();
    assert_eq!(program.words(), &[0xD102, 0x9204, 0x000B, 0x0009]);
    assert_eq!(
        program
            .literals()
            .iter()
            .map(|literal| literal.value())
            .collect::<Vec<_>>(),
        [LiteralValue::Long(0xDEAD_BEEF), LiteralValue::Word(0x4321),]
    );
    assert_eq!(program.literals()[0].address() & 3, 0);
    assert!(
        program
            .bytes()
            .ends_with(&[0xEF, 0xBE, 0xAD, 0xDE, 0x21, 0x43])
    );
}

#[test]
fn explicit_pools_bound_reach_and_cannot_replace_a_delay_slot() {
    let mut near = Assembler::new();
    near.mov_l_literal(7, Register::R0)
        .emit_literal_pool()
        .emit(Instruction::Nop);
    assert!(near.assemble(0, SINGLE).is_ok());

    let mut far = Assembler::new();
    far.mov_w_literal(7, Register::R0)
        .emit_all(std::iter::repeat_n(Instruction::Nop, 257));
    assert!(matches!(
        far.assemble(0, SINGLE),
        Err(AssemblerError::LiteralOutOfRange { .. })
    ));

    let mut missing_slot = Assembler::new();
    missing_slot
        .bra("end")
        .mov_l_literal(7, Register::R0)
        .emit_literal_pool()
        .label("end");
    assert!(matches!(
        missing_slot.assemble(0, SINGLE),
        Err(AssemblerError::IllegalDelaySlot { .. })
    ));
}
