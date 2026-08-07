//! Complete static instruction model for the Sharp SM83 CPU used by Game Boy systems.
//!
//! All base and CB-prefixed opcode selectors are classified. Valid instructions
//! decode to typed operands and lower back to exact machine code. Illegal
//! selectors fail closed. Execution state, cycle timing, peripherals, and
//! simulation are outside this crate.
//!
//! The encoding profile follows the Pan Docs CPU Instruction Set and the RGBDS
//! `gbz80(7)` opcode reference. The model is implemented locally rather than
//! exposing an emulator or third-party decoder as its semantic API.

#![deny(missing_docs)]

use std::convert::Infallible;

use typed_isa_core::{
    DecodedInstruction as ProtocolDecodedInstruction, InstructionBytes, TypedIsa,
};

/// Stable identifier for the exact CPU profile implemented by this crate.
pub const PROFILE_ID: &str = env!("CARGO_PKG_NAME");

/// SM83 implementation of the workspace-wide static ISA protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sm83;

mod assembler;
mod codec;
mod instruction;

pub use assembler::{AssembledProgram, Assembler, AssemblerError, InstructionSpan};
pub use codec::{
    BaseOpcodeClass, CanonicalInstruction, DecodeError, DecodedInstruction, canonical_instructions,
    classify_base_opcode, decode_bytes, encode_bytes,
};
pub use instruction::{
    AluOperation, BitIndex, ByteOperand, Condition, IndirectAddress, Instruction, InstructionForm,
    Register16, RestartVector, RotateShiftOperation, StackRegister,
};

impl TypedIsa for Sm83 {
    const PROFILE_ID: &'static str = PROFILE_ID;

    type Instruction = Instruction;
    type Form = InstructionForm;
    type DecodeError = DecodeError;
    type EncodeError = Infallible;

    fn form(instruction: &Self::Instruction) -> Result<Self::Form, Self::EncodeError> {
        Ok(instruction.form())
    }

    fn decode_one(
        source: &[u8],
        _address: u64,
    ) -> Result<ProtocolDecodedInstruction<Self::Instruction>, Self::DecodeError> {
        let decoded = decode_bytes(source)?;
        let exact_source = InstructionBytes::new(source[..decoded.length()].to_vec())
            .expect("decoded SM83 instructions are non-empty");
        Ok(ProtocolDecodedInstruction::new(
            decoded.into_instruction(),
            exact_source,
        ))
    }

    fn encode_one(
        instruction: &Self::Instruction,
        _address: u64,
    ) -> Result<InstructionBytes, Self::EncodeError> {
        Ok(InstructionBytes::new(encode_bytes(instruction))
            .expect("encoded SM83 instructions are non-empty"))
    }
}
