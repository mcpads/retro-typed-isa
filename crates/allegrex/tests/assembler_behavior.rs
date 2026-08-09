use allegrex::{
    Assembler, AssemblerError, EqualityCondition, ExpectedWriteVerifier, GeneralRegister,
    Instruction,
};
use expected_write::{
    ExpectedWrite, ImageRegion, MachineCodeProvenance, MachineCodeVerifierError, RegionKind,
    WriteIntent, WritePlan, WritePlanError,
};

const ORIGIN: allegrex::CodeLocation = 0x0880_0000;

#[test]
fn labels_lower_to_checked_control_flow_and_data_stays_out_of_instruction_spans() {
    let mut assembler = Assembler::new();
    assembler
        .label("entry")
        .emit(Instruction::Nop)
        .equality_branch_to_label(
            EqualityCondition::Equal,
            false,
            GeneralRegister::ZERO,
            GeneralRegister::ZERO,
            "exit",
        )
        .data([0xaa, 0xbb, 0xcc, 0xdd])
        .label("exit")
        .emit(Instruction::Nop);

    let program = assembler.assemble(ORIGIN).unwrap();
    assert_eq!(
        program.bytes(),
        [
            0x00, 0x00, 0x00, 0x00, // NOP
            0x01, 0x00, 0x00, 0x10, // BEQ to exit
            0xaa, 0xbb, 0xcc, 0xdd, // Declared data
            0x00, 0x00, 0x00, 0x00, // NOP
        ]
    );
    assert_eq!(program.labels().get("entry"), Some(&0x0880_0000));
    assert_eq!(program.labels().get("exit"), Some(&0x0880_000c));
    assert_eq!(
        program
            .instruction_spans()
            .iter()
            .map(|span| (span.offset, span.location))
            .collect::<Vec<_>>(),
        [(0, 0x0880_0000), (4, 0x0880_0004), (12, 0x0880_000c)]
    );
}

#[test]
fn assembler_rejects_undefined_duplicate_and_unaligned_placements() {
    let mut undefined = Assembler::new();
    undefined.jump_to_label(false, "missing");
    assert_eq!(
        undefined.assemble(ORIGIN),
        Err(AssemblerError::UndefinedLabel {
            label: "missing".to_owned(),
        })
    );

    let mut duplicate = Assembler::new();
    duplicate.label("same").label("same");
    assert_eq!(
        duplicate.assemble(ORIGIN),
        Err(AssemblerError::DuplicateLabel {
            label: "same".to_owned(),
        })
    );

    let mut unaligned = Assembler::new();
    unaligned.data([0]).emit(Instruction::Nop);
    assert_eq!(
        unaligned.assemble(ORIGIN),
        Err(AssemblerError::UnalignedInstruction { offset: 1 })
    );
    assert_eq!(
        Assembler::new().assemble(ORIGIN + 2),
        Err(AssemblerError::UnalignedOrigin { origin: ORIGIN + 2 })
    );
}

#[test]
fn expected_write_accepts_only_the_named_typed_allegrex_program() {
    let mut assembler = Assembler::new();
    assembler
        .emit(Instruction::Nop)
        .emit(Instruction::Synchronize);
    let source = assembler.assemble(ORIGIN).unwrap();
    let verifier = ExpectedWriteVerifier::new(move |source_id: &str| {
        if source_id == "hook-body" {
            Ok(source.clone())
        } else {
            Err(MachineCodeVerifierError::new("unknown typed source"))
        }
    });
    let baseline = [0xff_u8; 9];
    let plan = WritePlan::new()
        .region(ImageRegion {
            id: "code".to_owned(),
            range: 1..9,
            kind: RegionKind::MachineCode,
            reason: "hook site".to_owned(),
        })
        .write(ExpectedWrite {
            id: "hook".to_owned(),
            owner: "renderer-hook".to_owned(),
            purpose: "install typed hook".to_owned(),
            offset: 1,
            expected_original: baseline[1..].to_vec(),
            replacement: vec![0, 0, 0, 0, 0x0f, 0, 0, 0],
            intent: WriteIntent::MachineCode(MachineCodeProvenance {
                assembly_source_id: "hook-body".to_owned(),
                isa_profile_id: allegrex::PROFILE_ID.to_owned(),
            }),
        });

    assert_eq!(
        plan.apply(&baseline, Some(&verifier)).unwrap(),
        [0xff, 0, 0, 0, 0, 0x0f, 0, 0, 0]
    );

    let wrong_profile = WritePlan::new()
        .region(ImageRegion {
            id: "code".to_owned(),
            range: 1..9,
            kind: RegionKind::MachineCode,
            reason: "hook site".to_owned(),
        })
        .write(ExpectedWrite {
            id: "hook".to_owned(),
            owner: "renderer-hook".to_owned(),
            purpose: "install typed hook".to_owned(),
            offset: 1,
            expected_original: baseline[1..].to_vec(),
            replacement: vec![0, 0, 0, 0, 0x0f, 0, 0, 0],
            intent: WriteIntent::MachineCode(MachineCodeProvenance {
                assembly_source_id: "hook-body".to_owned(),
                isa_profile_id: "mips-looks-close".to_owned(),
            }),
        });
    assert!(matches!(
        wrong_profile.apply(&baseline, Some(&verifier)),
        Err(WritePlanError::MachineCodeVerification { .. })
    ));
}
