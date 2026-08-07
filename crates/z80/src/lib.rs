//! Zilog-documented Z80 instruction semantics and checked byte generation.
//!
//! Undocumented silicon behavior, redundant prefixes, execution state, cycle
//! timing, and simulation are deliberately outside this profile.

#![deny(missing_docs)]

/// Stable identifier for the exact CPU profile implemented by this crate.
pub const PROFILE_ID: &str = env!("CARGO_PKG_NAME");

mod assembler;
mod codec;
mod display;
mod encoder;
mod instruction;

pub use assembler::{AssembledProgram, Assembler, AssemblerError};
pub use codec::{DecodeError, DecodedInstruction, decode_bytes};
pub use encoder::{EncodeError, VerifyError, encode_bytes, verify_placed_program};
pub use instruction::{
    AluOperation, ByteOperand, Condition, ControlFlow, IndexRegister, Instruction, InstructionForm,
    InterruptMode, Register16, RotateShiftOperation, StackRegister,
};

/// Shared-protocol marker for the documented Z80 profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Z80;

impl typed_isa_core::TypedIsa for Z80 {
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
        let source = typed_isa_core::InstructionBytes::new(source[..decoded.length()].to_vec())
            .expect("decoded Z80 instructions are non-empty");
        Ok(typed_isa_core::DecodedInstruction::new(
            decoded.into_instruction(),
            source,
        ))
    }

    fn encode_one(
        instruction: &Self::Instruction,
        _address: u64,
    ) -> Result<typed_isa_core::InstructionBytes, Self::EncodeError> {
        Ok(
            typed_isa_core::InstructionBytes::new(encode_bytes(instruction)?)
                .expect("encoded Z80 instructions are non-empty"),
        )
    }
}
