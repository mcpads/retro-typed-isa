use arm7tdmi::{
    AddressOffset, AddressingMode2, AddressingMode3, Condition, Coprocessor, CoprocessorOpcode,
    CoprocessorRegister, HalfwordOffset, ImmediateShift, IndexMode, Register, ShiftKind,
};
use thiserror::Error;
use typed_isa_core::{DecodedInstruction, InstructionBytes, TypedIsa};

use crate::{
    ArmInstruction, ArmInstructionForm, HalfwordSelection, SaturatingOperation, ThumbInstruction,
    ThumbInstructionForm,
};

/// A fail-closed reason that source bytes are not one valid ARM946E-S instruction.
#[derive(Debug, Error)]
pub enum DecodeError {
    /// The source ends before the complete ARM or Thumb instruction is available.
    #[error("ARM946E-S instruction needs {needed} bytes, but only {got} are available")]
    UnexpectedEnd {
        /// Minimum byte count required to continue decoding.
        needed: usize,
        /// Byte count available to the decoder.
        got: usize,
    },
    /// The supplied placement address violates the selected execution state's alignment.
    #[error("ARM946E-S code location {location:#x} is not aligned to {alignment} bytes")]
    UnalignedAddress {
        /// Rejected code location.
        location: crate::CodeLocation,
        /// Required alignment in bytes.
        alignment: u8,
    },
    /// The selector is reserved by the ARMv5TE architecture.
    #[error("reserved ARM946E-S {width}-bit instruction encoding {value:#010x}")]
    ReservedEncoding {
        /// Instruction width used to interpret `value`.
        width: u8,
        /// Rejected instruction bits in the least-significant part of the word.
        value: u32,
    },
    /// The bits select an architecturally unpredictable operand combination.
    #[error(
        "architecturally unpredictable ARM946E-S {width}-bit instruction encoding {value:#010x}"
    )]
    UnpredictableEncoding {
        /// Instruction width used to interpret `value`.
        width: u8,
        /// Rejected instruction bits in the least-significant part of the word.
        value: u32,
    },
    /// An inherited ARMv4T instruction failed its profile decoder.
    #[error(transparent)]
    Armv4T(#[from] arm7tdmi::DecodeError),
}

/// A fail-closed reason that a typed ARM946E-S instruction cannot be lowered.
#[derive(Debug, Error)]
pub enum EncodeError {
    /// A typed integer field exceeds the bit width assigned by the architecture.
    #[error("{field} value {value} is outside the encoded maximum {maximum}")]
    FieldOutOfRange {
        /// Stable name of the rejected encoding field.
        field: &'static str,
        /// Rejected field value.
        value: u32,
        /// Largest value representable by the field.
        maximum: u32,
    },
    /// A signed displacement or other field violates its encoded alignment.
    #[error("{field} value {value} does not have required alignment {alignment}")]
    UnalignedField {
        /// Stable name of the rejected encoding field.
        field: &'static str,
        /// Rejected signed field value.
        value: i32,
        /// Required alignment in bytes.
        alignment: u8,
    },
    /// The supplied placement address violates the selected execution state's alignment.
    #[error("ARM946E-S code location {location:#x} is not aligned to {alignment} bytes")]
    UnalignedAddress {
        /// Rejected code location.
        location: crate::CodeLocation,
        /// Required alignment in bytes.
        alignment: u8,
    },
    /// Typed operands form an architecturally undefined or unpredictable combination.
    #[error("operand combination is not defined by ARMv5TE: {reason}")]
    InvalidOperand {
        /// Static explanation of the violated architectural rule.
        reason: &'static str,
    },
    /// An inherited ARMv4T instruction failed its profile encoder.
    #[error(transparent)]
    Armv4T(#[from] arm7tdmi::EncodeError),
}

/// Little-endian ARM-state implementation of the shared static ISA protocol.
pub struct Arm946EsArm;
/// Little-endian Thumb-state implementation of the shared static ISA protocol.
pub struct Arm946EsThumb;

impl TypedIsa for Arm946EsArm {
    const PROFILE_ID: &'static str = "arm946e-s-arm-le";

    type Instruction = ArmInstruction;
    type Form = ArmInstructionForm;
    type CodeLocation = crate::CodeLocation;
    type DecodeError = DecodeError;
    type EncodeError = EncodeError;

    fn form(instruction: &Self::Instruction) -> Result<Self::Form, Self::EncodeError> {
        Ok(instruction.form())
    }

    fn decode_one(
        source: &[u8],
        at: &Self::CodeLocation,
    ) -> Result<DecodedInstruction<Self::Instruction>, Self::DecodeError> {
        require_decode_alignment(*at, 4)?;
        let instruction = decode_arm_bytes(source)?;
        let bytes = InstructionBytes::new(source[..4].to_vec()).expect("four bytes are non-empty");
        Ok(DecodedInstruction::new(instruction, bytes))
    }

    fn encode_one(
        instruction: &Self::Instruction,
        at: &Self::CodeLocation,
    ) -> Result<InstructionBytes, Self::EncodeError> {
        require_encode_alignment(*at, 4)?;
        Ok(
            InstructionBytes::new(encode_arm_bytes(instruction)?.to_vec())
                .expect("four bytes are non-empty"),
        )
    }
}

impl TypedIsa for Arm946EsThumb {
    const PROFILE_ID: &'static str = "arm946e-s-thumb-le";

    type Instruction = ThumbInstruction;
    type Form = ThumbInstructionForm;
    type CodeLocation = crate::CodeLocation;
    type DecodeError = DecodeError;
    type EncodeError = EncodeError;

    fn form(instruction: &Self::Instruction) -> Result<Self::Form, Self::EncodeError> {
        Ok(instruction.form())
    }

    fn decode_one(
        source: &[u8],
        at: &Self::CodeLocation,
    ) -> Result<DecodedInstruction<Self::Instruction>, Self::DecodeError> {
        require_decode_alignment(*at, 2)?;
        let instruction = decode_thumb_bytes(source)?;
        let len = instruction.encoded_len();
        let bytes =
            InstructionBytes::new(source[..len].to_vec()).expect("instruction is non-empty");
        Ok(DecodedInstruction::new(instruction, bytes))
    }

    fn encode_one(
        instruction: &Self::Instruction,
        at: &Self::CodeLocation,
    ) -> Result<InstructionBytes, Self::EncodeError> {
        require_encode_alignment(*at, 2)?;
        Ok(InstructionBytes::new(encode_thumb_bytes(instruction)?)
            .expect("instruction is non-empty"))
    }
}

/// Decode one little-endian 32-bit ARM-state instruction from `source`.
pub fn decode_arm_bytes(source: &[u8]) -> Result<ArmInstruction, DecodeError> {
    let bytes: [u8; 4] = source
        .get(..4)
        .ok_or(DecodeError::UnexpectedEnd {
            needed: 4,
            got: source.len(),
        })?
        .try_into()
        .expect("slice length was checked");
    decode_arm_word(u32::from_le_bytes(bytes))
}

/// Lower one typed ARM-state instruction to a little-endian 32-bit word.
pub fn encode_arm_bytes(instruction: &ArmInstruction) -> Result<[u8; 4], EncodeError> {
    Ok(encode_arm_word(instruction)?.to_le_bytes())
}

fn decode_arm_word(word: u32) -> Result<ArmInstruction, DecodeError> {
    if word & 0xfff0_00f0 == 0xe120_0070 {
        return Ok(ArmInstruction::Breakpoint {
            comment: (((word >> 8) & 0xfff) << 4 | (word & 0xf)) as u16,
        });
    }
    if word & 0xfe00_0000 == 0xfa00_0000 {
        let encoded = ((word & 0x00ff_ffff) << 2) | ((word >> 23) & 0x2);
        return Ok(ArmInstruction::BranchLinkExchangeImmediate {
            displacement: sign_extend(encoded, 26),
        });
    }

    let condition_field = (word >> 28) as u8;
    if condition_field == 0xf {
        return decode_unconditional_extension(word);
    }
    let condition = condition(condition_field).expect("NV was handled");

    if word & 0x0fff_fff0 == 0x012f_ff30 {
        let target = reg(word);
        if target == Register::PC {
            return Err(unpredictable_arm(word));
        }
        return Ok(ArmInstruction::BranchLinkExchangeRegister { condition, target });
    }
    if word & 0x0fff_0ff0 == 0x016f_0f10 {
        let destination = reg(word >> 12);
        let source = reg(word);
        if destination == Register::PC || source == Register::PC {
            return Err(unpredictable_arm(word));
        }
        return Ok(ArmInstruction::CountLeadingZeros {
            condition,
            destination,
            source,
        });
    }
    if word & 0x0f90_0ff0 == 0x0100_0050 {
        let instruction = ArmInstruction::SaturatingArithmetic {
            condition,
            operation: SaturatingOperation::from_field(((word >> 21) & 3) as u8),
            second: reg(word >> 16),
            destination: reg(word >> 12),
            first: reg(word),
        };
        validate_arm_extension(&instruction).map_err(|_| unpredictable_arm(word))?;
        return Ok(instruction);
    }
    if word & 0x0f90_0090 == 0x0100_0080 {
        return decode_signed_multiply(word, condition);
    }
    if word & 0x0fe0_0000 == 0x0c40_0000 {
        let instruction = ArmInstruction::CoprocessorDoubleRegisterTransfer {
            condition,
            load: word & (1 << 20) != 0,
            second_arm_register: reg(word >> 16),
            first_arm_register: reg(word >> 12),
            coprocessor: coprocessor(word >> 8),
            opcode: coprocessor_opcode(word >> 4),
            coprocessor_register: coprocessor_register(word),
        };
        validate_arm_extension(&instruction).map_err(|_| unpredictable_arm(word))?;
        return Ok(instruction);
    }
    if word & 0x0e10_00d0 == 0x0000_00d0 {
        return decode_doubleword_transfer(word, condition);
    }

    arm7tdmi::decode_arm_bytes(&word.to_le_bytes())
        .map(ArmInstruction::Armv4T)
        .map_err(DecodeError::Armv4T)
}

fn decode_unconditional_extension(word: u32) -> Result<ArmInstruction, DecodeError> {
    if is_preload_data(word) {
        let offset = decode_address_offset(word)?;
        return Ok(ArmInstruction::PreloadData {
            address: AddressingMode2 {
                base: reg(word >> 16),
                offset,
                add: word & (1 << 23) != 0,
                index: IndexMode::Offset,
            },
        });
    }
    match (word >> 25) & 7 {
        0b110 => Ok(ArmInstruction::CoprocessorDataTransfer2 {
            load: word & (1 << 20) != 0,
            long: word & (1 << 22) != 0,
            coprocessor: coprocessor(word >> 8),
            register: coprocessor_register(word >> 12),
            base: reg(word >> 16),
            offset_words: word as u8,
            add: word & (1 << 23) != 0,
            index: decode_coprocessor_index(word)?,
        }),
        0b111 if word & (1 << 24) == 0 && word & (1 << 4) == 0 => {
            Ok(ArmInstruction::CoprocessorDataOperation2 {
                coprocessor: coprocessor(word >> 8),
                opcode: coprocessor_opcode(word >> 20),
                info: ((word >> 5) & 7) as u8,
                destination: coprocessor_register(word >> 12),
                first: coprocessor_register(word >> 16),
                second: coprocessor_register(word),
            })
        }
        0b111 if word & (1 << 24) == 0 && word & (1 << 4) != 0 => {
            Ok(ArmInstruction::CoprocessorRegisterTransfer2 {
                load: word & (1 << 20) != 0,
                coprocessor: coprocessor(word >> 8),
                opcode: ((word >> 21) & 7) as u8,
                info: ((word >> 5) & 7) as u8,
                arm_register: reg(word >> 12),
                first: coprocessor_register(word >> 16),
                second: coprocessor_register(word),
            })
        }
        _ => Err(reserved_arm(word)),
    }
}

fn decode_signed_multiply(word: u32, condition: Condition) -> Result<ArmInstruction, DecodeError> {
    let first_half = HalfwordSelection::from_bit(word & (1 << 5) != 0);
    let second_half = HalfwordSelection::from_bit(word & (1 << 6) != 0);
    let operation = ((word >> 21) & 3) as u8;
    let instruction = match operation {
        0 => ArmInstruction::SignedHalfwordMultiplyAccumulate {
            condition,
            destination: reg(word >> 16),
            addend: reg(word >> 12),
            second: reg(word >> 8),
            second_half,
            first_half,
            first: reg(word),
        },
        1 if first_half == HalfwordSelection::Bottom => {
            ArmInstruction::SignedWordHalfwordMultiplyAccumulate {
                condition,
                destination: reg(word >> 16),
                addend: reg(word >> 12),
                halfword: reg(word >> 8),
                half: second_half,
                word: reg(word),
            }
        }
        1 => {
            if word & 0x0000_f000 != 0 {
                return Err(unpredictable_arm(word));
            }
            ArmInstruction::SignedWordHalfwordMultiply {
                condition,
                destination: reg(word >> 16),
                halfword: reg(word >> 8),
                half: second_half,
                word: reg(word),
            }
        }
        2 => ArmInstruction::SignedHalfwordMultiplyAccumulateLong {
            condition,
            destination_high: reg(word >> 16),
            destination_low: reg(word >> 12),
            second: reg(word >> 8),
            second_half,
            first_half,
            first: reg(word),
        },
        3 => {
            if word & 0x0000_f000 != 0 {
                return Err(unpredictable_arm(word));
            }
            ArmInstruction::SignedHalfwordMultiply {
                condition,
                destination: reg(word >> 16),
                second: reg(word >> 8),
                second_half,
                first_half,
                first: reg(word),
            }
        }
        _ => unreachable!(),
    };
    validate_arm_extension(&instruction).map_err(|_| unpredictable_arm(word))?;
    Ok(instruction)
}

fn decode_doubleword_transfer(
    word: u32,
    condition: Condition,
) -> Result<ArmInstruction, DecodeError> {
    let offset = if word & (1 << 22) != 0 {
        HalfwordOffset::Immediate((((word >> 4) & 0xf0) | (word & 0xf)) as u8)
    } else {
        if word & 0x0000_0f00 != 0 {
            return Err(reserved_arm(word));
        }
        HalfwordOffset::Register(reg(word))
    };
    let instruction = ArmInstruction::DoublewordTransfer {
        condition,
        load: word & (1 << 5) == 0,
        first_register: reg(word >> 12),
        address: AddressingMode3 {
            base: reg(word >> 16),
            offset,
            add: word & (1 << 23) != 0,
            index: decode_mode3_index(word)?,
        },
    };
    validate_arm_extension(&instruction).map_err(|_| unpredictable_arm(word))?;
    Ok(instruction)
}

fn encode_arm_word(instruction: &ArmInstruction) -> Result<u32, EncodeError> {
    validate_arm_extension(instruction)?;
    Ok(match *instruction {
        ArmInstruction::Armv4T(instruction) => {
            u32::from_le_bytes(arm7tdmi::encode_arm_bytes(&instruction)?)
        }
        ArmInstruction::Breakpoint { comment } => {
            0xe120_0070 | (u32::from(comment >> 4) << 8) | u32::from(comment & 0xf)
        }
        ArmInstruction::BranchLinkExchangeImmediate { displacement } => {
            check_signed_aligned("ARM BLX displacement", displacement, 26, 2)?;
            0xfa00_0000
                | ((displacement as u32 >> 2) & 0x00ff_ffff)
                | ((displacement as u32 & 2) << 23)
        }
        ArmInstruction::BranchLinkExchangeRegister { condition, target } => {
            condition_bits(condition) | 0x012f_ff30 | u32::from(target.index())
        }
        ArmInstruction::CountLeadingZeros {
            condition,
            destination,
            source,
        } => {
            condition_bits(condition)
                | 0x016f_0f10
                | (u32::from(destination.index()) << 12)
                | u32::from(source.index())
        }
        ArmInstruction::DoublewordTransfer {
            condition,
            load,
            first_register,
            address,
        } => {
            let (immediate, offset) = encode_halfword_offset(address.offset);
            let (pre, write) = encode_mode3_index(address.index)?;
            condition_bits(condition)
                | (u32::from(pre) << 24)
                | (u32::from(address.add) << 23)
                | (u32::from(immediate) << 22)
                | (u32::from(write) << 21)
                | (u32::from(address.base.index()) << 16)
                | (u32::from(first_register.index()) << 12)
                | offset
                | 0xd0
                | (u32::from(!load) << 5)
        }
        ArmInstruction::CoprocessorDataOperation2 {
            coprocessor,
            opcode,
            info,
            destination,
            first,
            second,
        } => {
            0xfe00_0000
                | (u32::from(opcode.value()) << 20)
                | (u32::from(first.value()) << 16)
                | (u32::from(destination.value()) << 12)
                | (u32::from(coprocessor.value()) << 8)
                | (u32::from(info) << 5)
                | u32::from(second.value())
        }
        ArmInstruction::CoprocessorRegisterTransfer2 {
            load,
            coprocessor,
            opcode,
            info,
            arm_register,
            first,
            second,
        } => {
            0xfe00_0010
                | (u32::from(opcode) << 21)
                | (u32::from(load) << 20)
                | (u32::from(first.value()) << 16)
                | (u32::from(arm_register.index()) << 12)
                | (u32::from(coprocessor.value()) << 8)
                | (u32::from(info) << 5)
                | u32::from(second.value())
        }
        ArmInstruction::CoprocessorDataTransfer2 {
            load,
            long,
            coprocessor,
            register,
            base,
            offset_words,
            add,
            index,
        } => {
            let (pre, write) = encode_coprocessor_index(index)?;
            0xfc00_0000
                | (u32::from(pre) << 24)
                | (u32::from(add) << 23)
                | (u32::from(long) << 22)
                | (u32::from(write) << 21)
                | (u32::from(load) << 20)
                | (u32::from(base.index()) << 16)
                | (u32::from(register.value()) << 12)
                | (u32::from(coprocessor.value()) << 8)
                | u32::from(offset_words)
        }
        ArmInstruction::CoprocessorDoubleRegisterTransfer {
            condition,
            load,
            coprocessor,
            opcode,
            first_arm_register,
            second_arm_register,
            coprocessor_register,
        } => {
            condition_bits(condition)
                | 0x0c40_0000
                | (u32::from(load) << 20)
                | (u32::from(second_arm_register.index()) << 16)
                | (u32::from(first_arm_register.index()) << 12)
                | (u32::from(coprocessor.value()) << 8)
                | (u32::from(opcode.value()) << 4)
                | u32::from(coprocessor_register.value())
        }
        ArmInstruction::PreloadData { address } => {
            let (register_offset, offset) = encode_address_offset(address.offset)?;
            0xf550_f000
                | (u32::from(register_offset) << 25)
                | (u32::from(address.add) << 23)
                | (u32::from(address.base.index()) << 16)
                | offset
        }
        ArmInstruction::SaturatingArithmetic {
            condition,
            operation,
            destination,
            first,
            second,
        } => {
            condition_bits(condition)
                | 0x0100_0050
                | ((operation as u32) << 21)
                | (u32::from(second.index()) << 16)
                | (u32::from(destination.index()) << 12)
                | u32::from(first.index())
        }
        ArmInstruction::SignedHalfwordMultiplyAccumulate {
            condition,
            destination,
            first,
            first_half,
            second,
            second_half,
            addend,
        } => encode_signed_multiply(
            condition,
            0,
            destination,
            addend,
            second,
            second_half,
            first_half,
            first,
        ),
        ArmInstruction::SignedHalfwordMultiplyAccumulateLong {
            condition,
            destination_low,
            destination_high,
            first,
            first_half,
            second,
            second_half,
        } => encode_signed_multiply(
            condition,
            2,
            destination_high,
            destination_low,
            second,
            second_half,
            first_half,
            first,
        ),
        ArmInstruction::SignedWordHalfwordMultiplyAccumulate {
            condition,
            destination,
            word,
            halfword,
            half,
            addend,
        } => encode_signed_multiply(
            condition,
            1,
            destination,
            addend,
            halfword,
            half,
            HalfwordSelection::Bottom,
            word,
        ),
        ArmInstruction::SignedHalfwordMultiply {
            condition,
            destination,
            first,
            first_half,
            second,
            second_half,
        } => encode_signed_multiply(
            condition,
            3,
            destination,
            Register::new(0).expect("R0 exists"),
            second,
            second_half,
            first_half,
            first,
        ),
        ArmInstruction::SignedWordHalfwordMultiply {
            condition,
            destination,
            word,
            halfword,
            half,
        } => encode_signed_multiply(
            condition,
            1,
            destination,
            Register::new(0).expect("R0 exists"),
            halfword,
            half,
            HalfwordSelection::Top,
            word,
        ),
    })
}

#[allow(clippy::too_many_arguments)]
fn encode_signed_multiply(
    condition: Condition,
    operation: u8,
    destination_or_high: Register,
    addend_or_low: Register,
    second: Register,
    second_half: HalfwordSelection,
    first_half: HalfwordSelection,
    first: Register,
) -> u32 {
    condition_bits(condition)
        | 0x0100_0080
        | (u32::from(operation) << 21)
        | (u32::from(destination_or_high.index()) << 16)
        | (u32::from(addend_or_low.index()) << 12)
        | (u32::from(second.index()) << 8)
        | (u32::from(second_half == HalfwordSelection::Top) << 6)
        | (u32::from(first_half == HalfwordSelection::Top) << 5)
        | u32::from(first.index())
}

fn validate_arm_extension(instruction: &ArmInstruction) -> Result<(), EncodeError> {
    match *instruction {
        ArmInstruction::Armv4T(_) | ArmInstruction::Breakpoint { .. } => {}
        ArmInstruction::BranchLinkExchangeRegister { target, .. } if target == Register::PC => {
            return invalid("BLX register target cannot use PC");
        }
        ArmInstruction::CountLeadingZeros {
            destination,
            source,
            ..
        } if destination == Register::PC || source == Register::PC => {
            return invalid("CLZ registers cannot use PC");
        }
        ArmInstruction::DoublewordTransfer {
            load,
            first_register,
            address,
            ..
        } => {
            if first_register.index() & 1 != 0 || first_register.index() > 12 {
                return invalid("LDRD and STRD require an even first register from R0 through R12");
            }
            if address.index == IndexMode::PostIndexedUser {
                return invalid("doubleword transfers have no user-mode post-index form");
            }
            if address.index != IndexMode::Offset && address.base == Register::PC {
                return invalid("doubleword write-back or post-index base cannot use PC");
            }
            if load
                && address.index != IndexMode::Offset
                && (address.base.index() == first_register.index()
                    || address.base.index() == first_register.index() + 1)
            {
                return invalid("LDRD write-back base cannot overlap its destination pair");
            }
        }
        ArmInstruction::CoprocessorDataOperation2 { info, .. } if info > 7 => {
            return field_error("coprocessor information", info as u32, 7);
        }
        ArmInstruction::CoprocessorRegisterTransfer2 { opcode, info, .. } => {
            if opcode > 7 {
                return field_error("coprocessor register-transfer opcode", opcode as u32, 7);
            }
            if info > 7 {
                return field_error("coprocessor information", info as u32, 7);
            }
        }
        ArmInstruction::CoprocessorDoubleRegisterTransfer {
            first_arm_register,
            second_arm_register,
            ..
        } => {
            if first_arm_register == Register::PC || second_arm_register == Register::PC {
                return invalid("MCRR and MRRC ARM registers cannot use PC");
            }
            if first_arm_register == second_arm_register {
                return invalid("MCRR and MRRC require two distinct ARM registers");
            }
        }
        ArmInstruction::PreloadData { address } => {
            if address.index != IndexMode::Offset {
                return invalid("PLD permits only offset addressing without write-back");
            }
        }
        ArmInstruction::SaturatingArithmetic {
            destination,
            first,
            second,
            ..
        } => validate_no_pc(&[destination, first, second], "saturating arithmetic")?,
        ArmInstruction::SignedHalfwordMultiplyAccumulate {
            destination,
            first,
            second,
            addend,
            ..
        } => validate_no_pc(
            &[destination, first, second, addend],
            "signed halfword multiply-accumulate",
        )?,
        ArmInstruction::SignedHalfwordMultiplyAccumulateLong {
            destination_low,
            destination_high,
            first,
            second,
            ..
        } => {
            validate_no_pc(
                &[destination_low, destination_high, first, second],
                "signed halfword multiply-accumulate long",
            )?;
            if destination_low == destination_high {
                return invalid("long multiply-accumulate destinations must be distinct");
            }
        }
        ArmInstruction::SignedWordHalfwordMultiplyAccumulate {
            destination,
            word,
            halfword,
            addend,
            ..
        } => validate_no_pc(
            &[destination, word, halfword, addend],
            "signed word-halfword multiply-accumulate",
        )?,
        ArmInstruction::SignedHalfwordMultiply {
            destination,
            first,
            second,
            ..
        } => validate_no_pc(&[destination, first, second], "signed halfword multiply")?,
        ArmInstruction::SignedWordHalfwordMultiply {
            destination,
            word,
            halfword,
            ..
        } => validate_no_pc(
            &[destination, word, halfword],
            "signed word-halfword multiply",
        )?,
        _ => {}
    }
    Ok(())
}

fn validate_no_pc(registers: &[Register], operation: &'static str) -> Result<(), EncodeError> {
    if registers.contains(&Register::PC) {
        invalid(operation)
    } else {
        Ok(())
    }
}

fn is_preload_data(word: u32) -> bool {
    word >> 28 == 0xf
        && (word >> 26) & 3 == 1
        && word & (1 << 24) != 0
        && word & (1 << 22) != 0
        && word & (1 << 21) == 0
        && word & (1 << 20) != 0
        && (word >> 12) & 0xf == 0xf
}

fn decode_address_offset(word: u32) -> Result<AddressOffset, DecodeError> {
    if word & (1 << 25) == 0 {
        Ok(AddressOffset::Immediate((word & 0xfff) as u16))
    } else {
        if word & (1 << 4) != 0 {
            return Err(reserved_arm(word));
        }
        Ok(AddressOffset::Register {
            register: reg(word),
            shift: decode_immediate_shift(
                shift_kind(((word >> 5) & 3) as u8),
                ((word >> 7) & 0x1f) as u8,
            ),
        })
    }
}

fn encode_address_offset(offset: AddressOffset) -> Result<(bool, u32), EncodeError> {
    Ok(match offset {
        AddressOffset::Immediate(value) if value <= 0xfff => (false, u32::from(value)),
        AddressOffset::Immediate(value) => {
            return field_error("mode 2 immediate offset", value as u32, 0xfff);
        }
        AddressOffset::Register { register, shift } => {
            let (kind, amount) = encode_immediate_shift(shift)?;
            (
                true,
                (u32::from(amount) << 7) | ((kind as u32) << 5) | u32::from(register.index()),
            )
        }
    })
}

fn encode_immediate_shift(shift: ImmediateShift) -> Result<(ShiftKind, u8), EncodeError> {
    match shift {
        ImmediateShift::LogicalLeft(value @ 0..=31) => Ok((ShiftKind::LogicalLeft, value)),
        ImmediateShift::LogicalRight(value @ 1..=31) => Ok((ShiftKind::LogicalRight, value)),
        ImmediateShift::LogicalRight(32) => Ok((ShiftKind::LogicalRight, 0)),
        ImmediateShift::ArithmeticRight(value @ 1..=31) => Ok((ShiftKind::ArithmeticRight, value)),
        ImmediateShift::ArithmeticRight(32) => Ok((ShiftKind::ArithmeticRight, 0)),
        ImmediateShift::RotateRight(value @ 1..=31) => Ok((ShiftKind::RotateRight, value)),
        ImmediateShift::RotateRightExtended => Ok((ShiftKind::RotateRight, 0)),
        ImmediateShift::LogicalLeft(value)
        | ImmediateShift::LogicalRight(value)
        | ImmediateShift::ArithmeticRight(value)
        | ImmediateShift::RotateRight(value) => {
            field_error("immediate shift amount", value as u32, 32)
        }
    }
}

fn decode_mode3_index(word: u32) -> Result<IndexMode, DecodeError> {
    match (word & (1 << 24) != 0, word & (1 << 21) != 0) {
        (true, false) => Ok(IndexMode::Offset),
        (true, true) => Ok(IndexMode::PreIndexed),
        (false, false) => Ok(IndexMode::PostIndexed),
        _ => Err(reserved_arm(word)),
    }
}

fn encode_mode3_index(index: IndexMode) -> Result<(bool, bool), EncodeError> {
    Ok(match index {
        IndexMode::Offset => (true, false),
        IndexMode::PreIndexed => (true, true),
        IndexMode::PostIndexed => (false, false),
        IndexMode::PostIndexedUser => {
            return invalid("mode 3 has no user-mode post-index encoding");
        }
    })
}

fn decode_coprocessor_index(word: u32) -> Result<IndexMode, DecodeError> {
    match (word & (1 << 24) != 0, word & (1 << 21) != 0) {
        (true, false) => Ok(IndexMode::Offset),
        (true, true) => Ok(IndexMode::PreIndexed),
        (false, true) => Ok(IndexMode::PostIndexed),
        _ => Err(reserved_arm(word)),
    }
}

fn encode_coprocessor_index(index: IndexMode) -> Result<(bool, bool), EncodeError> {
    Ok(match index {
        IndexMode::Offset => (true, false),
        IndexMode::PreIndexed => (true, true),
        IndexMode::PostIndexed => (false, true),
        IndexMode::PostIndexedUser => {
            return invalid("coprocessor transfers have no user-mode post-index form");
        }
    })
}

fn encode_halfword_offset(offset: HalfwordOffset) -> (bool, u32) {
    match offset {
        HalfwordOffset::Immediate(value) => (
            true,
            (u32::from(value & 0xf0) << 4) | u32::from(value & 0xf),
        ),
        HalfwordOffset::Register(register) => (false, u32::from(register.index())),
    }
}

/// Decode one little-endian Thumb-state instruction from `source`.
pub fn decode_thumb_bytes(source: &[u8]) -> Result<ThumbInstruction, DecodeError> {
    let first = read_thumb_halfword(source, 0)?;
    if first & 0xff00 == 0xbe00 {
        return Ok(ThumbInstruction::Breakpoint {
            comment: first as u8,
        });
    }
    if first & 0xff87 == 0x4780 {
        let target = Register::new(((first >> 3) & 0xf) as u8).expect("four-bit register");
        if target == Register::PC {
            return Err(unpredictable_thumb(first));
        }
        return Ok(ThumbInstruction::BranchLinkExchangeRegister { target });
    }
    if first >> 11 == 0b11110 {
        let second = read_thumb_halfword(source, 2)?;
        if second & 0xf801 == 0xe800 {
            let encoded = (u32::from(first & 0x7ff) << 12) | (u32::from(second & 0x7fe) << 1);
            return Ok(ThumbInstruction::BranchLinkExchangeImmediate {
                displacement: sign_extend(encoded, 23),
            });
        }
    }
    arm7tdmi::decode_thumb_bytes(source)
        .map(ThumbInstruction::Armv4T)
        .map_err(DecodeError::Armv4T)
}

/// Lower one typed Thumb-state instruction to two or four little-endian bytes.
pub fn encode_thumb_bytes(instruction: &ThumbInstruction) -> Result<Vec<u8>, EncodeError> {
    match *instruction {
        ThumbInstruction::Armv4T(instruction) => {
            arm7tdmi::encode_thumb_bytes(&instruction).map_err(EncodeError::Armv4T)
        }
        ThumbInstruction::Breakpoint { comment } => {
            Ok((0xbe00 | u16::from(comment)).to_le_bytes().to_vec())
        }
        ThumbInstruction::BranchLinkExchangeRegister { target } => {
            if target == Register::PC {
                return invalid("Thumb BLX register target cannot use PC");
            }
            Ok((0x4780 | (u16::from(target.index()) << 3))
                .to_le_bytes()
                .to_vec())
        }
        ThumbInstruction::BranchLinkExchangeImmediate { displacement } => {
            check_signed_aligned("Thumb BLX displacement", displacement, 23, 4)?;
            let encoded = displacement as u32 & 0x007f_fffc;
            let first = 0xf000 | ((encoded >> 12) as u16 & 0x7ff);
            let second = 0xe800 | ((encoded >> 1) as u16 & 0x7fe);
            let mut bytes = Vec::with_capacity(4);
            bytes.extend_from_slice(&first.to_le_bytes());
            bytes.extend_from_slice(&second.to_le_bytes());
            Ok(bytes)
        }
    }
}

fn read_thumb_halfword(source: &[u8], offset: usize) -> Result<u16, DecodeError> {
    let bytes: [u8; 2] = source
        .get(offset..offset + 2)
        .ok_or(DecodeError::UnexpectedEnd {
            needed: offset + 2,
            got: source.len(),
        })?
        .try_into()
        .expect("slice length was checked");
    Ok(u16::from_le_bytes(bytes))
}

fn require_decode_alignment(
    location: crate::CodeLocation,
    alignment: u8,
) -> Result<(), DecodeError> {
    if location & (u32::from(alignment) - 1) == 0 {
        Ok(())
    } else {
        Err(DecodeError::UnalignedAddress {
            location,
            alignment,
        })
    }
}

fn require_encode_alignment(
    location: crate::CodeLocation,
    alignment: u8,
) -> Result<(), EncodeError> {
    if location & (u32::from(alignment) - 1) == 0 {
        Ok(())
    } else {
        Err(EncodeError::UnalignedAddress {
            location,
            alignment,
        })
    }
}

fn condition_bits(condition: Condition) -> u32 {
    (condition as u32) << 28
}

fn condition(value: u8) -> Option<Condition> {
    Some(match value {
        0x0 => Condition::Equal,
        0x1 => Condition::NotEqual,
        0x2 => Condition::CarrySet,
        0x3 => Condition::CarryClear,
        0x4 => Condition::Negative,
        0x5 => Condition::PositiveOrZero,
        0x6 => Condition::Overflow,
        0x7 => Condition::NoOverflow,
        0x8 => Condition::UnsignedHigher,
        0x9 => Condition::UnsignedLowerOrSame,
        0xa => Condition::SignedGreaterOrEqual,
        0xb => Condition::SignedLessThan,
        0xc => Condition::SignedGreaterThan,
        0xd => Condition::SignedLessOrEqual,
        0xe => Condition::Always,
        _ => return None,
    })
}

fn shift_kind(value: u8) -> ShiftKind {
    match value {
        0 => ShiftKind::LogicalLeft,
        1 => ShiftKind::LogicalRight,
        2 => ShiftKind::ArithmeticRight,
        3 => ShiftKind::RotateRight,
        _ => unreachable!(),
    }
}

fn decode_immediate_shift(kind: ShiftKind, encoded_amount: u8) -> ImmediateShift {
    match (kind, encoded_amount) {
        (ShiftKind::LogicalLeft, value) => ImmediateShift::LogicalLeft(value),
        (ShiftKind::LogicalRight, 0) => ImmediateShift::LogicalRight(32),
        (ShiftKind::LogicalRight, value) => ImmediateShift::LogicalRight(value),
        (ShiftKind::ArithmeticRight, 0) => ImmediateShift::ArithmeticRight(32),
        (ShiftKind::ArithmeticRight, value) => ImmediateShift::ArithmeticRight(value),
        (ShiftKind::RotateRight, 0) => ImmediateShift::RotateRightExtended,
        (ShiftKind::RotateRight, value) => ImmediateShift::RotateRight(value),
    }
}

fn reg(value: u32) -> Register {
    Register::new((value & 0xf) as u8).expect("four-bit register field")
}

fn coprocessor(value: u32) -> Coprocessor {
    Coprocessor::new((value & 0xf) as u8).expect("four-bit coprocessor field")
}

fn coprocessor_register(value: u32) -> CoprocessorRegister {
    CoprocessorRegister::new((value & 0xf) as u8).expect("four-bit coprocessor register field")
}

fn coprocessor_opcode(value: u32) -> CoprocessorOpcode {
    CoprocessorOpcode::new((value & 0xf) as u8).expect("four-bit coprocessor opcode field")
}

const fn sign_extend(value: u32, width: u32) -> i32 {
    ((value as i32) << (32 - width)) >> (32 - width)
}

fn check_signed_aligned(
    field: &'static str,
    value: i32,
    width: u8,
    alignment: u8,
) -> Result<(), EncodeError> {
    if value % i32::from(alignment) != 0 {
        return Err(EncodeError::UnalignedField {
            field,
            value,
            alignment,
        });
    }
    let minimum = -(1_i64 << (width - 1));
    let maximum = (1_i64 << (width - 1)) - i64::from(alignment);
    if i64::from(value) < minimum || i64::from(value) > maximum {
        return Err(EncodeError::FieldOutOfRange {
            field,
            value: value as u32,
            maximum: maximum as u32,
        });
    }
    Ok(())
}

fn reserved_arm(word: u32) -> DecodeError {
    DecodeError::ReservedEncoding {
        width: 32,
        value: word,
    }
}

fn unpredictable_arm(word: u32) -> DecodeError {
    DecodeError::UnpredictableEncoding {
        width: 32,
        value: word,
    }
}

fn unpredictable_thumb(word: u16) -> DecodeError {
    DecodeError::UnpredictableEncoding {
        width: 16,
        value: u32::from(word),
    }
}

fn invalid<T>(reason: &'static str) -> Result<T, EncodeError> {
    Err(EncodeError::InvalidOperand { reason })
}

fn field_error<T>(field: &'static str, value: u32, maximum: u32) -> Result<T, EncodeError> {
    Err(EncodeError::FieldOutOfRange {
        field,
        value,
        maximum,
    })
}
