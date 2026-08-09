use expected_write::{
    ExpectedWrite, ImageRegion, MachineCodeProvenance, MachineCodeVerifierError, RegionKind,
    WriteIntent, WritePlan,
};
use v30mz::{Assembler, CodeLocation, ExpectedWriteVerifier, Instruction, VSeriesInstruction};

#[test]
fn expected_write_accepts_the_external_typed_source() {
    let mut assembler = Assembler::new();
    assembler
        .emit(Instruction::new(VSeriesInstruction::Nop).unwrap())
        .emit(Instruction::new(VSeriesInstruction::Ret { pop: 0 }).unwrap());
    let source = assembler
        .assemble(CodeLocation {
            seg: 0x2000,
            off: 0x0100,
        })
        .unwrap();

    let verifier = ExpectedWriteVerifier::new(move |source_id: &str| {
        if source_id == "hook-body" {
            Ok(source.clone())
        } else {
            Err(MachineCodeVerifierError::new("unknown typed source"))
        }
    });

    let baseline = [0xff_u8; 3];
    let plan = WritePlan::new()
        .region(ImageRegion {
            id: "code".to_owned(),
            range: 1..3,
            kind: RegionKind::MachineCode,
            reason: "hook site".to_owned(),
        })
        .write(ExpectedWrite {
            id: "hook".to_owned(),
            owner: "renderer-hook".to_owned(),
            purpose: "install typed hook".to_owned(),
            offset: 1,
            expected_original: baseline[1..].to_vec(),
            replacement: vec![0x90, 0xc3],
            intent: WriteIntent::MachineCode(MachineCodeProvenance {
                assembly_source_id: "hook-body".to_owned(),
                isa_profile_id: v30mz::PROFILE_ID.to_owned(),
            }),
        });

    assert_eq!(
        plan.apply(&baseline, Some(&verifier)).unwrap(),
        [0xff, 0x90, 0xc3]
    );
}
