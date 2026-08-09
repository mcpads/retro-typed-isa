//! Complete static meaning for the VR4300 FR=0 and FR=1 profiles.

use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlBoundary, ControlTarget, DelaySlot,
    DelaySlotExecution, LocationAccess, ReferenceKind, StaticReference, StorageRelation,
};

use crate::{
    Coprocessor, Coprocessor2ControlRegister, Coprocessor2Register, CoprocessorMemoryOperation,
    CoprocessorMemoryRegister, Cp0Operation, Cp0Register, EncodeError, FixedFormat, FloatingFormat,
    FloatingRegister, FpuControlRegister, GeneralRegister, Instruction, MemoryOperation,
    NumericFormat, SpecialRegister, StatusFr,
};

/// Placed VR4300 instruction used by static analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnalysisPoint {
    /// 64-bit virtual instruction address.
    pub location: crate::CodeLocation,
}

impl AnalysisPoint {
    /// Constructs a placed VR4300 instruction.
    pub const fn new(location: crate::CodeLocation) -> Self {
        Self { location }
    }
}

/// Runtime-derived VR4300 control destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndirectTarget {
    /// Address held in one general-purpose register.
    Register(GeneralRegister),
    /// Address recovered from CP0 exception state by ERET.
    ExceptionReturn,
    /// Processor-defined general exception vector.
    GeneralExceptionVector,
}

/// Synchronous VR4300 trap kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Boundary {
    /// SYSCALL with its code field.
    SystemCall(u32),
    /// BREAK with its code field.
    Breakpoint(u32),
    /// Conditional integer trap.
    IntegerTrap,
}

/// One physical FPU register byte range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FloatingStorage {
    /// Byte offset in the profile-selected physical FPU register file.
    pub byte_offset: u16,
    /// Referenced byte count.
    pub byte_len: u8,
}

/// One FPU control/status subfield.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FpuState {
    /// Rounding-mode field.
    RoundingMode,
    /// Cause, flag, and enable state changed by arithmetic exceptions.
    ExceptionState,
    /// Condition bit consumed by BC1 and written by compare instructions.
    Condition,
}

/// Stable processor state not represented by an explicit operand register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImplicitState {
    /// HI multiply/divide result register.
    Hi,
    /// LO multiply/divide result register.
    Lo,
    /// LLbit and associated linked-address reservation.
    LoadLinkedReservation,
    /// Translation lookaside buffer contents.
    Tlb,
    /// Cache state selected by a CACHE operation.
    Cache(u8),
    /// Ordering effect established by SYNC.
    MemoryOrdering,
}

/// Symbolic effective address of one data access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryAddress {
    /// Base general-purpose register.
    pub base: GeneralRegister,
    /// Signed byte displacement.
    pub offset: i16,
}

/// Width and merge rule of one memory access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryWidth {
    /// Fixed contiguous byte count.
    Known(u8),
    /// Unaligned left/right merge into a register of this byte width.
    Merge(u8),
}

/// Symbolic VR4300 memory storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryLocation {
    /// Effective-address rule.
    pub address: MemoryAddress,
    /// Transfer width or merge rule.
    pub width: MemoryWidth,
}

/// VR4300 architectural storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Location {
    /// One 64-bit general-purpose register.
    GeneralRegister(GeneralRegister),
    /// Profile-resolved physical FPU storage range.
    Floating(FloatingStorage),
    /// One CP0 register.
    Cp0(Cp0Register),
    /// One FPU control register.
    FpuControl(FpuControlRegister),
    /// One FPU control/status subfield.
    FpuState(FpuState),
    /// One CP2 general register.
    Coprocessor2(Coprocessor2Register),
    /// One CP2 control register.
    Coprocessor2Control(Coprocessor2ControlRegister),
    /// Stable implicit processor state.
    Implicit(ImplicitState),
    /// Symbolic memory storage.
    Memory(MemoryLocation),
}

/// Statically resolved VR4300 address reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reference {
    /// Memory range whose base is the constant-zero register.
    Memory {
        /// First referenced virtual byte address.
        address: u64,
        /// Referenced byte count or merge-register width.
        byte_len: u8,
    },
    /// Statically formed address without a claimed data transfer.
    Address(u64),
}

/// Failure to form sound VR4300 static facts.
#[derive(Debug, thiserror::Error)]
pub enum SemanticError {
    /// The typed instruction is not encodable in the selected Status.FR profile.
    #[error(transparent)]
    Encode(#[from] EncodeError),
}

/// Shared control-flow result specialized with VR4300 vocabulary.
pub type ControlFlow = typed_isa_core::ControlFlow<AnalysisPoint, IndirectTarget, Boundary>;

/// Shared instruction-semantics result specialized with VR4300 vocabulary.
pub type InstructionSemantics = typed_isa_core::InstructionSemantics<
    AnalysisPoint,
    IndirectTarget,
    Boundary,
    Location,
    Reference,
>;

pub(crate) fn semantics(
    instruction: &Instruction,
    at: AnalysisPoint,
    status_fr: StatusFr,
) -> Result<InstructionSemantics, SemanticError> {
    crate::encode_word(instruction, at.location, status_fr)?;
    let next = AnalysisPoint::new(at.location.wrapping_add(4));
    let after_slot = AnalysisPoint::new(at.location.wrapping_add(8));
    let mut accesses = Vec::new();
    let mut references = Vec::new();
    instruction_accesses(instruction, status_fr, &mut accesses, &mut references);
    Ok(InstructionSemantics {
        control_flow: control_flow(instruction, next, after_slot),
        location_accesses: accesses,
        static_references: references,
    })
}

fn control_flow(
    instruction: &Instruction,
    next: AnalysisPoint,
    after_slot: AnalysisPoint,
) -> ControlFlow {
    match *instruction {
        Instruction::EqualityBranch { likely, target, .. }
        | Instruction::SignBranch {
            likely,
            link: false,
            target,
            ..
        }
        | Instruction::CoprocessorBranch { likely, target, .. } => {
            branch_flow(target, likely, next, after_slot)
        }
        Instruction::SignBranch {
            likely,
            link: true,
            target,
            ..
        } => ControlFlow {
            action: ControlAction::LinkedTransfer {
                target: ControlTarget::Direct(AnalysisPoint::new(target)),
                return_site: after_slot,
            },
            fallthrough: Some(after_slot),
            delay_slot: Some(DelaySlot {
                point: next,
                execution: if likely {
                    DelaySlotExecution::WhenTransferOccurs
                } else {
                    DelaySlotExecution::Always
                },
            }),
        },
        Instruction::Jump { link, target } => ControlFlow {
            action: if link {
                ControlAction::LinkedTransfer {
                    target: ControlTarget::Direct(AnalysisPoint::new(target)),
                    return_site: after_slot,
                }
            } else {
                ControlAction::Transfer {
                    target: ControlTarget::Direct(AnalysisPoint::new(target)),
                }
            },
            fallthrough: None,
            delay_slot: Some(DelaySlot {
                point: next,
                execution: DelaySlotExecution::Always,
            }),
        },
        Instruction::JumpRegister { link, target } => ControlFlow {
            action: match link {
                Some(_) => ControlAction::LinkedTransfer {
                    target: ControlTarget::Indirect(IndirectTarget::Register(target)),
                    return_site: after_slot,
                },
                None => ControlAction::Transfer {
                    target: ControlTarget::Indirect(IndirectTarget::Register(target)),
                },
            },
            fallthrough: None,
            delay_slot: Some(DelaySlot {
                point: next,
                execution: DelaySlotExecution::Always,
            }),
        },
        Instruction::SystemCall { code } => trap_flow(Boundary::SystemCall(code), next, false),
        Instruction::Breakpoint { code } => trap_flow(Boundary::Breakpoint(code), next, false),
        Instruction::TrapRegister { .. } | Instruction::TrapImmediate { .. } => {
            trap_flow(Boundary::IntegerTrap, next, true)
        }
        Instruction::Cp0(Cp0Operation::ReturnFromException) => ControlFlow {
            action: ControlAction::ExceptionReturn {
                target: IndirectTarget::ExceptionReturn,
            },
            fallthrough: None,
            delay_slot: None,
        },
        _ => ControlFlow {
            action: ControlAction::Continue,
            fallthrough: Some(next),
            delay_slot: None,
        },
    }
}

fn branch_flow(
    target: u64,
    likely: bool,
    next: AnalysisPoint,
    after_slot: AnalysisPoint,
) -> ControlFlow {
    ControlFlow {
        action: ControlAction::Transfer {
            target: ControlTarget::Direct(AnalysisPoint::new(target)),
        },
        fallthrough: Some(after_slot),
        delay_slot: Some(DelaySlot {
            point: next,
            execution: if likely {
                DelaySlotExecution::WhenTransferOccurs
            } else {
                DelaySlotExecution::Always
            },
        }),
    }
}

fn trap_flow(kind: Boundary, next: AnalysisPoint, conditional: bool) -> ControlFlow {
    ControlFlow {
        action: ControlAction::Boundary(ControlBoundary::Trap {
            kind,
            target: Some(ControlTarget::Indirect(
                IndirectTarget::GeneralExceptionVector,
            )),
            resume: None,
        }),
        fallthrough: conditional.then_some(next),
        delay_slot: None,
    }
}

fn instruction_accesses(
    instruction: &Instruction,
    status_fr: StatusFr,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    use Instruction::*;
    match *instruction {
        ThreeRegister {
            destination,
            left,
            right,
            ..
        } => {
            read_gprs(accesses, [left, right]);
            write_gpr(accesses, destination);
        }
        ImmediateArithmetic {
            destination,
            source,
            ..
        }
        | LogicalImmediate {
            destination,
            source,
            ..
        }
        | ShiftImmediate {
            destination,
            source,
            ..
        } => {
            read_gpr(accesses, source);
            write_gpr(accesses, destination);
        }
        LoadUpperImmediate { destination, .. } => write_gpr(accesses, destination),
        ShiftVariable {
            destination,
            source,
            amount_register,
            ..
        } => {
            read_gprs(accesses, [source, amount_register]);
            write_gpr(accesses, destination);
        }
        MultiplyDivide { left, right, .. } => {
            read_gprs(accesses, [left, right]);
            for state in [ImplicitState::Hi, ImplicitState::Lo] {
                write(
                    accesses,
                    Location::Implicit(state),
                    AccessOccurrence::Always,
                );
            }
        }
        MoveFromSpecial {
            source,
            destination,
        } => {
            read(accesses, special(source), AccessOccurrence::Always);
            write_gpr(accesses, destination);
        }
        MoveToSpecial {
            source,
            destination,
        } => {
            read_gpr(accesses, source);
            write(accesses, special(destination), AccessOccurrence::Always);
        }
        Memory {
            operation,
            register,
            base,
            offset,
        } => cpu_memory(operation, register, base, offset, accesses, references),
        EqualityBranch {
            left,
            right,
            target: _,
            ..
        } => {
            read_gprs(accesses, [left, right]);
        }
        SignBranch {
            link,
            source,
            target: _,
            ..
        } => {
            read_gpr(accesses, source);
            if link {
                write_gpr(accesses, GeneralRegister::RA);
            }
        }
        Jump { link, target: _ } => {
            if link {
                write_gpr(accesses, GeneralRegister::RA);
            }
        }
        JumpRegister { link, target } => {
            read_gpr(accesses, target);
            if let Some(destination) = link {
                write_gpr(accesses, destination);
            }
        }
        SystemCall { .. } | Breakpoint { .. } => {
            exception_state(accesses, AccessOccurrence::Always)
        }
        Synchronize => write(
            accesses,
            Location::Implicit(ImplicitState::MemoryOrdering),
            AccessOccurrence::Always,
        ),
        TrapRegister { left, right, .. } => {
            read_gprs(accesses, [left, right]);
            exception_state(accesses, AccessOccurrence::Conditional);
        }
        TrapImmediate { source, .. } => {
            read_gpr(accesses, source);
            exception_state(accesses, AccessOccurrence::Conditional);
        }
        MoveFromCp0 {
            destination,
            source,
            ..
        } => {
            read(accesses, Location::Cp0(source), AccessOccurrence::Always);
            write_gpr(accesses, destination);
        }
        MoveToCp0 {
            source,
            destination,
            ..
        } => {
            read_gpr(accesses, source);
            write(
                accesses,
                Location::Cp0(destination),
                AccessOccurrence::Always,
            );
        }
        MoveFromFpu {
            doubleword,
            destination,
            source,
        } => {
            read(
                accesses,
                Location::Floating(fpr(status_fr, source, if doubleword { 8 } else { 4 })),
                AccessOccurrence::Always,
            );
            write_gpr(accesses, destination);
        }
        MoveToFpu {
            doubleword,
            source,
            destination,
        } => {
            read_gpr(accesses, source);
            write(
                accesses,
                Location::Floating(fpr(status_fr, destination, if doubleword { 8 } else { 4 })),
                AccessOccurrence::Always,
            );
        }
        MoveControlFromFpu {
            destination,
            source,
        } => {
            read(
                accesses,
                Location::FpuControl(source),
                AccessOccurrence::Always,
            );
            write_gpr(accesses, destination);
        }
        MoveControlToFpu {
            source,
            destination,
        } => {
            read_gpr(accesses, source);
            write(
                accesses,
                Location::FpuControl(destination),
                AccessOccurrence::Always,
            );
        }
        MoveFromCoprocessor2 {
            destination,
            source,
            ..
        } => {
            read(
                accesses,
                Location::Coprocessor2(source),
                AccessOccurrence::Always,
            );
            write_gpr(accesses, destination);
        }
        MoveToCoprocessor2 {
            source,
            destination,
            ..
        } => {
            read_gpr(accesses, source);
            write(
                accesses,
                Location::Coprocessor2(destination),
                AccessOccurrence::Always,
            );
        }
        MoveControlFromCoprocessor2 {
            destination,
            source,
        } => {
            read(
                accesses,
                Location::Coprocessor2Control(source),
                AccessOccurrence::Always,
            );
            write_gpr(accesses, destination);
        }
        MoveControlToCoprocessor2 {
            source,
            destination,
        } => {
            read_gpr(accesses, source);
            write(
                accesses,
                Location::Coprocessor2Control(destination),
                AccessOccurrence::Always,
            );
        }
        CoprocessorBranch {
            coprocessor,
            target: _,
            ..
        } => {
            read(
                accesses,
                coprocessor_condition(coprocessor),
                AccessOccurrence::Always,
            );
        }
        CoprocessorMemory {
            operation,
            register,
            base,
            offset,
        } => coprocessor_memory(
            operation, register, base, offset, status_fr, accesses, references,
        ),
        Cp0(operation) => cp0_operation(operation, accesses),
        Cache {
            operation,
            base,
            offset,
        } => {
            read_gpr(accesses, base);
            write(
                accesses,
                Location::Implicit(ImplicitState::Cache(operation.value())),
                AccessOccurrence::Always,
            );
            add_address_reference(references, base, offset);
        }
        FloatingBinary {
            format,
            destination,
            left,
            right,
            ..
        } => {
            let width = floating_width(format);
            read_fprs(accesses, status_fr, [(left, width), (right, width)]);
            write_fpr(accesses, status_fr, destination, width);
            floating_arithmetic_state(accesses, true);
        }
        FloatingUnary {
            operation,
            format,
            destination,
            source,
        } => {
            let width = floating_width(format);
            read_fpr(accesses, status_fr, source, width);
            write_fpr(accesses, status_fr, destination, width);
            if matches!(operation, crate::FloatingUnaryOperation::SquareRoot) {
                floating_arithmetic_state(accesses, true);
            }
        }
        FloatingRoundToInteger {
            destination_format,
            source_format,
            destination,
            source,
            ..
        } => {
            read_fpr(accesses, status_fr, source, floating_width(source_format));
            write_fpr(
                accesses,
                status_fr,
                destination,
                fixed_width(destination_format),
            );
            floating_arithmetic_state(accesses, false);
        }
        FloatingConvert {
            destination_format,
            source_format,
            destination,
            source,
        } => {
            read_fpr(accesses, status_fr, source, numeric_width(source_format));
            write_fpr(
                accesses,
                status_fr,
                destination,
                numeric_width(destination_format),
            );
            floating_arithmetic_state(accesses, true);
        }
        FloatingCompare {
            format,
            left,
            right,
            ..
        } => {
            let width = floating_width(format);
            read_fprs(accesses, status_fr, [(left, width), (right, width)]);
            write(
                accesses,
                Location::FpuState(FpuState::Condition),
                AccessOccurrence::Always,
            );
            write(
                accesses,
                Location::FpuState(FpuState::ExceptionState),
                AccessOccurrence::Conditional,
            );
        }
    }
}

fn cpu_memory(
    operation: MemoryOperation,
    register: GeneralRegister,
    base: GeneralRegister,
    offset: i16,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    read_gpr(accesses, base);
    let (load, width, linked, conditional) = cpu_memory_kind(operation);
    let memory = Location::Memory(MemoryLocation {
        address: MemoryAddress { base, offset },
        width,
    });
    if load {
        read(accesses, memory, AccessOccurrence::Always);
        if matches!(width, MemoryWidth::Merge(_)) {
            read_gpr(accesses, register);
        }
        write_gpr(accesses, register);
    } else {
        read_gpr(accesses, register);
        write(
            accesses,
            memory,
            if conditional {
                AccessOccurrence::Conditional
            } else {
                AccessOccurrence::Always
            },
        );
        if conditional {
            write_gpr(accesses, register);
        }
    }
    if linked {
        write(
            accesses,
            Location::Implicit(ImplicitState::LoadLinkedReservation),
            AccessOccurrence::Always,
        );
    }
    if conditional {
        read(
            accesses,
            Location::Implicit(ImplicitState::LoadLinkedReservation),
            AccessOccurrence::Always,
        );
        write(
            accesses,
            Location::Implicit(ImplicitState::LoadLinkedReservation),
            AccessOccurrence::Always,
        );
    }
    add_memory_reference(
        references,
        base,
        offset,
        width_bytes(width),
        if load {
            ReferenceKind::Read
        } else {
            ReferenceKind::Write
        },
    );
}

fn cpu_memory_kind(operation: MemoryOperation) -> (bool, MemoryWidth, bool, bool) {
    use MemoryOperation::*;
    match operation {
        LoadByte | LoadByteUnsigned => (true, MemoryWidth::Known(1), false, false),
        LoadHalfword | LoadHalfwordUnsigned => (true, MemoryWidth::Known(2), false, false),
        LoadWord | LoadWordUnsigned => (true, MemoryWidth::Known(4), false, false),
        LoadDoubleword => (true, MemoryWidth::Known(8), false, false),
        LoadWordLeft | LoadWordRight => (true, MemoryWidth::Merge(4), false, false),
        LoadDoublewordLeft | LoadDoublewordRight => (true, MemoryWidth::Merge(8), false, false),
        LoadLinked => (true, MemoryWidth::Known(4), true, false),
        LoadLinkedDoubleword => (true, MemoryWidth::Known(8), true, false),
        StoreByte => (false, MemoryWidth::Known(1), false, false),
        StoreHalfword => (false, MemoryWidth::Known(2), false, false),
        StoreWord => (false, MemoryWidth::Known(4), false, false),
        StoreDoubleword => (false, MemoryWidth::Known(8), false, false),
        StoreWordLeft | StoreWordRight => (false, MemoryWidth::Merge(4), false, false),
        StoreDoublewordLeft | StoreDoublewordRight => (false, MemoryWidth::Merge(8), false, false),
        StoreConditional => (false, MemoryWidth::Known(4), false, true),
        StoreConditionalDoubleword => (false, MemoryWidth::Known(8), false, true),
    }
}

fn coprocessor_memory(
    operation: CoprocessorMemoryOperation,
    register: CoprocessorMemoryRegister,
    base: GeneralRegister,
    offset: i16,
    status_fr: StatusFr,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    read_gpr(accesses, base);
    let (load, width) = match operation {
        CoprocessorMemoryOperation::LoadWord => (true, 4),
        CoprocessorMemoryOperation::StoreWord => (false, 4),
        CoprocessorMemoryOperation::LoadDoubleword => (true, 8),
        CoprocessorMemoryOperation::StoreDoubleword => (false, 8),
    };
    let memory = Location::Memory(MemoryLocation {
        address: MemoryAddress { base, offset },
        width: MemoryWidth::Known(width),
    });
    let register = match register {
        CoprocessorMemoryRegister::FloatingPoint(register) => {
            Location::Floating(fpr(status_fr, register, width))
        }
        CoprocessorMemoryRegister::Coprocessor2(register) => Location::Coprocessor2(register),
    };
    if load {
        read(accesses, memory, AccessOccurrence::Always);
        write(accesses, register, AccessOccurrence::Always);
    } else {
        read(accesses, register, AccessOccurrence::Always);
        write(accesses, memory, AccessOccurrence::Always);
    }
    add_memory_reference(
        references,
        base,
        offset,
        width,
        if load {
            ReferenceKind::Read
        } else {
            ReferenceKind::Write
        },
    );
}

fn cp0_operation(operation: Cp0Operation, accesses: &mut Vec<LocationAccess<Location>>) {
    use Cp0Operation::*;
    match operation {
        ReadIndexedTlbEntry => {
            read(
                accesses,
                Location::Cp0(Cp0Register::Index),
                AccessOccurrence::Always,
            );
            read(
                accesses,
                Location::Implicit(ImplicitState::Tlb),
                AccessOccurrence::Always,
            );
            for register in [
                Cp0Register::EntryHi,
                Cp0Register::EntryLo0,
                Cp0Register::EntryLo1,
                Cp0Register::PageMask,
            ] {
                write(accesses, Location::Cp0(register), AccessOccurrence::Always);
            }
        }
        WriteIndexedTlbEntry | WriteRandomTlbEntry => {
            let selector = if operation == WriteIndexedTlbEntry {
                Cp0Register::Index
            } else {
                Cp0Register::Random
            };
            read(accesses, Location::Cp0(selector), AccessOccurrence::Always);
            for register in [
                Cp0Register::EntryHi,
                Cp0Register::EntryLo0,
                Cp0Register::EntryLo1,
                Cp0Register::PageMask,
            ] {
                read(accesses, Location::Cp0(register), AccessOccurrence::Always);
            }
            write(
                accesses,
                Location::Implicit(ImplicitState::Tlb),
                AccessOccurrence::Always,
            );
        }
        ProbeTlb => {
            read(
                accesses,
                Location::Cp0(Cp0Register::EntryHi),
                AccessOccurrence::Always,
            );
            read(
                accesses,
                Location::Implicit(ImplicitState::Tlb),
                AccessOccurrence::Always,
            );
            write(
                accesses,
                Location::Cp0(Cp0Register::Index),
                AccessOccurrence::Always,
            );
        }
        ReturnFromException => {
            for register in [
                Cp0Register::Status,
                Cp0Register::ExceptionProgramCounter,
                Cp0Register::ErrorExceptionProgramCounter,
            ] {
                read(accesses, Location::Cp0(register), AccessOccurrence::Always);
            }
            write(
                accesses,
                Location::Cp0(Cp0Register::Status),
                AccessOccurrence::Always,
            );
        }
    }
}

fn exception_state(accesses: &mut Vec<LocationAccess<Location>>, occurrence: AccessOccurrence) {
    read(accesses, Location::Cp0(Cp0Register::Status), occurrence);
    for register in [
        Cp0Register::Status,
        Cp0Register::Cause,
        Cp0Register::ExceptionProgramCounter,
    ] {
        write(accesses, Location::Cp0(register), occurrence);
    }
}

fn floating_arithmetic_state(accesses: &mut Vec<LocationAccess<Location>>, reads_rounding: bool) {
    if reads_rounding {
        read(
            accesses,
            Location::FpuState(FpuState::RoundingMode),
            AccessOccurrence::Always,
        );
    }
    write(
        accesses,
        Location::FpuState(FpuState::ExceptionState),
        AccessOccurrence::Conditional,
    );
}

fn coprocessor_condition(coprocessor: Coprocessor) -> Location {
    match coprocessor {
        Coprocessor::SystemControl => Location::Cp0(Cp0Register::Cause),
        Coprocessor::FloatingPoint => Location::FpuState(FpuState::Condition),
        Coprocessor::Coprocessor2 => Location::Coprocessor2Control(
            Coprocessor2ControlRegister::new(31).expect("31 fits CP2 control selector"),
        ),
    }
}

fn special(register: SpecialRegister) -> Location {
    Location::Implicit(match register {
        SpecialRegister::Hi => ImplicitState::Hi,
        SpecialRegister::Lo => ImplicitState::Lo,
    })
}

fn floating_width(format: FloatingFormat) -> u8 {
    match format {
        FloatingFormat::Single => 4,
        FloatingFormat::Double => 8,
    }
}

fn fixed_width(format: FixedFormat) -> u8 {
    match format {
        FixedFormat::Word => 4,
        FixedFormat::Longword => 8,
    }
}

fn numeric_width(format: NumericFormat) -> u8 {
    match format {
        NumericFormat::Floating(format) => floating_width(format),
        NumericFormat::Fixed(format) => fixed_width(format),
    }
}

fn fpr(status_fr: StatusFr, register: FloatingRegister, byte_len: u8) -> FloatingStorage {
    FloatingStorage {
        byte_offset: match status_fr {
            StatusFr::Zero => u16::from(register.index()) * 4,
            StatusFr::One => u16::from(register.index()) * 8,
        },
        byte_len,
    }
}

fn read_fpr(
    accesses: &mut Vec<LocationAccess<Location>>,
    status_fr: StatusFr,
    register: FloatingRegister,
    width: u8,
) {
    read(
        accesses,
        Location::Floating(fpr(status_fr, register, width)),
        AccessOccurrence::Always,
    );
}

fn write_fpr(
    accesses: &mut Vec<LocationAccess<Location>>,
    status_fr: StatusFr,
    register: FloatingRegister,
    width: u8,
) {
    write(
        accesses,
        Location::Floating(fpr(status_fr, register, width)),
        AccessOccurrence::Always,
    );
}

fn read_fprs<const N: usize>(
    accesses: &mut Vec<LocationAccess<Location>>,
    status_fr: StatusFr,
    registers: [(FloatingRegister, u8); N],
) {
    for (register, width) in registers {
        read_fpr(accesses, status_fr, register, width);
    }
}

trait GprInputs {
    fn visit(self, visitor: impl FnMut(GeneralRegister));
}

impl GprInputs for GeneralRegister {
    fn visit(self, mut visitor: impl FnMut(GeneralRegister)) {
        visitor(self);
    }
}

impl<const N: usize> GprInputs for [GeneralRegister; N] {
    fn visit(self, mut visitor: impl FnMut(GeneralRegister)) {
        for register in self {
            visitor(register);
        }
    }
}

fn read_gprs(registers: &mut Vec<LocationAccess<Location>>, inputs: impl GprInputs) {
    inputs.visit(|register| read_gpr(registers, register));
}

fn read_gpr(accesses: &mut Vec<LocationAccess<Location>>, register: GeneralRegister) {
    if register != GeneralRegister::ZERO {
        read(
            accesses,
            Location::GeneralRegister(register),
            AccessOccurrence::Always,
        );
    }
}

fn write_gpr(accesses: &mut Vec<LocationAccess<Location>>, register: GeneralRegister) {
    if register != GeneralRegister::ZERO {
        write(
            accesses,
            Location::GeneralRegister(register),
            AccessOccurrence::Always,
        );
    }
}

fn add_address_reference(
    references: &mut Vec<StaticReference<Reference>>,
    base: GeneralRegister,
    offset: i16,
) {
    if base == GeneralRegister::ZERO {
        add_reference(
            references,
            StaticReference {
                target: Reference::Address(offset as i64 as u64),
                kind: ReferenceKind::Address,
                occurrence: AccessOccurrence::Always,
            },
        );
    }
}

fn add_memory_reference(
    references: &mut Vec<StaticReference<Reference>>,
    base: GeneralRegister,
    offset: i16,
    byte_len: u8,
    kind: ReferenceKind,
) {
    if base == GeneralRegister::ZERO {
        add_reference(
            references,
            StaticReference {
                target: Reference::Memory {
                    address: offset as i64 as u64,
                    byte_len,
                },
                kind,
                occurrence: AccessOccurrence::Always,
            },
        );
    }
}

fn width_bytes(width: MemoryWidth) -> u8 {
    match width {
        MemoryWidth::Known(bytes) | MemoryWidth::Merge(bytes) => bytes,
    }
}

fn read(
    accesses: &mut Vec<LocationAccess<Location>>,
    location: Location,
    occurrence: AccessOccurrence,
) {
    access(accesses, location, AccessKind::Read, occurrence);
}

fn write(
    accesses: &mut Vec<LocationAccess<Location>>,
    location: Location,
    occurrence: AccessOccurrence,
) {
    access(accesses, location, AccessKind::Write, occurrence);
}

fn access(
    accesses: &mut Vec<LocationAccess<Location>>,
    location: Location,
    kind: AccessKind,
    occurrence: AccessOccurrence,
) {
    let candidate = LocationAccess {
        location,
        kind,
        occurrence,
    };
    if !accesses.contains(&candidate) {
        accesses.push(candidate);
    }
}

fn add_reference(
    references: &mut Vec<StaticReference<Reference>>,
    candidate: StaticReference<Reference>,
) {
    if !references.contains(&candidate) {
        references.push(candidate);
    }
}

pub(crate) fn storage_relation(left: &Location, right: &Location) -> StorageRelation {
    if left == right {
        return StorageRelation::Same;
    }
    match (left, right) {
        (Location::Floating(left), Location::Floating(right)) => range_relation(
            u64::from(left.byte_offset),
            left.byte_len,
            u64::from(right.byte_offset),
            right.byte_len,
        ),
        (Location::FpuControl(FpuControlRegister::ControlStatus), Location::FpuState(_))
        | (Location::FpuState(_), Location::FpuControl(FpuControlRegister::ControlStatus)) => {
            StorageRelation::Overlapping
        }
        (Location::Memory(left), Location::Memory(right)) => memory_relation(*left, *right),
        _ => StorageRelation::Disjoint,
    }
}

fn memory_relation(left: MemoryLocation, right: MemoryLocation) -> StorageRelation {
    if left.address != right.address {
        return StorageRelation::Unknown;
    }
    let left_len = width_bytes(left.width);
    let right_len = width_bytes(right.width);
    if left_len == right_len && left.width == right.width {
        StorageRelation::Same
    } else {
        StorageRelation::Overlapping
    }
}

fn range_relation(left: u64, left_len: u8, right: u64, right_len: u8) -> StorageRelation {
    let overlap = (0..u64::from(left_len)).any(|left_offset| {
        (0..u64::from(right_len)).any(|right_offset| left + left_offset == right + right_offset)
    });
    if overlap {
        StorageRelation::Overlapping
    } else {
        StorageRelation::Disjoint
    }
}
