//! Complete static meaning for the Sony/Toshiba R5900 profile.

use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlBoundary, ControlTarget, DelaySlot,
    DelaySlotExecution, LocationAccess, ReferenceKind, StaticReference, StorageRelation,
};

use crate::{
    EncodeError, FloatingControlRegister, FloatingRegister, GeneralRegister, Instruction,
    InstructionForm, Operands, PerformanceCounter, SystemControlRegister, VectorControlRegister,
    VectorDestinationMask, VectorFloatingRegister, VectorIntegerRegister, VectorLane,
};

/// Placed R5900 instruction used by static analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnalysisPoint {
    /// 32-bit virtual instruction address.
    pub location: crate::CodeLocation,
}

impl AnalysisPoint {
    /// Constructs a placed R5900 instruction.
    pub const fn new(location: crate::CodeLocation) -> Self {
        Self { location }
    }
}

/// Runtime-derived R5900 control destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndirectTarget {
    /// Address held in one general-purpose register.
    Register(GeneralRegister),
    /// Address recovered from COP0 exception state by `eret`.
    ExceptionReturn,
    /// Processor-defined general exception vector.
    GeneralExceptionVector,
}

/// Synchronous R5900 trap kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Boundary {
    /// `syscall` with its embedded code.
    SystemCall(u32),
    /// `break` with its embedded code.
    Breakpoint(u32),
    /// Instruction-defined conditional integer trap.
    IntegerTrap,
    /// Signed integer arithmetic overflow.
    IntegerOverflow,
}

/// A byte range within one physical 128-bit EE general-purpose register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GeneralStorage {
    /// Physical general-purpose register.
    pub register: GeneralRegister,
    /// First referenced byte within the register.
    pub byte_offset: u8,
    /// Referenced byte count.
    pub byte_len: u8,
}

/// One of the two physical 128-bit multiply/divide registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProductRegister {
    /// HI register, containing independently addressable HI0 and HI1 halves.
    Hi,
    /// LO register, containing independently addressable LO0 and LO1 halves.
    Lo,
}

/// A byte range within one physical HI or LO register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProductStorage {
    /// Physical product register.
    pub register: ProductRegister,
    /// First referenced byte within the register.
    pub byte_offset: u8,
    /// Referenced byte count.
    pub byte_len: u8,
}

/// EE breakpoint register outside the ordinary COP0 selector set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DebugRegister {
    /// Breakpoint control register.
    BreakpointControl,
    /// Data address breakpoint register.
    DataAddress,
    /// Data address breakpoint mask register.
    DataAddressMask,
    /// Data value breakpoint register.
    DataValue,
    /// Data value breakpoint mask register.
    DataValueMask,
    /// Instruction address breakpoint register.
    InstructionAddress,
    /// Instruction address breakpoint mask register.
    InstructionAddressMask,
}

/// One FPU control/status subfield.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FpuState {
    /// Cause and sticky arithmetic flags in FCR31.
    ArithmeticFlags,
    /// Condition bit consumed by BC1 and written by comparisons.
    Condition,
}

/// VU0 state with a stable role that is not a numbered register operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VectorState {
    /// Selected lanes of the floating-point accumulator.
    Accumulator(VectorDestinationMask),
    /// Selected per-lane MAC flags.
    MacFlags(VectorDestinationMask),
    /// Aggregated current and sticky status flags.
    StatusFlags,
    /// Clipping-history flags.
    ClippingFlags,
    /// VU0 microprogram execution initiated by a macro instruction.
    MicroExecution,
}

/// Stable processor state not represented by an explicit operand register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImplicitState {
    /// Translation lookaside buffer contents.
    Tlb,
    /// Cache state selected by one cache or prefetch form.
    Cache(InstructionForm),
    /// Ordering effect established by `sync`.
    MemoryOrdering,
    /// COP0 branch condition signal.
    Coprocessor0Condition,
    /// COP2 branch condition signal.
    Coprocessor2Condition,
}

/// Symbolic effective address of one EE main-memory access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryAddress {
    /// Base general-purpose register.
    pub base: GeneralRegister,
    /// Signed byte displacement.
    pub offset: i16,
}

/// Width and merge rule of one main-memory access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryWidth {
    /// Fixed contiguous byte count.
    Known(u8),
    /// Left/right merge into an existing register view.
    Merge(u8),
}

/// Symbolic EE main-memory storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryLocation {
    /// Effective-address rule.
    pub address: MemoryAddress,
    /// Transfer width or merge rule.
    pub width: MemoryWidth,
}

/// Symbolic VU0 local-memory address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VectorMemoryAddress {
    /// VU integer register containing the quadword index.
    pub base: VectorIntegerRegister,
    /// Quadword displacement applied before the memory access.
    pub quadword_offset: i8,
}

/// Symbolic VU0 local-memory storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VectorMemoryLocation {
    /// Effective-address rule in VU quadword units.
    pub address: VectorMemoryAddress,
    /// Referenced 32-bit lanes within the selected quadword.
    pub lanes: VectorDestinationMask,
}

/// R5900 architectural storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Location {
    /// One physical byte range in a 128-bit GPR.
    General(GeneralStorage),
    /// One physical byte range in HI or LO.
    Product(ProductStorage),
    /// Shift-amount state used by funnel shifts.
    ShiftAmount,
    /// One implemented COP0 register.
    SystemControl(SystemControlRegister),
    /// One EE breakpoint register.
    Debug(DebugRegister),
    /// Performance counter control state.
    PerformanceControl,
    /// One EE performance counter.
    PerformanceCounter(PerformanceCounter),
    /// One 32-bit FPU register.
    Floating(FloatingRegister),
    /// The 32-bit FPU multiply-accumulate register.
    FloatingAccumulator,
    /// One implemented FPU control register.
    FloatingControl(FloatingControlRegister),
    /// One FPU control/status subfield.
    FpuState(FpuState),
    /// One VU0 floating register with an exact lane mask.
    VectorFloating {
        /// Physical vector register.
        register: VectorFloatingRegister,
        /// Referenced x, y, z, and w lanes.
        lanes: VectorDestinationMask,
    },
    /// One VU0 integer register.
    VectorInteger(VectorIntegerRegister),
    /// One CFC2/CTC2-visible VU0 control selector.
    VectorControl(VectorControlRegister),
    /// VU0 accumulator, flag, or micro-execution state.
    VectorState(VectorState),
    /// Stable implicit EE state.
    Implicit(ImplicitState),
    /// Symbolic EE main-memory storage.
    Memory(MemoryLocation),
    /// Symbolic VU0 local-memory storage.
    VectorMemory(VectorMemoryLocation),
}

/// Statically resolved R5900 address reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reference {
    /// Main-memory range whose base is the constant-zero GPR.
    Memory {
        /// First referenced virtual byte address.
        address: u32,
        /// Referenced byte count or merge-register width.
        byte_len: u8,
    },
    /// Statically formed main-memory address without a claimed transfer.
    Address(u32),
    /// Direct VU0 local-memory byte range based on constant VI0.
    VectorMemory {
        /// First byte in VU0 data memory.
        address: u32,
        /// Referenced byte count.
        byte_len: u8,
    },
    /// Direct VU0 MicroMem subroutine entry.
    VectorMicrocode(u32),
}

/// Failure to form sound R5900 static facts.
#[derive(Debug, thiserror::Error)]
pub enum SemanticError {
    /// The typed instruction is not encodable by the exact profile.
    #[error(transparent)]
    Encode(#[from] EncodeError),
}

/// Shared control-flow result specialized with R5900 vocabulary.
pub type ControlFlow = typed_isa_core::ControlFlow<AnalysisPoint, IndirectTarget, Boundary>;

/// Shared instruction-semantics result specialized with R5900 vocabulary.
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
    let form = instruction.form();
    match instruction.operands() {
        Operands::BranchTwo { target, .. }
        | Operands::BranchOne { target, .. }
        | Operands::Branch { target } => {
            if is_link_branch(form) {
                ControlFlow {
                    action: ControlAction::LinkedTransfer {
                        target: ControlTarget::Direct(AnalysisPoint::new(*target)),
                        return_site: after_slot,
                    },
                    fallthrough: Some(after_slot),
                    delay_slot: Some(delay_slot(next, is_likely_branch(form))),
                }
            } else {
                branch_flow(*target, is_likely_branch(form), next, after_slot)
            }
        }
        Operands::Jump { target } => ControlFlow {
            action: if form == InstructionForm::Jal {
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
        Operands::JumpRegister { source, link } => ControlFlow {
            action: if link.is_some() {
                ControlAction::LinkedTransfer {
                    target: ControlTarget::Indirect(IndirectTarget::Register(*source)),
                    return_site: after_slot,
                }
            } else {
                ControlAction::Transfer {
                    target: ControlTarget::Indirect(IndirectTarget::Register(*source)),
                }
            },
            fallthrough: None,
            delay_slot: Some(delay_slot(next, false)),
        },
        Operands::System { code } if form == InstructionForm::Syscall => {
            trap_flow(Boundary::SystemCall(code.value()), None)
        }
        Operands::System { code } if form == InstructionForm::Break => {
            trap_flow(Boundary::Breakpoint(code.value()), None)
        }
        Operands::TrapGeneral { .. } | Operands::TrapImmediate { .. } => {
            trap_flow(Boundary::IntegerTrap, Some(next))
        }
        _ if is_overflow_form(form) => trap_flow(Boundary::IntegerOverflow, Some(next)),
        _ if form == InstructionForm::Eret => ControlFlow {
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

fn trap_flow(kind: Boundary, fallthrough: Option<AnalysisPoint>) -> ControlFlow {
    ControlFlow {
        action: ControlAction::Boundary(ControlBoundary::Trap {
            kind,
            target: Some(ControlTarget::Indirect(
                IndirectTarget::GeneralExceptionVector,
            )),
            resume: None,
        }),
        fallthrough,
        delay_slot: None,
    }
}

fn instruction_accesses(
    instruction: &Instruction,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    let form = instruction.form();
    let destination_occurrence = if is_overflow_form(form) || is_conditional_move(form) {
        AccessOccurrence::Conditional
    } else {
        AccessOccurrence::Always
    };
    match instruction.operands() {
        Operands::None => none_accesses(form, accesses),
        Operands::ThreeGeneral {
            destination,
            left,
            right,
        } => {
            let width = if is_scalar_three_general(form) { 8 } else { 16 };
            read_gpr(accesses, *left, width);
            read_gpr(accesses, *right, width);
            write_gpr(accesses, *destination, width, destination_occurrence);
            packed_product_accesses(form, accesses);
            if form == InstructionForm::Qfsrv {
                read(accesses, Location::ShiftAmount, AccessOccurrence::Always);
            }
        }
        Operands::TwoGeneral {
            destination,
            source,
        } => {
            read_gpr(accesses, *source, 16);
            write_gpr(accesses, *destination, 16, AccessOccurrence::Always);
        }
        Operands::GeneralPair { left, right } => {
            let packed = matches!(
                form,
                InstructionForm::Pdivbw | InstructionForm::Pdivuw | InstructionForm::Pdivw
            );
            let width = if packed { 16 } else { 8 };
            read_gpr(accesses, *left, width);
            read_gpr(accesses, *right, width);
            if packed {
                write_product_full(accesses, AccessOccurrence::Always);
            } else {
                let upper = matches!(form, InstructionForm::Div1 | InstructionForm::Divu1);
                write_product_bank(accesses, upper, AccessOccurrence::Always);
            }
        }
        Operands::GeneralDestination { destination } => {
            let full = matches!(
                form,
                InstructionForm::Pmfhi
                    | InstructionForm::PmfhlLh
                    | InstructionForm::PmfhlLw
                    | InstructionForm::PmfhlSh
                    | InstructionForm::PmfhlSlw
                    | InstructionForm::PmfhlUw
                    | InstructionForm::Pmflo
            );
            general_destination_source(form, accesses);
            write_gpr(
                accesses,
                *destination,
                if full { 16 } else { 8 },
                AccessOccurrence::Always,
            );
        }
        Operands::GeneralSource { source } => {
            let full = matches!(
                form,
                InstructionForm::Pmthi | InstructionForm::PmthlLw | InstructionForm::Pmtlo
            );
            read_gpr(accesses, *source, if full { 16 } else { 8 });
            general_source_destination(form, accesses);
        }
        Operands::SignedImmediate {
            destination,
            source,
            ..
        }
        | Operands::UnsignedImmediate {
            destination,
            source,
            ..
        } => {
            read_gpr(accesses, *source, 8);
            write_gpr(accesses, *destination, 8, destination_occurrence);
        }
        Operands::LoadUpper { destination, .. } => {
            write_gpr(accesses, *destination, 8, AccessOccurrence::Always)
        }
        Operands::ShiftImmediate {
            destination,
            source,
            ..
        } => {
            let width = if matches!(
                form,
                InstructionForm::Psllh
                    | InstructionForm::Psllw
                    | InstructionForm::Psrah
                    | InstructionForm::Psraw
                    | InstructionForm::Psrlh
                    | InstructionForm::Psrlw
            ) {
                16
            } else {
                8
            };
            read_gpr(accesses, *source, width);
            write_gpr(accesses, *destination, width, AccessOccurrence::Always);
        }
        Operands::ShiftVariable {
            destination,
            source,
            amount,
        } => {
            let width = if matches!(
                form,
                InstructionForm::Psllvw | InstructionForm::Psravw | InstructionForm::Psrlvw
            ) {
                16
            } else {
                8
            };
            read_gpr(accesses, *source, width);
            read_gpr(accesses, *amount, width);
            write_gpr(accesses, *destination, width, AccessOccurrence::Always);
        }
        Operands::Multiply {
            destination,
            left,
            right,
        } => {
            read_gpr(accesses, *left, 8);
            read_gpr(accesses, *right, 8);
            let upper = matches!(
                form,
                InstructionForm::Madd1
                    | InstructionForm::Maddu1
                    | InstructionForm::Mult1
                    | InstructionForm::Multu1
            );
            if matches!(
                form,
                InstructionForm::Madd
                    | InstructionForm::Madd1
                    | InstructionForm::Maddu
                    | InstructionForm::Maddu1
            ) {
                read_product_bank(accesses, upper);
            }
            write_product_bank(accesses, upper, AccessOccurrence::Always);
            if let Some(destination) = destination {
                write_gpr(accesses, *destination, 8, AccessOccurrence::Always);
            }
        }
        Operands::Jump { target: _ } => {
            if form == InstructionForm::Jal {
                write_gpr(accesses, GeneralRegister::RA, 8, AccessOccurrence::Always);
            }
        }
        Operands::JumpRegister { source, link } => {
            read_gpr(accesses, *source, 8);
            if let Some(link) = link {
                write_gpr(accesses, *link, 8, AccessOccurrence::Always);
            }
        }
        Operands::BranchTwo {
            left,
            right,
            target: _,
        } => {
            read_gpr(accesses, *left, 8);
            read_gpr(accesses, *right, 8);
            branch_accesses(form, accesses);
        }
        Operands::BranchOne { source, target: _ } => {
            read_gpr(accesses, *source, 8);
            branch_accesses(form, accesses);
        }
        Operands::Branch { .. } => branch_accesses(form, accesses),
        Operands::MemoryGeneral {
            register,
            base,
            offset,
        } => main_memory_accesses(form, *register, *base, *offset, accesses, references),
        Operands::Prefetch { base, offset, .. } => {
            read_gpr(accesses, *base, 8);
            write(
                accesses,
                Location::Implicit(ImplicitState::Cache(form)),
                AccessOccurrence::Always,
            );
            add_address_reference(references, *base, *offset);
        }
        Operands::Sync { .. } => write(
            accesses,
            Location::Implicit(ImplicitState::MemoryOrdering),
            AccessOccurrence::Always,
        ),
        Operands::System { .. } => exception_state(accesses, AccessOccurrence::Always),
        Operands::TrapGeneral { left, right, .. } => {
            read_gpr(accesses, *left, 8);
            read_gpr(accesses, *right, 8);
            exception_state(accesses, AccessOccurrence::Conditional);
        }
        Operands::TrapImmediate { source, .. } => {
            read_gpr(accesses, *source, 8);
            exception_state(accesses, AccessOccurrence::Conditional);
        }
        Operands::SourceImmediate { source, .. } => {
            read_gpr(accesses, *source, 8);
            write(accesses, Location::ShiftAmount, AccessOccurrence::Always);
        }
        Operands::Cache { base, offset } => {
            read_gpr(accesses, *base, 8);
            write(
                accesses,
                Location::Implicit(ImplicitState::Cache(form)),
                AccessOccurrence::Always,
            );
            add_address_reference(references, *base, *offset);
        }
        Operands::SystemControl { general, control } => {
            if form == InstructionForm::Mfc0 {
                read(
                    accesses,
                    Location::SystemControl(*control),
                    AccessOccurrence::Always,
                );
                write_gpr(accesses, *general, 8, AccessOccurrence::Always);
            } else {
                read_gpr(accesses, *general, 8);
                write(
                    accesses,
                    Location::SystemControl(*control),
                    AccessOccurrence::Always,
                );
            }
        }
        Operands::Performance { general, counter } => {
            let location = if matches!(form, InstructionForm::Mfps | InstructionForm::Mtps) {
                Location::PerformanceControl
            } else {
                Location::PerformanceCounter(*counter)
            };
            if matches!(form, InstructionForm::Mfps | InstructionForm::Mfpc) {
                read(accesses, location, AccessOccurrence::Always);
                write_gpr(accesses, *general, 8, AccessOccurrence::Always);
            } else {
                read_gpr(accesses, *general, 8);
                write(accesses, location, AccessOccurrence::Always);
            }
        }
        Operands::FloatingTransfer { general, floating } => {
            if form == InstructionForm::Mfc1 {
                read(
                    accesses,
                    Location::Floating(*floating),
                    AccessOccurrence::Always,
                );
                write_gpr(accesses, *general, 8, AccessOccurrence::Always);
            } else {
                read_gpr(accesses, *general, 8);
                write(
                    accesses,
                    Location::Floating(*floating),
                    AccessOccurrence::Always,
                );
            }
        }
        Operands::FloatingControl { general, control } => {
            if form == InstructionForm::Cfc1 {
                read(
                    accesses,
                    Location::FloatingControl(*control),
                    AccessOccurrence::Always,
                );
                write_gpr(accesses, *general, 8, AccessOccurrence::Always);
            } else {
                read_gpr(accesses, *general, 8);
                write(
                    accesses,
                    Location::FloatingControl(*control),
                    AccessOccurrence::Always,
                );
            }
        }
        Operands::FloatingThree {
            destination,
            left,
            right,
        } => {
            read_floating(accesses, *left);
            read_floating(accesses, *right);
            if matches!(form, InstructionForm::MaddS | InstructionForm::MsubS) {
                read(
                    accesses,
                    Location::FloatingAccumulator,
                    AccessOccurrence::Always,
                );
            }
            write_floating(accesses, *destination);
            floating_flags(accesses);
        }
        Operands::FloatingTwo {
            destination,
            source,
        } => {
            read_floating(accesses, *source);
            write_floating(accesses, *destination);
            if matches!(
                form,
                InstructionForm::AbsS | InstructionForm::NegS | InstructionForm::SqrtS
            ) {
                floating_flags(accesses);
            }
        }
        Operands::FloatingAccumulatorPair { left, right } => {
            read_floating(accesses, *left);
            read_floating(accesses, *right);
            if matches!(form, InstructionForm::MaddaS | InstructionForm::MsubaS) {
                read(
                    accesses,
                    Location::FloatingAccumulator,
                    AccessOccurrence::Always,
                );
            }
            write(
                accesses,
                Location::FloatingAccumulator,
                AccessOccurrence::Always,
            );
            floating_flags(accesses);
        }
        Operands::FloatingCompare { left, right } => {
            read_floating(accesses, *left);
            read_floating(accesses, *right);
            write(
                accesses,
                Location::FpuState(FpuState::Condition),
                AccessOccurrence::Always,
            );
        }
        Operands::FloatingMemory {
            floating,
            base,
            offset,
        } => floating_memory_accesses(form, *floating, *base, *offset, accesses, references),
        Operands::VectorControlTransfer {
            general, control, ..
        } => {
            if form == InstructionForm::Cfc2 {
                read_vector_control(accesses, *control);
                write_gpr(accesses, *general, 8, AccessOccurrence::Always);
            } else {
                read_gpr(accesses, *general, 8);
                write_vector_control(accesses, *control);
            }
        }
        Operands::VectorFloatingTransfer {
            general, vector, ..
        } => {
            if form == InstructionForm::Qmfc2 {
                read_vector(accesses, *vector, VectorDestinationMask::XYZW);
                write_gpr(accesses, *general, 16, AccessOccurrence::Always);
            } else {
                read_gpr(accesses, *general, 16);
                write_vector(accesses, *vector, VectorDestinationMask::XYZW);
            }
        }
        Operands::VectorMemory {
            vector,
            base,
            offset,
        } => vector_main_memory_accesses(form, *vector, *base, *offset, accesses, references),
        Operands::VectorThree {
            destination_mask,
            destination,
            left,
            right,
        } => vector_three_accesses(
            form,
            *destination_mask,
            *destination,
            *left,
            *right,
            accesses,
        ),
        Operands::VectorBroadcast {
            destination_mask,
            destination,
            left,
            right,
            broadcast,
        } => {
            read_vector(accesses, *left, *destination_mask);
            read_vector(accesses, *right, lane_mask(*broadcast));
            vector_accumulator_input(form, *destination_mask, accesses);
            write_vector(accesses, *destination, *destination_mask);
            vector_flags(form, *destination_mask, accesses);
        }
        Operands::VectorTwo {
            destination_mask,
            destination,
            source,
        } => {
            read_vector(accesses, *source, *destination_mask);
            vector_scalar_input(form, accesses);
            vector_accumulator_input(form, *destination_mask, accesses);
            write_vector(accesses, *destination, *destination_mask);
            vector_flags(form, *destination_mask, accesses);
        }
        Operands::VectorAccumulatorPair {
            destination_mask,
            left,
            right,
        } => {
            read_vector(accesses, *left, *destination_mask);
            read_vector(accesses, *right, *destination_mask);
            vector_accumulator_update(form, *destination_mask, accesses);
            vector_flags(form, *destination_mask, accesses);
        }
        Operands::VectorAccumulatorSource {
            destination_mask,
            source,
        } => {
            read_vector(accesses, *source, *destination_mask);
            vector_scalar_input(form, accesses);
            vector_accumulator_update(form, *destination_mask, accesses);
            vector_flags(form, *destination_mask, accesses);
        }
        Operands::VectorAccumulatorBroadcast {
            destination_mask,
            left,
            right,
            broadcast,
        } => {
            read_vector(accesses, *left, *destination_mask);
            read_vector(accesses, *right, lane_mask(*broadcast));
            vector_accumulator_update(form, *destination_mask, accesses);
            vector_flags(form, *destination_mask, accesses);
        }
        Operands::VectorIntegerThree {
            destination,
            left,
            right,
        } => {
            read_vector_integer(accesses, *left);
            read_vector_integer(accesses, *right);
            write_vector_integer(accesses, *destination);
        }
        Operands::VectorIntegerImmediate {
            destination,
            source,
            ..
        } => {
            read_vector_integer(accesses, *source);
            write_vector_integer(accesses, *destination);
        }
        Operands::VectorIntegerLoad {
            lane,
            destination,
            base,
        } => vector_integer_memory(
            false,
            VectorDestinationMask::new(lane.destination_bits()).unwrap(),
            *destination,
            *base,
            accesses,
            references,
        ),
        Operands::VectorIntegerStore {
            destination_mask,
            source,
            base,
        } => vector_integer_memory(
            true,
            *destination_mask,
            *source,
            *base,
            accesses,
            references,
        ),
        Operands::VectorMemoryUpdate {
            destination_mask,
            vector,
            base,
        } => vector_local_memory_update(
            form,
            *destination_mask,
            *vector,
            *base,
            accesses,
            references,
        ),
        Operands::VectorFieldPair {
            left,
            left_lane,
            right,
            right_lane,
        } => {
            read_vector(accesses, *left, lane_mask(*left_lane));
            read_vector(accesses, *right, lane_mask(*right_lane));
            write_vector_control(accesses, VectorControlRegister::QUOTIENT);
            write(
                accesses,
                Location::VectorState(VectorState::StatusFlags),
                AccessOccurrence::Always,
            );
        }
        Operands::VectorFieldSource { source, lane } => {
            read_vector(accesses, *source, lane_mask(*lane));
            if form == InstructionForm::Vsqrt {
                write_vector_control(accesses, VectorControlRegister::QUOTIENT);
                write(
                    accesses,
                    Location::VectorState(VectorState::StatusFlags),
                    AccessOccurrence::Always,
                );
            } else {
                random_source_accesses(form, accesses);
            }
        }
        Operands::VectorIntegerToFloating {
            destination_mask,
            destination,
            source,
        } => {
            read_vector_integer(accesses, *source);
            write_vector(accesses, *destination, *destination_mask);
        }
        Operands::VectorFloatingToInteger {
            destination,
            source,
            lane,
        } => {
            read_vector(accesses, *source, lane_mask(*lane));
            write_vector_integer(accesses, *destination);
        }
        Operands::VectorRandomDestination {
            destination_mask,
            destination,
        } => {
            read_vector_control(accesses, VectorControlRegister::RANDOM);
            if form == InstructionForm::Vrnext {
                write_vector_control(accesses, VectorControlRegister::RANDOM);
            }
            write_vector(accesses, *destination, *destination_mask);
        }
        Operands::VectorPairXyz { left, right } => {
            let xyz = VectorDestinationMask::XYZ;
            read_vector(accesses, *left, xyz);
            if form == InstructionForm::Vclip {
                read_vector(accesses, *right, lane_mask(VectorLane::W));
                write(
                    accesses,
                    Location::VectorState(VectorState::ClippingFlags),
                    AccessOccurrence::Always,
                );
            } else {
                read_vector(accesses, *right, xyz);
                write(
                    accesses,
                    Location::VectorState(VectorState::Accumulator(xyz)),
                    AccessOccurrence::Always,
                );
                vector_flags(form, xyz, accesses);
            }
        }
        Operands::VectorThreeXyz {
            destination,
            left,
            right,
        } => {
            let xyz = VectorDestinationMask::XYZ;
            read_vector(accesses, *left, xyz);
            read_vector(accesses, *right, xyz);
            read(
                accesses,
                Location::VectorState(VectorState::Accumulator(xyz)),
                AccessOccurrence::Always,
            );
            write_vector(accesses, *destination, xyz);
            vector_flags(form, xyz, accesses);
        }
        Operands::VectorMicroCall { target } => {
            write(
                accesses,
                Location::VectorState(VectorState::MicroExecution),
                AccessOccurrence::Always,
            );
            add_reference(
                references,
                StaticReference {
                    target: Reference::VectorMicrocode(target.byte_address()),
                    kind: ReferenceKind::Address,
                    occurrence: AccessOccurrence::Always,
                },
            );
        }
    }

    if is_overflow_form(form) {
        exception_state(accesses, AccessOccurrence::Conditional);
    }
}

fn none_accesses(form: InstructionForm, accesses: &mut Vec<LocationAccess<Location>>) {
    match form {
        InstructionForm::Di | InstructionForm::Ei => {
            read(
                accesses,
                Location::SystemControl(SystemControlRegister::Status),
                AccessOccurrence::Always,
            );
            write(
                accesses,
                Location::SystemControl(SystemControlRegister::Status),
                AccessOccurrence::Always,
            );
        }
        InstructionForm::Eret => {
            read(
                accesses,
                Location::SystemControl(SystemControlRegister::Status),
                AccessOccurrence::Always,
            );
            read(
                accesses,
                Location::SystemControl(SystemControlRegister::ExceptionProgramCounter),
                AccessOccurrence::Always,
            );
            read(
                accesses,
                Location::SystemControl(SystemControlRegister::ErrorExceptionProgramCounter),
                AccessOccurrence::Conditional,
            );
            write(
                accesses,
                Location::SystemControl(SystemControlRegister::Status),
                AccessOccurrence::Always,
            );
        }
        InstructionForm::Tlbp => {
            read(
                accesses,
                Location::SystemControl(SystemControlRegister::EntryHigh),
                AccessOccurrence::Always,
            );
            read(
                accesses,
                Location::Implicit(ImplicitState::Tlb),
                AccessOccurrence::Always,
            );
            write(
                accesses,
                Location::SystemControl(SystemControlRegister::Index),
                AccessOccurrence::Always,
            );
        }
        InstructionForm::Tlbr => {
            read(
                accesses,
                Location::SystemControl(SystemControlRegister::Index),
                AccessOccurrence::Always,
            );
            read(
                accesses,
                Location::Implicit(ImplicitState::Tlb),
                AccessOccurrence::Always,
            );
            for register in tlb_entry_registers() {
                write(
                    accesses,
                    Location::SystemControl(register),
                    AccessOccurrence::Always,
                );
            }
        }
        InstructionForm::Tlbwi | InstructionForm::Tlbwr => {
            read(
                accesses,
                Location::SystemControl(if form == InstructionForm::Tlbwi {
                    SystemControlRegister::Index
                } else {
                    SystemControlRegister::Random
                }),
                AccessOccurrence::Always,
            );
            for register in tlb_entry_registers() {
                read(
                    accesses,
                    Location::SystemControl(register),
                    AccessOccurrence::Always,
                );
            }
            write(
                accesses,
                Location::Implicit(ImplicitState::Tlb),
                AccessOccurrence::Always,
            );
        }
        InstructionForm::Vcallmsr => {
            read_vector_control(accesses, VectorControlRegister::CALL_ADDRESS_ZERO);
            write(
                accesses,
                Location::VectorState(VectorState::MicroExecution),
                AccessOccurrence::Always,
            );
        }
        InstructionForm::Vwaitq => read_vector_control(accesses, VectorControlRegister::QUOTIENT),
        _ => {}
    }
}

fn branch_accesses(form: InstructionForm, accesses: &mut Vec<LocationAccess<Location>>) {
    if is_link_branch(form) {
        write_gpr(accesses, GeneralRegister::RA, 8, AccessOccurrence::Always);
    }
    match form {
        InstructionForm::Bc0f
        | InstructionForm::Bc0fl
        | InstructionForm::Bc0t
        | InstructionForm::Bc0tl => read(
            accesses,
            Location::Implicit(ImplicitState::Coprocessor0Condition),
            AccessOccurrence::Always,
        ),
        InstructionForm::Bc1f
        | InstructionForm::Bc1fl
        | InstructionForm::Bc1t
        | InstructionForm::Bc1tl => read(
            accesses,
            Location::FpuState(FpuState::Condition),
            AccessOccurrence::Always,
        ),
        InstructionForm::Bc2f
        | InstructionForm::Bc2fl
        | InstructionForm::Bc2t
        | InstructionForm::Bc2tl => read(
            accesses,
            Location::Implicit(ImplicitState::Coprocessor2Condition),
            AccessOccurrence::Always,
        ),
        _ => {}
    }
}

fn main_memory_accesses(
    form: InstructionForm,
    register: GeneralRegister,
    base: GeneralRegister,
    offset: i16,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    let (load, width, register_width) = match form {
        InstructionForm::Lb | InstructionForm::Lbu => (true, MemoryWidth::Known(1), 8),
        InstructionForm::Lh | InstructionForm::Lhu => (true, MemoryWidth::Known(2), 8),
        InstructionForm::Lw | InstructionForm::Lwu => (true, MemoryWidth::Known(4), 8),
        InstructionForm::Lwl | InstructionForm::Lwr => (true, MemoryWidth::Merge(4), 8),
        InstructionForm::Ld => (true, MemoryWidth::Known(8), 8),
        InstructionForm::Ldl | InstructionForm::Ldr => (true, MemoryWidth::Merge(8), 8),
        InstructionForm::Lq => (true, MemoryWidth::Known(16), 16),
        InstructionForm::Sb => (false, MemoryWidth::Known(1), 8),
        InstructionForm::Sh => (false, MemoryWidth::Known(2), 8),
        InstructionForm::Sw => (false, MemoryWidth::Known(4), 8),
        InstructionForm::Swl | InstructionForm::Swr => (false, MemoryWidth::Merge(4), 8),
        InstructionForm::Sd => (false, MemoryWidth::Known(8), 8),
        InstructionForm::Sdl | InstructionForm::Sdr => (false, MemoryWidth::Merge(8), 8),
        InstructionForm::Sq => (false, MemoryWidth::Known(16), 16),
        _ => unreachable!(),
    };
    read_gpr(accesses, base, 8);
    let memory = Location::Memory(MemoryLocation {
        address: MemoryAddress { base, offset },
        width,
    });
    if load {
        read(accesses, memory, AccessOccurrence::Always);
        if matches!(width, MemoryWidth::Merge(_)) {
            read_gpr(accesses, register, register_width);
        }
        write_gpr(accesses, register, register_width, AccessOccurrence::Always);
    } else {
        read_gpr(accesses, register, register_width);
        write(accesses, memory, AccessOccurrence::Always);
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

fn floating_memory_accesses(
    form: InstructionForm,
    floating: FloatingRegister,
    base: GeneralRegister,
    offset: i16,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    let load = form == InstructionForm::Lwc1;
    read_gpr(accesses, base, 8);
    let memory = Location::Memory(MemoryLocation {
        address: MemoryAddress { base, offset },
        width: MemoryWidth::Known(4),
    });
    if load {
        read(accesses, memory, AccessOccurrence::Always);
        write_floating(accesses, floating);
    } else {
        read_floating(accesses, floating);
        write(accesses, memory, AccessOccurrence::Always);
    }
    add_memory_reference(
        references,
        base,
        offset,
        4,
        if load {
            ReferenceKind::Read
        } else {
            ReferenceKind::Write
        },
    );
}

fn vector_main_memory_accesses(
    form: InstructionForm,
    vector: VectorFloatingRegister,
    base: GeneralRegister,
    offset: i16,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    let load = form == InstructionForm::Lqc2;
    read_gpr(accesses, base, 8);
    let memory = Location::Memory(MemoryLocation {
        address: MemoryAddress { base, offset },
        width: MemoryWidth::Known(16),
    });
    if load {
        read(accesses, memory, AccessOccurrence::Always);
        write_vector(accesses, vector, VectorDestinationMask::XYZW);
    } else {
        read_vector(accesses, vector, VectorDestinationMask::XYZW);
        write(accesses, memory, AccessOccurrence::Always);
    }
    add_memory_reference(
        references,
        base,
        offset,
        16,
        if load {
            ReferenceKind::Read
        } else {
            ReferenceKind::Write
        },
    );
}

fn vector_integer_memory(
    store: bool,
    lanes: VectorDestinationMask,
    register: VectorIntegerRegister,
    base: VectorIntegerRegister,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    read_vector_integer(accesses, base);
    let memory = Location::VectorMemory(VectorMemoryLocation {
        address: VectorMemoryAddress {
            base,
            quadword_offset: 0,
        },
        lanes,
    });
    if store {
        read_vector_integer(accesses, register);
        write(accesses, memory, AccessOccurrence::Always);
    } else {
        read(accesses, memory, AccessOccurrence::Always);
        write_vector_integer(accesses, register);
    }
    if base.index() == 0 {
        add_vector_memory_references(references, 0, lanes, store);
    }
}

fn vector_local_memory_update(
    form: InstructionForm,
    lanes: VectorDestinationMask,
    vector: VectorFloatingRegister,
    base: VectorIntegerRegister,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    let load = matches!(form, InstructionForm::Vlqd | InstructionForm::Vlqi);
    let decrement = matches!(form, InstructionForm::Vlqd | InstructionForm::Vsqd);
    read_vector_integer(accesses, base);
    let memory = Location::VectorMemory(VectorMemoryLocation {
        address: VectorMemoryAddress {
            base,
            quadword_offset: if decrement { -1 } else { 0 },
        },
        lanes,
    });
    if load {
        read(accesses, memory, AccessOccurrence::Always);
        write_vector(accesses, vector, lanes);
    } else {
        read_vector(accesses, vector, lanes);
        write(accesses, memory, AccessOccurrence::Always);
    }
    write_vector_integer(accesses, base);
    if base.index() == 0 {
        add_vector_memory_references(references, if decrement { 0x0ff0 } else { 0 }, lanes, !load);
    }
}

fn vector_three_accesses(
    form: InstructionForm,
    lanes: VectorDestinationMask,
    destination: VectorFloatingRegister,
    left: VectorFloatingRegister,
    right: VectorFloatingRegister,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_vector(accesses, left, lanes);
    read_vector(accesses, right, lanes);
    vector_accumulator_input(form, lanes, accesses);
    write_vector(accesses, destination, lanes);
    vector_flags(form, lanes, accesses);
}

fn vector_scalar_input(form: InstructionForm, accesses: &mut Vec<LocationAccess<Location>>) {
    if matches!(
        form,
        InstructionForm::VaddI
            | InstructionForm::VaddaI
            | InstructionForm::VmaddI
            | InstructionForm::VmaddaI
            | InstructionForm::VmaxI
            | InstructionForm::VminiI
            | InstructionForm::VmsubI
            | InstructionForm::VmsubaI
            | InstructionForm::VmulI
            | InstructionForm::VmulaI
            | InstructionForm::VsubI
            | InstructionForm::VsubaI
    ) {
        read_vector_control(accesses, VectorControlRegister::IMMEDIATE);
    }
    if matches!(
        form,
        InstructionForm::VaddQ
            | InstructionForm::VaddaQ
            | InstructionForm::VmaddQ
            | InstructionForm::VmaddaQ
            | InstructionForm::VmsubQ
            | InstructionForm::VmsubaQ
            | InstructionForm::VmulQ
            | InstructionForm::VmulaQ
            | InstructionForm::VsubQ
            | InstructionForm::VsubaQ
    ) {
        read_vector_control(accesses, VectorControlRegister::QUOTIENT);
    }
}

fn vector_accumulator_input(
    form: InstructionForm,
    lanes: VectorDestinationMask,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    if matches!(
        form,
        InstructionForm::Vmadd
            | InstructionForm::VmaddI
            | InstructionForm::VmaddQ
            | InstructionForm::VmaddBc
            | InstructionForm::Vmsub
            | InstructionForm::VmsubI
            | InstructionForm::VmsubQ
            | InstructionForm::VmsubBc
    ) {
        read(
            accesses,
            Location::VectorState(VectorState::Accumulator(lanes)),
            AccessOccurrence::Always,
        );
    }
}

fn vector_accumulator_update(
    form: InstructionForm,
    lanes: VectorDestinationMask,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    if matches!(
        form,
        InstructionForm::Vmadda
            | InstructionForm::VmaddaI
            | InstructionForm::VmaddaQ
            | InstructionForm::VmaddaBc
            | InstructionForm::Vmsuba
            | InstructionForm::VmsubaI
            | InstructionForm::VmsubaQ
            | InstructionForm::VmsubaBc
    ) {
        read(
            accesses,
            Location::VectorState(VectorState::Accumulator(lanes)),
            AccessOccurrence::Always,
        );
    }
    write(
        accesses,
        Location::VectorState(VectorState::Accumulator(lanes)),
        AccessOccurrence::Always,
    );
}

fn vector_flags(
    form: InstructionForm,
    lanes: VectorDestinationMask,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    if vector_flag_form(form) {
        write(
            accesses,
            Location::VectorState(VectorState::MacFlags(lanes)),
            AccessOccurrence::Always,
        );
        write(
            accesses,
            Location::VectorState(VectorState::StatusFlags),
            AccessOccurrence::Always,
        );
    }
}

fn random_source_accesses(form: InstructionForm, accesses: &mut Vec<LocationAccess<Location>>) {
    if form == InstructionForm::Vrxor {
        read_vector_control(accesses, VectorControlRegister::RANDOM);
    }
    write_vector_control(accesses, VectorControlRegister::RANDOM);
}

fn packed_product_accesses(form: InstructionForm, accesses: &mut Vec<LocationAccess<Location>>) {
    if matches!(
        form,
        InstructionForm::Phmadh
            | InstructionForm::Phmsbh
            | InstructionForm::Pmaddh
            | InstructionForm::Pmadduw
            | InstructionForm::Pmaddw
            | InstructionForm::Pmsubh
            | InstructionForm::Pmsubw
    ) {
        read_product_full(accesses);
        write_product_full(accesses, AccessOccurrence::Always);
    } else if matches!(
        form,
        InstructionForm::Pmulth | InstructionForm::Pmultuw | InstructionForm::Pmultw
    ) {
        write_product_full(accesses, AccessOccurrence::Always);
    }
}

fn general_destination_source(form: InstructionForm, accesses: &mut Vec<LocationAccess<Location>>) {
    match form {
        InstructionForm::Mfhi => read_product_half(accesses, ProductRegister::Hi, false),
        InstructionForm::Mflo => read_product_half(accesses, ProductRegister::Lo, false),
        InstructionForm::Mfhi1 => read_product_half(accesses, ProductRegister::Hi, true),
        InstructionForm::Mflo1 => read_product_half(accesses, ProductRegister::Lo, true),
        InstructionForm::Mfsa => read(accesses, Location::ShiftAmount, AccessOccurrence::Always),
        InstructionForm::Pmfhi => read_product_register_full(accesses, ProductRegister::Hi),
        InstructionForm::Pmflo => read_product_register_full(accesses, ProductRegister::Lo),
        InstructionForm::PmfhlLh
        | InstructionForm::PmfhlLw
        | InstructionForm::PmfhlSh
        | InstructionForm::PmfhlSlw
        | InstructionForm::PmfhlUw => read_product_full(accesses),
        form => read(
            accesses,
            Location::Debug(debug_register(form).unwrap()),
            AccessOccurrence::Always,
        ),
    }
}

fn general_source_destination(form: InstructionForm, accesses: &mut Vec<LocationAccess<Location>>) {
    match form {
        InstructionForm::Mthi => write_product_half(
            accesses,
            ProductRegister::Hi,
            false,
            AccessOccurrence::Always,
        ),
        InstructionForm::Mtlo => write_product_half(
            accesses,
            ProductRegister::Lo,
            false,
            AccessOccurrence::Always,
        ),
        InstructionForm::Mthi1 => write_product_half(
            accesses,
            ProductRegister::Hi,
            true,
            AccessOccurrence::Always,
        ),
        InstructionForm::Mtlo1 => write_product_half(
            accesses,
            ProductRegister::Lo,
            true,
            AccessOccurrence::Always,
        ),
        InstructionForm::Mtsa => write(accesses, Location::ShiftAmount, AccessOccurrence::Always),
        InstructionForm::Pmthi => write(
            accesses,
            Location::Product(ProductStorage {
                register: ProductRegister::Hi,
                byte_offset: 0,
                byte_len: 16,
            }),
            AccessOccurrence::Always,
        ),
        InstructionForm::PmthlLw => write_product_full(accesses, AccessOccurrence::Always),
        InstructionForm::Pmtlo => write(
            accesses,
            Location::Product(ProductStorage {
                register: ProductRegister::Lo,
                byte_offset: 0,
                byte_len: 16,
            }),
            AccessOccurrence::Always,
        ),
        form => write(
            accesses,
            Location::Debug(debug_register(form).unwrap()),
            AccessOccurrence::Always,
        ),
    }
}

fn debug_register(form: InstructionForm) -> Option<DebugRegister> {
    Some(match form {
        InstructionForm::Mfbpc | InstructionForm::Mtbpc => DebugRegister::BreakpointControl,
        InstructionForm::Mfdab | InstructionForm::Mtdab => DebugRegister::DataAddress,
        InstructionForm::Mfdabm | InstructionForm::Mtdabm => DebugRegister::DataAddressMask,
        InstructionForm::Mfdvb | InstructionForm::Mtdvb => DebugRegister::DataValue,
        InstructionForm::Mfdvbm | InstructionForm::Mtdvbm => DebugRegister::DataValueMask,
        InstructionForm::Mfiab | InstructionForm::Mtiab => DebugRegister::InstructionAddress,
        InstructionForm::Mfiabm | InstructionForm::Mtiabm => DebugRegister::InstructionAddressMask,
        _ => return None,
    })
}

fn exception_state(accesses: &mut Vec<LocationAccess<Location>>, occurrence: AccessOccurrence) {
    read(
        accesses,
        Location::SystemControl(SystemControlRegister::Status),
        occurrence,
    );
    for register in [
        SystemControlRegister::Status,
        SystemControlRegister::Cause,
        SystemControlRegister::ExceptionProgramCounter,
    ] {
        write(accesses, Location::SystemControl(register), occurrence);
    }
}

fn floating_flags(accesses: &mut Vec<LocationAccess<Location>>) {
    write(
        accesses,
        Location::FpuState(FpuState::ArithmeticFlags),
        AccessOccurrence::Always,
    );
}

fn read_floating(accesses: &mut Vec<LocationAccess<Location>>, register: FloatingRegister) {
    read(
        accesses,
        Location::Floating(register),
        AccessOccurrence::Always,
    );
}

fn write_floating(accesses: &mut Vec<LocationAccess<Location>>, register: FloatingRegister) {
    write(
        accesses,
        Location::Floating(register),
        AccessOccurrence::Always,
    );
}

fn read_gpr(accesses: &mut Vec<LocationAccess<Location>>, register: GeneralRegister, byte_len: u8) {
    if register != GeneralRegister::ZERO {
        read(
            accesses,
            Location::General(GeneralStorage {
                register,
                byte_offset: 0,
                byte_len,
            }),
            AccessOccurrence::Always,
        );
    }
}

fn write_gpr(
    accesses: &mut Vec<LocationAccess<Location>>,
    register: GeneralRegister,
    byte_len: u8,
    occurrence: AccessOccurrence,
) {
    if register != GeneralRegister::ZERO {
        write(
            accesses,
            Location::General(GeneralStorage {
                register,
                byte_offset: 0,
                byte_len,
            }),
            occurrence,
        );
    }
}

fn read_product_bank(accesses: &mut Vec<LocationAccess<Location>>, upper: bool) {
    for register in [ProductRegister::Hi, ProductRegister::Lo] {
        read_product_half(accesses, register, upper);
    }
}

fn write_product_bank(
    accesses: &mut Vec<LocationAccess<Location>>,
    upper: bool,
    occurrence: AccessOccurrence,
) {
    for register in [ProductRegister::Hi, ProductRegister::Lo] {
        write_product_half(accesses, register, upper, occurrence);
    }
}

fn read_product_half(
    accesses: &mut Vec<LocationAccess<Location>>,
    register: ProductRegister,
    upper: bool,
) {
    read(
        accesses,
        Location::Product(ProductStorage {
            register,
            byte_offset: if upper { 8 } else { 0 },
            byte_len: 8,
        }),
        AccessOccurrence::Always,
    );
}

fn write_product_half(
    accesses: &mut Vec<LocationAccess<Location>>,
    register: ProductRegister,
    upper: bool,
    occurrence: AccessOccurrence,
) {
    write(
        accesses,
        Location::Product(ProductStorage {
            register,
            byte_offset: if upper { 8 } else { 0 },
            byte_len: 8,
        }),
        occurrence,
    );
}

fn read_product_full(accesses: &mut Vec<LocationAccess<Location>>) {
    for register in [ProductRegister::Hi, ProductRegister::Lo] {
        read_product_register_full(accesses, register);
    }
}

fn read_product_register_full(
    accesses: &mut Vec<LocationAccess<Location>>,
    register: ProductRegister,
) {
    read(
        accesses,
        Location::Product(ProductStorage {
            register,
            byte_offset: 0,
            byte_len: 16,
        }),
        AccessOccurrence::Always,
    );
}

fn write_product_full(accesses: &mut Vec<LocationAccess<Location>>, occurrence: AccessOccurrence) {
    for register in [ProductRegister::Hi, ProductRegister::Lo] {
        write(
            accesses,
            Location::Product(ProductStorage {
                register,
                byte_offset: 0,
                byte_len: 16,
            }),
            occurrence,
        );
    }
}

fn read_vector(
    accesses: &mut Vec<LocationAccess<Location>>,
    register: VectorFloatingRegister,
    lanes: VectorDestinationMask,
) {
    if register.index() != 0 && lanes != VectorDestinationMask::NONE {
        read(
            accesses,
            Location::VectorFloating { register, lanes },
            AccessOccurrence::Always,
        );
    }
}

fn write_vector(
    accesses: &mut Vec<LocationAccess<Location>>,
    register: VectorFloatingRegister,
    lanes: VectorDestinationMask,
) {
    if register.index() != 0 && lanes != VectorDestinationMask::NONE {
        write(
            accesses,
            Location::VectorFloating { register, lanes },
            AccessOccurrence::Always,
        );
    }
}

fn read_vector_integer(
    accesses: &mut Vec<LocationAccess<Location>>,
    register: VectorIntegerRegister,
) {
    if register.index() != 0 {
        read(
            accesses,
            Location::VectorInteger(register),
            AccessOccurrence::Always,
        );
    }
}

fn write_vector_integer(
    accesses: &mut Vec<LocationAccess<Location>>,
    register: VectorIntegerRegister,
) {
    if register.index() != 0 {
        write(
            accesses,
            Location::VectorInteger(register),
            AccessOccurrence::Always,
        );
    }
}

fn read_vector_control(
    accesses: &mut Vec<LocationAccess<Location>>,
    register: VectorControlRegister,
) {
    if register.index() != 0 {
        read(
            accesses,
            Location::VectorControl(register),
            AccessOccurrence::Always,
        );
    }
}

fn write_vector_control(
    accesses: &mut Vec<LocationAccess<Location>>,
    register: VectorControlRegister,
) {
    if register.index() != 0 {
        write(
            accesses,
            Location::VectorControl(register),
            AccessOccurrence::Always,
        );
    }
}

fn lane_mask(lane: VectorLane) -> VectorDestinationMask {
    VectorDestinationMask::new(lane.destination_bits()).unwrap()
}

fn add_vector_memory_references(
    references: &mut Vec<StaticReference<Reference>>,
    quadword_address: u32,
    lanes: VectorDestinationMask,
    store: bool,
) {
    for lane in [VectorLane::X, VectorLane::Y, VectorLane::Z, VectorLane::W] {
        if lanes.bits() & lane.destination_bits() != 0 {
            add_reference(
                references,
                StaticReference {
                    target: Reference::VectorMemory {
                        address: quadword_address + u32::from(lane.field()) * 4,
                        byte_len: 4,
                    },
                    kind: if store {
                        ReferenceKind::Write
                    } else {
                        ReferenceKind::Read
                    },
                    occurrence: AccessOccurrence::Always,
                },
            );
        }
    }
}

fn tlb_entry_registers() -> [SystemControlRegister; 4] {
    [
        SystemControlRegister::PageMask,
        SystemControlRegister::EntryHigh,
        SystemControlRegister::EntryLow0,
        SystemControlRegister::EntryLow1,
    ]
}

fn is_scalar_three_general(form: InstructionForm) -> bool {
    matches!(
        form,
        InstructionForm::Add
            | InstructionForm::Addu
            | InstructionForm::And
            | InstructionForm::Dadd
            | InstructionForm::Daddu
            | InstructionForm::Dsub
            | InstructionForm::Dsubu
            | InstructionForm::Movn
            | InstructionForm::Movz
            | InstructionForm::Nor
            | InstructionForm::Or
            | InstructionForm::Slt
            | InstructionForm::Sltu
            | InstructionForm::Sub
            | InstructionForm::Subu
            | InstructionForm::Xor
    )
}

fn is_overflow_form(form: InstructionForm) -> bool {
    matches!(
        form,
        InstructionForm::Add
            | InstructionForm::Addi
            | InstructionForm::Dadd
            | InstructionForm::Daddi
            | InstructionForm::Dsub
            | InstructionForm::Sub
    )
}

fn is_conditional_move(form: InstructionForm) -> bool {
    matches!(form, InstructionForm::Movn | InstructionForm::Movz)
}

fn is_link_branch(form: InstructionForm) -> bool {
    matches!(
        form,
        InstructionForm::Bgezal
            | InstructionForm::Bgezall
            | InstructionForm::Bltzal
            | InstructionForm::Bltzall
    )
}

fn is_likely_branch(form: InstructionForm) -> bool {
    matches!(
        form,
        InstructionForm::Beql
            | InstructionForm::Bgezall
            | InstructionForm::Bgezl
            | InstructionForm::Bgtzl
            | InstructionForm::Blezl
            | InstructionForm::Bltzall
            | InstructionForm::Bltzl
            | InstructionForm::Bnel
            | InstructionForm::Bc0fl
            | InstructionForm::Bc0tl
            | InstructionForm::Bc1fl
            | InstructionForm::Bc1tl
            | InstructionForm::Bc2fl
            | InstructionForm::Bc2tl
    )
}

fn vector_flag_form(form: InstructionForm) -> bool {
    matches!(
        form,
        InstructionForm::Vadd
            | InstructionForm::VaddI
            | InstructionForm::VaddQ
            | InstructionForm::VaddBc
            | InstructionForm::Vadda
            | InstructionForm::VaddaI
            | InstructionForm::VaddaQ
            | InstructionForm::VaddaBc
            | InstructionForm::Vmadd
            | InstructionForm::VmaddI
            | InstructionForm::VmaddQ
            | InstructionForm::VmaddBc
            | InstructionForm::Vmadda
            | InstructionForm::VmaddaI
            | InstructionForm::VmaddaQ
            | InstructionForm::VmaddaBc
            | InstructionForm::Vmsub
            | InstructionForm::VmsubI
            | InstructionForm::VmsubQ
            | InstructionForm::VmsubBc
            | InstructionForm::Vmsuba
            | InstructionForm::VmsubaI
            | InstructionForm::VmsubaQ
            | InstructionForm::VmsubaBc
            | InstructionForm::Vmul
            | InstructionForm::VmulI
            | InstructionForm::VmulQ
            | InstructionForm::VmulBc
            | InstructionForm::Vmula
            | InstructionForm::VmulaI
            | InstructionForm::VmulaQ
            | InstructionForm::VmulaBc
            | InstructionForm::Vopmula
            | InstructionForm::Vopmsub
            | InstructionForm::Vsub
            | InstructionForm::VsubI
            | InstructionForm::VsubQ
            | InstructionForm::VsubBc
            | InstructionForm::Vsuba
            | InstructionForm::VsubaI
            | InstructionForm::VsubaQ
            | InstructionForm::VsubaBc
    )
}

fn width_bytes(width: MemoryWidth) -> u8 {
    match width {
        MemoryWidth::Known(bytes) | MemoryWidth::Merge(bytes) => bytes,
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
                occurrence: AccessOccurrence::Always,
            },
        );
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
        (Location::General(left), Location::General(right)) if left.register == right.register => {
            range_relation(
                left.byte_offset,
                left.byte_len,
                right.byte_offset,
                right.byte_len,
            )
        }
        (Location::General(_), Location::General(_)) => Disjoint,
        (Location::Product(left), Location::Product(right)) if left.register == right.register => {
            range_relation(
                left.byte_offset,
                left.byte_len,
                right.byte_offset,
                right.byte_len,
            )
        }
        (Location::Product(_), Location::Product(_)) => Disjoint,
        (
            Location::FloatingControl(FloatingControlRegister::ControlStatus),
            Location::FpuState(_),
        )
        | (
            Location::FpuState(_),
            Location::FloatingControl(FloatingControlRegister::ControlStatus),
        ) => Overlapping,
        (
            Location::VectorFloating {
                register: left_register,
                lanes: left_lanes,
            },
            Location::VectorFloating {
                register: right_register,
                lanes: right_lanes,
            },
        ) if left_register == right_register => lane_relation(*left_lanes, *right_lanes),
        (Location::VectorFloating { .. }, Location::VectorFloating { .. }) => Disjoint,
        (
            Location::VectorState(VectorState::Accumulator(left)),
            Location::VectorState(VectorState::Accumulator(right)),
        )
        | (
            Location::VectorState(VectorState::MacFlags(left)),
            Location::VectorState(VectorState::MacFlags(right)),
        ) => lane_relation(*left, *right),
        (Location::VectorInteger(integer), Location::VectorControl(control))
        | (Location::VectorControl(control), Location::VectorInteger(integer))
            if integer.index() == control.index() =>
        {
            Same
        }
        (
            Location::VectorControl(VectorControlRegister::STATUS),
            Location::VectorState(VectorState::StatusFlags),
        )
        | (
            Location::VectorState(VectorState::StatusFlags),
            Location::VectorControl(VectorControlRegister::STATUS),
        )
        | (
            Location::VectorControl(VectorControlRegister::MAC),
            Location::VectorState(VectorState::MacFlags(_)),
        )
        | (
            Location::VectorState(VectorState::MacFlags(_)),
            Location::VectorControl(VectorControlRegister::MAC),
        )
        | (
            Location::VectorControl(VectorControlRegister::CLIPPING),
            Location::VectorState(VectorState::ClippingFlags),
        )
        | (
            Location::VectorState(VectorState::ClippingFlags),
            Location::VectorControl(VectorControlRegister::CLIPPING),
        ) => Overlapping,
        (Location::Memory(left), Location::Memory(right)) => {
            if left.address == right.address {
                Overlapping
            } else {
                Unknown
            }
        }
        (Location::VectorMemory(left), Location::VectorMemory(right)) => {
            if left.address == right.address {
                lane_relation(left.lanes, right.lanes)
            } else {
                Unknown
            }
        }
        _ => Disjoint,
    }
}

fn range_relation(
    left_offset: u8,
    left_len: u8,
    right_offset: u8,
    right_len: u8,
) -> StorageRelation {
    let left_end = u16::from(left_offset) + u16::from(left_len);
    let right_end = u16::from(right_offset) + u16::from(right_len);
    if u16::from(left_offset) < right_end && u16::from(right_offset) < left_end {
        StorageRelation::Overlapping
    } else {
        StorageRelation::Disjoint
    }
}

fn lane_relation(left: VectorDestinationMask, right: VectorDestinationMask) -> StorageRelation {
    if left.bits() & right.bits() == 0 {
        StorageRelation::Disjoint
    } else {
        StorageRelation::Overlapping
    }
}

impl typed_isa_core::StaticSemantics for crate::R5900 {
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
