//! Static HuC6280 instruction-set profile with checked encoding and semantics.
//!
//! The profile models all 234 opcode selectors defined by Hudson's software
//! manual. The remaining 22 selector slots are classified as undocumented and
//! fail closed. Static semantics expose register, status, memory, port, and MPR
//! accesses without executing those state transitions. CPU execution, value
//! propagation, cycle timing, and simulation are outside this crate.

#![deny(missing_docs)]

/// Stable identifier for the CPU profile implemented by this crate.
pub const PROFILE_ID: &str = env!("CARGO_PKG_NAME");

/// Wrapping 16-bit HuC6280 program-counter location.
pub type CodeLocation = u16;

mod assembler;
mod codec;
mod display;
mod instruction;
mod semantics;

pub use assembler::{AssembledProgram, Assembler, AssemblerError, InstructionSpan};
pub use codec::{DecodeError, EncodeError, Instruction, Operand, decode_bytes, encode_bytes};
pub use display::{format_bytes, format_instruction};
pub use instruction::{
    AddressingMode, InstructionForm, Mnemonic, instruction_form, opcode_is_documented,
};
pub use semantics::{
    AddressPattern, AnalysisPoint, Boundary, ControlFlow, HardwarePort, IndexRegister,
    IndirectTarget, InstructionSemantics, Location, MemoryAddress, MemoryLocation, PointerAddress,
    Reference, RegisterLocation, ReturnAddressRule, SemanticError, StatusLocation, TState,
};

/// Shared-protocol marker for the HuC6280 profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuC6280;

impl typed_isa_core::TypedIsa for HuC6280 {
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
        let instruction = decode_bytes(source)?;
        let source =
            typed_isa_core::InstructionBytes::new(source[..instruction.encoded_len()].to_vec())
                .expect("decoded HuC6280 instructions are non-empty");
        Ok(typed_isa_core::DecodedInstruction::new(instruction, source))
    }

    fn encode_one(
        instruction: &Self::Instruction,
        _at: &Self::CodeLocation,
    ) -> Result<typed_isa_core::InstructionBytes, Self::EncodeError> {
        Ok(
            typed_isa_core::InstructionBytes::new(encode_bytes(instruction)?)
                .expect("encoded HuC6280 instructions are non-empty"),
        )
    }
}

impl typed_isa_core::StaticSemantics for HuC6280 {
    type AnalysisPoint = AnalysisPoint;
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
        Ok(semantics::semantics(instruction, *at))
    }

    fn storage_relation(
        left: &Self::Location,
        right: &Self::Location,
    ) -> typed_isa_core::StorageRelation {
        semantics::storage_relation(left, right)
    }
}
