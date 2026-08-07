//! x86 encoder: `InstructionKind` to bytes.
//!
//! Internal emission of `Unknown { raw }` exists only for import regression tests. Public
//! `encode_bytes` and the checked assembler reject executable code containing `Unknown`,
//! including when it is nested in a wrapper.
//!
//! ## Canonical policy (spec sections 1.1 and 4.4.1)
//!
//! The 8086 ISA permits redundant byte encodings of the same instruction. For example,
//! `MOV BX, AX` may be `0x89 0xC3` or `0x8B 0xD8`. This encoder always emits one
//! **canonical form** for deterministic output:
//!
//! 1. **Reg-reg MOV**: `0x88/0x89` (r/m <- r, dir=0). `0x8A/0x8B` (r <- r/m)
//!    remains decode-only for mod=11; both mean the same thing, but emission uses dir=0.
//! 2. **Memory disp=0** (except BP): `mod=00` with no displacement.
//! 3. **Memory based on BP or disp in [-128..=127]**: `mod=01 disp8`.
//! 4. **Other memory displacements**: `mod=10 disp16`.
//! 5. **POP r16**: short form `0x58+r`, even after decoding Group form `0x8F /0 reg`.
//! 6. **PUSH/POP sreg**: short forms `0x06/0E/16/1E` and `0x07/17/1F`.
//! 7. **r/m=AL/AX moffs MOV**: short form `0xA0-0xA3` (Disp16 plus implicit segment).
//! 8. **r <- imm MOV**: short form `0xB0+rb` / `0xB8+rw`, ahead of `0xC6/0xC7 /0`.
//!
//! Thus `decode(emit(kind)) == kind` always holds, while `emit(decode(b)) == b` holds only
//! when `b` is already canonical. Raw source bytes are preserved by `Instruction.bytes` or
//! the surrounding container rather than by the semantic ISA layer.
//!
//! ## Exact source and canonical generation
//!
//! Public decode results preserve legal aliases and repeated prefixes exactly in
//! `DecodedInstruction::source_bytes()` and `PrefixSequence`. A typed `Instruction` carries
//! only the meaning of the last effective prefix, and this module emits one canonical order
//! and encoding. Expected Write re-decodes and reassembles these canonical executable bytes.
//!
//! - **Repeated prefixes in one category**: source order and repetition survive decoding;
//!   the typed instruction applies the last prefix. New code never emits duplicates.
//! - **Asymmetric canonical XCHG**: `Xchg { a: AX, b: r }` uses short form `0x90+r`, while
//!   `Xchg { a: r, b: AX }` uses general form `0x87 /r`. The IR preserves operand order, so
//!   the bytes differ while meaning remains equivalent. Both preserve the canonical
//!   `decode(emit(kind)) == kind` fixed point.

use crate::instruction::*;

#[derive(Debug, thiserror::Error)]
/// Failure returned when an internal typed instruction has no executable encoding.
pub enum EmitError {
    #[error("invalid encoding for kind")]
    /// The invalid form.
    Invalid,
}

/// Encodes one internal typed instruction as executable bytes.
pub fn emit(kind: &InstructionKind) -> Result<Vec<u8>, EmitError> {
    // If a memory operand has a segment override, prepend its prefix and recurse on a clone
    // with the segment removed. Encoding branches therefore do not handle prefixes themselves.
    // REP/REPNE wrappers recurse on their inner instruction and naturally follow this path.
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

        // Flag and sign-extension instructions are all single-byte forms (Intel SDM Vol. 2A, Appendix B).
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

        // Unconditional branches.
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
                // Group 5 0xFF /4 = indirect near JMP r/m16.
                let mut out = vec![0xFF];
                match op {
                    Operand::Reg16(_) => out.extend(emit_reg_rm(op, 4)?),
                    Operand::Mem(m) => out.extend(emit_mem_rm(m, 4)?),
                    _ => return Err(EmitError::Invalid),
                }
                Ok(out)
            }
            JmpTarget::IndirectFar(m) => {
                // Group 5 0xFF /5 = indirect far JMP m16:16; memory only.
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
                // Group 5 0xFF /2 = indirect near CALL r/m16.
                let mut out = vec![0xFF];
                match op {
                    Operand::Reg16(_) => out.extend(emit_reg_rm(op, 2)?),
                    Operand::Mem(m) => out.extend(emit_mem_rm(m, 2)?),
                    _ => return Err(EmitError::Invalid),
                }
                Ok(out)
            }
            CallTarget::IndirectFar(m) => {
                // Group 5 0xFF /3 = indirect far CALL m16:16; memory only.
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

        // Prefer PUSH/POP short forms.
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
            Sreg::CS => return Err(EmitError::Invalid), // POP CS is unsupported.
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

        // MOV forms are selected in canonical priority order.
        InstructionKind::Mov { dest, src } => emit_mov(dest, src),

        // ALU family, consolidated through emit_alu.
        InstructionKind::Add { dest, src } => emit_alu(EmitAluOp::Add, dest, src),
        InstructionKind::Or { dest, src } => emit_alu(EmitAluOp::Or, dest, src),
        InstructionKind::Adc { dest, src } => emit_alu(EmitAluOp::Adc, dest, src),
        InstructionKind::Sbb { dest, src } => emit_alu(EmitAluOp::Sbb, dest, src),
        InstructionKind::And { dest, src } => emit_alu(EmitAluOp::And, dest, src),
        InstructionKind::Sub { dest, src } => emit_alu(EmitAluOp::Sub, dest, src),
        InstructionKind::Xor { dest, src } => emit_alu(EmitAluOp::Xor, dest, src),
        InstructionKind::Cmp { a, b } => emit_alu(EmitAluOp::Cmp, a, b),

        // TEST has distinct encodings from the ALU family (0x84/85, 0xA8/A9, 0xF6/F7 /0).
        InstructionKind::Test { a, b } => emit_test(a, b),

        // Canonical INC/DEC reg16 short forms (0x40+r / 0x48+r).
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

        // Shifts and rotates (Group 2 0xD0-0xD3 plus reg field).
        // NEC and Intel define SAL as the same `/4` selector as SHL, so the typed form uses SHL.
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

        // XCHG prefers the AX, r16 short form (0x90+rw); others use general 0x86/0x87 /r.
        InstructionKind::Xchg { a, b } => emit_xchg(a, b),

        // Single-byte string instructions. REP/REPNE use wrapper variants.
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

        // REP/REPNE wrappers prepend 0xF3/0xF2 to the emitted inner instruction.
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

        // Canonical V30 (NEC uPD70116) 0x0F secondary-opcode extensions.
        // Source: NEC uPD70116 data sheet, section 3.
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

/// Encodes byte-form r/m instructions in the V30 0x0F group.
/// `secondary` is the second opcode byte, such as 0x28 for ROL4.
/// `rm` is a byte operand (`Reg8` or `Mem` with byte size).
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

/// Encodes mod, reg, and r/m into one ModR/M byte.
fn modrm_byte(m: u8, reg: u8, rm: u8) -> u8 {
    ((m & 0b11) << 6) | ((reg & 0b111) << 3) | (rm & 0b111)
}

#[derive(Clone, Copy)]
enum MemMode {
    Mod00Disp16,
    ZeroOrDisp,
    ForceDisp,
}

/// Encodes a memory operand as a ModR/M r/m field plus displacement bytes.
/// The caller supplies `reg_field`.
///
/// **The return value does not include a segment override prefix.** A prefix precedes the
/// opcode, while this helper only sees ModR/M and displacement bytes. The public `emit`
/// entry point strips the segment and places its prefix before the opcode.
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

/// Maps segment overrides to prefix bytes.
fn seg_override_prefix(s: Sreg) -> u8 {
    match s {
        Sreg::ES => 0x26,
        Sreg::CS => 0x2E,
        Sreg::SS => 0x36,
        Sreg::DS => 0x3E,
    }
}

/// Returns the segment register and a cloned kind with segment overrides removed.
/// Returns `None` when no memory operand has an override. The caller prepends the prefix.
///
/// The 8086 has no encoding with two simultaneous memory operands. For completeness, if a
/// kind contains multiple memory operands, this applies and removes the same segment on all.
fn strip_seg_override(kind: &InstructionKind) -> Option<(Sreg, InstructionKind)> {
    let seg = find_mem_seg(kind)?;
    let stripped = strip_mem_seg(kind);
    Some((seg, stripped))
}

/// Finds the first segment override among an instruction's memory operands.
///
/// **When adding a memory-bearing `InstructionKind` variant, update all three sites:**
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
        // V30 0x0F extensions with possible memory operands.
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

/// Clones an instruction while setting every memory operand's segment to `None`.
///
/// **When adding a memory-bearing `InstructionKind` variant, update all three sites:**
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
        // Remove segment overrides from V30 0x0F extension memory operands.
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

/// Encodes a register operand as a ModR/M r/m field with mod=11.
fn emit_reg_rm(op: &Operand, reg_field: u8) -> Result<Vec<u8>, EmitError> {
    let rm = match op {
        Operand::Reg8(r) => r.0,
        Operand::Reg16(r) => r.0,
        _ => return Err(EmitError::Invalid),
    };
    Ok(vec![modrm_byte(0b11, reg_field, rm)])
}

/// Encodes MOV forms in this priority order:
/// 1. acc(AL/AX) <-> [moffs16] -> 0xA0-0xA3 (shortest)
/// 2. r8 ← imm8 / r16 ← imm16 → 0xB0+rb ib / 0xB8+rw iw
/// 3. r/m8 ← imm8 / r/m16 ← imm16 → 0xC6 /0 ib / 0xC7 /0 iw
/// 4. sreg ↔ r/m16 → 0x8C / 0x8E
/// 5. r ↔ r/m → 0x88-0x8B
fn emit_mov(dest: &Operand, src: &Operand) -> Result<Vec<u8>, EmitError> {
    // 1. AL/AX <-> [moffs16] (Disp16); prepend a segment override when present.
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

    // 2. r <- imm short form.
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

    // 3. r/m <- imm (memory or a register without a short form).
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
    // A register source uses 0x88/0x89 (r/m <- r), preferred for reg-reg forms.
    // A register destination with a memory source uses 0x8A/0x8B (r <- r/m).
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

/// Emission-side counterpart to `decode.rs::AluOp`.
/// `group1_reg` selects the Group 1 reg field; `base_opcode` selects the direct-encoding base.
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
        // Direct-encoding base: Group 1 reg index << 3.
        // ADD=0x00, OR=0x08, AND=0x20, SUB=0x28, XOR=0x30, CMP=0x38
        self.group1_reg() << 3
    }
}

/// Encodes ALU forms in canonical priority order:
/// 1. AL/AX <-> imm short forms (`base+4`, `base+5`).
/// 2. Group 1 r/m <- imm: 0x83 for a sign-extendable word immediate, otherwise 0x81;
///    byte immediates use 0x80.
/// 3. Direct r <-> r/m forms: a register source prefers r/m <- r (`base+0/1`),
///    otherwise a register destination uses r <- r/m (`base+2/3`).
fn emit_alu(op: EmitAluOp, dest: &Operand, src: &Operand) -> Result<Vec<u8>, EmitError> {
    let base = op.base_opcode();

    // 1. AL/AX <-> imm short form, the shortest encoding.
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
            // Word form: 0x83 for a sign-extendable immediate, otherwise 0x81.
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

    // 3. Direct r <-> r/m form: a register source canonically selects `base+0/1`.
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

/// Encodes TEST separately from the ALU family, in this priority order:
/// 1. AL/AX <-> imm short forms (0xA8/0xA9).
/// 2. r/m <-> imm (Group 3 0xF6/0xF7 /0), with no sign-extended alternative.
/// 3. Direct r <-> r/m (0x84/0x85).
fn emit_test(a: &Operand, b: &Operand) -> Result<Vec<u8>, EmitError> {
    // 1. AL/AX <-> imm short form (0xA8/0xA9).
    match (a, b) {
        (Operand::Reg8(Reg8(0)), Operand::Imm8(v)) => return Ok(vec![0xA8, *v]),
        (Operand::Reg16(Reg16(0)), Operand::Imm16(v)) => {
            let bytes = v.to_le_bytes();
            return Ok(vec![0xA9, bytes[0], bytes[1]]);
        }
        _ => {}
    }
    // 2. r/m <- imm (0xF6 /0 / 0xF7 /0), without sign extension.
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
    // 3. r/m <-> r (0x84/0x85). TEST is commutative, so either operand may be the register.
    //    Canonically prefer b=Reg; otherwise use a=Reg.
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

/// Encodes INC/DEC r/m using Group 4 (0xFE) or Group 5 (0xFF) plus a reg field.
/// `reg_field`: 0=INC, 1=DEC.
/// The `emit` match handles reg16 short forms (0x40+r / 0x48+r) first, so this function
/// only accepts Reg8 and Mem.
fn emit_inc_dec(dest: &Operand, reg_field: u8) -> Result<Vec<u8>, EmitError> {
    let opcode = match dest {
        Operand::Reg8(_) => 0xFE,
        Operand::Mem(m) if m.size == OpSize::Byte => 0xFE,
        Operand::Mem(m) if m.size == OpSize::Word => 0xFF,
        // Reg16 is handled by the short-form arm; reaching this point is invalid.
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

/// Encodes Group 3 unary forms as 0xF6/0xF7 plus a reg field.
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

/// Encodes shifts and rotates (Group 2 0xD0-0xD3 plus reg field).
/// `reg_field`: ROL=0, ROR=1, SHL/SAL=4, SHR=5, SAR=7.
/// `One` selects 0xD0/0xD1 and `Cl` selects 0xD2/0xD3; the destination selects byte or word.
/// `Imm8(_)` uses the V30/186+ 0xC0/0xC1 selector.
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

/// Encodes `MOV AL/AX <-> [moffs16]` short forms (0xA0/0xA1/0xA2/0xA3).
/// These forms encode disp16 directly without ModR/M. The `emit` entry point strips and
/// prepends segment overrides, so this helper does not handle prefixes.
fn emit_mov_acc_moffs(opcode: u8, mem: &Mem) -> Result<Vec<u8>, EmitError> {
    let mut out = vec![opcode];
    out.extend_from_slice(&(mem.disp as u16).to_le_bytes());
    Ok(out)
}

/// Encodes XCHG canonically, preferring `XCHG AX, r16` short form (0x90+rw) when a=AX.
/// The b=AX order uses the general form to preserve operand order; others use 0x86/0x87 /r.
fn emit_xchg(a: &Operand, b: &Operand) -> Result<Vec<u8>, EmitError> {
    // 1. `XCHG AX, r16` short form (0x90+rw) when a=AX and b is another 16-bit GPR.
    //    `XCHG r16, AX` (b=AX) uses the general r/m, r form (0x87 /r).
    if let (Operand::Reg16(Reg16(0)), Operand::Reg16(other)) = (a, b)
        && other.0 != 0
    {
        return Ok(vec![0x90 + other.0]);
    }

    // 2. General 0x86 (byte) / 0x87 (word) /r requires one register operand.
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
