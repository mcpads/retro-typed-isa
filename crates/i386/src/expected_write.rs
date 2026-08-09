use std::marker::PhantomData;

use crate::{AssembledProgram, CodeLocation, I386Profile, format_instruction};
use expected_write::{
    DecodedInstruction as WritePlanInstruction, MachineCodeCheck, MachineCodeVerifier,
    MachineCodeVerifierError,
};

/// Expected Write machine-code verifier for one explicit 80386 code mode.
///
/// The resolver maps an external assembly-source identity to an independently
/// assembled program. Variable-length replacement bytes are decoded and
/// re-encoded through the selected profile before admission.
pub struct ExpectedWriteVerifier<Profile, ResolveSource> {
    resolve_source: ResolveSource,
    profile: PhantomData<Profile>,
}

impl<Profile, ResolveSource> ExpectedWriteVerifier<Profile, ResolveSource> {
    /// Creates a verifier with the external assembly-source resolver.
    pub fn new(resolve_source: ResolveSource) -> Self {
        Self {
            resolve_source,
            profile: PhantomData,
        }
    }
}

impl<Profile, ResolveSource> MachineCodeVerifier for ExpectedWriteVerifier<Profile, ResolveSource>
where
    Profile: I386Profile + Send + Sync,
    ResolveSource: Fn(&str) -> Result<AssembledProgram, MachineCodeVerifierError> + Send + Sync,
{
    fn assemble_source(
        &self,
        check: &MachineCodeCheck<'_>,
    ) -> Result<Vec<u8>, MachineCodeVerifierError> {
        require_profile::<Profile>(check)?;
        let program = (self.resolve_source)(&check.provenance.assembly_source_id)?;
        if program.code_mode() != Profile::CODE_MODE {
            return Err(MachineCodeVerifierError::new(
                "assembled Intel 80386 source uses the wrong code mode",
            ));
        }
        Ok(program.into_bytes())
    }

    fn disassemble(
        &self,
        check: &MachineCodeCheck<'_>,
    ) -> Result<Vec<WritePlanInstruction>, MachineCodeVerifierError> {
        require_profile::<Profile>(check)?;
        let program = self.resolve_matching_source(check)?;
        decode_program::<Profile>(&check.write.replacement, &program)
    }

    fn assemble_decoded(
        &self,
        check: &MachineCodeCheck<'_>,
        instructions: &[WritePlanInstruction],
    ) -> Result<Vec<u8>, MachineCodeVerifierError> {
        require_profile::<Profile>(check)?;
        let program = self.resolve_matching_source(check)?;
        let origin = program.origin();
        let current = decode_program::<Profile>(&check.write.replacement, &program)?;
        if current != instructions {
            return Err(MachineCodeVerifierError::new(
                "Expected Write decode records changed before reassembly",
            ));
        }

        let mut bytes = Vec::new();
        let mut offset = 0;
        while offset < check.write.replacement.len() {
            let location = placed_location(origin, offset)?;
            let decoded = Profile::decode_one(&check.write.replacement[offset..], &location)
                .map_err(|error| MachineCodeVerifierError::new(error.to_string()))?;
            let encoded = Profile::encode_one(decoded.instruction(), &location)
                .map_err(|error| MachineCodeVerifierError::new(error.to_string()))?;
            bytes.extend_from_slice(encoded.as_slice());
            offset += decoded.source().len().get();
        }
        Ok(bytes)
    }
}

impl<Profile, ResolveSource> ExpectedWriteVerifier<Profile, ResolveSource>
where
    Profile: I386Profile,
    ResolveSource: Fn(&str) -> Result<AssembledProgram, MachineCodeVerifierError>,
{
    fn resolve_matching_source(
        &self,
        check: &MachineCodeCheck<'_>,
    ) -> Result<AssembledProgram, MachineCodeVerifierError> {
        let program = (self.resolve_source)(&check.provenance.assembly_source_id)?;
        if program.code_mode() != Profile::CODE_MODE {
            return Err(MachineCodeVerifierError::new(
                "resolved Intel 80386 source uses the wrong code mode",
            ));
        }
        if program.bytes() != check.write.replacement {
            return Err(MachineCodeVerifierError::new(
                "resolved Intel 80386 source changed before placed verification",
            ));
        }
        Ok(program)
    }
}

fn decode_program<Profile: I386Profile>(
    bytes: &[u8],
    program: &AssembledProgram,
) -> Result<Vec<WritePlanInstruction>, MachineCodeVerifierError> {
    let origin = program.origin();
    let mut instructions = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        let location = placed_location(origin, offset)?;
        let decoded = Profile::decode_one(&bytes[offset..], &location)
            .map_err(|error| MachineCodeVerifierError::new(error.to_string()))?;
        let len = decoded.source().len().get();
        let source_span = program
            .instruction_spans()
            .get(instructions.len())
            .ok_or_else(|| {
                MachineCodeVerifierError::new(
                    "resolved Intel 80386 source does not cover every replacement instruction",
                )
            })?;
        if source_span.offset != offset
            || source_span.location != location
            || source_span.bytes.as_slice() != decoded.source().as_slice()
            || source_span.instruction != *decoded.instruction()
        {
            return Err(MachineCodeVerifierError::new(
                "resolved Intel 80386 source placement or instruction identity changed",
            ));
        }
        instructions.push(WritePlanInstruction {
            offset,
            len,
            canonical: format_instruction(decoded.instruction()),
        });
        offset += len;
    }
    if instructions.len() != program.instruction_spans().len() {
        return Err(MachineCodeVerifierError::new(
            "resolved Intel 80386 source contains non-instruction bytes",
        ));
    }
    Ok(instructions)
}

fn placed_location(
    origin: CodeLocation,
    offset: usize,
) -> Result<CodeLocation, MachineCodeVerifierError> {
    let offset = u32::try_from(offset)
        .map_err(|_| MachineCodeVerifierError::new("Intel 80386 code offset exceeds 32 bits"))?;
    let placed_offset = origin.offset().checked_add(offset).ok_or_else(|| {
        MachineCodeVerifierError::new("Intel 80386 code location overflowed during verification")
    })?;
    Ok(CodeLocation::new(origin.selector(), placed_offset))
}

fn require_profile<Profile: I386Profile>(
    check: &MachineCodeCheck<'_>,
) -> Result<(), MachineCodeVerifierError> {
    if check.provenance.isa_profile_id != Profile::PROFILE_ID {
        return Err(MachineCodeVerifierError::new(format!(
            "ISA profile {} is not {}",
            check.provenance.isa_profile_id,
            Profile::PROFILE_ID,
        )));
    }
    Ok(())
}
