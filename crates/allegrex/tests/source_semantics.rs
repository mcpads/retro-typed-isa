use allegrex::{
    Allegrex, AnalysisPoint, FpuState, GeneralRegister, ImplicitState, IndirectTarget, Location,
    MemoryLocation, MemoryWidth, Reference, SystemControlRegister, VfpuConditionBit,
    VfpuControlRegister, decode_word,
};
use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlBoundary, DelaySlotExecution,
    ReferenceKind, StaticSemantics, StorageRelation,
};

const ADDRESS: u32 = 0x0880_0000;

fn semantics(word: u32) -> allegrex::InstructionSemantics {
    let instruction = decode_word(word, ADDRESS).unwrap();
    Allegrex::semantics(&instruction, &AnalysisPoint::new(ADDRESS)).unwrap()
}

#[test]
fn sony_likely_branch_annuls_only_the_not_taken_delay_slot() {
    let semantics = semantics(0x5000_0001); // beql $zero, $zero, ADDRESS + 8
    assert_eq!(
        semantics.control_flow.delay_slot.unwrap().execution,
        DelaySlotExecution::WhenTransferOccurs
    );
    assert_eq!(
        semantics.control_flow.fallthrough,
        Some(AnalysisPoint::new(ADDRESS + 8))
    );
    assert!(semantics.static_references.is_empty());
}

#[test]
fn sony_linked_jump_returns_after_the_delay_slot() {
    let semantics = semantics(0x0e20_0008); // jal 0x0880_0020
    assert!(matches!(
        semantics.control_flow.action,
        ControlAction::LinkedTransfer {
            return_site: AnalysisPoint {
                location: 0x0880_0008
            },
            ..
        }
    ));
}

#[test]
fn jump_through_ra_remains_an_indirect_transfer() {
    let semantics = semantics(0x03e0_0008); // jr $ra
    assert_eq!(
        semantics.control_flow.action,
        ControlAction::Transfer {
            target: typed_isa_core::ControlTarget::Indirect(IndirectTarget::Register(
                GeneralRegister::RA,
            )),
        }
    );
    assert!(semantics.static_references.is_empty());
}

#[test]
fn sony_merge_load_and_conditional_store_preserve_hidden_inputs() {
    let load = semantics(0x8881_0001); // lwl $at, 1($a0)
    assert!(load.location_accesses.iter().any(|access| {
        access.location == Location::GeneralRegister(GeneralRegister::AT)
            && access.kind == AccessKind::Read
    }));
    assert!(load.location_accesses.iter().any(|access| {
        matches!(
            access.location,
            Location::Memory(MemoryLocation {
                width: MemoryWidth::Merge(4),
                ..
            })
        )
    }));

    let store = semantics(0xe081_0000); // sc $at, 0($a0)
    for kind in [AccessKind::Read, AccessKind::Write] {
        assert!(store.location_accesses.iter().any(|access| {
            access.location == Location::Implicit(ImplicitState::LoadLinkedReservation)
                && access.kind == kind
        }));
    }
    assert!(store.location_accesses.iter().any(|access| {
        matches!(access.location, Location::Memory(_))
            && access.kind == AccessKind::Write
            && access.occurrence == AccessOccurrence::Conditional
    }));
}

#[test]
fn sony_zero_based_load_exposes_a_static_memory_reference() {
    let semantics = semantics(0x8c01_0020); // lw $at, 0x20($zero)
    assert!(semantics.static_references.iter().any(|reference| {
        reference.target
            == Reference::Memory {
                address: 0x20,
                byte_len: 4,
            }
            && reference.kind == ReferenceKind::Read
            && reference.occurrence == AccessOccurrence::Always
    }));
}

#[test]
fn sony_trap_reads_status_and_does_not_invent_a_resume_address() {
    let semantics = semantics(0x0000_004c); // syscall 1
    assert!(matches!(
        semantics.control_flow.action,
        ControlAction::Boundary(ControlBoundary::Trap { resume: None, .. })
    ));
    let status = Location::SystemControl(SystemControlRegister::new(12).unwrap());
    for kind in [AccessKind::Read, AccessKind::Write] {
        assert!(
            semantics
                .location_accesses
                .iter()
                .any(|access| access.location == status && access.kind == kind)
        );
    }
}

#[test]
fn sony_fpu_compare_updates_condition_and_possible_exception_state() {
    let semantics = semantics(0x4600_0032); // c.eq.s $f0, $f0
    assert!(semantics.location_accesses.iter().any(|access| {
        access.location == Location::FpuState(FpuState::Condition)
            && access.kind == AccessKind::Write
            && access.occurrence == AccessOccurrence::Always
    }));
    assert!(semantics.location_accesses.iter().any(|access| {
        access.location == Location::FpuState(FpuState::ExceptionState)
            && access.kind == AccessKind::Write
            && access.occurrence == AccessOccurrence::Conditional
    }));
}

#[test]
fn sony_vfpu_condition_bits_and_whole_register_preserve_overlap() {
    let semantics = semantics(0x4900_0001); // bvf 0, ADDRESS + 8
    let bit = Location::VfpuCondition(VfpuConditionBit::new(0).unwrap());
    assert!(semantics.location_accesses.iter().any(|access| {
        access.location == bit
            && access.kind == AccessKind::Read
            && access.occurrence == AccessOccurrence::Always
    }));
    assert_eq!(
        Allegrex::storage_relation(&Location::VfpuControl(VfpuControlRegister::Condition), &bit,),
        StorageRelation::Overlapping
    );
}

#[test]
fn sony_vfpu_arithmetic_exposes_prefix_inputs_without_claiming_physical_aliases() {
    let semantics = semantics(0x6000_0080); // vadd.p C000, C000, C000
    for control in [
        VfpuControlRegister::SourcePrefix,
        VfpuControlRegister::TargetPrefix,
        VfpuControlRegister::DestinationPrefix,
    ] {
        assert!(semantics.location_accesses.iter().any(|access| {
            access.location == Location::VfpuControl(control) && access.kind == AccessKind::Read
        }));
    }
    let views: Vec<_> = semantics
        .location_accesses
        .iter()
        .filter_map(|access| {
            matches!(access.location, Location::VfpuVector { .. }).then_some(access.location)
        })
        .collect();
    assert!(!views.is_empty());
    assert_eq!(
        Allegrex::storage_relation(&views[0], &views[0]),
        StorageRelation::Same
    );
}
