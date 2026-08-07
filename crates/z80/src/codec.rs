use crate::instruction::*;

/// Error returned when byte input is not one documented Z80 instruction.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    /// The source ends at the reported offset before the instruction is complete.
    #[error("unexpected end of input at offset {0}")]
    UnexpectedEnd(usize),
    /// Bytes select undocumented silicon behavior or an unsupported redundant form.
    #[error("unsupported or undocumented Z80 encoding: {bytes:02X?}")]
    UnsupportedEncoding {
        /// Leading source bytes sufficient to identify the rejected encoding.
        bytes: Vec<u8>,
    },
}

// ---------------------------------------------------------------------------
// Byte helpers
// ---------------------------------------------------------------------------

fn read_u8(bytes: &[u8], offset: usize) -> Result<u8, DecodeError> {
    bytes
        .get(offset)
        .copied()
        .ok_or(DecodeError::UnexpectedEnd(offset))
}

fn read_i8(bytes: &[u8], offset: usize) -> Result<i8, DecodeError> {
    read_u8(bytes, offset).map(|b| b as i8)
}

fn read_u16_le(bytes: &[u8], offset: usize) -> Result<u16, DecodeError> {
    let lo = read_u8(bytes, offset)? as u16;
    let hi = read_u8(bytes, offset + 1)? as u16;
    Ok((hi << 8) | lo)
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

fn decode_parts(bytes: &[u8]) -> Result<(Instruction, usize), DecodeError> {
    let b0 = read_u8(bytes, 0)?;
    match b0 {
        0xCB => decode_cb(bytes),
        0xDD => decode_dd_fd(bytes, IndexRegister::Ix),
        0xED => decode_ed(bytes),
        0xFD => decode_dd_fd(bytes, IndexRegister::Iy),
        _ => decode_unprefixed(bytes),
    }
}

/// One decoded instruction and the number of source bytes it consumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodedInstruction {
    instruction: Instruction,
    length: usize,
}

impl DecodedInstruction {
    /// Borrow the typed instruction identity.
    pub fn instruction(&self) -> &Instruction {
        &self.instruction
    }

    /// Consume the result and return its typed instruction identity.
    pub fn into_instruction(self) -> Instruction {
        self.instruction
    }

    /// Return the exact number of source bytes consumed.
    pub fn length(&self) -> usize {
        self.length
    }
}

/// Decode one documented Z80 instruction from the start of `bytes`.
pub fn decode_bytes(bytes: &[u8]) -> Result<DecodedInstruction, DecodeError> {
    let (instruction, length) = decode_parts(bytes)?;
    Ok(DecodedInstruction {
        instruction,
        length,
    })
}

// ---------------------------------------------------------------------------
// CB prefix
// ---------------------------------------------------------------------------

/// Decode a CB-prefixed instruction (bit ops, rotates, shifts).
///
/// Second byte bit-fields: x(7-6) selects ROT/BIT/RES/SET, y(5-3) is the bit
/// index or shift variant, z(2-0) selects the operand register.  All CB
/// instructions are exactly 2 bytes.
fn decode_cb(bytes: &[u8]) -> Result<(Instruction, usize), DecodeError> {
    let b = read_u8(bytes, 1)?;
    let x = (b >> 6) & 0x3;
    let y = (b >> 3) & 0x7;
    let z = b & 0x7;

    let inst = match x {
        0 => Instruction::Rot(
            RotateShiftOperation::from_field(y).ok_or_else(|| unsupported(bytes, 2))?,
            ByteOperand::from_field(z),
        ),
        1 => Instruction::Bit(y, ByteOperand::from_field(z)),
        2 => Instruction::Res(y, ByteOperand::from_field(z)),
        3 => Instruction::Set(y, ByteOperand::from_field(z)),
        _ => unreachable!(),
    };
    Ok((inst, 2))
}

fn unsupported(bytes: &[u8], length: usize) -> DecodeError {
    DecodeError::UnsupportedEncoding {
        bytes: bytes[..bytes.len().min(length)].to_vec(),
    }
}

// ---------------------------------------------------------------------------
// Prefix stubs (DD / ED / FD)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// ED prefix
// ---------------------------------------------------------------------------

fn decode_ed(bytes: &[u8]) -> Result<(Instruction, usize), DecodeError> {
    let b = read_u8(bytes, 1)?;
    let y = (b >> 3) & 0x7;
    let z = b & 0x7;
    let p = (y >> 1) & 0x3;
    let q = y & 0x1;

    let instruction = match b {
        0x40..=0x7f if z == 0 && y != 6 => Instruction::InRC(ByteOperand::from_field(y)),
        0x40..=0x7f if z == 1 && y != 6 => Instruction::OutCR(ByteOperand::from_field(y)),
        0x40..=0x7f if z == 2 && q == 0 => Instruction::SbcHLRR(Register16::from_field(p)),
        0x40..=0x7f if z == 2 => Instruction::AdcHLRR(Register16::from_field(p)),
        0x40..=0x7f if z == 3 => {
            let address = read_u16_le(bytes, 2)?;
            return Ok((
                if q == 0 {
                    Instruction::LdAddrRR(address, Register16::from_field(p))
                } else {
                    Instruction::LdRRAddr(Register16::from_field(p), address)
                },
                4,
            ));
        }
        0x44 => Instruction::Neg,
        0x45 => Instruction::Retn,
        0x4d => Instruction::Reti,
        0x46 => Instruction::Im(InterruptMode::Im0),
        0x56 => Instruction::Im(InterruptMode::Im1),
        0x5e => Instruction::Im(InterruptMode::Im2),
        0x47 => Instruction::LdIA,
        0x4f => Instruction::LdRA,
        0x57 => Instruction::LdAI,
        0x5f => Instruction::LdAR,
        0x67 => Instruction::Rrd,
        0x6f => Instruction::Rld,
        0xa0 => Instruction::Ldi,
        0xa1 => Instruction::Cpi,
        0xa2 => Instruction::Ini,
        0xa3 => Instruction::Outi,
        0xa8 => Instruction::Ldd,
        0xa9 => Instruction::Cpd,
        0xaa => Instruction::Ind,
        0xab => Instruction::Outd,
        0xb0 => Instruction::Ldir,
        0xb1 => Instruction::Cpir,
        0xb2 => Instruction::Inir,
        0xb3 => Instruction::Otir,
        0xb8 => Instruction::Lddr,
        0xb9 => Instruction::Cpdr,
        0xba => Instruction::Indr,
        0xbb => Instruction::Otdr,
        _ => return Err(unsupported(bytes, 2)),
    };
    Ok((instruction, 2))
}

// ---------------------------------------------------------------------------
// DD/FD prefix (Ix/Iy)
// ---------------------------------------------------------------------------

fn decode_dd_fd(bytes: &[u8], ix: IndexRegister) -> Result<(Instruction, usize), DecodeError> {
    let op2 = read_u8(bytes, 1)?;

    // DD CB / FD CB → DDCB/FDCB compound
    if op2 == 0xCB {
        return decode_ddcb(bytes, ix);
    }

    let x = (op2 >> 6) & 0x3;
    let y = (op2 >> 3) & 0x7;
    let z = op2 & 0x7;
    let p = (op2 >> 4) & 0x3;

    match x {
        0 => match op2 {
            // LD Ix, nn (21)
            0x21 => {
                let nn = read_u16_le(bytes, 2)?;
                Ok((Instruction::LdIxImm(ix, nn), 4))
            }
            // LD (nn), Ix (22)
            0x22 => {
                let nn = read_u16_le(bytes, 2)?;
                Ok((Instruction::LdAddrIx(nn, ix), 4))
            }
            // INC Ix (23)
            0x23 => Ok((Instruction::IncIx(ix), 2)),
            // LD Ix, (nn) (2A)
            0x2A => {
                let nn = read_u16_le(bytes, 2)?;
                Ok((Instruction::LdIxAddr(ix, nn), 4))
            }
            // DEC Ix (2B)
            0x2B => Ok((Instruction::DecIx(ix), 2)),
            // INC (Ix+d) (34)
            0x34 => {
                let d = read_i8(bytes, 2)?;
                Ok((Instruction::IncIxDisp(ix, d), 3))
            }
            // DEC (Ix+d) (35)
            0x35 => {
                let d = read_i8(bytes, 2)?;
                Ok((Instruction::DecIxDisp(ix, d), 3))
            }
            // LD (Ix+d), n (36)
            0x36 => {
                let d = read_i8(bytes, 2)?;
                let n = read_u8(bytes, 3)?;
                Ok((Instruction::LdIxDispImm(ix, d, n), 4))
            }
            // ADD Ix, rp[p] (09/19/29/39)
            0x09 | 0x19 | 0x29 | 0x39 => {
                // p = bits 5-4 = op2 >> 4 & 3
                // When p=2 (Hl), substitute Ix/Iy
                let rr = match p {
                    0 => Register16::Bc,
                    1 => Register16::De,
                    2 => Register16::Hl, // Ix+Ix or Iy+Iy conceptually, but we use Hl slot
                    3 => Register16::Sp,
                    _ => unreachable!(),
                };
                Ok((Instruction::AddIxRR(ix, rr), 2))
            }
            _ => Err(unsupported(bytes, 2)),
        },

        1 => {
            let dst_is_ind = y == 6;
            let src_is_ind = z == 6;

            if dst_is_ind && !src_is_ind {
                // LD (Ix+d), r
                let d = read_i8(bytes, 2)?;
                Ok((Instruction::LdIxDispR(ix, d, ByteOperand::from_field(z)), 3))
            } else if src_is_ind && !dst_is_ind {
                // LD r, (Ix+d)
                let d = read_i8(bytes, 2)?;
                Ok((Instruction::LdRIxDisp(ByteOperand::from_field(y), ix, d), 3))
            } else {
                Err(unsupported(bytes, 2))
            }
        }

        2 => {
            if z == 6 {
                // ALU A, (Ix+d)
                let d = read_i8(bytes, 2)?;
                Ok((
                    Instruction::AluIxDisp(AluOperation::from_field(y), ix, d),
                    3,
                ))
            } else {
                Err(unsupported(bytes, 2))
            }
        }

        3 => match op2 {
            0xE1 => Ok((Instruction::PopIx(ix), 2)),
            0xE3 => Ok((Instruction::ExSpIx(ix), 2)),
            0xE5 => Ok((Instruction::PushIx(ix), 2)),
            0xE9 => Ok((Instruction::JpIx(ix), 2)),
            0xF9 => Ok((Instruction::LdSpIx(ix), 2)),
            _ => Err(unsupported(bytes, 2)),
        },

        _ => unreachable!(),
    }
}

// ---------------------------------------------------------------------------
// DDCB/FDCB prefix (4-byte indexed bit operations)
// ---------------------------------------------------------------------------

fn decode_ddcb(bytes: &[u8], ix: IndexRegister) -> Result<(Instruction, usize), DecodeError> {
    // Format: [DD/FD] [CB] [disp] [op]
    let d = read_i8(bytes, 2)?;
    let op = read_u8(bytes, 3)?;
    let x = (op >> 6) & 0x3;
    let y = (op >> 3) & 0x7;
    let z = op & 0x7;

    if z != 6 {
        return Err(unsupported(bytes, 4));
    }

    let inst = match x {
        0 => Instruction::RotIxDisp(
            RotateShiftOperation::from_field(y).ok_or_else(|| unsupported(bytes, 4))?,
            ix,
            d,
        ),
        1 => Instruction::BitIxDisp(y, ix, d),
        2 => Instruction::ResIxDisp(y, ix, d),
        3 => Instruction::SetIxDisp(y, ix, d),
        _ => unreachable!(),
    };
    Ok((inst, 4))
}

// ---------------------------------------------------------------------------
// Unprefixed decoder
// ---------------------------------------------------------------------------

fn decode_unprefixed(bytes: &[u8]) -> Result<(Instruction, usize), DecodeError> {
    let b = bytes[0]; // already validated by caller
    let x = (b >> 6) & 0x3;
    let y = (b >> 3) & 0x7;
    let z = b & 0x7;
    let p = (b >> 4) & 0x3;
    let q = (b >> 3) & 0x1;

    match x {
        0 => decode_x0(bytes, y, z, p, q),
        1 => decode_x1(y, z),
        2 => Ok((
            Instruction::AluR(AluOperation::from_field(y), ByteOperand::from_field(z)),
            1,
        )),
        3 => decode_x3(bytes, y, z, p, q),
        _ => unreachable!(),
    }
}

// ---------------------------------------------------------------------------
// x = 0
// ---------------------------------------------------------------------------

fn decode_x0(
    bytes: &[u8],
    y: u8,
    z: u8,
    p: u8,
    q: u8,
) -> Result<(Instruction, usize), DecodeError> {
    match z {
        0 => match y {
            0 => Ok((Instruction::Nop, 1)),
            1 => Ok((Instruction::ExAFAF, 1)),
            2 => {
                let d = read_i8(bytes, 1)?;
                Ok((Instruction::Djnz(d), 2))
            }
            3 => {
                let d = read_i8(bytes, 1)?;
                Ok((Instruction::Jr(d), 2))
            }
            4..=7 => {
                let d = read_i8(bytes, 1)?;
                Ok((Instruction::JrCond(Condition::from_field(y - 4), d), 2))
            }
            _ => unreachable!(),
        },

        1 => {
            if q == 0 {
                let nn = read_u16_le(bytes, 1)?;
                Ok((Instruction::LdRRImm(Register16::from_field(p), nn), 3))
            } else {
                Ok((Instruction::AddHLRR(Register16::from_field(p)), 1))
            }
        }

        2 => {
            if q == 0 {
                let inst = match p {
                    0 => Instruction::LdBCA,
                    1 => Instruction::LdDEA,
                    2 => {
                        let nn = read_u16_le(bytes, 1)?;
                        return Ok((Instruction::LdAddrHL(nn), 3));
                    }
                    3 => {
                        let nn = read_u16_le(bytes, 1)?;
                        return Ok((Instruction::LdAddrA(nn), 3));
                    }
                    _ => unreachable!(),
                };
                Ok((inst, 1))
            } else {
                let inst = match p {
                    0 => Instruction::LdABC,
                    1 => Instruction::LdADE,
                    2 => {
                        let nn = read_u16_le(bytes, 1)?;
                        return Ok((Instruction::LdHLAddr(nn), 3));
                    }
                    3 => {
                        let nn = read_u16_le(bytes, 1)?;
                        return Ok((Instruction::LdAAddr(nn), 3));
                    }
                    _ => unreachable!(),
                };
                Ok((inst, 1))
            }
        }

        3 => {
            if q == 0 {
                Ok((Instruction::IncRR(Register16::from_field(p)), 1))
            } else {
                Ok((Instruction::DecRR(Register16::from_field(p)), 1))
            }
        }

        4 => Ok((Instruction::IncR(ByteOperand::from_field(y)), 1)),

        5 => Ok((Instruction::DecR(ByteOperand::from_field(y)), 1)),

        6 => {
            let n = read_u8(bytes, 1)?;
            Ok((Instruction::LdRImm(ByteOperand::from_field(y), n), 2))
        }

        7 => {
            let inst = match y {
                0 => Instruction::Rlca,
                1 => Instruction::Rrca,
                2 => Instruction::Rla,
                3 => Instruction::Rra,
                4 => Instruction::Daa,
                5 => Instruction::Cpl,
                6 => Instruction::Scf,
                7 => Instruction::Ccf,
                _ => unreachable!(),
            };
            Ok((inst, 1))
        }

        _ => unreachable!(),
    }
}

// ---------------------------------------------------------------------------
// x = 1  (LD r,r' or HALT)
// ---------------------------------------------------------------------------

fn decode_x1(y: u8, z: u8) -> Result<(Instruction, usize), DecodeError> {
    if y == 6 && z == 6 {
        Ok((Instruction::Halt, 1))
    } else {
        Ok((
            Instruction::LdRR(ByteOperand::from_field(y), ByteOperand::from_field(z)),
            1,
        ))
    }
}

// ---------------------------------------------------------------------------
// x = 3
// ---------------------------------------------------------------------------

fn decode_x3(
    bytes: &[u8],
    y: u8,
    z: u8,
    p: u8,
    q: u8,
) -> Result<(Instruction, usize), DecodeError> {
    match z {
        0 => Ok((Instruction::RetCond(Condition::from_field(y)), 1)),

        1 => {
            if q == 0 {
                Ok((Instruction::Pop(StackRegister::from_field(p)), 1))
            } else {
                let inst = match p {
                    0 => Instruction::Ret,
                    1 => Instruction::Exx,
                    2 => Instruction::JpHL,
                    3 => Instruction::LdSPHL,
                    _ => unreachable!(),
                };
                Ok((inst, 1))
            }
        }

        2 => {
            let nn = read_u16_le(bytes, 1)?;
            Ok((Instruction::JpCond(Condition::from_field(y), nn), 3))
        }

        3 => {
            let inst = match y {
                0 => {
                    let nn = read_u16_le(bytes, 1)?;
                    return Ok((Instruction::Jp(nn), 3));
                }
                1 => return decode_cb(bytes), // 0xCB prefix — re-route
                2 => {
                    let n = read_u8(bytes, 1)?;
                    return Ok((Instruction::OutPortA(n), 2));
                }
                3 => {
                    let n = read_u8(bytes, 1)?;
                    return Ok((Instruction::InAPort(n), 2));
                }
                4 => Instruction::ExSPHL,
                5 => Instruction::ExDEHL,
                6 => Instruction::Di,
                7 => Instruction::Ei,
                _ => unreachable!(),
            };
            Ok((inst, 1))
        }

        4 => {
            let nn = read_u16_le(bytes, 1)?;
            Ok((Instruction::CallCond(Condition::from_field(y), nn), 3))
        }

        5 => {
            if q == 0 {
                Ok((Instruction::Push(StackRegister::from_field(p)), 1))
            } else {
                match p {
                    0 => {
                        let nn = read_u16_le(bytes, 1)?;
                        Ok((Instruction::Call(nn), 3))
                    }
                    1 => decode_dd_fd(bytes, IndexRegister::Ix),
                    2 => decode_ed(bytes),
                    3 => decode_dd_fd(bytes, IndexRegister::Iy),
                    _ => unreachable!(),
                }
            }
        }

        6 => {
            let n = read_u8(bytes, 1)?;
            Ok((Instruction::AluImm(AluOperation::from_field(y), n), 2))
        }

        7 => Ok((Instruction::Rst(y * 8), 1)),

        _ => unreachable!(),
    }
}
