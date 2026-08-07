//! Write ownership and machine-code verification against immutable input.

#![deny(missing_docs)]

use std::{collections::BTreeSet, error::Error, fmt, ops::Range};

/// The meaning assigned to an image region by a write plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionKind {
    /// Non-executable payload bytes interpreted by the target program.
    Data,
    /// Structural bytes such as headers, tables, checksums, or indexes.
    Metadata,
    /// Executable instructions that require independent static verification.
    MachineCode,
    /// Original bytes that no declared write may change.
    Protected,
}

/// One non-overlapping image region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageRegion {
    /// Plan-unique region identifier used in diagnostics and ownership checks.
    pub id: String,
    /// Half-open byte range owned by the region in the final image.
    pub range: Range<usize>,
    /// Semantic role that writes within this region must declare.
    pub kind: RegionKind,
    /// Human-readable justification for declaring the region and its bounds.
    pub reason: String,
}

/// A single-owner operation that changes the image length.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResizePlan {
    /// Producer solely responsible for changing the image length.
    pub owner: String,
    /// Human-readable reason that the output must have a different length.
    pub purpose: String,
    /// Required immutable-input length before the resize is admitted.
    pub expected_input_len: usize,
    /// Exact final image length after all planned writes.
    pub output_len: usize,
}

/// The identity of the typed assembly source that produced a machine-code write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineCodeProvenance {
    /// Project-defined identity of the independent assembly source.
    pub assembly_source_id: String,
    /// Exact typed ISA profile used to decode and reassemble the write.
    pub isa_profile_id: String,
}

/// The role of the bytes in a write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteIntent {
    /// Write non-executable payload bytes.
    Data,
    /// Write structural metadata bytes.
    Metadata,
    /// Write executable bytes derived from the declared assembly source.
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

/// An immutable-input precondition and the final bytes owned by one producer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedWrite {
    /// Plan-unique identity used in diagnostics and audit results.
    pub id: String,
    /// Producer with sole ownership of this byte range.
    pub owner: String,
    /// Human-readable reason for changing the bytes.
    pub purpose: String,
    /// Start offset of the write in the final image.
    pub offset: usize,
    /// Exact original bytes required where the write overlaps immutable input.
    pub expected_original: Vec<u8>,
    /// Exact final bytes owned by this write.
    pub replacement: Vec<u8>,
    /// Semantic role used to match the write to its containing region.
    pub intent: WriteIntent,
}

/// Write context supplied to a machine-code verifier.
pub struct MachineCodeCheck<'a> {
    /// Declared machine-code region that contains the complete write.
    pub region: &'a ImageRegion,
    /// Machine-code write being independently verified.
    pub write: &'a ExpectedWrite,
    /// Declared assembly-source and ISA-profile identities.
    pub provenance: &'a MachineCodeProvenance,
    /// Complete immutable input image against which the plan is checked.
    pub baseline: &'a [u8],
}

/// One sequential static decode result relative to the start of a write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedInstruction {
    /// Byte offset relative to the start of the machine-code write.
    pub offset: usize,
    /// Non-zero number of bytes consumed by this instruction.
    pub len: usize,
    /// Canonical, non-empty instruction identity accepted by the assembler.
    pub canonical: String,
}

/// A verification failure reported by a project source resolver or ISA adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineCodeVerifierError {
    message: String,
}

impl MachineCodeVerifierError {
    /// Construct a verifier failure with a diagnostic suitable for plan errors.
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

/// An adapter that independently verifies a declared assembly source and ISA profile.
pub trait MachineCodeVerifier {
    /// Assemble the independently resolved source declared by the write.
    ///
    /// The returned bytes must equal the write replacement exactly. A verifier
    /// must resolve `assembly_source_id` from project-owned configuration rather
    /// than treating the replacement bytes as assembly input.
    fn assemble_source(
        &self,
        check: &MachineCodeCheck<'_>,
    ) -> Result<Vec<u8>, MachineCodeVerifierError>;

    /// Sequentially decode the complete replacement under the declared profile.
    ///
    /// Instruction offsets are relative to the write and are checked for gaps,
    /// zero lengths, missing canonical identities, and incomplete coverage.
    fn disassemble(
        &self,
        check: &MachineCodeCheck<'_>,
    ) -> Result<Vec<DecodedInstruction>, MachineCodeVerifierError>;

    /// Assemble the decoded canonical instructions independently of source assembly.
    ///
    /// The returned bytes must equal the replacement exactly, establishing a
    /// decode/reassembly check in addition to source/replacement equality.
    fn assemble_decoded(
        &self,
        check: &MachineCodeCheck<'_>,
        instructions: &[DecodedInstruction],
    ) -> Result<Vec<u8>, MachineCodeVerifierError>;
}

/// A global plan validated in full before any write is applied.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WritePlan {
    /// Optional single-owner image-length change.
    pub resize: Option<ResizePlan>,
    /// Complete set of non-overlapping semantic regions in the output image.
    pub regions: Vec<ImageRegion>,
    /// Complete set of non-overlapping byte changes owned by producers.
    pub writes: Vec<ExpectedWrite>,
}

impl WritePlan {
    /// Construct an empty write plan without a resize, regions, or writes.
    pub fn new() -> Self {
        Self::default()
    }

    /// Declare the sole operation permitted to change the image length.
    pub fn resize(mut self, resize: ResizePlan) -> Self {
        self.resize = Some(resize);
        self
    }

    /// Add one semantic region to the plan.
    pub fn region(mut self, region: ImageRegion) -> Self {
        self.regions.push(region);
        self
    }

    /// Add one immutable-input-guarded write to the plan.
    pub fn write(mut self, write: ExpectedWrite) -> Self {
        self.writes.push(write);
        self
    }

    /// Validate the complete plan against immutable input, then build the final image.
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

    /// Apply the same rules to an externally produced image and reject unplanned changes.
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

/// A fail-closed reason that a write plan or produced image was rejected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WritePlanError {
    /// A required identity, owner, purpose, reason, or provenance field is blank.
    #[error("{label} is empty")]
    EmptyText {
        /// Name of the required field.
        label: &'static str,
    },
    /// The immutable input length differs from the resize precondition.
    #[error("resize expected input length {expected}, got {actual}")]
    ResizeInputLengthMismatch {
        /// Length declared by the resize plan.
        expected: usize,
        /// Actual immutable-input length.
        actual: usize,
    },
    /// A resize plan was supplied but declares the existing image length.
    #[error("resize plan does not change the image length")]
    ResizeDoesNotChangeLength,
    /// Two regions use the same plan-local identifier.
    #[error("duplicate region id {id}")]
    DuplicateRegionId {
        /// Identifier shared by the conflicting regions.
        id: String,
    },
    /// A region has an empty, reversed, or out-of-bounds output range.
    #[error("region {id} is outside the planned output")]
    RegionOutsideOutput {
        /// Identifier of the invalid region.
        id: String,
    },
    /// A protected region extends into bytes not present in immutable input.
    #[error("protected region {id} is outside the original image")]
    ProtectedRegionOutsideOriginal {
        /// Identifier of the invalid protected region.
        id: String,
    },
    /// Two declared semantic regions intersect.
    #[error("region {first_id} overlaps {second_id}")]
    OverlappingRegions {
        /// Identifier of the earlier declared region.
        first_id: String,
        /// Identifier of the later overlapping region.
        second_id: String,
    },
    /// A write owns no replacement bytes.
    #[error("Expected Write {write_id} is empty")]
    EmptyWrite {
        /// Identifier of the empty write.
        write_id: String,
    },
    /// Computing the write's exclusive end offset overflowed `usize`.
    #[error("Expected Write {write_id} range overflow")]
    WriteRangeOverflow {
        /// Identifier of the overflowing write.
        write_id: String,
    },
    /// Two writes use the same plan-local identifier.
    #[error("duplicate Expected Write id {id}")]
    DuplicateWriteId {
        /// Identifier shared by the conflicting writes.
        id: String,
    },
    /// A replacement extends beyond the planned final image.
    #[error("Expected Write {write_id} is outside the planned output")]
    WriteOutsideOutput {
        /// Identifier of the out-of-bounds write.
        write_id: String,
    },
    /// The original-byte precondition has the wrong length for its input overlap.
    #[error("Expected Write {write_id} has {actual} original bytes; {expected} required")]
    OriginalLengthMismatch {
        /// Identifier of the invalid write.
        write_id: String,
        /// Number of bytes in the write that overlap immutable input.
        expected: usize,
        /// Number of supplied original precondition bytes.
        actual: usize,
    },
    /// The immutable input does not equal the declared original bytes.
    #[error("Expected Write {write_id} original-byte precondition failed")]
    OriginalBytesMismatch {
        /// Identifier of the write whose precondition failed.
        write_id: String,
    },
    /// Two producers claim intersecting final byte ranges.
    #[error("Expected Write {first_id} by {first_owner} overlaps {second_id} by {second_owner}")]
    OverlappingWrites {
        /// Identifier of the earlier declared write.
        first_id: String,
        /// Owner of the earlier declared write.
        first_owner: String,
        /// Identifier of the later overlapping write.
        second_id: String,
        /// Owner of the later overlapping write.
        second_owner: String,
    },
    /// A write is not contained by exactly one declared semantic region.
    #[error("Expected Write {write_id} is contained by {containing_regions} declared regions")]
    WriteRegionOwnership {
        /// Identifier of the write without singular region ownership.
        write_id: String,
        /// Number of regions that completely contain the write.
        containing_regions: usize,
    },
    /// A write attempts to change a region declared immutable.
    #[error("Expected Write {write_id} targets protected region {region_id}")]
    ProtectedRegionWrite {
        /// Identifier of the rejected write.
        write_id: String,
        /// Identifier of the protected region.
        region_id: String,
    },
    /// A write's declared role differs from the role of its containing region.
    #[error(
        "Expected Write {write_id} intent {intent:?} does not match region {region_id} kind {region_kind:?}"
    )]
    IntentRegionMismatch {
        /// Identifier of the rejected write.
        write_id: String,
        /// Identifier of the containing region.
        region_id: String,
        /// Region kind implied by the write intent.
        intent: RegionKind,
        /// Declared kind of the containing region.
        region_kind: RegionKind,
    },
    /// A machine-code write was checked without an independent verifier.
    #[error("Expected Write {write_id} targets machine code without a verifier")]
    MachineCodeVerifierMissing {
        /// Identifier of the unverifiable machine-code write.
        write_id: String,
    },
    /// A project source resolver or typed ISA adapter failed during verification.
    #[error("Expected Write {write_id} machine-code {stage} failed: {message}")]
    MachineCodeVerification {
        /// Identifier of the machine-code write being checked.
        write_id: String,
        /// Verification operation that returned the failure.
        stage: &'static str,
        /// Diagnostic supplied by the verifier adapter.
        message: String,
    },
    /// Independent assembly source does not produce the declared replacement.
    #[error("Expected Write {write_id} differs from assembled source {assembly_source_id}")]
    AssembledSourceMismatch {
        /// Identifier of the inconsistent write.
        write_id: String,
        /// Identity of the source that produced different bytes.
        assembly_source_id: String,
    },
    /// The declared ISA profile decoded no instruction from the replacement.
    #[error("Expected Write {write_id} has no decoded instruction under {isa_profile_id}")]
    NoDecodedInstructions {
        /// Identifier of the undecodable write.
        write_id: String,
        /// Profile under which decoding was attempted.
        isa_profile_id: String,
    },
    /// Sequential decoding left a gap or reported an overlapping offset.
    #[error(
        "Expected Write {write_id} decode continues at {actual_offset:#X}, expected {expected_offset:#X}"
    )]
    DecodedInstructionGap {
        /// Identifier of the write with a non-contiguous decode.
        write_id: String,
        /// Required next offset after preceding instructions.
        expected_offset: usize,
        /// Offset reported by the verifier.
        actual_offset: usize,
    },
    /// A decoded instruction claims to consume zero bytes.
    #[error("Expected Write {write_id} has an empty decoded instruction at {offset:#X}")]
    EmptyDecodedInstruction {
        /// Identifier of the write containing the invalid instruction.
        write_id: String,
        /// Relative offset of the invalid instruction.
        offset: usize,
    },
    /// A decoded instruction lacks a canonical identity for reassembly.
    #[error("Expected Write {write_id} has no canonical instruction at {offset:#X}")]
    MissingCanonicalInstruction {
        /// Identifier of the write containing the invalid instruction.
        write_id: String,
        /// Relative offset of the invalid instruction.
        offset: usize,
    },
    /// Summing decoded instruction lengths overflowed `usize`.
    #[error("Expected Write {write_id} decoded length overflow")]
    DecodedLengthOverflow {
        /// Identifier of the write whose decoded length overflowed.
        write_id: String,
    },
    /// Sequential decoding did not consume the complete replacement.
    #[error("Expected Write {write_id} decoded {decoded_len} of {replacement_len} bytes")]
    IncompleteMachineCodeDecode {
        /// Identifier of the incompletely decoded write.
        write_id: String,
        /// Total number of bytes claimed by decoded instructions.
        decoded_len: usize,
        /// Exact number of replacement bytes that must be consumed.
        replacement_len: usize,
    },
    /// Reassembling canonical decoded instructions changed the replacement bytes.
    #[error("Expected Write {write_id} does not reassemble under {isa_profile_id}")]
    DecodedReassemblyMismatch {
        /// Identifier of the non-round-tripping write.
        write_id: String,
        /// Profile used for decoding and reassembly.
        isa_profile_id: String,
    },
    /// A newly allocated output byte has no declared write owner.
    #[error("grown output byte {offset:#X} has no Expected Write owner")]
    UnownedGrownByte {
        /// Absolute image offset without an owner.
        offset: usize,
    },
    /// An audited image has a length different from the validated plan.
    #[error("output length {actual} differs from planned length {expected}")]
    OutputLengthMismatch {
        /// Final image length required by the plan.
        expected: usize,
        /// Length of the audited image.
        actual: usize,
    },
    /// An audited image does not contain a write's exact replacement bytes.
    #[error("output does not contain planned bytes for Expected Write {write_id}")]
    PlannedBytesMismatch {
        /// Identifier of the write whose final bytes differ.
        write_id: String,
    },
    /// An audited image changed bytes in a protected region.
    #[error("protected region {region_id} changed")]
    ProtectedRegionChanged {
        /// Identifier of the changed protected region.
        region_id: String,
    },
    /// An audited image differs from immutable input outside all planned writes.
    #[error("untracked final diff at offset {offset:#X}")]
    UntrackedFinalDiff {
        /// First absolute offset containing an unplanned change.
        offset: usize,
    },
}
