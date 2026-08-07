//! Typed Motorola MC68000 instruction model and checked byte lowering.
//!
//! This crate models the MC68000 instruction set. It does not include the
//! MC68010-only `BKPT`, `MOVEC`, `MOVES`, or `RTD` instructions, and it does
//! not execute instructions or model cycle timing.

mod assembler;
mod codec;
mod instruction;

pub use assembler::{
    AssembledProgram, Assembler, AssemblerError, PlacedInstruction, verify_placed_program,
};
pub use codec::{
    DecodeError, DecodedInstruction, DecodedWord, EncodeError, classify_word, decode_bytes,
    encode_bytes,
};
pub use instruction::*;

/// Stable identifier for the exact CPU profile implemented by this crate.
pub const PROFILE_ID: &str = "m68000";

/// Shared-protocol marker for the Motorola MC68000 profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mc68000;

impl typed_isa_core::TypedIsa for Mc68000 {
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
    ) -> Result<typed_isa_core::DecodedInstruction<Self::Instruction>, Self::DecodeError> {
        let decoded = decode_bytes(source)?;
        let source = typed_isa_core::InstructionBytes::new(source[..decoded.byte_len].to_vec())
            .expect("decoded MC68000 instructions are non-empty");
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
                .expect("encoded MC68000 instructions are non-empty"),
        )
    }
}
