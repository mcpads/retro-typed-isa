# Typed ISA Protocol

This document defines the CPU-independent static protocol implemented by each exact ISA profile. It applies the project-wide [design principles](DESIGN.md) to byte identity, placement, and checked lowering. Static instruction meaning is defined separately by the [static semantics protocol](SEMANTICS.md).

## Exact profile

One protocol implementation represents one exact processor, extension scope, byte serialization, and decode state. Family names, consoles, host file formats, and consumer defaults are not profiles.

State that selects instruction identity or extent is explicit in the profile type or its required decode input. ARM and Thumb, Intel code-16 and code-32, W65C816 M/X widths, SH-4 FPSCR.PR/SZ, and VR4300 Status.FR therefore cannot rely on implicit defaults.

`PROFILE_ID` is a stable identity for this exact boundary. It is not a completeness certificate or a package version.

## Core types

The common byte-oriented capability has this conceptual shape:

```rust
pub trait TypedIsa {
    const PROFILE_ID: &'static str;

    type Instruction;
    type Form;
    type CodeLocation;
    type DecodeError;
    type EncodeError;

    fn form(
        instruction: &Self::Instruction,
    ) -> Result<Self::Form, Self::EncodeError>;

    fn decode_one(
        source: &[u8],
        at: &Self::CodeLocation,
    ) -> Result<DecodedInstruction<Self::Instruction>, Self::DecodeError>;

    fn encode_one(
        instruction: &Self::Instruction,
        at: &Self::CodeLocation,
    ) -> Result<InstructionBytes, Self::EncodeError>;
}
```

The profile owns every associated type except the shared byte wrappers. Related first-party profiles may reuse a type only under the ownership rules in [Design principles](DESIGN.md#ownership).

## Instruction identity and form

`Instruction` preserves every typed operand and encoding choice required for exact lowering. Distinct source encodings remain distinguishable when their identity matters for analysis or round-trip preservation, even if an assembler normally selects one canonical form.

`Form` is the operand-independent source form used for coverage and dispatch. It is not a substitute for the placed instruction and does not imply that every operand combination is valid.

Opaque raw bytes, unknown operation fields, emulator callbacks, and third-party instruction values are not executable `Instruction` variants. Source bytes that do not decode within the profile remain rejected source evidence.

## Code locations

`CodeLocation` is an architectural executable location, not a host file offset or universal address integer. Its representation preserves every fact required for instruction placement and target calculation, including alignment, wrapping rules, banks, segments, or address-space identity.

Examples include bank plus program counter, segment plus offset, a logical PC plus its narrower bus projection, and an aligned flat virtual address. A profile may use an integer only when the architecture establishes that it fully identifies a code location.

The common protocol never defines a packed integer convention for richer locations. File mapping, ROM banks, executable sections, overlays, and disc extents belong to consumers and must convert to a profile location explicitly.

The common protocol also does not define universal location arithmetic. Sequential placement and decoding advance according to the exact profile's alignment, overflow, wrapping, bank, segment, and address-space rules. That checked progression belongs to the profile assembler or verifier; it must not be approximated by adding a byte length to a packed integer.

Public APIs use `CodeLocation` and `location` for architectural instruction placement. `address` remains appropriate only for an architectural address operand, an address calculation, or an independently named non-code address-space fact. Compatibility aliases must not leave two public names for the same code-location concept.

## Exact bytes

`InstructionBytes` contains the non-empty bytes occupied by exactly one accepted instruction. `DecodedInstruction` binds one typed instruction to the exact source bytes consumed from the beginning of a caller-supplied slice.

Trailing input is allowed when decoding one instruction. Truncation, misalignment, address overflow, unavailable decode state, and invalid placement are structural errors. They are not rejected instruction classifications and must not be recovered as executable raw bytes.

Encoding validates the same exact profile boundary as decoding. Successful encoding proves that the typed instruction is executable at the supplied architectural location and yields its exact serialized bytes; it does not prove source preconditions or authorize a write.

## Rejected encoding classification

Lossless rejected-byte classification is separate from strict decoding and is exposed only when the architecture establishes the rejected source extent:

```rust
pub enum ClassifiedEncoding<I, R> {
    Instruction(DecodedInstruction<I>),
    Rejected {
        source: InstructionBytes,
        reason: R,
    },
}
```

`R` is profile-owned and source-bound. A generic unknown reason must not hide an incomplete encoding catalog. Each profile declares one policy in [CPU profile semantics](PROFILE_SEMANTICS.md):

- **Exact encoding** binds both accepted and rejected results to an architecturally justified complete byte extent.
- **Selector classification** classifies an opcode or first-word selector for source coverage but does not claim a lossless stream unit.
- **Strict decode only** exposes no rejected-byte recovery unit because a variable-length failure has no architecture-defined extent.

Classification never creates an executable instruction or decides that rejected bytes are data.

## Static meaning

An accepted, placed instruction may implement the common [static semantics protocol](SEMANTICS.md). Its analysis point contains the code location, exact decode profile, and minimal semantic context needed to interpret successor state and physical storage.

Static meaning is total only after the profile passes an independent semantic coverage gate. Encoding completeness does not imply semantic completeness, and the byte protocol remains usable without publishing a partial semantic table.

## Checked assembly

A checked assembler owns labels, declared data, instruction placement, and exact lowering for one profile. It resolves labels to `CodeLocation`, rejects unrepresentable or misaligned targets, and keeps declared data structurally distinct from executable instructions.

An assembled instruction span records both its output offset and architectural code location:

```rust
pub struct InstructionSpan<C, I> {
    pub offset: usize,
    pub location: C,
    pub bytes: InstructionBytes,
    pub instruction: I,
}
```

The output offset indexes the assembled byte buffer. It is not an alternative architectural address.

Every checked assembler exposes this same base span through `instruction_spans()`. Profile-specific placement evidence does not change the span shape: W65C816 width state, SH-4 FPSCR state, and V-series control-flow analysis are exposed separately and refer back to the same instruction offsets and code locations. A length-only span, a typed-instruction-only list, or a profile-specific placement value cannot substitute for this contract or be published under the `InstructionSpan` name.

Instruction progression is profile-owned. An assembler or verifier must fail closed when advancing would violate the profile's alignment, representable range, bank, segment, or explicit decode-state boundary.

## Final-write boundary

Typed decoding, static analysis, placement, and encoding establish intended machine code. The `expected-write` protocol separately establishes immutable-source preconditions, ownership, protected regions, collision policy, and final output bytes. Neither protocol infers the other's evidence.

An Expected Write adapter obtains each architectural code location from its externally resolved typed assembly source or from an explicit consumer mapping. A write offset, file extent, or disc location must never be cast or otherwise treated as a CPU code location. Re-decoding generated bytes at a location invented from the destination write would make the implementation validate itself instead of validating the declared source program.
