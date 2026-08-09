//! Zilog-documented Z80 instruction semantics and checked byte generation.
//!
//! Undocumented silicon behavior, redundant prefixes, execution state, cycle
//! timing, and simulation are deliberately outside this profile.

#![deny(missing_docs)]

/// Stable identifier for the exact CPU profile implemented by this crate.
pub const PROFILE_ID: &str = env!("CARGO_PKG_NAME");

/// Wrapping 16-bit Z80 program-counter location.
pub type CodeLocation = u16;

mod assembler;
mod codec;
mod display;
mod encoder;
mod instruction;
mod semantics;

pub use assembler::{AssembledProgram, Assembler, AssemblerError, InstructionSpan};
pub use codec::{DecodeError, DecodedInstruction, decode_bytes};
pub use encoder::{EncodeError, VerifyError, encode_bytes, verify_placed_program};
pub use instruction::{
    AluOperation, ByteOperand, Condition, IndexRegister, Instruction, InstructionForm,
    InterruptMode, Register16, RotateShiftOperation, StackRegister,
};
pub use semantics::{
    Boundary, ByteRegister, ControlFlow, Flag, IndirectTarget, InstructionSemantics, Location,
    MemoryAddress, PortAddress, Reference, SemanticError,
};

/// Shared-protocol marker for the documented Z80 profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Z80;

impl typed_isa_core::TypedIsa for Z80 {
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
        let decoded = decode_bytes(source)?;
        let source = typed_isa_core::InstructionBytes::new(source[..decoded.length()].to_vec())
            .expect("decoded Z80 instructions are non-empty");
        Ok(typed_isa_core::DecodedInstruction::new(
            decoded.into_instruction(),
            source,
        ))
    }

    fn encode_one(
        instruction: &Self::Instruction,
        _at: &Self::CodeLocation,
    ) -> Result<typed_isa_core::InstructionBytes, Self::EncodeError> {
        Ok(
            typed_isa_core::InstructionBytes::new(encode_bytes(instruction)?)
                .expect("encoded Z80 instructions are non-empty"),
        )
    }
}

impl typed_isa_core::StaticSemantics for Z80 {
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
