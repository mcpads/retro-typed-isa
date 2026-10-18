use typed_isa_core::TypedIsa;
use v810::{FloatingOperation, Instruction, Register, V810, decode_bytes, encode_bytes};

fn register(index: u8) -> Register {
    Register::new(index).unwrap()
}

#[test]
fn protocol_decode_requires_an_even_location() {
    for location in [1, 0x1001, u32::MAX] {
        assert_eq!(
            V810::decode_one(&[0x00, 0x9a], &location),
            Err(v810::DecodeError::UnalignedCodeLocation { location }),
        );
    }
    for location in [0, 2, 0x1000] {
        let decoded = V810::decode_one(&[0x00, 0x9a, 0xff], &location).unwrap();
        assert_eq!(decoded.instruction(), &Instruction::Nop);
        assert_eq!(decoded.source().as_slice(), [0x00, 0x9a]);
    }
}

#[test]
fn protocol_encode_requires_an_even_location() {
    for location in [1, 0x1001, u32::MAX] {
        assert_eq!(
            V810::encode_one(&Instruction::Nop, &location),
            Err(v810::EncodeError::UnalignedCodeLocation { location }),
        );
    }
    for location in [0, 2, 0x1000] {
        assert_eq!(
            V810::encode_one(&Instruction::Nop, &location)
                .unwrap()
                .as_slice(),
            [0x00, 0x9a],
        );
    }
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
    let decoded = V810::decode_one(&[0x41, 0xa4, 0xfc, 0xff, 0x00, 0x00], &0).unwrap();

    assert_eq!(decoded.source().as_slice(), [0x41, 0xa4, 0xfc, 0xff]);
    assert_eq!(
        V810::encode_one(decoded.instruction(), &0)
            .unwrap()
            .as_slice(),
        decoded.source().as_slice()
    );
}
