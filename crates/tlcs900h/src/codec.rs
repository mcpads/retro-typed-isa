use thiserror::Error;
use typed_isa_core::{
    DecodedInstruction as ProtocolDecodedInstruction, InstructionBytes, TypedIsa,
};

use crate::{
    ArithmeticOperation, BitOperation, Condition, CurrentRegister, DestinationMemoryInstruction,
    DestinationMemoryOperation, GeneralRegister, Instruction, MainInstruction, MemoryAddress,
    OperandSize, RegisterEncoding, RegisterInstruction, RegisterOperation, ShiftOperation,
    SourceMemoryInstruction, SourceMemoryOperation, Step, Tlcs900H,
};

/// One decoded variable-length instruction and its consumed byte count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodedInstruction {
    instruction: Instruction,
    byte_len: usize,
}

impl DecodedInstruction {
    /// Borrows the semantic instruction.
    pub const fn instruction(&self) -> &Instruction {
        &self.instruction
    }

    /// Consumes the wrapper and returns the semantic instruction.
    pub const fn into_instruction(self) -> Instruction {
        self.instruction
    }

    /// Returns the number of source bytes consumed.
    pub const fn byte_len(&self) -> usize {
        self.byte_len
    }
}

/// Failure to decode one TLCS-900/H instruction.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DecodeError {
    /// The byte stream ended before the selected form was complete.
    #[error("truncated TLCS-900/H instruction: needed {needed} bytes, got {got}")]
    UnexpectedEnd {
        /// Required total byte count.
        needed: usize,
        /// Supplied byte count.
        got: usize,
    },
    /// The primary opcode is undefined or a prohibited dummy instruction.
    #[error("TLCS-900/H primary opcode {opcode:#04x} is undefined or a prohibited dummy")]
    ReservedPrimaryOpcode {
        /// Rejected primary opcode.
        opcode: u8,
    },
    /// A family prefix is followed by an unassigned secondary opcode.
    #[error(
        "TLCS-900/H secondary opcode {secondary:#04x} is undefined after primary {primary:#04x}"
    )]
    ReservedSecondaryOpcode {
        /// Family-selecting primary opcode.
        primary: u8,
        /// Rejected secondary opcode.
        secondary: u8,
    },
    /// An address-prefix extension selector is reserved.
    #[error("TLCS-900/H addressing extension {extension:#04x} is reserved")]
    ReservedAddressingExtension {
        /// Rejected extension byte.
        extension: u8,
    },
    /// A general-register code is unmapped or misaligned for the operand size.
    #[error("TLCS-900/H register code {code:#04x} is invalid for {size:?} access")]
    InvalidRegisterCode {
        /// Rejected register code.
        code: u8,
        /// Requested operand size.
        size: OperandSize,
    },
    /// A control-register selector is not available for the operand size.
    #[error("TLCS-900/H control-register code {code:#04x} is invalid for {size:?} access")]
    InvalidControlRegisterCode {
        /// Rejected control-register code.
        code: u8,
        /// Requested operand size.
        size: OperandSize,
    },
    /// A recognized form contains a reserved field value.
    #[error("TLCS-900/H field {field} has reserved value {value:#x}")]
    ReservedField {
        /// Stable field name.
        field: &'static str,
        /// Rejected field value.
        value: u32,
    },
}

/// Failure to encode a typed TLCS-900/H instruction.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EncodeError {
    /// A numeric operand does not fit its encoded bit width.
    #[error("TLCS-900/H {field} value {value:#x} does not fit {bits} bits")]
    ValueOutOfRange {
        /// Stable field name.
        field: &'static str,
        /// Rejected value.
        value: u32,
        /// Encoded field width.
        bits: u8,
    },
    /// A general-register code is unmapped or misaligned for the operand size.
    #[error("TLCS-900/H register code {code:#04x} is invalid for {size:?} access")]
    InvalidRegisterCode {
        /// Rejected register code.
        code: u8,
        /// Requested operand size.
        size: OperandSize,
    },
    /// A control-register selector is not available for the operand size.
    #[error("TLCS-900/H control-register code {code:#04x} is invalid for {size:?} access")]
    InvalidControlRegisterCode {
        /// Rejected control-register code.
        code: u8,
        /// Requested operand size.
        size: OperandSize,
    },
    /// The selected secondary form is not defined for the prefix operand size.
    #[error("TLCS-900/H {form} is not defined for {size:?} operands")]
    InvalidSize {
        /// Stable form name.
        form: &'static str,
        /// Rejected operand size.
        size: OperandSize,
    },
    /// A semantic field violates a form-specific range or selector rule.
    #[error("TLCS-900/H {field} value {value} is outside {range}")]
    InvalidField {
        /// Stable field name.
        field: &'static str,
        /// Rejected value.
        value: u32,
        /// Human-readable accepted range.
        range: &'static str,
    },
    /// The addressing mode is unavailable in the selected opcode family.
    #[error("TLCS-900/H addressing mode is not available in this opcode family")]
    InvalidAddressingMode,
}

/// Decodes the first variable-length TLCS-900/H instruction.
pub fn decode_bytes(bytes: &[u8]) -> Result<DecodedInstruction, DecodeError> {
    let mut cursor = Cursor::new(bytes);
    let primary = cursor.read_u8()?;
    let instruction = decode_primary(primary, &mut cursor)?;
    Ok(DecodedInstruction {
        instruction,
        byte_len: cursor.position,
    })
}

fn decode_primary(primary: u8, cursor: &mut Cursor<'_>) -> Result<Instruction, DecodeError> {
    let main = match primary {
        0x00 => Some(MainInstruction::Nop),
        0x02 => Some(MainInstruction::PushStatus),
        0x03 => Some(MainInstruction::PopStatus),
        0x05 => Some(MainInstruction::Halt),
        0x06 => {
            let level = cursor.read_u8()?;
            require_at_most("interrupt level", level, 7)?;
            Some(if level == 7 {
                MainInstruction::DisableInterrupt
            } else {
                MainInstruction::EnableInterrupt(level)
            })
        }
        0x07 => Some(MainInstruction::ReturnFromInterrupt),
        0x08 => Some(MainInstruction::LoadDirect8Byte {
            address: cursor.read_u8()?,
            value: cursor.read_u8()?,
        }),
        0x09 => Some(MainInstruction::PushImmediateByte(cursor.read_u8()?)),
        0x0a => Some(MainInstruction::LoadDirect8Word {
            address: cursor.read_u8()?,
            value: cursor.read_u16()?,
        }),
        0x0b => Some(MainInstruction::PushImmediateWord(cursor.read_u16()?)),
        0x0c => Some(MainInstruction::IncrementRegisterFilePointer),
        0x0d => Some(MainInstruction::DecrementRegisterFilePointer),
        0x0e => Some(MainInstruction::Return),
        0x0f => Some(MainInstruction::ReturnAndDeallocate(cursor.read_u16()?)),
        0x10 => Some(MainInstruction::ResetCarry),
        0x11 => Some(MainInstruction::SetCarry),
        0x12 => Some(MainInstruction::ComplementCarry),
        0x13 => Some(MainInstruction::ZeroToCarry),
        0x14 => Some(MainInstruction::PushAccumulator),
        0x15 => Some(MainInstruction::PopAccumulator),
        0x16 => Some(MainInstruction::ExchangeFlags),
        0x17 => {
            let bank = cursor.read_u8()?;
            require_at_most("register-file pointer", bank, 7)?;
            Some(MainInstruction::LoadRegisterFilePointer(bank))
        }
        0x18 => Some(MainInstruction::PushFlags),
        0x19 => Some(MainInstruction::PopFlags),
        0x1a => Some(MainInstruction::JumpDirect16(cursor.read_u16()?)),
        0x1b => Some(MainInstruction::JumpDirect24(cursor.read_u24()?)),
        0x1c => Some(MainInstruction::CallDirect16(cursor.read_u16()?)),
        0x1d => Some(MainInstruction::CallDirect24(cursor.read_u24()?)),
        0x1e => Some(MainInstruction::CallRelative(cursor.read_i16()?)),
        0x20..=0x27 => Some(MainInstruction::LoadCurrentImmediate {
            destination: current(primary),
            size: OperandSize::Byte,
            value: u32::from(cursor.read_u8()?),
        }),
        0x28..=0x2f => Some(MainInstruction::PushCurrent {
            source: current(primary),
            size: OperandSize::Word,
        }),
        0x30..=0x37 => Some(MainInstruction::LoadCurrentImmediate {
            destination: current(primary),
            size: OperandSize::Word,
            value: u32::from(cursor.read_u16()?),
        }),
        0x38..=0x3f => Some(MainInstruction::PushCurrent {
            source: current(primary),
            size: OperandSize::Long,
        }),
        0x40..=0x47 => Some(MainInstruction::LoadCurrentImmediate {
            destination: current(primary),
            size: OperandSize::Long,
            value: cursor.read_u32()?,
        }),
        0x48..=0x4f => Some(MainInstruction::PopCurrent {
            destination: current(primary),
            size: OperandSize::Word,
        }),
        0x58..=0x5f => Some(MainInstruction::PopCurrent {
            destination: current(primary),
            size: OperandSize::Long,
        }),
        0x60..=0x6f => Some(MainInstruction::JumpRelative {
            condition: Condition::from_code(primary & 0x0f),
            displacement: cursor.read_i8()?,
        }),
        0x70..=0x7f => Some(MainInstruction::JumpRelativeLong {
            condition: Condition::from_code(primary & 0x0f),
            displacement: cursor.read_i16()?,
        }),
        0xf7 => Some(MainInstruction::LoadExtract {
            filler: [cursor.read_u8()?, 0, 0],
            address: cursor.read_u8()?,
            value: 0,
        }),
        0xf8..=0xff => Some(MainInstruction::SoftwareInterrupt(primary & 7)),
        _ => None,
    };

    if let Some(mut main) = main {
        if let MainInstruction::LoadExtract {
            address,
            value,
            filler,
        } = &mut main
        {
            filler[1] = cursor.read_u8()?;
            *value = cursor.read_u8()?;
            filler[2] = cursor.read_u8()?;
            let _ = address;
        }
        return Ok(Instruction::Main(main));
    }

    match primary {
        0x80..=0xaf | 0xc0..=0xc5 | 0xd0..=0xd5 | 0xe0..=0xe5 => {
            decode_source_memory(primary, cursor)
        }
        0xb0..=0xbf | 0xf0..=0xf5 => decode_destination_memory(primary, cursor),
        0xc7..=0xcf | 0xd7..=0xdf | 0xe7..=0xef => decode_register(primary, cursor),
        _ => Err(DecodeError::ReservedPrimaryOpcode { opcode: primary }),
    }
}

fn decode_register(primary: u8, cursor: &mut Cursor<'_>) -> Result<Instruction, DecodeError> {
    let size = match primary >> 4 {
        0xc => OperandSize::Byte,
        0xd => OperandSize::Word,
        0xe => OperandSize::Long,
        _ => unreachable!(),
    };
    let register = if primary & 0x0f == 7 {
        let code = cursor.read_u8()?;
        let register = GeneralRegister::new(code)
            .map_err(|_| DecodeError::InvalidRegisterCode { code, size })?;
        RegisterEncoding::Extended(register)
    } else {
        RegisterEncoding::Current(current(primary))
    };
    let secondary = cursor.read_u8()?;
    let instruction = decode_register_secondary(primary, secondary, size, cursor)?;
    validate_register_operation(size, register, instruction).map_err(decode_from_encode)?;
    Ok(Instruction::Register(RegisterOperation {
        size,
        register,
        instruction,
    }))
}

fn decode_register_secondary(
    primary: u8,
    secondary: u8,
    size: OperandSize,
    cursor: &mut Cursor<'_>,
) -> Result<RegisterInstruction, DecodeError> {
    let instruction = match secondary {
        0x03 => RegisterInstruction::LoadImmediate(cursor.read_immediate(size)?),
        0x04 => RegisterInstruction::Push,
        0x05 => RegisterInstruction::Pop,
        0x06 => RegisterInstruction::Complement,
        0x07 => RegisterInstruction::Negate,
        0x08 | 0x09 => RegisterInstruction::MultiplyImmediate {
            signed: secondary == 0x09,
            value: cursor.read_immediate(size)?,
        },
        0x0a | 0x0b => RegisterInstruction::DivideImmediate {
            signed: secondary == 0x0b,
            value: cursor.read_immediate(size)?,
        },
        0x0c => RegisterInstruction::Link(cursor.read_i16()?),
        0x0d => RegisterInstruction::Unlink,
        0x0e => RegisterInstruction::BitSearchForward,
        0x0f => RegisterInstruction::BitSearchBackward,
        0x10 => RegisterInstruction::DecimalAdjust,
        0x12 => RegisterInstruction::ExtendZero,
        0x13 => RegisterInstruction::ExtendSign,
        0x14 => RegisterInstruction::PointerAdjust,
        0x16 => RegisterInstruction::Mirror,
        0x19 => RegisterInstruction::MultiplyAccumulate,
        0x1c => RegisterInstruction::DecrementJump(cursor.read_i8()?),
        0x20..=0x24 => RegisterInstruction::CarryImmediate {
            operation: carry_operation(secondary - 0x20),
            bit: checked_nibble(cursor.read_u8()?)?,
        },
        0x28..=0x2c => RegisterInstruction::CarryAccumulator(carry_operation(secondary - 0x28)),
        0x2e => RegisterInstruction::LoadControl {
            control_code: cursor.read_u8()?,
        },
        0x2f => RegisterInstruction::StoreControl {
            control_code: cursor.read_u8()?,
        },
        0x30..=0x34 => RegisterInstruction::BitImmediate {
            operation: bit_operation(secondary - 0x30),
            bit: checked_nibble(cursor.read_u8()?)?,
        },
        0x38..=0x3a => {
            let step = Step::from_code(secondary - 0x38).expect("three modulo increments");
            let encoded = cursor.read_u16()?;
            RegisterInstruction::ModuloIncrement {
                step,
                modulus: encoded.wrapping_add(u16::from(step.bytes())),
            }
        }
        0x3c..=0x3e => {
            let step = Step::from_code(secondary - 0x3c).expect("three modulo decrements");
            let encoded = cursor.read_u16()?;
            RegisterInstruction::ModuloDecrement {
                step,
                modulus: encoded.wrapping_add(u16::from(step.bytes())),
            }
        }
        0x40..=0x4f => RegisterInstruction::MultiplyCurrent {
            signed: secondary >= 0x48,
            source: current(secondary),
        },
        0x50..=0x5f => RegisterInstruction::DivideCurrent {
            signed: secondary >= 0x58,
            source: current(secondary),
        },
        0x60..=0x67 => RegisterInstruction::Increment(decode_one_to_eight(secondary & 7)),
        0x68..=0x6f => RegisterInstruction::Decrement(decode_one_to_eight(secondary & 7)),
        0x70..=0x7f => RegisterInstruction::SetCondition(Condition::from_code(secondary & 0x0f)),
        0x80..=0x87 => RegisterInstruction::ArithmeticCurrent {
            operation: ArithmeticOperation::Add,
            other: current(secondary),
        },
        0x88..=0x8f => RegisterInstruction::LoadToCurrent(current(secondary)),
        0x90..=0x97 => RegisterInstruction::ArithmeticCurrent {
            operation: ArithmeticOperation::AddCarry,
            other: current(secondary),
        },
        0x98..=0x9f => RegisterInstruction::LoadFromCurrent(current(secondary)),
        0xa0..=0xa7 => RegisterInstruction::ArithmeticCurrent {
            operation: ArithmeticOperation::Subtract,
            other: current(secondary),
        },
        0xa8..=0xaf => RegisterInstruction::LoadSmallImmediate(secondary & 7),
        0xb0..=0xb7 => RegisterInstruction::ArithmeticCurrent {
            operation: ArithmeticOperation::SubtractCarry,
            other: current(secondary),
        },
        0xb8..=0xbf => RegisterInstruction::ExchangeCurrent(current(secondary)),
        0xc0..=0xc7 => RegisterInstruction::ArithmeticCurrent {
            operation: ArithmeticOperation::And,
            other: current(secondary),
        },
        0xc8..=0xcf => RegisterInstruction::ArithmeticImmediate {
            operation: arithmetic_operation(secondary & 7),
            value: cursor.read_immediate(size)?,
        },
        0xd0..=0xd7 => RegisterInstruction::ArithmeticCurrent {
            operation: ArithmeticOperation::Xor,
            other: current(secondary),
        },
        0xd8..=0xdf => RegisterInstruction::CompareSmallImmediate(secondary & 7),
        0xe0..=0xe7 => RegisterInstruction::ArithmeticCurrent {
            operation: ArithmeticOperation::Or,
            other: current(secondary),
        },
        0xe8..=0xef => RegisterInstruction::ShiftImmediate {
            operation: shift_operation(secondary & 7),
            count: decode_one_to_sixteen(checked_nibble(cursor.read_u8()?)?),
        },
        0xf0..=0xf7 => RegisterInstruction::ArithmeticCurrent {
            operation: ArithmeticOperation::Compare,
            other: current(secondary),
        },
        0xf8..=0xff => RegisterInstruction::ShiftAccumulator(shift_operation(secondary & 7)),
        _ => {
            return Err(DecodeError::ReservedSecondaryOpcode { primary, secondary });
        }
    };
    Ok(instruction)
}

fn decode_source_memory(primary: u8, cursor: &mut Cursor<'_>) -> Result<Instruction, DecodeError> {
    let (size, source) = decode_source_address(primary, cursor)?;
    let secondary = cursor.read_u8()?;
    let instruction = match secondary {
        0x04 => SourceMemoryInstruction::Push,
        0x06 => SourceMemoryInstruction::RotateLeftDigit,
        0x07 => SourceMemoryInstruction::RotateRightDigit,
        0x10 => SourceMemoryInstruction::LoadIncrement,
        0x11 => SourceMemoryInstruction::LoadIncrementRepeat,
        0x12 => SourceMemoryInstruction::LoadDecrement,
        0x13 => SourceMemoryInstruction::LoadDecrementRepeat,
        0x14 => SourceMemoryInstruction::CompareIncrement,
        0x15 => SourceMemoryInstruction::CompareIncrementRepeat,
        0x16 => SourceMemoryInstruction::CompareDecrement,
        0x17 => SourceMemoryInstruction::CompareDecrementRepeat,
        0x19 => SourceMemoryInstruction::LoadToDirect16(cursor.read_u16()?),
        0x20..=0x27 => SourceMemoryInstruction::LoadToCurrent(current(secondary)),
        0x30..=0x37 => SourceMemoryInstruction::ExchangeCurrent(current(secondary)),
        0x38..=0x3f => SourceMemoryInstruction::ArithmeticImmediate {
            operation: arithmetic_operation(secondary & 7),
            value: cursor.read_immediate(size)?,
        },
        0x40..=0x4f => SourceMemoryInstruction::MultiplyCurrent {
            signed: secondary >= 0x48,
            destination: current(secondary),
        },
        0x50..=0x5f => SourceMemoryInstruction::DivideCurrent {
            signed: secondary >= 0x58,
            destination: current(secondary),
        },
        0x60..=0x67 => SourceMemoryInstruction::Increment(decode_one_to_eight(secondary & 7)),
        0x68..=0x6f => SourceMemoryInstruction::Decrement(decode_one_to_eight(secondary & 7)),
        0x78..=0x7f => SourceMemoryInstruction::Shift(shift_operation(secondary & 7)),
        0x80..=0xff => {
            let group = (secondary >> 4) & 7;
            SourceMemoryInstruction::ArithmeticCurrent {
                operation: arithmetic_operation(group),
                current: current(secondary),
                destination_is_memory: secondary & 8 != 0,
            }
        }
        _ => {
            return Err(DecodeError::ReservedSecondaryOpcode { primary, secondary });
        }
    };
    validate_source_memory(size, source, instruction).map_err(decode_from_encode)?;
    Ok(Instruction::SourceMemory(SourceMemoryOperation {
        size,
        source,
        instruction,
    }))
}

fn decode_destination_memory(
    primary: u8,
    cursor: &mut Cursor<'_>,
) -> Result<Instruction, DecodeError> {
    let destination = decode_destination_address(primary, cursor)?;
    let secondary = cursor.read_u8()?;
    let instruction = match secondary {
        0x00 => DestinationMemoryInstruction::LoadImmediateByte(cursor.read_u8()?),
        0x02 => DestinationMemoryInstruction::LoadImmediateWord(cursor.read_u16()?),
        0x04 => DestinationMemoryInstruction::PopByte,
        0x06 => DestinationMemoryInstruction::PopWord,
        0x14 => DestinationMemoryInstruction::LoadFromDirect16Byte(cursor.read_u16()?),
        0x16 => DestinationMemoryInstruction::LoadFromDirect16Word(cursor.read_u16()?),
        0x20..=0x27 => DestinationMemoryInstruction::LoadAddressWord(current(secondary)),
        0x28..=0x2c => {
            DestinationMemoryInstruction::CarryAccumulator(carry_operation(secondary - 0x28))
        }
        0x30..=0x37 => DestinationMemoryInstruction::LoadAddressLong(current(secondary)),
        0x40..=0x47 => DestinationMemoryInstruction::LoadFromCurrent {
            size: OperandSize::Byte,
            source: current(secondary),
        },
        0x50..=0x57 => DestinationMemoryInstruction::LoadFromCurrent {
            size: OperandSize::Word,
            source: current(secondary),
        },
        0x60..=0x67 => DestinationMemoryInstruction::LoadFromCurrent {
            size: OperandSize::Long,
            source: current(secondary),
        },
        0x80..=0xcf => DestinationMemoryInstruction::BitImmediate {
            operation: destination_bit_operation(secondary >> 3)
                .ok_or(DecodeError::ReservedSecondaryOpcode { primary, secondary })?,
            bit: secondary & 7,
        },
        0xd0..=0xdf => DestinationMemoryInstruction::Jump(Condition::from_code(secondary & 0x0f)),
        0xe0..=0xef => DestinationMemoryInstruction::Call(Condition::from_code(secondary & 0x0f)),
        0xf0..=0xff if primary == 0xb0 => {
            DestinationMemoryInstruction::Return(Condition::from_code(secondary & 0x0f))
        }
        _ => {
            return Err(DecodeError::ReservedSecondaryOpcode { primary, secondary });
        }
    };
    Ok(Instruction::DestinationMemory(DestinationMemoryOperation {
        destination,
        instruction,
    }))
}

fn decode_source_address(
    primary: u8,
    cursor: &mut Cursor<'_>,
) -> Result<(OperandSize, MemoryAddress), DecodeError> {
    let size = match primary {
        0x80..=0x8f | 0xc0..=0xc5 => OperandSize::Byte,
        0x90..=0x9f | 0xd0..=0xd5 => OperandSize::Word,
        0xa0..=0xaf | 0xe0..=0xe5 => OperandSize::Long,
        _ => unreachable!(),
    };
    let address = match primary {
        0x80..=0x87 | 0x90..=0x97 | 0xa0..=0xa7 => MemoryAddress::CurrentIndirect(current(primary)),
        0x88..=0x8f | 0x98..=0x9f | 0xa8..=0xaf => MemoryAddress::CurrentDisplacement {
            base: current(primary),
            displacement: cursor.read_i8()?,
        },
        _ => decode_generic_address(primary & 0x0f, cursor)?,
    };
    Ok((size, address))
}

fn decode_destination_address(
    primary: u8,
    cursor: &mut Cursor<'_>,
) -> Result<MemoryAddress, DecodeError> {
    Ok(match primary {
        0xb0..=0xb7 => MemoryAddress::CurrentIndirect(current(primary)),
        0xb8..=0xbf => MemoryAddress::CurrentDisplacement {
            base: current(primary),
            displacement: cursor.read_i8()?,
        },
        0xf0..=0xf5 => decode_generic_address(primary & 0x0f, cursor)?,
        _ => unreachable!(),
    })
}

fn decode_generic_address(mode: u8, cursor: &mut Cursor<'_>) -> Result<MemoryAddress, DecodeError> {
    Ok(match mode {
        0 => MemoryAddress::Direct8(cursor.read_u8()?),
        1 => MemoryAddress::Direct16(cursor.read_u16()?),
        2 => MemoryAddress::Direct24(cursor.read_u24()?),
        3 => {
            let extension = cursor.read_u8()?;
            match extension & 3 {
                0 => {
                    MemoryAddress::Register(decode_general_register(extension, OperandSize::Long)?)
                }
                1 => MemoryAddress::RegisterDisplacement {
                    base: decode_general_register(extension & !3, OperandSize::Long)?,
                    displacement: cursor.read_i16()?,
                },
                2 => return Err(DecodeError::ReservedAddressingExtension { extension }),
                3 => match extension {
                    0x03 => MemoryAddress::IndexedByte {
                        base: decode_general_register(cursor.read_u8()?, OperandSize::Long)?,
                        index: decode_general_register(cursor.read_u8()?, OperandSize::Byte)?,
                    },
                    0x07 => MemoryAddress::IndexedWord {
                        base: decode_general_register(cursor.read_u8()?, OperandSize::Long)?,
                        index: decode_general_register(cursor.read_u8()?, OperandSize::Word)?,
                    },
                    0x13 => MemoryAddress::PcRelative(cursor.read_i16()?),
                    _ => return Err(DecodeError::ReservedAddressingExtension { extension }),
                },
                _ => unreachable!(),
            }
        }
        4 | 5 => {
            let extension = cursor.read_u8()?;
            let step = Step::from_code(extension & 3)
                .ok_or(DecodeError::ReservedAddressingExtension { extension })?;
            let register = decode_general_register(extension & !3, OperandSize::Long)?;
            if mode == 4 {
                MemoryAddress::PreDecrement { register, step }
            } else {
                MemoryAddress::PostIncrement { register, step }
            }
        }
        _ => unreachable!(),
    })
}

fn decode_general_register(code: u8, size: OperandSize) -> Result<GeneralRegister, DecodeError> {
    let register =
        GeneralRegister::new(code).map_err(|_| DecodeError::InvalidRegisterCode { code, size })?;
    if !register.supports(size) {
        return Err(DecodeError::InvalidRegisterCode { code, size });
    }
    Ok(register)
}

fn validate_register_operation(
    size: OperandSize,
    register: RegisterEncoding,
    instruction: RegisterInstruction,
) -> Result<(), EncodeError> {
    use RegisterInstruction as R;

    let allowed = match instruction {
        R::LoadImmediate(_)
        | R::Increment(_)
        | R::Decrement(_)
        | R::ArithmeticCurrent { .. }
        | R::LoadFromCurrent(_)
        | R::LoadToCurrent(_)
        | R::LoadSmallImmediate(_)
        | R::ArithmeticImmediate { .. }
        | R::ShiftImmediate { .. }
        | R::ShiftAccumulator(_) => true,
        R::Push | R::Pop => true,
        R::Complement | R::Negate | R::ExchangeCurrent(_) | R::CompareSmallImmediate(_) => {
            size != OperandSize::Long
        }
        R::MultiplyImmediate { .. } | R::DivideImmediate { .. } => size != OperandSize::Long,
        R::Link(_) | R::Unlink => size == OperandSize::Long,
        R::BitSearchForward | R::BitSearchBackward | R::Mirror | R::MultiplyAccumulate => {
            size == OperandSize::Word
        }
        R::DecimalAdjust => size == OperandSize::Byte,
        R::ExtendZero | R::ExtendSign | R::PointerAdjust => size != OperandSize::Byte,
        R::DecrementJump(_) | R::CarryImmediate { .. } | R::CarryAccumulator(_) => {
            size != OperandSize::Long
        }
        R::LoadControl { control_code } | R::StoreControl { control_code } => {
            if !control_code_valid(control_code, size) {
                return Err(EncodeError::InvalidControlRegisterCode {
                    code: control_code,
                    size,
                });
            }
            true
        }
        R::BitImmediate { .. } | R::ModuloIncrement { .. } | R::ModuloDecrement { .. } => {
            size == OperandSize::Word
                || matches!(instruction, R::BitImmediate { .. }) && size == OperandSize::Byte
        }
        R::MultiplyCurrent { source, .. } | R::DivideCurrent { source, .. } => {
            size != OperandSize::Long && (size != OperandSize::Byte || source.index() & 1 == 1)
        }
        R::SetCondition(_) => size != OperandSize::Long,
    };
    if !allowed {
        return Err(EncodeError::InvalidSize {
            form: register_form_name(instruction),
            size,
        });
    }

    let register_size = match instruction {
        R::MultiplyImmediate { .. } | R::DivideImmediate { .. } | R::MultiplyAccumulate => {
            match size {
                OperandSize::Byte => OperandSize::Word,
                OperandSize::Word => OperandSize::Long,
                OperandSize::Long => OperandSize::Long,
            }
        }
        _ => size,
    };
    match register {
        RegisterEncoding::Current(current)
            if size == OperandSize::Byte
                && matches!(
                    instruction,
                    R::MultiplyImmediate { .. } | R::DivideImmediate { .. }
                )
                && current.index() & 1 == 0 =>
        {
            return Err(EncodeError::InvalidField {
                field: "paired current register selector",
                value: u32::from(current.index()),
                range: "1, 3, 5, or 7",
            });
        }
        RegisterEncoding::Extended(register) if !register.supports(register_size) => {
            return Err(EncodeError::InvalidRegisterCode {
                code: register.code(),
                size: register_size,
            });
        }
        _ => {}
    }

    validate_register_fields(instruction)
}

fn validate_register_fields(instruction: RegisterInstruction) -> Result<(), EncodeError> {
    use RegisterInstruction as R;
    match instruction {
        R::Increment(value) | R::Decrement(value) => require_range("increment", value, 1, 8),
        R::LoadSmallImmediate(value) | R::CompareSmallImmediate(value) => {
            require_range("small immediate", value, 0, 7)
        }
        R::CarryImmediate { operation, bit } => {
            require_range("bit number", bit, 0, 15)?;
            require_carry_operation(operation)
        }
        R::BitImmediate { operation, bit } => {
            require_range("bit number", bit, 0, 15)?;
            require_register_bit_operation(operation)
        }
        R::ShiftImmediate { count, .. } => require_range("shift count", count, 1, 16),
        R::ModuloIncrement { step, modulus } | R::ModuloDecrement { step, modulus } => {
            validate_modulus(step, modulus)
        }
        R::CarryAccumulator(operation) => require_carry_operation(operation),
        _ => Ok(()),
    }
}

fn validate_source_memory(
    size: OperandSize,
    source: MemoryAddress,
    instruction: SourceMemoryInstruction,
) -> Result<(), EncodeError> {
    use SourceMemoryInstruction as S;
    let allowed = match instruction {
        S::Push
        | S::LoadToDirect16(_)
        | S::ExchangeCurrent(_)
        | S::ArithmeticImmediate { .. }
        | S::MultiplyCurrent { .. }
        | S::DivideCurrent { .. }
        | S::Increment(_)
        | S::Decrement(_)
        | S::Shift(_) => size != OperandSize::Long,
        S::RotateLeftDigit | S::RotateRightDigit => size == OperandSize::Byte,
        S::LoadIncrement | S::LoadIncrementRepeat | S::LoadDecrement | S::LoadDecrementRepeat => {
            size != OperandSize::Long
                && matches!(source, MemoryAddress::CurrentIndirect(base) if matches!(base.index(), 3 | 5))
        }
        S::CompareIncrement
        | S::CompareIncrementRepeat
        | S::CompareDecrement
        | S::CompareDecrementRepeat => {
            size != OperandSize::Long && matches!(source, MemoryAddress::CurrentIndirect(_))
        }
        S::LoadToCurrent(_) | S::ArithmeticCurrent { .. } => true,
    };
    if !allowed {
        return Err(EncodeError::InvalidSize {
            form: source_form_name(instruction),
            size,
        });
    }
    match instruction {
        S::Increment(value) | S::Decrement(value) => require_range("increment", value, 1, 8),
        S::MultiplyCurrent { destination, .. } | S::DivideCurrent { destination, .. }
            if size == OperandSize::Byte && destination.index() & 1 == 0 =>
        {
            Err(EncodeError::InvalidField {
                field: "paired current register selector",
                value: u32::from(destination.index()),
                range: "1, 3, 5, or 7",
            })
        }
        _ => Ok(()),
    }
}

/// Encodes one typed instruction into its variable-length byte sequence.
pub fn encode_bytes(instruction: &Instruction) -> Result<Vec<u8>, EncodeError> {
    let mut bytes = Vec::new();
    match *instruction {
        Instruction::Main(main) => encode_main(main, &mut bytes)?,
        Instruction::Register(operation) => encode_register(operation, &mut bytes)?,
        Instruction::SourceMemory(operation) => encode_source_memory(operation, &mut bytes)?,
        Instruction::DestinationMemory(operation) => {
            encode_destination_memory(operation, &mut bytes)?
        }
    }
    Ok(bytes)
}

fn encode_main(instruction: MainInstruction, bytes: &mut Vec<u8>) -> Result<(), EncodeError> {
    use MainInstruction as M;
    match instruction {
        M::Nop => bytes.push(0x00),
        M::PushStatus => bytes.push(0x02),
        M::PopStatus => bytes.push(0x03),
        M::Halt => bytes.push(0x05),
        M::EnableInterrupt(level) => {
            require_range("interrupt level", level, 0, 6)?;
            bytes.extend([0x06, level]);
        }
        M::DisableInterrupt => bytes.extend([0x06, 0x07]),
        M::ReturnFromInterrupt => bytes.push(0x07),
        M::LoadDirect8Byte { address, value } => bytes.extend([0x08, address, value]),
        M::PushImmediateByte(value) => bytes.extend([0x09, value]),
        M::LoadDirect8Word { address, value } => {
            bytes.extend([0x0a, address]);
            push_u16(bytes, value);
        }
        M::PushImmediateWord(value) => {
            bytes.push(0x0b);
            push_u16(bytes, value);
        }
        M::IncrementRegisterFilePointer => bytes.push(0x0c),
        M::DecrementRegisterFilePointer => bytes.push(0x0d),
        M::Return => bytes.push(0x0e),
        M::ReturnAndDeallocate(value) => {
            bytes.push(0x0f);
            push_u16(bytes, value);
        }
        M::ResetCarry => bytes.push(0x10),
        M::SetCarry => bytes.push(0x11),
        M::ComplementCarry => bytes.push(0x12),
        M::ZeroToCarry => bytes.push(0x13),
        M::PushAccumulator => bytes.push(0x14),
        M::PopAccumulator => bytes.push(0x15),
        M::ExchangeFlags => bytes.push(0x16),
        M::LoadRegisterFilePointer(bank) => {
            require_range("register-file pointer", bank, 0, 7)?;
            bytes.extend([0x17, bank]);
        }
        M::PushFlags => bytes.push(0x18),
        M::PopFlags => bytes.push(0x19),
        M::JumpDirect16(target) => {
            bytes.push(0x1a);
            push_u16(bytes, target);
        }
        M::JumpDirect24(target) => {
            require_u24("jump target", target)?;
            bytes.push(0x1b);
            push_u24(bytes, target);
        }
        M::CallDirect16(target) => {
            bytes.push(0x1c);
            push_u16(bytes, target);
        }
        M::CallDirect24(target) => {
            require_u24("call target", target)?;
            bytes.push(0x1d);
            push_u24(bytes, target);
        }
        M::CallRelative(displacement) => {
            bytes.push(0x1e);
            push_i16(bytes, displacement);
        }
        M::LoadCurrentImmediate {
            destination,
            size,
            value,
        } => {
            bytes.push(
                match size {
                    OperandSize::Byte => 0x20,
                    OperandSize::Word => 0x30,
                    OperandSize::Long => 0x40,
                } + destination.index(),
            );
            push_immediate(bytes, size, value)?;
        }
        M::PushCurrent { source, size } => {
            let base = match size {
                OperandSize::Word => 0x28,
                OperandSize::Long => 0x38,
                OperandSize::Byte => {
                    return Err(EncodeError::InvalidSize {
                        form: "compact PUSH",
                        size,
                    });
                }
            };
            bytes.push(base + source.index());
        }
        M::PopCurrent { destination, size } => {
            let base = match size {
                OperandSize::Word => 0x48,
                OperandSize::Long => 0x58,
                OperandSize::Byte => {
                    return Err(EncodeError::InvalidSize {
                        form: "compact POP",
                        size,
                    });
                }
            };
            bytes.push(base + destination.index());
        }
        M::JumpRelative {
            condition,
            displacement,
        } => bytes.extend([0x60 + condition as u8, displacement as u8]),
        M::JumpRelativeLong {
            condition,
            displacement,
        } => {
            bytes.push(0x70 + condition as u8);
            push_i16(bytes, displacement);
        }
        M::LoadExtract {
            address,
            value,
            filler,
        } => bytes.extend([0xf7, filler[0], address, filler[1], value, filler[2]]),
        M::SoftwareInterrupt(vector) => {
            require_range("software interrupt vector", vector, 0, 7)?;
            bytes.push(0xf8 + vector);
        }
    }
    Ok(())
}

fn encode_register(operation: RegisterOperation, bytes: &mut Vec<u8>) -> Result<(), EncodeError> {
    validate_register_operation(operation.size, operation.register, operation.instruction)?;
    encode_register_prefix(operation.size, operation.register, bytes);
    encode_register_secondary(operation.size, operation.instruction, bytes)
}

fn encode_register_prefix(size: OperandSize, register: RegisterEncoding, bytes: &mut Vec<u8>) {
    let (extension, compact) = match size {
        OperandSize::Byte => (0xc7, 0xc8),
        OperandSize::Word => (0xd7, 0xd8),
        OperandSize::Long => (0xe7, 0xe8),
    };
    match register {
        RegisterEncoding::Current(register) => bytes.push(compact + register.index()),
        RegisterEncoding::Extended(register) => bytes.extend([extension, register.code()]),
    }
}

fn encode_register_secondary(
    size: OperandSize,
    instruction: RegisterInstruction,
    bytes: &mut Vec<u8>,
) -> Result<(), EncodeError> {
    use RegisterInstruction as R;
    match instruction {
        R::LoadImmediate(value) => {
            bytes.push(0x03);
            push_immediate(bytes, size, value)?;
        }
        R::Push => bytes.push(0x04),
        R::Pop => bytes.push(0x05),
        R::Complement => bytes.push(0x06),
        R::Negate => bytes.push(0x07),
        R::MultiplyImmediate { signed, value } => {
            bytes.push(if signed { 0x09 } else { 0x08 });
            push_immediate(bytes, size, value)?;
        }
        R::DivideImmediate { signed, value } => {
            bytes.push(if signed { 0x0b } else { 0x0a });
            push_immediate(bytes, size, value)?;
        }
        R::Link(displacement) => {
            bytes.push(0x0c);
            push_i16(bytes, displacement);
        }
        R::Unlink => bytes.push(0x0d),
        R::BitSearchForward => bytes.push(0x0e),
        R::BitSearchBackward => bytes.push(0x0f),
        R::DecimalAdjust => bytes.push(0x10),
        R::ExtendZero => bytes.push(0x12),
        R::ExtendSign => bytes.push(0x13),
        R::PointerAdjust => bytes.push(0x14),
        R::Mirror => bytes.push(0x16),
        R::MultiplyAccumulate => bytes.push(0x19),
        R::DecrementJump(displacement) => bytes.extend([0x1c, displacement as u8]),
        R::CarryImmediate { operation, bit } => {
            bytes.extend([0x20 + carry_operation_code(operation)?, bit]);
        }
        R::CarryAccumulator(operation) => bytes.push(0x28 + carry_operation_code(operation)?),
        R::LoadControl { control_code } => bytes.extend([0x2e, control_code]),
        R::StoreControl { control_code } => bytes.extend([0x2f, control_code]),
        R::BitImmediate { operation, bit } => {
            bytes.extend([0x30 + register_bit_operation_code(operation)?, bit]);
        }
        R::ModuloIncrement { step, modulus } => {
            bytes.push(0x38 + step as u8);
            push_u16(bytes, modulus - u16::from(step.bytes()));
        }
        R::ModuloDecrement { step, modulus } => {
            bytes.push(0x3c + step as u8);
            push_u16(bytes, modulus - u16::from(step.bytes()));
        }
        R::MultiplyCurrent { signed, source } => {
            bytes.push((if signed { 0x48 } else { 0x40 }) + source.index());
        }
        R::DivideCurrent { signed, source } => {
            bytes.push((if signed { 0x58 } else { 0x50 }) + source.index());
        }
        R::Increment(value) => bytes.push(0x60 + encode_one_to_eight(value)),
        R::Decrement(value) => bytes.push(0x68 + encode_one_to_eight(value)),
        R::SetCondition(condition) => bytes.push(0x70 + condition as u8),
        R::ArithmeticCurrent { operation, other } => {
            bytes.push(arithmetic_register_base(operation) + other.index());
        }
        R::LoadToCurrent(other) => bytes.push(0x88 + other.index()),
        R::LoadFromCurrent(other) => bytes.push(0x98 + other.index()),
        R::LoadSmallImmediate(value) => bytes.push(0xa8 + value),
        R::ExchangeCurrent(other) => bytes.push(0xb8 + other.index()),
        R::ArithmeticImmediate { operation, value } => {
            bytes.push(0xc8 + arithmetic_operation_code(operation));
            push_immediate(bytes, size, value)?;
        }
        R::CompareSmallImmediate(value) => bytes.push(0xd8 + value),
        R::ShiftImmediate { operation, count } => {
            bytes.extend([
                0xe8 + shift_operation_code(operation),
                encode_one_to_sixteen(count),
            ]);
        }
        R::ShiftAccumulator(operation) => bytes.push(0xf8 + shift_operation_code(operation)),
    }
    Ok(())
}

fn encode_source_memory(
    operation: SourceMemoryOperation,
    bytes: &mut Vec<u8>,
) -> Result<(), EncodeError> {
    validate_source_memory(operation.size, operation.source, operation.instruction)?;
    encode_source_address(operation.size, operation.source, bytes)?;
    let size = operation.size;
    use SourceMemoryInstruction as S;
    match operation.instruction {
        S::Push => bytes.push(0x04),
        S::RotateLeftDigit => bytes.push(0x06),
        S::RotateRightDigit => bytes.push(0x07),
        S::LoadIncrement => bytes.push(0x10),
        S::LoadIncrementRepeat => bytes.push(0x11),
        S::LoadDecrement => bytes.push(0x12),
        S::LoadDecrementRepeat => bytes.push(0x13),
        S::CompareIncrement => bytes.push(0x14),
        S::CompareIncrementRepeat => bytes.push(0x15),
        S::CompareDecrement => bytes.push(0x16),
        S::CompareDecrementRepeat => bytes.push(0x17),
        S::LoadToDirect16(address) => {
            bytes.push(0x19);
            push_u16(bytes, address);
        }
        S::LoadToCurrent(destination) => bytes.push(0x20 + destination.index()),
        S::ExchangeCurrent(other) => bytes.push(0x30 + other.index()),
        S::ArithmeticImmediate { operation, value } => {
            bytes.push(0x38 + arithmetic_operation_code(operation));
            push_immediate(bytes, size, value)?;
        }
        S::MultiplyCurrent {
            signed,
            destination,
        } => bytes.push((if signed { 0x48 } else { 0x40 }) + destination.index()),
        S::DivideCurrent {
            signed,
            destination,
        } => bytes.push((if signed { 0x58 } else { 0x50 }) + destination.index()),
        S::Increment(value) => bytes.push(0x60 + encode_one_to_eight(value)),
        S::Decrement(value) => bytes.push(0x68 + encode_one_to_eight(value)),
        S::Shift(operation) => bytes.push(0x78 + shift_operation_code(operation)),
        S::ArithmeticCurrent {
            operation,
            current,
            destination_is_memory,
        } => {
            bytes.push(
                0x80 + arithmetic_operation_code(operation) * 0x10
                    + if destination_is_memory { 8 } else { 0 }
                    + current.index(),
            );
        }
    }
    Ok(())
}

fn encode_destination_memory(
    operation: DestinationMemoryOperation,
    bytes: &mut Vec<u8>,
) -> Result<(), EncodeError> {
    if matches!(
        operation.instruction,
        DestinationMemoryInstruction::Return(_)
    ) && operation.destination != MemoryAddress::CurrentIndirect(CurrentRegister::from_index(0))
    {
        return Err(EncodeError::InvalidAddressingMode);
    }
    encode_destination_address(operation.destination, bytes)?;
    use DestinationMemoryInstruction as D;
    match operation.instruction {
        D::LoadImmediateByte(value) => bytes.extend([0x00, value]),
        D::LoadImmediateWord(value) => {
            bytes.push(0x02);
            push_u16(bytes, value);
        }
        D::PopByte => bytes.push(0x04),
        D::PopWord => bytes.push(0x06),
        D::LoadFromDirect16Byte(address) => {
            bytes.push(0x14);
            push_u16(bytes, address);
        }
        D::LoadFromDirect16Word(address) => {
            bytes.push(0x16);
            push_u16(bytes, address);
        }
        D::LoadAddressWord(destination) => bytes.push(0x20 + destination.index()),
        D::LoadAddressLong(destination) => bytes.push(0x30 + destination.index()),
        D::CarryAccumulator(operation) => bytes.push(0x28 + carry_operation_code(operation)?),
        D::LoadFromCurrent { size, source } => {
            let base = match size {
                OperandSize::Byte => 0x40,
                OperandSize::Word => 0x50,
                OperandSize::Long => 0x60,
            };
            bytes.push(base + source.index());
        }
        D::BitImmediate { operation, bit } => {
            require_range("bit number", bit, 0, 7)?;
            bytes.push(destination_bit_operation_base(operation)? + bit);
        }
        D::Jump(condition) => bytes.push(0xd0 + condition as u8),
        D::Call(condition) => bytes.push(0xe0 + condition as u8),
        D::Return(condition) => bytes.push(0xf0 + condition as u8),
    }
    Ok(())
}

fn encode_source_address(
    size: OperandSize,
    address: MemoryAddress,
    bytes: &mut Vec<u8>,
) -> Result<(), EncodeError> {
    let (compact, displaced, generic) = match size {
        OperandSize::Byte => (0x80, 0x88, 0xc0),
        OperandSize::Word => (0x90, 0x98, 0xd0),
        OperandSize::Long => (0xa0, 0xa8, 0xe0),
    };
    match address {
        MemoryAddress::CurrentIndirect(register) => bytes.push(compact + register.index()),
        MemoryAddress::CurrentDisplacement { base, displacement } => {
            bytes.extend([displaced + base.index(), displacement as u8]);
        }
        _ => encode_generic_address(generic, address, bytes)?,
    }
    Ok(())
}

fn encode_destination_address(
    address: MemoryAddress,
    bytes: &mut Vec<u8>,
) -> Result<(), EncodeError> {
    match address {
        MemoryAddress::CurrentIndirect(register) => bytes.push(0xb0 + register.index()),
        MemoryAddress::CurrentDisplacement { base, displacement } => {
            bytes.extend([0xb8 + base.index(), displacement as u8]);
        }
        _ => encode_generic_address(0xf0, address, bytes)?,
    }
    Ok(())
}

fn encode_generic_address(
    base: u8,
    address: MemoryAddress,
    bytes: &mut Vec<u8>,
) -> Result<(), EncodeError> {
    match address {
        MemoryAddress::Direct8(address) => bytes.extend([base, address]),
        MemoryAddress::Direct16(address) => {
            bytes.push(base + 1);
            push_u16(bytes, address);
        }
        MemoryAddress::Direct24(address) => {
            require_u24("direct address", address)?;
            bytes.push(base + 2);
            push_u24(bytes, address);
        }
        MemoryAddress::Register(register) => {
            require_register_size(register, OperandSize::Long)?;
            bytes.extend([base + 3, register.code()]);
        }
        MemoryAddress::RegisterDisplacement {
            base: register,
            displacement,
        } => {
            require_register_size(register, OperandSize::Long)?;
            bytes.extend([base + 3, register.code() | 1]);
            push_i16(bytes, displacement);
        }
        MemoryAddress::IndexedByte {
            base: register,
            index,
        } => {
            require_register_size(register, OperandSize::Long)?;
            require_register_size(index, OperandSize::Byte)?;
            bytes.extend([base + 3, 0x03, register.code(), index.code()]);
        }
        MemoryAddress::IndexedWord {
            base: register,
            index,
        } => {
            require_register_size(register, OperandSize::Long)?;
            require_register_size(index, OperandSize::Word)?;
            bytes.extend([base + 3, 0x07, register.code(), index.code()]);
        }
        MemoryAddress::PcRelative(displacement) => {
            bytes.extend([base + 3, 0x13]);
            push_i16(bytes, displacement);
        }
        MemoryAddress::PreDecrement { register, step } => {
            require_register_size(register, OperandSize::Long)?;
            bytes.extend([base + 4, register.code() | step as u8]);
        }
        MemoryAddress::PostIncrement { register, step } => {
            require_register_size(register, OperandSize::Long)?;
            bytes.extend([base + 5, register.code() | step as u8]);
        }
        MemoryAddress::CurrentIndirect(_) | MemoryAddress::CurrentDisplacement { .. } => {
            return Err(EncodeError::InvalidAddressingMode);
        }
    }
    Ok(())
}

fn current(code: u8) -> CurrentRegister {
    CurrentRegister::from_index(code & 7)
}

fn arithmetic_operation(code: u8) -> ArithmeticOperation {
    match code {
        0 => ArithmeticOperation::Add,
        1 => ArithmeticOperation::AddCarry,
        2 => ArithmeticOperation::Subtract,
        3 => ArithmeticOperation::SubtractCarry,
        4 => ArithmeticOperation::And,
        5 => ArithmeticOperation::Xor,
        6 => ArithmeticOperation::Or,
        7 => ArithmeticOperation::Compare,
        _ => unreachable!(),
    }
}

const fn arithmetic_operation_code(operation: ArithmeticOperation) -> u8 {
    match operation {
        ArithmeticOperation::Add => 0,
        ArithmeticOperation::AddCarry => 1,
        ArithmeticOperation::Subtract => 2,
        ArithmeticOperation::SubtractCarry => 3,
        ArithmeticOperation::And => 4,
        ArithmeticOperation::Xor => 5,
        ArithmeticOperation::Or => 6,
        ArithmeticOperation::Compare => 7,
    }
}

const fn arithmetic_register_base(operation: ArithmeticOperation) -> u8 {
    match operation {
        ArithmeticOperation::Add => 0x80,
        ArithmeticOperation::AddCarry => 0x90,
        ArithmeticOperation::Subtract => 0xa0,
        ArithmeticOperation::SubtractCarry => 0xb0,
        ArithmeticOperation::And => 0xc0,
        ArithmeticOperation::Xor => 0xd0,
        ArithmeticOperation::Or => 0xe0,
        ArithmeticOperation::Compare => 0xf0,
    }
}

fn shift_operation(code: u8) -> ShiftOperation {
    match code {
        0 => ShiftOperation::RotateLeftCircular,
        1 => ShiftOperation::RotateRightCircular,
        2 => ShiftOperation::RotateLeftCarry,
        3 => ShiftOperation::RotateRightCarry,
        4 => ShiftOperation::ShiftLeftArithmetic,
        5 => ShiftOperation::ShiftRightArithmetic,
        6 => ShiftOperation::ShiftLeftLogical,
        7 => ShiftOperation::ShiftRightLogical,
        _ => unreachable!(),
    }
}

const fn shift_operation_code(operation: ShiftOperation) -> u8 {
    match operation {
        ShiftOperation::RotateLeftCircular => 0,
        ShiftOperation::RotateRightCircular => 1,
        ShiftOperation::RotateLeftCarry => 2,
        ShiftOperation::RotateRightCarry => 3,
        ShiftOperation::ShiftLeftArithmetic => 4,
        ShiftOperation::ShiftRightArithmetic => 5,
        ShiftOperation::ShiftLeftLogical => 6,
        ShiftOperation::ShiftRightLogical => 7,
    }
}

fn carry_operation(code: u8) -> BitOperation {
    match code {
        0 => BitOperation::AndCarry,
        1 => BitOperation::OrCarry,
        2 => BitOperation::XorCarry,
        3 => BitOperation::LoadCarry,
        4 => BitOperation::StoreCarry,
        _ => unreachable!(),
    }
}

fn carry_operation_code(operation: BitOperation) -> Result<u8, EncodeError> {
    Ok(match operation {
        BitOperation::AndCarry => 0,
        BitOperation::OrCarry => 1,
        BitOperation::XorCarry => 2,
        BitOperation::LoadCarry => 3,
        BitOperation::StoreCarry => 4,
        _ => return Err(invalid_bit_operation(operation)),
    })
}

fn bit_operation(code: u8) -> BitOperation {
    match code {
        0 => BitOperation::Reset,
        1 => BitOperation::Set,
        2 => BitOperation::Change,
        3 => BitOperation::Test,
        4 => BitOperation::TestAndSet,
        _ => unreachable!(),
    }
}

fn register_bit_operation_code(operation: BitOperation) -> Result<u8, EncodeError> {
    Ok(match operation {
        BitOperation::Reset => 0,
        BitOperation::Set => 1,
        BitOperation::Change => 2,
        BitOperation::Test => 3,
        BitOperation::TestAndSet => 4,
        _ => return Err(invalid_bit_operation(operation)),
    })
}

fn destination_bit_operation(code: u8) -> Option<BitOperation> {
    Some(match code {
        0x10 => BitOperation::AndCarry,
        0x11 => BitOperation::OrCarry,
        0x12 => BitOperation::XorCarry,
        0x13 => BitOperation::LoadCarry,
        0x14 => BitOperation::StoreCarry,
        0x15 => BitOperation::TestAndSet,
        0x16 => BitOperation::Reset,
        0x17 => BitOperation::Set,
        0x18 => BitOperation::Change,
        0x19 => BitOperation::Test,
        _ => return None,
    })
}

fn destination_bit_operation_base(operation: BitOperation) -> Result<u8, EncodeError> {
    Ok(match operation {
        BitOperation::AndCarry => 0x80,
        BitOperation::OrCarry => 0x88,
        BitOperation::XorCarry => 0x90,
        BitOperation::LoadCarry => 0x98,
        BitOperation::StoreCarry => 0xa0,
        BitOperation::TestAndSet => 0xa8,
        BitOperation::Reset => 0xb0,
        BitOperation::Set => 0xb8,
        BitOperation::Change => 0xc0,
        BitOperation::Test => 0xc8,
    })
}

fn invalid_bit_operation(operation: BitOperation) -> EncodeError {
    EncodeError::InvalidField {
        field: "bit operation",
        value: destination_bit_operation_base(operation).unwrap_or(0) as u32,
        range: "an operation defined by this instruction form",
    }
}

fn control_code_valid(code: u8, size: OperandSize) -> bool {
    let width = match code {
        0x00..=0x1f => OperandSize::Long,
        0x20..=0x2f | 0x3c..=0x3d => OperandSize::Word,
        _ => return false,
    };
    match size {
        OperandSize::Byte => true,
        OperandSize::Word => code & 1 == 0,
        OperandSize::Long => width == OperandSize::Long && code & 3 == 0,
    }
}

fn validate_modulus(step: Step, modulus: u16) -> Result<(), EncodeError> {
    let minimum = match step {
        Step::One => 2,
        Step::Two => 4,
        Step::Four => 8,
    };
    if modulus < minimum || modulus > 0x8000 || !modulus.is_power_of_two() {
        return Err(EncodeError::InvalidField {
            field: "modulus",
            value: u32::from(modulus),
            range: "a permitted power of two through 32768",
        });
    }
    Ok(())
}

fn require_register_size(register: GeneralRegister, size: OperandSize) -> Result<(), EncodeError> {
    if register.supports(size) {
        Ok(())
    } else {
        Err(EncodeError::InvalidRegisterCode {
            code: register.code(),
            size,
        })
    }
}

fn require_carry_operation(operation: BitOperation) -> Result<(), EncodeError> {
    carry_operation_code(operation).map(|_| ())
}

fn require_register_bit_operation(operation: BitOperation) -> Result<(), EncodeError> {
    register_bit_operation_code(operation).map(|_| ())
}

fn require_range(
    field: &'static str,
    value: u8,
    minimum: u8,
    maximum: u8,
) -> Result<(), EncodeError> {
    if (minimum..=maximum).contains(&value) {
        Ok(())
    } else {
        Err(EncodeError::InvalidField {
            field,
            value: u32::from(value),
            range: match (minimum, maximum) {
                (0, 7) => "0..7",
                (0, 15) => "0..15",
                (1, 8) => "1..8",
                (1, 16) => "1..16",
                _ => "the documented range",
            },
        })
    }
}

fn require_at_most(field: &'static str, value: u8, maximum: u8) -> Result<(), DecodeError> {
    if value <= maximum {
        Ok(())
    } else {
        Err(DecodeError::ReservedField {
            field,
            value: u32::from(value),
        })
    }
}

fn checked_nibble(value: u8) -> Result<u8, DecodeError> {
    require_at_most("4-bit immediate", value, 15)?;
    Ok(value)
}

const fn decode_one_to_eight(encoded: u8) -> u8 {
    if encoded == 0 { 8 } else { encoded }
}

const fn encode_one_to_eight(value: u8) -> u8 {
    value & 7
}

const fn decode_one_to_sixteen(encoded: u8) -> u8 {
    if encoded == 0 { 16 } else { encoded }
}

const fn encode_one_to_sixteen(value: u8) -> u8 {
    value & 15
}

fn push_immediate(bytes: &mut Vec<u8>, size: OperandSize, value: u32) -> Result<(), EncodeError> {
    match size {
        OperandSize::Byte => {
            let value = u8::try_from(value).map_err(|_| EncodeError::ValueOutOfRange {
                field: "immediate",
                value,
                bits: 8,
            })?;
            bytes.push(value);
        }
        OperandSize::Word => {
            let value = u16::try_from(value).map_err(|_| EncodeError::ValueOutOfRange {
                field: "immediate",
                value,
                bits: 16,
            })?;
            push_u16(bytes, value);
        }
        OperandSize::Long => push_u32(bytes, value),
    }
    Ok(())
}

fn require_u24(field: &'static str, value: u32) -> Result<(), EncodeError> {
    if value <= 0x00ff_ffff {
        Ok(())
    } else {
        Err(EncodeError::ValueOutOfRange {
            field,
            value,
            bits: 24,
        })
    }
}

fn push_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend(value.to_le_bytes());
}

fn push_i16(bytes: &mut Vec<u8>, value: i16) {
    bytes.extend(value.to_le_bytes());
}

fn push_u24(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend([value as u8, (value >> 8) as u8, (value >> 16) as u8]);
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend(value.to_le_bytes());
}

fn register_form_name(instruction: RegisterInstruction) -> &'static str {
    match instruction {
        RegisterInstruction::LoadImmediate(_) => "LD r,#",
        RegisterInstruction::Push => "PUSH r",
        RegisterInstruction::Pop => "POP r",
        RegisterInstruction::Complement => "CPL r",
        RegisterInstruction::Negate => "NEG r",
        RegisterInstruction::MultiplyImmediate { .. } => "MUL r,#",
        RegisterInstruction::DivideImmediate { .. } => "DIV r,#",
        RegisterInstruction::Link(_) => "LINK",
        RegisterInstruction::Unlink => "UNLK",
        RegisterInstruction::BitSearchForward | RegisterInstruction::BitSearchBackward => "BS1",
        RegisterInstruction::DecimalAdjust => "DAA",
        RegisterInstruction::ExtendZero | RegisterInstruction::ExtendSign => "EXT",
        RegisterInstruction::PointerAdjust => "PAA",
        RegisterInstruction::Mirror => "MIRR",
        RegisterInstruction::MultiplyAccumulate => "MULA",
        RegisterInstruction::DecrementJump(_) => "DJNZ",
        RegisterInstruction::CarryImmediate { .. } | RegisterInstruction::CarryAccumulator(_) => {
            "carry-bit operation"
        }
        RegisterInstruction::LoadControl { .. } | RegisterInstruction::StoreControl { .. } => "LDC",
        RegisterInstruction::BitImmediate { .. } => "bit operation",
        RegisterInstruction::ModuloIncrement { .. }
        | RegisterInstruction::ModuloDecrement { .. } => "modulo operation",
        RegisterInstruction::MultiplyCurrent { .. } => "MUL R,r",
        RegisterInstruction::DivideCurrent { .. } => "DIV R,r",
        RegisterInstruction::Increment(_) => "INC",
        RegisterInstruction::Decrement(_) => "DEC",
        RegisterInstruction::SetCondition(_) => "SCC",
        RegisterInstruction::ArithmeticCurrent { .. }
        | RegisterInstruction::ArithmeticImmediate { .. } => "arithmetic operation",
        RegisterInstruction::LoadFromCurrent(_)
        | RegisterInstruction::LoadToCurrent(_)
        | RegisterInstruction::LoadSmallImmediate(_) => "LD",
        RegisterInstruction::ExchangeCurrent(_) => "EX",
        RegisterInstruction::CompareSmallImmediate(_) => "CP",
        RegisterInstruction::ShiftImmediate { .. } | RegisterInstruction::ShiftAccumulator(_) => {
            "shift operation"
        }
    }
}

fn source_form_name(instruction: SourceMemoryInstruction) -> &'static str {
    match instruction {
        SourceMemoryInstruction::Push => "PUSH (mem)",
        SourceMemoryInstruction::RotateLeftDigit | SourceMemoryInstruction::RotateRightDigit => {
            "digit rotate"
        }
        SourceMemoryInstruction::LoadIncrement
        | SourceMemoryInstruction::LoadIncrementRepeat
        | SourceMemoryInstruction::LoadDecrement
        | SourceMemoryInstruction::LoadDecrementRepeat => "block load",
        SourceMemoryInstruction::CompareIncrement
        | SourceMemoryInstruction::CompareIncrementRepeat
        | SourceMemoryInstruction::CompareDecrement
        | SourceMemoryInstruction::CompareDecrementRepeat => "block compare",
        SourceMemoryInstruction::LoadToDirect16(_) | SourceMemoryInstruction::LoadToCurrent(_) => {
            "LD"
        }
        SourceMemoryInstruction::ExchangeCurrent(_) => "EX",
        SourceMemoryInstruction::ArithmeticImmediate { .. }
        | SourceMemoryInstruction::ArithmeticCurrent { .. } => "arithmetic operation",
        SourceMemoryInstruction::MultiplyCurrent { .. } => "MUL",
        SourceMemoryInstruction::DivideCurrent { .. } => "DIV",
        SourceMemoryInstruction::Increment(_) => "INC",
        SourceMemoryInstruction::Decrement(_) => "DEC",
        SourceMemoryInstruction::Shift(_) => "shift operation",
    }
}

fn decode_from_encode(error: EncodeError) -> DecodeError {
    match error {
        EncodeError::InvalidRegisterCode { code, size } => {
            DecodeError::InvalidRegisterCode { code, size }
        }
        EncodeError::InvalidControlRegisterCode { code, size } => {
            DecodeError::InvalidControlRegisterCode { code, size }
        }
        EncodeError::InvalidField { field, value, .. }
        | EncodeError::ValueOutOfRange { field, value, .. } => {
            DecodeError::ReservedField { field, value }
        }
        EncodeError::InvalidSize { form, size } => DecodeError::ReservedField {
            field: form,
            value: size as u32,
        },
        EncodeError::InvalidAddressingMode => DecodeError::ReservedField {
            field: "addressing mode",
            value: 0,
        },
    }
}

struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn read_u8(&mut self) -> Result<u8, DecodeError> {
        let value = self
            .bytes
            .get(self.position)
            .copied()
            .ok_or(DecodeError::UnexpectedEnd {
                needed: self.position + 1,
                got: self.bytes.len(),
            })?;
        self.position += 1;
        Ok(value)
    }

    fn read_i8(&mut self) -> Result<i8, DecodeError> {
        Ok(self.read_u8()? as i8)
    }

    fn read_u16(&mut self) -> Result<u16, DecodeError> {
        Ok(u16::from_le_bytes([self.read_u8()?, self.read_u8()?]))
    }

    fn read_i16(&mut self) -> Result<i16, DecodeError> {
        Ok(self.read_u16()? as i16)
    }

    fn read_u24(&mut self) -> Result<u32, DecodeError> {
        Ok(u32::from(self.read_u8()?)
            | (u32::from(self.read_u8()?) << 8)
            | (u32::from(self.read_u8()?) << 16))
    }

    fn read_u32(&mut self) -> Result<u32, DecodeError> {
        Ok(u32::from_le_bytes([
            self.read_u8()?,
            self.read_u8()?,
            self.read_u8()?,
            self.read_u8()?,
        ]))
    }

    fn read_immediate(&mut self, size: OperandSize) -> Result<u32, DecodeError> {
        Ok(match size {
            OperandSize::Byte => u32::from(self.read_u8()?),
            OperandSize::Word => u32::from(self.read_u16()?),
            OperandSize::Long => self.read_u32()?,
        })
    }
}

impl TypedIsa for Tlcs900H {
    const PROFILE_ID: &'static str = "tlcs900h";

    type Instruction = Instruction;
    type Form = crate::InstructionForm;
    type CodeLocation = crate::CodeLocation;
    type DecodeError = DecodeError;
    type EncodeError = EncodeError;

    fn form(instruction: &Self::Instruction) -> Result<Self::Form, Self::EncodeError> {
        Ok(instruction.form())
    }

    fn decode_one(
        source: &[u8],
        _at: &Self::CodeLocation,
    ) -> Result<ProtocolDecodedInstruction<Self::Instruction>, Self::DecodeError> {
        let decoded = decode_bytes(source)?;
        let bytes = InstructionBytes::new(source[..decoded.byte_len()].to_vec())
            .expect("decoded instruction is non-empty");
        Ok(ProtocolDecodedInstruction::new(
            decoded.into_instruction(),
            bytes,
        ))
    }

    fn encode_one(
        instruction: &Self::Instruction,
        _at: &Self::CodeLocation,
    ) -> Result<InstructionBytes, Self::EncodeError> {
        Ok(InstructionBytes::new(encode_bytes(instruction)?)
            .expect("encoded instruction is non-empty"))
    }
}
