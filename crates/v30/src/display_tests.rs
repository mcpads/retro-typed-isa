//! `display::format_line` 골든 출력 테스트.

use super::display::*;
use super::instruction::*;

fn at(off: u16) -> SegOff {
    SegOff { seg: 0, off }
}

#[test]
fn display_simple_nop() {
    let insn = Instruction {
        cs_ip: SegOff {
            seg: 0x40,
            off: 0x100,
        },
        bytes: vec![0x90],
        kind: InstructionKind::Nop,
        prefixes: Prefixes::default(),
    };
    assert_eq!(format_line(&insn), "0040:0100  90               nop");
}

#[test]
fn display_mov_imm() {
    let insn = Instruction {
        cs_ip: SegOff {
            seg: 0x40,
            off: 0x100,
        },
        bytes: vec![0xB8, 0x34, 0x12],
        kind: InstructionKind::Mov {
            dest: Operand::Reg16(Reg16::AX),
            src: Operand::Imm16(0x1234),
        },
        prefixes: Prefixes::default(),
    };
    assert_eq!(
        format_line(&insn),
        "0040:0100  B8 34 12         mov ax, 0x1234"
    );
}

#[test]
fn display_int() {
    let insn = Instruction {
        cs_ip: SegOff {
            seg: 0x40,
            off: 0x100,
        },
        bytes: vec![0xCD, 0x18],
        kind: InstructionKind::Int { vector: 0x18 },
        prefixes: Prefixes::default(),
    };
    assert_eq!(format_line(&insn), "0040:0100  CD 18            int 0x18");
}

#[test]
fn display_mem_with_seg() {
    let mem = Mem {
        seg: Some(Sreg::ES),
        base: MemBase::Bx,
        disp: 0,
        size: OpSize::Word,
    };
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0x26, 0x8B, 0x07],
        kind: InstructionKind::Mov {
            dest: Operand::Reg16(Reg16::AX),
            src: Operand::Mem(mem),
        },
        prefixes: Prefixes {
            seg_override: Some(Sreg::ES),
            bus_lock: false,
            repc: false,
            repnc: false,
        },
    };
    assert_eq!(
        format_line(&insn),
        "0000:0100  26 8B 07         mov ax, es:[bx]"
    );
}

#[test]
fn display_jcc_short() {
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0x75, 0xFC],
        kind: InstructionKind::Jcc {
            cond: Cond::Ne,
            target: -4,
        },
        prefixes: Prefixes::default(),
    };
    assert_eq!(
        format_line(&insn),
        "0000:0100  75 FC            jne short -0x4"
    );
}

#[test]
fn display_loop_family() {
    for (bytes, condition, target, expected) in [
        (
            vec![0xE0, 0x05],
            LoopCondition::NotEqual,
            5,
            "0000:0100  E0 05            loopne short 0x5",
        ),
        (
            vec![0xE1, 0xFC],
            LoopCondition::Equal,
            -4,
            "0000:0100  E1 FC            loope short -0x4",
        ),
        (
            vec![0xE2, 0xFE],
            LoopCondition::Always,
            -2,
            "0000:0100  E2 FE            loop short -0x2",
        ),
    ] {
        let insn = Instruction {
            cs_ip: at(0x100),
            bytes,
            kind: InstructionKind::Loop { condition, target },
            prefixes: Prefixes::default(),
        };
        assert_eq!(format_line(&insn), expected);
    }
}

#[test]
fn display_unknown() {
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0xDE, 0xAD],
        kind: InstructionKind::Unknown {
            raw: vec![0xDE, 0xAD],
        },
        prefixes: Prefixes::default(),
    };
    assert_eq!(
        format_line(&insn),
        "0000:0100  DE AD            db 0xDE, 0xAD"
    );
}

#[test]
fn display_jmp_short_positive() {
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0xEB, 0x05],
        kind: InstructionKind::Jmp {
            target: JmpTarget::Rel8(5),
        },
        prefixes: Prefixes::default(),
    };
    assert_eq!(
        format_line(&insn),
        "0000:0100  EB 05            jmp short 0x5"
    );
}

#[test]
fn display_jmp_short_negative() {
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0xEB, 0xFE],
        kind: InstructionKind::Jmp {
            target: JmpTarget::Rel8(-2),
        },
        prefixes: Prefixes::default(),
    };
    assert_eq!(
        format_line(&insn),
        "0000:0100  EB FE            jmp short -0x2"
    );
}

#[test]
fn display_jmp_near_rel16() {
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0xE9, 0x34, 0x12],
        kind: InstructionKind::Jmp {
            target: JmpTarget::Rel16(0x1234),
        },
        prefixes: Prefixes::default(),
    };
    assert_eq!(
        format_line(&insn),
        "0000:0100  E9 34 12         jmp near 0x1234"
    );
}

#[test]
fn display_jmp_far_ptr() {
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0xEA, 0x00, 0x00, 0x00, 0xF0],
        kind: InstructionKind::Jmp {
            target: JmpTarget::Far {
                seg: 0xF000,
                off: 0x0000,
            },
        },
        prefixes: Prefixes::default(),
    };
    assert_eq!(
        format_line(&insn),
        "0000:0100  EA 00 00 00 F0   jmp far 0xF000:0x0"
    );
}

#[test]
fn display_jmp_indirect_near_reg() {
    // 0xFF /4 — JMP near BX (간접): bytes [0xFF, 0xE3] (mod=11, reg=4, rm=3).
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0xFF, 0xE3],
        kind: InstructionKind::Jmp {
            target: JmpTarget::IndirectNear(Operand::Reg16(Reg16::BX)),
        },
        prefixes: Prefixes::default(),
    };
    assert_eq!(format_line(&insn), "0000:0100  FF E3            jmp bx");
}

#[test]
fn display_jmp_far_indirect() {
    // 0xFF /5 — JMP far [bx+si]: bytes [0xFF, 0x28] (mod=00, reg=5, rm=0).
    let mem = Mem {
        seg: None,
        base: MemBase::BxSi,
        disp: 0,
        size: OpSize::Word,
    };
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0xFF, 0x28],
        kind: InstructionKind::Jmp {
            target: JmpTarget::IndirectFar(mem),
        },
        prefixes: Prefixes::default(),
    };
    assert_eq!(
        format_line(&insn),
        "0000:0100  FF 28            jmp far [bx+si]"
    );
}

#[test]
fn display_call_near_rel16() {
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0xE8, 0x10, 0x00],
        kind: InstructionKind::Call {
            target: CallTarget::Rel16(0x10),
        },
        prefixes: Prefixes::default(),
    };
    assert_eq!(
        format_line(&insn),
        "0000:0100  E8 10 00         call near 0x10"
    );
}

#[test]
fn display_call_far_ptr() {
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0x9A, 0x00, 0x10, 0x00, 0x20],
        kind: InstructionKind::Call {
            target: CallTarget::Far {
                seg: 0x2000,
                off: 0x1000,
            },
        },
        prefixes: Prefixes::default(),
    };
    assert_eq!(
        format_line(&insn),
        "0000:0100  9A 00 10 00 20   call far 0x2000:0x1000"
    );
}

#[test]
fn display_call_indirect_near() {
    // 0xFF /2 — CALL near AX: bytes [0xFF, 0xD0] (mod=11, reg=2, rm=0).
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0xFF, 0xD0],
        kind: InstructionKind::Call {
            target: CallTarget::IndirectNear(Operand::Reg16(Reg16::AX)),
        },
        prefixes: Prefixes::default(),
    };
    assert_eq!(format_line(&insn), "0000:0100  FF D0            call ax");
}

#[test]
fn display_rep_movsb() {
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0xF3, 0xA4],
        kind: InstructionKind::Rep(Box::new(InstructionKind::Movsb)),
        prefixes: Prefixes::default(),
    };
    assert_eq!(format_line(&insn), "0000:0100  F3 A4            rep movsb");
}

#[test]
fn display_repne_scasb() {
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0xF2, 0xAE],
        kind: InstructionKind::Repne(Box::new(InstructionKind::Scasb)),
        prefixes: Prefixes::default(),
    };
    assert_eq!(
        format_line(&insn),
        "0000:0100  F2 AE            repne scasb"
    );
}

#[test]
fn display_lea() {
    // LEA BX, [SI+4] → 0x8D /011 (BX) /100 (SI) mod=01 disp=04 → 0x8D 0x5C 0x04
    let mem = Mem {
        seg: None,
        base: MemBase::Si,
        disp: 4,
        size: OpSize::Word,
    };
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0x8D, 0x5C, 0x04],
        kind: InstructionKind::Lea {
            dest: Reg16::BX,
            src: mem,
        },
        prefixes: Prefixes::default(),
    };
    assert_eq!(
        format_line(&insn),
        "0000:0100  8D 5C 04         lea bx, [si+0x4]"
    );
}

#[test]
fn display_xchg_short() {
    // XCHG AX, CX → 0x91
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0x91],
        kind: InstructionKind::Xchg {
            a: Operand::Reg16(Reg16::AX),
            b: Operand::Reg16(Reg16::CX),
        },
        prefixes: Prefixes::default(),
    };
    assert_eq!(
        format_line(&insn),
        "0000:0100  91               xchg ax, cx"
    );
}

#[test]
fn display_alu_add() {
    // ADD AX, BX → 0x01 0xD8 (canonical r/m ← r form)
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0x01, 0xD8],
        kind: InstructionKind::Add {
            dest: Operand::Reg16(Reg16::AX),
            src: Operand::Reg16(Reg16::BX),
        },
        prefixes: Prefixes::default(),
    };
    assert_eq!(format_line(&insn), "0000:0100  01 D8            add ax, bx");
}

#[test]
fn display_shift_one() {
    // SHL AX, 1 → 0xD1 /4 → 0xD1 0xE0
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0xD1, 0xE0],
        kind: InstructionKind::Shl {
            dest: Operand::Reg16(Reg16::AX),
            count: ShiftCount::One,
        },
        prefixes: Prefixes::default(),
    };
    assert_eq!(format_line(&insn), "0000:0100  D1 E0            shl ax, 1");
}

#[test]
fn display_shift_cl() {
    // SHR BL, CL → 0xD2 /5 → 0xD2 0xEB
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0xD2, 0xEB],
        kind: InstructionKind::Shr {
            dest: Operand::Reg8(Reg8::BL),
            count: ShiftCount::Cl,
        },
        prefixes: Prefixes::default(),
    };
    assert_eq!(format_line(&insn), "0000:0100  D2 EB            shr bl, cl");
}

#[test]
fn display_in_al_imm() {
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0xE4, 0x60],
        kind: InstructionKind::InAl {
            port: PortAddr::Imm8(0x60),
        },
        prefixes: Prefixes::default(),
    };
    assert_eq!(
        format_line(&insn),
        "0000:0100  E4 60            in al, 0x60"
    );
}

#[test]
fn display_out_ax_dx() {
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0xEF],
        kind: InstructionKind::OutAx { port: PortAddr::Dx },
        prefixes: Prefixes::default(),
    };
    assert_eq!(format_line(&insn), "0000:0100  EF               out dx, ax");
}

#[test]
fn display_ret_no_pop() {
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0xC3],
        kind: InstructionKind::Ret { pop: 0 },
        prefixes: Prefixes::default(),
    };
    assert_eq!(format_line(&insn), "0000:0100  C3               ret");
}

#[test]
fn display_ret_with_pop() {
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0xC2, 0x04, 0x00],
        kind: InstructionKind::Ret { pop: 4 },
        prefixes: Prefixes::default(),
    };
    assert_eq!(format_line(&insn), "0000:0100  C2 04 00         ret 0x4");
}

#[test]
fn display_mem_disp16_absolute() {
    // [0x1234] — MemBase::Direct
    let mem = Mem {
        seg: None,
        base: MemBase::Direct,
        disp: 0x1234,
        size: OpSize::Word,
    };
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0xA1, 0x34, 0x12],
        kind: InstructionKind::Mov {
            dest: Operand::Reg16(Reg16::AX),
            src: Operand::Mem(mem),
        },
        prefixes: Prefixes::default(),
    };
    assert_eq!(
        format_line(&insn),
        "0000:0100  A1 34 12         mov ax, [0x1234]"
    );
}

#[test]
fn display_mem_disp_negative() {
    // [BP-0x4]
    let mem = Mem {
        seg: None,
        base: MemBase::Bp,
        disp: -4,
        size: OpSize::Word,
    };
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0x8B, 0x46, 0xFC],
        kind: InstructionKind::Mov {
            dest: Operand::Reg16(Reg16::AX),
            src: Operand::Mem(mem),
        },
        prefixes: Prefixes::default(),
    };
    assert_eq!(
        format_line(&insn),
        "0000:0100  8B 46 FC         mov ax, [bp-0x4]"
    );
}

// ----------------------------------------------------------------------------
// 플래그 명령 + sign-extend NASM mnemonic 출력.
// ----------------------------------------------------------------------------

#[test]
fn display_flag_mnemonics() {
    // Intel SDM Vol 2A 부록 B에 따른 NASM 표기.
    let cases: &[(InstructionKind, u8, &str)] = &[
        (InstructionKind::Wait, 0x9B, "wait"),
        (InstructionKind::Pushf, 0x9C, "pushf"),
        (InstructionKind::Popf, 0x9D, "popf"),
        (InstructionKind::Sahf, 0x9E, "sahf"),
        (InstructionKind::Lahf, 0x9F, "lahf"),
        (InstructionKind::Cmc, 0xF5, "cmc"),
        (InstructionKind::Clc, 0xF8, "clc"),
        (InstructionKind::Stc, 0xF9, "stc"),
        (InstructionKind::Cli, 0xFA, "cli"),
        (InstructionKind::Sti, 0xFB, "sti"),
        (InstructionKind::Cld, 0xFC, "cld"),
        (InstructionKind::Std, 0xFD, "std"),
        (InstructionKind::Cbw, 0x98, "cbw"),
        (InstructionKind::Cwd, 0x99, "cwd"),
    ];
    for (kind, opcode, mnemonic) in cases {
        let insn = Instruction {
            cs_ip: at(0x100),
            bytes: vec![*opcode],
            kind: kind.clone(),
            prefixes: Prefixes::default(),
        };
        // 16자리 width로 padding된 byte hex 영역 + 1칸 공백 + mnemonic.
        let expected = format!("0000:0100  {:<16} {}", format!("{:02X}", opcode), mnemonic);
        assert_eq!(format_line(&insn), expected, "{:?} 표기 오류", kind);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// V30 (NEC uPD70116) 0x0F 확장 명령어 — NASM mnemonic 표기 테스트.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn display_v30_add4s() {
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0x0F, 0x20],
        kind: InstructionKind::Add4s,
        prefixes: Prefixes::default(),
    };
    assert_eq!(format_line(&insn), "0000:0100  0F 20            add4s");
}

#[test]
fn display_v30_sub4s() {
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0x0F, 0x22],
        kind: InstructionKind::Sub4s,
        prefixes: Prefixes::default(),
    };
    assert_eq!(format_line(&insn), "0000:0100  0F 22            sub4s");
}

#[test]
fn display_v30_cmp4s() {
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0x0F, 0x26],
        kind: InstructionKind::Cmp4s,
        prefixes: Prefixes::default(),
    };
    assert_eq!(format_line(&insn), "0000:0100  0F 26            cmp4s");
}

#[test]
fn display_v30_rol4_register() {
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0x0F, 0x28, 0xC3],
        kind: InstructionKind::Rol4 {
            rm: Operand::Reg8(Reg8::BL),
        },
        prefixes: Prefixes::default(),
    };
    assert_eq!(format_line(&insn), "0000:0100  0F 28 C3         rol4 bl");
}

#[test]
fn display_v30_ror4_register() {
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0x0F, 0x2A, 0xC0],
        kind: InstructionKind::Ror4 {
            rm: Operand::Reg8(Reg8::AL),
        },
        prefixes: Prefixes::default(),
    };
    assert_eq!(format_line(&insn), "0000:0100  0F 2A C0         ror4 al");
}

#[test]
fn display_v30_ins_register() {
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0x0F, 0x31, 0xD4],
        kind: InstructionKind::Ins {
            bit_offset: Reg8::AH,
            length: Reg8::DL,
        },
        prefixes: Prefixes::default(),
    };
    assert_eq!(format_line(&insn), "0000:0100  0F 31 D4         ins ah, dl");
}

#[test]
fn display_v30_ext_register() {
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0x0F, 0x33, 0xC8],
        kind: InstructionKind::Ext {
            bit_offset: Reg8::AL,
            length: Reg8::CL,
        },
        prefixes: Prefixes::default(),
    };
    assert_eq!(format_line(&insn), "0000:0100  0F 33 C8         ext al, cl");
}

#[test]
fn display_v30_ins_imm() {
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0x0F, 0x39, 0xC3, 0x07],
        kind: InstructionKind::InsImm {
            bit_offset: Reg8::BL,
            length: BitFieldLength::try_from(0x07).unwrap(),
        },
        prefixes: Prefixes::default(),
    };
    assert_eq!(
        format_line(&insn),
        "0000:0100  0F 39 C3 07      ins bl, 0x7"
    );
}

#[test]
fn display_v30_ext_imm() {
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0x0F, 0x3B, 0xC1, 0x0F],
        kind: InstructionKind::ExtImm {
            bit_offset: Reg8::CL,
            length: BitFieldLength::try_from(0x0F).unwrap(),
        },
        prefixes: Prefixes::default(),
    };
    assert_eq!(
        format_line(&insn),
        "0000:0100  0F 3B C1 0F      ext cl, 0xF"
    );
}

#[test]
fn display_v30_brkem() {
    let insn = Instruction {
        cs_ip: at(0x100),
        bytes: vec![0x0F, 0xFF, 0x42],
        kind: InstructionKind::Brkem { vector: 0x42 },
        prefixes: Prefixes::default(),
    };
    assert_eq!(format_line(&insn), "0000:0100  0F FF 42         brkem 0x42");
}
