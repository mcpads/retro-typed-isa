//! Complete W65C816 instruction encoding plus placement-aware assembly.
//!
//! The crate deliberately separates CPU facts from game-specific ROM layout.
//! All 256 opcode values belong to the declared W65C816 profile. Immediate
//! operand length always requires an explicit accumulator/index width state.
//! Arbitrary data can be emitted by the assembler, but it is never accepted as
//! a fallback for an unsupported instruction.

/// Stable identifier for the CPU profile implemented by this crate.
pub const PROFILE_ID: &str = env!("CARGO_PKG_NAME");

mod assembler;
mod codec;
mod disassembly;
mod isa;

pub use assembler::{AssembledProgram, Assembler, AssemblerError, CpuMode, InstructionSpan};
pub use codec::{
    DecodeError, DecodedInstruction, EncodeError, Instruction, Operand, decode, decode_bytes,
    encode, encode_bytes,
};
pub use disassembly::{
    ControlFlow, Disassembly, DisassemblyError, PlacedInstruction, disassemble_linear,
};
pub use isa::{
    AddressingMode, InstructionForm, Mnemonic, OPCODE_TABLE, Width, WidthState, opcode_for,
};

// Kept until the existing consumer can be migrated in its own authorized change.
#[doc(hidden)]
pub type Program = Assembler;
#[doc(hidden)]
pub type AsmError = AssemblerError;
#[doc(hidden)]
pub type DisasmError = DisassemblyError;
