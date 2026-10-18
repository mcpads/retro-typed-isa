use thiserror::Error;
use typed_isa_core::{
    DecodedInstruction as ProtocolDecodedInstruction, InstructionBytes, TypedIsa,
};

use crate::{
    BitStringOperation, Condition, FloatingOperation, ImmediateOperation, Instruction,
    LogicalImmediateOperation, MemoryOperation, Register, RegisterOperation, ShiftOperation,
    SystemRegister, V810,
};

/// One typed instruction decoded from one or two leading halfwords.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodedInstruction {
    instruction: Instruction,
    byte_len: usize,
}

impl DecodedInstruction {
    /// Borrow the typed instruction identity.
    pub const fn instruction(&self) -> &Instruction {
        &self.instruction
    }

    /// Consume the result and return its typed instruction identity.
    pub const fn into_instruction(self) -> Instruction {
        self.instruction
    }

    /// Return the exact number of source bytes consumed.
    pub const fn byte_len(&self) -> usize {
        self.byte_len
    }
}

/// A fail-closed reason that source halfwords are not one valid V810 instruction.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DecodeError {
    /// The supplied instruction location is not halfword aligned.
    #[error("V810 code location 0x{location:08X} is not halfword aligned")]
    UnalignedCodeLocation {
        /// Rejected executable location.
        location: crate::CodeLocation,
    },
    /// The source ends before all required little-endian halfwords are available.
    #[error("truncated V810 instruction: needed {needed} bytes, got {got}")]
    UnexpectedEnd {
        /// Minimum byte count required to continue decoding.
        needed: usize,
        /// Byte count available to the decoder.
        got: usize,
    },
    /// The primary opcode selector is reserved by the architecture.
    #[error("reserved V810 primary opcode in first halfword {first:#06x}")]
    ReservedOpcode {
        /// First instruction halfword containing the rejected opcode.
        first: u16,
    },
    /// A field required to be zero contains non-zero bits.
    #[error("reserved V810 field is non-zero in instruction {first:#06x} {second:#06x}")]
    ReservedField {
        /// First instruction halfword.
        first: u16,
        /// Second halfword, or zero for a one-halfword instruction.
        second: u16,
    },
    /// A system-register selector has no architectural assignment.
    #[error("V810 system register ID {id} is reserved")]
    ReservedSystemRegister {
        /// Rejected five-bit system-register identifier.
        id: u8,
    },
    /// `LDSR` attempts to write a read-only system register.
    #[error("V810 system register {register:?} cannot be written by LDSR")]
    WriteDisabledSystemRegister {
        /// Read-only system register selected by the instruction.
        register: SystemRegister,
    },
    /// The bit-string extension selector is reserved.
    #[error("reserved V810 bit-string sub-opcode {subopcode:#04x}")]
    ReservedBitStringSubopcode {
        /// Rejected five-bit extension selector.
        subopcode: u8,
    },
    /// The floating-point extension selector is reserved.
    #[error("reserved V810 floating-point sub-opcode {subopcode:#04x}")]
    ReservedFloatingSubopcode {
        /// Rejected six-bit extension selector.
        subopcode: u8,
    },
}

/// A fail-closed reason that a typed V810 instruction cannot be lowered.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EncodeError {
    /// The supplied instruction location is not halfword aligned.
    #[error("V810 code location 0x{location:08X} is not halfword aligned")]
    UnalignedCodeLocation {
        /// Rejected executable location.
        location: crate::CodeLocation,
    },
    /// A signed five-bit immediate lies outside `-16..=15`.
    #[error("V810 signed 5-bit immediate {value} is outside -16..15")]
    SignedImmediateOutOfRange {
        /// Rejected immediate value.
        value: i8,
    },
    /// An immediate shift count cannot be represented in five bits.
    #[error("V810 shift amount {value} is outside 0..31")]
    ShiftAmountOutOfRange {
        /// Rejected shift count.
        value: u8,
    },
    /// A software-trap vector cannot be represented in five bits.
    #[error("V810 trap vector {value} is outside 0..31")]
    TrapVectorOutOfRange {
        /// Rejected trap vector.
        value: u8,
    },
    /// `LDSR` attempts to write a read-only system register.
    #[error("V810 system register {register:?} cannot be written by LDSR")]
    WriteDisabledSystemRegister {
        /// Read-only system register selected by the instruction.
        register: SystemRegister,
    },
    /// A conditional branch displacement is odd or outside signed nine-bit range.
    #[error("V810 branch displacement {value} is not an even signed 9-bit value")]
    BranchDisplacementOutOfRange {
        /// Rejected byte displacement.
        value: i16,
    },
    /// Condition code 13 cannot produce a branch because its selector is `NOP`.
    #[error("V810 condition code 13 encodes NOP and cannot be used as a branch")]
    NeverConditionIsNop,
    /// A jump displacement is odd or outside signed 26-bit range.
    #[error("V810 jump displacement {value} is not an even signed 26-bit value")]
    JumpDisplacementOutOfRange {
        /// Rejected byte displacement.
        value: i32,
    },
}

/// Decode one instruction from the beginning of a little-endian byte slice.
pub fn decode_bytes(bytes: &[u8]) -> Result<DecodedInstruction, DecodeError> {
    let first = read_halfword(bytes, 0)?;
    let needs_second = matches!(first >> 10, 0x28..=0x3f)
        && !matches!(first >> 10, 0x30..=0x3f if matches!(first >> 10, 0x32 | 0x36));
    let second = if needs_second {
        Some(read_halfword(bytes, 2)?)
    } else {
        None
    };
    decode_halfwords(first, second)
}

fn read_halfword(bytes: &[u8], offset: usize) -> Result<u16, DecodeError> {
    let slice = bytes
        .get(offset..offset + 2)
        .ok_or(DecodeError::UnexpectedEnd {
            needed: offset + 2,
            got: bytes.len(),
        })?;
    Ok(u16::from_le_bytes(
        slice.try_into().expect("two-byte slice"),
    ))
}

/// Decode one instruction from its first and optional second architectural halfword.
pub fn decode_halfwords(
    first: u16,
    second: Option<u16>,
) -> Result<DecodedInstruction, DecodeError> {
    if first >> 13 == 0b100 {
        return decode_branch(first);
    }

    let opcode = (first >> 10) as u8;
    let register1 = Register::from_field(first & 0x1f);
    let register2 = Register::from_field((first >> 5) & 0x1f);
    let instruction = if let Some(operation) = RegisterOperation::from_opcode(opcode) {
        Instruction::Register {
            operation,
            source: register1,
            destination: register2,
        }
    } else if opcode == 0x06 {
        require_zero(first, 0, register2.index() as u16)?;
        Instruction::JumpRegister(register1)
    } else if let Some(operation) = ImmediateOperation::from_opcode(opcode) {
        Instruction::Immediate {
            operation,
            immediate: sign_extend(first & 0x1f, 5) as i8,
            destination: register2,
        }
    } else if let Some(operation) = ShiftOperation::from_immediate_opcode(opcode) {
        Instruction::ShiftImmediate {
            operation,
            amount: (first & 0x1f) as u8,
            destination: register2,
        }
    } else {
        match opcode {
            0x12 => {
                let field = (first & 0x1f) as u8;
                if field & 0x10 != 0 {
                    return Err(DecodeError::ReservedField { first, second: 0 });
                }
                Instruction::SetFlag {
                    condition: Condition::from_field(field),
                    destination: register2,
                }
            }
            0x16 | 0x1b | 0x1e => return Err(DecodeError::ReservedOpcode { first }),
            0x18 => {
                require_zero(first, 0, (first >> 5) & 0x1f)?;
                Instruction::Trap((first & 0x1f) as u8)
            }
            0x19 => {
                require_zero(first, 0, first & 0x03ff)?;
                Instruction::ReturnFromInterrupt
            }
            0x1a => {
                require_zero(first, 0, first & 0x03ff)?;
                Instruction::Halt
            }
            0x1c => {
                let destination = decode_system_register((first & 0x1f) as u8)?;
                if !destination.is_writable() {
                    return Err(DecodeError::WriteDisabledSystemRegister {
                        register: destination,
                    });
                }
                Instruction::LoadSystemRegister {
                    source: register2,
                    destination,
                }
            }
            0x1d => Instruction::StoreSystemRegister {
                source: decode_system_register((first & 0x1f) as u8)?,
                destination: register2,
            },
            0x1f => {
                require_zero(first, 0, (first >> 5) & 0x1f)?;
                let subopcode = (first & 0x1f) as u8;
                Instruction::BitString(
                    BitStringOperation::from_subopcode(subopcode)
                        .ok_or(DecodeError::ReservedBitStringSubopcode { subopcode })?,
                )
            }
            0x28 => Instruction::MoveAddress {
                immediate: second_required(first, second)? as i16,
                source: register1,
                destination: register2,
            },
            0x29 => Instruction::AddImmediate {
                immediate: second_required(first, second)? as i16,
                source: register1,
                destination: register2,
            },
            0x2a => {
                let second = second_required(first, second)?;
                if second & 1 != 0 {
                    return Err(DecodeError::ReservedField { first, second });
                }
                Instruction::JumpRelative(decode_long_displacement(first, second))
            }
            0x2b => {
                let second = second_required(first, second)?;
                if second & 1 != 0 {
                    return Err(DecodeError::ReservedField { first, second });
                }
                Instruction::JumpAndLink(decode_long_displacement(first, second))
            }
            0x2c..=0x2f => Instruction::LogicalImmediate {
                operation: LogicalImmediateOperation::from_opcode(opcode)
                    .expect("logical immediate opcode"),
                immediate: second_required(first, second)?,
                source: register1,
                destination: register2,
            },
            0x30..=0x3d | 0x3f => {
                let operation = MemoryOperation::from_opcode(opcode)
                    .ok_or(DecodeError::ReservedOpcode { first })?;
                Instruction::Memory {
                    operation,
                    displacement: second_required(first, second)? as i16,
                    base: register1,
                    register: register2,
                }
            }
            0x3e => {
                let second = second_required(first, second)?;
                if second & 0x03ff != 0 {
                    return Err(DecodeError::ReservedField { first, second });
                }
                let subopcode = (second >> 10) as u8;
                Instruction::Floating {
                    operation: FloatingOperation::from_subopcode(subopcode)
                        .ok_or(DecodeError::ReservedFloatingSubopcode { subopcode })?,
                    source: register1,
                    destination: register2,
                }
            }
            _ => return Err(DecodeError::ReservedOpcode { first }),
        }
    };

    Ok(DecodedInstruction {
        byte_len: instruction.encoded_len(),
        instruction,
    })
}

fn decode_branch(first: u16) -> Result<DecodedInstruction, DecodeError> {
    if first & 1 != 0 {
        return Err(DecodeError::ReservedField { first, second: 0 });
    }
    let condition_field = ((first >> 9) & 0x0f) as u8;
    let raw_displacement = first & 0x01ff;
    let instruction = if condition_field == 0x0d {
        if raw_displacement != 0 {
            return Err(DecodeError::ReservedField { first, second: 0 });
        }
        Instruction::Nop
    } else {
        Instruction::Branch {
            condition: Condition::from_field(condition_field),
            displacement: sign_extend(raw_displacement, 9) as i16,
        }
    };
    Ok(DecodedInstruction {
        instruction,
        byte_len: 2,
    })
}

fn decode_system_register(id: u8) -> Result<SystemRegister, DecodeError> {
    SystemRegister::from_id(id).ok_or(DecodeError::ReservedSystemRegister { id })
}

fn require_zero(first: u16, second: u16, value: u16) -> Result<(), DecodeError> {
    if value == 0 {
        Ok(())
    } else {
        Err(DecodeError::ReservedField { first, second })
    }
}

fn second_required(_first: u16, second: Option<u16>) -> Result<u16, DecodeError> {
    second.ok_or(DecodeError::UnexpectedEnd { needed: 4, got: 2 })
}

const fn sign_extend(value: u16, width: u32) -> i32 {
    ((value as i32) << (32 - width)) >> (32 - width)
}

const fn decode_long_displacement(first: u16, second: u16) -> i32 {
    let encoded = ((first & 0x03ff) as u32) << 16 | second as u32;
    ((encoded as i32) << 6) >> 6
}

/// Lower one typed instruction to its first and optional second halfword.
pub fn encode_halfwords(instruction: &Instruction) -> Result<(u16, Option<u16>), EncodeError> {
    let encoded = match *instruction {
        Instruction::Register {
            operation,
            source,
            destination,
        } => (reg_word(operation as u8, source, destination), None),
        Instruction::JumpRegister(source) => ((0x06_u16 << 10) | u16::from(source.index()), None),
        Instruction::Immediate {
            operation,
            immediate,
            destination,
        } => {
            if !(-16..=15).contains(&immediate) {
                return Err(EncodeError::SignedImmediateOutOfRange { value: immediate });
            }
            (
                (u16::from(operation as u8) << 10)
                    | (u16::from(destination.index()) << 5)
                    | (u16::from(immediate as u8) & 0x1f),
                None,
            )
        }
        Instruction::ShiftImmediate {
            operation,
            amount,
            destination,
        } => {
            if amount >= 32 {
                return Err(EncodeError::ShiftAmountOutOfRange { value: amount });
            }
            (
                (u16::from(operation.immediate_opcode()) << 10)
                    | (u16::from(destination.index()) << 5)
                    | u16::from(amount),
                None,
            )
        }
        Instruction::SetFlag {
            condition,
            destination,
        } => (
            (0x12_u16 << 10) | (u16::from(destination.index()) << 5) | u16::from(condition as u8),
            None,
        ),
        Instruction::Trap(vector) => {
            if vector >= 32 {
                return Err(EncodeError::TrapVectorOutOfRange { value: vector });
            }
            ((0x18_u16 << 10) | u16::from(vector), None)
        }
        Instruction::ReturnFromInterrupt => (0x19_u16 << 10, None),
        Instruction::Halt => (0x1a_u16 << 10, None),
        Instruction::LoadSystemRegister {
            source,
            destination,
        } => {
            if !destination.is_writable() {
                return Err(EncodeError::WriteDisabledSystemRegister {
                    register: destination,
                });
            }
            (
                (0x1c_u16 << 10) | (u16::from(source.index()) << 5) | u16::from(destination.id()),
                None,
            )
        }
        Instruction::StoreSystemRegister {
            source,
            destination,
        } => (
            (0x1d_u16 << 10) | (u16::from(destination.index()) << 5) | u16::from(source.id()),
            None,
        ),
        Instruction::BitString(operation) => ((0x1f_u16 << 10) | u16::from(operation as u8), None),
        Instruction::Nop => (0x9a00, None),
        Instruction::Branch {
            condition,
            displacement,
        } => {
            if condition == Condition::Never {
                return Err(EncodeError::NeverConditionIsNop);
            }
            if !(-256..=254).contains(&displacement) || displacement & 1 != 0 {
                return Err(EncodeError::BranchDisplacementOutOfRange {
                    value: displacement,
                });
            }
            (
                0x8000 | (u16::from(condition as u8) << 9) | ((displacement as u16) & 0x01ff),
                None,
            )
        }
        Instruction::MoveAddress {
            immediate,
            source,
            destination,
        } => (reg_word(0x28, source, destination), Some(immediate as u16)),
        Instruction::AddImmediate {
            immediate,
            source,
            destination,
        } => (reg_word(0x29, source, destination), Some(immediate as u16)),
        Instruction::LogicalImmediate {
            operation,
            immediate,
            source,
            destination,
        } => (
            reg_word(operation as u8, source, destination),
            Some(immediate),
        ),
        Instruction::JumpRelative(displacement) => encode_long_jump(0x2a, displacement)?,
        Instruction::JumpAndLink(displacement) => encode_long_jump(0x2b, displacement)?,
        Instruction::Memory {
            operation,
            displacement,
            base,
            register,
        } => (
            reg_word(operation as u8, base, register),
            Some(displacement as u16),
        ),
        Instruction::Floating {
            operation,
            source,
            destination,
        } => (
            reg_word(0x3e, source, destination),
            Some(u16::from(operation as u8) << 10),
        ),
    };
    Ok(encoded)
}

const fn reg_word(opcode: u8, register1: Register, register2: Register) -> u16 {
    (opcode as u16) << 10 | (register2.index() as u16) << 5 | register1.index() as u16
}

fn encode_long_jump(opcode: u8, displacement: i32) -> Result<(u16, Option<u16>), EncodeError> {
    if !(-33_554_432..=33_554_430).contains(&displacement) || displacement & 1 != 0 {
        return Err(EncodeError::JumpDisplacementOutOfRange {
            value: displacement,
        });
    }
    let encoded = displacement as u32 & 0x03ff_ffff;
    Ok((
        (u16::from(opcode) << 10) | ((encoded >> 16) as u16 & 0x03ff),
        Some(encoded as u16),
    ))
}

/// Lower one typed instruction to little-endian halfword bytes.
pub fn encode_bytes(instruction: &Instruction) -> Result<Vec<u8>, EncodeError> {
    let (first, second) = encode_halfwords(instruction)?;
    let mut bytes = Vec::with_capacity(instruction.encoded_len());
    bytes.extend_from_slice(&first.to_le_bytes());
    if let Some(second) = second {
        bytes.extend_from_slice(&second.to_le_bytes());
    }
    Ok(bytes)
}

impl TypedIsa for V810 {
    const PROFILE_ID: &'static str = crate::PROFILE_ID;

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
        at: &Self::CodeLocation,
    ) -> Result<ProtocolDecodedInstruction<Self::Instruction>, Self::DecodeError> {
        let location = *at;
        if location & 1 != 0 {
            return Err(DecodeError::UnalignedCodeLocation { location });
        }
        let decoded = decode_bytes(source)?;
        let exact_source = InstructionBytes::new(source[..decoded.byte_len()].to_vec())
            .expect("decoded V810 instructions are non-empty");
        Ok(ProtocolDecodedInstruction::new(
            decoded.into_instruction(),
            exact_source,
        ))
    }

    fn encode_one(
        instruction: &Self::Instruction,
        at: &Self::CodeLocation,
    ) -> Result<InstructionBytes, Self::EncodeError> {
        let location = *at;
        if location & 1 != 0 {
            return Err(EncodeError::UnalignedCodeLocation { location });
        }
        Ok(InstructionBytes::new(encode_bytes(instruction)?)
            .expect("encoded V810 instructions are non-empty"))
    }
}
