use r5900::{
    AnalysisPoint, Boundary, GeneralRegister, GeneralStorage, Location, MemoryLocation,
    MemoryWidth, ProductRegister, ProductStorage, R5900, Reference, VectorDestinationMask,
    VectorFloatingRegister, VectorLane, VectorState, decode_word,
};
use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlBoundary, DelaySlotExecution,
    ReferenceKind, StaticSemantics, StorageRelation,
};

const ADDRESS: u32 = 0x0010_0000;

fn semantics(word: u32) -> r5900::InstructionSemantics {
    let instruction = decode_word(word, ADDRESS).unwrap();
    R5900::semantics(&instruction, &AnalysisPoint::new(ADDRESS)).unwrap()
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
fn sony_jr_ra_remains_an_architectural_indirect_jump() {
    let semantics = semantics(0x03e0_0008); // jr $ra
    assert!(matches!(
        semantics.control_flow.action,
        ControlAction::Transfer { .. }
    ));
}

#[test]
fn sony_overflowing_arithmetic_has_conditional_write_and_trap_paths() {
    let semantics = semantics(0x0021_0820); // add $at, $at, $at
    assert!(matches!(
        semantics.control_flow.action,
        ControlAction::Boundary(ControlBoundary::Trap {
            kind: Boundary::IntegerOverflow,
            ..
        })
    ));
    assert_eq!(
        semantics.control_flow.fallthrough,
        Some(AnalysisPoint::new(ADDRESS + 4))
    );
    assert!(semantics.location_accesses.iter().any(|access| {
        matches!(
            access.location,
            Location::General(GeneralStorage {
                register: GeneralRegister::AT,
                byte_len: 8,
                ..
            })
        ) && access.kind == AccessKind::Write
            && access.occurrence == AccessOccurrence::Conditional
    }));
}

#[test]
fn sony_scalar_and_multimedia_gpr_views_preserve_physical_overlap() {
    let scalar = Location::General(GeneralStorage {
        register: GeneralRegister::AT,
        byte_offset: 0,
        byte_len: 8,
    });
    let packed = Location::General(GeneralStorage {
        register: GeneralRegister::AT,
        byte_offset: 0,
        byte_len: 16,
    });
    assert_eq!(
        R5900::storage_relation(&scalar, &packed),
        StorageRelation::Overlapping
    );
}

#[test]
fn sony_hi_lo_banks_are_independent_but_share_physical_registers() {
    let hi0 = Location::Product(ProductStorage {
        register: ProductRegister::Hi,
        byte_offset: 0,
        byte_len: 8,
    });
    let hi1 = Location::Product(ProductStorage {
        register: ProductRegister::Hi,
        byte_offset: 8,
        byte_len: 8,
    });
    let hi = Location::Product(ProductStorage {
        register: ProductRegister::Hi,
        byte_offset: 0,
        byte_len: 16,
    });
    assert_eq!(
        R5900::storage_relation(&hi0, &hi1),
        StorageRelation::Disjoint
    );
    assert_eq!(
        R5900::storage_relation(&hi0, &hi),
        StorageRelation::Overlapping
    );
}

#[test]
fn sony_quadword_load_exposes_full_gpr_and_static_memory_range() {
    let semantics = semantics(0x7801_0020); // lq $at, 0x20($zero)
    assert!(semantics.location_accesses.iter().any(|access| {
        matches!(
            access.location,
            Location::General(GeneralStorage {
                register: GeneralRegister::AT,
                byte_len: 16,
                ..
            })
        ) && access.kind == AccessKind::Write
    }));
    assert!(semantics.location_accesses.iter().any(|access| {
        matches!(
            access.location,
            Location::Memory(MemoryLocation {
                width: MemoryWidth::Known(16),
                ..
            })
        ) && access.kind == AccessKind::Read
    }));
    assert!(semantics.static_references.iter().any(|reference| {
        reference.target
            == Reference::Memory {
                address: 0x20,
                byte_len: 16,
            }
            && reference.kind == ReferenceKind::Read
    }));
}

#[test]
fn sony_vu_fmac_preserves_destination_lane_and_flag_masks() {
    let semantics = semantics(0x4b02_08e8); // vadd.x VF3, VF1, VF2
    let x = VectorDestinationMask::new(VectorLane::X.destination_bits()).unwrap();
    let vf3 = VectorFloatingRegister::new(3).unwrap();
    assert!(semantics.location_accesses.iter().any(|access| {
        access.location
            == Location::VectorFloating {
                register: vf3,
                lanes: x,
            }
            && access.kind == AccessKind::Write
    }));
    assert!(semantics.location_accesses.iter().any(|access| {
        access.location == Location::VectorState(VectorState::MacFlags(x))
            && access.kind == AccessKind::Write
    }));
}

#[test]
fn sony_vclip_reads_xyz_against_only_the_w_divisor_lane() {
    let semantics = semantics(0x4bc2_09ff); // vclip VF1.xyz, VF2.w
    let vf2 = VectorFloatingRegister::new(2).unwrap();
    let w = VectorDestinationMask::new(VectorLane::W.destination_bits()).unwrap();
    assert!(semantics.location_accesses.iter().any(|access| {
        access.location
            == Location::VectorFloating {
                register: vf2,
                lanes: w,
            }
            && access.kind == AccessKind::Read
    }));
    assert!(semantics.location_accesses.iter().any(|access| {
        access.location == Location::VectorState(VectorState::ClippingFlags)
            && access.kind == AccessKind::Write
    }));
}

#[test]
fn sony_vu_zero_based_lane_store_exposes_a_local_memory_reference() {
    let semantics = semantics(0x4b01_03ff); // viswr.x VI1, (VI0)x
    assert!(semantics.static_references.iter().any(|reference| {
        reference.target
            == Reference::VectorMemory {
                address: 0,
                byte_len: 4,
            }
            && reference.kind == ReferenceKind::Write
    }));
}

#[test]
fn sony_vcallms_is_a_microcode_reference_not_an_ee_control_transfer() {
    let semantics = semantics(0x4a00_0038); // vcallms 0
    assert_eq!(semantics.control_flow.action, ControlAction::Continue);
    assert!(semantics.static_references.iter().any(|reference| {
        reference.target == Reference::VectorMicrocode(0)
            && reference.kind == ReferenceKind::Address
    }));
    assert!(semantics.location_accesses.iter().any(|access| {
        access.location == Location::VectorState(VectorState::MicroExecution)
            && access.kind == AccessKind::Write
    }));
}
