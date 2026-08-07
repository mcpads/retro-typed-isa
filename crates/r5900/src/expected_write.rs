use crate::{AssembledProgram, R5900, decode_bytes, encode_bytes, format_instruction};
use expected_write::{
    DecodedInstruction as WritePlanInstruction, MachineCodeCheck, MachineCodeVerifier,
    MachineCodeVerifierError,
};

/// Expected Write machine-code verifier backed by the R5900 profile.
///
/// The resolver maps an external assembly-source identity to an independently
/// assembled program. Replacement bytes are decoded and re-encoded through the
/// same typed profile before the shared verifier admits the write.
pub struct ExpectedWriteVerifier<ResolveSource> {
    resolve_source: ResolveSource,
}

impl<ResolveSource> ExpectedWriteVerifier<ResolveSource> {
    /// Creates a verifier with the external assembly-source resolver.
    pub const fn new(resolve_source: ResolveSource) -> Self {
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
        Ok((self.resolve_source)(&check.provenance.assembly_source_id)?.into_bytes())
    }

    fn disassemble(
        &self,
        check: &MachineCodeCheck<'_>,
    ) -> Result<Vec<WritePlanInstruction>, MachineCodeVerifierError> {
        require_profile(check)?;
        decode_program(&check.write.replacement, check.write.offset as u64)
    }

    fn assemble_decoded(
        &self,
        check: &MachineCodeCheck<'_>,
        instructions: &[WritePlanInstruction],
    ) -> Result<Vec<u8>, MachineCodeVerifierError> {
        require_profile(check)?;
        let origin = check.write.offset as u64;
        if decode_program(&check.write.replacement, origin)? != instructions {
            return Err(MachineCodeVerifierError::new(
                "Expected Write decode records changed before reassembly",
            ));
        }

        let mut bytes = Vec::with_capacity(check.write.replacement.len());
        for (index, chunk) in check.write.replacement.chunks_exact(4).enumerate() {
            let address = origin + index as u64 * 4;
            let instruction = decode_bytes(chunk, address)
                .map_err(|error| MachineCodeVerifierError::new(error.to_string()))?;
            bytes.extend_from_slice(
                &encode_bytes(&instruction, address)
                    .map_err(|error| MachineCodeVerifierError::new(error.to_string()))?,
            );
        }
        Ok(bytes)
    }
}

fn decode_program(
    bytes: &[u8],
    origin: u64,
) -> Result<Vec<WritePlanInstruction>, MachineCodeVerifierError> {
    if bytes.len() % 4 != 0 {
        return Err(MachineCodeVerifierError::new(
            "R5900 machine-code write length is not a multiple of four bytes",
        ));
    }
    bytes
        .chunks_exact(4)
        .enumerate()
        .map(|(index, chunk)| {
            let offset = index * 4;
            let instruction = decode_bytes(chunk, origin + offset as u64)
                .map_err(|error| MachineCodeVerifierError::new(error.to_string()))?;
            Ok(WritePlanInstruction {
                offset,
                len: 4,
                canonical: format_instruction(&instruction),
            })
        })
        .collect()
}

fn require_profile(check: &MachineCodeCheck<'_>) -> Result<(), MachineCodeVerifierError> {
    if check.provenance.isa_profile_id != <R5900 as typed_isa_core::TypedIsa>::PROFILE_ID {
        return Err(MachineCodeVerifierError::new(format!(
            "ISA profile {} is not {}",
            check.provenance.isa_profile_id,
            <R5900 as typed_isa_core::TypedIsa>::PROFILE_ID,
        )));
    }
    Ok(())
}
