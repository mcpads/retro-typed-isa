//! Zilog-documented Z80 instruction semantics and checked byte generation.
//!
//! Undocumented silicon behavior, redundant prefixes, execution state, cycle
//! timing, and simulation are deliberately outside this profile.

pub const PROFILE_ID: &str = env!("CARGO_PKG_NAME");

mod assembler;
mod codec;
mod display;
mod encoder;
mod instruction;

pub use assembler::{AssembledProgram, Assembler, AssemblerError};
pub use codec::{DecodeError, DecodedInstruction, decode_bytes};
pub use encoder::{EncodeError, VerifyError, encode_bytes, verify_placed_program};
pub use instruction::{
    AluOperation, ByteOperand, Condition, ControlFlow, IndexRegister, Instruction, InstructionForm,
    InterruptMode, Register16, RotateShiftOperation, StackRegister,
};
