use arm7tdmi::Register;
use arm946e_s::{
    AnalysisPoint, Arm946EsArm, Arm946EsThumb, Boundary, ControlBit, Flag, IndirectTarget,
    Location, MemoryAddress, MemoryLocation, MemoryWidth, ProcessorMode, Reference,
    RegisterLocation, decode_arm_bytes, decode_thumb_bytes,
};
use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlBoundary, ControlTarget, ReferenceKind,
    StaticSemantics,
};

fn arm(word: u32) -> arm946e_s::InstructionSemantics {
    let instruction = decode_arm_bytes(&word.to_le_bytes()).unwrap();
    Arm946EsArm::semantics(
        &instruction,
        &AnalysisPoint::arm(0x1000, ProcessorMode::System),
    )
    .unwrap()
}

fn thumb(bytes: &[u8]) -> arm946e_s::InstructionSemantics {
    let instruction = decode_thumb_bytes(bytes).unwrap();
    Arm946EsThumb::semantics(
        &instruction,
        &AnalysisPoint::thumb(0x1000, ProcessorMode::System),
    )
    .unwrap()
}

#[test]
fn ddi_0100i_arm_blx_immediate_has_a_thumb_target_and_arm_return_site() {
    let semantics = arm(0xfaff_fffe);
    assert!(matches!(
        semantics.control_flow.action,
        ControlAction::LinkedTransfer {
            target: ControlTarget::Direct(AnalysisPoint {
                location: 0x1000,
                state: arm946e_s::ExecutionState::Thumb,
                ..
            }),
            return_site: AnalysisPoint {
                location: 0x1004,
                state: arm946e_s::ExecutionState::Arm,
                ..
            },
        }
    ));
}

#[test]
fn ddi_0100i_breakpoint_does_not_claim_one_debug_system_outcome() {
    let semantics = arm(0xe121_2374);
    assert!(matches!(
        semantics.control_flow.action,
        ControlAction::Boundary(ControlBoundary::Trap {
            kind: Boundary::Breakpoint(0x1234),
            target: None,
            resume: None,
        })
    ));
}

#[test]
fn ddi_0100i_saturating_add_marks_only_the_sticky_q_write_as_data_dependent() {
    let semantics = arm(0xe102_0051);
    assert!(semantics.location_accesses.iter().any(|access| {
        access.location == Location::Flag(Flag::Saturation)
            && access.kind == AccessKind::Write
            && access.occurrence == AccessOccurrence::Conditional
    }));
    assert!(!semantics.location_accesses.iter().any(|access| {
        matches!(
            access.location,
            Location::Flag(Flag::Negative | Flag::Zero | Flag::Carry | Flag::Overflow)
        ) && access.kind == AccessKind::Write
    }));
}

#[test]
fn ddi_0100i_doubleword_load_names_one_eight_byte_range_and_two_registers() {
    let semantics = arm(0xe1c2_00d4);
    assert!(semantics.location_accesses.iter().any(|access| {
        matches!(
            access.location,
            Location::Memory(MemoryLocation {
                width: MemoryWidth::Known(8),
                ..
            })
        ) && access.kind == AccessKind::Read
    }));
    for register in [Register::new(0).unwrap(), Register::new(1).unwrap()] {
        assert!(semantics.location_accesses.iter().any(|access| {
            access.location == Location::Register(RegisterLocation::Unbanked(register))
                && access.kind == AccessKind::Write
        }));
    }
}

#[test]
fn ddi_0100i_pld_exposes_address_formation_without_claiming_a_memory_read() {
    let semantics = arm(0xf5df_f004);
    assert!(
        !semantics
            .location_accesses
            .iter()
            .any(|access| matches!(access.location, Location::Memory(_)))
    );
    assert!(semantics.static_references.iter().any(|reference| {
        reference.target == Reference::Address(0x100c) && reference.kind == ReferenceKind::Address
    }));
}

#[test]
fn armv5t_inherited_ldr_pc_uses_interworking_load_semantics() {
    let semantics = arm(0xe59f_f000);
    assert!(matches!(
        semantics.control_flow.action,
        ControlAction::Transfer {
            target: ControlTarget::Indirect(IndirectTarget::InterworkingLoadedProgramCounter)
        }
    ));
    assert!(semantics.location_accesses.iter().any(|access| {
        access.location == Location::ControlBit(ControlBit::ThumbState)
            && access.kind == AccessKind::Write
    }));
}

#[test]
fn armv5t_inherited_thumb_pop_pc_is_an_interworking_indirect_transfer() {
    let semantics = thumb(&0xbd01_u16.to_le_bytes());
    assert!(matches!(
        semantics.control_flow.action,
        ControlAction::Transfer {
            target: ControlTarget::Indirect(IndirectTarget::InterworkingLoadedProgramCounter)
        }
    ));
}

#[test]
fn ddi_0100i_thumb_blx_immediate_has_a_word_aligned_arm_target() {
    let semantics = thumb(&[0xff, 0xf7, 0xfe, 0xef]);
    assert!(matches!(
        semantics.control_flow.action,
        ControlAction::LinkedTransfer {
            target: ControlTarget::Direct(AnalysisPoint {
                location: 0x1000,
                state: arm946e_s::ExecutionState::Arm,
                ..
            }),
            ..
        }
    ));
}

#[test]
fn armv4t_bx_now_records_the_cpsr_state_bit_write_in_the_shared_model() {
    let semantics = arm(0xe12f_ff1e);
    assert!(semantics.location_accesses.iter().any(|access| {
        access.location == Location::ControlBit(ControlBit::ThumbState)
            && access.kind == AccessKind::Write
    }));
}

#[test]
fn literal_doubleword_ranges_overlap_their_constituent_words() {
    let doubleword = Location::Memory(MemoryLocation {
        address: MemoryAddress::Literal(0x1000),
        width: MemoryWidth::Known(8),
    });
    let second_word = Location::Memory(MemoryLocation {
        address: MemoryAddress::Literal(0x1004),
        width: MemoryWidth::Known(4),
    });
    assert_eq!(
        Arm946EsArm::storage_relation(&doubleword, &second_word),
        typed_isa_core::StorageRelation::Overlapping
    );
}
