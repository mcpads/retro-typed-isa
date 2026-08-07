use thiserror::Error;
use typed_isa_core::{
    DecodedInstruction as ProtocolDecodedInstruction, InstructionBytes, TypedIsa,
};

use crate::{
    ConstructionError, FloatingControlRegister, FloatingRegister, GeneralRegister, Instruction,
    InstructionForm, Operands, PerformanceCounter, PrefetchHint, ShiftAmount, SyncType, SystemCode,
    SystemControlRegister, TrapCode, VectorControlRegister, VectorDestinationMask,
    VectorFloatingRegister, VectorImmediate5, VectorIntegerRegister, VectorInterlock, VectorLane,
    VectorMicroAddress, forms::r5900_forms,
};

/// Sony/Toshiba R5900 protocol profile for the PlayStation 2 EE.
#[derive(Debug, Clone, Copy)]
pub struct R5900;

/// Failure to decode one fixed-width R5900 instruction.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DecodeError {
    /// The supplied instruction address is not four-byte aligned.
    #[error("R5900 instruction address {address:#x} is not four-byte aligned")]
    UnalignedAddress {
        /// Supplied instruction address.
        address: u64,
    },
    /// The supplied instruction address exceeds the 32-bit EE address space.
    #[error("R5900 instruction address {address:#x} exceeds the 32-bit address space")]
    AddressOutOfRange {
        /// Supplied instruction address.
        address: u64,
    },
    /// Fewer than four source bytes were available.
    #[error("truncated R5900 instruction: needed 4 bytes, got {got}")]
    UnexpectedEnd {
        /// Number of source bytes available.
        got: usize,
    },
    /// The word is reserved or outside the declared R5900 profile.
    #[error("reserved or out-of-profile R5900 instruction word {word:#010x}")]
    ReservedWord {
        /// Rejected instruction word.
        word: u32,
    },
}

/// Failure to validate placement or encode one R5900 instruction.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EncodeError {
    /// The supplied instruction address is not four-byte aligned.
    #[error("R5900 instruction address {address:#x} is not four-byte aligned")]
    UnalignedAddress {
        /// Supplied instruction address.
        address: u64,
    },
    /// The supplied instruction address exceeds the 32-bit EE address space.
    #[error("R5900 instruction address {address:#x} exceeds the 32-bit address space")]
    AddressOutOfRange {
        /// Supplied instruction address.
        address: u64,
    },
    /// A control-flow target is not four-byte aligned.
    #[error("control-flow target {target:#010x} is not four-byte aligned")]
    UnalignedTarget {
        /// Rejected target address.
        target: u32,
    },
    /// A PC-relative branch cannot represent the requested displacement.
    #[error("branch at {address:#010x} cannot reach {target:#010x}")]
    BranchOutOfRange {
        /// Address of the branch instruction.
        address: u32,
        /// Requested absolute target address.
        target: u32,
    },
    /// A direct jump target is outside the instruction's 256 MiB region.
    #[error("jump at {address:#010x} cannot reach region containing {target:#010x}")]
    JumpRegionMismatch {
        /// Address of the jump instruction.
        address: u32,
        /// Requested absolute target address.
        target: u32,
    },
    /// Operand bits conflict with bits fixed by the selected instruction form.
    #[error("operands overwrite fixed bits of R5900 instruction {mnemonic}")]
    FixedFieldConflict {
        /// Mnemonic of the conflicting form.
        mnemonic: &'static str,
    },
    /// The instruction and operand shape fail typed construction rules.
    #[error("invalid R5900 instruction: {0}")]
    InvalidInstruction(#[from] ConstructionError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OperandLayout {
    None,
    ThreeGeneral,
    TwoGeneral,
    GeneralPair,
    GeneralDestination,
    GeneralSource,
    GeneralDestinationRt,
    GeneralSourceRt,
    SignedImmediate,
    UnsignedImmediate,
    LoadUpper,
    ShiftImmediate,
    ShiftVariable,
    Multiply,
    Jump,
    JumpRegister,
    JumpLinkRegister,
    BranchTwo,
    BranchOne,
    BranchCop0,
    BranchCop1,
    BranchCop2,
    MemoryGeneral,
    Prefetch,
    Sync,
    System,
    TrapGeneral,
    TrapImmediate,
    SourceImmediate,
    Cache,
    SystemControl,
    Performance,
    PerformanceControl,
    FloatingTransfer,
    FloatingControl,
    FloatingThree,
    FloatingTwo,
    FloatingTwoTarget,
    FloatingAccumulatorPair,
    FloatingCompare,
    FloatingMemory,
    VectorControlTransfer,
    VectorFloatingTransfer,
    VectorMemory,
    VectorThree,
    VectorScalar,
    VectorBroadcast,
    VectorTwo,
    VectorAccumulatorPair,
    VectorAccumulatorSource,
    VectorAccumulatorBroadcast,
    VectorIntegerThree,
    VectorIntegerImmediate,
    VectorIntegerLoad,
    VectorIntegerStore,
    VectorLoadUpdate,
    VectorStoreUpdate,
    VectorFieldPair,
    VectorSourceField,
    VectorTargetField,
    VectorIntegerToFloating,
    VectorFloatingToInteger,
    VectorRandomDestination,
    VectorPairXyz,
    VectorThreeXyz,
    VectorMicroCall,
}

#[derive(Debug, Clone, Copy)]
struct Descriptor {
    form: InstructionForm,
    mnemonic: &'static str,
    mask: u32,
    value: u32,
    layout: OperandLayout,
}

macro_rules! define_descriptors {
    ($( $variant:ident, $mnemonic:literal, $mask:literal, $value:literal, $layout:ident; )+) => {
        const DESCRIPTORS: &[Descriptor] = &[
            $(Descriptor {
                form: InstructionForm::$variant,
                mnemonic: $mnemonic,
                mask: $mask,
                value: $value,
                layout: OperandLayout::$layout,
            },)+
        ];
    };
}

r5900_forms!(define_descriptors);

/// Decodes one little-endian-independent R5900 instruction word at `address`.
pub fn decode_word(word: u32, address: u64) -> Result<Instruction, DecodeError> {
    let address = checked_decode_address(address)?;
    for specificity in (0..=32).rev() {
        for descriptor in DESCRIPTORS.iter().filter(|descriptor| {
            descriptor.mask.count_ones() == specificity
                && word & descriptor.mask == descriptor.value
        }) {
            if let Some(operands) = decode_operands(descriptor, word, address) {
                return Ok(Instruction::from_decoded(descriptor.form, operands));
            }
        }
    }
    Err(DecodeError::ReservedWord { word })
}

/// Encodes one typed instruction into an R5900 instruction word.
pub fn encode_word(instruction: &Instruction, address: u64) -> Result<u32, EncodeError> {
    let address = checked_encode_address(address)?;
    validate_operands(instruction.form(), instruction.operands())?;
    let descriptor = descriptor_for_form(instruction.form());
    let mut word = descriptor.value;

    match instruction.operands() {
        Operands::None => {}
        Operands::ThreeGeneral {
            destination,
            left,
            right,
        } => word |= three_general(*destination, *left, *right),
        Operands::TwoGeneral {
            destination,
            source,
        } => word |= u32::from(source.index()) << 16 | u32::from(destination.index()) << 11,
        Operands::GeneralPair { left, right } => {
            word |= u32::from(left.index()) << 21 | u32::from(right.index()) << 16;
        }
        Operands::GeneralDestination { destination } => {
            word |= u32::from(destination.index())
                << if descriptor.layout == OperandLayout::GeneralDestinationRt {
                    16
                } else {
                    11
                };
        }
        Operands::GeneralSource { source } => {
            word |= u32::from(source.index())
                << if descriptor.layout == OperandLayout::GeneralSourceRt {
                    16
                } else {
                    21
                };
        }
        Operands::SignedImmediate {
            destination,
            source,
            immediate,
        } => {
            word |= u32::from(source.index()) << 21
                | u32::from(destination.index()) << 16
                | u32::from(*immediate as u16);
        }
        Operands::UnsignedImmediate {
            destination,
            source,
            immediate,
        } => {
            word |= u32::from(source.index()) << 21
                | u32::from(destination.index()) << 16
                | u32::from(*immediate);
        }
        Operands::LoadUpper {
            destination,
            immediate,
        } => word |= u32::from(destination.index()) << 16 | u32::from(*immediate),
        Operands::ShiftImmediate {
            destination,
            source,
            amount,
        } => {
            word |= u32::from(source.index()) << 16
                | u32::from(destination.index()) << 11
                | u32::from(amount.value()) << 6;
        }
        Operands::ShiftVariable {
            destination,
            source,
            amount,
        } => word |= three_general(*destination, *amount, *source),
        Operands::Multiply {
            destination,
            left,
            right,
        } => {
            word |= u32::from(left.index()) << 21
                | u32::from(right.index()) << 16
                | u32::from(destination.unwrap_or(GeneralRegister::ZERO).index()) << 11;
        }
        Operands::Jump { target } => word |= jump_field(address, *target)?,
        Operands::JumpRegister { source, link } => {
            word |= u32::from(source.index()) << 21;
            if descriptor.layout == OperandLayout::JumpLinkRegister {
                word |= u32::from(
                    link.ok_or(ConstructionError::OperandShapeMismatch {
                        mnemonic: descriptor.mnemonic,
                    })?
                    .index(),
                ) << 11;
            }
        }
        Operands::BranchTwo {
            left,
            right,
            target,
        } => {
            word |= u32::from(left.index()) << 21
                | u32::from(right.index()) << 16
                | u32::from(branch_immediate(address, *target)?);
        }
        Operands::BranchOne { source, target } => {
            word |=
                u32::from(source.index()) << 21 | u32::from(branch_immediate(address, *target)?);
        }
        Operands::Branch { target } => word |= u32::from(branch_immediate(address, *target)?),
        Operands::MemoryGeneral {
            register,
            base,
            offset,
        } => {
            word |= u32::from(base.index()) << 21
                | u32::from(register.index()) << 16
                | u32::from(*offset as u16);
        }
        Operands::Prefetch { hint, base, offset } => {
            word |= u32::from(base.index()) << 21
                | u32::from(hint.value()) << 16
                | u32::from(*offset as u16);
        }
        Operands::Sync { sync_type } => word |= u32::from(sync_type.value()) << 6,
        Operands::System { code } => word |= code.value() << 6,
        Operands::TrapGeneral { left, right, code } => {
            word |= u32::from(left.index()) << 21
                | u32::from(right.index()) << 16
                | u32::from(code.value()) << 6;
        }
        Operands::TrapImmediate { source, immediate } => {
            word |= u32::from(source.index()) << 21 | u32::from(*immediate as u16);
        }
        Operands::SourceImmediate { source, immediate } => {
            word |= u32::from(source.index()) << 21 | u32::from(*immediate);
        }
        Operands::Cache { base, offset } => {
            word |= u32::from(base.index()) << 21 | u32::from(*offset as u16);
        }
        Operands::SystemControl { general, control } => {
            word |= u32::from(general.index()) << 16 | u32::from(control.index()) << 11;
        }
        Operands::Performance { general, counter } => {
            word |= u32::from(general.index()) << 16 | u32::from(counter.value()) << 1;
        }
        Operands::FloatingTransfer { general, floating } => {
            word |= u32::from(general.index()) << 16 | u32::from(floating.index()) << 11;
        }
        Operands::FloatingControl { general, control } => {
            word |= u32::from(general.index()) << 16 | u32::from(control.index()) << 11;
        }
        Operands::FloatingThree {
            destination,
            left,
            right,
        } => word |= three_floating(*destination, *left, *right),
        Operands::FloatingTwo {
            destination,
            source,
        } => {
            word |= u32::from(source.index())
                << if descriptor.layout == OperandLayout::FloatingTwoTarget {
                    16
                } else {
                    11
                }
                | u32::from(destination.index()) << 6;
        }
        Operands::FloatingAccumulatorPair { left, right }
        | Operands::FloatingCompare { left, right } => {
            word |= u32::from(right.index()) << 16 | u32::from(left.index()) << 11;
        }
        Operands::FloatingMemory {
            floating,
            base,
            offset,
        } => {
            word |= u32::from(base.index()) << 21
                | u32::from(floating.index()) << 16
                | u32::from(*offset as u16);
        }
        Operands::VectorControlTransfer {
            general,
            control,
            interlock,
        } => {
            word |= u32::from(general.index()) << 16
                | u32::from(control.index()) << 11
                | u32::from(interlock.bit());
        }
        Operands::VectorFloatingTransfer {
            general,
            vector,
            interlock,
        } => {
            word |= u32::from(general.index()) << 16
                | u32::from(vector.index()) << 11
                | u32::from(interlock.bit());
        }
        Operands::VectorMemory {
            vector,
            base,
            offset,
        } => {
            word |= u32::from(base.index()) << 21
                | u32::from(vector.index()) << 16
                | u32::from(*offset as u16);
        }
        Operands::VectorThree {
            destination_mask,
            destination,
            left,
            right,
        } => {
            word |= u32::from(destination_mask.bits()) << 21
                | u32::from(right.index()) << 16
                | u32::from(left.index()) << 11
                | u32::from(destination.index()) << 6;
        }
        Operands::VectorBroadcast {
            destination_mask,
            destination,
            left,
            right,
            broadcast,
        } => {
            word |= u32::from(destination_mask.bits()) << 21
                | u32::from(right.index()) << 16
                | u32::from(left.index()) << 11
                | u32::from(destination.index()) << 6
                | u32::from(broadcast.field());
        }
        Operands::VectorTwo {
            destination_mask,
            destination,
            source,
        } => {
            word |= u32::from(destination_mask.bits()) << 21 | u32::from(source.index()) << 11;
            word |= u32::from(destination.index())
                << if descriptor.layout == OperandLayout::VectorScalar {
                    6
                } else {
                    16
                };
        }
        Operands::VectorAccumulatorPair {
            destination_mask,
            left,
            right,
        } => {
            word |= u32::from(destination_mask.bits()) << 21
                | u32::from(right.index()) << 16
                | u32::from(left.index()) << 11;
        }
        Operands::VectorAccumulatorSource {
            destination_mask,
            source,
        } => {
            word |= u32::from(destination_mask.bits()) << 21 | u32::from(source.index()) << 11;
        }
        Operands::VectorAccumulatorBroadcast {
            destination_mask,
            left,
            right,
            broadcast,
        } => {
            word |= u32::from(destination_mask.bits()) << 21
                | u32::from(right.index()) << 16
                | u32::from(left.index()) << 11
                | u32::from(broadcast.field());
        }
        Operands::VectorIntegerThree {
            destination,
            left,
            right,
        } => {
            word |= u32::from(right.index()) << 16
                | u32::from(left.index()) << 11
                | u32::from(destination.index()) << 6;
        }
        Operands::VectorIntegerImmediate {
            destination,
            source,
            immediate,
        } => {
            word |= u32::from(destination.index()) << 16
                | u32::from(source.index()) << 11
                | u32::from(immediate.field()) << 6;
        }
        Operands::VectorIntegerLoad {
            lane,
            destination,
            base,
        } => {
            word |= u32::from(lane.destination_bits()) << 21
                | u32::from(destination.index()) << 16
                | u32::from(base.index()) << 11;
        }
        Operands::VectorIntegerStore {
            destination_mask,
            source,
            base,
        } => {
            word |= u32::from(destination_mask.bits()) << 21
                | u32::from(source.index()) << 16
                | u32::from(base.index()) << 11;
        }
        Operands::VectorMemoryUpdate {
            destination_mask,
            vector,
            base,
        } => {
            word |= u32::from(destination_mask.bits()) << 21;
            if descriptor.layout == OperandLayout::VectorLoadUpdate {
                word |= u32::from(vector.index()) << 16 | u32::from(base.index()) << 11;
            } else {
                word |= u32::from(base.index()) << 16 | u32::from(vector.index()) << 11;
            }
        }
        Operands::VectorFieldPair {
            left,
            left_lane,
            right,
            right_lane,
        } => {
            word |= u32::from(right_lane.field()) << 23
                | u32::from(left_lane.field()) << 21
                | u32::from(right.index()) << 16
                | u32::from(left.index()) << 11;
        }
        Operands::VectorFieldSource { source, lane } => {
            if descriptor.layout == OperandLayout::VectorTargetField {
                word |= u32::from(lane.field()) << 23 | u32::from(source.index()) << 16;
            } else {
                word |= u32::from(lane.field()) << 21 | u32::from(source.index()) << 11;
            }
        }
        Operands::VectorIntegerToFloating {
            destination_mask,
            destination,
            source,
        } => {
            word |= u32::from(destination_mask.bits()) << 21
                | u32::from(destination.index()) << 16
                | u32::from(source.index()) << 11;
        }
        Operands::VectorFloatingToInteger {
            destination,
            source,
            lane,
        } => {
            word |= u32::from(lane.field()) << 21
                | u32::from(destination.index()) << 16
                | u32::from(source.index()) << 11;
        }
        Operands::VectorRandomDestination {
            destination_mask,
            destination,
        } => {
            word |= u32::from(destination_mask.bits()) << 21 | u32::from(destination.index()) << 16;
        }
        Operands::VectorPairXyz { left, right } => {
            word |= u32::from(right.index()) << 16 | u32::from(left.index()) << 11;
        }
        Operands::VectorThreeXyz {
            destination,
            left,
            right,
        } => {
            word |= u32::from(right.index()) << 16
                | u32::from(left.index()) << 11
                | u32::from(destination.index()) << 6;
        }
        Operands::VectorMicroCall { target } => word |= u32::from(target.field()) << 6,
    }

    if word & descriptor.mask != descriptor.value {
        return Err(EncodeError::FixedFieldConflict {
            mnemonic: descriptor.mnemonic,
        });
    }
    Ok(word)
}

/// Decodes the first four source bytes as a little-endian R5900 instruction.
pub fn decode_bytes(source: &[u8], address: u64) -> Result<Instruction, DecodeError> {
    let bytes = source
        .get(..4)
        .ok_or(DecodeError::UnexpectedEnd { got: source.len() })?;
    decode_word(
        u32::from_le_bytes(bytes.try_into().expect("four-byte instruction")),
        address,
    )
}

/// Encodes one typed instruction as four little-endian bytes.
pub fn encode_bytes(instruction: &Instruction, address: u64) -> Result<[u8; 4], EncodeError> {
    Ok(encode_word(instruction, address)?.to_le_bytes())
}

impl TypedIsa for R5900 {
    const PROFILE_ID: &'static str = crate::PROFILE_ID;

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
        let instruction = decode_bytes(source, address)?;
        let bytes = InstructionBytes::new(source[..4].to_vec())
            .expect("R5900 instruction bytes are non-empty");
        Ok(ProtocolDecodedInstruction::new(instruction, bytes))
    }

    fn encode_one(
        instruction: &Self::Instruction,
        address: u64,
    ) -> Result<InstructionBytes, Self::EncodeError> {
        Ok(
            InstructionBytes::new(encode_bytes(instruction, address)?.to_vec())
                .expect("R5900 instruction bytes are non-empty"),
        )
    }
}

pub(crate) fn validate_operands(
    form: InstructionForm,
    operands: &Operands,
) -> Result<(), ConstructionError> {
    let descriptor = descriptor_for_form(form);
    let shape_matches = matches!(
        (descriptor.layout, operands),
        (OperandLayout::None, Operands::None)
            | (OperandLayout::ThreeGeneral, Operands::ThreeGeneral { .. })
            | (OperandLayout::TwoGeneral, Operands::TwoGeneral { .. })
            | (OperandLayout::GeneralPair, Operands::GeneralPair { .. })
            | (
                OperandLayout::GeneralDestination,
                Operands::GeneralDestination { .. }
            )
            | (OperandLayout::GeneralSource, Operands::GeneralSource { .. })
            | (
                OperandLayout::GeneralDestinationRt,
                Operands::GeneralDestination { .. }
            )
            | (
                OperandLayout::GeneralSourceRt,
                Operands::GeneralSource { .. }
            )
            | (
                OperandLayout::SignedImmediate,
                Operands::SignedImmediate { .. }
            )
            | (
                OperandLayout::UnsignedImmediate,
                Operands::UnsignedImmediate { .. }
            )
            | (OperandLayout::LoadUpper, Operands::LoadUpper { .. })
            | (
                OperandLayout::ShiftImmediate,
                Operands::ShiftImmediate { .. }
            )
            | (OperandLayout::ShiftVariable, Operands::ShiftVariable { .. })
            | (OperandLayout::Multiply, Operands::Multiply { .. })
            | (OperandLayout::Jump, Operands::Jump { .. })
            | (
                OperandLayout::JumpRegister,
                Operands::JumpRegister { link: None, .. }
            )
            | (
                OperandLayout::JumpLinkRegister,
                Operands::JumpRegister { link: Some(_), .. }
            )
            | (OperandLayout::BranchTwo, Operands::BranchTwo { .. })
            | (OperandLayout::BranchOne, Operands::BranchOne { .. })
            | (OperandLayout::BranchCop0, Operands::Branch { .. })
            | (OperandLayout::BranchCop1, Operands::Branch { .. })
            | (OperandLayout::BranchCop2, Operands::Branch { .. })
            | (OperandLayout::MemoryGeneral, Operands::MemoryGeneral { .. })
            | (OperandLayout::Prefetch, Operands::Prefetch { .. })
            | (OperandLayout::Sync, Operands::Sync { .. })
            | (OperandLayout::System, Operands::System { .. })
            | (OperandLayout::TrapGeneral, Operands::TrapGeneral { .. })
            | (OperandLayout::TrapImmediate, Operands::TrapImmediate { .. })
            | (
                OperandLayout::SourceImmediate,
                Operands::SourceImmediate { .. }
            )
            | (OperandLayout::Cache, Operands::Cache { .. })
            | (OperandLayout::SystemControl, Operands::SystemControl { .. })
            | (OperandLayout::Performance, Operands::Performance { .. })
            | (
                OperandLayout::PerformanceControl,
                Operands::Performance { .. }
            )
            | (
                OperandLayout::FloatingTransfer,
                Operands::FloatingTransfer { .. }
            )
            | (
                OperandLayout::FloatingControl,
                Operands::FloatingControl { .. }
            )
            | (OperandLayout::FloatingThree, Operands::FloatingThree { .. })
            | (OperandLayout::FloatingTwo, Operands::FloatingTwo { .. })
            | (
                OperandLayout::FloatingTwoTarget,
                Operands::FloatingTwo { .. }
            )
            | (
                OperandLayout::FloatingAccumulatorPair,
                Operands::FloatingAccumulatorPair { .. }
            )
            | (
                OperandLayout::FloatingCompare,
                Operands::FloatingCompare { .. }
            )
            | (
                OperandLayout::FloatingMemory,
                Operands::FloatingMemory { .. }
            )
            | (
                OperandLayout::VectorControlTransfer,
                Operands::VectorControlTransfer { .. }
            )
            | (
                OperandLayout::VectorFloatingTransfer,
                Operands::VectorFloatingTransfer { .. }
            )
            | (OperandLayout::VectorMemory, Operands::VectorMemory { .. })
            | (OperandLayout::VectorThree, Operands::VectorThree { .. })
            | (OperandLayout::VectorScalar, Operands::VectorTwo { .. })
            | (
                OperandLayout::VectorBroadcast,
                Operands::VectorBroadcast { .. }
            )
            | (OperandLayout::VectorTwo, Operands::VectorTwo { .. })
            | (
                OperandLayout::VectorAccumulatorPair,
                Operands::VectorAccumulatorPair { .. }
            )
            | (
                OperandLayout::VectorAccumulatorSource,
                Operands::VectorAccumulatorSource { .. }
            )
            | (
                OperandLayout::VectorAccumulatorBroadcast,
                Operands::VectorAccumulatorBroadcast { .. }
            )
            | (
                OperandLayout::VectorIntegerThree,
                Operands::VectorIntegerThree { .. }
            )
            | (
                OperandLayout::VectorIntegerImmediate,
                Operands::VectorIntegerImmediate { .. }
            )
            | (
                OperandLayout::VectorIntegerLoad,
                Operands::VectorIntegerLoad { .. }
            )
            | (
                OperandLayout::VectorIntegerStore,
                Operands::VectorIntegerStore { .. }
            )
            | (
                OperandLayout::VectorLoadUpdate,
                Operands::VectorMemoryUpdate { .. }
            )
            | (
                OperandLayout::VectorStoreUpdate,
                Operands::VectorMemoryUpdate { .. }
            )
            | (
                OperandLayout::VectorFieldPair,
                Operands::VectorFieldPair { .. }
            )
            | (
                OperandLayout::VectorSourceField,
                Operands::VectorFieldSource { .. }
            )
            | (
                OperandLayout::VectorTargetField,
                Operands::VectorFieldSource { .. }
            )
            | (
                OperandLayout::VectorIntegerToFloating,
                Operands::VectorIntegerToFloating { .. }
            )
            | (
                OperandLayout::VectorFloatingToInteger,
                Operands::VectorFloatingToInteger { .. }
            )
            | (
                OperandLayout::VectorRandomDestination,
                Operands::VectorRandomDestination { .. }
            )
            | (OperandLayout::VectorPairXyz, Operands::VectorPairXyz { .. })
            | (
                OperandLayout::VectorThreeXyz,
                Operands::VectorThreeXyz { .. }
            )
            | (
                OperandLayout::VectorMicroCall,
                Operands::VectorMicroCall { .. }
            )
    );
    if !shape_matches {
        return Err(ConstructionError::OperandShapeMismatch {
            mnemonic: descriptor.mnemonic,
        });
    }

    if descriptor.layout == OperandLayout::PerformanceControl
        && !matches!(
            operands,
            Operands::Performance { counter, .. } if counter.value() == 0
        )
    {
        return Err(ConstructionError::OperandShapeMismatch {
            mnemonic: descriptor.mnemonic,
        });
    }
    if form == InstructionForm::Ctc1
        && !matches!(
            operands,
            Operands::FloatingControl {
                control: FloatingControlRegister::ControlStatus,
                ..
            }
        )
    {
        return Err(ConstructionError::InvalidFloatingControlWrite);
    }
    Ok(())
}

fn decode_operands(descriptor: &Descriptor, word: u32, address: u32) -> Option<Operands> {
    let rs = || GeneralRegister::from_field(((word >> 21) & 31) as u8);
    let rt = || GeneralRegister::from_field(((word >> 16) & 31) as u8);
    let rd = || GeneralRegister::from_field(((word >> 11) & 31) as u8);
    let fs = || FloatingRegister::from_field(((word >> 11) & 31) as u8);
    let ft = || FloatingRegister::from_field(((word >> 16) & 31) as u8);
    let fd = || FloatingRegister::from_field(((word >> 6) & 31) as u8);
    let vector_fs = || VectorFloatingRegister::from_field(((word >> 11) & 31) as u8);
    let vector_ft = || VectorFloatingRegister::from_field(((word >> 16) & 31) as u8);
    let vector_fd = || VectorFloatingRegister::from_field(((word >> 6) & 31) as u8);
    let vector_is = || VectorIntegerRegister::from_field(((word >> 11) & 31) as u8);
    let vector_it = || VectorIntegerRegister::from_field(((word >> 16) & 31) as u8);
    let vector_id = || VectorIntegerRegister::from_field(((word >> 6) & 31) as u8);
    let destination_mask = || VectorDestinationMask::from_field(((word >> 21) & 15) as u8);

    Some(match descriptor.layout {
        OperandLayout::None => Operands::None,
        OperandLayout::ThreeGeneral => Operands::ThreeGeneral {
            destination: rd(),
            left: rs(),
            right: rt(),
        },
        OperandLayout::TwoGeneral => Operands::TwoGeneral {
            destination: rd(),
            source: rt(),
        },
        OperandLayout::GeneralPair => Operands::GeneralPair {
            left: rs(),
            right: rt(),
        },
        OperandLayout::GeneralDestination => Operands::GeneralDestination { destination: rd() },
        OperandLayout::GeneralSource => Operands::GeneralSource { source: rs() },
        OperandLayout::GeneralDestinationRt => Operands::GeneralDestination { destination: rt() },
        OperandLayout::GeneralSourceRt => Operands::GeneralSource { source: rt() },
        OperandLayout::SignedImmediate => Operands::SignedImmediate {
            destination: rt(),
            source: rs(),
            immediate: word as u16 as i16,
        },
        OperandLayout::UnsignedImmediate => Operands::UnsignedImmediate {
            destination: rt(),
            source: rs(),
            immediate: word as u16,
        },
        OperandLayout::LoadUpper => Operands::LoadUpper {
            destination: rt(),
            immediate: word as u16,
        },
        OperandLayout::ShiftImmediate => Operands::ShiftImmediate {
            destination: rd(),
            source: rt(),
            amount: ShiftAmount::from_field(((word >> 6) & 31) as u8),
        },
        OperandLayout::ShiftVariable => Operands::ShiftVariable {
            destination: rd(),
            source: rt(),
            amount: rs(),
        },
        OperandLayout::Multiply => Operands::Multiply {
            destination: (rd() != GeneralRegister::ZERO).then(rd),
            left: rs(),
            right: rt(),
        },
        OperandLayout::Jump => Operands::Jump {
            target: jump_target(word, address),
        },
        OperandLayout::JumpRegister => Operands::JumpRegister {
            source: rs(),
            link: None,
        },
        OperandLayout::JumpLinkRegister => Operands::JumpRegister {
            source: rs(),
            link: Some(rd()),
        },
        OperandLayout::BranchTwo => Operands::BranchTwo {
            left: rs(),
            right: rt(),
            target: branch_target(word, address),
        },
        OperandLayout::BranchOne => Operands::BranchOne {
            source: rs(),
            target: branch_target(word, address),
        },
        OperandLayout::BranchCop0 | OperandLayout::BranchCop1 | OperandLayout::BranchCop2 => {
            Operands::Branch {
                target: branch_target(word, address),
            }
        }
        OperandLayout::MemoryGeneral => Operands::MemoryGeneral {
            register: rt(),
            base: rs(),
            offset: word as u16 as i16,
        },
        OperandLayout::Prefetch => Operands::Prefetch {
            hint: PrefetchHint::from_field(((word >> 16) & 31) as u8),
            base: rs(),
            offset: word as u16 as i16,
        },
        OperandLayout::Sync => Operands::Sync {
            sync_type: SyncType::from_field(((word >> 6) & 31) as u8),
        },
        OperandLayout::System => Operands::System {
            code: SystemCode::from_field((word >> 6) & 0x000f_ffff),
        },
        OperandLayout::TrapGeneral => Operands::TrapGeneral {
            left: rs(),
            right: rt(),
            code: TrapCode::from_field(((word >> 6) & 0x03ff) as u16),
        },
        OperandLayout::TrapImmediate => Operands::TrapImmediate {
            source: rs(),
            immediate: word as u16 as i16,
        },
        OperandLayout::SourceImmediate => Operands::SourceImmediate {
            source: rs(),
            immediate: word as u16,
        },
        OperandLayout::Cache => Operands::Cache {
            base: rs(),
            offset: word as u16 as i16,
        },
        OperandLayout::SystemControl => Operands::SystemControl {
            general: rt(),
            control: SystemControlRegister::new(((word >> 11) & 31) as u8)?,
        },
        OperandLayout::Performance => Operands::Performance {
            general: rt(),
            counter: PerformanceCounter::new(((word >> 1) & 31) as u8)?,
        },
        OperandLayout::PerformanceControl => {
            let raw_counter = ((word >> 1) & 31) as u8;
            if raw_counter != 0 {
                return None;
            }
            let counter = PerformanceCounter::from_field(raw_counter);
            Operands::Performance {
                general: rt(),
                counter,
            }
        }
        OperandLayout::FloatingTransfer => Operands::FloatingTransfer {
            general: rt(),
            floating: fs(),
        },
        OperandLayout::FloatingControl => {
            let control = FloatingControlRegister::from_field(((word >> 11) & 31) as u8)?;
            if descriptor.form == InstructionForm::Ctc1
                && control != FloatingControlRegister::ControlStatus
            {
                return None;
            }
            Operands::FloatingControl {
                general: rt(),
                control,
            }
        }
        OperandLayout::FloatingThree => Operands::FloatingThree {
            destination: fd(),
            left: fs(),
            right: ft(),
        },
        OperandLayout::FloatingTwo => Operands::FloatingTwo {
            destination: fd(),
            source: fs(),
        },
        OperandLayout::FloatingTwoTarget => Operands::FloatingTwo {
            destination: fd(),
            source: ft(),
        },
        OperandLayout::FloatingAccumulatorPair => Operands::FloatingAccumulatorPair {
            left: fs(),
            right: ft(),
        },
        OperandLayout::FloatingCompare => Operands::FloatingCompare {
            left: fs(),
            right: ft(),
        },
        OperandLayout::FloatingMemory => Operands::FloatingMemory {
            floating: ft(),
            base: rs(),
            offset: word as u16 as i16,
        },
        OperandLayout::VectorControlTransfer => Operands::VectorControlTransfer {
            general: rt(),
            control: VectorControlRegister::new(((word >> 11) & 31) as u8)?,
            interlock: VectorInterlock::from_bit(word as u8),
        },
        OperandLayout::VectorFloatingTransfer => Operands::VectorFloatingTransfer {
            general: rt(),
            vector: vector_fs(),
            interlock: VectorInterlock::from_bit(word as u8),
        },
        OperandLayout::VectorMemory => Operands::VectorMemory {
            vector: vector_ft(),
            base: rs(),
            offset: word as u16 as i16,
        },
        OperandLayout::VectorThree => Operands::VectorThree {
            destination_mask: destination_mask(),
            destination: vector_fd(),
            left: vector_fs(),
            right: vector_ft(),
        },
        OperandLayout::VectorScalar => Operands::VectorTwo {
            destination_mask: destination_mask(),
            destination: vector_fd(),
            source: vector_fs(),
        },
        OperandLayout::VectorBroadcast => Operands::VectorBroadcast {
            destination_mask: destination_mask(),
            destination: vector_fd(),
            left: vector_fs(),
            right: vector_ft(),
            broadcast: VectorLane::from_field(word as u8),
        },
        OperandLayout::VectorTwo => Operands::VectorTwo {
            destination_mask: destination_mask(),
            destination: vector_ft(),
            source: vector_fs(),
        },
        OperandLayout::VectorAccumulatorPair => Operands::VectorAccumulatorPair {
            destination_mask: destination_mask(),
            left: vector_fs(),
            right: vector_ft(),
        },
        OperandLayout::VectorAccumulatorSource => Operands::VectorAccumulatorSource {
            destination_mask: destination_mask(),
            source: vector_fs(),
        },
        OperandLayout::VectorAccumulatorBroadcast => Operands::VectorAccumulatorBroadcast {
            destination_mask: destination_mask(),
            left: vector_fs(),
            right: vector_ft(),
            broadcast: VectorLane::from_field(word as u8),
        },
        OperandLayout::VectorIntegerThree => Operands::VectorIntegerThree {
            destination: vector_id()?,
            left: vector_is()?,
            right: vector_it()?,
        },
        OperandLayout::VectorIntegerImmediate => Operands::VectorIntegerImmediate {
            destination: vector_it()?,
            source: vector_is()?,
            immediate: VectorImmediate5::from_field(((word >> 6) & 31) as u8),
        },
        OperandLayout::VectorIntegerLoad => Operands::VectorIntegerLoad {
            lane: VectorLane::from_destination_bits(destination_mask().bits())?,
            destination: vector_it()?,
            base: vector_is()?,
        },
        OperandLayout::VectorIntegerStore => Operands::VectorIntegerStore {
            destination_mask: destination_mask(),
            source: vector_it()?,
            base: vector_is()?,
        },
        OperandLayout::VectorLoadUpdate => Operands::VectorMemoryUpdate {
            destination_mask: destination_mask(),
            vector: vector_ft(),
            base: vector_is()?,
        },
        OperandLayout::VectorStoreUpdate => Operands::VectorMemoryUpdate {
            destination_mask: destination_mask(),
            vector: vector_fs(),
            base: vector_it()?,
        },
        OperandLayout::VectorFieldPair => Operands::VectorFieldPair {
            left: vector_fs(),
            left_lane: VectorLane::from_field(((word >> 21) & 3) as u8),
            right: vector_ft(),
            right_lane: VectorLane::from_field(((word >> 23) & 3) as u8),
        },
        OperandLayout::VectorSourceField => Operands::VectorFieldSource {
            source: vector_fs(),
            lane: VectorLane::from_field(((word >> 21) & 3) as u8),
        },
        OperandLayout::VectorTargetField => Operands::VectorFieldSource {
            source: vector_ft(),
            lane: VectorLane::from_field(((word >> 23) & 3) as u8),
        },
        OperandLayout::VectorIntegerToFloating => Operands::VectorIntegerToFloating {
            destination_mask: destination_mask(),
            destination: vector_ft(),
            source: vector_is()?,
        },
        OperandLayout::VectorFloatingToInteger => Operands::VectorFloatingToInteger {
            destination: vector_it()?,
            source: vector_fs(),
            lane: VectorLane::from_field(((word >> 21) & 3) as u8),
        },
        OperandLayout::VectorRandomDestination => Operands::VectorRandomDestination {
            destination_mask: destination_mask(),
            destination: vector_ft(),
        },
        OperandLayout::VectorPairXyz => Operands::VectorPairXyz {
            left: vector_fs(),
            right: vector_ft(),
        },
        OperandLayout::VectorThreeXyz => Operands::VectorThreeXyz {
            destination: vector_fd(),
            left: vector_fs(),
            right: vector_ft(),
        },
        OperandLayout::VectorMicroCall => Operands::VectorMicroCall {
            target: VectorMicroAddress::from_field(((word >> 6) & 0x7fff) as u16),
        },
    })
}

fn descriptor_for_form(form: InstructionForm) -> &'static Descriptor {
    DESCRIPTORS
        .iter()
        .find(|descriptor| descriptor.form == form)
        .expect("every public R5900 form has a descriptor")
}

fn three_general(
    destination: GeneralRegister,
    left: GeneralRegister,
    right: GeneralRegister,
) -> u32 {
    u32::from(left.index()) << 21
        | u32::from(right.index()) << 16
        | u32::from(destination.index()) << 11
}

fn three_floating(
    destination: FloatingRegister,
    left: FloatingRegister,
    right: FloatingRegister,
) -> u32 {
    u32::from(right.index()) << 16
        | u32::from(left.index()) << 11
        | u32::from(destination.index()) << 6
}

fn checked_decode_address(address: u64) -> Result<u32, DecodeError> {
    if address & 3 != 0 {
        return Err(DecodeError::UnalignedAddress { address });
    }
    u32::try_from(address).map_err(|_| DecodeError::AddressOutOfRange { address })
}

fn checked_encode_address(address: u64) -> Result<u32, EncodeError> {
    if address & 3 != 0 {
        return Err(EncodeError::UnalignedAddress { address });
    }
    u32::try_from(address).map_err(|_| EncodeError::AddressOutOfRange { address })
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

fn jump_field(address: u32, target: u32) -> Result<u32, EncodeError> {
    if target & 3 != 0 {
        return Err(EncodeError::UnalignedTarget { target });
    }
    if target & 0xf000_0000 != address.wrapping_add(4) & 0xf000_0000 {
        return Err(EncodeError::JumpRegionMismatch { address, target });
    }
    Ok(target >> 2 & 0x03ff_ffff)
}

fn branch_target(word: u32, address: u32) -> u32 {
    address
        .wrapping_add(4)
        .wrapping_add(((word as u16 as i16 as i32) << 2) as u32)
}

fn jump_target(word: u32, address: u32) -> u32 {
    address.wrapping_add(4) & 0xf000_0000 | (word & 0x03ff_ffff) << 2
}
