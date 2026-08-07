//! Static NEC V30MZ profile for WonderSwan executable code.
//!
//! The normative sources are NEC's [*V30MZ 16-Bit Microprocessor Core
//! Hardware Preliminary User's Manual*][hardware-manual], document
//! A13761EJ1V0UM00, and the companion [*16-Bit V Series Instruction User's
//! Manual*][instruction-manual]. The hardware manual defines the exact V30MZ
//! boundary: unlike V30HL, V30MZ does not implement the `0F` extension space or
//! the `REPC` and `REPNC` prefixes. Those encodings fail closed here.
//!
//! The V30 crate supplies shared, typed V-series operands and the common
//! 8086/80186 encoding machinery. It is an implementation component, not the
//! coverage oracle. The V30MZ selector census is derived independently from
//! the NEC manuals.
//!
//! Execution, timing, peripherals, and simulation are outside this crate.
//!
//! [hardware-manual]: https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary
//! [instruction-manual]: https://www.renesas.com/en/document/mah/16-bit-v-seriestm-instruction

use typed_isa_core::{
    DecodedInstruction as ProtocolDecodedInstruction, InstructionBytes, TypedIsa,
};

mod assembler;
mod expected_write;

pub use assembler::{AssembledProgram, Assembler, AssemblerError};
pub use expected_write::ExpectedWriteVerifier;
pub use v30::Instruction as VSeriesInstruction;
pub use v30::{
    CallTarget, Condition, ControlFlow, EffectiveAddress, EffectiveAddressBase,
    EffectiveAddressDisplacement, EffectiveAddressError, EscapeOpcode, FloatingPointOperand,
    InstructionAddress, JmpTarget, LoopCondition, Operand, OperandForm, OperandSize, PortAddress,
    Register8, Register16, SegmentRegister, SemanticPrefix, ShiftCount, SignedImmediate,
};

/// Stable identifier for the exact CPU profile implemented by this crate.
pub const PROFILE_ID: &str = env!("CARGO_PKG_NAME");

/// V30MZ implementation of the workspace-wide static ISA protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct V30Mz;

/// A typed instruction proven to have a valid V30MZ encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction(VSeriesInstruction);

impl Instruction {
    pub fn new(instruction: VSeriesInstruction) -> Result<Self, EncodeError> {
        let bytes = v30::encode_bytes(&instruction).map_err(EncodeError::BaseProfile)?;
        validate_v30mz_encoding(&bytes)?;
        Ok(Self(instruction))
    }

    pub fn as_v_series(&self) -> &VSeriesInstruction {
        &self.0
    }

    pub fn into_v_series(self) -> VSeriesInstruction {
        self.0
    }

    pub fn form(&self) -> InstructionForm {
        InstructionForm(
            v30::InstructionForm::of(&self.0)
                .expect("validated V30MZ instructions have a V-series instruction form"),
        )
    }
}

impl TryFrom<VSeriesInstruction> for Instruction {
    type Error = EncodeError;

    fn try_from(value: VSeriesInstruction) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// The common V-series form of a V30MZ-valid instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionForm(v30::InstructionForm);

impl InstructionForm {
    pub fn as_v_series(&self) -> &v30::InstructionForm {
        &self.0
    }
}

/// A V30MZ prefix byte preserved in source order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prefix {
    Segment(SegmentRegister),
    BusLock,
    Repne,
    Rep,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PrefixSequence(Vec<Prefix>);

impl PrefixSequence {
    pub fn as_slice(&self) -> &[Prefix] {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// One strictly decoded V30MZ instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedInstruction {
    instruction: Instruction,
    byte_len: usize,
    prefixes: PrefixSequence,
    source_bytes: Vec<u8>,
}

impl DecodedInstruction {
    pub fn instruction(&self) -> &Instruction {
        &self.instruction
    }

    pub fn into_instruction(self) -> Instruction {
        self.instruction
    }

    pub fn byte_len(&self) -> usize {
        self.byte_len
    }

    pub fn prefixes(&self) -> &PrefixSequence {
        &self.prefixes
    }

    pub fn source_bytes(&self) -> &[u8] {
        &self.source_bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    #[error("input bytes truncated: needed at least {needed}, got {got}")]
    Truncated { needed: usize, got: usize },
    #[error("undefined V30MZ primary opcode {opcode:#04X}")]
    UndefinedPrimaryOpcode { opcode: u8 },
    #[error("invalid V30MZ encoding")]
    InvalidEncoding,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EncodeError {
    #[error("instruction uses undefined V30MZ primary opcode {opcode:#04X}")]
    UndefinedPrimaryOpcode { opcode: u8 },
    #[error(transparent)]
    BaseProfile(v30::EncodeError),
}

/// Decode the first V30MZ instruction in `bytes`.
pub fn decode_bytes(bytes: &[u8]) -> Result<DecodedInstruction, DecodeError> {
    if bytes.is_empty() {
        return Err(DecodeError::Truncated { needed: 1, got: 0 });
    }
    if let Some(opcode) = first_primary_opcode(bytes)
        && is_undefined_primary(opcode)
    {
        return Err(DecodeError::UndefinedPrimaryOpcode { opcode });
    }

    let decoded = v30::decode_bytes(bytes).map_err(|error| match error {
        v30::DecodeError::Truncated { needed, got } => DecodeError::Truncated { needed, got },
        v30::DecodeError::ReservedSelector { opcode } => {
            DecodeError::UndefinedPrimaryOpcode { opcode }
        }
        v30::DecodeError::InvalidEncoding => DecodeError::InvalidEncoding,
    })?;
    let source_bytes = decoded.source_bytes().to_vec();
    validate_v30mz_encoding(&source_bytes).map_err(|error| match error {
        EncodeError::UndefinedPrimaryOpcode { opcode } => {
            DecodeError::UndefinedPrimaryOpcode { opcode }
        }
        EncodeError::BaseProfile(_) => DecodeError::InvalidEncoding,
    })?;
    let instruction =
        Instruction::new(decoded.instruction).map_err(|_| DecodeError::InvalidEncoding)?;
    let prefixes = PrefixSequence(parse_prefixes(&source_bytes));
    Ok(DecodedInstruction {
        instruction,
        byte_len: source_bytes.len(),
        prefixes,
        source_bytes,
    })
}

/// Lower a typed V30MZ instruction to canonical bytes.
pub fn encode_bytes(instruction: &Instruction) -> Result<Vec<u8>, EncodeError> {
    let bytes = v30::encode_bytes(instruction.as_v_series()).map_err(EncodeError::BaseProfile)?;
    validate_v30mz_encoding(&bytes)?;
    Ok(bytes)
}

pub fn format_instruction(instruction: &Instruction) -> String {
    v30::format_instruction(instruction.as_v_series())
}

fn validate_v30mz_encoding(bytes: &[u8]) -> Result<(), EncodeError> {
    if let Some(opcode) = first_primary_opcode(bytes)
        && is_undefined_primary(opcode)
    {
        return Err(EncodeError::UndefinedPrimaryOpcode { opcode });
    }
    Ok(())
}

fn first_primary_opcode(bytes: &[u8]) -> Option<u8> {
    bytes
        .iter()
        .copied()
        .take(15)
        .find(|byte| !is_supported_prefix(*byte))
}

const fn is_supported_prefix(byte: u8) -> bool {
    matches!(byte, 0x26 | 0x2e | 0x36 | 0x3e | 0xf0 | 0xf2 | 0xf3)
}

const fn is_undefined_primary(byte: u8) -> bool {
    matches!(byte, 0x0f | 0x63 | 0x64 | 0x65 | 0x66 | 0x67 | 0xd6 | 0xf1)
}

fn parse_prefixes(bytes: &[u8]) -> Vec<Prefix> {
    bytes
        .iter()
        .copied()
        .map_while(|byte| match byte {
            0x26 => Some(Prefix::Segment(SegmentRegister::ES)),
            0x2e => Some(Prefix::Segment(SegmentRegister::CS)),
            0x36 => Some(Prefix::Segment(SegmentRegister::SS)),
            0x3e => Some(Prefix::Segment(SegmentRegister::DS)),
            0xf0 => Some(Prefix::BusLock),
            0xf2 => Some(Prefix::Repne),
            0xf3 => Some(Prefix::Rep),
            _ => None,
        })
        .collect()
}

impl TypedIsa for V30Mz {
    const PROFILE_ID: &'static str = PROFILE_ID;

    type Instruction = Instruction;
    type Form = InstructionForm;
    type DecodeError = DecodeError;
    type EncodeError = EncodeError;

    fn form(instruction: &Self::Instruction) -> Result<Self::Form, Self::EncodeError> {
        Ok(instruction.form())
    }

    fn decode_one(
        source: &[u8],
        _address: u64,
    ) -> Result<ProtocolDecodedInstruction<Self::Instruction>, Self::DecodeError> {
        let decoded = decode_bytes(source)?;
        let exact_source = InstructionBytes::new(decoded.source_bytes().to_vec())
            .expect("decoded V30MZ instructions are non-empty");
        Ok(ProtocolDecodedInstruction::new(
            decoded.into_instruction(),
            exact_source,
        ))
    }

    fn encode_one(
        instruction: &Self::Instruction,
        _address: u64,
    ) -> Result<InstructionBytes, Self::EncodeError> {
        let bytes = encode_bytes(instruction)?;
        Ok(InstructionBytes::new(bytes).expect("encoded V30MZ instructions are non-empty"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedInstruction {
    pub address: InstructionAddress,
    pub offset: usize,
    pub byte_len: usize,
    pub instruction: Instruction,
    pub control_flow: ControlFlow,
}

pub fn verify_placed_program(
    bytes: &[u8],
    origin: InstructionAddress,
) -> Result<Vec<PlacedInstruction>, ProgramVerificationError> {
    if usize::from(origin.off) + bytes.len() > 0x1_0000 {
        return Err(ProgramVerificationError::CodeSegmentBoundary {
            origin,
            byte_len: bytes.len(),
        });
    }

    let mut instructions = Vec::new();
    let mut offset = 0usize;
    while offset < bytes.len() {
        let address = InstructionAddress {
            seg: origin.seg,
            off: origin.off + offset as u16,
        };
        let decoded = decode_bytes(&bytes[offset..])
            .map_err(|source| ProgramVerificationError::Decode { address, source })?;
        let reencoded = encode_bytes(decoded.instruction())
            .map_err(|source| ProgramVerificationError::Encode { address, source })?;
        if reencoded != decoded.source_bytes() {
            return Err(ProgramVerificationError::NonCanonicalEncoding { address });
        }
        let next = InstructionAddress {
            seg: address.seg,
            off: address.off + decoded.byte_len() as u16,
        };
        let control_flow = v30::control_flow(decoded.instruction().as_v_series(), next);
        let byte_len = decoded.byte_len();
        instructions.push(PlacedInstruction {
            address,
            offset,
            byte_len,
            instruction: decoded.into_instruction(),
            control_flow,
        });
        offset += byte_len;
    }
    Ok(instructions)
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProgramVerificationError {
    #[error("program at {origin:?} with {byte_len} bytes crosses the 64 KiB code segment")]
    CodeSegmentBoundary {
        origin: InstructionAddress,
        byte_len: usize,
    },
    #[error("decode failed at {address:?}: {source}")]
    Decode {
        address: InstructionAddress,
        source: DecodeError,
    },
    #[error("encode failed at {address:?}: {source}")]
    Encode {
        address: InstructionAddress,
        source: EncodeError,
    },
    #[error("instruction at {address:?} is not in canonical generated-code form")]
    NonCanonicalEncoding { address: InstructionAddress },
}
