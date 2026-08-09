//! Static ARM946E-S (ARMv5TE) instruction model.
//!
//! The profile follows ARM DDI 0201D for the processor scope and the ARMv5TE
//! instruction tables in ARM DDI 0100I. It extends the workspace-owned ARMv4T
//! profile with the version 5 and enhanced-DSP instruction forms; it does not
//! widen the profile to ARMv5TEJ or ARMv6.

#![deny(missing_docs)]

/// Aligned 32-bit ARM946E-S executable address.
pub type CodeLocation = u32;

mod assembler;
mod codec;
mod instruction;
mod semantics;

pub use arm7tdmi::{
    AddressOffsetLocation, AnalysisPoint, Boundary, ControlBit, ExecutionState, Flag,
    HalfwordOffsetLocation, IndirectTarget, Location, MemoryAddress, MemoryLocation, MemoryWidth,
    ProcessorMode, Reference, RegisterBank, RegisterLocation, StatusField, StatusLocation,
};
pub use assembler::{
    ArmAssembler, ArmAssemblerError, ArmProgram, ThumbAssembler, ThumbAssemblerError, ThumbProgram,
};
pub use codec::{
    Arm946EsArm, Arm946EsThumb, DecodeError, EncodeError, decode_arm_bytes, decode_thumb_bytes,
    encode_arm_bytes, encode_thumb_bytes,
};
pub use instruction::*;
pub use semantics::{ControlFlow, InstructionSemantics, SemanticError};

impl typed_isa_core::StaticSemantics for Arm946EsArm {
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
        <arm7tdmi::Arm7TdmiArm as typed_isa_core::StaticSemantics>::storage_relation(left, right)
    }
}

impl typed_isa_core::StaticSemantics for Arm946EsThumb {
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
        <arm7tdmi::Arm7TdmiThumb as typed_isa_core::StaticSemantics>::storage_relation(left, right)
    }
}
