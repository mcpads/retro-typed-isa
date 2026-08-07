use thiserror::Error;

use crate::instruction::*;

#[derive(Debug, Clone, PartialEq, Eq)]
/// One decoded instruction and the number of source bytes it consumed.
pub struct DecodedInstruction {
    /// The decoded typed instruction.
    pub instruction: Instruction,
    /// The number of input bytes consumed.
    pub byte_len: usize,
}

/// Classification of every possible first instruction word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodedWord {
    /// The first word begins a supported instruction form.
    Instruction(InstructionForm),
    /// The first word is reserved or requires extensions that do not produce a valid form.
    Reserved,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
/// Failure returned while decoding an MC68000 instruction.
pub enum DecodeError {
    #[error("MC68000 instruction needs at least {needed} bytes, got {available}")]
    /// The input ends before the opcode's extension words are complete.
    TruncatedInput {
        /// The minimum total input length required.
        needed: usize,
        /// The available input length.
        available: usize,
    },
    #[error("reserved MC68000 instruction word 0x{word:04X}")]
    /// The first word is reserved or belongs to an excluded processor generation.
    ReservedOpcode {
        /// The rejected first word.
        word: u16,
    },
    #[error("reserved MC68000 extension word 0x{word:04X}")]
    /// An extension word uses a field combination reserved on the MC68000.
    ReservedExtension {
        /// The rejected extension word.
        word: u16,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
/// Failure returned when a typed instruction has no legal MC68000 encoding.
pub enum EncodeError {
    #[error("{mnemonic:?} does not support {size:?} operands")]
    /// The instruction family does not define the requested operand size.
    InvalidSize {
        /// The instruction family being encoded.
        mnemonic: Mnemonic,
        /// The rejected operand size.
        size: OperandSize,
    },
    #[error("{mnemonic:?} does not support {mode:?} addressing")]
    /// The instruction family does not admit the supplied effective address.
    InvalidEffectiveAddress {
        /// The instruction family being encoded.
        mnemonic: Mnemonic,
        /// The rejected effective-address mode.
        mode: AddressingMode,
    },
    #[error("immediate size {actual:?} does not match operand size {expected:?}")]
    /// An immediate's typed width differs from the instruction width.
    ImmediateSizeMismatch {
        /// The operand size required by the instruction.
        expected: OperandSize,
        /// The immediate's carried size.
        actual: OperandSize,
    },
    #[error("{mnemonic:?} immediate {value} is outside 1..=8")]
    /// A quick immediate is outside its architectural range.
    InvalidQuickImmediate {
        /// The instruction family being encoded.
        mnemonic: Mnemonic,
        /// The rejected immediate value.
        value: u8,
    },
    #[error("conditional branch selector cannot use condition {condition:?}")]
    /// A conditional-only instruction was given an unconditional selector.
    InvalidBranchCondition {
        /// The rejected condition.
        condition: Condition,
    },
    #[error("byte branch displacement zero selects the word-displacement encoding")]
    /// A zero byte displacement would select the word-displacement encoding instead.
    ZeroByteBranchDisplacement,
    #[error("trap vector {vector} is outside 0..=15")]
    /// A trap vector exceeds its four-bit encoding.
    InvalidTrapVector {
        /// The rejected vector.
        vector: u8,
    },
}

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn word(&mut self) -> Result<u16, DecodeError> {
        let end = self.position + 2;
        let Some(bytes) = self.bytes.get(self.position..end) else {
            return Err(DecodeError::TruncatedInput {
                needed: end,
                available: self.bytes.len(),
            });
        };
        self.position = end;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn long(&mut self) -> Result<u32, DecodeError> {
        let high = u32::from(self.word()?);
        let low = u32::from(self.word()?);
        Ok((high << 16) | low)
    }
}

/// Decode one MC68000 instruction from the start of `bytes`.
pub fn decode_bytes(bytes: &[u8]) -> Result<DecodedInstruction, DecodeError> {
    let mut reader = Reader::new(bytes);
    let word = reader.word()?;
    let instruction = decode_word(word, &mut reader)?;
    if encode_bytes(&instruction).is_err() {
        return Err(DecodeError::ReservedOpcode { word });
    }
    Ok(DecodedInstruction {
        instruction,
        byte_len: reader.position,
    })
}

/// Classify one possible first instruction word using zero-valued extensions.
pub fn classify_word(word: u16) -> DecodedWord {
    let mut bytes = [0_u8; 24];
    bytes[..2].copy_from_slice(&word.to_be_bytes());
    match decode_bytes(&bytes) {
        Ok(decoded) => DecodedWord::Instruction(decoded.instruction.form()),
        Err(_) => DecodedWord::Reserved,
    }
}

fn decode_word(word: u16, reader: &mut Reader<'_>) -> Result<Instruction, DecodeError> {
    match word >> 12 {
        0x0 => decode_group_0(word, reader),
        0x1 => decode_move(word, OperandSize::Byte, reader),
        0x2 => decode_move(word, OperandSize::Long, reader),
        0x3 => decode_move(word, OperandSize::Word, reader),
        0x4 => decode_group_4(word, reader),
        0x5 => decode_group_5(word, reader),
        0x6 => decode_branch(word, reader),
        0x7 => {
            if word & 0x0100 != 0 {
                return reserved(word);
            }
            Ok(Instruction::MoveQuick {
                immediate: word as u8 as i8,
                destination: DataRegister::from_bits(word >> 9),
            })
        }
        0x8 => decode_group_8(word, reader),
        0x9 => decode_add_sub(word, false, reader),
        0xA | 0xF => reserved(word),
        0xB => decode_group_b(word, reader),
        0xC => decode_group_c(word, reader),
        0xD => decode_add_sub(word, true, reader),
        0xE => decode_shift(word, reader),
        _ => unreachable!(),
    }
}

fn reserved<T>(word: u16) -> Result<T, DecodeError> {
    Err(DecodeError::ReservedOpcode { word })
}

fn standard_size(bits: u16, word: u16) -> Result<OperandSize, DecodeError> {
    match bits & 3 {
        0 => Ok(OperandSize::Byte),
        1 => Ok(OperandSize::Word),
        2 => Ok(OperandSize::Long),
        _ => reserved(word),
    }
}

fn address_size(bit: u16) -> AddressSize {
    if bit & 1 == 0 {
        AddressSize::Word
    } else {
        AddressSize::Long
    }
}

fn decode_immediate(size: OperandSize, reader: &mut Reader<'_>) -> Result<Immediate, DecodeError> {
    Ok(match size {
        OperandSize::Byte => {
            let word = reader.word()?;
            if word & 0xff00 != 0 {
                return Err(DecodeError::ReservedExtension { word });
            }
            Immediate::Byte(word as u8)
        }
        OperandSize::Word => Immediate::Word(reader.word()?),
        OperandSize::Long => Immediate::Long(reader.long()?),
    })
}

fn decode_byte_extension(reader: &mut Reader<'_>) -> Result<u8, DecodeError> {
    let word = reader.word()?;
    if word & 0xff00 != 0 {
        return Err(DecodeError::ReservedExtension { word });
    }
    Ok(word as u8)
}

fn decode_index(word: u16) -> Result<IndexRegister, DecodeError> {
    // The MC68000 brief extension has no scale or full-format fields.
    if word & 0x0700 != 0 {
        return Err(DecodeError::ReservedExtension { word });
    }
    let register = if word & 0x8000 == 0 {
        Register::Data(DataRegister::from_bits(word >> 12))
    } else {
        Register::Address(AddressRegister::from_bits(word >> 12))
    };
    let size = if word & 0x0800 == 0 {
        IndexSize::Word
    } else {
        IndexSize::Long
    };
    Ok(IndexRegister { register, size })
}

fn decode_ea(
    mode: u16,
    register: u16,
    size: OperandSize,
    reader: &mut Reader<'_>,
    first_word: u16,
) -> Result<EffectiveAddress, DecodeError> {
    Ok(match mode & 7 {
        0 => EffectiveAddress::DataRegister(DataRegister::from_bits(register)),
        1 => EffectiveAddress::AddressRegister(AddressRegister::from_bits(register)),
        2 => EffectiveAddress::AddressIndirect(AddressRegister::from_bits(register)),
        3 => EffectiveAddress::AddressPostincrement(AddressRegister::from_bits(register)),
        4 => EffectiveAddress::AddressPredecrement(AddressRegister::from_bits(register)),
        5 => EffectiveAddress::AddressDisplacement {
            register: AddressRegister::from_bits(register),
            displacement: reader.word()? as i16,
        },
        6 => {
            let extension = reader.word()?;
            EffectiveAddress::AddressIndex {
                register: AddressRegister::from_bits(register),
                index: decode_index(extension)?,
                displacement: extension as u8 as i8,
            }
        }
        7 => match register & 7 {
            0 => EffectiveAddress::AbsoluteWord(reader.word()?),
            1 => EffectiveAddress::AbsoluteLong(reader.long()?),
            2 => EffectiveAddress::ProgramCounterDisplacement(reader.word()? as i16),
            3 => {
                let extension = reader.word()?;
                EffectiveAddress::ProgramCounterIndex {
                    index: decode_index(extension)?,
                    displacement: extension as u8 as i8,
                }
            }
            4 => EffectiveAddress::Immediate(decode_immediate(size, reader)?),
            _ => return reserved(first_word),
        },
        _ => unreachable!(),
    })
}

fn decode_ea_low(
    word: u16,
    size: OperandSize,
    reader: &mut Reader<'_>,
) -> Result<EffectiveAddress, DecodeError> {
    decode_ea((word >> 3) & 7, word & 7, size, reader, word)
}

fn decode_group_0(word: u16, reader: &mut Reader<'_>) -> Result<Instruction, DecodeError> {
    match word {
        0x003c => {
            return Ok(Instruction::OrImmediateToConditionCode(
                decode_byte_extension(reader)?,
            ));
        }
        0x007c => return Ok(Instruction::OrImmediateToStatus(reader.word()?)),
        0x023c => {
            return Ok(Instruction::AndImmediateToConditionCode(
                decode_byte_extension(reader)?,
            ));
        }
        0x027c => return Ok(Instruction::AndImmediateToStatus(reader.word()?)),
        0x0a3c => {
            return Ok(Instruction::ExclusiveOrImmediateToConditionCode(
                decode_byte_extension(reader)?,
            ));
        }
        0x0a7c => return Ok(Instruction::ExclusiveOrImmediateToStatus(reader.word()?)),
        _ => {}
    }

    if word & 0xf138 == 0x0108 {
        let size = address_size((word >> 6) & 1);
        let direction = if word & 0x0080 == 0 {
            MovePeripheralDirection::MemoryToRegister
        } else {
            MovePeripheralDirection::RegisterToMemory
        };
        return Ok(Instruction::MovePeripheral {
            size,
            direction,
            data_register: DataRegister::from_bits(word >> 9),
            address_register: AddressRegister::from_bits(word),
            displacement: reader.word()? as i16,
        });
    }

    if word & 0xf100 == 0x0100 {
        let operation = bit_operation((word >> 6) & 3);
        let destination = decode_ea_low(word, bit_ea_size(word), reader)?;
        return Ok(Instruction::Bit {
            operation,
            bit_number: BitNumber::DataRegister(DataRegister::from_bits(word >> 9)),
            destination,
        });
    }

    if word & 0xff00 == 0x0800 {
        let operation = bit_operation((word >> 6) & 3);
        let extension = reader.word()?;
        if extension & 0xff00 != 0 {
            return Err(DecodeError::ReservedExtension { word: extension });
        }
        let destination = decode_ea_low(word, bit_ea_size(word), reader)?;
        return Ok(Instruction::Bit {
            operation,
            bit_number: BitNumber::Immediate(extension as u8),
            destination,
        });
    }

    let size = standard_size((word >> 6) & 3, word)?;
    let immediate = decode_immediate(size, reader)?;
    let destination = decode_ea_low(word, size, reader)?;
    Ok(match word & 0xff00 {
        0x0000 => Instruction::OrImmediate {
            size,
            immediate,
            destination,
        },
        0x0200 => Instruction::AndImmediate {
            size,
            immediate,
            destination,
        },
        0x0400 => Instruction::SubtractImmediate {
            size,
            immediate,
            destination,
        },
        0x0600 => Instruction::AddImmediate {
            size,
            immediate,
            destination,
        },
        0x0a00 => Instruction::ExclusiveOrImmediate {
            size,
            immediate,
            destination,
        },
        0x0c00 => Instruction::CompareImmediate {
            size,
            immediate,
            destination,
        },
        _ => return reserved(word),
    })
}

fn bit_operation(bits: u16) -> BitOperation {
    match bits & 3 {
        0 => BitOperation::Test,
        1 => BitOperation::Change,
        2 => BitOperation::Clear,
        3 => BitOperation::Set,
        _ => unreachable!(),
    }
}

fn bit_ea_size(word: u16) -> OperandSize {
    if word & 0x0038 == 0 {
        OperandSize::Long
    } else {
        OperandSize::Byte
    }
}

fn decode_move(
    word: u16,
    size: OperandSize,
    reader: &mut Reader<'_>,
) -> Result<Instruction, DecodeError> {
    let source = decode_ea_low(word, size, reader)?;
    let destination = decode_ea((word >> 6) & 7, (word >> 9) & 7, size, reader, word)?;
    if let EffectiveAddress::AddressRegister(destination) = destination {
        let size = match size {
            OperandSize::Word => AddressSize::Word,
            OperandSize::Long => AddressSize::Long,
            OperandSize::Byte => return reserved(word),
        };
        Ok(Instruction::MoveAddress {
            size,
            source,
            destination,
        })
    } else {
        Ok(Instruction::Move {
            size,
            source,
            destination,
        })
    }
}

fn decode_group_4(word: u16, reader: &mut Reader<'_>) -> Result<Instruction, DecodeError> {
    match word {
        0x4afc => return Ok(Instruction::Illegal),
        0x4e70 => return Ok(Instruction::Reset),
        0x4e71 => return Ok(Instruction::Nop),
        0x4e72 => {
            return Ok(Instruction::Stop {
                status: reader.word()?,
            });
        }
        0x4e73 => return Ok(Instruction::ReturnFromException),
        0x4e75 => return Ok(Instruction::ReturnFromSubroutine),
        0x4e76 => return Ok(Instruction::TrapOnOverflow),
        0x4e77 => return Ok(Instruction::ReturnAndRestore),
        _ => {}
    }
    if word & 0xfff0 == 0x4e40 {
        return Ok(Instruction::Trap {
            vector: (word & 0x0f) as u8,
        });
    }
    if word & 0xfff8 == 0x4e50 {
        return Ok(Instruction::Link {
            register: AddressRegister::from_bits(word),
            displacement: reader.word()? as i16,
        });
    }
    if word & 0xfff8 == 0x4e58 {
        return Ok(Instruction::Unlink {
            register: AddressRegister::from_bits(word),
        });
    }
    if word & 0xfff0 == 0x4e60 {
        let direction = if word & 8 == 0 {
            UserStackPointerDirection::AddressRegisterToUserStackPointer
        } else {
            UserStackPointerDirection::UserStackPointerToAddressRegister
        };
        return Ok(Instruction::MoveUserStackPointer {
            direction,
            register: AddressRegister::from_bits(word),
        });
    }
    if word & 0xffc0 == 0x4e80 {
        return Ok(Instruction::JumpSubroutine {
            target: decode_ea_low(word, OperandSize::Long, reader)?,
        });
    }
    if word & 0xffc0 == 0x4ec0 {
        return Ok(Instruction::Jump {
            target: decode_ea_low(word, OperandSize::Long, reader)?,
        });
    }
    if word & 0xffc0 == 0x40c0 {
        return Ok(Instruction::MoveFromStatus {
            destination: decode_ea_low(word, OperandSize::Word, reader)?,
        });
    }
    if word & 0xffc0 == 0x44c0 {
        return Ok(Instruction::MoveToConditionCode {
            source: decode_ea_low(word, OperandSize::Word, reader)?,
        });
    }
    if word & 0xffc0 == 0x46c0 {
        return Ok(Instruction::MoveToStatus {
            source: decode_ea_low(word, OperandSize::Word, reader)?,
        });
    }
    if word & 0xf1c0 == 0x4180 {
        return Ok(Instruction::Check {
            source: decode_ea_low(word, OperandSize::Word, reader)?,
            destination: DataRegister::from_bits(word >> 9),
        });
    }
    if word & 0xf1c0 == 0x41c0 {
        return Ok(Instruction::LoadEffectiveAddress {
            source: decode_ea_low(word, OperandSize::Long, reader)?,
            destination: AddressRegister::from_bits(word >> 9),
        });
    }
    if word & 0xffc0 == 0x4800 {
        return Ok(Instruction::NegateDecimal {
            destination: decode_ea_low(word, OperandSize::Byte, reader)?,
        });
    }
    if word & 0xfff8 == 0x4840 {
        return Ok(Instruction::Swap {
            register: DataRegister::from_bits(word),
        });
    }
    if word & 0xffc0 == 0x4840 {
        return Ok(Instruction::PushEffectiveAddress {
            source: decode_ea_low(word, OperandSize::Long, reader)?,
        });
    }
    if word & 0xfff8 == 0x4880 {
        return Ok(Instruction::Extend {
            size: ExtendSize::ByteToWord,
            register: DataRegister::from_bits(word),
        });
    }
    if word & 0xfff8 == 0x48c0 {
        return Ok(Instruction::Extend {
            size: ExtendSize::WordToLong,
            register: DataRegister::from_bits(word),
        });
    }
    if word & 0xfb80 == 0x4880 {
        let direction = if word & 0x0400 == 0 {
            MoveMultipleDirection::RegistersToMemory
        } else {
            MoveMultipleDirection::MemoryToRegisters
        };
        let size = if word & 0x0040 == 0 {
            AddressSize::Word
        } else {
            AddressSize::Long
        };
        let encoded_registers = reader.word()?;
        let effective_address = decode_ea_low(word, size.into(), reader)?;
        let registers = if direction == MoveMultipleDirection::RegistersToMemory
            && matches!(effective_address, EffectiveAddress::AddressPredecrement(_))
        {
            RegisterMask::from_bits(encoded_registers.reverse_bits())
        } else {
            RegisterMask::from_bits(encoded_registers)
        };
        return Ok(Instruction::MoveMultiple {
            size,
            direction,
            registers,
            effective_address,
        });
    }
    if word & 0xffc0 == 0x4ac0 {
        return Ok(Instruction::TestAndSet {
            destination: decode_ea_low(word, OperandSize::Byte, reader)?,
        });
    }

    let size = standard_size((word >> 6) & 3, word)?;
    let ea = decode_ea_low(word, size, reader)?;
    Ok(match word & 0xff00 {
        0x4000 => Instruction::NegateExtend {
            size,
            destination: ea,
        },
        0x4200 => Instruction::Clear {
            size,
            destination: ea,
        },
        0x4400 => Instruction::Negate {
            size,
            destination: ea,
        },
        0x4600 => Instruction::Not {
            size,
            destination: ea,
        },
        0x4a00 => Instruction::Test { size, source: ea },
        _ => return reserved(word),
    })
}

fn decode_group_5(word: u16, reader: &mut Reader<'_>) -> Result<Instruction, DecodeError> {
    if word & 0x00c0 == 0x00c0 {
        let condition = Condition::from_bits(word >> 8);
        if word & 0x0038 == 0x0008 {
            return Ok(Instruction::DecrementBranch {
                condition,
                register: DataRegister::from_bits(word),
                displacement: reader.word()? as i16,
            });
        }
        return Ok(Instruction::SetCondition {
            condition,
            destination: decode_ea_low(word, OperandSize::Byte, reader)?,
        });
    }
    let size = standard_size((word >> 6) & 3, word)?;
    let immediate = match (word >> 9) & 7 {
        0 => 8,
        value => value as u8,
    };
    let destination = decode_ea_low(word, size, reader)?;
    if word & 0x0100 == 0 {
        Ok(Instruction::AddQuick {
            size,
            immediate,
            destination,
        })
    } else {
        Ok(Instruction::SubtractQuick {
            size,
            immediate,
            destination,
        })
    }
}

fn decode_branch(word: u16, reader: &mut Reader<'_>) -> Result<Instruction, DecodeError> {
    let kind = match (word >> 8) & 0x0f {
        0 => BranchKind::Always,
        1 => BranchKind::Subroutine,
        condition => BranchKind::Conditional(Condition::from_bits(condition)),
    };
    let displacement = if word as u8 == 0 {
        BranchDisplacement::Word(reader.word()? as i16)
    } else {
        BranchDisplacement::Byte(word as u8 as i8)
    };
    Ok(Instruction::Branch { kind, displacement })
}

fn decode_register_pair(word: u16) -> RegisterPair {
    if word & 8 == 0 {
        RegisterPair::Data {
            source: DataRegister::from_bits(word),
            destination: DataRegister::from_bits(word >> 9),
        }
    } else {
        RegisterPair::AddressPredecrement {
            source: AddressRegister::from_bits(word),
            destination: AddressRegister::from_bits(word >> 9),
        }
    }
}

fn decode_group_8(word: u16, reader: &mut Reader<'_>) -> Result<Instruction, DecodeError> {
    if word & 0xf1f0 == 0x8100 {
        return Ok(Instruction::SubtractDecimal {
            operands: decode_register_pair(word),
        });
    }
    let opmode = (word >> 6) & 7;
    if opmode == 3 || opmode == 7 {
        return Ok(Instruction::Divide {
            sign: if opmode == 3 {
                DivisionSign::Unsigned
            } else {
                DivisionSign::Signed
            },
            source: decode_ea_low(word, OperandSize::Word, reader)?,
            destination: DataRegister::from_bits(word >> 9),
        });
    }
    decode_binary_data(word, reader, false)
}

fn decode_add_sub(
    word: u16,
    add: bool,
    reader: &mut Reader<'_>,
) -> Result<Instruction, DecodeError> {
    if word & 0xf130 == (if add { 0xd100 } else { 0x9100 }) && word & 0x00c0 != 0x00c0 {
        let size = standard_size((word >> 6) & 3, word)?;
        return Ok(if add {
            Instruction::AddExtend {
                size,
                operands: decode_register_pair(word),
            }
        } else {
            Instruction::SubtractExtend {
                size,
                operands: decode_register_pair(word),
            }
        });
    }
    let opmode = (word >> 6) & 7;
    if opmode == 3 || opmode == 7 {
        let size = address_size(opmode >> 2);
        let source = decode_ea_low(word, size.into(), reader)?;
        let destination = AddressRegister::from_bits(word >> 9);
        return Ok(if add {
            Instruction::AddAddress {
                size,
                source,
                destination,
            }
        } else {
            Instruction::SubtractAddress {
                size,
                source,
                destination,
            }
        });
    }
    let size = standard_size(opmode & 3, word)?;
    let direction = if opmode & 4 == 0 {
        DataDirection::ToDataRegister
    } else {
        DataDirection::ToEffectiveAddress
    };
    let effective_address = decode_ea_low(word, size, reader)?;
    let data_register = DataRegister::from_bits(word >> 9);
    Ok(if add {
        Instruction::Add {
            size,
            direction,
            data_register,
            effective_address,
        }
    } else {
        Instruction::Subtract {
            size,
            direction,
            data_register,
            effective_address,
        }
    })
}

fn decode_group_b(word: u16, reader: &mut Reader<'_>) -> Result<Instruction, DecodeError> {
    if word & 0xf138 == 0xb108 && word & 0x00c0 != 0x00c0 {
        return Ok(Instruction::CompareMemory {
            size: standard_size((word >> 6) & 3, word)?,
            source: AddressRegister::from_bits(word),
            destination: AddressRegister::from_bits(word >> 9),
        });
    }
    let opmode = (word >> 6) & 7;
    if opmode == 3 || opmode == 7 {
        let size = address_size(opmode >> 2);
        return Ok(Instruction::CompareAddress {
            size,
            source: decode_ea_low(word, size.into(), reader)?,
            destination: AddressRegister::from_bits(word >> 9),
        });
    }
    let size = standard_size(opmode & 3, word)?;
    if opmode & 4 == 0 {
        Ok(Instruction::Compare {
            size,
            source: decode_ea_low(word, size, reader)?,
            destination: DataRegister::from_bits(word >> 9),
        })
    } else {
        Ok(Instruction::ExclusiveOr {
            size,
            source: DataRegister::from_bits(word >> 9),
            destination: decode_ea_low(word, size, reader)?,
        })
    }
}

fn decode_group_c(word: u16, reader: &mut Reader<'_>) -> Result<Instruction, DecodeError> {
    if word & 0xf1f0 == 0xc100 {
        return Ok(Instruction::AddDecimal {
            operands: decode_register_pair(word),
        });
    }
    let registers = match word & 0xf1f8 {
        0xc140 => Some(ExchangeRegisters::Data {
            source: DataRegister::from_bits(word),
            destination: DataRegister::from_bits(word >> 9),
        }),
        0xc148 => Some(ExchangeRegisters::Address {
            source: AddressRegister::from_bits(word),
            destination: AddressRegister::from_bits(word >> 9),
        }),
        0xc188 => Some(ExchangeRegisters::DataAddress {
            data: DataRegister::from_bits(word >> 9),
            address: AddressRegister::from_bits(word),
        }),
        _ => None,
    };
    if let Some(registers) = registers {
        return Ok(Instruction::Exchange { registers });
    }
    let opmode = (word >> 6) & 7;
    if opmode == 3 || opmode == 7 {
        return Ok(Instruction::Multiply {
            sign: if opmode == 3 {
                MultiplicationSign::Unsigned
            } else {
                MultiplicationSign::Signed
            },
            source: decode_ea_low(word, OperandSize::Word, reader)?,
            destination: DataRegister::from_bits(word >> 9),
        });
    }
    decode_binary_data(word, reader, true)
}

fn decode_binary_data(
    word: u16,
    reader: &mut Reader<'_>,
    and: bool,
) -> Result<Instruction, DecodeError> {
    let opmode = (word >> 6) & 7;
    let size = standard_size(opmode & 3, word)?;
    let direction = if opmode & 4 == 0 {
        DataDirection::ToDataRegister
    } else {
        DataDirection::ToEffectiveAddress
    };
    let data_register = DataRegister::from_bits(word >> 9);
    let effective_address = decode_ea_low(word, size, reader)?;
    Ok(if and {
        Instruction::And {
            size,
            direction,
            data_register,
            effective_address,
        }
    } else {
        Instruction::Or {
            size,
            direction,
            data_register,
            effective_address,
        }
    })
}

fn decode_shift(word: u16, reader: &mut Reader<'_>) -> Result<Instruction, DecodeError> {
    if word & 0x00c0 == 0x00c0 {
        if word & 0x0800 != 0 {
            return reserved(word);
        }
        let operation = match (word >> 9) & 3 {
            0 => ShiftOperation::Arithmetic,
            1 => ShiftOperation::Logical,
            2 => ShiftOperation::RotateWithExtend,
            3 => ShiftOperation::Rotate,
            _ => unreachable!(),
        };
        let direction = if word & 0x0100 == 0 {
            ShiftDirection::Right
        } else {
            ShiftDirection::Left
        };
        return Ok(Instruction::ShiftMemory {
            operation,
            direction,
            destination: decode_ea_low(word, OperandSize::Word, reader)?,
        });
    }
    let direction = if word & 0x0100 == 0 {
        ShiftDirection::Right
    } else {
        ShiftDirection::Left
    };
    let size = standard_size((word >> 6) & 3, word)?;
    let count = if word & 0x0020 == 0 {
        ShiftCount::Immediate(match (word >> 9) & 7 {
            0 => 8,
            value => value as u8,
        })
    } else {
        ShiftCount::DataRegister(DataRegister::from_bits(word >> 9))
    };
    let operation = match (word >> 3) & 3 {
        0 => ShiftOperation::Arithmetic,
        1 => ShiftOperation::Logical,
        2 => ShiftOperation::RotateWithExtend,
        3 => ShiftOperation::Rotate,
        _ => unreachable!(),
    };
    Ok(Instruction::ShiftRegister {
        operation,
        direction,
        size,
        count,
        destination: DataRegister::from_bits(word),
    })
}

struct EncodedEffectiveAddress {
    field: u16,
    extensions: Vec<u8>,
}

/// Lower one typed instruction to its canonical big-endian MC68000 bytes.
pub fn encode_bytes(instruction: &Instruction) -> Result<Vec<u8>, EncodeError> {
    let mut bytes = Vec::new();
    encode_instruction(instruction, &mut bytes)?;
    Ok(bytes)
}

fn encode_instruction(instruction: &Instruction, bytes: &mut Vec<u8>) -> Result<(), EncodeError> {
    match instruction {
        Instruction::AddDecimal { operands } => {
            push_word(bytes, 0xc100 | encode_pair(*operands));
        }
        Instruction::SubtractDecimal { operands } => {
            push_word(bytes, 0x8100 | encode_pair(*operands));
        }
        Instruction::Add {
            size,
            direction,
            data_register,
            effective_address,
        } => encode_data_binary(
            bytes,
            0xd000,
            Mnemonic::Add,
            *size,
            *direction,
            *data_register,
            *effective_address,
        )?,
        Instruction::Subtract {
            size,
            direction,
            data_register,
            effective_address,
        } => encode_data_binary(
            bytes,
            0x9000,
            Mnemonic::Sub,
            *size,
            *direction,
            *data_register,
            *effective_address,
        )?,
        Instruction::And {
            size,
            direction,
            data_register,
            effective_address,
        } => encode_data_binary(
            bytes,
            0xc000,
            Mnemonic::And,
            *size,
            *direction,
            *data_register,
            *effective_address,
        )?,
        Instruction::Or {
            size,
            direction,
            data_register,
            effective_address,
        } => encode_data_binary(
            bytes,
            0x8000,
            Mnemonic::Or,
            *size,
            *direction,
            *data_register,
            *effective_address,
        )?,
        Instruction::AddAddress {
            size,
            source,
            destination,
        } => encode_address_binary(bytes, 0xd0c0, *size, *source, *destination)?,
        Instruction::SubtractAddress {
            size,
            source,
            destination,
        } => encode_address_binary(bytes, 0x90c0, *size, *source, *destination)?,
        Instruction::CompareAddress {
            size,
            source,
            destination,
        } => encode_address_binary(bytes, 0xb0c0, *size, *source, *destination)?,
        Instruction::AddImmediate {
            size,
            immediate,
            destination,
        } => encode_immediate_operation(
            bytes,
            0x0600,
            Mnemonic::Addi,
            *size,
            *immediate,
            *destination,
        )?,
        Instruction::SubtractImmediate {
            size,
            immediate,
            destination,
        } => encode_immediate_operation(
            bytes,
            0x0400,
            Mnemonic::Subi,
            *size,
            *immediate,
            *destination,
        )?,
        Instruction::AndImmediate {
            size,
            immediate,
            destination,
        } => encode_immediate_operation(
            bytes,
            0x0200,
            Mnemonic::Andi,
            *size,
            *immediate,
            *destination,
        )?,
        Instruction::OrImmediate {
            size,
            immediate,
            destination,
        } => encode_immediate_operation(
            bytes,
            0x0000,
            Mnemonic::Ori,
            *size,
            *immediate,
            *destination,
        )?,
        Instruction::ExclusiveOrImmediate {
            size,
            immediate,
            destination,
        } => encode_immediate_operation(
            bytes,
            0x0a00,
            Mnemonic::Eori,
            *size,
            *immediate,
            *destination,
        )?,
        Instruction::CompareImmediate {
            size,
            immediate,
            destination,
        } => encode_immediate_operation(
            bytes,
            0x0c00,
            Mnemonic::Cmpi,
            *size,
            *immediate,
            *destination,
        )?,
        Instruction::AddQuick {
            size,
            immediate,
            destination,
        } => encode_quick(
            bytes,
            0x5000,
            Mnemonic::Addq,
            *size,
            *immediate,
            *destination,
        )?,
        Instruction::SubtractQuick {
            size,
            immediate,
            destination,
        } => encode_quick(
            bytes,
            0x5100,
            Mnemonic::Subq,
            *size,
            *immediate,
            *destination,
        )?,
        Instruction::AddExtend { size, operands } => {
            push_word(
                bytes,
                0xd100 | (size_bits(*size) << 6) | encode_pair(*operands),
            );
        }
        Instruction::SubtractExtend { size, operands } => {
            push_word(
                bytes,
                0x9100 | (size_bits(*size) << 6) | encode_pair(*operands),
            );
        }
        Instruction::AndImmediateToConditionCode(value) => {
            push_word(bytes, 0x023c);
            push_word(bytes, u16::from(*value));
        }
        Instruction::AndImmediateToStatus(value) => {
            push_word(bytes, 0x027c);
            push_word(bytes, *value);
        }
        Instruction::OrImmediateToConditionCode(value) => {
            push_word(bytes, 0x003c);
            push_word(bytes, u16::from(*value));
        }
        Instruction::OrImmediateToStatus(value) => {
            push_word(bytes, 0x007c);
            push_word(bytes, *value);
        }
        Instruction::ExclusiveOrImmediateToConditionCode(value) => {
            push_word(bytes, 0x0a3c);
            push_word(bytes, u16::from(*value));
        }
        Instruction::ExclusiveOrImmediateToStatus(value) => {
            push_word(bytes, 0x0a7c);
            push_word(bytes, *value);
        }
        Instruction::ShiftRegister {
            operation,
            direction,
            size,
            count,
            destination,
        } => {
            let (count_bits, register_count) = match count {
                ShiftCount::Immediate(value) => (
                    encode_quick_value(shift_mnemonic(*operation, *direction), *value)?,
                    0,
                ),
                ShiftCount::DataRegister(register) => (register.bits(), 0x20),
            };
            push_word(
                bytes,
                0xe000
                    | (count_bits << 9)
                    | (direction_bit(*direction) << 8)
                    | (size_bits(*size) << 6)
                    | register_count
                    | (shift_operation_bits(*operation) << 3)
                    | destination.bits(),
            );
        }
        Instruction::ShiftMemory {
            operation,
            direction,
            destination,
        } => {
            require_ea(
                shift_mnemonic(*operation, *direction),
                *destination,
                is_memory_alterable,
            )?;
            let ea = encode_effective_address(*destination, OperandSize::Word)?;
            push_word(
                bytes,
                0xe0c0
                    | (shift_operation_bits(*operation) << 9)
                    | (direction_bit(*direction) << 8)
                    | ea.field,
            );
            bytes.extend_from_slice(&ea.extensions);
        }
        Instruction::Branch { kind, displacement } => {
            let condition = match kind {
                BranchKind::Always => 0,
                BranchKind::Subroutine => 1,
                BranchKind::Conditional(condition) => {
                    if matches!(condition, Condition::True | Condition::False) {
                        return Err(EncodeError::InvalidBranchCondition {
                            condition: *condition,
                        });
                    }
                    condition.bits()
                }
            };
            match displacement {
                BranchDisplacement::Byte(0) => {
                    return Err(EncodeError::ZeroByteBranchDisplacement);
                }
                BranchDisplacement::Byte(displacement) => {
                    push_word(
                        bytes,
                        0x6000 | (condition << 8) | u16::from(*displacement as u8),
                    );
                }
                BranchDisplacement::Word(displacement) => {
                    push_word(bytes, 0x6000 | (condition << 8));
                    push_word(bytes, *displacement as u16);
                }
            }
        }
        Instruction::Bit {
            operation,
            bit_number,
            destination,
        } => {
            let legal_destination = match (operation, bit_number) {
                (BitOperation::Test, BitNumber::DataRegister(_)) => is_data(*destination),
                (BitOperation::Test, BitNumber::Immediate(_)) => {
                    is_data(*destination) && !matches!(destination, EffectiveAddress::Immediate(_))
                }
                _ => is_data_alterable(*destination),
            };
            if !legal_destination {
                return Err(EncodeError::InvalidEffectiveAddress {
                    mnemonic: bit_mnemonic(*operation),
                    mode: destination.mode(),
                });
            }
            let ea = encode_effective_address(*destination, bit_size(*destination))?;
            let operation = bit_operation_bits(*operation) << 6;
            match bit_number {
                BitNumber::DataRegister(register) => {
                    push_word(
                        bytes,
                        0x0100 | (register.bits() << 9) | operation | ea.field,
                    );
                }
                BitNumber::Immediate(value) => {
                    push_word(bytes, 0x0800 | operation | ea.field);
                    push_word(bytes, u16::from(*value));
                }
            }
            bytes.extend_from_slice(&ea.extensions);
        }
        Instruction::Check {
            source,
            destination,
        } => encode_word_ea(
            bytes,
            0x4180 | (destination.bits() << 9),
            Mnemonic::Chk,
            *source,
            OperandSize::Word,
            is_data,
        )?,
        Instruction::Clear { size, destination } => encode_unary(
            bytes,
            0x4200,
            Mnemonic::Clr,
            *size,
            *destination,
            is_data_alterable,
        )?,
        Instruction::Compare {
            size,
            source,
            destination,
        } => {
            if !is_all_source_for_size(*source, *size) {
                return Err(EncodeError::InvalidEffectiveAddress {
                    mnemonic: Mnemonic::Cmp,
                    mode: source.mode(),
                });
            }
            encode_word_ea(
                bytes,
                0xb000 | (destination.bits() << 9) | (size_bits(*size) << 6),
                Mnemonic::Cmp,
                *source,
                *size,
                |_| true,
            )?;
        }
        Instruction::CompareMemory {
            size,
            source,
            destination,
        } => {
            push_word(
                bytes,
                0xb108 | (destination.bits() << 9) | (size_bits(*size) << 6) | source.bits(),
            );
        }
        Instruction::DecrementBranch {
            condition,
            register,
            displacement,
        } => {
            push_word(bytes, 0x50c8 | (condition.bits() << 8) | register.bits());
            push_word(bytes, *displacement as u16);
        }
        Instruction::Divide {
            sign,
            source,
            destination,
        } => {
            let base = match sign {
                DivisionSign::Unsigned => 0x80c0,
                DivisionSign::Signed => 0x81c0,
            };
            encode_word_ea(
                bytes,
                base | (destination.bits() << 9),
                match sign {
                    DivisionSign::Unsigned => Mnemonic::Divu,
                    DivisionSign::Signed => Mnemonic::Divs,
                },
                *source,
                OperandSize::Word,
                is_data,
            )?;
        }
        Instruction::ExclusiveOr {
            size,
            source,
            destination,
        } => encode_word_ea(
            bytes,
            0xb100 | (source.bits() << 9) | (size_bits(*size) << 6),
            Mnemonic::Eor,
            *destination,
            *size,
            is_data_alterable,
        )?,
        Instruction::Exchange { registers } => {
            let word = match registers {
                ExchangeRegisters::Data {
                    source,
                    destination,
                } => 0xc140 | (destination.bits() << 9) | source.bits(),
                ExchangeRegisters::Address {
                    source,
                    destination,
                } => 0xc148 | (destination.bits() << 9) | source.bits(),
                ExchangeRegisters::DataAddress { data, address } => {
                    0xc188 | (data.bits() << 9) | address.bits()
                }
            };
            push_word(bytes, word);
        }
        Instruction::Extend { size, register } => {
            let base = match size {
                ExtendSize::ByteToWord => 0x4880,
                ExtendSize::WordToLong => 0x48c0,
            };
            push_word(bytes, base | register.bits());
        }
        Instruction::Illegal => push_word(bytes, 0x4afc),
        Instruction::Jump { target } => encode_word_ea(
            bytes,
            0x4ec0,
            Mnemonic::Jmp,
            *target,
            OperandSize::Long,
            is_control,
        )?,
        Instruction::JumpSubroutine { target } => encode_word_ea(
            bytes,
            0x4e80,
            Mnemonic::Jsr,
            *target,
            OperandSize::Long,
            is_control,
        )?,
        Instruction::LoadEffectiveAddress {
            source,
            destination,
        } => encode_word_ea(
            bytes,
            0x41c0 | (destination.bits() << 9),
            Mnemonic::Lea,
            *source,
            OperandSize::Long,
            is_control,
        )?,
        Instruction::Link {
            register,
            displacement,
        } => {
            push_word(bytes, 0x4e50 | register.bits());
            push_word(bytes, *displacement as u16);
        }
        Instruction::Move {
            size,
            source,
            destination,
        } => encode_move(bytes, *size, *source, *destination)?,
        Instruction::MoveAddress {
            size,
            source,
            destination,
        } => encode_move_address(bytes, *size, *source, *destination)?,
        Instruction::MoveToConditionCode { source } => encode_word_ea(
            bytes,
            0x44c0,
            Mnemonic::Move,
            *source,
            OperandSize::Word,
            is_data,
        )?,
        Instruction::MoveFromStatus { destination } => encode_word_ea(
            bytes,
            0x40c0,
            Mnemonic::Move,
            *destination,
            OperandSize::Word,
            is_data_alterable,
        )?,
        Instruction::MoveToStatus { source } => encode_word_ea(
            bytes,
            0x46c0,
            Mnemonic::Move,
            *source,
            OperandSize::Word,
            is_data,
        )?,
        Instruction::MoveUserStackPointer {
            direction,
            register,
        } => {
            let base = match direction {
                UserStackPointerDirection::AddressRegisterToUserStackPointer => 0x4e60,
                UserStackPointerDirection::UserStackPointerToAddressRegister => 0x4e68,
            };
            push_word(bytes, base | register.bits());
        }
        Instruction::MoveMultiple {
            size,
            direction,
            registers,
            effective_address,
        } => encode_movem(bytes, *size, *direction, *registers, *effective_address)?,
        Instruction::MovePeripheral {
            size,
            direction,
            data_register,
            address_register,
            displacement,
        } => {
            let direction = match direction {
                MovePeripheralDirection::MemoryToRegister => 0,
                MovePeripheralDirection::RegisterToMemory => 0x80,
            };
            push_word(
                bytes,
                0x0108
                    | (data_register.bits() << 9)
                    | (address_size_bit(*size) << 6)
                    | direction
                    | address_register.bits(),
            );
            push_word(bytes, *displacement as u16);
        }
        Instruction::MoveQuick {
            immediate,
            destination,
        } => push_word(
            bytes,
            0x7000 | (destination.bits() << 9) | u16::from(*immediate as u8),
        ),
        Instruction::Multiply {
            sign,
            source,
            destination,
        } => {
            let base = match sign {
                MultiplicationSign::Unsigned => 0xc0c0,
                MultiplicationSign::Signed => 0xc1c0,
            };
            encode_word_ea(
                bytes,
                base | (destination.bits() << 9),
                match sign {
                    MultiplicationSign::Unsigned => Mnemonic::Mulu,
                    MultiplicationSign::Signed => Mnemonic::Muls,
                },
                *source,
                OperandSize::Word,
                is_data,
            )?;
        }
        Instruction::NegateDecimal { destination } => encode_word_ea(
            bytes,
            0x4800,
            Mnemonic::Nbcd,
            *destination,
            OperandSize::Byte,
            is_data_alterable,
        )?,
        Instruction::Negate { size, destination } => encode_unary(
            bytes,
            0x4400,
            Mnemonic::Neg,
            *size,
            *destination,
            is_data_alterable,
        )?,
        Instruction::NegateExtend { size, destination } => encode_unary(
            bytes,
            0x4000,
            Mnemonic::Negx,
            *size,
            *destination,
            is_data_alterable,
        )?,
        Instruction::Nop => push_word(bytes, 0x4e71),
        Instruction::Not { size, destination } => encode_unary(
            bytes,
            0x4600,
            Mnemonic::Not,
            *size,
            *destination,
            is_data_alterable,
        )?,
        Instruction::PushEffectiveAddress { source } => encode_word_ea(
            bytes,
            0x4840,
            Mnemonic::Pea,
            *source,
            OperandSize::Long,
            is_control,
        )?,
        Instruction::Reset => push_word(bytes, 0x4e70),
        Instruction::ReturnFromException => push_word(bytes, 0x4e73),
        Instruction::ReturnAndRestore => push_word(bytes, 0x4e77),
        Instruction::ReturnFromSubroutine => push_word(bytes, 0x4e75),
        Instruction::SetCondition {
            condition,
            destination,
        } => encode_word_ea(
            bytes,
            0x50c0 | (condition.bits() << 8),
            Mnemonic::Scc,
            *destination,
            OperandSize::Byte,
            is_data_alterable,
        )?,
        Instruction::Stop { status } => {
            push_word(bytes, 0x4e72);
            push_word(bytes, *status);
        }
        Instruction::Swap { register } => push_word(bytes, 0x4840 | register.bits()),
        Instruction::TestAndSet { destination } => encode_word_ea(
            bytes,
            0x4ac0,
            Mnemonic::Tas,
            *destination,
            OperandSize::Byte,
            is_data_alterable,
        )?,
        Instruction::Trap { vector } => {
            if *vector > 15 {
                return Err(EncodeError::InvalidTrapVector { vector: *vector });
            }
            push_word(bytes, 0x4e40 | u16::from(*vector));
        }
        Instruction::TrapOnOverflow => push_word(bytes, 0x4e76),
        Instruction::Test { size, source } => encode_unary(
            bytes,
            0x4a00,
            Mnemonic::Tst,
            *size,
            *source,
            is_data_alterable,
        )?,
        Instruction::Unlink { register } => push_word(bytes, 0x4e58 | register.bits()),
    }
    Ok(())
}

fn push_word(bytes: &mut Vec<u8>, word: u16) {
    bytes.extend_from_slice(&word.to_be_bytes());
}

fn push_long(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn size_bits(size: OperandSize) -> u16 {
    match size {
        OperandSize::Byte => 0,
        OperandSize::Word => 1,
        OperandSize::Long => 2,
    }
}

fn address_size_bit(size: AddressSize) -> u16 {
    match size {
        AddressSize::Word => 0,
        AddressSize::Long => 1,
    }
}

fn direction_bit(direction: ShiftDirection) -> u16 {
    match direction {
        ShiftDirection::Right => 0,
        ShiftDirection::Left => 1,
    }
}

fn shift_operation_bits(operation: ShiftOperation) -> u16 {
    match operation {
        ShiftOperation::Arithmetic => 0,
        ShiftOperation::Logical => 1,
        ShiftOperation::RotateWithExtend => 2,
        ShiftOperation::Rotate => 3,
    }
}

fn bit_operation_bits(operation: BitOperation) -> u16 {
    match operation {
        BitOperation::Test => 0,
        BitOperation::Change => 1,
        BitOperation::Clear => 2,
        BitOperation::Set => 3,
    }
}

fn shift_mnemonic(operation: ShiftOperation, direction: ShiftDirection) -> Mnemonic {
    match (operation, direction) {
        (ShiftOperation::Arithmetic, ShiftDirection::Left) => Mnemonic::Asl,
        (ShiftOperation::Arithmetic, ShiftDirection::Right) => Mnemonic::Asr,
        (ShiftOperation::Logical, ShiftDirection::Left) => Mnemonic::Lsl,
        (ShiftOperation::Logical, ShiftDirection::Right) => Mnemonic::Lsr,
        (ShiftOperation::RotateWithExtend, ShiftDirection::Left) => Mnemonic::Roxl,
        (ShiftOperation::RotateWithExtend, ShiftDirection::Right) => Mnemonic::Roxr,
        (ShiftOperation::Rotate, ShiftDirection::Left) => Mnemonic::Rol,
        (ShiftOperation::Rotate, ShiftDirection::Right) => Mnemonic::Ror,
    }
}

fn bit_mnemonic(operation: BitOperation) -> Mnemonic {
    match operation {
        BitOperation::Test => Mnemonic::Btst,
        BitOperation::Change => Mnemonic::Bchg,
        BitOperation::Clear => Mnemonic::Bclr,
        BitOperation::Set => Mnemonic::Bset,
    }
}

fn bit_size(destination: EffectiveAddress) -> OperandSize {
    if matches!(destination, EffectiveAddress::DataRegister(_)) {
        OperandSize::Long
    } else {
        OperandSize::Byte
    }
}

fn encode_quick_value(mnemonic: Mnemonic, value: u8) -> Result<u16, EncodeError> {
    if !(1..=8).contains(&value) {
        return Err(EncodeError::InvalidQuickImmediate { mnemonic, value });
    }
    Ok(if value == 8 { 0 } else { u16::from(value) })
}

fn encode_pair(pair: RegisterPair) -> u16 {
    match pair {
        RegisterPair::Data {
            source,
            destination,
        } => (destination.bits() << 9) | source.bits(),
        RegisterPair::AddressPredecrement {
            source,
            destination,
        } => (destination.bits() << 9) | 8 | source.bits(),
    }
}

fn require_ea(
    mnemonic: Mnemonic,
    ea: EffectiveAddress,
    predicate: fn(EffectiveAddress) -> bool,
) -> Result<(), EncodeError> {
    if predicate(ea) {
        Ok(())
    } else {
        Err(EncodeError::InvalidEffectiveAddress {
            mnemonic,
            mode: ea.mode(),
        })
    }
}

fn is_data(ea: EffectiveAddress) -> bool {
    !matches!(ea, EffectiveAddress::AddressRegister(_))
}

fn is_all_source_for_size(ea: EffectiveAddress, size: OperandSize) -> bool {
    size != OperandSize::Byte || !matches!(ea, EffectiveAddress::AddressRegister(_))
}

fn is_data_alterable(ea: EffectiveAddress) -> bool {
    matches!(ea, EffectiveAddress::DataRegister(_)) || is_memory_alterable(ea)
}

fn is_memory_alterable(ea: EffectiveAddress) -> bool {
    matches!(
        ea,
        EffectiveAddress::AddressIndirect(_)
            | EffectiveAddress::AddressPostincrement(_)
            | EffectiveAddress::AddressPredecrement(_)
            | EffectiveAddress::AddressDisplacement { .. }
            | EffectiveAddress::AddressIndex { .. }
            | EffectiveAddress::AbsoluteWord(_)
            | EffectiveAddress::AbsoluteLong(_)
    )
}

fn is_alterable(ea: EffectiveAddress) -> bool {
    matches!(ea, EffectiveAddress::AddressRegister(_)) || is_data_alterable(ea)
}

fn is_control(ea: EffectiveAddress) -> bool {
    matches!(
        ea,
        EffectiveAddress::AddressIndirect(_)
            | EffectiveAddress::AddressDisplacement { .. }
            | EffectiveAddress::AddressIndex { .. }
            | EffectiveAddress::AbsoluteWord(_)
            | EffectiveAddress::AbsoluteLong(_)
            | EffectiveAddress::ProgramCounterDisplacement(_)
            | EffectiveAddress::ProgramCounterIndex { .. }
    )
}

fn encode_effective_address(
    ea: EffectiveAddress,
    size: OperandSize,
) -> Result<EncodedEffectiveAddress, EncodeError> {
    let mut extensions = Vec::new();
    let field = match ea {
        EffectiveAddress::DataRegister(register) => register.bits(),
        EffectiveAddress::AddressRegister(register) => 0x08 | register.bits(),
        EffectiveAddress::AddressIndirect(register) => 0x10 | register.bits(),
        EffectiveAddress::AddressPostincrement(register) => 0x18 | register.bits(),
        EffectiveAddress::AddressPredecrement(register) => 0x20 | register.bits(),
        EffectiveAddress::AddressDisplacement {
            register,
            displacement,
        } => {
            push_word(&mut extensions, displacement as u16);
            0x28 | register.bits()
        }
        EffectiveAddress::AddressIndex {
            register,
            index,
            displacement,
        } => {
            push_word(&mut extensions, encode_index(index, displacement));
            0x30 | register.bits()
        }
        EffectiveAddress::AbsoluteWord(address) => {
            push_word(&mut extensions, address);
            0x38
        }
        EffectiveAddress::AbsoluteLong(address) => {
            push_long(&mut extensions, address);
            0x39
        }
        EffectiveAddress::ProgramCounterDisplacement(displacement) => {
            push_word(&mut extensions, displacement as u16);
            0x3a
        }
        EffectiveAddress::ProgramCounterIndex {
            index,
            displacement,
        } => {
            push_word(&mut extensions, encode_index(index, displacement));
            0x3b
        }
        EffectiveAddress::Immediate(immediate) => {
            encode_immediate(&mut extensions, immediate, size)?;
            0x3c
        }
    };
    Ok(EncodedEffectiveAddress { field, extensions })
}

fn encode_index(index: IndexRegister, displacement: i8) -> u16 {
    let (register_class, register) = match index.register {
        Register::Data(register) => (0, register.bits()),
        Register::Address(register) => (0x8000, register.bits()),
    };
    let size = match index.size {
        IndexSize::Word => 0,
        IndexSize::Long => 0x0800,
    };
    register_class | (register << 12) | size | u16::from(displacement as u8)
}

fn encode_immediate(
    bytes: &mut Vec<u8>,
    immediate: Immediate,
    expected: OperandSize,
) -> Result<(), EncodeError> {
    if immediate.size() != expected {
        return Err(EncodeError::ImmediateSizeMismatch {
            expected,
            actual: immediate.size(),
        });
    }
    match immediate {
        Immediate::Byte(value) => push_word(bytes, u16::from(value)),
        Immediate::Word(value) => push_word(bytes, value),
        Immediate::Long(value) => push_long(bytes, value),
    }
    Ok(())
}

fn encode_word_ea(
    bytes: &mut Vec<u8>,
    base: u16,
    mnemonic: Mnemonic,
    ea: EffectiveAddress,
    size: OperandSize,
    predicate: fn(EffectiveAddress) -> bool,
) -> Result<(), EncodeError> {
    require_ea(mnemonic, ea, predicate)?;
    let ea = encode_effective_address(ea, size)?;
    push_word(bytes, base | ea.field);
    bytes.extend_from_slice(&ea.extensions);
    Ok(())
}

fn encode_unary(
    bytes: &mut Vec<u8>,
    base: u16,
    mnemonic: Mnemonic,
    size: OperandSize,
    ea: EffectiveAddress,
    predicate: fn(EffectiveAddress) -> bool,
) -> Result<(), EncodeError> {
    encode_word_ea(
        bytes,
        base | (size_bits(size) << 6),
        mnemonic,
        ea,
        size,
        predicate,
    )
}

fn encode_data_binary(
    bytes: &mut Vec<u8>,
    base: u16,
    mnemonic: Mnemonic,
    size: OperandSize,
    direction: DataDirection,
    data_register: DataRegister,
    ea: EffectiveAddress,
) -> Result<(), EncodeError> {
    let (direction, legal) = match direction {
        DataDirection::ToDataRegister => (
            0,
            if matches!(mnemonic, Mnemonic::Add | Mnemonic::Sub) {
                is_all_source_for_size(ea, size)
            } else {
                is_data(ea)
            },
        ),
        DataDirection::ToEffectiveAddress => (0x0100, is_memory_alterable(ea)),
    };
    if !legal {
        return Err(EncodeError::InvalidEffectiveAddress {
            mnemonic,
            mode: ea.mode(),
        });
    }
    encode_word_ea(
        bytes,
        base | (data_register.bits() << 9) | direction | (size_bits(size) << 6),
        mnemonic,
        ea,
        size,
        |_| true,
    )
}

fn encode_address_binary(
    bytes: &mut Vec<u8>,
    base: u16,
    size: AddressSize,
    source: EffectiveAddress,
    destination: AddressRegister,
) -> Result<(), EncodeError> {
    let size_bit = match size {
        AddressSize::Word => 0,
        AddressSize::Long => 0x0100,
    };
    let ea = encode_effective_address(source, size.into())?;
    push_word(
        bytes,
        base | (destination.bits() << 9) | size_bit | ea.field,
    );
    bytes.extend_from_slice(&ea.extensions);
    // All twelve effective-address field forms are legal sources here.
    Ok(())
}

fn encode_immediate_operation(
    bytes: &mut Vec<u8>,
    base: u16,
    mnemonic: Mnemonic,
    size: OperandSize,
    immediate: Immediate,
    destination: EffectiveAddress,
) -> Result<(), EncodeError> {
    require_ea(mnemonic, destination, is_data_alterable)?;
    let ea = encode_effective_address(destination, size)?;
    push_word(bytes, base | (size_bits(size) << 6) | ea.field);
    encode_immediate(bytes, immediate, size)?;
    bytes.extend_from_slice(&ea.extensions);
    Ok(())
}

fn encode_quick(
    bytes: &mut Vec<u8>,
    base: u16,
    mnemonic: Mnemonic,
    size: OperandSize,
    immediate: u8,
    destination: EffectiveAddress,
) -> Result<(), EncodeError> {
    require_ea(mnemonic, destination, is_alterable)?;
    if matches!(destination, EffectiveAddress::AddressRegister(_)) && size == OperandSize::Byte {
        return Err(EncodeError::InvalidSize { mnemonic, size });
    }
    let ea = encode_effective_address(destination, size)?;
    push_word(
        bytes,
        base | (encode_quick_value(mnemonic, immediate)? << 9) | (size_bits(size) << 6) | ea.field,
    );
    bytes.extend_from_slice(&ea.extensions);
    Ok(())
}

fn encode_move(
    bytes: &mut Vec<u8>,
    size: OperandSize,
    source: EffectiveAddress,
    destination: EffectiveAddress,
) -> Result<(), EncodeError> {
    if size == OperandSize::Byte && matches!(source, EffectiveAddress::AddressRegister(_)) {
        return Err(EncodeError::InvalidEffectiveAddress {
            mnemonic: Mnemonic::Move,
            mode: source.mode(),
        });
    }
    require_ea(Mnemonic::Move, destination, is_data_alterable)?;
    let source = encode_effective_address(source, size)?;
    let destination = encode_effective_address(destination, size)?;
    let base = match size {
        OperandSize::Byte => 0x1000,
        OperandSize::Long => 0x2000,
        OperandSize::Word => 0x3000,
    };
    let destination_mode = destination.field >> 3;
    let destination_register = destination.field & 7;
    push_word(
        bytes,
        base | (destination_register << 9) | (destination_mode << 6) | source.field,
    );
    bytes.extend_from_slice(&source.extensions);
    bytes.extend_from_slice(&destination.extensions);
    Ok(())
}

fn encode_move_address(
    bytes: &mut Vec<u8>,
    size: AddressSize,
    source: EffectiveAddress,
    destination: AddressRegister,
) -> Result<(), EncodeError> {
    let operand_size: OperandSize = size.into();
    let source = encode_effective_address(source, operand_size)?;
    let base = match size {
        AddressSize::Word => 0x3040,
        AddressSize::Long => 0x2040,
    };
    push_word(bytes, base | (destination.bits() << 9) | source.field);
    bytes.extend_from_slice(&source.extensions);
    Ok(())
}

fn encode_movem(
    bytes: &mut Vec<u8>,
    size: AddressSize,
    direction: MoveMultipleDirection,
    registers: RegisterMask,
    effective_address: EffectiveAddress,
) -> Result<(), EncodeError> {
    let legal = match direction {
        MoveMultipleDirection::RegistersToMemory => matches!(
            effective_address,
            EffectiveAddress::AddressIndirect(_)
                | EffectiveAddress::AddressPredecrement(_)
                | EffectiveAddress::AddressDisplacement { .. }
                | EffectiveAddress::AddressIndex { .. }
                | EffectiveAddress::AbsoluteWord(_)
                | EffectiveAddress::AbsoluteLong(_)
        ),
        MoveMultipleDirection::MemoryToRegisters => matches!(
            effective_address,
            EffectiveAddress::AddressIndirect(_)
                | EffectiveAddress::AddressPostincrement(_)
                | EffectiveAddress::AddressDisplacement { .. }
                | EffectiveAddress::AddressIndex { .. }
                | EffectiveAddress::AbsoluteWord(_)
                | EffectiveAddress::AbsoluteLong(_)
                | EffectiveAddress::ProgramCounterDisplacement(_)
                | EffectiveAddress::ProgramCounterIndex { .. }
        ),
    };
    if !legal {
        return Err(EncodeError::InvalidEffectiveAddress {
            mnemonic: Mnemonic::Movem,
            mode: effective_address.mode(),
        });
    }
    let ea = encode_effective_address(effective_address, size.into())?;
    let base = match direction {
        MoveMultipleDirection::RegistersToMemory => 0x4880,
        MoveMultipleDirection::MemoryToRegisters => 0x4c80,
    } | (address_size_bit(size) << 6);
    push_word(bytes, base | ea.field);
    let mut mask = registers.bits();
    if direction == MoveMultipleDirection::RegistersToMemory
        && matches!(effective_address, EffectiveAddress::AddressPredecrement(_))
    {
        mask = mask.reverse_bits();
    }
    push_word(bytes, mask);
    bytes.extend_from_slice(&ea.extensions);
    Ok(())
}
