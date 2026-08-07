use huc6280::{
    AddressingMode, ControlFlow, DecodeError, EncodeError, Instruction, Mnemonic, Operand,
    decode_bytes, encode_bytes, format_bytes, format_instruction, instruction_form,
};

#[test]
fn every_multibyte_opcode_rejects_each_truncated_prefix() {
    let padded = [0u8; 7];
    for opcode in 0..=u8::MAX {
        let Some(form) = instruction_form(opcode) else {
            continue;
        };
        let expected = form.encoded_len();
        for found in 1..expected {
            let mut bytes = padded;
            bytes[0] = opcode;
            assert_eq!(
                decode_bytes(&bytes[..found]),
                Err(DecodeError::Truncated {
                    opcode,
                    expected,
                    found,
                }),
                "opcode {opcode:#04x}, prefix length {found}"
            );
        }
    }
}

#[test]
fn brk_is_one_encoded_byte_but_reports_interrupt_flow() {
    let instruction = decode_bytes(&[0x00]).unwrap();
    assert_eq!(instruction.encoded_len(), 1);
    assert_eq!(encode_bytes(&instruction).unwrap(), [0x00]);
    assert_eq!(instruction.fallthrough_address(0xFFFF), 0x0000);
    assert_eq!(instruction.control_flow(0x8000), ControlFlow::Interrupt);
}

#[test]
fn operand_shape_and_unknown_forms_fail_closed() {
    assert!(matches!(
        Instruction::new(
            Mnemonic::Lda,
            AddressingMode::Immediate,
            Operand::Word(0x1234)
        ),
        Err(EncodeError::OperandMismatch { .. })
    ));
    assert_eq!(
        Instruction::new(Mnemonic::Lda, AddressingMode::Implied, Operand::None),
        Err(EncodeError::UnknownEncoding {
            mnemonic: Mnemonic::Lda,
            mode: AddressingMode::Implied,
        })
    );
    assert_eq!(
        Instruction::new(
            Mnemonic::Rmb { bit: 8 },
            AddressingMode::ZeroPage,
            Operand::Byte(0x10),
        ),
        Err(EncodeError::InvalidBitIndex { bit: 8 })
    );
}

#[test]
fn long_and_special_forms_lower_in_documented_byte_order() {
    let transfer = Instruction::new(
        Mnemonic::Tii,
        AddressingMode::BlockTransfer,
        Operand::BlockTransfer {
            source: 0x1234,
            destination: 0xABCD,
            length: 0x0080,
        },
    )
    .unwrap();
    assert_eq!(
        encode_bytes(&transfer).unwrap(),
        [0x73, 0x34, 0x12, 0xCD, 0xAB, 0x80, 0x00]
    );

    let tst = Instruction::new(
        Mnemonic::Tst,
        AddressingMode::ImmediateAbsoluteX,
        Operand::ImmediateAbsolute {
            immediate: 0xF0,
            address: 0x2468,
        },
    )
    .unwrap();
    assert_eq!(encode_bytes(&tst).unwrap(), [0xB3, 0xF0, 0x68, 0x24]);
}

#[test]
fn placement_derived_control_flow_uses_wrapping_16_bit_addresses() {
    let conditional = Instruction::new(
        Mnemonic::Bne,
        AddressingMode::Relative,
        Operand::Relative(-4),
    )
    .unwrap();
    assert_eq!(
        conditional.control_flow(0x0001),
        ControlFlow::Branch {
            target: 0xFFFF,
            fallthrough: Some(0x0003),
        }
    );

    let call = Instruction::new(
        Mnemonic::Bsr,
        AddressingMode::Relative,
        Operand::Relative(0x20),
    )
    .unwrap();
    assert_eq!(
        call.control_flow(0xFFF0),
        ControlFlow::Call {
            target: 0x0012,
            return_address: 0xFFF2,
        }
    );

    let indirect = Instruction::new(
        Mnemonic::Jmp,
        AddressingMode::AbsoluteIndirect,
        Operand::Word(0x1234),
    )
    .unwrap();
    assert_eq!(
        indirect.control_flow(0x8000),
        ControlFlow::Jump { target: None }
    );
}

#[test]
fn display_uses_pceas_style_without_affecting_encoding() {
    let instruction = decode_bytes(&[0xAF, 0x42, 0xFC]).unwrap();
    assert_eq!(format_instruction(instruction, 0x8000), "BBS2 $42,$7FFF");
    assert_eq!(format_bytes(&[0xAF, 0x42, 0xFC], 7), "AF 42 FC            ");
}
