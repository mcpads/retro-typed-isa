use typed_isa_core::TypedIsa;
use v810::{FloatingOperation, Instruction, Register, V810, decode_bytes, encode_bytes};

fn register(index: u8) -> Register {
    Register::new(index).unwrap()
}

#[test]
fn typed_instructions_lower_to_little_endian_halfwords() {
    let instruction = Instruction::AddImmediate {
        immediate: -4,
        source: register(1),
        destination: register(2),
    };

    assert_eq!(
        encode_bytes(&instruction).unwrap(),
        [0x41, 0xa4, 0xfc, 0xff]
    );
    assert_eq!(
        decode_bytes(&[0x41, 0xa4, 0xfc, 0xff])
            .unwrap()
            .into_instruction(),
        instruction
    );
}

#[test]
fn extension_instruction_preserves_subopcode_and_reserved_zeroes() {
    let instruction = Instruction::Floating {
        operation: FloatingOperation::Add,
        source: register(3),
        destination: register(4),
    };

    assert_eq!(
        encode_bytes(&instruction).unwrap(),
        [0x83, 0xf8, 0x00, 0x10]
    );
    assert_eq!(
        decode_bytes(&[0x83, 0xf8, 0x00, 0x10])
            .unwrap()
            .into_instruction(),
        instruction
    );
}

#[test]
fn shared_protocol_consumes_one_instruction_from_a_longer_source() {
    let decoded = V810::decode_one(&[0x41, 0xa4, 0xfc, 0xff, 0x00, 0x00], 0).unwrap();

    assert_eq!(decoded.source().as_slice(), [0x41, 0xa4, 0xfc, 0xff]);
    assert_eq!(
        V810::encode_one(decoded.instruction(), 0)
            .unwrap()
            .as_slice(),
        decoded.source().as_slice()
    );
}
