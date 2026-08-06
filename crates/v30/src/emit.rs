//! x86 인코더 — InstructionKind → bytes.
//!
//! 내부 `Unknown { raw }` 출력은 반입 회귀 시험만 지원한다. 공개 `encode_bytes`와
//! checked assembler는 wrapper 안에 든 `Unknown`까지 실행 코드로 내리지 않는다.
//!
//! ## Canonical 정책 (spec §1.1, §4.4.1)
//!
//! 8086 ISA는 같은 명령어의 redundant byte encoding을 허용한다 (예: `MOV BX, AX`는
//! `0x89 0xC3` 또는 `0x8B 0xD8` 양쪽 가능). 본 인코더는 항상 **canonical form** 하나만
//! 생성해 결정적 결과를 보장:
//!
//! 1. **Reg-reg MOV**: `0x88/0x89` (r/m ← r 형식, dir=0). `0x8A/0x8B`(r ← r/m)는
//!    decode-only — mod=11 케이스에서는 같은 의미를 만들지만 emit은 항상 dir=0.
//! 2. **Memory disp=0** (BP 제외): `mod=00` (no disp)
//! 3. **Memory BP base 또는 disp ∈ [-128..=127]**: `mod=01 disp8`
//! 4. **Memory disp 그 외**: `mod=10 disp16`
//! 5. **POP r16**: `0x58+r` 단축형. Group form `0x8F /0 reg`로 디코드되어도 emit은 단축형.
//! 6. **PUSH/POP sreg**: `0x06/0E/16/1E`, `0x07/17/1F` 단축형.
//! 7. **r/m=AL/AX moffs MOV**: `0xA0-0xA3` 단축형 (Disp16 + 묵시적 segment).
//! 8. **r ← imm MOV**: `0xB0+rb` / `0xB8+rw` 단축형 (r/m form `0xC6/0xC7 /0`보다 우선).
//!
//! 따라서 `decode(emit(kind)) == kind`는 항상 성립하지만, `emit(decode(b)) == b`는
//! `b`가 이미 canonical일 때만 성립한다. 디스크의 raw byte 보존은 ISA 레벨이 아닌
//! `Instruction.bytes` 또는 디스크 컨테이너 레벨에서 보장한다.
//!
//! ## Exact source와 canonical 생성
//!
//! 공개 decode 결과는 합법 alias와 중복 prefix를 `DecodedInstruction::source_bytes()`와
//! `PrefixSequence`에 그대로 보존한다. typed `Instruction`은 마지막으로 유효한 prefix의
//! 의미만 가지며 이 모듈은 canonical 순서와 encoding 하나를 생성한다. Expected Write의
//! 실행 코드는 이 canonical byte를 다시 decode/reassemble해 검증한다.
//!
//! - **다중 prefix 동일 카테고리**: source 순서와 중복은 decode 결과에 보존되고 typed
//!   instruction에는 마지막 prefix가 적용된다. 새 코드는 중복을 만들지 않는다.
//! - **XCHG canonical 비대칭**: `Xchg { a: AX, b: r }`는 `0x90+r` 단축, `Xchg { a: r, b: AX }`는
//!   `0x87 /r` 일반형. operand 순서를 IR이 보존하므로 byte는 다르나 의미 동등.
//!   `decode(emit(kind)) == kind` canonical 안정점은 양쪽 모두 보존.

use crate::instruction::*;

#[derive(Debug, thiserror::Error)]
pub enum EmitError {
    #[error("invalid encoding for kind")]
    Invalid,
}

pub fn emit(kind: &InstructionKind) -> Result<Vec<u8>, EmitError> {
    // segment override prefix 처리 — 메모리 operand에 `seg`가 있으면 prefix를 결과 맨
    // 앞에 prepend. seg를 제거한 clone으로 재귀 호출해 모든 인코딩 분기가 prefix를
    // 신경쓰지 않도록 한다. REP/REPNE wrap variant (`Rep`/`Repne`)은 inner를 재귀 emit하므로
    // 자연히 이 경로를 통과한다.
    if let Some((s, stripped)) = strip_seg_override(kind) {
        let inner = emit(&stripped)?;
        let mut out = vec![seg_override_prefix(s)];
        out.extend(inner);
        return Ok(out);
    }
    emit_inner(kind)
}

fn emit_inner(kind: &InstructionKind) -> Result<Vec<u8>, EmitError> {
    match kind {
        InstructionKind::Nop => Ok(vec![0x90]),
        InstructionKind::Hlt => Ok(vec![0xF4]),
        InstructionKind::Xlat => Ok(vec![0xD7]),
        InstructionKind::Iret => Ok(vec![0xCF]),
        InstructionKind::Into => Ok(vec![0xCE]),
        InstructionKind::Int { vector: 3 } => Ok(vec![0xCC]),
        InstructionKind::Int { vector } => Ok(vec![0xCD, *vector]),

        // 플래그 명령 / sign-extend — 모두 single-byte (Intel SDM Vol 2A 부록 B).
        InstructionKind::Wait => Ok(vec![0x9B]),
        InstructionKind::Pushf => Ok(vec![0x9C]),
        InstructionKind::Popf => Ok(vec![0x9D]),
        InstructionKind::Sahf => Ok(vec![0x9E]),
        InstructionKind::Lahf => Ok(vec![0x9F]),
        InstructionKind::Cmc => Ok(vec![0xF5]),
        InstructionKind::Clc => Ok(vec![0xF8]),
        InstructionKind::Stc => Ok(vec![0xF9]),
        InstructionKind::Cli => Ok(vec![0xFA]),
        InstructionKind::Sti => Ok(vec![0xFB]),
        InstructionKind::Cld => Ok(vec![0xFC]),
        InstructionKind::Std => Ok(vec![0xFD]),
        InstructionKind::Cbw => Ok(vec![0x98]),
        InstructionKind::Cwd => Ok(vec![0x99]),
        InstructionKind::Aaa => Ok(vec![0x37]),
        InstructionKind::Aas => Ok(vec![0x3F]),
        InstructionKind::Daa => Ok(vec![0x27]),
        InstructionKind::Das => Ok(vec![0x2F]),
        InstructionKind::Aam { base } => Ok(vec![0xD4, *base]),
        InstructionKind::Aad { base } => Ok(vec![0xD5, *base]),

        // Jcc
        InstructionKind::Jcc {
            cond: Cond::Cxz,
            target,
        } => Ok(vec![0xE3, *target as u8]),
        InstructionKind::Jcc { cond, target } => {
            let i = cond.to_jcc_index().ok_or(EmitError::Invalid)?;
            Ok(vec![0x70 + i, *target as u8])
        }
        InstructionKind::Loop { condition, target } => {
            let opcode = match condition {
                LoopCondition::NotEqual => 0xE0,
                LoopCondition::Equal => 0xE1,
                LoopCondition::Always => 0xE2,
            };
            Ok(vec![opcode, *target as u8])
        }

        // 무조건 분기
        InstructionKind::Jmp { target } => match target {
            JmpTarget::Rel8(r) => Ok(vec![0xEB, *r as u8]),
            JmpTarget::Rel16(r) => {
                let b = r.to_le_bytes();
                Ok(vec![0xE9, b[0], b[1]])
            }
            JmpTarget::Far { seg, off } => {
                let o = off.to_le_bytes();
                let s = seg.to_le_bytes();
                Ok(vec![0xEA, o[0], o[1], s[0], s[1]])
            }
            JmpTarget::IndirectNear(op) => {
                // Group 5 0xFF /4 = JMP near r/m16 (간접)
                let mut out = vec![0xFF];
                match op {
                    Operand::Reg16(_) => out.extend(emit_reg_rm(op, 4)?),
                    Operand::Mem(m) => out.extend(emit_mem_rm(m, 4)?),
                    _ => return Err(EmitError::Invalid),
                }
                Ok(out)
            }
            JmpTarget::IndirectFar(m) => {
                // Group 5 0xFF /5 = JMP far m16:16 (간접) — 메모리 전용
                let mut out = vec![0xFF];
                out.extend(emit_mem_rm(m, 5)?);
                Ok(out)
            }
        },
        InstructionKind::Call { target } => match target {
            CallTarget::Rel16(r) => {
                let b = r.to_le_bytes();
                Ok(vec![0xE8, b[0], b[1]])
            }
            CallTarget::Far { seg, off } => {
                let o = off.to_le_bytes();
                let s = seg.to_le_bytes();
                Ok(vec![0x9A, o[0], o[1], s[0], s[1]])
            }
            CallTarget::IndirectNear(op) => {
                // Group 5 0xFF /2 = CALL near r/m16 (간접)
                let mut out = vec![0xFF];
                match op {
                    Operand::Reg16(_) => out.extend(emit_reg_rm(op, 2)?),
                    Operand::Mem(m) => out.extend(emit_mem_rm(m, 2)?),
                    _ => return Err(EmitError::Invalid),
                }
                Ok(out)
            }
            CallTarget::IndirectFar(m) => {
                // Group 5 0xFF /3 = CALL far m16:16 (간접) — 메모리 전용
                let mut out = vec![0xFF];
                out.extend(emit_mem_rm(m, 3)?);
                Ok(out)
            }
        },
        InstructionKind::Ret { pop: 0 } => Ok(vec![0xC3]),
        InstructionKind::Ret { pop } => {
            let b = pop.to_le_bytes();
            Ok(vec![0xC2, b[0], b[1]])
        }
        InstructionKind::Retf { pop: 0 } => Ok(vec![0xCB]),
        InstructionKind::Retf { pop } => {
            let b = pop.to_le_bytes();
            Ok(vec![0xCA, b[0], b[1]])
        }

        // PUSH/POP 단축 우선
        InstructionKind::Push {
            src: Operand::Reg16(r),
        } => Ok(vec![0x50 + r.0]),
        InstructionKind::Pop {
            dest: Operand::Reg16(r),
        } => Ok(vec![0x58 + r.0]),
        InstructionKind::Push {
            src: Operand::Sreg(s),
        } => Ok(vec![match s {
            Sreg::ES => 0x06,
            Sreg::CS => 0x0E,
            Sreg::SS => 0x16,
            Sreg::DS => 0x1E,
        }]),
        InstructionKind::Pop {
            dest: Operand::Sreg(s),
        } => Ok(vec![match s {
            Sreg::ES => 0x07,
            Sreg::SS => 0x17,
            Sreg::DS => 0x1F,
            Sreg::CS => return Err(EmitError::Invalid), // POP CS는 미지원
        }]),
        InstructionKind::Push {
            src: Operand::Mem(m),
        } => {
            let mut out = vec![0xFF];
            out.extend(emit_mem_rm(m, 6)?);
            Ok(out)
        }
        InstructionKind::Pop {
            dest: Operand::Mem(m),
        } => {
            let mut out = vec![0x8F];
            out.extend(emit_mem_rm(m, 0)?);
            Ok(out)
        }
        InstructionKind::Push {
            src: Operand::Imm8(value),
        } => Ok(vec![0x6A, *value]),
        InstructionKind::Push {
            src: Operand::Imm16(value),
        } => {
            let value = value.to_le_bytes();
            Ok(vec![0x68, value[0], value[1]])
        }
        InstructionKind::Pusha => Ok(vec![0x60]),
        InstructionKind::Popa => Ok(vec![0x61]),
        InstructionKind::Bound { index, bounds } => {
            if bounds.size != OpSize::Word {
                return Err(EmitError::Invalid);
            }
            let mut out = vec![0x62];
            out.extend(emit_mem_rm(bounds, index.0)?);
            Ok(out)
        }

        // MOV — 다양한 변종을 우선순위로 인코딩
        InstructionKind::Mov { dest, src } => emit_mov(dest, src),

        // ALU family — emit_alu 헬퍼로 통합
        InstructionKind::Add { dest, src } => emit_alu(EmitAluOp::Add, dest, src),
        InstructionKind::Or { dest, src } => emit_alu(EmitAluOp::Or, dest, src),
        InstructionKind::Adc { dest, src } => emit_alu(EmitAluOp::Adc, dest, src),
        InstructionKind::Sbb { dest, src } => emit_alu(EmitAluOp::Sbb, dest, src),
        InstructionKind::And { dest, src } => emit_alu(EmitAluOp::And, dest, src),
        InstructionKind::Sub { dest, src } => emit_alu(EmitAluOp::Sub, dest, src),
        InstructionKind::Xor { dest, src } => emit_alu(EmitAluOp::Xor, dest, src),
        InstructionKind::Cmp { a, b } => emit_alu(EmitAluOp::Cmp, a, b),

        // TEST — ALU와 다른 인코딩 (0x84/85, 0xA8/A9, 0xF6/F7 /0)
        InstructionKind::Test { a, b } => emit_test(a, b),

        // INC/DEC reg16 단축형 (0x40+r / 0x48+r) — canonical
        InstructionKind::Inc {
            dest: Operand::Reg16(r),
        } => Ok(vec![0x40 + r.0]),
        InstructionKind::Dec {
            dest: Operand::Reg16(r),
        } => Ok(vec![0x48 + r.0]),

        // INC/DEC r/m (Group 4 0xFE: r/m8, Group 5 0xFF /0,/1: r/m16)
        InstructionKind::Inc { dest } => emit_inc_dec(dest, /*reg_field=*/ 0),
        InstructionKind::Dec { dest } => emit_inc_dec(dest, /*reg_field=*/ 1),

        // Group 3 unary operations.
        InstructionKind::Not { dest } => emit_group_3_unary(dest, 2),
        InstructionKind::Neg { dest } => emit_group_3_unary(dest, 3),
        InstructionKind::Mul { src } => emit_group_3_unary(src, 4),
        InstructionKind::Imul { src } => emit_group_3_unary(src, 5),
        InstructionKind::ImulImmediate {
            dest,
            src,
            immediate,
        } => emit_imul_immediate(*dest, src, *immediate),
        InstructionKind::Div { src } => emit_group_3_unary(src, 6),
        InstructionKind::Idiv { src } => emit_group_3_unary(src, 7),

        // 시프트/로테이트 (Group 2 0xD0-0xD3 + reg field)
        // NEC/Intel의 SAL 표기는 SHL과 같은 `/4` selector이므로 typed 값은 SHL로 통일한다.
        InstructionKind::Rol { dest, count } => emit_shift(0, dest, *count),
        InstructionKind::Ror { dest, count } => emit_shift(1, dest, *count),
        InstructionKind::Rcl { dest, count } => emit_shift(2, dest, *count),
        InstructionKind::Rcr { dest, count } => emit_shift(3, dest, *count),
        InstructionKind::Shl { dest, count } => emit_shift(4, dest, *count),
        InstructionKind::Shr { dest, count } => emit_shift(5, dest, *count),
        InstructionKind::Sar { dest, count } => emit_shift(7, dest, *count),

        // I/O — imm8 form (0xE4-0xE7) + DX form (0xEC-0xEF)
        InstructionKind::InAl {
            port: PortAddr::Imm8(p),
        } => Ok(vec![0xE4, *p]),
        InstructionKind::InAx {
            port: PortAddr::Imm8(p),
        } => Ok(vec![0xE5, *p]),
        InstructionKind::OutAl {
            port: PortAddr::Imm8(p),
        } => Ok(vec![0xE6, *p]),
        InstructionKind::OutAx {
            port: PortAddr::Imm8(p),
        } => Ok(vec![0xE7, *p]),
        InstructionKind::InAl { port: PortAddr::Dx } => Ok(vec![0xEC]),
        InstructionKind::InAx { port: PortAddr::Dx } => Ok(vec![0xED]),
        InstructionKind::OutAl { port: PortAddr::Dx } => Ok(vec![0xEE]),
        InstructionKind::OutAx { port: PortAddr::Dx } => Ok(vec![0xEF]),
        InstructionKind::Insb => Ok(vec![0x6C]),
        InstructionKind::Insw => Ok(vec![0x6D]),
        InstructionKind::Outsb => Ok(vec![0x6E]),
        InstructionKind::Outsw => Ok(vec![0x6F]),

        // LEA r16, m (0x8D /r)
        InstructionKind::Lea { dest, src } => {
            let mut out = vec![0x8D];
            out.extend(emit_mem_rm(src, dest.0)?);
            Ok(out)
        }
        InstructionKind::Les { dest, src } => {
            let mut out = vec![0xC4];
            out.extend(emit_mem_rm(src, dest.0)?);
            Ok(out)
        }
        InstructionKind::Lds { dest, src } => {
            let mut out = vec![0xC5];
            out.extend(emit_mem_rm(src, dest.0)?);
            Ok(out)
        }

        // XCHG — AX, r16 단축형 (0x90+rw) 우선, 그 외는 0x86/0x87 /r 일반형
        InstructionKind::Xchg { a, b } => emit_xchg(a, b),

        // 문자열 명령 (단일 바이트). REP/REPNE은 별도 variant로 wrap.
        InstructionKind::Movsb => Ok(vec![0xA4]),
        InstructionKind::Movsw => Ok(vec![0xA5]),
        InstructionKind::Cmpsb => Ok(vec![0xA6]),
        InstructionKind::Cmpsw => Ok(vec![0xA7]),
        InstructionKind::Stosb => Ok(vec![0xAA]),
        InstructionKind::Stosw => Ok(vec![0xAB]),
        InstructionKind::Lodsb => Ok(vec![0xAC]),
        InstructionKind::Lodsw => Ok(vec![0xAD]),
        InstructionKind::Scasb => Ok(vec![0xAE]),
        InstructionKind::Scasw => Ok(vec![0xAF]),

        // REP/REPNE prefix wrap — inner 명령을 emit한 결과 앞에 0xF3/0xF2 prepend.
        InstructionKind::Rep(inner) => {
            require_repeatable(inner)?;
            let mut out = vec![0xF3];
            out.extend(emit(inner)?);
            Ok(out)
        }
        InstructionKind::Repne(inner) => {
            require_comparison_repeatable(inner)?;
            let mut out = vec![0xF2];
            out.extend(emit(inner)?);
            Ok(out)
        }
        InstructionKind::Repc(inner) => {
            require_comparison_repeatable(inner)?;
            let mut out = vec![0x65];
            out.extend(emit(inner)?);
            Ok(out)
        }
        InstructionKind::Repnc(inner) => {
            require_comparison_repeatable(inner)?;
            let mut out = vec![0x64];
            out.extend(emit(inner)?);
            Ok(out)
        }
        InstructionKind::BusLock(inner) => {
            let mut out = vec![0xF0];
            out.extend(emit(inner)?);
            Ok(out)
        }
        InstructionKind::SegmentOverride {
            segment,
            instruction,
        } => {
            if !has_implicit_segment_source(instruction) {
                return Err(EmitError::Invalid);
            }
            let mut out = vec![seg_override_prefix(*segment)];
            out.extend(emit(instruction)?);
            Ok(out)
        }

        InstructionKind::Enter {
            frame_size,
            nesting_level,
        } => {
            let frame_size = frame_size.to_le_bytes();
            Ok(vec![0xC8, frame_size[0], frame_size[1], *nesting_level])
        }
        InstructionKind::Leave => Ok(vec![0xC9]),
        InstructionKind::Esc { operation, operand } => {
            emit_external_operation(0xD8, operation.value(), operand)
        }
        InstructionKind::Fpo2 { operation, operand } => {
            emit_external_operation(0x66, operation.value(), operand)
        }

        // V30 (NEC uPD70116) 0x0F secondary opcode 확장 — canonical 인코딩.
        // 출처: NEC uPD70116 데이터시트 §3.
        InstructionKind::Add4s => Ok(vec![0x0F, 0x20]),
        InstructionKind::Sub4s => Ok(vec![0x0F, 0x22]),
        InstructionKind::Cmp4s => Ok(vec![0x0F, 0x26]),
        InstructionKind::Rol4 { rm } => emit_v30_rm8(0x28, rm, 0),
        InstructionKind::Ror4 { rm } => emit_v30_rm8(0x2A, rm, 0),
        InstructionKind::Ins { bit_offset, length } => Ok(vec![
            0x0F,
            0x31,
            modrm_byte(3, length.index(), bit_offset.index()),
        ]),
        InstructionKind::Ext { bit_offset, length } => Ok(vec![
            0x0F,
            0x33,
            modrm_byte(3, length.index(), bit_offset.index()),
        ]),
        InstructionKind::InsImm { bit_offset, length } => Ok(vec![
            0x0F,
            0x39,
            modrm_byte(3, 0, bit_offset.index()),
            length.selector(),
        ]),
        InstructionKind::ExtImm { bit_offset, length } => Ok(vec![
            0x0F,
            0x3B,
            modrm_byte(3, 0, bit_offset.index()),
            length.selector(),
        ]),
        InstructionKind::Test1 { target, bit } => emit_v30_bit(0x10, target, *bit),
        InstructionKind::Clr1 { target, bit } => emit_v30_bit(0x12, target, *bit),
        InstructionKind::Set1 { target, bit } => emit_v30_bit(0x14, target, *bit),
        InstructionKind::Not1 { target, bit } => emit_v30_bit(0x16, target, *bit),
        InstructionKind::Brkem { vector } => Ok(vec![0x0F, 0xFF, *vector]),

        InstructionKind::Unknown { raw } => Ok(raw.clone()),
        _ => Err(EmitError::Invalid),
    }
}

/// V30 0x0F 그룹의 byte-form r/m 명령어 인코더.
/// `secondary`: 2번째 opcode byte (예: 0x28 for ROL4).
/// `rm`: byte operand (Reg8 또는 Mem with size=Byte).
/// `reg_field`: ModR/M reg field. ROL4/ROR4 reserve it as zero.
fn emit_v30_rm8(secondary: u8, rm: &Operand, reg_field: u8) -> Result<Vec<u8>, EmitError> {
    let mut out = vec![0x0F, secondary];
    match rm {
        Operand::Reg8(_) => out.extend(emit_reg_rm(rm, reg_field)?),
        Operand::Mem(m) if m.size == OpSize::Byte => out.extend(emit_mem_rm(m, reg_field)?),
        _ => return Err(EmitError::Invalid),
    }
    Ok(out)
}

fn emit_v30_bit(base: u8, target: &Operand, bit: BitIndex) -> Result<Vec<u8>, EmitError> {
    let word = match target {
        Operand::Reg8(_) => false,
        Operand::Reg16(_) => true,
        Operand::Mem(memory) => match memory.size {
            OpSize::Byte => false,
            OpSize::Word => true,
        },
        _ => return Err(EmitError::Invalid),
    };
    let immediate = matches!(bit, BitIndex::Immediate(_));
    let mut out = vec![
        0x0F,
        base | u8::from(word) | if immediate { 0x08 } else { 0 },
    ];
    match target {
        Operand::Reg8(_) | Operand::Reg16(_) => out.extend(emit_reg_rm(target, 0)?),
        Operand::Mem(memory) => out.extend(emit_mem_rm(memory, 0)?),
        _ => unreachable!("target kind was checked above"),
    }
    if let BitIndex::Immediate(value) = bit {
        let maximum = if word { 15 } else { 7 };
        if value > maximum {
            return Err(EmitError::Invalid);
        }
        out.push(value);
    }
    Ok(out)
}

fn emit_imul_immediate(
    dest: Reg16,
    src: &Operand,
    immediate: SignedImmediate,
) -> Result<Vec<u8>, EmitError> {
    let mut out = vec![match immediate {
        SignedImmediate::Byte(_) => 0x6B,
        SignedImmediate::Word(_) => 0x69,
    }];
    match src {
        Operand::Reg16(_) => out.extend(emit_reg_rm(src, dest.0)?),
        Operand::Mem(memory) if memory.size == OpSize::Word => {
            out.extend(emit_mem_rm(memory, dest.0)?)
        }
        _ => return Err(EmitError::Invalid),
    }
    match immediate {
        SignedImmediate::Byte(value) => out.push(value as u8),
        SignedImmediate::Word(value) => out.extend(value.to_le_bytes()),
    }
    Ok(out)
}

fn emit_external_operation(
    opcode_base: u8,
    operation: u8,
    operand: &FloatingPointOperand,
) -> Result<Vec<u8>, EmitError> {
    let mut out = vec![opcode_base + (operation >> 3)];
    let function = operation & 7;
    match operand {
        FloatingPointOperand::Register(register) => {
            out.push(modrm_byte(0b11, function, register.index()));
        }
        FloatingPointOperand::Memory(memory) => {
            out.extend(emit_mem_rm(memory, function)?);
        }
    }
    Ok(out)
}

fn require_repeatable(instruction: &InstructionKind) -> Result<(), EmitError> {
    if matches!(
        instruction,
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
    ) {
        Ok(())
    } else {
        Err(EmitError::Invalid)
    }
}

fn require_comparison_repeatable(instruction: &InstructionKind) -> Result<(), EmitError> {
    if matches!(
        instruction,
        InstructionKind::Cmpsb
            | InstructionKind::Cmpsw
            | InstructionKind::Scasb
            | InstructionKind::Scasw
    ) {
        Ok(())
    } else {
        Err(EmitError::Invalid)
    }
}

fn has_implicit_segment_source(instruction: &InstructionKind) -> bool {
    match instruction {
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

/// ModR/M 인코더: mod, reg, r/m을 1바이트로.
fn modrm_byte(m: u8, reg: u8, rm: u8) -> u8 {
    ((m & 0b11) << 6) | ((reg & 0b111) << 3) | (rm & 0b111)
}

#[derive(Clone, Copy)]
enum MemMode {
    Mod00Disp16,
    ZeroOrDisp,
    ForceDisp,
}

/// 메모리 Operand → ModR/M r/m 필드 + displacement 바이트들.
/// `reg_field`는 호출자가 지정.
///
/// **반환값에는 segment override prefix가 포함되지 않는다.** prefix는 명령어 opcode 바이트
/// **앞에** 와야 하는데, 본 헬퍼는 ModR/M+disp만 다루므로 opcode를 알 수 없다. 따라서
/// 공개 진입점 `emit`이 instruction의 segment를 먼저 제거하고 opcode 앞에 prefix를
/// 붙이므로 이 helper는 ModR/M과 displacement만 책임진다.
fn emit_mem_rm(mem: &Mem, reg_field: u8) -> Result<Vec<u8>, EmitError> {
    let (rm, mode_disp) = match mem.base {
        MemBase::BxSi => (0b000, MemMode::ZeroOrDisp),
        MemBase::BxDi => (0b001, MemMode::ZeroOrDisp),
        MemBase::BpSi => (0b010, MemMode::ZeroOrDisp),
        MemBase::BpDi => (0b011, MemMode::ZeroOrDisp),
        MemBase::Si => (0b100, MemMode::ZeroOrDisp),
        MemBase::Di => (0b101, MemMode::ZeroOrDisp),
        MemBase::Bp => (0b110, MemMode::ForceDisp),
        MemBase::Bx => (0b111, MemMode::ZeroOrDisp),
        MemBase::Direct => (0b110, MemMode::Mod00Disp16),
    };

    let (mode, disp_bytes) = match mode_disp {
        MemMode::Mod00Disp16 => (0b00u8, mem.disp.to_le_bytes().to_vec()),
        MemMode::ZeroOrDisp => {
            if mem.disp == 0 {
                (0b00, vec![])
            } else if (-128..=127).contains(&(mem.disp as i32)) {
                (0b01, vec![mem.disp as i8 as u8])
            } else {
                (0b10, mem.disp.to_le_bytes().to_vec())
            }
        }
        MemMode::ForceDisp => {
            if (-128..=127).contains(&(mem.disp as i32)) {
                (0b01, vec![mem.disp as i8 as u8])
            } else {
                (0b10, mem.disp.to_le_bytes().to_vec())
            }
        }
    };

    let mut out = vec![modrm_byte(mode, reg_field, rm)];
    out.extend_from_slice(&disp_bytes);
    Ok(out)
}

/// segment override prefix 바이트 매핑.
fn seg_override_prefix(s: Sreg) -> u8 {
    match s {
        Sreg::ES => 0x26,
        Sreg::CS => 0x2E,
        Sreg::SS => 0x36,
        Sreg::DS => 0x3E,
    }
}

/// 메모리 operand에 segment override가 있으면 (Sreg, seg가 제거된 kind clone)을 반환.
/// 없으면 `None`. 호출자는 prefix를 결과 맨 앞에 prepend하고 stripped kind로 emit.
///
/// 메모리 operand 여러 개가 있는 명령(예: 일부 ALU r/m, r/m? — 8086은 두 메모리 operand 동시
/// 사용 인코딩이 없음. 여기선 모든 메모리 operand에 동일한 seg을 적용 후 제거.)
fn strip_seg_override(kind: &InstructionKind) -> Option<(Sreg, InstructionKind)> {
    let seg = find_mem_seg(kind)?;
    let stripped = strip_mem_seg(kind);
    Some((seg, stripped))
}

/// 명령어의 메모리 operand에서 seg를 찾는다 (첫 번째 매치).
///
/// **신규 메모리-바운드 InstructionKind variant 추가 시 세 곳 모두 갱신 필요**:
/// `decode.rs::inject_seg_override`, `emit.rs::find_mem_seg`, `emit.rs::strip_mem_seg`.
fn find_mem_seg(kind: &InstructionKind) -> Option<Sreg> {
    let from_op = |op: &Operand| -> Option<Sreg> {
        if let Operand::Mem(m) = op {
            m.seg
        } else {
            None
        }
    };
    match kind {
        InstructionKind::Mov { dest, src } => from_op(dest).or_else(|| from_op(src)),
        InstructionKind::Lea { src, .. } => src.seg,
        InstructionKind::Les { src, .. } | InstructionKind::Lds { src, .. } => src.seg,
        InstructionKind::Add { dest, src }
        | InstructionKind::Adc { dest, src }
        | InstructionKind::Sbb { dest, src }
        | InstructionKind::Sub { dest, src }
        | InstructionKind::And { dest, src }
        | InstructionKind::Or { dest, src }
        | InstructionKind::Xor { dest, src } => from_op(dest).or_else(|| from_op(src)),
        InstructionKind::Cmp { a, b }
        | InstructionKind::Test { a, b }
        | InstructionKind::Xchg { a, b } => from_op(a).or_else(|| from_op(b)),
        InstructionKind::Push { src } => from_op(src),
        InstructionKind::Bound { bounds, .. } => bounds.seg,
        InstructionKind::Pop { dest }
        | InstructionKind::Inc { dest }
        | InstructionKind::Dec { dest }
        | InstructionKind::Neg { dest }
        | InstructionKind::Not { dest } => from_op(dest),
        InstructionKind::Mul { src }
        | InstructionKind::Imul { src }
        | InstructionKind::Div { src }
        | InstructionKind::Idiv { src } => from_op(src),
        InstructionKind::ImulImmediate { src, .. } => from_op(src),
        InstructionKind::Rol { dest, .. }
        | InstructionKind::Ror { dest, .. }
        | InstructionKind::Rcl { dest, .. }
        | InstructionKind::Rcr { dest, .. }
        | InstructionKind::Shl { dest, .. }
        | InstructionKind::Shr { dest, .. }
        | InstructionKind::Sar { dest, .. } => from_op(dest),
        // V30 0x0F 확장 — 메모리 operand 가능
        InstructionKind::Rol4 { rm } | InstructionKind::Ror4 { rm } => from_op(rm),
        InstructionKind::Test1 { target, .. }
        | InstructionKind::Clr1 { target, .. }
        | InstructionKind::Set1 { target, .. }
        | InstructionKind::Not1 { target, .. } => from_op(target),
        InstructionKind::Esc {
            operand: FloatingPointOperand::Memory(memory),
            ..
        }
        | InstructionKind::Fpo2 {
            operand: FloatingPointOperand::Memory(memory),
            ..
        } => memory.seg,
        InstructionKind::Jmp {
            target: JmpTarget::IndirectNear(operand),
        }
        | InstructionKind::Call {
            target: CallTarget::IndirectNear(operand),
        } => from_op(operand),
        InstructionKind::Jmp {
            target: JmpTarget::IndirectFar(memory),
        }
        | InstructionKind::Call {
            target: CallTarget::IndirectFar(memory),
        } => memory.seg,
        _ => None,
    }
}

/// 명령어의 모든 메모리 operand에서 seg를 None으로 만든 clone 반환.
///
/// **신규 메모리-바운드 InstructionKind variant 추가 시 세 곳 모두 갱신 필요**:
/// `decode.rs::inject_seg_override`, `emit.rs::find_mem_seg`, `emit.rs::strip_mem_seg`.
fn strip_mem_seg(kind: &InstructionKind) -> InstructionKind {
    let strip = |op: &Operand| -> Operand {
        match op {
            Operand::Mem(m) => {
                let mut nm = *m;
                nm.seg = None;
                Operand::Mem(nm)
            }
            other => other.clone(),
        }
    };
    match kind {
        InstructionKind::Mov { dest, src } => InstructionKind::Mov {
            dest: strip(dest),
            src: strip(src),
        },
        InstructionKind::Lea { dest, src } => {
            let mut ns = *src;
            ns.seg = None;
            InstructionKind::Lea {
                dest: *dest,
                src: ns,
            }
        }
        InstructionKind::Les { dest, src } => {
            let mut src = *src;
            src.seg = None;
            InstructionKind::Les { dest: *dest, src }
        }
        InstructionKind::Lds { dest, src } => {
            let mut src = *src;
            src.seg = None;
            InstructionKind::Lds { dest: *dest, src }
        }
        InstructionKind::Add { dest, src } => InstructionKind::Add {
            dest: strip(dest),
            src: strip(src),
        },
        InstructionKind::Adc { dest, src } => InstructionKind::Adc {
            dest: strip(dest),
            src: strip(src),
        },
        InstructionKind::Sbb { dest, src } => InstructionKind::Sbb {
            dest: strip(dest),
            src: strip(src),
        },
        InstructionKind::Sub { dest, src } => InstructionKind::Sub {
            dest: strip(dest),
            src: strip(src),
        },
        InstructionKind::And { dest, src } => InstructionKind::And {
            dest: strip(dest),
            src: strip(src),
        },
        InstructionKind::Or { dest, src } => InstructionKind::Or {
            dest: strip(dest),
            src: strip(src),
        },
        InstructionKind::Xor { dest, src } => InstructionKind::Xor {
            dest: strip(dest),
            src: strip(src),
        },
        InstructionKind::Cmp { a, b } => InstructionKind::Cmp {
            a: strip(a),
            b: strip(b),
        },
        InstructionKind::Test { a, b } => InstructionKind::Test {
            a: strip(a),
            b: strip(b),
        },
        InstructionKind::Xchg { a, b } => InstructionKind::Xchg {
            a: strip(a),
            b: strip(b),
        },
        InstructionKind::Push { src } => InstructionKind::Push { src: strip(src) },
        InstructionKind::Bound { index, bounds } => {
            let mut bounds = *bounds;
            bounds.seg = None;
            InstructionKind::Bound {
                index: *index,
                bounds,
            }
        }
        InstructionKind::Pop { dest } => InstructionKind::Pop { dest: strip(dest) },
        InstructionKind::Inc { dest } => InstructionKind::Inc { dest: strip(dest) },
        InstructionKind::Dec { dest } => InstructionKind::Dec { dest: strip(dest) },
        InstructionKind::Neg { dest } => InstructionKind::Neg { dest: strip(dest) },
        InstructionKind::Not { dest } => InstructionKind::Not { dest: strip(dest) },
        InstructionKind::Mul { src } => InstructionKind::Mul { src: strip(src) },
        InstructionKind::Imul { src } => InstructionKind::Imul { src: strip(src) },
        InstructionKind::ImulImmediate {
            dest,
            src,
            immediate,
        } => InstructionKind::ImulImmediate {
            dest: *dest,
            src: strip(src),
            immediate: *immediate,
        },
        InstructionKind::Div { src } => InstructionKind::Div { src: strip(src) },
        InstructionKind::Idiv { src } => InstructionKind::Idiv { src: strip(src) },
        InstructionKind::Rol { dest, count } => InstructionKind::Rol {
            dest: strip(dest),
            count: *count,
        },
        InstructionKind::Ror { dest, count } => InstructionKind::Ror {
            dest: strip(dest),
            count: *count,
        },
        InstructionKind::Rcl { dest, count } => InstructionKind::Rcl {
            dest: strip(dest),
            count: *count,
        },
        InstructionKind::Rcr { dest, count } => InstructionKind::Rcr {
            dest: strip(dest),
            count: *count,
        },
        InstructionKind::Shl { dest, count } => InstructionKind::Shl {
            dest: strip(dest),
            count: *count,
        },
        InstructionKind::Shr { dest, count } => InstructionKind::Shr {
            dest: strip(dest),
            count: *count,
        },
        InstructionKind::Sar { dest, count } => InstructionKind::Sar {
            dest: strip(dest),
            count: *count,
        },
        // V30 0x0F 확장 — 메모리 operand에서 seg 제거
        InstructionKind::Rol4 { rm } => InstructionKind::Rol4 { rm: strip(rm) },
        InstructionKind::Ror4 { rm } => InstructionKind::Ror4 { rm: strip(rm) },
        InstructionKind::Test1 { target, bit } => InstructionKind::Test1 {
            target: strip(target),
            bit: *bit,
        },
        InstructionKind::Clr1 { target, bit } => InstructionKind::Clr1 {
            target: strip(target),
            bit: *bit,
        },
        InstructionKind::Set1 { target, bit } => InstructionKind::Set1 {
            target: strip(target),
            bit: *bit,
        },
        InstructionKind::Not1 { target, bit } => InstructionKind::Not1 {
            target: strip(target),
            bit: *bit,
        },
        InstructionKind::Esc { operation, operand } => InstructionKind::Esc {
            operation: *operation,
            operand: strip_floating_point_operand(operand),
        },
        InstructionKind::Fpo2 { operation, operand } => InstructionKind::Fpo2 {
            operation: *operation,
            operand: strip_floating_point_operand(operand),
        },
        InstructionKind::Jmp {
            target: JmpTarget::IndirectNear(operand),
        } => InstructionKind::Jmp {
            target: JmpTarget::IndirectNear(strip(operand)),
        },
        InstructionKind::Jmp {
            target: JmpTarget::IndirectFar(memory),
        } => {
            let mut memory = *memory;
            memory.seg = None;
            InstructionKind::Jmp {
                target: JmpTarget::IndirectFar(memory),
            }
        }
        InstructionKind::Call {
            target: CallTarget::IndirectNear(operand),
        } => InstructionKind::Call {
            target: CallTarget::IndirectNear(strip(operand)),
        },
        InstructionKind::Call {
            target: CallTarget::IndirectFar(memory),
        } => {
            let mut memory = *memory;
            memory.seg = None;
            InstructionKind::Call {
                target: CallTarget::IndirectFar(memory),
            }
        }
        other => other.clone(),
    }
}

fn strip_floating_point_operand(operand: &FloatingPointOperand) -> FloatingPointOperand {
    match operand {
        FloatingPointOperand::Register(register) => FloatingPointOperand::Register(*register),
        FloatingPointOperand::Memory(memory) => {
            let mut memory = *memory;
            memory.seg = None;
            FloatingPointOperand::Memory(memory)
        }
    }
}

/// 레지스터 Operand → ModR/M r/m 필드 (mod=11 모드).
fn emit_reg_rm(op: &Operand, reg_field: u8) -> Result<Vec<u8>, EmitError> {
    let rm = match op {
        Operand::Reg8(r) => r.0,
        Operand::Reg16(r) => r.0,
        _ => return Err(EmitError::Invalid),
    };
    Ok(vec![modrm_byte(0b11, reg_field, rm)])
}

/// MOV 인코딩 — 변종 우선순위:
/// 1. acc(AL/AX) ↔ [moffs16] → 0xA0-0xA3 (가장 짧음)
/// 2. r8 ← imm8 / r16 ← imm16 → 0xB0+rb ib / 0xB8+rw iw
/// 3. r/m8 ← imm8 / r/m16 ← imm16 → 0xC6 /0 ib / 0xC7 /0 iw
/// 4. sreg ↔ r/m16 → 0x8C / 0x8E
/// 5. r ↔ r/m → 0x88-0x8B
fn emit_mov(dest: &Operand, src: &Operand) -> Result<Vec<u8>, EmitError> {
    // 1. AL/AX ↔ [moffs16] (Disp16) — segment override 있으면 prefix prepend.
    match (dest, src) {
        (Operand::Reg8(Reg8(0)), Operand::Mem(m)) if m.base == MemBase::Direct => {
            return emit_mov_acc_moffs(0xA0, m);
        }
        (Operand::Reg16(Reg16(0)), Operand::Mem(m)) if m.base == MemBase::Direct => {
            return emit_mov_acc_moffs(0xA1, m);
        }
        (Operand::Mem(m), Operand::Reg8(Reg8(0))) if m.base == MemBase::Direct => {
            return emit_mov_acc_moffs(0xA2, m);
        }
        (Operand::Mem(m), Operand::Reg16(Reg16(0))) if m.base == MemBase::Direct => {
            return emit_mov_acc_moffs(0xA3, m);
        }
        _ => {}
    }

    // 2. r ← imm 단축형
    match (dest, src) {
        (Operand::Reg8(r), Operand::Imm8(v)) => {
            return Ok(vec![0xB0 + r.0, *v]);
        }
        (Operand::Reg16(r), Operand::Imm16(v)) => {
            let b = v.to_le_bytes();
            return Ok(vec![0xB8 + r.0, b[0], b[1]]);
        }
        _ => {}
    }

    // 3. r/m ← imm (메모리 또는 비-축약 레지스터)
    match (dest, src) {
        (Operand::Mem(m), Operand::Imm8(v)) => {
            let mut out = vec![0xC6];
            out.extend(emit_mem_rm(m, 0)?);
            out.push(*v);
            return Ok(out);
        }
        (Operand::Mem(m), Operand::Imm16(v)) => {
            let mut out = vec![0xC7];
            out.extend(emit_mem_rm(m, 0)?);
            out.extend_from_slice(&v.to_le_bytes());
            return Ok(out);
        }
        _ => {}
    }

    // 4. Sreg ↔ r/m16
    match (dest, src) {
        (Operand::Sreg(s), src_op) => {
            // MOV sreg, r/m16 → 0x8E /reg
            if *s == Sreg::CS {
                return Err(EmitError::Invalid);
            }
            let mut out = vec![0x8E];
            match src_op {
                Operand::Reg16(r) => out.extend(emit_reg_rm(&Operand::Reg16(*r), s.to_index())?),
                Operand::Mem(m) => out.extend(emit_mem_rm(m, s.to_index())?),
                _ => return Err(EmitError::Invalid),
            }
            return Ok(out);
        }
        (dest_op, Operand::Sreg(s)) => {
            // MOV r/m16, sreg → 0x8C /reg
            let mut out = vec![0x8C];
            match dest_op {
                Operand::Reg16(r) => out.extend(emit_reg_rm(&Operand::Reg16(*r), s.to_index())?),
                Operand::Mem(m) => out.extend(emit_mem_rm(m, s.to_index())?),
                _ => return Err(EmitError::Invalid),
            }
            return Ok(out);
        }
        _ => {}
    }

    // 5. r ↔ r/m (0x88-0x8B)
    // src가 reg면 0x88/0x89 (r/m ← r) — reg-reg일 때 우선 (Intel syntax 자연스러움)
    // dest가 reg(메모리 src)면 0x8A/0x8B (r ← r/m)
    let (opcode, reg_field, rm_op) = match (dest, src) {
        (other, Operand::Reg8(r)) => (0x88, r.0, other),
        (other, Operand::Reg16(r)) => (0x89, r.0, other),
        (Operand::Reg8(r), other) => (0x8A, r.0, other),
        (Operand::Reg16(r), other) => (0x8B, r.0, other),
        _ => return Err(EmitError::Invalid),
    };

    let mut out = vec![opcode];
    match rm_op {
        Operand::Reg8(_) | Operand::Reg16(_) => out.extend(emit_reg_rm(rm_op, reg_field)?),
        Operand::Mem(m) => out.extend(emit_mem_rm(m, reg_field)?),
        _ => return Err(EmitError::Invalid),
    }
    Ok(out)
}

/// ALU op enum — `decode.rs::AluOp`의 emit-side 미러.
/// `group1_reg`로 Group 1 reg field 인덱스, `base_opcode`로 직접 인코딩 base 결정.
#[derive(Clone, Copy)]
enum EmitAluOp {
    Add,
    Or,
    Adc,
    Sbb,
    And,
    Sub,
    Xor,
    Cmp,
}

impl EmitAluOp {
    fn group1_reg(self) -> u8 {
        match self {
            EmitAluOp::Add => 0,
            EmitAluOp::Or => 1,
            EmitAluOp::Adc => 2,
            EmitAluOp::Sbb => 3,
            EmitAluOp::And => 4,
            EmitAluOp::Sub => 5,
            EmitAluOp::Xor => 6,
            EmitAluOp::Cmp => 7,
        }
    }
    fn base_opcode(self) -> u8 {
        // 직접 인코딩의 base — Group 1 reg index << 3.
        // ADD=0x00, OR=0x08, AND=0x20, SUB=0x28, XOR=0x30, CMP=0x38
        self.group1_reg() << 3
    }
}

/// ALU 인코딩 — 변종 우선순위 (canonical 정책):
/// 1. AL/AX ↔ imm 단축형 (`base+4`, `base+5`)
/// 2. r/m ← imm Group 1: imm이 sign-ext 가능하면 0x83(2바이트), 아니면 0x81(3바이트)
///    또는 imm8 → 0x80 (byte form)
/// 3. r ↔ r/m 직접 인코딩: src=Reg면 r/m ← r form (`base+0/1`) 우선,
///    그 외 dest=Reg면 r ← r/m form (`base+2/3`).
fn emit_alu(op: EmitAluOp, dest: &Operand, src: &Operand) -> Result<Vec<u8>, EmitError> {
    let base = op.base_opcode();

    // 1. AL/AX ↔ imm 단축형 — 가장 짧음
    match (dest, src) {
        (Operand::Reg8(Reg8(0)), Operand::Imm8(v)) => {
            return Ok(vec![base + 4, *v]);
        }
        (Operand::Reg16(Reg16(0)), Operand::Imm16(v)) => {
            let b = v.to_le_bytes();
            return Ok(vec![base + 5, b[0], b[1]]);
        }
        _ => {}
    }

    // 2. r/m ← imm — Group 1 (0x80/0x81/0x83)
    match (dest, src) {
        (rm_op, Operand::Imm8(v)) => {
            // byte form: 0x80 /reg ib
            let mut out = vec![0x80];
            match rm_op {
                Operand::Reg8(_) => out.extend(emit_reg_rm(rm_op, op.group1_reg())?),
                Operand::Mem(m) if m.size == OpSize::Byte => {
                    out.extend(emit_mem_rm(m, op.group1_reg())?)
                }
                _ => return Err(EmitError::Invalid),
            }
            out.push(*v);
            return Ok(out);
        }
        (rm_op, Operand::Imm16(v)) => {
            // word form: sign-extendable이면 0x83 (2바이트 inst), 아니면 0x81 (3바이트 inst)
            let signed = *v as i16;
            let opcode = if (-128..=127).contains(&(signed as i32)) {
                0x83
            } else {
                0x81
            };
            let mut out = vec![opcode];
            match rm_op {
                Operand::Reg16(_) => out.extend(emit_reg_rm(rm_op, op.group1_reg())?),
                Operand::Mem(m) if m.size == OpSize::Word => {
                    out.extend(emit_mem_rm(m, op.group1_reg())?)
                }
                _ => return Err(EmitError::Invalid),
            }
            if opcode == 0x83 {
                out.push((signed as i8) as u8);
            } else {
                out.extend_from_slice(&v.to_le_bytes());
            }
            return Ok(out);
        }
        _ => {}
    }

    // 3. r ↔ r/m 직접 인코딩 — canonical: src=Reg → `base+0/1` (r/m ← r form) 우선
    let (opcode, reg_field, rm_op) = match (dest, src) {
        (other, Operand::Reg8(r)) => (base, r.0, other),
        (other, Operand::Reg16(r)) => (base + 1, r.0, other),
        (Operand::Reg8(r), other) => (base + 2, r.0, other),
        (Operand::Reg16(r), other) => (base + 3, r.0, other),
        _ => return Err(EmitError::Invalid),
    };
    let mut out = vec![opcode];
    match rm_op {
        Operand::Reg8(_) | Operand::Reg16(_) => out.extend(emit_reg_rm(rm_op, reg_field)?),
        Operand::Mem(m) => out.extend(emit_mem_rm(m, reg_field)?),
        _ => return Err(EmitError::Invalid),
    }
    Ok(out)
}

/// TEST 인코딩 — ALU와 별도 인코딩.
/// 우선순위:
/// 1. AL/AX ↔ imm 단축형 (0xA8/0xA9)
/// 2. r/m ↔ imm (Group 3 0xF6/0xF7 /0) — sign-extension 없음 (0x83 같은 form 없음)
/// 3. r ↔ r/m 직접 (0x84/0x85)
fn emit_test(a: &Operand, b: &Operand) -> Result<Vec<u8>, EmitError> {
    // 1. AL/AX ↔ imm 단축형 (0xA8/0xA9)
    match (a, b) {
        (Operand::Reg8(Reg8(0)), Operand::Imm8(v)) => return Ok(vec![0xA8, *v]),
        (Operand::Reg16(Reg16(0)), Operand::Imm16(v)) => {
            let bytes = v.to_le_bytes();
            return Ok(vec![0xA9, bytes[0], bytes[1]]);
        }
        _ => {}
    }
    // 2. r/m ← imm (0xF6 /0 / 0xF7 /0) — sign-extension 없음
    match (a, b) {
        (rm_op, Operand::Imm8(v)) => {
            let mut out = vec![0xF6];
            match rm_op {
                Operand::Reg8(_) => out.extend(emit_reg_rm(rm_op, 0)?),
                Operand::Mem(m) if m.size == OpSize::Byte => out.extend(emit_mem_rm(m, 0)?),
                _ => return Err(EmitError::Invalid),
            }
            out.push(*v);
            return Ok(out);
        }
        (rm_op, Operand::Imm16(v)) => {
            let mut out = vec![0xF7];
            match rm_op {
                Operand::Reg16(_) => out.extend(emit_reg_rm(rm_op, 0)?),
                Operand::Mem(m) if m.size == OpSize::Word => out.extend(emit_mem_rm(m, 0)?),
                _ => return Err(EmitError::Invalid),
            }
            out.extend_from_slice(&v.to_le_bytes());
            return Ok(out);
        }
        _ => {}
    }
    // 3. r/m ↔ r (0x84/0x85) — TEST는 가환적이라 a/b 어느 쪽이 reg여도 OK.
    //    canonical: b=Reg면 r/m ↔ r form, 아니면 a=Reg.
    let (opcode, reg_field, rm_op) = match (a, b) {
        (other, Operand::Reg8(r)) => (0x84, r.0, other),
        (other, Operand::Reg16(r)) => (0x85, r.0, other),
        (Operand::Reg8(r), other) => (0x84, r.0, other),
        (Operand::Reg16(r), other) => (0x85, r.0, other),
        _ => return Err(EmitError::Invalid),
    };
    let mut out = vec![opcode];
    match rm_op {
        Operand::Reg8(_) | Operand::Reg16(_) => out.extend(emit_reg_rm(rm_op, reg_field)?),
        Operand::Mem(m) => out.extend(emit_mem_rm(m, reg_field)?),
        _ => return Err(EmitError::Invalid),
    }
    Ok(out)
}

/// INC/DEC r/m 인코딩 — Group 4 (0xFE) 또는 Group 5 (0xFF) + reg field로 구분.
/// `reg_field`: 0=INC, 1=DEC.
/// reg16 단축형 (0x40+r / 0x48+r)은 `emit` match에서 우선 처리되므로 본 함수는
/// Reg8 / Mem만 다룬다.
fn emit_inc_dec(dest: &Operand, reg_field: u8) -> Result<Vec<u8>, EmitError> {
    let opcode = match dest {
        Operand::Reg8(_) => 0xFE,
        Operand::Mem(m) if m.size == OpSize::Byte => 0xFE,
        Operand::Mem(m) if m.size == OpSize::Word => 0xFF,
        // Reg16은 단축형 arm에서 처리됨 — 여기 도달하면 invalid
        _ => return Err(EmitError::Invalid),
    };
    let mut out = vec![opcode];
    match dest {
        Operand::Reg8(_) => out.extend(emit_reg_rm(dest, reg_field)?),
        Operand::Mem(m) => out.extend(emit_mem_rm(m, reg_field)?),
        _ => return Err(EmitError::Invalid),
    }
    Ok(out)
}

/// Group 3 unary 인코딩 — 0xF6/0xF7 + reg field.
fn emit_group_3_unary(operand: &Operand, reg_field: u8) -> Result<Vec<u8>, EmitError> {
    let opcode = match operand {
        Operand::Reg8(_) => 0xF6,
        Operand::Reg16(_) => 0xF7,
        Operand::Mem(m) if m.size == OpSize::Byte => 0xF6,
        Operand::Mem(m) if m.size == OpSize::Word => 0xF7,
        _ => return Err(EmitError::Invalid),
    };
    let mut out = vec![opcode];
    match operand {
        Operand::Reg8(_) | Operand::Reg16(_) => out.extend(emit_reg_rm(operand, reg_field)?),
        Operand::Mem(m) => out.extend(emit_mem_rm(m, reg_field)?),
        _ => return Err(EmitError::Invalid),
    }
    Ok(out)
}

/// 시프트/로테이트 (Group 2 0xD0-0xD3 + reg field) 인코더.
/// `reg_field`: ROL=0, ROR=1, SHL/SAL=4, SHR=5, SAR=7.
/// `count`가 `One`이면 0xD0/0xD1, `Cl`이면 0xD2/0xD3. byte/word는 dest로 결정.
/// `count=Imm8(_)`은 V30/186+의 0xC0/0xC1 selector를 사용한다.
fn emit_shift(reg_field: u8, dest: &Operand, count: ShiftCount) -> Result<Vec<u8>, EmitError> {
    let base = match count {
        ShiftCount::One => 0xD0u8,
        ShiftCount::Cl => 0xD2u8,
        ShiftCount::Imm8(_) => 0xC0u8,
    };
    // bit 0 = W
    let opcode = match dest {
        Operand::Reg8(_) => base,
        Operand::Reg16(_) => base | 1,
        Operand::Mem(m) if m.size == OpSize::Byte => base,
        Operand::Mem(m) if m.size == OpSize::Word => base | 1,
        _ => return Err(EmitError::Invalid),
    };
    let mut out = vec![opcode];
    match dest {
        Operand::Reg8(_) | Operand::Reg16(_) => out.extend(emit_reg_rm(dest, reg_field)?),
        Operand::Mem(m) => out.extend(emit_mem_rm(m, reg_field)?),
        _ => return Err(EmitError::Invalid),
    }
    if let ShiftCount::Imm8(count) = count {
        out.push(count);
    }
    Ok(out)
}

/// `MOV AL/AX ↔ [moffs16]` 단축형 (0xA0/0xA1/0xA2/0xA3) 인코더.
/// 본 헬퍼는 ModR/M byte 없이 직접 disp16을 인코딩하는 단축형. segment override는 emit
/// 진입부 (`emit`)에서 일괄 stripping/prepend되므로 본 헬퍼는 prefix를 다루지 않는다.
fn emit_mov_acc_moffs(opcode: u8, mem: &Mem) -> Result<Vec<u8>, EmitError> {
    let mut out = vec![opcode];
    out.extend_from_slice(&(mem.disp as u16).to_le_bytes());
    Ok(out)
}

/// XCHG 인코더 — canonical: `XCHG AX, r16` 단축형 (0x90+rw) 우선 (a=AX 형태).
/// b=AX 형태는 일반 형식으로 emit (operand 순서 의미 보존). 그 외는 0x86/0x87 /r.
fn emit_xchg(a: &Operand, b: &Operand) -> Result<Vec<u8>, EmitError> {
    // 1. `XCHG AX, r16` 단축형 (0x90+rw) — a=AX이고 b가 다른 16비트 GPR.
    //    `XCHG r16, AX` (b=AX)은 디코더가 r/m, r form (0x87 /r)으로 emit하도록 일반 형식.
    if let (Operand::Reg16(Reg16(0)), Operand::Reg16(other)) = (a, b)
        && other.0 != 0
    {
        return Ok(vec![0x90 + other.0]);
    }

    // 2. 일반 형식 0x86 (byte) / 0x87 (word) /r — 한 쪽이 reg여야 r/m ↔ r form 가능.
    let (opcode, reg_field, rm_op) = match (a, b) {
        (other, Operand::Reg8(r)) => (0x86, r.0, other),
        (other, Operand::Reg16(r)) => (0x87, r.0, other),
        (Operand::Reg8(r), other) => (0x86, r.0, other),
        (Operand::Reg16(r), other) => (0x87, r.0, other),
        _ => return Err(EmitError::Invalid),
    };
    let mut out = vec![opcode];
    match rm_op {
        Operand::Reg8(_) | Operand::Reg16(_) => out.extend(emit_reg_rm(rm_op, reg_field)?),
        Operand::Mem(m) => out.extend(emit_mem_rm(m, reg_field)?),
        _ => return Err(EmitError::Invalid),
    }
    Ok(out)
}
