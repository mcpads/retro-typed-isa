use m68000::{
    AddressRegister, AddressRegisterLocation, AnalysisPoint, Boundary, BranchDisplacement,
    BranchKind, CodeLocation, ConditionCode, DataRegister, EffectiveAddress, Instruction, Location,
    Mc68000, MemoryAddress, MovePeripheralDirection, OperandSize, PrivilegeMode, Reference,
};
use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, LocationAccess, ReferenceKind, StaticReference,
    StaticSemantics, StorageRelation,
};

// Normative source: Motorola, M68000 8-/16-/32-Bit Microprocessors User's
// Manual, Ninth Edition, sections 2, 3, 6, and 8. These assertions transcribe
// individual architectural effects rather than production implementation data.
//
// https://www.nxp.com/docs/en/reference-manual/MC68000UM.pdf

fn point(privilege: PrivilegeMode) -> AnalysisPoint {
    AnalysisPoint::new(CodeLocation::new(0x1000), privilege)
}

fn has_access(
    accesses: &[LocationAccess<Location>],
    location: Location,
    kind: AccessKind,
    occurrence: AccessOccurrence,
) -> bool {
    accesses.contains(&LocationAccess {
        location,
        kind,
        occurrence,
    })
}

#[test]
fn branches_use_the_word_after_the_opcode_as_the_displacement_base() {
    let branch = Instruction::Branch {
        kind: BranchKind::Always,
        displacement: BranchDisplacement::Word(-4),
    };
    let semantics = Mc68000::semantics(&branch, &point(PrivilegeMode::Supervisor)).unwrap();
    assert!(matches!(
        semantics.control_flow.action,
        ControlAction::Transfer {
            target: typed_isa_core::ControlTarget::Direct(target)
        } if target.location == CodeLocation::new(0x0ffe)
    ));
    assert_eq!(semantics.control_flow.fallthrough, None);
}

#[test]
fn a7_selects_physical_usp_or_ssp_and_unknown_mode_preserves_alias_uncertainty() {
    let instruction = Instruction::Move {
        size: OperandSize::Long,
        source: EffectiveAddress::AddressPostincrement(AddressRegister::A7),
        destination: EffectiveAddress::DataRegister(DataRegister::D0),
    };
    let user = Mc68000::semantics(&instruction, &point(PrivilegeMode::User)).unwrap();
    assert!(has_access(
        &user.location_accesses,
        Location::AddressRegister(AddressRegisterLocation::UserStackPointer),
        AccessKind::Write,
        AccessOccurrence::Always,
    ));
    let supervisor = Mc68000::semantics(&instruction, &point(PrivilegeMode::Supervisor)).unwrap();
    assert!(has_access(
        &supervisor.location_accesses,
        Location::AddressRegister(AddressRegisterLocation::SupervisorStackPointer),
        AccessKind::Write,
        AccessOccurrence::Always,
    ));
    assert_eq!(
        Mc68000::storage_relation(
            &Location::AddressRegister(AddressRegisterLocation::ActiveStackPointerUnknown),
            &Location::AddressRegister(AddressRegisterLocation::UserStackPointer),
        ),
        StorageRelation::Unknown,
    );
}

#[test]
fn partial_data_register_views_overlap_without_becoming_identical() {
    let byte = Location::DataRegister {
        register: DataRegister::D3,
        byte_len: 1,
    };
    let word = Location::DataRegister {
        register: DataRegister::D3,
        byte_len: 2,
    };
    let other = Location::DataRegister {
        register: DataRegister::D4,
        byte_len: 1,
    };
    assert_eq!(
        Mc68000::storage_relation(&byte, &word),
        StorageRelation::Overlapping
    );
    assert_eq!(
        Mc68000::storage_relation(&byte, &other),
        StorageRelation::Disjoint
    );
}

#[test]
fn absolute_reads_and_lea_addresses_are_distinct_static_reference_kinds() {
    let read = Instruction::Move {
        size: OperandSize::Word,
        source: EffectiveAddress::AbsoluteLong(0x1234_5678),
        destination: EffectiveAddress::DataRegister(DataRegister::D0),
    };
    let read_semantics = Mc68000::semantics(&read, &point(PrivilegeMode::Supervisor)).unwrap();
    assert!(read_semantics.static_references.contains(&StaticReference {
        target: Reference::Memory {
            address: 0x1234_5678,
            byte_len: 2,
        },
        kind: ReferenceKind::Read,
        occurrence: AccessOccurrence::Always,
    }));

    let lea = Instruction::LoadEffectiveAddress {
        source: EffectiveAddress::AbsoluteLong(0x1234_5678),
        destination: AddressRegister::A0,
    };
    let lea_semantics = Mc68000::semantics(&lea, &point(PrivilegeMode::Supervisor)).unwrap();
    assert!(lea_semantics.static_references.contains(&StaticReference {
        target: Reference::Address(0x1234_5678),
        kind: ReferenceKind::Address,
        occurrence: AccessOccurrence::Always,
    }));
}

#[test]
fn movep_exposes_noncontiguous_even_addressed_bytes() {
    let instruction = Instruction::MovePeripheral {
        size: m68000::AddressSize::Long,
        direction: MovePeripheralDirection::RegisterToMemory,
        data_register: DataRegister::D2,
        address_register: AddressRegister::A1,
        displacement: 8,
    };
    let semantics = Mc68000::semantics(&instruction, &point(PrivilegeMode::Supervisor)).unwrap();
    assert!(semantics.location_accesses.iter().any(|access| {
        matches!(
            access,
            LocationAccess {
                location: Location::Memory(memory),
                kind: AccessKind::Write,
                occurrence: AccessOccurrence::Always,
            } if memory.address == MemoryAddress::Displacement {
                base: AddressRegisterLocation::Register(AddressRegister::A1),
                displacement: 8,
            } && memory.element_byte_len == 1 && memory.element_count == 4 && memory.stride == 2
        )
    }));
}

#[test]
fn trapv_has_conditional_exception_storage_and_a_normal_fallthrough() {
    let semantics = Mc68000::semantics(
        &Instruction::TrapOnOverflow,
        &point(PrivilegeMode::Supervisor),
    )
    .unwrap();
    assert!(matches!(
        semantics.control_flow.action,
        ControlAction::Boundary(typed_isa_core::ControlBoundary::Trap {
            kind: Boundary::TrapOnOverflow,
            ..
        })
    ));
    assert_eq!(
        semantics.control_flow.fallthrough.unwrap().location,
        CodeLocation::new(0x1002),
    );
    assert!(has_access(
        &semantics.location_accesses,
        Location::ConditionCode(ConditionCode::Overflow),
        AccessKind::Read,
        AccessOccurrence::Always,
    ));
    assert!(semantics.location_accesses.iter().any(|access| {
        access.kind == AccessKind::Write
            && access.occurrence == AccessOccurrence::Conditional
            && matches!(access.location, Location::Memory(_))
    }));
}
