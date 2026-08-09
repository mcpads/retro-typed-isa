# Design Principles

This project provides static, typed instruction-set models for retro-game code analysis and patch generation.

## Purpose

An ISA profile describes executable instruction forms, typed operands, exact byte decoding and encoding, static meaning, and placement constraints. It exists so a tool can state what code it expects to read, what a source program intends to do, and what code it intends to write.

CPU execution, mutable machine state, timing, pipelines, devices, and simulation belong elsewhere.

## Architectural layers

The project separates four kinds of evidence:

1. **Encoding identity** states which exact source bytes name an instruction in an explicit profile.
2. **Static meaning** states the instruction's possible control transfer, architectural storage accesses, and statically resolvable non-control references.
3. **Placement and lowering** prove that typed instructions can occupy their selected addresses and lower to exact bytes.
4. **Final-write verification** proves that immutable input, write ownership, protected regions, and final output agree with the declared patch.

No layer may infer the evidence owned by another. A successful decode does not authorize a write, a successful encode does not prove the original bytes, and a static effect does not simulate execution.

## Static meaning

Static meaning contains facts that follow from an instruction, its placement, and explicitly supplied architectural context. It is declarative and conservative. It may report multiple possible successors or accesses, but it does not calculate runtime values or mutate a machine state.

Decode state and semantic context are distinct. Decode state selects instruction identity or extent and therefore belongs to an explicit protocol profile. Semantic context identifies architectural storage or successor state without changing the encoded instruction, such as the active register bank. A required context value is never supplied by an implicit default. An explicitly unknown value must produce a sound union of effects or a fail-closed analysis boundary.

Every profile uses the same concepts for the same facts. Architecture-specific registers, status fields, address spaces, and instruction-set states remain profile-owned values inside those common concepts. A profile difference may change the data carried by a concept; it must not silently change the concept's meaning or public shape.

Static meaning is total within its declared scope. Every typed instruction form has an explicit control-flow classification and an explicit set of architectural storage accesses, including implicit operands and address updates. The empty set means that the source establishes no such effect. Missing analysis is not represented as an empty set.

Static meaning supports source lowering and binary patching, but it is not a compiler or a proof that two arbitrary programs compute the same values. Higher-level instruction selection and value equivalence belong to separate layers that consume the ISA facts.

## Congruence

The shared protocol is the single vocabulary for cross-profile concepts. A consumer requirement may reveal a missing concept, but no consumer may define that concept by adding a one-off profile method.

A new cross-profile concept is designed before any profile exposes it. Its name, invariants, failure boundary, and applicability are reviewed across the whole workspace, and every applicable profile adopts it in the same migration. Convenience queries derive from the shared fact model rather than becoming independent sources of truth.

Architecture differences that prevent a capability from being sound are explicit. In particular, lossless recovery after a rejected variable-length encoding is not invented merely to make one API appear universal.

Implementation is evidence about design pressure, not authority over the design. If an implementation cannot conform to the current design, work stops at that boundary. Changing only the affected profile, test, or local design paragraph to accommodate it is not sufficient. The proposed design change must re-review the project purpose, layer ownership, common vocabulary, invariants, failure behavior, every applicable profile, consumer obligations, and both completeness gates. Implementation resumes only after the revised whole remains congruent; otherwise the implementation must change or be removed.

## Authority

The processor architecture manual is the normative source for an ISA profile. Corrections and well-established hardware findings may supplement it when the manual is incomplete or wrong, but each departure must be explicit and source-bound.

Independent decoders and assemblers may be used as separate research cross-checks. They can expose mistakes in this project, but they do not replace the source model, determine the public API, or participate in the completeness decision.

Static meaning follows the same authority rule as encoding. Operand names, instruction variants, and consumer behavior are not evidence for an implicit register access, control transfer, or static reference.

## Ownership

Each profile owns its instruction, operand, form, error, code-location, analysis-point, and architectural storage-location vocabulary. Consumers must not need a third-party decoder or emulator type to describe an instruction or its static meaning.

Closely related first-party profiles may share an operand or location type only when the architecture sources give it the same encoding and meaning. The exact profile must still own its instruction, form, error, and semantic coverage boundary and reject every inherited form outside its scope. Shared implementation is never coverage evidence.

The shared core is a protocol for congruent APIs. It does not contain a universal instruction catalog, universal register enum, or universal address integer, and it does not certify profile completeness.

Decode state that changes instruction identity belongs to an explicit protocol profile. A caller must not rely on an implicit default for instruction-set state such as ARM versus Thumb, code-segment size, W65C816 register widths, SH-4 FPSCR.PR/SZ, or VR4300 Status.FR.

## Completeness

Completeness claims are capability-qualified and limited to an explicit processor and extension scope. `Encoding-complete` and `semantic-complete` are independent claims. Unqualified `complete` may mean only that both gates pass and should otherwise be avoided.

Encoding completion requires all of the following:

- every encoding region in scope is classified as valid, reserved, or intentionally excluded;
- every valid form has a typed representation and checked byte lowering;
- decoding rejects reserved fields and encodings outside the profile;
- decoding and encoding preserve the exact selected form, including aliases that have distinct encodings;
- byte order, instruction length, address-relative fields, and placement ranges are enforced;
- the coverage claim is checked against source-derived facts that are independent of the implementation's own catalog.

Static-meaning completion additionally requires all of the following:

- every source-listed instruction form has an independently derived semantic entry;
- explicit and implicit architectural storage accesses are classified;
- direct, indirect, conditional, delayed, explicit synchronous-boundary, and terminating control flow are distinguished where the profile defines them;
- statically resolvable references use the profile's real address model;
- behavior outside the static model is stated as out of scope rather than hidden behind an opaque effect.

Adding an instruction to production code and to a mirror test does not establish either kind of completeness.

Completeness is a reviewed conclusion over independent evidence, not the
result of one self-certifying test. The source-derived denominator proves that
the declared profile boundary reaches every admitted form. The exhaustive
production rule mapping is then reviewed against the architecture source, and
source-focused behavioral tests preserve error-prone distinctions such as
implicit storage, aliasing, delayed flow, state changes, and address
calculation. A denominator test that merely obtains a semantic result proves
total dispatch, not that the result is correct. Conversely, a collection of
examples cannot replace the independent denominator or the whole-profile
source review.

## Source lowering and patching

Source lowering selects typed instructions, then uses static meaning to validate control flow, clobbers, references, instruction-set state, and placement before checked byte lowering.

Binary patching strictly decodes a declared code region, derives its static facts, constructs a replacement from typed instructions, validates the replacement facts, and only then lowers it. Rejected encodings and declared data may be preserved for analysis, but they never become executable instructions through a raw-byte fallback.

## Final writes

Typed assembly proves the intended machine code. Expected-write verification separately proves that the immutable input bytes, ownership, protected regions, and final output bytes agree with that intent.

These are complementary gates. Neither substitutes for the other.

The externally resolved typed assembly source owns replacement instruction identity and architectural placement. The final-write destination owns file, ROM, or disc offsets. An adapter must compare the two through explicit consumer mapping; it must not derive a CPU location from a destination offset or reconstruct its authority from the replacement bytes it is supposed to verify.
