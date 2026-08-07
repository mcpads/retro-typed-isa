//! NEC V30 native-mode typed instruction model and static byte analysis.
//!
//! `BRKEM` is classified as a transfer out of the native static-analysis
//! boundary. This crate does not execute V30 instructions or decode the
//! subsequent 8080-emulation instruction stream.

mod analysis;
mod assembler;
mod decode;
#[cfg(test)]
mod decode_tests;
mod display;
#[cfg(test)]
mod display_tests;
mod emit;
#[cfg(test)]
mod emit_tests;
mod expected_write;
mod form;
mod instruction;
#[cfg(test)]
mod instruction_tests;
mod selector;

pub use analysis::{
    ControlFlow, PlacedInstruction, ProgramVerificationError, control_flow, verify_placed_program,
};
pub use assembler::{AssembledProgram, Assembler, AssemblerError};
pub use expected_write::ExpectedWriteVerifier;
pub use form::{CanonicalInstruction, InstructionForm, Mnemonic, OperandForm, SemanticPrefix};
pub use selector::{
    ModRmEncodingClass, PrimaryOpcodeClass, SecondaryOpcodeClass, SecondaryOperandEncoding,
    classify_primary_modrm, classify_primary_opcode, classify_secondary_modrm,
    classify_secondary_opcode,
};

pub use instruction::{
    BitFieldLength, BitFieldLengthError, BitIndex, CallTarget, Cond as Condition,
    EffectiveAddressDisplacement, EffectiveAddressError, EscapeOpcode, EscapeOpcodeError,
    ExternalRegister, ExternalRegisterError, FloatingPointOperand, Fpo2Opcode, Fpo2OpcodeError,
    InstructionKind as Instruction, JmpTarget, LoopCondition, Mem as EffectiveAddress,
    MemBase as EffectiveAddressBase, OpSize as OperandSize, Operand, PortAddr as PortAddress,
    Reg8 as Register8, Reg16 as Register16, RegisterIndexError, SegOff as InstructionAddress,
    ShiftCount, SignedImmediate, Sreg as SegmentRegister,
};

/// Stable identifier for the exact CPU profile implemented by this crate.
pub const PROFILE_ID: &str = "v30";

/// Shared-protocol marker for the NEC V30 native-mode profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct V30;

/// A native V30 prefix byte preserved in source order by static decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prefix {
    Segment(SegmentRegister),
    BusLock,
    Repne,
    Rep,
    Repnc,
    Repc,
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

/// One strictly decoded native-mode instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedInstruction {
    pub instruction: Instruction,
    pub byte_len: usize,
    pub prefixes: PrefixSequence,
    source_bytes: Vec<u8>,
}

impl DecodedInstruction {
    /// Exact validated source serialization consumed by this decode result.
    pub fn source_bytes(&self) -> &[u8] {
        &self.source_bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    #[error("input bytes truncated: needed at least {needed}, got {got}")]
    Truncated { needed: usize, got: usize },
    #[error("reserved or unsupported native V30 selector at byte {opcode:#04X}")]
    ReservedSelector { opcode: u8 },
    #[error("invalid native V30 encoding")]
    InvalidEncoding,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EncodeError {
    #[error("instruction has no valid native V30 encoding")]
    InvalidInstruction,
    #[error("raw or reserved bytes cannot be emitted as an instruction")]
    RawBytesNotExecutable,
}

/// Decode the first native-mode V30 instruction in `bytes`.
pub fn decode_bytes(bytes: &[u8]) -> Result<DecodedInstruction, DecodeError> {
    if bytes.is_empty() {
        return Err(DecodeError::Truncated { needed: 1, got: 0 });
    }
    let decoded = decode::decode(bytes, InstructionAddress { seg: 0, off: 0 }).map_err(
        |error| match error {
            decode::DecodeError::Truncated { needed, got } => {
                DecodeError::Truncated { needed, got }
            }
        },
    )?;
    if let Some(raw) = unknown_raw(&decoded.kind) {
        if let Some(needed) = inferred_truncated_len(bytes) {
            return Err(DecodeError::Truncated {
                needed,
                got: bytes.len(),
            });
        }
        let opcode = raw.first().copied().unwrap_or(bytes[0]);
        return Err(DecodeError::ReservedSelector { opcode });
    }
    let canonical_bytes = encode_bytes(&decoded.kind).map_err(|_| DecodeError::InvalidEncoding)?;
    let parsed_prefixes = parse_prefixes(&decoded.bytes);
    if let Some(effective_repeat) = parsed_prefixes.iter().rev().copied().find(|prefix| {
        matches!(
            prefix,
            Prefix::Rep | Prefix::Repne | Prefix::Repc | Prefix::Repnc
        )
    }) && !instruction_has_prefix(&decoded.kind, effective_repeat)
    {
        return Err(DecodeError::InvalidEncoding);
    }
    let canonical_prefixes = parse_prefixes(&canonical_bytes);
    let effective_segment = parsed_prefixes
        .iter()
        .rev()
        .copied()
        .find(|prefix| matches!(prefix, Prefix::Segment(_)));
    let canonical_segment = canonical_prefixes
        .iter()
        .copied()
        .find(|prefix| matches!(prefix, Prefix::Segment(_)));
    if effective_segment != canonical_segment {
        return Err(DecodeError::InvalidEncoding);
    }
    let prefixes = PrefixSequence(parsed_prefixes);
    let source_bytes = decoded.bytes;
    Ok(DecodedInstruction {
        instruction: decoded.kind,
        byte_len: source_bytes.len(),
        prefixes,
        source_bytes,
    })
}

fn instruction_has_prefix(instruction: &Instruction, prefix: Prefix) -> bool {
    match (instruction, prefix) {
        (Instruction::Rep(_), Prefix::Rep)
        | (Instruction::Repne(_), Prefix::Repne)
        | (Instruction::Repc(_), Prefix::Repc)
        | (Instruction::Repnc(_), Prefix::Repnc) => true,
        (Instruction::BusLock(inner), prefix) => instruction_has_prefix(inner, prefix),
        (
            Instruction::SegmentOverride {
                instruction: inner, ..
            },
            prefix,
        ) => instruction_has_prefix(inner, prefix),
        _ => false,
    }
}

fn unknown_raw(instruction: &Instruction) -> Option<&[u8]> {
    match instruction {
        Instruction::Unknown { raw } => Some(raw),
        Instruction::Rep(inner)
        | Instruction::Repne(inner)
        | Instruction::Repc(inner)
        | Instruction::Repnc(inner)
        | Instruction::BusLock(inner)
        | Instruction::SegmentOverride {
            instruction: inner, ..
        } => unknown_raw(inner),
        _ => None,
    }
}

fn inferred_truncated_len(bytes: &[u8]) -> Option<usize> {
    let prefix_count = bytes
        .iter()
        .take(15)
        .take_while(|byte| {
            matches!(
                byte,
                0x26 | 0x2e | 0x36 | 0x3e | 0xf0 | 0xf2 | 0xf3 | 0x64 | 0x65
            )
        })
        .count();
    if prefix_count == bytes.len() && prefix_count < 15 {
        return Some(prefix_count + 1);
    }
    let opcode_offset = bytes.iter().take(15).position(|byte| {
        !matches!(
            byte,
            0x26 | 0x2e | 0x36 | 0x3e | 0xf0 | 0xf2 | 0xf3 | 0x64 | 0x65
        )
    });
    if let Some(opcode_offset) = opcode_offset {
        if bytes.len() == opcode_offset + 1 && bytes[opcode_offset] == 0x0f {
            return Some(opcode_offset + 2);
        }
        if bytes.len() == opcode_offset + 1
            && classify_primary_modrm(bytes[opcode_offset], 0) != ModRmEncodingClass::NotUsed
        {
            return Some(opcode_offset + 2);
        }
        if bytes.len() == opcode_offset + 2 && bytes[opcode_offset] == 0x0f {
            if let SecondaryOpcodeClass::Instruction { operands, .. } =
                classify_secondary_opcode(bytes[opcode_offset + 1])
            {
                if !matches!(operands, SecondaryOperandEncoding::None) {
                    return Some(opcode_offset + 3);
                }
            }
        }
    }
    if bytes.len() >= 15 {
        return None;
    }
    let mut probe = bytes.to_vec();
    probe.resize(15, 0);
    let decoded = decode::decode(&probe, InstructionAddress { seg: 0, off: 0 }).ok()?;
    if unknown_raw(&decoded.kind).is_some() || decoded.bytes.len() <= bytes.len() {
        return None;
    }
    encode_bytes(&decoded.kind).ok()?;
    Some(decoded.bytes.len())
}

/// Lower a typed native-mode instruction to canonical bytes.
pub fn encode_bytes(instruction: &Instruction) -> Result<Vec<u8>, EncodeError> {
    if unknown_raw(instruction).is_some() {
        return Err(EncodeError::RawBytesNotExecutable);
    }
    if !has_canonical_prefix_structure(instruction, 0) {
        return Err(EncodeError::InvalidInstruction);
    }
    emit::emit(instruction).map_err(|_| EncodeError::InvalidInstruction)
}

fn has_canonical_prefix_structure(instruction: &Instruction, minimum_rank: u8) -> bool {
    let (rank, inner) = match instruction {
        Instruction::BusLock(inner) => (0, inner.as_ref()),
        Instruction::SegmentOverride {
            instruction: inner, ..
        } => (1, inner.as_ref()),
        Instruction::Rep(inner)
        | Instruction::Repne(inner)
        | Instruction::Repc(inner)
        | Instruction::Repnc(inner) => (2, inner.as_ref()),
        _ => return true,
    };
    rank >= minimum_rank && has_canonical_prefix_structure(inner, rank + 1)
}

pub fn format_instruction(instruction: &Instruction) -> String {
    display::format_kind(instruction)
}

impl typed_isa_core::TypedIsa for V30 {
    const PROFILE_ID: &'static str = PROFILE_ID;

    type Instruction = Instruction;
    type Form = InstructionForm;
    type DecodeError = DecodeError;
    type EncodeError = EncodeError;

    fn form(instruction: &Self::Instruction) -> Result<Self::Form, Self::EncodeError> {
        InstructionForm::of(instruction).ok_or(EncodeError::RawBytesNotExecutable)
    }

    fn decode_one(
        source: &[u8],
        _address: u64,
    ) -> Result<typed_isa_core::DecodedInstruction<Self::Instruction>, Self::DecodeError> {
        let decoded = decode_bytes(source)?;
        let source = typed_isa_core::InstructionBytes::new(decoded.source_bytes().to_vec())
            .expect("decoded V30 instructions are non-empty");
        Ok(typed_isa_core::DecodedInstruction::new(
            decoded.instruction,
            source,
        ))
    }

    fn encode_one(
        instruction: &Self::Instruction,
        _address: u64,
    ) -> Result<typed_isa_core::InstructionBytes, Self::EncodeError> {
        Ok(
            typed_isa_core::InstructionBytes::new(encode_bytes(instruction)?)
                .expect("encoded V30 instructions are non-empty"),
        )
    }
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
            0x64 => Some(Prefix::Repnc),
            0x65 => Some(Prefix::Repc),
            _ => None,
        })
        .collect()
}
