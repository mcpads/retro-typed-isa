use std::collections::BTreeMap;

use thiserror::Error;

use crate::{Condition, DecodeError, EncodeError, Instruction, decode_bytes, encode_bytes};

#[derive(Debug, Clone)]
enum Node {
    Label(String),
    Instruction(Instruction),
    Branch { condition: Condition, label: String },
    JumpRelative(String),
    JumpAndLink(String),
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

    pub fn branch_to_label(&mut self, condition: Condition, label: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::Branch {
            condition,
            label: label.into(),
        });
        self
    }

    pub fn jump_relative(&mut self, label: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::JumpRelative(label.into()));
        self
    }

    pub fn jump_and_link(&mut self, label: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::JumpAndLink(label.into()));
        self
    }

    /// Emit declared non-executable data without treating it as an instruction.
    pub fn data(&mut self, bytes: impl Into<Vec<u8>>) -> &mut Self {
        self.nodes.push(Node::Data(bytes.into()));
        self
    }

    pub fn assemble(&self, origin: u32) -> Result<AssembledProgram, AssemblerError> {
        if origin & 1 != 0 {
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
                Node::Instruction(instruction) => *instruction,
                Node::Branch { condition, label } => Instruction::Branch {
                    condition: *condition,
                    displacement: i16::try_from(relative_displacement(
                        placed_address(origin, bytes.len())?,
                        label_address(&labels, label)?,
                    )?)
                    .map_err(|_| AssemblerError::BranchOutOfRange {
                        label: label.clone(),
                    })?,
                },
                Node::JumpRelative(label) => Instruction::JumpRelative(relative_displacement(
                    placed_address(origin, bytes.len())?,
                    label_address(&labels, label)?,
                )?),
                Node::JumpAndLink(label) => Instruction::JumpAndLink(relative_displacement(
                    placed_address(origin, bytes.len())?,
                    label_address(&labels, label)?,
                )?),
            };

            if bytes.len() & 1 != 0 {
                return Err(AssemblerError::UnalignedInstruction {
                    offset: bytes.len(),
                });
            }
            let address = placed_address(origin, bytes.len())?;
            let encoded = encode_bytes(&instruction)?;
            let offset = bytes.len();
            bytes.extend_from_slice(&encoded);
            instruction_spans.push(InstructionSpan {
                offset,
                address,
                instruction,
                byte_len: encoded.len(),
            });
        }

        verify_spans(&bytes, &instruction_spans)?;
        Ok(AssembledProgram {
            origin,
            bytes,
            labels,
            instruction_spans,
        })
    }

    fn collect_labels(
        &self,
        origin: u32,
    ) -> Result<(BTreeMap<String, u32>, usize), AssemblerError> {
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
                continue;
            }
            offset = offset
                .checked_add(node_len(node))
                .ok_or(AssemblerError::AddressOverflow { origin })?;
            placed_address(origin, offset)?;
        }
        Ok((labels, offset))
    }
}

fn node_len(node: &Node) -> usize {
    match node {
        Node::Label(_) => 0,
        Node::Instruction(instruction) => instruction.encoded_len(),
        Node::Branch { .. } => 2,
        Node::JumpRelative(_) | Node::JumpAndLink(_) => 4,
        Node::Data(data) => data.len(),
    }
}

fn label_address(labels: &BTreeMap<String, u32>, label: &str) -> Result<u32, AssemblerError> {
    labels
        .get(label)
        .copied()
        .ok_or_else(|| AssemblerError::UndefinedLabel {
            label: label.to_owned(),
        })
}

fn placed_address(origin: u32, offset: usize) -> Result<u32, AssemblerError> {
    origin
        .checked_add(u32::try_from(offset).map_err(|_| AssemblerError::AddressOverflow { origin })?)
        .ok_or(AssemblerError::AddressOverflow { origin })
}

fn relative_displacement(address: u32, target: u32) -> Result<i32, AssemblerError> {
    i32::try_from(i64::from(target) - i64::from(address))
        .map_err(|_| AssemblerError::RelativeAddressOutOfRange { address, target })
}

fn verify_spans(bytes: &[u8], spans: &[InstructionSpan]) -> Result<(), AssemblerError> {
    for span in spans {
        let source = bytes.get(span.offset..span.offset + span.byte_len).ok_or(
            AssemblerError::VerificationFailed {
                offset: span.offset,
            },
        )?;
        let decoded = decode_bytes(source)?;
        if decoded.instruction() != &span.instruction
            || encode_bytes(decoded.instruction())? != source
        {
            return Err(AssemblerError::VerificationFailed {
                offset: span.offset,
            });
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionSpan {
    pub offset: usize,
    pub address: u32,
    pub instruction: Instruction,
    pub byte_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledProgram {
    origin: u32,
    bytes: Vec<u8>,
    labels: BTreeMap<String, u32>,
    instruction_spans: Vec<InstructionSpan>,
}

impl AssembledProgram {
    pub const fn origin(&self) -> u32 {
        self.origin
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn label_address(&self, name: &str) -> Option<u32> {
        self.labels.get(name).copied()
    }

    pub fn instruction_spans(&self) -> &[InstructionSpan] {
        &self.instruction_spans
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AssemblerError {
    #[error("V810 assembly origin {origin:#010x} is not two-byte aligned")]
    UnalignedOrigin { origin: u32 },
    #[error("V810 instruction at output offset {offset:#x} is not two-byte aligned")]
    UnalignedInstruction { offset: usize },
    #[error("duplicate label `{label}`")]
    DuplicateLabel { label: String },
    #[error("undefined label `{label}`")]
    UndefinedLabel { label: String },
    #[error("V810 assembly from {origin:#010x} exceeds the address space")]
    AddressOverflow { origin: u32 },
    #[error("relative address from {address:#010x} to {target:#010x} exceeds signed 32-bit range")]
    RelativeAddressOutOfRange { address: u32, target: u32 },
    #[error("V810 conditional branch cannot reach label `{label}`")]
    BranchOutOfRange { label: String },
    #[error(transparent)]
    Encode(#[from] EncodeError),
    #[error(transparent)]
    Decode(#[from] DecodeError),
    #[error("V810 instruction verification failed at output offset {offset:#x}")]
    VerificationFailed { offset: usize },
}
