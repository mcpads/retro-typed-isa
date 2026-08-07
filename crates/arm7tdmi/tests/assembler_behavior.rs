use arm7tdmi::{
    ArmAssembler, ArmInstruction, Condition, DataOperation, LowRegister, Operand2, Register,
    RotatedImmediate, ThumbAssembler, ThumbImmediateOperation, ThumbInstruction,
};

fn register(index: u8) -> Register {
    Register::new(index).unwrap()
}

fn low(index: u8) -> LowRegister {
    LowRegister::new(index).unwrap()
}

fn arm_move_one() -> ArmInstruction {
    ArmInstruction::DataProcessing {
        condition: Condition::Always,
        operation: DataOperation::Move,
        set_flags: false,
        destination: register(0),
        first: register(0),
        second: Operand2::Immediate(RotatedImmediate::new(1, 0).unwrap()),
    }
}

#[test]
fn arm_labels_use_the_architectural_pc_plus_eight_base() {
    let mut assembler = ArmAssembler::new();
    assembler
        .label("loop")
        .emit(arm_move_one())
        .branch_to_label(Condition::Always, "loop");
    let program = assembler.assemble(0x1000).unwrap();
    assert_eq!(program.label_address("loop"), Some(0x1000));
    assert_eq!(
        program.bytes(),
        [0x01, 0x00, 0xa0, 0xe3, 0xfd, 0xff, 0xff, 0xea]
    );
}

#[test]
fn thumb_labels_use_the_architectural_pc_plus_four_base() {
    let mut assembler = ThumbAssembler::new();
    assembler
        .label("loop")
        .emit(ThumbInstruction::Immediate {
            operation: ThumbImmediateOperation::Move,
            register: low(0),
            immediate: 1,
        })
        .branch_to_label(Condition::NotEqual, "loop")
        .call_label("loop");
    let program = assembler.assemble(0x1000).unwrap();
    assert_eq!(program.label_address("loop"), Some(0x1000));
    assert_eq!(
        program.bytes(),
        [0x01, 0x20, 0xfd, 0xd1, 0xff, 0xf7, 0xfc, 0xff]
    );
}

#[test]
fn declared_data_cannot_silently_misalign_following_code() {
    let mut arm = ArmAssembler::new();
    arm.data([0xaa]).emit(arm_move_one());
    assert!(arm.assemble(0).is_err());

    let mut thumb = ThumbAssembler::new();
    thumb.data([0xaa]).emit(ThumbInstruction::BranchExchange {
        source: Register::LINK,
    });
    assert!(thumb.assemble(0).is_err());
}
