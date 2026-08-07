# Contributing

Read [Design principles](DESIGN.md) before adding or changing an ISA profile.

## Profile boundary

- Name a crate after the exact CPU or architecture profile it implements.
- State the processor revision and included extensions in its crate-level documentation.
- Keep instruction, operand, form, decode error, and encode error types owned by the profile.
- Implement the shared protocol for decode and encode congruence.
- Keep execution and simulation code out of this workspace.

## Source work

- Start from the architecture manual's instruction formats, opcode maps, reserved fields, and byte order.
- Record public source references in crate-level documentation or source-focused tests.
- Treat undocumented hardware behavior as a scoped supplement, not an unmarked replacement for the manual.
- Use third-party implementations only as manually invoked comparison points. Do not add them as normal or development dependencies, run them as the completeness gate, re-export their semantic types, or use their catalog as the production model.

## Coverage gate

- Derive the test denominator from public architecture sources, not from the profile's own form iterator or another implementation's catalog.
- Exhaust raw selector spaces when they are finite enough to enumerate.
- For larger fixed-width ISAs, check every source-listed form and every reserved-bit rule, then use independent differential decoding over generated field combinations.
- Require typed encode, byte decode, and placed round-trip checks for every valid form.
- Fail closed on unclassified encodings. Do not use an opaque instruction or raw-byte fallback for executable code.
- Do not describe a profile as complete until the independent gate passes.

## Public conventions

- Use `Instruction` for typed instruction values and `InstructionForm` for operand-independent forms.
- Use role-based operand names such as `Register`, `Condition`, `Address`, `Displacement`, and `Immediate`; add width or register class when ambiguity remains.
- Expose byte-oriented decode and encode functions. Natural instruction words may be additional APIs.
- Checked assemblers must distinguish executable instructions from declared data and reject placement overflow, misalignment, unresolved labels, and out-of-range control flow.
- Write public documentation, comments, errors, and test names in English.
