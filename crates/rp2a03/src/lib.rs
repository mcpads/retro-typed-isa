//! Complete static instruction model for the Ricoh RP2A03 used by the Famicom.
//!
//! All 256 physical opcode selectors are classified so original code can be
//! decoded and re-encoded without losing unofficial selector identity. New
//! instructions use canonical selectors, typed operands, and checked
//! placement. CPU execution, the APU, cycle timing, and simulation are outside
//! this crate.

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
