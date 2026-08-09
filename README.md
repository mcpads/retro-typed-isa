# Retro Typed ISA

Rust libraries for safely analyzing and generating executable code in retro-game patch tools.

Each CPU profile is an independent crate with typed instructions and operands, byte decoding and encoding, placement-aware checked assembly, and the common `StaticSemantics` interface. The workspace also contains the CPU-independent `expected-write` final-byte verifier. Instruction execution and simulation are outside the project scope.

Every listed exact profile has source-derived encoding and semantic coverage gates. “Complete” in this repository is always capability- and profile-qualified: it covers the declared instruction profile, not timing, device behavior, undocumented extensions, or every processor in a wider architecture family.

## Documentation

Read the documents that own the relevant decision:

- [Design principles](DESIGN.md): project purpose, authority, layer ownership, completeness, and congruence.
- [Typed ISA protocol](ISA.md): common byte, code-location, classification, placement, and final-write boundaries.
- [Static semantics protocol](SEMANTICS.md): the implemented common vocabulary for static instruction meaning and its admission gate.
- [CPU profile semantics](PROFILE_SEMANTICS.md): application of that vocabulary to every supported profile.
- [Contributing](CONTRIBUTING.md): implementation and review requirements.

## Crates

- CPU profiles: `w65c816`, `psx-r3000a`, `huc6280`, `rp2a03`, `sh2`, `z80`, `sh4`, `m68000`, `v30`, `v30mz`, `sm83`, `v810`, `arm7tdmi`, `arm946e-s`, `tlcs900h`, `vr4300`, `powerpc-gekko-750cl`, `i386`, `allegrex`, `r5900`
- Shared protocol: `typed-isa-core`
- Final-byte verification: `expected-write`

Cargo registry publishing is currently disabled.

## License

MIT License. See [LICENSE](LICENSE).
