//! Static NEC VR4300 profile for Nintendo 64 executable code.
//!
//! The normative source is NEC's [*VR4300, VR4305, VR4310 64-Bit
//! Microprocessor User's Manual*][nec-manual], document U10504EJ7V0UM00. The profile covers
//! the VR4300 CPU instructions in Figure 16-1 and the implemented FPU
//! combinations in Tables 17-1 and 17-2. The corresponding tables in the
//! original [MIPS R4000 Microprocessor User's Manual][mips-manual] are used as
//! a second primary-source check. It deliberately excludes the
//! implementation-defined CP2 operation field: the manual assigns no semantic
//! operations to CP2, so accepting its raw 25-bit field would create an opaque
//! executable-code escape hatch.
//!
//! Instructions are fixed-width and serialize in the big-endian byte order
//! used by Nintendo 64 executable images. Execution, timing, caches, and
//! simulation are outside this crate.
//!
//! [nec-manual]: https://datasheets.chipdb.org/NEC/Vr-Series/Vr43xx/U10504EJ7V0UMJ1.pdf
//! [mips-manual]: https://groups.csail.mit.edu/cag/raw/documents/R4400_Uman_book_Ed2.pdf

#![deny(missing_docs)]

/// Stable identifier for the big-endian VR4300 protocol profile.
pub const PROFILE_ID: &str = env!("CARGO_PKG_NAME");

mod assembler;
mod codec;
mod instruction;

pub use assembler::{AssembledProgram, Assembler, AssemblerError};
pub use codec::{
    DecodeError, EncodeError, Vr4300, decode_bytes, decode_word, encode_bytes, encode_word,
};
pub use instruction::{
    CacheOperation, Coprocessor, Coprocessor2ControlRegister, Coprocessor2Register,
    CoprocessorBranchCondition, CoprocessorMemoryOperation, CoprocessorMemoryRegister,
    Cp0Operation, Cp0Register, EqualityCondition, FixedFormat, FloatingBinaryOperation,
    FloatingComparePredicate, FloatingFormat, FloatingRegister, FloatingRounding,
    FloatingUnaryOperation, FpuControlRegister, GeneralRegister, ImmediateArithmeticOperation,
    Instruction, InstructionForm, LogicalImmediateOperation, MemoryOperation,
    MultiplyDivideOperation, NumericFormat, ShiftOperation, SignCondition, SpecialRegister,
    ThreeRegisterOperation, TrapCondition,
};
pub use typed_isa_core::InstructionSpan;
