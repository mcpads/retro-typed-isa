use typed_isa_core::TypedIsa;
use v30mz::{
    DecodeError, Instruction, InstructionAddress, Operand, Register16, V30Mz, VSeriesInstruction,
    decode_bytes, encode_bytes,
};

// NEC A13761EJ1V0UM00 Appendix A assigns every other primary byte to an
// instruction. These eight bytes are the entire undefined primary set for
// V30MZ; 0F, 64/65, and the 66/67 FPO2 forms are defined by V30HL but explicitly
// removed by section 3.6.1 remark 1.
const MANUAL_UNDEFINED_PRIMARY: [u8; 8] = [0x0f, 0x63, 0x64, 0x65, 0x66, 0x67, 0xd6, 0xf1];
const MANUAL_PREFIX_PRIMARY: [u8; 7] = [0x26, 0x2e, 0x36, 0x3e, 0xf0, 0xf2, 0xf3];

#[test]
fn every_primary_opcode_matches_the_manual_partition() {
    let mut instruction_count = 0;
    let mut prefix_count = 0;
    let mut undefined_count = 0;

    for opcode in u8::MIN..=u8::MAX {
        if MANUAL_UNDEFINED_PRIMARY.contains(&opcode) {
            undefined_count += 1;
            assert!(matches!(
                decode_bytes(&[opcode, 0, 0, 0, 0, 0]),
                Err(DecodeError::UndefinedPrimaryOpcode { opcode: rejected }) if rejected == opcode
            ));
        } else if MANUAL_PREFIX_PRIMARY.contains(&opcode) {
            prefix_count += 1;
            let bytes: &[u8] = match opcode {
                0xf0 => &[0xf0, 0x90],
                0xf2 => &[0xf2, 0xa6],
                0xf3 => &[0xf3, 0xa4],
                _ => &[opcode, 0xa4],
            };
            assert!(decode_bytes(bytes).is_ok(), "prefix {opcode:#04x}");
        } else {
            instruction_count += 1;
            let mut bytes = [0_u8; 15];
            bytes[0] = opcode;
            assert!(
                decode_bytes(&bytes).is_ok(),
                "manual instruction primary {opcode:#04x} did not decode"
            );
        }
    }

    assert_eq!(
        (instruction_count, prefix_count, undefined_count),
        (241, 7, 8)
    );
}

#[test]
fn v30hl_only_extensions_cannot_enter_the_typed_profile() {
    for bytes in [
        &[0x0f, 0x10, 0xc0][..],
        &[0x0f, 0xff, 0x80][..],
        &[0x64, 0xa6][..],
        &[0x65, 0xa6][..],
    ] {
        assert!(matches!(
            decode_bytes(bytes),
            Err(DecodeError::UndefinedPrimaryOpcode { .. })
        ));
        let v30_instruction = v30::decode_bytes(bytes).unwrap().instruction;
        assert!(Instruction::new(v30_instruction).is_err());
    }
}

#[test]
fn standard_v30mz_forms_round_trip_through_the_profile() {
    let instruction = Instruction::new(VSeriesInstruction::Mov {
        dest: Operand::Reg16(Register16::AX),
        src: Operand::Reg16(Register16::BX),
    })
    .unwrap();
    assert_eq!(encode_bytes(&instruction).unwrap(), [0x89, 0xd8]);
    assert_eq!(
        decode_bytes(&[0x8b, 0xc3]).unwrap().instruction(),
        &instruction
    );
}

#[test]
fn profile_implements_the_shared_static_protocol() {
    assert_eq!(V30Mz::PROFILE_ID, "v30mz");
    let decoded = V30Mz::decode_one(&[0x90], 0).unwrap();
    assert_eq!(decoded.source().as_slice(), [0x90]);
    assert_eq!(
        V30Mz::encode_one(decoded.instruction(), 0)
            .unwrap()
            .as_slice(),
        [0x90]
    );
}

#[test]
fn checked_assembler_keeps_relative_targets_inside_the_exact_profile() {
    let mut assembler = v30mz::Assembler::new();
    assembler.label("again");
    assembler
        .emit(Instruction::new(VSeriesInstruction::Nop).unwrap())
        .emit_jump_short("again");
    let program = assembler
        .assemble(InstructionAddress {
            seg: 0x2000,
            off: 0x100,
        })
        .unwrap();
    assert_eq!(program.bytes(), [0x90, 0xeb, 0xfd]);
    assert_eq!(program.instruction_spans().len(), 2);
}
