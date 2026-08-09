# Static Semantics Protocol

This document defines the implementation vocabulary that CPU profiles use to expose static meaning. It extends the byte and placement contract in the [typed ISA protocol](ISA.md) and derives from the project-wide [design principles](DESIGN.md); it does not introduce a separate project purpose.

The intended readers are profile implementers and authors of source-lowering or binary-patching tools. The protocol describes facts available without executing an instruction.

## Semantic result

One placed typed instruction produces one semantic result with three independent parts:

```rust
pub struct InstructionSemantics<P, I, B, L, R> {
    pub control_flow: ControlFlow<P, I, B>,
    pub location_accesses: Vec<LocationAccess<L>>,
    pub static_references: Vec<StaticReference<R>>,
}
```

`P`, `I`, `B`, `L`, and `R` are owned by the exact CPU profile or a source-identical first-party profile family:

- `P` represents one static analysis point: an architectural code location plus the explicit decode profile and semantic context required at that location.
- `I` represents the statically known part of an indirect target, such as its source register, pointer form, and instruction-set-state rule.
- `B` identifies an architecture-defined control boundary, such as a trap kind, stop cause, wait state, or exit to another instruction-set profile.
- `L` represents one architectural storage location or symbolic memory location.
- `R` represents a statically resolvable non-control reference in the architecture's address model.

The shared protocol must not replace them with a universal integer address or register enum.

The common capability is an extension of the byte-oriented profile, not a profile-specific helper:

```rust
pub trait StaticSemantics: TypedIsa {
    type AnalysisPoint;
    type IndirectTarget;
    type Boundary;
    type Location;
    type Reference;
    type SemanticError: std::error::Error + Send + Sync + 'static;

    fn semantics(
        instruction: &Self::Instruction,
        at: &Self::AnalysisPoint,
    ) -> Result<
        InstructionSemantics<
            Self::AnalysisPoint,
            Self::IndirectTarget,
            Self::Boundary,
            Self::Location,
            Self::Reference,
        >,
        Self::SemanticError,
    >;

    fn storage_relation(
        left: &Self::Location,
        right: &Self::Location,
    ) -> StorageRelation;
}
```

`SemanticError` is limited to structural inability to form sound facts, such as an invalid analysis point or unavailable required context. An instruction with no non-sequential transfer, location access, or reference returns `Continue` with explicit empty access and reference sets; it is not an error.

## Analysis points, locations, and profile state

An analysis point is not a file offset or a mutable CPU snapshot. It contains only the architectural code location and state required to decode and statically interpret the instruction at that location. A profile may use a plain aligned address only when that address completely identifies the point.

State that changes instruction identity or extent selects the protocol profile used for decoding. State that changes physical storage identity without changing the encoded instruction belongs to profile-owned semantic context. Both are preserved in a direct successor analysis point. A runtime-derived successor preserves its known state rule in the indirect-target description.

Semantic context is not a general register-value environment. It contains only architecture state needed to identify static storage or successor meaning, such as W65C816 emulation state, HuC6280 T state, SH-4 register-bank state, ARM processor mode, an x87 stack top, or a TLCS-900/H current register bank. Each required value is explicit and may be unknown only when the profile can return a conservative union. Instructions such as REP/SEP, SET, FSCHG/FRCHG, or a known mode change therefore produce successor points with the changed context. If a sound successor point cannot be formed, sequence analysis fails closed at that boundary.

Misalignment, address overflow, unavailable decode state, and a truncated instruction are structural errors. They are not instruction semantics and must not be converted into a rejected encoding or an indirect target.

## Control flow

Control flow reports every statically possible continuation of the placed instruction.

```rust
pub struct ControlFlow<P, I, B> {
    pub action: ControlAction<P, I, B>,
    pub fallthrough: Option<P>,
    pub delay_slot: Option<DelaySlot<P>>,
}

pub enum ControlAction<P, I, B> {
    Continue,
    Transfer { target: ControlTarget<P, I> },
    LinkedTransfer {
        target: ControlTarget<P, I>,
        return_site: P,
    },
    Return { target: I },
    ExceptionReturn { target: I },
    Boundary(ControlBoundary<P, I, B>),
}

pub enum ControlTarget<P, I> {
    Direct(P),
    Indirect(I),
}

pub enum ControlBoundary<P, I, B> {
    Trap {
        kind: B,
        target: Option<ControlTarget<P, I>>,
        resume: Option<P>,
    },
    ProfileExit {
        kind: B,
        target: Option<ControlTarget<P, I>>,
    },
    Wait { kind: B, resume: P },
    Stop { kind: B },
}

pub struct DelaySlot<P> {
    pub point: P,
    pub execution: DelaySlotExecution,
}

pub enum DelaySlotExecution {
    Always,
    WhenTransferOccurs,
    WhenFallthroughOccurs,
}
```

The fields have one meaning across all profiles:

- `action` describes the instruction's sequential action, non-sequential transfer, or architecture-defined boundary.
- `fallthrough` is present exactly when sequential continuation remains possible.
- `delay_slot` identifies the analysis point that executes before the transfer takes effect and states which control-flow outcome executes it. This distinction preserves annulled or branch-likely slots.
- `ControlTarget::Indirect` means that the instruction names a runtime-derived target. Its profile-owned value preserves the known pointer and instruction-set-state rules; it does not mean that analysis failed.
- `Transfer` covers every non-linking branch or jump; direct versus indirect target and the presence of `fallthrough` carry the static distinction without importing mnemonic families.
- `LinkedTransfer` means that the instruction itself establishes an architectural return site. `return_site` is the logical continuation used by the matching return rule, not necessarily the raw value stored by the processor. An ABI convention or mnemonic alias must not turn an ordinary indirect transfer into a linked transfer or return.
- `Return` is reserved for a dedicated architectural subroutine-return form. `ExceptionReturn` is reserved for a dedicated exception-return form. Their indirect-target values preserve the stack, link, saved-PC, or state-restoration rule that supplies the destination.
- `Trap` is a synchronous instruction-caused boundary. Asynchronous interrupts are not instruction control flow. A statically known vector rule or resumption site is preserved even when the final handler address requires runtime state.
- `ProfileExit` identifies a documented transfer to an instruction set outside the selected profile.
- `Wait` resumes at its stated location after an external event; `Stop` has no architectural resumption except through the profile-owned reset or restart boundary.

A conditional transfer or boundary has both a non-`Continue` action and a fallthrough. A predicated instruction whose transfer may not occur uses the same rule. `Continue` requires a fallthrough and no delay slot. Profile-specific control-flow enums, boolean `conditional` fields, and separate direct/indirect variants must be replaced by this result rather than kept as competing public models.

`Trap` covers instruction forms whose architectural purpose explicitly invokes or checks for a synchronous exception, such as software-interrupt, trap-on-condition, breakpoint, and illegal-instruction forms accepted by the profile. General data, privilege, alignment, coprocessor-availability, and memory faults are not control-flow edges in this protocol. Adding a fault model would require a separate common design and whole-workspace review.

## Architectural location accesses

Location accesses report possible architectural reads and writes when an instruction reaches execution. They do not calculate values.

```rust
pub struct LocationAccess<L> {
    pub location: L,
    pub kind: AccessKind,
    pub occurrence: AccessOccurrence,
}

pub enum AccessKind {
    Read,
    Write,
}

pub enum AccessOccurrence {
    Always,
    Conditional,
}

pub enum StorageRelation {
    Same,
    Overlapping,
    Disjoint,
    Unknown,
}
```

A read-modify-write instruction reports both a read and a write. `Conditional` records architectural predication or an instruction-defined condition; ordinary fault behavior is not modeled as conditionality.

Each profile's location vocabulary must cover:

- explicit register operands;
- implicit general, special, status, link, and stack locations;
- architectural address-register updates;
- register banks and individually meaningful status fields where the source distinguishes them;
- symbolic memory locations, including width and addressing form when those facts are statically known.

`Same` means the two values name the same set of architectural storage bits. `Overlapping` means their sets intersect but are not equal. `Disjoint` is a proof that they do not intersect. `Unknown` means the static facts cannot decide, and consumers treat it as possible overlap.

Partial registers, register pairs, banked registers, floating stacks, vector lanes, and symbolic memory must not be treated as independent merely because they have different operand names. The relation is part of the common protocol even though each profile owns the location representation.

The program counter is represented through control flow rather than duplicated as an ordinary storage write. Device registers reached through memory-mapped I/O remain memory locations; the device behavior behind them is outside this workspace.

Queries such as `may_write(location)` or `written_registers()` may be provided as views over `location_accesses`. They must not maintain independent instruction tables.

## Static references

A static reference exists only when placement and typed operands establish a non-control address without runtime register values.

```rust
pub struct StaticReference<R> {
    pub target: R,
    pub kind: ReferenceKind,
    pub occurrence: AccessOccurrence,
}

pub enum ReferenceKind {
    Read,
    Write,
    Address,
}
```

PC-relative literal loads, absolute memory operands, address-producing instructions, and immediate block-transfer endpoints may produce static references. `Address` does not claim that the referenced bytes are data; executable-region ownership belongs to the consumer. Register-indirect memory access still appears in `location_accesses`, but it does not invent a static target. A separate value-analysis layer may refine a symbolic access after proving a register value; that derived result is not retroactively an ISA fact.

Code targets used by the instruction's control action are owned by `control_flow` and are not duplicated as static references.

## Sequence analysis

Consumers compose per-instruction results into control-flow graphs, use/definition sets, static-reference indexes, and patch requirements. Sequence analysis may prove that a replacement preserves declared locations and exits. It does not claim value equivalence unless a separate value-transfer model supplies that proof.

Source lowering and patch planning therefore consume the same ISA facts:

```text
source or patch intent
    -> typed instruction selection
    -> static semantic validation
    -> checked placement and encoding
    -> expected-write verification

declared source bytes
    -> strict typed decoding
    -> static sequence analysis
    -> replacement planning
    -> typed instruction selection
    -> static semantic validation
    -> checked placement and encoding
    -> expected-write verification
```

## Semantic coverage gate

A profile exposes this protocol only after an independent source-derived semantic denominator covers every instruction form in that exact profile.

The gate must verify:

1. every source-listed form has one auditable semantic rule covering every required profile and context combination;
2. all explicit and implicit architectural location accesses are represented;
3. all control-transfer, delay-slot, predication, trap, wait, stop, and instruction-set-exit cases in scope are represented;
4. every static reference follows the architecture's placement and address rules;
5. forms inherited from a related profile are rechecked against the exact profile boundary;
6. forms with no access, transfer, or reference are explicitly present in the denominator.

An exhaustive Rust match ensures that implementation variants are handled; it is not the independent denominator. Tests derived from the same production table are mirror tests and do not establish semantic completeness.

## Workspace adoption rule

The shared vocabulary and every applicable profile migrate as one reviewed workspace change. The migration removes older profile-specific public models and updates their consumers; it does not leave permanent aliases with different meanings.

No consumer-specific method lands before this protocol. No profile may publish a partial semantic table under the common API. Architecture-specific exclusions are recorded in the profile design and enforced by types or fail-closed errors rather than undocumented omissions.

If implementation work exposes a contradiction or missing concept, the implementation pauses and triggers the whole-design review required by [Congruence](DESIGN.md#congruence). A local API addition, profile exception, or implementation-shaped test cannot close that review. The common protocol, every applicable profile design, consumer obligations, and coverage gates are reviewed together before implementation resumes.
