//! Static Sony Allegrex instruction-set profile for PSP executable code.
//!
//! The normative sources are Sony Computer Entertainment's *ALLEGREX
//! Instruction Manual*, [*FPU Instruction Manual*][fpu-manual], and *VFPU Instruction
//! Manual*, all from PSP Hardware Manual Release 1.0.0. This crate covers the
//! instruction encodings documented by those three manuals. It does not use an
//! external disassembler as a completeness oracle.
//!
//! The FPU manual's fixed word conversions use function selectors 0x0C-0x0F
//! (ROUND, TRUNC, CEIL, FLOOR; pages 45, 52, 59, 63-64). Selectors 0x08-0x0B
//! are unassigned in this profile.
//!
//! [fpu-manual]: https://github.com/Decompollaborate/rabbitizer/files/14950191/FPU-Instruction_Manual-English.pdf
//!
//! Instructions are fixed-width and serialize in the little-endian byte order
//! used by PSP executable images. Execution, timing, pipeline state, and
//! simulation are outside this crate.

#![deny(missing_docs)]

/// Stable identifier for the little-endian Allegrex protocol profile.
pub const PROFILE_ID: &str = env!("CARGO_PKG_NAME");

/// Aligned 32-bit Allegrex executable address.
pub type CodeLocation = u32;

mod assembler;
mod codec;
mod expected_write;
mod instruction;
mod semantics;
mod vfpu;

pub use assembler::{AssembledProgram, Assembler, AssemblerError};
pub use codec::{
    Allegrex, DecodeError, EncodeError, decode_bytes, decode_word, encode_bytes, encode_word,
};
pub use expected_write::ExpectedWriteVerifier;
pub use instruction::*;
pub use semantics::{
    AnalysisPoint, Boundary, ControlFlow, DestinationPrefixState, FpuState, ImplicitState,
    IndirectTarget, InstructionSemantics, Location, MemoryAddress, MemoryLocation, MemoryWidth,
    Reference, SemanticError, SourcePrefixState, VfpuPrefixState,
};
pub use typed_isa_core::InstructionSpan;
pub use vfpu::*;

/// Formats one typed instruction using a stable debug-oriented representation.
pub fn format_instruction(instruction: &Instruction) -> String {
    format!("{:?}", instruction)
}
