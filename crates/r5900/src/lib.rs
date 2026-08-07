//! Static Sony/Toshiba R5900 instruction-set profile for PlayStation 2 EE code.
//!
//! The normative sources are Sony Computer Entertainment's
//! [*EE Core Instruction Set Manual*], [*EE Core User's Manual*], and
//! [*VU User's Manual*], Version 6.0, April 2002. This crate covers the CPU,
//! EE Core multimedia, COP0, COP1, and VU0 macro-mode COP2 instruction sets
//! documented by those manuals.
//!
//! Instructions are fixed-width and serialize in the little-endian byte order
//! used by PlayStation 2 executable images. Execution, timing, pipelines, and
//! simulation are outside this crate.
//!
//! [*EE Core Instruction Set Manual*]: https://github.com/DarrenRainey/PS2-Programming-Docs/blob/master/EE_Core_Instruction_Set_Manual.pdf
//! [*EE Core User's Manual*]: https://github.com/DarrenRainey/PS2-Programming-Docs/blob/master/EE_Core_Users_Manual.pdf
//! [*VU User's Manual*]: https://github.com/DarrenRainey/PS2-Programming-Docs/blob/master/VU_Users_Manual.pdf

pub const PROFILE_ID: &str = env!("CARGO_PKG_NAME");

mod assembler;
mod codec;
mod expected_write;
mod forms;
mod instruction;

pub use assembler::{AssembledProgram, Assembler, AssemblerError};
pub use codec::{
    DecodeError, EncodeError, R5900, decode_bytes, decode_word, encode_bytes, encode_word,
};
pub use expected_write::ExpectedWriteVerifier;
pub use forms::InstructionForm;
pub use instruction::*;
pub use typed_isa_core::InstructionSpan;

pub fn format_instruction(instruction: &Instruction) -> String {
    format!("{:?}", instruction)
}
