//! Shared protocol for static typed instruction-set profiles.
//!
//! This crate aligns profile APIs and concepts. It is not an instruction
//! catalog and does not prove that a CPU profile is complete. Coverage belongs
//! to profile-specific gates derived from public architecture sources and kept
//! independent of the implementation's own catalog. Execution state and
//! simulation are outside this protocol.

use std::fmt::Debug;
use std::num::NonZeroUsize;

/// Non-empty bytes representing exactly one instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionBytes(Vec<u8>);

impl InstructionBytes {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Option<Self> {
        let bytes = bytes.into();
        (!bytes.is_empty()).then_some(Self(bytes))
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.0
    }

    pub fn len(&self) -> NonZeroUsize {
        NonZeroUsize::new(self.0.len()).expect("instruction bytes are non-empty")
    }
}

/// One typed instruction decoded from the leading bytes of a source slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedInstruction<I> {
    instruction: I,
    source: InstructionBytes,
}

impl<I> DecodedInstruction<I> {
    pub const fn new(instruction: I, source: InstructionBytes) -> Self {
        Self {
            instruction,
            source,
        }
    }

    pub const fn instruction(&self) -> &I {
        &self.instruction
    }

    pub const fn source(&self) -> &InstructionBytes {
        &self.source
    }

    pub fn into_instruction(self) -> I {
        self.instruction
    }
}

/// Common static interface implemented by one explicitly scoped CPU profile.
pub trait TypedIsa {
    const PROFILE_ID: &'static str;

    type Instruction: Clone + Debug + Eq;
    type Form: Clone + Debug + Eq;
    type DecodeError: std::error::Error + Send + Sync + 'static;
    type EncodeError: std::error::Error + Send + Sync + 'static;

    /// Return the exact encoded form selected by `instruction`.
    ///
    /// This is checked because some profiles retain non-executable values for
    /// source inspection. Such values are not instruction forms and must fail
    /// through the same boundary as byte lowering.
    fn form(instruction: &Self::Instruction) -> Result<Self::Form, Self::EncodeError>;

    /// Decode the first instruction from `source`; trailing bytes are allowed.
    fn decode_one(
        source: &[u8],
        address: u64,
    ) -> Result<DecodedInstruction<Self::Instruction>, Self::DecodeError>;

    /// Lower one typed instruction to its exact bytes at `address`.
    fn encode_one(
        instruction: &Self::Instruction,
        address: u64,
    ) -> Result<InstructionBytes, Self::EncodeError>;
}

/// Location and typed identity of one instruction in assembled output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionSpan<I> {
    pub offset: usize,
    pub address: u64,
    pub bytes: InstructionBytes,
    pub instruction: I,
}
