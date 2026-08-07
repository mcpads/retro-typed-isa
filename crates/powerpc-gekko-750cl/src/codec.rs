use thiserror::Error;
use typed_isa_core::{
    DecodedInstruction as ProtocolDecodedInstruction, InstructionBytes, TypedIsa,
};

use crate::instruction::{
    BitIndex, BranchOptions, ConditionRegisterBit, ConditionRegisterField, ConditionRegisterMask,
    Field, FloatingRegister, FpscrFieldMask, FpscrImmediate, GeneralRegister, Instruction,
    InstructionForm, InstructionModifiers, Operand, OperandKind, PairedSingleWidth,
    QuantizationRegister, SegmentRegister, SpecialPurposeRegister, StringByteCount,
    TimeBaseRegister, TrapCondition,
};

/// IBM PowerPC 750CL protocol profile.
///
/// This profile admits the complete 750CL SPR map, including registers not
/// implemented by Gekko.
#[derive(Debug, Clone, Copy)]
pub struct PowerPc750Cl;

/// IBM Gekko protocol profile.
///
/// Instruction forms match [`PowerPc750Cl`], while unavailable 750CL-only
/// special-purpose registers are rejected.
#[derive(Debug, Clone, Copy)]
pub struct PowerPcGekko;

/// Failure to decode one fixed-width instruction.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DecodeError {
    /// The supplied instruction address is not four-byte aligned.
    #[error("PowerPC Gekko/750CL instruction address {address:#018x} is not four-byte aligned")]
    UnalignedAddress {
        /// Supplied instruction address.
        address: u64,
    },
    /// The supplied instruction address does not fit the 32-bit address space.
    #[error("PowerPC Gekko/750CL address {address:#018x} exceeds the 32-bit address space")]
    AddressOutOfRange {
        /// Supplied instruction address.
        address: u64,
    },
    /// Fewer than four source bytes were available.
    #[error("truncated PowerPC Gekko/750CL instruction: needed 4 bytes, got {got}")]
    UnexpectedEnd {
        /// Number of bytes available from the requested position.
        got: usize,
    },
    /// The word is reserved or not part of the declared instruction catalog.
    #[error("reserved or out-of-profile PowerPC Gekko/750CL instruction word {word:#010x}")]
    ReservedWord {
        /// Rejected instruction word.
        word: u32,
    },
    /// A valid 750CL instruction names an SPR that Gekko does not provide.
    #[error("special-purpose register {register:?} is not available on IBM Gekko")]
    GekkoUnavailableSpecialPurposeRegister {
        /// Register unavailable in the Gekko profile.
        register: SpecialPurposeRegister,
    },
}

/// Failure to construct or encode one instruction.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EncodeError {
    /// The supplied instruction address is not four-byte aligned.
    #[error("PowerPC Gekko/750CL instruction address {address:#018x} is not four-byte aligned")]
    UnalignedAddress {
        /// Supplied instruction address.
        address: u64,
    },
    /// The supplied instruction address does not fit the 32-bit address space.
    #[error("PowerPC Gekko/750CL address {address:#018x} exceeds the 32-bit address space")]
    AddressOutOfRange {
        /// Supplied instruction address.
        address: u64,
    },
    /// The form received the wrong number of operands.
    #[error("instruction has {actual} operands; {expected} required")]
    OperandCount {
        /// Number of supplied operands.
        actual: usize,
        /// Number of operands required by the form.
        expected: usize,
    },
    /// An operand's typed variant does not match the form's field.
    #[error("operand {index} has kind {actual:?}; expected {expected:?}")]
    OperandKind {
        /// Zero-based operand position.
        index: usize,
        /// Operand kind required by the form.
        expected: OperandKind,
        /// Operand kind that was supplied.
        actual: OperandKind,
    },
    /// One or more optional bits are unavailable on the chosen form.
    #[error("modifiers {modifiers:?} are not valid for {form:?}")]
    InvalidModifiers {
        /// Instruction form being constructed.
        form: InstructionForm,
        /// Rejected modifier set.
        modifiers: InstructionModifiers,
    },
    /// The instruction attempts a read-only or write-only SPR in the wrong direction.
    #[error("{form:?} cannot access special-purpose register {register:?} in that direction")]
    InvalidSpecialPurposeAccess {
        /// Instruction form performing the access.
        form: InstructionForm,
        /// Register whose access direction is invalid.
        register: SpecialPurposeRegister,
    },
    /// An update-form load or store uses general register zero as its base.
    #[error("{form:?} requires a nonzero update base register")]
    ZeroUpdateBase {
        /// Update-form instruction.
        form: InstructionForm,
    },
    /// An integer update-form load aliases its destination and base registers.
    #[error("{form:?} update base {register:?} aliases its integer load destination")]
    UpdateBaseAliasesDestination {
        /// Update-form load instruction.
        form: InstructionForm,
        /// Aliased destination and base register.
        register: GeneralRegister,
    },
    /// A multiple or string load would overwrite a register used for addressing.
    #[error("{form:?} loads through address register {register:?}")]
    LoadRangeIncludesAddress {
        /// Multiple or string load instruction.
        form: InstructionForm,
        /// Address register included in the destination range.
        register: GeneralRegister,
    },
    /// An indexed string load uses its first destination as an address register.
    #[error("Lswx destination {register:?} aliases an address register")]
    StringLoadDestinationAliasesAddress {
        /// Destination register that aliases a base or index register.
        register: GeneralRegister,
    },
    /// A `bcctr` BO field incorrectly requests count-register decrement and testing.
    #[error("Bcctr branch options {options:?} request a count-register decrement")]
    CountRegisterBranchDecrementsCount {
        /// Rejected branch-options field.
        options: BranchOptions,
    },
    /// A branch target is not four-byte aligned.
    #[error("control-flow target {target:#010x} is not four-byte aligned")]
    UnalignedTarget {
        /// Rejected absolute target address.
        target: u32,
    },
    /// A branch displacement cannot represent the requested target.
    #[error("branch at {address:#010x} cannot reach {target:#010x}")]
    BranchOutOfRange {
        /// Address of the branch instruction.
        address: u32,
        /// Requested absolute target address.
        target: u32,
    },
    /// A signed immediate does not fit its instruction field.
    #[error("signed immediate {value} does not fit the {bits}-bit field")]
    ImmediateOutOfRange {
        /// Rejected immediate value.
        value: i16,
        /// Width of the signed field.
        bits: u8,
    },
    /// An instruction names a 750CL-only SPR under the Gekko profile.
    #[error("special-purpose register {register:?} is not available on IBM Gekko")]
    GekkoUnavailableSpecialPurposeRegister {
        /// Register unavailable in the Gekko profile.
        register: SpecialPurposeRegister,
    },
}

fn decode_common_word(word: u32, address: u64) -> Result<Instruction, DecodeError> {
    let address = checked_address_decode(address)?;
    let form = InstructionForm::detect(word).ok_or(DecodeError::ReservedWord { word })?;
    let support = form.modifier_support();
    let modifiers = InstructionModifiers {
        record: support.record && word & 1 != 0,
        overflow: support.overflow && word & (1 << 10) != 0,
        link: support.link && word & 1 != 0,
        absolute: support.absolute && word & 2 != 0,
    };
    let definition = form.definition();
    let operands = definition
        .fields
        .iter()
        .copied()
        .map(|field| decode_field(field, word, address, modifiers))
        .collect::<Result<Vec<_>, _>>()?;
    Instruction::with_modifiers(form, modifiers, operands)
        .map_err(|_| DecodeError::ReservedWord { word })
}

fn decode_common_bytes(source: &[u8], address: u64) -> Result<Instruction, DecodeError> {
    let bytes = source
        .get(..4)
        .ok_or(DecodeError::UnexpectedEnd { got: source.len() })?;
    decode_common_word(
        u32::from_be_bytes(bytes.try_into().expect("four-byte slice")),
        address,
    )
}

/// Decodes one PowerPC 750CL instruction word at `address`.
pub fn decode_750cl_word(word: u32, address: u64) -> Result<Instruction, DecodeError> {
    decode_common_word(word, address)
}

/// Decodes the first four big-endian bytes as a PowerPC 750CL instruction.
pub fn decode_750cl_bytes(source: &[u8], address: u64) -> Result<Instruction, DecodeError> {
    decode_common_bytes(source, address)
}

/// Decodes one IBM Gekko instruction word at `address`.
///
/// This additionally rejects SPR selectors that exist only on the 750CL.
pub fn decode_gekko_word(word: u32, address: u64) -> Result<Instruction, DecodeError> {
    let instruction = decode_common_word(word, address)?;
    validate_gekko_decode(&instruction)?;
    Ok(instruction)
}

/// Decodes the first four big-endian bytes as an IBM Gekko instruction.
pub fn decode_gekko_bytes(source: &[u8], address: u64) -> Result<Instruction, DecodeError> {
    let bytes = source
        .get(..4)
        .ok_or(DecodeError::UnexpectedEnd { got: source.len() })?;
    decode_gekko_word(
        u32::from_be_bytes(bytes.try_into().expect("four-byte slice")),
        address,
    )
}

fn encode_common_word(instruction: &Instruction, address: u64) -> Result<u32, EncodeError> {
    let address = checked_address_encode(address)?;
    let definition = instruction.form().definition();
    let mut word = definition.bits;
    for (field, operand) in definition
        .fields
        .iter()
        .copied()
        .zip(instruction.operands().iter().copied())
    {
        word |= encode_field(field, operand, address, instruction.modifiers())?;
    }
    let modifiers = instruction.modifiers();
    if modifiers.record || modifiers.link {
        word |= 1;
    }
    if modifiers.absolute {
        word |= 2;
    }
    if modifiers.overflow {
        word |= 1 << 10;
    }
    debug_assert_eq!(word & definition.mask, definition.bits);
    Ok(word)
}

/// Encodes a validated instruction as a PowerPC 750CL instruction word.
pub fn encode_750cl_word(instruction: &Instruction, address: u64) -> Result<u32, EncodeError> {
    encode_common_word(instruction, address)
}

/// Encodes a validated instruction as four big-endian PowerPC 750CL bytes.
pub fn encode_750cl_bytes(instruction: &Instruction, address: u64) -> Result<[u8; 4], EncodeError> {
    Ok(encode_common_word(instruction, address)?.to_be_bytes())
}

/// Encodes a validated instruction as an IBM Gekko instruction word.
///
/// This additionally rejects SPR selectors that exist only on the 750CL.
pub fn encode_gekko_word(instruction: &Instruction, address: u64) -> Result<u32, EncodeError> {
    validate_gekko_encode(instruction)?;
    encode_common_word(instruction, address)
}

/// Encodes a validated instruction as four big-endian IBM Gekko bytes.
pub fn encode_gekko_bytes(instruction: &Instruction, address: u64) -> Result<[u8; 4], EncodeError> {
    Ok(encode_gekko_word(instruction, address)?.to_be_bytes())
}

fn gekko_unavailable_register(instruction: &Instruction) -> Option<SpecialPurposeRegister> {
    instruction
        .operands()
        .iter()
        .find_map(|operand| match operand {
            Operand::SpecialPurposeRegister(register) if !register.is_available_on_gekko() => {
                Some(*register)
            }
            _ => None,
        })
}

fn validate_gekko_decode(instruction: &Instruction) -> Result<(), DecodeError> {
    if let Some(register) = gekko_unavailable_register(instruction) {
        return Err(DecodeError::GekkoUnavailableSpecialPurposeRegister { register });
    }
    Ok(())
}

fn validate_gekko_encode(instruction: &Instruction) -> Result<(), EncodeError> {
    if let Some(register) = gekko_unavailable_register(instruction) {
        return Err(EncodeError::GekkoUnavailableSpecialPurposeRegister { register });
    }
    Ok(())
}

fn checked_address_decode(address: u64) -> Result<u32, DecodeError> {
    if address & 3 != 0 {
        return Err(DecodeError::UnalignedAddress { address });
    }
    u32::try_from(address).map_err(|_| DecodeError::AddressOutOfRange { address })
}

fn checked_address_encode(address: u64) -> Result<u32, EncodeError> {
    if address & 3 != 0 {
        return Err(EncodeError::UnalignedAddress { address });
    }
    u32::try_from(address).map_err(|_| EncodeError::AddressOutOfRange { address })
}

fn field(word: u32, shift: u8, mask: u32) -> u32 {
    (word >> shift) & mask
}

fn split_selector(word: u32) -> u16 {
    (field(word, 16, 0x1f) | (field(word, 11, 0x1f) << 5)) as u16
}

fn decode_field(
    field_kind: Field,
    word: u32,
    address: u32,
    modifiers: InstructionModifiers,
) -> Result<Operand, DecodeError> {
    let operand = match field_kind {
        Field::Gpr21 => {
            Operand::GeneralRegister(GeneralRegister::from_field(field_u8(word, 21, 0x1f)))
        }
        Field::Gpr16 => {
            Operand::GeneralRegister(GeneralRegister::from_field(field_u8(word, 16, 0x1f)))
        }
        Field::Gpr11 => {
            Operand::GeneralRegister(GeneralRegister::from_field(field_u8(word, 11, 0x1f)))
        }
        Field::Fpr21 => {
            Operand::FloatingRegister(FloatingRegister::from_field(field_u8(word, 21, 0x1f)))
        }
        Field::Fpr16 => {
            Operand::FloatingRegister(FloatingRegister::from_field(field_u8(word, 16, 0x1f)))
        }
        Field::Fpr11 => {
            Operand::FloatingRegister(FloatingRegister::from_field(field_u8(word, 11, 0x1f)))
        }
        Field::Fpr6 => {
            Operand::FloatingRegister(FloatingRegister::from_field(field_u8(word, 6, 0x1f)))
        }
        Field::CrField23 => Operand::ConditionRegisterField(ConditionRegisterField::from_field(
            field_u8(word, 23, 7),
        )),
        Field::CrField18 => Operand::ConditionRegisterField(ConditionRegisterField::from_field(
            field_u8(word, 18, 7),
        )),
        Field::CrBit21 => Operand::ConditionRegisterBit(ConditionRegisterBit::from_field(
            field_u8(word, 21, 0x1f),
        )),
        Field::CrBit16 | Field::ConditionBit16 => Operand::ConditionRegisterBit(
            ConditionRegisterBit::from_field(field_u8(word, 16, 0x1f)),
        ),
        Field::CrBit11 => Operand::ConditionRegisterBit(ConditionRegisterBit::from_field(
            field_u8(word, 11, 0x1f),
        )),
        Field::Signed16 => Operand::SignedImmediate(word as i16),
        Field::Unsigned16 => Operand::UnsignedImmediate(word as u16),
        Field::MemoryOffset16 => Operand::MemoryOffset(word as i16),
        Field::PairedOffset12 => {
            Operand::SignedImmediate((((word & 0x0fff) << 20) as i32 >> 20) as i16)
        }
        Field::BranchOptions21 => {
            Operand::BranchOptions(BranchOptions::from_field(field_u8(word, 21, 0x1f)))
        }
        Field::BranchTarget14 => {
            Operand::BranchTarget(branch_target(word, address, modifiers.absolute, 14))
        }
        Field::BranchTarget24 => {
            Operand::BranchTarget(branch_target(word, address, modifiers.absolute, 24))
        }
        Field::ShiftAmount11 => {
            Operand::ShiftOrMaskBit(BitIndex::from_field(field_u8(word, 11, 0x1f)))
        }
        Field::MaskBit6 => Operand::ShiftOrMaskBit(BitIndex::from_field(field_u8(word, 6, 0x1f))),
        Field::MaskBit1 => Operand::ShiftOrMaskBit(BitIndex::from_field(field_u8(word, 1, 0x1f))),
        Field::StringByteCount11 => {
            Operand::StringByteCount(StringByteCount::from_field(field_u8(word, 11, 0x1f)))
        }
        Field::SpecialRegisterSplit => {
            let selector = split_selector(word);
            let register = SpecialPurposeRegister::from_field(selector)
                .ok_or(DecodeError::ReservedWord { word })?;
            Operand::SpecialPurposeRegister(register)
        }
        Field::TimeBaseRegisterSplit => {
            let selector = split_selector(word);
            let register =
                TimeBaseRegister::from_field(selector).ok_or(DecodeError::ReservedWord { word })?;
            Operand::TimeBaseRegister(register)
        }
        Field::SegmentRegister16 => {
            Operand::SegmentRegister(SegmentRegister::from_field(field_u8(word, 16, 0x0f)))
        }
        Field::ConditionMask12 => {
            Operand::ConditionRegisterMask(ConditionRegisterMask::new(field_u8(word, 12, 0xff)))
        }
        Field::FpscrMask17 => {
            Operand::FpscrFieldMask(FpscrFieldMask::new(field_u8(word, 17, 0xff)))
        }
        Field::FpscrImmediate12 => {
            Operand::FpscrImmediate(FpscrImmediate::from_field(field_u8(word, 12, 0x0f)))
        }
        Field::TrapCondition21 => {
            Operand::TrapCondition(TrapCondition::from_field(field_u8(word, 21, 0x1f)))
        }
        Field::PairedWidth15 => {
            Operand::PairedSingleWidth(PairedSingleWidth::from_field(field_u8(word, 15, 1)))
        }
        Field::QuantizationRegister12 => {
            Operand::QuantizationRegister(QuantizationRegister::from_field(field_u8(word, 12, 7)))
        }
        Field::PairedWidth10 => {
            Operand::PairedSingleWidth(PairedSingleWidth::from_field(field_u8(word, 10, 1)))
        }
        Field::QuantizationRegister7 => {
            Operand::QuantizationRegister(QuantizationRegister::from_field(field_u8(word, 7, 7)))
        }
    };
    Ok(operand)
}

fn field_u8(word: u32, shift: u8, mask: u32) -> u8 {
    u8::try_from(field(word, shift, mask)).expect("field fits in u8")
}

fn branch_target(word: u32, address: u32, absolute: bool, field_bits: u8) -> u32 {
    let displacement = if field_bits == 14 {
        i32::from(word as i16 & !3)
    } else {
        ((word & 0x03ff_fffc) << 6) as i32 >> 6
    };
    if absolute {
        displacement as u32
    } else {
        address.wrapping_add_signed(displacement)
    }
}

fn encode_field(
    field_kind: Field,
    operand: Operand,
    address: u32,
    modifiers: InstructionModifiers,
) -> Result<u32, EncodeError> {
    let encoded = match (field_kind, operand) {
        (Field::Gpr21, Operand::GeneralRegister(register)) => u32::from(register.value()) << 21,
        (Field::Gpr16, Operand::GeneralRegister(register)) => u32::from(register.value()) << 16,
        (Field::Gpr11, Operand::GeneralRegister(register)) => u32::from(register.value()) << 11,
        (Field::Fpr21, Operand::FloatingRegister(register)) => u32::from(register.value()) << 21,
        (Field::Fpr16, Operand::FloatingRegister(register)) => u32::from(register.value()) << 16,
        (Field::Fpr11, Operand::FloatingRegister(register)) => u32::from(register.value()) << 11,
        (Field::Fpr6, Operand::FloatingRegister(register)) => u32::from(register.value()) << 6,
        (Field::CrField23, Operand::ConditionRegisterField(register)) => {
            u32::from(register.value()) << 23
        }
        (Field::CrField18, Operand::ConditionRegisterField(register)) => {
            u32::from(register.value()) << 18
        }
        (Field::CrBit21, Operand::ConditionRegisterBit(bit)) => u32::from(bit.value()) << 21,
        (Field::CrBit16 | Field::ConditionBit16, Operand::ConditionRegisterBit(bit)) => {
            u32::from(bit.value()) << 16
        }
        (Field::CrBit11, Operand::ConditionRegisterBit(bit)) => u32::from(bit.value()) << 11,
        (Field::Signed16, Operand::SignedImmediate(value)) => u32::from(value as u16),
        (Field::Unsigned16, Operand::UnsignedImmediate(value)) => u32::from(value),
        (Field::MemoryOffset16, Operand::MemoryOffset(value)) => u32::from(value as u16),
        (Field::PairedOffset12, Operand::SignedImmediate(value)) => {
            if !(-2048..=2047).contains(&value) {
                return Err(EncodeError::ImmediateOutOfRange { value, bits: 12 });
            }
            u32::from(value as u16) & 0x0fff
        }
        (Field::BranchOptions21, Operand::BranchOptions(options)) => {
            u32::from(options.value()) << 21
        }
        (Field::BranchTarget14, Operand::BranchTarget(target)) => {
            encode_branch_target(address, target, modifiers.absolute, 14)?
        }
        (Field::BranchTarget24, Operand::BranchTarget(target)) => {
            encode_branch_target(address, target, modifiers.absolute, 24)?
        }
        (Field::ShiftAmount11, Operand::ShiftOrMaskBit(bit)) => u32::from(bit.value()) << 11,
        (Field::MaskBit6, Operand::ShiftOrMaskBit(bit)) => u32::from(bit.value()) << 6,
        (Field::MaskBit1, Operand::ShiftOrMaskBit(bit)) => u32::from(bit.value()) << 1,
        (Field::StringByteCount11, Operand::StringByteCount(count)) => {
            u32::from(count.field()) << 11
        }
        (Field::SpecialRegisterSplit, Operand::SpecialPurposeRegister(register)) => {
            encode_split_selector(register.selector())
        }
        (Field::TimeBaseRegisterSplit, Operand::TimeBaseRegister(register)) => {
            encode_split_selector(register.selector())
        }
        (Field::SegmentRegister16, Operand::SegmentRegister(register)) => {
            u32::from(register.value()) << 16
        }
        (Field::ConditionMask12, Operand::ConditionRegisterMask(mask)) => {
            u32::from(mask.bits()) << 12
        }
        (Field::FpscrMask17, Operand::FpscrFieldMask(mask)) => u32::from(mask.bits()) << 17,
        (Field::FpscrImmediate12, Operand::FpscrImmediate(value)) => u32::from(value.value()) << 12,
        (Field::TrapCondition21, Operand::TrapCondition(condition)) => {
            u32::from(condition.value()) << 21
        }
        (Field::PairedWidth15, Operand::PairedSingleWidth(width)) => u32::from(width.field()) << 15,
        (Field::QuantizationRegister12, Operand::QuantizationRegister(register)) => {
            u32::from(register.value()) << 12
        }
        (Field::PairedWidth10, Operand::PairedSingleWidth(width)) => u32::from(width.field()) << 10,
        (Field::QuantizationRegister7, Operand::QuantizationRegister(register)) => {
            u32::from(register.value()) << 7
        }
        _ => unreachable!("validated instruction operand kind"),
    };
    Ok(encoded)
}

fn encode_split_selector(selector: u16) -> u32 {
    (u32::from(selector & 0x1f) << 16) | (u32::from(selector & 0x03e0) << 6)
}

fn encode_branch_target(
    address: u32,
    target: u32,
    absolute: bool,
    field_bits: u8,
) -> Result<u32, EncodeError> {
    if target & 3 != 0 {
        return Err(EncodeError::UnalignedTarget { target });
    }
    let displacement = if absolute {
        target as i32
    } else {
        target.wrapping_sub(address) as i32
    };
    let (minimum, maximum, mask) = if field_bits == 14 {
        (-32_768, 32_764, 0x0000_fffc)
    } else {
        (-33_554_432, 33_554_428, 0x03ff_fffc)
    };
    if displacement < minimum || displacement > maximum {
        return Err(EncodeError::BranchOutOfRange { address, target });
    }
    Ok(displacement as u32 & mask)
}

impl TypedIsa for PowerPc750Cl {
    const PROFILE_ID: &'static str = crate::POWERPC_750CL_PROFILE_ID;

    type Instruction = Instruction;
    type Form = InstructionForm;
    type DecodeError = DecodeError;
    type EncodeError = EncodeError;

    fn form(instruction: &Self::Instruction) -> Result<Self::Form, Self::EncodeError> {
        Ok(instruction.form())
    }

    fn decode_one(
        source: &[u8],
        address: u64,
    ) -> Result<ProtocolDecodedInstruction<Self::Instruction>, Self::DecodeError> {
        let instruction = decode_750cl_bytes(source, address)?;
        let bytes = InstructionBytes::new(source[..4].to_vec())
            .expect("PowerPC instructions are non-empty");
        Ok(ProtocolDecodedInstruction::new(instruction, bytes))
    }

    fn encode_one(
        instruction: &Self::Instruction,
        address: u64,
    ) -> Result<InstructionBytes, Self::EncodeError> {
        let bytes = encode_750cl_bytes(instruction, address)?;
        Ok(InstructionBytes::new(bytes.to_vec()).expect("PowerPC instructions are non-empty"))
    }
}

impl TypedIsa for PowerPcGekko {
    const PROFILE_ID: &'static str = crate::GEKKO_PROFILE_ID;

    type Instruction = Instruction;
    type Form = InstructionForm;
    type DecodeError = DecodeError;
    type EncodeError = EncodeError;

    fn form(instruction: &Self::Instruction) -> Result<Self::Form, Self::EncodeError> {
        validate_gekko_encode(instruction)?;
        Ok(instruction.form())
    }

    fn decode_one(
        source: &[u8],
        address: u64,
    ) -> Result<ProtocolDecodedInstruction<Self::Instruction>, Self::DecodeError> {
        let instruction = decode_gekko_bytes(source, address)?;
        let bytes = InstructionBytes::new(source[..4].to_vec())
            .expect("PowerPC instructions are non-empty");
        Ok(ProtocolDecodedInstruction::new(instruction, bytes))
    }

    fn encode_one(
        instruction: &Self::Instruction,
        address: u64,
    ) -> Result<InstructionBytes, Self::EncodeError> {
        let bytes = encode_gekko_bytes(instruction, address)?;
        Ok(InstructionBytes::new(bytes.to_vec()).expect("PowerPC instructions are non-empty"))
    }
}
