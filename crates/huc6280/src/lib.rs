//! Complete HuC6280 instruction encoding and placement-aware assembly.
//!
//! The profile includes all 256 hardware opcode selectors. Hudson's documented
//! forms and the 22 one-byte undocumented NOP aliases are distinguished at the
//! opcode layer while sharing the same semantic `NOP` instruction form. This
//! crate models instruction bytes and static placement only; it does not model
//! CPU execution, MPR state transitions, cycle timing, or simulation.

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
