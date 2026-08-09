use expected_write::{
    DecodedInstruction as WritePlanInstruction, MachineCodeCheck, MachineCodeVerifier,
    MachineCodeVerifierError,
};

use crate::{AssembledProgram, PROFILE_ID, V30Mz, format_instruction, verify_placed_program};

/// Expected Write adapter backed by a project-controlled typed-source resolver.
///
/// The resolver receives an `assembly_source_id` and must return the assembled
/// V30MZ program associated with that external identity. Replacement bytes are
/// independently decoded and reassembled by this adapter.
pub struct ExpectedWriteVerifier<ResolveSource> {
    resolve_source: ResolveSource,
}

impl<ResolveSource> ExpectedWriteVerifier<ResolveSource> {
    /// Construct a verifier from a project-controlled assembly-source resolver.
    pub fn new(resolve_source: ResolveSource) -> Self {
        Self { resolve_source }
    }
}

impl<ResolveSource> MachineCodeVerifier for ExpectedWriteVerifier<ResolveSource>
where
    ResolveSource: Fn(&str) -> Result<AssembledProgram, MachineCodeVerifierError> + Send + Sync,
{
    fn assemble_source(
        &self,
        check: &MachineCodeCheck<'_>,
    ) -> Result<Vec<u8>, MachineCodeVerifierError> {
        require_profile(check)?;
        (self.resolve_source)(&check.provenance.assembly_source_id)
            .map(AssembledProgram::into_bytes)
    }

    fn disassemble(
        &self,
        check: &MachineCodeCheck<'_>,
    ) -> Result<Vec<WritePlanInstruction>, MachineCodeVerifierError> {
        require_profile(check)?;
        let program = self.resolve_matching_source(check)?;
        let instructions = decode_source_program(&check.write.replacement, &program)?;
        Ok(instructions
            .into_iter()
            .map(|placed| WritePlanInstruction {
                offset: placed.offset,
                len: placed.byte_len,
                canonical: format_instruction(&placed.instruction),
            })
            .collect())
    }

    fn assemble_decoded(
        &self,
        check: &MachineCodeCheck<'_>,
        instructions: &[WritePlanInstruction],
    ) -> Result<Vec<u8>, MachineCodeVerifierError> {
        require_profile(check)?;
        let program = self.resolve_matching_source(check)?;
        let decoded = decode_source_program(&check.write.replacement, &program)?;
        if decoded.len() != instructions.len()
            || decoded.iter().zip(instructions).any(|(placed, declared)| {
                placed.offset != declared.offset
                    || placed.byte_len != declared.len
                    || format_instruction(&placed.instruction) != declared.canonical
            })
        {
            return Err(MachineCodeVerifierError::new(
                "Expected Write decode records changed before reassembly",
            ));
        }

        decoded.iter().try_fold(Vec::new(), |mut bytes, placed| {
            let encoded = <V30Mz as typed_isa_core::TypedIsa>::encode_one(
                &placed.instruction,
                &placed.location,
            )
            .map_err(|error| MachineCodeVerifierError::new(error.to_string()))?;
            bytes.extend_from_slice(encoded.as_slice());
            Ok(bytes)
        })
    }
}

fn decode_source_program(
    bytes: &[u8],
    program: &AssembledProgram,
) -> Result<Vec<crate::PlacedInstruction>, MachineCodeVerifierError> {
    let decoded = verify_placed_program(bytes, program.origin())
        .map_err(|error| MachineCodeVerifierError::new(error.to_string()))?;
    if decoded.len() != program.instruction_spans().len()
        || decoded
            .iter()
            .zip(program.instruction_spans())
            .any(|(actual, source)| {
                actual.offset != source.offset
                    || actual.byte_len != source.bytes.len().get()
                    || actual.location != source.location
                    || actual.instruction != source.instruction
                    || &bytes[actual.offset..actual.offset + actual.byte_len]
                        != source.bytes.as_slice()
            })
    {
        return Err(MachineCodeVerifierError::new(
            "resolved V30MZ source placement or instruction identity changed",
        ));
    }
    Ok(decoded)
}

impl<ResolveSource> ExpectedWriteVerifier<ResolveSource>
where
    ResolveSource: Fn(&str) -> Result<AssembledProgram, MachineCodeVerifierError>,
{
    fn resolve_matching_source(
        &self,
        check: &MachineCodeCheck<'_>,
    ) -> Result<AssembledProgram, MachineCodeVerifierError> {
        let program = (self.resolve_source)(&check.provenance.assembly_source_id)?;
        if program.bytes() != check.write.replacement {
            return Err(MachineCodeVerifierError::new(
                "resolved V30MZ source changed before placed verification",
            ));
        }
        Ok(program)
    }
}

fn require_profile(check: &MachineCodeCheck<'_>) -> Result<(), MachineCodeVerifierError> {
    if check.provenance.isa_profile_id != PROFILE_ID {
        return Err(MachineCodeVerifierError::new(format!(
            "ISA profile {} is not {PROFILE_ID}",
            check.provenance.isa_profile_id
        )));
    }
    Ok(())
}
