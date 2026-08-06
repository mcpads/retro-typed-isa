use sh2::{
    DecodeError, DecodedWord, EncodeError, Instruction, InstructionForm, Register, classify_word,
    decode_bytes, decode_word, encode_bytes, encode_word,
};

#[test]
fn byte_lowering_is_big_endian() {
    let instruction = Instruction::MovImm {
        imm: -2,
        rn: Register::R3,
    };
    assert_eq!(encode_word(&instruction).unwrap(), 0xE3FE);
    assert_eq!(encode_bytes(&instruction).unwrap(), [0xE3, 0xFE]);
    assert_eq!(decode_bytes(&[0xE3, 0xFE]).unwrap(), instruction);
    assert_eq!(decode_bytes(&[0xE3, 0xFE, 0xFF]).unwrap(), instruction);
}

#[test]
fn truncated_and_reserved_input_fail_closed() {
    assert_eq!(
        decode_bytes(&[]),
        Err(DecodeError::TruncatedInput { len: 0 })
    );
    assert_eq!(
        decode_bytes(&[0x00]),
        Err(DecodeError::TruncatedInput { len: 1 })
    );
    assert_eq!(
        decode_word(0xFFFF),
        Err(DecodeError::UnknownWord { word: 0xFFFF })
    );
}

#[test]
fn sh_dsp_stc_and_sts_selectors_are_not_aliased_into_sh2() {
    for register in 0u16..16 {
        for selector in 3u16..16 {
            for low_nibble in [0x2, 0xA] {
                let word = register << 8 | selector << 4 | low_nibble;
                assert_eq!(classify_word(word), DecodedWord::Unknown { word });
            }
        }
    }

    assert_eq!(
        decode_word(0x0F22).unwrap(),
        Instruction::StcVbr { rn: Register::R15 }
    );
    assert_eq!(
        decode_word(0x0F2A).unwrap(),
        Instruction::StsPr { rn: Register::R15 }
    );
}

#[test]
fn encoder_rejects_out_of_range_encoded_fields() {
    assert_eq!(
        encode_word(&Instruction::MovBDispStore {
            disp: 16,
            rn: Register::R0,
        }),
        Err(EncodeError::FourBitDisplacementOutOfRange {
            form: InstructionForm::MovBDispStore,
            displacement: 16,
        })
    );
    assert_eq!(
        encode_word(&Instruction::Bra { disp: 2048 }),
        Err(EncodeError::TwelveBitDisplacementOutOfRange { displacement: 2048 })
    );
    assert!(Register::new(15).is_some());
    assert!(Register::new(16).is_none());
}
