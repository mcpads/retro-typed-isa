use crate::instruction::*;
use crate::{DecodeError, decode_bytes};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EncodeError {
    #[error("{instruction} cannot use byte operand {operand:?}")]
    InvalidByteOperand {
        instruction: &'static str,
        operand: ByteOperand,
    },
    #[error("bit index {bit} exceeds the Z80 three-bit field")]
    BitIndexOutOfRange { bit: u8 },
    #[error("condition {condition:?} is unavailable for JR")]
    InvalidRelativeCondition { condition: Condition },
    #[error("RST target 0x{target:02X} is not one of 00/08/10/18/20/28/30/38")]
    InvalidRestartTarget { target: u8 },
}

fn bit_field(bit: u8) -> Result<u8, EncodeError> {
    (bit < 8)
        .then_some(bit)
        .ok_or(EncodeError::BitIndexOutOfRange { bit })
}

fn register_field(instruction: &'static str, operand: ByteOperand) -> Result<u8, EncodeError> {
    if operand == ByteOperand::IndirectHl {
        Err(EncodeError::InvalidByteOperand {
            instruction,
            operand,
        })
    } else {
        Ok(operand.field())
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Encode a Z80 instruction to bytes.
///
/// Every value in the declared documented profile is supported. Operand
/// combinations that do not denote an instruction fail instead of aliasing.
pub fn encode_bytes(instruction: &Instruction) -> Result<Vec<u8>, EncodeError> {
    match instruction {
        // ----------------------------------------------------------------
        // Misc / control
        // ----------------------------------------------------------------
        Instruction::Nop => Ok(vec![0x00]),
        Instruction::Halt => Ok(vec![0x76]),
        Instruction::Di => Ok(vec![0xF3]),
        Instruction::Ei => Ok(vec![0xFB]),
        Instruction::ExDEHL => Ok(vec![0xEB]),
        Instruction::Rlca => Ok(vec![0x07]),
        Instruction::Rrca => Ok(vec![0x0F]),
        Instruction::Rla => Ok(vec![0x17]),
        Instruction::Rra => Ok(vec![0x1F]),
        Instruction::ExAFAF => Ok(vec![0x08]),
        Instruction::Exx => Ok(vec![0xD9]),
        Instruction::ExSPHL => Ok(vec![0xE3]),
        Instruction::Daa => Ok(vec![0x27]),
        Instruction::Cpl => Ok(vec![0x2F]),
        Instruction::Scf => Ok(vec![0x37]),
        Instruction::Ccf => Ok(vec![0x3F]),

        // ----------------------------------------------------------------
        // 8-bit loads: LD r, r' — 0x40 | (dst << 3) | src
        // Special case: LD (Hl), (Hl) = 0x76 = HALT, but we won't generate that.
        // ----------------------------------------------------------------
        Instruction::LdRR(dst, src) => {
            let d = dst.field();
            let s = src.field();
            if *dst == ByteOperand::IndirectHl && *src == ByteOperand::IndirectHl {
                return Err(EncodeError::InvalidByteOperand {
                    instruction: "LD (HL),(HL)",
                    operand: ByteOperand::IndirectHl,
                });
            }
            Ok(vec![0x40 | (d << 3) | s])
        }

        // LD r, n — 0x06 | (r << 3), n
        Instruction::LdRImm(r, n) => Ok(vec![0x06 | (r.field() << 3), *n]),

        // LD (nn), A — 0x32, lo, hi
        Instruction::LdAddrA(nn) => Ok(vec![0x32, (*nn & 0xFF) as u8, ((*nn >> 8) & 0xFF) as u8]),

        // LD A, (nn) — 0x3A, lo, hi
        Instruction::LdAAddr(nn) => Ok(vec![0x3A, (*nn & 0xFF) as u8, ((*nn >> 8) & 0xFF) as u8]),

        // LD (Bc), A — 0x02
        Instruction::LdBCA => Ok(vec![0x02]),
        // LD (De), A — 0x12
        Instruction::LdDEA => Ok(vec![0x12]),
        // LD A, (Bc) — 0x0A
        Instruction::LdABC => Ok(vec![0x0A]),
        // LD A, (De) — 0x1A
        Instruction::LdADE => Ok(vec![0x1A]),

        // LD (nn), Hl — 0x22, lo, hi
        Instruction::LdAddrHL(nn) => Ok(vec![0x22, (*nn & 0xFF) as u8, ((*nn >> 8) & 0xFF) as u8]),

        // LD Hl, (nn) — 0x2A, lo, hi
        Instruction::LdHLAddr(nn) => Ok(vec![0x2A, (*nn & 0xFF) as u8, ((*nn >> 8) & 0xFF) as u8]),

        // ----------------------------------------------------------------
        // 16-bit loads
        // ----------------------------------------------------------------

        // LD rr, nn — 0x01 | (rr << 4), lo, hi
        Instruction::LdRRImm(rr, nn) => Ok(vec![
            0x01 | (rr.field() << 4),
            (*nn & 0xFF) as u8,
            ((*nn >> 8) & 0xFF) as u8,
        ]),
        Instruction::LdSPHL => Ok(vec![0xF9]),

        // PUSH rr — 0xC5 | (rr << 4)
        Instruction::Push(rr) => Ok(vec![0xC5 | (rr.field() << 4)]),

        // POP rr — 0xC1 | (rr << 4)
        Instruction::Pop(rr) => Ok(vec![0xC1 | (rr.field() << 4)]),

        // ----------------------------------------------------------------
        // 8-bit arithmetic / logic
        // ----------------------------------------------------------------

        // ALU r — 0x80 | (op << 3) | r
        Instruction::AluR(op, r) => Ok(vec![0x80 | (op.field() << 3) | r.field()]),

        // ALU n — 0xC6 | (op << 3), n
        Instruction::AluImm(op, n) => Ok(vec![0xC6 | (op.field() << 3), *n]),

        // INC r — 0x04 | (r << 3)
        Instruction::IncR(r) => Ok(vec![0x04 | (r.field() << 3)]),

        // DEC r — 0x05 | (r << 3)
        Instruction::DecR(r) => Ok(vec![0x05 | (r.field() << 3)]),

        // ----------------------------------------------------------------
        // 16-bit arithmetic
        // ----------------------------------------------------------------

        // INC rr — 0x03 | (rr << 4)
        Instruction::IncRR(rr) => Ok(vec![0x03 | (rr.field() << 4)]),

        // DEC rr — 0x0B | (rr << 4)
        Instruction::DecRR(rr) => Ok(vec![0x0B | (rr.field() << 4)]),

        // ADD Hl, rr — 0x09 | (rr << 4)
        Instruction::AddHLRR(rr) => Ok(vec![0x09 | (rr.field() << 4)]),

        // ----------------------------------------------------------------
        // Jumps
        // ----------------------------------------------------------------

        // JP nn — 0xC3, lo, hi
        Instruction::Jp(nn) => Ok(vec![0xC3, (*nn & 0xFF) as u8, ((*nn >> 8) & 0xFF) as u8]),

        // JP cc, nn — 0xC2 | (cc << 3), lo, hi
        Instruction::JpCond(cc, nn) => Ok(vec![
            0xC2 | (cc.field() << 3),
            (*nn & 0xFF) as u8,
            ((*nn >> 8) & 0xFF) as u8,
        ]),

        // JR e — 0x18, d
        Instruction::Jr(d) => Ok(vec![0x18, *d as u8]),

        // JR cc, e — only Nz(0)/Z(1)/Nc(2)/C(3), opcode 0x20 + (cc_index << 3)
        Instruction::JrCond(cc, d) => {
            let cc_idx = cc.field();
            if cc_idx > 3 {
                return Err(EncodeError::InvalidRelativeCondition { condition: *cc });
            }
            Ok(vec![0x20 | (cc_idx << 3), *d as u8])
        }

        // DJNZ e — 0x10, d
        Instruction::Djnz(d) => Ok(vec![0x10, *d as u8]),

        // ----------------------------------------------------------------
        // Calls / returns
        // ----------------------------------------------------------------

        // CALL nn — 0xCD, lo, hi
        Instruction::Call(nn) => Ok(vec![0xCD, (*nn & 0xFF) as u8, ((*nn >> 8) & 0xFF) as u8]),

        // CALL cc, nn — 0xC4 | (cc << 3), lo, hi
        Instruction::CallCond(cc, nn) => Ok(vec![
            0xC4 | (cc.field() << 3),
            (*nn & 0xFF) as u8,
            ((*nn >> 8) & 0xFF) as u8,
        ]),

        // RET — 0xC9
        Instruction::Ret => Ok(vec![0xC9]),

        // RST p — 0xC7 | p (valid targets: 0x00, 0x08, 0x10, 0x18, 0x20, 0x28, 0x30, 0x38)
        Instruction::Rst(p) => {
            if *p > 0x38 || p % 8 != 0 {
                return Err(EncodeError::InvalidRestartTarget { target: *p });
            }
            Ok(vec![0xC7 | p])
        }

        // ----------------------------------------------------------------
        // I/O
        // ----------------------------------------------------------------

        // IN A, (n) — 0xDB, n
        Instruction::InAPort(n) => Ok(vec![0xDB, *n]),
        // OUT (n), A — 0xD3, n
        Instruction::OutPortA(n) => Ok(vec![0xD3, *n]),

        // ----------------------------------------------------------------
        // Block I/O (ED-prefixed)
        // ----------------------------------------------------------------
        Instruction::Outi => Ok(vec![0xED, 0xA3]),
        Instruction::Otir => Ok(vec![0xED, 0xB3]),

        // ----------------------------------------------------------------
        // CB prefix: bit operations
        // ----------------------------------------------------------------

        // BIT b, r — 0xCB, 0x40 | (b << 3) | r
        Instruction::Bit(b, r) => Ok(vec![0xCB, 0x40 | (bit_field(*b)? << 3) | r.field()]),

        // SET b, r — 0xCB, 0xC0 | (b << 3) | r
        Instruction::Set(b, r) => Ok(vec![0xCB, 0xC0 | (bit_field(*b)? << 3) | r.field()]),

        // RES b, r — 0xCB, 0x80 | (b << 3) | r
        Instruction::Res(b, r) => Ok(vec![0xCB, 0x80 | (bit_field(*b)? << 3) | r.field()]),

        // ----------------------------------------------------------------
        // ED prefix: 16-bit loads to/from memory
        // ----------------------------------------------------------------

        // LD (nn), rr — ED 43/53/63/73
        Instruction::LdAddrRR(nn, rr) => {
            let r = rr.field();
            Ok(vec![
                0xED,
                0x43 | (r << 4),
                (*nn & 0xFF) as u8,
                ((*nn >> 8) & 0xFF) as u8,
            ])
        }

        // LD rr, (nn) — ED 4B/5B/6B/7B
        Instruction::LdRRAddr(rr, nn) => {
            let r = rr.field();
            Ok(vec![
                0xED,
                0x4B | (r << 4),
                (*nn & 0xFF) as u8,
                ((*nn >> 8) & 0xFF) as u8,
            ])
        }

        // ----------------------------------------------------------------
        // Ix/Iy prefix: PUSH/POP
        // ----------------------------------------------------------------

        // PUSH Ix — DD E5, PUSH Iy — FD E5
        Instruction::PushIx(ix) => Ok(vec![ix.prefix(), 0xE5]),

        // POP Ix — DD E1, POP Iy — FD E1
        Instruction::PopIx(ix) => Ok(vec![ix.prefix(), 0xE1]),

        // ----------------------------------------------------------------
        // Remaining documented Z80 profile
        // ----------------------------------------------------------------
        Instruction::JpHL => Ok(vec![0xE9]),
        Instruction::RetCond(condition) => Ok(vec![0xC0 | (condition.field() << 3)]),
        Instruction::Reti => Ok(vec![0xED, 0x4D]),
        Instruction::Retn => Ok(vec![0xED, 0x45]),
        Instruction::Neg => Ok(vec![0xED, 0x44]),

        Instruction::InRC(operand) => Ok(vec![
            0xED,
            0x40 | (register_field("IN r,(C)", *operand)? << 3),
        ]),
        Instruction::OutCR(operand) => Ok(vec![
            0xED,
            0x41 | (register_field("OUT (C),r", *operand)? << 3),
        ]),

        Instruction::Ini => Ok(vec![0xED, 0xA2]),
        Instruction::Ind => Ok(vec![0xED, 0xAA]),
        Instruction::Inir => Ok(vec![0xED, 0xB2]),
        Instruction::Indr => Ok(vec![0xED, 0xBA]),
        Instruction::Outd => Ok(vec![0xED, 0xAB]),
        Instruction::Otdr => Ok(vec![0xED, 0xBB]),
        Instruction::Ldi => Ok(vec![0xED, 0xA0]),
        Instruction::Ldd => Ok(vec![0xED, 0xA8]),
        Instruction::Ldir => Ok(vec![0xED, 0xB0]),
        Instruction::Lddr => Ok(vec![0xED, 0xB8]),
        Instruction::Cpi => Ok(vec![0xED, 0xA1]),
        Instruction::Cpd => Ok(vec![0xED, 0xA9]),
        Instruction::Cpir => Ok(vec![0xED, 0xB1]),
        Instruction::Cpdr => Ok(vec![0xED, 0xB9]),

        Instruction::Rot(operation, operand) => {
            Ok(vec![0xCB, (operation.field() << 3) | operand.field()])
        }
        Instruction::SbcHLRR(register) => Ok(vec![0xED, 0x42 | (register.field() << 4)]),
        Instruction::AdcHLRR(register) => Ok(vec![0xED, 0x4A | (register.field() << 4)]),
        Instruction::LdIA => Ok(vec![0xED, 0x47]),
        Instruction::LdRA => Ok(vec![0xED, 0x4F]),
        Instruction::LdAI => Ok(vec![0xED, 0x57]),
        Instruction::LdAR => Ok(vec![0xED, 0x5F]),
        Instruction::Rrd => Ok(vec![0xED, 0x67]),
        Instruction::Rld => Ok(vec![0xED, 0x6F]),
        Instruction::Im(mode) => Ok(vec![
            0xED,
            match mode {
                InterruptMode::Im0 => 0x46,
                InterruptMode::Im1 => 0x56,
                InterruptMode::Im2 => 0x5E,
            },
        ]),

        Instruction::LdIxImm(index, value) => {
            let [low, high] = value.to_le_bytes();
            Ok(vec![index.prefix(), 0x21, low, high])
        }
        Instruction::LdIxAddr(index, address) => {
            let [low, high] = address.to_le_bytes();
            Ok(vec![index.prefix(), 0x2A, low, high])
        }
        Instruction::LdAddrIx(address, index) => {
            let [low, high] = address.to_le_bytes();
            Ok(vec![index.prefix(), 0x22, low, high])
        }
        Instruction::IncIx(index) => Ok(vec![index.prefix(), 0x23]),
        Instruction::DecIx(index) => Ok(vec![index.prefix(), 0x2B]),
        Instruction::AddIxRR(index, register) => {
            Ok(vec![index.prefix(), 0x09 | (register.field() << 4)])
        }
        Instruction::LdSpIx(index) => Ok(vec![index.prefix(), 0xF9]),
        Instruction::JpIx(index) => Ok(vec![index.prefix(), 0xE9]),
        Instruction::ExSpIx(index) => Ok(vec![index.prefix(), 0xE3]),

        Instruction::LdRIxDisp(destination, index, displacement) => Ok(vec![
            index.prefix(),
            0x46 | (register_field("LD r,(IX/IY+d)", *destination)? << 3),
            *displacement as u8,
        ]),
        Instruction::LdIxDispR(index, displacement, source) => Ok(vec![
            index.prefix(),
            0x70 | register_field("LD (IX/IY+d),r", *source)?,
            *displacement as u8,
        ]),
        Instruction::LdIxDispImm(index, displacement, immediate) => {
            Ok(vec![index.prefix(), 0x36, *displacement as u8, *immediate])
        }
        Instruction::IncIxDisp(index, displacement) => {
            Ok(vec![index.prefix(), 0x34, *displacement as u8])
        }
        Instruction::DecIxDisp(index, displacement) => {
            Ok(vec![index.prefix(), 0x35, *displacement as u8])
        }
        Instruction::AluIxDisp(operation, index, displacement) => Ok(vec![
            index.prefix(),
            0x86 | (operation.field() << 3),
            *displacement as u8,
        ]),
        Instruction::RotIxDisp(operation, index, displacement) => Ok(vec![
            index.prefix(),
            0xCB,
            *displacement as u8,
            (operation.field() << 3) | 6,
        ]),
        Instruction::BitIxDisp(bit, index, displacement) => Ok(vec![
            index.prefix(),
            0xCB,
            *displacement as u8,
            0x40 | (bit_field(*bit)? << 3) | 6,
        ]),
        Instruction::ResIxDisp(bit, index, displacement) => Ok(vec![
            index.prefix(),
            0xCB,
            *displacement as u8,
            0x80 | (bit_field(*bit)? << 3) | 6,
        ]),
        Instruction::SetIxDisp(bit, index, displacement) => Ok(vec![
            index.prefix(),
            0xCB,
            *displacement as u8,
            0xC0 | (bit_field(*bit)? << 3) | 6,
        ]),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VerifyError {
    #[error("Z80 program from 0x{origin:04X} with {length} bytes exceeds its address space")]
    AddressOverflow { origin: u16, length: usize },
    #[error("Z80 decode failed at byte offset {offset}: {source}")]
    Decode {
        offset: usize,
        #[source]
        source: DecodeError,
    },
    #[error(transparent)]
    Encode(#[from] EncodeError),
    #[error("Z80 byte round-trip mismatch at byte offset {offset}")]
    ByteMismatch { offset: usize },
}

/// Decode and re-encode a complete documented Z80 code byte sequence.
pub fn verify_placed_program(bytes: &[u8], origin: u16) -> Result<Vec<Instruction>, VerifyError> {
    if usize::from(origin) + bytes.len() > 0x1_0000 {
        return Err(VerifyError::AddressOverflow {
            origin,
            length: bytes.len(),
        });
    }
    let mut instructions = Vec::new();
    let mut offset = 0usize;
    while offset < bytes.len() {
        let decoded = decode_bytes(&bytes[offset..])
            .map_err(|source| VerifyError::Decode { offset, source })?;
        let length = decoded.length();
        let instruction = decoded.into_instruction();
        let actual = encode_bytes(&instruction)?;
        if actual.as_slice() != &bytes[offset..offset + length] {
            return Err(VerifyError::ByteMismatch { offset });
        }
        instructions.push(instruction);
        offset += length;
    }
    Ok(instructions)
}
