use powerpc_gekko_750cl::{
    AnalysisPoint, Boundary, ConditionRegisterBit, ConditionRegisterField, FloatingRegister,
    GeneralRegister, ImplicitState, IndirectTarget, Instruction, InstructionForm,
    InstructionModifiers, IntegerExceptionField, Location, Operand, PairedSingleWidth,
    PowerPc750Cl, PowerPcGekko, QuantizationRegister, Reference, SemanticError,
    SpecialPurposeRegister,
};
use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlBoundary, ControlTarget, ReferenceKind,
    StaticSemantics, StorageRelation,
};

fn gpr(index: u8) -> GeneralRegister {
    GeneralRegister::new(index).unwrap()
}

fn fpr(index: u8) -> FloatingRegister {
    FloatingRegister::new(index).unwrap()
}

fn has_access(
    semantics: &powerpc_gekko_750cl::InstructionSemantics,
    location: Location,
    kind: AccessKind,
    occurrence: AccessOccurrence,
) -> bool {
    semantics.location_accesses.iter().any(|access| {
        access.location == location && access.kind == kind && access.occurrence == occurrence
    })
}

#[test]
fn branch_options_control_fallthrough_and_condition_inputs() {
    let conditional = Instruction::new(
        InstructionForm::Bc,
        [
            Operand::BranchOptions(powerpc_gekko_750cl::BranchOptions::new(0).unwrap()),
            Operand::ConditionRegisterBit(ConditionRegisterBit::new(7).unwrap()),
            Operand::BranchTarget(0x1020),
        ],
    )
    .unwrap();
    let semantics = PowerPc750Cl::semantics(&conditional, &AnalysisPoint::new(0x1000)).unwrap();

    assert_eq!(
        semantics.control_flow.fallthrough,
        Some(AnalysisPoint::new(0x1004))
    );
    assert!(has_access(
        &semantics,
        Location::ConditionBit(ConditionRegisterBit::new(7).unwrap()),
        AccessKind::Read,
        AccessOccurrence::Always,
    ));
    assert!(has_access(
        &semantics,
        Location::SpecialPurposeRegister(SpecialPurposeRegister::Count),
        AccessKind::Write,
        AccessOccurrence::Always,
    ));
    assert!(semantics.static_references.is_empty());

    let unconditional = Instruction::new(
        InstructionForm::Bc,
        [
            Operand::BranchOptions(powerpc_gekko_750cl::BranchOptions::new(0b1_0100).unwrap()),
            Operand::ConditionRegisterBit(ConditionRegisterBit::new(0).unwrap()),
            Operand::BranchTarget(0x1040),
        ],
    )
    .unwrap();
    let semantics = PowerPc750Cl::semantics(&unconditional, &AnalysisPoint::new(0x1000)).unwrap();
    assert_eq!(semantics.control_flow.fallthrough, None);
    assert!(semantics.static_references.is_empty());
}

#[test]
fn linked_branch_exposes_the_direct_target_and_link_register_write() {
    let instruction = Instruction::with_modifiers(
        InstructionForm::B,
        InstructionModifiers {
            link: true,
            ..InstructionModifiers::default()
        },
        [Operand::BranchTarget(0x1100)],
    )
    .unwrap();
    let semantics = PowerPcGekko::semantics(&instruction, &AnalysisPoint::new(0x1000)).unwrap();

    assert_eq!(
        semantics.control_flow.action,
        ControlAction::LinkedTransfer {
            target: ControlTarget::Direct(AnalysisPoint::new(0x1100)),
            return_site: AnalysisPoint::new(0x1004),
        }
    );
    assert!(has_access(
        &semantics,
        Location::SpecialPurposeRegister(SpecialPurposeRegister::Link),
        AccessKind::Write,
        AccessOccurrence::Always,
    ));
}

#[test]
fn branch_to_link_register_is_not_reclassified_as_a_return() {
    let instruction = Instruction::new(
        InstructionForm::Bclr,
        [
            Operand::BranchOptions(powerpc_gekko_750cl::BranchOptions::new(0b1_0100).unwrap()),
            Operand::ConditionRegisterBit(ConditionRegisterBit::new(0).unwrap()),
        ],
    )
    .unwrap();
    let semantics = PowerPc750Cl::semantics(&instruction, &AnalysisPoint::new(0x1000)).unwrap();

    assert_eq!(
        semantics.control_flow.action,
        ControlAction::Transfer {
            target: ControlTarget::Indirect(IndirectTarget::LinkRegister),
        }
    );
    assert!(semantics.static_references.is_empty());
}

#[test]
fn reservation_and_conditional_store_are_explicit() {
    let load = Instruction::new(
        InstructionForm::Lwarx,
        [
            Operand::GeneralRegister(gpr(3)),
            Operand::GeneralRegister(gpr(4)),
            Operand::GeneralRegister(gpr(5)),
        ],
    )
    .unwrap();
    let load = PowerPc750Cl::semantics(&load, &AnalysisPoint::new(0)).unwrap();
    assert!(has_access(
        &load,
        Location::Implicit(ImplicitState::Reservation),
        AccessKind::Write,
        AccessOccurrence::Always,
    ));

    let store = Instruction::new(
        InstructionForm::Stwcx_,
        [
            Operand::GeneralRegister(gpr(3)),
            Operand::GeneralRegister(gpr(4)),
            Operand::GeneralRegister(gpr(5)),
        ],
    )
    .unwrap();
    let store = PowerPc750Cl::semantics(&store, &AnalysisPoint::new(0)).unwrap();
    assert!(store.location_accesses.iter().any(|access| {
        matches!(access.location, Location::Memory(_))
            && access.kind == AccessKind::Write
            && access.occurrence == AccessOccurrence::Conditional
    }));
    assert!(has_access(
        &store,
        Location::Implicit(ImplicitState::Reservation),
        AccessKind::Read,
        AccessOccurrence::Always,
    ));
    assert!(has_access(
        &store,
        Location::ConditionField(ConditionRegisterField::new(0).unwrap()),
        AccessKind::Write,
        AccessOccurrence::Always,
    ));
}

#[test]
fn string_register_ranges_and_xer_fields_preserve_overlap() {
    let immediate = Instruction::new(
        InstructionForm::Lswi,
        [
            Operand::GeneralRegister(gpr(30)),
            Operand::GeneralRegister(gpr(4)),
            Operand::StringByteCount(powerpc_gekko_750cl::StringByteCount::new(12).unwrap()),
        ],
    )
    .unwrap();
    let immediate = PowerPc750Cl::semantics(&immediate, &AnalysisPoint::new(0)).unwrap();
    let range = immediate
        .location_accesses
        .iter()
        .find_map(|access| match access.location {
            Location::GeneralRegisterRange { .. } => Some(access.location),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        PowerPc750Cl::storage_relation(&range, &Location::GeneralRegister(gpr(0))),
        StorageRelation::Overlapping
    );

    let indexed = Instruction::new(
        InstructionForm::Lswx,
        [
            Operand::GeneralRegister(gpr(6)),
            Operand::GeneralRegister(gpr(7)),
            Operand::GeneralRegister(gpr(8)),
        ],
    )
    .unwrap();
    let indexed = PowerPc750Cl::semantics(&indexed, &AnalysisPoint::new(0)).unwrap();
    assert!(has_access(
        &indexed,
        Location::IntegerExceptionField(IntegerExceptionField::StringByteCount),
        AccessKind::Read,
        AccessOccurrence::Always,
    ));
}

#[test]
fn quantized_transfer_does_not_invent_a_fixed_memory_width() {
    let instruction = Instruction::new(
        InstructionForm::PsqL,
        [
            Operand::FloatingRegister(fpr(3)),
            Operand::SignedImmediate(-16),
            Operand::GeneralRegister(gpr(0)),
            Operand::PairedSingleWidth(PairedSingleWidth::Pair),
            Operand::QuantizationRegister(QuantizationRegister::new(2).unwrap()),
        ],
    )
    .unwrap();
    let semantics = PowerPc750Cl::semantics(&instruction, &AnalysisPoint::new(0)).unwrap();

    assert!(semantics.static_references.iter().any(|reference| {
        reference.target == Reference::Address(0xffff_fff0)
            && reference.kind == ReferenceKind::Address
    }));
    assert!(
        !semantics
            .static_references
            .iter()
            .any(|reference| matches!(reference.target, Reference::Memory { .. }))
    );
}

#[test]
fn floating_record_and_mcrxr_use_their_architectural_subfields() {
    let floating = Instruction::with_modifiers(
        InstructionForm::Fadd,
        InstructionModifiers {
            record: true,
            ..InstructionModifiers::default()
        },
        [
            Operand::FloatingRegister(fpr(1)),
            Operand::FloatingRegister(fpr(2)),
            Operand::FloatingRegister(fpr(3)),
        ],
    )
    .unwrap();
    let floating = PowerPc750Cl::semantics(&floating, &AnalysisPoint::new(0)).unwrap();
    assert!(has_access(
        &floating,
        Location::ConditionField(ConditionRegisterField::new(1).unwrap()),
        AccessKind::Write,
        AccessOccurrence::Always,
    ));
    assert!(!has_access(
        &floating,
        Location::ConditionField(ConditionRegisterField::new(0).unwrap()),
        AccessKind::Write,
        AccessOccurrence::Always,
    ));

    let mcrxr = Instruction::new(
        InstructionForm::Mcrxr,
        [Operand::ConditionRegisterField(
            ConditionRegisterField::new(5).unwrap(),
        )],
    )
    .unwrap();
    let mcrxr = PowerPc750Cl::semantics(&mcrxr, &AnalysisPoint::new(0)).unwrap();
    for field in [
        IntegerExceptionField::SummaryOverflow,
        IntegerExceptionField::Overflow,
        IntegerExceptionField::Carry,
    ] {
        assert!(has_access(
            &mcrxr,
            Location::IntegerExceptionField(field),
            AccessKind::Read,
            AccessOccurrence::Always,
        ));
        assert!(has_access(
            &mcrxr,
            Location::IntegerExceptionField(field),
            AccessKind::Write,
            AccessOccurrence::Always,
        ));
    }
}

#[test]
fn profile_validation_and_system_call_boundary_fail_closed() {
    let added_register = Instruction::new(
        InstructionForm::Mfspr,
        [
            Operand::GeneralRegister(gpr(3)),
            Operand::SpecialPurposeRegister(SpecialPurposeRegister::HardwareImplementation4),
        ],
    )
    .unwrap();
    assert!(matches!(
        PowerPcGekko::semantics(&added_register, &AnalysisPoint::new(0)),
        Err(SemanticError::Gekko(_))
    ));
    PowerPc750Cl::semantics(&added_register, &AnalysisPoint::new(0)).unwrap();

    let call = Instruction::new(InstructionForm::Sc, []).unwrap();
    let call = PowerPc750Cl::semantics(&call, &AnalysisPoint::new(0x8000_0000)).unwrap();
    assert_eq!(
        call.control_flow.action,
        ControlAction::Boundary(ControlBoundary::Trap {
            kind: Boundary::SystemCall,
            target: Some(ControlTarget::Indirect(
                powerpc_gekko_750cl::IndirectTarget::SystemCallVector,
            )),
            resume: None,
        })
    );
}
