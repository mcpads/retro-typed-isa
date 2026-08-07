use std::marker::PhantomData;

use crate::{AssembledProgram, I386Profile, decode_bytes, encode_bytes, format_instruction};
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
        decode_program(&check.write.replacement, Profile::CODE_MODE)
    }

    fn assemble_decoded(
        &self,
        check: &MachineCodeCheck<'_>,
        instructions: &[WritePlanInstruction],
    ) -> Result<Vec<u8>, MachineCodeVerifierError> {
        require_profile::<Profile>(check)?;
        let current = decode_program(&check.write.replacement, Profile::CODE_MODE)?;
        if current != instructions {
            return Err(MachineCodeVerifierError::new(
                "Expected Write decode records changed before reassembly",
            ));
        }

        let mut bytes = Vec::new();
        let mut offset = 0;
        while offset < check.write.replacement.len() {
            let decoded = decode_bytes(&check.write.replacement[offset..], Profile::CODE_MODE)
                .map_err(|error| MachineCodeVerifierError::new(error.to_string()))?;
            bytes.extend(
                encode_bytes(decoded.instruction())
                    .map_err(|error| MachineCodeVerifierError::new(error.to_string()))?,
            );
            offset += decoded.byte_len();
        }
        Ok(bytes)
    }
}

fn decode_program(
    bytes: &[u8],
    code_mode: crate::CodeMode,
) -> Result<Vec<WritePlanInstruction>, MachineCodeVerifierError> {
    let mut instructions = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        let decoded = decode_bytes(&bytes[offset..], code_mode)
            .map_err(|error| MachineCodeVerifierError::new(error.to_string()))?;
        instructions.push(WritePlanInstruction {
            offset,
            len: decoded.byte_len(),
            canonical: format_instruction(decoded.instruction()),
        });
        offset += decoded.byte_len();
    }
    Ok(instructions)
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
