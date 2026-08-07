use std::collections::BTreeMap;

use thiserror::Error;
use typed_isa_core::{InstructionBytes, InstructionSpan};

use crate::{EncodeError, GeneralRegister, Instruction, InstructionForm, Operands, encode_bytes};

#[derive(Debug, Clone)]
enum Node {
    Label(String),
    Instruction(Instruction),
    BranchTwo {
        form: InstructionForm,
        left: GeneralRegister,
        right: GeneralRegister,
        label: String,
    },
    BranchOne {
        form: InstructionForm,
        source: GeneralRegister,
        label: String,
    },
    Branch {
        form: InstructionForm,
        label: String,
    },
    Jump {
        form: InstructionForm,
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

    pub fn branch_two_to_label(
        &mut self,
        form: InstructionForm,
        left: GeneralRegister,
        right: GeneralRegister,
        label: impl Into<String>,
    ) -> &mut Self {
        self.nodes.push(Node::BranchTwo {
            form,
            left,
            right,
            label: label.into(),
        });
        self
    }

    pub fn branch_one_to_label(
        &mut self,
        form: InstructionForm,
        source: GeneralRegister,
        label: impl Into<String>,
    ) -> &mut Self {
        self.nodes.push(Node::BranchOne {
            form,
            source,
            label: label.into(),
        });
        self
    }

    pub fn coprocessor_branch_to_label(
        &mut self,
        form: InstructionForm,
        label: impl Into<String>,
    ) -> &mut Self {
        self.nodes.push(Node::Branch {
            form,
            label: label.into(),
        });
        self
    }

    pub fn jump_to_label(&mut self, form: InstructionForm, label: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::Jump {
            form,
            label: label.into(),
        });
        self
    }

    /// Emit declared non-executable data without treating it as an instruction.
    pub fn data(&mut self, bytes: impl Into<Vec<u8>>) -> &mut Self {
        self.nodes.push(Node::Data(bytes.into()));
        self
    }

    pub fn assemble(&self, origin: u64) -> Result<AssembledProgram, AssemblerError> {
        validate_origin(origin)?;
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
                Node::BranchTwo {
                    form,
                    left,
                    right,
                    label,
                } => Instruction::new(
                    *form,
                    Operands::BranchTwo {
                        left: *left,
                        right: *right,
                        target: label_address(&labels, label)?,
                    },
                )?,
                Node::BranchOne {
                    form,
                    source,
                    label,
                } => Instruction::new(
                    *form,
                    Operands::BranchOne {
                        source: *source,
                        target: label_address(&labels, label)?,
                    },
                )?,
                Node::Branch { form, label } => Instruction::new(
                    *form,
                    Operands::Branch {
                        target: label_address(&labels, label)?,
                    },
                )?,
                Node::Jump { form, label } => Instruction::new(
                    *form,
                    Operands::Jump {
                        target: label_address(&labels, label)?,
                    },
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
                    .expect("R5900 instruction bytes are non-empty"),
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
    ) -> Result<(BTreeMap<String, u32>, usize), AssemblerError> {
        let mut labels = BTreeMap::new();
        let mut offset = 0usize;
        for node in &self.nodes {
            if let Node::Label(label) = node {
                let address = u32::try_from(placed_address(origin, offset)?)
                    .map_err(|_| AssemblerError::AddressOverflow { origin, offset })?;
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
                    .ok_or(AssemblerError::AddressOverflow { origin, offset })?;
                validate_extent(origin, offset)?;
            }
        }
        Ok((labels, offset))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledProgram {
    origin: u64,
    bytes: Vec<u8>,
    labels: BTreeMap<String, u32>,
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

    pub fn labels(&self) -> &BTreeMap<String, u32> {
        &self.labels
    }

    pub fn instruction_spans(&self) -> &[InstructionSpan<Instruction>] {
        &self.instruction_spans
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AssemblerError {
    #[error("R5900 origin {origin:#x} is not four-byte aligned")]
    UnalignedOrigin { origin: u64 },
    #[error("R5900 origin {origin:#x} exceeds the 32-bit address space")]
    OriginOutOfRange { origin: u64 },
    #[error("duplicate label {label}")]
    DuplicateLabel { label: String },
    #[error("undefined label {label}")]
    UndefinedLabel { label: String },
    #[error("instruction at output offset {offset:#x} is not four-byte aligned")]
    UnalignedInstruction { offset: usize },
    #[error("address overflow from origin {origin:#x} at output offset {offset:#x}")]
    AddressOverflow { origin: u64, offset: usize },
    #[error(transparent)]
    Construction(#[from] crate::ConstructionError),
    #[error(transparent)]
    Encode(#[from] EncodeError),
}

fn validate_origin(origin: u64) -> Result<(), AssemblerError> {
    if origin & 3 != 0 {
        return Err(AssemblerError::UnalignedOrigin { origin });
    }
    if origin > u64::from(u32::MAX) {
        return Err(AssemblerError::OriginOutOfRange { origin });
    }
    Ok(())
}

fn placed_address(origin: u64, offset: usize) -> Result<u64, AssemblerError> {
    let offset_u64 =
        u64::try_from(offset).map_err(|_| AssemblerError::AddressOverflow { origin, offset })?;
    let address = origin
        .checked_add(offset_u64)
        .ok_or(AssemblerError::AddressOverflow { origin, offset })?;
    if address > u64::from(u32::MAX) {
        return Err(AssemblerError::AddressOverflow { origin, offset });
    }
    Ok(address)
}

fn validate_extent(origin: u64, byte_len: usize) -> Result<(), AssemblerError> {
    let byte_len_u64 = u64::try_from(byte_len).map_err(|_| AssemblerError::AddressOverflow {
        origin,
        offset: byte_len,
    })?;
    let end = origin
        .checked_add(byte_len_u64)
        .ok_or(AssemblerError::AddressOverflow {
            origin,
            offset: byte_len,
        })?;
    if end > u64::from(u32::MAX) + 1 {
        return Err(AssemblerError::AddressOverflow {
            origin,
            offset: byte_len,
        });
    }
    Ok(())
}

fn label_address(labels: &BTreeMap<String, u32>, label: &str) -> Result<u32, AssemblerError> {
    labels
        .get(label)
        .copied()
        .ok_or_else(|| AssemblerError::UndefinedLabel {
            label: label.to_owned(),
        })
}
