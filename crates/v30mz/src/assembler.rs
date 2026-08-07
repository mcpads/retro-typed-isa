use crate::{
    Condition, Instruction, InstructionAddress, LoopCondition, PlacedInstruction,
    ProgramVerificationError, verify_placed_program,
};

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
    pub fn assemble(&self, origin: InstructionAddress) -> Result<AssembledProgram, AssemblerError> {
        let inner = self
            .inner
            .assemble(origin)
            .map_err(AssemblerError::BaseAssembler)?;
        let instructions =
            verify_placed_program(inner.bytes(), origin).map_err(AssemblerError::Verify)?;
        Ok(AssembledProgram {
            inner,
            instructions,
        })
    }
}

/// Canonically encoded V30MZ bytes and their verified placement metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledProgram {
    inner: v30::AssembledProgram,
    instructions: Vec<PlacedInstruction>,
}

impl AssembledProgram {
    /// Return the segmented address assigned to the first program byte.
    pub fn origin(&self) -> InstructionAddress {
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
    pub fn instruction_spans(&self) -> &[PlacedInstruction] {
        &self.instructions
    }

    /// Return the resolved address of a defined label, if present.
    pub fn label_address(&self, label: &str) -> Option<InstructionAddress> {
        self.inner.label_address(label)
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
