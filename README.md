# Retro Typed ISA

Rust libraries for safely analyzing and generating executable code in retro-game patch tools.

Each CPU profile is an independent crate with typed instructions and operands, byte decoding and encoding, and a placement-aware checked assembler. Instruction execution, CPU state transitions, pipelines, cycle timing, and simulation are outside this project's scope.

`typed-isa-core` defines the shared `TypedIsa` protocol and vocabulary, and every CPU crate exposes
one or more exact profiles through it. Decode state that changes instruction identity is named in
the profile, such as ARM/Thumb state, Intel code-segment size, W65C816 register widths, or SH-4
FPSCR.PR/SZ. The protocol does not certify instruction coverage.

The public semantic model is owned by this workspace. Public architecture manuals define each profile and its coverage denominator. Independent decoders or assemblers may be consulted separately, but they are not Cargo dependencies, executable test oracles, or sources of public instruction types.

See [Design principles](DESIGN.md) for the stable project boundaries and [Contributing](CONTRIBUTING.md) for profile implementation rules.

The CPU-independent `expected-write` crate verifies immutable-source preconditions, write ownership, protected regions, and final typed machine code.

## Crates

- CPU profiles: `w65c816`, `psx-r3000a`, `huc6280`, `rp2a03`, `sh2`, `z80`, `sh4`, `m68000`, `v30`, `v30mz`, `sm83`, `v810`, `arm7tdmi`, `arm946e-s`, `tlcs900h`, `vr4300`, `powerpc-gekko-750cl`, `i386`, `allegrex`, `r5900`
- Shared protocol: `typed-isa-core`
- Final-byte verification: `expected-write`

A CPU profile is described as complete only after its typed model has passed a source-derived coverage gate that is independent of the implementation's own catalog. Publishing to Cargo registries is currently disabled.

## License

MIT License. See [LICENSE](LICENSE).
