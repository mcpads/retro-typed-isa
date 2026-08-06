use expected_write::{
    ExpectedWrite, ImageRegion, MachineCodeProvenance, MachineCodeVerifierError, RegionKind,
    WriteIntent, WritePlan,
};
use v30::{
    Assembler, AssemblerError, Condition, ControlFlow, ExpectedWriteVerifier, Instruction,
    InstructionAddress, LoopCondition, ProgramVerificationError,
};

fn origin() -> InstructionAddress {
    InstructionAddress {
        seg: 0x1234,
        off: 0x0100,
    }
}

#[test]
fn labels_lower_to_checked_relative_control_flow() {
    let mut assembler = Assembler::new();
    assembler
        .label("start")
        .emit(Instruction::Nop)
        .emit_branch(Condition::Ne, "start")
        .emit_loop(LoopCondition::Always, "start")
        .emit_call_near("finish")
        .emit_jump_short("finish")
        .label("finish")
        .emit(Instruction::Ret { pop: 0 });

    let program = assembler.assemble(origin()).unwrap();
    assert_eq!(
        program.bytes(),
        [
            0x90, 0x75, 0xfd, 0xe2, 0xfb, 0xe8, 0x02, 0x00, 0xeb, 0x00, 0xc3
        ]
    );
    assert_eq!(program.label_address("start"), Some(origin()));
    assert_eq!(program.instruction_spans().len(), 6);
    assert_eq!(
        program.instruction_spans()[1].control_flow,
        ControlFlow::Branch {
            target: origin(),
            fallthrough: InstructionAddress {
                seg: 0x1234,
                off: 0x0103,
            },
        }
    );
}

#[test]
fn undefined_far_and_segment_crossing_programs_fail() {
    let mut undefined = Assembler::new();
    undefined.emit_jump_short("missing");
    assert_eq!(
        undefined.assemble(origin()),
        Err(AssemblerError::UndefinedLabel {
            label: "missing".to_owned(),
        })
    );

    let mut far = Assembler::new();
    far.emit_jump_short("far");
    for _ in 0..129 {
        far.emit(Instruction::Nop);
    }
    far.label("far");
    assert!(matches!(
        far.assemble(origin()),
        Err(AssemblerError::Relative8OutOfRange { .. })
    ));

    let mut crossing = Assembler::new();
    crossing.emit(Instruction::Nop).emit(Instruction::Nop);
    assert!(matches!(
        crossing.assemble(InstructionAddress {
            seg: 0,
            off: 0xffff,
        }),
        Err(AssemblerError::Verify(
            ProgramVerificationError::CodeSegmentBoundary { .. }
        ))
    ));
}

#[test]
fn expected_write_accepts_only_the_resolved_typed_v30_program() {
    let mut assembler = Assembler::new();
    assembler
        .emit(Instruction::Nop)
        .emit(Instruction::Ret { pop: 0 });
    let source = assembler
        .assemble(InstructionAddress { seg: 0, off: 0 })
        .unwrap();
    let verifier = ExpectedWriteVerifier::new(move |source_id: &str| {
        if source_id == "hook-body" {
            Ok(source.clone())
        } else {
            Err(MachineCodeVerifierError::new("unknown typed source"))
        }
    });
    let baseline = [0_u8, 1];
    let plan = WritePlan::new()
        .region(ImageRegion {
            id: "code".to_owned(),
            range: 0..2,
            kind: RegionKind::MachineCode,
            reason: "hook site".to_owned(),
        })
        .write(ExpectedWrite {
            id: "hook".to_owned(),
            owner: "renderer-hook".to_owned(),
            purpose: "install typed hook".to_owned(),
            offset: 0,
            expected_original: baseline.to_vec(),
            replacement: vec![0x90, 0xc3],
            intent: WriteIntent::MachineCode(MachineCodeProvenance {
                assembly_source_id: "hook-body".to_owned(),
                isa_profile_id: "v30".to_owned(),
            }),
        });

    assert_eq!(
        plan.apply(&baseline, Some(&verifier)).unwrap(),
        [0x90, 0xc3]
    );
}
