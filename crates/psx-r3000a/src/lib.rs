//! PlayStation R3000A instruction semantics and assembly primitives.
//!
//! The declared profile covers the MIPS I integer core used by PlayStation
//! hooks, the PS1 COP0 transfer/RFE forms, COP2 register transfers and memory
//! forms, and the 22 documented GTE commands. Decoding is fail-closed for
//! unsupported opcodes, unknown command functions, and non-zero reserved
//! fields.

#![deny(missing_docs)]

/// Stable identifier for the CPU profile implemented by this crate.
pub const PROFILE_ID: &str = env!("CARGO_PKG_NAME");

/// Aligned 32-bit virtual address of PlayStation executable code.
pub type CodeLocation = u32;

mod assembler;
mod codec;
mod instruction;
mod semantics;

pub use assembler::{AssembledProgram, Assembler, AssemblerError, InstructionSpan, load_address};
pub use codec::{
    DecodeError, EncodeError, VerifyError, decode, decode_bytes, decode_le_bytes, decode_word,
    encode, encode_bytes, encode_le_bytes, encode_word, verify_placed_program,
};
pub use instruction::{GteCommand, GteOpcode, Instruction, InstructionForm, Register};
pub use semantics::{
    Boundary, ControlFlow, Cop0Register, Cop0StatusField, GteControlRegister, GteDataRegister,
    IndirectTarget, InstructionSemantics, Location, MemoryLocation, Reference, SemanticError,
};

/// Shared-protocol marker for the PlayStation R3000A profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PsxR3000A;

impl typed_isa_core::TypedIsa for PsxR3000A {
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
        at: &Self::CodeLocation,
    ) -> Result<typed_isa_core::DecodedInstruction<Self::Instruction>, Self::DecodeError> {
        let instruction = decode_bytes(source, *at)?;
        let source = typed_isa_core::InstructionBytes::new(source[..4].to_vec())
            .expect("decoded PlayStation R3000A instructions are non-empty");
        Ok(typed_isa_core::DecodedInstruction::new(instruction, source))
    }

    fn encode_one(
        instruction: &Self::Instruction,
        at: &Self::CodeLocation,
    ) -> Result<typed_isa_core::InstructionBytes, Self::EncodeError> {
        Ok(
            typed_isa_core::InstructionBytes::new(encode_bytes(instruction, *at)?.to_vec())
                .expect("encoded PlayStation R3000A instructions are non-empty"),
        )
    }
}

impl typed_isa_core::StaticSemantics for PsxR3000A {
    type AnalysisPoint = CodeLocation;
    type IndirectTarget = IndirectTarget;
    type Boundary = Boundary;
    type Location = Location;
    type Reference = Reference;
    type SemanticError = SemanticError;

    fn semantics(
        instruction: &Self::Instruction,
        at: &Self::AnalysisPoint,
    ) -> Result<InstructionSemantics, Self::SemanticError> {
        semantics::semantics(instruction, *at)
    }

    fn storage_relation(
        left: &Self::Location,
        right: &Self::Location,
    ) -> typed_isa_core::StorageRelation {
        semantics::storage_relation(left, right)
    }
}
