use super::decode::decode;
use super::emit::*;
use super::instruction::*;

#[test]
fn emit_nop() {
    assert_eq!(emit(&InstructionKind::Nop).unwrap(), vec![0x90]);
}

#[test]
fn emit_unknown_returns_raw() {
    let raw = vec![0xDE, 0xAD, 0xBE, 0xEF];
    let r = emit(&InstructionKind::Unknown { raw: raw.clone() }).unwrap();
    assert_eq!(r, raw);
}

#[test]
fn emit_hlt() {
    assert_eq!(emit(&InstructionKind::Hlt).unwrap(), vec![0xF4]);
}

#[test]
fn emit_iret() {
    assert_eq!(emit(&InstructionKind::Iret).unwrap(), vec![0xCF]);
}

#[test]
fn emit_int3() {
    assert_eq!(
        emit(&InstructionKind::Int { vector: 3 }).unwrap(),
        vec![0xCC]
    );
}

#[test]
fn emit_int_n() {
    assert_eq!(
        emit(&InstructionKind::Int { vector: 0x21 }).unwrap(),
        vec![0xCD, 0x21]
    );
    assert_eq!(
        emit(&InstructionKind::Int { vector: 0xFF }).unwrap(),
        vec![0xCD, 0xFF]
    );
}

#[test]
fn emit_jcc() {
    assert_eq!(
        emit(&InstructionKind::Jcc {
            cond: Cond::E,
            target: 5
        })
        .unwrap(),
        vec![0x74, 0x05]
    );
    assert_eq!(
        emit(&InstructionKind::Jcc {
            cond: Cond::Ne,
            target: -4
        })
        .unwrap(),
        vec![0x75, 0xFC]
    );
    assert_eq!(
        emit(&InstructionKind::Jcc {
            cond: Cond::Cxz,
            target: 16
        })
        .unwrap(),
        vec![0xE3, 0x10]
    );
}

#[test]
fn emit_loop_family() {
    for (condition, target, expected) in [
        (LoopCondition::NotEqual, 5, vec![0xE0, 0x05]),
        (LoopCondition::Equal, -4, vec![0xE1, 0xFC]),
        (LoopCondition::Always, -2, vec![0xE2, 0xFE]),
    ] {
        assert_eq!(
            emit(&InstructionKind::Loop { condition, target }).unwrap(),
            expected
        );
    }
}

#[test]
fn emit_jmp_short_near_far() {
    assert_eq!(
        emit(&InstructionKind::Jmp {
            target: JmpTarget::Rel8(-2)
        })
        .unwrap(),
        vec![0xEB, 0xFE]
    );
    assert_eq!(
        emit(&InstructionKind::Jmp {
            target: JmpTarget::Rel16(0x1234)
        })
        .unwrap(),
        vec![0xE9, 0x34, 0x12]
    );
    assert_eq!(
        emit(&InstructionKind::Jmp {
            target: JmpTarget::Far {
                seg: 0xF000,
                off: 0x0000
            }
        })
        .unwrap(),
        vec![0xEA, 0x00, 0x00, 0x00, 0xF0]
    );
}

#[test]
fn emit_call_near_far() {
    assert_eq!(
        emit(&InstructionKind::Call {
            target: CallTarget::Rel16(0x10)
        })
        .unwrap(),
        vec![0xE8, 0x10, 0x00]
    );
    assert_eq!(
        emit(&InstructionKind::Call {
            target: CallTarget::Far {
                seg: 0x2000,
                off: 0x1000
            }
        })
        .unwrap(),
        vec![0x9A, 0x00, 0x10, 0x00, 0x20]
    );
}

#[test]
fn emit_ret_retf() {
    assert_eq!(emit(&InstructionKind::Ret { pop: 0 }).unwrap(), vec![0xC3]);
    assert_eq!(
        emit(&InstructionKind::Ret { pop: 4 }).unwrap(),
        vec![0xC2, 0x04, 0x00]
    );
    assert_eq!(emit(&InstructionKind::Retf { pop: 0 }).unwrap(), vec![0xCB]);
    assert_eq!(
        emit(&InstructionKind::Retf { pop: 8 }).unwrap(),
        vec![0xCA, 0x08, 0x00]
    );
}

#[test]
fn emit_push_pop_reg16() {
    assert_eq!(
        emit(&InstructionKind::Push {
            src: Operand::Reg16(Reg16::AX)
        })
        .unwrap(),
        vec![0x50]
    );
    assert_eq!(
        emit(&InstructionKind::Push {
            src: Operand::Reg16(Reg16::DI)
        })
        .unwrap(),
        vec![0x57]
    );
    assert_eq!(
        emit(&InstructionKind::Pop {
            dest: Operand::Reg16(Reg16::AX)
        })
        .unwrap(),
        vec![0x58]
    );
    assert_eq!(
        emit(&InstructionKind::Pop {
            dest: Operand::Reg16(Reg16::DI)
        })
        .unwrap(),
        vec![0x5F]
    );
}

#[test]
fn emit_push_pop_sreg() {
    assert_eq!(
        emit(&InstructionKind::Push {
            src: Operand::Sreg(Sreg::ES)
        })
        .unwrap(),
        vec![0x06]
    );
    assert_eq!(
        emit(&InstructionKind::Push {
            src: Operand::Sreg(Sreg::CS)
        })
        .unwrap(),
        vec![0x0E]
    );
    assert_eq!(
        emit(&InstructionKind::Push {
            src: Operand::Sreg(Sreg::SS)
        })
        .unwrap(),
        vec![0x16]
    );
    assert_eq!(
        emit(&InstructionKind::Push {
            src: Operand::Sreg(Sreg::DS)
        })
        .unwrap(),
        vec![0x1E]
    );
    assert_eq!(
        emit(&InstructionKind::Pop {
            dest: Operand::Sreg(Sreg::ES)
        })
        .unwrap(),
        vec![0x07]
    );
    assert_eq!(
        emit(&InstructionKind::Pop {
            dest: Operand::Sreg(Sreg::SS)
        })
        .unwrap(),
        vec![0x17]
    );
    assert_eq!(
        emit(&InstructionKind::Pop {
            dest: Operand::Sreg(Sreg::DS)
        })
        .unwrap(),
        vec![0x1F]
    );
}

#[test]
fn emit_push_mem() {
    let m = Operand::Mem(Mem {
        seg: None,
        base: MemBase::Direct,
        disp: 0x1234u16 as i16,
        size: OpSize::Word,
    });
    assert_eq!(
        emit(&InstructionKind::Push { src: m }).unwrap(),
        vec![0xFF, 0x36, 0x34, 0x12]
    );
}

#[test]
fn emit_pop_reg_uses_short_form() {
    // POP BX → 0x5B (단축형 우선)
    assert_eq!(
        emit(&InstructionKind::Pop {
            dest: Operand::Reg16(Reg16::BX)
        })
        .unwrap(),
        vec![0x5B]
    );
}

#[test]
fn emit_mov_rm_r_word() {
    // MOV BX, AX → 0x89 0xC3
    let kind = InstructionKind::Mov {
        dest: Operand::Reg16(Reg16::BX),
        src: Operand::Reg16(Reg16::AX),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0x89, 0xC3]);
}

#[test]
fn emit_mov_r_rm_byte() {
    // MOV AL, CL → 0x8A 0xC1
    let kind = InstructionKind::Mov {
        dest: Operand::Reg8(Reg8::AL),
        src: Operand::Reg8(Reg8::CL),
    };
    // 0x8A는 r ← r/m, 0x88은 r/m ← r — 같은 의미를 두 가지 인코딩으로 표현 가능
    // emit는 한 가지로 결정해 일관되게 인코딩한다 (여기선 0x8A 선택)
    let out = emit(&kind).unwrap();
    // 두 인코딩 모두 허용 (0x88 0xC8 = MOV CL, AL과 동치 — 단, 우리는 dest=AL src=CL이므로 0x8A를 사용)
    assert!(out == vec![0x8A, 0xC1] || out == vec![0x88, 0xC8]);
}

#[test]
fn emit_mov_sreg() {
    // MOV DS, AX → 0x8E 0xD8
    let kind = InstructionKind::Mov {
        dest: Operand::Sreg(Sreg::DS),
        src: Operand::Reg16(Reg16::AX),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0x8E, 0xD8]);

    // MOV AX, ES → 0x8C 0xC0
    let kind = InstructionKind::Mov {
        dest: Operand::Reg16(Reg16::AX),
        src: Operand::Sreg(Sreg::ES),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0x8C, 0xC0]);
}

#[test]
fn emit_mov_acc_moffs() {
    // MOV AX, [0x1234] → 0xA1 0x34 0x12
    let mem = Operand::Mem(Mem {
        seg: None,
        base: MemBase::Direct,
        disp: 0x1234u16 as i16,
        size: OpSize::Word,
    });
    let kind = InstructionKind::Mov {
        dest: Operand::Reg16(Reg16::AX),
        src: mem.clone(),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0xA1, 0x34, 0x12]);

    // MOV [0x1234], AL → 0xA2 0x34 0x12
    let mem8 = Operand::Mem(Mem {
        seg: None,
        base: MemBase::Direct,
        disp: 0x1234u16 as i16,
        size: OpSize::Byte,
    });
    let kind = InstructionKind::Mov {
        dest: mem8,
        src: Operand::Reg8(Reg8::AL),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0xA2, 0x34, 0x12]);
}

#[test]
fn emit_mov_r8_imm() {
    // MOV AL, 0x42 → 0xB0 0x42
    let kind = InstructionKind::Mov {
        dest: Operand::Reg8(Reg8::AL),
        src: Operand::Imm8(0x42),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0xB0, 0x42]);

    // MOV BH, 0xFF → 0xB7 0xFF
    let kind = InstructionKind::Mov {
        dest: Operand::Reg8(Reg8::BH),
        src: Operand::Imm8(0xFF),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0xB7, 0xFF]);
}

#[test]
fn emit_mov_r16_imm() {
    // MOV AX, 0x1234 → 0xB8 0x34 0x12
    let kind = InstructionKind::Mov {
        dest: Operand::Reg16(Reg16::AX),
        src: Operand::Imm16(0x1234),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0xB8, 0x34, 0x12]);

    // MOV DI, 0xF000 → 0xBF 0x00 0xF0
    let kind = InstructionKind::Mov {
        dest: Operand::Reg16(Reg16::DI),
        src: Operand::Imm16(0xF000),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0xBF, 0x00, 0xF0]);
}

#[test]
fn emit_mov_rm8_imm() {
    // MOV byte [BX], 0xAB → 0xC6 0x07 0xAB
    let mem = Operand::Mem(Mem {
        seg: None,
        base: MemBase::Bx,
        disp: 0,
        size: OpSize::Byte,
    });
    let kind = InstructionKind::Mov {
        dest: mem,
        src: Operand::Imm8(0xAB),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0xC6, 0x07, 0xAB]);
}

#[test]
fn emit_mov_rm8_imm_mem_disp16() {
    // MOV byte [0x1000], 0xAB → 0xC6 0x06 0x00 0x10 0xAB
    let mem = Operand::Mem(Mem {
        seg: None,
        base: MemBase::Direct,
        disp: 0x1000u16 as i16,
        size: OpSize::Byte,
    });
    let kind = InstructionKind::Mov {
        dest: mem,
        src: Operand::Imm8(0xAB),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0xC6, 0x06, 0x00, 0x10, 0xAB]);
}

#[test]
fn emit_mov_rm16_imm_mem_disp16() {
    // MOV word [0x1000], 0x1234 → 0xC7 0x06 0x00 0x10 0x34 0x12
    let mem = Operand::Mem(Mem {
        seg: None,
        base: MemBase::Direct,
        disp: 0x1000u16 as i16,
        size: OpSize::Word,
    });
    let kind = InstructionKind::Mov {
        dest: mem,
        src: Operand::Imm16(0x1234),
    };
    assert_eq!(
        emit(&kind).unwrap(),
        vec![0xC7, 0x06, 0x00, 0x10, 0x34, 0x12]
    );
}

// ─── ALU family (ADD/OR/AND/SUB/XOR/CMP) ────────────────────────────────────

#[test]
fn emit_alu_add_rm_r() {
    // ADD BX, AX → 0x01 0xC3 (canonical: src=Reg → r/m ← r form)
    let kind = InstructionKind::Add {
        dest: Operand::Reg16(Reg16::BX),
        src: Operand::Reg16(Reg16::AX),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0x01, 0xC3]);
}

#[test]
fn emit_alu_xor_self() {
    // XOR AX, AX → 0x31 0xC0 (canonical r/m ← r)
    let kind = InstructionKind::Xor {
        dest: Operand::Reg16(Reg16::AX),
        src: Operand::Reg16(Reg16::AX),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0x31, 0xC0]);
}

#[test]
fn emit_alu_cmp_al_imm() {
    // CMP AL, 0x42 → 0x3C 0x42 (acc 단축형 우선)
    let kind = InstructionKind::Cmp {
        a: Operand::Reg8(Reg8::AL),
        b: Operand::Imm8(0x42),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0x3C, 0x42]);
}

#[test]
fn emit_alu_cmp_ax_imm16() {
    // CMP AX, 0x1234 → 0x3D 0x34 0x12 (acc 단축형 우선; sign-ext 불가)
    let kind = InstructionKind::Cmp {
        a: Operand::Reg16(Reg16::AX),
        b: Operand::Imm16(0x1234),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0x3D, 0x34, 0x12]);
}

#[test]
fn emit_alu_adc_and_sbb() {
    assert_eq!(
        emit(&InstructionKind::Adc {
            dest: Operand::Reg16(Reg16::AX),
            src: Operand::Imm16(0x1234),
        })
        .unwrap(),
        vec![0x15, 0x34, 0x12]
    );
    assert_eq!(
        emit(&InstructionKind::Sbb {
            dest: Operand::Reg8(Reg8::AL),
            src: Operand::Imm8(0x7F),
        })
        .unwrap(),
        vec![0x1C, 0x7F]
    );
}

#[test]
fn emit_alu_sub_sp_imm8_uses_0x83() {
    // SUB SP, 0x10 → 0x83 0xEC 0x10 (sign-extendable imm → 0x83 우선)
    // SP는 acc가 아니므로 단축형 없음 → Group 1.
    let kind = InstructionKind::Sub {
        dest: Operand::Reg16(Reg16::SP),
        src: Operand::Imm16(0x0010),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0x83, 0xEC, 0x10]);
}

#[test]
fn emit_alu_add_sp_neg_imm8_uses_0x83() {
    // ADD SP, -2 (0xFFFE) → 0x83 0xC4 0xFE (sign-ext 가능 → 0x83)
    let kind = InstructionKind::Add {
        dest: Operand::Reg16(Reg16::SP),
        src: Operand::Imm16(0xFFFE),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0x83, 0xC4, 0xFE]);
}

#[test]
fn emit_alu_and_bx_imm16_uses_0x81() {
    // AND BX, 0x1234 → 0x81 0xE3 0x34 0x12 (sign-ext 불가 → 0x81)
    // BX는 acc가 아니므로 단축형 없음.
    let kind = InstructionKind::And {
        dest: Operand::Reg16(Reg16::BX),
        src: Operand::Imm16(0x1234),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0x81, 0xE3, 0x34, 0x12]);
}

#[test]
fn emit_alu_cmp_rm8_imm8() {
    // CMP CL, 0x42 → 0x80 0xF9 0x42 (acc 아님 → Group 1 0x80)
    let kind = InstructionKind::Cmp {
        a: Operand::Reg8(Reg8::CL),
        b: Operand::Imm8(0x42),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0x80, 0xF9, 0x42]);
}

#[test]
fn emit_alu_or_rm_r_byte() {
    // OR CL, AL → 0x08 0xC1 (canonical: src=Reg → r/m ← r form)
    let kind = InstructionKind::Or {
        dest: Operand::Reg8(Reg8::CL),
        src: Operand::Reg8(Reg8::AL),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0x08, 0xC1]);
}

#[test]
fn emit_alu_and_al_imm() {
    // AND AL, 0x0F → 0x24 0x0F (acc 단축형)
    let kind = InstructionKind::And {
        dest: Operand::Reg8(Reg8::AL),
        src: Operand::Imm8(0x0F),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0x24, 0x0F]);
}

#[test]
fn emit_alu_canonical_round_trip() {
    // 0x03 0xD8 = ADD BX, AX (r ← r/m form, non-canonical)
    // canonical: src=Reg → r/m ← r → 0x01 0xC3
    let original = vec![0x03, 0xD8];
    let kind = decode(&original, ip0()).unwrap().kind;
    let canonical = emit(&kind).unwrap();
    assert_eq!(canonical, vec![0x01, 0xC3]);
    // 두 번째 round-trip: 안정점 확인
    let kind2 = decode(&canonical, ip0()).unwrap().kind;
    assert_eq!(kind, kind2);
    assert_eq!(emit(&kind2).unwrap(), canonical);
}

// ─── TEST + INC/DEC + NEG + Group 5 indirect JMP/CALL ───────────────────────

#[test]
fn emit_test_rm_r() {
    // TEST AX, AX → 0x85 0xC0 (canonical: src=Reg → r/m ↔ r form)
    let kind = InstructionKind::Test {
        a: Operand::Reg16(Reg16::AX),
        b: Operand::Reg16(Reg16::AX),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0x85, 0xC0]);
}

#[test]
fn emit_test_al_imm() {
    // TEST AL, 0x42 → 0xA8 0x42 (acc 단축형 우선)
    let kind = InstructionKind::Test {
        a: Operand::Reg8(Reg8::AL),
        b: Operand::Imm8(0x42),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0xA8, 0x42]);
}

#[test]
fn emit_test_ax_imm() {
    // TEST AX, 0x00FF → 0xA9 0xFF 0x00
    let kind = InstructionKind::Test {
        a: Operand::Reg16(Reg16::AX),
        b: Operand::Imm16(0x00FF),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0xA9, 0xFF, 0x00]);
}

#[test]
fn emit_test_rm_imm_non_acc() {
    // TEST CL, 0x42 → 0xF6 0xC1 0x42 (acc 아님 → Group 3 0xF6 /0)
    let kind = InstructionKind::Test {
        a: Operand::Reg8(Reg8::CL),
        b: Operand::Imm8(0x42),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0xF6, 0xC1, 0x42]);

    // TEST BX, 0x1234 → 0xF7 0xC3 0x34 0x12 (sign-ext 불가, 항상 imm16)
    let kind = InstructionKind::Test {
        a: Operand::Reg16(Reg16::BX),
        b: Operand::Imm16(0x1234),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0xF7, 0xC3, 0x34, 0x12]);
}

#[test]
fn emit_inc_dec_short() {
    // INC AX → 0x40 (단축형)
    assert_eq!(
        emit(&InstructionKind::Inc {
            dest: Operand::Reg16(Reg16::AX)
        })
        .unwrap(),
        vec![0x40]
    );
    // DEC DI → 0x4F
    assert_eq!(
        emit(&InstructionKind::Dec {
            dest: Operand::Reg16(Reg16::DI)
        })
        .unwrap(),
        vec![0x4F]
    );
}

#[test]
fn emit_inc_rm8() {
    // INC byte [BX] → 0xFE 0x07 (mod=00 reg=000 r/m=111)
    let mem = Operand::Mem(Mem {
        seg: None,
        base: MemBase::Bx,
        disp: 0,
        size: OpSize::Byte,
    });
    assert_eq!(
        emit(&InstructionKind::Inc { dest: mem }).unwrap(),
        vec![0xFE, 0x07]
    );

    // INC AL → 0xFE 0xC0 (mod=11 reg=000 r/m=000)
    assert_eq!(
        emit(&InstructionKind::Inc {
            dest: Operand::Reg8(Reg8::AL)
        })
        .unwrap(),
        vec![0xFE, 0xC0]
    );
}

#[test]
fn emit_dec_rm() {
    // DEC CL → 0xFE 0xC9 (mod=11 reg=001 r/m=001)
    assert_eq!(
        emit(&InstructionKind::Dec {
            dest: Operand::Reg8(Reg8::CL)
        })
        .unwrap(),
        vec![0xFE, 0xC9]
    );

    // DEC word [BX] → 0xFF 0x0F (mod=00 reg=001 r/m=111)
    let mem = Operand::Mem(Mem {
        seg: None,
        base: MemBase::Bx,
        disp: 0,
        size: OpSize::Word,
    });
    assert_eq!(
        emit(&InstructionKind::Dec { dest: mem }).unwrap(),
        vec![0xFF, 0x0F]
    );
}

#[test]
fn emit_neg() {
    // NEG AX → 0xF7 0xD8 (mod=11 reg=011 r/m=000)
    let kind = InstructionKind::Neg {
        dest: Operand::Reg16(Reg16::AX),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0xF7, 0xD8]);

    // NEG CL → 0xF6 0xD9 (mod=11 reg=011 r/m=001)
    let kind = InstructionKind::Neg {
        dest: Operand::Reg8(Reg8::CL),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0xF6, 0xD9]);
}

#[test]
fn emit_group3_mul_div_family() {
    assert_eq!(
        emit(&InstructionKind::Div {
            src: Operand::Reg8(Reg8::CL),
        })
        .unwrap(),
        vec![0xF6, 0xF1]
    );
    assert_eq!(
        emit(&InstructionKind::Mul {
            src: Operand::Reg16(Reg16::BX),
        })
        .unwrap(),
        vec![0xF7, 0xE3]
    );
}

#[test]
fn emit_call_indirect() {
    // CALL BX → 0xFF 0xD3 (mod=11 reg=010 r/m=011)
    let kind = InstructionKind::Call {
        target: CallTarget::IndirectNear(Operand::Reg16(Reg16::BX)),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0xFF, 0xD3]);
}

#[test]
fn emit_jmp_indirect() {
    // JMP BX → 0xFF 0xE3 (mod=11 reg=100 r/m=011)
    let kind = InstructionKind::Jmp {
        target: JmpTarget::IndirectNear(Operand::Reg16(Reg16::BX)),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0xFF, 0xE3]);
}

#[test]
fn emit_call_far_indirect() {
    // CALL FAR [0x1234] → 0xFF 0x1E 0x34 0x12 (mod=00 reg=011=/3 r/m=110=[disp16])
    let m = Mem {
        seg: None,
        base: MemBase::Direct,
        disp: 0x1234u16 as i16,
        size: OpSize::Word,
    };
    let kind = InstructionKind::Call {
        target: CallTarget::IndirectFar(m),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0xFF, 0x1E, 0x34, 0x12]);

    // round-trip: decode → emit → 동일 bytes
    let bytes = emit(&kind).unwrap();
    let kind2 = decode(&bytes, ip0()).unwrap().kind;
    assert_eq!(kind, kind2);
    assert_eq!(emit(&kind2).unwrap(), bytes);
}

#[test]
fn emit_jmp_far_indirect() {
    // JMP FAR [0x1234] → 0xFF 0x2E 0x34 0x12 (mod=00 reg=101=/5 r/m=110=[disp16])
    let m = Mem {
        seg: None,
        base: MemBase::Direct,
        disp: 0x1234u16 as i16,
        size: OpSize::Word,
    };
    let kind = InstructionKind::Jmp {
        target: JmpTarget::IndirectFar(m),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0xFF, 0x2E, 0x34, 0x12]);

    // round-trip: decode → emit → 동일 bytes
    let bytes = emit(&kind).unwrap();
    let kind2 = decode(&bytes, ip0()).unwrap().kind;
    assert_eq!(kind, kind2);
    assert_eq!(emit(&kind2).unwrap(), bytes);
}

#[test]
fn emit_inc_reg16_uses_short_form_round_trip() {
    // INC BX를 0xFF /0 form으로 디코드해도 emit는 항상 단축형 0x43.
    let kind = decode(&[0xFF, 0xC3], ip0()).unwrap().kind;
    let canonical = emit(&kind).unwrap();
    assert_eq!(canonical, vec![0x43], "INC BX 단축형");
    // 안정점 확인
    let kind2 = decode(&canonical, ip0()).unwrap().kind;
    assert_eq!(kind, kind2);
}

// ─── Canonical 안정점 테스트 (spec §1.1, §4.4.1) ──────────────────────────────
//
// 8086 ISA의 redundant encoding(예: reg-reg MOV의 0x88/0x89 vs 0x8A/0x8B)은
// 하나의 canonical form으로 결정적 인코딩됨을 검증한다.

fn ip0() -> SegOff {
    SegOff { seg: 0, off: 0 }
}

#[test]
fn mov_reg_reg_emits_canonical_dir0_form() {
    // MOV SP, AX의 두 redundant encoding이 같은 kind → 같은 canonical bytes (0x89 0xC4).
    let kind1 = decode(&[0x89, 0xC4], ip0()).unwrap().kind;
    let kind2 = decode(&[0x8B, 0xE0], ip0()).unwrap().kind;
    assert_eq!(kind1, kind2, "두 redundant encoding이 같은 kind여야");

    // emit는 항상 canonical 0x89 form (r/m ← r, dir=0)
    let canonical = emit(&kind1).unwrap();
    assert_eq!(
        canonical,
        vec![0x89, 0xC4],
        "canonical = r/m=SP/100, reg=AX/000"
    );

    // 두 번째 kind도 같은 canonical bytes
    let canonical2 = emit(&kind2).unwrap();
    assert_eq!(canonical2, canonical);
}

#[test]
fn mov_reg_reg_canonical_round_trip_fixed_point() {
    // non-canonical 입력의 안정점 도달 확인:
    //   decode(0x8B 0xE0) → kind1
    //   emit(kind1) → bytes1 (canonical, 0x89 0xC4 — original과 다름)
    //   decode(bytes1) → kind2
    //   emit(kind2) → bytes2
    // 안정점: kind1 == kind2, bytes1 == bytes2.
    let original = vec![0x8B, 0xE0]; // non-canonical MOV SP, AX
    let kind1 = decode(&original, ip0()).unwrap().kind;
    let bytes1 = emit(&kind1).unwrap();
    let kind2 = decode(&bytes1, ip0()).unwrap().kind;
    let bytes2 = emit(&kind2).unwrap();

    assert_eq!(kind1, kind2, "decode kind 불안정");
    assert_eq!(bytes1, bytes2, "canonical 안정점 도달 실패");
    assert_ne!(
        bytes1, original,
        "non-canonical 입력은 첫 emit에서 byte 변경되어야 정상"
    );
}

#[test]
fn pop_rm_reg_form_emits_short_form() {
    // POP BX를 Group form 0x8F /0으로 디코드해도 emit는 항상 단축형 0x58+r.
    let kind = decode(&[0x8F, 0xC3], ip0()).unwrap().kind; // POP BX (Group form)
    let canonical = emit(&kind).unwrap();
    assert_eq!(canonical, vec![0x5B], "POP BX 단축형");

    // 다시 디코드해도 같은 kind, 같은 emit 결과
    let kind2 = decode(&canonical, ip0()).unwrap().kind;
    assert_eq!(kind, kind2);
    assert_eq!(emit(&kind2).unwrap(), canonical);
}

#[test]
fn mov_mem_disp_zero_canonical_uses_mod00() {
    // [BX+SI] + disp=0 → mod=00 (no disp byte) canonical.
    // BP base는 mod=00이 [disp16]로 예약되어 강제 disp이지만, 그 외 base는 disp=0이면 mod=00.
    let mem = Operand::Mem(Mem {
        seg: None,
        base: MemBase::BxSi,
        disp: 0,
        size: OpSize::Word,
    });
    let kind = InstructionKind::Mov {
        dest: mem,
        src: Operand::Reg16(Reg16::AX),
    };
    let bytes = emit(&kind).unwrap();
    // 0x89 (MOV r/m16, r16) + ModR/M = mod=00 reg=000(AX) r/m=000(BX+SI) = 0x00
    // 총 2바이트, displacement 없음.
    assert_eq!(bytes, vec![0x89, 0x00], "disp=0 BxSi → mod=00, no disp");
}

// ----------------------------------------------------------------------------
// Phase D-4: 시프트/로테이트 + I/O + LEA + XCHG
// ----------------------------------------------------------------------------

#[test]
fn emit_shift_rol_one() {
    let kind = InstructionKind::Rol {
        dest: Operand::Reg16(Reg16::AX),
        count: ShiftCount::One,
    };
    assert_eq!(emit(&kind).unwrap(), vec![0xD1, 0xC0]);
}

#[test]
fn emit_shift_through_carry() {
    assert_eq!(
        emit(&InstructionKind::Rcr {
            dest: Operand::Reg8(Reg8::AL),
            count: ShiftCount::One,
        })
        .unwrap(),
        vec![0xD0, 0xD8]
    );
    assert_eq!(
        emit(&InstructionKind::Rcl {
            dest: Operand::Reg16(Reg16::AX),
            count: ShiftCount::One,
        })
        .unwrap(),
        vec![0xD1, 0xD0]
    );
}

#[test]
fn emit_shift_shl_cl() {
    let kind = InstructionKind::Shl {
        dest: Operand::Reg16(Reg16::AX),
        count: ShiftCount::Cl,
    };
    assert_eq!(emit(&kind).unwrap(), vec![0xD3, 0xE0]);
}

#[test]
fn shl_is_the_typed_name_for_the_sal_selector() {
    let bytes = [0xd1, 0xe0];
    let instruction = decode(&bytes, ip0()).unwrap().kind;
    assert!(matches!(instruction, InstructionKind::Shl { .. }));
    assert_eq!(emit(&instruction).unwrap(), bytes);
}

#[test]
fn emit_io() {
    assert_eq!(
        emit(&InstructionKind::InAl {
            port: PortAddr::Imm8(0x60)
        })
        .unwrap(),
        vec![0xE4, 0x60]
    );
    assert_eq!(
        emit(&InstructionKind::OutAl { port: PortAddr::Dx }).unwrap(),
        vec![0xEE]
    );
    assert_eq!(
        emit(&InstructionKind::OutAx { port: PortAddr::Dx }).unwrap(),
        vec![0xEF]
    );
}

#[test]
fn emit_lea() {
    let mem = Mem {
        seg: None,
        base: MemBase::Bp,
        disp: 4,
        size: OpSize::Word,
    };
    let kind = InstructionKind::Lea {
        dest: Reg16::AX,
        src: mem,
    };
    assert_eq!(emit(&kind).unwrap(), vec![0x8D, 0x46, 0x04]);
}

#[test]
fn emit_les_lds_and_xlat() {
    let src = Mem {
        seg: None,
        base: MemBase::Direct,
        disp: 0x1234,
        size: OpSize::Word,
    };
    assert_eq!(
        emit(&InstructionKind::Les {
            dest: Reg16::BX,
            src,
        })
        .unwrap(),
        vec![0xC4, 0x1E, 0x34, 0x12]
    );
    assert_eq!(
        emit(&InstructionKind::Lds {
            dest: Reg16::SI,
            src: Mem {
                disp: 0x5678,
                ..src
            },
        })
        .unwrap(),
        vec![0xC5, 0x36, 0x78, 0x56]
    );
    assert_eq!(emit(&InstructionKind::Xlat).unwrap(), vec![0xD7]);
}

#[test]
fn emit_xchg_ax_short() {
    let kind = InstructionKind::Xchg {
        a: Operand::Reg16(Reg16::AX),
        b: Operand::Reg16(Reg16::CX),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0x91]);
}

#[test]
fn emit_xchg_rm_r() {
    let kind = InstructionKind::Xchg {
        a: Operand::Reg16(Reg16::BX),
        b: Operand::Reg16(Reg16::AX),
    };
    // 0x87 0xC3 = XCHG BX, AX (mod=11 reg=000=AX r/m=011=BX)
    assert_eq!(emit(&kind).unwrap(), vec![0x87, 0xC3]);
}

// ----------------------------------------------------------------------------
// Phase D-4: 문자열 명령 + REP/REPNE + segment override prefix
// ----------------------------------------------------------------------------

#[test]
fn emit_string_ops() {
    assert_eq!(emit(&InstructionKind::Movsb).unwrap(), vec![0xA4]);
    assert_eq!(emit(&InstructionKind::Movsw).unwrap(), vec![0xA5]);
    assert_eq!(emit(&InstructionKind::Cmpsb).unwrap(), vec![0xA6]);
    assert_eq!(emit(&InstructionKind::Cmpsw).unwrap(), vec![0xA7]);
    assert_eq!(emit(&InstructionKind::Stosb).unwrap(), vec![0xAA]);
    assert_eq!(emit(&InstructionKind::Stosw).unwrap(), vec![0xAB]);
    assert_eq!(emit(&InstructionKind::Lodsb).unwrap(), vec![0xAC]);
    assert_eq!(emit(&InstructionKind::Lodsw).unwrap(), vec![0xAD]);
    assert_eq!(emit(&InstructionKind::Scasb).unwrap(), vec![0xAE]);
    assert_eq!(emit(&InstructionKind::Scasw).unwrap(), vec![0xAF]);
}

#[test]
fn emit_rep_repne() {
    let kind = InstructionKind::Rep(Box::new(InstructionKind::Movsb));
    assert_eq!(emit(&kind).unwrap(), vec![0xF3, 0xA4]);

    let kind = InstructionKind::Repne(Box::new(InstructionKind::Scasb));
    assert_eq!(emit(&kind).unwrap(), vec![0xF2, 0xAE]);
}

#[test]
fn emit_seg_override_mov() {
    // MOV AX, ES:[BX] → 0x26 0x8B 0x07
    let mem = Operand::Mem(Mem {
        seg: Some(Sreg::ES),
        base: MemBase::Bx,
        disp: 0,
        size: OpSize::Word,
    });
    let kind = InstructionKind::Mov {
        dest: Operand::Reg16(Reg16::AX),
        src: mem,
    };
    assert_eq!(emit(&kind).unwrap(), vec![0x26, 0x8B, 0x07]);
}

#[test]
fn emit_seg_override_moffs_short_form() {
    // MOV AX, DS:[0x1234] — 단축형 0xA1이 segment override 처리.
    // 0x3E 0xA1 0x34 0x12
    let mem = Operand::Mem(Mem {
        seg: Some(Sreg::DS),
        base: MemBase::Direct,
        disp: 0x1234,
        size: OpSize::Word,
    });
    let kind = InstructionKind::Mov {
        dest: Operand::Reg16(Reg16::AX),
        src: mem,
    };
    assert_eq!(emit(&kind).unwrap(), vec![0x3E, 0xA1, 0x34, 0x12]);
}

#[test]
fn emit_decode_round_trip_seg_override() {
    // 0x36 0x88 0x07 = MOV SS:[BX], AL
    let bytes = vec![0x36u8, 0x88, 0x07];
    let r = decode(&bytes, super::instruction::SegOff { seg: 0, off: 0 }).unwrap();
    let re = emit(&r.kind).unwrap();
    assert_eq!(re, bytes);
}

// ----------------------------------------------------------------------------
// 플래그 명령 + sign-extend — emit + decode round-trip 검증.
// ----------------------------------------------------------------------------

#[test]
fn emit_flag_single_bytes() {
    // Intel SDM Vol 2A 부록 B의 single-byte 플래그/sign-extend 명령들.
    let cases: &[(InstructionKind, u8)] = &[
        (InstructionKind::Wait, 0x9B),
        (InstructionKind::Pushf, 0x9C),
        (InstructionKind::Popf, 0x9D),
        (InstructionKind::Sahf, 0x9E),
        (InstructionKind::Lahf, 0x9F),
        (InstructionKind::Cmc, 0xF5),
        (InstructionKind::Clc, 0xF8),
        (InstructionKind::Stc, 0xF9),
        (InstructionKind::Cli, 0xFA),
        (InstructionKind::Sti, 0xFB),
        (InstructionKind::Cld, 0xFC),
        (InstructionKind::Std, 0xFD),
        (InstructionKind::Cbw, 0x98),
        (InstructionKind::Cwd, 0x99),
    ];
    for (kind, opcode) in cases {
        let bs = emit(kind).unwrap_or_else(|e| panic!("emit {:?} failed: {:?}", kind, e));
        assert_eq!(bs, vec![*opcode], "emit({:?}) != [0x{:02X}]", kind, opcode);
        // round-trip via decode.
        let cs_ip = super::instruction::SegOff { seg: 0, off: 0 };
        let r = decode(&bs, cs_ip).expect("decode flag op");
        assert_eq!(&r.kind, kind, "decode(emit({:?})) != {:?}", kind, kind);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// V30 (NEC uPD70116) 0x0F secondary opcode emit + round-trip 테스트.
// ─────────────────────────────────────────────────────────────────────────────

fn ip() -> SegOff {
    SegOff { seg: 0, off: 0 }
}

#[test]
fn emit_v30_add4s_sub4s_cmp4s() {
    assert_eq!(emit(&InstructionKind::Add4s).unwrap(), vec![0x0F, 0x20]);
    assert_eq!(emit(&InstructionKind::Sub4s).unwrap(), vec![0x0F, 0x22]);
    assert_eq!(emit(&InstructionKind::Cmp4s).unwrap(), vec![0x0F, 0x26]);
}

#[test]
fn emit_v30_rol4_register() {
    let kind = InstructionKind::Rol4 {
        rm: Operand::Reg8(Reg8::BL),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0x0F, 0x28, 0xC3]);
}

#[test]
fn emit_v30_ror4_register() {
    let kind = InstructionKind::Ror4 {
        rm: Operand::Reg8(Reg8::AL),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0x0F, 0x2A, 0xC0]);
}

#[test]
fn emit_v30_ins_register() {
    // INS AH, DL — bit offset=AH(100), length=DL(010), mod=11 → 0xD4
    let kind = InstructionKind::Ins {
        bit_offset: Reg8::AH,
        length: Reg8::DL,
    };
    assert_eq!(emit(&kind).unwrap(), vec![0x0F, 0x31, 0xD4]);
}

#[test]
fn emit_v30_ext_imm() {
    // EXT CL, 0x0F — rm=CL, imm=0x0F. mod=11 reg=000 rm=001 = 0xC1
    let kind = InstructionKind::ExtImm {
        bit_offset: Reg8::CL,
        length: BitFieldLength::try_from(0x0F).unwrap(),
    };
    assert_eq!(emit(&kind).unwrap(), vec![0x0F, 0x3B, 0xC1, 0x0F]);
}

#[test]
fn emit_v30_brkem() {
    let kind = InstructionKind::Brkem { vector: 0x42 };
    assert_eq!(emit(&kind).unwrap(), vec![0x0F, 0xFF, 0x42]);
}

#[test]
fn round_trip_v30_brkem() {
    let bytes = vec![0x0Fu8, 0xFF, 0x05];
    let r = decode(&bytes, ip()).unwrap();
    assert_eq!(emit(&r.kind).unwrap(), bytes);
}

#[test]
fn round_trip_v30_rol4_memory() {
    // ROL4 [BX+SI] — mod=00 r/m=000 = 0x00 → 0x0F 0x28 0x00
    let bytes = vec![0x0Fu8, 0x28, 0x00];
    let r = decode(&bytes, ip()).unwrap();
    assert_eq!(emit(&r.kind).unwrap(), bytes);
}

#[test]
fn round_trip_v30_ext_immediate_register() {
    let bytes = vec![0x0Fu8, 0x3B, 0xC1, 0x07];
    let r = decode(&bytes, ip()).unwrap();
    assert_eq!(emit(&r.kind).unwrap(), bytes);
}

#[test]
fn round_trip_v30_ins_register() {
    let bytes = vec![0x0Fu8, 0x31, 0xD4];
    let r = decode(&bytes, ip()).unwrap();
    let re = emit(&r.kind).unwrap();
    assert_eq!(re, bytes);
}
