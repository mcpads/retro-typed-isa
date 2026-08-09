//! Semantic facts transcribed independently from the IDT R30xx software
//! reference and Sony Inline Programming Reference release 4.3.

use std::collections::BTreeSet;

use psx_r3000a::{
    Cop0StatusField, GteCommand, GteControlRegister, GteDataRegister, GteOpcode, IndirectTarget,
    Instruction, Location, PsxR3000A, Register,
};
use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlTarget, DelaySlotExecution, StaticSemantics,
};

const PC: u32 = 0x8001_0000;

fn command(opcode: GteOpcode) -> Instruction {
    Instruction::Gte(GteCommand {
        opcode,
        fake: 0,
        shift_fraction: true,
        matrix: 0,
        vector: 0,
        control_vector: 0,
        limit_unsigned: false,
    })
}

fn data_accesses(instruction: &Instruction, kind: AccessKind) -> BTreeSet<u8> {
    PsxR3000A::semantics(instruction, &PC)
        .unwrap()
        .location_accesses
        .into_iter()
        .filter_map(|access| match (access.kind, access.location) {
            (actual, Location::GteData(register)) if actual == kind => Some(register.index()),
            _ => None,
        })
        .collect()
}

fn control_reads(instruction: &Instruction) -> BTreeSet<u8> {
    PsxR3000A::semantics(instruction, &PC)
        .unwrap()
        .location_accesses
        .into_iter()
        .filter_map(|access| match (access.kind, access.location) {
            (AccessKind::Read, Location::GteControl(register)) => Some(register.index()),
            _ => None,
        })
        .collect()
}

#[test]
fn every_sony_gte_command_has_its_calculation_inputs_outputs_and_flag() {
    struct Row {
        opcode: GteOpcode,
        data_reads: &'static [u8],
        data_writes: &'static [u8],
        control_reads: &'static [u8],
    }

    const COLOR_OUT: &[u8] = &[9, 10, 11, 20, 21, 22, 25, 26, 27];
    const LIGHT_CONTROL: &[u8] = &[8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20];
    const DEPTH_CONTROL: &[u8] = &[21, 22, 23];
    const ROWS: &[Row] = &[
        Row {
            opcode: GteOpcode::Rtps,
            data_reads: &[0, 1, 13, 14, 17, 18, 19],
            data_writes: &[8, 9, 10, 11, 12, 13, 14, 16, 17, 18, 19, 24, 25, 26, 27],
            control_reads: &[0, 1, 2, 3, 4, 5, 6, 7, 24, 25, 26, 27, 28],
        },
        Row {
            opcode: GteOpcode::Rtpt,
            data_reads: &[0, 1, 2, 3, 4, 5, 19],
            data_writes: &[8, 9, 10, 11, 12, 13, 14, 16, 17, 18, 19, 24, 25, 26, 27],
            control_reads: &[0, 1, 2, 3, 4, 5, 6, 7, 24, 25, 26, 27, 28],
        },
        Row {
            opcode: GteOpcode::Ncds,
            data_reads: &[0, 1, 6, 8, 21, 22],
            data_writes: COLOR_OUT,
            control_reads: &[8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23],
        },
        Row {
            opcode: GteOpcode::Ncdt,
            data_reads: &[0, 1, 2, 3, 4, 5, 6, 8],
            data_writes: COLOR_OUT,
            control_reads: &[8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23],
        },
        Row {
            opcode: GteOpcode::Nccs,
            data_reads: &[0, 1, 6, 21, 22],
            data_writes: COLOR_OUT,
            control_reads: LIGHT_CONTROL,
        },
        Row {
            opcode: GteOpcode::Ncct,
            data_reads: &[0, 1, 2, 3, 4, 5, 6],
            data_writes: COLOR_OUT,
            control_reads: LIGHT_CONTROL,
        },
        Row {
            opcode: GteOpcode::Cdp,
            data_reads: &[6, 8, 9, 10, 11, 21, 22],
            data_writes: COLOR_OUT,
            control_reads: &[13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23],
        },
        Row {
            opcode: GteOpcode::Cc,
            data_reads: &[6, 9, 10, 11, 21, 22],
            data_writes: COLOR_OUT,
            control_reads: &[13, 14, 15, 16, 17, 18, 19, 20],
        },
        Row {
            opcode: GteOpcode::Ncs,
            data_reads: &[0, 1, 21, 22],
            data_writes: COLOR_OUT,
            control_reads: LIGHT_CONTROL,
        },
        Row {
            opcode: GteOpcode::Nct,
            data_reads: &[0, 1, 2, 3, 4, 5],
            data_writes: COLOR_OUT,
            control_reads: LIGHT_CONTROL,
        },
        Row {
            opcode: GteOpcode::Mvmva,
            data_reads: &[0, 1],
            data_writes: &[9, 10, 11, 25, 26, 27],
            control_reads: &[0, 1, 2, 3, 4, 5, 6, 7],
        },
        Row {
            opcode: GteOpcode::Intpl,
            data_reads: &[6, 8, 9, 10, 11, 21, 22],
            data_writes: COLOR_OUT,
            control_reads: DEPTH_CONTROL,
        },
        Row {
            opcode: GteOpcode::Dcpl,
            data_reads: &[6, 8, 9, 10, 11, 21, 22],
            data_writes: COLOR_OUT,
            control_reads: DEPTH_CONTROL,
        },
        Row {
            opcode: GteOpcode::Dpcs,
            data_reads: &[6, 8, 21, 22],
            data_writes: COLOR_OUT,
            control_reads: DEPTH_CONTROL,
        },
        Row {
            opcode: GteOpcode::Dpct,
            data_reads: &[6, 8, 20, 21, 22],
            data_writes: COLOR_OUT,
            control_reads: DEPTH_CONTROL,
        },
        Row {
            opcode: GteOpcode::Sqr,
            data_reads: &[9, 10, 11],
            data_writes: &[9, 10, 11, 25, 26, 27],
            control_reads: &[],
        },
        Row {
            opcode: GteOpcode::Avsz3,
            data_reads: &[17, 18, 19],
            data_writes: &[7, 24],
            control_reads: &[29],
        },
        Row {
            opcode: GteOpcode::Avsz4,
            data_reads: &[16, 17, 18, 19],
            data_writes: &[7, 24],
            control_reads: &[30],
        },
        Row {
            opcode: GteOpcode::Nclip,
            data_reads: &[12, 13, 14],
            data_writes: &[24],
            control_reads: &[],
        },
        Row {
            opcode: GteOpcode::Op,
            data_reads: &[9, 10, 11],
            data_writes: &[9, 10, 11, 25, 26, 27],
            control_reads: &[0, 2, 4],
        },
        Row {
            opcode: GteOpcode::Gpf,
            data_reads: &[6, 8, 9, 10, 11, 21, 22],
            data_writes: COLOR_OUT,
            control_reads: &[],
        },
        Row {
            opcode: GteOpcode::Gpl,
            data_reads: &[6, 8, 9, 10, 11, 21, 22, 25, 26, 27],
            data_writes: COLOR_OUT,
            control_reads: &[],
        },
    ];

    assert_eq!(ROWS.len(), GteOpcode::ALL.len());
    assert_eq!(
        ROWS.iter().map(|row| row.opcode).collect::<BTreeSet<_>>(),
        GteOpcode::ALL.into_iter().collect()
    );
    for row in ROWS {
        let instruction = command(row.opcode);
        assert_eq!(
            data_accesses(&instruction, AccessKind::Read),
            row.data_reads.iter().copied().collect(),
            "{:?} data reads",
            row.opcode
        );
        assert_eq!(
            data_accesses(&instruction, AccessKind::Write),
            row.data_writes.iter().copied().collect(),
            "{:?} data writes",
            row.opcode
        );
        assert_eq!(
            control_reads(&instruction),
            row.control_reads.iter().copied().collect(),
            "{:?} control reads",
            row.opcode
        );
        let semantics = PsxR3000A::semantics(&instruction, &PC).unwrap();
        assert!(
            semantics
                .location_accesses
                .iter()
                .any(|access| access.kind == AccessKind::Write
                    && access.location == Location::GteControl(GteControlRegister::FLAG))
        );
    }
}

#[test]
fn r3000_control_rules_do_not_import_abi_return_conventions() {
    let jump = PsxR3000A::semantics(&Instruction::Jr { rs: Register::RA }, &PC).unwrap();
    assert_eq!(
        jump.control_flow.action,
        ControlAction::Transfer {
            target: ControlTarget::Indirect(IndirectTarget::Register(Register::RA))
        }
    );
    assert_eq!(
        jump.control_flow.delay_slot.unwrap().execution,
        DelaySlotExecution::Always
    );

    let rfe = PsxR3000A::semantics(&Instruction::Rfe, &PC).unwrap();
    assert_eq!(rfe.control_flow.action, ControlAction::Continue);
    assert!(rfe.location_accesses.iter().any(|access| access.location
        == Location::Cop0Status(Cop0StatusField::ModeStack)
        && access.kind == AccessKind::Read));
}

#[test]
fn merge_load_and_conditional_link_preserve_manual_side_effects() {
    let merge = PsxR3000A::semantics(
        &Instruction::Lwl {
            rt: Register::T0,
            base: Register::T1,
            offset: 1,
        },
        &PC,
    )
    .unwrap();
    assert!(
        merge
            .location_accesses
            .iter()
            .any(|access| access.location == Location::Gpr(Register::T0)
                && access.kind == AccessKind::Read)
    );
    assert!(
        merge
            .location_accesses
            .iter()
            .any(|access| access.location == Location::Gpr(Register::T0)
                && access.kind == AccessKind::Write)
    );

    let link = PsxR3000A::semantics(
        &Instruction::Bgezal {
            rs: Register::T0,
            target: PC + 16,
        },
        &PC,
    )
    .unwrap();
    assert!(
        link.location_accesses
            .iter()
            .any(|access| access.location == Location::Gpr(Register::RA)
                && access.kind == AccessKind::Write
                && access.occurrence == AccessOccurrence::Conditional)
    );
}

#[test]
fn gte_transfer_ports_expand_their_documented_implicit_effects() {
    let sxyp = PsxR3000A::semantics(
        &Instruction::Mtc2 {
            rt: Register::T0,
            rd: 15,
        },
        &PC,
    )
    .unwrap();
    for index in [12, 13, 14] {
        assert!(
            sxyp.location_accesses
                .iter()
                .any(|access| access.kind == AccessKind::Write
                    && access.location == Location::GteData(GteDataRegister::new(index).unwrap()))
        );
    }

    let irgb = PsxR3000A::semantics(
        &Instruction::Mtc2 {
            rt: Register::T0,
            rd: 28,
        },
        &PC,
    )
    .unwrap();
    for index in [9, 10, 11] {
        assert!(
            irgb.location_accesses
                .iter()
                .any(|access| access.kind == AccessKind::Write
                    && access.location == Location::GteData(GteDataRegister::new(index).unwrap()))
        );
    }
}
