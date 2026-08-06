use psx_r3000a::{
    DecodeError, EncodeError, GteCommand, GteOpcode, Instruction, Register, decode_bytes,
    decode_le_bytes, encode_word,
};

const PC: u32 = 0x8001_0000;

fn assert_field_out_of_range(instruction: Instruction, field: &'static str) {
    assert!(matches!(
        encode_word(&instruction, PC),
        Err(EncodeError::FieldOutOfRange {
            field: actual,
            ..
        }) if actual == field
    ));
}

#[test]
fn every_bounded_encoding_field_rejects_its_first_invalid_value() {
    assert_field_out_of_range(
        Instruction::Sll {
            rd: Register::T0,
            rt: Register::T1,
            shift: 32,
        },
        "shift",
    );
    assert_field_out_of_range(Instruction::Syscall { code: 0x10_0000 }, "system code");
    assert_field_out_of_range(
        Instruction::Mfc0 {
            rt: Register::T0,
            rd: 32,
        },
        "coprocessor register",
    );
    assert_field_out_of_range(
        Instruction::Lwc2 {
            rt: 32,
            base: Register::T0,
            offset: 0,
        },
        "COP2 register",
    );

    let valid_gte = GteCommand {
        opcode: GteOpcode::Mvmva,
        fake: 0,
        shift_fraction: false,
        matrix: 0,
        vector: 0,
        control_vector: 0,
        limit_unsigned: false,
    };
    assert_field_out_of_range(
        Instruction::Gte(GteCommand {
            fake: 32,
            ..valid_gte
        }),
        "GTE fake opcode",
    );
    assert_field_out_of_range(
        Instruction::Gte(GteCommand {
            matrix: 4,
            ..valid_gte
        }),
        "GTE matrix",
    );
    assert_field_out_of_range(
        Instruction::Gte(GteCommand {
            vector: 4,
            ..valid_gte
        }),
        "GTE vector",
    );
    assert_field_out_of_range(
        Instruction::Gte(GteCommand {
            control_vector: 4,
            ..valid_gte
        }),
        "GTE control vector",
    );
}

#[test]
fn placed_control_flow_rejects_unrepresentable_targets() {
    assert!(matches!(
        encode_word(
            &Instruction::Beq {
                rs: Register::T0,
                rt: Register::T1,
                target: PC + 2,
            },
            PC,
        ),
        Err(EncodeError::UnalignedControlFlow { .. })
    ));
    assert!(matches!(
        encode_word(
            &Instruction::Beq {
                rs: Register::T0,
                rt: Register::T1,
                target: PC + 4 + 0x8000 * 4,
            },
            PC,
        ),
        Err(EncodeError::BranchOutOfRange { .. })
    ));
    assert!(matches!(
        encode_word(
            &Instruction::J {
                target: 0x9000_0000,
            },
            PC,
        ),
        Err(EncodeError::JumpRegionMismatch { .. })
    ));
}

#[test]
fn byte_decoder_rejects_partial_instruction_words() {
    assert_eq!(
        decode_bytes(&[0; 3], PC),
        Err(DecodeError::TruncatedInput { len: 3 })
    );
    assert!(matches!(
        decode_le_bytes(&[0, 0, 0], PC),
        Err(DecodeError::TruncatedInput { len: 3 })
    ));
}
