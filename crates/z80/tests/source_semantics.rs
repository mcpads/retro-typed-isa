//! Selected semantic facts independently grounded in the Zilog UM0080
//! instruction descriptions.

use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlTarget, ReferenceKind, StaticSemantics,
    StorageRelation,
};
use z80::{
    ByteRegister, Condition, Flag, IndirectTarget, Instruction, Location, MemoryAddress, Reference,
    Register16, Z80,
};

fn has(semantics: &z80::InstructionSemantics, location: Location, kind: AccessKind) -> bool {
    semantics
        .location_accesses
        .iter()
        .any(|access| access.location == location && access.kind == kind)
}

#[test]
fn register_pairs_expand_to_physical_bytes_and_flags_overlap_f() {
    let semantics = Z80::semantics(&Instruction::AddHLRR(Register16::Bc), &0).unwrap();
    for byte in [
        ByteRegister::B,
        ByteRegister::C,
        ByteRegister::H,
        ByteRegister::L,
    ] {
        assert!(has(
            &semantics,
            Location::ByteRegister(byte),
            AccessKind::Read
        ));
    }
    assert_eq!(
        Z80::storage_relation(
            &Location::ByteRegister(ByteRegister::F),
            &Location::Flag(Flag::Carry)
        ),
        StorageRelation::Overlapping
    );
}

#[test]
fn conditional_calls_and_returns_guard_stack_effects() {
    let call = Z80::semantics(&Instruction::CallCond(Condition::Nz, 0x3456), &0x1000).unwrap();
    assert_eq!(
        call.control_flow.action,
        ControlAction::LinkedTransfer {
            target: ControlTarget::Direct(0x3456),
            return_site: 0x1003,
        }
    );
    assert_eq!(call.control_flow.fallthrough, Some(0x1003));
    assert!(call.location_accesses.iter().any(|access| {
        access.location
            == Location::Memory {
                address: MemoryAddress::Sp,
                byte_len: 2,
            }
            && access.kind == AccessKind::Write
            && access.occurrence == AccessOccurrence::Conditional
    }));

    let ret = Z80::semantics(&Instruction::RetCond(Condition::C), &0).unwrap();
    assert_eq!(
        ret.control_flow.action,
        ControlAction::Return {
            target: IndirectTarget::Stack
        }
    );
    assert_eq!(ret.control_flow.fallthrough, Some(1));
}

#[test]
fn repeat_instructions_expose_the_manual_loop_edge_and_updates() {
    let semantics = Z80::semantics(&Instruction::Ldir, &0x2000).unwrap();
    assert_eq!(
        semantics.control_flow.action,
        ControlAction::Transfer {
            target: ControlTarget::Direct(0x2000)
        }
    );
    assert_eq!(semantics.control_flow.fallthrough, Some(0x2002));
    assert!(has(
        &semantics,
        Location::Memory {
            address: MemoryAddress::Hl,
            byte_len: 1,
        },
        AccessKind::Read
    ));
    assert!(has(
        &semantics,
        Location::Memory {
            address: MemoryAddress::De,
            byte_len: 1,
        },
        AccessKind::Write
    ));
}

#[test]
fn absolute_memory_and_immediate_port_operands_produce_typed_references() {
    let memory = Z80::semantics(&Instruction::LdAAddr(0xff10), &0).unwrap();
    assert_eq!(
        memory.static_references[0].target,
        Reference::Memory {
            address: 0xff10,
            byte_len: 1
        }
    );
    assert_eq!(memory.static_references[0].kind, ReferenceKind::Read);

    let port = Z80::semantics(&Instruction::OutPortA(0x7f), &0).unwrap();
    assert_eq!(
        port.static_references[0].target,
        Reference::PortImmediateLow(0x7f)
    );
    assert_eq!(port.static_references[0].kind, ReferenceKind::Write);
}

#[test]
fn retn_restores_interrupt_enable_state_in_addition_to_the_stack_target() {
    let semantics = Z80::semantics(&Instruction::Retn, &0).unwrap();
    assert!(has(&semantics, Location::Iff2, AccessKind::Read));
    assert!(has(&semantics, Location::Iff1, AccessKind::Write));
}
