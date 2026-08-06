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
