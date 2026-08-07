use std::collections::BTreeMap;

use thiserror::Error;
use typed_isa_core::{InstructionBytes, InstructionSpan};

use crate::{
    EncodeError, EqualityCondition, FloatingBranchCondition, GeneralRegister, Instruction,
    SignCondition, VfpuConditionBit, VfpuForm, VfpuInstruction, VfpuOperands, encode_bytes,
};

#[derive(Debug, Clone)]
enum Node {
    Label(String),
    Instruction(Instruction),
    EqualityBranch {
        condition: EqualityCondition,
        likely: bool,
        left: GeneralRegister,
        right: GeneralRegister,
        label: String,
    },
    SignBranch {
        condition: SignCondition,
        likely: bool,
        link: bool,
        source: GeneralRegister,
        label: String,
    },
    FloatingBranch {
        condition: FloatingBranchCondition,
        likely: bool,
        label: String,
    },
    VfpuBranch {
        form: VfpuForm,
        condition: VfpuConditionBit,
        label: String,
    },
    Jump {
        link: bool,
        label: String,
    },
    Data(Vec<u8>),
}

#[derive(Debug, Clone, Default)]
/// A checked Allegrex assembler with symbolic control-flow targets and explicit data.
pub struct Assembler {
    nodes: Vec<Node>,
}

impl Assembler {
    /// Constructs the value after checking its encoding and operand-layout constraints.
    pub fn new() -> Self {
        Self::default()
    }

    /// Declares a label at the current output position.
    pub fn label(&mut self, name: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::Label(name.into()));
        self
    }

    /// Performs the emit operation.
    pub fn emit(&mut self, instruction: Instruction) -> &mut Self {
        self.nodes.push(Node::Instruction(instruction));
        self
    }

    /// Performs the equality branch to label operation.
    pub fn equality_branch_to_label(
        &mut self,
        condition: EqualityCondition,
        likely: bool,
        left: GeneralRegister,
        right: GeneralRegister,
        label: impl Into<String>,
    ) -> &mut Self {
        self.nodes.push(Node::EqualityBranch {
            condition,
            likely,
            left,
            right,
            label: label.into(),
        });
        self
    }

    /// Performs the sign branch to label operation.
    pub fn sign_branch_to_label(
        &mut self,
        condition: SignCondition,
        likely: bool,
        link: bool,
        source: GeneralRegister,
        label: impl Into<String>,
    ) -> &mut Self {
        self.nodes.push(Node::SignBranch {
            condition,
            likely,
            link,
            source,
            label: label.into(),
        });
        self
    }

    /// Performs the floating branch to label operation.
    pub fn floating_branch_to_label(
        &mut self,
        condition: FloatingBranchCondition,
        likely: bool,
        label: impl Into<String>,
    ) -> &mut Self {
        self.nodes.push(Node::FloatingBranch {
            condition,
            likely,
            label: label.into(),
        });
        self
    }

    /// Performs the vfpu branch to label operation.
    pub fn vfpu_branch_to_label(
        &mut self,
        form: VfpuForm,
        condition: VfpuConditionBit,
        label: impl Into<String>,
    ) -> &mut Self {
        self.nodes.push(Node::VfpuBranch {
            form,
            condition,
            label: label.into(),
        });
        self
    }

    /// Performs the jump to label operation.
    pub fn jump_to_label(&mut self, link: bool, label: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::Jump {
            link,
            label: label.into(),
        });
        self
    }

    /// Emit declared non-executable data without treating it as an instruction.
    pub fn data(&mut self, bytes: impl Into<Vec<u8>>) -> &mut Self {
        self.nodes.push(Node::Data(bytes.into()));
        self
    }

    /// Resolves labels, lowers instructions, and records executable instruction spans.
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
                Node::EqualityBranch {
                    condition,
                    likely,
                    left,
                    right,
                    label,
                } => Instruction::EqualityBranch {
                    condition: *condition,
                    likely: *likely,
                    left: *left,
                    right: *right,
                    target: label_address(&labels, label)?,
                },
                Node::SignBranch {
                    condition,
                    likely,
                    link,
                    source,
                    label,
                } => Instruction::SignBranch {
                    condition: *condition,
                    likely: *likely,
                    link: *link,
                    source: *source,
                    target: label_address(&labels, label)?,
                },
                Node::FloatingBranch {
                    condition,
                    likely,
                    label,
                } => Instruction::FloatingBranch {
                    condition: *condition,
                    likely: *likely,
                    target: label_address(&labels, label)?,
                },
                Node::VfpuBranch {
                    form,
                    condition,
                    label,
                } => Instruction::Vfpu(VfpuInstruction::new(
                    *form,
                    VfpuOperands::Branch {
                        condition: *condition,
                        target: label_address(&labels, label)?,
                    },
                )?),
                Node::Jump { link, label } => Instruction::Jump {
                    link: *link,
                    target: label_address(&labels, label)?,
                },
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
                    .expect("Allegrex instruction bytes are non-empty"),
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
                placed_address(origin, offset)?;
            }
        }
        Ok((labels, offset))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Assembled bytes with labels and executable instruction spans.
pub struct AssembledProgram {
    origin: u64,
    bytes: Vec<u8>,
    labels: BTreeMap<String, u32>,
    instruction_spans: Vec<InstructionSpan<Instruction>>,
}

impl AssembledProgram {
    /// Performs the origin operation.
    pub const fn origin(&self) -> u64 {
        self.origin
    }

    /// Borrows the complete output bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consumes the program and returns its bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Returns all resolved labels.
    pub fn labels(&self) -> &BTreeMap<String, u32> {
        &self.labels
    }

    /// Returns executable instruction placements.
    pub fn instruction_spans(&self) -> &[InstructionSpan<Instruction>] {
        &self.instruction_spans
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
/// Failure returned while placing or lowering an Allegrex assembly unit.
pub enum AssemblerError {
    #[error("Allegrex origin {origin:#x} is not four-byte aligned")]
    /// The unaligned origin form.
    UnalignedOrigin {
        /// The first output address.
        origin: u64,
    },
    #[error("Allegrex origin {origin:#x} exceeds the 32-bit address space")]
    /// The origin out of range form.
    OriginOutOfRange {
        /// The first output address.
        origin: u64,
    },
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
    #[error("instruction at output offset {offset:#x} is not four-byte aligned")]
    /// The unaligned instruction form.
    UnalignedInstruction {
        /// The output byte offset.
        offset: usize,
    },
    #[error("address overflow from origin {origin:#x} at output offset {offset:#x}")]
    /// The address overflow form.
    AddressOverflow {
        /// The first output address.
        origin: u64,
        /// The output byte offset.
        offset: usize,
    },
    #[error(transparent)]
    /// The encode form.
    Encode(#[from] EncodeError),
    #[error(transparent)]
    /// The vfpu form.
    Vfpu(#[from] crate::VfpuConstructionError),
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
    let offset =
        u64::try_from(offset).map_err(|_| AssemblerError::AddressOverflow { origin, offset })?;
    let address = origin
        .checked_add(offset)
        .ok_or(AssemblerError::AddressOverflow {
            origin,
            offset: usize::MAX,
        })?;
    if address > u64::from(u32::MAX) {
        return Err(AssemblerError::AddressOverflow {
            origin,
            offset: usize::try_from(offset).unwrap_or(usize::MAX),
        });
    }
    Ok(address)
}

fn label_address(labels: &BTreeMap<String, u32>, label: &str) -> Result<u32, AssemblerError> {
    labels
        .get(label)
        .copied()
        .ok_or_else(|| AssemblerError::UndefinedLabel {
            label: label.to_owned(),
        })
}
