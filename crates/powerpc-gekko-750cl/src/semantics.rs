//! Complete static meaning shared by the Gekko and PowerPC 750CL profiles.

use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlBoundary, ControlTarget, LocationAccess,
    ReferenceKind, StaticReference, StorageRelation,
};

use crate::{
    ConditionRegisterBit, ConditionRegisterField, EncodeError, FloatingRegister, GeneralRegister,
    Instruction, InstructionForm, Operand, PairedSingleWidth, QuantizationRegister,
    SegmentRegister, SpecialPurposeRegister, TimeBaseRegister,
};

/// Placed PowerPC instruction used by static analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnalysisPoint {
    /// 32-bit architectural instruction address.
    pub location: crate::CodeLocation,
}

impl AnalysisPoint {
    /// Constructs a placed instruction.
    pub const fn new(location: crate::CodeLocation) -> Self {
        Self { location }
    }
}

/// Runtime-derived PowerPC control destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndirectTarget {
    /// Target read from the link register.
    LinkRegister,
    /// Target read from the count register.
    CountRegister,
    /// Target and machine state restored by `rfi`.
    ReturnFromInterrupt,
    /// System-call vector selected by machine configuration.
    SystemCallVector,
    /// Program exception vector selected by a trap instruction.
    ProgramExceptionVector,
}

/// Synchronous PowerPC boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Boundary {
    /// `sc` system-call exception.
    SystemCall,
    /// Conditional `tw` or `twi` program exception.
    Trap,
}

/// Effective-address rule used by a memory instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryAddress {
    /// Base register plus a signed displacement.
    BaseOffset {
        /// `None` represents the architectural zero base selected by r0.
        base: Option<GeneralRegister>,
        /// Signed byte displacement.
        offset: i16,
    },
    /// Base register plus index register.
    BaseIndex {
        /// `None` represents the architectural zero base selected by r0.
        base: Option<GeneralRegister>,
        /// Index register.
        index: GeneralRegister,
    },
}

/// Width of a PowerPC memory operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryWidth {
    /// Fixed contiguous byte count.
    Known(u8),
    /// String or multiple-register transfer with a known byte count.
    Sequence(u8),
    /// String transfer whose count is read from XER.
    XerSelectedSequence,
    /// Quantized paired-single transfer whose element representation is GQR-defined.
    Quantized(PairedSingleWidth),
    /// Cache-block operation without an architecturally fixed transfer size.
    CacheBlock,
}

/// Symbolic PowerPC memory storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryLocation {
    /// Effective-address rule.
    pub address: MemoryAddress,
    /// Transfer width.
    pub width: MemoryWidth,
}

/// Stable implicit processor state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImplicitState {
    /// Machine state register.
    MachineState,
    /// Reservation used by `lwarx` and `stwcx.`.
    Reservation,
    /// Translation lookaside buffer state.
    Tlb,
    /// The segment-register bank selected indirectly from a general register.
    SegmentRegisters,
    /// Cache state selected by a cache instruction.
    Cache,
    /// Ordering state established by synchronization instructions.
    MemoryOrdering,
}

/// Independently addressable fields of the fixed-point exception register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntegerExceptionField {
    /// Sticky summary-overflow bit (`XER[SO]`).
    SummaryOverflow,
    /// Overflow bit (`XER[OV]`).
    Overflow,
    /// Carry bit (`XER[CA]`).
    Carry,
    /// Runtime byte count used by indexed string transfers.
    StringByteCount,
}

/// PowerPC architectural storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Location {
    /// One 32-bit general-purpose register.
    GeneralRegister(GeneralRegister),
    /// A wrapping sequence of general-purpose registers used by string/multiple forms.
    GeneralRegisterRange {
        /// First register.
        first: GeneralRegister,
        /// Number of transferred bytes.
        byte_len: u8,
    },
    /// A wrapping general-register sequence whose byte count comes from XER.
    GeneralRegisterRangeSelectedByXer {
        /// First register in the sequence.
        first: GeneralRegister,
    },
    /// One 64-bit floating-point or paired-single register.
    FloatingRegister(FloatingRegister),
    /// Whole 32-bit condition register.
    ConditionRegister,
    /// One four-bit condition-register field.
    ConditionField(ConditionRegisterField),
    /// One condition-register bit.
    ConditionBit(ConditionRegisterBit),
    /// Whole floating-point status/control register.
    FloatingStatusControl,
    /// One FPSCR field selected by a mask or field index.
    FloatingStatusField(u8),
    /// One segment register.
    SegmentRegister(SegmentRegister),
    /// One special-purpose register.
    SpecialPurposeRegister(SpecialPurposeRegister),
    /// One independently observable field of XER.
    IntegerExceptionField(IntegerExceptionField),
    /// One time-base half.
    TimeBase(TimeBaseRegister),
    /// One graphics quantization register.
    QuantizationRegister(QuantizationRegister),
    /// Stable implicit state.
    Implicit(ImplicitState),
    /// Symbolic memory storage.
    Memory(MemoryLocation),
}

/// Statically resolved PowerPC address reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reference {
    /// Data range addressed with the architectural zero base.
    Memory {
        /// First referenced byte.
        address: u32,
        /// Referenced byte count when fixed.
        byte_len: u8,
    },
    /// Address formed without claiming a data transfer.
    Address(u32),
}

/// Failure to form sound profile-specific static facts.
#[derive(Debug, thiserror::Error)]
pub enum SemanticError {
    /// The instruction is not encodable by the Gekko exact profile.
    #[error("Gekko semantic validation failed: {0}")]
    Gekko(EncodeError),
    /// The instruction is not encodable by the PowerPC 750CL exact profile.
    #[error("PowerPC 750CL semantic validation failed: {0}")]
    PowerPc750Cl(EncodeError),
}

/// Shared control-flow result specialized with PowerPC vocabulary.
pub type ControlFlow = typed_isa_core::ControlFlow<AnalysisPoint, IndirectTarget, Boundary>;

/// Shared instruction-semantics result specialized with PowerPC vocabulary.
pub type InstructionSemantics = typed_isa_core::InstructionSemantics<
    AnalysisPoint,
    IndirectTarget,
    Boundary,
    Location,
    Reference,
>;

#[derive(Debug, Clone, Copy)]
pub(crate) enum Profile {
    Gekko,
    PowerPc750Cl,
}

pub(crate) fn semantics(
    instruction: &Instruction,
    at: AnalysisPoint,
    profile: Profile,
) -> Result<InstructionSemantics, SemanticError> {
    match profile {
        Profile::Gekko => {
            crate::encode_gekko_word(instruction, at.location).map_err(SemanticError::Gekko)?
        }
        Profile::PowerPc750Cl => crate::encode_750cl_word(instruction, at.location)
            .map_err(SemanticError::PowerPc750Cl)?,
    };
    let next = AnalysisPoint::new(at.location.wrapping_add(4));
    let mut accesses = Vec::new();
    let mut references = Vec::new();
    derive_accesses(instruction, &mut accesses, &mut references);
    Ok(InstructionSemantics {
        control_flow: control_flow(instruction, next),
        location_accesses: accesses,
        static_references: references,
    })
}

fn control_flow(instruction: &Instruction, next: AnalysisPoint) -> ControlFlow {
    let form = instruction.form();
    let modifiers = instruction.modifiers();
    match form {
        InstructionForm::B => {
            let target = branch_target(instruction);
            branch_action(target, modifiers.link, next, None)
        }
        InstructionForm::Bc => {
            let target = branch_target(instruction);
            branch_action(
                target,
                modifiers.link,
                next,
                branch_fallthrough(instruction, next),
            )
        }
        InstructionForm::Bclr => ControlFlow {
            action: if modifiers.link {
                ControlAction::LinkedTransfer {
                    target: ControlTarget::Indirect(IndirectTarget::LinkRegister),
                    return_site: next,
                }
            } else {
                ControlAction::Transfer {
                    target: ControlTarget::Indirect(IndirectTarget::LinkRegister),
                }
            },
            fallthrough: branch_fallthrough(instruction, next),
            delay_slot: None,
        },
        InstructionForm::Bcctr => ControlFlow {
            action: if modifiers.link {
                ControlAction::LinkedTransfer {
                    target: ControlTarget::Indirect(IndirectTarget::CountRegister),
                    return_site: next,
                }
            } else {
                ControlAction::Transfer {
                    target: ControlTarget::Indirect(IndirectTarget::CountRegister),
                }
            },
            fallthrough: branch_fallthrough(instruction, next),
            delay_slot: None,
        },
        InstructionForm::Sc => ControlFlow {
            action: ControlAction::Boundary(ControlBoundary::Trap {
                kind: Boundary::SystemCall,
                target: Some(ControlTarget::Indirect(IndirectTarget::SystemCallVector)),
                resume: None,
            }),
            fallthrough: None,
            delay_slot: None,
        },
        InstructionForm::Tw | InstructionForm::Twi => ControlFlow {
            action: ControlAction::Boundary(ControlBoundary::Trap {
                kind: Boundary::Trap,
                target: Some(ControlTarget::Indirect(
                    IndirectTarget::ProgramExceptionVector,
                )),
                resume: None,
            }),
            fallthrough: Some(next),
            delay_slot: None,
        },
        InstructionForm::Rfi => ControlFlow {
            action: ControlAction::ExceptionReturn {
                target: IndirectTarget::ReturnFromInterrupt,
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

fn branch_action(
    target: u32,
    link: bool,
    next: AnalysisPoint,
    fallthrough: Option<AnalysisPoint>,
) -> ControlFlow {
    ControlFlow {
        action: if link {
            ControlAction::LinkedTransfer {
                target: ControlTarget::Direct(AnalysisPoint::new(target)),
                return_site: next,
            }
        } else {
            ControlAction::Transfer {
                target: ControlTarget::Direct(AnalysisPoint::new(target)),
            }
        },
        fallthrough,
        delay_slot: None,
    }
}

fn branch_target(instruction: &Instruction) -> u32 {
    instruction
        .operands()
        .iter()
        .find_map(|operand| match operand {
            Operand::BranchTarget(target) => Some(*target),
            _ => None,
        })
        .expect("branch form has a validated branch target")
}

fn branch_fallthrough(instruction: &Instruction, next: AnalysisPoint) -> Option<AnalysisPoint> {
    let options = instruction
        .operands()
        .iter()
        .find_map(|operand| match operand {
            Operand::BranchOptions(options) => Some(*options),
            _ => None,
        });
    options
        .is_none_or(|options| options.value() & 0b1_0100 != 0b1_0100)
        .then_some(next)
}

fn derive_accesses(
    instruction: &Instruction,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    let form = instruction.form();
    if is_memory_form(form) {
        memory_accesses(instruction, accesses, references);
    } else {
        non_memory_operand_accesses(instruction, accesses);
    }
    control_and_system_accesses(instruction, accesses);
    modifier_accesses(instruction, accesses);
}

fn non_memory_operand_accesses(
    instruction: &Instruction,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    let form = instruction.form();
    match form {
        InstructionForm::B
        | InstructionForm::Bc
        | InstructionForm::Bcctr
        | InstructionForm::Bclr
        | InstructionForm::Sc
        | InstructionForm::Rfi
        | InstructionForm::Isync
        | InstructionForm::Sync
        | InstructionForm::Eieio
        | InstructionForm::Tlbsync => {}
        InstructionForm::Mfspr => {
            write_operand(accesses, instruction.operands()[0]);
            read_operand(accesses, instruction.operands()[1]);
        }
        InstructionForm::Mtspr => {
            write_operand(accesses, instruction.operands()[0]);
            read_operand(accesses, instruction.operands()[1]);
        }
        InstructionForm::Mfcr
        | InstructionForm::Mfmsr
        | InstructionForm::Mfsr
        | InstructionForm::Mftb => {
            write_operand(accesses, instruction.operands()[0]);
            for operand in &instruction.operands()[1..] {
                read_operand(accesses, *operand);
            }
            match form {
                InstructionForm::Mfcr => read(
                    accesses,
                    Location::ConditionRegister,
                    AccessOccurrence::Always,
                ),
                InstructionForm::Mfmsr => read(
                    accesses,
                    Location::Implicit(ImplicitState::MachineState),
                    AccessOccurrence::Always,
                ),
                _ => {}
            }
        }
        InstructionForm::Mfsrin => {
            write_operand(accesses, instruction.operands()[0]);
            read_operand(accesses, instruction.operands()[1]);
            read(
                accesses,
                Location::Implicit(ImplicitState::SegmentRegisters),
                AccessOccurrence::Always,
            );
        }
        InstructionForm::Mtcrf => {
            read_operand(accesses, instruction.operands()[1]);
            let Operand::ConditionRegisterMask(mask) = instruction.operands()[0] else {
                unreachable!("validated mtcrf has a condition-register mask")
            };
            for field in 0..8 {
                if mask.bits() & (0x80 >> field) != 0 {
                    write(
                        accesses,
                        Location::ConditionField(ConditionRegisterField::new(field).unwrap()),
                        AccessOccurrence::Always,
                    );
                }
            }
        }
        InstructionForm::Mtmsr => {
            read_operand(accesses, instruction.operands()[0]);
            write(
                accesses,
                Location::Implicit(ImplicitState::MachineState),
                AccessOccurrence::Always,
            );
        }
        InstructionForm::Mtsr => {
            let Operand::SegmentRegister(register) = instruction.operands()[0] else {
                unreachable!("validated mtsr has a segment-register destination")
            };
            read_operand(accesses, instruction.operands()[1]);
            write(
                accesses,
                Location::SegmentRegister(register),
                AccessOccurrence::Always,
            );
        }
        InstructionForm::Mtsrin => {
            for operand in instruction.operands() {
                read_operand(accesses, *operand);
            }
            write(
                accesses,
                Location::Implicit(ImplicitState::SegmentRegisters),
                AccessOccurrence::Always,
            );
        }
        InstructionForm::Mffs => {
            write_operand(accesses, instruction.operands()[0]);
            read(
                accesses,
                Location::FloatingStatusControl,
                AccessOccurrence::Always,
            );
        }
        InstructionForm::Mtfsf => {
            read_operand(accesses, instruction.operands()[1]);
            let Operand::FpscrFieldMask(mask) = instruction.operands()[0] else {
                unreachable!("validated mtfsf has an FPSCR field mask")
            };
            for field in 0..8 {
                if mask.bits() & (0x80 >> field) != 0 {
                    write(
                        accesses,
                        Location::FloatingStatusField(field),
                        AccessOccurrence::Always,
                    );
                }
            }
        }
        InstructionForm::Mtfsfi => {
            let Operand::ConditionRegisterField(field) = instruction.operands()[0] else {
                unreachable!("validated mtfsfi has an FPSCR field selector")
            };
            write(
                accesses,
                Location::FloatingStatusField(field.value()),
                AccessOccurrence::Always,
            );
        }
        InstructionForm::Mtfsb0 | InstructionForm::Mtfsb1 => {
            let Operand::ConditionRegisterBit(bit) = instruction.operands()[0] else {
                unreachable!("validated mtfsb has an FPSCR bit selector")
            };
            write(
                accesses,
                Location::FloatingStatusField(bit.value() / 4),
                AccessOccurrence::Always,
            );
        }
        InstructionForm::Mcrfs => {
            write_operand(accesses, instruction.operands()[0]);
            let Operand::ConditionRegisterField(field) = instruction.operands()[1] else {
                unreachable!("validated mcrfs has an FPSCR field selector")
            };
            read_write(accesses, Location::FloatingStatusField(field.value()));
        }
        InstructionForm::Mcrxr => {
            write_operand(accesses, instruction.operands()[0]);
            for field in [
                IntegerExceptionField::SummaryOverflow,
                IntegerExceptionField::Overflow,
                IntegerExceptionField::Carry,
            ] {
                read_write(accesses, Location::IntegerExceptionField(field));
            }
        }
        _ if is_compare_form(form) => {
            write_operand(accesses, instruction.operands()[0]);
            for operand in &instruction.operands()[1..] {
                read_operand(accesses, *operand);
            }
            if is_floating_compare_form(form) {
                read_write(accesses, Location::FloatingStatusControl);
            }
        }
        InstructionForm::Rlwimi => {
            read_operand(accesses, instruction.operands()[0]);
            write_operand(accesses, instruction.operands()[0]);
            for operand in &instruction.operands()[1..] {
                read_operand(accesses, *operand);
            }
        }
        _ => {
            let mut wrote_destination = false;
            for operand in instruction.operands() {
                if !wrote_destination && operand_can_be_destination(*operand) {
                    write_operand(accesses, *operand);
                    wrote_destination = true;
                } else {
                    read_operand(accesses, *operand);
                }
            }
        }
    }
    if updates_floating_status(form) {
        read_write(accesses, Location::FloatingStatusControl);
    }
}

fn control_and_system_accesses(
    instruction: &Instruction,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    use InstructionForm::*;
    let form = instruction.form();
    match form {
        B | Bc => {
            if form == Bc {
                branch_condition_accesses(instruction, accesses);
            }
            if instruction.modifiers().link {
                write(
                    accesses,
                    Location::SpecialPurposeRegister(SpecialPurposeRegister::Link),
                    AccessOccurrence::Always,
                );
            }
        }
        Bclr | Bcctr => {
            branch_condition_accesses(instruction, accesses);
            read(
                accesses,
                Location::SpecialPurposeRegister(if form == Bclr {
                    SpecialPurposeRegister::Link
                } else {
                    SpecialPurposeRegister::Count
                }),
                AccessOccurrence::Always,
            );
            if instruction.modifiers().link {
                write(
                    accesses,
                    Location::SpecialPurposeRegister(SpecialPurposeRegister::Link),
                    AccessOccurrence::Always,
                );
            }
        }
        Sc | Tw | Twi => {
            if matches!(form, Tw | Twi) {
                for operand in instruction.operands() {
                    read_operand(accesses, *operand);
                }
            }
            exception_entry(
                accesses,
                if form == Sc {
                    AccessOccurrence::Always
                } else {
                    AccessOccurrence::Conditional
                },
            );
        }
        Rfi => {
            for register in [
                SpecialPurposeRegister::SaveRestore0,
                SpecialPurposeRegister::SaveRestore1,
            ] {
                read(
                    accesses,
                    Location::SpecialPurposeRegister(register),
                    AccessOccurrence::Always,
                );
            }
            write(
                accesses,
                Location::Implicit(ImplicitState::MachineState),
                AccessOccurrence::Always,
            );
        }
        Isync | Sync | Eieio => write(
            accesses,
            Location::Implicit(ImplicitState::MemoryOrdering),
            AccessOccurrence::Always,
        ),
        Tlbie => {
            for operand in instruction.operands() {
                read_operand(accesses, *operand);
            }
            write(
                accesses,
                Location::Implicit(ImplicitState::Tlb),
                AccessOccurrence::Always,
            );
        }
        Tlbsync => read_write(accesses, Location::Implicit(ImplicitState::Tlb)),
        _ => {}
    }
}

fn branch_condition_accesses(
    instruction: &Instruction,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    let options = instruction
        .operands()
        .iter()
        .find_map(|operand| match operand {
            Operand::BranchOptions(options) => Some(*options),
            _ => None,
        });
    if let Some(options) = options {
        if options.value() & 0b0_0100 == 0 {
            read_write(
                accesses,
                Location::SpecialPurposeRegister(SpecialPurposeRegister::Count),
            );
        }
        if options.value() & 0b1_0000 == 0 {
            if let Some(bit) = instruction
                .operands()
                .iter()
                .find_map(|operand| match operand {
                    Operand::ConditionRegisterBit(bit) => Some(*bit),
                    _ => None,
                })
            {
                read(
                    accesses,
                    Location::ConditionBit(bit),
                    AccessOccurrence::Always,
                );
            }
        }
    }
}

fn exception_entry(accesses: &mut Vec<LocationAccess<Location>>, occurrence: AccessOccurrence) {
    read(
        accesses,
        Location::Implicit(ImplicitState::MachineState),
        occurrence,
    );
    for register in [
        SpecialPurposeRegister::SaveRestore0,
        SpecialPurposeRegister::SaveRestore1,
    ] {
        write(
            accesses,
            Location::SpecialPurposeRegister(register),
            occurrence,
        );
    }
    write(
        accesses,
        Location::Implicit(ImplicitState::MachineState),
        occurrence,
    );
}

fn modifier_accesses(instruction: &Instruction, accesses: &mut Vec<LocationAccess<Location>>) {
    let modifiers = instruction.modifiers();
    if modifiers.record || fixed_record_form(instruction.form()) {
        let floating = is_floating_form(instruction.form());
        write(
            accesses,
            Location::ConditionField(
                ConditionRegisterField::new(if floating { 1 } else { 0 }).unwrap(),
            ),
            AccessOccurrence::Always,
        );
        if floating {
            read(
                accesses,
                Location::FloatingStatusControl,
                AccessOccurrence::Always,
            );
        } else {
            read(
                accesses,
                Location::IntegerExceptionField(IntegerExceptionField::SummaryOverflow),
                AccessOccurrence::Always,
            );
        }
    }
    if modifiers.overflow {
        write(
            accesses,
            Location::IntegerExceptionField(IntegerExceptionField::Overflow),
            AccessOccurrence::Always,
        );
        read_write(
            accesses,
            Location::IntegerExceptionField(IntegerExceptionField::SummaryOverflow),
        );
    }
    if xer_carry_input_form(instruction.form()) {
        read(
            accesses,
            Location::IntegerExceptionField(IntegerExceptionField::Carry),
            AccessOccurrence::Always,
        );
    }
    if xer_carry_output_form(instruction.form()) {
        write(
            accesses,
            Location::IntegerExceptionField(IntegerExceptionField::Carry),
            AccessOccurrence::Always,
        );
    }
}

fn memory_accesses(
    instruction: &Instruction,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    let form = instruction.form();
    let load = is_load_form(form);
    let address = memory_address(instruction);
    let width = memory_width(instruction);
    address_inputs(address, accesses);
    let memory = Location::Memory(MemoryLocation { address, width });
    let data = instruction
        .operands()
        .first()
        .copied()
        .expect("memory form has a data or address operand");

    if is_cache_form(form) {
        write(
            accesses,
            Location::Implicit(ImplicitState::Cache),
            AccessOccurrence::Always,
        );
        if matches!(form, InstructionForm::Dcbz | InstructionForm::DcbzL) {
            write(accesses, memory, AccessOccurrence::Always);
        }
        add_static_memory_reference(
            references,
            address,
            1,
            ReferenceKind::Address,
            AccessOccurrence::Always,
        );
        return;
    }

    if load {
        read(accesses, memory, AccessOccurrence::Always);
        write_data_operand(accesses, data, width);
    } else {
        read_data_operand(accesses, data, width);
        write(
            accesses,
            memory,
            if form == InstructionForm::Stwcx_ {
                AccessOccurrence::Conditional
            } else {
                AccessOccurrence::Always
            },
        );
    }
    if is_update_form(form) {
        let base = memory_base_operand(instruction);
        write_operand(accesses, base);
    }
    if form == InstructionForm::Lwarx {
        write(
            accesses,
            Location::Implicit(ImplicitState::Reservation),
            AccessOccurrence::Always,
        );
    }
    if form == InstructionForm::Stwcx_ {
        read_write(accesses, Location::Implicit(ImplicitState::Reservation));
        write(
            accesses,
            Location::ConditionField(ConditionRegisterField::new(0).unwrap()),
            AccessOccurrence::Always,
        );
    }
    if is_quantized_form(form) {
        if let Some(register) = instruction
            .operands()
            .iter()
            .find_map(|operand| match operand {
                Operand::QuantizationRegister(register) => Some(*register),
                _ => None,
            })
        {
            read(
                accesses,
                Location::QuantizationRegister(register),
                AccessOccurrence::Always,
            );
        }
    }
    if let Some(byte_len) = fixed_width_bytes(width) {
        add_static_memory_reference(
            references,
            address,
            byte_len,
            if load {
                ReferenceKind::Read
            } else {
                ReferenceKind::Write
            },
            if form == InstructionForm::Stwcx_ {
                AccessOccurrence::Conditional
            } else {
                AccessOccurrence::Always
            },
        );
    } else {
        add_static_memory_reference(
            references,
            address,
            1,
            ReferenceKind::Address,
            AccessOccurrence::Always,
        );
    }
}

fn memory_address(instruction: &Instruction) -> MemoryAddress {
    let operands = instruction.operands();
    if matches!(
        instruction.form(),
        InstructionForm::Lswi | InstructionForm::Stswi
    ) {
        let Operand::GeneralRegister(base) = operands[1] else {
            unreachable!("validated immediate string form has a base register")
        };
        return MemoryAddress::BaseOffset {
            base: (base.value() != 0).then_some(base),
            offset: 0,
        };
    }
    if let Some(offset) = operands.iter().find_map(|operand| match operand {
        Operand::MemoryOffset(offset) | Operand::SignedImmediate(offset) => Some(*offset),
        _ => None,
    }) {
        let base = operands.iter().rev().find_map(|operand| match operand {
            Operand::GeneralRegister(register) => Some(*register),
            _ => None,
        });
        MemoryAddress::BaseOffset {
            base: base.filter(|register| register.value() != 0),
            offset,
        }
    } else {
        let registers = operands
            .iter()
            .filter_map(|operand| match operand {
                Operand::GeneralRegister(register) => Some(*register),
                _ => None,
            })
            .collect::<Vec<_>>();
        let index = *registers.last().expect("indexed memory form has an index");
        let base = registers
            .get(registers.len().saturating_sub(2))
            .copied()
            .filter(|register| register.value() != 0);
        MemoryAddress::BaseIndex { base, index }
    }
}

fn memory_base_operand(instruction: &Instruction) -> Operand {
    match memory_address(instruction) {
        MemoryAddress::BaseOffset {
            base: Some(base), ..
        }
        | MemoryAddress::BaseIndex {
            base: Some(base), ..
        } => Operand::GeneralRegister(base),
        _ => unreachable!("validated update form has a nonzero base"),
    }
}

fn address_inputs(address: MemoryAddress, accesses: &mut Vec<LocationAccess<Location>>) {
    match address {
        MemoryAddress::BaseOffset { base, .. } => {
            if let Some(base) = base {
                read(
                    accesses,
                    Location::GeneralRegister(base),
                    AccessOccurrence::Always,
                );
            }
        }
        MemoryAddress::BaseIndex { base, index } => {
            if let Some(base) = base {
                read(
                    accesses,
                    Location::GeneralRegister(base),
                    AccessOccurrence::Always,
                );
            }
            read(
                accesses,
                Location::GeneralRegister(index),
                AccessOccurrence::Always,
            );
        }
    }
}

fn memory_width(instruction: &Instruction) -> MemoryWidth {
    use InstructionForm::*;
    let form = instruction.form();
    match form {
        Lbz | Lbzu | Lbzux | Lbzx | Stb | Stbu | Stbux | Stbx => MemoryWidth::Known(1),
        Lha | Lhau | Lhaux | Lhax | Lhbrx | Lhz | Lhzu | Lhzux | Lhzx | Sth | Sthbrx | Sthu
        | Sthux | Sthx => MemoryWidth::Known(2),
        Lfs | Lfsu | Lfsux | Lfsx | Lwarx | Lwbrx | Lwz | Lwzu | Lwzux | Lwzx | Stfiwx | Stfs
        | Stfsu | Stfsux | Stfsx | Stw | Stwbrx | Stwcx_ | Stwu | Stwux | Stwx | Eciwx | Ecowx => {
            MemoryWidth::Known(4)
        }
        Lfd | Lfdu | Lfdux | Lfdx | Stfd | Stfdu | Stfdux | Stfdx => MemoryWidth::Known(8),
        Lmw | Stmw => {
            let first = match instruction.operands()[0] {
                Operand::GeneralRegister(register) => register,
                _ => unreachable!(),
            };
            MemoryWidth::Sequence((32 - first.value()) * 4)
        }
        Lswi | Stswi => {
            let bytes = instruction
                .operands()
                .iter()
                .find_map(|operand| match operand {
                    Operand::StringByteCount(count) => Some(count.bytes()),
                    _ => None,
                })
                .unwrap();
            MemoryWidth::Sequence(bytes)
        }
        Lswx | Stswx => MemoryWidth::XerSelectedSequence,
        PsqL | PsqLu | PsqLux | PsqLx | PsqSt | PsqStu | PsqStux | PsqStx => {
            let width = instruction
                .operands()
                .iter()
                .find_map(|operand| match operand {
                    Operand::PairedSingleWidth(width) => Some(*width),
                    _ => None,
                })
                .unwrap();
            MemoryWidth::Quantized(width)
        }
        _ if is_cache_form(form) => MemoryWidth::CacheBlock,
        _ => MemoryWidth::Known(4),
    }
}

fn read_data_operand(
    accesses: &mut Vec<LocationAccess<Location>>,
    operand: Operand,
    width: MemoryWidth,
) {
    match width {
        MemoryWidth::Sequence(bytes) => {
            if let Operand::GeneralRegister(first) = operand {
                read(
                    accesses,
                    Location::GeneralRegisterRange {
                        first,
                        byte_len: bytes,
                    },
                    AccessOccurrence::Always,
                );
            } else {
                read_operand(accesses, operand);
            }
        }
        MemoryWidth::XerSelectedSequence => {
            if let Operand::GeneralRegister(first) = operand {
                read(
                    accesses,
                    Location::GeneralRegisterRangeSelectedByXer { first },
                    AccessOccurrence::Always,
                );
            } else {
                read_operand(accesses, operand);
            }
            read(
                accesses,
                Location::IntegerExceptionField(IntegerExceptionField::StringByteCount),
                AccessOccurrence::Always,
            );
        }
        _ => read_operand(accesses, operand),
    }
}

fn write_data_operand(
    accesses: &mut Vec<LocationAccess<Location>>,
    operand: Operand,
    width: MemoryWidth,
) {
    match width {
        MemoryWidth::Sequence(bytes) => {
            if let Operand::GeneralRegister(first) = operand {
                write(
                    accesses,
                    Location::GeneralRegisterRange {
                        first,
                        byte_len: bytes,
                    },
                    AccessOccurrence::Always,
                );
            } else {
                write_operand(accesses, operand);
            }
        }
        MemoryWidth::XerSelectedSequence => {
            if let Operand::GeneralRegister(first) = operand {
                write(
                    accesses,
                    Location::GeneralRegisterRangeSelectedByXer { first },
                    AccessOccurrence::Always,
                );
            } else {
                write_operand(accesses, operand);
            }
            read(
                accesses,
                Location::IntegerExceptionField(IntegerExceptionField::StringByteCount),
                AccessOccurrence::Always,
            );
        }
        _ => write_operand(accesses, operand),
    }
}

fn read_operand(accesses: &mut Vec<LocationAccess<Location>>, operand: Operand) {
    if let Some(location) = operand_location(operand) {
        read(accesses, location, AccessOccurrence::Always);
    }
}

fn write_operand(accesses: &mut Vec<LocationAccess<Location>>, operand: Operand) {
    if let Some(location) = operand_location(operand) {
        write(accesses, location, AccessOccurrence::Always);
    }
}

fn operand_location(operand: Operand) -> Option<Location> {
    Some(match operand {
        Operand::GeneralRegister(register) => Location::GeneralRegister(register),
        Operand::FloatingRegister(register) => Location::FloatingRegister(register),
        Operand::ConditionRegisterField(field) => Location::ConditionField(field),
        Operand::ConditionRegisterBit(bit) => Location::ConditionBit(bit),
        Operand::SegmentRegister(register) => Location::SegmentRegister(register),
        Operand::SpecialPurposeRegister(register) => Location::SpecialPurposeRegister(register),
        Operand::TimeBaseRegister(register) => Location::TimeBase(register),
        Operand::QuantizationRegister(register) => Location::QuantizationRegister(register),
        Operand::ConditionRegisterMask(_)
        | Operand::FpscrFieldMask(_)
        | Operand::FpscrImmediate(_)
        | Operand::SignedImmediate(_)
        | Operand::UnsignedImmediate(_)
        | Operand::MemoryOffset(_)
        | Operand::BranchTarget(_)
        | Operand::ShiftOrMaskBit(_)
        | Operand::BranchOptions(_)
        | Operand::TrapCondition(_)
        | Operand::StringByteCount(_)
        | Operand::PairedSingleWidth(_) => return None,
    })
}

fn operand_can_be_destination(operand: Operand) -> bool {
    matches!(
        operand,
        Operand::GeneralRegister(_)
            | Operand::FloatingRegister(_)
            | Operand::ConditionRegisterField(_)
            | Operand::ConditionRegisterBit(_)
    )
}

fn is_compare_form(form: InstructionForm) -> bool {
    matches!(
        form,
        InstructionForm::Cmp
            | InstructionForm::Cmpi
            | InstructionForm::Cmpl
            | InstructionForm::Cmpli
            | InstructionForm::Fcmpo
            | InstructionForm::Fcmpu
            | InstructionForm::PsCmpo0
            | InstructionForm::PsCmpo1
            | InstructionForm::PsCmpu0
            | InstructionForm::PsCmpu1
            | InstructionForm::Mcrf
            | InstructionForm::Mcrxr
            | InstructionForm::Crand
            | InstructionForm::Crandc
            | InstructionForm::Creqv
            | InstructionForm::Crnand
            | InstructionForm::Crnor
            | InstructionForm::Cror
            | InstructionForm::Crorc
            | InstructionForm::Crxor
    )
}

fn is_floating_compare_form(form: InstructionForm) -> bool {
    matches!(
        form,
        InstructionForm::Fcmpo
            | InstructionForm::Fcmpu
            | InstructionForm::PsCmpo0
            | InstructionForm::PsCmpo1
            | InstructionForm::PsCmpu0
            | InstructionForm::PsCmpu1
    )
}

fn is_floating_form(form: InstructionForm) -> bool {
    let mnemonic = form.mnemonic();
    mnemonic.starts_with('f')
        || mnemonic.starts_with("ps_")
        || mnemonic.starts_with("mtfs")
        || mnemonic == "mffs"
}

fn updates_floating_status(form: InstructionForm) -> bool {
    use InstructionForm::*;
    matches!(
        form,
        Fadd | Fadds
            | Fcmpo
            | Fcmpu
            | Fctiw
            | Fctiwz
            | Fdiv
            | Fdivs
            | Fmadd
            | Fmadds
            | Fmsub
            | Fmsubs
            | Fmul
            | Fmuls
            | Fnmadd
            | Fnmadds
            | Fnmsub
            | Fnmsubs
            | Fres
            | Frsp
            | Frsqrte
            | Fsub
            | Fsubs
            | PsAdd
            | PsCmpo0
            | PsCmpo1
            | PsCmpu0
            | PsCmpu1
            | PsDiv
            | PsMadd
            | PsMadds0
            | PsMadds1
            | PsMsub
            | PsMul
            | PsMuls0
            | PsMuls1
            | PsNmadd
            | PsNmsub
            | PsRes
            | PsRsqrte
            | PsSub
            | PsSum0
            | PsSum1
    )
}

fn fixed_record_form(form: InstructionForm) -> bool {
    matches!(
        form,
        InstructionForm::Addic_
            | InstructionForm::Andi_
            | InstructionForm::Andis_
            | InstructionForm::Stwcx_
    )
}

fn is_memory_form(form: InstructionForm) -> bool {
    let mnemonic = form.mnemonic();
    mnemonic.starts_with('l')
        || mnemonic.starts_with("st")
        || mnemonic.starts_with("psq_l")
        || mnemonic.starts_with("psq_st")
        || is_cache_form(form)
        || matches!(form, InstructionForm::Eciwx | InstructionForm::Ecowx)
}

fn is_load_form(form: InstructionForm) -> bool {
    let mnemonic = form.mnemonic();
    mnemonic.starts_with('l') || mnemonic.starts_with("psq_l") || form == InstructionForm::Eciwx
}

fn is_cache_form(form: InstructionForm) -> bool {
    matches!(
        form,
        InstructionForm::Dcbf
            | InstructionForm::Dcbi
            | InstructionForm::Dcbst
            | InstructionForm::Dcbt
            | InstructionForm::Dcbtst
            | InstructionForm::Dcbz
            | InstructionForm::DcbzL
            | InstructionForm::Icbi
    )
}

fn is_update_form(form: InstructionForm) -> bool {
    let mnemonic = form.mnemonic();
    mnemonic.ends_with('u') || mnemonic.ends_with("ux")
}

fn is_quantized_form(form: InstructionForm) -> bool {
    form.mnemonic().starts_with("psq_")
}

fn xer_carry_input_form(form: InstructionForm) -> bool {
    matches!(
        form,
        InstructionForm::Adde
            | InstructionForm::Addme
            | InstructionForm::Addze
            | InstructionForm::Subfe
            | InstructionForm::Subfme
            | InstructionForm::Subfze
    )
}

fn xer_carry_output_form(form: InstructionForm) -> bool {
    matches!(
        form,
        InstructionForm::Subfic
            | InstructionForm::Addic
            | InstructionForm::Addic_
            | InstructionForm::Addc
            | InstructionForm::Adde
            | InstructionForm::Addme
            | InstructionForm::Addze
            | InstructionForm::Subfc
            | InstructionForm::Subfe
            | InstructionForm::Subfme
            | InstructionForm::Subfze
            | InstructionForm::Sraw
            | InstructionForm::Srawi
    )
}

fn fixed_width_bytes(width: MemoryWidth) -> Option<u8> {
    match width {
        MemoryWidth::Known(bytes) | MemoryWidth::Sequence(bytes) => Some(bytes),
        MemoryWidth::Quantized(_) | MemoryWidth::XerSelectedSequence | MemoryWidth::CacheBlock => {
            None
        }
    }
}

fn add_static_memory_reference(
    references: &mut Vec<StaticReference<Reference>>,
    address: MemoryAddress,
    byte_len: u8,
    kind: ReferenceKind,
    occurrence: AccessOccurrence,
) {
    let MemoryAddress::BaseOffset { base: None, offset } = address else {
        return;
    };
    let address = offset as i32 as u32;
    add_reference(
        references,
        StaticReference {
            target: if kind == ReferenceKind::Address {
                Reference::Address(address)
            } else {
                Reference::Memory { address, byte_len }
            },
            kind,
            occurrence,
        },
    );
}

fn read_write(accesses: &mut Vec<LocationAccess<Location>>, location: Location) {
    read(accesses, location, AccessOccurrence::Always);
    write(accesses, location, AccessOccurrence::Always);
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
        (Location::ConditionRegister, Location::ConditionField(_) | Location::ConditionBit(_))
        | (Location::ConditionField(_) | Location::ConditionBit(_), Location::ConditionRegister) => {
            Overlapping
        }
        (Location::ConditionField(field), Location::ConditionBit(bit))
        | (Location::ConditionBit(bit), Location::ConditionField(field)) => {
            if bit.value() / 4 == field.value() {
                Overlapping
            } else {
                Disjoint
            }
        }
        (
            Location::GeneralRegister(register),
            Location::GeneralRegisterRange { first, byte_len },
        )
        | (
            Location::GeneralRegisterRange { first, byte_len },
            Location::GeneralRegister(register),
        ) => {
            if register_in_range(*register, *first, *byte_len) {
                Overlapping
            } else {
                Disjoint
            }
        }
        (Location::GeneralRegister(_), Location::GeneralRegisterRangeSelectedByXer { .. })
        | (Location::GeneralRegisterRangeSelectedByXer { .. }, Location::GeneralRegister(_)) => {
            Overlapping
        }
        (
            Location::GeneralRegisterRange {
                first: left,
                byte_len: left_len,
            },
            Location::GeneralRegisterRange {
                first: right,
                byte_len: right_len,
            },
        ) => {
            if ranges_share_register(*left, *left_len, *right, *right_len) {
                Overlapping
            } else {
                Disjoint
            }
        }
        (
            Location::GeneralRegisterRangeSelectedByXer { .. },
            Location::GeneralRegisterRange { .. },
        )
        | (
            Location::GeneralRegisterRange { .. },
            Location::GeneralRegisterRangeSelectedByXer { .. },
        )
        | (
            Location::GeneralRegisterRangeSelectedByXer { .. },
            Location::GeneralRegisterRangeSelectedByXer { .. },
        ) => Overlapping,
        (Location::FloatingStatusControl, Location::FloatingStatusField(_))
        | (Location::FloatingStatusField(_), Location::FloatingStatusControl) => Overlapping,
        (
            Location::SpecialPurposeRegister(SpecialPurposeRegister::Xer),
            Location::IntegerExceptionField(_),
        )
        | (
            Location::IntegerExceptionField(_),
            Location::SpecialPurposeRegister(SpecialPurposeRegister::Xer),
        ) => Overlapping,
        (Location::Implicit(ImplicitState::SegmentRegisters), Location::SegmentRegister(_))
        | (Location::SegmentRegister(_), Location::Implicit(ImplicitState::SegmentRegisters)) => {
            Overlapping
        }
        (Location::QuantizationRegister(left), Location::SpecialPurposeRegister(right))
        | (Location::SpecialPurposeRegister(right), Location::QuantizationRegister(left)) => {
            if quantization_spr(*left) == *right {
                Overlapping
            } else {
                Disjoint
            }
        }
        (Location::TimeBase(left), Location::SpecialPurposeRegister(right))
        | (Location::SpecialPurposeRegister(right), Location::TimeBase(left)) => {
            if time_base_spr_matches(*left, *right) {
                Overlapping
            } else {
                Disjoint
            }
        }
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

fn quantization_spr(register: QuantizationRegister) -> SpecialPurposeRegister {
    match register.value() {
        0 => SpecialPurposeRegister::GraphicsQuantization0,
        1 => SpecialPurposeRegister::GraphicsQuantization1,
        2 => SpecialPurposeRegister::GraphicsQuantization2,
        3 => SpecialPurposeRegister::GraphicsQuantization3,
        4 => SpecialPurposeRegister::GraphicsQuantization4,
        5 => SpecialPurposeRegister::GraphicsQuantization5,
        6 => SpecialPurposeRegister::GraphicsQuantization6,
        7 => SpecialPurposeRegister::GraphicsQuantization7,
        _ => unreachable!("quantization register is bounded"),
    }
}

fn time_base_spr_matches(register: TimeBaseRegister, special: SpecialPurposeRegister) -> bool {
    matches!(
        (register, special),
        (
            TimeBaseRegister::Lower,
            SpecialPurposeRegister::TimeBaseLowerRead | SpecialPurposeRegister::TimeBaseLowerWrite
        ) | (
            TimeBaseRegister::Upper,
            SpecialPurposeRegister::TimeBaseUpperRead | SpecialPurposeRegister::TimeBaseUpperWrite
        )
    )
}

fn register_in_range(register: GeneralRegister, first: GeneralRegister, byte_len: u8) -> bool {
    let count = byte_len.div_ceil(4).min(32);
    (register.value() + 32 - first.value()) % 32 < count
}

fn ranges_share_register(
    left: GeneralRegister,
    left_len: u8,
    right: GeneralRegister,
    right_len: u8,
) -> bool {
    (0..32).any(|index| {
        let register = GeneralRegister::new(index).unwrap();
        register_in_range(register, left, left_len) && register_in_range(register, right, right_len)
    })
}
