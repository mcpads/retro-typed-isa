use thiserror::Error;
use typed_isa_core::{
    DecodedInstruction as ProtocolDecodedInstruction, InstructionBytes, TypedIsa,
};

use crate::instruction::*;
use crate::vfpu::{
    DestinationPrefixLane, DestinationSaturation, Immediate5, Immediate8, Immediate16,
    MatrixRegister, OperandLayout, SourcePrefixConstant, SourcePrefixLane, SourcePrefixValue,
    VFPU_DESCRIPTORS, VectorComponent, VectorRegister, VectorSize, VfpuCompareCondition,
    VfpuConstant, VfpuConstructionError, VfpuControlRegister, VfpuForm, VfpuInstruction,
    VfpuMemoryOffset, VfpuOperands, VfpuOperation,
};

#[derive(Debug, Clone, Copy)]
/// The fixed-width little-endian Allegrex protocol profile.
pub struct Allegrex;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
/// Failure returned while decoding one Allegrex instruction.
pub enum DecodeError {
    #[error("Allegrex code location {location:#x} is not four-byte aligned")]
    /// The unaligned address form.
    UnalignedAddress {
        /// The instruction or target address.
        location: crate::CodeLocation,
    },
    #[error("truncated Allegrex instruction: needed 4 bytes, got {got}")]
    /// The unexpected end form.
    UnexpectedEnd {
        /// The available input length.
        got: usize,
    },
    #[error("reserved or out-of-profile Allegrex instruction word {word:#010x}")]
    /// The reserved word form.
    ReservedWord {
        /// The rejected instruction word.
        word: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
/// Failure returned when a typed Allegrex instruction cannot be placed and encoded.
pub enum EncodeError {
    #[error("Allegrex code location {location:#x} is not four-byte aligned")]
    /// The unaligned address form.
    UnalignedAddress {
        /// The instruction or target address.
        location: crate::CodeLocation,
    },
    #[error("control-flow target {target:#010x} is not four-byte aligned")]
    /// The unaligned target form.
    UnalignedTarget {
        /// The requested control-flow target.
        target: u32,
    },
    #[error("branch at {address:#010x} cannot reach {target:#010x}")]
    /// The branch out of range form.
    BranchOutOfRange {
        /// The instruction or target address.
        address: u32,
        /// The requested control-flow target.
        target: u32,
    },
    #[error("jump at {address:#010x} cannot reach region containing {target:#010x}")]
    /// The jump region mismatch form.
    JumpRegionMismatch {
        /// The instruction or target address.
        address: u32,
        /// The requested control-flow target.
        target: u32,
    },
    #[error("{field} value {value} exceeds {maximum}")]
    /// The field out of range form.
    FieldOutOfRange {
        /// The field operand.
        field: &'static str,
        /// The selected source-prefix value.
        value: u32,
        /// The greatest encodable value.
        maximum: u32,
    },
    #[error("invalid Allegrex instruction form: {reason}")]
    /// The invalid instruction form form.
    InvalidInstructionForm {
        /// A stable explanation of the rejection.
        reason: &'static str,
    },
    #[error("invalid VFPU instruction: {0}")]
    /// The invalid vfpu form.
    InvalidVfpu(#[from] VfpuConstructionError),
}

/// Decodes one host-order instruction word at its code location.
pub fn decode_word(word: u32, location: crate::CodeLocation) -> Result<Instruction, DecodeError> {
    let location = checked_decode_location(location)?;
    if let Some(instruction) = decode_vfpu(word, location)? {
        return Ok(Instruction::Vfpu(instruction));
    }

    match opcode(word) {
        0x00 => decode_special(word),
        0x01 => decode_regimm(word, location),
        0x02 | 0x03 => Ok(Instruction::Jump {
            link: opcode(word) == 0x03,
            target: jump_target(word, location),
        }),
        0x04 | 0x05 | 0x14 | 0x15 => Ok(Instruction::EqualityBranch {
            condition: if opcode(word) & 1 == 0 {
                EqualityCondition::Equal
            } else {
                EqualityCondition::NotEqual
            },
            likely: opcode(word) & 0x10 != 0,
            left: rs(word),
            right: rt(word),
            target: branch_target(word, location),
        }),
        0x06 | 0x07 | 0x16 | 0x17 => {
            require(word, rt_index(word) == 0)?;
            Ok(Instruction::SignBranch {
                condition: if opcode(word) & 1 == 0 {
                    SignCondition::LessThanOrEqualZero
                } else {
                    SignCondition::GreaterThanZero
                },
                likely: opcode(word) & 0x10 != 0,
                link: false,
                source: rs(word),
                target: branch_target(word, location),
            })
        }
        0x08..=0x0b => Ok(Instruction::ImmediateArithmetic {
            operation: match opcode(word) {
                0x08 => ImmediateArithmeticOperation::Add,
                0x09 => ImmediateArithmeticOperation::AddUnsigned,
                0x0a => ImmediateArithmeticOperation::SetLessThan,
                0x0b => ImmediateArithmeticOperation::SetLessThanUnsigned,
                _ => unreachable!(),
            },
            destination: rt(word),
            source: rs(word),
            immediate: word as u16 as i16,
        }),
        0x0c..=0x0e => Ok(Instruction::LogicalImmediate {
            operation: match opcode(word) {
                0x0c => LogicalImmediateOperation::And,
                0x0d => LogicalImmediateOperation::Or,
                0x0e => LogicalImmediateOperation::ExclusiveOr,
                _ => unreachable!(),
            },
            destination: rt(word),
            source: rs(word),
            immediate: word as u16,
        }),
        0x0f => {
            require(word, rs_index(word) == 0)?;
            Ok(Instruction::LoadUpperImmediate {
                destination: rt(word),
                immediate: word as u16,
            })
        }
        0x10 => decode_cp0(word),
        0x11 => decode_fpu(word, location),
        0x1f => decode_special3(word),
        0x20..=0x26 | 0x28..=0x2b | 0x2e | 0x30 | 0x38 => decode_memory(word),
        0x2f => {
            let operation = CacheOperation::from_field(rt_index(word))
                .ok_or(DecodeError::ReservedWord { word })?;
            Ok(Instruction::Cache {
                operation,
                base: rs(word),
                offset: word as u16 as i16,
            })
        }
        0x31 | 0x39 => Ok(Instruction::FloatingMemory {
            store: opcode(word) == 0x39,
            floating: FloatingRegister::from_field(rt_index(word)),
            base: rs(word),
            offset: word as u16 as i16,
        }),
        _ => reserved(word),
    }
}

/// Encodes one typed instruction as a host-order word at its code location.
pub fn encode_word(
    instruction: &Instruction,
    location: crate::CodeLocation,
) -> Result<u32, EncodeError> {
    let address = checked_encode_location(location)?;
    use Instruction::*;

    match instruction {
        Nop => Ok(0),
        ThreeRegister {
            operation,
            destination,
            left,
            right,
        } => Ok(r_type(
            *left,
            *right,
            *destination,
            0,
            match operation {
                ThreeRegisterOperation::Add => 0x20,
                ThreeRegisterOperation::AddUnsigned => 0x21,
                ThreeRegisterOperation::Subtract => 0x22,
                ThreeRegisterOperation::SubtractUnsigned => 0x23,
                ThreeRegisterOperation::And => 0x24,
                ThreeRegisterOperation::Or => 0x25,
                ThreeRegisterOperation::ExclusiveOr => 0x26,
                ThreeRegisterOperation::Nor => 0x27,
                ThreeRegisterOperation::SetLessThan => 0x2a,
                ThreeRegisterOperation::SetLessThanUnsigned => 0x2b,
                ThreeRegisterOperation::Maximum => 0x2c,
                ThreeRegisterOperation::Minimum => 0x2d,
            },
        )),
        ImmediateArithmetic {
            operation,
            destination,
            source,
            immediate,
        } => Ok(i_type(
            match operation {
                ImmediateArithmeticOperation::Add => 0x08,
                ImmediateArithmeticOperation::AddUnsigned => 0x09,
                ImmediateArithmeticOperation::SetLessThan => 0x0a,
                ImmediateArithmeticOperation::SetLessThanUnsigned => 0x0b,
            },
            *source,
            destination.index(),
            *immediate as u16,
        )),
        LogicalImmediate {
            operation,
            destination,
            source,
            immediate,
        } => Ok(i_type(
            match operation {
                LogicalImmediateOperation::And => 0x0c,
                LogicalImmediateOperation::Or => 0x0d,
                LogicalImmediateOperation::ExclusiveOr => 0x0e,
            },
            *source,
            destination.index(),
            *immediate,
        )),
        LoadUpperImmediate {
            destination,
            immediate,
        } => Ok(i_type(
            0x0f,
            GeneralRegister::ZERO,
            destination.index(),
            *immediate,
        )),
        ShiftImmediate {
            operation,
            destination,
            source,
            amount,
        } => {
            check_field("shift amount", u32::from(*amount), 31)?;
            Ok(match operation {
                ShiftOperation::LeftLogical => {
                    r_type(GeneralRegister::ZERO, *source, *destination, *amount, 0x00)
                }
                ShiftOperation::RightLogical => {
                    r_type(GeneralRegister::ZERO, *source, *destination, *amount, 0x02)
                }
                ShiftOperation::RightArithmetic => {
                    r_type(GeneralRegister::ZERO, *source, *destination, *amount, 0x03)
                }
                ShiftOperation::RotateRight => {
                    r_type(GeneralRegister::AT, *source, *destination, *amount, 0x02)
                }
            })
        }
        ShiftVariable {
            operation,
            destination,
            source,
            amount_register,
        } => Ok(match operation {
            ShiftOperation::LeftLogical => r_type(*amount_register, *source, *destination, 0, 0x04),
            ShiftOperation::RightLogical => {
                r_type(*amount_register, *source, *destination, 0, 0x06)
            }
            ShiftOperation::RightArithmetic => {
                r_type(*amount_register, *source, *destination, 0, 0x07)
            }
            ShiftOperation::RotateRight => r_type(*amount_register, *source, *destination, 1, 0x06),
        }),
        MultiplyDivide {
            operation,
            left,
            right,
        } => Ok(r_type(
            *left,
            *right,
            GeneralRegister::ZERO,
            0,
            match operation {
                MultiplyDivideOperation::Multiply => 0x18,
                MultiplyDivideOperation::MultiplyUnsigned => 0x19,
                MultiplyDivideOperation::Divide => 0x1a,
                MultiplyDivideOperation::DivideUnsigned => 0x1b,
            },
        )),
        Accumulate {
            operation,
            left,
            right,
        } => Ok(r_type(
            *left,
            *right,
            GeneralRegister::ZERO,
            0,
            match operation {
                AccumulateOperation::MultiplyAdd => 0x1c,
                AccumulateOperation::MultiplyAddUnsigned => 0x1d,
                AccumulateOperation::MultiplySubtract => 0x2e,
                AccumulateOperation::MultiplySubtractUnsigned => 0x2f,
            },
        )),
        MoveFromSpecial {
            source,
            destination,
        } => Ok(r_type(
            GeneralRegister::ZERO,
            GeneralRegister::ZERO,
            *destination,
            0,
            if *source == SpecialRegister::Hi {
                0x10
            } else {
                0x12
            },
        )),
        MoveToSpecial {
            destination,
            source,
        } => Ok(r_type(
            *source,
            GeneralRegister::ZERO,
            GeneralRegister::ZERO,
            0,
            if *destination == SpecialRegister::Hi {
                0x11
            } else {
                0x13
            },
        )),
        ConditionalMove {
            condition,
            destination,
            source,
            condition_register,
        } => Ok(r_type(
            *source,
            *condition_register,
            *destination,
            0,
            if *condition == ConditionalMoveCondition::Zero {
                0x0a
            } else {
                0x0b
            },
        )),
        BitCount {
            operation,
            destination,
            source,
        } => Ok(r_type(
            *source,
            GeneralRegister::ZERO,
            *destination,
            0,
            if *operation == BitCountOperation::LeadingZeros {
                0x16
            } else {
                0x17
            },
        )),
        BitField {
            operation,
            destination,
            source,
            position,
            size,
        } => encode_bit_field(*operation, *destination, *source, *position, *size),
        ByteTransform {
            operation,
            destination,
            source,
        } => Ok(special3_transform(*operation, *destination, *source)),
        JumpRegister { source, link } => Ok(r_type(
            *source,
            GeneralRegister::ZERO,
            link.unwrap_or(GeneralRegister::ZERO),
            0,
            if link.is_some() { 0x09 } else { 0x08 },
        )),
        Jump { link, target } => jump_word(if *link { 0x03 } else { 0x02 }, address, *target),
        EqualityBranch {
            condition,
            likely,
            left,
            right,
            target,
        } => branch_word(
            match (*condition, *likely) {
                (EqualityCondition::Equal, false) => 0x04,
                (EqualityCondition::NotEqual, false) => 0x05,
                (EqualityCondition::Equal, true) => 0x14,
                (EqualityCondition::NotEqual, true) => 0x15,
            },
            *left,
            right.index(),
            address,
            *target,
        ),
        SignBranch {
            condition,
            likely,
            link,
            source,
            target,
        } => encode_sign_branch(*condition, *likely, *link, *source, address, *target),
        Memory {
            operation,
            register,
            base,
            offset,
        } => Ok(i_type(
            memory_opcode(*operation),
            *base,
            register.index(),
            *offset as u16,
        )),
        Cache {
            operation,
            base,
            offset,
        } => Ok(i_type(0x2f, *base, operation.code(), *offset as u16)),
        SystemCall { code } => system_word(*code, 0x0c),
        Break { code } => system_word(*code, 0x0d),
        Synchronize => Ok(0x0000_000f),
        ExceptionReturn => Ok(0x4200_0018),
        SystemControlTransfer {
            to_control,
            general,
            control,
        } => Ok(0x4000_0000
            | if *to_control { 4_u32 } else { 0 } << 21
            | u32::from(general.index()) << 16
            | u32::from(control.index()) << 11),
        FloatingTransfer {
            to_floating,
            general,
            floating,
        } => Ok(0x4400_0000
            | if *to_floating { 4_u32 } else { 0 } << 21
            | u32::from(general.index()) << 16
            | u32::from(floating.index()) << 11),
        FloatingControlTransfer {
            to_control,
            general,
            control,
        } => {
            if *to_control && *control != FloatingControlRegister::ControlStatus {
                return Err(EncodeError::InvalidInstructionForm {
                    reason: "CTC1 only accepts FCR31",
                });
            }
            Ok(0x4400_0000
                | if *to_control { 6_u32 } else { 2 } << 21
                | u32::from(general.index()) << 16
                | u32::from(control.index()) << 11)
        }
        FloatingBranch {
            condition,
            likely,
            target,
        } => branch_word(
            0x11,
            GeneralRegister::from_field(8),
            u8::from(*condition == FloatingBranchCondition::True) | u8::from(*likely) << 1,
            address,
            *target,
        ),
        FloatingMemory {
            store,
            floating,
            base,
            offset,
        } => Ok(i_type(
            if *store { 0x39 } else { 0x31 },
            *base,
            floating.index(),
            *offset as u16,
        )),
        FloatingBinary {
            operation,
            destination,
            left,
            right,
        } => Ok(fpu_word(
            0x10,
            right.index(),
            left.index(),
            destination.index(),
            match operation {
                FloatingBinaryOperation::Add => 0x00,
                FloatingBinaryOperation::Subtract => 0x01,
                FloatingBinaryOperation::Multiply => 0x02,
                FloatingBinaryOperation::Divide => 0x03,
            },
        )),
        FloatingUnary {
            operation,
            destination,
            source,
        } => Ok(fpu_word(
            0x10,
            0,
            source.index(),
            destination.index(),
            match operation {
                FloatingUnaryOperation::SquareRoot => 0x04,
                FloatingUnaryOperation::AbsoluteValue => 0x05,
                FloatingUnaryOperation::Move => 0x06,
                FloatingUnaryOperation::Negate => 0x07,
            },
        )),
        ConvertWordToSingle {
            destination,
            source,
        } => Ok(fpu_word(0x14, 0, source.index(), destination.index(), 0x20)),
        ConvertSingleToWord {
            destination,
            source,
        } => Ok(fpu_word(0x10, 0, source.index(), destination.index(), 0x24)),
        FloatingRoundToWord {
            operation,
            destination,
            source,
        } => Ok(fpu_word(
            0x10,
            0,
            source.index(),
            destination.index(),
            match operation {
                FloatingRoundingOperation::Round => 0x0c,
                FloatingRoundingOperation::Truncate => 0x0d,
                FloatingRoundingOperation::Ceiling => 0x0e,
                FloatingRoundingOperation::Floor => 0x0f,
            },
        )),
        FloatingCompare {
            predicate,
            left,
            right,
        } => Ok(fpu_word(
            0x10,
            right.index(),
            left.index(),
            0,
            0x30 | predicate.code(),
        )),
        Vfpu(instruction) => encode_vfpu(instruction, address),
    }
}

/// Decodes one four-byte little-endian instruction at its code location.
pub fn decode_bytes(
    source: &[u8],
    location: crate::CodeLocation,
) -> Result<Instruction, DecodeError> {
    let bytes = source
        .get(..4)
        .ok_or(DecodeError::UnexpectedEnd { got: source.len() })?;
    decode_word(
        u32::from_le_bytes(bytes.try_into().expect("four-byte instruction")),
        location,
    )
}

/// Encodes one typed instruction as four little-endian bytes at its code location.
pub fn encode_bytes(
    instruction: &Instruction,
    location: crate::CodeLocation,
) -> Result<[u8; 4], EncodeError> {
    Ok(encode_word(instruction, location)?.to_le_bytes())
}

impl TypedIsa for Allegrex {
    const PROFILE_ID: &'static str = crate::PROFILE_ID;

    type Instruction = Instruction;
    type Form = InstructionForm;
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
        let instruction = decode_bytes(source, *at)?;
        let bytes = InstructionBytes::new(source[..4].to_vec())
            .expect("Allegrex instruction bytes are non-empty");
        Ok(ProtocolDecodedInstruction::new(instruction, bytes))
    }

    fn encode_one(
        instruction: &Self::Instruction,
        at: &Self::CodeLocation,
    ) -> Result<InstructionBytes, Self::EncodeError> {
        let bytes = encode_bytes(instruction, *at)?;
        Ok(
            InstructionBytes::new(bytes.to_vec())
                .expect("Allegrex instruction bytes are non-empty"),
        )
    }
}

fn decode_special(word: u32) -> Result<Instruction, DecodeError> {
    if word == 0 {
        return Ok(Instruction::Nop);
    }
    let instruction = match function(word) {
        0x00 => {
            require(word, rs_index(word) == 0)?;
            Instruction::ShiftImmediate {
                operation: ShiftOperation::LeftLogical,
                destination: rd(word),
                source: rt(word),
                amount: shift(word),
            }
        }
        0x02 => {
            let operation = match rs_index(word) {
                0 => ShiftOperation::RightLogical,
                1 => ShiftOperation::RotateRight,
                _ => return reserved(word),
            };
            Instruction::ShiftImmediate {
                operation,
                destination: rd(word),
                source: rt(word),
                amount: shift(word),
            }
        }
        0x03 => {
            require(word, rs_index(word) == 0)?;
            Instruction::ShiftImmediate {
                operation: ShiftOperation::RightArithmetic,
                destination: rd(word),
                source: rt(word),
                amount: shift(word),
            }
        }
        0x04 | 0x07 => {
            require(word, shift(word) == 0)?;
            Instruction::ShiftVariable {
                operation: if function(word) == 0x04 {
                    ShiftOperation::LeftLogical
                } else {
                    ShiftOperation::RightArithmetic
                },
                destination: rd(word),
                source: rt(word),
                amount_register: rs(word),
            }
        }
        0x06 => {
            let operation = match shift(word) {
                0 => ShiftOperation::RightLogical,
                1 => ShiftOperation::RotateRight,
                _ => return reserved(word),
            };
            Instruction::ShiftVariable {
                operation,
                destination: rd(word),
                source: rt(word),
                amount_register: rs(word),
            }
        }
        0x08 => {
            require(
                word,
                rt_index(word) == 0 && rd_index(word) == 0 && shift(word) == 0,
            )?;
            Instruction::JumpRegister {
                source: rs(word),
                link: None,
            }
        }
        0x09 => {
            require(word, rt_index(word) == 0 && shift(word) == 0)?;
            Instruction::JumpRegister {
                source: rs(word),
                link: Some(rd(word)),
            }
        }
        0x0a | 0x0b => {
            require(word, shift(word) == 0)?;
            Instruction::ConditionalMove {
                condition: if function(word) == 0x0a {
                    ConditionalMoveCondition::Zero
                } else {
                    ConditionalMoveCondition::Nonzero
                },
                destination: rd(word),
                source: rs(word),
                condition_register: rt(word),
            }
        }
        0x0c => Instruction::SystemCall {
            code: (word >> 6) & 0x000f_ffff,
        },
        0x0d => Instruction::Break {
            code: (word >> 6) & 0x000f_ffff,
        },
        0x0f => {
            require(word, word == 0x0000_000f)?;
            Instruction::Synchronize
        }
        0x10 | 0x12 => {
            require(
                word,
                rs_index(word) == 0 && rt_index(word) == 0 && shift(word) == 0,
            )?;
            Instruction::MoveFromSpecial {
                source: if function(word) == 0x10 {
                    SpecialRegister::Hi
                } else {
                    SpecialRegister::Lo
                },
                destination: rd(word),
            }
        }
        0x11 | 0x13 => {
            require(
                word,
                rt_index(word) == 0 && rd_index(word) == 0 && shift(word) == 0,
            )?;
            Instruction::MoveToSpecial {
                destination: if function(word) == 0x11 {
                    SpecialRegister::Hi
                } else {
                    SpecialRegister::Lo
                },
                source: rs(word),
            }
        }
        0x16 | 0x17 => {
            require(word, rt_index(word) == 0 && shift(word) == 0)?;
            Instruction::BitCount {
                operation: if function(word) == 0x16 {
                    BitCountOperation::LeadingZeros
                } else {
                    BitCountOperation::LeadingOnes
                },
                destination: rd(word),
                source: rs(word),
            }
        }
        0x18..=0x1b => {
            require(word, rd_index(word) == 0 && shift(word) == 0)?;
            Instruction::MultiplyDivide {
                operation: match function(word) {
                    0x18 => MultiplyDivideOperation::Multiply,
                    0x19 => MultiplyDivideOperation::MultiplyUnsigned,
                    0x1a => MultiplyDivideOperation::Divide,
                    0x1b => MultiplyDivideOperation::DivideUnsigned,
                    _ => unreachable!(),
                },
                left: rs(word),
                right: rt(word),
            }
        }
        0x1c | 0x1d | 0x2e | 0x2f => {
            require(word, rd_index(word) == 0 && shift(word) == 0)?;
            Instruction::Accumulate {
                operation: match function(word) {
                    0x1c => AccumulateOperation::MultiplyAdd,
                    0x1d => AccumulateOperation::MultiplyAddUnsigned,
                    0x2e => AccumulateOperation::MultiplySubtract,
                    0x2f => AccumulateOperation::MultiplySubtractUnsigned,
                    _ => unreachable!(),
                },
                left: rs(word),
                right: rt(word),
            }
        }
        0x20..=0x27 | 0x2a..=0x2d => {
            require(word, shift(word) == 0)?;
            Instruction::ThreeRegister {
                operation: match function(word) {
                    0x20 => ThreeRegisterOperation::Add,
                    0x21 => ThreeRegisterOperation::AddUnsigned,
                    0x22 => ThreeRegisterOperation::Subtract,
                    0x23 => ThreeRegisterOperation::SubtractUnsigned,
                    0x24 => ThreeRegisterOperation::And,
                    0x25 => ThreeRegisterOperation::Or,
                    0x26 => ThreeRegisterOperation::ExclusiveOr,
                    0x27 => ThreeRegisterOperation::Nor,
                    0x2a => ThreeRegisterOperation::SetLessThan,
                    0x2b => ThreeRegisterOperation::SetLessThanUnsigned,
                    0x2c => ThreeRegisterOperation::Maximum,
                    0x2d => ThreeRegisterOperation::Minimum,
                    _ => unreachable!(),
                },
                destination: rd(word),
                left: rs(word),
                right: rt(word),
            }
        }
        _ => return reserved(word),
    };
    Ok(instruction)
}

fn decode_regimm(word: u32, address: u32) -> Result<Instruction, DecodeError> {
    let (condition, likely, link) = match rt_index(word) {
        0x00 => (SignCondition::LessThanZero, false, false),
        0x01 => (SignCondition::GreaterThanOrEqualZero, false, false),
        0x02 => (SignCondition::LessThanZero, true, false),
        0x03 => (SignCondition::GreaterThanOrEqualZero, true, false),
        0x10 => (SignCondition::LessThanZero, false, true),
        0x11 => (SignCondition::GreaterThanOrEqualZero, false, true),
        0x12 => (SignCondition::LessThanZero, true, true),
        0x13 => (SignCondition::GreaterThanOrEqualZero, true, true),
        _ => return reserved(word),
    };
    Ok(Instruction::SignBranch {
        condition,
        likely,
        link,
        source: rs(word),
        target: branch_target(word, address),
    })
}

fn decode_cp0(word: u32) -> Result<Instruction, DecodeError> {
    match rs_index(word) {
        0 | 4 => {
            require(word, word & 0x7ff == 0)?;
            Ok(Instruction::SystemControlTransfer {
                to_control: rs_index(word) == 4,
                general: rt(word),
                control: SystemControlRegister::from_field(rd_index(word)),
            })
        }
        16 => {
            require(word, word == 0x4200_0018)?;
            Ok(Instruction::ExceptionReturn)
        }
        _ => reserved(word),
    }
}

fn decode_fpu(word: u32, address: u32) -> Result<Instruction, DecodeError> {
    match rs_index(word) {
        0 | 4 => {
            require(word, word & 0x7ff == 0)?;
            Ok(Instruction::FloatingTransfer {
                to_floating: rs_index(word) == 4,
                general: rt(word),
                floating: FloatingRegister::from_field(rd_index(word)),
            })
        }
        2 => {
            require(word, word & 0x7ff == 0)?;
            let control = FloatingControlRegister::from_cfc1_field(rd_index(word))
                .ok_or(DecodeError::ReservedWord { word })?;
            Ok(Instruction::FloatingControlTransfer {
                to_control: false,
                general: rt(word),
                control,
            })
        }
        6 => {
            require(word, word & 0x7ff == 0 && rd_index(word) == 31)?;
            Ok(Instruction::FloatingControlTransfer {
                to_control: true,
                general: rt(word),
                control: FloatingControlRegister::ControlStatus,
            })
        }
        8 => {
            let (condition, likely) = match rt_index(word) {
                0 => (FloatingBranchCondition::False, false),
                1 => (FloatingBranchCondition::True, false),
                2 => (FloatingBranchCondition::False, true),
                3 => (FloatingBranchCondition::True, true),
                _ => return reserved(word),
            };
            Ok(Instruction::FloatingBranch {
                condition,
                likely,
                target: branch_target(word, address),
            })
        }
        16 => decode_fpu_single(word),
        20 => {
            require(word, ft_index(word) == 0 && function(word) == 0x20)?;
            Ok(Instruction::ConvertWordToSingle {
                destination: fd(word),
                source: fs(word),
            })
        }
        _ => reserved(word),
    }
}

fn decode_fpu_single(word: u32) -> Result<Instruction, DecodeError> {
    match function(word) {
        0x00..=0x03 => Ok(Instruction::FloatingBinary {
            operation: match function(word) {
                0x00 => FloatingBinaryOperation::Add,
                0x01 => FloatingBinaryOperation::Subtract,
                0x02 => FloatingBinaryOperation::Multiply,
                0x03 => FloatingBinaryOperation::Divide,
                _ => unreachable!(),
            },
            destination: fd(word),
            left: fs(word),
            right: ft(word),
        }),
        0x04..=0x07 => {
            require(word, ft_index(word) == 0)?;
            Ok(Instruction::FloatingUnary {
                operation: match function(word) {
                    0x04 => FloatingUnaryOperation::SquareRoot,
                    0x05 => FloatingUnaryOperation::AbsoluteValue,
                    0x06 => FloatingUnaryOperation::Move,
                    0x07 => FloatingUnaryOperation::Negate,
                    _ => unreachable!(),
                },
                destination: fd(word),
                source: fs(word),
            })
        }
        0x0c..=0x0f => {
            require(word, ft_index(word) == 0)?;
            Ok(Instruction::FloatingRoundToWord {
                operation: match function(word) {
                    0x0c => FloatingRoundingOperation::Round,
                    0x0d => FloatingRoundingOperation::Truncate,
                    0x0e => FloatingRoundingOperation::Ceiling,
                    0x0f => FloatingRoundingOperation::Floor,
                    _ => unreachable!(),
                },
                destination: fd(word),
                source: fs(word),
            })
        }
        0x24 => {
            require(word, ft_index(word) == 0)?;
            Ok(Instruction::ConvertSingleToWord {
                destination: fd(word),
                source: fs(word),
            })
        }
        0x30..=0x3f => {
            require(word, fd_index(word) == 0)?;
            Ok(Instruction::FloatingCompare {
                predicate: FloatingComparePredicate::from_field(function(word) & 0x0f),
                left: fs(word),
                right: ft(word),
            })
        }
        _ => reserved(word),
    }
}

fn decode_special3(word: u32) -> Result<Instruction, DecodeError> {
    match function(word) {
        0x00 => {
            let position = shift(word);
            let size = rd_index(word) + 1;
            require(word, u16::from(position) + u16::from(size) <= 32)?;
            Ok(Instruction::BitField {
                operation: BitFieldOperation::Extract,
                destination: rt(word),
                source: rs(word),
                position,
                size,
            })
        }
        0x04 => {
            let position = shift(word);
            let most_significant = rd_index(word);
            require(word, most_significant >= position)?;
            Ok(Instruction::BitField {
                operation: BitFieldOperation::Insert,
                destination: rt(word),
                source: rs(word),
                position,
                size: most_significant - position + 1,
            })
        }
        0x20 => {
            require(word, rs_index(word) == 0)?;
            let operation = match shift(word) {
                0x02 => ByteTransformOperation::SwapBytesWithinHalfwords,
                0x03 => ByteTransformOperation::SwapBytesWithinWord,
                0x10 => ByteTransformOperation::SignExtendByte,
                0x14 => ByteTransformOperation::ReverseBits,
                0x18 => ByteTransformOperation::SignExtendHalfword,
                _ => return reserved(word),
            };
            Ok(Instruction::ByteTransform {
                operation,
                destination: rd(word),
                source: rt(word),
            })
        }
        _ => reserved(word),
    }
}

fn decode_memory(word: u32) -> Result<Instruction, DecodeError> {
    Ok(Instruction::Memory {
        operation: match opcode(word) {
            0x20 => MemoryOperation::LoadByte,
            0x21 => MemoryOperation::LoadHalfword,
            0x22 => MemoryOperation::LoadWordLeft,
            0x23 => MemoryOperation::LoadWord,
            0x24 => MemoryOperation::LoadByteUnsigned,
            0x25 => MemoryOperation::LoadHalfwordUnsigned,
            0x26 => MemoryOperation::LoadWordRight,
            0x28 => MemoryOperation::StoreByte,
            0x29 => MemoryOperation::StoreHalfword,
            0x2a => MemoryOperation::StoreWordLeft,
            0x2b => MemoryOperation::StoreWord,
            0x2e => MemoryOperation::StoreWordRight,
            0x30 => MemoryOperation::LoadLinked,
            0x38 => MemoryOperation::StoreConditional,
            _ => unreachable!(),
        },
        register: rt(word),
        base: rs(word),
        offset: word as u16 as i16,
    })
}

fn decode_vfpu(word: u32, address: u32) -> Result<Option<VfpuInstruction>, DecodeError> {
    let mut saw_candidate = false;
    for specificity in (0..=32).rev() {
        for descriptor in VFPU_DESCRIPTORS.iter().filter(|descriptor| {
            descriptor.mask.count_ones() == specificity
                && word & descriptor.mask == descriptor.value
        }) {
            saw_candidate = true;
            if let Some(operands) =
                decode_vfpu_operands(descriptor.form, descriptor.layout, word, address)
            {
                return VfpuInstruction::new(descriptor.form, operands)
                    .map(Some)
                    .map_err(|_| DecodeError::ReservedWord { word });
            }
        }
    }
    if saw_candidate {
        Err(DecodeError::ReservedWord { word })
    } else {
        Ok(None)
    }
}

fn decode_vfpu_operands(
    form: VfpuForm,
    layout: OperandLayout,
    word: u32,
    address: u32,
) -> Option<VfpuOperands> {
    let vd = || VectorRegister::from_field((word & 0x7f) as u8);
    let vs = || VectorRegister::from_field(((word >> 8) & 0x7f) as u8);
    let vt = || VectorRegister::from_field(((word >> 16) & 0x7f) as u8);
    Some(match layout {
        OperandLayout::None => VfpuOperands::None,
        OperandLayout::Branch => VfpuOperands::Branch {
            condition: crate::VfpuConditionBit::from_field(((word >> 18) & 7) as u8),
            target: branch_target(word, address),
        },
        OperandLayout::MemoryVector => {
            let size = form.size()?;
            let index = match size {
                VectorSize::Single => rt_index(word) | ((word & 3) as u8) << 5,
                VectorSize::Quad => rt_index(word) | ((word & 1) as u8) << 5,
                _ => return None,
            };
            VfpuOperands::MemoryVector {
                vector: VectorRegister::from_field(index),
                base: rs(word),
                offset: VfpuMemoryOffset::new((word as u16 & 0xfffc) as i16)?,
            }
        }
        OperandLayout::GeneralAndVector => VfpuOperands::GeneralAndVector {
            general: rt(word),
            vector: vd(),
        },
        OperandLayout::GeneralAndControl => {
            let control = VfpuControlRegister::new(word as u8)?;
            if form.operation() == VfpuOperation::MoveToControl && !control.is_writable() {
                return None;
            }
            VfpuOperands::GeneralAndControl {
                general: rt(word),
                control,
            }
        }
        OperandLayout::ThreeVectors => VfpuOperands::ThreeVectors {
            destination: vd(),
            left: vs(),
            right: vt(),
        },
        OperandLayout::TwoVectors => VfpuOperands::TwoVectors {
            destination: vd(),
            source: vs(),
        },
        OperandLayout::ConditionalMove => VfpuOperands::ConditionalMove {
            destination: vd(),
            source: vs(),
            condition: crate::VfpuConditionBit::from_field(((word >> 16) & 7) as u8),
        },
        OperandLayout::Compare => VfpuOperands::Compare {
            condition: VfpuCompareCondition::from_field((word & 0x0f) as u8),
            left: vs(),
            right: vt(),
        },
        OperandLayout::Constant => VfpuOperands::Constant {
            destination: vd(),
            constant: VfpuConstant::new(((word >> 16) & 0x1f) as u8)?,
        },
        OperandLayout::ImmediateAndTwoVectors => VfpuOperands::ImmediateAndTwoVectors {
            destination: vd(),
            source: vs(),
            immediate: Immediate5::from_field((word >> 16) & 0x1f),
        },
        OperandLayout::VectorImmediate16 => VfpuOperands::VectorImmediate16 {
            destination: VectorRegister::from_field(((word >> 16) & 0x7f) as u8),
            immediate: Immediate16::from_field(word & 0xffff),
        },
        OperandLayout::DestinationVector => VfpuOperands::DestinationVector { destination: vd() },
        OperandLayout::ControlAndVector => match form.operation() {
            VfpuOperation::MoveFromVfpuControl => VfpuOperands::ControlAndVector {
                control: VfpuControlRegister::new(((word >> 8) & 0xff) as u8)?,
                vector: vd(),
            },
            VfpuOperation::MoveToVfpuControl => {
                let control = VfpuControlRegister::new((word & 0xff) as u8)?;
                if !control.is_writable() {
                    return None;
                }
                VfpuOperands::ControlAndVector {
                    control,
                    vector: vs(),
                }
            }
            _ => return None,
        },
        OperandLayout::SourcePrefix => {
            if word & 0x00f0_0000 != 0 {
                return None;
            }
            VfpuOperands::SourcePrefix {
                lanes: decode_source_prefix(word),
            }
        }
        OperandLayout::DestinationPrefix => {
            if word & 0x00ff_f000 != 0 {
                return None;
            }
            VfpuOperands::DestinationPrefix {
                lanes: decode_destination_prefix(word)?,
            }
        }
        OperandLayout::SourceVector => VfpuOperands::SourceVector { source: vs() },
        OperandLayout::ThreeMatrices => {
            let size = form.size()?;
            VfpuOperands::ThreeMatrices {
                destination: MatrixRegister::from_field((word & 0x7f) as u8),
                left: MatrixRegister::from_field(decode_transposed_matrix(
                    ((word >> 8) & 0x7f) as u8,
                    size,
                )),
                right: MatrixRegister::from_field(((word >> 16) & 0x7f) as u8),
            }
        }
        OperandLayout::TwoMatrices => VfpuOperands::TwoMatrices {
            destination: MatrixRegister::from_field((word & 0x7f) as u8),
            source: MatrixRegister::from_field(((word >> 8) & 0x7f) as u8),
        },
        OperandLayout::DestinationMatrix => VfpuOperands::DestinationMatrix {
            destination: MatrixRegister::from_field((word & 0x7f) as u8),
        },
        OperandLayout::MatrixScale => VfpuOperands::MatrixScale {
            destination: MatrixRegister::from_field((word & 0x7f) as u8),
            source: MatrixRegister::from_field(((word >> 8) & 0x7f) as u8),
            scale: vt(),
        },
        OperandLayout::MatrixVectorTransform => VfpuOperands::MatrixVectorTransform {
            destination: vd(),
            matrix: MatrixRegister::from_field(((word >> 8) & 0x7f) as u8),
            vector: vt(),
        },
        OperandLayout::Immediate8AndTwoVectors => VfpuOperands::Immediate8AndTwoVectors {
            destination: vd(),
            source: vs(),
            immediate: Immediate8::from_field((word >> 16) & 0xff),
        },
    })
}

fn encode_vfpu(instruction: &VfpuInstruction, address: u32) -> Result<u32, EncodeError> {
    let descriptor = crate::vfpu::descriptor_for_form(instruction.form()).ok_or(
        EncodeError::InvalidInstructionForm {
            reason: "VFPU form is outside the manual profile",
        },
    )?;
    let mut word = descriptor.value;
    match instruction.operands() {
        VfpuOperands::None => {}
        VfpuOperands::Branch { condition, target } => {
            word |= u32::from(condition.index()) << 18;
            word |= u32::from(branch_immediate(address, *target)?);
        }
        VfpuOperands::MemoryVector {
            vector,
            base,
            offset,
        } => {
            word |= u32::from(base.index()) << 21;
            word |= u32::from(vector.index() & 0x1f) << 16;
            word |= u32::from(offset.value() as u16 & 0xfffc);
            match instruction.form().size() {
                Some(VectorSize::Single) => word |= u32::from(vector.index() >> 5) & 3,
                Some(VectorSize::Quad) => {
                    if vector.index() >= 64 {
                        return Err(EncodeError::FieldOutOfRange {
                            field: "quad VFPU register",
                            value: u32::from(vector.index()),
                            maximum: 63,
                        });
                    }
                    word |= u32::from(vector.index() >> 5) & 1;
                }
                _ => {
                    return Err(EncodeError::InvalidInstructionForm {
                        reason: "VFPU memory instruction has an unsupported vector size",
                    });
                }
            }
        }
        VfpuOperands::GeneralAndVector { general, vector } => {
            word |= u32::from(general.index()) << 16 | u32::from(vector.index());
        }
        VfpuOperands::GeneralAndControl { general, control } => {
            word |= u32::from(general.index()) << 16 | u32::from(control.index());
        }
        VfpuOperands::ThreeVectors {
            destination,
            left,
            right,
        } => word |= three_vfpu_fields(destination.index(), left.index(), right.index()),
        VfpuOperands::TwoVectors {
            destination,
            source,
        } => word |= u32::from(source.index()) << 8 | u32::from(destination.index()),
        VfpuOperands::ConditionalMove {
            destination,
            source,
            condition,
        } => {
            word |= u32::from(condition.index()) << 16
                | u32::from(source.index()) << 8
                | u32::from(destination.index());
        }
        VfpuOperands::Compare {
            condition,
            left,
            right,
        } => {
            word |= u32::from(right.index()) << 16
                | u32::from(left.index()) << 8
                | u32::from(condition.code());
        }
        VfpuOperands::Constant {
            destination,
            constant,
        } => word |= u32::from(constant.code()) << 16 | u32::from(destination.index()),
        VfpuOperands::ImmediateAndTwoVectors {
            destination,
            source,
            immediate,
        } => {
            word |= immediate.value() << 16
                | u32::from(source.index()) << 8
                | u32::from(destination.index());
        }
        VfpuOperands::VectorImmediate16 {
            destination,
            immediate,
        } => word |= u32::from(destination.index()) << 16 | immediate.value(),
        VfpuOperands::DestinationVector { destination } => {
            word |= u32::from(destination.index());
        }
        VfpuOperands::ControlAndVector { control, vector } => {
            match instruction.form().operation() {
                VfpuOperation::MoveFromVfpuControl => {
                    word |= u32::from(control.index()) << 8 | u32::from(vector.index());
                }
                VfpuOperation::MoveToVfpuControl => {
                    word |= u32::from(vector.index()) << 8 | u32::from(control.index());
                }
                _ => {
                    return Err(EncodeError::InvalidInstructionForm {
                        reason: "VFPU control operands used with a non-control form",
                    });
                }
            }
        }
        VfpuOperands::SourcePrefix { lanes } => word |= encode_source_prefix(*lanes),
        VfpuOperands::DestinationPrefix { lanes } => word |= encode_destination_prefix(*lanes),
        VfpuOperands::SourceVector { source } => word |= u32::from(source.index()) << 8,
        VfpuOperands::ThreeMatrices {
            destination,
            left,
            right,
        } => {
            let size = instruction
                .form()
                .size()
                .ok_or(EncodeError::InvalidInstructionForm {
                    reason: "matrix multiply has no matrix size",
                })?;
            word |= three_vfpu_fields(
                destination.index(),
                encode_transposed_matrix(left.index(), size),
                right.index(),
            );
        }
        VfpuOperands::TwoMatrices {
            destination,
            source,
        } => word |= u32::from(source.index()) << 8 | u32::from(destination.index()),
        VfpuOperands::DestinationMatrix { destination } => {
            word |= u32::from(destination.index());
        }
        VfpuOperands::MatrixScale {
            destination,
            source,
            scale,
        } => word |= three_vfpu_fields(destination.index(), source.index(), scale.index()),
        VfpuOperands::MatrixVectorTransform {
            destination,
            matrix,
            vector,
        } => word |= three_vfpu_fields(destination.index(), matrix.index(), vector.index()),
        VfpuOperands::Immediate8AndTwoVectors {
            destination,
            source,
            immediate,
        } => {
            word |= immediate.value() << 16
                | u32::from(source.index()) << 8
                | u32::from(destination.index());
        }
    }
    if word & descriptor.mask != descriptor.value {
        return Err(EncodeError::InvalidInstructionForm {
            reason: "VFPU operands overwrite fixed instruction bits",
        });
    }
    Ok(word)
}

fn decode_source_prefix(word: u32) -> [SourcePrefixLane; 4] {
    std::array::from_fn(|lane| {
        let selector = ((word >> (lane * 2)) & 3) as u8;
        let absolute_or_constant_high = (word >> (8 + lane)) & 1 != 0;
        let constant = (word >> (12 + lane)) & 1 != 0;
        let value = if constant {
            let code = 8 | u8::from(absolute_or_constant_high) << 2 | selector;
            SourcePrefixValue::Constant(SourcePrefixConstant::from_field(code))
        } else {
            SourcePrefixValue::Component {
                component: VectorComponent::from_field(selector),
                absolute: absolute_or_constant_high,
            }
        };
        SourcePrefixLane {
            value,
            negate: word >> (16 + lane) & 1 != 0,
        }
    })
}

fn encode_source_prefix(lanes: [SourcePrefixLane; 4]) -> u32 {
    lanes
        .into_iter()
        .enumerate()
        .fold(0, |word, (lane, prefix)| {
            let (selector, absolute_or_constant_high, constant) = match prefix.value {
                SourcePrefixValue::Component {
                    component,
                    absolute,
                } => (component as u8, absolute, false),
                SourcePrefixValue::Constant(value) => {
                    let code = value as u8;
                    (code & 3, code & 4 != 0, true)
                }
            };
            word | u32::from(selector) << (lane * 2)
                | u32::from(absolute_or_constant_high) << (8 + lane)
                | u32::from(constant) << (12 + lane)
                | u32::from(prefix.negate) << (16 + lane)
        })
}

fn decode_destination_prefix(word: u32) -> Option<[DestinationPrefixLane; 4]> {
    fn lane(word: u32, index: usize) -> Option<DestinationPrefixLane> {
        Some(DestinationPrefixLane {
            saturation: DestinationSaturation::from_field(((word >> (index * 2)) & 3) as u8)?,
            mask: word >> (8 + index) & 1 != 0,
        })
    }
    Some([
        lane(word, 0)?,
        lane(word, 1)?,
        lane(word, 2)?,
        lane(word, 3)?,
    ])
}

fn encode_destination_prefix(lanes: [DestinationPrefixLane; 4]) -> u32 {
    lanes
        .into_iter()
        .enumerate()
        .fold(0, |word, (lane, prefix)| {
            word | u32::from(prefix.saturation as u8) << (lane * 2)
                | u32::from(prefix.mask) << (8 + lane)
        })
}

fn encode_bit_field(
    operation: BitFieldOperation,
    destination: GeneralRegister,
    source: GeneralRegister,
    position: u8,
    size: u8,
) -> Result<u32, EncodeError> {
    check_field("bit-field position", u32::from(position), 31)?;
    if size == 0 || u16::from(position) + u16::from(size) > 32 {
        return Err(EncodeError::InvalidInstructionForm {
            reason: "bit-field size must be non-zero and remain within a 32-bit word",
        });
    }
    let encoded_size = match operation {
        BitFieldOperation::Extract => size - 1,
        BitFieldOperation::Insert => position + size - 1,
    };
    Ok(0x7c00_0000
        | u32::from(source.index()) << 21
        | u32::from(destination.index()) << 16
        | u32::from(encoded_size) << 11
        | u32::from(position) << 6
        | if operation == BitFieldOperation::Extract {
            0
        } else {
            4
        })
}

fn special3_transform(
    operation: ByteTransformOperation,
    destination: GeneralRegister,
    source: GeneralRegister,
) -> u32 {
    0x7c00_0020
        | u32::from(source.index()) << 16
        | u32::from(destination.index()) << 11
        | (match operation {
            ByteTransformOperation::SwapBytesWithinHalfwords => 0x02,
            ByteTransformOperation::SwapBytesWithinWord => 0x03,
            ByteTransformOperation::SignExtendByte => 0x10,
            ByteTransformOperation::ReverseBits => 0x14,
            ByteTransformOperation::SignExtendHalfword => 0x18,
        } as u32)
            << 6
}

fn encode_sign_branch(
    condition: SignCondition,
    likely: bool,
    link: bool,
    source: GeneralRegister,
    address: u32,
    target: u32,
) -> Result<u32, EncodeError> {
    match condition {
        SignCondition::LessThanZero | SignCondition::GreaterThanOrEqualZero => {
            let selector = match (condition, likely, link) {
                (SignCondition::LessThanZero, false, false) => 0x00,
                (SignCondition::GreaterThanOrEqualZero, false, false) => 0x01,
                (SignCondition::LessThanZero, true, false) => 0x02,
                (SignCondition::GreaterThanOrEqualZero, true, false) => 0x03,
                (SignCondition::LessThanZero, false, true) => 0x10,
                (SignCondition::GreaterThanOrEqualZero, false, true) => 0x11,
                (SignCondition::LessThanZero, true, true) => 0x12,
                (SignCondition::GreaterThanOrEqualZero, true, true) => 0x13,
                _ => unreachable!(),
            };
            branch_word(0x01, source, selector, address, target)
        }
        SignCondition::LessThanOrEqualZero | SignCondition::GreaterThanZero => {
            if link {
                return Err(EncodeError::InvalidInstructionForm {
                    reason: "BLEZ and BGTZ forms do not link",
                });
            }
            branch_word(
                match (condition, likely) {
                    (SignCondition::LessThanOrEqualZero, false) => 0x06,
                    (SignCondition::GreaterThanZero, false) => 0x07,
                    (SignCondition::LessThanOrEqualZero, true) => 0x16,
                    (SignCondition::GreaterThanZero, true) => 0x17,
                    _ => unreachable!(),
                },
                source,
                0,
                address,
                target,
            )
        }
    }
}

fn memory_opcode(operation: MemoryOperation) -> u8 {
    match operation {
        MemoryOperation::LoadByte => 0x20,
        MemoryOperation::LoadHalfword => 0x21,
        MemoryOperation::LoadWordLeft => 0x22,
        MemoryOperation::LoadWord => 0x23,
        MemoryOperation::LoadByteUnsigned => 0x24,
        MemoryOperation::LoadHalfwordUnsigned => 0x25,
        MemoryOperation::LoadWordRight => 0x26,
        MemoryOperation::StoreByte => 0x28,
        MemoryOperation::StoreHalfword => 0x29,
        MemoryOperation::StoreWordLeft => 0x2a,
        MemoryOperation::StoreWord => 0x2b,
        MemoryOperation::StoreWordRight => 0x2e,
        MemoryOperation::LoadLinked => 0x30,
        MemoryOperation::StoreConditional => 0x38,
    }
}

fn branch_word(
    opcode: u8,
    rs: GeneralRegister,
    rt: u8,
    address: u32,
    target: u32,
) -> Result<u32, EncodeError> {
    Ok(i_type(opcode, rs, rt, branch_immediate(address, target)?))
}

fn branch_immediate(address: u32, target: u32) -> Result<u16, EncodeError> {
    if target & 3 != 0 {
        return Err(EncodeError::UnalignedTarget { target });
    }
    let displacement = target.wrapping_sub(address.wrapping_add(4)) as i32;
    if displacement & 3 != 0 {
        return Err(EncodeError::UnalignedTarget { target });
    }
    let words = displacement / 4;
    Ok(i16::try_from(words).map_err(|_| EncodeError::BranchOutOfRange { address, target })? as u16)
}

fn jump_word(opcode: u8, address: u32, target: u32) -> Result<u32, EncodeError> {
    if target & 3 != 0 {
        return Err(EncodeError::UnalignedTarget { target });
    }
    if target & 0xf000_0000 != address.wrapping_add(4) & 0xf000_0000 {
        return Err(EncodeError::JumpRegionMismatch { address, target });
    }
    Ok(u32::from(opcode) << 26 | target >> 2 & 0x03ff_ffff)
}

fn branch_target(word: u32, address: u32) -> u32 {
    address
        .wrapping_add(4)
        .wrapping_add(((word as u16 as i16 as i32) << 2) as u32)
}

fn jump_target(word: u32, address: u32) -> u32 {
    address.wrapping_add(4) & 0xf000_0000 | (word & 0x03ff_ffff) << 2
}

fn system_word(code: u32, function: u8) -> Result<u32, EncodeError> {
    check_field("system exception code", code, 0x000f_ffff)?;
    Ok(code << 6 | u32::from(function))
}

fn r_type(
    rs: GeneralRegister,
    rt: GeneralRegister,
    rd: GeneralRegister,
    shift: u8,
    function: u8,
) -> u32 {
    u32::from(rs.index()) << 21
        | u32::from(rt.index()) << 16
        | u32::from(rd.index()) << 11
        | u32::from(shift) << 6
        | u32::from(function)
}

fn i_type(opcode: u8, rs: GeneralRegister, rt: u8, immediate: u16) -> u32 {
    u32::from(opcode) << 26
        | u32::from(rs.index()) << 21
        | u32::from(rt) << 16
        | u32::from(immediate)
}

fn fpu_word(format: u8, ft: u8, fs: u8, fd: u8, function: u8) -> u32 {
    0x4400_0000
        | u32::from(format) << 21
        | u32::from(ft) << 16
        | u32::from(fs) << 11
        | u32::from(fd) << 6
        | u32::from(function)
}

fn three_vfpu_fields(destination: u8, left: u8, right: u8) -> u32 {
    u32::from(right) << 16 | u32::from(left) << 8 | u32::from(destination)
}

fn encode_transposed_matrix(index: u8, size: VectorSize) -> u8 {
    transpose_matrix_bits(index, size)
}

fn decode_transposed_matrix(index: u8, size: VectorSize) -> u8 {
    transpose_matrix_bits(index, size)
}

fn transpose_matrix_bits(mut index: u8, size: VectorSize) -> u8 {
    index ^= 1 << 5;
    let index_bit = if size == VectorSize::Triple { 0 } else { 1 };
    let field_select = (index >> 6) & 1;
    let index_select = (index >> index_bit) & 1;
    index = (index & !(1 << 6) & !(1 << index_bit)) | index_select << 6 | field_select << index_bit;
    index
}

fn checked_decode_location(location: crate::CodeLocation) -> Result<u32, DecodeError> {
    if location & 3 != 0 {
        return Err(DecodeError::UnalignedAddress { location });
    }
    Ok(location)
}

fn checked_encode_location(location: crate::CodeLocation) -> Result<u32, EncodeError> {
    if location & 3 != 0 {
        return Err(EncodeError::UnalignedAddress { location });
    }
    Ok(location)
}

fn check_field(field: &'static str, value: u32, maximum: u32) -> Result<(), EncodeError> {
    if value <= maximum {
        Ok(())
    } else {
        Err(EncodeError::FieldOutOfRange {
            field,
            value,
            maximum,
        })
    }
}

fn require(word: u32, condition: bool) -> Result<(), DecodeError> {
    if condition { Ok(()) } else { reserved(word) }
}

fn reserved<T>(word: u32) -> Result<T, DecodeError> {
    Err(DecodeError::ReservedWord { word })
}

const fn opcode(word: u32) -> u8 {
    (word >> 26) as u8
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

const fn shift(word: u32) -> u8 {
    ((word >> 6) & 0x1f) as u8
}

const fn function(word: u32) -> u8 {
    (word & 0x3f) as u8
}

const fn rs(word: u32) -> GeneralRegister {
    GeneralRegister::from_field(rs_index(word))
}

const fn rt(word: u32) -> GeneralRegister {
    GeneralRegister::from_field(rt_index(word))
}

const fn rd(word: u32) -> GeneralRegister {
    GeneralRegister::from_field(rd_index(word))
}

const fn ft_index(word: u32) -> u8 {
    rt_index(word)
}

const fn fs_index(word: u32) -> u8 {
    rd_index(word)
}

const fn fd_index(word: u32) -> u8 {
    shift(word)
}

const fn ft(word: u32) -> FloatingRegister {
    FloatingRegister::from_field(ft_index(word))
}

const fn fs(word: u32) -> FloatingRegister {
    FloatingRegister::from_field(fs_index(word))
}

const fn fd(word: u32) -> FloatingRegister {
    FloatingRegister::from_field(fd_index(word))
}
