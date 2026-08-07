//! Shared protocol for static typed instruction-set profiles.
//!
//! This crate aligns profile APIs and concepts. It is not an instruction
//! catalog and does not prove that a CPU profile is complete. Coverage belongs
//! to profile-specific gates derived from public architecture sources and kept
//! independent of the implementation's own catalog. Execution state and
//! simulation are outside this protocol.

#![deny(missing_docs)]

use std::fmt::Debug;
use std::num::NonZeroUsize;

/// Non-empty bytes representing exactly one instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionBytes(Vec<u8>);

impl InstructionBytes {
    /// Construct exact instruction bytes, returning `None` for an empty sequence.
    pub fn new(bytes: impl Into<Vec<u8>>) -> Option<Self> {
        let bytes = bytes.into();
        (!bytes.is_empty()).then_some(Self(bytes))
    }

    /// Borrow the exact bytes occupied by the instruction.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Consume the wrapper and return the exact instruction bytes.
    pub fn into_vec(self) -> Vec<u8> {
        self.0
    }

    /// Return the instruction's non-zero encoded length.
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
    /// Bind a typed instruction to the exact source bytes consumed for it.
    pub const fn new(instruction: I, source: InstructionBytes) -> Self {
        Self {
            instruction,
            source,
        }
    }

    /// Borrow the typed instruction identity.
    pub const fn instruction(&self) -> &I {
        &self.instruction
    }

    /// Borrow the exact bytes consumed from the source slice.
    pub const fn source(&self) -> &InstructionBytes {
        &self.source
    }

    /// Consume the decoded value and return its typed instruction identity.
    pub fn into_instruction(self) -> I {
        self.instruction
    }
}

/// Common static interface implemented by one explicitly scoped CPU profile.
pub trait TypedIsa {
    /// Stable identifier for the exact CPU profile implemented by this type.
    ///
    /// Implementations must not use a broader CPU-family name when the profile
    /// intentionally supports only a model, execution mode, or instruction
    /// subset.
    const PROFILE_ID: &'static str;

    /// Typed identity of one instruction accepted by the profile.
    type Instruction: Clone + Debug + Eq;
    /// Exact encoded form selected for an instruction.
    type Form: Clone + Debug + Eq;
    /// Failure produced when source bytes cannot be decoded as one instruction.
    type DecodeError: std::error::Error + Send + Sync + 'static;
    /// Failure produced when an instruction cannot be lowered to executable bytes.
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
    /// Byte offset of the instruction within the assembled output.
    pub offset: usize,
    /// CPU address used while lowering the instruction.
    pub address: u64,
    /// Exact bytes emitted for the instruction.
    pub bytes: InstructionBytes,
    /// Typed instruction identity that produced `bytes`.
    pub instruction: I,
}
