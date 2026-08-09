use i386::{
    AddressSize, AnalysisPoint, Boundary, CodeLocation, CodeMode, GeneralRegister, I386Code16,
    I386Code32, InterruptState, Location, MemoryExtent, MemoryWidth, OffsetAddress, Reference,
    Register8, Register16, Register32, SegmentRegister, SemanticError, decode_bytes,
};
use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlBoundary, ControlTarget, StaticSemantics,
    StorageRelation,
};

fn point(offset: u32, stack_address_size: AddressSize) -> AnalysisPoint {
    AnalysisPoint::new(
        CodeLocation::new(8, offset),
        InterruptState::Enabled,
        stack_address_size,
    )
}

fn has_access(
    semantics: &i386::InstructionSemantics,
    location: Location,
    kind: AccessKind,
    occurrence: AccessOccurrence,
) -> bool {
    semantics.location_accesses.iter().any(|access| {
        access.location == location && access.kind == kind && access.occurrence == occurrence
    })
}

#[test]
fn near_conditions_resolve_a_segmented_target_and_keep_fallthrough() {
    let instruction = decode_bytes(&[0x75, 0x02], CodeMode::Code16)
        .unwrap()
        .into_instruction();
    let semantics =
        I386Code16::semantics(&instruction, &point(0x1000, AddressSize::Bits16)).unwrap();

    assert_eq!(
        semantics.control_flow.fallthrough.unwrap().location,
        CodeLocation::new(8, 0x1002)
    );
    assert_eq!(
        semantics.control_flow.action,
        ControlAction::Transfer {
            target: ControlTarget::Direct(point(0x1004, AddressSize::Bits16)),
        }
    );
    assert!(semantics.static_references.is_empty());
}

#[test]
fn code16_successors_and_short_targets_wrap_at_sixteen_bits() {
    let nop = decode_bytes(&[0x90], CodeMode::Code16)
        .unwrap()
        .into_instruction();
    let nop = I386Code16::semantics(&nop, &point(0xffff, AddressSize::Bits16)).unwrap();
    assert_eq!(
        nop.control_flow.fallthrough.unwrap().location,
        CodeLocation::new(8, 0)
    );

    let jump = decode_bytes(&[0xeb, 0], CodeMode::Code16)
        .unwrap()
        .into_instruction();
    let jump = I386Code16::semantics(&jump, &point(0xffff, AddressSize::Bits16)).unwrap();
    assert_eq!(
        jump.control_flow.action,
        ControlAction::Transfer {
            target: ControlTarget::Direct(point(1, AddressSize::Bits16)),
        }
    );
}

#[test]
fn stack_address_size_is_independent_of_the_code_profile() {
    let instruction = decode_bytes(&[0x50], CodeMode::Code16)
        .unwrap()
        .into_instruction();
    let semantics = I386Code16::semantics(&instruction, &point(0, AddressSize::Bits32)).unwrap();

    assert!(has_access(
        &semantics,
        Location::GeneralRegister(GeneralRegister::Doubleword(Register32::Esp)),
        AccessKind::Write,
        AccessOccurrence::Always,
    ));
    assert!(!has_access(
        &semantics,
        Location::GeneralRegister(GeneralRegister::Word(Register16::Sp)),
        AccessKind::Write,
        AccessOccurrence::Always,
    ));
}

#[test]
fn subregister_storage_relations_follow_the_80386_register_file() {
    let al = Location::GeneralRegister(GeneralRegister::Byte(Register8::Al));
    let ah = Location::GeneralRegister(GeneralRegister::Byte(Register8::Ah));
    let ax = Location::GeneralRegister(GeneralRegister::Word(Register16::Ax));
    let eax = Location::GeneralRegister(GeneralRegister::Doubleword(Register32::Eax));
    let ecx = Location::GeneralRegister(GeneralRegister::Doubleword(Register32::Ecx));

    assert_eq!(
        I386Code32::storage_relation(&al, &ah),
        StorageRelation::Disjoint
    );
    assert_eq!(
        I386Code32::storage_relation(&al, &ax),
        StorageRelation::Overlapping
    );
    assert_eq!(
        I386Code32::storage_relation(&ah, &eax),
        StorageRelation::Overlapping
    );
    assert_eq!(
        I386Code32::storage_relation(&eax, &ecx),
        StorageRelation::Disjoint
    );
}

#[test]
fn divide_fault_restarts_the_instruction_but_software_interrupt_resumes_after_it() {
    let divide = decode_bytes(&[0xf7, 0xf0], CodeMode::Code16)
        .unwrap()
        .into_instruction();
    let divide = I386Code16::semantics(&divide, &point(0x2000, AddressSize::Bits16)).unwrap();
    assert_eq!(
        divide.control_flow.action,
        ControlAction::Boundary(ControlBoundary::Trap {
            kind: Boundary::DivideError,
            target: Some(typed_isa_core::ControlTarget::Indirect(
                i386::IndirectTarget::InterruptVector(0),
            )),
            resume: Some(point(0x2000, AddressSize::Bits16)),
        })
    );

    let interrupt = decode_bytes(&[0xcd, 0x21], CodeMode::Code16)
        .unwrap()
        .into_instruction();
    let interrupt = I386Code16::semantics(&interrupt, &point(0x2000, AddressSize::Bits16)).unwrap();
    assert!(matches!(
        interrupt.control_flow.action,
        ControlAction::Boundary(ControlBoundary::Trap {
            kind: Boundary::Interrupt(0x21),
            resume: Some(AnalysisPoint {
                location,
                ..
            }),
            ..
        }) if location == CodeLocation::new(8, 0x2002)
    ));
    assert!(interrupt.location_accesses.iter().any(|access| {
        matches!(
            access.location,
            Location::Memory(i386::MemoryLocation {
                extent: MemoryExtent::KnownBytes(6),
                ..
            })
        ) && access.kind == AccessKind::Write
    }));
}

#[test]
fn direct_segment_offset_is_a_typed_memory_reference() {
    let instruction = decode_bytes(&[0xa1, 0x34, 0x12], CodeMode::Code16)
        .unwrap()
        .into_instruction();
    let semantics = I386Code16::semantics(&instruction, &point(0, AddressSize::Bits16)).unwrap();

    assert!(semantics.static_references.iter().any(|reference| {
        reference.target
            == Reference::Memory {
                segment: i386::SegmentSelection::Register(SegmentRegister::Ds),
                offset: 0x1234,
                width: MemoryWidth::Word,
            }
    }));
}

#[test]
fn repeated_string_accesses_use_address_size_and_are_conditional() {
    let instruction = decode_bytes(&[0xf3, 0xa5], CodeMode::Code32)
        .unwrap()
        .into_instruction();
    let semantics = I386Code32::semantics(&instruction, &point(0, AddressSize::Bits32)).unwrap();

    assert!(semantics.location_accesses.iter().any(|access| {
        matches!(
            access.location,
            Location::Memory(i386::MemoryLocation {
                offset: OffsetAddress::StringSource(AddressSize::Bits32),
                ..
            })
        ) && access.kind == AccessKind::Read
            && access.occurrence == AccessOccurrence::Conditional
    }));
    assert!(has_access(
        &semantics,
        Location::GeneralRegister(GeneralRegister::Doubleword(Register32::Ecx)),
        AccessKind::Write,
        AccessOccurrence::Conditional,
    ));
}

#[test]
fn code_size_profile_mismatch_fails_before_semantics_are_returned() {
    let instruction = decode_bytes(&[0xb8, 1, 0, 0, 0], CodeMode::Code32)
        .unwrap()
        .into_instruction();
    assert!(matches!(
        I386Code16::semantics(&instruction, &point(0, AddressSize::Bits16)),
        Err(SemanticError::Encode(
            i386::EncodeError::CodeModeMismatch { .. }
        ))
    ));
}
