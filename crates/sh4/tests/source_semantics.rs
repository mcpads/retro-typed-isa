use sh4::{
    AnalysisPoint, BankedRegister, FloatBank, FloatLaneBank, FpscrState, FpuPrecision,
    FpuTransferSize, GeneralRegisterBank, GeneralRegisterLocation, Instruction, Location, Register,
    SemanticError, Sh4Pr0Sz0, VectorRegister,
};
use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlTarget, LocationAccess, StaticSemantics,
    StorageRelation,
};

// Normative source: STMicroelectronics and Hitachi, SH-4 32-bit CPU Core
// Architecture, ADCS 7182230F. These assertions transcribe the architectural
// effects in the individual instruction descriptions; they are independent of
// the production decoder and semantic dispatch table.
//
// https://www.st.com/resource/en/user_manual/cd00147165-sh-4-32-bit-cpu-core-architecture-stmicroelectronics.pdf

fn point(general_bank: GeneralRegisterBank, float_bank: FloatBank) -> AnalysisPoint {
    AnalysisPoint::for_profile::<Sh4Pr0Sz0>(0x1000, general_bank, float_bank)
}

fn has_access(accesses: &[LocationAccess<Location>], location: Location, kind: AccessKind) -> bool {
    accesses.contains(&LocationAccess {
        location,
        kind,
        occurrence: AccessOccurrence::Always,
    })
}

#[test]
fn banked_register_operands_name_the_bank_opposite_sr_rb() {
    let instruction = Instruction::StcBank {
        bank: BankedRegister::R3,
        rn: Register::R10,
    };

    let primary = Sh4Pr0Sz0::semantics(
        &instruction,
        &point(GeneralRegisterBank::Primary, FloatBank::Primary),
    )
    .unwrap();
    assert!(has_access(
        &primary.location_accesses,
        Location::General(GeneralRegisterLocation::Banked(BankedRegister::R3)),
        AccessKind::Read,
    ));

    let banked = Sh4Pr0Sz0::semantics(
        &instruction,
        &point(GeneralRegisterBank::Banked, FloatBank::Primary),
    )
    .unwrap();
    assert!(has_access(
        &banked.location_accesses,
        Location::General(GeneralRegisterLocation::Primary(Register::R3)),
        AccessKind::Read,
    ));
    assert!(has_access(
        &banked.location_accesses,
        Location::General(GeneralRegisterLocation::Primary(Register::R10)),
        AccessKind::Write,
    ));

    let unresolved = Location::General(GeneralRegisterLocation::OppositeUnknown(Register::R3));
    assert_eq!(
        Sh4Pr0Sz0::storage_relation(
            &unresolved,
            &Location::General(GeneralRegisterLocation::Primary(Register::R3)),
        ),
        StorageRelation::Unknown,
    );
    assert_eq!(
        Sh4Pr0Sz0::storage_relation(
            &unresolved,
            &Location::General(GeneralRegisterLocation::Banked(BankedRegister::R3)),
        ),
        StorageRelation::Unknown,
    );
}

#[test]
fn fpscr_control_writes_update_or_invalidate_successor_decode_state() {
    let initial = point(GeneralRegisterBank::Primary, FloatBank::Primary);

    let size_change = Sh4Pr0Sz0::semantics(&Instruction::Fschg, &initial).unwrap();
    let size_successor = size_change.control_flow.fallthrough.unwrap();
    assert_eq!(
        size_successor.fpscr,
        Some(FpscrState::new(FpuPrecision::Single, FpuTransferSize::Pair,)),
    );

    let bank_change = Sh4Pr0Sz0::semantics(&Instruction::Frchg, &initial).unwrap();
    assert_eq!(
        bank_change.control_flow.fallthrough.unwrap().float_bank,
        FloatBank::Alternate,
    );

    let unknown = Sh4Pr0Sz0::semantics(&Instruction::LdsFpscr { rm: Register::R2 }, &initial)
        .unwrap()
        .control_flow
        .fallthrough
        .unwrap();
    assert_eq!(unknown.fpscr, None);
    assert_eq!(unknown.float_bank, FloatBank::Unknown);
    assert_eq!(
        Sh4Pr0Sz0::semantics(&Instruction::Nop, &unknown),
        Err(SemanticError::UnavailableDecodeState),
    );
}

#[test]
fn delayed_transfer_defers_post_slot_float_context_until_slot_composition() {
    let initial = point(GeneralRegisterBank::Primary, FloatBank::Primary);
    let semantics = Sh4Pr0Sz0::semantics(&Instruction::Bra { disp: 2 }, &initial).unwrap();
    let slot = semantics.control_flow.delay_slot.unwrap().point;
    assert_eq!(slot.general_bank, GeneralRegisterBank::Primary);
    assert_eq!(slot.float_bank, FloatBank::Primary);
    assert_eq!(slot.fpscr, initial.fpscr);

    let target = match semantics.control_flow.action {
        ControlAction::Transfer {
            target: ControlTarget::Direct(target),
        } => target,
        action => panic!("expected a direct transfer, got {action:?}"),
    };
    assert_eq!(target.general_bank, GeneralRegisterBank::Primary);
    assert_eq!(target.float_bank, FloatBank::Unknown);
    assert_eq!(target.fpscr, None);
}

#[test]
fn rte_delay_slot_uses_the_sr_context_restored_from_ssr() {
    let initial = point(GeneralRegisterBank::Primary, FloatBank::Primary);
    let slot = Sh4Pr0Sz0::semantics(&Instruction::Rte, &initial)
        .unwrap()
        .control_flow
        .delay_slot
        .unwrap()
        .point;

    assert_eq!(slot.general_bank, GeneralRegisterBank::Unknown);
    assert_eq!(slot.float_bank, FloatBank::Primary);
    assert_eq!(slot.fpscr, initial.fpscr);
}

#[test]
fn ftrv_reads_xmtrx_from_the_opposite_physical_float_bank() {
    let instruction = Instruction::Ftrv {
        fvn: VectorRegister::new(4).unwrap(),
    };
    let semantics = Sh4Pr0Sz0::semantics(
        &instruction,
        &point(GeneralRegisterBank::Primary, FloatBank::Primary),
    )
    .unwrap();

    for index in 0..16 {
        assert!(has_access(
            &semantics.location_accesses,
            Location::FloatLane {
                bank: FloatLaneBank::Extended,
                index,
            },
            AccessKind::Read,
        ));
    }
    for index in 4..8 {
        let lane = Location::FloatLane {
            bank: FloatLaneBank::Primary,
            index,
        };
        assert!(has_access(
            &semantics.location_accesses,
            lane,
            AccessKind::Read,
        ));
        assert!(has_access(
            &semantics.location_accesses,
            lane,
            AccessKind::Write,
        ));
    }
}

#[test]
fn privileged_boundaries_and_ldtlb_expose_their_architectural_state() {
    let initial = point(GeneralRegisterBank::Primary, FloatBank::Primary);
    let ldtlb = Sh4Pr0Sz0::semantics(&Instruction::Ldtlb, &initial).unwrap();
    for location in [
        Location::Pteh,
        Location::Ptel,
        Location::Ptea,
        Location::Mmucr,
    ] {
        assert!(has_access(
            &ldtlb.location_accesses,
            location,
            AccessKind::Read,
        ));
    }
    assert!(has_access(
        &ldtlb.location_accesses,
        Location::Tlb,
        AccessKind::Write,
    ));

    let trap = Sh4Pr0Sz0::semantics(&Instruction::Trapa { imm: 0x34 }, &initial).unwrap();
    for location in [Location::Ssr, Location::Spc, Location::Sgr] {
        assert!(has_access(
            &trap.location_accesses,
            location,
            AccessKind::Write,
        ));
    }
}
