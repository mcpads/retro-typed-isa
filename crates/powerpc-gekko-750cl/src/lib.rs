//! Static IBM Gekko and PowerPC 750CL instruction-set profiles.
//!
//! The normative sources are IBM's [*Gekko RISC Microprocessor User's
//! Manual*][gekko-manual], version 1.2, and [*PowerPC 750CL RISC Microprocessor
//! User's Manual*][ibm-manual], preliminary version 1.0. Their executable
//! instruction forms are identical, including paired-single, quantized
//! load/store, and locked-cache instructions. Their special-purpose-register
//! maps differ: 750CL adds four instruction/data BAT pairs, HID4, TDCL, and
//! TDCH. `PowerPcGekko` and `PowerPc750Cl` therefore expose distinct protocol
//! profiles and reject registers outside the selected processor.
//!
//! The
//! corresponding base instruction tables in NXP's [*MPC750 RISC
//! Microprocessor Family User's Manual*][nxp-manual] provide an independent
//! primary-source check. In particular, MPC750 reserves primary opcode 4 while
//! 750CL assigns that space to its paired-single extension.
//!
//! Instructions are fixed-width and serialize in big-endian byte order.
//! Execution, timing, caches, and simulation are outside this crate.
//! Dolphin's pinned [Gekko/Broadway opcode table][dolphin-table] and
//! devkitPro's pinned [PowerPC opcode table][devkit-table] are additional
//! compatibility checks, not normative catalogs. Dolphin retains a `dcba`
//! dispatch slot, but its [interpreter explicitly identifies it as not a Gekko
//! instruction][dolphin-dcba]; devkitPro likewise does not enable `dcba` for
//! its PPC750/Gekko/Broadway target.
//!
//! [gekko-manual]: https://doc.kodewerx.org/documents/gekko_user_manual.pdf
//! [ibm-manual]: https://fail0verflow.com/media/files/ppc_750cl.pdf
//! [nxp-manual]: https://www.nxp.com/docs/en/reference-manual/MPC750UM.pdf
//! [dolphin-table]: https://github.com/dolphin-emu/dolphin/blob/66d8a892203d76a1f7349ed07526089b828e3835/Source/Core/Core/PowerPC/PPCTables.cpp
//! [dolphin-dcba]: https://github.com/dolphin-emu/dolphin/blob/66d8a892203d76a1f7349ed07526089b828e3835/Source/Core/Core/PowerPC/Interpreter/Interpreter_LoadStore.cpp#L463-L466
//! [devkit-table]: https://github.com/devkitPro/binutils-gdb/blob/55113f8e40d341b80d6f4b73cefeb2d3f0c134b1/opcodes/ppc-opc.c

#![deny(missing_docs)]

/// Stable profile identifier for the IBM Gekko instruction set.
pub const GEKKO_PROFILE_ID: &str = "powerpc-gekko";
/// Stable profile identifier for the IBM PowerPC 750CL instruction set.
pub const POWERPC_750CL_PROFILE_ID: &str = "powerpc-750cl";
/// Number of executable instruction forms declared by both profiles.
pub const DECLARED_FORM_COUNT: usize = 222;

mod assembler;
mod codec;
mod instruction;

pub use assembler::{
    AssembledProgram, Assembler, AssemblerError, GekkoAssembler, ProfileAssembler,
};
pub use codec::{
    DecodeError, EncodeError, PowerPc750Cl, PowerPcGekko, decode_750cl_bytes, decode_750cl_word,
    decode_gekko_bytes, decode_gekko_word, encode_750cl_bytes, encode_750cl_word,
    encode_gekko_bytes, encode_gekko_word,
};
pub use instruction::{
    BitIndex, BranchOptions, ConditionRegisterBit, ConditionRegisterField, ConditionRegisterMask,
    FloatingRegister, FpscrFieldMask, FpscrImmediate, GeneralRegister, Instruction,
    InstructionForm, InstructionModifiers, Operand, OperandKind, PairedSingleWidth,
    QuantizationRegister, SegmentRegister, SpecialPurposeRegister, StringByteCount,
    TimeBaseRegister, TrapCondition,
};
pub use typed_isa_core::InstructionSpan;
