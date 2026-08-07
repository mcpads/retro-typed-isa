use powerpc_750cl::{
    DecodeError, FloatingRegister, Instruction, InstructionForm, InstructionModifiers, Operand,
    decode_word, encode_word,
};

#[test]
fn ps_merge10_detail_encoding_beats_the_appendix_typo() {
    // IBM Appendix B Table B-1 misprints the selector. The instruction detail
    // on page 583, the timing table on page 242, and Table B-4 all agree on
    // XO 592 and the word below.
    let instruction = Instruction::with_modifiers(
        InstructionForm::PsMerge10,
        InstructionModifiers::default(),
        [
            Operand::FloatingRegister(FloatingRegister::new(2).unwrap()),
            Operand::FloatingRegister(FloatingRegister::new(3).unwrap()),
            Operand::FloatingRegister(FloatingRegister::new(4).unwrap()),
        ],
    )
    .unwrap();

    assert_eq!(encode_word(&instruction, 0), Ok(0x1043_24a0));
}

#[test]
fn dcba_is_not_admitted_to_the_gekko_compatible_profile() {
    // Dolphin commit 66d8a892 retains selector 758 in a generic dispatch
    // table, but identifies it as not a Gekko instruction. devkitPro binutils
    // commit 55113f8e also excludes dcba from PPC750 targets.
    let word = 0x7c00_05ec;
    assert_eq!(
        decode_word(word, 0),
        Err(DecodeError::ReservedWord { word })
    );
}
