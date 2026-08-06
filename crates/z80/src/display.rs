use crate::instruction::*;
use std::fmt;

// ---------------------------------------------------------------------------
// Register display impls
// ---------------------------------------------------------------------------

impl fmt::Display for ByteOperand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ByteOperand::B => write!(f, "B"),
            ByteOperand::C => write!(f, "C"),
            ByteOperand::D => write!(f, "D"),
            ByteOperand::E => write!(f, "E"),
            ByteOperand::H => write!(f, "H"),
            ByteOperand::L => write!(f, "L"),
            ByteOperand::IndirectHl => write!(f, "(HL)"),
            ByteOperand::A => write!(f, "A"),
        }
    }
}

impl fmt::Display for Register16 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Register16::Bc => write!(f, "BC"),
            Register16::De => write!(f, "DE"),
            Register16::Hl => write!(f, "HL"),
            Register16::Sp => write!(f, "SP"),
        }
    }
}

impl fmt::Display for StackRegister {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StackRegister::Bc => write!(f, "BC"),
            StackRegister::De => write!(f, "DE"),
            StackRegister::Hl => write!(f, "HL"),
            StackRegister::Af => write!(f, "AF"),
        }
    }
}

impl fmt::Display for IndexRegister {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IndexRegister::Ix => write!(f, "IX"),
            IndexRegister::Iy => write!(f, "IY"),
        }
    }
}

impl fmt::Display for Condition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Condition::Nz => write!(f, "NZ"),
            Condition::Z => write!(f, "Z"),
            Condition::Nc => write!(f, "NC"),
            Condition::C => write!(f, "C"),
            Condition::Po => write!(f, "PO"),
            Condition::Pe => write!(f, "PE"),
            Condition::P => write!(f, "P"),
            Condition::M => write!(f, "M"),
        }
    }
}

impl fmt::Display for AluOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AluOperation::Add => write!(f, "ADD"),
            AluOperation::Adc => write!(f, "ADC"),
            AluOperation::Sub => write!(f, "SUB"),
            AluOperation::Sbc => write!(f, "SBC"),
            AluOperation::And => write!(f, "AND"),
            AluOperation::Xor => write!(f, "XOR"),
            AluOperation::Or => write!(f, "OR"),
            AluOperation::Cp => write!(f, "CP"),
        }
    }
}

impl fmt::Display for RotateShiftOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RotateShiftOperation::Rlc => write!(f, "RLC"),
            RotateShiftOperation::Rrc => write!(f, "RRC"),
            RotateShiftOperation::Rl => write!(f, "RL"),
            RotateShiftOperation::Rr => write!(f, "RR"),
            RotateShiftOperation::Sla => write!(f, "SLA"),
            RotateShiftOperation::Sra => write!(f, "SRA"),
            RotateShiftOperation::Srl => write!(f, "SRL"),
        }
    }
}

impl fmt::Display for InterruptMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InterruptMode::Im0 => write!(f, "0"),
            InterruptMode::Im1 => write!(f, "1"),
            InterruptMode::Im2 => write!(f, "2"),
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: format a displacement as (Ix+$05) or (Ix-$03)
// ---------------------------------------------------------------------------

fn fmt_ix_disp(f: &mut fmt::Formatter<'_>, reg: IndexRegister, disp: i8) -> fmt::Result {
    if disp >= 0 {
        write!(f, "({}+${:02X})", reg, disp as u8)
    } else {
        write!(f, "({}-${:02X})", reg, (-(disp as i16)) as u8)
    }
}

// ---------------------------------------------------------------------------
// Instruction display
// ---------------------------------------------------------------------------

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // ----------------------------------------------------------------
            // Misc / control
            // ----------------------------------------------------------------
            Instruction::Nop => write!(f, "NOP"),
            Instruction::Halt => write!(f, "HALT"),
            Instruction::Di => write!(f, "DI"),
            Instruction::Ei => write!(f, "EI"),

            // ----------------------------------------------------------------
            // 8-bit loads
            // ----------------------------------------------------------------
            Instruction::LdRR(dst, src) => write!(f, "LD {}, {}", dst, src),
            Instruction::LdRImm(dst, n) => write!(f, "LD {}, ${:02X}", dst, n),
            Instruction::LdAddrA(nn) => write!(f, "LD (${:04X}), A", nn),
            Instruction::LdAAddr(nn) => write!(f, "LD A, (${:04X})", nn),
            Instruction::LdBCA => write!(f, "LD (BC), A"),
            Instruction::LdDEA => write!(f, "LD (DE), A"),
            Instruction::LdABC => write!(f, "LD A, (BC)"),
            Instruction::LdADE => write!(f, "LD A, (DE)"),
            Instruction::LdAddrHL(nn) => write!(f, "LD (${:04X}), HL", nn),
            Instruction::LdHLAddr(nn) => write!(f, "LD HL, (${:04X})", nn),

            // ----------------------------------------------------------------
            // 16-bit loads
            // ----------------------------------------------------------------
            Instruction::LdRRImm(rr, nn) => write!(f, "LD {}, ${:04X}", rr, nn),
            Instruction::LdSPHL => write!(f, "LD SP, HL"),
            Instruction::Push(rr) => write!(f, "PUSH {}", rr),
            Instruction::Pop(rr) => write!(f, "POP {}", rr),

            // ----------------------------------------------------------------
            // 8-bit arithmetic / logic
            // ----------------------------------------------------------------
            Instruction::AluR(op, r) => match op {
                AluOperation::Add | AluOperation::Adc | AluOperation::Sbc => {
                    write!(f, "{} A, {}", op, r)
                }
                _ => write!(f, "{} {}", op, r),
            },
            Instruction::AluImm(op, n) => match op {
                AluOperation::Add | AluOperation::Adc | AluOperation::Sbc => {
                    write!(f, "{} A, ${:02X}", op, n)
                }
                _ => write!(f, "{} ${:02X}", op, n),
            },
            Instruction::IncR(r) => write!(f, "INC {}", r),
            Instruction::DecR(r) => write!(f, "DEC {}", r),

            // ----------------------------------------------------------------
            // 16-bit arithmetic
            // ----------------------------------------------------------------
            Instruction::IncRR(rr) => write!(f, "INC {}", rr),
            Instruction::DecRR(rr) => write!(f, "DEC {}", rr),
            Instruction::AddHLRR(rr) => write!(f, "ADD HL, {}", rr),

            // ----------------------------------------------------------------
            // Jumps
            // ----------------------------------------------------------------
            Instruction::Jp(nn) => write!(f, "JP ${:04X}", nn),
            Instruction::JpCond(cc, nn) => write!(f, "JP {}, ${:04X}", cc, nn),
            Instruction::JpHL => write!(f, "JP (HL)"),
            // JR/DJNZ: show raw displacement byte as hex
            Instruction::Jr(d) => write!(f, "JR ${:02X}", *d as u8),
            Instruction::JrCond(cc, d) => write!(f, "JR {}, ${:02X}", cc, *d as u8),
            Instruction::Djnz(d) => write!(f, "DJNZ ${:02X}", *d as u8),

            // ----------------------------------------------------------------
            // Calls / returns
            // ----------------------------------------------------------------
            Instruction::Call(nn) => write!(f, "CALL ${:04X}", nn),
            Instruction::CallCond(cc, nn) => write!(f, "CALL {}, ${:04X}", cc, nn),
            Instruction::Ret => write!(f, "RET"),
            Instruction::RetCond(cc) => write!(f, "RET {}", cc),
            Instruction::Reti => write!(f, "RETI"),
            Instruction::Retn => write!(f, "RETN"),
            Instruction::Rst(p) => write!(f, "RST ${:02X}", p),

            // ----------------------------------------------------------------
            // Accumulator rotates
            // ----------------------------------------------------------------
            Instruction::Rlca => write!(f, "RLCA"),
            Instruction::Rrca => write!(f, "RRCA"),
            Instruction::Rla => write!(f, "RLA"),
            Instruction::Rra => write!(f, "RRA"),

            // ----------------------------------------------------------------
            // Misc accumulator / flag ops
            // ----------------------------------------------------------------
            Instruction::ExAFAF => write!(f, "EX AF, AF'"),
            Instruction::Exx => write!(f, "EXX"),
            Instruction::ExDEHL => write!(f, "EX DE, HL"),
            Instruction::ExSPHL => write!(f, "EX (SP), HL"),
            Instruction::Daa => write!(f, "DAA"),
            Instruction::Cpl => write!(f, "CPL"),
            Instruction::Neg => write!(f, "NEG"),
            Instruction::Scf => write!(f, "SCF"),
            Instruction::Ccf => write!(f, "CCF"),

            // ----------------------------------------------------------------
            // I/O
            // ----------------------------------------------------------------
            Instruction::InAPort(n) => write!(f, "IN A, (${:02X})", n),
            Instruction::OutPortA(n) => write!(f, "OUT (${:02X}), A", n),
            Instruction::InRC(r) => write!(f, "IN {}, (C)", r),
            Instruction::OutCR(r) => write!(f, "OUT (C), {}", r),

            // ----------------------------------------------------------------
            // Block I/O
            // ----------------------------------------------------------------
            Instruction::Ini => write!(f, "INI"),
            Instruction::Ind => write!(f, "IND"),
            Instruction::Inir => write!(f, "INIR"),
            Instruction::Indr => write!(f, "INDR"),
            Instruction::Outi => write!(f, "OUTI"),
            Instruction::Outd => write!(f, "OUTD"),
            Instruction::Otir => write!(f, "OTIR"),
            Instruction::Otdr => write!(f, "OTDR"),

            // ----------------------------------------------------------------
            // Block transfer / search
            // ----------------------------------------------------------------
            Instruction::Ldi => write!(f, "LDI"),
            Instruction::Ldd => write!(f, "LDD"),
            Instruction::Ldir => write!(f, "LDIR"),
            Instruction::Lddr => write!(f, "LDDR"),
            Instruction::Cpi => write!(f, "CPI"),
            Instruction::Cpd => write!(f, "CPD"),
            Instruction::Cpir => write!(f, "CPIR"),
            Instruction::Cpdr => write!(f, "CPDR"),

            // ----------------------------------------------------------------
            // CB prefix: bit operations
            // ----------------------------------------------------------------
            Instruction::Rot(op, r) => write!(f, "{} {}", op, r),
            Instruction::Bit(b, r) => write!(f, "BIT {}, {}", b, r),
            Instruction::Res(b, r) => write!(f, "RES {}, {}", b, r),
            Instruction::Set(b, r) => write!(f, "SET {}, {}", b, r),

            // ----------------------------------------------------------------
            // ED prefix: 16-bit arithmetic
            // ----------------------------------------------------------------
            Instruction::SbcHLRR(rr) => write!(f, "SBC HL, {}", rr),
            Instruction::AdcHLRR(rr) => write!(f, "ADC HL, {}", rr),
            Instruction::LdAddrRR(nn, rr) => write!(f, "LD (${:04X}), {}", nn, rr),
            Instruction::LdRRAddr(rr, nn) => write!(f, "LD {}, (${:04X})", rr, nn),

            // ----------------------------------------------------------------
            // ED prefix: I/R register transfers
            // ----------------------------------------------------------------
            Instruction::LdIA => write!(f, "LD I, A"),
            Instruction::LdRA => write!(f, "LD R, A"),
            Instruction::LdAI => write!(f, "LD A, I"),
            Instruction::LdAR => write!(f, "LD A, R"),

            // ----------------------------------------------------------------
            // ED prefix: rotate digit
            // ----------------------------------------------------------------
            Instruction::Rrd => write!(f, "RRD"),
            Instruction::Rld => write!(f, "RLD"),

            // ----------------------------------------------------------------
            // ED prefix: interrupt mode
            // ----------------------------------------------------------------
            Instruction::Im(mode) => write!(f, "IM {}", mode),

            // ----------------------------------------------------------------
            // DD/FD prefix: Ix/Iy whole-register ops
            // ----------------------------------------------------------------
            Instruction::LdIxImm(reg, nn) => write!(f, "LD {}, ${:04X}", reg, nn),
            Instruction::LdIxAddr(reg, nn) => write!(f, "LD {}, (${:04X})", reg, nn),
            Instruction::LdAddrIx(nn, reg) => write!(f, "LD (${:04X}), {}", nn, reg),
            Instruction::PushIx(reg) => write!(f, "PUSH {}", reg),
            Instruction::PopIx(reg) => write!(f, "POP {}", reg),
            Instruction::IncIx(reg) => write!(f, "INC {}", reg),
            Instruction::DecIx(reg) => write!(f, "DEC {}", reg),
            Instruction::AddIxRR(reg, rr) => write!(f, "ADD {}, {}", reg, rr),
            Instruction::LdSpIx(reg) => write!(f, "LD SP, {}", reg),
            Instruction::JpIx(reg) => write!(f, "JP ({})", reg),
            Instruction::ExSpIx(reg) => write!(f, "EX (SP), {}", reg),

            // ----------------------------------------------------------------
            // DD/FD prefix: Ix/Iy + displacement
            // ----------------------------------------------------------------
            Instruction::LdRIxDisp(r, reg, d) => {
                write!(f, "LD {}, ", r)?;
                fmt_ix_disp(f, *reg, *d)
            }
            Instruction::LdIxDispR(reg, d, r) => {
                write!(f, "LD ")?;
                fmt_ix_disp(f, *reg, *d)?;
                write!(f, ", {}", r)
            }
            Instruction::LdIxDispImm(reg, d, n) => {
                write!(f, "LD ")?;
                fmt_ix_disp(f, *reg, *d)?;
                write!(f, ", ${:02X}", n)
            }
            Instruction::IncIxDisp(reg, d) => {
                write!(f, "INC ")?;
                fmt_ix_disp(f, *reg, *d)
            }
            Instruction::DecIxDisp(reg, d) => {
                write!(f, "DEC ")?;
                fmt_ix_disp(f, *reg, *d)
            }
            Instruction::AluIxDisp(op, reg, d) => match op {
                AluOperation::Add | AluOperation::Adc | AluOperation::Sbc => {
                    write!(f, "{} A, ", op)?;
                    fmt_ix_disp(f, *reg, *d)
                }
                _ => {
                    write!(f, "{} ", op)?;
                    fmt_ix_disp(f, *reg, *d)
                }
            },

            // ----------------------------------------------------------------
            // DDCB/FDCB prefix: indexed bit ops
            // ----------------------------------------------------------------
            Instruction::RotIxDisp(op, reg, d) => {
                write!(f, "{} ", op)?;
                fmt_ix_disp(f, *reg, *d)
            }
            Instruction::BitIxDisp(b, reg, d) => {
                write!(f, "BIT {}, ", b)?;
                fmt_ix_disp(f, *reg, *d)
            }
            Instruction::ResIxDisp(b, reg, d) => {
                write!(f, "RES {}, ", b)?;
                fmt_ix_disp(f, *reg, *d)
            }
            Instruction::SetIxDisp(b, reg, d) => {
                write!(f, "SET {}, ", b)?;
                fmt_ix_disp(f, *reg, *d)
            }
        }
    }
}
