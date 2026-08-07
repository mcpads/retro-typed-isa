use thiserror::Error;
use typed_isa_core::{
    DecodedInstruction as ProtocolDecodedInstruction, InstructionBytes, TypedIsa,
};

use crate::instruction::{
    CacheOperation, Coprocessor, Coprocessor2ControlRegister, Coprocessor2Register,
    CoprocessorBranchCondition, CoprocessorMemoryOperation, CoprocessorMemoryRegister,
    Cp0Operation, Cp0Register, EqualityCondition, FixedFormat, FloatingBinaryOperation,
    FloatingComparePredicate, FloatingFormat, FloatingRegister, FloatingRounding,
    FloatingUnaryOperation, FpuControlRegister, GeneralRegister, ImmediateArithmeticOperation,
    Instruction, LogicalImmediateOperation, MemoryOperation, MultiplyDivideOperation,
    NumericFormat, ShiftOperation, SignCondition, SpecialRegister, ThreeRegisterOperation,
    TrapCondition,
};

#[derive(Debug, Clone, Copy)]
/// The fixed-width, big-endian VR4300 protocol profile.
pub struct Vr4300;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
/// Failure returned while decoding a VR4300 instruction.
pub enum DecodeError {
    #[error("VR4300 instruction address {address:#018x} is not four-byte aligned")]
    /// The supplied instruction address is not four-byte aligned.
    UnalignedAddress {
        /// The rejected address.
        address: u64,
    },
    #[error("truncated VR4300 instruction: needed 4 bytes, got {got}")]
    /// Fewer than four input bytes are available.
    UnexpectedEnd {
        /// The available input length.
        got: usize,
    },
    #[error("reserved or out-of-profile VR4300 instruction word {word:#010x}")]
    /// The word is reserved or belongs to an excluded implementation-defined operation.
    ReservedWord {
        /// The rejected instruction word.
        word: u32,
    },
    #[error("VR4300 instruction at {address:#018x} overflows the address space")]
    /// Computing the architecturally visible next program counter overflowed.
    AddressOverflow {
        /// The instruction address.
        address: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
/// Failure returned when a typed instruction cannot be represented at its placement address.
pub enum EncodeError {
    #[error("VR4300 instruction address {address:#018x} is not four-byte aligned")]
    /// The supplied instruction address is not four-byte aligned.
    UnalignedAddress {
        /// The rejected address.
        address: u64,
    },
    #[error("control-flow target {target:#018x} is not four-byte aligned")]
    /// A control-flow target is not four-byte aligned.
    UnalignedTarget {
        /// The rejected target.
        target: u64,
    },
    #[error("VR4300 instruction at {address:#018x} overflows the address space")]
    /// Computing the architecturally visible next program counter overflowed.
    AddressOverflow {
        /// The instruction address.
        address: u64,
    },
    #[error("branch at {address:#018x} cannot reach {target:#018x}")]
    /// A PC-relative target does not fit the signed sixteen-bit word displacement.
    BranchOutOfRange {
        /// The branch instruction address.
        address: u64,
        /// The requested target address.
        target: u64,
    },
    #[error("jump at {address:#018x} cannot reach region containing {target:#018x}")]
    /// A direct jump target is outside the high-address region selected by `PC + 4`.
    JumpRegionMismatch {
        /// The jump instruction address.
        address: u64,
        /// The requested target address.
        target: u64,
    },
    #[error("shift amount {amount} exceeds the {maximum}-bit instruction field")]
    /// A fixed shift amount exceeds its instruction field.
    ShiftAmountOutOfRange {
        /// The rejected shift amount.
        amount: u8,
        /// The greatest encodable amount.
        maximum: u8,
    },
    #[error("system exception code {code:#x} exceeds 20 bits")]
    /// A system-call or breakpoint code exceeds twenty bits.
    SystemCodeOutOfRange {
        /// The rejected exception code.
        code: u32,
    },
    #[error("trap code {code:#x} exceeds 10 bits")]
    /// A conditional-trap code exceeds ten bits.
    TrapCodeOutOfRange {
        /// The rejected trap code.
        code: u16,
    },
    #[error("invalid VR4300 instruction form: {reason}")]
    /// The operand combination is reserved or architecturally invalid.
    InvalidInstructionForm {
        /// A stable explanation of the rejected combination.
        reason: &'static str,
    },
    #[error("the VR4300 FPU does not define conversion from {from_format:?} to {to_format:?}")]
    /// The requested source and destination numeric formats have no defined conversion.
    InvalidFloatingConversion {
        /// The source numeric format.
        from_format: NumericFormat,
        /// The destination numeric format.
        to_format: NumericFormat,
    },
}

/// Decodes one host-order instruction word at its placement address.
pub fn decode_word(word: u32, address: u64) -> Result<Instruction, DecodeError> {
    require_aligned_decode(address)?;
    match opcode(word) {
        0x00 => decode_special(word),
        0x01 => decode_regimm(word, address),
        0x02 => Ok(Instruction::Jump {
            link: false,
            target: jump_target(word, address)?,
        }),
        0x03 => Ok(Instruction::Jump {
            link: true,
            target: jump_target(word, address)?,
        }),
        0x04 | 0x05 | 0x14 | 0x15 => {
            let primary = opcode(word);
            Ok(Instruction::EqualityBranch {
                condition: if primary & 1 == 0 {
                    EqualityCondition::Equal
                } else {
                    EqualityCondition::NotEqual
                },
                likely: primary & 0x10 != 0,
                left: rs(word),
                right: rt(word),
                target: branch_target(word, address)?,
            })
        }
        0x06 | 0x07 | 0x16 | 0x17 => {
            require(word, rt_index(word) == 0)?;
            let primary = opcode(word);
            Ok(Instruction::SignBranch {
                condition: if primary & 1 == 0 {
                    SignCondition::LessThanOrEqualZero
                } else {
                    SignCondition::GreaterThanZero
                },
                likely: primary & 0x10 != 0,
                link: false,
                source: rs(word),
                target: branch_target(word, address)?,
            })
        }
        0x08..=0x0b | 0x18 | 0x19 => Ok(Instruction::ImmediateArithmetic {
            operation: match opcode(word) {
                0x08 => ImmediateArithmeticOperation::Add,
                0x09 => ImmediateArithmeticOperation::AddUnsigned,
                0x0a => ImmediateArithmeticOperation::SetLessThan,
                0x0b => ImmediateArithmeticOperation::SetLessThanUnsigned,
                0x18 => ImmediateArithmeticOperation::DoublewordAdd,
                0x19 => ImmediateArithmeticOperation::DoublewordAddUnsigned,
                _ => unreachable!(),
            },
            destination: rt(word),
            source: rs(word),
            immediate: immediate(word),
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
        0x10 => decode_cp0(word, address),
        0x11 => decode_fpu(word, address),
        0x12 => decode_cp2(word, address),
        0x1a | 0x1b | 0x20..=0x2e | 0x30 | 0x34 | 0x37 | 0x38 | 0x3c | 0x3f => decode_memory(word),
        0x2f => Ok(Instruction::Cache {
            operation: CacheOperation::from_field(rt_index(word)),
            base: rs(word),
            offset: immediate(word),
        }),
        0x31 | 0x35 | 0x39 | 0x3d => decode_coprocessor_memory(word, true),
        0x32 | 0x36 | 0x3a | 0x3e => decode_coprocessor_memory(word, false),
        _ => reserved(word),
    }
}

/// Encodes one typed instruction as a host-order word at its placement address.
pub fn encode_word(instruction: &Instruction, address: u64) -> Result<u32, EncodeError> {
    require_aligned_encode(address)?;
    use Instruction::*;

    match *instruction {
        ThreeRegister {
            operation,
            destination,
            left,
            right,
        } => Ok(r_type(
            left,
            right,
            destination,
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
                ThreeRegisterOperation::DoublewordAdd => 0x2c,
                ThreeRegisterOperation::DoublewordAddUnsigned => 0x2d,
                ThreeRegisterOperation::DoublewordSubtract => 0x2e,
                ThreeRegisterOperation::DoublewordSubtractUnsigned => 0x2f,
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
                ImmediateArithmeticOperation::DoublewordAdd => 0x18,
                ImmediateArithmeticOperation::DoublewordAddUnsigned => 0x19,
            },
            source,
            destination.index(),
            immediate as u16,
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
            source,
            destination.index(),
            immediate,
        )),
        LoadUpperImmediate {
            destination,
            immediate,
        } => Ok(i_type(
            0x0f,
            GeneralRegister::ZERO,
            destination.index(),
            immediate,
        )),
        ShiftImmediate {
            operation,
            destination,
            source,
            amount,
        } => {
            let (function, encoded_amount) = shift_immediate_function(operation, amount)?;
            Ok(r_type(
                GeneralRegister::ZERO,
                source,
                destination,
                encoded_amount,
                function,
            ))
        }
        ShiftVariable {
            operation,
            destination,
            source,
            amount_register,
        } => Ok(r_type(
            amount_register,
            source,
            destination,
            0,
            match operation {
                ShiftOperation::LeftLogical => 0x04,
                ShiftOperation::RightLogical => 0x06,
                ShiftOperation::RightArithmetic => 0x07,
                ShiftOperation::DoublewordLeftLogical => 0x14,
                ShiftOperation::DoublewordRightLogical => 0x16,
                ShiftOperation::DoublewordRightArithmetic => 0x17,
            },
        )),
        MultiplyDivide {
            operation,
            left,
            right,
        } => Ok(r_type(
            left,
            right,
            GeneralRegister::ZERO,
            0,
            match operation {
                MultiplyDivideOperation::Multiply => 0x18,
                MultiplyDivideOperation::MultiplyUnsigned => 0x19,
                MultiplyDivideOperation::Divide => 0x1a,
                MultiplyDivideOperation::DivideUnsigned => 0x1b,
                MultiplyDivideOperation::DoublewordMultiply => 0x1c,
                MultiplyDivideOperation::DoublewordMultiplyUnsigned => 0x1d,
                MultiplyDivideOperation::DoublewordDivide => 0x1e,
                MultiplyDivideOperation::DoublewordDivideUnsigned => 0x1f,
            },
        )),
        MoveFromSpecial {
            source,
            destination,
        } => Ok(r_type(
            GeneralRegister::ZERO,
            GeneralRegister::ZERO,
            destination,
            0,
            match source {
                SpecialRegister::Hi => 0x10,
                SpecialRegister::Lo => 0x12,
            },
        )),
        MoveToSpecial {
            source,
            destination,
        } => Ok(r_type(
            source,
            GeneralRegister::ZERO,
            GeneralRegister::ZERO,
            0,
            match destination {
                SpecialRegister::Hi => 0x11,
                SpecialRegister::Lo => 0x13,
            },
        )),
        Memory {
            operation,
            register,
            base,
            offset,
        } => Ok(i_type(
            memory_opcode(operation),
            base,
            register.index(),
            offset as u16,
        )),
        EqualityBranch {
            condition,
            likely,
            left,
            right,
            target,
        } => branch_word(
            match (condition, likely) {
                (EqualityCondition::Equal, false) => 0x04,
                (EqualityCondition::NotEqual, false) => 0x05,
                (EqualityCondition::Equal, true) => 0x14,
                (EqualityCondition::NotEqual, true) => 0x15,
            },
            left,
            right.index(),
            address,
            target,
        ),
        SignBranch {
            condition,
            likely,
            link,
            source,
            target,
        } => match condition {
            SignCondition::LessThanZero | SignCondition::GreaterThanOrEqualZero => {
                let rt = match (condition, likely, link) {
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
                branch_word(0x01, source, rt, address, target)
            }
            SignCondition::LessThanOrEqualZero | SignCondition::GreaterThanZero => {
                if link {
                    return Err(EncodeError::InvalidInstructionForm {
                        reason: "BLEZ and BGTZ forms cannot link",
                    });
                }
                let primary = match (condition, likely) {
                    (SignCondition::LessThanOrEqualZero, false) => 0x06,
                    (SignCondition::GreaterThanZero, false) => 0x07,
                    (SignCondition::LessThanOrEqualZero, true) => 0x16,
                    (SignCondition::GreaterThanZero, true) => 0x17,
                    _ => unreachable!(),
                };
                branch_word(primary, source, 0, address, target)
            }
        },
        Jump { link, target } => jump_word(if link { 0x03 } else { 0x02 }, address, target),
        JumpRegister { link, target } => Ok(match link {
            None => r_type(
                target,
                GeneralRegister::ZERO,
                GeneralRegister::ZERO,
                0,
                0x08,
            ),
            Some(destination) => r_type(target, GeneralRegister::ZERO, destination, 0, 0x09),
        }),
        SystemCall { code } => system_word(code, 0x0c),
        Breakpoint { code } => system_word(code, 0x0d),
        Synchronize => Ok(0x0000_000f),
        TrapRegister {
            condition,
            left,
            right,
            code,
        } => {
            if code >= 1 << 10 {
                return Err(EncodeError::TrapCodeOutOfRange { code });
            }
            Ok(r_type(
                left,
                right,
                GeneralRegister::from_field((code >> 5) as u8),
                (code & 0x1f) as u8,
                trap_function(condition),
            ))
        }
        TrapImmediate {
            condition,
            source,
            immediate,
        } => Ok(i_type(
            0x01,
            source,
            trap_immediate_selector(condition),
            immediate as u16,
        )),
        MoveFromCp0 {
            doubleword,
            destination,
            source,
        } => Ok(cop_transfer(
            0x10,
            if doubleword { 1 } else { 0 },
            destination,
            source.index(),
        )),
        MoveToCp0 {
            doubleword,
            source,
            destination,
        } => Ok(cop_transfer(
            0x10,
            if doubleword { 5 } else { 4 },
            source,
            destination.index(),
        )),
        MoveFromFpu {
            doubleword,
            destination,
            source,
        } => Ok(cop_transfer(
            0x11,
            if doubleword { 1 } else { 0 },
            destination,
            source.index(),
        )),
        MoveToFpu {
            doubleword,
            source,
            destination,
        } => Ok(cop_transfer(
            0x11,
            if doubleword { 5 } else { 4 },
            source,
            destination.index(),
        )),
        MoveControlFromFpu {
            destination,
            source,
        } => Ok(cop_transfer(0x11, 2, destination, source.index())),
        MoveControlToFpu {
            source,
            destination,
        } => Ok(cop_transfer(0x11, 6, source, destination.index())),
        MoveFromCoprocessor2 {
            doubleword,
            destination,
            source,
        } => Ok(cop_transfer(
            0x12,
            if doubleword { 1 } else { 0 },
            destination,
            source.index(),
        )),
        MoveToCoprocessor2 {
            doubleword,
            source,
            destination,
        } => Ok(cop_transfer(
            0x12,
            if doubleword { 5 } else { 4 },
            source,
            destination.index(),
        )),
        MoveControlFromCoprocessor2 {
            destination,
            source,
        } => Ok(cop_transfer(0x12, 2, destination, source.index())),
        MoveControlToCoprocessor2 {
            source,
            destination,
        } => Ok(cop_transfer(0x12, 6, source, destination.index())),
        CoprocessorBranch {
            coprocessor,
            condition,
            likely,
            target,
        } => {
            let primary = coprocessor_opcode(coprocessor);
            let selector = match (condition, likely) {
                (CoprocessorBranchCondition::False, false) => 0,
                (CoprocessorBranchCondition::True, false) => 1,
                (CoprocessorBranchCondition::False, true) => 2,
                (CoprocessorBranchCondition::True, true) => 3,
            };
            branch_word(
                primary,
                GeneralRegister::from_field(8),
                selector,
                address,
                target,
            )
        }
        CoprocessorMemory {
            operation,
            register,
            base,
            offset,
        } => {
            let primary = coprocessor_memory_opcode(register.coprocessor(), operation)?;
            Ok(i_type(primary, base, register.index(), offset as u16))
        }
        Cp0(operation) => Ok(match operation {
            Cp0Operation::ReadIndexedTlbEntry => 0x4200_0001,
            Cp0Operation::WriteIndexedTlbEntry => 0x4200_0002,
            Cp0Operation::WriteRandomTlbEntry => 0x4200_0006,
            Cp0Operation::ProbeTlb => 0x4200_0008,
            Cp0Operation::ReturnFromException => 0x4200_0018,
        }),
        Cache {
            operation,
            base,
            offset,
        } => Ok(i_type(0x2f, base, operation.value(), offset as u16)),
        FloatingBinary {
            operation,
            format,
            destination,
            left,
            right,
        } => Ok(fpu_word(
            numeric_format_code(NumericFormat::Floating(format)),
            right,
            left,
            destination,
            match operation {
                FloatingBinaryOperation::Add => 0,
                FloatingBinaryOperation::Subtract => 1,
                FloatingBinaryOperation::Multiply => 2,
                FloatingBinaryOperation::Divide => 3,
            },
        )),
        FloatingUnary {
            operation,
            format,
            destination,
            source,
        } => Ok(fpu_word(
            numeric_format_code(NumericFormat::Floating(format)),
            FloatingRegister::from_field(0),
            source,
            destination,
            match operation {
                FloatingUnaryOperation::SquareRoot => 4,
                FloatingUnaryOperation::AbsoluteValue => 5,
                FloatingUnaryOperation::Move => 6,
                FloatingUnaryOperation::Negate => 7,
            },
        )),
        FloatingRoundToInteger {
            rounding,
            destination_format,
            source_format,
            destination,
            source,
        } => {
            let base = match destination_format {
                FixedFormat::Longword => 8,
                FixedFormat::Word => 12,
            };
            Ok(fpu_word(
                numeric_format_code(NumericFormat::Floating(source_format)),
                FloatingRegister::from_field(0),
                source,
                destination,
                base + rounding_code(rounding),
            ))
        }
        FloatingConvert {
            destination_format,
            source_format,
            destination,
            source,
        } => {
            validate_floating_conversion(source_format, destination_format)?;
            let function = match destination_format {
                NumericFormat::Floating(FloatingFormat::Single) => 0x20,
                NumericFormat::Floating(FloatingFormat::Double) => 0x21,
                NumericFormat::Fixed(FixedFormat::Word) => 0x24,
                NumericFormat::Fixed(FixedFormat::Longword) => 0x25,
            };
            Ok(fpu_word(
                numeric_format_code(source_format),
                FloatingRegister::from_field(0),
                source,
                destination,
                function,
            ))
        }
        FloatingCompare {
            predicate,
            format,
            left,
            right,
        } => Ok(fpu_word(
            numeric_format_code(NumericFormat::Floating(format)),
            right,
            left,
            FloatingRegister::from_field(0),
            0x30 + predicate as u8,
        )),
    }
}

/// Decodes one four-byte big-endian instruction at its placement address.
pub fn decode_bytes(source: &[u8], address: u64) -> Result<Instruction, DecodeError> {
    let bytes = source
        .get(..4)
        .ok_or(DecodeError::UnexpectedEnd { got: source.len() })?;
    decode_word(
        u32::from_be_bytes(bytes.try_into().expect("four-byte slice")),
        address,
    )
}

/// Encodes one typed instruction as four big-endian bytes at its placement address.
pub fn encode_bytes(instruction: &Instruction, address: u64) -> Result<[u8; 4], EncodeError> {
    Ok(encode_word(instruction, address)?.to_be_bytes())
}

fn decode_special(word: u32) -> Result<Instruction, DecodeError> {
    let function = function(word);
    match function {
        0x00 | 0x02 | 0x03 | 0x38 | 0x3a | 0x3b | 0x3c | 0x3e | 0x3f => {
            require(word, rs_index(word) == 0)?;
            let (operation, high_half) = match function {
                0x00 => (ShiftOperation::LeftLogical, false),
                0x02 => (ShiftOperation::RightLogical, false),
                0x03 => (ShiftOperation::RightArithmetic, false),
                0x38 => (ShiftOperation::DoublewordLeftLogical, false),
                0x3a => (ShiftOperation::DoublewordRightLogical, false),
                0x3b => (ShiftOperation::DoublewordRightArithmetic, false),
                0x3c => (ShiftOperation::DoublewordLeftLogical, true),
                0x3e => (ShiftOperation::DoublewordRightLogical, true),
                0x3f => (ShiftOperation::DoublewordRightArithmetic, true),
                _ => unreachable!(),
            };
            Ok(Instruction::ShiftImmediate {
                operation,
                destination: rd(word),
                source: rt(word),
                amount: shift_amount(word) + if high_half { 32 } else { 0 },
            })
        }
        0x04 | 0x06 | 0x07 | 0x14 | 0x16 | 0x17 => {
            require(word, shift_amount(word) == 0)?;
            Ok(Instruction::ShiftVariable {
                operation: match function {
                    0x04 => ShiftOperation::LeftLogical,
                    0x06 => ShiftOperation::RightLogical,
                    0x07 => ShiftOperation::RightArithmetic,
                    0x14 => ShiftOperation::DoublewordLeftLogical,
                    0x16 => ShiftOperation::DoublewordRightLogical,
                    0x17 => ShiftOperation::DoublewordRightArithmetic,
                    _ => unreachable!(),
                },
                destination: rd(word),
                source: rt(word),
                amount_register: rs(word),
            })
        }
        0x08 => {
            require(
                word,
                rt_index(word) == 0 && rd_index(word) == 0 && shift_amount(word) == 0,
            )?;
            Ok(Instruction::JumpRegister {
                link: None,
                target: rs(word),
            })
        }
        0x09 => {
            require(word, rt_index(word) == 0 && shift_amount(word) == 0)?;
            Ok(Instruction::JumpRegister {
                link: Some(rd(word)),
                target: rs(word),
            })
        }
        0x0c | 0x0d => Ok(if function == 0x0c {
            Instruction::SystemCall {
                code: (word >> 6) & 0x000f_ffff,
            }
        } else {
            Instruction::Breakpoint {
                code: (word >> 6) & 0x000f_ffff,
            }
        }),
        0x0f => {
            require(word, word & 0x03ff_ffc0 == 0)?;
            Ok(Instruction::Synchronize)
        }
        0x10 | 0x12 => {
            require(
                word,
                rs_index(word) == 0 && rt_index(word) == 0 && shift_amount(word) == 0,
            )?;
            Ok(Instruction::MoveFromSpecial {
                source: if function == 0x10 {
                    SpecialRegister::Hi
                } else {
                    SpecialRegister::Lo
                },
                destination: rd(word),
            })
        }
        0x11 | 0x13 => {
            require(
                word,
                rt_index(word) == 0 && rd_index(word) == 0 && shift_amount(word) == 0,
            )?;
            Ok(Instruction::MoveToSpecial {
                source: rs(word),
                destination: if function == 0x11 {
                    SpecialRegister::Hi
                } else {
                    SpecialRegister::Lo
                },
            })
        }
        0x18..=0x1f => {
            require(word, rd_index(word) == 0 && shift_amount(word) == 0)?;
            Ok(Instruction::MultiplyDivide {
                operation: match function {
                    0x18 => MultiplyDivideOperation::Multiply,
                    0x19 => MultiplyDivideOperation::MultiplyUnsigned,
                    0x1a => MultiplyDivideOperation::Divide,
                    0x1b => MultiplyDivideOperation::DivideUnsigned,
                    0x1c => MultiplyDivideOperation::DoublewordMultiply,
                    0x1d => MultiplyDivideOperation::DoublewordMultiplyUnsigned,
                    0x1e => MultiplyDivideOperation::DoublewordDivide,
                    0x1f => MultiplyDivideOperation::DoublewordDivideUnsigned,
                    _ => unreachable!(),
                },
                left: rs(word),
                right: rt(word),
            })
        }
        0x20..=0x27 | 0x2a..=0x2f => {
            require(word, shift_amount(word) == 0)?;
            Ok(Instruction::ThreeRegister {
                operation: match function {
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
                    0x2c => ThreeRegisterOperation::DoublewordAdd,
                    0x2d => ThreeRegisterOperation::DoublewordAddUnsigned,
                    0x2e => ThreeRegisterOperation::DoublewordSubtract,
                    0x2f => ThreeRegisterOperation::DoublewordSubtractUnsigned,
                    _ => unreachable!(),
                },
                destination: rd(word),
                left: rs(word),
                right: rt(word),
            })
        }
        0x30..=0x34 | 0x36 => Ok(Instruction::TrapRegister {
            condition: match function {
                0x30 => TrapCondition::GreaterThanOrEqual,
                0x31 => TrapCondition::GreaterThanOrEqualUnsigned,
                0x32 => TrapCondition::LessThan,
                0x33 => TrapCondition::LessThanUnsigned,
                0x34 => TrapCondition::Equal,
                0x36 => TrapCondition::NotEqual,
                _ => unreachable!(),
            },
            left: rs(word),
            right: rt(word),
            code: ((word >> 6) & 0x03ff) as u16,
        }),
        _ => reserved(word),
    }
}

fn decode_regimm(word: u32, address: u64) -> Result<Instruction, DecodeError> {
    match rt_index(word) {
        0x00..=0x03 | 0x10..=0x13 => Ok(Instruction::SignBranch {
            condition: if rt_index(word) & 1 == 0 {
                SignCondition::LessThanZero
            } else {
                SignCondition::GreaterThanOrEqualZero
            },
            likely: rt_index(word) & 0x02 != 0,
            link: rt_index(word) & 0x10 != 0,
            source: rs(word),
            target: branch_target(word, address)?,
        }),
        0x08..=0x0c | 0x0e => Ok(Instruction::TrapImmediate {
            condition: match rt_index(word) {
                0x08 => TrapCondition::GreaterThanOrEqual,
                0x09 => TrapCondition::GreaterThanOrEqualUnsigned,
                0x0a => TrapCondition::LessThan,
                0x0b => TrapCondition::LessThanUnsigned,
                0x0c => TrapCondition::Equal,
                0x0e => TrapCondition::NotEqual,
                _ => unreachable!(),
            },
            source: rs(word),
            immediate: immediate(word),
        }),
        _ => reserved(word),
    }
}

fn decode_cp0(word: u32, address: u64) -> Result<Instruction, DecodeError> {
    match rs_index(word) {
        0 | 1 | 4 | 5 => {
            require(word, word & 0x7ff == 0)?;
            let register = Cp0Register::from_field(rd_index(word))
                .ok_or(DecodeError::ReservedWord { word })?;
            Ok(match rs_index(word) {
                0 | 1 => Instruction::MoveFromCp0 {
                    doubleword: rs_index(word) == 1,
                    destination: rt(word),
                    source: register,
                },
                4 | 5 => Instruction::MoveToCp0 {
                    doubleword: rs_index(word) == 5,
                    source: rt(word),
                    destination: register,
                },
                _ => unreachable!(),
            })
        }
        8 => decode_coprocessor_branch(word, address, Coprocessor::SystemControl),
        16..=31 => {
            let operation = match word & 0x03ff_ffff {
                0x0200_0001 => Cp0Operation::ReadIndexedTlbEntry,
                0x0200_0002 => Cp0Operation::WriteIndexedTlbEntry,
                0x0200_0006 => Cp0Operation::WriteRandomTlbEntry,
                0x0200_0008 => Cp0Operation::ProbeTlb,
                0x0200_0018 => Cp0Operation::ReturnFromException,
                _ => return reserved(word),
            };
            Ok(Instruction::Cp0(operation))
        }
        _ => reserved(word),
    }
}

fn decode_fpu(word: u32, address: u64) -> Result<Instruction, DecodeError> {
    match rs_index(word) {
        0 | 1 | 4 | 5 => {
            require(word, word & 0x7ff == 0)?;
            let register = FloatingRegister::from_field(rd_index(word));
            Ok(match rs_index(word) {
                0 | 1 => Instruction::MoveFromFpu {
                    doubleword: rs_index(word) == 1,
                    destination: rt(word),
                    source: register,
                },
                4 | 5 => Instruction::MoveToFpu {
                    doubleword: rs_index(word) == 5,
                    source: rt(word),
                    destination: register,
                },
                _ => unreachable!(),
            })
        }
        2 | 6 => {
            require(word, word & 0x7ff == 0)?;
            let control = FpuControlRegister::from_field(rd_index(word))
                .ok_or(DecodeError::ReservedWord { word })?;
            Ok(if rs_index(word) == 2 {
                Instruction::MoveControlFromFpu {
                    destination: rt(word),
                    source: control,
                }
            } else {
                Instruction::MoveControlToFpu {
                    source: rt(word),
                    destination: control,
                }
            })
        }
        8 => decode_coprocessor_branch(word, address, Coprocessor::FloatingPoint),
        16 | 17 | 20 | 21 => decode_fpu_computation(word),
        _ => reserved(word),
    }
}

fn decode_cp2(word: u32, address: u64) -> Result<Instruction, DecodeError> {
    match rs_index(word) {
        0 | 1 | 2 | 4 | 5 | 6 => {
            require(word, word & 0x7ff == 0)?;
            Ok(match rs_index(word) {
                0 | 1 => Instruction::MoveFromCoprocessor2 {
                    doubleword: rs_index(word) == 1,
                    destination: rt(word),
                    source: Coprocessor2Register::from_field(rd_index(word)),
                },
                2 => Instruction::MoveControlFromCoprocessor2 {
                    destination: rt(word),
                    source: Coprocessor2ControlRegister::from_field(rd_index(word)),
                },
                4 | 5 => Instruction::MoveToCoprocessor2 {
                    doubleword: rs_index(word) == 5,
                    source: rt(word),
                    destination: Coprocessor2Register::from_field(rd_index(word)),
                },
                6 => Instruction::MoveControlToCoprocessor2 {
                    source: rt(word),
                    destination: Coprocessor2ControlRegister::from_field(rd_index(word)),
                },
                _ => unreachable!(),
            })
        }
        8 => decode_coprocessor_branch(word, address, Coprocessor::Coprocessor2),
        _ => reserved(word),
    }
}

fn decode_coprocessor_branch(
    word: u32,
    address: u64,
    coprocessor: Coprocessor,
) -> Result<Instruction, DecodeError> {
    require(word, rt_index(word) < 4)?;
    Ok(Instruction::CoprocessorBranch {
        coprocessor,
        condition: if rt_index(word) & 1 == 0 {
            CoprocessorBranchCondition::False
        } else {
            CoprocessorBranchCondition::True
        },
        likely: rt_index(word) & 2 != 0,
        target: branch_target(word, address)?,
    })
}

fn decode_fpu_computation(word: u32) -> Result<Instruction, DecodeError> {
    let source_format = match rs_index(word) {
        16 => NumericFormat::Floating(FloatingFormat::Single),
        17 => NumericFormat::Floating(FloatingFormat::Double),
        20 => NumericFormat::Fixed(FixedFormat::Word),
        21 => NumericFormat::Fixed(FixedFormat::Longword),
        _ => unreachable!(),
    };
    let function = function(word);
    match function {
        0..=3 => {
            let format = floating_only(word, source_format)?;
            Ok(Instruction::FloatingBinary {
                operation: match function {
                    0 => FloatingBinaryOperation::Add,
                    1 => FloatingBinaryOperation::Subtract,
                    2 => FloatingBinaryOperation::Multiply,
                    3 => FloatingBinaryOperation::Divide,
                    _ => unreachable!(),
                },
                format,
                destination: floating_rd(word),
                left: floating_rs(word),
                right: floating_rt(word),
            })
        }
        4..=7 => {
            let format = floating_only(word, source_format)?;
            require(word, rt_index(word) == 0)?;
            Ok(Instruction::FloatingUnary {
                operation: match function {
                    4 => FloatingUnaryOperation::SquareRoot,
                    5 => FloatingUnaryOperation::AbsoluteValue,
                    6 => FloatingUnaryOperation::Move,
                    7 => FloatingUnaryOperation::Negate,
                    _ => unreachable!(),
                },
                format,
                destination: floating_rd(word),
                source: floating_rs(word),
            })
        }
        8..=15 => {
            let format = floating_only(word, source_format)?;
            require(word, rt_index(word) == 0)?;
            Ok(Instruction::FloatingRoundToInteger {
                rounding: match function & 3 {
                    0 => FloatingRounding::NearestEven,
                    1 => FloatingRounding::TowardZero,
                    2 => FloatingRounding::TowardPositiveInfinity,
                    3 => FloatingRounding::TowardNegativeInfinity,
                    _ => unreachable!(),
                },
                destination_format: if function < 12 {
                    FixedFormat::Longword
                } else {
                    FixedFormat::Word
                },
                source_format: format,
                destination: floating_rd(word),
                source: floating_rs(word),
            })
        }
        0x20 | 0x21 | 0x24 | 0x25 => {
            require(word, rt_index(word) == 0)?;
            let destination_format = match function {
                0x20 => NumericFormat::Floating(FloatingFormat::Single),
                0x21 => NumericFormat::Floating(FloatingFormat::Double),
                0x24 => NumericFormat::Fixed(FixedFormat::Word),
                0x25 => NumericFormat::Fixed(FixedFormat::Longword),
                _ => unreachable!(),
            };
            if !floating_conversion_is_valid(source_format, destination_format) {
                return reserved(word);
            }
            Ok(Instruction::FloatingConvert {
                destination_format,
                source_format,
                destination: floating_rd(word),
                source: floating_rs(word),
            })
        }
        0x30..=0x3f => {
            let format = floating_only(word, source_format)?;
            require(word, rd_index(word) == 0)?;
            Ok(Instruction::FloatingCompare {
                predicate: FloatingComparePredicate::from_field(function - 0x30),
                format,
                left: floating_rs(word),
                right: floating_rt(word),
            })
        }
        _ => reserved(word),
    }
}

fn decode_memory(word: u32) -> Result<Instruction, DecodeError> {
    let operation = match opcode(word) {
        0x1a => MemoryOperation::LoadDoublewordLeft,
        0x1b => MemoryOperation::LoadDoublewordRight,
        0x20 => MemoryOperation::LoadByte,
        0x21 => MemoryOperation::LoadHalfword,
        0x22 => MemoryOperation::LoadWordLeft,
        0x23 => MemoryOperation::LoadWord,
        0x24 => MemoryOperation::LoadByteUnsigned,
        0x25 => MemoryOperation::LoadHalfwordUnsigned,
        0x26 => MemoryOperation::LoadWordRight,
        0x27 => MemoryOperation::LoadWordUnsigned,
        0x28 => MemoryOperation::StoreByte,
        0x29 => MemoryOperation::StoreHalfword,
        0x2a => MemoryOperation::StoreWordLeft,
        0x2b => MemoryOperation::StoreWord,
        0x2c => MemoryOperation::StoreDoublewordLeft,
        0x2d => MemoryOperation::StoreDoublewordRight,
        0x2e => MemoryOperation::StoreWordRight,
        0x30 => MemoryOperation::LoadLinked,
        0x34 => MemoryOperation::LoadLinkedDoubleword,
        0x37 => MemoryOperation::LoadDoubleword,
        0x38 => MemoryOperation::StoreConditional,
        0x3c => MemoryOperation::StoreConditionalDoubleword,
        0x3f => MemoryOperation::StoreDoubleword,
        _ => unreachable!(),
    };
    Ok(Instruction::Memory {
        operation,
        register: rt(word),
        base: rs(word),
        offset: immediate(word),
    })
}

fn decode_coprocessor_memory(word: u32, floating_point: bool) -> Result<Instruction, DecodeError> {
    let operation = match opcode(word) {
        0x31 | 0x32 => CoprocessorMemoryOperation::LoadWord,
        0x35 | 0x36 => CoprocessorMemoryOperation::LoadDoubleword,
        0x39 | 0x3a => CoprocessorMemoryOperation::StoreWord,
        0x3d | 0x3e => CoprocessorMemoryOperation::StoreDoubleword,
        _ => unreachable!(),
    };
    let register = if floating_point {
        CoprocessorMemoryRegister::FloatingPoint(FloatingRegister::from_field(rt_index(word)))
    } else {
        CoprocessorMemoryRegister::Coprocessor2(Coprocessor2Register::from_field(rt_index(word)))
    };
    Ok(Instruction::CoprocessorMemory {
        operation,
        register,
        base: rs(word),
        offset: immediate(word),
    })
}

fn memory_opcode(operation: MemoryOperation) -> u8 {
    match operation {
        MemoryOperation::LoadDoublewordLeft => 0x1a,
        MemoryOperation::LoadDoublewordRight => 0x1b,
        MemoryOperation::LoadByte => 0x20,
        MemoryOperation::LoadHalfword => 0x21,
        MemoryOperation::LoadWordLeft => 0x22,
        MemoryOperation::LoadWord => 0x23,
        MemoryOperation::LoadByteUnsigned => 0x24,
        MemoryOperation::LoadHalfwordUnsigned => 0x25,
        MemoryOperation::LoadWordRight => 0x26,
        MemoryOperation::LoadWordUnsigned => 0x27,
        MemoryOperation::StoreByte => 0x28,
        MemoryOperation::StoreHalfword => 0x29,
        MemoryOperation::StoreWordLeft => 0x2a,
        MemoryOperation::StoreWord => 0x2b,
        MemoryOperation::StoreDoublewordLeft => 0x2c,
        MemoryOperation::StoreDoublewordRight => 0x2d,
        MemoryOperation::StoreWordRight => 0x2e,
        MemoryOperation::LoadLinked => 0x30,
        MemoryOperation::LoadLinkedDoubleword => 0x34,
        MemoryOperation::LoadDoubleword => 0x37,
        MemoryOperation::StoreConditional => 0x38,
        MemoryOperation::StoreConditionalDoubleword => 0x3c,
        MemoryOperation::StoreDoubleword => 0x3f,
    }
}

fn shift_immediate_function(
    operation: ShiftOperation,
    amount: u8,
) -> Result<(u8, u8), EncodeError> {
    match operation {
        ShiftOperation::LeftLogical
        | ShiftOperation::RightLogical
        | ShiftOperation::RightArithmetic => {
            if amount >= 32 {
                return Err(EncodeError::ShiftAmountOutOfRange { amount, maximum: 5 });
            }
            Ok((
                match operation {
                    ShiftOperation::LeftLogical => 0x00,
                    ShiftOperation::RightLogical => 0x02,
                    ShiftOperation::RightArithmetic => 0x03,
                    _ => unreachable!(),
                },
                amount,
            ))
        }
        ShiftOperation::DoublewordLeftLogical
        | ShiftOperation::DoublewordRightLogical
        | ShiftOperation::DoublewordRightArithmetic => {
            if amount >= 64 {
                return Err(EncodeError::ShiftAmountOutOfRange { amount, maximum: 6 });
            }
            let high = amount >= 32;
            Ok((
                match (operation, high) {
                    (ShiftOperation::DoublewordLeftLogical, false) => 0x38,
                    (ShiftOperation::DoublewordRightLogical, false) => 0x3a,
                    (ShiftOperation::DoublewordRightArithmetic, false) => 0x3b,
                    (ShiftOperation::DoublewordLeftLogical, true) => 0x3c,
                    (ShiftOperation::DoublewordRightLogical, true) => 0x3e,
                    (ShiftOperation::DoublewordRightArithmetic, true) => 0x3f,
                    _ => unreachable!(),
                },
                amount & 31,
            ))
        }
    }
}

fn trap_function(condition: TrapCondition) -> u8 {
    match condition {
        TrapCondition::GreaterThanOrEqual => 0x30,
        TrapCondition::GreaterThanOrEqualUnsigned => 0x31,
        TrapCondition::LessThan => 0x32,
        TrapCondition::LessThanUnsigned => 0x33,
        TrapCondition::Equal => 0x34,
        TrapCondition::NotEqual => 0x36,
    }
}

fn trap_immediate_selector(condition: TrapCondition) -> u8 {
    match condition {
        TrapCondition::GreaterThanOrEqual => 0x08,
        TrapCondition::GreaterThanOrEqualUnsigned => 0x09,
        TrapCondition::LessThan => 0x0a,
        TrapCondition::LessThanUnsigned => 0x0b,
        TrapCondition::Equal => 0x0c,
        TrapCondition::NotEqual => 0x0e,
    }
}

fn coprocessor_opcode(coprocessor: Coprocessor) -> u8 {
    match coprocessor {
        Coprocessor::SystemControl => 0x10,
        Coprocessor::FloatingPoint => 0x11,
        Coprocessor::Coprocessor2 => 0x12,
    }
}

fn coprocessor_memory_opcode(
    coprocessor: Coprocessor,
    operation: CoprocessorMemoryOperation,
) -> Result<u8, EncodeError> {
    let low = match coprocessor {
        Coprocessor::FloatingPoint => 1,
        Coprocessor::Coprocessor2 => 2,
        Coprocessor::SystemControl => {
            return Err(EncodeError::InvalidInstructionForm {
                reason: "the VR4300 defines no CP0 load/store primary opcode",
            });
        }
    };
    Ok(match operation {
        CoprocessorMemoryOperation::LoadWord => 0x30 + low,
        CoprocessorMemoryOperation::LoadDoubleword => 0x34 + low,
        CoprocessorMemoryOperation::StoreWord => 0x38 + low,
        CoprocessorMemoryOperation::StoreDoubleword => 0x3c + low,
    })
}

fn floating_only(word: u32, format: NumericFormat) -> Result<FloatingFormat, DecodeError> {
    match format {
        NumericFormat::Floating(format) => Ok(format),
        NumericFormat::Fixed(_) => reserved(word),
    }
}

fn floating_conversion_is_valid(source: NumericFormat, destination: NumericFormat) -> bool {
    matches!(
        (source, destination),
        (
            NumericFormat::Floating(FloatingFormat::Double),
            NumericFormat::Floating(FloatingFormat::Single)
        ) | (
            NumericFormat::Fixed(_),
            NumericFormat::Floating(FloatingFormat::Single)
        ) | (
            NumericFormat::Floating(FloatingFormat::Single),
            NumericFormat::Floating(FloatingFormat::Double)
        ) | (
            NumericFormat::Fixed(_),
            NumericFormat::Floating(FloatingFormat::Double)
        ) | (NumericFormat::Floating(_), NumericFormat::Fixed(_))
    )
}

fn validate_floating_conversion(
    source: NumericFormat,
    destination: NumericFormat,
) -> Result<(), EncodeError> {
    if floating_conversion_is_valid(source, destination) {
        Ok(())
    } else {
        Err(EncodeError::InvalidFloatingConversion {
            from_format: source,
            to_format: destination,
        })
    }
}

fn numeric_format_code(format: NumericFormat) -> u8 {
    match format {
        NumericFormat::Floating(FloatingFormat::Single) => 16,
        NumericFormat::Floating(FloatingFormat::Double) => 17,
        NumericFormat::Fixed(FixedFormat::Word) => 20,
        NumericFormat::Fixed(FixedFormat::Longword) => 21,
    }
}

fn rounding_code(rounding: FloatingRounding) -> u8 {
    match rounding {
        FloatingRounding::NearestEven => 0,
        FloatingRounding::TowardZero => 1,
        FloatingRounding::TowardPositiveInfinity => 2,
        FloatingRounding::TowardNegativeInfinity => 3,
    }
}

fn branch_target(word: u32, address: u64) -> Result<u64, DecodeError> {
    let base = address
        .checked_add(4)
        .ok_or(DecodeError::AddressOverflow { address })?;
    let displacement = i64::from(immediate(word)) * 4;
    if displacement >= 0 {
        base.checked_add(displacement as u64)
    } else {
        base.checked_sub(displacement.unsigned_abs())
    }
    .ok_or(DecodeError::AddressOverflow { address })
}

fn jump_target(word: u32, address: u64) -> Result<u64, DecodeError> {
    let base = address
        .checked_add(4)
        .ok_or(DecodeError::AddressOverflow { address })?;
    Ok((base & !0x0fff_ffff) | (u64::from(word & 0x03ff_ffff) << 2))
}

fn branch_word(
    opcode: u8,
    rs: GeneralRegister,
    rt: u8,
    address: u64,
    target: u64,
) -> Result<u32, EncodeError> {
    if target & 3 != 0 {
        return Err(EncodeError::UnalignedTarget { target });
    }
    let base = address
        .checked_add(4)
        .ok_or(EncodeError::AddressOverflow { address })?;
    let difference = i128::from(target) - i128::from(base);
    if difference % 4 != 0 {
        return Err(EncodeError::UnalignedTarget { target });
    }
    let words = difference / 4;
    if !(i128::from(i16::MIN)..=i128::from(i16::MAX)).contains(&words) {
        return Err(EncodeError::BranchOutOfRange { address, target });
    }
    Ok(i_type(opcode, rs, rt, words as i16 as u16))
}

fn jump_word(opcode: u8, address: u64, target: u64) -> Result<u32, EncodeError> {
    if target & 3 != 0 {
        return Err(EncodeError::UnalignedTarget { target });
    }
    let base = address
        .checked_add(4)
        .ok_or(EncodeError::AddressOverflow { address })?;
    if base & !0x0fff_ffff != target & !0x0fff_ffff {
        return Err(EncodeError::JumpRegionMismatch { address, target });
    }
    Ok(u32::from(opcode) << 26 | ((target >> 2) as u32 & 0x03ff_ffff))
}

fn system_word(code: u32, function: u8) -> Result<u32, EncodeError> {
    if code >= 1 << 20 {
        return Err(EncodeError::SystemCodeOutOfRange { code });
    }
    Ok(code << 6 | u32::from(function))
}

fn i_type(opcode: u8, rs: GeneralRegister, rt: u8, immediate: u16) -> u32 {
    u32::from(opcode) << 26
        | u32::from(rs.index()) << 21
        | u32::from(rt) << 16
        | u32::from(immediate)
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

fn cop_transfer(opcode: u8, sub: u8, general: GeneralRegister, coprocessor_register: u8) -> u32 {
    u32::from(opcode) << 26
        | u32::from(sub) << 21
        | u32::from(general.index()) << 16
        | u32::from(coprocessor_register) << 11
}

fn fpu_word(
    format: u8,
    ft: FloatingRegister,
    fs: FloatingRegister,
    fd: FloatingRegister,
    function: u8,
) -> u32 {
    0x11 << 26
        | u32::from(format) << 21
        | u32::from(ft.index()) << 16
        | u32::from(fs.index()) << 11
        | u32::from(fd.index()) << 6
        | u32::from(function)
}

const fn opcode(word: u32) -> u8 {
    (word >> 26) as u8
}
const fn rs_index(word: u32) -> u8 {
    ((word >> 21) & 31) as u8
}
const fn rt_index(word: u32) -> u8 {
    ((word >> 16) & 31) as u8
}
const fn rd_index(word: u32) -> u8 {
    ((word >> 11) & 31) as u8
}
const fn shift_amount(word: u32) -> u8 {
    ((word >> 6) & 31) as u8
}
const fn function(word: u32) -> u8 {
    (word & 63) as u8
}
const fn immediate(word: u32) -> i16 {
    word as u16 as i16
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
const fn floating_rt(word: u32) -> FloatingRegister {
    FloatingRegister::from_field(rt_index(word))
}
const fn floating_rs(word: u32) -> FloatingRegister {
    FloatingRegister::from_field(rd_index(word))
}
const fn floating_rd(word: u32) -> FloatingRegister {
    FloatingRegister::from_field(shift_amount(word))
}

fn require(word: u32, condition: bool) -> Result<(), DecodeError> {
    if condition { Ok(()) } else { reserved(word) }
}

fn reserved<T>(word: u32) -> Result<T, DecodeError> {
    Err(DecodeError::ReservedWord { word })
}

fn require_aligned_decode(address: u64) -> Result<(), DecodeError> {
    if address & 3 == 0 {
        Ok(())
    } else {
        Err(DecodeError::UnalignedAddress { address })
    }
}

fn require_aligned_encode(address: u64) -> Result<(), EncodeError> {
    if address & 3 == 0 {
        Ok(())
    } else {
        Err(EncodeError::UnalignedAddress { address })
    }
}

impl TypedIsa for Vr4300 {
    const PROFILE_ID: &'static str = crate::PROFILE_ID;

    type Instruction = Instruction;
    type Form = crate::InstructionForm;
    type DecodeError = DecodeError;
    type EncodeError = EncodeError;

    fn form(instruction: &Self::Instruction) -> Result<Self::Form, Self::EncodeError> {
        Ok(instruction.form())
    }

    fn decode_one(
        source: &[u8],
        address: u64,
    ) -> Result<ProtocolDecodedInstruction<Self::Instruction>, Self::DecodeError> {
        let instruction = decode_bytes(source, address)?;
        let exact =
            InstructionBytes::new(source[..4].to_vec()).expect("VR4300 instructions are non-empty");
        Ok(ProtocolDecodedInstruction::new(instruction, exact))
    }

    fn encode_one(
        instruction: &Self::Instruction,
        address: u64,
    ) -> Result<InstructionBytes, Self::EncodeError> {
        Ok(
            InstructionBytes::new(encode_bytes(instruction, address)?.to_vec())
                .expect("VR4300 instructions are non-empty"),
        )
    }
}
