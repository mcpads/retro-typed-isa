use std::collections::HashSet;

use huc6280::{
    AddressingMode, DecodeError, EncodeError, Instruction, InstructionForm, Mnemonic, Operand,
    decode_bytes, encode_bytes, instruction_form, opcode_is_documented,
};

const UNDOCUMENTED_OPCODES: [u8; 22] = [
    0x0B, 0x1B, 0x2B, 0x33, 0x3B, 0x4B, 0x5B, 0x5C, 0x63, 0x6B, 0x7B, 0x8B, 0x9B, 0xAB, 0xBB, 0xCB,
    0xDB, 0xDC, 0xE2, 0xEB, 0xFB, 0xFC,
];

#[test]
fn opcode_and_semantic_denominators_are_explicit() {
    let forms: Vec<_> = InstructionForm::all().collect();
    let unique: HashSet<_> = forms.iter().copied().collect();
    assert_eq!(forms.len(), 234);
    assert_eq!(unique.len(), 234);

    let undocumented: Vec<_> = (0..=u8::MAX)
        .filter(|opcode| !opcode_is_documented(*opcode))
        .collect();
    assert_eq!(undocumented, UNDOCUMENTED_OPCODES);
    assert_eq!(256 - undocumented.len(), 234);

    let nop_form = InstructionForm::new(Mnemonic::Nop, AddressingMode::Implied);
    let nop_slots: Vec<_> = (0..=u8::MAX)
        .filter(|opcode| instruction_form(*opcode) == Some(nop_form))
        .collect();
    assert_eq!(nop_slots, [0xEA]);
}

#[test]
fn every_documented_opcode_round_trips_without_selector_canonicalization() {
    for opcode in 0..=u8::MAX {
        let bytes = [opcode, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC];
        if !opcode_is_documented(opcode) {
            assert_eq!(
                decode_bytes(&bytes),
                Err(DecodeError::UndocumentedOpcode { opcode })
            );
            continue;
        }
        let instruction = decode_bytes(&bytes).unwrap();
        assert_eq!(instruction.opcode(), opcode);
        assert_eq!(Some(instruction.form()), instruction_form(opcode));
        assert_eq!(
            encode_bytes(&instruction).unwrap(),
            bytes[..instruction.encoded_len()]
        );
    }
}

#[test]
fn every_semantic_form_has_a_canonical_documented_encoding() {
    for form in InstructionForm::all() {
        let instruction = Instruction::new(form.mnemonic, form.mode, sample_operand(form.mode))
            .unwrap_or_else(|error| panic!("missing canonical encoding for {form:?}: {error}"));
        assert_eq!(instruction.form(), form);
        assert!(instruction.opcode_is_documented());
        let encoded = encode_bytes(&instruction).unwrap();
        assert_eq!(decode_bytes(&encoded).unwrap(), instruction);
    }
}

#[test]
fn undocumented_selectors_cannot_be_constructed_as_nops() {
    let canonical =
        Instruction::new(Mnemonic::Nop, AddressingMode::Implied, Operand::None).unwrap();
    assert_eq!(canonical.opcode(), 0xEA);
    assert_eq!(encode_bytes(&canonical).unwrap(), [0xEA]);

    for opcode in UNDOCUMENTED_OPCODES {
        assert_eq!(
            Instruction::from_opcode(opcode, Operand::None),
            Err(EncodeError::UndocumentedOpcode { opcode })
        );
    }
}

fn sample_operand(mode: AddressingMode) -> Operand {
    match mode {
        AddressingMode::Implied | AddressingMode::Accumulator => Operand::None,
        AddressingMode::Immediate
        | AddressingMode::ZeroPage
        | AddressingMode::ZeroPageX
        | AddressingMode::ZeroPageY
        | AddressingMode::ZeroPageIndexedIndirectX
        | AddressingMode::ZeroPageIndirectIndexedY
        | AddressingMode::ZeroPageIndirect => Operand::Byte(0xA5),
        AddressingMode::Absolute
        | AddressingMode::AbsoluteX
        | AddressingMode::AbsoluteY
        | AddressingMode::AbsoluteIndirect
        | AddressingMode::AbsoluteIndexedIndirectX => Operand::Word(0xA55A),
        AddressingMode::Relative => Operand::Relative(-7),
        AddressingMode::ZeroPageRelative => Operand::ZeroPageRelative {
            address: 0x44,
            displacement: 12,
        },
        AddressingMode::BlockTransfer => Operand::BlockTransfer {
            source: 0x1234,
            destination: 0x5678,
            length: 0x09AB,
        },
        AddressingMode::ImmediateZeroPage | AddressingMode::ImmediateZeroPageX => {
            Operand::ImmediateZeroPage {
                immediate: 0xAA,
                address: 0x55,
            }
        }
        AddressingMode::ImmediateAbsolute | AddressingMode::ImmediateAbsoluteX => {
            Operand::ImmediateAbsolute {
                immediate: 0xAA,
                address: 0x55CC,
            }
        }
    }
}
