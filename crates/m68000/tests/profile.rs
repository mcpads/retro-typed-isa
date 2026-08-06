use std::collections::BTreeSet;

use m68000::*;

#[test]
fn every_first_word_is_classified_and_known_words_round_trip() {
    let mut known = 0usize;
    let mut reserved = 0usize;
    for word in u16::MIN..=u16::MAX {
        match classify_word(word) {
            DecodedWord::Instruction(form) => {
                known += 1;
                let mut fixture = [0_u8; 24];
                fixture[..2].copy_from_slice(&word.to_be_bytes());
                let decoded = decode_bytes(&fixture).unwrap();
                assert_eq!(decoded.instruction.form(), form, "word 0x{word:04X}");
                let encoded = encode_bytes(&decoded.instruction).unwrap();
                assert_eq!(encoded, fixture[..decoded.byte_len], "word 0x{word:04X}");
            }
            DecodedWord::Reserved => reserved += 1,
        }
    }
    assert_eq!(known + reserved, 65_536);
    assert_eq!(known, 45_816);
    assert_eq!(reserved, 19_720);
}

#[test]
fn all_declared_mnemonics_have_an_encodable_first_word() {
    assert_eq!(AddressingMode::ALL.len(), 14);
    let actual: BTreeSet<_> = InstructionForm::all()
        .into_iter()
        .map(|form| form.mnemonic)
        .collect();
    assert_eq!(InstructionForm::all().len(), 1_365);
    let expected = BTreeSet::from([
        Mnemonic::Abcd,
        Mnemonic::Add,
        Mnemonic::Adda,
        Mnemonic::Addi,
        Mnemonic::Addq,
        Mnemonic::Addx,
        Mnemonic::And,
        Mnemonic::Andi,
        Mnemonic::Asl,
        Mnemonic::Asr,
        Mnemonic::Bcc,
        Mnemonic::Bchg,
        Mnemonic::Bclr,
        Mnemonic::Bra,
        Mnemonic::Bset,
        Mnemonic::Bsr,
        Mnemonic::Btst,
        Mnemonic::Chk,
        Mnemonic::Clr,
        Mnemonic::Cmp,
        Mnemonic::Cmpa,
        Mnemonic::Cmpi,
        Mnemonic::Cmpm,
        Mnemonic::Dbcc,
        Mnemonic::Divs,
        Mnemonic::Divu,
        Mnemonic::Eor,
        Mnemonic::Eori,
        Mnemonic::Exg,
        Mnemonic::Ext,
        Mnemonic::Illegal,
        Mnemonic::Jmp,
        Mnemonic::Jsr,
        Mnemonic::Lea,
        Mnemonic::Link,
        Mnemonic::Lsl,
        Mnemonic::Lsr,
        Mnemonic::Move,
        Mnemonic::Movea,
        Mnemonic::Movem,
        Mnemonic::Movep,
        Mnemonic::Moveq,
        Mnemonic::Muls,
        Mnemonic::Mulu,
        Mnemonic::Nbcd,
        Mnemonic::Neg,
        Mnemonic::Negx,
        Mnemonic::Nop,
        Mnemonic::Not,
        Mnemonic::Or,
        Mnemonic::Ori,
        Mnemonic::Pea,
        Mnemonic::Reset,
        Mnemonic::Rol,
        Mnemonic::Ror,
        Mnemonic::Roxl,
        Mnemonic::Roxr,
        Mnemonic::Rte,
        Mnemonic::Rtr,
        Mnemonic::Rts,
        Mnemonic::Sbcd,
        Mnemonic::Scc,
        Mnemonic::Stop,
        Mnemonic::Sub,
        Mnemonic::Suba,
        Mnemonic::Subi,
        Mnemonic::Subq,
        Mnemonic::Subx,
        Mnemonic::Swap,
        Mnemonic::Tas,
        Mnemonic::Trap,
        Mnemonic::Trapv,
        Mnemonic::Tst,
        Mnemonic::Unlk,
    ]);
    assert_eq!(actual, expected);
}

#[test]
fn source_repository_addressing_fixture_is_preserved() {
    let instructions = [
        Instruction::Move {
            size: OperandSize::Byte,
            source: EffectiveAddress::Immediate(Immediate::Byte(0)),
            destination: EffectiveAddress::AbsoluteLong(0x00a1_30f1),
        },
        Instruction::LoadEffectiveAddress {
            source: EffectiveAddress::AbsoluteLong(0x0034_0000),
            destination: AddressRegister::A2,
        },
        Instruction::AddAddress {
            size: AddressSize::Word,
            source: EffectiveAddress::DataRegister(DataRegister::D0),
            destination: AddressRegister::A2,
        },
        Instruction::Jump {
            target: EffectiveAddress::AbsoluteLong(0x0004_921c),
        },
    ];
    let actual: Vec<_> = instructions
        .iter()
        .flat_map(|instruction| encode_bytes(instruction).unwrap())
        .collect();
    assert_eq!(
        actual,
        [
            0x13, 0xfc, 0x00, 0x00, 0x00, 0xa1, 0x30, 0xf1, 0x45, 0xf9, 0x00, 0x34, 0x00, 0x00,
            0xd4, 0xc0, 0x4e, 0xf9, 0x00, 0x04, 0x92, 0x1c,
        ]
    );
}

#[test]
fn assembler_resolves_variable_length_instruction_addresses() {
    let mut assembler = Assembler::new();
    assembler
        .bcc(Condition::CarrySet, "target")
        .emit(Instruction::Move {
            size: OperandSize::Long,
            source: EffectiveAddress::Immediate(Immediate::Long(0x1234_5678)),
            destination: EffectiveAddress::DataRegister(DataRegister::D0),
        })
        .label("target")
        .emit(Instruction::ReturnFromSubroutine);

    let program = assembler.assemble(0x0010_0000).unwrap();
    assert_eq!(
        program.bytes,
        [
            0x65, 0x00, 0x00, 0x08, 0x20, 0x3c, 0x12, 0x34, 0x56, 0x78, 0x4e, 0x75,
        ]
    );
    assert_eq!(program.instructions[2].address, 0x0010_000a);
}

#[test]
fn rejects_non_mc68000_extensions_and_illegal_operands() {
    assert_eq!(
        decode_bytes(&[0x30, 0x30, 0x01, 0x00]),
        Err(DecodeError::ReservedExtension { word: 0x0100 })
    );
    assert!(matches!(
        encode_bytes(&Instruction::Move {
            size: OperandSize::Byte,
            source: EffectiveAddress::AddressRegister(AddressRegister::A0),
            destination: EffectiveAddress::DataRegister(DataRegister::D0),
        }),
        Err(EncodeError::InvalidEffectiveAddress { .. })
    ));
    assert_eq!(
        encode_bytes(&Instruction::Branch {
            kind: BranchKind::Always,
            displacement: BranchDisplacement::Byte(0),
        }),
        Err(EncodeError::ZeroByteBranchDisplacement)
    );
}

#[test]
fn mc68000_revision_and_addressing_boundaries_are_explicit() {
    assert_eq!(
        decode_bytes(&[0x90, 0x48]).unwrap().instruction,
        Instruction::Subtract {
            size: OperandSize::Word,
            direction: DataDirection::ToDataRegister,
            data_register: DataRegister::D0,
            effective_address: EffectiveAddress::AddressRegister(AddressRegister::A0),
        }
    );
    assert_eq!(
        decode_bytes(&[0x01, 0x3a, 0, 0]).unwrap().instruction,
        Instruction::Bit {
            operation: BitOperation::Test,
            bit_number: BitNumber::DataRegister(DataRegister::D0),
            destination: EffectiveAddress::ProgramCounterDisplacement(0),
        }
    );
    assert_eq!(
        decode_bytes(&[0x60, 0xff]).unwrap().instruction,
        Instruction::Branch {
            kind: BranchKind::Always,
            displacement: BranchDisplacement::Byte(-1),
        }
    );

    for later_cpu_word in [0x0cfc, 0x0efc, 0x42c0, 0x4848, 0x4e74, 0x4e7a, 0xe8c0] {
        assert_eq!(classify_word(later_cpu_word), DecodedWord::Reserved);
    }
}

#[test]
fn source_repository_hook_fixture_decodes_and_round_trips() {
    let bytes = [
        0x2f, 0x00, 0x30, 0x01, 0x57, 0x00, 0x90, 0x68, 0x00, 0x38, 0xe2, 0x48, 0x30, 0x31, 0x00,
        0x00, 0x20, 0x1f, 0x3a, 0x28, 0x00, 0x4c, 0x00, 0x7c, 0x07, 0x00, 0x02, 0x7c, 0xf8, 0xff,
    ];
    let placed = verify_placed_program(&bytes, 0x0008_0000).unwrap();
    assert_eq!(placed.len(), 10);
    let rebuilt: Vec<_> = placed
        .iter()
        .flat_map(|instruction| instruction.bytes.iter().copied())
        .collect();
    assert_eq!(rebuilt, bytes);
}

#[test]
fn movem_predecrement_mask_is_semantically_normalized() {
    let instruction = Instruction::MoveMultiple {
        size: AddressSize::Long,
        direction: MoveMultipleDirection::RegistersToMemory,
        registers: RegisterMask::from_bits(0x0003),
        effective_address: EffectiveAddress::AddressPredecrement(AddressRegister::A7),
    };
    let bytes = encode_bytes(&instruction).unwrap();
    assert_eq!(bytes, [0x48, 0xe7, 0xc0, 0x00]);
    assert_eq!(decode_bytes(&bytes).unwrap().instruction, instruction);
}
