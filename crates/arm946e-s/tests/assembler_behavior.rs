use arm7tdmi::{Condition, Register};
use arm946e_s::{ArmAssembler, ArmInstruction, ThumbAssembler, ThumbInstruction};

#[test]
fn arm_blx_uses_pc_plus_eight_and_preserves_halfword_target_bit() {
    let mut assembler = ArmAssembler::new();
    assembler
        .label("thumb")
        .emit(ArmInstruction::BranchLinkExchangeRegister {
            condition: Condition::Always,
            target: Register::new(1).unwrap(),
        })
        .call_thumb_label("thumb");
    let program = assembler.assemble(0x1000).unwrap();
    assert_eq!(
        program.bytes(),
        [0x31, 0xff, 0x2f, 0xe1, 0xfd, 0xff, 0xff, 0xfa]
    );
}

#[test]
fn thumb_blx_uses_the_word_aligned_pc_plus_four_base() {
    let mut assembler = ThumbAssembler::new();
    assembler
        .label("arm")
        .emit(ThumbInstruction::Breakpoint { comment: 0x12 })
        .call_arm_label("arm");
    let program = assembler.assemble(0x1000).unwrap();
    assert_eq!(program.bytes(), [0x12, 0xbe, 0xff, 0xf7, 0xfe, 0xef]);
}

#[test]
fn declared_data_cannot_silently_misalign_following_code() {
    let mut arm = ArmAssembler::new();
    arm.data([0xaa])
        .emit(ArmInstruction::Breakpoint { comment: 0 });
    assert!(arm.assemble(0).is_err());

    let mut thumb = ThumbAssembler::new();
    thumb
        .data([0xaa])
        .emit(ThumbInstruction::Breakpoint { comment: 0 });
    assert!(thumb.assemble(0).is_err());
}
