use crate::{
    Condition, Instruction, InstructionAddress, LoopCondition, PlacedInstruction,
    ProgramVerificationError, verify_placed_program,
};

#[derive(Debug, Clone, Default)]
pub struct Assembler {
    inner: v30::Assembler,
}

impl Assembler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn label(&mut self, name: impl Into<String>) -> &mut Self {
        self.inner.label(name);
        self
    }

    pub fn emit(&mut self, instruction: Instruction) -> &mut Self {
        self.inner.emit(instruction.into_v_series());
        self
    }

    pub fn emit_branch(&mut self, condition: Condition, label: impl Into<String>) -> &mut Self {
        self.inner.emit_branch(condition, label);
        self
    }

    pub fn emit_loop(&mut self, condition: LoopCondition, label: impl Into<String>) -> &mut Self {
        self.inner.emit_loop(condition, label);
        self
    }

    pub fn emit_jump_short(&mut self, label: impl Into<String>) -> &mut Self {
        self.inner.emit_jump_short(label);
        self
    }

    pub fn emit_jump_near(&mut self, label: impl Into<String>) -> &mut Self {
        self.inner.emit_jump_near(label);
        self
    }

    pub fn emit_call_near(&mut self, label: impl Into<String>) -> &mut Self {
        self.inner.emit_call_near(label);
        self
    }

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledProgram {
    inner: v30::AssembledProgram,
    instructions: Vec<PlacedInstruction>,
}

impl AssembledProgram {
    pub fn origin(&self) -> InstructionAddress {
        self.inner.origin()
    }

    pub fn bytes(&self) -> &[u8] {
        self.inner.bytes()
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.inner.into_bytes()
    }

    pub fn instruction_spans(&self) -> &[PlacedInstruction] {
        &self.instructions
    }

    pub fn label_address(&self, label: &str) -> Option<InstructionAddress> {
        self.inner.label_address(label)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AssemblerError {
    #[error(transparent)]
    BaseAssembler(v30::AssemblerError),
    #[error(transparent)]
    Verify(ProgramVerificationError),
}
