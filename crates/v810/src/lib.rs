//! Static NEC V810 instruction model for PC-FX executable code.
//!
//! The profile follows NEC document U10082E, *V810 Family 32-Bit
//! Microprocessor Architecture*, including all seven instruction formats,
//! bit-string and floating-point sub-opcodes, reserved primary opcodes, and
//! reserved fields. Instructions lower to the little-endian halfword order
//! specified by the architecture. Execution and simulation are out of scope.

pub const PROFILE_ID: &str = env!("CARGO_PKG_NAME");

mod assembler;
mod codec;
mod instruction;

pub use assembler::{AssembledProgram, Assembler, AssemblerError, InstructionSpan};
pub use codec::{
    DecodeError, DecodedInstruction, EncodeError, decode_bytes, decode_halfwords, encode_bytes,
    encode_halfwords,
};
pub use instruction::{
    BitStringOperation, Condition, FloatingOperation, ImmediateOperation, Instruction,
    InstructionForm, LogicalImmediateOperation, MemoryOperation, Register, RegisterOperation,
    ShiftOperation, SystemRegister,
};

/// NEC V810 implementation of the shared static ISA protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct V810;
