use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlBoundary, DelaySlotExecution,
    ReferenceKind, StaticSemantics, StorageRelation,
};
use vr4300::{
    AnalysisPoint, Boundary, FloatingBinaryOperation, FloatingFormat, FloatingRegister, FpuState,
    GeneralRegister, ImplicitState, IndirectTarget, Instruction, Location, MemoryLocation,
    MemoryOperation, MemoryWidth, Reference, Vr4300Fr0, Vr4300Fr1,
};

const ADDRESS: u64 = 0x8000_0000;

#[test]
fn nec_branch_likely_annuls_only_the_not_taken_delay_slot() {
    let instruction = Instruction::EqualityBranch {
        condition: vr4300::EqualityCondition::Equal,
        likely: true,
        left: GeneralRegister::AT,
        right: GeneralRegister::ZERO,
        target: ADDRESS + 0x20,
    };
    let semantics = Vr4300Fr1::semantics(&instruction, &AnalysisPoint::new(ADDRESS)).unwrap();
    assert_eq!(
        semantics.control_flow.delay_slot.unwrap().execution,
        DelaySlotExecution::WhenTransferOccurs
    );
    assert_eq!(
        semantics.control_flow.fallthrough.unwrap().location,
        ADDRESS + 8
    );
}

#[test]
fn jump_through_ra_remains_an_indirect_transfer() {
    let instruction = Instruction::JumpRegister {
        target: GeneralRegister::RA,
        link: None,
    };
    let semantics = Vr4300Fr1::semantics(&instruction, &AnalysisPoint::new(ADDRESS)).unwrap();
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
fn nec_fr_zero_pairs_overlap_where_fr_one_registers_are_disjoint() {
    let f0 = FloatingRegister::new(0).unwrap();
    let f1 = FloatingRegister::new(1).unwrap();
    let double_f0 = Instruction::MoveFromFpu {
        doubleword: true,
        destination: GeneralRegister::AT,
        source: f0,
    };
    let word_f1 = Instruction::MoveFromFpu {
        doubleword: false,
        destination: GeneralRegister::AT,
        source: f1,
    };

    let fr0_double = Vr4300Fr0::semantics(&double_f0, &AnalysisPoint::new(ADDRESS))
        .unwrap()
        .location_accesses
        .into_iter()
        .find(|access| matches!(access.location, Location::Floating(_)))
        .unwrap()
        .location;
    let fr0_word = Vr4300Fr0::semantics(&word_f1, &AnalysisPoint::new(ADDRESS))
        .unwrap()
        .location_accesses
        .into_iter()
        .find(|access| matches!(access.location, Location::Floating(_)))
        .unwrap()
        .location;
    let fr1_double = Vr4300Fr1::semantics(&double_f0, &AnalysisPoint::new(ADDRESS))
        .unwrap()
        .location_accesses
        .into_iter()
        .find(|access| matches!(access.location, Location::Floating(_)))
        .unwrap()
        .location;
    let fr1_word = Vr4300Fr1::semantics(&word_f1, &AnalysisPoint::new(ADDRESS))
        .unwrap()
        .location_accesses
        .into_iter()
        .find(|access| matches!(access.location, Location::Floating(_)))
        .unwrap()
        .location;

    assert_eq!(
        Vr4300Fr0::storage_relation(&fr0_double, &fr0_word),
        StorageRelation::Overlapping
    );
    assert_eq!(
        Vr4300Fr1::storage_relation(&fr1_double, &fr1_word),
        StorageRelation::Disjoint
    );
}

#[test]
fn nec_unaligned_load_reads_the_old_merge_destination() {
    let instruction = Instruction::Memory {
        operation: MemoryOperation::LoadWordLeft,
        register: GeneralRegister::AT,
        base: GeneralRegister::A0,
        offset: 1,
    };
    let semantics = Vr4300Fr1::semantics(&instruction, &AnalysisPoint::new(ADDRESS)).unwrap();
    assert!(semantics.location_accesses.iter().any(|access| {
        access.location == Location::GeneralRegister(GeneralRegister::AT)
            && access.kind == AccessKind::Read
    }));
    assert!(semantics.location_accesses.iter().any(|access| {
        matches!(
            access.location,
            Location::Memory(MemoryLocation {
                width: MemoryWidth::Merge(4),
                ..
            })
        )
    }));
}

#[test]
fn nec_store_conditional_reads_and_clears_the_linked_reservation() {
    let instruction = Instruction::Memory {
        operation: MemoryOperation::StoreConditional,
        register: GeneralRegister::AT,
        base: GeneralRegister::A0,
        offset: 0,
    };
    let semantics = Vr4300Fr1::semantics(&instruction, &AnalysisPoint::new(ADDRESS)).unwrap();
    for kind in [AccessKind::Read, AccessKind::Write] {
        assert!(semantics.location_accesses.iter().any(|access| {
            access.location == Location::Implicit(ImplicitState::LoadLinkedReservation)
                && access.kind == kind
        }));
    }
    assert!(semantics.location_accesses.iter().any(|access| {
        matches!(access.location, Location::Memory(_))
            && access.kind == AccessKind::Write
            && access.occurrence == AccessOccurrence::Conditional
    }));
}

#[test]
fn nec_syscall_does_not_invent_an_architectural_resume_address() {
    let instruction = Instruction::SystemCall { code: 7 };
    let semantics = Vr4300Fr1::semantics(&instruction, &AnalysisPoint::new(ADDRESS)).unwrap();
    assert!(matches!(
        semantics.control_flow.action,
        ControlAction::Boundary(ControlBoundary::Trap {
            kind: Boundary::SystemCall(7),
            resume: None,
            ..
        })
    ));
}

#[test]
fn zero_based_memory_access_exposes_a_static_reference() {
    let instruction = Instruction::Memory {
        operation: MemoryOperation::LoadWord,
        register: GeneralRegister::AT,
        base: GeneralRegister::ZERO,
        offset: 0x20,
    };
    let semantics = Vr4300Fr1::semantics(&instruction, &AnalysisPoint::new(ADDRESS)).unwrap();
    assert!(semantics.static_references.iter().any(|reference| {
        reference.target
            == Reference::Memory {
                address: 0x20,
                byte_len: 4,
            }
            && reference.kind == ReferenceKind::Read
    }));
}

#[test]
fn fpu_move_does_not_claim_arithmetic_exception_state() {
    let instruction = Instruction::FloatingUnary {
        operation: vr4300::FloatingUnaryOperation::Move,
        format: FloatingFormat::Single,
        destination: FloatingRegister::new(0).unwrap(),
        source: FloatingRegister::new(2).unwrap(),
    };
    let semantics = Vr4300Fr1::semantics(&instruction, &AnalysisPoint::new(ADDRESS)).unwrap();
    assert!(
        !semantics
            .location_accesses
            .iter()
            .any(|access| { access.location == Location::FpuState(FpuState::ExceptionState) })
    );
}

#[test]
fn fpu_add_reads_rounding_and_may_update_exception_state() {
    let instruction = Instruction::FloatingBinary {
        operation: FloatingBinaryOperation::Add,
        format: FloatingFormat::Double,
        destination: FloatingRegister::new(0).unwrap(),
        left: FloatingRegister::new(0).unwrap(),
        right: FloatingRegister::new(2).unwrap(),
    };
    let semantics = Vr4300Fr1::semantics(&instruction, &AnalysisPoint::new(ADDRESS)).unwrap();
    assert!(semantics.location_accesses.iter().any(|access| {
        access.location == Location::FpuState(FpuState::RoundingMode)
            && access.kind == AccessKind::Read
    }));
    assert!(semantics.location_accesses.iter().any(|access| {
        access.location == Location::FpuState(FpuState::ExceptionState)
            && access.kind == AccessKind::Write
            && access.occurrence == AccessOccurrence::Conditional
    }));
}
