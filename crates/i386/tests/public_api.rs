use expected_write::{
    ExpectedWrite, ImageRegion, MachineCodeProvenance, MachineCodeVerifierError, RegionKind,
    WriteIntent, WritePlan,
};
use i386::{
    Assembler, CODE16_PROFILE_ID, CODE32_PROFILE_ID, CodeLocation, CodeMode, EncodeError,
    ExpectedWriteVerifier, GeneralRegister, I386Code16, I386Code32, Immediate, Operand, Operation,
    PrefixSequence, Register16, decode_bytes, encode_bytes,
};
use typed_isa_core::TypedIsa;

fn instruction(bytes: &[u8], code_mode: CodeMode) -> i386::Instruction {
    decode_bytes(bytes, code_mode).unwrap().into_instruction()
}

#[test]
fn typed_semantics_can_be_lowered_without_a_machine_code_input() {
    let instruction = i386::Instruction::new(
        CodeMode::Code16,
        PrefixSequence::default(),
        Operation::Move,
        vec![
            Operand::GeneralRegister(GeneralRegister::Word(Register16::Ax)),
            Operand::Immediate(Immediate::Word(0x1234)),
        ],
    )
    .unwrap();

    assert_eq!(
        encode_bytes(&instruction).unwrap(),
        [0xc7, 0xc0, 0x34, 0x12]
    );
}

#[test]
fn code_segment_profiles_preserve_the_external_decode_context() {
    assert_eq!(I386Code16::PROFILE_ID, CODE16_PROFILE_ID);
    assert_eq!(I386Code32::PROFILE_ID, CODE32_PROFILE_ID);

    let source = [0xb8, 0x78, 0x56, 0x34, 0x12];
    let at = CodeLocation::new(0x08, 0);
    let code16 = I386Code16::decode_one(&source, &at).unwrap();
    assert_eq!(code16.source().as_slice(), &source[..3]);
    let code32 = I386Code32::decode_one(&source, &at).unwrap();
    assert_eq!(code32.source().as_slice(), source);

    assert!(matches!(
        I386Code16::encode_one(code32.instruction(), &at),
        Err(EncodeError::CodeModeMismatch {
            expected: CodeMode::Code16,
            actual: CodeMode::Code32,
        })
    ));
}

#[test]
fn checked_assembler_rejects_mixed_code_modes_and_preserves_instruction_spans() {
    let mut assembler = Assembler::new(CodeMode::Code16);
    assembler
        .label("entry")
        .emit(instruction(&[0x90], CodeMode::Code16))
        .emit(instruction(&[0x89, 0xd8], CodeMode::Code16));
    let origin = CodeLocation::new(0x08, 0x1000);
    let program = assembler.assemble(origin).unwrap();
    assert_eq!(program.bytes(), [0x90, 0x89, 0xd8]);
    assert_eq!(program.label_location("entry"), Some(origin));
    assert_eq!(
        program.instruction_spans()[1].location,
        CodeLocation::new(0x08, 0x1001)
    );

    let mut mixed = Assembler::new(CodeMode::Code16);
    mixed.emit(instruction(&[0x90], CodeMode::Code32));
    assert!(matches!(
        mixed.assemble(CodeLocation::new(0x08, 0)),
        Err(i386::AssemblerError::CodeModeMismatch { .. })
    ));
}

#[test]
fn expected_write_accepts_only_the_resolved_code16_program() {
    let mut assembler = Assembler::new(CodeMode::Code16);
    assembler
        .emit(instruction(&[0x90], CodeMode::Code16))
        .emit(instruction(&[0x89, 0xd8], CodeMode::Code16))
        .emit(instruction(&[0xc3], CodeMode::Code16));
    let source = assembler.assemble(CodeLocation::new(0x08, 0)).unwrap();
    let verifier = ExpectedWriteVerifier::<I386Code16, _>::new(move |source_id: &str| {
        if source_id == "hook-body" {
            Ok(source.clone())
        } else {
            Err(MachineCodeVerifierError::new("unknown typed source"))
        }
    });
    let baseline = [0_u8; 4];
    let plan = WritePlan::new()
        .region(ImageRegion {
            id: "code".to_owned(),
            range: 0..4,
            kind: RegionKind::MachineCode,
            reason: "hook site".to_owned(),
        })
        .write(ExpectedWrite {
            id: "hook".to_owned(),
            owner: "renderer-hook".to_owned(),
            purpose: "install typed hook".to_owned(),
            offset: 0,
            expected_original: baseline.to_vec(),
            replacement: vec![0x90, 0x89, 0xd8, 0xc3],
            intent: WriteIntent::MachineCode(MachineCodeProvenance {
                assembly_source_id: "hook-body".to_owned(),
                isa_profile_id: CODE16_PROFILE_ID.to_owned(),
            }),
        });

    assert_eq!(
        plan.apply(&baseline, Some(&verifier)).unwrap(),
        [0x90, 0x89, 0xd8, 0xc3]
    );
}
