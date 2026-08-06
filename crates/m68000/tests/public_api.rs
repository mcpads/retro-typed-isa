use m68000::{
    Assembler, ControlFlow, DecodedInstruction, Instruction, InstructionForm, PROFILE_ID,
    decode_bytes, encode_bytes, verify_placed_program,
};

#[test]
fn profile_and_common_names_are_stable() {
    assert_eq!(PROFILE_ID, "m68000");
    assert_eq!(InstructionForm::all().len(), 1_365);

    let mut assembler = Assembler::new();
    assembler.emit(Instruction::Nop);
    let program = assembler.assemble(0x1000).unwrap();
    assert_eq!(program.origin(), 0x1000);
    assert_eq!(program.bytes(), [0x4e, 0x71]);
    assert_eq!(program.instructions().len(), 1);
    assert_eq!(program.clone().into_bytes(), [0x4e, 0x71]);

    let DecodedInstruction {
        instruction,
        byte_len,
    } = decode_bytes(program.bytes()).unwrap();
    assert_eq!(instruction, Instruction::Nop);
    assert_eq!(byte_len, 2);
    assert_eq!(encode_bytes(&instruction).unwrap(), program.bytes());
    assert_eq!(
        verify_placed_program(program.bytes(), 0x1000)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        instruction.control_flow(0x1000).unwrap(),
        ControlFlow::FallThrough { next: 0x1002 }
    );
}
