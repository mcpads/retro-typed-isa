use super::decode::*;
use super::instruction::*;

#[test]
fn modrm_register_mode_word() {
    // mod=11 r/m=000 (AX), W=1 → Reg16(AX)
    let (op, used) = decode_modrm_rm(0b11_000_000, OpSize::Word, &[]).unwrap();
    assert_eq!(op, Operand::Reg16(Reg16::AX));
    assert_eq!(used, 0);

    // mod=11 r/m=011 (BX), W=1 → Reg16(BX)
    let (op, _) = decode_modrm_rm(0b11_000_011, OpSize::Word, &[]).unwrap();
    assert_eq!(op, Operand::Reg16(Reg16::BX));

    // mod=11 r/m=111 (DI), W=1 → Reg16(DI)
    let (op, _) = decode_modrm_rm(0b11_000_111, OpSize::Word, &[]).unwrap();
    assert_eq!(op, Operand::Reg16(Reg16::DI));
}

#[test]
fn modrm_register_mode_byte() {
    // mod=11 r/m=000 (AL), W=0 → Reg8(AL)
    let (op, _) = decode_modrm_rm(0b11_000_000, OpSize::Byte, &[]).unwrap();
    assert_eq!(op, Operand::Reg8(Reg8::AL));

    // mod=11 r/m=100 (AH), W=0 → Reg8(AH)
    let (op, _) = decode_modrm_rm(0b11_000_100, OpSize::Byte, &[]).unwrap();
    assert_eq!(op, Operand::Reg8(Reg8::AH));
}

#[test]
fn modrm_mod00_no_disp() {
    // mod=00 r/m=000 → [BX+SI]
    let (op, used) = decode_modrm_rm(0b00_000_000, OpSize::Word, &[]).unwrap();
    assert_eq!(used, 0);
    match op {
        Operand::Mem(m) => {
            assert_eq!(m.base, MemBase::BxSi);
            assert_eq!(m.disp, 0);
            assert_eq!(m.size, OpSize::Word);
            assert_eq!(m.seg, None);
        }
        _ => panic!("expected Mem"),
    }
}

#[test]
fn modrm_mod00_rm110_disp16_special() {
    // mod=00 r/m=110 -> [disp16] (absolute, not BP-relative).
    // displacement = 0x1234 (little-endian)
    let (op, used) = decode_modrm_rm(0b00_000_110, OpSize::Word, &[0x34, 0x12]).unwrap();
    assert_eq!(used, 2);
    match op {
        Operand::Mem(m) => {
            assert_eq!(m.base, MemBase::Direct);
            assert_eq!(m.disp, 0x1234u16 as i16);
        }
        _ => panic!("expected Mem"),
    }
}

#[test]
fn modrm_mod01_disp8_sign_extend() {
    // mod=01 r/m=111 → [BX + disp8]
    // disp8 = 0xFC (= -4 sign-extended)
    let (op, used) = decode_modrm_rm(0b01_000_111, OpSize::Byte, &[0xFC]).unwrap();
    assert_eq!(used, 1);
    match op {
        Operand::Mem(m) => {
            assert_eq!(m.base, MemBase::Bx);
            assert_eq!(m.disp, -4);
            assert_eq!(m.size, OpSize::Byte);
        }
        _ => panic!("expected Mem"),
    }
}

#[test]
fn modrm_mod10_disp16() {
    // mod=10 r/m=110 → [BP + disp16]
    let (op, used) = decode_modrm_rm(0b10_000_110, OpSize::Word, &[0x34, 0x12]).unwrap();
    assert_eq!(used, 2);
    match op {
        Operand::Mem(m) => {
            assert_eq!(m.base, MemBase::Bp);
            assert_eq!(m.disp, 0x1234);
        }
        _ => panic!("expected Mem"),
    }
}

#[test]
fn modrm_reg_field_word() {
    // Extract the reg field.
    assert_eq!(decode_modrm_reg16(0b00_000_000), Reg16::AX);
    assert_eq!(decode_modrm_reg16(0b00_011_000), Reg16::BX);
    assert_eq!(decode_modrm_reg16(0b00_111_000), Reg16::DI);
}

#[test]
fn modrm_reg_field_byte() {
    assert_eq!(decode_modrm_reg8(0b00_000_000), Reg8::AL);
    assert_eq!(decode_modrm_reg8(0b00_100_000), Reg8::AH);
    assert_eq!(decode_modrm_reg8(0b00_111_000), Reg8::BH);
}

#[test]
fn modrm_truncated_displacement_returns_err() {
    // mod=01 with a missing disp8 byte.
    assert!(decode_modrm_rm(0b01_000_000, OpSize::Word, &[]).is_err());
    // mod=10 with only one disp16 byte.
    assert!(decode_modrm_rm(0b10_000_000, OpSize::Word, &[0x12]).is_err());
}

#[test]
fn decode_unknown_byte_falls_through() {
    let r = decode(&[0xDE], SegOff { seg: 0, off: 0 }).unwrap();
    assert_eq!(r.bytes, vec![0xDE]);
    assert_eq!(r.kind, InstructionKind::Unknown { raw: vec![0xDE] });
}

#[test]
fn decode_block_handles_multiple_unknowns() {
    let r = decode_block(&[0xDE, 0xAD, 0xBE], SegOff { seg: 0, off: 0x100 });
    assert_eq!(r.len(), 3);
    assert_eq!(r[0].cs_ip.off, 0x100);
    assert_eq!(r[1].cs_ip.off, 0x101);
    assert_eq!(r[2].cs_ip.off, 0x102);
}

#[test]
fn decode_empty_returns_truncated() {
    assert!(decode(&[], SegOff { seg: 0, off: 0 }).is_err());
}

fn ip0() -> SegOff {
    SegOff { seg: 0, off: 0 }
}

#[test]
fn decode_nop() {
    let r = decode(&[0x90], ip0()).unwrap();
    assert_eq!(r.kind, InstructionKind::Nop);
    assert_eq!(r.bytes, vec![0x90]);
}

#[test]
fn decode_hlt() {
    let r = decode(&[0xF4], ip0()).unwrap();
    assert_eq!(r.kind, InstructionKind::Hlt);
    assert_eq!(r.bytes, vec![0xF4]);
}

#[test]
fn decode_iret() {
    let r = decode(&[0xCF], ip0()).unwrap();
    assert_eq!(r.kind, InstructionKind::Iret);
    assert_eq!(r.bytes, vec![0xCF]);
}

#[test]
fn decode_int3() {
    let r = decode(&[0xCC], ip0()).unwrap();
    assert_eq!(r.kind, InstructionKind::Int { vector: 3 });
    assert_eq!(r.bytes, vec![0xCC]);
}

#[test]
fn decode_int_n() {
    let r = decode(&[0xCD, 0x21], ip0()).unwrap();
    assert_eq!(r.kind, InstructionKind::Int { vector: 0x21 });
    assert_eq!(r.bytes, vec![0xCD, 0x21]);
}

#[test]
fn decode_int_n_truncated() {
    let r = decode(&[0xCD], ip0()).unwrap();
    assert_eq!(r.kind, InstructionKind::Unknown { raw: vec![0xCD] });
}

#[test]
fn decode_jcc_je() {
    // 0x74 0x05 = JE +5
    let r = decode(&[0x74, 0x05], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Jcc {
            cond: Cond::E,
            target: 5
        }
    );
    assert_eq!(r.bytes, vec![0x74, 0x05]);
}

#[test]
fn decode_jcc_jne_negative() {
    let r = decode(&[0x75, 0xFC], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Jcc {
            cond: Cond::Ne,
            target: -4
        }
    );
}

#[test]
fn decode_jcxz() {
    let r = decode(&[0xE3, 0x10], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Jcc {
            cond: Cond::Cxz,
            target: 16
        }
    );
}

#[test]
fn decode_loop_family() {
    for (bytes, condition, target) in [
        ([0xE0, 0x05], LoopCondition::NotEqual, 5),
        ([0xE1, 0xFC], LoopCondition::Equal, -4),
        ([0xE2, 0xFE], LoopCondition::Always, -2),
    ] {
        let r = decode(&bytes, ip0()).unwrap();
        assert_eq!(r.kind, InstructionKind::Loop { condition, target });
        assert_eq!(r.bytes, bytes);
    }
}

#[test]
fn decode_loop_truncated_preserves_opcode() {
    let r = decode(&[0xE2], ip0()).unwrap();
    assert_eq!(r.kind, InstructionKind::Unknown { raw: vec![0xE2] });
}

#[test]
fn decode_jmp_short() {
    // 0xEB 0xFE = JMP $-2 (infinite loop).
    let r = decode(&[0xEB, 0xFE], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Jmp {
            target: JmpTarget::Rel8(-2)
        }
    );
    assert_eq!(r.bytes, vec![0xEB, 0xFE]);
}

#[test]
fn decode_jmp_near() {
    let r = decode(&[0xE9, 0x34, 0x12], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Jmp {
            target: JmpTarget::Rel16(0x1234)
        }
    );
}

#[test]
fn decode_jmp_far() {
    // 0xEA 0x00 0x00 0x00 0xF0 = JMP F000:0000
    let r = decode(&[0xEA, 0x00, 0x00, 0x00, 0xF0], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Jmp {
            target: JmpTarget::Far {
                seg: 0xF000,
                off: 0x0000
            }
        }
    );
}

#[test]
fn decode_call_near() {
    let r = decode(&[0xE8, 0x10, 0x00], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Call {
            target: CallTarget::Rel16(0x10)
        }
    );
}

#[test]
fn decode_call_far() {
    let r = decode(&[0x9A, 0x00, 0x10, 0x00, 0x20], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Call {
            target: CallTarget::Far {
                seg: 0x2000,
                off: 0x1000
            }
        }
    );
}

#[test]
fn decode_ret() {
    let r = decode(&[0xC3], ip0()).unwrap();
    assert_eq!(r.kind, InstructionKind::Ret { pop: 0 });
}

#[test]
fn decode_ret_imm() {
    let r = decode(&[0xC2, 0x04, 0x00], ip0()).unwrap();
    assert_eq!(r.kind, InstructionKind::Ret { pop: 4 });
}

#[test]
fn decode_retf() {
    let r = decode(&[0xCB], ip0()).unwrap();
    assert_eq!(r.kind, InstructionKind::Retf { pop: 0 });
}

#[test]
fn decode_retf_imm() {
    let r = decode(&[0xCA, 0x08, 0x00], ip0()).unwrap();
    assert_eq!(r.kind, InstructionKind::Retf { pop: 8 });
}

#[test]
fn decode_jmp_near_truncated() {
    // A lone 0xE9 must become Unknown { raw: vec![0xE9] } so the sweep advances.
    let r = decode(&[0xE9], ip0()).unwrap();
    assert_eq!(r.kind, InstructionKind::Unknown { raw: vec![0xE9] });
    assert_eq!(r.bytes, vec![0xE9]);
}

#[test]
fn decode_jmp_far_truncated() {
    // 0xEA plus only three of four required bytes becomes Unknown { raw: vec![0xEA] }.
    let r = decode(&[0xEA, 0x00, 0x00, 0x00], ip0()).unwrap();
    assert_eq!(r.kind, InstructionKind::Unknown { raw: vec![0xEA] });
    assert_eq!(r.bytes, vec![0xEA]);
}

#[test]
fn decode_ret_imm_truncated() {
    // 0xC2 plus only one of two required bytes becomes Unknown { raw: vec![0xC2] }.
    let r = decode(&[0xC2, 0x04], ip0()).unwrap();
    assert_eq!(r.kind, InstructionKind::Unknown { raw: vec![0xC2] });
    assert_eq!(r.bytes, vec![0xC2]);
}

#[test]
fn decode_push_pop_reg16() {
    let r = decode(&[0x50], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Push {
            src: Operand::Reg16(Reg16::AX)
        }
    );

    let r = decode(&[0x53], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Push {
            src: Operand::Reg16(Reg16::BX)
        }
    );

    let r = decode(&[0x57], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Push {
            src: Operand::Reg16(Reg16::DI)
        }
    );

    let r = decode(&[0x58], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Pop {
            dest: Operand::Reg16(Reg16::AX)
        }
    );

    let r = decode(&[0x5F], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Pop {
            dest: Operand::Reg16(Reg16::DI)
        }
    );
}

#[test]
fn decode_push_pop_sreg() {
    let r = decode(&[0x06], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Push {
            src: Operand::Sreg(Sreg::ES)
        }
    );

    let r = decode(&[0x0E], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Push {
            src: Operand::Sreg(Sreg::CS)
        }
    );

    let r = decode(&[0x16], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Push {
            src: Operand::Sreg(Sreg::SS)
        }
    );

    let r = decode(&[0x1E], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Push {
            src: Operand::Sreg(Sreg::DS)
        }
    );

    let r = decode(&[0x07], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Pop {
            dest: Operand::Sreg(Sreg::ES)
        }
    );

    // POP CS (0x0F) remains unknown in this milestone.
    let r = decode(&[0x0F], ip0()).unwrap();
    assert!(matches!(r.kind, InstructionKind::Unknown { .. }));
}

#[test]
fn decode_pop_cs_invalid_unknown() {
    // 0x0F is POP CS on 8086 (rare in ROMs) and an escape opcode on 286+.
    // This decoder returns Unknown to preserve linear-sweep continuity.
    let r = decode(&[0x0F], ip0()).unwrap();
    assert!(matches!(r.kind, InstructionKind::Unknown { .. }));
}

#[test]
fn decode_push_rm16_via_ff_6() {
    // 0xFF /6 r/m16 = PUSH r/m16
    // 0xFF 0x36 0x34 0x12 = PUSH word [0x1234]
    // ModR/M = 0x36 = mod=00 reg=110 r/m=110 → mod=00 r/m=110 = [disp16]
    let r = decode(&[0xFF, 0x36, 0x34, 0x12], ip0()).unwrap();
    match r.kind {
        InstructionKind::Push {
            src: Operand::Mem(m),
        } => {
            assert_eq!(m.base, MemBase::Direct);
            assert_eq!(m.disp, 0x1234u16 as i16);
            assert_eq!(m.size, OpSize::Word);
        }
        other => panic!("expected Push Mem, got {:?}", other),
    }
}

#[test]
fn decode_pop_rm16_via_8f_0() {
    // 0x8F /0 r/m16 = POP r/m16
    // 0x8F 0xC3 = POP BX (mod=11 reg=000 r/m=011)
    let r = decode(&[0x8F, 0xC3], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Pop {
            dest: Operand::Reg16(Reg16::BX)
        }
    );
}

#[test]
fn decode_mov_rm_r_word() {
    // 0x89 /r — MOV r/m16, r16
    // 0x89 0xC3 = mod=11 reg=000(AX) r/m=011(BX) → MOV BX, AX
    let r = decode(&[0x89, 0xC3], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Mov {
            dest: Operand::Reg16(Reg16::BX),
            src: Operand::Reg16(Reg16::AX),
        }
    );
}

#[test]
fn decode_mov_r_rm_word() {
    // 0x8B /r — MOV r16, r/m16
    // 0x8B 0xD8 = mod=11 reg=011(BX) r/m=000(AX) → MOV BX, AX
    let r = decode(&[0x8B, 0xD8], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Mov {
            dest: Operand::Reg16(Reg16::BX),
            src: Operand::Reg16(Reg16::AX),
        }
    );
}

#[test]
fn decode_mov_r8_rm() {
    // 0x8A /r — MOV r8, r/m8
    // 0x8A 0xC1 = mod=11 reg=000(AL) r/m=001(CL) → MOV AL, CL
    let r = decode(&[0x8A, 0xC1], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Mov {
            dest: Operand::Reg8(Reg8::AL),
            src: Operand::Reg8(Reg8::CL),
        }
    );
}

#[test]
fn decode_mov_rm8_r() {
    // 0x88 /r — MOV r/m8, r8
    // 0x88 0xC1 = mod=11 reg=000(AL) r/m=001(CL) → MOV CL, AL
    let r = decode(&[0x88, 0xC1], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Mov {
            dest: Operand::Reg8(Reg8::CL),
            src: Operand::Reg8(Reg8::AL),
        }
    );
}

#[test]
fn decode_mov_rm_sreg() {
    // 0x8C /r — MOV r/m16, sreg
    // 0x8C 0xC0 = mod=11 reg=000(ES) r/m=000(AX) → MOV AX, ES
    let r = decode(&[0x8C, 0xC0], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Mov {
            dest: Operand::Reg16(Reg16::AX),
            src: Operand::Sreg(Sreg::ES),
        }
    );
}

#[test]
fn decode_mov_sreg_rm() {
    // 0x8E /r — MOV sreg, r/m16
    // 0x8E 0xD8 = mod=11 reg=011(DS) r/m=000(AX) → MOV DS, AX
    let r = decode(&[0x8E, 0xD8], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Mov {
            dest: Operand::Sreg(Sreg::DS),
            src: Operand::Reg16(Reg16::AX),
        }
    );
}

#[test]
fn decode_mov_al_moffs() {
    // 0xA0 0x34 0x12 = MOV AL, [0x1234]
    let r = decode(&[0xA0, 0x34, 0x12], ip0()).unwrap();
    match r.kind {
        InstructionKind::Mov { dest, src } => {
            assert_eq!(dest, Operand::Reg8(Reg8::AL));
            match src {
                Operand::Mem(m) => {
                    assert_eq!(m.base, MemBase::Direct);
                    assert_eq!(m.disp, 0x1234u16 as i16);
                    assert_eq!(m.size, OpSize::Byte);
                }
                _ => panic!("expected Mem"),
            }
        }
        _ => panic!("expected Mov"),
    }
}

#[test]
fn decode_mov_ax_moffs() {
    // 0xA1 0x34 0x12 = MOV AX, [0x1234]
    let r = decode(&[0xA1, 0x34, 0x12], ip0()).unwrap();
    match r.kind {
        InstructionKind::Mov { dest, src } => {
            assert_eq!(dest, Operand::Reg16(Reg16::AX));
            assert!(matches!(src, Operand::Mem(_)));
        }
        _ => panic!("expected Mov"),
    }
}

#[test]
fn decode_mov_moffs_al() {
    // 0xA2 0x34 0x12 = MOV [0x1234], AL
    let r = decode(&[0xA2, 0x34, 0x12], ip0()).unwrap();
    match r.kind {
        InstructionKind::Mov { dest, src } => {
            assert!(matches!(dest, Operand::Mem(_)));
            assert_eq!(src, Operand::Reg8(Reg8::AL));
        }
        _ => panic!("expected Mov"),
    }
}

#[test]
fn decode_mov_moffs_ax() {
    let r = decode(&[0xA3, 0x34, 0x12], ip0()).unwrap();
    match r.kind {
        InstructionKind::Mov { dest, src } => {
            assert!(matches!(dest, Operand::Mem(_)));
            assert_eq!(src, Operand::Reg16(Reg16::AX));
        }
        _ => panic!("expected Mov"),
    }
}

#[test]
fn decode_mov_r8_imm() {
    // 0xB0+rb ib — MOV r8, imm8
    // 0xB0 0x42 = MOV AL, 0x42
    let r = decode(&[0xB0, 0x42], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Mov {
            dest: Operand::Reg8(Reg8::AL),
            src: Operand::Imm8(0x42),
        }
    );

    // 0xB7 0xFF = MOV BH, 0xFF
    let r = decode(&[0xB7, 0xFF], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Mov {
            dest: Operand::Reg8(Reg8::BH),
            src: Operand::Imm8(0xFF),
        }
    );
}

#[test]
fn decode_mov_r16_imm() {
    // 0xB8 0x34 0x12 = MOV AX, 0x1234
    let r = decode(&[0xB8, 0x34, 0x12], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Mov {
            dest: Operand::Reg16(Reg16::AX),
            src: Operand::Imm16(0x1234),
        }
    );

    // 0xBF 0x00 0xF0 = MOV DI, 0xF000
    let r = decode(&[0xBF, 0x00, 0xF0], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Mov {
            dest: Operand::Reg16(Reg16::DI),
            src: Operand::Imm16(0xF000),
        }
    );
}

#[test]
fn decode_mov_rm8_imm() {
    // 0xC6 /0 ib — MOV r/m8, imm8
    // 0xC6 0x07 0xAB = MOV byte [BX], 0xAB (mod=00 reg=000 r/m=111 → [BX])
    let r = decode(&[0xC6, 0x07, 0xAB], ip0()).unwrap();
    match r.kind {
        InstructionKind::Mov { dest, src } => {
            match dest {
                Operand::Mem(m) => {
                    assert_eq!(m.base, MemBase::Bx);
                    assert_eq!(m.size, OpSize::Byte);
                }
                _ => panic!("expected Mem"),
            }
            assert_eq!(src, Operand::Imm8(0xAB));
        }
        _ => panic!("expected Mov"),
    }
}

#[test]
fn decode_mov_rm16_imm() {
    // 0xC7 /0 iw — MOV r/m16, imm16
    // 0xC7 0xC0 0x34 0x12 = MOV AX, 0x1234 (mod=11 reg=000 r/m=000 → AX)
    let r = decode(&[0xC7, 0xC0, 0x34, 0x12], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Mov {
            dest: Operand::Reg16(Reg16::AX),
            src: Operand::Imm16(0x1234),
        }
    );
}

#[test]
fn decode_mov_rm8_imm_mem_form() {
    // 0xC6 /0 ib — MOV r/m8, imm8 (memory form, [disp16])
    // 0xC6 0x06 0x00 0x10 0xAB = MOV byte [0x1000], 0xAB
    //   ModR/M 0x06 = mod=00 reg=000 r/m=110 → [disp16]
    let r = decode(&[0xC6, 0x06, 0x00, 0x10, 0xAB], ip0()).unwrap();
    assert_eq!(r.bytes, vec![0xC6, 0x06, 0x00, 0x10, 0xAB]);
    match r.kind {
        InstructionKind::Mov { dest, src } => {
            match dest {
                Operand::Mem(m) => {
                    assert_eq!(m.base, MemBase::Direct);
                    assert_eq!(m.disp, 0x1000u16 as i16);
                    assert_eq!(m.size, OpSize::Byte);
                }
                _ => panic!("expected Mem"),
            }
            assert_eq!(src, Operand::Imm8(0xAB));
        }
        _ => panic!("expected Mov"),
    }
}

#[test]
fn decode_mov_rm16_imm_mem_form() {
    // 0xC7 /0 iw — MOV r/m16, imm16 (memory form, [disp16])
    // 0xC7 0x06 0x00 0x10 0x34 0x12 = MOV word [0x1000], 0x1234
    let r = decode(&[0xC7, 0x06, 0x00, 0x10, 0x34, 0x12], ip0()).unwrap();
    assert_eq!(r.bytes, vec![0xC7, 0x06, 0x00, 0x10, 0x34, 0x12]);
    match r.kind {
        InstructionKind::Mov { dest, src } => {
            match dest {
                Operand::Mem(m) => {
                    assert_eq!(m.base, MemBase::Direct);
                    assert_eq!(m.disp, 0x1000u16 as i16);
                    assert_eq!(m.size, OpSize::Word);
                }
                _ => panic!("expected Mem"),
            }
            assert_eq!(src, Operand::Imm16(0x1234));
        }
        _ => panic!("expected Mov"),
    }
}

// Canonical round-trip tests (spec sections 1.1 and 4.4.1).

#[test]
fn decode_mov_reg_reg_via_8b_form_yields_correct_kind() {
    // 0x8B 0xE0 = MOV r16, r/m16 (r ← r/m form)
    //   ModR/M 0xE0 = mod=11 reg=100(SP) r/m=000(AX)
    //   → dest=reg=SP, src=r/m=AX → MOV SP, AX
    let r = decode(&[0x8B, 0xE0], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Mov {
            dest: Operand::Reg16(Reg16::SP),
            src: Operand::Reg16(Reg16::AX),
        }
    );
}

#[test]
fn decode_mov_reg_reg_two_encodings_yield_same_kind() {
    // Two redundant encodings of MOV SP, AX must produce the same InstructionKind
    // for the canonical round trip to hold.
    //
    //   0x89 0xC4 = MOV r/m16, r16 (r/m ← r form, dir=0)
    //     ModR/M 0xC4 = mod=11 reg=000(AX) r/m=100(SP)
    //     → dest=r/m=SP, src=reg=AX → MOV SP, AX
    //
    //   0x8B 0xE0 = MOV r16, r/m16 (r ← r/m form, dir=1)
    //     ModR/M 0xE0 = mod=11 reg=100(SP) r/m=000(AX)
    //     → dest=reg=SP, src=r/m=AX → MOV SP, AX
    let r1 = decode(&[0x89, 0xC4], ip0()).unwrap();
    let r2 = decode(&[0x8B, 0xE0], ip0()).unwrap();
    assert_eq!(
        r1.kind, r2.kind,
        "redundant encodings produced different kinds"
    );
    match r1.kind {
        InstructionKind::Mov {
            dest: Operand::Reg16(Reg16::SP),
            src: Operand::Reg16(Reg16::AX),
        } => {}
        other => panic!("expected MOV SP, AX, got {:?}", other),
    }
}

// ─── ALU family (ADD/OR/ADC/SBB/AND/SUB/XOR/CMP) ────────────────────────────
//
// Eight operations share six direct encodings and the Group 1 immediate forms.

#[test]
fn decode_alu_add_rm_r() {
    // 0x01 0xC3 = ADD BX, AX (mod=11 reg=000=AX r/m=011=BX) → ADD r/m16, r16 form
    let r = decode(&[0x01, 0xC3], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Add {
            dest: Operand::Reg16(Reg16::BX),
            src: Operand::Reg16(Reg16::AX),
        }
    );
}

#[test]
fn decode_alu_sub_r_rm() {
    // 0x2B 0xD8 = SUB BX, AX (r ← r/m form, reg=011=BX r/m=000=AX)
    let r = decode(&[0x2B, 0xD8], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Sub {
            dest: Operand::Reg16(Reg16::BX),
            src: Operand::Reg16(Reg16::AX),
        }
    );
}

#[test]
fn decode_alu_cmp_al_imm() {
    // 0x3C 0x42 = CMP AL, 0x42
    let r = decode(&[0x3C, 0x42], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Cmp {
            a: Operand::Reg8(Reg8::AL),
            b: Operand::Imm8(0x42),
        }
    );
}

#[test]
fn decode_alu_cmp_ax_imm16() {
    // 0x3D 0x34 0x12 = CMP AX, 0x1234
    let r = decode(&[0x3D, 0x34, 0x12], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Cmp {
            a: Operand::Reg16(Reg16::AX),
            b: Operand::Imm16(0x1234),
        }
    );
}

#[test]
fn decode_alu_adc_and_sbb_accumulator_immediates() {
    assert_eq!(
        decode(&[0x15, 0x34, 0x12], ip0()).unwrap().kind,
        InstructionKind::Adc {
            dest: Operand::Reg16(Reg16::AX),
            src: Operand::Imm16(0x1234),
        }
    );
    assert_eq!(
        decode(&[0x1C, 0x7F], ip0()).unwrap().kind,
        InstructionKind::Sbb {
            dest: Operand::Reg8(Reg8::AL),
            src: Operand::Imm8(0x7F),
        }
    );
}

#[test]
fn decode_alu_xor_self() {
    // 0x33 0xC0 = XOR AX, AX (a common zeroing idiom).
    // r ← r/m form: reg=000=AX r/m=000=AX
    let r = decode(&[0x33, 0xC0], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Xor {
            dest: Operand::Reg16(Reg16::AX),
            src: Operand::Reg16(Reg16::AX),
        }
    );
}

#[test]
fn decode_alu_or_rm_r_byte() {
    // 0x08 /r — OR r/m8, r8
    // 0x08 0xC1 = mod=11 reg=000(AL) r/m=001(CL) → OR CL, AL
    let r = decode(&[0x08, 0xC1], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Or {
            dest: Operand::Reg8(Reg8::CL),
            src: Operand::Reg8(Reg8::AL),
        }
    );
}

#[test]
fn decode_alu_and_al_imm() {
    // 0x24 ib = AND AL, imm8
    let r = decode(&[0x24, 0x0F], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::And {
            dest: Operand::Reg8(Reg8::AL),
            src: Operand::Imm8(0x0F),
        }
    );
}

#[test]
fn decode_alu_group1_imm16() {
    // 0x81 /4 iw = AND r/m16, imm16
    // 0x81 0xE0 0xFF 0x00 = AND AX, 0x00FF (mod=11 reg=100=AND r/m=000=AX)
    let r = decode(&[0x81, 0xE0, 0xFF, 0x00], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::And {
            dest: Operand::Reg16(Reg16::AX),
            src: Operand::Imm16(0x00FF),
        }
    );
}

#[test]
fn decode_alu_group1_imm8_sx() {
    // 0x83 /5 ib(sx) = SUB r/m16, sign-extended imm8
    // 0x83 0xEC 0x10 = SUB SP, 0x10 (mod=11 reg=101=SUB r/m=100=SP)
    let r = decode(&[0x83, 0xEC, 0x10], ip0()).unwrap();
    // sign-extended 0x10 = 0x0010
    assert_eq!(
        r.kind,
        InstructionKind::Sub {
            dest: Operand::Reg16(Reg16::SP),
            src: Operand::Imm16(0x0010),
        }
    );
}

#[test]
fn decode_alu_group1_imm8_sx_negative() {
    // 0x83 /0 ib(sx) = ADD r/m16, sign-extended negative imm8.
    // 0x83 0xC4 0xFE = ADD SP, -2 (mod=11 reg=000=ADD r/m=100=SP, imm=0xFE=-2 sign-extended → 0xFFFE)
    let r = decode(&[0x83, 0xC4, 0xFE], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Add {
            dest: Operand::Reg16(Reg16::SP),
            src: Operand::Imm16(0xFFFE),
        }
    );
}

#[test]
fn decode_alu_group1_byte_imm() {
    // 0x80 /7 ib = CMP r/m8, imm8
    // 0x80 0xF8 0x42 = CMP AL, 0x42 (mod=11 reg=111=CMP r/m=000=AL)
    let r = decode(&[0x80, 0xF8, 0x42], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Cmp {
            a: Operand::Reg8(Reg8::AL),
            b: Operand::Imm8(0x42),
        }
    );
}

#[test]
fn decode_alu_adc_direct_and_group1() {
    let r = decode(&[0x10, 0xC3], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Adc {
            dest: Operand::Reg8(Reg8::BL),
            src: Operand::Reg8(Reg8::AL),
        }
    );

    // 0x80 0xD0 0x42 = ADC AL, 0x42 (mod=11 reg=010=ADC r/m=000=AL)
    let r = decode(&[0x80, 0xD0, 0x42], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Adc {
            dest: Operand::Reg8(Reg8::AL),
            src: Operand::Imm8(0x42),
        }
    );
}

#[test]
fn decode_alu_sbb_direct_and_group1() {
    let r = decode(&[0x19, 0xC3], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Sbb {
            dest: Operand::Reg16(Reg16::BX),
            src: Operand::Reg16(Reg16::AX),
        }
    );

    let r = decode(&[0x83, 0xDC, 0x10], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Sbb {
            dest: Operand::Reg16(Reg16::SP),
            src: Operand::Imm16(0x10),
        }
    );
}

// ─── TEST + INC/DEC + NEG + Group 5 indirect JMP/CALL ───────────────────────

#[test]
fn decode_test_rm_r() {
    // 0x85 0xC0 = TEST AX, AX (mod=11 reg=000=AX r/m=000=AX)
    let r = decode(&[0x85, 0xC0], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Test {
            a: Operand::Reg16(Reg16::AX),
            b: Operand::Reg16(Reg16::AX),
        }
    );
}

#[test]
fn decode_test_rm_r_byte() {
    // 0x84 0xC1 = TEST CL, AL (mod=11 reg=000=AL r/m=001=CL)
    let r = decode(&[0x84, 0xC1], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Test {
            a: Operand::Reg8(Reg8::CL),
            b: Operand::Reg8(Reg8::AL),
        }
    );
}

#[test]
fn decode_test_al_imm() {
    // 0xA8 0x42 = TEST AL, 0x42
    let r = decode(&[0xA8, 0x42], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Test {
            a: Operand::Reg8(Reg8::AL),
            b: Operand::Imm8(0x42),
        }
    );
}

#[test]
fn decode_test_ax_imm() {
    // 0xA9 0xFF 0x00 = TEST AX, 0x00FF
    let r = decode(&[0xA9, 0xFF, 0x00], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Test {
            a: Operand::Reg16(Reg16::AX),
            b: Operand::Imm16(0x00FF),
        }
    );
}

#[test]
fn decode_test_rm_imm_via_f7_0() {
    // 0xF7 /0 iw = TEST r/m16, imm16
    // 0xF7 0xC0 0x34 0x12 = TEST AX, 0x1234 (mod=11 reg=000 r/m=000=AX)
    let r = decode(&[0xF7, 0xC0, 0x34, 0x12], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Test {
            a: Operand::Reg16(Reg16::AX),
            b: Operand::Imm16(0x1234),
        }
    );
}

#[test]
fn decode_test_rm_imm_via_f6_0() {
    // 0xF6 /0 ib = TEST r/m8, imm8
    // 0xF6 0xC1 0x42 = TEST CL, 0x42 (mod=11 reg=000 r/m=001=CL)
    let r = decode(&[0xF6, 0xC1, 0x42], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Test {
            a: Operand::Reg8(Reg8::CL),
            b: Operand::Imm8(0x42),
        }
    );
}

#[test]
fn decode_inc_dec_reg16_short() {
    // 0x40 = INC AX, 0x47 = INC DI
    assert_eq!(
        decode(&[0x40], ip0()).unwrap().kind,
        InstructionKind::Inc {
            dest: Operand::Reg16(Reg16::AX)
        }
    );
    assert_eq!(
        decode(&[0x47], ip0()).unwrap().kind,
        InstructionKind::Inc {
            dest: Operand::Reg16(Reg16::DI)
        }
    );

    // 0x48 = DEC AX, 0x4F = DEC DI
    assert_eq!(
        decode(&[0x48], ip0()).unwrap().kind,
        InstructionKind::Dec {
            dest: Operand::Reg16(Reg16::AX)
        }
    );
    assert_eq!(
        decode(&[0x4F], ip0()).unwrap().kind,
        InstructionKind::Dec {
            dest: Operand::Reg16(Reg16::DI)
        }
    );
}

#[test]
fn decode_inc_rm_via_ff_0() {
    // 0xFF /0 = INC r/m16
    // 0xFF 0xC3 = INC BX (mod=11 reg=000 r/m=011)
    let r = decode(&[0xFF, 0xC3], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Inc {
            dest: Operand::Reg16(Reg16::BX)
        }
    );
}

#[test]
fn decode_dec_rm8_via_fe_1() {
    // 0xFE /1 = DEC r/m8
    // 0xFE 0xC9 = DEC CL (mod=11 reg=001 r/m=001)
    let r = decode(&[0xFE, 0xC9], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Dec {
            dest: Operand::Reg8(Reg8::CL)
        }
    );
}

#[test]
fn decode_inc_rm8_via_fe_0() {
    // 0xFE /0 = INC r/m8
    // 0xFE 0xC0 = INC AL (mod=11 reg=000 r/m=000)
    let r = decode(&[0xFE, 0xC0], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Inc {
            dest: Operand::Reg8(Reg8::AL)
        }
    );
}

#[test]
fn decode_neg_via_f7_3() {
    // 0xF7 /3 = NEG r/m16
    // 0xF7 0xD8 = NEG AX (mod=11 reg=011 r/m=000)
    let r = decode(&[0xF7, 0xD8], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Neg {
            dest: Operand::Reg16(Reg16::AX)
        }
    );
}

#[test]
fn decode_group3_mul_div_family() {
    assert_eq!(
        decode(&[0xF6, 0xF1], ip0()).unwrap().kind,
        InstructionKind::Div {
            src: Operand::Reg8(Reg8::CL),
        }
    );
    assert_eq!(
        decode(&[0xF7, 0xE3], ip0()).unwrap().kind,
        InstructionKind::Mul {
            src: Operand::Reg16(Reg16::BX),
        }
    );
}

#[test]
fn decode_neg_via_f6_3() {
    // 0xF6 /3 = NEG r/m8
    // 0xF6 0xD9 = NEG CL (mod=11 reg=011 r/m=001)
    let r = decode(&[0xF6, 0xD9], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Neg {
            dest: Operand::Reg8(Reg8::CL)
        }
    );
}

#[test]
fn decode_call_indirect_via_ff_2() {
    // 0xFF /2 = CALL near r/m16
    // 0xFF 0xD3 = CALL BX (mod=11 reg=010 r/m=011)
    let r = decode(&[0xFF, 0xD3], ip0()).unwrap();
    match r.kind {
        InstructionKind::Call {
            target: CallTarget::IndirectNear(op),
        } => {
            assert_eq!(op, Operand::Reg16(Reg16::BX));
        }
        _ => panic!("expected Call IndirectNear"),
    }
}

#[test]
fn decode_jmp_indirect_via_ff_4() {
    // 0xFF /4 = JMP near r/m16
    // 0xFF 0xE3 = JMP BX (mod=11 reg=100 r/m=011)
    let r = decode(&[0xFF, 0xE3], ip0()).unwrap();
    match r.kind {
        InstructionKind::Jmp {
            target: JmpTarget::IndirectNear(op),
        } => {
            assert_eq!(op, Operand::Reg16(Reg16::BX));
        }
        _ => panic!("expected Jmp IndirectNear"),
    }
}

#[test]
fn decode_call_far_indirect() {
    // 0xFF /3 = indirect far CALL m16:16; memory operand only.
    // 0xFF 0x1E 0x34 0x12 = CALL FAR [0x1234]
    //   ModR/M 0x1E = mod=00 reg=011(/3) r/m=110 → [disp16]
    let r = decode(&[0xFF, 0x1E, 0x34, 0x12], ip0()).unwrap();
    match r.kind {
        InstructionKind::Call {
            target: CallTarget::IndirectFar(m),
        } => {
            assert_eq!(m.base, MemBase::Direct);
            assert_eq!(m.disp, 0x1234u16 as i16);
            assert_eq!(m.size, OpSize::Word);
            assert_eq!(m.seg, None);
        }
        other => panic!("expected Call IndirectFar, got {:?}", other),
    }
    assert_eq!(r.bytes, vec![0xFF, 0x1E, 0x34, 0x12]);
}

#[test]
fn decode_jmp_far_indirect() {
    // 0xFF /5 = indirect far JMP m16:16; memory operand only.
    // 0xFF 0x2E 0x34 0x12 = JMP FAR [0x1234]
    //   ModR/M 0x2E = mod=00 reg=101(/5) r/m=110 → [disp16]
    let r = decode(&[0xFF, 0x2E, 0x34, 0x12], ip0()).unwrap();
    match r.kind {
        InstructionKind::Jmp {
            target: JmpTarget::IndirectFar(m),
        } => {
            assert_eq!(m.base, MemBase::Direct);
            assert_eq!(m.disp, 0x1234u16 as i16);
            assert_eq!(m.size, OpSize::Word);
            assert_eq!(m.seg, None);
        }
        other => panic!("expected Jmp IndirectFar, got {:?}", other),
    }
    assert_eq!(r.bytes, vec![0xFF, 0x2E, 0x34, 0x12]);
}

#[test]
fn decode_call_far_indirect_reg_form_unknown() {
    // The 0xFF /3 register form (mod=11) is undefined and must become Unknown.
    // Far indirect addressing loads a 32-bit far pointer from memory and has no register form.
    // 0xFF 0xDB = mod=11 reg=011(/3) r/m=011(BX)
    let r = decode(&[0xFF, 0xDB], ip0()).unwrap();
    assert!(
        matches!(r.kind, InstructionKind::Unknown { .. }),
        "0xFF /3 register form must be Unknown, got {:?}",
        r.kind
    );
}

#[test]
fn decode_jmp_far_indirect_reg_form_unknown() {
    // The 0xFF /5 register form (mod=11) is likewise undefined and becomes Unknown.
    // 0xFF 0xEB = mod=11 reg=101(/5) r/m=011(BX)
    let r = decode(&[0xFF, 0xEB], ip0()).unwrap();
    assert!(
        matches!(r.kind, InstructionKind::Unknown { .. }),
        "0xFF /5 register form must be Unknown, got {:?}",
        r.kind
    );
}

#[test]
fn decode_alu_byte_form_0x82() {
    // 0x82 is an alternate byte form of 0x80 (ALU r/m8, imm8). Both appear in the
    // official 8086 table, though newer manuals recommend 0x80. Both decode to one kind.
    // 0x82 0xF8 0x42 = CMP AL, 0x42 (mod=11 reg=111=CMP r/m=000=AL)
    let r = decode(&[0x82, 0xF8, 0x42], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Cmp {
            a: Operand::Reg8(Reg8::AL),
            b: Operand::Imm8(0x42),
        }
    );
}

#[test]
fn decode_push_rm16_via_ff_6_still_works() {
    // 0xFF /6 = PUSH r/m16 must still work after consolidating Group 5.
    // 0xFF 0x36 0x34 0x12 = PUSH word [0x1234]
    let r = decode(&[0xFF, 0x36, 0x34, 0x12], ip0()).unwrap();
    match r.kind {
        InstructionKind::Push {
            src: Operand::Mem(m),
        } => {
            assert_eq!(m.base, MemBase::Direct);
            assert_eq!(m.disp, 0x1234u16 as i16);
            assert_eq!(m.size, OpSize::Word);
        }
        other => panic!("expected Push Mem, got {:?}", other),
    }
}

#[test]
fn decode_group3_not() {
    let r = decode(&[0xF6, 0xD0], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Not {
            dest: Operand::Reg8(Reg8::AL),
        }
    );
}

#[test]
fn decode_group4_unsupported_returns_unknown() {
    // 0xFE /2..7 are invalid and become Unknown.
    // 0xFE 0xD0 = (reg=010 r/m=000 AL) → Unknown
    let r = decode(&[0xFE, 0xD0], ip0()).unwrap();
    assert!(matches!(r.kind, InstructionKind::Unknown { .. }));
}

#[test]
fn decode_pop_rm16_reg_form_yields_pop_reg_kind() {
    // The register form of POP r/m16 (0x8F /0) must decode to the same kind as
    // short form 0x58+r; emission always selects the short form.
    //   0x8F 0xC3 = mod=11 reg=000 r/m=011(BX) → POP BX
    //   0x5B = POP BX (short form)
    let group_form = decode(&[0x8F, 0xC3], ip0()).unwrap();
    let short_form = decode(&[0x5B], ip0()).unwrap();
    assert_eq!(group_form.kind, short_form.kind);
    assert_eq!(
        group_form.kind,
        InstructionKind::Pop {
            dest: Operand::Reg16(Reg16::BX)
        }
    );
}

// ----------------------------------------------------------------------------
// Phase D-4: shifts/rotates, I/O, LEA, and XCHG.
// ----------------------------------------------------------------------------

#[test]
fn decode_shift_rol_one() {
    // 0xD1 /0 = ROL r/m16, 1
    // 0xD1 0xC0 = ROL AX, 1
    let r = decode(&[0xD1, 0xC0], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Rol {
            dest: Operand::Reg16(Reg16::AX),
            count: ShiftCount::One
        }
    );
}

#[test]
fn decode_shift_through_carry() {
    assert_eq!(
        decode(&[0xD0, 0xD8], ip0()).unwrap().kind,
        InstructionKind::Rcr {
            dest: Operand::Reg8(Reg8::AL),
            count: ShiftCount::One,
        }
    );
    assert_eq!(
        decode(&[0xD1, 0xD0], ip0()).unwrap().kind,
        InstructionKind::Rcl {
            dest: Operand::Reg16(Reg16::AX),
            count: ShiftCount::One,
        }
    );
}

#[test]
fn decode_shift_shl_cl() {
    // 0xD3 /4 = SHL r/m16, CL
    // 0xD3 0xE0 = SHL AX, CL (mod=11 reg=100 r/m=000)
    let r = decode(&[0xD3, 0xE0], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Shl {
            dest: Operand::Reg16(Reg16::AX),
            count: ShiftCount::Cl
        }
    );
}

#[test]
fn decode_shift_sar_one() {
    // 0xD1 /7 = SAR r/m16, 1
    // 0xD1 0xF8 = SAR AX, 1 (mod=11 reg=111 r/m=000)
    let r = decode(&[0xD1, 0xF8], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Sar {
            dest: Operand::Reg16(Reg16::AX),
            count: ShiftCount::One
        }
    );
}

#[test]
fn decode_io_in_al_imm() {
    // 0xE4 0x60 = IN AL, 0x60
    let r = decode(&[0xE4, 0x60], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::InAl {
            port: PortAddr::Imm8(0x60)
        }
    );
}

#[test]
fn decode_io_out_dx() {
    // 0xEE = OUT DX, AL
    let r = decode(&[0xEE], ip0()).unwrap();
    assert_eq!(r.kind, InstructionKind::OutAl { port: PortAddr::Dx });

    // 0xEF = OUT DX, AX
    let r = decode(&[0xEF], ip0()).unwrap();
    assert_eq!(r.kind, InstructionKind::OutAx { port: PortAddr::Dx });
}

#[test]
fn decode_lea() {
    // 0x8D /r = LEA r16, m
    // 0x8D 0x46 0x04 = LEA AX, [BP+4] (mod=01 reg=000=AX r/m=110=BP, disp8=04)
    let r = decode(&[0x8D, 0x46, 0x04], ip0()).unwrap();
    match r.kind {
        InstructionKind::Lea { dest, src } => {
            assert_eq!(dest, Reg16::AX);
            assert_eq!(src.base, MemBase::Bp);
            assert_eq!(src.disp, 4);
        }
        _ => panic!("expected Lea"),
    }
}

#[test]
fn decode_les_lds_and_xlat() {
    let les = decode(&[0xC4, 0x1E, 0x34, 0x12], ip0()).unwrap();
    assert!(matches!(
        les.kind,
        InstructionKind::Les {
            dest: Reg16::BX,
            src: Mem {
                base: MemBase::Direct,
                disp: 0x1234,
                ..
            }
        }
    ));
    let lds = decode(&[0xC5, 0x36, 0x78, 0x56], ip0()).unwrap();
    assert!(matches!(
        lds.kind,
        InstructionKind::Lds {
            dest: Reg16::SI,
            src: Mem {
                base: MemBase::Direct,
                disp: 0x5678,
                ..
            }
        }
    ));
    assert_eq!(decode(&[0xD7], ip0()).unwrap().kind, InstructionKind::Xlat);
}

#[test]
fn decode_xchg_ax_short() {
    // 0x91 = XCHG AX, CX (0x90+rw)
    let r = decode(&[0x91], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Xchg {
            a: Operand::Reg16(Reg16::AX),
            b: Operand::Reg16(Reg16::CX),
        }
    );
    // 0x90 is the special NOP encoding; XCHG AX, AX decodes as NOP.
    assert_eq!(decode(&[0x90], ip0()).unwrap().kind, InstructionKind::Nop);
}

#[test]
fn decode_xchg_rm_r() {
    // 0x87 /r = XCHG r/m16, r16
    // 0x87 0xC3 = XCHG BX, AX
    let r = decode(&[0x87, 0xC3], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Xchg {
            a: Operand::Reg16(Reg16::BX),
            b: Operand::Reg16(Reg16::AX),
        }
    );
}

// ----------------------------------------------------------------------------
// Phase D-4: string instructions, REP/REPNE, and segment override prefixes.
// ----------------------------------------------------------------------------

#[test]
fn decode_string_movsb() {
    let r = decode(&[0xA4], ip0()).unwrap();
    assert_eq!(r.kind, InstructionKind::Movsb);
}

#[test]
fn decode_string_all() {
    let cases: &[(u8, InstructionKind)] = &[
        (0xA4, InstructionKind::Movsb),
        (0xA5, InstructionKind::Movsw),
        (0xA6, InstructionKind::Cmpsb),
        (0xA7, InstructionKind::Cmpsw),
        (0xAA, InstructionKind::Stosb),
        (0xAB, InstructionKind::Stosw),
        (0xAC, InstructionKind::Lodsb),
        (0xAD, InstructionKind::Lodsw),
        (0xAE, InstructionKind::Scasb),
        (0xAF, InstructionKind::Scasw),
    ];
    for (op, expected) in cases {
        let r = decode(&[*op], ip0()).unwrap();
        assert_eq!(r.kind, *expected, "opcode 0x{:02X}", op);
    }
}

#[test]
fn decode_rep_movsb() {
    // 0xF3 0xA4 = REP MOVSB
    let r = decode(&[0xF3, 0xA4], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Rep(Box::new(InstructionKind::Movsb))
    );
    assert_eq!(r.bytes, vec![0xF3, 0xA4]);
}

#[test]
fn decode_repne_scasb() {
    // 0xF2 0xAE = REPNE SCASB
    let r = decode(&[0xF2, 0xAE], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Repne(Box::new(InstructionKind::Scasb))
    );
    assert_eq!(r.bytes, vec![0xF2, 0xAE]);
}

#[test]
fn decode_segment_override_es_mov() {
    // 0x26 0x8B 0x07 = MOV AX, ES:[BX] (ES override + MOV r16, r/m16, mod=00 r/m=111=BX)
    let r = decode(&[0x26, 0x8B, 0x07], ip0()).unwrap();
    assert_eq!(r.bytes, vec![0x26, 0x8B, 0x07]);
    match r.kind {
        InstructionKind::Mov { dest, src } => {
            assert_eq!(dest, Operand::Reg16(Reg16::AX));
            match src {
                Operand::Mem(m) => {
                    assert_eq!(m.base, MemBase::Bx);
                    assert_eq!(m.seg, Some(Sreg::ES));
                }
                _ => panic!("expected Mem with seg=ES"),
            }
        }
        _ => panic!("expected Mov"),
    }
}

#[test]
fn decode_bus_lock_prefix_preserved() {
    // 0xF0 0x87 0x07 = BUSLOCK XCHG [BW], AW; preserve both prefix and typed meaning.
    let r = decode(&[0xF0, 0x87, 0x07], ip0()).unwrap();
    assert!(r.prefixes.bus_lock);
    assert!(matches!(
        r.kind,
        InstructionKind::BusLock(inner)
            if matches!(*inner, InstructionKind::Xchg {
                a: Operand::Mem(Mem { base: MemBase::Bx, .. }),
                b: Operand::Reg16(Reg16::AX),
            })
    ));
    assert_eq!(r.bytes, vec![0xF0, 0x87, 0x07]);
}

#[test]
fn decode_rep_on_non_string_falls_through() {
    // REP 0xF3 on a non-string instruction does not create a semantic wrapper.
    // 0xF3 0x90 = REP NOP; this decoder returns NOP only.
    let r = decode(&[0xF3, 0x90], ip0()).unwrap();
    assert_eq!(r.kind, InstructionKind::Nop);
}

// ----------------------------------------------------------------------------
// Flag instructions (Intel SDM Volume 2A, Appendix B) and sign extension (CBW/CWD).
// ----------------------------------------------------------------------------

#[test]
fn decode_flag_pushf_popf() {
    // 0x9C = PUSHF, 0x9D = POPF
    assert_eq!(decode(&[0x9C], ip0()).unwrap().kind, InstructionKind::Pushf);
    assert_eq!(decode(&[0x9D], ip0()).unwrap().kind, InstructionKind::Popf);
}

#[test]
fn decode_flag_sahf_lahf() {
    // 0x9E = SAHF (AH → flags low), 0x9F = LAHF (flags low → AH)
    assert_eq!(decode(&[0x9E], ip0()).unwrap().kind, InstructionKind::Sahf);
    assert_eq!(decode(&[0x9F], ip0()).unwrap().kind, InstructionKind::Lahf);
}

#[test]
fn decode_flag_wait() {
    // 0x9B = WAIT (FPU synchronization; nearly equivalent to NOP without an FPU).
    assert_eq!(decode(&[0x9B], ip0()).unwrap().kind, InstructionKind::Wait);
}

#[test]
fn decode_flag_carry() {
    // 0xF5 = CMC, 0xF8 = CLC, 0xF9 = STC
    assert_eq!(decode(&[0xF5], ip0()).unwrap().kind, InstructionKind::Cmc);
    assert_eq!(decode(&[0xF8], ip0()).unwrap().kind, InstructionKind::Clc);
    assert_eq!(decode(&[0xF9], ip0()).unwrap().kind, InstructionKind::Stc);
}

#[test]
fn decode_flag_interrupt() {
    // 0xFA = CLI, 0xFB = STI
    assert_eq!(decode(&[0xFA], ip0()).unwrap().kind, InstructionKind::Cli);
    assert_eq!(decode(&[0xFB], ip0()).unwrap().kind, InstructionKind::Sti);
}

#[test]
fn decode_flag_direction() {
    // 0xFC = CLD, 0xFD = STD
    assert_eq!(decode(&[0xFC], ip0()).unwrap().kind, InstructionKind::Cld);
    assert_eq!(decode(&[0xFD], ip0()).unwrap().kind, InstructionKind::Std);
}

#[test]
fn decode_sign_extend_cbw_cwd() {
    // 0x98 = CBW (AL→AX), 0x99 = CWD (AX→DX:AX)
    assert_eq!(decode(&[0x98], ip0()).unwrap().kind, InstructionKind::Cbw);
    assert_eq!(decode(&[0x99], ip0()).unwrap().kind, InstructionKind::Cwd);
}

// ----------------------------------------------------------------------------
// V30 (uPD70116) extension prefixes: REPC (0x65) and REPNC (0x64).
// ----------------------------------------------------------------------------

#[test]
fn decode_v30_repc_prefix_sets_flag() {
    // 0x65 0xA6 = REPC CMPSB (V30).
    let r = decode(&[0x65, 0xA6], ip0()).unwrap();
    assert!(
        r.prefixes.repc,
        "REPC prefix must be preserved in Prefixes.repc"
    );
    assert!(!r.prefixes.repnc);
    assert_eq!(r.bytes, vec![0x65, 0xA6]);
}

#[test]
fn decode_v30_repnc_prefix_sets_flag() {
    // 0x64 0xAE = REPNC SCASB (V30)
    let r = decode(&[0x64, 0xAE], ip0()).unwrap();
    assert!(
        r.prefixes.repnc,
        "REPNC prefix must be preserved in Prefixes.repnc"
    );
    assert!(!r.prefixes.repc);
    assert_eq!(r.bytes, vec![0x64, 0xAE]);
}

// ─────────────────────────────────────────────────────────────────────────────
// V30 (NEC uPD70116) 0x0F secondary-opcode decoding tests.
// Source: NEC uPD70116 data sheet, section 3, "Instruction Set Extensions".
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn decode_v30_add4s() {
    // 0F 20 — ADD4S (no operand BCD string add)
    let r = decode(&[0x0F, 0x20], ip0()).unwrap();
    assert_eq!(r.kind, InstructionKind::Add4s);
    assert_eq!(r.bytes, vec![0x0F, 0x20]);
}

#[test]
fn decode_v30_sub4s_cmp4s() {
    let r = decode(&[0x0F, 0x22], ip0()).unwrap();
    assert_eq!(r.kind, InstructionKind::Sub4s);
    let r = decode(&[0x0F, 0x26], ip0()).unwrap();
    assert_eq!(r.kind, InstructionKind::Cmp4s);
}

#[test]
fn decode_v30_rol4_register() {
    // 0F 28 /0 — ROL4 r/m8. mod=11 r/m=011 → BL
    let r = decode(&[0x0F, 0x28, 0xC3], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Rol4 {
            rm: Operand::Reg8(Reg8::BL),
        }
    );
    assert_eq!(r.bytes, vec![0x0F, 0x28, 0xC3]);
}

#[test]
fn decode_v30_ror4_memory() {
    // 0F 2A — ROR4 r/m8. mod=00 r/m=110 → [disp16=0x1234]
    let r = decode(&[0x0F, 0x2A, 0x06, 0x34, 0x12], ip0()).unwrap();
    match r.kind {
        InstructionKind::Ror4 {
            rm: Operand::Mem(m),
        } => {
            assert_eq!(m.base, MemBase::Direct);
            assert_eq!(m.disp, 0x1234u16 as i16);
            assert_eq!(m.size, OpSize::Byte);
        }
        other => panic!("expected Ror4 mem, got {other:?}"),
    }
}

#[test]
fn decode_v30_ins_register() {
    // 0F 31 /r — INS bit-offset register, length register.
    let r = decode(&[0x0F, 0x31, 0xD4], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Ins {
            bit_offset: Reg8::AH,
            length: Reg8::DL,
        }
    );
}

#[test]
fn decode_v30_ext_register() {
    // 0F 33 /r — EXT bit-offset register, length register.
    let r = decode(&[0x0F, 0x33, 0xC8], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Ext {
            bit_offset: Reg8::AL,
            length: Reg8::CL,
        }
    );
}

#[test]
fn decode_v30_ins_imm() {
    // 0F 39 /r ib — INS r/m, imm8. ModR/M = 11 000 011 = 0xC3 → r/m=BL, imm=0x07
    let r = decode(&[0x0F, 0x39, 0xC3, 0x07], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::InsImm {
            bit_offset: Reg8::BL,
            length: BitFieldLength::try_from(0x07).unwrap(),
        }
    );
    assert_eq!(r.bytes.len(), 4);
}

#[test]
fn decode_v30_ext_imm() {
    // 0F 3B /r ib — EXT r/m, imm8. ModR/M = 0xC1 → r/m=CL, imm=0x0F
    let r = decode(&[0x0F, 0x3B, 0xC1, 0x0F], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::ExtImm {
            bit_offset: Reg8::CL,
            length: BitFieldLength::try_from(0x0F).unwrap(),
        }
    );
}

#[test]
fn decode_v30_brkem() {
    // 0F FF ib — BRKEM imm8 (native static-analysis boundary)
    let r = decode(&[0x0F, 0xFF, 0x42], ip0()).unwrap();
    assert_eq!(r.kind, InstructionKind::Brkem { vector: 0x42 });
    assert_eq!(r.bytes, vec![0x0F, 0xFF, 0x42]);
}

#[test]
fn decode_v30_unknown_secondary_two_byte_unknown() {
    // 0F 99 is absent from the V30 reference and becomes a two-byte Unknown,
    // advancing one byte farther than the 8086 path.
    let r = decode(&[0x0F, 0x99], ip0()).unwrap();
    assert_eq!(
        r.kind,
        InstructionKind::Unknown {
            raw: vec![0x0F, 0x99],
        }
    );
    assert_eq!(r.bytes, vec![0x0F, 0x99]);
}

#[test]
fn decode_v30_truncated_secondary_falls_back() {
    // A lone 0x0F becomes a one-byte Unknown to preserve sweep continuity.
    let r = decode(&[0x0F], ip0()).unwrap();
    assert_eq!(r.kind, InstructionKind::Unknown { raw: vec![0x0F] });
    assert_eq!(r.bytes, vec![0x0F]);
}
