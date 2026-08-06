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
