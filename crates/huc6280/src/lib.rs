//! Complete HuC6280 instruction encoding and placement-aware assembly.
//!
//! The profile models all 234 opcode selectors defined by Hudson's software
//! manual. The remaining 22 selector slots are classified as undocumented and
//! fail closed. This crate models instruction bytes and static placement only;
//! it does not model CPU execution, MPR state transitions, cycle timing, or
//! simulation.

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
    AddressingMode, ControlFlow, InstructionForm, Mnemonic, instruction_form, opcode_is_documented,
};

/// Shared-protocol marker for the HuC6280 profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuC6280;

impl typed_isa_core::TypedIsa for HuC6280 {
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
                .expect("decoded HuC6280 instructions are non-empty");
        Ok(typed_isa_core::DecodedInstruction::new(instruction, source))
    }

    fn encode_one(
        instruction: &Self::Instruction,
        _address: u64,
    ) -> Result<typed_isa_core::InstructionBytes, Self::EncodeError> {
        Ok(
            typed_isa_core::InstructionBytes::new(encode_bytes(instruction)?)
                .expect("encoded HuC6280 instructions are non-empty"),
        )
    }
}
