use thiserror::Error;

use crate::instruction::{GteCommand, GteOpcode, Instruction, Register};

/// Decode one PlayStation instruction from its natural 32-bit word.
pub fn decode_word(word: u32, pc: u32) -> Result<Instruction, DecodeError> {
    if pc & 3 != 0 {
        return Err(DecodeError::UnalignedInstructionAddress { address: pc });
    }
    match opcode(word) {
        0x00 => decode_special(word),
        0x01 => decode_regimm(word, pc),
        0x02 => Ok(Instruction::J {
            target: jump_target(pc, word & 0x03ff_ffff),
        }),
        0x03 => Ok(Instruction::Jal {
            target: jump_target(pc, word & 0x03ff_ffff),
        }),
        0x04 => Ok(Instruction::Beq {
            rs: rs(word),
            rt: rt(word),
            target: branch_target(pc, immediate_signed(word)),
        }),
        0x05 => Ok(Instruction::Bne {
            rs: rs(word),
            rt: rt(word),
            target: branch_target(pc, immediate_signed(word)),
        }),
        0x06 => {
            require(rt_index(word) == 0, word, "BLEZ rt field must be zero")?;
            Ok(Instruction::Blez {
                rs: rs(word),
                target: branch_target(pc, immediate_signed(word)),
            })
        }
        0x07 => {
            require(rt_index(word) == 0, word, "BGTZ rt field must be zero")?;
            Ok(Instruction::Bgtz {
                rs: rs(word),
                target: branch_target(pc, immediate_signed(word)),
            })
        }
        0x08 => Ok(Instruction::Addi {
            rt: rt(word),
            rs: rs(word),
            immediate: immediate_signed(word),
        }),
        0x09 => Ok(Instruction::Addiu {
            rt: rt(word),
            rs: rs(word),
            immediate: immediate_signed(word),
        }),
        0x0a => Ok(Instruction::Slti {
            rt: rt(word),
            rs: rs(word),
            immediate: immediate_signed(word),
        }),
        0x0b => Ok(Instruction::Sltiu {
            rt: rt(word),
            rs: rs(word),
            immediate: immediate_signed(word),
        }),
        0x0c => Ok(Instruction::Andi {
            rt: rt(word),
            rs: rs(word),
            immediate: word as u16,
        }),
        0x0d => Ok(Instruction::Ori {
            rt: rt(word),
            rs: rs(word),
            immediate: word as u16,
        }),
        0x0e => Ok(Instruction::Xori {
            rt: rt(word),
            rs: rs(word),
            immediate: word as u16,
        }),
        0x0f => {
            require(rs_index(word) == 0, word, "LUI rs field must be zero")?;
            Ok(Instruction::Lui {
                rt: rt(word),
                immediate: word as u16,
            })
        }
        0x10 => decode_cop0(word),
        0x12 => decode_cop2(word),
        0x20 => Ok(load(word, 0x20)),
        0x21 => Ok(load(word, 0x21)),
        0x22 => Ok(load(word, 0x22)),
        0x23 => Ok(load(word, 0x23)),
        0x24 => Ok(load(word, 0x24)),
        0x25 => Ok(load(word, 0x25)),
        0x26 => Ok(load(word, 0x26)),
        0x28 => Ok(store(word, 0x28)),
        0x29 => Ok(store(word, 0x29)),
        0x2a => Ok(store(word, 0x2a)),
        0x2b => Ok(store(word, 0x2b)),
        0x2e => Ok(store(word, 0x2e)),
        0x32 => Ok(Instruction::Lwc2 {
            rt: rt_index(word),
            base: rs(word),
            offset: immediate_signed(word),
        }),
        0x3a => Ok(Instruction::Swc2 {
            rt: rt_index(word),
            base: rs(word),
            offset: immediate_signed(word),
        }),
        unsupported => Err(DecodeError::UnsupportedOpcode {
            word,
            opcode: unsupported,
        }),
    }
}

/// Encode one PlayStation instruction as its natural 32-bit word.
pub fn encode_word(instruction: &Instruction, pc: u32) -> Result<u32, EncodeError> {
    if pc & 3 != 0 {
        return Err(EncodeError::UnalignedInstructionAddress { address: pc });
    }
    use Instruction::*;

    let word = match instruction {
        Add { rd, rs, rt } => r_type(*rs, *rt, *rd, 0, 0x20),
        Addu { rd, rs, rt } => r_type(*rs, *rt, *rd, 0, 0x21),
        Sub { rd, rs, rt } => r_type(*rs, *rt, *rd, 0, 0x22),
        Subu { rd, rs, rt } => r_type(*rs, *rt, *rd, 0, 0x23),
        And { rd, rs, rt } => r_type(*rs, *rt, *rd, 0, 0x24),
        Or { rd, rs, rt } => r_type(*rs, *rt, *rd, 0, 0x25),
        Xor { rd, rs, rt } => r_type(*rs, *rt, *rd, 0, 0x26),
        Nor { rd, rs, rt } => r_type(*rs, *rt, *rd, 0, 0x27),
        Slt { rd, rs, rt } => r_type(*rs, *rt, *rd, 0, 0x2a),
        Sltu { rd, rs, rt } => r_type(*rs, *rt, *rd, 0, 0x2b),
        Addi { rt, rs, immediate } => i_type(0x08, *rs, rt.index(), *immediate as u16),
        Addiu { rt, rs, immediate } => i_type(0x09, *rs, rt.index(), *immediate as u16),
        Slti { rt, rs, immediate } => i_type(0x0a, *rs, rt.index(), *immediate as u16),
        Sltiu { rt, rs, immediate } => i_type(0x0b, *rs, rt.index(), *immediate as u16),
        Andi { rt, rs, immediate } => i_type(0x0c, *rs, rt.index(), *immediate),
        Ori { rt, rs, immediate } => i_type(0x0d, *rs, rt.index(), *immediate),
        Xori { rt, rs, immediate } => i_type(0x0e, *rs, rt.index(), *immediate),
        Lui { rt, immediate } => i_type(0x0f, Register::ZERO, rt.index(), *immediate),
        Sll { rd, rt, shift } => {
            check_five_bit("shift", *shift)?;
            r_type(Register::ZERO, *rt, *rd, *shift, 0x00)
        }
        Srl { rd, rt, shift } => {
            check_five_bit("shift", *shift)?;
            r_type(Register::ZERO, *rt, *rd, *shift, 0x02)
        }
        Sra { rd, rt, shift } => {
            check_five_bit("shift", *shift)?;
            r_type(Register::ZERO, *rt, *rd, *shift, 0x03)
        }
        Sllv { rd, rt, rs } => r_type(*rs, *rt, *rd, 0, 0x04),
        Srlv { rd, rt, rs } => r_type(*rs, *rt, *rd, 0, 0x06),
        Srav { rd, rt, rs } => r_type(*rs, *rt, *rd, 0, 0x07),
        Jr { rs } => r_type(*rs, Register::ZERO, Register::ZERO, 0, 0x08),
        Jalr { rd, rs } => r_type(*rs, Register::ZERO, *rd, 0, 0x09),
        Syscall { code } => system_word(*code, 0x0c)?,
        Break { code } => system_word(*code, 0x0d)?,
        Mfhi { rd } => r_type(Register::ZERO, Register::ZERO, *rd, 0, 0x10),
        Mthi { rs } => r_type(*rs, Register::ZERO, Register::ZERO, 0, 0x11),
        Mflo { rd } => r_type(Register::ZERO, Register::ZERO, *rd, 0, 0x12),
        Mtlo { rs } => r_type(*rs, Register::ZERO, Register::ZERO, 0, 0x13),
        Mult { rs, rt } => r_type(*rs, *rt, Register::ZERO, 0, 0x18),
        Multu { rs, rt } => r_type(*rs, *rt, Register::ZERO, 0, 0x19),
        Div { rs, rt } => r_type(*rs, *rt, Register::ZERO, 0, 0x1a),
        Divu { rs, rt } => r_type(*rs, *rt, Register::ZERO, 0, 0x1b),
        Lb { rt, base, offset } => i_type(0x20, *base, rt.index(), *offset as u16),
        Lh { rt, base, offset } => i_type(0x21, *base, rt.index(), *offset as u16),
        Lwl { rt, base, offset } => i_type(0x22, *base, rt.index(), *offset as u16),
        Lw { rt, base, offset } => i_type(0x23, *base, rt.index(), *offset as u16),
        Lbu { rt, base, offset } => i_type(0x24, *base, rt.index(), *offset as u16),
        Lhu { rt, base, offset } => i_type(0x25, *base, rt.index(), *offset as u16),
        Lwr { rt, base, offset } => i_type(0x26, *base, rt.index(), *offset as u16),
        Sb { rt, base, offset } => i_type(0x28, *base, rt.index(), *offset as u16),
        Sh { rt, base, offset } => i_type(0x29, *base, rt.index(), *offset as u16),
        Swl { rt, base, offset } => i_type(0x2a, *base, rt.index(), *offset as u16),
        Sw { rt, base, offset } => i_type(0x2b, *base, rt.index(), *offset as u16),
        Swr { rt, base, offset } => i_type(0x2e, *base, rt.index(), *offset as u16),
        Beq { rs, rt, target } => branch_word(0x04, *rs, rt.index(), pc, *target)?,
        Bne { rs, rt, target } => branch_word(0x05, *rs, rt.index(), pc, *target)?,
        Blez { rs, target } => branch_word(0x06, *rs, 0, pc, *target)?,
        Bgtz { rs, target } => branch_word(0x07, *rs, 0, pc, *target)?,
        Bltz { rs, target } => branch_word(0x01, *rs, 0x00, pc, *target)?,
        Bgez { rs, target } => branch_word(0x01, *rs, 0x01, pc, *target)?,
        Bltzal { rs, target } => branch_word(0x01, *rs, 0x10, pc, *target)?,
        Bgezal { rs, target } => branch_word(0x01, *rs, 0x11, pc, *target)?,
        J { target } => jump_word(0x02, pc, *target)?,
        Jal { target } => jump_word(0x03, pc, *target)?,
        Mfc0 { rt, rd } => cop_transfer(0x10, 0x00, *rt, *rd)?,
        Mtc0 { rt, rd } => cop_transfer(0x10, 0x04, *rt, *rd)?,
        Rfe => 0x4200_0010,
        Mfc2 { rt, rd } => cop_transfer(0x12, 0x00, *rt, *rd)?,
        Cfc2 { rt, rd } => cop_transfer(0x12, 0x02, *rt, *rd)?,
        Mtc2 { rt, rd } => cop_transfer(0x12, 0x04, *rt, *rd)?,
        Ctc2 { rt, rd } => cop_transfer(0x12, 0x06, *rt, *rd)?,
        Lwc2 { rt, base, offset } => {
            check_five_bit("COP2 register", *rt)?;
            i_type(0x32, *base, *rt, *offset as u16)
        }
        Swc2 { rt, base, offset } => {
            check_five_bit("COP2 register", *rt)?;
            i_type(0x3a, *base, *rt, *offset as u16)
        }
        Gte(command) => encode_gte(*command)?,
    };
    Ok(word)
}

// Compatibility names from the source crate. New code uses names that state
// whether it is operating on a word or on serialized bytes.
#[doc(hidden)]
pub fn decode(word: u32, pc: u32) -> Result<Instruction, DecodeError> {
    decode_word(word, pc)
}

#[doc(hidden)]
pub fn encode(instruction: &Instruction, pc: u32) -> Result<u32, EncodeError> {
    encode_word(instruction, pc)
}

/// Decode one instruction from the first four bytes in PlayStation byte order.
pub fn decode_bytes(bytes: &[u8], pc: u32) -> Result<Instruction, DecodeError> {
    let Some(bytes) = bytes.get(..4) else {
        return Err(DecodeError::TruncatedInput { len: bytes.len() });
    };
    decode_word(
        u32::from_le_bytes(bytes.try_into().expect("four-byte instruction")),
        pc,
    )
}

/// Encode one instruction in PlayStation byte order.
pub fn encode_bytes(instruction: &Instruction, pc: u32) -> Result<[u8; 4], EncodeError> {
    Ok(encode_word(instruction, pc)?.to_le_bytes())
}

/// Decodes a whole little-endian instruction stream placed at `origin`.
pub fn decode_le_bytes(bytes: &[u8], origin: u32) -> Result<Vec<Instruction>, DecodeError> {
    if origin & 3 != 0 {
        return Err(DecodeError::UnalignedInstructionAddress { address: origin });
    }
    if bytes.len() % 4 != 0 {
        return Err(DecodeError::TruncatedInput { len: bytes.len() });
    }
    let mut instructions = Vec::with_capacity(bytes.len() / 4);
    for (index, chunk) in bytes.chunks_exact(4).enumerate() {
        let offset = u32::try_from(index)
            .ok()
            .and_then(|value| value.checked_mul(4))
            .ok_or(DecodeError::AddressOverflow { origin, index })?;
        let pc = origin
            .checked_add(offset)
            .ok_or(DecodeError::AddressOverflow { origin, index })?;
        instructions.push(decode_bytes(chunk, pc)?);
    }
    Ok(instructions)
}

/// Encodes a whole semantic instruction stream in PlayStation byte order.
pub fn encode_le_bytes(instructions: &[Instruction], origin: u32) -> Result<Vec<u8>, EncodeError> {
    if origin & 3 != 0 {
        return Err(EncodeError::UnalignedInstructionAddress { address: origin });
    }
    let mut output = Vec::with_capacity(instructions.len() * 4);
    for (index, instruction) in instructions.iter().enumerate() {
        let offset = u32::try_from(index)
            .ok()
            .and_then(|value| value.checked_mul(4))
            .ok_or(EncodeError::AddressOverflow { origin, index })?;
        let pc = origin
            .checked_add(offset)
            .ok_or(EncodeError::AddressOverflow { origin, index })?;
        output.extend_from_slice(&encode_bytes(instruction, pc)?);
    }
    Ok(output)
}

/// Disassemble placed code and require every instruction to re-encode to its
/// exact original word at the same address.
pub fn verify_placed_program(bytes: &[u8], origin: u32) -> Result<Vec<Instruction>, VerifyError> {
    let instructions = decode_le_bytes(bytes, origin).map_err(VerifyError::Decode)?;
    for (index, (instruction, chunk)) in instructions.iter().zip(bytes.chunks_exact(4)).enumerate()
    {
        let pc = origin + (index as u32) * 4;
        let original = u32::from_le_bytes(chunk.try_into().expect("chunk length is four"));
        let encoded = encode_word(instruction, pc).map_err(VerifyError::Encode)?;
        if encoded != original {
            return Err(VerifyError::RoundTripMismatch {
                pc,
                original,
                encoded,
            });
        }
    }
    Ok(instructions)
}

fn decode_special(word: u32) -> Result<Instruction, DecodeError> {
    let instruction = match function(word) {
        0x00 => {
            require(rs_index(word) == 0, word, "SLL rs field must be zero")?;
            Instruction::Sll {
                rd: rd(word),
                rt: rt(word),
                shift: shift(word),
            }
        }
        0x02 => {
            require(rs_index(word) == 0, word, "SRL rs field must be zero")?;
            Instruction::Srl {
                rd: rd(word),
                rt: rt(word),
                shift: shift(word),
            }
        }
        0x03 => {
            require(rs_index(word) == 0, word, "SRA rs field must be zero")?;
            Instruction::Sra {
                rd: rd(word),
                rt: rt(word),
                shift: shift(word),
            }
        }
        0x04 | 0x06 | 0x07 => {
            require(
                shift(word) == 0,
                word,
                "variable shift sa field must be zero",
            )?;
            match function(word) {
                0x04 => Instruction::Sllv {
                    rd: rd(word),
                    rt: rt(word),
                    rs: rs(word),
                },
                0x06 => Instruction::Srlv {
                    rd: rd(word),
                    rt: rt(word),
                    rs: rs(word),
                },
                _ => Instruction::Srav {
                    rd: rd(word),
                    rt: rt(word),
                    rs: rs(word),
                },
            }
        }
        0x08 => {
            require(
                rt_index(word) == 0 && rd_index(word) == 0 && shift(word) == 0,
                word,
                "JR reserved fields must be zero",
            )?;
            Instruction::Jr { rs: rs(word) }
        }
        0x09 => {
            require(
                rt_index(word) == 0 && shift(word) == 0,
                word,
                "JALR reserved fields must be zero",
            )?;
            Instruction::Jalr {
                rd: rd(word),
                rs: rs(word),
            }
        }
        0x0c => Instruction::Syscall {
            code: (word >> 6) & 0x000f_ffff,
        },
        0x0d => Instruction::Break {
            code: (word >> 6) & 0x000f_ffff,
        },
        0x10 | 0x12 => {
            require(
                rs_index(word) == 0 && rt_index(word) == 0 && shift(word) == 0,
                word,
                "MFHI/MFLO reserved fields must be zero",
            )?;
            if function(word) == 0x10 {
                Instruction::Mfhi { rd: rd(word) }
            } else {
                Instruction::Mflo { rd: rd(word) }
            }
        }
        0x11 | 0x13 => {
            require(
                rt_index(word) == 0 && rd_index(word) == 0 && shift(word) == 0,
                word,
                "MTHI/MTLO reserved fields must be zero",
            )?;
            if function(word) == 0x11 {
                Instruction::Mthi { rs: rs(word) }
            } else {
                Instruction::Mtlo { rs: rs(word) }
            }
        }
        0x18..=0x1b => {
            require(
                rd_index(word) == 0 && shift(word) == 0,
                word,
                "multiply/divide reserved fields must be zero",
            )?;
            match function(word) {
                0x18 => Instruction::Mult {
                    rs: rs(word),
                    rt: rt(word),
                },
                0x19 => Instruction::Multu {
                    rs: rs(word),
                    rt: rt(word),
                },
                0x1a => Instruction::Div {
                    rs: rs(word),
                    rt: rt(word),
                },
                _ => Instruction::Divu {
                    rs: rs(word),
                    rt: rt(word),
                },
            }
        }
        0x20..=0x27 | 0x2a | 0x2b => {
            require(shift(word) == 0, word, "ALU sa field must be zero")?;
            match function(word) {
                0x20 => Instruction::Add {
                    rd: rd(word),
                    rs: rs(word),
                    rt: rt(word),
                },
                0x21 => Instruction::Addu {
                    rd: rd(word),
                    rs: rs(word),
                    rt: rt(word),
                },
                0x22 => Instruction::Sub {
                    rd: rd(word),
                    rs: rs(word),
                    rt: rt(word),
                },
                0x23 => Instruction::Subu {
                    rd: rd(word),
                    rs: rs(word),
                    rt: rt(word),
                },
                0x24 => Instruction::And {
                    rd: rd(word),
                    rs: rs(word),
                    rt: rt(word),
                },
                0x25 => Instruction::Or {
                    rd: rd(word),
                    rs: rs(word),
                    rt: rt(word),
                },
                0x26 => Instruction::Xor {
                    rd: rd(word),
                    rs: rs(word),
                    rt: rt(word),
                },
                0x27 => Instruction::Nor {
                    rd: rd(word),
                    rs: rs(word),
                    rt: rt(word),
                },
                0x2a => Instruction::Slt {
                    rd: rd(word),
                    rs: rs(word),
                    rt: rt(word),
                },
                _ => Instruction::Sltu {
                    rd: rd(word),
                    rs: rs(word),
                    rt: rt(word),
                },
            }
        }
        unsupported => {
            return Err(DecodeError::UnsupportedFunction {
                word,
                function: unsupported,
            });
        }
    };
    Ok(instruction)
}

fn decode_regimm(word: u32, pc: u32) -> Result<Instruction, DecodeError> {
    let target = branch_target(pc, immediate_signed(word));
    match rt_index(word) {
        0x00 => Ok(Instruction::Bltz {
            rs: rs(word),
            target,
        }),
        0x01 => Ok(Instruction::Bgez {
            rs: rs(word),
            target,
        }),
        0x10 => Ok(Instruction::Bltzal {
            rs: rs(word),
            target,
        }),
        0x11 => Ok(Instruction::Bgezal {
            rs: rs(word),
            target,
        }),
        selector => Err(DecodeError::UnsupportedRegimm { word, selector }),
    }
}

fn decode_cop0(word: u32) -> Result<Instruction, DecodeError> {
    match rs_index(word) {
        0x00 => {
            require(word & 0x7ff == 0, word, "MFC0 low fields must be zero")?;
            Ok(Instruction::Mfc0 {
                rt: rt(word),
                rd: rd_index(word),
            })
        }
        0x04 => {
            require(word & 0x7ff == 0, word, "MTC0 low fields must be zero")?;
            Ok(Instruction::Mtc0 {
                rt: rt(word),
                rd: rd_index(word),
            })
        }
        0x10 if word == 0x4200_0010 => Ok(Instruction::Rfe),
        selector => Err(DecodeError::UnsupportedCoprocessorForm {
            word,
            coprocessor: 0,
            selector,
        }),
    }
}

fn decode_cop2(word: u32) -> Result<Instruction, DecodeError> {
    if word & 0x0200_0000 != 0 {
        require(
            word & 0x0000_1bc0 == 0,
            word,
            "GTE padding fields must be zero",
        )?;
        let function = (word & 0x3f) as u8;
        let opcode = GteOpcode::from_function(function)
            .ok_or(DecodeError::UnsupportedGteFunction { word, function })?;
        return Ok(Instruction::Gte(GteCommand {
            opcode,
            fake: ((word >> 20) & 0x1f) as u8,
            shift_fraction: word & (1 << 19) != 0,
            matrix: ((word >> 17) & 0x03) as u8,
            vector: ((word >> 15) & 0x03) as u8,
            control_vector: ((word >> 13) & 0x03) as u8,
            limit_unsigned: word & (1 << 10) != 0,
        }));
    }

    match rs_index(word) {
        0x00 | 0x02 | 0x04 | 0x06 => {
            require(
                word & 0x7ff == 0,
                word,
                "COP2 transfer low fields must be zero",
            )?;
            let instruction = match rs_index(word) {
                0x00 => Instruction::Mfc2 {
                    rt: rt(word),
                    rd: rd_index(word),
                },
                0x02 => Instruction::Cfc2 {
                    rt: rt(word),
                    rd: rd_index(word),
                },
                0x04 => Instruction::Mtc2 {
                    rt: rt(word),
                    rd: rd_index(word),
                },
                _ => Instruction::Ctc2 {
                    rt: rt(word),
                    rd: rd_index(word),
                },
            };
            Ok(instruction)
        }
        selector => Err(DecodeError::UnsupportedCoprocessorForm {
            word,
            coprocessor: 2,
            selector,
        }),
    }
}

fn load(word: u32, opcode: u8) -> Instruction {
    let rt = rt(word);
    let base = rs(word);
    let offset = immediate_signed(word);
    match opcode {
        0x20 => Instruction::Lb { rt, base, offset },
        0x21 => Instruction::Lh { rt, base, offset },
        0x22 => Instruction::Lwl { rt, base, offset },
        0x23 => Instruction::Lw { rt, base, offset },
        0x24 => Instruction::Lbu { rt, base, offset },
        0x25 => Instruction::Lhu { rt, base, offset },
        0x26 => Instruction::Lwr { rt, base, offset },
        _ => unreachable!(),
    }
}

fn store(word: u32, opcode: u8) -> Instruction {
    let rt = rt(word);
    let base = rs(word);
    let offset = immediate_signed(word);
    match opcode {
        0x28 => Instruction::Sb { rt, base, offset },
        0x29 => Instruction::Sh { rt, base, offset },
        0x2a => Instruction::Swl { rt, base, offset },
        0x2b => Instruction::Sw { rt, base, offset },
        0x2e => Instruction::Swr { rt, base, offset },
        _ => unreachable!(),
    }
}

fn encode_gte(command: GteCommand) -> Result<u32, EncodeError> {
    check_five_bit("GTE fake opcode", command.fake)?;
    check_two_bit("GTE matrix", command.matrix)?;
    check_two_bit("GTE vector", command.vector)?;
    check_two_bit("GTE control vector", command.control_vector)?;
    Ok(0x4a00_0000
        | u32::from(command.fake) << 20
        | u32::from(command.shift_fraction) << 19
        | u32::from(command.matrix) << 17
        | u32::from(command.vector) << 15
        | u32::from(command.control_vector) << 13
        | u32::from(command.limit_unsigned) << 10
        | u32::from(command.opcode.function()))
}

fn r_type(rs: Register, rt: Register, rd: Register, shift: u8, function: u8) -> u32 {
    u32::from(rs.index()) << 21
        | u32::from(rt.index()) << 16
        | u32::from(rd.index()) << 11
        | u32::from(shift) << 6
        | u32::from(function)
}

fn i_type(opcode: u8, rs: Register, rt: u8, immediate: u16) -> u32 {
    u32::from(opcode) << 26
        | u32::from(rs.index()) << 21
        | u32::from(rt) << 16
        | u32::from(immediate)
}

fn system_word(code: u32, function: u8) -> Result<u32, EncodeError> {
    if code > 0x000f_ffff {
        return Err(EncodeError::FieldOutOfRange {
            field: "system code",
            value: code,
            maximum: 0x000f_ffff,
        });
    }
    Ok(code << 6 | u32::from(function))
}

fn cop_transfer(opcode: u8, selector: u8, rt: Register, rd: u8) -> Result<u32, EncodeError> {
    check_five_bit("coprocessor register", rd)?;
    Ok(u32::from(opcode) << 26
        | u32::from(selector) << 21
        | u32::from(rt.index()) << 16
        | u32::from(rd) << 11)
}

fn branch_word(opcode: u8, rs: Register, rt: u8, pc: u32, target: u32) -> Result<u32, EncodeError> {
    if pc & 3 != 0 || target & 3 != 0 {
        return Err(EncodeError::UnalignedControlFlow { pc, target });
    }
    let displacement = target.wrapping_sub(pc.wrapping_add(4)) as i32;
    if displacement & 3 != 0 {
        return Err(EncodeError::UnalignedControlFlow { pc, target });
    }
    let words = displacement / 4;
    let immediate =
        i16::try_from(words).map_err(|_| EncodeError::BranchOutOfRange { pc, target })?;
    Ok(i_type(opcode, rs, rt, immediate as u16))
}

fn jump_word(opcode: u8, pc: u32, target: u32) -> Result<u32, EncodeError> {
    if pc & 3 != 0 || target & 3 != 0 {
        return Err(EncodeError::UnalignedControlFlow { pc, target });
    }
    let region = pc.wrapping_add(4) & 0xf000_0000;
    if target & 0xf000_0000 != region {
        return Err(EncodeError::JumpRegionMismatch { pc, target });
    }
    Ok(u32::from(opcode) << 26 | (target >> 2) & 0x03ff_ffff)
}

fn branch_target(pc: u32, immediate: i16) -> u32 {
    pc.wrapping_add(4)
        .wrapping_add(((immediate as i32) << 2) as u32)
}

fn jump_target(pc: u32, index: u32) -> u32 {
    (pc.wrapping_add(4) & 0xf000_0000) | (index << 2)
}

fn require(condition: bool, word: u32, reason: &'static str) -> Result<(), DecodeError> {
    if condition {
        Ok(())
    } else {
        Err(DecodeError::ReservedEncoding { word, reason })
    }
}

fn check_five_bit(field: &'static str, value: u8) -> Result<(), EncodeError> {
    if value < 32 {
        Ok(())
    } else {
        Err(EncodeError::FieldOutOfRange {
            field,
            value: u32::from(value),
            maximum: 31,
        })
    }
}

fn check_two_bit(field: &'static str, value: u8) -> Result<(), EncodeError> {
    if value < 4 {
        Ok(())
    } else {
        Err(EncodeError::FieldOutOfRange {
            field,
            value: u32::from(value),
            maximum: 3,
        })
    }
}

const fn opcode(word: u32) -> u8 {
    ((word >> 26) & 0x3f) as u8
}

const fn function(word: u32) -> u8 {
    (word & 0x3f) as u8
}

const fn rs_index(word: u32) -> u8 {
    ((word >> 21) & 0x1f) as u8
}

const fn rt_index(word: u32) -> u8 {
    ((word >> 16) & 0x1f) as u8
}

const fn rd_index(word: u32) -> u8 {
    ((word >> 11) & 0x1f) as u8
}

const fn rs(word: u32) -> Register {
    Register::from_field(rs_index(word))
}

const fn rt(word: u32) -> Register {
    Register::from_field(rt_index(word))
}

const fn rd(word: u32) -> Register {
    Register::from_field(rd_index(word))
}

const fn shift(word: u32) -> u8 {
    ((word >> 6) & 0x1f) as u8
}

const fn immediate_signed(word: u32) -> i16 {
    word as u16 as i16
}

/// Failure to decode one instruction or a placed instruction stream.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// A shared-protocol address exceeds the 32-bit PlayStation address space.
    #[error("instruction address {address:#x} exceeds the 32-bit PlayStation address space")]
    AddressOutOfRange {
        /// Rejected address.
        address: u64,
    },
    /// An instruction address is not four-byte aligned.
    #[error("instruction address {address:#010x} is not word-aligned")]
    UnalignedInstructionAddress {
        /// Rejected address.
        address: u32,
    },
    /// A byte stream does not contain a whole number of instructions.
    #[error("input length {len} is not a whole number of 32-bit instructions")]
    TruncatedInput {
        /// Supplied byte length.
        len: usize,
    },
    /// A stream instruction address overflows 32 bits.
    #[error("instruction address overflows for origin {origin:#010x}, index {index}")]
    AddressOverflow {
        /// Stream origin.
        origin: u32,
        /// Zero-based instruction index.
        index: usize,
    },
    /// The primary opcode is not supported by the declared profile.
    #[error("unsupported opcode {opcode:#04x} in word {word:#010x}")]
    UnsupportedOpcode {
        /// Rejected instruction word.
        word: u32,
        /// Unsupported six-bit primary opcode.
        opcode: u8,
    },
    /// A SPECIAL instruction uses an unsupported function selector.
    #[error("unsupported SPECIAL function {function:#04x} in word {word:#010x}")]
    UnsupportedFunction {
        /// Rejected instruction word.
        word: u32,
        /// Unsupported six-bit function selector.
        function: u8,
    },
    /// A REGIMM instruction uses an unsupported `rt` selector.
    #[error("unsupported REGIMM selector {selector:#04x} in word {word:#010x}")]
    UnsupportedRegimm {
        /// Rejected instruction word.
        word: u32,
        /// Unsupported five-bit REGIMM selector.
        selector: u8,
    },
    /// A coprocessor opcode uses a selector outside the PS1 profile.
    #[error("unsupported COP{coprocessor} selector {selector:#04x} in word {word:#010x}")]
    UnsupportedCoprocessorForm {
        /// Rejected instruction word.
        word: u32,
        /// Coprocessor number from the primary opcode.
        coprocessor: u8,
        /// Unsupported five-bit `rs` selector.
        selector: u8,
    },
    /// A COP2 command uses an undocumented GTE function selector.
    #[error("unsupported GTE function {function:#04x} in word {word:#010x}")]
    UnsupportedGteFunction {
        /// Rejected instruction word.
        word: u32,
        /// Unsupported six-bit GTE function.
        function: u8,
    },
    /// A recognized form has nonzero or otherwise invalid reserved fields.
    #[error("reserved encoding in word {word:#010x}: {reason}")]
    ReservedEncoding {
        /// Rejected instruction word.
        word: u32,
        /// Static description of the violated reserved-field rule.
        reason: &'static str,
    },
}

/// Failure to encode one instruction or a placed instruction stream.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EncodeError {
    /// A shared-protocol address exceeds the 32-bit PlayStation address space.
    #[error("instruction address {address:#x} exceeds the 32-bit PlayStation address space")]
    AddressOutOfRange {
        /// Rejected address.
        address: u64,
    },
    /// An instruction address is not four-byte aligned.
    #[error("instruction address {address:#010x} is not word-aligned")]
    UnalignedInstructionAddress {
        /// Rejected address.
        address: u32,
    },
    /// A semantic numeric field exceeds its encoded width.
    #[error("{field} value {value:#x} exceeds {maximum:#x}")]
    FieldOutOfRange {
        /// Stable field name.
        field: &'static str,
        /// Rejected value.
        value: u32,
        /// Largest encodable value.
        maximum: u32,
    },
    /// A stream instruction address overflows 32 bits.
    #[error("instruction address overflows for origin {origin:#010x}, index {index}")]
    AddressOverflow {
        /// Stream origin.
        origin: u32,
        /// Zero-based instruction index.
        index: usize,
    },
    /// A branch or jump target is not four-byte aligned.
    #[error("unaligned control flow from {pc:#010x} to {target:#010x}")]
    UnalignedControlFlow {
        /// Address of the control-flow instruction.
        pc: u32,
        /// Rejected target address.
        target: u32,
    },
    /// A PC-relative branch displacement does not fit 16 signed words.
    #[error("branch from {pc:#010x} to {target:#010x} exceeds signed 16-bit displacement")]
    BranchOutOfRange {
        /// Address of the branch instruction.
        pc: u32,
        /// Requested absolute target address.
        target: u32,
    },
    /// A direct jump target is outside the region selected by `pc + 4`.
    #[error("jump from {pc:#010x} to {target:#010x} crosses a 256 MiB region")]
    JumpRegionMismatch {
        /// Address of the jump instruction.
        pc: u32,
        /// Requested absolute target address.
        target: u32,
    },
}

/// Failure of placed-program decode and exact byte round-trip verification.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum VerifyError {
    /// Decoding the placed bytes failed.
    #[error("placed-program decode failed: {0}")]
    Decode(DecodeError),
    /// Re-encoding a decoded instruction at the same address failed.
    #[error("placed-program encode failed: {0}")]
    Encode(EncodeError),
    /// Re-encoding produced a different instruction word.
    #[error(
        "placed instruction at {pc:#010x} round-tripped as {encoded:#010x}, not {original:#010x}"
    )]
    RoundTripMismatch {
        /// Address of the mismatched instruction.
        pc: u32,
        /// Original instruction word.
        original: u32,
        /// Word produced by semantic re-encoding.
        encoded: u32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jump_uses_pc_plus_four_region_at_boundary() {
        let instruction = decode_word(0x0800_0000, 0x0fff_fffc).unwrap();
        assert_eq!(
            instruction,
            Instruction::J {
                target: 0x1000_0000
            }
        );
        assert_eq!(encode_word(&instruction, 0x0fff_fffc).unwrap(), 0x0800_0000);
    }

    #[test]
    fn reserved_fields_and_unknown_opcodes_fail_closed() {
        assert!(matches!(
            decode_word(0x0020_0000, 0x8001_0000),
            Err(DecodeError::ReservedEncoding { .. })
        ));
        assert!(matches!(
            decode_word(0xfc00_0000, 0x8001_0000),
            Err(DecodeError::UnsupportedOpcode { .. })
        ));
        assert!(matches!(
            decode_word(0x4a00_0000, 0x8001_0000),
            Err(DecodeError::UnsupportedGteFunction { .. })
        ));
    }

    #[test]
    fn instruction_addresses_must_be_word_aligned() {
        let address = 0x8001_0002;
        assert!(matches!(
            decode_word(0, address),
            Err(DecodeError::UnalignedInstructionAddress { .. })
        ));
        assert!(matches!(
            encode_word(&Instruction::nop(), address),
            Err(EncodeError::UnalignedInstructionAddress { .. })
        ));
        assert!(matches!(
            decode_le_bytes(&[], address),
            Err(DecodeError::UnalignedInstructionAddress { .. })
        ));
        assert!(matches!(
            encode_le_bytes(&[], address),
            Err(EncodeError::UnalignedInstructionAddress { .. })
        ));
    }

    #[test]
    fn all_documented_gte_functions_round_trip() {
        for (fake, opcode) in GteOpcode::ALL.into_iter().enumerate() {
            let instruction = Instruction::Gte(GteCommand {
                opcode,
                fake: fake as u8,
                shift_fraction: fake & 1 != 0,
                matrix: (fake & 3) as u8,
                vector: ((fake + 1) & 3) as u8,
                control_vector: ((fake + 2) & 3) as u8,
                limit_unsigned: fake & 2 != 0,
            });
            let word = encode_word(&instruction, 0x8001_0000).unwrap();
            assert_eq!(decode_word(word, 0x8001_0000).unwrap(), instruction);
        }
    }
}
