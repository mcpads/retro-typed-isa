//! NASM 호환 텍스트 포맷 — `Instruction` → 한 줄 문자열.
//!
//! 형식: `SSSS:OOOO  BB BB BB BB ...    mnemonic operands`
//!
//! - cs:ip는 4자리 16진수, 대문자.
//! - byte hex 영역은 16자 폭으로 left-pad 후 한 칸 띄우고 mnemonic.
//!   (6바이트(`BB BB BB BB BB BB` = 17chars)까지 정렬을 의도하지만, 실제로는 16자
//!   폭에 한 칸 explicit 공백을 더해 17열이 mnemonic 시작 위치.)
//! - 메모리 operand의 size prefix(`byte`/`word`)는 본 마일스톤에서 생략 — NASM이 컨텍스트에서 추론.
//! - segment override는 `mem.seg`에 보존된 경우 `es:[...]` 형태로 출력 (raw byte의
//!   prefix 0x26은 byte hex 영역에서만 표시).

use crate::instruction::*;

/// 원본 표시 회귀 fixture를 위한 주소·byte 포함 한 줄 표현.
#[cfg(test)]
pub fn format_line(insn: &Instruction) -> String {
    use std::fmt::Write;

    let mut output = String::new();
    write!(
        &mut output,
        "{:04X}:{:04X}  ",
        insn.cs_ip.seg, insn.cs_ip.off
    )
    .unwrap();
    let mut bytes = String::new();
    for (index, byte) in insn.bytes.iter().enumerate() {
        if index != 0 {
            bytes.push(' ');
        }
        write!(&mut bytes, "{byte:02X}").unwrap();
    }
    write!(&mut output, "{bytes:<16} ").unwrap();
    output.push_str(&format_kind(&insn.kind));
    output
}

/// `InstructionKind` → mnemonic + operand 문자열.
pub fn format_kind(k: &InstructionKind) -> String {
    match k {
        InstructionKind::Nop => "nop".to_string(),
        InstructionKind::Hlt => "hlt".to_string(),
        InstructionKind::Xlat => "xlat".to_string(),
        InstructionKind::Iret => "iret".to_string(),
        InstructionKind::Int { vector } => format!("int 0x{:X}", vector),
        InstructionKind::Into => "into".to_string(),

        // 플래그 명령 / sign-extend — NASM mnemonic.
        InstructionKind::Wait => "wait".to_string(),
        InstructionKind::Pushf => "pushf".to_string(),
        InstructionKind::Popf => "popf".to_string(),
        InstructionKind::Sahf => "sahf".to_string(),
        InstructionKind::Lahf => "lahf".to_string(),
        InstructionKind::Cmc => "cmc".to_string(),
        InstructionKind::Clc => "clc".to_string(),
        InstructionKind::Stc => "stc".to_string(),
        InstructionKind::Cli => "cli".to_string(),
        InstructionKind::Sti => "sti".to_string(),
        InstructionKind::Cld => "cld".to_string(),
        InstructionKind::Std => "std".to_string(),
        InstructionKind::Cbw => "cbw".to_string(),
        InstructionKind::Cwd => "cwd".to_string(),
        InstructionKind::Aaa => "aaa".to_string(),
        InstructionKind::Aas => "aas".to_string(),
        InstructionKind::Daa => "daa".to_string(),
        InstructionKind::Das => "das".to_string(),
        InstructionKind::Aam { base } => format!("aam 0x{base:X}"),
        InstructionKind::Aad { base } => format!("aad 0x{base:X}"),

        InstructionKind::Mov { dest, src } => format!("mov {}, {}", fmt_op(dest), fmt_op(src)),
        InstructionKind::Lea { dest, src } => format!("lea {}, {}", dest.name(), fmt_mem(src)),
        InstructionKind::Les { dest, src } => format!("les {}, {}", dest.name(), fmt_mem(src)),
        InstructionKind::Lds { dest, src } => format!("lds {}, {}", dest.name(), fmt_mem(src)),
        InstructionKind::Xchg { a, b } => format!("xchg {}, {}", fmt_op(a), fmt_op(b)),
        InstructionKind::Push { src } => format!("push {}", fmt_op(src)),
        InstructionKind::Pop { dest } => format!("pop {}", fmt_op(dest)),
        InstructionKind::Pusha => "pusha".to_string(),
        InstructionKind::Popa => "popa".to_string(),
        InstructionKind::Bound { index, bounds } => {
            format!("bound {}, {}", index.name(), fmt_mem(bounds))
        }

        InstructionKind::Add { dest, src } => format!("add {}, {}", fmt_op(dest), fmt_op(src)),
        InstructionKind::Adc { dest, src } => format!("adc {}, {}", fmt_op(dest), fmt_op(src)),
        InstructionKind::Sub { dest, src } => format!("sub {}, {}", fmt_op(dest), fmt_op(src)),
        InstructionKind::Sbb { dest, src } => format!("sbb {}, {}", fmt_op(dest), fmt_op(src)),
        InstructionKind::Cmp { a, b } => format!("cmp {}, {}", fmt_op(a), fmt_op(b)),
        InstructionKind::Test { a, b } => format!("test {}, {}", fmt_op(a), fmt_op(b)),
        InstructionKind::And { dest, src } => format!("and {}, {}", fmt_op(dest), fmt_op(src)),
        InstructionKind::Or { dest, src } => format!("or {}, {}", fmt_op(dest), fmt_op(src)),
        InstructionKind::Xor { dest, src } => format!("xor {}, {}", fmt_op(dest), fmt_op(src)),
        InstructionKind::Inc { dest } => format!("inc {}", fmt_op(dest)),
        InstructionKind::Dec { dest } => format!("dec {}", fmt_op(dest)),
        InstructionKind::Neg { dest } => format!("neg {}", fmt_op(dest)),
        InstructionKind::Not { dest } => format!("not {}", fmt_op(dest)),
        InstructionKind::Mul { src } => format!("mul {}", fmt_op(src)),
        InstructionKind::Imul { src } => format!("imul {}", fmt_op(src)),
        InstructionKind::ImulImmediate {
            dest,
            src,
            immediate,
        } => format!(
            "imul {}, {}, {}",
            dest.name(),
            fmt_op(src),
            fmt_signed_immediate(*immediate)
        ),
        InstructionKind::Div { src } => format!("div {}", fmt_op(src)),
        InstructionKind::Idiv { src } => format!("idiv {}", fmt_op(src)),

        InstructionKind::Shl { dest, count } => {
            format!("shl {}, {}", fmt_op(dest), fmt_count(*count))
        }
        InstructionKind::Shr { dest, count } => {
            format!("shr {}, {}", fmt_op(dest), fmt_count(*count))
        }
        InstructionKind::Sar { dest, count } => {
            format!("sar {}, {}", fmt_op(dest), fmt_count(*count))
        }
        InstructionKind::Rol { dest, count } => {
            format!("rol {}, {}", fmt_op(dest), fmt_count(*count))
        }
        InstructionKind::Ror { dest, count } => {
            format!("ror {}, {}", fmt_op(dest), fmt_count(*count))
        }
        InstructionKind::Rcl { dest, count } => {
            format!("rcl {}, {}", fmt_op(dest), fmt_count(*count))
        }
        InstructionKind::Rcr { dest, count } => {
            format!("rcr {}, {}", fmt_op(dest), fmt_count(*count))
        }

        InstructionKind::Jcc { cond, target } => {
            let mn = if *cond == Cond::Cxz {
                "jcxz".to_string()
            } else {
                format!("j{}", cond.name())
            };
            let sign = if *target < 0 { "-" } else { "" };
            format!("{} short {}0x{:X}", mn, sign, target.unsigned_abs())
        }
        InstructionKind::Loop { condition, target } => {
            let mnemonic = match condition {
                LoopCondition::NotEqual => "loopne",
                LoopCondition::Equal => "loope",
                LoopCondition::Always => "loop",
            };
            let sign = if *target < 0 { "-" } else { "" };
            format!("{} short {}0x{:X}", mnemonic, sign, target.unsigned_abs())
        }
        InstructionKind::Jmp { target } => format!("jmp {}", fmt_jmp_target(target)),
        InstructionKind::Call { target } => format!("call {}", fmt_call_target(target)),
        InstructionKind::Ret { pop: 0 } => "ret".to_string(),
        InstructionKind::Ret { pop } => format!("ret 0x{:X}", pop),
        InstructionKind::Retf { pop: 0 } => "retf".to_string(),
        InstructionKind::Retf { pop } => format!("retf 0x{:X}", pop),

        InstructionKind::InAl { port } => format!("in al, {}", fmt_port(port)),
        InstructionKind::InAx { port } => format!("in ax, {}", fmt_port(port)),
        InstructionKind::OutAl { port } => format!("out {}, al", fmt_port(port)),
        InstructionKind::OutAx { port } => format!("out {}, ax", fmt_port(port)),
        InstructionKind::Insb => "insb".to_string(),
        InstructionKind::Insw => "insw".to_string(),
        InstructionKind::Outsb => "outsb".to_string(),
        InstructionKind::Outsw => "outsw".to_string(),

        InstructionKind::Movsb => "movsb".to_string(),
        InstructionKind::Movsw => "movsw".to_string(),
        InstructionKind::Cmpsb => "cmpsb".to_string(),
        InstructionKind::Cmpsw => "cmpsw".to_string(),
        InstructionKind::Stosb => "stosb".to_string(),
        InstructionKind::Stosw => "stosw".to_string(),
        InstructionKind::Lodsb => "lodsb".to_string(),
        InstructionKind::Lodsw => "lodsw".to_string(),
        InstructionKind::Scasb => "scasb".to_string(),
        InstructionKind::Scasw => "scasw".to_string(),
        InstructionKind::Rep(inner) => format!("rep {}", format_kind(inner)),
        InstructionKind::Repne(inner) => format!("repne {}", format_kind(inner)),
        InstructionKind::Repc(inner) => format!("repc {}", format_kind(inner)),
        InstructionKind::Repnc(inner) => format!("repnc {}", format_kind(inner)),
        InstructionKind::BusLock(inner) => format!("buslock {}", format_kind(inner)),
        InstructionKind::SegmentOverride {
            segment,
            instruction,
        } => format!("{}: {}", segment.name(), format_kind(instruction)),
        InstructionKind::Enter {
            frame_size,
            nesting_level,
        } => format!("enter 0x{frame_size:X}, 0x{nesting_level:X}"),
        InstructionKind::Leave => "leave".to_string(),
        InstructionKind::Esc { operation, operand } => {
            format!("esc 0x{:X}, {}", operation.value(), fmt_floating(operand))
        }
        InstructionKind::Fpo2 { operation, operand } => {
            format!("fpo2 0x{:X}, {}", operation.value(), fmt_floating(operand))
        }

        // V30 (NEC uPD70116) 0x0F 확장 — NASM mnemonic.
        InstructionKind::Add4s => "add4s".to_string(),
        InstructionKind::Sub4s => "sub4s".to_string(),
        InstructionKind::Cmp4s => "cmp4s".to_string(),
        InstructionKind::Rol4 { rm } => format!("rol4 {}", fmt_op(rm)),
        InstructionKind::Ror4 { rm } => format!("ror4 {}", fmt_op(rm)),
        InstructionKind::Ins { bit_offset, length } => {
            format!("ins {}, {}", bit_offset.name(), length.name())
        }
        InstructionKind::Ext { bit_offset, length } => {
            format!("ext {}, {}", bit_offset.name(), length.name())
        }
        InstructionKind::InsImm { bit_offset, length } => {
            format!("ins {}, 0x{:X}", bit_offset.name(), length.selector())
        }
        InstructionKind::ExtImm { bit_offset, length } => {
            format!("ext {}, 0x{:X}", bit_offset.name(), length.selector())
        }
        InstructionKind::Test1 { target, bit } => {
            format!("test1 {}, {}", fmt_op(target), fmt_bit(*bit))
        }
        InstructionKind::Clr1 { target, bit } => {
            format!("clr1 {}, {}", fmt_op(target), fmt_bit(*bit))
        }
        InstructionKind::Set1 { target, bit } => {
            format!("set1 {}, {}", fmt_op(target), fmt_bit(*bit))
        }
        InstructionKind::Not1 { target, bit } => {
            format!("not1 {}, {}", fmt_op(target), fmt_bit(*bit))
        }
        InstructionKind::Brkem { vector } => format!("brkem 0x{:X}", vector),

        InstructionKind::Unknown { raw } => {
            let parts: Vec<String> = raw.iter().map(|b| format!("0x{:02X}", b)).collect();
            format!("db {}", parts.join(", "))
        }
    }
}

fn fmt_bit(bit: BitIndex) -> String {
    match bit {
        BitIndex::Cl => "cl".to_string(),
        BitIndex::Immediate(value) => format!("0x{value:X}"),
    }
}

fn fmt_op(op: &Operand) -> String {
    match op {
        Operand::Reg8(r) => r.name().to_string(),
        Operand::Reg16(r) => r.name().to_string(),
        Operand::Sreg(s) => s.name().to_string(),
        Operand::Imm8(v) => format!("0x{:X}", v),
        Operand::Imm16(v) => format!("0x{:X}", v),
        Operand::Mem(m) => fmt_mem(m),
        Operand::Rel8(v) => {
            let sign = if *v < 0 { "-" } else { "" };
            format!("{}0x{:X}", sign, v.unsigned_abs())
        }
        Operand::Rel16(v) => {
            let sign = if *v < 0 { "-" } else { "" };
            format!("{}0x{:X}", sign, v.unsigned_abs())
        }
        Operand::Far { seg, off } => format!("0x{:X}:0x{:X}", seg, off),
    }
}

/// 메모리 operand 텍스트화.
///
/// - `MemBase::Direct` (절대주소 `[disp16]`): `disp`는 unsigned u16.
/// - 그 외 base는 `disp`가 signed i16. `disp == 0`이면 disp 표시 없음.
/// - segment override (`mem.seg`)는 `es:[...]` 형태로 prefix.
/// - size prefix(`byte`/`word`)는 본 마일스톤에서 생략 (NASM이 컨텍스트에서 추론).
fn fmt_mem(m: &Mem) -> String {
    let seg_prefix = m.seg.map(|s| format!("{}:", s.name())).unwrap_or_default();
    let inside = match m.base {
        MemBase::BxSi => fmt_indexed("bx+si", m.disp),
        MemBase::BxDi => fmt_indexed("bx+di", m.disp),
        MemBase::BpSi => fmt_indexed("bp+si", m.disp),
        MemBase::BpDi => fmt_indexed("bp+di", m.disp),
        MemBase::Si => fmt_indexed("si", m.disp),
        MemBase::Di => fmt_indexed("di", m.disp),
        MemBase::Bp => fmt_indexed("bp", m.disp),
        MemBase::Bx => fmt_indexed("bx", m.disp),
        MemBase::Direct => format!("0x{:X}", m.disp as u16),
    };
    format!("{}[{}]", seg_prefix, inside)
}

fn fmt_indexed(base: &str, disp: i16) -> String {
    if disp == 0 {
        base.to_string()
    } else if disp > 0 {
        format!("{}+0x{:X}", base, disp as u16)
    } else {
        // disp < 0 — abs로 표시. -0x8000 (i16::MIN) 케이스는 unsigned_abs 사용해 panic 방지.
        format!("{}-0x{:X}", base, (disp as i32).unsigned_abs())
    }
}

fn fmt_count(c: ShiftCount) -> String {
    match c {
        ShiftCount::One => "1".to_string(),
        ShiftCount::Cl => "cl".to_string(),
        ShiftCount::Imm8(v) => format!("0x{:X}", v),
    }
}

fn fmt_signed_immediate(immediate: SignedImmediate) -> String {
    match immediate {
        SignedImmediate::Byte(value) => value.to_string(),
        SignedImmediate::Word(value) => value.to_string(),
    }
}

fn fmt_floating(operand: &FloatingPointOperand) -> String {
    match operand {
        FloatingPointOperand::Register(register) => format!("st({})", register.index()),
        FloatingPointOperand::Memory(memory) => fmt_mem(memory),
    }
}

fn fmt_port(p: &PortAddr) -> String {
    match p {
        PortAddr::Imm8(v) => format!("0x{:X}", v),
        PortAddr::Dx => "dx".to_string(),
    }
}

fn fmt_jmp_target(t: &JmpTarget) -> String {
    match t {
        JmpTarget::Rel8(v) => {
            let sign = if *v < 0 { "-" } else { "" };
            format!("short {}0x{:X}", sign, v.unsigned_abs())
        }
        JmpTarget::Rel16(v) => {
            let sign = if *v < 0 { "-" } else { "" };
            format!("near {}0x{:X}", sign, v.unsigned_abs())
        }
        JmpTarget::Far { seg, off } => format!("far 0x{:X}:0x{:X}", seg, off),
        // 0xFF /4 — JMP near r/m16. NASM은 reg/mem만 보고 near를 추론하므로 prefix 생략.
        JmpTarget::IndirectNear(op) => fmt_op(op),
        // 0xFF /5 — JMP far m16:16. 메모리 operand 전용. `far` prefix 필요 (NASM이 m16:16과
        // m16를 구분 못 하므로 명시).
        JmpTarget::IndirectFar(m) => format!("far {}", fmt_mem(m)),
    }
}

fn fmt_call_target(t: &CallTarget) -> String {
    match t {
        CallTarget::Rel16(v) => {
            let sign = if *v < 0 { "-" } else { "" };
            format!("near {}0x{:X}", sign, v.unsigned_abs())
        }
        CallTarget::Far { seg, off } => format!("far 0x{:X}:0x{:X}", seg, off),
        // 0xFF /2 — CALL near r/m16. NASM이 컨텍스트에서 near 추론.
        CallTarget::IndirectNear(op) => fmt_op(op),
        // 0xFF /3 — CALL far m16:16. 메모리 operand 전용.
        CallTarget::IndirectFar(m) => format!("far {}", fmt_mem(m)),
    }
}
