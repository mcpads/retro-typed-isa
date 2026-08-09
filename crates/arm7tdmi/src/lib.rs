//! Static ARM7TDMI (ARMv4T) instruction model.
//!
//! The profile follows the ARM and Thumb instruction summaries in ARM DDI
//! 0210C, revision r4p1. ARM state and Thumb state are separate protocol
//! profiles because instruction width and decoding state are externally known.
//! Reserved and architecturally unpredictable encodings fail closed.

#![deny(missing_docs)]

/// Aligned 32-bit ARM7TDMI executable address.
pub type CodeLocation = u32;

mod assembler;
mod codec;
mod instruction;
mod semantics;

pub use assembler::{
    ArmAssembler, ArmAssemblerError, ArmProgram, ThumbAssembler, ThumbAssemblerError, ThumbProgram,
};
pub use codec::{
    Arm7TdmiArm, Arm7TdmiThumb, DecodeError, EncodeError, decode_arm_bytes, decode_thumb_bytes,
    encode_arm_bytes, encode_thumb_bytes,
};
pub use instruction::*;
pub use semantics::{
    AddressOffsetLocation, AnalysisPoint, Boundary, ControlBit, ControlFlow, ExecutionState, Flag,
    HalfwordOffsetLocation, IndirectTarget, InstructionSemantics, Location, MemoryAddress,
    MemoryLocation, MemoryWidth, ProcessorMode, Reference, RegisterBank, RegisterLocation,
    SemanticError, StatusField, StatusLocation,
};

impl typed_isa_core::StaticSemantics for Arm7TdmiArm {
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
        semantics::arm_semantics(instruction, *at)
    }

    fn storage_relation(
        left: &Self::Location,
        right: &Self::Location,
    ) -> typed_isa_core::StorageRelation {
        semantics::storage_relation(left, right)
    }
}

impl typed_isa_core::StaticSemantics for Arm7TdmiThumb {
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
        semantics::thumb_semantics(instruction, *at)
    }

    fn storage_relation(
        left: &Self::Location,
        right: &Self::Location,
    ) -> typed_isa_core::StorageRelation {
        semantics::storage_relation(left, right)
    }
}
