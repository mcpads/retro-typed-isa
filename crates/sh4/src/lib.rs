//! Hitachi SH-4 instruction semantics, byte lowering, and checked assembly.
//!
//! This crate covers the complete fixed-width 16-bit SH-4 CPU and FPU
//! instruction profile from the SH-4 Software Manual. Byte APIs use the
//! little-endian serialization used by the imported Dreamcast consumers;
//! natural word APIs are endian-neutral. FPSCR.PR and FPSCR.SZ are explicit
//! decode inputs. Execution, pipeline timing, scheduling, and simulation are
//! deliberately excluded.

/// Stable identifier for the CPU profile implemented by this crate.
pub const PROFILE_ID: &str = env!("CARGO_PKG_NAME");

mod assembler;
mod codec;
mod display;
mod encoder;
mod instruction;

pub use assembler::{
    AssembledProgram, Assembler, AssemblerError, LiteralValue, PlacedInstruction, PlacedLiteral,
};
pub use codec::{DecodeError, DecodedWord, classify_word, decode_bytes, decode_word};
pub use encoder::{EncodeError, VerifyError, encode_bytes, encode_word, verify_placed_program};
pub use instruction::{
    BankedRegister, ControlFlow, DoubleRegister, ExtendedDoubleRegister, FloatRegister, FpscrState,
    FpuPairRegister, FpuPrecision, FpuTransferSize, Instruction, InstructionForm, Register,
    VectorRegister,
};
