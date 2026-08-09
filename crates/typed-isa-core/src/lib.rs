//! Shared protocol for static typed instruction-set profiles.
//!
//! This crate aligns profile APIs and concepts. It is not an instruction
//! catalog and does not prove encoding or semantic completeness. Coverage belongs
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
    /// Architectural executable location used for decoding and lowering.
    ///
    /// This is never a host file offset. The profile may use an integer only
    /// when it preserves every architectural placement fact needed by that
    /// profile.
    type CodeLocation: Clone + Debug + Eq;
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
        at: &Self::CodeLocation,
    ) -> Result<DecodedInstruction<Self::Instruction>, Self::DecodeError>;

    /// Lower one typed instruction to its exact bytes at `at`.
    fn encode_one(
        instruction: &Self::Instruction,
        at: &Self::CodeLocation,
    ) -> Result<InstructionBytes, Self::EncodeError>;
}

/// Static facts established by one placed typed instruction.
///
/// The profile owns the concrete vocabularies carried by this result. The
/// shared shape keeps control flow, architectural storage accesses, and
/// non-control references independent so consumers do not infer one from
/// another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionSemantics<P, I, B, L, R> {
    /// Possible control transfer and sequential continuation.
    pub control_flow: ControlFlow<P, I, B>,
    /// Architectural storage locations read or written by the instruction.
    pub location_accesses: Vec<LocationAccess<L>>,
    /// Non-control addresses resolved from placement and typed operands alone.
    pub static_references: Vec<StaticReference<R>>,
}

/// Result of deriving one profile's static instruction semantics.
pub type StaticSemanticResult<P, I, B, L, R, E> = Result<InstructionSemantics<P, I, B, L, R>, E>;

/// Static-semantics result specialized by a profile implementing [`StaticSemantics`].
pub type ProfileSemanticsResult<P> = StaticSemanticResult<
    <P as StaticSemantics>::AnalysisPoint,
    <P as StaticSemantics>::IndirectTarget,
    <P as StaticSemantics>::Boundary,
    <P as StaticSemantics>::Location,
    <P as StaticSemantics>::Reference,
    <P as StaticSemantics>::SemanticError,
>;

/// Static control-flow facts for one placed instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlFlow<P, I, B> {
    /// The instruction's control action.
    pub action: ControlAction<P, I, B>,
    /// Sequential continuation when it remains possible.
    pub fallthrough: Option<P>,
    /// Instruction point executed before a transfer takes effect.
    pub delay_slot: Option<DelaySlot<P>>,
}

/// Architectural control action performed by an instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlAction<P, I, B> {
    /// Continue only through the sequential successor.
    Continue,
    /// Transfer without establishing an architectural return site.
    Transfer {
        /// Direct or runtime-derived destination.
        target: ControlTarget<P, I>,
    },
    /// Transfer while establishing an architectural return site.
    LinkedTransfer {
        /// Direct or runtime-derived destination.
        target: ControlTarget<P, I>,
        /// Logical continuation used by the matching return rule.
        return_site: P,
    },
    /// Return through the architecture's dedicated subroutine-return rule.
    Return {
        /// Statically known description of the runtime-derived destination.
        target: I,
    },
    /// Return through the architecture's dedicated exception-return rule.
    ExceptionReturn {
        /// Statically known description of the restored destination.
        target: I,
    },
    /// Reach an architecture-defined synchronous boundary.
    Boundary(ControlBoundary<P, I, B>),
}

/// Destination of a non-sequential control action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlTarget<P, I> {
    /// Destination resolved completely from placement and typed operands.
    Direct(P),
    /// Runtime-derived destination described by the exact profile.
    Indirect(I),
}

/// Synchronous control boundary defined by the exact profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlBoundary<P, I, B> {
    /// An instruction-caused synchronous exception or trap.
    Trap {
        /// Profile-owned trap or exception kind.
        kind: B,
        /// Handler destination when the architecture makes it statically known.
        target: Option<ControlTarget<P, I>>,
        /// Resumption point when the architecture defines one.
        resume: Option<P>,
    },
    /// Transfer to an instruction set outside the selected exact profile.
    ProfileExit {
        /// Profile-owned reason or destination instruction-set rule.
        kind: B,
        /// Exit destination when it can be described statically.
        target: Option<ControlTarget<P, I>>,
    },
    /// Suspend until an external event, then resume at a known point.
    Wait {
        /// Profile-owned wait-state kind.
        kind: B,
        /// Architectural resumption point.
        resume: P,
    },
    /// Stop with no ordinary architectural resumption.
    Stop {
        /// Profile-owned stop-state kind.
        kind: B,
    },
}

/// An instruction point executed before a control transfer takes effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelaySlot<P> {
    /// Architectural point occupied by the delay-slot instruction.
    pub point: P,
    /// Control outcome under which the slot executes.
    pub execution: DelaySlotExecution,
}

/// Condition under which a delay-slot instruction executes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DelaySlotExecution {
    /// Execute regardless of whether a conditional transfer is taken.
    Always,
    /// Execute only when the transfer occurs.
    WhenTransferOccurs,
    /// Execute only when sequential fallthrough occurs.
    WhenFallthroughOccurs,
}

/// One possible architectural storage access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationAccess<L> {
    /// Profile-owned architectural storage location.
    pub location: L,
    /// Whether the instruction reads or writes the location.
    pub kind: AccessKind,
    /// Whether execution of the instruction always performs the access.
    pub occurrence: AccessOccurrence,
}

/// Direction of an architectural storage access or static reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessKind {
    /// Read existing contents.
    Read,
    /// Write new contents.
    Write,
}

/// Whether an instruction-defined condition guards a fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessOccurrence {
    /// The fact applies whenever the instruction reaches execution.
    Always,
    /// Architectural predication or an instruction-defined condition guards it.
    Conditional,
}

/// Proven relationship between two architectural storage locations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StorageRelation {
    /// Both values name exactly the same set of storage bits.
    Same,
    /// The named storage bits intersect but are not equal.
    Overlapping,
    /// The named storage bits are proven not to intersect.
    Disjoint,
    /// Available static facts cannot decide whether the locations intersect.
    Unknown,
}

/// One statically resolved non-control reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticReference<R> {
    /// Profile-owned referenced address or address range.
    pub target: R,
    /// How the instruction uses the referenced address.
    pub kind: ReferenceKind,
    /// Whether an instruction-defined condition guards the reference.
    pub occurrence: AccessOccurrence,
}

/// Use made of a statically resolved non-control address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReferenceKind {
    /// Read contents at the referenced address.
    Read,
    /// Write contents at the referenced address.
    Write,
    /// Produce or otherwise use the address without claiming a memory access.
    Address,
}

/// Common static-semantics interface implemented by one exact CPU profile.
///
/// Implementations are published only when their profile-specific,
/// source-derived semantic coverage gate accounts for every accepted
/// instruction form. This trait models facts available without executing an
/// instruction or supplying general register values.
pub trait StaticSemantics: TypedIsa + Sized {
    /// Placed instruction point plus minimal architectural semantic context.
    type AnalysisPoint: Clone + Debug + Eq;
    /// Statically known description of a runtime-derived control destination.
    type IndirectTarget: Clone + Debug + Eq;
    /// Architecture-defined trap, wait, stop, or profile-exit kind.
    type Boundary: Clone + Debug + Eq;
    /// Profile-owned architectural storage location.
    type Location: Clone + Debug + Eq;
    /// Profile-owned statically resolvable non-control address.
    type Reference: Clone + Debug + Eq;
    /// Structural failure to form sound static facts.
    type SemanticError: std::error::Error + Send + Sync + 'static;

    /// Derive static facts for `instruction` at `at` without executing it.
    fn semantics(
        instruction: &Self::Instruction,
        at: &Self::AnalysisPoint,
    ) -> ProfileSemanticsResult<Self>;

    /// Determine whether two profile-owned locations can name common storage.
    fn storage_relation(left: &Self::Location, right: &Self::Location) -> StorageRelation;
}

/// Location and typed identity of one instruction in assembled output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionSpan<C, I> {
    /// Byte offset of the instruction within the assembled output.
    pub offset: usize,
    /// Architectural code location used while lowering the instruction.
    pub location: C,
    /// Exact bytes emitted for the instruction.
    pub bytes: InstructionBytes,
    /// Typed instruction identity that produced `bytes`.
    pub instruction: I,
}
