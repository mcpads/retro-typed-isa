//! Hitachi SH-2 instruction semantics, byte lowering, and checked assembly.
//!
//! This crate covers the fixed-width 16-bit SH-2 CPU instruction profile. It
//! deliberately excludes SH-DSP and later SuperH extensions, execution state,
//! pipeline timing, and simulation.

/// Stable identifier for the CPU profile implemented by this crate.
pub const PROFILE_ID: &str = env!("CARGO_PKG_NAME");

mod assembler;
mod codec;
mod display;
mod encoder;
mod instruction;

pub use assembler::{AssembledProgram, Assembler, AssemblerError};
pub use codec::{DecodeError, DecodedWord, classify_word, decode_bytes, decode_word};
pub use encoder::{EncodeError, VerifyError, encode_bytes, encode_word, verify_placed_program};
pub use instruction::{ControlFlow, Instruction, InstructionForm, Register};

/// Shared-protocol marker for the SH-2 profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sh2;

impl typed_isa_core::TypedIsa for Sh2 {
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
        let instruction = decode_bytes(source)?;
        let source = typed_isa_core::InstructionBytes::new(source[..2].to_vec())
            .expect("decoded SH-2 instructions are non-empty");
        Ok(typed_isa_core::DecodedInstruction::new(instruction, source))
    }

    fn encode_one(
        instruction: &Self::Instruction,
        _address: u64,
    ) -> Result<typed_isa_core::InstructionBytes, Self::EncodeError> {
        Ok(
            typed_isa_core::InstructionBytes::new(encode_bytes(instruction)?.to_vec())
                .expect("encoded SH-2 instructions are non-empty"),
        )
    }
}
