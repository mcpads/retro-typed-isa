use sm83::{
    Assembler, BitIndex, ByteOperand, Condition, Instruction, PROFILE_ID, Register16, Sm83,
    decode_bytes, encode_bytes,
};
use typed_isa_core::TypedIsa;

#[test]
fn consumer_can_generate_and_verify_sm83_code() {
    assert_eq!(PROFILE_ID, "sm83");

    let instruction = Instruction::Load16Immediate(Register16::Hl, 0xc000);
    assert_eq!(encode_bytes(&instruction), [0x21, 0x00, 0xc0]);
    assert_eq!(
        decode_bytes(&[0x21, 0x00, 0xc0])
            .unwrap()
            .into_instruction(),
        instruction
    );

    let mut assembler = Assembler::new();
    assembler
        .label("loop")
        .emit(instruction)
        .emit(Instruction::Increment8(ByteOperand::IndirectHl))
        .emit(Instruction::Bit(BitIndex::B7, ByteOperand::IndirectHl))
        .jump_relative_if(Condition::Nz, "loop");

    assert_eq!(
        assembler.assemble(0x4000).unwrap().bytes(),
        [0x21, 0x00, 0xc0, 0x34, 0xcb, 0x7e, 0x20, 0xf8]
    );
}

#[test]
fn shared_protocol_decodes_only_the_leading_instruction() {
    let decoded = Sm83::decode_one(&[0x21, 0x00, 0xc0, 0x00], &0x4000).unwrap();

    assert_eq!(decoded.source().as_slice(), [0x21, 0x00, 0xc0]);
    assert_eq!(
        decoded.instruction(),
        &Instruction::Load16Immediate(Register16::Hl, 0xc000)
    );
    assert_eq!(
        Sm83::encode_one(decoded.instruction(), &0x4000)
            .unwrap()
            .as_slice(),
        decoded.source().as_slice()
    );
}
