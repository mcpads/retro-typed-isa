use std::collections::BTreeMap;

use crate::{
    Condition, ControlFlow, EncodeError, Instruction, InstructionAddress, JmpTarget, LoopCondition,
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
pub struct Assembler {
    items: Vec<AssemblyItem>,
}

impl Assembler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn label(&mut self, name: impl Into<String>) -> &mut Self {
        self.items.push(AssemblyItem::Label(name.into()));
        self
    }

    pub fn emit(&mut self, instruction: Instruction) -> &mut Self {
        self.items.push(AssemblyItem::Instruction(instruction));
        self
    }

    pub fn emit_branch(&mut self, condition: Condition, label: impl Into<String>) -> &mut Self {
        self.items.push(AssemblyItem::Relative8 {
            operation: Relative8Operation::Branch(condition),
            label: label.into(),
        });
        self
    }

    pub fn emit_loop(&mut self, condition: LoopCondition, label: impl Into<String>) -> &mut Self {
        self.items.push(AssemblyItem::Relative8 {
            operation: Relative8Operation::Loop(condition),
            label: label.into(),
        });
        self
    }

    pub fn emit_jump_short(&mut self, label: impl Into<String>) -> &mut Self {
        self.items.push(AssemblyItem::Relative8 {
            operation: Relative8Operation::Jump,
            label: label.into(),
        });
        self
    }

    pub fn emit_jump_near(&mut self, label: impl Into<String>) -> &mut Self {
        self.items.push(AssemblyItem::Relative16 {
            operation: Relative16Operation::Jump,
            label: label.into(),
        });
        self
    }

    pub fn emit_call_near(&mut self, label: impl Into<String>) -> &mut Self {
        self.items.push(AssemblyItem::Relative16 {
            operation: Relative16Operation::Call,
            label: label.into(),
        });
        self
    }

    pub fn assemble(&self, origin: InstructionAddress) -> Result<AssembledProgram, AssemblerError> {
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

        let instructions = verify_placed_program(&bytes, origin).map_err(AssemblerError::Verify)?;
        let labels = labels
            .into_iter()
            .map(|(name, offset)| {
                (
                    name,
                    InstructionAddress {
                        seg: origin.seg,
                        off: origin.off + offset as u16,
                    },
                )
            })
            .collect();
        Ok(AssembledProgram {
            origin,
            bytes,
            instructions,
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
pub struct AssembledProgram {
    origin: InstructionAddress,
    bytes: Vec<u8>,
    instructions: Vec<PlacedInstruction>,
    labels: BTreeMap<String, InstructionAddress>,
}

impl AssembledProgram {
    pub fn origin(&self) -> InstructionAddress {
        self.origin
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn instruction_spans(&self) -> &[PlacedInstruction] {
        &self.instructions
    }

    pub fn label_address(&self, label: &str) -> Option<InstructionAddress> {
        self.labels.get(label).copied()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AssemblerError {
    #[error("label is empty")]
    EmptyLabel,
    #[error("duplicate label {label}")]
    DuplicateLabel { label: String },
    #[error("undefined label {label}")]
    UndefinedLabel { label: String },
    #[error("8-bit relative displacement {displacement} to {label} is out of range")]
    Relative8OutOfRange { label: String, displacement: i64 },
    #[error("16-bit relative displacement {displacement} to {label} is out of range")]
    Relative16OutOfRange { label: String, displacement: i64 },
    #[error("instruction at byte offset {offset:#X} cannot be encoded: {source}")]
    Encode { offset: usize, source: EncodeError },
    #[error("assembled program length overflow")]
    ProgramLengthOverflow,
    #[error(transparent)]
    Verify(ProgramVerificationError),
}

#[allow(dead_code)]
fn _keep_control_flow_name(_: ControlFlow) {}
