use std::collections::BTreeSet;

use psx_r3000a::{
    GteCommand, GteOpcode, Instruction, InstructionForm, PsxR3000A, Register, decode_word,
    encode_word,
};
use typed_isa_core::StaticSemantics;

#[test]
fn every_manual_instruction_form_has_a_roundtrip_and_static_semantics() {
    let pc = 0x8001_0000;
    let a = Register::T0;
    let b = Register::T1;
    let d = Register::T2;
    let near = pc + 0x40;
    let mut instructions = vec![
        Instruction::Add {
            rd: d,
            rs: a,
            rt: b,
        },
        Instruction::Addu {
            rd: d,
            rs: a,
            rt: b,
        },
        Instruction::Sub {
            rd: d,
            rs: a,
            rt: b,
        },
        Instruction::Subu {
            rd: d,
            rs: a,
            rt: b,
        },
        Instruction::And {
            rd: d,
            rs: a,
            rt: b,
        },
        Instruction::Or {
            rd: d,
            rs: a,
            rt: b,
        },
        Instruction::Xor {
            rd: d,
            rs: a,
            rt: b,
        },
        Instruction::Nor {
            rd: d,
            rs: a,
            rt: b,
        },
        Instruction::Slt {
            rd: d,
            rs: a,
            rt: b,
        },
        Instruction::Sltu {
            rd: d,
            rs: a,
            rt: b,
        },
        Instruction::Addi {
            rt: d,
            rs: a,
            immediate: -123,
        },
        Instruction::Addiu {
            rt: d,
            rs: a,
            immediate: -123,
        },
        Instruction::Andi {
            rt: d,
            rs: a,
            immediate: 0x89ab,
        },
        Instruction::Ori {
            rt: d,
            rs: a,
            immediate: 0x89ab,
        },
        Instruction::Xori {
            rt: d,
            rs: a,
            immediate: 0x89ab,
        },
        Instruction::Lui {
            rt: d,
            immediate: 0x8001,
        },
        Instruction::Slti {
            rt: d,
            rs: a,
            immediate: -123,
        },
        Instruction::Sltiu {
            rt: d,
            rs: a,
            immediate: -123,
        },
        Instruction::Sll {
            rd: d,
            rt: b,
            shift: 7,
        },
        Instruction::Srl {
            rd: d,
            rt: b,
            shift: 7,
        },
        Instruction::Sra {
            rd: d,
            rt: b,
            shift: 7,
        },
        Instruction::Sllv {
            rd: d,
            rt: b,
            rs: a,
        },
        Instruction::Srlv {
            rd: d,
            rt: b,
            rs: a,
        },
        Instruction::Srav {
            rd: d,
            rt: b,
            rs: a,
        },
        Instruction::Mult { rs: a, rt: b },
        Instruction::Multu { rs: a, rt: b },
        Instruction::Div { rs: a, rt: b },
        Instruction::Divu { rs: a, rt: b },
        Instruction::Mfhi { rd: d },
        Instruction::Mthi { rs: a },
        Instruction::Mflo { rd: d },
        Instruction::Mtlo { rs: a },
        Instruction::Lb {
            rt: d,
            base: a,
            offset: -16,
        },
        Instruction::Lbu {
            rt: d,
            base: a,
            offset: -16,
        },
        Instruction::Lh {
            rt: d,
            base: a,
            offset: -16,
        },
        Instruction::Lhu {
            rt: d,
            base: a,
            offset: -16,
        },
        Instruction::Lw {
            rt: d,
            base: a,
            offset: -16,
        },
        Instruction::Lwl {
            rt: d,
            base: a,
            offset: -16,
        },
        Instruction::Lwr {
            rt: d,
            base: a,
            offset: -16,
        },
        Instruction::Sb {
            rt: d,
            base: a,
            offset: -16,
        },
        Instruction::Sh {
            rt: d,
            base: a,
            offset: -16,
        },
        Instruction::Sw {
            rt: d,
            base: a,
            offset: -16,
        },
        Instruction::Swl {
            rt: d,
            base: a,
            offset: -16,
        },
        Instruction::Swr {
            rt: d,
            base: a,
            offset: -16,
        },
        Instruction::Beq {
            rs: a,
            rt: b,
            target: near,
        },
        Instruction::Bne {
            rs: a,
            rt: b,
            target: near,
        },
        Instruction::Blez {
            rs: a,
            target: near,
        },
        Instruction::Bgtz {
            rs: a,
            target: near,
        },
        Instruction::Bltz {
            rs: a,
            target: near,
        },
        Instruction::Bgez {
            rs: a,
            target: near,
        },
        Instruction::Bltzal {
            rs: a,
            target: near,
        },
        Instruction::Bgezal {
            rs: a,
            target: near,
        },
        Instruction::J {
            target: 0x8002_0000,
        },
        Instruction::Jal {
            target: 0x8002_0000,
        },
        Instruction::Jr { rs: Register::RA },
        Instruction::Jalr {
            rd: Register::RA,
            rs: a,
        },
        Instruction::Syscall { code: 0x54321 },
        Instruction::Break { code: 0x12345 },
        Instruction::Mfc0 { rt: d, rd: 12 },
        Instruction::Mtc0 { rt: d, rd: 12 },
        Instruction::Rfe,
        Instruction::Mfc2 { rt: d, rd: 6 },
        Instruction::Mtc2 { rt: d, rd: 6 },
        Instruction::Cfc2 { rt: d, rd: 6 },
        Instruction::Ctc2 { rt: d, rd: 6 },
        Instruction::Lwc2 {
            rt: 6,
            base: a,
            offset: -16,
        },
        Instruction::Swc2 {
            rt: 6,
            base: a,
            offset: -16,
        },
    ];
    instructions.extend(GteOpcode::ALL.into_iter().map(|opcode| {
        Instruction::Gte(GteCommand {
            opcode,
            fake: 4,
            shift_fraction: true,
            matrix: 2,
            vector: 3,
            control_vector: 1,
            limit_unsigned: true,
        })
    }));

    let covered_forms: BTreeSet<_> = instructions.iter().map(Instruction::form).collect();
    let declared_forms: BTreeSet<_> = InstructionForm::all().collect();
    assert_eq!(declared_forms.len(), 89, "declared PSX instruction forms");
    assert_eq!(
        covered_forms, declared_forms,
        "profile vectors must cover every declared instruction form"
    );
    assert_eq!(
        instructions.len(),
        declared_forms.len(),
        "each instruction form must have exactly one profile vector"
    );

    for instruction in instructions {
        let word = encode_word(&instruction, pc).unwrap();
        assert_eq!(
            decode_word(word, pc).unwrap(),
            instruction,
            "word {word:#010x}"
        );
        PsxR3000A::semantics(&instruction, &pc).unwrap_or_else(|error| {
            panic!("no static semantics for {:?}: {error}", instruction.form())
        });
    }
}
