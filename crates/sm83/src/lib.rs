//! Static Sharp SM83 instruction-set profile for Game Boy executable code.
//!
//! All base and CB-prefixed opcode selectors are classified. Valid instructions
//! decode to typed operands, lower back to exact machine code, and expose
//! architectural static semantics. Illegal selectors fail closed. Execution,
//! value propagation, cycle timing, peripherals, and simulation are outside
//! this crate.
//!
//! The encoding profile follows the Pan Docs CPU Instruction Set and the RGBDS
//! `gbz80(7)` opcode reference. Those independent source tables define the
//! coverage denominator; an emulator or third-party decoder is not exposed as
//! the semantic API.

#![deny(missing_docs)]

use std::convert::Infallible;

use typed_isa_core::{
    DecodedInstruction as ProtocolDecodedInstruction, InstructionBytes, TypedIsa,
};

/// Stable identifier for the exact CPU profile implemented by this crate.
pub const PROFILE_ID: &str = env!("CARGO_PKG_NAME");

/// Wrapping 16-bit SM83 program-counter location.
pub type CodeLocation = u16;

/// SM83 implementation of the workspace-wide static ISA protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sm83;

mod assembler;
mod codec;
mod instruction;
mod semantics;

pub use assembler::{AssembledProgram, Assembler, AssemblerError, InstructionSpan};
pub use codec::{
    BaseOpcodeClass, CanonicalInstruction, DecodeError, DecodedInstruction, canonical_instructions,
    classify_base_opcode, decode_bytes, encode_bytes,
};
pub use instruction::{
    AluOperation, BitIndex, ByteOperand, Condition, IndirectAddress, Instruction, InstructionForm,
    Register16, RestartVector, RotateShiftOperation, StackRegister,
};
pub use semantics::{
    Boundary, ByteRegister, ControlFlow, IndirectTarget, InstructionSemantics, Location,
    MemoryAddress, Reference, ReturnAddressRule, StatusLocation,
};

impl TypedIsa for Sm83 {
    const PROFILE_ID: &'static str = PROFILE_ID;

    type Instruction = Instruction;
    type Form = InstructionForm;
    type CodeLocation = CodeLocation;
    type DecodeError = DecodeError;
    type EncodeError = Infallible;

    fn form(instruction: &Self::Instruction) -> Result<Self::Form, Self::EncodeError> {
        Ok(instruction.form())
    }

    fn decode_one(
        source: &[u8],
        _at: &Self::CodeLocation,
    ) -> Result<ProtocolDecodedInstruction<Self::Instruction>, Self::DecodeError> {
        let decoded = decode_bytes(source)?;
        let exact_source = InstructionBytes::new(source[..decoded.length()].to_vec())
            .expect("decoded SM83 instructions are non-empty");
        Ok(ProtocolDecodedInstruction::new(
            decoded.into_instruction(),
            exact_source,
        ))
    }

    fn encode_one(
        instruction: &Self::Instruction,
        _at: &Self::CodeLocation,
    ) -> Result<InstructionBytes, Self::EncodeError> {
        Ok(InstructionBytes::new(encode_bytes(instruction))
            .expect("encoded SM83 instructions are non-empty"))
    }
}

impl typed_isa_core::StaticSemantics for Sm83 {
    type AnalysisPoint = CodeLocation;
    type IndirectTarget = IndirectTarget;
    type Boundary = Boundary;
    type Location = Location;
    type Reference = Reference;
    type SemanticError = Infallible;

    fn semantics(
        instruction: &Self::Instruction,
        at: &Self::AnalysisPoint,
    ) -> Result<
        typed_isa_core::InstructionSemantics<
            Self::AnalysisPoint,
            Self::IndirectTarget,
            Self::Boundary,
            Self::Location,
            Self::Reference,
        >,
        Self::SemanticError,
    > {
        Ok(semantics::semantics(instruction, *at))
    }

    fn storage_relation(
        left: &Self::Location,
        right: &Self::Location,
    ) -> typed_isa_core::StorageRelation {
        semantics::storage_relation(left, right)
    }
}
