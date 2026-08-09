//! Static NEC VR4300 profiles for Nintendo 64 executable code.
//!
//! The normative source is NEC's [*VR4300, VR4305, VR4310 64-Bit
//! Microprocessor User's Manual*][nec-manual], document U10504EJ7V0UM00. The profiles cover
//! the VR4300 CPU instructions in Figure 16-1 and the implemented FPU
//! combinations in Tables 17-1 and 17-2. The corresponding tables in the
//! original [MIPS R4000 Microprocessor User's Manual][mips-manual] are used as
//! a second primary-source check. `Vr4300Fr0` and `Vr4300Fr1` make CP0
//! Status.FR explicit because it changes
//! which floating-register fields identify valid logical FPRs. Both exclude the
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

/// Stable identifier for the big-endian VR4300 profile with CP0 Status.FR clear.
pub const FR0_PROFILE_ID: &str = "vr4300-fr0-be";
/// Stable identifier for the big-endian VR4300 profile with CP0 Status.FR set.
pub const FR1_PROFILE_ID: &str = "vr4300-fr1-be";

/// Aligned 64-bit VR4300 virtual code location.
pub type CodeLocation = u64;

mod assembler;
mod codec;
mod instruction;
mod semantics;

pub use assembler::{AssembledProgram, Assembler, AssemblerError};
pub use codec::{
    DecodeError, EncodeError, StatusFr, Vr4300Fr0, Vr4300Fr1, decode_bytes, decode_word,
    encode_bytes, encode_word,
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
pub use semantics::{
    AnalysisPoint, Boundary, ControlFlow, FloatingStorage, FpuState, ImplicitState, IndirectTarget,
    InstructionSemantics, Location, MemoryAddress, MemoryLocation, MemoryWidth, Reference,
    SemanticError,
};
pub use typed_isa_core::InstructionSpan;

macro_rules! implement_static_semantics {
    ($profile:ty, $status_fr:expr) => {
        impl typed_isa_core::StaticSemantics for $profile {
            type AnalysisPoint = AnalysisPoint;
            type IndirectTarget = IndirectTarget;
            type Boundary = Boundary;
            type Location = Location;
            type Reference = Reference;
            type SemanticError = SemanticError;

            fn semantics(
                instruction: &Self::Instruction,
                at: &Self::AnalysisPoint,
            ) -> Result<InstructionSemantics, Self::SemanticError> {
                semantics::semantics(instruction, *at, $status_fr)
            }

            fn storage_relation(
                left: &Self::Location,
                right: &Self::Location,
            ) -> typed_isa_core::StorageRelation {
                semantics::storage_relation(left, right)
            }
        }
    };
}

implement_static_semantics!(Vr4300Fr0, StatusFr::Zero);
implement_static_semantics!(Vr4300Fr1, StatusFr::One);
