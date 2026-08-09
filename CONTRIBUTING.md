# Contributing

Read [Design principles](DESIGN.md), the [typed ISA protocol](ISA.md), the common [static semantics protocol](SEMANTICS.md), and the relevant [CPU profile design](PROFILE_SEMANTICS.md) before adding or changing an ISA profile.

## Profile boundary

- Name a crate after the exact CPU or architecture profile it implements.
- State the processor revision and included extensions in its crate-level documentation.
- Keep instruction, operand, form, decode error, and encode error types owned by the profile.
- Implement the shared protocol for decode and encode congruence.
- Make the analysis point, semantic context, indirect targets, architectural locations, storage overlap, control boundaries, and static references explicit for the profile.
- Keep execution and simulation code out of this workspace.

## Source work

- Start from the architecture manual's instruction formats, opcode maps, reserved fields, and byte order.
- Record public source references in crate-level documentation or source-focused tests.
- Treat undocumented hardware behavior as a scoped supplement, not an unmarked replacement for the manual.
- Use third-party implementations only as manually invoked comparison points. Do not add them as normal or development dependencies, run them as the completeness gate, re-export their semantic types, or use their catalog as the production model.

## Coverage gate

- Derive the test denominator from public architecture sources, not from the profile's own form iterator or another implementation's catalog.
- Exhaust raw selector spaces when they are finite enough to enumerate.
- For larger fixed-width ISAs, derive field predicates and reserved-bit rules directly from reviewed source tables, then enumerate generated field combinations against that independent denominator.
- Require typed encode, byte decode, and placed round-trip checks for every valid form.
- Fail closed on unclassified encodings. Do not use an opaque instruction or raw-byte fallback for executable code.
- Do not describe a profile as encoding-complete until the independent encoding gate passes.

## Static semantics gate

- Derive a semantic denominator from the architecture sources independently of the production instruction enum and tables.
- Use that denominator to prove that every valid form and required context reaches an explicit semantic rule. A total-dispatch test does not by itself prove that the returned facts are correct.
- Review the exhaustive rule mapping against the architecture source, including explicit and implicit storage accesses, partial-storage overlap, control flow, boundaries, delay slots, state transitions, and static references.
- Add source-focused behavioral tests for distinctions that are easy to erase accidentally. Do not duplicate the production semantic table as a self-mirroring expected-output catalog.
- Treat empty effects as explicit source-derived entries. Missing analysis must fail closed and must not masquerade as an empty result.
- Keep ABI conventions, consumer memory maps, runtime register values, device behavior, and simulation out of ISA facts.
- Migrate all applicable profiles to a new common concept as one workspace change. Profile-specific queries may remain only as derived views over the shared result.
- Do not describe a profile as semantic-complete until the independent semantic gate passes.

## Design-change review

Implementation does not amend the design by precedent. If implementation exposes a contradiction or missing concept, pause it and perform the whole-workspace review required by [Congruence](DESIGN.md#congruence). The review covers the central protocol, every applicable profile, consumers, and both completeness gates; a local exception or implementation-shaped test is not sufficient.

Keep implementation units reviewable: one unit establishes a complete shared capability or a complete profile boundary together with its independent evidence and tests. Do not publish a partial common semantic table under a complete-looking API.

## Public conventions

- Use `Instruction` for typed instruction values and `InstructionForm` for operand-independent forms.
- Use role-based operand names such as `Register`, `Condition`, `Address`, `Displacement`, and `Immediate`; add width or register class when ambiguity remains.
- Use the common semantic names and meanings from [the protocol](SEMANTICS.md); profile-owned types carry architecture distinctions rather than redefining the common concepts.
- Expose byte-oriented decode and encode functions. Natural instruction words may be additional APIs.
- Checked assemblers must distinguish executable instructions from declared data and reject placement overflow, misalignment, unresolved labels, and out-of-range control flow.
- Write public documentation, comments, errors, and test names in English.
