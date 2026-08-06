use super::instruction::*;

#[test]
fn reg8_constants_have_correct_indices() {
    assert_eq!(Reg8::AL.0, 0);
    assert_eq!(Reg8::CL.0, 1);
    assert_eq!(Reg8::DL.0, 2);
    assert_eq!(Reg8::BL.0, 3);
    assert_eq!(Reg8::AH.0, 4);
    assert_eq!(Reg8::CH.0, 5);
    assert_eq!(Reg8::DH.0, 6);
    assert_eq!(Reg8::BH.0, 7);
}

#[test]
fn reg8_names_are_lowercase() {
    assert_eq!(Reg8::AL.name(), "al");
    assert_eq!(Reg8::AH.name(), "ah");
    assert_eq!(Reg8::BL.name(), "bl");
    assert_eq!(Reg8::BH.name(), "bh");
}

#[test]
fn reg8_invalid_index_returns_unknown() {
    assert_eq!(Reg8(8).name(), "??");
    assert_eq!(Reg8(255).name(), "??");
}

#[test]
fn reg16_constants_have_correct_indices() {
    assert_eq!(Reg16::AX.0, 0);
    assert_eq!(Reg16::CX.0, 1);
    assert_eq!(Reg16::DX.0, 2);
    assert_eq!(Reg16::BX.0, 3);
    assert_eq!(Reg16::SP.0, 4);
    assert_eq!(Reg16::BP.0, 5);
    assert_eq!(Reg16::SI.0, 6);
    assert_eq!(Reg16::DI.0, 7);
}

#[test]
fn reg16_names_are_lowercase() {
    assert_eq!(Reg16::AX.name(), "ax");
    assert_eq!(Reg16::SP.name(), "sp");
    assert_eq!(Reg16::DI.name(), "di");
}

#[test]
fn sreg_names() {
    assert_eq!(Sreg::ES.name(), "es");
    assert_eq!(Sreg::CS.name(), "cs");
    assert_eq!(Sreg::SS.name(), "ss");
    assert_eq!(Sreg::DS.name(), "ds");
}

#[test]
fn membase_constants_distinct() {
    let bases = [
        MemBase::BxSi,
        MemBase::BxDi,
        MemBase::BpSi,
        MemBase::BpDi,
        MemBase::Si,
        MemBase::Di,
        MemBase::Bp,
        MemBase::Bx,
        MemBase::Direct,
    ];
    for (i, a) in bases.iter().enumerate() {
        for (j, b) in bases.iter().enumerate() {
            if i != j {
                assert_ne!(a, b);
            }
        }
    }
}

#[test]
fn mem_struct_holds_all_fields() {
    let m = Mem {
        seg: Some(Sreg::ES),
        base: MemBase::BxSi,
        disp: -4,
        size: OpSize::Word,
    };
    assert_eq!(m.seg, Some(Sreg::ES));
    assert_eq!(m.base, MemBase::BxSi);
    assert_eq!(m.disp, -4);
    assert_eq!(m.size, OpSize::Word);
}

#[test]
fn opsize_two_variants() {
    assert_ne!(OpSize::Byte, OpSize::Word);
}

#[test]
fn sreg_index_round_trip_and_undefined_high() {
    for s in [Sreg::ES, Sreg::CS, Sreg::SS, Sreg::DS] {
        assert_eq!(Sreg::from_index(s.to_index()), Some(s));
    }
    assert_eq!(Sreg::from_index(4), None);
    assert_eq!(Sreg::from_index(7), None);
}

#[test]
fn operand_can_hold_each_variant() {
    let _ = Operand::Reg8(Reg8::AL);
    let _ = Operand::Reg16(Reg16::CX);
    let _ = Operand::Sreg(Sreg::ES);
    let _ = Operand::Imm8(0xAB);
    let _ = Operand::Imm16(0x1234);
    let _ = Operand::Mem(Mem {
        seg: None,
        base: MemBase::Bx,
        disp: 0,
        size: OpSize::Word,
    });
    let _ = Operand::Rel8(-1);
    let _ = Operand::Rel16(0x100);
    let _ = Operand::Far {
        seg: 0xF000,
        off: 0xFFF0,
    };
}

#[test]
fn operand_equality() {
    assert_eq!(Operand::Imm16(0x1234), Operand::Imm16(0x1234));
    assert_ne!(Operand::Imm16(0x1234), Operand::Imm16(0x1235));
    assert_ne!(Operand::Reg8(Reg8::AL), Operand::Reg16(Reg16::AX));
}

#[test]
fn cond_has_seventeen_variants() {
    let all = [
        Cond::O,
        Cond::No,
        Cond::B,
        Cond::Nb,
        Cond::E,
        Cond::Ne,
        Cond::Be,
        Cond::A,
        Cond::S,
        Cond::Ns,
        Cond::P,
        Cond::Np,
        Cond::L,
        Cond::Ge,
        Cond::Le,
        Cond::G,
        Cond::Cxz,
    ];
    assert_eq!(all.len(), 17);
}

#[test]
fn cond_from_jcc_opcode_low_nibble() {
    assert_eq!(Cond::from_jcc_index(0x0), Some(Cond::O));
    assert_eq!(Cond::from_jcc_index(0x1), Some(Cond::No));
    assert_eq!(Cond::from_jcc_index(0x4), Some(Cond::E));
    assert_eq!(Cond::from_jcc_index(0xF), Some(Cond::G));
    assert_eq!(Cond::from_jcc_index(0x10), None);
}

#[test]
fn cond_to_from_jcc_round_trip() {
    let indexed = [
        Cond::O,
        Cond::No,
        Cond::B,
        Cond::Nb,
        Cond::E,
        Cond::Ne,
        Cond::Be,
        Cond::A,
        Cond::S,
        Cond::Ns,
        Cond::P,
        Cond::Np,
        Cond::L,
        Cond::Ge,
        Cond::Le,
        Cond::G,
    ];
    for c in indexed {
        let i = c.to_jcc_index().unwrap();
        assert_eq!(Cond::from_jcc_index(i), Some(c));
    }
    assert_eq!(Cond::Cxz.to_jcc_index(), None);
}

#[test]
fn jmp_target_variants_exist() {
    let _ = JmpTarget::Rel8(-1);
    let _ = JmpTarget::Rel16(0x100);
    let _ = JmpTarget::Far { seg: 0, off: 0 };
    let _ = JmpTarget::IndirectNear(Operand::Reg16(Reg16::BX));
    let _ = JmpTarget::IndirectFar(Mem {
        seg: None,
        base: MemBase::Direct,
        disp: 0x1234,
        size: OpSize::Word,
    });
}

#[test]
fn call_target_variants_exist() {
    let _ = CallTarget::Rel16(0x100);
    let _ = CallTarget::Far { seg: 0, off: 0 };
    let _ = CallTarget::IndirectNear(Operand::Mem(Mem {
        seg: None,
        base: MemBase::Bx,
        disp: 0,
        size: OpSize::Word,
    }));
    let _ = CallTarget::IndirectFar(Mem {
        seg: None,
        base: MemBase::Direct,
        disp: 0x1234,
        size: OpSize::Word,
    });
}

#[test]
fn shift_count_variants() {
    let _ = ShiftCount::One;
    let _ = ShiftCount::Cl;
    let _ = ShiftCount::Imm8(5);
}

#[test]
fn port_addr_variants() {
    let _ = PortAddr::Imm8(0x60);
    let _ = PortAddr::Dx;
}

#[test]
fn segoff_linear_address() {
    let s = SegOff {
        seg: 0x0040,
        off: 0x0080,
    };
    assert_eq!(s.linear(), 0x00480);
    let s = SegOff {
        seg: 0xF000,
        off: 0xFFF0,
    };
    assert_eq!(s.linear(), 0xFFFF0);
}

#[test]
fn prefixes_default() {
    let p = Prefixes::default();
    assert_eq!(p.seg_override, None);
    assert!(!p.bus_lock);
}

#[test]
fn instruction_struct_holds_metadata() {
    let insn = Instruction {
        cs_ip: SegOff { seg: 0, off: 0x100 },
        bytes: vec![0x90],
        kind: InstructionKind::Nop,
        prefixes: Prefixes::default(),
    };
    assert_eq!(insn.bytes, vec![0x90]);
    assert_eq!(insn.kind, InstructionKind::Nop);
    assert_eq!(insn.cs_ip.linear(), 0x100);
}
