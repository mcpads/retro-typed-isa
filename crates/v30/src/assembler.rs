use std::collections::BTreeMap;

use typed_isa_core::InstructionBytes;

use crate::{
    CodeLocation, Condition, ControlFlow, EncodeError, Instruction, JmpTarget, LoopCondition,
    PlacedInstruction, ProgramVerificationError, encode_bytes, verify_placed_program,
};

#[derive(Debug, Clone)]
enum AssemblyItem {
    Label(String),
    Instruction(Instruction),
    Relative8 {
        operation: Relative8Operation,
        label: String,
    },
    Relative16 {
        operation: Relative16Operation,
        label: String,
    },
}

#[derive(Debug, Clone, Copy)]
enum Relative8Operation {
    Branch(Condition),
    Loop(LoopCondition),
    Jump,
}

#[derive(Debug, Clone, Copy)]
enum Relative16Operation {
    Jump,
    Call,
}

#[derive(Debug, Clone, Default)]
/// A checked native V30 assembler with symbolic relative control flow.
pub struct Assembler {
    items: Vec<AssemblyItem>,
}

impl Assembler {
    /// Constructs the value after checking its architectural encoding constraints.
    pub fn new() -> Self {
        Self::default()
    }

    /// Declares a label at the current output position.
    pub fn label(&mut self, name: impl Into<String>) -> &mut Self {
        self.items.push(AssemblyItem::Label(name.into()));
        self
    }

    /// Encodes one internal typed instruction as executable bytes.
    pub fn emit(&mut self, instruction: Instruction) -> &mut Self {
        self.items.push(AssemblyItem::Instruction(instruction));
        self
    }

    /// Appends a symbolic conditional short branch.
    pub fn emit_branch(&mut self, condition: Condition, label: impl Into<String>) -> &mut Self {
        self.items.push(AssemblyItem::Relative8 {
            operation: Relative8Operation::Branch(condition),
            label: label.into(),
        });
        self
    }

    /// Appends a symbolic loop-family short branch.
    pub fn emit_loop(&mut self, condition: LoopCondition, label: impl Into<String>) -> &mut Self {
        self.items.push(AssemblyItem::Relative8 {
            operation: Relative8Operation::Loop(condition),
            label: label.into(),
        });
        self
    }

    /// Appends a symbolic short jump.
    pub fn emit_jump_short(&mut self, label: impl Into<String>) -> &mut Self {
        self.items.push(AssemblyItem::Relative8 {
            operation: Relative8Operation::Jump,
            label: label.into(),
        });
        self
    }

    /// Appends a symbolic near jump.
    pub fn emit_jump_near(&mut self, label: impl Into<String>) -> &mut Self {
        self.items.push(AssemblyItem::Relative16 {
            operation: Relative16Operation::Jump,
            label: label.into(),
        });
        self
    }

    /// Appends a symbolic near call.
    pub fn emit_call_near(&mut self, label: impl Into<String>) -> &mut Self {
        self.items.push(AssemblyItem::Relative16 {
            operation: Relative16Operation::Call,
            label: label.into(),
        });
        self
    }

    /// Resolves labels, encodes instructions, and verifies the placed program.
    pub fn assemble(
        &self,
        origin: crate::CodeLocation,
    ) -> Result<AssembledProgram, AssemblerError> {
        let (labels, byte_len) = self.collect_labels()?;
        crate::analysis::ensure_segment_capacity(origin, byte_len)
            .map_err(AssemblerError::Verify)?;

        let mut bytes = Vec::with_capacity(byte_len);
        for item in &self.items {
            let instruction = match item {
                AssemblyItem::Label(_) => continue,
                AssemblyItem::Instruction(instruction) => instruction.clone(),
                AssemblyItem::Relative8 { operation, label } => {
                    let target = label_offset(&labels, label)?;
                    let displacement = relative_displacement(bytes.len(), 2, target);
                    let displacement = i8::try_from(displacement).map_err(|_| {
                        AssemblerError::Relative8OutOfRange {
                            label: label.clone(),
                            displacement,
                        }
                    })?;
                    match operation {
                        Relative8Operation::Branch(condition) => Instruction::Jcc {
                            cond: *condition,
                            target: displacement,
                        },
                        Relative8Operation::Loop(condition) => Instruction::Loop {
                            condition: *condition,
                            target: displacement,
                        },
                        Relative8Operation::Jump => Instruction::Jmp {
                            target: JmpTarget::Rel8(displacement),
                        },
                    }
                }
                AssemblyItem::Relative16 { operation, label } => {
                    let target = label_offset(&labels, label)?;
                    let displacement = relative_displacement(bytes.len(), 3, target);
                    let displacement = i16::try_from(displacement).map_err(|_| {
                        AssemblerError::Relative16OutOfRange {
                            label: label.clone(),
                            displacement,
                        }
                    })?;
                    match operation {
                        Relative16Operation::Jump => Instruction::Jmp {
                            target: JmpTarget::Rel16(displacement),
                        },
                        Relative16Operation::Call => Instruction::Call {
                            target: crate::CallTarget::Rel16(displacement),
                        },
                    }
                }
            };
            bytes.extend(
                encode_bytes(&instruction).map_err(|source| AssemblerError::Encode {
                    offset: bytes.len(),
                    source,
                })?,
            );
        }

        let placed_instructions =
            verify_placed_program(&bytes, origin).map_err(AssemblerError::Verify)?;
        let instruction_spans = placed_instructions
            .iter()
            .map(|placed| InstructionSpan {
                offset: placed.offset,
                location: placed.location,
                bytes: InstructionBytes::new(
                    bytes[placed.offset..placed.offset + placed.byte_len].to_vec(),
                )
                .expect("encoded V30 instructions are non-empty"),
                instruction: placed.instruction.clone(),
            })
            .collect();
        let labels = labels
            .into_iter()
            .map(|(name, offset)| {
                (
                    name,
                    CodeLocation {
                        seg: origin.seg,
                        off: origin.off + offset as u16,
                    },
                )
            })
            .collect();
        Ok(AssembledProgram {
            origin,
            bytes,
            instruction_spans,
            placed_instructions,
            labels,
        })
    }

    fn collect_labels(&self) -> Result<(BTreeMap<String, usize>, usize), AssemblerError> {
        let mut labels = BTreeMap::new();
        let mut offset = 0usize;
        for item in &self.items {
            match item {
                AssemblyItem::Label(name) => {
                    if name.trim().is_empty() {
                        return Err(AssemblerError::EmptyLabel);
                    }
                    if labels.insert(name.clone(), offset).is_some() {
                        return Err(AssemblerError::DuplicateLabel {
                            label: name.clone(),
                        });
                    }
                }
                AssemblyItem::Instruction(instruction) => {
                    let bytes = encode_bytes(instruction)
                        .map_err(|source| AssemblerError::Encode { offset, source })?;
                    offset = offset
                        .checked_add(bytes.len())
                        .ok_or(AssemblerError::ProgramLengthOverflow)?;
                }
                AssemblyItem::Relative8 { .. } => {
                    offset = offset
                        .checked_add(2)
                        .ok_or(AssemblerError::ProgramLengthOverflow)?;
                }
                AssemblyItem::Relative16 { .. } => {
                    offset = offset
                        .checked_add(3)
                        .ok_or(AssemblerError::ProgramLengthOverflow)?;
                }
            }
        }
        Ok((labels, offset))
    }
}

fn label_offset(labels: &BTreeMap<String, usize>, label: &str) -> Result<usize, AssemblerError> {
    labels
        .get(label)
        .copied()
        .ok_or_else(|| AssemblerError::UndefinedLabel {
            label: label.to_owned(),
        })
}

fn relative_displacement(offset: usize, instruction_len: usize, target: usize) -> i64 {
    target as i64 - (offset + instruction_len) as i64
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Verified native V30 bytes, labels, and placed-instruction metadata.
pub struct AssembledProgram {
    origin: CodeLocation,
    bytes: Vec<u8>,
    instruction_spans: Vec<InstructionSpan>,
    placed_instructions: Vec<PlacedInstruction>,
    labels: BTreeMap<String, CodeLocation>,
}

/// Workspace-wide instruction-placement evidence specialized for V30.
pub type InstructionSpan = typed_isa_core::InstructionSpan<CodeLocation, Instruction>;

impl AssembledProgram {
    /// Performs the origin operation.
    pub fn origin(&self) -> crate::CodeLocation {
        self.origin
    }

    /// Returns the canonical representative bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consumes the assembled program and returns its bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Returns verified instruction placements.
    pub fn instruction_spans(&self) -> &[InstructionSpan] {
        &self.instruction_spans
    }

    /// Returns V30-specific placement and static control-flow analysis.
    pub fn placed_instructions(&self) -> &[PlacedInstruction] {
        &self.placed_instructions
    }

    /// Returns a resolved label code location.
    pub fn label_location(&self, label: &str) -> Option<crate::CodeLocation> {
        self.labels.get(label).copied()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
/// Failure returned while placing, resolving, or verifying a V30 assembly unit.
pub enum AssemblerError {
    #[error("label is empty")]
    /// The empty label form.
    EmptyLabel,
    #[error("duplicate label {label}")]
    /// The duplicate label form.
    DuplicateLabel {
        /// The symbolic label.
        label: String,
    },
    #[error("undefined label {label}")]
    /// The undefined label form.
    UndefinedLabel {
        /// The symbolic label.
        label: String,
    },
    #[error("8-bit relative displacement {displacement} to {label} is out of range")]
    /// The relative8 out of range form.
    Relative8OutOfRange {
        /// The symbolic label.
        label: String,
        /// The resolved relative displacement.
        displacement: i64,
    },
    #[error("16-bit relative displacement {displacement} to {label} is out of range")]
    /// The relative16 out of range form.
    Relative16OutOfRange {
        /// The symbolic label.
        label: String,
        /// The resolved relative displacement.
        displacement: i64,
    },
    #[error("instruction at byte offset {offset:#X} cannot be encoded: {source}")]
    /// The encode form.
    Encode {
        /// The output byte offset.
        offset: usize,
        /// The underlying failure.
        source: EncodeError,
    },
    #[error("assembled program length overflow")]
    /// The program length overflow form.
    ProgramLengthOverflow,
    #[error(transparent)]
    /// The verify form.
    Verify(ProgramVerificationError),
}

#[allow(dead_code)]
fn _keep_control_flow_name(_: ControlFlow) {}
