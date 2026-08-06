use psx_r3000a::{
    Assembler, ControlFlow, Instruction, InstructionForm, PROFILE_ID, Register, encode_bytes,
    verify_placed_program,
};

#[test]
fn consumer_can_assemble_and_verify_placed_control_flow() {
    assert_eq!(PROFILE_ID, "psx-r3000a");

    let origin = 0x8001_0000;
    let mut assembler = Assembler::new();
    assembler
        .label("loop")
        .emit(Instruction::Addiu {
            rt: Register::T0,
            rs: Register::T0,
            immediate: 1,
        })
        .bne(Register::T0, Register::T1, "loop")
        .emit(Instruction::nop());

    let program = assembler.assemble(origin).unwrap();
    let decoded = verify_placed_program(program.bytes(), origin).unwrap();
    assert_eq!(decoded, program.instructions());
    assert_eq!(
        encode_bytes(&decoded[0], origin).unwrap(),
        program.bytes()[..4]
    );
    assert_eq!(decoded[1].form(), InstructionForm::Bne);
    assert_eq!(
        decoded[1].control_flow(),
        ControlFlow::ConditionalBranch { target: origin }
    );
}
