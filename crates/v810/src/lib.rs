//! Static NEC V810 instruction model for PC-FX executable code.
//!
//! The profile follows NEC document U10082E, *V810 Family 32-Bit
//! Microprocessor Architecture*, including all seven instruction formats,
//! bit-string and floating-point sub-opcodes, reserved primary opcodes, and
//! reserved fields. Instructions lower to the little-endian halfword order
//! specified by the architecture. Execution and simulation are out of scope.

#![deny(missing_docs)]

/// Stable identifier for the exact CPU profile implemented by this crate.
pub const PROFILE_ID: &str = env!("CARGO_PKG_NAME");

/// Halfword-aligned 32-bit V810 executable address.
pub type CodeLocation = u32;

mod assembler;
mod codec;
mod instruction;
mod semantics;

pub use assembler::{AssembledProgram, Assembler, AssemblerError, InstructionSpan};
pub use codec::{
    DecodeError, DecodedInstruction, EncodeError, decode_bytes, decode_halfwords, encode_bytes,
    encode_halfwords,
};
pub use instruction::{
    BitStringOperation, Condition, FloatingOperation, ImmediateOperation, Instruction,
    InstructionForm, LogicalImmediateOperation, MemoryOperation, Register, RegisterOperation,
    ShiftOperation, SystemRegister,
};
pub use semantics::{
    AddressSpace, BitStringMemory, Boundary, ControlFlow, IndirectTarget, InstructionSemantics,
    Location, MemoryLocation, PswField, Reference, SemanticError,
};

/// NEC V810 implementation of the shared static ISA protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct V810;

impl typed_isa_core::StaticSemantics for V810 {
    type AnalysisPoint = CodeLocation;
    type IndirectTarget = IndirectTarget;
    type Boundary = Boundary;
    type Location = Location;
    type Reference = Reference;
    type SemanticError = SemanticError;

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
        semantics::semantics(instruction, *at)
    }

    fn storage_relation(
        left: &Self::Location,
        right: &Self::Location,
    ) -> typed_isa_core::StorageRelation {
        semantics::storage_relation(left, right)
    }
}
