//! Selected access facts independently transcribed from the Hitachi SH-1/SH-2
//! Programming Manual instruction descriptions.

use sh2::{Addressing, Instruction, Location, MemoryLocation, Reference, Register, Sh2, SrField};
use typed_isa_core::{
    AccessKind, ControlAction, DelaySlotExecution, ReferenceKind, StaticSemantics,
};

fn has(semantics: &sh2::InstructionSemantics, location: Location, kind: AccessKind) -> bool {
    semantics
        .location_accesses
        .iter()
        .any(|access| access.location == location && access.kind == kind)
}

#[test]
fn predecrement_postincrement_and_same_register_exception_are_explicit() {
    let store = Sh2::semantics(
        &Instruction::MovLStorePreDec {
            rm: Register::R1,
            rn: Register::R2,
        },
        &0,
    )
    .unwrap();
    assert!(has(
        &store,
        Location::Register(Register::R2),
        AccessKind::Write
    ));
    assert!(has(
        &store,
        Location::Memory(MemoryLocation {
            addressing: Addressing::PreDecrement(Register::R2),
            byte_len: 4,
        }),
        AccessKind::Write
    ));

    let same = Sh2::semantics(
        &Instruction::MovLLoadPostInc {
            rm: Register::R3,
            rn: Register::R3,
        },
        &0,
    )
    .unwrap();
    assert_eq!(
        same.location_accesses
            .iter()
            .filter(|access| {
                access.location == Location::Register(Register::R3)
                    && access.kind == AccessKind::Write
            })
            .count(),
        1,
        "the destination write replaces the normally separate post-increment"
    );
}

#[test]
fn pc_relative_literal_and_mova_references_follow_alignment_rules() {
    let word = Sh2::semantics(
        &Instruction::MovWPcRel {
            disp: 3,
            rn: Register::R1,
        },
        &0x1002,
    )
    .unwrap();
    assert_eq!(
        word.static_references[0].target,
        Reference::Memory {
            address: 0x100c,
            byte_len: 2
        }
    );
    assert_eq!(word.static_references[0].kind, ReferenceKind::Read);

    let address = Sh2::semantics(&Instruction::Mova { disp: 2 }, &0x1002).unwrap();
    assert_eq!(
        address.static_references[0].target,
        Reference::Address(0x100c)
    );
    assert_eq!(address.static_references[0].kind, ReferenceKind::Address);
}

#[test]
fn delayed_conditionals_always_execute_the_slot_and_read_t() {
    let semantics = Sh2::semantics(&Instruction::Bts { disp: 2 }, &0x2000).unwrap();
    assert!(matches!(
        semantics.control_flow.action,
        ControlAction::Transfer { .. }
    ));
    assert_eq!(semantics.control_flow.fallthrough, Some(0x2004));
    assert_eq!(
        semantics
            .control_flow
            .delay_slot
            .as_ref()
            .unwrap()
            .execution,
        DelaySlotExecution::Always
    );
    assert!(has(
        &semantics,
        Location::SrField(SrField::T),
        AccessKind::Read
    ));
}

#[test]
fn mac_and_trap_include_manual_implicit_storage() {
    let mac = Sh2::semantics(
        &Instruction::MacL {
            rm: Register::R4,
            rn: Register::R5,
        },
        &0,
    )
    .unwrap();
    for location in [Location::Mach, Location::Macl] {
        assert!(has(&mac, location, AccessKind::Read));
        assert!(has(&mac, location, AccessKind::Write));
    }
    assert!(has(&mac, Location::SrField(SrField::S), AccessKind::Read));

    let trap = Sh2::semantics(&Instruction::Trapa { imm: 0x20 }, &0).unwrap();
    assert!(has(&trap, Location::Vbr, AccessKind::Read));
    assert!(has(
        &trap,
        Location::Memory(MemoryLocation {
            addressing: Addressing::ExceptionFrame,
            byte_len: 8,
        }),
        AccessKind::Write
    ));
}
