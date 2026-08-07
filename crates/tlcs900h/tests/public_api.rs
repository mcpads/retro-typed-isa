use tlcs900h::{
    ArithmeticOperation, BitOperation, CurrentRegister, DestinationMemoryInstruction,
    DestinationMemoryOperation, Instruction, MainInstruction, MemoryAddress, OperandSize,
    RegisterEncoding, RegisterInstruction, RegisterOperation, Tlcs900H, decode_bytes, encode_bytes,
};
use typed_isa_core::TypedIsa;

fn current(index: u8) -> CurrentRegister {
    CurrentRegister::new(index).unwrap()
}

#[test]
fn manual_code_examples_lower_to_little_endian_bytes() {
    let add_a_b = Instruction::Register(RegisterOperation {
        size: OperandSize::Byte,
        register: RegisterEncoding::Current(current(2)),
        instruction: RegisterInstruction::ArithmeticCurrent {
            operation: ArithmeticOperation::Add,
            other: current(1),
        },
    });
    assert_eq!(encode_bytes(&add_a_b).unwrap(), [0xca, 0x81]);

    let store_a = Instruction::DestinationMemory(DestinationMemoryOperation {
        destination: MemoryAddress::Direct24(0x12_3456),
        instruction: DestinationMemoryInstruction::LoadFromCurrent {
            size: OperandSize::Byte,
            source: current(1),
        },
    });
    assert_eq!(
        encode_bytes(&store_a).unwrap(),
        [0xf2, 0x56, 0x34, 0x12, 0x41]
    );
}

#[test]
fn deleted_tlcs900_modes_are_rejected_as_h_dummies() {
    assert!(decode_bytes(&[0x01]).is_err());
    assert!(decode_bytes(&[0x04]).is_err());
}

#[test]
fn load_extract_preserves_all_architecturally_ignored_filler_bytes() {
    let bytes = [0xf7, 0xaa, 0x6a, 0xbb, 0x17, 0xcc];
    let decoded = decode_bytes(&bytes).unwrap();

    assert_eq!(encode_bytes(decoded.instruction()).unwrap(), bytes);
    assert_eq!(
        decoded.into_instruction(),
        Instruction::Main(MainInstruction::LoadExtract {
            address: 0x6a,
            value: 0x17,
            filler: [0xaa, 0xbb, 0xcc],
        })
    );
}

#[test]
fn bit_operations_remain_typed_through_memory_lowering() {
    let instruction = Instruction::DestinationMemory(DestinationMemoryOperation {
        destination: MemoryAddress::CurrentIndirect(current(3)),
        instruction: DestinationMemoryInstruction::BitImmediate {
            operation: BitOperation::TestAndSet,
            bit: 5,
        },
    });

    assert_eq!(encode_bytes(&instruction).unwrap(), [0xb3, 0xad]);
    assert_eq!(
        decode_bytes(&[0xb3, 0xad]).unwrap().into_instruction(),
        instruction
    );
}

#[test]
fn shared_protocol_consumes_only_the_leading_instruction() {
    let decoded = Tlcs900H::decode_one(&[0x30, 0x34, 0x12, 0xff], 0).unwrap();

    assert_eq!(decoded.source().as_slice(), [0x30, 0x34, 0x12]);
    assert_eq!(
        Tlcs900H::encode_one(decoded.instruction(), 0)
            .unwrap()
            .as_slice(),
        decoded.source().as_slice()
    );
}
