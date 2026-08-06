use std::ops::Range;

use expected_write::{
    DecodedInstruction, ExpectedWrite, ImageRegion, MachineCodeCheck, MachineCodeProvenance,
    MachineCodeVerifier, MachineCodeVerifierError, RegionKind, ResizePlan, WriteIntent, WritePlan,
    WritePlanError,
};

fn region(id: &str, range: Range<usize>, kind: RegionKind) -> ImageRegion {
    ImageRegion {
        id: id.to_owned(),
        range,
        kind,
        reason: "behavior fixture".to_owned(),
    }
}

fn write(
    id: &str,
    owner: &str,
    offset: usize,
    expected_original: &[u8],
    replacement: &[u8],
    intent: WriteIntent,
) -> ExpectedWrite {
    ExpectedWrite {
        id: id.to_owned(),
        owner: owner.to_owned(),
        purpose: "behavior fixture".to_owned(),
        offset,
        expected_original: expected_original.to_vec(),
        replacement: replacement.to_vec(),
        intent,
    }
}

fn data_plan() -> WritePlan {
    WritePlan::new()
        .region(region("data", 1..3, RegionKind::Data))
        .write(write(
            "text",
            "layout",
            1,
            &[1, 2],
            &[8, 9],
            WriteIntent::Data,
        ))
}

#[test]
fn apply_accepts_only_the_owned_change() {
    let baseline = [0, 1, 2, 3];
    let output = data_plan().apply(&baseline, None).unwrap();
    assert_eq!(output, [0, 8, 9, 3]);
    data_plan().audit(&baseline, &output, None).unwrap();
}

#[test]
fn original_mismatch_fails_before_writing() {
    let baseline = [0, 7, 2, 3];
    assert!(matches!(
        data_plan().apply(&baseline, None),
        Err(WritePlanError::OriginalBytesMismatch { write_id }) if write_id == "text"
    ));
}

#[test]
fn owners_cannot_overlap() {
    let baseline = [0, 1, 2, 3];
    let plan = data_plan().write(write(
        "second",
        "checksum",
        2,
        &[2],
        &[7],
        WriteIntent::Data,
    ));
    assert!(matches!(
        plan.apply(&baseline, None),
        Err(WritePlanError::OverlappingWrites { first_owner, second_owner, .. })
            if first_owner == "layout" && second_owner == "checksum"
    ));
}

#[test]
fn protected_ranges_and_untracked_diffs_fail_closed() {
    let baseline = [0, 1, 2, 3];
    let protected = WritePlan::new()
        .region(region("header", 0..2, RegionKind::Protected))
        .write(write(
            "header-edit",
            "layout",
            0,
            &[0],
            &[9],
            WriteIntent::Data,
        ));
    assert!(matches!(
        protected.apply(&baseline, None),
        Err(WritePlanError::ProtectedRegionWrite { .. })
    ));

    let mut output = data_plan().apply(&baseline, None).unwrap();
    output[3] = 4;
    assert_eq!(
        data_plan().audit(&baseline, &output, None),
        Err(WritePlanError::UntrackedFinalDiff { offset: 3 })
    );
}

#[test]
fn every_grown_byte_needs_an_owner_and_no_original_precondition() {
    let baseline = [0, 1];
    let resize = ResizePlan {
        owner: "rom-layout".to_owned(),
        purpose: "append bank".to_owned(),
        expected_input_len: 2,
        output_len: 4,
    };
    let incomplete = WritePlan::new()
        .resize(resize.clone())
        .region(region("new-bank", 2..4, RegionKind::Data))
        .write(write(
            "first-grown-byte",
            "rom-layout",
            2,
            &[],
            &[8],
            WriteIntent::Data,
        ));
    assert_eq!(
        incomplete.apply(&baseline, None),
        Err(WritePlanError::UnownedGrownByte { offset: 3 })
    );

    let complete = WritePlan::new()
        .resize(resize)
        .region(region("new-bank", 2..4, RegionKind::Data))
        .write(write(
            "new-bank-bytes",
            "rom-layout",
            2,
            &[],
            &[8, 9],
            WriteIntent::Data,
        ));
    assert_eq!(complete.apply(&baseline, None).unwrap(), [0, 1, 8, 9]);
}

#[derive(Clone, Copy)]
enum VerificationFault {
    None,
    Source,
    Gap,
    DecodeTail,
    Reassembly,
}

struct FixtureVerifier(VerificationFault);

impl MachineCodeVerifier for FixtureVerifier {
    fn assemble_source(
        &self,
        _check: &MachineCodeCheck<'_>,
    ) -> Result<Vec<u8>, MachineCodeVerifierError> {
        Ok(match self.0 {
            VerificationFault::Source => vec![0x90, 0xcc],
            _ => vec![0x90, 0xc3],
        })
    }

    fn disassemble(
        &self,
        _check: &MachineCodeCheck<'_>,
    ) -> Result<Vec<DecodedInstruction>, MachineCodeVerifierError> {
        let second_offset = match self.0 {
            VerificationFault::Gap => 2,
            _ => 1,
        };
        let second_len = match self.0 {
            VerificationFault::DecodeTail => 0,
            _ => 1,
        };
        Ok(vec![
            DecodedInstruction {
                offset: 0,
                len: 1,
                canonical: "nop".to_owned(),
            },
            DecodedInstruction {
                offset: second_offset,
                len: second_len,
                canonical: "ret".to_owned(),
            },
        ])
    }

    fn assemble_decoded(
        &self,
        _check: &MachineCodeCheck<'_>,
        _instructions: &[DecodedInstruction],
    ) -> Result<Vec<u8>, MachineCodeVerifierError> {
        Ok(match self.0 {
            VerificationFault::Reassembly => vec![0x90, 0xcc],
            _ => vec![0x90, 0xc3],
        })
    }
}

fn machine_code_plan() -> WritePlan {
    WritePlan::new()
        .region(region("code", 0..2, RegionKind::MachineCode))
        .write(write(
            "hook",
            "renderer-hook",
            0,
            &[0, 1],
            &[0x90, 0xc3],
            WriteIntent::MachineCode(MachineCodeProvenance {
                assembly_source_id: "hook-body".to_owned(),
                isa_profile_id: "fixture-isa".to_owned(),
            }),
        ))
}

#[test]
fn machine_code_requires_source_decode_and_reassembly_agreement() {
    let baseline = [0, 1];
    machine_code_plan()
        .apply(&baseline, Some(&FixtureVerifier(VerificationFault::None)))
        .unwrap();

    assert!(matches!(
        machine_code_plan().apply(&baseline, None),
        Err(WritePlanError::MachineCodeVerifierMissing { .. })
    ));
    assert!(matches!(
        machine_code_plan().apply(&baseline, Some(&FixtureVerifier(VerificationFault::Source))),
        Err(WritePlanError::AssembledSourceMismatch { .. })
    ));
    assert!(matches!(
        machine_code_plan().apply(&baseline, Some(&FixtureVerifier(VerificationFault::Gap))),
        Err(WritePlanError::DecodedInstructionGap { .. })
    ));
    assert!(matches!(
        machine_code_plan().apply(
            &baseline,
            Some(&FixtureVerifier(VerificationFault::DecodeTail))
        ),
        Err(WritePlanError::EmptyDecodedInstruction { .. })
    ));
    assert!(matches!(
        machine_code_plan().apply(
            &baseline,
            Some(&FixtureVerifier(VerificationFault::Reassembly))
        ),
        Err(WritePlanError::DecodedReassemblyMismatch { .. })
    ));
}
