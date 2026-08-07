use std::collections::BTreeMap;

use thiserror::Error;
use typed_isa_core::{InstructionBytes, InstructionSpan};

use crate::{
    BranchOptions, ConditionRegisterBit, EncodeError, Instruction, InstructionForm,
    InstructionModifiers, Operand, encode_bytes,
};

#[derive(Debug, Clone)]
enum Node {
    Label(String),
    Instruction(Instruction),
    Branch {
        link: bool,
        label: String,
    },
    ConditionalBranch {
        options: BranchOptions,
        condition: ConditionRegisterBit,
        link: bool,
        label: String,
    },
    Data(Vec<u8>),
}

#[derive(Debug, Clone, Default)]
pub struct Assembler {
    nodes: Vec<Node>,
}

impl Assembler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn label(&mut self, name: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::Label(name.into()));
        self
    }

    pub fn emit(&mut self, instruction: Instruction) -> &mut Self {
        self.nodes.push(Node::Instruction(instruction));
        self
    }

    pub fn branch_to_label(&mut self, link: bool, label: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::Branch {
            link,
            label: label.into(),
        });
        self
    }

    pub fn conditional_branch_to_label(
        &mut self,
        options: BranchOptions,
        condition: ConditionRegisterBit,
        link: bool,
        label: impl Into<String>,
    ) -> &mut Self {
        self.nodes.push(Node::ConditionalBranch {
            options,
            condition,
            link,
            label: label.into(),
        });
        self
    }

    /// Emit declared non-executable data without interpreting it as an instruction.
    pub fn data(&mut self, bytes: impl Into<Vec<u8>>) -> &mut Self {
        self.nodes.push(Node::Data(bytes.into()));
        self
    }

    pub fn assemble(&self, origin: u64) -> Result<AssembledProgram, AssemblerError> {
        if origin & 3 != 0 {
            return Err(AssemblerError::UnalignedOrigin { origin });
        }
        let (labels, byte_len) = self.collect_labels(origin)?;
        let mut bytes = Vec::with_capacity(byte_len);
        let mut instruction_spans = Vec::new();

        for node in &self.nodes {
            let instruction = match node {
                Node::Label(_) => continue,
                Node::Data(data) => {
                    bytes.extend_from_slice(data);
                    continue;
                }
                Node::Instruction(instruction) => instruction.clone(),
                Node::Branch { link, label } => Instruction::with_modifiers(
                    InstructionForm::B,
                    InstructionModifiers {
                        link: *link,
                        ..InstructionModifiers::default()
                    },
                    [Operand::BranchTarget(label_target(&labels, label)?)],
                )?,
                Node::ConditionalBranch {
                    options,
                    condition,
                    link,
                    label,
                } => Instruction::with_modifiers(
                    InstructionForm::Bc,
                    InstructionModifiers {
                        link: *link,
                        ..InstructionModifiers::default()
                    },
                    [
                        Operand::BranchOptions(*options),
                        Operand::ConditionRegisterBit(*condition),
                        Operand::BranchTarget(label_target(&labels, label)?),
                    ],
                )?,
            };
            if bytes.len() & 3 != 0 {
                return Err(AssemblerError::UnalignedInstruction {
                    offset: bytes.len(),
                });
            }
            let address = placed_address(origin, bytes.len())?;
            let encoded = encode_bytes(&instruction, address)?;
            let offset = bytes.len();
            bytes.extend_from_slice(&encoded);
            instruction_spans.push(InstructionSpan {
                offset,
                address,
                bytes: InstructionBytes::new(encoded.to_vec())
                    .expect("PowerPC instructions are non-empty"),
                instruction,
            });
        }

        Ok(AssembledProgram {
            origin,
            bytes,
            labels,
            instruction_spans,
        })
    }

    fn collect_labels(
        &self,
        origin: u64,
    ) -> Result<(BTreeMap<String, u64>, usize), AssemblerError> {
        let mut labels = BTreeMap::new();
        let mut offset = 0usize;
        for node in &self.nodes {
            if let Node::Label(label) = node {
                let address = placed_address(origin, offset)?;
                if labels.insert(label.clone(), address).is_some() {
                    return Err(AssemblerError::DuplicateLabel {
                        label: label.clone(),
                    });
                }
            } else {
                let length = match node {
                    Node::Data(data) => data.len(),
                    _ => 4,
                };
                offset = offset
                    .checked_add(length)
                    .ok_or(AssemblerError::AddressOverflow { origin })?;
                placed_address(origin, offset)?;
            }
        }
        Ok((labels, offset))
    }
}

fn label_target(labels: &BTreeMap<String, u64>, label: &str) -> Result<u32, AssemblerError> {
    let address = labels
        .get(label)
        .copied()
        .ok_or_else(|| AssemblerError::UndefinedLabel {
            label: label.to_owned(),
        })?;
    u32::try_from(address).map_err(|_| AssemblerError::AddressOutOfRange { address })
}

fn placed_address(origin: u64, offset: usize) -> Result<u64, AssemblerError> {
    origin
        .checked_add(u64::try_from(offset).map_err(|_| AssemblerError::AddressOverflow { origin })?)
        .ok_or(AssemblerError::AddressOverflow { origin })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledProgram {
    origin: u64,
    bytes: Vec<u8>,
    labels: BTreeMap<String, u64>,
    instruction_spans: Vec<InstructionSpan<Instruction>>,
}

impl AssembledProgram {
    pub const fn origin(&self) -> u64 {
        self.origin
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn label_address(&self, name: &str) -> Option<u64> {
        self.labels.get(name).copied()
    }

    pub fn instruction_spans(&self) -> &[InstructionSpan<Instruction>] {
        &self.instruction_spans
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AssemblerError {
    #[error("PowerPC assembly origin {origin:#018x} is not four-byte aligned")]
    UnalignedOrigin { origin: u64 },
    #[error("PowerPC instruction at output offset {offset:#x} is not four-byte aligned")]
    UnalignedInstruction { offset: usize },
    #[error("duplicate label `{label}`")]
    DuplicateLabel { label: String },
    #[error("undefined label `{label}`")]
    UndefinedLabel { label: String },
    #[error("PowerPC assembly from {origin:#018x} exceeds the address space")]
    AddressOverflow { origin: u64 },
    #[error("PowerPC label address {address:#018x} exceeds the 32-bit address space")]
    AddressOutOfRange { address: u64 },
    #[error(transparent)]
    Encode(#[from] EncodeError),
}
