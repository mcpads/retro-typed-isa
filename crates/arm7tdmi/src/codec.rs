use thiserror::Error;
use typed_isa_core::{DecodedInstruction, InstructionBytes, TypedIsa};

use crate::{
    AddressOffset, AddressingMode2, AddressingMode3, ArmInstruction, ArmInstructionForm,
    BlockAddressing, Condition, Coprocessor, CoprocessorOpcode, CoprocessorRegister, DataOperation,
    HalfwordOffset, HalfwordTransferKind, ImmediateShift, IndexMode, LowRegister, Operand2,
    Register, RegisterList, RotatedImmediate, ShiftKind, StatusMask, StatusRegister, StatusSource,
    ThumbAddOperand, ThumbAluOperation, ThumbHighOperation, ThumbImmediateOperation,
    ThumbInstruction, ThumbInstructionForm, ThumbRegisterList, ThumbTransferOperation,
    ThumbTransferWidth, TransferWidth,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
/// Failure returned while decoding an ARM or Thumb instruction.
pub enum DecodeError {
    #[error("ARM7TDMI instruction needs {needed} bytes, but only {got} are available")]
    /// The input does not contain one complete instruction.
    UnexpectedEnd {
        /// The required instruction length.
        needed: usize,
        /// The available input length.
        got: usize,
    },
    #[error("ARM7TDMI code location {location:#x} is not aligned to {alignment} bytes")]
    /// The instruction address violates the active state's alignment.
    UnalignedAddress {
        /// The supplied code location.
        location: crate::CodeLocation,
        /// The required byte alignment.
        alignment: u8,
    },
    #[error("reserved ARM7TDMI {width}-bit instruction encoding {value:#010x}")]
    /// The word or halfword belongs to a reserved encoding.
    ReservedEncoding {
        /// The instruction width in bits.
        width: u8,
        /// The rejected encoding.
        value: u32,
    },
    #[error(
        "architecturally unpredictable ARM7TDMI {width}-bit instruction encoding {value:#010x}"
    )]
    /// ARMv4T classifies the otherwise recognized encoding as unpredictable.
    UnpredictableEncoding {
        /// The instruction width in bits.
        width: u8,
        /// The rejected encoding.
        value: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
/// Failure returned when a typed instruction cannot be represented by ARMv4T bytes.
pub enum EncodeError {
    #[error("{field} value {value} is outside the encoded maximum {maximum}")]
    /// A numeric operand exceeds its encoding field.
    FieldOutOfRange {
        /// The rejected field name.
        field: &'static str,
        /// The rejected field value.
        value: u32,
        /// The greatest encodable value.
        maximum: u32,
    },
    #[error("{field} value {value} does not have required alignment {alignment}")]
    /// A displacement or offset is not aligned as required by its encoding.
    UnalignedField {
        /// The rejected field name.
        field: &'static str,
        /// The rejected field value.
        value: i32,
        /// The required byte alignment.
        alignment: u8,
    },
    #[error("ARM7TDMI code location {location:#x} is not aligned to {alignment} bytes")]
    /// The output address violates the active state's alignment.
    UnalignedAddress {
        /// The supplied code location.
        location: crate::CodeLocation,
        /// The required byte alignment.
        alignment: u8,
    },
    #[error("operand combination is not defined by ARMv4T: {reason}")]
    /// The operand combination is reserved or architecturally unpredictable.
    InvalidOperand {
        /// A stable explanation of the rejected combination.
        reason: &'static str,
    },
}

/// The little-endian ARM-state protocol profile with four-byte instructions.
pub struct Arm7TdmiArm;
/// The little-endian Thumb-state protocol profile with two- or four-byte instructions.
pub struct Arm7TdmiThumb;

impl TypedIsa for Arm7TdmiArm {
    const PROFILE_ID: &'static str = "arm7tdmi-arm-le";

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
        require_location_alignment(*at, 4)?;
        let instruction = decode_arm_bytes(source)?;
        let bytes = InstructionBytes::new(source[..4].to_vec()).expect("four bytes are non-empty");
        Ok(DecodedInstruction::new(instruction, bytes))
    }

    fn encode_one(
        instruction: &Self::Instruction,
        at: &Self::CodeLocation,
    ) -> Result<InstructionBytes, Self::EncodeError> {
        require_encode_location_alignment(*at, 4)?;
        Ok(
            InstructionBytes::new(encode_arm_bytes(instruction)?.to_vec())
                .expect("four bytes are non-empty"),
        )
    }
}

impl TypedIsa for Arm7TdmiThumb {
    const PROFILE_ID: &'static str = "arm7tdmi-thumb-le";

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
        require_location_alignment(*at, 2)?;
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
        require_encode_location_alignment(*at, 2)?;
        Ok(InstructionBytes::new(encode_thumb_bytes(instruction)?)
            .expect("instruction is non-empty"))
    }
}

fn require_location_alignment(
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

fn require_encode_location_alignment(
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

/// Decodes one four-byte little-endian ARM-state instruction.
///
/// This byte-only helper does not validate an external instruction address. Use
/// [`TypedIsa::decode_one`] through [`Arm7TdmiArm`] when address alignment is
/// part of the contract.
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

/// Encodes one typed ARM-state instruction as four little-endian bytes.
pub fn encode_arm_bytes(instruction: &ArmInstruction) -> Result<[u8; 4], EncodeError> {
    Ok(encode_arm_word(instruction)?.to_le_bytes())
}

fn decode_arm_word(word: u32) -> Result<ArmInstruction, DecodeError> {
    let condition = Condition::from_field((word >> 28) as u8).ok_or(reserved_arm(word))?;

    if word & 0x0fff_fff0 == 0x012f_ff10 {
        return Ok(ArmInstruction::BranchExchange {
            condition,
            target: reg(word),
        });
    }
    if word & 0x0fbf_0fff == 0x010f_0000 {
        return Ok(ArmInstruction::MoveStatusToRegister {
            condition,
            source: status_register(word & (1 << 22) != 0),
            destination: reg(word >> 12),
        });
    }
    if word & 0x0fb0_fff0 == 0x0120_f000 {
        let bits = ((word >> 16) & 0xf) as u8;
        if bits == 0 {
            return Err(unpredictable_arm(word));
        }
        return Ok(ArmInstruction::MoveToStatusRegister {
            condition,
            destination: status_register(word & (1 << 22) != 0),
            mask: StatusMask::from_bits(bits),
            source: StatusSource::Register(reg(word)),
        });
    }
    if word & 0x0fb0_f000 == 0x0320_f000 {
        let bits = ((word >> 16) & 0xf) as u8;
        if bits != StatusMask::FLAGS.bits() {
            return Err(unpredictable_arm(word));
        }
        return Ok(ArmInstruction::MoveToStatusRegister {
            condition,
            destination: status_register(word & (1 << 22) != 0),
            mask: StatusMask::FLAGS,
            source: StatusSource::Immediate(decode_rotated_immediate(word)),
        });
    }
    if word & 0x0fb0_0ff0 == 0x0100_0090 {
        let instruction = ArmInstruction::Swap {
            condition,
            width: if word & (1 << 22) != 0 {
                TransferWidth::Byte
            } else {
                TransferWidth::Word
            },
            base: reg(word >> 16),
            destination: reg(word >> 12),
            source: reg(word),
        };
        validate_arm_instruction(&instruction).map_err(|_| unpredictable_arm(word))?;
        return Ok(instruction);
    }
    if word & 0x0f80_00f0 == 0x0080_0090 {
        let instruction = ArmInstruction::MultiplyLong {
            condition,
            signed: word & (1 << 22) != 0,
            accumulate: word & (1 << 21) != 0,
            set_flags: word & (1 << 20) != 0,
            destination_high: reg(word >> 16),
            destination_low: reg(word >> 12),
            multiplier: reg(word >> 8),
            multiplicand: reg(word),
        };
        validate_arm_instruction(&instruction).map_err(|_| unpredictable_arm(word))?;
        return Ok(instruction);
    }
    if word & 0x0fc0_00f0 == 0x0000_0090 {
        let instruction = ArmInstruction::Multiply {
            condition,
            accumulate: word & (1 << 21) != 0,
            set_flags: word & (1 << 20) != 0,
            destination: reg(word >> 16),
            addend: reg(word >> 12),
            multiplier: reg(word >> 8),
            multiplicand: reg(word),
        };
        validate_arm_instruction(&instruction).map_err(|_| unpredictable_arm(word))?;
        return Ok(instruction);
    }
    if word & 0x0e00_0090 == 0x0000_0090 && word & 0x60 != 0 {
        return decode_halfword_transfer(word, condition);
    }

    match (word >> 25) & 0x7 {
        0b000 | 0b001 => decode_data_processing(word, condition),
        0b010 | 0b011 => decode_single_transfer(word, condition),
        0b100 => decode_block_transfer(word, condition),
        0b101 => Ok(ArmInstruction::Branch {
            condition,
            link: word & (1 << 24) != 0,
            displacement: sign_extend((word & 0x00ff_ffff) << 2, 26),
        }),
        0b110 => decode_coprocessor_transfer(word, condition),
        0b111 if word & (1 << 24) != 0 => Ok(ArmInstruction::SoftwareInterrupt {
            condition,
            comment: word & 0x00ff_ffff,
        }),
        0b111 if word & (1 << 4) == 0 => decode_coprocessor_operation(word, condition),
        0b111 => decode_coprocessor_register_transfer(word, condition),
        _ => Err(reserved_arm(word)),
    }
}

fn decode_data_processing(word: u32, condition: Condition) -> Result<ArmInstruction, DecodeError> {
    let operation = DataOperation::from_field(((word >> 21) & 0xf) as u8);
    let set_flags = word & (1 << 20) != 0;
    let destination = reg(word >> 12);
    let first = reg(word >> 16);
    let second = if word & (1 << 25) != 0 {
        Operand2::Immediate(decode_rotated_immediate(word))
    } else if word & (1 << 4) == 0 {
        let kind = ShiftKind::from_field(((word >> 5) & 0x3) as u8);
        let amount = ((word >> 7) & 0x1f) as u8;
        Operand2::Register {
            register: reg(word),
            shift: ImmediateShift::from_fields(kind, amount),
        }
    } else {
        if word & (1 << 7) != 0 {
            return Err(reserved_arm(word));
        }
        Operand2::RegisterShift {
            register: reg(word),
            kind: ShiftKind::from_field(((word >> 5) & 0x3) as u8),
            amount: reg(word >> 8),
        }
    };
    let instruction = ArmInstruction::DataProcessing {
        condition,
        operation,
        set_flags,
        destination,
        first,
        second,
    };
    validate_arm_instruction(&instruction).map_err(|_| unpredictable_arm(word))?;
    Ok(instruction)
}

fn decode_single_transfer(word: u32, condition: Condition) -> Result<ArmInstruction, DecodeError> {
    if word & (1 << 25) != 0 && word & (1 << 4) != 0 {
        return Err(reserved_arm(word));
    }
    let offset = if word & (1 << 25) == 0 {
        AddressOffset::Immediate((word & 0xfff) as u16)
    } else {
        AddressOffset::Register {
            register: reg(word),
            shift: ImmediateShift::from_fields(
                ShiftKind::from_field(((word >> 5) & 3) as u8),
                ((word >> 7) & 0x1f) as u8,
            ),
        }
    };
    let instruction = ArmInstruction::SingleTransfer {
        condition,
        load: word & (1 << 20) != 0,
        width: if word & (1 << 22) != 0 {
            TransferWidth::Byte
        } else {
            TransferWidth::Word
        },
        register: reg(word >> 12),
        address: AddressingMode2 {
            base: reg(word >> 16),
            offset,
            add: word & (1 << 23) != 0,
            index: decode_index(word, true)?,
        },
    };
    validate_arm_instruction(&instruction).map_err(|_| unpredictable_arm(word))?;
    Ok(instruction)
}

fn decode_halfword_transfer(
    word: u32,
    condition: Condition,
) -> Result<ArmInstruction, DecodeError> {
    let load = word & (1 << 20) != 0;
    let kind = match ((word >> 5) & 0x3, load) {
        (0b01, _) => HalfwordTransferKind::UnsignedHalfword,
        (0b10, true) => HalfwordTransferKind::SignedByte,
        (0b11, true) => HalfwordTransferKind::SignedHalfword,
        _ => return Err(reserved_arm(word)),
    };
    let offset = if word & (1 << 22) != 0 {
        HalfwordOffset::Immediate((((word >> 4) & 0xf0) | (word & 0xf)) as u8)
    } else {
        if word & 0x0000_0f00 != 0 {
            return Err(reserved_arm(word));
        }
        HalfwordOffset::Register(reg(word))
    };
    let instruction = ArmInstruction::HalfwordTransfer {
        condition,
        load,
        kind,
        register: reg(word >> 12),
        address: AddressingMode3 {
            base: reg(word >> 16),
            offset,
            add: word & (1 << 23) != 0,
            index: decode_index(word, false)?,
        },
    };
    validate_arm_instruction(&instruction).map_err(|_| unpredictable_arm(word))?;
    Ok(instruction)
}

fn decode_block_transfer(word: u32, condition: Condition) -> Result<ArmInstruction, DecodeError> {
    if word & 0xffff == 0 {
        return Err(unpredictable_arm(word));
    }
    let instruction = ArmInstruction::BlockTransfer {
        condition,
        load: word & (1 << 20) != 0,
        addressing: BlockAddressing::from_fields(word & (1 << 24) != 0, word & (1 << 23) != 0),
        user_registers: word & (1 << 22) != 0,
        write_back: word & (1 << 21) != 0,
        base: reg(word >> 16),
        registers: RegisterList::from_bits(word as u16),
    };
    validate_arm_instruction(&instruction).map_err(|_| unpredictable_arm(word))?;
    Ok(instruction)
}

fn decode_coprocessor_transfer(
    word: u32,
    condition: Condition,
) -> Result<ArmInstruction, DecodeError> {
    let instruction = ArmInstruction::CoprocessorDataTransfer {
        condition,
        load: word & (1 << 20) != 0,
        long: word & (1 << 22) != 0,
        coprocessor: Coprocessor::from_field(word >> 8),
        register: CoprocessorRegister::from_field(word >> 12),
        base: reg(word >> 16),
        offset_words: word as u8,
        add: word & (1 << 23) != 0,
        index: decode_coprocessor_index(word)?,
    };
    validate_arm_instruction(&instruction).map_err(|_| unpredictable_arm(word))?;
    Ok(instruction)
}

fn decode_coprocessor_operation(
    word: u32,
    condition: Condition,
) -> Result<ArmInstruction, DecodeError> {
    Ok(ArmInstruction::CoprocessorDataOperation {
        condition,
        coprocessor: Coprocessor::from_field(word >> 8),
        opcode: CoprocessorOpcode::from_field(word >> 20),
        info: ((word >> 5) & 0x7) as u8,
        destination: CoprocessorRegister::from_field(word >> 12),
        first: CoprocessorRegister::from_field(word >> 16),
        second: CoprocessorRegister::from_field(word),
    })
}

fn decode_coprocessor_register_transfer(
    word: u32,
    condition: Condition,
) -> Result<ArmInstruction, DecodeError> {
    Ok(ArmInstruction::CoprocessorRegisterTransfer {
        condition,
        load: word & (1 << 20) != 0,
        coprocessor: Coprocessor::from_field(word >> 8),
        opcode: ((word >> 21) & 0x7) as u8,
        info: ((word >> 5) & 0x7) as u8,
        arm_register: reg(word >> 12),
        first: CoprocessorRegister::from_field(word >> 16),
        second: CoprocessorRegister::from_field(word),
    })
}

fn decode_index(word: u32, user_allowed: bool) -> Result<IndexMode, DecodeError> {
    match (word & (1 << 24) != 0, word & (1 << 21) != 0) {
        (true, false) => Ok(IndexMode::Offset),
        (true, true) => Ok(IndexMode::PreIndexed),
        (false, false) => Ok(IndexMode::PostIndexed),
        (false, true) if user_allowed => Ok(IndexMode::PostIndexedUser),
        _ => Err(reserved_arm(word)),
    }
}

fn decode_coprocessor_index(word: u32) -> Result<IndexMode, DecodeError> {
    match (word & (1 << 24) != 0, word & (1 << 21) != 0) {
        (true, false) => Ok(IndexMode::Offset),
        (true, true) => Ok(IndexMode::PreIndexed),
        (false, true) => Ok(IndexMode::PostIndexed),
        _ => Err(reserved_arm(word)),
    }
}

fn encode_arm_word(instruction: &ArmInstruction) -> Result<u32, EncodeError> {
    validate_arm_instruction(instruction)?;
    let (condition, body) = match *instruction {
        ArmInstruction::DataProcessing {
            condition,
            operation,
            set_flags,
            destination,
            first,
            second,
        } => {
            let (immediate, operand) = encode_operand2(second)?;
            (
                condition,
                (u32::from(immediate) << 25)
                    | ((operation as u32) << 21)
                    | (u32::from(set_flags) << 20)
                    | (u32::from(first.index()) << 16)
                    | (u32::from(destination.index()) << 12)
                    | operand,
            )
        }
        ArmInstruction::Multiply {
            condition,
            accumulate,
            set_flags,
            destination,
            multiplicand,
            multiplier,
            addend,
        } => (
            condition,
            (u32::from(accumulate) << 21)
                | (u32::from(set_flags) << 20)
                | (u32::from(destination.index()) << 16)
                | (u32::from(addend.index()) << 12)
                | (u32::from(multiplier.index()) << 8)
                | 0x90
                | u32::from(multiplicand.index()),
        ),
        ArmInstruction::MultiplyLong {
            condition,
            signed,
            accumulate,
            set_flags,
            destination_low,
            destination_high,
            multiplicand,
            multiplier,
        } => (
            condition,
            0x0080_0090
                | (u32::from(signed) << 22)
                | (u32::from(accumulate) << 21)
                | (u32::from(set_flags) << 20)
                | (u32::from(destination_high.index()) << 16)
                | (u32::from(destination_low.index()) << 12)
                | (u32::from(multiplier.index()) << 8)
                | u32::from(multiplicand.index()),
        ),
        ArmInstruction::BranchExchange { condition, target } => {
            (condition, 0x012f_ff10 | u32::from(target.index()))
        }
        ArmInstruction::Branch {
            condition,
            link,
            displacement,
        } => {
            check_signed_aligned("branch displacement", displacement, 26, 4)?;
            (
                condition,
                0x0a00_0000 | (u32::from(link) << 24) | ((displacement as u32 >> 2) & 0x00ff_ffff),
            )
        }
        ArmInstruction::SingleTransfer {
            condition,
            load,
            width,
            register,
            address,
        } => {
            let (register_offset, offset) = encode_address_offset(address.offset)?;
            let (pre, write) = encode_index(address.index, true)?;
            (
                condition,
                0x0400_0000
                    | (u32::from(register_offset) << 25)
                    | (u32::from(pre) << 24)
                    | (u32::from(address.add) << 23)
                    | (u32::from(width == TransferWidth::Byte) << 22)
                    | (u32::from(write) << 21)
                    | (u32::from(load) << 20)
                    | (u32::from(address.base.index()) << 16)
                    | (u32::from(register.index()) << 12)
                    | offset,
            )
        }
        ArmInstruction::HalfwordTransfer {
            condition,
            load,
            kind,
            register,
            address,
        } => {
            let (immediate, offset) = encode_halfword_offset(address.offset);
            let (pre, write) = encode_index(address.index, false)?;
            let kind_bits = match kind {
                HalfwordTransferKind::UnsignedHalfword => 0b01,
                HalfwordTransferKind::SignedByte => 0b10,
                HalfwordTransferKind::SignedHalfword => 0b11,
            };
            (
                condition,
                (u32::from(pre) << 24)
                    | (u32::from(address.add) << 23)
                    | (u32::from(immediate) << 22)
                    | (u32::from(write) << 21)
                    | (u32::from(load) << 20)
                    | (u32::from(address.base.index()) << 16)
                    | (u32::from(register.index()) << 12)
                    | offset
                    | (kind_bits << 5)
                    | 0x90,
            )
        }
        ArmInstruction::BlockTransfer {
            condition,
            load,
            addressing,
            write_back,
            user_registers,
            base,
            registers,
        } => {
            let (pre, add) = addressing.fields();
            (
                condition,
                0x0800_0000
                    | (u32::from(pre) << 24)
                    | (u32::from(add) << 23)
                    | (u32::from(user_registers) << 22)
                    | (u32::from(write_back) << 21)
                    | (u32::from(load) << 20)
                    | (u32::from(base.index()) << 16)
                    | u32::from(registers.bits()),
            )
        }
        ArmInstruction::Swap {
            condition,
            width,
            destination,
            source,
            base,
        } => (
            condition,
            0x0100_0090
                | (u32::from(width == TransferWidth::Byte) << 22)
                | (u32::from(base.index()) << 16)
                | (u32::from(destination.index()) << 12)
                | u32::from(source.index()),
        ),
        ArmInstruction::MoveStatusToRegister {
            condition,
            source,
            destination,
        } => (
            condition,
            0x010f_0000
                | (u32::from(source == StatusRegister::Saved) << 22)
                | (u32::from(destination.index()) << 12),
        ),
        ArmInstruction::MoveToStatusRegister {
            condition,
            destination,
            mask,
            source,
        } => {
            let source_bits = match source {
                StatusSource::Register(register) => 0x0120_f000 | u32::from(register.index()),
                StatusSource::Immediate(immediate) => {
                    0x0320_f000 | encode_rotated_immediate(immediate)
                }
            };
            (
                condition,
                source_bits
                    | (u32::from(destination == StatusRegister::Saved) << 22)
                    | (u32::from(mask.bits()) << 16),
            )
        }
        ArmInstruction::CoprocessorDataOperation {
            condition,
            coprocessor,
            opcode,
            info,
            destination,
            first,
            second,
        } => (
            condition,
            0x0e00_0000
                | (u32::from(opcode.value()) << 20)
                | (u32::from(first.value()) << 16)
                | (u32::from(destination.value()) << 12)
                | (u32::from(coprocessor.value()) << 8)
                | (u32::from(info) << 5)
                | u32::from(second.value()),
        ),
        ArmInstruction::CoprocessorRegisterTransfer {
            condition,
            load,
            coprocessor,
            opcode,
            info,
            arm_register,
            first,
            second,
        } => (
            condition,
            0x0e00_0010
                | (u32::from(opcode) << 21)
                | (u32::from(load) << 20)
                | (u32::from(first.value()) << 16)
                | (u32::from(arm_register.index()) << 12)
                | (u32::from(coprocessor.value()) << 8)
                | (u32::from(info) << 5)
                | u32::from(second.value()),
        ),
        ArmInstruction::CoprocessorDataTransfer {
            condition,
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
            (
                condition,
                0x0c00_0000
                    | (u32::from(pre) << 24)
                    | (u32::from(add) << 23)
                    | (u32::from(long) << 22)
                    | (u32::from(write) << 21)
                    | (u32::from(load) << 20)
                    | (u32::from(base.index()) << 16)
                    | (u32::from(register.value()) << 12)
                    | (u32::from(coprocessor.value()) << 8)
                    | u32::from(offset_words),
            )
        }
        ArmInstruction::SoftwareInterrupt { condition, comment } => {
            (condition, 0x0f00_0000 | comment)
        }
    };
    Ok(((condition as u32) << 28) | body)
}

fn validate_arm_instruction(instruction: &ArmInstruction) -> Result<(), EncodeError> {
    match *instruction {
        ArmInstruction::DataProcessing {
            operation,
            set_flags,
            destination,
            first,
            second,
            ..
        } => {
            if operation.is_test() && (!set_flags || destination.index() != 0) {
                return invalid("test and compare encodings require S=1 and Rd=R0");
            }
            if operation.ignores_first_register() && first.index() != 0 {
                return invalid("MOV and MVN encodings require Rn=R0");
            }
            if let Operand2::RegisterShift { amount, .. } = second
                && amount == Register::PC
            {
                return invalid("register-specified shift amount cannot use PC");
            }
        }
        ArmInstruction::Multiply {
            accumulate,
            destination,
            multiplicand,
            multiplier,
            addend,
            ..
        } => {
            if [destination, multiplicand, multiplier, addend].contains(&Register::PC) {
                return invalid("multiply registers cannot use PC");
            }
            if destination == multiplicand {
                return invalid("multiply destination cannot equal multiplicand Rm on ARM7TDMI");
            }
            if !accumulate && addend.index() != 0 {
                return invalid("MUL requires its unused addend field to be R0");
            }
        }
        ArmInstruction::MultiplyLong {
            destination_low,
            destination_high,
            multiplicand,
            multiplier,
            ..
        } => {
            if [destination_low, destination_high, multiplicand, multiplier].contains(&Register::PC)
            {
                return invalid("long multiply registers cannot use PC");
            }
            if destination_low == destination_high
                || destination_low == multiplicand
                || destination_high == multiplicand
            {
                return invalid(
                    "long multiply destination registers must be distinct from each other and Rm",
                );
            }
        }
        ArmInstruction::Swap {
            destination,
            source,
            base,
            ..
        } => {
            if [destination, source, base].contains(&Register::PC) {
                return invalid("SWP registers cannot use PC");
            }
            if base == destination || base == source {
                return invalid("SWP base cannot equal either transfer register");
            }
        }
        ArmInstruction::SingleTransfer {
            load,
            width,
            register,
            address,
            ..
        } => {
            if load && width == TransferWidth::Byte && register == Register::PC {
                return invalid("byte load destination cannot use PC");
            }
            if load && address.index != IndexMode::Offset && register == address.base {
                return invalid("load with write-back cannot use the same base and destination");
            }
        }
        ArmInstruction::HalfwordTransfer {
            load,
            kind,
            register,
            address,
            ..
        } => {
            if !load && kind != HalfwordTransferKind::UnsignedHalfword {
                return invalid("ARMv4T has no signed halfword or signed byte store");
            }
            if register == Register::PC {
                return invalid("halfword and signed-transfer register cannot use PC");
            }
            if load && address.index != IndexMode::Offset && register == address.base {
                return invalid("load with write-back cannot use the same base and destination");
            }
        }
        ArmInstruction::BlockTransfer {
            load,
            base,
            registers,
            write_back,
            ..
        } => {
            if base == Register::PC {
                return invalid("block-transfer base cannot use PC");
            }
            if write_back && registers.bits() & (1 << base.index()) != 0 {
                let lowest = registers.bits().trailing_zeros() as u8;
                if load || lowest != base.index() {
                    return invalid(
                        "write-back block transfer can include its base only as the lowest store register",
                    );
                }
            }
        }
        ArmInstruction::MoveStatusToRegister { destination, .. } if destination == Register::PC => {
            return invalid("MRS destination cannot use PC");
        }
        ArmInstruction::MoveToStatusRegister { mask, source, .. } => {
            if mask.bits() == 0 {
                return invalid("MSR status field mask cannot be empty");
            }
            if matches!(source, StatusSource::Immediate(_)) && mask != StatusMask::FLAGS {
                return invalid("ARMv4T immediate MSR can update only the flags field");
            }
            if matches!(source, StatusSource::Register(Register::PC)) {
                return invalid("MSR source cannot use PC");
            }
        }
        ArmInstruction::CoprocessorDataOperation { info, .. } if info > 7 => {
            return field_error("coprocessor information", info as u32, 7);
        }
        ArmInstruction::CoprocessorRegisterTransfer { opcode, info, .. } => {
            if opcode > 7 {
                return field_error("coprocessor register-transfer opcode", opcode as u32, 7);
            }
            if info > 7 {
                return field_error("coprocessor information", info as u32, 7);
            }
        }
        ArmInstruction::SoftwareInterrupt { comment, .. } if comment > 0x00ff_ffff => {
            return field_error("software interrupt comment", comment, 0x00ff_ffff);
        }
        _ => {}
    }
    if let ArmInstruction::SingleTransfer { address, .. } = *instruction {
        validate_mode2(address)?;
    }
    if let ArmInstruction::HalfwordTransfer { address, .. } = *instruction {
        validate_mode3(address)?;
    }
    Ok(())
}

fn validate_mode2(address: AddressingMode2) -> Result<(), EncodeError> {
    if let AddressOffset::Immediate(value) = address.offset
        && value > 0xfff
    {
        return field_error("mode 2 immediate offset", value as u32, 0xfff);
    }
    if matches!(
        address.index,
        IndexMode::PreIndexed | IndexMode::PostIndexed | IndexMode::PostIndexedUser
    ) && address.base == Register::PC
    {
        return invalid("mode 2 write-back or post-index base cannot use PC");
    }
    Ok(())
}

fn validate_mode3(address: AddressingMode3) -> Result<(), EncodeError> {
    if address.index == IndexMode::PostIndexedUser {
        return invalid("mode 3 has no user-mode post-index encoding");
    }
    if address.index != IndexMode::Offset && address.base == Register::PC {
        return invalid("mode 3 write-back or post-index base cannot use PC");
    }
    Ok(())
}

fn encode_operand2(operand: Operand2) -> Result<(bool, u32), EncodeError> {
    Ok(match operand {
        Operand2::Immediate(value) => (true, encode_rotated_immediate(value)),
        Operand2::Register { register, shift } => {
            let (kind, amount) = encode_immediate_shift(shift)?;
            (
                false,
                (u32::from(amount) << 7) | ((kind as u32) << 5) | u32::from(register.index()),
            )
        }
        Operand2::RegisterShift {
            register,
            kind,
            amount,
        } => (
            false,
            (u32::from(amount.index()) << 8)
                | ((kind as u32) << 5)
                | (1 << 4)
                | u32::from(register.index()),
        ),
    })
}

fn encode_rotated_immediate(value: RotatedImmediate) -> u32 {
    (u32::from(value.rotate_right() / 2) << 8) | u32::from(value.value())
}

fn decode_rotated_immediate(word: u32) -> RotatedImmediate {
    RotatedImmediate::from_fields(word as u8, ((word >> 8) & 0xf) as u8)
}

fn encode_immediate_shift(shift: ImmediateShift) -> Result<(ShiftKind, u8), EncodeError> {
    match shift {
        ImmediateShift::LogicalLeft(0..=31)
        | ImmediateShift::LogicalRight(1..=32)
        | ImmediateShift::ArithmeticRight(1..=32)
        | ImmediateShift::RotateRight(1..=31)
        | ImmediateShift::RotateRightExtended => Ok(shift.fields()),
        ImmediateShift::LogicalLeft(value)
        | ImmediateShift::LogicalRight(value)
        | ImmediateShift::ArithmeticRight(value)
        | ImmediateShift::RotateRight(value) => {
            field_error("immediate shift amount", value as u32, 32)
        }
    }
}

fn encode_address_offset(offset: AddressOffset) -> Result<(bool, u32), EncodeError> {
    Ok(match offset {
        AddressOffset::Immediate(value) => {
            if value > 0xfff {
                return field_error("mode 2 immediate offset", value as u32, 0xfff);
            }
            (false, u32::from(value))
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

fn encode_halfword_offset(offset: HalfwordOffset) -> (bool, u32) {
    match offset {
        HalfwordOffset::Immediate(value) => (
            true,
            (u32::from(value & 0xf0) << 4) | u32::from(value & 0x0f),
        ),
        HalfwordOffset::Register(register) => (false, u32::from(register.index())),
    }
}

fn encode_index(index: IndexMode, user_allowed: bool) -> Result<(bool, bool), EncodeError> {
    Ok(match index {
        IndexMode::Offset => (true, false),
        IndexMode::PreIndexed => (true, true),
        IndexMode::PostIndexed => (false, false),
        IndexMode::PostIndexedUser if user_allowed => (false, true),
        IndexMode::PostIndexedUser => {
            return invalid("this addressing mode has no user post-index");
        }
    })
}

fn encode_coprocessor_index(index: IndexMode) -> Result<(bool, bool), EncodeError> {
    Ok(match index {
        IndexMode::Offset => (true, false),
        IndexMode::PreIndexed => (true, true),
        IndexMode::PostIndexed => (false, true),
        IndexMode::PostIndexedUser => {
            return invalid("coprocessor transfers have no user-mode post-index");
        }
    })
}

fn status_register(saved: bool) -> StatusRegister {
    if saved {
        StatusRegister::Saved
    } else {
        StatusRegister::Current
    }
}

fn reg(value: u32) -> Register {
    Register::from_field(value & 0xf)
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

/// Decodes one little-endian Thumb instruction, consuming two or four bytes as required.
///
/// This byte-only helper does not validate an external instruction address. Use
/// [`TypedIsa::decode_one`] through [`Arm7TdmiThumb`] when address alignment is
/// part of the contract.
pub fn decode_thumb_bytes(source: &[u8]) -> Result<ThumbInstruction, DecodeError> {
    let first_bytes: [u8; 2] = source
        .get(..2)
        .ok_or(DecodeError::UnexpectedEnd {
            needed: 2,
            got: source.len(),
        })?
        .try_into()
        .expect("slice length was checked");
    let first = u16::from_le_bytes(first_bytes);
    if first >> 11 == 0b11110 {
        let second_bytes: [u8; 2] = source
            .get(2..4)
            .ok_or(DecodeError::UnexpectedEnd {
                needed: 4,
                got: source.len(),
            })?
            .try_into()
            .expect("slice length was checked");
        let second = u16::from_le_bytes(second_bytes);
        if second >> 11 != 0b11111 {
            return Err(reserved_thumb(first));
        }
        let encoded = (u32::from(first & 0x7ff) << 12) | (u32::from(second & 0x7ff) << 1);
        return Ok(ThumbInstruction::BranchLink {
            displacement: sign_extend(encoded, 23),
        });
    }
    decode_thumb_halfword(first)
}

/// Encodes one typed Thumb instruction as two or four little-endian bytes.
pub fn encode_thumb_bytes(instruction: &ThumbInstruction) -> Result<Vec<u8>, EncodeError> {
    if let ThumbInstruction::BranchLink { displacement } = *instruction {
        check_signed_aligned("Thumb BL displacement", displacement, 23, 2)?;
        let encoded = displacement as u32 & 0x007f_fffe;
        let first = 0xf000 | ((encoded >> 12) as u16 & 0x07ff);
        let second = 0xf800 | ((encoded >> 1) as u16 & 0x07ff);
        let mut bytes = Vec::with_capacity(4);
        bytes.extend_from_slice(&first.to_le_bytes());
        bytes.extend_from_slice(&second.to_le_bytes());
        return Ok(bytes);
    }
    Ok(encode_thumb_halfword(instruction)?.to_le_bytes().to_vec())
}

fn decode_thumb_halfword(word: u16) -> Result<ThumbInstruction, DecodeError> {
    let instruction = match word >> 13 {
        0b000 if word & 0x1800 != 0x1800 => {
            let kind = match (word >> 11) & 0x3 {
                0 => ShiftKind::LogicalLeft,
                1 => ShiftKind::LogicalRight,
                2 => ShiftKind::ArithmeticRight,
                _ => unreachable!(),
            };
            let encoded = ((word >> 6) & 0x1f) as u8;
            let amount = if encoded == 0 && kind != ShiftKind::LogicalLeft {
                32
            } else {
                encoded
            };
            ThumbInstruction::ShiftImmediate {
                kind,
                destination: low(word),
                source: low(word >> 3),
                amount,
            }
        }
        0b000 => ThumbInstruction::AddSubtract {
            subtract: word & (1 << 9) != 0,
            destination: low(word),
            source: low(word >> 3),
            operand: if word & (1 << 10) != 0 {
                ThumbAddOperand::Immediate(((word >> 6) & 7) as u8)
            } else {
                ThumbAddOperand::Register(low(word >> 6))
            },
        },
        0b001 => ThumbInstruction::Immediate {
            operation: ThumbImmediateOperation::from_field(((word >> 11) & 3) as u8),
            register: low(word >> 8),
            immediate: word as u8,
        },
        0b010 if word & 0xfc00 == 0x4000 => ThumbInstruction::Alu {
            operation: ThumbAluOperation::from_field(((word >> 6) & 0xf) as u8),
            destination: low(word),
            source: low(word >> 3),
        },
        0b010 if word & 0xfc00 == 0x4400 => decode_thumb_high(word)?,
        0b010 if word & 0xf800 == 0x4800 => ThumbInstruction::PcRelativeLoad {
            destination: low(word >> 8),
            offset: (word & 0xff) * 4,
        },
        0b010 if word & 0xf000 == 0x5000 => ThumbInstruction::RegisterTransfer {
            operation: decode_thumb_transfer_operation(word),
            destination: low(word),
            base: low(word >> 3),
            offset: low(word >> 6),
        },
        0b011 => {
            let width = if word & (1 << 12) != 0 {
                ThumbTransferWidth::Byte
            } else {
                ThumbTransferWidth::Word
            };
            let scale = if width == ThumbTransferWidth::Word {
                4
            } else {
                1
            };
            ThumbInstruction::ImmediateTransfer {
                load: word & (1 << 11) != 0,
                width,
                destination: low(word),
                base: low(word >> 3),
                offset: (((word >> 6) & 0x1f) as u8) * scale,
            }
        }
        0b100 if word & 0xf000 == 0x8000 => ThumbInstruction::ImmediateTransfer {
            load: word & (1 << 11) != 0,
            width: ThumbTransferWidth::Halfword,
            destination: low(word),
            base: low(word >> 3),
            offset: (((word >> 6) & 0x1f) as u8) * 2,
        },
        0b100 => ThumbInstruction::SpRelativeTransfer {
            load: word & (1 << 11) != 0,
            register: low(word >> 8),
            offset: (word & 0xff) * 4,
        },
        0b101 if word & 0xf000 == 0xa000 => ThumbInstruction::LoadAddress {
            from_sp: word & (1 << 11) != 0,
            destination: low(word >> 8),
            offset: (word & 0xff) * 4,
        },
        0b101 if word & 0xff00 == 0xb000 => ThumbInstruction::AddStackPointer {
            subtract: word & (1 << 7) != 0,
            offset: (word & 0x7f) * 4,
        },
        0b101 if word & 0xf600 == 0xb400 => {
            let registers = ThumbRegisterList::from_bits(word as u8);
            let special = word & (1 << 8) != 0;
            if registers.bits() == 0 && !special {
                return Err(unpredictable_thumb(word));
            }
            if word & (1 << 11) != 0 {
                ThumbInstruction::Pop {
                    registers,
                    program_counter: special,
                }
            } else {
                ThumbInstruction::Push {
                    registers,
                    link: special,
                }
            }
        }
        0b101 => return Err(reserved_thumb(word)),
        0b110 if word & 0xf000 == 0xc000 => {
            let registers = ThumbRegisterList::from_bits(word as u8);
            if registers.bits() == 0 {
                return Err(unpredictable_thumb(word));
            }
            ThumbInstruction::MultipleTransfer {
                load: word & (1 << 11) != 0,
                base: low(word >> 8),
                registers,
            }
        }
        0b110 => {
            let condition_field = ((word >> 8) & 0xf) as u8;
            if condition_field == 0xf {
                ThumbInstruction::SoftwareInterrupt {
                    comment: word as u8,
                }
            } else {
                let condition =
                    Condition::from_field(condition_field).ok_or_else(|| reserved_thumb(word))?;
                if condition == Condition::Always {
                    return Err(reserved_thumb(word));
                }
                ThumbInstruction::ConditionalBranch {
                    condition,
                    displacement: sign_extend(u32::from(word as u8) << 1, 9) as i16,
                }
            }
        }
        0b111 if word & 0xf800 == 0xe000 => ThumbInstruction::UnconditionalBranch {
            displacement: sign_extend(u32::from(word & 0x7ff) << 1, 12) as i16,
        },
        0b111 => return Err(reserved_thumb(word)),
        _ => unreachable!(),
    };
    Ok(instruction)
}

fn decode_thumb_high(word: u16) -> Result<ThumbInstruction, DecodeError> {
    let operation = ((word >> 8) & 0x3) as u8;
    let destination = Register::from_field(u32::from(word & 7) | u32::from((word >> 4) & 8));
    let source = Register::from_field(u32::from((word >> 3) & 7) | u32::from((word >> 3) & 8));
    if operation == 3 {
        if destination.index() != 0 {
            return Err(unpredictable_thumb(word));
        }
        return Ok(ThumbInstruction::BranchExchange { source });
    }
    if destination.index() < 8 && source.index() < 8 {
        return Err(unpredictable_thumb(word));
    }
    Ok(ThumbInstruction::HighRegister {
        operation: ThumbHighOperation::from_field(operation).expect("BX was handled"),
        destination,
        source,
    })
}

fn decode_thumb_transfer_operation(word: u16) -> ThumbTransferOperation {
    match ((word >> 9) & 0x7) as u8 {
        0 => ThumbTransferOperation::StoreWord,
        1 => ThumbTransferOperation::StoreHalfword,
        2 => ThumbTransferOperation::StoreByte,
        3 => ThumbTransferOperation::LoadSignedByte,
        4 => ThumbTransferOperation::LoadWord,
        5 => ThumbTransferOperation::LoadHalfword,
        6 => ThumbTransferOperation::LoadByte,
        7 => ThumbTransferOperation::LoadSignedHalfword,
        _ => unreachable!(),
    }
}

fn encode_thumb_halfword(instruction: &ThumbInstruction) -> Result<u16, EncodeError> {
    Ok(match *instruction {
        ThumbInstruction::ShiftImmediate {
            kind,
            destination,
            source,
            amount,
        } => {
            let (operation, encoded) = match (kind, amount) {
                (ShiftKind::LogicalLeft, 0..=31) => (0_u16, amount),
                (ShiftKind::LogicalRight, 1..=32) => (1, amount & 31),
                (ShiftKind::ArithmeticRight, 1..=32) => (2, amount & 31),
                _ => return invalid("Thumb immediate shifts accept LSL 0..31 or LSR/ASR 1..32"),
            };
            (operation << 11)
                | (u16::from(encoded) << 6)
                | (u16::from(source.index()) << 3)
                | u16::from(destination.index())
        }
        ThumbInstruction::AddSubtract {
            subtract,
            destination,
            source,
            operand,
        } => {
            let (immediate, value) = match operand {
                ThumbAddOperand::Register(register) => (false, register.index()),
                ThumbAddOperand::Immediate(value) if value < 8 => (true, value),
                ThumbAddOperand::Immediate(value) => {
                    return field_error("Thumb add/subtract immediate", value as u32, 7);
                }
            };
            0x1800
                | (u16::from(immediate) << 10)
                | (u16::from(subtract) << 9)
                | (u16::from(value) << 6)
                | (u16::from(source.index()) << 3)
                | u16::from(destination.index())
        }
        ThumbInstruction::Immediate {
            operation,
            register,
            immediate,
        } => {
            0x2000
                | ((operation as u16) << 11)
                | (u16::from(register.index()) << 8)
                | u16::from(immediate)
        }
        ThumbInstruction::Alu {
            operation,
            destination,
            source,
        } => {
            0x4000
                | ((operation as u16) << 6)
                | (u16::from(source.index()) << 3)
                | u16::from(destination.index())
        }
        ThumbInstruction::HighRegister {
            operation,
            destination,
            source,
        } => {
            if destination.index() < 8 && source.index() < 8 {
                return invalid(
                    "Thumb high-register operation requires at least one high register",
                );
            }
            0x4400
                | ((operation as u16) << 8)
                | (u16::from(destination.index() & 8) << 4)
                | (u16::from(source.index() & 8) << 3)
                | (u16::from(source.index() & 7) << 3)
                | u16::from(destination.index() & 7)
        }
        ThumbInstruction::BranchExchange { source } => {
            0x4700 | (u16::from(source.index() & 8) << 3) | (u16::from(source.index() & 7) << 3)
        }
        ThumbInstruction::PcRelativeLoad {
            destination,
            offset,
        } => {
            check_unsigned_scaled("Thumb PC-relative load offset", offset, 0x3fc, 4)?;
            0x4800 | (u16::from(destination.index()) << 8) | (offset >> 2)
        }
        ThumbInstruction::RegisterTransfer {
            operation,
            destination,
            base,
            offset,
        } => {
            0x5000
                | (u16::from(thumb_transfer_operation_field(operation)) << 9)
                | (u16::from(offset.index()) << 6)
                | (u16::from(base.index()) << 3)
                | u16::from(destination.index())
        }
        ThumbInstruction::ImmediateTransfer {
            load,
            width,
            destination,
            base,
            offset,
        } => {
            let (base_word, scale, maximum) = match width {
                ThumbTransferWidth::Word => (0x6000, 4, 124),
                ThumbTransferWidth::Byte => (0x7000, 1, 31),
                ThumbTransferWidth::Halfword => (0x8000, 2, 62),
            };
            check_unsigned_scaled(
                "Thumb immediate transfer offset",
                u16::from(offset),
                maximum,
                scale,
            )?;
            base_word
                | (u16::from(load) << 11)
                | (u16::from(offset / scale) << 6)
                | (u16::from(base.index()) << 3)
                | u16::from(destination.index())
        }
        ThumbInstruction::SpRelativeTransfer {
            load,
            register,
            offset,
        } => {
            check_unsigned_scaled("Thumb SP-relative offset", offset, 0x3fc, 4)?;
            0x9000 | (u16::from(load) << 11) | (u16::from(register.index()) << 8) | (offset >> 2)
        }
        ThumbInstruction::LoadAddress {
            from_sp,
            destination,
            offset,
        } => {
            check_unsigned_scaled("Thumb address offset", offset, 0x3fc, 4)?;
            0xa000
                | (u16::from(from_sp) << 11)
                | (u16::from(destination.index()) << 8)
                | (offset >> 2)
        }
        ThumbInstruction::AddStackPointer { subtract, offset } => {
            check_unsigned_scaled("Thumb stack-pointer offset", offset, 0x1fc, 4)?;
            0xb000 | (u16::from(subtract) << 7) | (offset >> 2)
        }
        ThumbInstruction::Push { registers, link } => {
            if registers.bits() == 0 && !link {
                return invalid("Thumb PUSH register list cannot be empty without LR");
            }
            0xb400 | (u16::from(link) << 8) | u16::from(registers.bits())
        }
        ThumbInstruction::Pop {
            registers,
            program_counter,
        } => {
            if registers.bits() == 0 && !program_counter {
                return invalid("Thumb POP register list cannot be empty without PC");
            }
            0xbc00 | (u16::from(program_counter) << 8) | u16::from(registers.bits())
        }
        ThumbInstruction::MultipleTransfer {
            load,
            base,
            registers,
        } => {
            if registers.bits() == 0 {
                return invalid("Thumb multiple-transfer register list cannot be empty");
            }
            0xc000
                | (u16::from(load) << 11)
                | (u16::from(base.index()) << 8)
                | u16::from(registers.bits())
        }
        ThumbInstruction::ConditionalBranch {
            condition,
            displacement,
        } => {
            if condition == Condition::Always {
                return invalid("Thumb conditional branch has no Always selector");
            }
            check_signed_aligned("Thumb conditional displacement", displacement.into(), 9, 2)?;
            0xd000 | ((condition as u16) << 8) | ((displacement as u16 >> 1) & 0xff)
        }
        ThumbInstruction::UnconditionalBranch { displacement } => {
            check_signed_aligned("Thumb branch displacement", displacement.into(), 12, 2)?;
            0xe000 | ((displacement as u16 >> 1) & 0x7ff)
        }
        ThumbInstruction::SoftwareInterrupt { comment } => 0xdf00 | u16::from(comment),
        ThumbInstruction::BranchLink { .. } => unreachable!("BL is encoded as two halfwords"),
    })
}

fn thumb_transfer_operation_field(operation: ThumbTransferOperation) -> u8 {
    match operation {
        ThumbTransferOperation::StoreWord => 0,
        ThumbTransferOperation::StoreHalfword => 1,
        ThumbTransferOperation::StoreByte => 2,
        ThumbTransferOperation::LoadSignedByte => 3,
        ThumbTransferOperation::LoadWord => 4,
        ThumbTransferOperation::LoadHalfword => 5,
        ThumbTransferOperation::LoadByte => 6,
        ThumbTransferOperation::LoadSignedHalfword => 7,
    }
}

fn low(value: u16) -> LowRegister {
    LowRegister::from_field(value & 7)
}

fn reserved_thumb(word: u16) -> DecodeError {
    DecodeError::ReservedEncoding {
        width: 16,
        value: u32::from(word),
    }
}

fn unpredictable_thumb(word: u16) -> DecodeError {
    DecodeError::UnpredictableEncoding {
        width: 16,
        value: u32::from(word),
    }
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

fn check_unsigned_scaled(
    field: &'static str,
    value: u16,
    maximum: u16,
    alignment: u8,
) -> Result<(), EncodeError> {
    if value > maximum {
        return field_error(field, u32::from(value), u32::from(maximum));
    }
    if value % u16::from(alignment) != 0 {
        return Err(EncodeError::UnalignedField {
            field,
            value: i32::from(value),
            alignment,
        });
    }
    Ok(())
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
