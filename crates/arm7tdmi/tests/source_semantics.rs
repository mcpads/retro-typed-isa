use arm7tdmi::{
    AnalysisPoint, Arm7TdmiArm, Arm7TdmiThumb, Boundary, Flag, IndirectTarget, Location,
    MemoryAddress, MemoryLocation, MemoryWidth, ProcessorMode, Reference, Register, RegisterBank,
    RegisterLocation, StatusLocation, decode_arm_bytes, decode_thumb_bytes,
};
use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlBoundary, ControlTarget, ReferenceKind,
    StaticSemantics, StorageRelation,
};

fn arm(word: u32, location: u32, mode: ProcessorMode) -> arm7tdmi::InstructionSemantics {
    let instruction = decode_arm_bytes(&word.to_le_bytes()).unwrap();
    Arm7TdmiArm::semantics(&instruction, &AnalysisPoint::arm(location, mode)).unwrap()
}

fn thumb(word: u16, location: u32, mode: ProcessorMode) -> arm7tdmi::InstructionSemantics {
    let instruction = decode_thumb_bytes(&word.to_le_bytes()).unwrap();
    Arm7TdmiThumb::semantics(&instruction, &AnalysisPoint::thumb(location, mode)).unwrap()
}

#[test]
fn ddi_0210c_pc_relative_load_exposes_the_resolved_external_reference() {
    let semantics = arm(0xe59f_0004, 0x1000, ProcessorMode::System);
    assert!(semantics.location_accesses.iter().any(|access| {
        access.kind == AccessKind::Read
            && access.location
                == Location::Memory(MemoryLocation {
                    address: MemoryAddress::Literal(0x100c),
                    width: MemoryWidth::Known(4),
                })
    }));
    assert!(semantics.static_references.iter().any(|reference| {
        reference.target
            == Reference::Memory {
                address: 0x100c,
                byte_len: 4,
            }
            && reference.kind == ReferenceKind::Read
            && reference.occurrence == AccessOccurrence::Always
    }));
}

#[test]
fn ddi_0210c_predication_guards_effects_but_not_condition_evaluation() {
    let semantics = arm(0x1591_0004, 0x1000, ProcessorMode::System);
    assert!(semantics.location_accesses.iter().any(|access| {
        access.location == Location::Flag(Flag::Zero)
            && access.kind == AccessKind::Read
            && access.occurrence == AccessOccurrence::Always
    }));
    assert!(semantics.location_accesses.iter().any(|access| {
        access.location == Location::Register(RegisterLocation::Unbanked(Register::new(0).unwrap()))
            && access.kind == AccessKind::Write
            && access.occurrence == AccessOccurrence::Conditional
    }));
}

#[test]
fn ddi_0210c_mode_context_selects_the_physical_stack_pointer_bank() {
    let semantics = arm(0xe1a0_d000, 0x1000, ProcessorMode::Fiq);
    assert!(semantics.location_accesses.iter().any(|access| {
        access.location
            == Location::Register(RegisterLocation::Banked {
                register: Register::SP,
                bank: RegisterBank::Fiq,
            })
            && access.kind == AccessKind::Write
    }));
}

#[test]
fn ddi_0210c_software_interrupt_records_the_boundary_and_exception_bank_writes() {
    let semantics = arm(0xef00_0001, 0x1000, ProcessorMode::System);
    assert!(matches!(
        semantics.control_flow.action,
        ControlAction::Boundary(ControlBoundary::Trap {
            kind: Boundary::SoftwareInterrupt(1),
            target: Some(_),
            resume: Some(_),
        })
    ));
    assert!(semantics.location_accesses.iter().any(|access| {
        access.location == Location::Status(StatusLocation::Saved(ProcessorMode::Supervisor))
            && access.kind == AccessKind::Write
    }));
    assert!(semantics.location_accesses.iter().any(|access| {
        access.location
            == Location::Register(RegisterLocation::Banked {
                register: Register::LINK,
                bank: RegisterBank::Supervisor,
            })
            && access.kind == AccessKind::Write
    }));
}

#[test]
fn ddi_0210c_thumb_offsets_are_not_scaled_twice() {
    let semantics = thumb(0x6848, 0x1000, ProcessorMode::System);
    assert!(semantics.location_accesses.iter().any(|access| {
        matches!(
            access.location,
            Location::Memory(MemoryLocation {
                address: MemoryAddress::ThumbImmediateOffset { offset: 4, .. },
                width: MemoryWidth::Known(4),
            })
        )
    }));
}

#[test]
fn ddi_0210c_thumb_pop_pc_is_an_indirect_transfer_through_the_loaded_program_counter() {
    let semantics = thumb(0xbd01, 0x1000, ProcessorMode::System);
    assert!(matches!(
        semantics.control_flow.action,
        ControlAction::Transfer {
            target: ControlTarget::Indirect(IndirectTarget::WrittenProgramCounter)
        }
    ));
}

#[test]
fn status_fields_overlap_only_their_containing_physical_status_register() {
    let saved = Location::Status(StatusLocation::Saved(ProcessorMode::Irq));
    let saved_flags = Location::StatusField {
        status: StatusLocation::Saved(ProcessorMode::Irq),
        field: arm7tdmi::StatusField::Flags,
    };
    let supervisor_flags = Location::StatusField {
        status: StatusLocation::Saved(ProcessorMode::Supervisor),
        field: arm7tdmi::StatusField::Flags,
    };
    assert_eq!(
        Arm7TdmiArm::storage_relation(&saved, &saved_flags),
        StorageRelation::Overlapping
    );
    assert_eq!(
        Arm7TdmiArm::storage_relation(&saved, &supervisor_flags),
        StorageRelation::Disjoint
    );
}
