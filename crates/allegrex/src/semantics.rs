//! Complete static meaning for the Sony Allegrex profile.

use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlBoundary, ControlTarget, DelaySlot,
    DelaySlotExecution, LocationAccess, ReferenceKind, StaticReference, StorageRelation,
};

use crate::{
    AccumulateOperation, Allegrex, BitFieldOperation, EncodeError, FloatingControlRegister,
    FloatingRegister, GeneralRegister, Instruction, MatrixRegister, MemoryOperation,
    SpecialRegister, SystemControlRegister, VectorRegister, VectorSize, VfpuConditionBit,
    VfpuControlRegister, VfpuInstruction, VfpuOperands, VfpuOperation,
};

/// Placed Allegrex instruction used by static analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnalysisPoint {
    /// 32-bit virtual instruction address.
    pub location: crate::CodeLocation,
}

impl AnalysisPoint {
    /// Constructs a placed Allegrex instruction.
    pub const fn new(location: crate::CodeLocation) -> Self {
        Self { location }
    }
}

/// Runtime-derived Allegrex control destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndirectTarget {
    /// Address held in one general-purpose register.
    Register(GeneralRegister),
    /// Address recovered from CP0 exception state by `eret`.
    ExceptionReturn,
    /// Processor-defined general exception vector.
    GeneralExceptionVector,
}

/// Synchronous Allegrex trap kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Boundary {
    /// `syscall` with its code field.
    SystemCall(u32),
    /// `break` with its code field.
    Breakpoint(u32),
}

/// One FPU control/status subfield.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FpuState {
    /// Rounding-mode field.
    RoundingMode,
    /// Arithmetic cause, flag, and enable state.
    ExceptionState,
    /// Condition bit consumed by BC1 and written by comparisons.
    Condition,
}

/// Stable processor state not represented by an explicit operand register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImplicitState {
    /// HI multiply/divide result register.
    Hi,
    /// LO multiply/divide result register.
    Lo,
    /// LLbit and linked-address reservation.
    LoadLinkedReservation,
    /// Cache state selected by a CACHE operation.
    Cache(u8),
    /// Ordering effect established by synchronization.
    MemoryOrdering,
}

/// Symbolic effective address of one Allegrex data access.
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
    /// Left/right merge into an existing register view.
    Merge(u8),
}

/// Symbolic Allegrex memory storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryLocation {
    /// Effective-address rule.
    pub address: MemoryAddress,
    /// Transfer width or merge rule.
    pub width: MemoryWidth,
}

/// Allegrex architectural storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Location {
    /// One 32-bit general-purpose register.
    GeneralRegister(GeneralRegister),
    /// HI or LO special register.
    SpecialRegister(SpecialRegister),
    /// One CP0 register selector.
    SystemControl(SystemControlRegister),
    /// One scalar FPU register.
    FloatingRegister(FloatingRegister),
    /// One FPU control register.
    FloatingControl(FloatingControlRegister),
    /// One FPU control/status subfield.
    FpuState(FpuState),
    /// One typed VFPU vector view.
    VfpuVector {
        /// Encoded vector view.
        register: VectorRegister,
        /// Lane count of the view.
        size: VectorSize,
    },
    /// One typed VFPU matrix view.
    VfpuMatrix {
        /// Encoded matrix view.
        register: MatrixRegister,
        /// Row and column count of the view.
        size: VectorSize,
    },
    /// One VFPU control register.
    VfpuControl(VfpuControlRegister),
    /// One bit within the VFPU condition control register.
    VfpuCondition(VfpuConditionBit),
    /// Stable implicit state.
    Implicit(ImplicitState),
    /// Symbolic memory storage.
    Memory(MemoryLocation),
}

/// Statically resolved Allegrex address reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reference {
    /// Memory range whose base is the constant-zero register.
    Memory {
        /// First referenced virtual byte address.
        address: u32,
        /// Referenced byte count or merge-register width.
        byte_len: u8,
    },
    /// Statically formed address without a claimed data transfer.
    Address(u32),
}

/// Failure to form sound Allegrex static facts.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SemanticError {
    /// The typed instruction is not encodable by the exact profile.
    #[error(transparent)]
    Encode(#[from] EncodeError),
}

/// Shared control-flow result specialized with Allegrex vocabulary.
pub type ControlFlow = typed_isa_core::ControlFlow<AnalysisPoint, IndirectTarget, Boundary>;

/// Shared instruction-semantics result specialized with Allegrex vocabulary.
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
) -> Result<InstructionSemantics, SemanticError> {
    crate::encode_word(instruction, at.location)?;
    let next = AnalysisPoint::new(at.location.wrapping_add(4));
    let after_slot = AnalysisPoint::new(at.location.wrapping_add(8));
    let mut accesses = Vec::new();
    let mut references = Vec::new();
    instruction_accesses(instruction, &mut accesses, &mut references);
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
    match instruction {
        Instruction::EqualityBranch { likely, target, .. }
        | Instruction::SignBranch {
            likely,
            link: false,
            target,
            ..
        }
        | Instruction::FloatingBranch { likely, target, .. } => {
            branch_flow(*target, *likely, next, after_slot)
        }
        Instruction::SignBranch {
            likely,
            link: true,
            target,
            ..
        } => ControlFlow {
            action: ControlAction::LinkedTransfer {
                target: ControlTarget::Direct(AnalysisPoint::new(*target)),
                return_site: after_slot,
            },
            fallthrough: Some(after_slot),
            delay_slot: Some(delay_slot(next, *likely)),
        },
        Instruction::Jump { link, target } => ControlFlow {
            action: if *link {
                ControlAction::LinkedTransfer {
                    target: ControlTarget::Direct(AnalysisPoint::new(*target)),
                    return_site: after_slot,
                }
            } else {
                ControlAction::Transfer {
                    target: ControlTarget::Direct(AnalysisPoint::new(*target)),
                }
            },
            fallthrough: None,
            delay_slot: Some(delay_slot(next, false)),
        },
        Instruction::JumpRegister { source, link } => ControlFlow {
            action: match link {
                Some(_) => ControlAction::LinkedTransfer {
                    target: ControlTarget::Indirect(IndirectTarget::Register(*source)),
                    return_site: after_slot,
                },
                None => ControlAction::Transfer {
                    target: ControlTarget::Indirect(IndirectTarget::Register(*source)),
                },
            },
            fallthrough: None,
            delay_slot: Some(delay_slot(next, false)),
        },
        Instruction::SystemCall { code } => trap_flow(Boundary::SystemCall(*code)),
        Instruction::Break { code } => trap_flow(Boundary::Breakpoint(*code)),
        Instruction::ExceptionReturn => ControlFlow {
            action: ControlAction::ExceptionReturn {
                target: IndirectTarget::ExceptionReturn,
            },
            fallthrough: None,
            delay_slot: None,
        },
        Instruction::Vfpu(vfpu)
            if matches!(
                vfpu.form().operation(),
                VfpuOperation::BranchFalse
                    | VfpuOperation::BranchFalseLikely
                    | VfpuOperation::BranchTrue
                    | VfpuOperation::BranchTrueLikely
            ) =>
        {
            let VfpuOperands::Branch { target, .. } = vfpu.operands() else {
                unreachable!()
            };
            let likely = matches!(
                vfpu.form().operation(),
                VfpuOperation::BranchFalseLikely | VfpuOperation::BranchTrueLikely
            );
            branch_flow(*target, likely, next, after_slot)
        }
        _ => ControlFlow {
            action: ControlAction::Continue,
            fallthrough: Some(next),
            delay_slot: None,
        },
    }
}

fn delay_slot(point: AnalysisPoint, likely: bool) -> DelaySlot<AnalysisPoint> {
    DelaySlot {
        point,
        execution: if likely {
            DelaySlotExecution::WhenTransferOccurs
        } else {
            DelaySlotExecution::Always
        },
    }
}

fn branch_flow(
    target: u32,
    likely: bool,
    next: AnalysisPoint,
    after_slot: AnalysisPoint,
) -> ControlFlow {
    ControlFlow {
        action: ControlAction::Transfer {
            target: ControlTarget::Direct(AnalysisPoint::new(target)),
        },
        fallthrough: Some(after_slot),
        delay_slot: Some(delay_slot(next, likely)),
    }
}

fn trap_flow(kind: Boundary) -> ControlFlow {
    ControlFlow {
        action: ControlAction::Boundary(ControlBoundary::Trap {
            kind,
            target: Some(ControlTarget::Indirect(
                IndirectTarget::GeneralExceptionVector,
            )),
            resume: None,
        }),
        fallthrough: None,
        delay_slot: None,
    }
}

fn instruction_accesses(
    instruction: &Instruction,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    use Instruction::*;
    match instruction {
        Nop => {}
        ThreeRegister {
            destination,
            left,
            right,
            ..
        } => {
            read_gprs(accesses, [*left, *right]);
            write_gpr(accesses, *destination, AccessOccurrence::Always);
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
        }
        | BitCount {
            destination,
            source,
            ..
        }
        | ByteTransform {
            destination,
            source,
            ..
        } => {
            read_gpr(accesses, *source);
            write_gpr(accesses, *destination, AccessOccurrence::Always);
        }
        LoadUpperImmediate { destination, .. } => {
            write_gpr(accesses, *destination, AccessOccurrence::Always)
        }
        ShiftVariable {
            destination,
            source,
            amount_register,
            ..
        } => {
            read_gprs(accesses, [*source, *amount_register]);
            write_gpr(accesses, *destination, AccessOccurrence::Always);
        }
        MultiplyDivide { left, right, .. } => {
            read_gprs(accesses, [*left, *right]);
            for register in [SpecialRegister::Hi, SpecialRegister::Lo] {
                write(
                    accesses,
                    Location::SpecialRegister(register),
                    AccessOccurrence::Always,
                );
            }
        }
        Accumulate {
            operation,
            left,
            right,
        } => {
            read_gprs(accesses, [*left, *right]);
            for register in [SpecialRegister::Hi, SpecialRegister::Lo] {
                if matches!(
                    operation,
                    AccumulateOperation::MultiplyAdd
                        | AccumulateOperation::MultiplyAddUnsigned
                        | AccumulateOperation::MultiplySubtract
                        | AccumulateOperation::MultiplySubtractUnsigned
                ) {
                    read(
                        accesses,
                        Location::SpecialRegister(register),
                        AccessOccurrence::Always,
                    );
                }
                write(
                    accesses,
                    Location::SpecialRegister(register),
                    AccessOccurrence::Always,
                );
            }
        }
        MoveFromSpecial {
            source,
            destination,
        } => {
            read(
                accesses,
                Location::SpecialRegister(*source),
                AccessOccurrence::Always,
            );
            write_gpr(accesses, *destination, AccessOccurrence::Always);
        }
        MoveToSpecial {
            destination,
            source,
        } => {
            read_gpr(accesses, *source);
            write(
                accesses,
                Location::SpecialRegister(*destination),
                AccessOccurrence::Always,
            );
        }
        ConditionalMove {
            destination,
            source,
            condition_register,
            ..
        } => {
            read_gprs(accesses, [*source, *condition_register]);
            write_gpr(accesses, *destination, AccessOccurrence::Conditional);
        }
        BitField {
            operation,
            destination,
            source,
            ..
        } => {
            read_gpr(accesses, *source);
            if *operation == BitFieldOperation::Insert {
                read_gpr(accesses, *destination);
            }
            write_gpr(accesses, *destination, AccessOccurrence::Always);
        }
        JumpRegister { source, link } => {
            read_gpr(accesses, *source);
            if let Some(link) = link {
                write_gpr(accesses, *link, AccessOccurrence::Always);
            }
        }
        Jump { link, target: _ } => {
            if *link {
                write_gpr(accesses, GeneralRegister::RA, AccessOccurrence::Always);
            }
        }
        EqualityBranch {
            left,
            right,
            target: _,
            ..
        } => {
            read_gprs(accesses, [*left, *right]);
        }
        SignBranch {
            source,
            link,
            target: _,
            ..
        } => {
            read_gpr(accesses, *source);
            if *link {
                write_gpr(accesses, GeneralRegister::RA, AccessOccurrence::Always);
            }
        }
        Memory {
            operation,
            register,
            base,
            offset,
        } => cpu_memory(*operation, *register, *base, *offset, accesses, references),
        Cache {
            operation,
            base,
            offset,
        } => {
            read_gpr(accesses, *base);
            write(
                accesses,
                Location::Implicit(ImplicitState::Cache(operation.code())),
                AccessOccurrence::Always,
            );
            add_address_reference(references, *base, *offset);
        }
        SystemCall { .. } | Break { .. } => exception_state(accesses),
        Synchronize => write(
            accesses,
            Location::Implicit(ImplicitState::MemoryOrdering),
            AccessOccurrence::Always,
        ),
        ExceptionReturn => {
            for selector in [12, 14] {
                read(
                    accesses,
                    Location::SystemControl(SystemControlRegister::new(selector).unwrap()),
                    AccessOccurrence::Always,
                );
            }
            write(
                accesses,
                Location::SystemControl(SystemControlRegister::new(12).unwrap()),
                AccessOccurrence::Always,
            );
        }
        SystemControlTransfer {
            to_control,
            general,
            control,
        } => {
            if *to_control {
                read_gpr(accesses, *general);
                write(
                    accesses,
                    Location::SystemControl(*control),
                    AccessOccurrence::Always,
                );
            } else {
                read(
                    accesses,
                    Location::SystemControl(*control),
                    AccessOccurrence::Always,
                );
                write_gpr(accesses, *general, AccessOccurrence::Always);
            }
        }
        FloatingTransfer {
            to_floating,
            general,
            floating,
        } => {
            if *to_floating {
                read_gpr(accesses, *general);
                write(
                    accesses,
                    Location::FloatingRegister(*floating),
                    AccessOccurrence::Always,
                );
            } else {
                read(
                    accesses,
                    Location::FloatingRegister(*floating),
                    AccessOccurrence::Always,
                );
                write_gpr(accesses, *general, AccessOccurrence::Always);
            }
        }
        FloatingControlTransfer {
            to_control,
            general,
            control,
        } => {
            if *to_control {
                read_gpr(accesses, *general);
                write(
                    accesses,
                    Location::FloatingControl(*control),
                    AccessOccurrence::Always,
                );
            } else {
                read(
                    accesses,
                    Location::FloatingControl(*control),
                    AccessOccurrence::Always,
                );
                write_gpr(accesses, *general, AccessOccurrence::Always);
            }
        }
        FloatingBranch { target: _, .. } => {
            read(
                accesses,
                Location::FpuState(FpuState::Condition),
                AccessOccurrence::Always,
            );
        }
        FloatingMemory {
            store,
            floating,
            base,
            offset,
        } => scalar_floating_memory(*store, *floating, *base, *offset, accesses, references),
        FloatingBinary {
            destination,
            left,
            right,
            ..
        } => {
            for register in [*left, *right] {
                read(
                    accesses,
                    Location::FloatingRegister(register),
                    AccessOccurrence::Always,
                );
            }
            write(
                accesses,
                Location::FloatingRegister(*destination),
                AccessOccurrence::Always,
            );
            floating_arithmetic_state(accesses, true);
        }
        FloatingUnary {
            operation,
            destination,
            source,
        } => {
            read(
                accesses,
                Location::FloatingRegister(*source),
                AccessOccurrence::Always,
            );
            write(
                accesses,
                Location::FloatingRegister(*destination),
                AccessOccurrence::Always,
            );
            if matches!(operation, crate::FloatingUnaryOperation::SquareRoot) {
                floating_arithmetic_state(accesses, true);
            }
        }
        ConvertWordToSingle {
            destination,
            source,
        } => {
            read(
                accesses,
                Location::FloatingRegister(*source),
                AccessOccurrence::Always,
            );
            write(
                accesses,
                Location::FloatingRegister(*destination),
                AccessOccurrence::Always,
            );
            floating_arithmetic_state(accesses, true);
        }
        ConvertSingleToWord {
            destination,
            source,
        }
        | FloatingRoundToWord {
            destination,
            source,
            ..
        } => {
            read(
                accesses,
                Location::FloatingRegister(*source),
                AccessOccurrence::Always,
            );
            write(
                accesses,
                Location::FloatingRegister(*destination),
                AccessOccurrence::Always,
            );
            floating_arithmetic_state(accesses, false);
        }
        FloatingCompare { left, right, .. } => {
            for register in [*left, *right] {
                read(
                    accesses,
                    Location::FloatingRegister(register),
                    AccessOccurrence::Always,
                );
            }
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
        Vfpu(vfpu) => vfpu_accesses(vfpu, accesses, references),
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
    let (load, width, linked, conditional) = cpu_memory_kind(operation);
    read_gpr(accesses, base);
    let memory = Location::Memory(MemoryLocation {
        address: MemoryAddress { base, offset },
        width,
    });
    if load {
        read(accesses, memory, AccessOccurrence::Always);
        if matches!(width, MemoryWidth::Merge(_)) {
            read_gpr(accesses, register);
        }
        write_gpr(accesses, register, AccessOccurrence::Always);
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
            write_gpr(accesses, register, AccessOccurrence::Always);
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
        if conditional {
            AccessOccurrence::Conditional
        } else {
            AccessOccurrence::Always
        },
    );
}

fn cpu_memory_kind(operation: MemoryOperation) -> (bool, MemoryWidth, bool, bool) {
    use MemoryOperation::*;
    match operation {
        LoadByte | LoadByteUnsigned => (true, MemoryWidth::Known(1), false, false),
        LoadHalfword | LoadHalfwordUnsigned => (true, MemoryWidth::Known(2), false, false),
        LoadWord => (true, MemoryWidth::Known(4), false, false),
        LoadWordLeft | LoadWordRight => (true, MemoryWidth::Merge(4), false, false),
        LoadLinked => (true, MemoryWidth::Known(4), true, false),
        StoreByte => (false, MemoryWidth::Known(1), false, false),
        StoreHalfword => (false, MemoryWidth::Known(2), false, false),
        StoreWord => (false, MemoryWidth::Known(4), false, false),
        StoreWordLeft | StoreWordRight => (false, MemoryWidth::Merge(4), false, false),
        StoreConditional => (false, MemoryWidth::Known(4), false, true),
    }
}

fn scalar_floating_memory(
    store: bool,
    floating: FloatingRegister,
    base: GeneralRegister,
    offset: i16,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    read_gpr(accesses, base);
    let memory = Location::Memory(MemoryLocation {
        address: MemoryAddress { base, offset },
        width: MemoryWidth::Known(4),
    });
    if store {
        read(
            accesses,
            Location::FloatingRegister(floating),
            AccessOccurrence::Always,
        );
        write(accesses, memory, AccessOccurrence::Always);
    } else {
        read(accesses, memory, AccessOccurrence::Always);
        write(
            accesses,
            Location::FloatingRegister(floating),
            AccessOccurrence::Always,
        );
    }
    add_memory_reference(
        references,
        base,
        offset,
        4,
        if store {
            ReferenceKind::Write
        } else {
            ReferenceKind::Read
        },
        AccessOccurrence::Always,
    );
}

fn vfpu_accesses(
    instruction: &VfpuInstruction,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    let operation = instruction.form().operation();
    let size = instruction.form().size().unwrap_or(VectorSize::Single);
    match instruction.operands() {
        VfpuOperands::None => {
            if matches!(
                operation,
                VfpuOperation::Synchronize | VfpuOperation::SynchronizeCpu
            ) {
                write(
                    accesses,
                    Location::Implicit(ImplicitState::MemoryOrdering),
                    AccessOccurrence::Always,
                );
            }
        }
        VfpuOperands::Branch {
            condition,
            target: _,
        } => {
            read(
                accesses,
                vfpu_condition(*condition),
                AccessOccurrence::Always,
            );
        }
        VfpuOperands::MemoryVector {
            vector,
            base,
            offset,
        } => {
            let width = if matches!(
                operation,
                VfpuOperation::StoreLeft | VfpuOperation::StoreRight
            ) {
                MemoryWidth::Merge(vector_byte_len(size))
            } else {
                MemoryWidth::Known(vector_byte_len(size))
            };
            read_gpr(accesses, *base);
            let memory = Location::Memory(MemoryLocation {
                address: MemoryAddress {
                    base: *base,
                    offset: offset.value(),
                },
                width,
            });
            let vector = vfpu_vector(*vector, size);
            let store = matches!(
                operation,
                VfpuOperation::Store
                    | VfpuOperation::StoreWriteBuffer
                    | VfpuOperation::StoreLeft
                    | VfpuOperation::StoreRight
            );
            if store {
                read(accesses, vector, AccessOccurrence::Always);
                write(accesses, memory, AccessOccurrence::Always);
            } else {
                read(accesses, memory, AccessOccurrence::Always);
                write(accesses, vector, AccessOccurrence::Always);
            }
            add_memory_reference(
                references,
                *base,
                offset.value(),
                vector_byte_len(size),
                if store {
                    ReferenceKind::Write
                } else {
                    ReferenceKind::Read
                },
                AccessOccurrence::Always,
            );
        }
        VfpuOperands::GeneralAndVector { general, vector } => {
            if operation == VfpuOperation::MoveFromVector {
                read(
                    accesses,
                    vfpu_vector(*vector, size),
                    AccessOccurrence::Always,
                );
                write_gpr(accesses, *general, AccessOccurrence::Always);
            } else {
                read_gpr(accesses, *general);
                write(
                    accesses,
                    vfpu_vector(*vector, size),
                    AccessOccurrence::Always,
                );
            }
        }
        VfpuOperands::GeneralAndControl { general, control } => {
            if operation == VfpuOperation::MoveFromControl {
                read(
                    accesses,
                    Location::VfpuControl(*control),
                    AccessOccurrence::Always,
                );
                write_gpr(accesses, *general, AccessOccurrence::Always);
            } else {
                read_gpr(accesses, *general);
                write(
                    accesses,
                    Location::VfpuControl(*control),
                    AccessOccurrence::Always,
                );
            }
        }
        VfpuOperands::ThreeVectors {
            destination,
            left,
            right,
        } => {
            read(accesses, vfpu_vector(*left, size), AccessOccurrence::Always);
            read(
                accesses,
                vfpu_vector(*right, size),
                AccessOccurrence::Always,
            );
            write(
                accesses,
                vfpu_vector(*destination, size),
                AccessOccurrence::Always,
            );
            prefix_state(accesses);
        }
        VfpuOperands::TwoVectors {
            destination,
            source,
        }
        | VfpuOperands::ImmediateAndTwoVectors {
            destination,
            source,
            ..
        }
        | VfpuOperands::Immediate8AndTwoVectors {
            destination,
            source,
            ..
        } => {
            read(
                accesses,
                vfpu_vector(*source, size),
                AccessOccurrence::Always,
            );
            write(
                accesses,
                vfpu_vector(*destination, size),
                AccessOccurrence::Always,
            );
            prefix_state(accesses);
        }
        VfpuOperands::ConditionalMove {
            destination,
            source,
            condition,
        } => {
            read(
                accesses,
                vfpu_vector(*source, size),
                AccessOccurrence::Always,
            );
            read(
                accesses,
                vfpu_condition(*condition),
                AccessOccurrence::Always,
            );
            write(
                accesses,
                vfpu_vector(*destination, size),
                AccessOccurrence::Conditional,
            );
            prefix_state(accesses);
        }
        VfpuOperands::Compare { left, right, .. } => {
            read(accesses, vfpu_vector(*left, size), AccessOccurrence::Always);
            read(
                accesses,
                vfpu_vector(*right, size),
                AccessOccurrence::Always,
            );
            write(
                accesses,
                Location::VfpuControl(VfpuControlRegister::Condition),
                AccessOccurrence::Always,
            );
            prefix_state(accesses);
        }
        VfpuOperands::Constant { destination, .. }
        | VfpuOperands::VectorImmediate16 { destination, .. }
        | VfpuOperands::DestinationVector { destination } => {
            write(
                accesses,
                vfpu_vector(*destination, size),
                AccessOccurrence::Always,
            );
            prefix_state(accesses);
            if matches!(
                operation,
                VfpuOperation::RandomFloat1
                    | VfpuOperation::RandomFloat2
                    | VfpuOperation::RandomInteger
            ) {
                random_state(accesses, true);
            }
        }
        VfpuOperands::ControlAndVector { control, vector } => {
            if operation == VfpuOperation::MoveFromVfpuControl {
                read(
                    accesses,
                    Location::VfpuControl(*control),
                    AccessOccurrence::Always,
                );
                write(
                    accesses,
                    vfpu_vector(*vector, VectorSize::Single),
                    AccessOccurrence::Always,
                );
            } else {
                read(
                    accesses,
                    vfpu_vector(*vector, VectorSize::Single),
                    AccessOccurrence::Always,
                );
                write(
                    accesses,
                    Location::VfpuControl(*control),
                    AccessOccurrence::Always,
                );
            }
        }
        VfpuOperands::SourcePrefix { .. } => write(
            accesses,
            Location::VfpuControl(if operation == VfpuOperation::SourcePrefix {
                VfpuControlRegister::SourcePrefix
            } else {
                VfpuControlRegister::TargetPrefix
            }),
            AccessOccurrence::Always,
        ),
        VfpuOperands::DestinationPrefix { .. } => write(
            accesses,
            Location::VfpuControl(VfpuControlRegister::DestinationPrefix),
            AccessOccurrence::Always,
        ),
        VfpuOperands::SourceVector { source } => {
            read(
                accesses,
                vfpu_vector(*source, size),
                AccessOccurrence::Always,
            );
            if operation == VfpuOperation::RandomSeed {
                random_state(accesses, false);
            }
        }
        VfpuOperands::ThreeMatrices {
            destination,
            left,
            right,
        } => {
            read(accesses, vfpu_matrix(*left, size), AccessOccurrence::Always);
            read(
                accesses,
                vfpu_matrix(*right, size),
                AccessOccurrence::Always,
            );
            write(
                accesses,
                vfpu_matrix(*destination, size),
                AccessOccurrence::Always,
            );
            prefix_state(accesses);
        }
        VfpuOperands::TwoMatrices {
            destination,
            source,
        } => {
            read(
                accesses,
                vfpu_matrix(*source, size),
                AccessOccurrence::Always,
            );
            write(
                accesses,
                vfpu_matrix(*destination, size),
                AccessOccurrence::Always,
            );
            prefix_state(accesses);
        }
        VfpuOperands::DestinationMatrix { destination } => {
            write(
                accesses,
                vfpu_matrix(*destination, size),
                AccessOccurrence::Always,
            );
            prefix_state(accesses);
        }
        VfpuOperands::MatrixScale {
            destination,
            source,
            scale,
        } => {
            read(
                accesses,
                vfpu_matrix(*source, size),
                AccessOccurrence::Always,
            );
            read(
                accesses,
                vfpu_vector(*scale, VectorSize::Single),
                AccessOccurrence::Always,
            );
            write(
                accesses,
                vfpu_matrix(*destination, size),
                AccessOccurrence::Always,
            );
            prefix_state(accesses);
        }
        VfpuOperands::MatrixVectorTransform {
            destination,
            matrix,
            vector,
        } => {
            read(
                accesses,
                vfpu_matrix(*matrix, size),
                AccessOccurrence::Always,
            );
            read(
                accesses,
                vfpu_vector(*vector, size),
                AccessOccurrence::Always,
            );
            write(
                accesses,
                vfpu_vector(*destination, size),
                AccessOccurrence::Always,
            );
            prefix_state(accesses);
        }
    }
}

fn vfpu_vector(register: VectorRegister, size: VectorSize) -> Location {
    Location::VfpuVector { register, size }
}

fn vfpu_matrix(register: MatrixRegister, size: VectorSize) -> Location {
    Location::VfpuMatrix { register, size }
}

fn vfpu_condition(condition: VfpuConditionBit) -> Location {
    Location::VfpuCondition(condition)
}

fn prefix_state(accesses: &mut Vec<LocationAccess<Location>>) {
    for control in [
        VfpuControlRegister::SourcePrefix,
        VfpuControlRegister::TargetPrefix,
        VfpuControlRegister::DestinationPrefix,
    ] {
        read(
            accesses,
            Location::VfpuControl(control),
            AccessOccurrence::Always,
        );
    }
}

fn random_state(accesses: &mut Vec<LocationAccess<Location>>, produces_value: bool) {
    for control in [
        VfpuControlRegister::RandomState0,
        VfpuControlRegister::RandomState1,
        VfpuControlRegister::RandomState2,
        VfpuControlRegister::RandomState3,
        VfpuControlRegister::RandomState4,
        VfpuControlRegister::RandomState5,
        VfpuControlRegister::RandomState6,
        VfpuControlRegister::RandomState7,
    ] {
        if produces_value {
            read(
                accesses,
                Location::VfpuControl(control),
                AccessOccurrence::Always,
            );
        }
        write(
            accesses,
            Location::VfpuControl(control),
            AccessOccurrence::Always,
        );
    }
}

fn vector_byte_len(size: VectorSize) -> u8 {
    match size {
        VectorSize::Single => 4,
        VectorSize::Pair => 8,
        VectorSize::Triple => 12,
        VectorSize::Quad => 16,
    }
}

fn exception_state(accesses: &mut Vec<LocationAccess<Location>>) {
    read(
        accesses,
        Location::SystemControl(SystemControlRegister::new(12).unwrap()),
        AccessOccurrence::Always,
    );
    for selector in [12, 13, 14] {
        write(
            accesses,
            Location::SystemControl(SystemControlRegister::new(selector).unwrap()),
            AccessOccurrence::Always,
        );
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

fn read_gprs<const N: usize>(
    accesses: &mut Vec<LocationAccess<Location>>,
    registers: [GeneralRegister; N],
) {
    for register in registers {
        read_gpr(accesses, register);
    }
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

fn write_gpr(
    accesses: &mut Vec<LocationAccess<Location>>,
    register: GeneralRegister,
    occurrence: AccessOccurrence,
) {
    if register != GeneralRegister::ZERO {
        write(accesses, Location::GeneralRegister(register), occurrence);
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
                target: Reference::Address(offset as i32 as u32),
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
    occurrence: AccessOccurrence,
) {
    if base == GeneralRegister::ZERO {
        add_reference(
            references,
            StaticReference {
                target: Reference::Memory {
                    address: offset as i32 as u32,
                    byte_len,
                },
                kind,
                occurrence,
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
    use StorageRelation::*;
    if left == right {
        return Same;
    }
    match (left, right) {
        (
            Location::FloatingControl(FloatingControlRegister::ControlStatus),
            Location::FpuState(_),
        )
        | (
            Location::FpuState(_),
            Location::FloatingControl(FloatingControlRegister::ControlStatus),
        ) => Overlapping,
        (Location::VfpuControl(VfpuControlRegister::Condition), Location::VfpuCondition(_))
        | (Location::VfpuCondition(_), Location::VfpuControl(VfpuControlRegister::Condition)) => {
            Overlapping
        }
        (Location::VfpuVector { .. }, Location::VfpuVector { .. })
        | (Location::VfpuVector { .. }, Location::VfpuMatrix { .. })
        | (Location::VfpuMatrix { .. }, Location::VfpuVector { .. })
        | (Location::VfpuMatrix { .. }, Location::VfpuMatrix { .. }) => Unknown,
        (Location::Memory(left), Location::Memory(right)) => {
            if left.address == right.address {
                Overlapping
            } else {
                Unknown
            }
        }
        _ => Disjoint,
    }
}

impl typed_isa_core::StaticSemantics for Allegrex {
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
        semantics(instruction, *at)
    }

    fn storage_relation(left: &Self::Location, right: &Self::Location) -> StorageRelation {
        storage_relation(left, right)
    }
}
