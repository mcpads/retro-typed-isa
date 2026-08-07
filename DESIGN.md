# Design Principles

This project provides static, typed instruction-set models for retro-game code analysis and patch generation.

## Purpose

An ISA profile describes executable instruction forms, typed operands, byte decoding and encoding, and placement constraints. It exists so a patch tool can state what code it expects to read and what code it intends to write.

CPU execution, mutable machine state, timing, pipelines, devices, and simulation belong elsewhere.

## Authority

The processor architecture manual is the normative source for an ISA profile. Corrections and well-established hardware findings may supplement it when the manual is incomplete or wrong, but each departure must be explicit and source-bound.

Independent decoders and assemblers may be used as separate research cross-checks. They can expose mistakes in this project, but they do not replace the source model, determine the public API, or run as Cargo dependencies or executable completeness oracles.

## Ownership

Each profile owns its instruction, operand, form, and error vocabulary. Consumers must not need a third-party decoder or emulator type to describe an instruction.

Closely related first-party profiles may share an operand type only when the architecture sources give that operand the same encoding and meaning. The exact profile must still own its instruction, form, and error boundary and reject every inherited encoding outside its scope. Shared implementation is never coverage evidence.

The shared core is a protocol for congruent APIs. It does not contain a universal instruction catalog and does not certify profile completeness.

Decode state that changes instruction identity belongs to an explicit protocol profile. A caller
must not rely on an implicit default for instruction-set state such as ARM versus Thumb, code-segment
size, W65C816 register widths, or SH-4 FPSCR.PR/SZ.

## Completeness

A profile is complete only within an explicit processor and extension scope. Completion requires all of the following:

- every encoding region in scope is classified as valid, reserved, or intentionally excluded;
- every valid form has a typed representation and checked byte lowering;
- decoding rejects reserved fields and encodings outside the profile;
- decoding and encoding preserve the exact selected form, including aliases that have distinct encodings;
- byte order, instruction length, address-relative fields, and placement ranges are enforced;
- the coverage claim is checked against source-derived facts that are independent of the implementation's own catalog.

Adding an instruction to both production code and its mirror test does not establish completeness.

## Final writes

Typed assembly proves the intended machine code. Expected-write verification separately proves that the immutable input bytes, ownership, protected regions, and final output bytes agree with that intent.

These are complementary gates. Neither substitutes for the other.
