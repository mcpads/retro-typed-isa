use expected_write::{
    DecodedInstruction as WritePlanInstruction, MachineCodeCheck, MachineCodeVerifier,
    MachineCodeVerifierError,
};

use crate::{AssembledProgram, InstructionAddress, PROFILE_ID, encode_bytes, format_instruction};

/// Adapter connecting typed V30 assembly sources to Expected Write verification.
pub struct ExpectedWriteVerifier<ResolveSource> {
    resolve_source: ResolveSource,
}

impl<ResolveSource> ExpectedWriteVerifier<ResolveSource> {
    /// Constructs the value after checking its architectural encoding constraints.
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
        let instructions = crate::verify_placed_program(
            &check.write.replacement,
            InstructionAddress { seg: 0, off: 0 },
        )
        .map_err(|error| MachineCodeVerifierError::new(error.to_string()))?;
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
        let decoded = crate::verify_placed_program(
            &check.write.replacement,
            InstructionAddress { seg: 0, off: 0 },
        )
        .map_err(|error| MachineCodeVerifierError::new(error.to_string()))?;
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
            bytes.extend(
                encode_bytes(&placed.instruction)
                    .map_err(|error| MachineCodeVerifierError::new(error.to_string()))?,
            );
            Ok(bytes)
        })
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
