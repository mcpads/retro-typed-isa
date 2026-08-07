//! Complete static instruction model for the Ricoh RP2A03 used by the Famicom.
//!
//! All 256 physical opcode selectors are classified so original code can be
//! decoded and re-encoded without losing unofficial selector identity. New
//! instructions use canonical selectors, typed operands, and checked
//! placement. CPU execution, the APU, cycle timing, and simulation are outside
//! this crate.

#![deny(missing_docs)]

/// Stable identifier for the CPU profile implemented by this crate.
pub const PROFILE_ID: &str = env!("CARGO_PKG_NAME");

mod assembler;
mod codec;
mod display;
mod instruction;

pub use assembler::{AssembledProgram, Assembler, AssemblerError, InstructionSpan};
pub use codec::{DecodeError, EncodeError, Instruction, Operand, decode_bytes, encode_bytes};
pub use display::{format_bytes, format_instruction};
pub use instruction::{
    AddressingMode, ControlFlow, InstructionForm, Mnemonic, OPCODE_TABLE, opcode_is_documented,
};

/// Shared-protocol marker for the Ricoh RP2A03 profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rp2A03;

impl typed_isa_core::TypedIsa for Rp2A03 {
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
        let source =
            typed_isa_core::InstructionBytes::new(source[..instruction.encoded_len()].to_vec())
                .expect("decoded RP2A03 instructions are non-empty");
        Ok(typed_isa_core::DecodedInstruction::new(instruction, source))
    }

    fn encode_one(
        instruction: &Self::Instruction,
        _address: u64,
    ) -> Result<typed_isa_core::InstructionBytes, Self::EncodeError> {
        Ok(
            typed_isa_core::InstructionBytes::new(encode_bytes(instruction)?)
                .expect("encoded RP2A03 instructions are non-empty"),
        )
    }
}
