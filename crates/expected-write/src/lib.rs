//! 불변 원본에 대한 쓰기 소유권과 기계어 검산 경계.

use std::{collections::BTreeSet, error::Error, fmt, ops::Range};

/// 계획이 선언하는 이미지 영역의 의미.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionKind {
    Data,
    Metadata,
    MachineCode,
    Protected,
}

/// 서로 겹치지 않는 이미지 영역 하나.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageRegion {
    pub id: String,
    pub range: Range<usize>,
    pub kind: RegionKind,
    pub reason: String,
}

/// 이미지 길이를 바꾸는 단일 소유 작업.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResizePlan {
    pub owner: String,
    pub purpose: String,
    pub expected_input_len: usize,
    pub output_len: usize,
}

/// 기계어 쓰기를 만든 typed assembly source의 신원.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineCodeProvenance {
    pub assembly_source_id: String,
    pub isa_profile_id: String,
}

/// 쓰기 바이트가 맡는 역할.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteIntent {
    Data,
    Metadata,
    MachineCode(MachineCodeProvenance),
}

impl WriteIntent {
    fn region_kind(&self) -> RegionKind {
        match self {
            Self::Data => RegionKind::Data,
            Self::Metadata => RegionKind::Metadata,
            Self::MachineCode(_) => RegionKind::MachineCode,
        }
    }
}

/// 불변 원본의 선조건과 한 소유자가 책임지는 최종 바이트.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedWrite {
    pub id: String,
    pub owner: String,
    pub purpose: String,
    pub offset: usize,
    pub expected_original: Vec<u8>,
    pub replacement: Vec<u8>,
    pub intent: WriteIntent,
}

/// 기계어 검증기에 제공하는 쓰기 문맥.
pub struct MachineCodeCheck<'a> {
    pub region: &'a ImageRegion,
    pub write: &'a ExpectedWrite,
    pub provenance: &'a MachineCodeProvenance,
    pub baseline: &'a [u8],
}

/// 쓰기 시작점을 기준으로 한 연속 정적 decode 결과.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedInstruction {
    pub offset: usize,
    pub len: usize,
    pub canonical: String,
}

/// 프로젝트별 source lookup이나 ISA adapter가 보고하는 검증 실패.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineCodeVerifierError {
    message: String,
}

impl MachineCodeVerifierError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for MachineCodeVerifierError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for MachineCodeVerifierError {}

/// 선언한 assembly source와 ISA profile을 독립적으로 검산하는 adapter.
pub trait MachineCodeVerifier {
    fn assemble_source(
        &self,
        check: &MachineCodeCheck<'_>,
    ) -> Result<Vec<u8>, MachineCodeVerifierError>;

    fn disassemble(
        &self,
        check: &MachineCodeCheck<'_>,
    ) -> Result<Vec<DecodedInstruction>, MachineCodeVerifierError>;

    fn assemble_decoded(
        &self,
        check: &MachineCodeCheck<'_>,
        instructions: &[DecodedInstruction],
    ) -> Result<Vec<u8>, MachineCodeVerifierError>;
}

/// 모든 쓰기를 적용하기 전에 한 번에 검증하는 전역 계획.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WritePlan {
    pub resize: Option<ResizePlan>,
    pub regions: Vec<ImageRegion>,
    pub writes: Vec<ExpectedWrite>,
}

impl WritePlan {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn resize(mut self, resize: ResizePlan) -> Self {
        self.resize = Some(resize);
        self
    }

    pub fn region(mut self, region: ImageRegion) -> Self {
        self.regions.push(region);
        self
    }

    pub fn write(mut self, write: ExpectedWrite) -> Self {
        self.writes.push(write);
        self
    }

    /// 전체 계획을 불변 원본에 대해 먼저 검증한 뒤 최종 이미지를 만든다.
    pub fn apply(
        &self,
        baseline: &[u8],
        machine_code_verifier: Option<&dyn MachineCodeVerifier>,
    ) -> Result<Vec<u8>, WritePlanError> {
        let validated = self.validate(baseline, machine_code_verifier)?;
        let mut output = baseline[..baseline.len().min(validated.output_len)].to_vec();
        output.resize(validated.output_len, 0);
        for (write, range) in self.writes.iter().zip(&validated.write_ranges) {
            output[range.clone()].copy_from_slice(&write.replacement);
        }
        self.audit(baseline, &output, machine_code_verifier)?;
        Ok(output)
    }

    /// 외부 writer가 만든 결과에도 계획 밖 변경이 없는지 같은 규칙을 적용한다.
    pub fn audit(
        &self,
        baseline: &[u8],
        output: &[u8],
        machine_code_verifier: Option<&dyn MachineCodeVerifier>,
    ) -> Result<(), WritePlanError> {
        let validated = self.validate(baseline, machine_code_verifier)?;
        if output.len() != validated.output_len {
            return Err(WritePlanError::OutputLengthMismatch {
                expected: validated.output_len,
                actual: output.len(),
            });
        }

        for (write, range) in self.writes.iter().zip(&validated.write_ranges) {
            if output[range.clone()] != write.replacement {
                return Err(WritePlanError::PlannedBytesMismatch {
                    write_id: write.id.clone(),
                });
            }
        }
        for region in &self.regions {
            if region.kind == RegionKind::Protected
                && output[region.range.clone()] != baseline[region.range.clone()]
            {
                return Err(WritePlanError::ProtectedRegionChanged {
                    region_id: region.id.clone(),
                });
            }
        }
        for offset in 0..baseline.len().min(output.len()) {
            if baseline[offset] != output[offset]
                && !validated
                    .write_ranges
                    .iter()
                    .any(|range| range.contains(&offset))
            {
                return Err(WritePlanError::UntrackedFinalDiff { offset });
            }
        }
        Ok(())
    }

    fn validate(
        &self,
        baseline: &[u8],
        machine_code_verifier: Option<&dyn MachineCodeVerifier>,
    ) -> Result<ValidatedPlan, WritePlanError> {
        let output_len = self.validate_resize(baseline.len())?;
        self.validate_regions(output_len, baseline.len())?;
        let write_ranges = self.write_ranges()?;
        self.validate_writes(baseline, output_len, &write_ranges, machine_code_verifier)?;

        if output_len > baseline.len() {
            for offset in baseline.len()..output_len {
                if !write_ranges.iter().any(|range| range.contains(&offset)) {
                    return Err(WritePlanError::UnownedGrownByte { offset });
                }
            }
        }

        Ok(ValidatedPlan {
            output_len,
            write_ranges,
        })
    }

    fn validate_resize(&self, baseline_len: usize) -> Result<usize, WritePlanError> {
        let Some(resize) = &self.resize else {
            return Ok(baseline_len);
        };
        require_text("resize owner", &resize.owner)?;
        require_text("resize purpose", &resize.purpose)?;
        if resize.expected_input_len != baseline_len {
            return Err(WritePlanError::ResizeInputLengthMismatch {
                expected: resize.expected_input_len,
                actual: baseline_len,
            });
        }
        if resize.output_len == baseline_len {
            return Err(WritePlanError::ResizeDoesNotChangeLength);
        }
        Ok(resize.output_len)
    }

    fn validate_regions(
        &self,
        output_len: usize,
        baseline_len: usize,
    ) -> Result<(), WritePlanError> {
        let mut ids = BTreeSet::new();
        for (index, region) in self.regions.iter().enumerate() {
            require_text("region id", &region.id)?;
            require_text("region reason", &region.reason)?;
            if !ids.insert(region.id.as_str()) {
                return Err(WritePlanError::DuplicateRegionId {
                    id: region.id.clone(),
                });
            }
            if region.range.start >= region.range.end || region.range.end > output_len {
                return Err(WritePlanError::RegionOutsideOutput {
                    id: region.id.clone(),
                });
            }
            if region.kind == RegionKind::Protected && region.range.end > baseline_len {
                return Err(WritePlanError::ProtectedRegionOutsideOriginal {
                    id: region.id.clone(),
                });
            }
            for other in self.regions.iter().take(index) {
                if intersects(&region.range, &other.range) {
                    return Err(WritePlanError::OverlappingRegions {
                        first_id: other.id.clone(),
                        second_id: region.id.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    fn write_ranges(&self) -> Result<Vec<Range<usize>>, WritePlanError> {
        self.writes
            .iter()
            .map(|write| {
                if write.replacement.is_empty() {
                    return Err(WritePlanError::EmptyWrite {
                        write_id: write.id.clone(),
                    });
                }
                let end = write
                    .offset
                    .checked_add(write.replacement.len())
                    .ok_or_else(|| WritePlanError::WriteRangeOverflow {
                        write_id: write.id.clone(),
                    })?;
                Ok(write.offset..end)
            })
            .collect()
    }

    fn validate_writes(
        &self,
        baseline: &[u8],
        output_len: usize,
        ranges: &[Range<usize>],
        machine_code_verifier: Option<&dyn MachineCodeVerifier>,
    ) -> Result<(), WritePlanError> {
        let mut ids = BTreeSet::new();
        for (index, (write, range)) in self.writes.iter().zip(ranges).enumerate() {
            require_text("write id", &write.id)?;
            require_text("write owner", &write.owner)?;
            require_text("write purpose", &write.purpose)?;
            if !ids.insert(write.id.as_str()) {
                return Err(WritePlanError::DuplicateWriteId {
                    id: write.id.clone(),
                });
            }
            if range.end > output_len {
                return Err(WritePlanError::WriteOutsideOutput {
                    write_id: write.id.clone(),
                });
            }

            let original_end = range.end.min(baseline.len());
            let overlap_len = original_end.saturating_sub(write.offset);
            if write.expected_original.len() != overlap_len {
                return Err(WritePlanError::OriginalLengthMismatch {
                    write_id: write.id.clone(),
                    expected: overlap_len,
                    actual: write.expected_original.len(),
                });
            }
            if overlap_len != 0
                && baseline.get(write.offset..original_end)
                    != Some(write.expected_original.as_slice())
            {
                return Err(WritePlanError::OriginalBytesMismatch {
                    write_id: write.id.clone(),
                });
            }

            for (other_index, other_range) in ranges.iter().enumerate().take(index) {
                if intersects(range, other_range) {
                    return Err(WritePlanError::OverlappingWrites {
                        first_id: self.writes[other_index].id.clone(),
                        first_owner: self.writes[other_index].owner.clone(),
                        second_id: write.id.clone(),
                        second_owner: write.owner.clone(),
                    });
                }
            }

            let containing: Vec<&ImageRegion> = self
                .regions
                .iter()
                .filter(|region| contains(&region.range, range))
                .collect();
            if containing.len() != 1 {
                return Err(WritePlanError::WriteRegionOwnership {
                    write_id: write.id.clone(),
                    containing_regions: containing.len(),
                });
            }
            let region = containing[0];
            if region.kind == RegionKind::Protected {
                return Err(WritePlanError::ProtectedRegionWrite {
                    write_id: write.id.clone(),
                    region_id: region.id.clone(),
                });
            }
            if write.intent.region_kind() != region.kind {
                return Err(WritePlanError::IntentRegionMismatch {
                    write_id: write.id.clone(),
                    region_id: region.id.clone(),
                    intent: write.intent.region_kind(),
                    region_kind: region.kind,
                });
            }

            if let WriteIntent::MachineCode(provenance) = &write.intent {
                require_text("assembly source id", &provenance.assembly_source_id)?;
                require_text("ISA profile id", &provenance.isa_profile_id)?;
                let verifier = machine_code_verifier.ok_or_else(|| {
                    WritePlanError::MachineCodeVerifierMissing {
                        write_id: write.id.clone(),
                    }
                })?;
                self.verify_machine_code(baseline, region, write, provenance, verifier)?;
            }
        }
        Ok(())
    }

    fn verify_machine_code(
        &self,
        baseline: &[u8],
        region: &ImageRegion,
        write: &ExpectedWrite,
        provenance: &MachineCodeProvenance,
        verifier: &dyn MachineCodeVerifier,
    ) -> Result<(), WritePlanError> {
        let check = MachineCodeCheck {
            region,
            write,
            provenance,
            baseline,
        };
        let assembled = verifier
            .assemble_source(&check)
            .map_err(|source| verification_error(write, "assemble source", source))?;
        if assembled != write.replacement {
            return Err(WritePlanError::AssembledSourceMismatch {
                write_id: write.id.clone(),
                assembly_source_id: provenance.assembly_source_id.clone(),
            });
        }

        let instructions = verifier
            .disassemble(&check)
            .map_err(|source| verification_error(write, "disassemble", source))?;
        if instructions.is_empty() {
            return Err(WritePlanError::NoDecodedInstructions {
                write_id: write.id.clone(),
                isa_profile_id: provenance.isa_profile_id.clone(),
            });
        }
        let mut decoded_len = 0usize;
        for instruction in &instructions {
            if instruction.offset != decoded_len {
                return Err(WritePlanError::DecodedInstructionGap {
                    write_id: write.id.clone(),
                    expected_offset: decoded_len,
                    actual_offset: instruction.offset,
                });
            }
            if instruction.len == 0 {
                return Err(WritePlanError::EmptyDecodedInstruction {
                    write_id: write.id.clone(),
                    offset: instruction.offset,
                });
            }
            if instruction.canonical.trim().is_empty() {
                return Err(WritePlanError::MissingCanonicalInstruction {
                    write_id: write.id.clone(),
                    offset: instruction.offset,
                });
            }
            decoded_len = decoded_len.checked_add(instruction.len).ok_or_else(|| {
                WritePlanError::DecodedLengthOverflow {
                    write_id: write.id.clone(),
                }
            })?;
        }
        if decoded_len != write.replacement.len() {
            return Err(WritePlanError::IncompleteMachineCodeDecode {
                write_id: write.id.clone(),
                decoded_len,
                replacement_len: write.replacement.len(),
            });
        }
        let reassembled = verifier
            .assemble_decoded(&check, &instructions)
            .map_err(|source| verification_error(write, "assemble decoded", source))?;
        if reassembled != write.replacement {
            return Err(WritePlanError::DecodedReassemblyMismatch {
                write_id: write.id.clone(),
                isa_profile_id: provenance.isa_profile_id.clone(),
            });
        }
        Ok(())
    }
}

struct ValidatedPlan {
    output_len: usize,
    write_ranges: Vec<Range<usize>>,
}

fn require_text(label: &'static str, value: &str) -> Result<(), WritePlanError> {
    if value.trim().is_empty() {
        return Err(WritePlanError::EmptyText { label });
    }
    Ok(())
}

fn contains(outer: &Range<usize>, inner: &Range<usize>) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

fn intersects(left: &Range<usize>, right: &Range<usize>) -> bool {
    left.start < right.end && right.start < left.end
}

fn verification_error(
    write: &ExpectedWrite,
    stage: &'static str,
    source: MachineCodeVerifierError,
) -> WritePlanError {
    WritePlanError::MachineCodeVerification {
        write_id: write.id.clone(),
        stage,
        message: source.to_string(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WritePlanError {
    #[error("{label} is empty")]
    EmptyText { label: &'static str },
    #[error("resize expected input length {expected}, got {actual}")]
    ResizeInputLengthMismatch { expected: usize, actual: usize },
    #[error("resize plan does not change the image length")]
    ResizeDoesNotChangeLength,
    #[error("duplicate region id {id}")]
    DuplicateRegionId { id: String },
    #[error("region {id} is outside the planned output")]
    RegionOutsideOutput { id: String },
    #[error("protected region {id} is outside the original image")]
    ProtectedRegionOutsideOriginal { id: String },
    #[error("region {first_id} overlaps {second_id}")]
    OverlappingRegions { first_id: String, second_id: String },
    #[error("Expected Write {write_id} is empty")]
    EmptyWrite { write_id: String },
    #[error("Expected Write {write_id} range overflow")]
    WriteRangeOverflow { write_id: String },
    #[error("duplicate Expected Write id {id}")]
    DuplicateWriteId { id: String },
    #[error("Expected Write {write_id} is outside the planned output")]
    WriteOutsideOutput { write_id: String },
    #[error("Expected Write {write_id} has {actual} original bytes; {expected} required")]
    OriginalLengthMismatch {
        write_id: String,
        expected: usize,
        actual: usize,
    },
    #[error("Expected Write {write_id} original-byte precondition failed")]
    OriginalBytesMismatch { write_id: String },
    #[error("Expected Write {first_id} by {first_owner} overlaps {second_id} by {second_owner}")]
    OverlappingWrites {
        first_id: String,
        first_owner: String,
        second_id: String,
        second_owner: String,
    },
    #[error("Expected Write {write_id} is contained by {containing_regions} declared regions")]
    WriteRegionOwnership {
        write_id: String,
        containing_regions: usize,
    },
    #[error("Expected Write {write_id} targets protected region {region_id}")]
    ProtectedRegionWrite { write_id: String, region_id: String },
    #[error(
        "Expected Write {write_id} intent {intent:?} does not match region {region_id} kind {region_kind:?}"
    )]
    IntentRegionMismatch {
        write_id: String,
        region_id: String,
        intent: RegionKind,
        region_kind: RegionKind,
    },
    #[error("Expected Write {write_id} targets machine code without a verifier")]
    MachineCodeVerifierMissing { write_id: String },
    #[error("Expected Write {write_id} machine-code {stage} failed: {message}")]
    MachineCodeVerification {
        write_id: String,
        stage: &'static str,
        message: String,
    },
    #[error("Expected Write {write_id} differs from assembled source {assembly_source_id}")]
    AssembledSourceMismatch {
        write_id: String,
        assembly_source_id: String,
    },
    #[error("Expected Write {write_id} has no decoded instruction under {isa_profile_id}")]
    NoDecodedInstructions {
        write_id: String,
        isa_profile_id: String,
    },
    #[error(
        "Expected Write {write_id} decode continues at {actual_offset:#X}, expected {expected_offset:#X}"
    )]
    DecodedInstructionGap {
        write_id: String,
        expected_offset: usize,
        actual_offset: usize,
    },
    #[error("Expected Write {write_id} has an empty decoded instruction at {offset:#X}")]
    EmptyDecodedInstruction { write_id: String, offset: usize },
    #[error("Expected Write {write_id} has no canonical instruction at {offset:#X}")]
    MissingCanonicalInstruction { write_id: String, offset: usize },
    #[error("Expected Write {write_id} decoded length overflow")]
    DecodedLengthOverflow { write_id: String },
    #[error("Expected Write {write_id} decoded {decoded_len} of {replacement_len} bytes")]
    IncompleteMachineCodeDecode {
        write_id: String,
        decoded_len: usize,
        replacement_len: usize,
    },
    #[error("Expected Write {write_id} does not reassemble under {isa_profile_id}")]
    DecodedReassemblyMismatch {
        write_id: String,
        isa_profile_id: String,
    },
    #[error("grown output byte {offset:#X} has no Expected Write owner")]
    UnownedGrownByte { offset: usize },
    #[error("output length {actual} differs from planned length {expected}")]
    OutputLengthMismatch { expected: usize, actual: usize },
    #[error("output does not contain planned bytes for Expected Write {write_id}")]
    PlannedBytesMismatch { write_id: String },
    #[error("protected region {region_id} changed")]
    ProtectedRegionChanged { region_id: String },
    #[error("untracked final diff at offset {offset:#X}")]
    UntrackedFinalDiff { offset: usize },
}
