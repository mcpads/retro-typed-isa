use psx_r3000a::{
    Assembler, Instruction, InstructionForm, PROFILE_ID, PsxR3000A, Register, encode_bytes,
    verify_placed_program,
};
use typed_isa_core::{ControlAction, ControlTarget, StaticSemantics};

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
    assert_eq!(program.instruction_spans()[1].offset, 4);
    assert_eq!(program.instruction_spans()[1].location, origin + 4);
    assert_eq!(
        program.instruction_spans()[1].bytes.as_slice(),
        &program.bytes()[4..8]
    );
    assert_eq!(
        encode_bytes(&decoded[0], origin).unwrap(),
        program.bytes()[..4]
    );
    assert_eq!(decoded[1].form(), InstructionForm::Bne);
    let semantics = PsxR3000A::semantics(&decoded[1], &(origin + 4)).unwrap();
    assert_eq!(
        semantics.control_flow.action,
        ControlAction::Transfer {
            target: ControlTarget::Direct(origin)
        }
    );
    assert_eq!(semantics.control_flow.fallthrough, Some(origin + 12));
}
