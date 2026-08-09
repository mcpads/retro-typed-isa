use crate::{
    AnalysisPoint, CodeLocation, DecodeError, Instruction, InterruptState, V30, decode_bytes,
    encode_bytes,
};
use typed_isa_core::StaticSemantics;

#[derive(Debug, Clone, PartialEq, Eq)]
/// A verified instruction, its segment-relative offset, and control-flow effect.
pub struct PlacedInstruction {
    /// The segmented architectural code location.
    pub location: crate::CodeLocation,
    /// The byte offset or encoded displacement.
    pub offset: usize,
    /// The encoded instruction length in bytes.
    pub byte_len: usize,
    /// The typed instruction.
    pub instruction: Instruction,
    /// The static control-flow effect.
    pub control_flow: crate::ControlFlow,
}

/// Strictly decodes and verifies a complete placed program.
pub fn verify_placed_program(
    bytes: &[u8],
    origin: crate::CodeLocation,
) -> Result<Vec<PlacedInstruction>, ProgramVerificationError> {
    ensure_segment_capacity(origin, bytes.len())?;
    let mut instructions = Vec::new();
    let mut offset = 0usize;
    while offset < bytes.len() {
        let location = CodeLocation {
            seg: origin.seg,
            off: origin.off + offset as u16,
        };
        let decoded = decode_bytes(&bytes[offset..])
            .map_err(|source| ProgramVerificationError::Decode { location, source })?;
        let instruction_bytes = &bytes[offset..offset + decoded.byte_len];
        let reencoded = encode_bytes(&decoded.instruction)
            .map_err(|source| ProgramVerificationError::Encode { location, source })?;
        if reencoded != instruction_bytes {
            return Err(ProgramVerificationError::NonCanonicalEncoding { location });
        }
        let control_flow = V30::semantics(
            &decoded.instruction,
            &AnalysisPoint::new(location, InterruptState::Unknown),
        )
        .expect("a canonically re-encoded instruction has static semantics")
        .control_flow;
        instructions.push(PlacedInstruction {
            location,
            offset,
            byte_len: decoded.byte_len,
            instruction: decoded.instruction,
            control_flow,
        });
        offset += decoded.byte_len;
    }
    Ok(instructions)
}

pub(crate) fn ensure_segment_capacity(
    origin: CodeLocation,
    byte_len: usize,
) -> Result<(), ProgramVerificationError> {
    if usize::from(origin.off) + byte_len > 0x1_0000 {
        return Err(ProgramVerificationError::CodeSegmentBoundary { origin, byte_len });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
/// Failure returned while decoding and verifying a placed native V30 byte stream.
pub enum ProgramVerificationError {
    #[error("program at {origin:?} with {byte_len} bytes crosses the 64 KiB code segment")]
    /// The code segment boundary form.
    CodeSegmentBoundary {
        /// The first instruction code location.
        origin: CodeLocation,
        /// The encoded instruction length in bytes.
        byte_len: usize,
    },
    #[error("decode failed at {location:?}: {source}")]
    /// The decode form.
    Decode {
        /// The segmented code location.
        location: CodeLocation,
        /// The underlying decode or encode failure.
        source: DecodeError,
    },
    #[error("encode failed at {location:?}: {source}")]
    /// The encode form.
    Encode {
        /// The segmented code location.
        location: CodeLocation,
        /// The underlying decode or encode failure.
        source: crate::EncodeError,
    },
    #[error("instruction at {location:?} is not in canonical generated-code form")]
    /// The non canonical encoding form.
    NonCanonicalEncoding {
        /// The affected code location.
        location: CodeLocation,
    },
}
