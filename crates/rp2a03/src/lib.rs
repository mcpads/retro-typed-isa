//! Static Ricoh RP2A03 instruction-set profile for Famicom executable code.
//!
//! All 256 physical opcode selectors are classified so original code can be
//! decoded and re-encoded without losing unofficial selector identity. New
//! instructions use canonical selectors, typed operands, and checked
//! placement. Static semantics cover control flow and architectural storage
//! accesses for every selector. CPU execution, the APU, value propagation,
//! cycle timing, and simulation are outside this crate.

#![deny(missing_docs)]

/// Stable identifier for the CPU profile implemented by this crate.
pub const PROFILE_ID: &str = env!("CARGO_PKG_NAME");

/// Wrapping 16-bit RP2A03 program-counter location.
pub type CodeLocation = u16;

mod assembler;
mod codec;
mod display;
mod instruction;
mod semantics;

pub use assembler::{AssembledProgram, Assembler, AssemblerError, InstructionSpan};
pub use codec::{DecodeError, EncodeError, Instruction, Operand, decode_bytes, encode_bytes};
pub use display::{format_bytes, format_instruction};
pub use instruction::{
    AddressingMode, InstructionForm, Mnemonic, OPCODE_TABLE, opcode_is_documented,
};
pub use semantics::{
    Boundary, ControlFlow, IndirectTarget, InstructionSemantics, Location, MemoryAddress,
    Reference, RegisterLocation, ReturnAddressRule, StatusLocation,
};

/// Shared-protocol marker for the Ricoh RP2A03 profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rp2A03;

impl typed_isa_core::TypedIsa for Rp2A03 {
    const PROFILE_ID: &'static str = PROFILE_ID;

    type Instruction = Instruction;
    type Form = InstructionForm;
    type CodeLocation = CodeLocation;
    type DecodeError = DecodeError;
    type EncodeError = EncodeError;

    fn form(instruction: &Self::Instruction) -> Result<Self::Form, Self::EncodeError> {
        Ok(instruction.form())
    }

    fn decode_one(
        source: &[u8],
        _at: &Self::CodeLocation,
    ) -> Result<typed_isa_core::DecodedInstruction<Self::Instruction>, Self::DecodeError> {
        let instruction = decode_bytes(source)?;
        let source =
            typed_isa_core::InstructionBytes::new(source[..instruction.encoded_len()].to_vec())
                .expect("decoded RP2A03 instructions are non-empty");
        Ok(typed_isa_core::DecodedInstruction::new(instruction, source))
    }

    fn encode_one(
        instruction: &Self::Instruction,
        _at: &Self::CodeLocation,
    ) -> Result<typed_isa_core::InstructionBytes, Self::EncodeError> {
        Ok(
            typed_isa_core::InstructionBytes::new(encode_bytes(instruction)?)
                .expect("encoded RP2A03 instructions are non-empty"),
        )
    }
}

impl typed_isa_core::StaticSemantics for Rp2A03 {
    type AnalysisPoint = CodeLocation;
    type IndirectTarget = IndirectTarget;
    type Boundary = Boundary;
    type Location = Location;
    type Reference = Reference;
    type SemanticError = std::convert::Infallible;

    fn semantics(
        instruction: &Self::Instruction,
        at: &Self::AnalysisPoint,
    ) -> Result<
        typed_isa_core::InstructionSemantics<
            Self::AnalysisPoint,
            Self::IndirectTarget,
            Self::Boundary,
            Self::Location,
            Self::Reference,
        >,
        Self::SemanticError,
    > {
        Ok(semantics::semantics(instruction, *at))
    }

    fn storage_relation(
        left: &Self::Location,
        right: &Self::Location,
    ) -> typed_isa_core::StorageRelation {
        semantics::storage_relation(left, right)
    }
}
