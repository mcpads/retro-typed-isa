//! Static Toshiba TLCS-900/H instruction model for Neo Geo Pocket code.
//!
//! The profile follows Toshiba's TLCS-900/H CPU instruction manuals, including
//! the complete primary, register, source-memory, and destination-memory code
//! maps published in the TMP95C061B instruction-manual set. The TLCS-900
//! `NORMAL` and `MAX` selectors are rejected because the TLCS-900/H map assigns
//! dummy instructions to those bytes. The source manuals are linked from
//! [Toshiba's product page](https://toshiba.semicon-storage.com/ap-en/semiconductor/product/microcontrollers/detail.TMP95C061BDFG.html).
//! Execution and simulation are out of scope.

#![deny(missing_docs)]

/// Stable identifier of the TLCS-900/H protocol profile.
pub const PROFILE_ID: &str = env!("CARGO_PKG_NAME");

mod assembler;
mod codec;
mod instruction;

pub use assembler::{AssembledProgram, Assembler, AssemblerError, InstructionSpan};
pub use codec::{DecodeError, DecodedInstruction, EncodeError, decode_bytes, encode_bytes};
pub use instruction::{
    ArithmeticOperation, BitOperation, Condition, CurrentRegister, DestinationMemoryInstruction,
    DestinationMemoryOperation, GeneralRegister, Instruction, InstructionForm, MainForm,
    MainInstruction, MemoryAddress, OperandSize, RegisterEncoding, RegisterError, RegisterForm,
    RegisterInstruction, RegisterOperation, ShiftOperation, SourceMemoryForm,
    SourceMemoryInstruction, SourceMemoryOperation, Step,
};

/// Toshiba TLCS-900/H implementation of the shared static ISA protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tlcs900H;
