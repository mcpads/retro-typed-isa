use crate::{AssembledProgram, CodeLocation, R5900, format_instruction};
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
        let program = self.resolve_matching_source(check)?;
        decode_program(&check.write.replacement, &program)
    }

    fn assemble_decoded(
        &self,
        check: &MachineCodeCheck<'_>,
        instructions: &[WritePlanInstruction],
    ) -> Result<Vec<u8>, MachineCodeVerifierError> {
        require_profile(check)?;
        let program = self.resolve_matching_source(check)?;
        let origin = source_origin(&program)?;
        if decode_program(&check.write.replacement, &program)? != instructions {
            return Err(MachineCodeVerifierError::new(
                "Expected Write decode records changed before reassembly",
            ));
        }

        let mut bytes = Vec::with_capacity(check.write.replacement.len());
        for (index, chunk) in check.write.replacement.chunks_exact(4).enumerate() {
            let location = placed_location(origin, index * 4)?;
            let decoded = <R5900 as typed_isa_core::TypedIsa>::decode_one(chunk, &location)
                .map_err(|error| MachineCodeVerifierError::new(error.to_string()))?;
            let encoded =
                <R5900 as typed_isa_core::TypedIsa>::encode_one(decoded.instruction(), &location)
                    .map_err(|error| MachineCodeVerifierError::new(error.to_string()))?;
            bytes.extend_from_slice(encoded.as_slice());
        }
        Ok(bytes)
    }
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
                "resolved R5900 source changed before placed verification",
            ));
        }
        Ok(program)
    }
}

fn decode_program(
    bytes: &[u8],
    program: &AssembledProgram,
) -> Result<Vec<WritePlanInstruction>, MachineCodeVerifierError> {
    if bytes.len() % 4 != 0 {
        return Err(MachineCodeVerifierError::new(
            "R5900 machine-code write length is not a multiple of four bytes",
        ));
    }
    let origin = source_origin(program)?;
    let decoded = bytes
        .chunks_exact(4)
        .enumerate()
        .map(|(index, chunk)| {
            let offset = index * 4;
            let location = placed_location(origin, offset)?;
            let instruction = <R5900 as typed_isa_core::TypedIsa>::decode_one(chunk, &location)
                .map_err(|error| MachineCodeVerifierError::new(error.to_string()))?;
            let source_span = program.instruction_spans().get(index).ok_or_else(|| {
                MachineCodeVerifierError::new(
                    "resolved R5900 source does not cover every replacement instruction",
                )
            })?;
            if source_span.offset != offset
                || source_span.location != location
                || source_span.bytes.as_slice() != chunk
                || source_span.instruction != *instruction.instruction()
            {
                return Err(MachineCodeVerifierError::new(
                    "resolved R5900 source placement or instruction identity changed",
                ));
            }
            Ok(WritePlanInstruction {
                offset,
                len: 4,
                canonical: format_instruction(instruction.instruction()),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if decoded.len() != program.instruction_spans().len() {
        return Err(MachineCodeVerifierError::new(
            "resolved R5900 source contains non-instruction bytes",
        ));
    }
    Ok(decoded)
}

fn source_origin(program: &AssembledProgram) -> Result<CodeLocation, MachineCodeVerifierError> {
    CodeLocation::try_from(program.origin())
        .map_err(|_| MachineCodeVerifierError::new("resolved R5900 source origin exceeds 32 bits"))
}

fn placed_location(
    origin: CodeLocation,
    offset: usize,
) -> Result<CodeLocation, MachineCodeVerifierError> {
    let offset = CodeLocation::try_from(offset)
        .map_err(|_| MachineCodeVerifierError::new("R5900 code offset exceeds 32 bits"))?;
    origin.checked_add(offset).ok_or_else(|| {
        MachineCodeVerifierError::new("R5900 code location overflowed during verification")
    })
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
