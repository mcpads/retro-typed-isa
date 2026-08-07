//! Hitachi SH-4 instruction semantics, byte lowering, and checked assembly.
//!
//! This crate covers the complete fixed-width 16-bit SH-4 CPU and FPU
//! instruction profile from the SH-4 Software Manual. Byte APIs use the
//! little-endian serialization used by the imported Dreamcast consumers;
//! natural word APIs are endian-neutral. FPSCR.PR and FPSCR.SZ are explicit
//! decode inputs. Execution, pipeline timing, scheduling, and simulation are
//! deliberately excluded.

#![deny(missing_docs)]

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

/// SH-4 profiles whose FPSCR.PR and FPSCR.SZ decode inputs are externally known.
pub trait Sh4Profile:
    typed_isa_core::TypedIsa<Instruction = Instruction, Form = InstructionForm>
{
    /// The externally known FPSCR precision and transfer-size decode state.
    const FPSCR: FpscrState;
}

/// Little-endian SH-4 profile with single precision and single-register transfers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sh4Pr0Sz0;
/// Little-endian SH-4 profile with single precision and register-pair transfers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sh4Pr0Sz1;
/// Little-endian SH-4 profile with double precision and single-register transfers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sh4Pr1Sz0;
/// Little-endian SH-4 profile with double precision and register-pair transfers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sh4Pr1Sz1;

fn validate_profile_instruction(
    instruction: &Instruction,
    fpscr: FpscrState,
) -> Result<(), EncodeError> {
    let bytes = encode_bytes(instruction)?;
    match decode_bytes(&bytes, fpscr) {
        Ok(decoded) if decoded == *instruction => Ok(()),
        _ => Err(EncodeError::FpscrStateMismatch {
            form: instruction.form(),
            fpscr,
        }),
    }
}

macro_rules! implement_profile {
    ($profile:ty, $id:expr, $fpscr:expr) => {
        impl typed_isa_core::TypedIsa for $profile {
            const PROFILE_ID: &'static str = $id;

            type Instruction = Instruction;
            type Form = InstructionForm;
            type DecodeError = DecodeError;
            type EncodeError = EncodeError;

            fn form(instruction: &Self::Instruction) -> Result<Self::Form, Self::EncodeError> {
                validate_profile_instruction(instruction, $fpscr)?;
                Ok(instruction.form())
            }

            fn decode_one(
                source: &[u8],
                address: u64,
            ) -> Result<typed_isa_core::DecodedInstruction<Self::Instruction>, Self::DecodeError>
            {
                let address = u32::try_from(address)
                    .map_err(|_| DecodeError::AddressOutOfRange { address })?;
                if address & 1 != 0 {
                    return Err(DecodeError::UnalignedInstructionAddress { address });
                }
                let instruction = decode_bytes(source, $fpscr)?;
                let source = typed_isa_core::InstructionBytes::new(source[..2].to_vec())
                    .expect("decoded SH-4 instructions are non-empty");
                Ok(typed_isa_core::DecodedInstruction::new(instruction, source))
            }

            fn encode_one(
                instruction: &Self::Instruction,
                address: u64,
            ) -> Result<typed_isa_core::InstructionBytes, Self::EncodeError> {
                let address = u32::try_from(address)
                    .map_err(|_| EncodeError::AddressOutOfRange { address })?;
                if address & 1 != 0 {
                    return Err(EncodeError::UnalignedInstructionAddress { address });
                }
                validate_profile_instruction(instruction, $fpscr)?;
                Ok(
                    typed_isa_core::InstructionBytes::new(encode_bytes(instruction)?.to_vec())
                        .expect("encoded SH-4 instructions are non-empty"),
                )
            }
        }

        impl Sh4Profile for $profile {
            const FPSCR: FpscrState = $fpscr;
        }
    };
}

implement_profile!(
    Sh4Pr0Sz0,
    "sh4-pr0-sz0-le",
    FpscrState::new(FpuPrecision::Single, FpuTransferSize::Single)
);
implement_profile!(
    Sh4Pr0Sz1,
    "sh4-pr0-sz1-le",
    FpscrState::new(FpuPrecision::Single, FpuTransferSize::Pair)
);
implement_profile!(
    Sh4Pr1Sz0,
    "sh4-pr1-sz0-le",
    FpscrState::new(FpuPrecision::Double, FpuTransferSize::Single)
);
implement_profile!(
    Sh4Pr1Sz1,
    "sh4-pr1-sz1-le",
    FpscrState::new(FpuPrecision::Double, FpuTransferSize::Pair)
);
