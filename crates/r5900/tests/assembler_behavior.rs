use expected_write::{
    ExpectedWrite, ImageRegion, MachineCodeProvenance, MachineCodeVerifierError, RegionKind,
    WriteIntent, WritePlan, WritePlanError,
};
use r5900::{
    Assembler, AssemblerError, ExpectedWriteVerifier, Instruction, InstructionForm, Operands,
};

const ORIGIN: r5900::CodeLocation = 0x0010_0000;

#[test]
fn labels_lower_to_checked_cop2_branches_and_data_is_not_executable() {
    let mut assembler = Assembler::new();
    assembler
        .label("entry")
        .emit(Instruction::new(InstructionForm::Vnop, Operands::None).unwrap())
        .coprocessor_branch_to_label(InstructionForm::Bc2t, "exit")
        .data([0xaa, 0xbb, 0xcc, 0xdd])
        .label("exit")
        .emit(Instruction::new(InstructionForm::Vwaitq, Operands::None).unwrap());

    let program = assembler.assemble(ORIGIN).unwrap();
    assert_eq!(
        program.bytes(),
        [
            0xff, 0x02, 0x00, 0x4a, // VNOP
            0x01, 0x00, 0x01, 0x49, // BC2T to exit
            0xaa, 0xbb, 0xcc, 0xdd, // Declared data
            0xbf, 0x03, 0x00, 0x4a, // VWAITQ
        ]
    );
    assert_eq!(program.labels().get("entry"), Some(&0x0010_0000));
    assert_eq!(program.labels().get("exit"), Some(&0x0010_000c));
    assert_eq!(
        program
            .instruction_spans()
            .iter()
            .map(|span| (span.offset, span.location))
            .collect::<Vec<_>>(),
        [(0, 0x0010_0000), (4, 0x0010_0004), (12, 0x0010_000c)]
    );
}

#[test]
fn assembler_accepts_the_last_complete_word_in_the_address_space() {
    let mut assembler = Assembler::new();
    assembler.emit(Instruction::new(InstructionForm::Vnop, Operands::None).unwrap());
    assert_eq!(
        assembler.assemble(0xffff_fffc).unwrap().bytes(),
        [0xff, 0x02, 0x00, 0x4a]
    );
}

#[test]
fn assembler_rejects_undefined_duplicate_and_unaligned_placements() {
    let mut undefined = Assembler::new();
    undefined.jump_to_label(InstructionForm::J, "missing");
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
    unaligned
        .data([0])
        .emit(Instruction::new(InstructionForm::Vnop, Operands::None).unwrap());
    assert_eq!(
        unaligned.assemble(ORIGIN),
        Err(AssemblerError::UnalignedInstruction { offset: 1 })
    );
}

#[test]
fn expected_write_accepts_only_the_named_typed_r5900_program() {
    let mut assembler = Assembler::new();
    assembler
        .emit(Instruction::new(InstructionForm::Vnop, Operands::None).unwrap())
        .emit(Instruction::new(InstructionForm::Vwaitq, Operands::None).unwrap());
    let source = assembler.assemble(ORIGIN).unwrap();
    let verifier = ExpectedWriteVerifier::new(move |source_id: &str| {
        if source_id == "hook-body" {
            Ok(source.clone())
        } else {
            Err(MachineCodeVerifierError::new("unknown typed source"))
        }
    });
    let baseline = [0xff_u8; 9];
    let replacement = vec![0xff, 0x02, 0x00, 0x4a, 0xbf, 0x03, 0x00, 0x4a];
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
            replacement: replacement.clone(),
            intent: WriteIntent::MachineCode(MachineCodeProvenance {
                assembly_source_id: "hook-body".to_owned(),
                isa_profile_id: r5900::PROFILE_ID.to_owned(),
            }),
        });
    let mut expected = vec![0xff];
    expected.extend_from_slice(&replacement);
    assert_eq!(plan.apply(&baseline, Some(&verifier)).unwrap(), expected);

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
            replacement,
            intent: WriteIntent::MachineCode(MachineCodeProvenance {
                assembly_source_id: "hook-body".to_owned(),
                isa_profile_id: "generic-mips".to_owned(),
            }),
        });
    assert!(matches!(
        wrong_profile.apply(&baseline, Some(&verifier)),
        Err(WritePlanError::MachineCodeVerification { .. })
    ));
}
