use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlBoundary, ControlTarget, ReferenceKind,
    StaticReference, StaticSemantics, StorageRelation,
};
use v30::{
    AnalysisPoint, Boundary, CodeLocation, Condition, EffectiveAddress, EffectiveAddressBase,
    EffectiveAddressDisplacement, Instruction, InterruptState, Location, Operand, OperandSize,
    Reference, Register8, Register16, SegmentRegister, SegmentSelection, V30,
};

// Normative source: NEC, 16-Bit V Series Instruction User's Manual. The cases
// below transcribe register overlap, segmented addressing, repeat, interrupt,
// and interrupt-inhibition rules independently of the production tables.
//
// https://www.renesas.com/en/document/mah/16-bit-v-seriestm-instruction

fn point(state: InterruptState) -> AnalysisPoint {
    AnalysisPoint::new(
        CodeLocation {
            seg: 0x1234,
            off: 0x0100,
        },
        state,
    )
}

fn direct_memory(
    segment: Option<SegmentRegister>,
    offset: u16,
    size: OperandSize,
) -> EffectiveAddress {
    EffectiveAddress::new(
        segment,
        EffectiveAddressBase::Direct,
        EffectiveAddressDisplacement::Absolute(offset),
        size,
    )
    .unwrap()
}

#[test]
fn byte_registers_are_physical_views_of_their_word_registers() {
    assert_eq!(
        V30::storage_relation(
            &Location::Register16(Register16::AX),
            &Location::Register8(Register8::AL),
        ),
        StorageRelation::Overlapping,
    );
    assert_eq!(
        V30::storage_relation(
            &Location::Register8(Register8::AL),
            &Location::Register8(Register8::AH),
        ),
        StorageRelation::Disjoint,
    );
}

#[test]
fn direct_memory_and_lea_preserve_segmented_reference_meaning() {
    let memory = direct_memory(Some(SegmentRegister::CS), 0x3456, OperandSize::Word);
    let load = Instruction::Mov {
        dest: Operand::Reg16(Register16::AX),
        src: Operand::Mem(memory),
    };
    let semantics = V30::semantics(&load, &point(InterruptState::Unknown)).unwrap();
    assert!(semantics.static_references.contains(&StaticReference {
        target: Reference::Memory {
            segment: SegmentSelection::CurrentCode(0x1234),
            offset: 0x3456,
            byte_len: 2,
        },
        kind: ReferenceKind::Read,
        occurrence: AccessOccurrence::Always,
    }));

    let lea = Instruction::Lea {
        dest: Register16::BX,
        src: direct_memory(None, 0x3456, OperandSize::Word),
    };
    let semantics = V30::semantics(&lea, &point(InterruptState::Unknown)).unwrap();
    assert_eq!(
        semantics.static_references,
        [StaticReference {
            target: Reference::Offset(0x3456),
            kind: ReferenceKind::Address,
            occurrence: AccessOccurrence::Always,
        }],
    );
    assert!(
        !semantics
            .location_accesses
            .iter()
            .any(|access| matches!(access.location, Location::SegmentRegister(_)))
    );
}

#[test]
fn rep_movsb_has_a_self_loop_and_conditional_data_effects() {
    let instruction = Instruction::Rep(Box::new(Instruction::Movsb));
    let at = point(InterruptState::Enabled);
    let semantics = V30::semantics(&instruction, &at).unwrap();
    assert!(matches!(
        semantics.control_flow.action,
        ControlAction::Transfer {
            target: ControlTarget::Direct(target)
        } if target == at
    ));
    assert!(semantics.control_flow.fallthrough.is_some());
    assert!(semantics.location_accesses.iter().any(|access| {
        access.kind == AccessKind::Write
            && access.occurrence == AccessOccurrence::Conditional
            && matches!(access.location, Location::Memory(_))
    }));
}

#[test]
fn sti_and_stack_segment_loads_carry_the_documented_one_instruction_inhibition() {
    let sti = V30::semantics(&Instruction::Sti, &point(InterruptState::Disabled)).unwrap();
    let after_sti = sti.control_flow.fallthrough.unwrap();
    assert_eq!(
        after_sti.interrupt_state,
        InterruptState::EnableAfterNextInstruction,
    );
    let after_following = V30::semantics(&Instruction::Nop, &after_sti)
        .unwrap()
        .control_flow
        .fallthrough
        .unwrap();
    assert_eq!(after_following.interrupt_state, InterruptState::Enabled);

    let load_ss = Instruction::Mov {
        dest: Operand::Sreg(SegmentRegister::SS),
        src: Operand::Reg16(Register16::AX),
    };
    let after_ss = V30::semantics(&load_ss, &point(InterruptState::Enabled))
        .unwrap()
        .control_flow
        .fallthrough
        .unwrap();
    assert_eq!(
        after_ss.interrupt_state,
        InterruptState::InhibitedAfterStackSegmentLoad,
    );
}

#[test]
fn software_interrupt_reads_its_vector_and_writes_a_stack_frame() {
    let semantics = V30::semantics(
        &Instruction::Int { vector: 0x21 },
        &point(InterruptState::Enabled),
    )
    .unwrap();
    assert!(matches!(
        semantics.control_flow.action,
        ControlAction::Boundary(ControlBoundary::Trap {
            kind: Boundary::Interrupt(0x21),
            ..
        })
    ));
    assert!(semantics.static_references.contains(&StaticReference {
        target: Reference::Memory {
            segment: SegmentSelection::Zero,
            offset: 0x84,
            byte_len: 4,
        },
        kind: ReferenceKind::Read,
        occurrence: AccessOccurrence::Always,
    }));
    assert!(semantics.location_accesses.iter().any(|access| {
        access.kind == AccessKind::Write && matches!(access.location, Location::Memory(_))
    }));
}

#[test]
fn brkem_is_an_explicit_native_profile_exit() {
    let semantics = V30::semantics(
        &Instruction::Brkem { vector: 0x80 },
        &point(InterruptState::Unknown),
    )
    .unwrap();
    assert!(matches!(
        semantics.control_flow.action,
        ControlAction::Boundary(ControlBoundary::ProfileExit {
            kind: Boundary::EmulationMode(0x80),
            ..
        })
    ));
    assert_eq!(semantics.control_flow.fallthrough, None);
}

#[test]
fn taken_branch_preserves_the_post_instruction_interrupt_state() {
    let at = point(InterruptState::EnableAfterNextInstruction);
    let semantics = V30::semantics(
        &Instruction::Jcc {
            cond: Condition::Ne,
            target: 2,
        },
        &at,
    )
    .unwrap();

    assert_eq!(
        semantics.control_flow.fallthrough.unwrap().interrupt_state,
        InterruptState::Enabled
    );
    assert!(matches!(
        semantics.control_flow.action,
        ControlAction::Transfer {
            target: ControlTarget::Direct(AnalysisPoint {
                interrupt_state: InterruptState::Enabled,
                ..
            })
        }
    ));
}
