use std::collections::BTreeMap;

use thiserror::Error;

use crate::{Condition, DecodeError, Instruction, decode_bytes, encode_bytes};

#[derive(Debug, Clone)]
enum Node {
    Label(String),
    Instruction(Instruction),
    RelativeBranch {
        condition: Option<Condition>,
        label: String,
    },
    AbsoluteBranch {
        branch: AbsoluteBranch,
        label: String,
    },
    Data(Vec<u8>),
    LabelWord(String),
}

#[derive(Debug, Clone, Copy)]
enum AbsoluteBranch {
    Jump,
    JumpIf(Condition),
    Call,
    CallIf(Condition),
}

/// Two-pass SM83 assembler with checked label placement.
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

    pub fn emit_all(&mut self, instructions: impl IntoIterator<Item = Instruction>) -> &mut Self {
        self.nodes
            .extend(instructions.into_iter().map(Node::Instruction));
        self
    }

    pub fn jump_relative(&mut self, label: impl Into<String>) -> &mut Self {
        self.relative_branch(None, label)
    }

    pub fn jump_relative_if(
        &mut self,
        condition: Condition,
        label: impl Into<String>,
    ) -> &mut Self {
        self.relative_branch(Some(condition), label)
    }

    fn relative_branch(
        &mut self,
        condition: Option<Condition>,
        label: impl Into<String>,
    ) -> &mut Self {
        self.nodes.push(Node::RelativeBranch {
            condition,
            label: label.into(),
        });
        self
    }

    pub fn jump(&mut self, label: impl Into<String>) -> &mut Self {
        self.absolute_branch(AbsoluteBranch::Jump, label)
    }

    pub fn jump_if(&mut self, condition: Condition, label: impl Into<String>) -> &mut Self {
        self.absolute_branch(AbsoluteBranch::JumpIf(condition), label)
    }

    pub fn call(&mut self, label: impl Into<String>) -> &mut Self {
        self.absolute_branch(AbsoluteBranch::Call, label)
    }

    pub fn call_if(&mut self, condition: Condition, label: impl Into<String>) -> &mut Self {
        self.absolute_branch(AbsoluteBranch::CallIf(condition), label)
    }

    fn absolute_branch(&mut self, branch: AbsoluteBranch, label: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::AbsoluteBranch {
            branch,
            label: label.into(),
        });
        self
    }

    /// Emit explicitly declared data, never an executable-instruction fallback.
    pub fn data(&mut self, bytes: impl Into<Vec<u8>>) -> &mut Self {
        self.nodes.push(Node::Data(bytes.into()));
        self
    }

    pub fn word(&mut self, value: u16) -> &mut Self {
        self.data(value.to_le_bytes().to_vec())
    }

    pub fn label_word(&mut self, label: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::LabelWord(label.into()));
        self
    }

    pub fn assemble(&self, origin: u16) -> Result<AssembledProgram, AssemblerError> {
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
                Node::LabelWord(label) => {
                    bytes.extend_from_slice(&label_address(&labels, label)?.to_le_bytes());
                    continue;
                }
                Node::Instruction(instruction) => *instruction,
                Node::RelativeBranch { condition, label } => {
                    let address = placed_address(origin, bytes.len())?;
                    let target = label_address(&labels, label)?;
                    let displacement = relative_displacement(address, target, label)?;
                    match condition {
                        Some(condition) => Instruction::JumpRelativeIf(*condition, displacement),
                        None => Instruction::JumpRelative(displacement),
                    }
                }
                Node::AbsoluteBranch { branch, label } => {
                    let address = label_address(&labels, label)?;
                    match branch {
                        AbsoluteBranch::Jump => Instruction::Jump(address),
                        AbsoluteBranch::JumpIf(condition) => {
                            Instruction::JumpIf(*condition, address)
                        }
                        AbsoluteBranch::Call => Instruction::Call(address),
                        AbsoluteBranch::CallIf(condition) => {
                            Instruction::CallIf(*condition, address)
                        }
                    }
                }
            };
            let encoded = encode_bytes(&instruction);
            let offset = bytes.len();
            bytes.extend_from_slice(&encoded);
            instruction_spans.push(InstructionSpan {
                offset,
                len: encoded.len(),
                instruction,
            });
        }

        debug_assert_eq!(bytes.len(), byte_len);
        verify_instruction_spans(&bytes, &instruction_spans)?;
        Ok(AssembledProgram {
            origin,
            bytes,
            labels,
            instruction_spans,
        })
    }

    fn collect_labels(
        &self,
        origin: u16,
    ) -> Result<(BTreeMap<String, u16>, usize), AssemblerError> {
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
                .ok_or(AssemblerError::AddressOverflow {
                    origin,
                    byte_len: usize::MAX,
                })?;
            ensure_address_span(origin, offset)?;
        }
        Ok((labels, offset))
    }
}

fn node_len(node: &Node) -> usize {
    match node {
        Node::Label(_) => 0,
        Node::Instruction(instruction) => instruction.encoded_len(),
        Node::RelativeBranch { .. } => 2,
        Node::AbsoluteBranch { .. } => 3,
        Node::Data(data) => data.len(),
        Node::LabelWord(_) => 2,
    }
}

fn placed_address(origin: u16, offset: usize) -> Result<u16, AssemblerError> {
    let address =
        usize::from(origin)
            .checked_add(offset)
            .ok_or(AssemblerError::AddressOverflow {
                origin,
                byte_len: offset,
            })?;
    u16::try_from(address).map_err(|_| AssemblerError::AddressOverflow {
        origin,
        byte_len: offset,
    })
}

fn ensure_address_span(origin: u16, byte_len: usize) -> Result<(), AssemblerError> {
    if usize::from(origin)
        .checked_add(byte_len)
        .is_none_or(|end| end > 0x1_0000)
    {
        return Err(AssemblerError::AddressOverflow { origin, byte_len });
    }
    Ok(())
}

fn label_address(labels: &BTreeMap<String, u16>, label: &str) -> Result<u16, AssemblerError> {
    labels
        .get(label)
        .copied()
        .ok_or_else(|| AssemblerError::UndefinedLabel {
            label: label.to_owned(),
        })
}

fn relative_displacement(address: u16, target: u16, label: &str) -> Result<i8, AssemblerError> {
    let next = i32::from(address) + 2;
    let displacement = i32::from(target) - next;
    i8::try_from(displacement).map_err(|_| AssemblerError::RelativeBranchOutOfRange {
        label: label.to_owned(),
        address,
        target,
    })
}

fn verify_instruction_spans(bytes: &[u8], spans: &[InstructionSpan]) -> Result<(), AssemblerError> {
    for span in spans {
        let source = bytes.get(span.offset..span.offset + span.len).ok_or(
            AssemblerError::SemanticRoundTripMismatch {
                offset: span.offset,
            },
        )?;
        let decoded = decode_bytes(source)?;
        if decoded.into_instruction() != span.instruction
            || encode_bytes(&span.instruction) != source
        {
            return Err(AssemblerError::SemanticRoundTripMismatch {
                offset: span.offset,
            });
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstructionSpan {
    pub offset: usize,
    pub len: usize,
    pub instruction: Instruction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledProgram {
    origin: u16,
    bytes: Vec<u8>,
    labels: BTreeMap<String, u16>,
    instruction_spans: Vec<InstructionSpan>,
}

impl AssembledProgram {
    pub const fn origin(&self) -> u16 {
        self.origin
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn label_address(&self, name: &str) -> Option<u16> {
        self.labels.get(name).copied()
    }

    pub fn instruction_spans(&self) -> &[InstructionSpan] {
        &self.instruction_spans
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AssemblerError {
    #[error("duplicate label `{label}`")]
    DuplicateLabel { label: String },
    #[error("undefined label `{label}`")]
    UndefinedLabel { label: String },
    #[error("SM83 program from {origin:#06x} with {byte_len} byte(s) exceeds its address space")]
    AddressOverflow { origin: u16, byte_len: usize },
    #[error("relative branch at {address:#06x} cannot reach label `{label}` at {target:#06x}")]
    RelativeBranchOutOfRange {
        label: String,
        address: u16,
        target: u16,
    },
    #[error(transparent)]
    Decode(#[from] DecodeError),
    #[error("SM83 semantic byte round-trip failed at output offset {offset:#x}")]
    SemanticRoundTripMismatch { offset: usize },
}
