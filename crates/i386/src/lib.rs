//! Static Intel 80386 instruction-set profile.
//!
//! The normative source is Intel's *80386 Programmer's Reference Manual*,
//! order number 230985, especially Appendix A's one-byte, two-byte, and ModR/M
//! group maps. The Intel386 SX has the same programmer-visible instruction set
//! as the Intel386 DX; its narrower external bus does not define a different
//! ISA profile.
//!
//! A code segment's D bit selects the default operand and address size, so the
//! shared protocol is implemented by separate code-16 and code-32 profile
//! types. Prefixes may still override either size for one instruction.
//! Execution modes, descriptor lookup, privilege checks, and simulation are
//! outside this crate.
//!
//! [Intel manual]: https://www.read.seas.harvard.edu/~kohler/class/aosref/i386.pdf

mod assembler;
mod codec;
mod expected_write;
mod instruction;
mod selector;

pub use assembler::{AssembledProgram, Assembler, AssemblerError};
pub use codec::{
    DecodeError, DecodedInstruction, EncodeError, Instruction, InstructionConstructionError,
    decode_bytes, encode_bytes,
};
pub use expected_write::ExpectedWriteVerifier;
pub use instruction::*;
pub use selector::*;

pub const PROFILE_ID: &str = env!("CARGO_PKG_NAME");
pub const CODE16_PROFILE_ID: &str = "i386-code16";
pub const CODE32_PROFILE_ID: &str = "i386-code32";

/// Intel 80386 code segments whose D bit is clear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct I386Code16;

/// Intel 80386 code segments whose D bit is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct I386Code32;

pub trait I386Profile:
    typed_isa_core::TypedIsa<Instruction = Instruction, Form = InstructionForm>
{
    const CODE_MODE: CodeMode;
}

pub fn format_instruction(instruction: &Instruction) -> String {
    if instruction.operands().is_empty() {
        format!("{:?}", instruction.operation())
    } else {
        format!("{:?} {:?}", instruction.operation(), instruction.operands())
    }
}

macro_rules! implement_profile {
    ($profile:ty, $id:expr, $mode:expr) => {
        impl typed_isa_core::TypedIsa for $profile {
            const PROFILE_ID: &'static str = $id;

            type Instruction = Instruction;
            type Form = InstructionForm;
            type DecodeError = DecodeError;
            type EncodeError = EncodeError;

            fn form(instruction: &Self::Instruction) -> Result<Self::Form, Self::EncodeError> {
                Ok(instruction.form())
            }

            fn decode_one(
                source: &[u8],
                _address: u64,
            ) -> Result<typed_isa_core::DecodedInstruction<Self::Instruction>, Self::DecodeError>
            {
                let decoded = decode_bytes(source, $mode)?;
                let source = typed_isa_core::InstructionBytes::new(decoded.source_bytes().to_vec())
                    .expect("decoded Intel 80386 instructions are non-empty");
                Ok(typed_isa_core::DecodedInstruction::new(
                    decoded.into_instruction(),
                    source,
                ))
            }

            fn encode_one(
                instruction: &Self::Instruction,
                _address: u64,
            ) -> Result<typed_isa_core::InstructionBytes, Self::EncodeError> {
                if instruction.code_mode() != $mode {
                    return Err(EncodeError::CodeModeMismatch {
                        expected: $mode,
                        actual: instruction.code_mode(),
                    });
                }
                let bytes = encode_bytes(instruction)?;
                Ok(typed_isa_core::InstructionBytes::new(bytes)
                    .expect("encoded Intel 80386 instructions are non-empty"))
            }
        }

        impl I386Profile for $profile {
            const CODE_MODE: CodeMode = $mode;
        }
    };
}

implement_profile!(I386Code16, CODE16_PROFILE_ID, CodeMode::Code16);
implement_profile!(I386Code32, CODE32_PROFILE_ID, CodeMode::Code32);
