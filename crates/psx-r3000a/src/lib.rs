//! PlayStation R3000A instruction semantics and assembly primitives.
//!
//! The declared profile covers the MIPS I integer core used by PlayStation
//! hooks, the PS1 COP0 transfer/RFE forms, COP2 register transfers and memory
//! forms, and the 22 documented GTE commands. Decoding is fail-closed for
//! unsupported opcodes, unknown command functions, and non-zero reserved
//! fields.

/// Stable identifier for the CPU profile implemented by this crate.
pub const PROFILE_ID: &str = env!("CARGO_PKG_NAME");

mod assembler;
mod codec;
mod instruction;

pub use assembler::{AssembledProgram, Assembler, AssemblerError, load_address};
pub use codec::{
    DecodeError, EncodeError, VerifyError, decode, decode_bytes, decode_le_bytes, decode_word,
    encode, encode_bytes, encode_le_bytes, encode_word, verify_placed_program,
};
pub use instruction::{ControlFlow, GteCommand, GteOpcode, Instruction, InstructionForm, Register};

/// Shared-protocol marker for the PlayStation R3000A profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PsxR3000A;

impl typed_isa_core::TypedIsa for PsxR3000A {
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
        address: u64,
    ) -> Result<typed_isa_core::DecodedInstruction<Self::Instruction>, Self::DecodeError> {
        let address =
            u32::try_from(address).map_err(|_| DecodeError::AddressOutOfRange { address })?;
        let instruction = decode_bytes(source, address)?;
        let source = typed_isa_core::InstructionBytes::new(source[..4].to_vec())
            .expect("decoded PlayStation R3000A instructions are non-empty");
        Ok(typed_isa_core::DecodedInstruction::new(instruction, source))
    }

    fn encode_one(
        instruction: &Self::Instruction,
        address: u64,
    ) -> Result<typed_isa_core::InstructionBytes, Self::EncodeError> {
        let address =
            u32::try_from(address).map_err(|_| EncodeError::AddressOutOfRange { address })?;
        Ok(
            typed_isa_core::InstructionBytes::new(encode_bytes(instruction, address)?.to_vec())
                .expect("encoded PlayStation R3000A instructions are non-empty"),
        )
    }
}
