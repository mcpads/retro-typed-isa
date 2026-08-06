//! Typed Motorola MC68000 instruction model and checked byte lowering.
//!
//! This crate models the MC68000 instruction set. It does not include the
//! MC68010-only `BKPT`, `MOVEC`, `MOVES`, or `RTD` instructions, and it does
//! not execute instructions or model cycle timing.

mod assembler;
mod codec;
mod instruction;

pub use assembler::{
    AssembledProgram, Assembler, AssemblerError, PlacedInstruction, verify_placed_program,
};
pub use codec::{
    DecodeError, DecodedInstruction, DecodedWord, EncodeError, classify_word, decode_bytes,
    encode_bytes,
};
pub use instruction::*;

/// Stable identifier for the exact CPU profile implemented by this crate.
pub const PROFILE_ID: &str = "m68000";
