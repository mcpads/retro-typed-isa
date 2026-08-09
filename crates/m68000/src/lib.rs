//! Static Motorola MC68000 instruction-set profile.
//!
//! Typed instructions support fail-closed decoding, checked byte lowering,
//! placement-aware assembly, and architectural static semantics. The profile
//! does not include the MC68010-only `BKPT`, `MOVEC`, `MOVES`, or `RTD`
//! instructions. Execution, value propagation, and cycle timing are outside
//! this crate.

#![deny(missing_docs)]

mod assembler;
mod codec;
mod instruction;
mod semantics;

pub use assembler::{
    AssembledProgram, Assembler, AssemblerError, InstructionSpan, PlacedInstruction,
    verify_placed_program,
};
pub use codec::{
    DecodeError, DecodedInstruction, DecodedWord, EncodeError, classify_word, decode_bytes,
    encode_bytes,
};
pub use instruction::*;
pub use semantics::{
    AddressRegisterLocation, AnalysisPoint, Boundary, ConditionCode, ControlFlow, IndirectTarget,
    InstructionSemantics, Location, MemoryAddress, MemoryLocation, PrivilegeMode, Reference,
    SemanticError,
};

/// Stable identifier for the exact CPU profile implemented by this crate.
pub const PROFILE_ID: &str = "m68000";

/// MC68000 logical program-counter location with its 24-bit bus projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CodeLocation(u32);

impl CodeLocation {
    /// Construct a logical 32-bit program-counter location.
    pub const fn new(logical_address: u32) -> Self {
        Self(logical_address)
    }

    /// Return the logical 32-bit program-counter value.
    pub const fn logical_address(self) -> u32 {
        self.0
    }

    /// Return the 24-bit address projected onto the original MC68000 bus.
    pub const fn bus_address(self) -> u32 {
        self.0 & 0x00ff_ffff
    }
}

/// Shared-protocol marker for the Motorola MC68000 profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mc68000;

impl typed_isa_core::TypedIsa for Mc68000 {
    const PROFILE_ID: &'static str = PROFILE_ID;

    type Instruction = Instruction;
    type Form = InstructionForm;
    type CodeLocation = CodeLocation;
    type DecodeError = DecodeError;
    type EncodeError = EncodeError;

    fn form(instruction: &Self::Instruction) -> Result<Self::Form, Self::EncodeError> {
        Ok(instruction.form())
    }

    fn decode_one(
        source: &[u8],
        _at: &Self::CodeLocation,
    ) -> Result<typed_isa_core::DecodedInstruction<Self::Instruction>, Self::DecodeError> {
        let decoded = decode_bytes(source)?;
        let source = typed_isa_core::InstructionBytes::new(source[..decoded.byte_len].to_vec())
            .expect("decoded MC68000 instructions are non-empty");
        Ok(typed_isa_core::DecodedInstruction::new(
            decoded.instruction,
            source,
        ))
    }

    fn encode_one(
        instruction: &Self::Instruction,
        _at: &Self::CodeLocation,
    ) -> Result<typed_isa_core::InstructionBytes, Self::EncodeError> {
        Ok(
            typed_isa_core::InstructionBytes::new(encode_bytes(instruction)?)
                .expect("encoded MC68000 instructions are non-empty"),
        )
    }
}

impl typed_isa_core::StaticSemantics for Mc68000 {
    type AnalysisPoint = AnalysisPoint;
    type IndirectTarget = IndirectTarget;
    type Boundary = Boundary;
    type Location = Location;
    type Reference = Reference;
    type SemanticError = SemanticError;

    fn semantics(
        instruction: &Self::Instruction,
        at: &Self::AnalysisPoint,
    ) -> Result<InstructionSemantics, Self::SemanticError> {
        semantics::semantics(instruction, *at)
    }

    fn storage_relation(
        left: &Self::Location,
        right: &Self::Location,
    ) -> typed_isa_core::StorageRelation {
        semantics::storage_relation(left, right)
    }
}
