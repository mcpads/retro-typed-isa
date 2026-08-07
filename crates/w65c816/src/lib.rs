//! Complete W65C816 instruction encoding plus placement-aware assembly.
//!
//! The crate deliberately separates CPU facts from game-specific ROM layout.
//! All 256 opcode values belong to the declared W65C816 profile. Immediate
//! operand length always requires an explicit accumulator/index width state.
//! Arbitrary data can be emitted by the assembler, but it is never accepted as
//! a fallback for an unsupported instruction.

#![deny(missing_docs)]

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

/// W65C816 profiles whose accumulator and index widths are externally known.
pub trait W65C816Profile:
    typed_isa_core::TypedIsa<Instruction = Instruction, Form = InstructionForm>
{
    /// Width state fixed by this static profile.
    const WIDTHS: WidthState;
}

/// Static profile with eight-bit accumulator and index registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W65C816M8X8;
/// Static profile with eight-bit accumulator and sixteen-bit index registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W65C816M8X16;
/// Static profile with sixteen-bit accumulator and eight-bit index registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W65C816M16X8;
/// Static profile with sixteen-bit accumulator and index registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct W65C816M16X16;

macro_rules! implement_profile {
    ($profile:ty, $id:expr, $widths:expr) => {
        impl typed_isa_core::TypedIsa for $profile {
            const PROFILE_ID: &'static str = $id;

            type Instruction = Instruction;
            type Form = InstructionForm;
            type DecodeError = DecodeError;
            type EncodeError = EncodeError;

            fn form(instruction: &Self::Instruction) -> Result<Self::Form, Self::EncodeError> {
                instruction.encoded_len($widths)?;
                Ok(instruction.form())
            }

            fn decode_one(
                source: &[u8],
                _address: u64,
            ) -> Result<typed_isa_core::DecodedInstruction<Self::Instruction>, Self::DecodeError>
            {
                let decoded = decode_bytes(source, $widths)?;
                let source = typed_isa_core::InstructionBytes::new(source[..decoded.len].to_vec())
                    .expect("decoded W65C816 instructions are non-empty");
                Ok(typed_isa_core::DecodedInstruction::new(
                    decoded.instruction,
                    source,
                ))
            }

            fn encode_one(
                instruction: &Self::Instruction,
                _address: u64,
            ) -> Result<typed_isa_core::InstructionBytes, Self::EncodeError> {
                Ok(
                    typed_isa_core::InstructionBytes::new(encode_bytes(instruction, $widths)?)
                        .expect("encoded W65C816 instructions are non-empty"),
                )
            }
        }

        impl W65C816Profile for $profile {
            const WIDTHS: WidthState = $widths;
        }
    };
}

implement_profile!(W65C816M8X8, "w65c816-m8-x8", WidthState::M8_X8);
implement_profile!(W65C816M8X16, "w65c816-m8-x16", WidthState::M8_X16);
implement_profile!(W65C816M16X8, "w65c816-m16-x8", WidthState::M16_X8);
implement_profile!(W65C816M16X16, "w65c816-m16-x16", WidthState::M16_X16);

// Kept until the existing consumer can be migrated in its own authorized change.
#[doc(hidden)]
/// Compatibility alias for the checked assembler.
pub type Program = Assembler;
#[doc(hidden)]
/// Compatibility alias for [`AssemblerError`].
pub type AsmError = AssemblerError;
#[doc(hidden)]
/// Compatibility alias for [`DisassemblyError`].
pub type DisasmError = DisassemblyError;
