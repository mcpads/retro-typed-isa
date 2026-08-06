//! x86 디코더 — bytes → Instruction.
//!
//! ROM 해킹의 도달 가능 코드에서 확인한 8086/V30 명령을 우선 디코드한다. 나머지는
//! `InstructionKind::Unknown { raw }`로 떨어져 상위 분석기가 결손을 fail-loud할 수 있다.

use crate::instruction::*;

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    /// 현재는 빈 입력에서만 발생. 명령어 truncated는 `unknown1` fallback으로 사용 — sweep 연속성 보존.
    #[error("input bytes truncated: needed {needed}, got {got}")]
    Truncated { needed: usize, got: usize },
}

/// ModR/M byte의 mod 필드(bits 7-6).
fn modrm_mod(b: u8) -> u8 {
    (b >> 6) & 0b11
}

/// ModR/M byte의 reg 필드(bits 5-3).
fn modrm_reg(b: u8) -> u8 {
    (b >> 3) & 0b111
}

/// ModR/M byte의 r/m 필드(bits 2-0).
fn modrm_rm(b: u8) -> u8 {
    b & 0b111
}

/// ModR/M의 reg 필드를 16비트 GPR로 해석.
pub fn decode_modrm_reg16(modrm: u8) -> Reg16 {
    Reg16(modrm_reg(modrm))
}

/// ModR/M의 reg 필드를 8비트 GPR로 해석.
pub fn decode_modrm_reg8(modrm: u8) -> Reg8 {
    Reg8(modrm_reg(modrm))
}

/// ModR/M의 r/m 필드를 Operand로 해석. mod 필드에 따라 displacement 바이트를
/// `disp_bytes`에서 소비하고, 소비한 바이트 수를 반환한다.
///
/// `op_size`는 Reg vs Mem의 크기 결정에 사용 (Mem.size).
pub fn decode_modrm_rm(
    modrm: u8,
    op_size: OpSize,
    disp_bytes: &[u8],
) -> Result<(Operand, usize), DecodeError> {
    let m = modrm_mod(modrm);
    let rm = modrm_rm(modrm);

    if m == 0b11 {
        // 레지스터
        let op = match op_size {
            OpSize::Byte => Operand::Reg8(Reg8(rm)),
            OpSize::Word => Operand::Reg16(Reg16(rm)),
        };
        return Ok((op, 0));
    }

    // 메모리 — base 결정
    let (base, special_disp16) = match (m, rm) {
        (0b00, 0b110) => (MemBase::Direct, true), // 특수: [disp16]
        (_, 0b000) => (MemBase::BxSi, false),
        (_, 0b001) => (MemBase::BxDi, false),
        (_, 0b010) => (MemBase::BpSi, false),
        (_, 0b011) => (MemBase::BpDi, false),
        (_, 0b100) => (MemBase::Si, false),
        (_, 0b101) => (MemBase::Di, false),
        (_, 0b110) => (MemBase::Bp, false), // mod≠00이면 BP+disp
        (_, 0b111) => (MemBase::Bx, false),
        _ => unreachable!("rm은 3비트"),
    };

    // displacement 결정
    let (disp, used) = if special_disp16 {
        // mod=00 r/m=110 — disp16 강제
        if disp_bytes.len() < 2 {
            return Err(DecodeError::Truncated {
                needed: 2,
                got: disp_bytes.len(),
            });
        }
        let v = u16::from_le_bytes([disp_bytes[0], disp_bytes[1]]) as i16;
        (v, 2)
    } else {
        match m {
            0b00 => (0i16, 0),
            0b01 => {
                if disp_bytes.is_empty() {
                    return Err(DecodeError::Truncated { needed: 1, got: 0 });
                }
                ((disp_bytes[0] as i8) as i16, 1)
            }
            0b10 => {
                if disp_bytes.len() < 2 {
                    return Err(DecodeError::Truncated {
                        needed: 2,
                        got: disp_bytes.len(),
                    });
                }
                (u16::from_le_bytes([disp_bytes[0], disp_bytes[1]]) as i16, 2)
            }
            _ => unreachable!(),
        }
    };

    Ok((
        Operand::Mem(Mem {
            seg: None,
            base,
            disp,
            size: op_size,
        }),
        used,
    ))
}

/// 단일 명령어 디코드 — bytes의 첫 명령어 하나를 읽는다.
///
/// 진입부에서 segment override, BUSLOCK과 repeat prefix를 source 순서대로 소비한 뒤
/// `decode_core`로 메인 opcode를 디코드한다. repeat은 허용된 block/string 명령을
/// typed wrapper로 감싸고, segment override는 명시적 또는 암시적 source operand에
/// 주입한다. BUSLOCK은 바로 다음 native instruction 전체를 감싼다.
///
/// 알려지지 않은 opcode는 1바이트 `Unknown { raw }`로 떨어진다 (linear sweep 진행 가능).
/// Truncated bytes도 동일하게 처리 (sweep 연속성 보존).
pub fn decode(bytes: &[u8], cs_ip: SegOff) -> Result<Instruction, DecodeError> {
    if bytes.is_empty() {
        return Err(DecodeError::Truncated { needed: 1, got: 0 });
    }

    // V30 instruction 길이 상한 안에서 prefix 순서를 보존한다.
    let mut prefixes = Prefixes::default();
    // 0xF2/0xF3은 REP/REPNE (8086+). 0x64/0x65는 V30 REPNC/REPC — string op에 결합될 때만
    // kind에 wrap. 충돌 시 마지막 prefix가 승리한다. 0x64/0x65도 `rep` 슬롯에 같이
    // 저장해 semantic dispatch를 한 곳에서 처리한다.
    let mut rep: Option<u8> = None;
    let mut consumed = 0;
    while consumed < bytes.len() && consumed < 15 {
        match bytes[consumed] {
            0x26 => prefixes.seg_override = Some(Sreg::ES),
            0x2E => prefixes.seg_override = Some(Sreg::CS),
            0x36 => prefixes.seg_override = Some(Sreg::SS),
            0x3E => prefixes.seg_override = Some(Sreg::DS),
            0xF0 => prefixes.bus_lock = true,
            0xF2 => rep = Some(0xF2),
            0xF3 => rep = Some(0xF3),
            // V30 (uPD70116) — REPNC (0x64) / REPC (0x65). 8086에는 없음. ROM이 V30용으로
            // 빌드된 경우만 출현. string op에 wrap, 그 외에는 prefix 슬롯에만 보존.
            0x64 => {
                prefixes.repnc = true;
                rep = Some(0x64);
            }
            0x65 => {
                prefixes.repc = true;
                rep = Some(0x65);
            }
            _ => break,
        }
        consumed += 1;
    }

    if consumed >= bytes.len() {
        // prefix만 있고 메인 opcode 없음 — 첫 prefix를 1바이트 Unknown으로 떨어뜨려
        // sweep 연속성 보존.
        return Ok(unknown1(cs_ip, bytes[0]));
    }

    // 메인 디코드
    let core_bytes = &bytes[consumed..];
    let core = decode_core(
        core_bytes,
        SegOff {
            seg: cs_ip.seg,
            off: cs_ip.off.wrapping_add(consumed as u16),
        },
    )?;

    // REP는 repeat 가능한 block/string 명령에, REPNE/REPC/REPNC는 비교 계열에만
    // semantic wrapper를 만든다. 공개 strict decode는 의미 없는 최종 repeat을 거부한다.
    let kind = match (rep, &core.kind) {
        (Some(0xF3), kind) if is_string_op(kind) => InstructionKind::Rep(Box::new(kind.clone())),
        (Some(0xF2), kind) if is_comparison_string_op(kind) => {
            InstructionKind::Repne(Box::new(kind.clone()))
        }
        (Some(0x65), kind) if is_comparison_string_op(kind) => {
            InstructionKind::Repc(Box::new(kind.clone()))
        }
        (Some(0x64), kind) if is_comparison_string_op(kind) => {
            InstructionKind::Repnc(Box::new(kind.clone()))
        }
        _ => core.kind.clone(),
    };

    // segment override는 메모리 operand에 주입
    let kind = inject_seg_override(kind, prefixes.seg_override);
    let kind = if prefixes.bus_lock {
        InstructionKind::BusLock(Box::new(kind))
    } else {
        kind
    };

    // 전체 바이트 = consumed prefix + core.bytes
    let mut total_bytes = bytes[..consumed].to_vec();
    total_bytes.extend_from_slice(&core.bytes);

    Ok(Instruction {
        cs_ip,
        bytes: total_bytes,
        kind,
        prefixes,
    })
}

/// 본 함수는 prefix가 모두 제거된 바이트 슬라이스를 받아 단일 명령어를 디코드.
/// 호출자는 `decode`이며, prefix 처리는 거기서 수행. opcode 그룹별 헬퍼들에 디스패치.
fn decode_core(bytes: &[u8], cs_ip: SegOff) -> Result<Instruction, DecodeError> {
    if bytes.is_empty() {
        return Err(DecodeError::Truncated { needed: 1, got: 0 });
    }
    let op0 = bytes[0];

    if let Some(i) = decode_simple_byte(bytes, cs_ip, op0) {
        return Ok(i);
    }
    if let Some(i) = decode_branches(bytes, cs_ip, op0) {
        return Ok(i);
    }
    if let Some(i) = decode_push_pop(bytes, cs_ip, op0) {
        return Ok(i);
    }
    if let Some(i) = decode_mov(bytes, cs_ip, op0) {
        return Ok(i);
    }
    if let Some(i) = decode_alu(bytes, cs_ip, op0) {
        return Ok(i);
    }
    if let Some(i) = decode_test_inc_dec_neg(bytes, cs_ip, op0) {
        return Ok(i);
    }
    if let Some(i) = decode_shift_io_lea_xchg(bytes, cs_ip, op0) {
        return Ok(i);
    }
    if let Some(i) = decode_v30_native_additions(bytes, cs_ip, op0) {
        return Ok(i);
    }

    // V30 (NEC uPD70116) 확장 — `0x0F` secondary opcode.
    // 8086에서는 0x0F가 `POP CS` (POP_CS는 `decode_push_pop`이 처리하지 않음 — 단지 V30이
    // 같은 byte를 secondary prefix로 redefine했을 뿐). 본 디코더는 V30 의미로 처리.
    if op0 == 0x0F {
        return Ok(decode_v30_0f(bytes, cs_ip));
    }

    // 알려지지 않은 opcode는 1바이트 Unknown으로
    Ok(unknown1(cs_ip, op0))
}

/// V30 (NEC uPD70116) `0x0F` secondary opcode 디코드.
///
/// 출처: NEC uPD70116 (V30) 데이터시트 §3 "Instruction Set Extensions".
/// 알 수 없는 secondary는 `Unknown { raw: [0x0F, secondary] }` (2바이트)로 떨어진다 —
/// linear sweep을 8086 path 대비 1바이트 더 진전시켜 V30-only 코드 영역의 mis-alignment 감소.
fn decode_v30_0f(bytes: &[u8], cs_ip: SegOff) -> Instruction {
    if bytes.len() < 2 {
        // 2번째 byte 없음 — 1바이트 Unknown으로
        return unknown1(cs_ip, 0x0F);
    }
    let sec = bytes[1];
    match sec {
        // TEST1/CLR1/SET1/NOT1 r/m8|r/m16, CL|imm
        0x10..=0x1F => {
            if bytes.len() < 3 {
                return unknown_v30(cs_ip, sec);
            }
            let mb = bytes[2];
            if modrm_reg(mb) != 0 {
                return unknown_v30(cs_ip, sec);
            }
            let size = if sec & 1 == 0 {
                OpSize::Byte
            } else {
                OpSize::Word
            };
            let (target, used) = match decode_modrm_rm(mb, size, &bytes[3..]) {
                Ok(value) => value,
                Err(_) => return unknown_v30(cs_ip, sec),
            };
            let immediate = sec >= 0x18;
            let (bit, total) = if immediate {
                let immediate_offset = 3 + used;
                let Some(&value) = bytes.get(immediate_offset) else {
                    return unknown_v30(cs_ip, sec);
                };
                let maximum = if size == OpSize::Byte { 7 } else { 15 };
                if value > maximum {
                    return unknown_v30(cs_ip, sec);
                }
                (BitIndex::Immediate(value), immediate_offset + 1)
            } else {
                (BitIndex::Cl, 3 + used)
            };
            let kind = match sec & 0xF6 {
                0x10 => InstructionKind::Test1 { target, bit },
                0x12 => InstructionKind::Clr1 { target, bit },
                0x14 => InstructionKind::Set1 { target, bit },
                0x16 => InstructionKind::Not1 { target, bit },
                _ => unreachable!("0x10..=0x1F is fully partitioned"),
            };
            insn(cs_ip, bytes[..total].to_vec(), kind)
        }
        // No-operand BCD string ops
        0x20 => insn(cs_ip, vec![0x0F, 0x20], InstructionKind::Add4s),
        0x22 => insn(cs_ip, vec![0x0F, 0x22], InstructionKind::Sub4s),
        0x26 => insn(cs_ip, vec![0x0F, 0x26], InstructionKind::Cmp4s),
        // ROL4/ROR4 r/m8
        0x28 | 0x2A => {
            if bytes.len() < 3 {
                return unknown_v30(cs_ip, sec);
            }
            let mb = bytes[2];
            if modrm_reg(mb) != 0 {
                return unknown_v30(cs_ip, sec);
            }
            let (rm_op, used) = match decode_modrm_rm(mb, OpSize::Byte, &bytes[3..]) {
                Ok(v) => v,
                Err(_) => return unknown_v30(cs_ip, sec),
            };
            let total = 3 + used;
            let kind = if sec == 0x28 {
                InstructionKind::Rol4 { rm: rm_op }
            } else {
                InstructionKind::Ror4 { rm: rm_op }
            };
            insn(cs_ip, bytes[..total].to_vec(), kind)
        }
        // INS / EXT register forms. Both operands are encoded in a register-only ModR/M byte.
        0x31 | 0x33 => {
            if bytes.len() < 3 {
                return unknown_v30(cs_ip, sec);
            }
            let mb = bytes[2];
            if modrm_mod(mb) != 3 {
                return unknown_v30(cs_ip, sec);
            }
            let bit_offset = Reg8::try_from(modrm_rm(mb)).expect("ModR/M r/m is three bits");
            let length = Reg8::try_from(modrm_reg(mb)).expect("ModR/M reg is three bits");
            let kind = if sec == 0x31 {
                InstructionKind::Ins { bit_offset, length }
            } else {
                InstructionKind::Ext { bit_offset, length }
            };
            insn(cs_ip, bytes[..3].to_vec(), kind)
        }
        // INS / EXT immediate forms. The selector is a four-bit field and ModR/M.reg is reserved.
        0x39 | 0x3B => {
            if bytes.len() < 4 {
                return unknown_v30(cs_ip, sec);
            }
            let mb = bytes[2];
            if modrm_mod(mb) != 3 || modrm_reg(mb) != 0 {
                return unknown_v30(cs_ip, sec);
            }
            let bit_offset = Reg8::try_from(modrm_rm(mb)).expect("ModR/M r/m is three bits");
            let Ok(length) = BitFieldLength::try_from(bytes[3]) else {
                return unknown_v30(cs_ip, sec);
            };
            let kind = if sec == 0x39 {
                InstructionKind::InsImm { bit_offset, length }
            } else {
                InstructionKind::ExtImm { bit_offset, length }
            };
            insn(cs_ip, bytes[..4].to_vec(), kind)
        }
        // BRKEM imm8 — native stream의 정적 분석 경계
        0xFF => {
            if bytes.len() < 3 {
                return unknown_v30(cs_ip, sec);
            }
            let vector = bytes[2];
            insn(
                cs_ip,
                vec![0x0F, 0xFF, vector],
                InstructionKind::Brkem { vector },
            )
        }
        _ => unknown_v30(cs_ip, sec),
    }
}

/// V30 secondary opcode 인식 실패 시 2바이트(`0x0F`+secondary)를 `Unknown`으로.
fn unknown_v30(cs_ip: SegOff, sec: u8) -> Instruction {
    Instruction {
        cs_ip,
        bytes: vec![0x0F, sec],
        kind: InstructionKind::Unknown {
            raw: vec![0x0F, sec],
        },
        prefixes: Prefixes::default(),
    }
}

/// REP/REPNE prefix가 의미를 갖는 string 명령어 판별.
fn is_string_op(kind: &InstructionKind) -> bool {
    matches!(
        kind,
        InstructionKind::Movsb
            | InstructionKind::Movsw
            | InstructionKind::Cmpsb
            | InstructionKind::Cmpsw
            | InstructionKind::Stosb
            | InstructionKind::Stosw
            | InstructionKind::Lodsb
            | InstructionKind::Lodsw
            | InstructionKind::Scasb
            | InstructionKind::Scasw
            | InstructionKind::Insb
            | InstructionKind::Insw
            | InstructionKind::Outsb
            | InstructionKind::Outsw
    )
}

fn is_comparison_string_op(kind: &InstructionKind) -> bool {
    matches!(
        kind,
        InstructionKind::Cmpsb
            | InstructionKind::Cmpsw
            | InstructionKind::Scasb
            | InstructionKind::Scasw
    )
}

/// segment override prefix를 명령어의 메모리 operand에 주입.
/// 메모리 operand가 없는 명령어는 그대로 반환.
///
/// **신규 메모리-바운드 InstructionKind variant 추가 시 세 곳 모두 갱신 필요**:
/// `decode.rs::inject_seg_override`, `emit.rs::find_mem_seg`, `emit.rs::strip_mem_seg`.
fn inject_seg_override(kind: InstructionKind, seg: Option<Sreg>) -> InstructionKind {
    let s = match seg {
        Some(s) => s,
        None => return kind,
    };
    let with_seg = |op: Operand| -> Operand {
        match op {
            Operand::Mem(mut m) => {
                m.seg = Some(s);
                Operand::Mem(m)
            }
            other => other,
        }
    };
    if has_implicit_segment_source(&kind) {
        return InstructionKind::SegmentOverride {
            segment: s,
            instruction: Box::new(kind),
        };
    }
    match kind {
        InstructionKind::BusLock(instruction) => {
            InstructionKind::BusLock(Box::new(inject_seg_override(*instruction, Some(s))))
        }
        InstructionKind::Mov { dest, src } => InstructionKind::Mov {
            dest: with_seg(dest),
            src: with_seg(src),
        },
        InstructionKind::Lea { dest, mut src } => {
            src.seg = Some(s);
            InstructionKind::Lea { dest, src }
        }
        InstructionKind::Les { dest, mut src } => {
            src.seg = Some(s);
            InstructionKind::Les { dest, src }
        }
        InstructionKind::Lds { dest, mut src } => {
            src.seg = Some(s);
            InstructionKind::Lds { dest, src }
        }
        InstructionKind::Add { dest, src } => InstructionKind::Add {
            dest: with_seg(dest),
            src: with_seg(src),
        },
        InstructionKind::Adc { dest, src } => InstructionKind::Adc {
            dest: with_seg(dest),
            src: with_seg(src),
        },
        InstructionKind::Sub { dest, src } => InstructionKind::Sub {
            dest: with_seg(dest),
            src: with_seg(src),
        },
        InstructionKind::Sbb { dest, src } => InstructionKind::Sbb {
            dest: with_seg(dest),
            src: with_seg(src),
        },
        InstructionKind::Cmp { a, b } => InstructionKind::Cmp {
            a: with_seg(a),
            b: with_seg(b),
        },
        InstructionKind::Test { a, b } => InstructionKind::Test {
            a: with_seg(a),
            b: with_seg(b),
        },
        InstructionKind::And { dest, src } => InstructionKind::And {
            dest: with_seg(dest),
            src: with_seg(src),
        },
        InstructionKind::Or { dest, src } => InstructionKind::Or {
            dest: with_seg(dest),
            src: with_seg(src),
        },
        InstructionKind::Xor { dest, src } => InstructionKind::Xor {
            dest: with_seg(dest),
            src: with_seg(src),
        },
        InstructionKind::Push { src } => InstructionKind::Push { src: with_seg(src) },
        InstructionKind::Pop { dest } => InstructionKind::Pop {
            dest: with_seg(dest),
        },
        InstructionKind::Inc { dest } => InstructionKind::Inc {
            dest: with_seg(dest),
        },
        InstructionKind::Dec { dest } => InstructionKind::Dec {
            dest: with_seg(dest),
        },
        InstructionKind::Neg { dest } => InstructionKind::Neg {
            dest: with_seg(dest),
        },
        InstructionKind::Not { dest } => InstructionKind::Not {
            dest: with_seg(dest),
        },
        InstructionKind::Mul { src } => InstructionKind::Mul { src: with_seg(src) },
        InstructionKind::Imul { src } => InstructionKind::Imul { src: with_seg(src) },
        InstructionKind::Div { src } => InstructionKind::Div { src: with_seg(src) },
        InstructionKind::Idiv { src } => InstructionKind::Idiv { src: with_seg(src) },
        InstructionKind::ImulImmediate {
            dest,
            src,
            immediate,
        } => InstructionKind::ImulImmediate {
            dest,
            src: with_seg(src),
            immediate,
        },
        InstructionKind::Bound { index, mut bounds } => {
            bounds.seg = Some(s);
            InstructionKind::Bound { index, bounds }
        }
        InstructionKind::Xchg { a, b } => InstructionKind::Xchg {
            a: with_seg(a),
            b: with_seg(b),
        },
        // 시프트/로테이트도 메모리 operand 가능
        InstructionKind::Rol { dest, count } => InstructionKind::Rol {
            dest: with_seg(dest),
            count,
        },
        InstructionKind::Ror { dest, count } => InstructionKind::Ror {
            dest: with_seg(dest),
            count,
        },
        InstructionKind::Rcl { dest, count } => InstructionKind::Rcl {
            dest: with_seg(dest),
            count,
        },
        InstructionKind::Rcr { dest, count } => InstructionKind::Rcr {
            dest: with_seg(dest),
            count,
        },
        InstructionKind::Shl { dest, count } => InstructionKind::Shl {
            dest: with_seg(dest),
            count,
        },
        InstructionKind::Shr { dest, count } => InstructionKind::Shr {
            dest: with_seg(dest),
            count,
        },
        InstructionKind::Sar { dest, count } => InstructionKind::Sar {
            dest: with_seg(dest),
            count,
        },
        // V30 0x0F 확장 — 메모리 operand를 받는 variant들
        InstructionKind::Rol4 { rm } => InstructionKind::Rol4 { rm: with_seg(rm) },
        InstructionKind::Ror4 { rm } => InstructionKind::Ror4 { rm: with_seg(rm) },
        InstructionKind::Test1 { target, bit } => InstructionKind::Test1 {
            target: with_seg(target),
            bit,
        },
        InstructionKind::Clr1 { target, bit } => InstructionKind::Clr1 {
            target: with_seg(target),
            bit,
        },
        InstructionKind::Set1 { target, bit } => InstructionKind::Set1 {
            target: with_seg(target),
            bit,
        },
        InstructionKind::Not1 { target, bit } => InstructionKind::Not1 {
            target: with_seg(target),
            bit,
        },
        InstructionKind::Esc { operation, operand } => InstructionKind::Esc {
            operation,
            operand: match operand {
                FloatingPointOperand::Memory(mut memory) => {
                    memory.seg = Some(s);
                    FloatingPointOperand::Memory(memory)
                }
                operand => operand,
            },
        },
        InstructionKind::Fpo2 { operation, operand } => InstructionKind::Fpo2 {
            operation,
            operand: match operand {
                FloatingPointOperand::Memory(mut memory) => {
                    memory.seg = Some(s);
                    FloatingPointOperand::Memory(memory)
                }
                operand => operand,
            },
        },
        InstructionKind::Jmp {
            target: JmpTarget::IndirectNear(operand),
        } => InstructionKind::Jmp {
            target: JmpTarget::IndirectNear(with_seg(operand)),
        },
        InstructionKind::Jmp {
            target: JmpTarget::IndirectFar(mut memory),
        } => {
            memory.seg = Some(s);
            InstructionKind::Jmp {
                target: JmpTarget::IndirectFar(memory),
            }
        }
        InstructionKind::Call {
            target: CallTarget::IndirectNear(operand),
        } => InstructionKind::Call {
            target: CallTarget::IndirectNear(with_seg(operand)),
        },
        InstructionKind::Call {
            target: CallTarget::IndirectFar(mut memory),
        } => {
            memory.seg = Some(s);
            InstructionKind::Call {
                target: CallTarget::IndirectFar(memory),
            }
        }
        other => other,
    }
}

fn has_implicit_segment_source(kind: &InstructionKind) -> bool {
    match kind {
        InstructionKind::Movsb
        | InstructionKind::Movsw
        | InstructionKind::Cmpsb
        | InstructionKind::Cmpsw
        | InstructionKind::Lodsb
        | InstructionKind::Lodsw
        | InstructionKind::Xlat
        | InstructionKind::Outsb
        | InstructionKind::Outsw
        | InstructionKind::Add4s
        | InstructionKind::Sub4s
        | InstructionKind::Cmp4s => true,
        InstructionKind::Rep(inner)
        | InstructionKind::Repne(inner)
        | InstructionKind::Repc(inner)
        | InstructionKind::Repnc(inner) => has_implicit_segment_source(inner),
        _ => false,
    }
}

/// 단일 바이트 명령어 그룹: NOP, HLT, IRET, INT3, INT n, 플래그 명령 (Intel SDM Vol 2A
/// 부록 B), sign-extend (CBW/CWD), 문자열 명령
/// (MOVSB/MOVSW/CMPSB/CMPSW/STOSB/STOSW/LODSB/LODSW/SCASB/SCASW).
fn decode_simple_byte(bytes: &[u8], cs_ip: SegOff, op0: u8) -> Option<Instruction> {
    match op0 {
        0x90 => Some(insn(cs_ip, vec![0x90], InstructionKind::Nop)),
        0xF4 => Some(insn(cs_ip, vec![0xF4], InstructionKind::Hlt)),
        0xD7 => Some(insn(cs_ip, vec![0xD7], InstructionKind::Xlat)),
        0xCF => Some(insn(cs_ip, vec![0xCF], InstructionKind::Iret)),
        0xCC => Some(insn(cs_ip, vec![0xCC], InstructionKind::Int { vector: 3 })),
        0xCD => {
            if bytes.len() < 2 {
                return Some(unknown1(cs_ip, op0));
            }
            Some(insn(
                cs_ip,
                vec![0xCD, bytes[1]],
                InstructionKind::Int { vector: bytes[1] },
            ))
        }
        0xCE => Some(insn(cs_ip, vec![0xCE], InstructionKind::Into)),
        0x27 => Some(insn(cs_ip, vec![0x27], InstructionKind::Daa)),
        0x2F => Some(insn(cs_ip, vec![0x2F], InstructionKind::Das)),
        0x37 => Some(insn(cs_ip, vec![0x37], InstructionKind::Aaa)),
        0x3F => Some(insn(cs_ip, vec![0x3F], InstructionKind::Aas)),
        0xD4 | 0xD5 => {
            if bytes.len() < 2 {
                return Some(unknown1(cs_ip, op0));
            }
            let kind = if op0 == 0xD4 {
                InstructionKind::Aam { base: bytes[1] }
            } else {
                InstructionKind::Aad { base: bytes[1] }
            };
            Some(insn(cs_ip, bytes[..2].to_vec(), kind))
        }
        // sign-extend (Intel SDM Vol 2A B.1.4.5).
        0x98 => Some(insn(cs_ip, vec![0x98], InstructionKind::Cbw)),
        0x99 => Some(insn(cs_ip, vec![0x99], InstructionKind::Cwd)),
        // 플래그 명령 (Intel SDM Vol 2A B.1.4.4 — `*F` family).
        0x9B => Some(insn(cs_ip, vec![0x9B], InstructionKind::Wait)),
        0x9C => Some(insn(cs_ip, vec![0x9C], InstructionKind::Pushf)),
        0x9D => Some(insn(cs_ip, vec![0x9D], InstructionKind::Popf)),
        0x9E => Some(insn(cs_ip, vec![0x9E], InstructionKind::Sahf)),
        0x9F => Some(insn(cs_ip, vec![0x9F], InstructionKind::Lahf)),
        0xF5 => Some(insn(cs_ip, vec![0xF5], InstructionKind::Cmc)),
        0xF8 => Some(insn(cs_ip, vec![0xF8], InstructionKind::Clc)),
        0xF9 => Some(insn(cs_ip, vec![0xF9], InstructionKind::Stc)),
        0xFA => Some(insn(cs_ip, vec![0xFA], InstructionKind::Cli)),
        0xFB => Some(insn(cs_ip, vec![0xFB], InstructionKind::Sti)),
        0xFC => Some(insn(cs_ip, vec![0xFC], InstructionKind::Cld)),
        0xFD => Some(insn(cs_ip, vec![0xFD], InstructionKind::Std)),
        // 문자열 명령 (단일 바이트). REP/REPNE prefix는 `decode` 진입부에서 처리.
        0xA4 => Some(insn(cs_ip, vec![0xA4], InstructionKind::Movsb)),
        0xA5 => Some(insn(cs_ip, vec![0xA5], InstructionKind::Movsw)),
        0xA6 => Some(insn(cs_ip, vec![0xA6], InstructionKind::Cmpsb)),
        0xA7 => Some(insn(cs_ip, vec![0xA7], InstructionKind::Cmpsw)),
        0xAA => Some(insn(cs_ip, vec![0xAA], InstructionKind::Stosb)),
        0xAB => Some(insn(cs_ip, vec![0xAB], InstructionKind::Stosw)),
        0xAC => Some(insn(cs_ip, vec![0xAC], InstructionKind::Lodsb)),
        0xAD => Some(insn(cs_ip, vec![0xAD], InstructionKind::Lodsw)),
        0xAE => Some(insn(cs_ip, vec![0xAE], InstructionKind::Scasb)),
        0xAF => Some(insn(cs_ip, vec![0xAF], InstructionKind::Scasw)),
        _ => None,
    }
}

/// 80186-compatible V30 forms와 native external-FPU selector.
fn decode_v30_native_additions(bytes: &[u8], cs_ip: SegOff, op0: u8) -> Option<Instruction> {
    let single = match op0 {
        0x60 => Some(InstructionKind::Pusha),
        0x61 => Some(InstructionKind::Popa),
        0x6C => Some(InstructionKind::Insb),
        0x6D => Some(InstructionKind::Insw),
        0x6E => Some(InstructionKind::Outsb),
        0x6F => Some(InstructionKind::Outsw),
        0xC9 => Some(InstructionKind::Leave),
        _ => None,
    };
    if let Some(kind) = single {
        return Some(insn(cs_ip, vec![op0], kind));
    }

    if op0 == 0x68 || op0 == 0x6A {
        let (src, len) = if op0 == 0x68 {
            if bytes.len() < 3 {
                return Some(unknown1(cs_ip, op0));
            }
            (Operand::Imm16(u16::from_le_bytes([bytes[1], bytes[2]])), 3)
        } else {
            if bytes.len() < 2 {
                return Some(unknown1(cs_ip, op0));
            }
            (Operand::Imm8(bytes[1]), 2)
        };
        return Some(insn(
            cs_ip,
            bytes[..len].to_vec(),
            InstructionKind::Push { src },
        ));
    }

    if op0 == 0x62 {
        if bytes.len() < 2 || modrm_mod(bytes[1]) == 0b11 {
            return Some(unknown1(cs_ip, op0));
        }
        let modrm = bytes[1];
        let (bounds, used) = match decode_modrm_rm(modrm, OpSize::Word, &bytes[2..]) {
            Ok((Operand::Mem(bounds), used)) => (bounds, used),
            _ => return Some(unknown1(cs_ip, op0)),
        };
        return Some(insn(
            cs_ip,
            bytes[..2 + used].to_vec(),
            InstructionKind::Bound {
                index: decode_modrm_reg16(modrm),
                bounds,
            },
        ));
    }

    if op0 == 0x69 || op0 == 0x6B {
        if bytes.len() < 2 {
            return Some(unknown1(cs_ip, op0));
        }
        let modrm = bytes[1];
        let (src, used) = match decode_modrm_rm(modrm, OpSize::Word, &bytes[2..]) {
            Ok(value) => value,
            Err(_) => return Some(unknown1(cs_ip, op0)),
        };
        let immediate_offset = 2 + used;
        let (immediate, immediate_len) = if op0 == 0x69 {
            if bytes.len() < immediate_offset + 2 {
                return Some(unknown1(cs_ip, op0));
            }
            (
                SignedImmediate::Word(i16::from_le_bytes([
                    bytes[immediate_offset],
                    bytes[immediate_offset + 1],
                ])),
                2,
            )
        } else {
            if bytes.len() < immediate_offset + 1 {
                return Some(unknown1(cs_ip, op0));
            }
            (SignedImmediate::Byte(bytes[immediate_offset] as i8), 1)
        };
        let len = immediate_offset + immediate_len;
        return Some(insn(
            cs_ip,
            bytes[..len].to_vec(),
            InstructionKind::ImulImmediate {
                dest: decode_modrm_reg16(modrm),
                src,
                immediate,
            },
        ));
    }

    if op0 == 0xC0 || op0 == 0xC1 {
        if bytes.len() < 2 {
            return Some(unknown1(cs_ip, op0));
        }
        let modrm = bytes[1];
        if modrm_reg(modrm) == 6 {
            return Some(unknown1(cs_ip, op0));
        }
        let size = if op0 == 0xC0 {
            OpSize::Byte
        } else {
            OpSize::Word
        };
        let (dest, used) = match decode_modrm_rm(modrm, size, &bytes[2..]) {
            Ok(value) => value,
            Err(_) => return Some(unknown1(cs_ip, op0)),
        };
        let immediate_offset = 2 + used;
        if bytes.len() <= immediate_offset {
            return Some(unknown1(cs_ip, op0));
        }
        let count = ShiftCount::Imm8(bytes[immediate_offset]);
        let kind = match modrm_reg(modrm) {
            0 => InstructionKind::Rol { dest, count },
            1 => InstructionKind::Ror { dest, count },
            2 => InstructionKind::Rcl { dest, count },
            3 => InstructionKind::Rcr { dest, count },
            4 => InstructionKind::Shl { dest, count },
            5 => InstructionKind::Shr { dest, count },
            7 => InstructionKind::Sar { dest, count },
            _ => unreachable!(),
        };
        return Some(insn(cs_ip, bytes[..=immediate_offset].to_vec(), kind));
    }

    if op0 == 0xC8 {
        if bytes.len() < 4 {
            return Some(unknown1(cs_ip, op0));
        }
        return Some(insn(
            cs_ip,
            bytes[..4].to_vec(),
            InstructionKind::Enter {
                frame_size: u16::from_le_bytes([bytes[1], bytes[2]]),
                nesting_level: bytes[3],
            },
        ));
    }

    if (0xD8..=0xDF).contains(&op0) {
        if bytes.len() < 2 {
            return Some(unknown1(cs_ip, op0));
        }
        let modrm = bytes[1];
        let operation = EscapeOpcode::try_from(((op0 - 0xD8) << 3) | modrm_reg(modrm))
            .expect("six-bit ESC operation");
        let (operand, used) = if modrm_mod(modrm) == 0b11 {
            (
                FloatingPointOperand::Register(
                    ExternalRegister::try_from(modrm_rm(modrm)).expect("three-bit selector"),
                ),
                0,
            )
        } else {
            match decode_modrm_rm(modrm, OpSize::Word, &bytes[2..]) {
                Ok((Operand::Mem(memory), used)) => (FloatingPointOperand::Memory(memory), used),
                _ => return Some(unknown1(cs_ip, op0)),
            }
        };
        return Some(insn(
            cs_ip,
            bytes[..2 + used].to_vec(),
            InstructionKind::Esc { operation, operand },
        ));
    }

    if op0 == 0x66 || op0 == 0x67 {
        if bytes.len() < 2 {
            return Some(unknown1(cs_ip, op0));
        }
        let modrm = bytes[1];
        let operation = Fpo2Opcode::try_from(((op0 - 0x66) << 3) | modrm_reg(modrm))
            .expect("four-bit FPO2 operation");
        let (operand, used) = if modrm_mod(modrm) == 0b11 {
            (
                FloatingPointOperand::Register(
                    ExternalRegister::try_from(modrm_rm(modrm)).expect("three-bit selector"),
                ),
                0,
            )
        } else {
            match decode_modrm_rm(modrm, OpSize::Word, &bytes[2..]) {
                Ok((Operand::Mem(memory), used)) => (FloatingPointOperand::Memory(memory), used),
                _ => return Some(unknown1(cs_ip, op0)),
            }
        };
        return Some(insn(
            cs_ip,
            bytes[..2 + used].to_vec(),
            InstructionKind::Fpo2 { operation, operand },
        ));
    }

    None
}

/// 분기 그룹: Jcc rel8, LOOP family, JCXZ, JMP short/near/far, CALL near/far, RET, RETF.
fn decode_branches(bytes: &[u8], cs_ip: SegOff, op0: u8) -> Option<Instruction> {
    // Jcc rel8 (0x70-0x7F)
    if (0x70..=0x7F).contains(&op0) {
        if bytes.len() < 2 {
            return Some(unknown1(cs_ip, op0));
        }
        let cond = Cond::from_jcc_index(op0 - 0x70).unwrap();
        let target = bytes[1] as i8;
        return Some(insn(
            cs_ip,
            vec![op0, bytes[1]],
            InstructionKind::Jcc { cond, target },
        ));
    }
    // LOOPNE/LOOPE/LOOP (0xE0-0xE2) — CX 감소를 수반하는 조건 분기.
    if (0xE0..=0xE2).contains(&op0) {
        if bytes.len() < 2 {
            return Some(unknown1(cs_ip, op0));
        }
        let condition = match op0 {
            0xE0 => LoopCondition::NotEqual,
            0xE1 => LoopCondition::Equal,
            0xE2 => LoopCondition::Always,
            _ => unreachable!(),
        };
        return Some(insn(
            cs_ip,
            vec![op0, bytes[1]],
            InstructionKind::Loop {
                condition,
                target: bytes[1] as i8,
            },
        ));
    }
    // JCXZ (0xE3) — Jcc::Cxz
    if op0 == 0xE3 {
        if bytes.len() < 2 {
            return Some(unknown1(cs_ip, op0));
        }
        let target = bytes[1] as i8;
        return Some(insn(
            cs_ip,
            vec![0xE3, bytes[1]],
            InstructionKind::Jcc {
                cond: Cond::Cxz,
                target,
            },
        ));
    }

    match op0 {
        0xEB => {
            if bytes.len() < 2 {
                return Some(unknown1(cs_ip, op0));
            }
            Some(insn(
                cs_ip,
                vec![op0, bytes[1]],
                InstructionKind::Jmp {
                    target: JmpTarget::Rel8(bytes[1] as i8),
                },
            ))
        }
        0xE9 => {
            if bytes.len() < 3 {
                return Some(unknown1(cs_ip, op0));
            }
            let rel = i16::from_le_bytes([bytes[1], bytes[2]]);
            Some(insn(
                cs_ip,
                vec![op0, bytes[1], bytes[2]],
                InstructionKind::Jmp {
                    target: JmpTarget::Rel16(rel),
                },
            ))
        }
        0xEA => {
            if bytes.len() < 5 {
                return Some(unknown1(cs_ip, op0));
            }
            let off = u16::from_le_bytes([bytes[1], bytes[2]]);
            let seg = u16::from_le_bytes([bytes[3], bytes[4]]);
            Some(insn(
                cs_ip,
                bytes[..5].to_vec(),
                InstructionKind::Jmp {
                    target: JmpTarget::Far { seg, off },
                },
            ))
        }
        0xE8 => {
            if bytes.len() < 3 {
                return Some(unknown1(cs_ip, op0));
            }
            let rel = i16::from_le_bytes([bytes[1], bytes[2]]);
            Some(insn(
                cs_ip,
                vec![op0, bytes[1], bytes[2]],
                InstructionKind::Call {
                    target: CallTarget::Rel16(rel),
                },
            ))
        }
        0x9A => {
            if bytes.len() < 5 {
                return Some(unknown1(cs_ip, op0));
            }
            let off = u16::from_le_bytes([bytes[1], bytes[2]]);
            let seg = u16::from_le_bytes([bytes[3], bytes[4]]);
            Some(insn(
                cs_ip,
                bytes[..5].to_vec(),
                InstructionKind::Call {
                    target: CallTarget::Far { seg, off },
                },
            ))
        }
        0xC3 => Some(insn(cs_ip, vec![0xC3], InstructionKind::Ret { pop: 0 })),
        0xC2 => {
            if bytes.len() < 3 {
                return Some(unknown1(cs_ip, op0));
            }
            let pop = u16::from_le_bytes([bytes[1], bytes[2]]);
            Some(insn(
                cs_ip,
                vec![op0, bytes[1], bytes[2]],
                InstructionKind::Ret { pop },
            ))
        }
        0xCB => Some(insn(cs_ip, vec![0xCB], InstructionKind::Retf { pop: 0 })),
        0xCA => {
            if bytes.len() < 3 {
                return Some(unknown1(cs_ip, op0));
            }
            let pop = u16::from_le_bytes([bytes[1], bytes[2]]);
            Some(insn(
                cs_ip,
                vec![op0, bytes[1], bytes[2]],
                InstructionKind::Retf { pop },
            ))
        }
        _ => None,
    }
}

/// PUSH/POP 그룹: r16 단축형, sreg 단축형, 0xFF /6 (PUSH r/m16), 0x8F /0 (POP r/m16).
fn decode_push_pop(bytes: &[u8], cs_ip: SegOff, op0: u8) -> Option<Instruction> {
    // PUSH r16 (0x50-0x57) / POP r16 (0x58-0x5F)
    if (0x50..=0x57).contains(&op0) {
        return Some(insn(
            cs_ip,
            vec![op0],
            InstructionKind::Push {
                src: Operand::Reg16(Reg16(op0 - 0x50)),
            },
        ));
    }
    if (0x58..=0x5F).contains(&op0) {
        return Some(insn(
            cs_ip,
            vec![op0],
            InstructionKind::Pop {
                dest: Operand::Reg16(Reg16(op0 - 0x58)),
            },
        ));
    }

    // PUSH ES/CS/SS/DS / POP ES/SS/DS
    match op0 {
        0x06 => {
            return Some(insn(
                cs_ip,
                vec![op0],
                InstructionKind::Push {
                    src: Operand::Sreg(Sreg::ES),
                },
            ));
        }
        0x0E => {
            return Some(insn(
                cs_ip,
                vec![op0],
                InstructionKind::Push {
                    src: Operand::Sreg(Sreg::CS),
                },
            ));
        }
        0x16 => {
            return Some(insn(
                cs_ip,
                vec![op0],
                InstructionKind::Push {
                    src: Operand::Sreg(Sreg::SS),
                },
            ));
        }
        0x1E => {
            return Some(insn(
                cs_ip,
                vec![op0],
                InstructionKind::Push {
                    src: Operand::Sreg(Sreg::DS),
                },
            ));
        }
        0x07 => {
            return Some(insn(
                cs_ip,
                vec![op0],
                InstructionKind::Pop {
                    dest: Operand::Sreg(Sreg::ES),
                },
            ));
        }
        0x17 => {
            return Some(insn(
                cs_ip,
                vec![op0],
                InstructionKind::Pop {
                    dest: Operand::Sreg(Sreg::SS),
                },
            ));
        }
        0x1F => {
            return Some(insn(
                cs_ip,
                vec![op0],
                InstructionKind::Pop {
                    dest: Operand::Sreg(Sreg::DS),
                },
            ));
        }
        _ => {}
    }

    // PUSH r/m16 (0xFF /6) — Phase D-3부터 Group 5 통합 헬퍼 `decode_group_5`에서 처리.
    // (이전엔 본 헬퍼에서 reg=6만 매치했으나 INC/DEC/CALL/JMP indirect와 함께 0xFF
    // 분기가 한 곳으로 집중되도록 이동.)

    // POP r/m16 (0x8F /0)
    // Canonical 정책 (emit.rs 모듈 doc 참조): reg form (mod=11)이 들어오면
    // `InstructionKind::Pop { dest: Reg16 }`로 디코드 — emit는 항상 0x58+r 단축형으로.
    // 즉 `0x8F 0xC3` → decode → emit → `0x5B` (byte 변경, kind 보존). 메모리 form (mod≠11)은
    // 0x8F /0으로 emit.
    if op0 == 0x8F {
        if bytes.len() < 2 {
            return Some(unknown1(cs_ip, op0));
        }
        let mb = bytes[1];
        if modrm_reg(mb) != 0 {
            return Some(unknown1(cs_ip, op0));
        }
        return Some(match decode_modrm_rm(mb, OpSize::Word, &bytes[2..]) {
            Ok((op2, used2)) => {
                let total = 2 + used2;
                insn(
                    cs_ip,
                    bytes[..total].to_vec(),
                    InstructionKind::Pop { dest: op2 },
                )
            }
            Err(_) => unknown1(cs_ip, op0),
        });
    }

    None
}

/// MOV 그룹: r/m ↔ r (0x88-0x8B), r/m ↔ sreg (0x8C/0x8E), acc ↔ moffs (0xA0-0xA3),
/// r ← imm 단축형 (0xB0-0xBF), r/m ← imm (0xC6/0xC7 /0).
fn decode_mov(bytes: &[u8], cs_ip: SegOff, op0: u8) -> Option<Instruction> {
    // MOV r/m, r (0x88, 0x89) — direction = 0 (r/m ← r)
    // MOV r, r/m (0x8A, 0x8B) — direction = 1 (r ← r/m)
    //
    // Canonical 정책 (emit.rs 모듈 doc 참조): reg-reg (mod=11) 케이스에서 0x88/0x89와
    // 0x8A/0x8B는 같은 의미의 redundant encoding. 본 디코더는 의미 그대로 디코드하므로
    // 두 인코딩이 같은 `Mov { dest, src }` kind를 만든다 — 예: `0x89 0xC4`와 `0x8B 0xE0`은
    // 모두 `Mov { dest: SP, src: AX }`로 디코드. emit는 항상 0x88/0x89 (dir=0)을 선택해
    // decode → emit → decode → emit 안정점에 도달.
    if matches!(op0, 0x88..=0x8B) {
        if bytes.len() < 2 {
            return Some(unknown1(cs_ip, op0));
        }
        let mb = bytes[1];
        let w = (op0 & 1) == 1;
        let dir = (op0 & 2) == 2;
        let size = if w { OpSize::Word } else { OpSize::Byte };
        let (rm_op, used) = match decode_modrm_rm(mb, size, &bytes[2..]) {
            Ok(v) => v,
            Err(_) => return Some(unknown1(cs_ip, op0)),
        };
        let reg_op = if w {
            Operand::Reg16(decode_modrm_reg16(mb))
        } else {
            Operand::Reg8(decode_modrm_reg8(mb))
        };
        let total = 2 + used;
        let kind = if dir {
            InstructionKind::Mov {
                dest: reg_op,
                src: rm_op,
            }
        } else {
            InstructionKind::Mov {
                dest: rm_op,
                src: reg_op,
            }
        };
        return Some(insn(cs_ip, bytes[..total].to_vec(), kind));
    }

    // MOV r/m16, sreg (0x8C) / MOV sreg, r/m16 (0x8E)
    if matches!(op0, 0x8C | 0x8E) {
        if bytes.len() < 2 {
            return Some(unknown1(cs_ip, op0));
        }
        let mb = bytes[1];
        let sreg_idx = modrm_reg(mb);
        let sreg = match Sreg::from_index(sreg_idx) {
            Some(s) => s,
            None => return Some(unknown1(cs_ip, op0)),
        };
        if op0 == 0x8E && sreg == Sreg::CS {
            return Some(unknown1(cs_ip, op0));
        }
        let (rm_op, used) = match decode_modrm_rm(mb, OpSize::Word, &bytes[2..]) {
            Ok(v) => v,
            Err(_) => return Some(unknown1(cs_ip, op0)),
        };
        let total = 2 + used;
        let kind = if op0 == 0x8C {
            InstructionKind::Mov {
                dest: rm_op,
                src: Operand::Sreg(sreg),
            }
        } else {
            InstructionKind::Mov {
                dest: Operand::Sreg(sreg),
                src: rm_op,
            }
        };
        return Some(insn(cs_ip, bytes[..total].to_vec(), kind));
    }

    // MOV AL/AX ↔ [moffs16] (0xA0/0xA1/0xA2/0xA3)
    if matches!(op0, 0xA0..=0xA3) {
        if bytes.len() < 3 {
            return Some(unknown1(cs_ip, op0));
        }
        let off = u16::from_le_bytes([bytes[1], bytes[2]]);
        let w = (op0 & 1) == 1;
        let size = if w { OpSize::Word } else { OpSize::Byte };
        let mem = Operand::Mem(Mem {
            seg: None,
            base: MemBase::Direct,
            disp: off as i16,
            size,
        });
        let acc = if w {
            Operand::Reg16(Reg16::AX)
        } else {
            Operand::Reg8(Reg8::AL)
        };
        let dir_to_mem = (op0 & 2) == 2; // 0xA2/0xA3 = mem ← acc
        let kind = if dir_to_mem {
            InstructionKind::Mov {
                dest: mem,
                src: acc,
            }
        } else {
            InstructionKind::Mov {
                dest: acc,
                src: mem,
            }
        };
        return Some(insn(cs_ip, vec![op0, bytes[1], bytes[2]], kind));
    }

    // MOV r8, imm8 (0xB0-0xB7)
    if (0xB0..=0xB7).contains(&op0) {
        if bytes.len() < 2 {
            return Some(unknown1(cs_ip, op0));
        }
        return Some(insn(
            cs_ip,
            vec![op0, bytes[1]],
            InstructionKind::Mov {
                dest: Operand::Reg8(Reg8(op0 - 0xB0)),
                src: Operand::Imm8(bytes[1]),
            },
        ));
    }

    // MOV r16, imm16 (0xB8-0xBF)
    if (0xB8..=0xBF).contains(&op0) {
        if bytes.len() < 3 {
            return Some(unknown1(cs_ip, op0));
        }
        let imm = u16::from_le_bytes([bytes[1], bytes[2]]);
        return Some(insn(
            cs_ip,
            vec![op0, bytes[1], bytes[2]],
            InstructionKind::Mov {
                dest: Operand::Reg16(Reg16(op0 - 0xB8)),
                src: Operand::Imm16(imm),
            },
        ));
    }

    // LES/LDS r16, m16:16 (0xC4/0xC5). Register source is invalid.
    if op0 == 0xC4 || op0 == 0xC5 {
        if bytes.len() < 2 || modrm_mod(bytes[1]) == 0b11 {
            return Some(unknown1(cs_ip, op0));
        }
        let mb = bytes[1];
        let (src, used) = match decode_modrm_rm(mb, OpSize::Word, &bytes[2..]) {
            Ok((Operand::Mem(mem), used)) => (mem, used),
            _ => return Some(unknown1(cs_ip, op0)),
        };
        let dest = decode_modrm_reg16(mb);
        let kind = if op0 == 0xC4 {
            InstructionKind::Les { dest, src }
        } else {
            InstructionKind::Lds { dest, src }
        };
        return Some(insn(cs_ip, bytes[..2 + used].to_vec(), kind));
    }

    // MOV r/m8, imm8 (0xC6 /0) / MOV r/m16, imm16 (0xC7 /0)
    if op0 == 0xC6 || op0 == 0xC7 {
        if bytes.len() < 2 {
            return Some(unknown1(cs_ip, op0));
        }
        let mb = bytes[1];
        if modrm_reg(mb) != 0 {
            return Some(unknown1(cs_ip, op0));
        }
        let w = op0 == 0xC7;
        let size = if w { OpSize::Word } else { OpSize::Byte };
        let (rm_op, used_rm) = match decode_modrm_rm(mb, size, &bytes[2..]) {
            Ok(v) => v,
            Err(_) => return Some(unknown1(cs_ip, op0)),
        };
        let imm_off = 2 + used_rm;
        let (src, used_imm) = if w {
            if bytes.len() < imm_off + 2 {
                return Some(unknown1(cs_ip, op0));
            }
            (
                Operand::Imm16(u16::from_le_bytes([bytes[imm_off], bytes[imm_off + 1]])),
                2,
            )
        } else {
            if bytes.len() < imm_off + 1 {
                return Some(unknown1(cs_ip, op0));
            }
            (Operand::Imm8(bytes[imm_off]), 1)
        };
        let total = imm_off + used_imm;
        return Some(insn(
            cs_ip,
            bytes[..total].to_vec(),
            InstructionKind::Mov { dest: rm_op, src },
        ));
    }

    None
}

/// ALU family op — Group 1 reg field 인덱스와 일치 (0..=7).
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AluOp {
    Add,
    Or,
    Adc,
    Sbb,
    And,
    Sub,
    Xor,
    Cmp,
}

impl AluOp {
    fn from_index(i: u8) -> Option<AluOp> {
        Some(match i {
            0 => AluOp::Add,
            1 => AluOp::Or,
            2 => AluOp::Adc,
            3 => AluOp::Sbb,
            4 => AluOp::And,
            5 => AluOp::Sub,
            6 => AluOp::Xor,
            7 => AluOp::Cmp,
            _ => return None,
        })
    }

    fn make_kind(self, dest: Operand, src: Operand) -> InstructionKind {
        match self {
            AluOp::Add => InstructionKind::Add { dest, src },
            AluOp::Or => InstructionKind::Or { dest, src },
            AluOp::Adc => InstructionKind::Adc { dest, src },
            AluOp::Sbb => InstructionKind::Sbb { dest, src },
            AluOp::And => InstructionKind::And { dest, src },
            AluOp::Sub => InstructionKind::Sub { dest, src },
            AluOp::Xor => InstructionKind::Xor { dest, src },
            AluOp::Cmp => InstructionKind::Cmp { a: dest, b: src },
        }
    }
}

/// ALU 그룹: ADD/OR/AND/SUB/XOR/CMP의 직접 인코딩 (0x00-0x05, 0x08-0x0D, 0x20-0x25,
/// 0x28-0x2D, 0x30-0x35, 0x38-0x3D) + Group 1 즉시값 (0x80/0x81/0x82/0x83).
///
/// 8086 ALU 직접 인코딩과 Group 1 immediate 형식을 모두 포함한다.
fn decode_alu(bytes: &[u8], cs_ip: SegOff, op0: u8) -> Option<Instruction> {
    // ALU 직접 인코딩 — base = (op_index << 3), offset 0..=5
    // base+0/1: OP r/m, r (W=offset&1)
    // base+2/3: OP r, r/m  (W=offset&1)
    // base+4:   OP AL, imm8
    // base+5:   OP AX, imm16
    let alu_base = op0 & 0xF8;
    let alu_off = op0 & 0x07;
    if matches!(
        alu_base,
        0x00 | 0x08 | 0x10 | 0x18 | 0x20 | 0x28 | 0x30 | 0x38
    ) && alu_off <= 5
    {
        let aluop_idx = alu_base >> 3;
        let aluop = AluOp::from_index(aluop_idx).unwrap();
        return Some(decode_alu_direct(bytes, cs_ip, op0, aluop, alu_off));
    }

    // Group 1 즉시값 (0x80, 0x81, 0x82, 0x83)
    if matches!(op0, 0x80..=0x83) {
        return Some(decode_alu_group1(bytes, cs_ip, op0));
    }

    None
}

/// ALU 직접 인코딩의 6가지 변종 처리 (op0 & 0x07 = 0..=5).
fn decode_alu_direct(
    bytes: &[u8],
    cs_ip: SegOff,
    op0: u8,
    aluop: AluOp,
    alu_off: u8,
) -> Instruction {
    match alu_off {
        0 | 1 => {
            // OP r/m, r (W=offset&1)
            if bytes.len() < 2 {
                return unknown1(cs_ip, op0);
            }
            let mb = bytes[1];
            let w = alu_off == 1;
            let size = if w { OpSize::Word } else { OpSize::Byte };
            let (rm_op, used) = match decode_modrm_rm(mb, size, &bytes[2..]) {
                Ok(v) => v,
                Err(_) => return unknown1(cs_ip, op0),
            };
            let reg_op = if w {
                Operand::Reg16(decode_modrm_reg16(mb))
            } else {
                Operand::Reg8(decode_modrm_reg8(mb))
            };
            let total = 2 + used;
            insn(
                cs_ip,
                bytes[..total].to_vec(),
                aluop.make_kind(rm_op, reg_op),
            )
        }
        2 | 3 => {
            // OP r, r/m
            if bytes.len() < 2 {
                return unknown1(cs_ip, op0);
            }
            let mb = bytes[1];
            let w = alu_off == 3;
            let size = if w { OpSize::Word } else { OpSize::Byte };
            let (rm_op, used) = match decode_modrm_rm(mb, size, &bytes[2..]) {
                Ok(v) => v,
                Err(_) => return unknown1(cs_ip, op0),
            };
            let reg_op = if w {
                Operand::Reg16(decode_modrm_reg16(mb))
            } else {
                Operand::Reg8(decode_modrm_reg8(mb))
            };
            let total = 2 + used;
            insn(
                cs_ip,
                bytes[..total].to_vec(),
                aluop.make_kind(reg_op, rm_op),
            )
        }
        4 => {
            // OP AL, imm8
            if bytes.len() < 2 {
                return unknown1(cs_ip, op0);
            }
            insn(
                cs_ip,
                vec![op0, bytes[1]],
                aluop.make_kind(Operand::Reg8(Reg8::AL), Operand::Imm8(bytes[1])),
            )
        }
        5 => {
            // OP AX, imm16
            if bytes.len() < 3 {
                return unknown1(cs_ip, op0);
            }
            let imm = u16::from_le_bytes([bytes[1], bytes[2]]);
            insn(
                cs_ip,
                vec![op0, bytes[1], bytes[2]],
                aluop.make_kind(Operand::Reg16(Reg16::AX), Operand::Imm16(imm)),
            )
        }
        _ => unreachable!("alu_off는 0..=5로 제한됨"),
    }
}

/// Group 1 즉시값 (0x80/0x81/0x82/0x83) — OP r/m, imm.
fn decode_alu_group1(bytes: &[u8], cs_ip: SegOff, op0: u8) -> Instruction {
    if bytes.len() < 2 {
        return unknown1(cs_ip, op0);
    }
    let mb = bytes[1];
    let aluop = match AluOp::from_index(modrm_reg(mb)) {
        Some(o) => o,
        None => return unknown1(cs_ip, op0),
    };
    // 0x80/0x82: byte form (W=0). 0x81/0x83: word form (W=1).
    let w = (op0 & 1) == 1;
    let size = if w { OpSize::Word } else { OpSize::Byte };
    let (rm_op, used) = match decode_modrm_rm(mb, size, &bytes[2..]) {
        Ok(v) => v,
        Err(_) => return unknown1(cs_ip, op0),
    };
    let imm_off = 2 + used;
    let (src, imm_used) = match op0 {
        0x80 | 0x82 => {
            if bytes.len() < imm_off + 1 {
                return unknown1(cs_ip, op0);
            }
            (Operand::Imm8(bytes[imm_off]), 1)
        }
        0x81 => {
            if bytes.len() < imm_off + 2 {
                return unknown1(cs_ip, op0);
            }
            (
                Operand::Imm16(u16::from_le_bytes([bytes[imm_off], bytes[imm_off + 1]])),
                2,
            )
        }
        0x83 => {
            if bytes.len() < imm_off + 1 {
                return unknown1(cs_ip, op0);
            }
            // sign-extended imm8 → imm16
            let v = (bytes[imm_off] as i8) as i16 as u16;
            (Operand::Imm16(v), 1)
        }
        _ => unreachable!("op0는 0x80..=0x83"),
    };
    let total = imm_off + imm_used;
    insn(cs_ip, bytes[..total].to_vec(), aluop.make_kind(rm_op, src))
}

/// TEST + INC/DEC short + Group 3/4/5 (0xF6/0xF7/0xFE/0xFF).
///
/// Group 3 (0xF6/0xF7): /0=TEST, /2=NOT, /3=NEG, /4-/7=MUL/IMUL/DIV/IDIV.
/// Group 4 (0xFE): /0=INC, /1=DEC. 그 외 invalid.
/// Group 5 (0xFF): /0=INC, /1=DEC, /2/3=CALL indirect, /4/5=JMP indirect, /6=PUSH r/m.
///
/// 0xFF는 Phase D-2에서 `decode_push_pop`이 reg=6만 처리하던 것을 본 헬퍼로 이동 — 모든
/// Group 5 케이스를 단일 진입점에서 처리.
fn decode_test_inc_dec_neg(bytes: &[u8], cs_ip: SegOff, op0: u8) -> Option<Instruction> {
    // TEST r/m, r (0x84/0x85)
    if matches!(op0, 0x84 | 0x85) {
        if bytes.len() < 2 {
            return Some(unknown1(cs_ip, op0));
        }
        let mb = bytes[1];
        let w = op0 == 0x85;
        let size = if w { OpSize::Word } else { OpSize::Byte };
        let (rm_op, used) = match decode_modrm_rm(mb, size, &bytes[2..]) {
            Ok(v) => v,
            Err(_) => return Some(unknown1(cs_ip, op0)),
        };
        let reg_op = if w {
            Operand::Reg16(decode_modrm_reg16(mb))
        } else {
            Operand::Reg8(decode_modrm_reg8(mb))
        };
        let total = 2 + used;
        return Some(insn(
            cs_ip,
            bytes[..total].to_vec(),
            InstructionKind::Test {
                a: rm_op,
                b: reg_op,
            },
        ));
    }

    // TEST AL, imm8 (0xA8) / TEST AX, imm16 (0xA9)
    if op0 == 0xA8 {
        if bytes.len() < 2 {
            return Some(unknown1(cs_ip, op0));
        }
        return Some(insn(
            cs_ip,
            vec![0xA8, bytes[1]],
            InstructionKind::Test {
                a: Operand::Reg8(Reg8::AL),
                b: Operand::Imm8(bytes[1]),
            },
        ));
    }
    if op0 == 0xA9 {
        if bytes.len() < 3 {
            return Some(unknown1(cs_ip, op0));
        }
        let imm = u16::from_le_bytes([bytes[1], bytes[2]]);
        return Some(insn(
            cs_ip,
            vec![0xA9, bytes[1], bytes[2]],
            InstructionKind::Test {
                a: Operand::Reg16(Reg16::AX),
                b: Operand::Imm16(imm),
            },
        ));
    }

    // INC reg16 (0x40-0x47) / DEC reg16 (0x48-0x4F) — 단축형
    if (0x40..=0x47).contains(&op0) {
        return Some(insn(
            cs_ip,
            vec![op0],
            InstructionKind::Inc {
                dest: Operand::Reg16(Reg16(op0 - 0x40)),
            },
        ));
    }
    if (0x48..=0x4F).contains(&op0) {
        return Some(insn(
            cs_ip,
            vec![op0],
            InstructionKind::Dec {
                dest: Operand::Reg16(Reg16(op0 - 0x48)),
            },
        ));
    }

    // Group 3 (0xF6/0xF7).
    if op0 == 0xF6 || op0 == 0xF7 {
        return Some(decode_group_3(bytes, cs_ip, op0));
    }

    // Group 4 (0xFE) — INC/DEC r/m8
    if op0 == 0xFE {
        return Some(decode_group_4(bytes, cs_ip, op0));
    }

    // Group 5 (0xFF) — INC/DEC r/m16, CALL/JMP indirect, PUSH r/m16
    if op0 == 0xFF {
        return Some(decode_group_5(bytes, cs_ip, op0));
    }

    None
}

/// Group 3 (0xF6/0xF7 + ModR/M).
/// /0 = TEST, /2 = NOT, /3 = NEG, /4-/7 = MUL/IMUL/DIV/IDIV.
fn decode_group_3(bytes: &[u8], cs_ip: SegOff, op0: u8) -> Instruction {
    if bytes.len() < 2 {
        return unknown1(cs_ip, op0);
    }
    let mb = bytes[1];
    let w = op0 == 0xF7;
    let size = if w { OpSize::Word } else { OpSize::Byte };
    match modrm_reg(mb) {
        0 => {
            // TEST r/m, imm
            let (rm_op, used) = match decode_modrm_rm(mb, size, &bytes[2..]) {
                Ok(v) => v,
                Err(_) => return unknown1(cs_ip, op0),
            };
            let imm_off = 2 + used;
            let (src, imm_used) = if w {
                if bytes.len() < imm_off + 2 {
                    return unknown1(cs_ip, op0);
                }
                (
                    Operand::Imm16(u16::from_le_bytes([bytes[imm_off], bytes[imm_off + 1]])),
                    2,
                )
            } else {
                if bytes.len() < imm_off + 1 {
                    return unknown1(cs_ip, op0);
                }
                (Operand::Imm8(bytes[imm_off]), 1)
            };
            let total = imm_off + imm_used;
            insn(
                cs_ip,
                bytes[..total].to_vec(),
                InstructionKind::Test { a: rm_op, b: src },
            )
        }
        operation @ 2..=7 => {
            let (rm_op, used) = match decode_modrm_rm(mb, size, &bytes[2..]) {
                Ok(v) => v,
                Err(_) => return unknown1(cs_ip, op0),
            };
            let total = 2 + used;
            let kind = match operation {
                2 => InstructionKind::Not { dest: rm_op },
                3 => InstructionKind::Neg { dest: rm_op },
                4 => InstructionKind::Mul { src: rm_op },
                5 => InstructionKind::Imul { src: rm_op },
                6 => InstructionKind::Div { src: rm_op },
                7 => InstructionKind::Idiv { src: rm_op },
                _ => unreachable!(),
            };
            insn(cs_ip, bytes[..total].to_vec(), kind)
        }
        _ => unknown1(cs_ip, op0),
    }
}

/// Group 4 (0xFE + ModR/M) — INC/DEC r/m8. /0=INC /1=DEC, 그 외는 invalid.
fn decode_group_4(bytes: &[u8], cs_ip: SegOff, op0: u8) -> Instruction {
    if bytes.len() < 2 {
        return unknown1(cs_ip, op0);
    }
    let mb = bytes[1];
    let is_inc = match modrm_reg(mb) {
        0 => true,
        1 => false,
        _ => return unknown1(cs_ip, op0),
    };
    let (rm_op, used) = match decode_modrm_rm(mb, OpSize::Byte, &bytes[2..]) {
        Ok(v) => v,
        Err(_) => return unknown1(cs_ip, op0),
    };
    let total = 2 + used;
    let kind = if is_inc {
        InstructionKind::Inc { dest: rm_op }
    } else {
        InstructionKind::Dec { dest: rm_op }
    };
    insn(cs_ip, bytes[..total].to_vec(), kind)
}

/// Group 5 (0xFF + ModR/M) — INC/DEC r/m16, CALL/JMP indirect, PUSH r/m16.
///
/// reg field 매핑:
/// - 0 = INC r/m16
/// - 1 = DEC r/m16
/// - 2 = CALL near r/m16 (간접) → `CallTarget::IndirectNear`
/// - 3 = CALL far m16:16 (간접) → `CallTarget::IndirectFar` (mem only — reg form은 Unknown)
/// - 4 = JMP near r/m16 (간접) → `JmpTarget::IndirectNear`
/// - 5 = JMP far m16:16 (간접) → `JmpTarget::IndirectFar` (mem only — reg form은 Unknown)
/// - 6 = PUSH r/m16
/// - 7 = invalid → Unknown
///
/// `/3`과 `/5`의 mem operand는 32비트 far pointer (CS:IP)를 메모리에서 로드 — near indirect와
/// 의미가 다르다. reg form (mod=11)은 spec-undefined.
fn decode_group_5(bytes: &[u8], cs_ip: SegOff, op0: u8) -> Instruction {
    if bytes.len() < 2 {
        return unknown1(cs_ip, op0);
    }
    let mb = bytes[1];
    let reg_field = modrm_reg(mb);
    if reg_field == 7 {
        return unknown1(cs_ip, op0);
    }
    let (rm_op, used) = match decode_modrm_rm(mb, OpSize::Word, &bytes[2..]) {
        Ok(v) => v,
        Err(_) => return unknown1(cs_ip, op0),
    };
    let total = 2 + used;
    let kind = match reg_field {
        0 => InstructionKind::Inc { dest: rm_op },
        1 => InstructionKind::Dec { dest: rm_op },
        2 => InstructionKind::Call {
            target: CallTarget::IndirectNear(rm_op),
        },
        3 => match rm_op {
            Operand::Mem(m) => InstructionKind::Call {
                target: CallTarget::IndirectFar(m),
            },
            // reg form은 spec-undefined — far indirect는 메모리에서 32비트 far pointer
            // 로드를 요구. silent하게 near로 normalize하지 않는다.
            _ => return unknown1(cs_ip, op0),
        },
        4 => InstructionKind::Jmp {
            target: JmpTarget::IndirectNear(rm_op),
        },
        5 => match rm_op {
            Operand::Mem(m) => InstructionKind::Jmp {
                target: JmpTarget::IndirectFar(m),
            },
            _ => return unknown1(cs_ip, op0),
        },
        6 => InstructionKind::Push { src: rm_op },
        _ => unreachable!("reg_field 7은 위에서 거부됨"),
    };
    insn(cs_ip, bytes[..total].to_vec(), kind)
}

/// Phase D-4: 시프트/로테이트 (Group 2, 0xD0-0xD3) + I/O (0xE4-0xE7, 0xEC-0xEF) +
/// LEA (0x8D) + XCHG (0x86/0x87 + 0x91-0x97).
///
/// - **시프트 Group 2** (0xD0-0xD3 + ModR/M): reg field로 op 선택.
///   0=ROL, 1=ROR, 2=RCL, 3=RCR, 4=SHL/SAL(canonical: SHL), 5=SHR, 7=SAR.
///   6=reserved는 Unknown.
///   bit 0 = W (byte/word), bit 1 = count source (0=imm 1, 1=CL).
/// - **I/O**: imm8 form (0xE4-0xE7) + DX form (0xEC-0xEF), AL/AX는 LSB로 결정.
/// - **LEA**: reg form (mod=11)은 의미상 invalid → Unknown.
/// - **XCHG**: 0x91-0x97은 `XCHG AX, r16` 단축형 (0x90은 NOP). 0x86/0x87은 일반 r/m ↔ r 형식.
fn decode_shift_io_lea_xchg(bytes: &[u8], cs_ip: SegOff, op0: u8) -> Option<Instruction> {
    // Group 2 시프트/로테이트 (0xD0/0xD1/0xD2/0xD3)
    if matches!(op0, 0xD0..=0xD3) {
        if bytes.len() < 2 {
            return Some(unknown1(cs_ip, op0));
        }
        let mb = bytes[1];
        let w = (op0 & 1) == 1;
        let count = if (op0 & 2) == 2 {
            ShiftCount::Cl
        } else {
            ShiftCount::One
        };
        let size = if w { OpSize::Word } else { OpSize::Byte };
        let (rm_op, used) = match decode_modrm_rm(mb, size, &bytes[2..]) {
            Ok(v) => v,
            Err(_) => return Some(unknown1(cs_ip, op0)),
        };
        let total = 2 + used;
        let kind = match modrm_reg(mb) {
            0 => InstructionKind::Rol { dest: rm_op, count },
            1 => InstructionKind::Ror { dest: rm_op, count },
            2 => InstructionKind::Rcl { dest: rm_op, count },
            3 => InstructionKind::Rcr { dest: rm_op, count },
            // canonical: SHL/SAL은 같은 인코딩 → 디코더는 Shl로 통일.
            4 => InstructionKind::Shl { dest: rm_op, count },
            5 => InstructionKind::Shr { dest: rm_op, count },
            7 => InstructionKind::Sar { dest: rm_op, count },
            // /6 is reserved.
            _ => return Some(unknown1(cs_ip, op0)),
        };
        return Some(insn(cs_ip, bytes[..total].to_vec(), kind));
    }

    // I/O — IN AL/AX, imm8 (0xE4/0xE5)
    if op0 == 0xE4 {
        if bytes.len() < 2 {
            return Some(unknown1(cs_ip, op0));
        }
        return Some(insn(
            cs_ip,
            vec![0xE4, bytes[1]],
            InstructionKind::InAl {
                port: PortAddr::Imm8(bytes[1]),
            },
        ));
    }
    if op0 == 0xE5 {
        if bytes.len() < 2 {
            return Some(unknown1(cs_ip, op0));
        }
        return Some(insn(
            cs_ip,
            vec![0xE5, bytes[1]],
            InstructionKind::InAx {
                port: PortAddr::Imm8(bytes[1]),
            },
        ));
    }
    // OUT imm8, AL/AX (0xE6/0xE7)
    if op0 == 0xE6 {
        if bytes.len() < 2 {
            return Some(unknown1(cs_ip, op0));
        }
        return Some(insn(
            cs_ip,
            vec![0xE6, bytes[1]],
            InstructionKind::OutAl {
                port: PortAddr::Imm8(bytes[1]),
            },
        ));
    }
    if op0 == 0xE7 {
        if bytes.len() < 2 {
            return Some(unknown1(cs_ip, op0));
        }
        return Some(insn(
            cs_ip,
            vec![0xE7, bytes[1]],
            InstructionKind::OutAx {
                port: PortAddr::Imm8(bytes[1]),
            },
        ));
    }
    // I/O — DX 형식 (0xEC-0xEF)
    if op0 == 0xEC {
        return Some(insn(
            cs_ip,
            vec![0xEC],
            InstructionKind::InAl { port: PortAddr::Dx },
        ));
    }
    if op0 == 0xED {
        return Some(insn(
            cs_ip,
            vec![0xED],
            InstructionKind::InAx { port: PortAddr::Dx },
        ));
    }
    if op0 == 0xEE {
        return Some(insn(
            cs_ip,
            vec![0xEE],
            InstructionKind::OutAl { port: PortAddr::Dx },
        ));
    }
    if op0 == 0xEF {
        return Some(insn(
            cs_ip,
            vec![0xEF],
            InstructionKind::OutAx { port: PortAddr::Dx },
        ));
    }

    // LEA r16, m (0x8D /r) — m이 register form (mod=11)이면 invalid → Unknown.
    if op0 == 0x8D {
        if bytes.len() < 2 {
            return Some(unknown1(cs_ip, op0));
        }
        let mb = bytes[1];
        if modrm_mod(mb) == 0b11 {
            return Some(unknown1(cs_ip, op0));
        }
        let (rm_op, used) = match decode_modrm_rm(mb, OpSize::Word, &bytes[2..]) {
            Ok(v) => v,
            Err(_) => return Some(unknown1(cs_ip, op0)),
        };
        let mem = match rm_op {
            Operand::Mem(m) => m,
            _ => return Some(unknown1(cs_ip, op0)),
        };
        let total = 2 + used;
        return Some(insn(
            cs_ip,
            bytes[..total].to_vec(),
            InstructionKind::Lea {
                dest: decode_modrm_reg16(mb),
                src: mem,
            },
        ));
    }

    // XCHG AX, r16 (0x91-0x97) — 0x90은 NOP으로 이미 처리됨 (decode_simple_byte).
    if (0x91..=0x97).contains(&op0) {
        return Some(insn(
            cs_ip,
            vec![op0],
            InstructionKind::Xchg {
                a: Operand::Reg16(Reg16::AX),
                b: Operand::Reg16(Reg16(op0 - 0x90)),
            },
        ));
    }

    // XCHG r/m, r (0x86/0x87)
    if matches!(op0, 0x86 | 0x87) {
        if bytes.len() < 2 {
            return Some(unknown1(cs_ip, op0));
        }
        let mb = bytes[1];
        let w = op0 == 0x87;
        let size = if w { OpSize::Word } else { OpSize::Byte };
        let (rm_op, used) = match decode_modrm_rm(mb, size, &bytes[2..]) {
            Ok(v) => v,
            Err(_) => return Some(unknown1(cs_ip, op0)),
        };
        let reg_op = if w {
            Operand::Reg16(decode_modrm_reg16(mb))
        } else {
            Operand::Reg8(decode_modrm_reg8(mb))
        };
        let total = 2 + used;
        return Some(insn(
            cs_ip,
            bytes[..total].to_vec(),
            InstructionKind::Xchg {
                a: rm_op,
                b: reg_op,
            },
        ));
    }

    None
}

fn insn(cs_ip: SegOff, bytes: Vec<u8>, kind: InstructionKind) -> Instruction {
    Instruction {
        cs_ip,
        bytes,
        kind,
        prefixes: Prefixes::default(),
    }
}

fn unknown1(cs_ip: SegOff, b: u8) -> Instruction {
    Instruction {
        cs_ip,
        bytes: vec![b],
        kind: InstructionKind::Unknown { raw: vec![b] },
        prefixes: Prefixes::default(),
    }
}

/// 원본 회귀 fixture만을 위한 관대한 linear sweep.
#[cfg(test)]
pub fn decode_block(bytes: &[u8], start: SegOff) -> Vec<Instruction> {
    let mut out = Vec::new();
    let mut pos = 0;
    let mut ip = start;
    while pos < bytes.len() {
        match decode(&bytes[pos..], ip) {
            Ok(insn) => {
                let len = insn.bytes.len();
                pos += len;
                ip = SegOff {
                    seg: ip.seg,
                    off: ip.off.wrapping_add(len as u16),
                };
                out.push(insn);
            }
            Err(_) => {
                out.push(Instruction {
                    cs_ip: ip,
                    bytes: vec![bytes[pos]],
                    kind: InstructionKind::Unknown {
                        raw: vec![bytes[pos]],
                    },
                    prefixes: Prefixes::default(),
                });
                pos += 1;
                ip = SegOff {
                    seg: ip.seg,
                    off: ip.off.wrapping_add(1),
                };
            }
        }
    }
    out
}
