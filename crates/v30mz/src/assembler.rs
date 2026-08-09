use crate::{
    Condition, Instruction, LoopCondition, PlacedInstruction, ProgramVerificationError,
    verify_placed_program,
};
use typed_isa_core::InstructionBytes;

/// Builder for typed V30MZ code, labels, and checked relative control flow.
#[derive(Debug, Clone, Default)]
pub struct Assembler {
    inner: v30::Assembler,
}

impl Assembler {
    /// Construct an empty assembler program.
    pub fn new() -> Self {
        Self::default()
    }

    /// Define a label at the current program position.
    pub fn label(&mut self, name: impl Into<String>) -> &mut Self {
        self.inner.label(name);
        self
    }

    /// Append one instruction already validated for the V30MZ profile.
    pub fn emit(&mut self, instruction: Instruction) -> &mut Self {
        self.inner.emit(instruction.into_v_series());
        self
    }

    /// Append a short conditional branch to a named label.
    pub fn emit_branch(&mut self, condition: Condition, label: impl Into<String>) -> &mut Self {
        self.inner.emit_branch(condition, label);
        self
    }

    /// Append a loop-family instruction targeting a named label.
    pub fn emit_loop(&mut self, condition: LoopCondition, label: impl Into<String>) -> &mut Self {
        self.inner.emit_loop(condition, label);
        self
    }

    /// Append a short relative jump to a named label.
    pub fn emit_jump_short(&mut self, label: impl Into<String>) -> &mut Self {
        self.inner.emit_jump_short(label);
        self
    }

    /// Append a near relative jump to a named label.
    pub fn emit_jump_near(&mut self, label: impl Into<String>) -> &mut Self {
        self.inner.emit_jump_near(label);
        self
    }

    /// Append a near relative call to a named label.
    pub fn emit_call_near(&mut self, label: impl Into<String>) -> &mut Self {
        self.inner.emit_call_near(label);
        self
    }

    /// Resolve labels, lower typed instructions, and verify the complete program.
    ///
    /// `origin` supplies the code segment and offset used for relative target
    /// calculations and the final 64 KiB segment-boundary check.
    pub fn assemble(
        &self,
        origin: crate::CodeLocation,
    ) -> Result<AssembledProgram, AssemblerError> {
        let inner = self
            .inner
            .assemble(origin)
            .map_err(AssemblerError::BaseAssembler)?;
        let instructions =
            verify_placed_program(inner.bytes(), origin).map_err(AssemblerError::Verify)?;
        let instruction_spans = instructions
            .iter()
            .map(|placed| InstructionSpan {
                offset: placed.offset,
                location: placed.location,
                bytes: InstructionBytes::new(
                    inner.bytes()[placed.offset..placed.offset + placed.byte_len].to_vec(),
                )
                .expect("encoded V30MZ instructions are non-empty"),
                instruction: placed.instruction.clone(),
            })
            .collect();
        Ok(AssembledProgram {
            inner,
            instruction_spans,
            placed_instructions: instructions,
        })
    }
}

/// Canonically encoded V30MZ bytes and their verified placement metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledProgram {
    inner: v30::AssembledProgram,
    instruction_spans: Vec<InstructionSpan>,
    placed_instructions: Vec<PlacedInstruction>,
}

/// Workspace-wide instruction-placement evidence specialized for V30MZ.
pub type InstructionSpan = typed_isa_core::InstructionSpan<crate::CodeLocation, Instruction>;

impl AssembledProgram {
    /// Return the segmented code location assigned to the first program byte.
    pub fn origin(&self) -> crate::CodeLocation {
        self.inner.origin()
    }

    /// Borrow the complete assembled byte sequence.
    pub fn bytes(&self) -> &[u8] {
        self.inner.bytes()
    }

    /// Consume the program and return its assembled byte sequence.
    pub fn into_bytes(self) -> Vec<u8> {
        self.inner.into_bytes()
    }

    /// Borrow the sequential instructions verified in the assembled output.
    pub fn instruction_spans(&self) -> &[InstructionSpan] {
        &self.instruction_spans
    }

    /// Borrow V30MZ-specific placement and static control-flow analysis.
    pub fn placed_instructions(&self) -> &[PlacedInstruction] {
        &self.placed_instructions
    }

    /// Return the resolved code location of a defined label, if present.
    pub fn label_location(&self, label: &str) -> Option<crate::CodeLocation> {
        self.inner.label_location(label)
    }
}

/// A fail-closed reason that typed V30MZ assembly did not produce a program.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AssemblerError {
    /// The shared V-series assembler rejected labels, placement, or lowering.
    #[error(transparent)]
    BaseAssembler(v30::AssemblerError),
    /// Final V30MZ decode and canonical reassembly verification failed.
    #[error(transparent)]
    Verify(ProgramVerificationError),
}
