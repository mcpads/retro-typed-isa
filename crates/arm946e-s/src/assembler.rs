use std::collections::BTreeMap;

use arm7tdmi::Condition;
use thiserror::Error;
use typed_isa_core::{InstructionBytes, InstructionSpan};

use crate::{
    ArmInstruction, DecodeError, EncodeError, ThumbInstruction, decode_arm_bytes,
    decode_thumb_bytes, encode_arm_bytes, encode_thumb_bytes,
};

#[derive(Debug, Clone)]
enum ArmNode {
    Label(String),
    Instruction(ArmInstruction),
    Branch {
        condition: Condition,
        link: bool,
        label: String,
    },
    BranchLinkExchange(String),
    Data(Vec<u8>),
}

#[derive(Debug, Clone, Default)]
pub struct ArmAssembler {
    nodes: Vec<ArmNode>,
}

impl ArmAssembler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn label(&mut self, name: impl Into<String>) -> &mut Self {
        self.nodes.push(ArmNode::Label(name.into()));
        self
    }

    pub fn emit(&mut self, instruction: ArmInstruction) -> &mut Self {
        self.nodes.push(ArmNode::Instruction(instruction));
        self
    }

    pub fn branch_to_label(&mut self, condition: Condition, label: impl Into<String>) -> &mut Self {
        self.nodes.push(ArmNode::Branch {
            condition,
            link: false,
            label: label.into(),
        });
        self
    }

    pub fn call_label(&mut self, condition: Condition, label: impl Into<String>) -> &mut Self {
        self.nodes.push(ArmNode::Branch {
            condition,
            link: true,
            label: label.into(),
        });
        self
    }

    pub fn call_thumb_label(&mut self, label: impl Into<String>) -> &mut Self {
        self.nodes.push(ArmNode::BranchLinkExchange(label.into()));
        self
    }

    /// Emit declared non-executable data without treating it as an instruction.
    pub fn data(&mut self, bytes: impl Into<Vec<u8>>) -> &mut Self {
        self.nodes.push(ArmNode::Data(bytes.into()));
        self
    }

    pub fn assemble(&self, origin: u32) -> Result<ArmProgram, ArmAssemblerError> {
        if origin & 3 != 0 {
            return Err(ArmAssemblerError::UnalignedOrigin { origin });
        }
        let (labels, byte_len) = collect_arm_labels(&self.nodes, origin)?;
        let mut bytes = Vec::with_capacity(byte_len);
        let mut spans = Vec::new();
        for node in &self.nodes {
            let instruction = match node {
                ArmNode::Label(_) => continue,
                ArmNode::Data(data) => {
                    bytes.extend_from_slice(data);
                    continue;
                }
                ArmNode::Instruction(instruction) => *instruction,
                ArmNode::Branch {
                    condition,
                    link,
                    label,
                } => {
                    let address = arm_address(origin, bytes.len())?;
                    let target = arm_label(&labels, label)?;
                    ArmInstruction::Armv4T(arm7tdmi::ArmInstruction::Branch {
                        condition: *condition,
                        link: *link,
                        displacement: arm_relative(address.wrapping_add(8), target)?,
                    })
                }
                ArmNode::BranchLinkExchange(label) => {
                    let address = arm_address(origin, bytes.len())?;
                    ArmInstruction::BranchLinkExchangeImmediate {
                        displacement: arm_relative(
                            address.wrapping_add(8),
                            arm_label(&labels, label)?,
                        )?,
                    }
                }
            };
            if bytes.len() & 3 != 0 {
                return Err(ArmAssemblerError::UnalignedInstruction {
                    offset: bytes.len(),
                });
            }
            let address = arm_address(origin, bytes.len())?;
            let encoded = encode_arm_bytes(&instruction)?;
            spans.push(InstructionSpan {
                offset: bytes.len(),
                address: u64::from(address),
                bytes: InstructionBytes::new(encoded.to_vec())
                    .expect("ARM instruction is non-empty"),
                instruction,
            });
            bytes.extend_from_slice(&encoded);
        }
        verify_arm(&bytes, &spans)?;
        Ok(ArmProgram {
            origin,
            bytes,
            labels,
            spans,
        })
    }
}

fn collect_arm_labels(
    nodes: &[ArmNode],
    origin: u32,
) -> Result<(BTreeMap<String, u32>, usize), ArmAssemblerError> {
    let mut labels = BTreeMap::new();
    let mut offset = 0usize;
    for node in nodes {
        if let ArmNode::Label(label) = node {
            let address = arm_address(origin, offset)?;
            if labels.insert(label.clone(), address).is_some() {
                return Err(ArmAssemblerError::DuplicateLabel {
                    label: label.clone(),
                });
            }
        } else {
            offset = offset
                .checked_add(match node {
                    ArmNode::Instruction(_)
                    | ArmNode::Branch { .. }
                    | ArmNode::BranchLinkExchange(_) => 4,
                    ArmNode::Data(data) => data.len(),
                    ArmNode::Label(_) => unreachable!(),
                })
                .ok_or(ArmAssemblerError::AddressOverflow { origin })?;
            arm_address(origin, offset)?;
        }
    }
    Ok((labels, offset))
}

fn verify_arm(
    bytes: &[u8],
    spans: &[InstructionSpan<ArmInstruction>],
) -> Result<(), ArmAssemblerError> {
    for span in spans {
        let source = bytes.get(span.offset..span.offset + 4).ok_or(
            ArmAssemblerError::VerificationFailed {
                offset: span.offset,
            },
        )?;
        let decoded = decode_arm_bytes(source)?;
        if decoded != span.instruction || encode_arm_bytes(&decoded)? != source {
            return Err(ArmAssemblerError::VerificationFailed {
                offset: span.offset,
            });
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArmProgram {
    origin: u32,
    bytes: Vec<u8>,
    labels: BTreeMap<String, u32>,
    spans: Vec<InstructionSpan<ArmInstruction>>,
}

impl ArmProgram {
    pub const fn origin(&self) -> u32 {
        self.origin
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn label_address(&self, label: &str) -> Option<u32> {
        self.labels.get(label).copied()
    }

    pub fn instruction_spans(&self) -> &[InstructionSpan<ArmInstruction>] {
        &self.spans
    }
}

#[derive(Debug, Error)]
pub enum ArmAssemblerError {
    #[error("ARM assembly origin {origin:#x} is not word-aligned")]
    UnalignedOrigin { origin: u32 },
    #[error("ARM instruction at output offset {offset:#x} is not word-aligned")]
    UnalignedInstruction { offset: usize },
    #[error("duplicate label {label}")]
    DuplicateLabel { label: String },
    #[error("undefined label {label}")]
    UndefinedLabel { label: String },
    #[error("assembled address exceeds the 32-bit ARM address space from origin {origin:#x}")]
    AddressOverflow { origin: u32 },
    #[error("relative target {target:#x} cannot be represented from ARM PC {pc:#x}")]
    RelativeAddressOutOfRange { pc: u32, target: u32 },
    #[error("assembled ARM instruction at output offset {offset:#x} failed byte verification")]
    VerificationFailed { offset: usize },
    #[error(transparent)]
    Decode(#[from] DecodeError),
    #[error(transparent)]
    Encode(#[from] EncodeError),
}

#[derive(Debug, Clone)]
enum ThumbNode {
    Label(String),
    Instruction(ThumbInstruction),
    ConditionalBranch { condition: Condition, label: String },
    Branch(String),
    BranchLink(String),
    BranchLinkExchange(String),
    Data(Vec<u8>),
}

#[derive(Debug, Clone, Default)]
pub struct ThumbAssembler {
    nodes: Vec<ThumbNode>,
}

impl ThumbAssembler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn label(&mut self, name: impl Into<String>) -> &mut Self {
        self.nodes.push(ThumbNode::Label(name.into()));
        self
    }

    pub fn emit(&mut self, instruction: ThumbInstruction) -> &mut Self {
        self.nodes.push(ThumbNode::Instruction(instruction));
        self
    }

    pub fn branch_to_label(&mut self, condition: Condition, label: impl Into<String>) -> &mut Self {
        self.nodes.push(ThumbNode::ConditionalBranch {
            condition,
            label: label.into(),
        });
        self
    }

    pub fn jump_label(&mut self, label: impl Into<String>) -> &mut Self {
        self.nodes.push(ThumbNode::Branch(label.into()));
        self
    }

    pub fn call_label(&mut self, label: impl Into<String>) -> &mut Self {
        self.nodes.push(ThumbNode::BranchLink(label.into()));
        self
    }

    pub fn call_arm_label(&mut self, label: impl Into<String>) -> &mut Self {
        self.nodes.push(ThumbNode::BranchLinkExchange(label.into()));
        self
    }

    /// Emit declared non-executable data without treating it as an instruction.
    pub fn data(&mut self, bytes: impl Into<Vec<u8>>) -> &mut Self {
        self.nodes.push(ThumbNode::Data(bytes.into()));
        self
    }

    pub fn assemble(&self, origin: u32) -> Result<ThumbProgram, ThumbAssemblerError> {
        if origin & 1 != 0 {
            return Err(ThumbAssemblerError::UnalignedOrigin { origin });
        }
        let (labels, byte_len) = collect_thumb_labels(&self.nodes, origin)?;
        let mut bytes = Vec::with_capacity(byte_len);
        let mut spans = Vec::new();
        for node in &self.nodes {
            let instruction = match node {
                ThumbNode::Label(_) => continue,
                ThumbNode::Data(data) => {
                    bytes.extend_from_slice(data);
                    continue;
                }
                ThumbNode::Instruction(instruction) => *instruction,
                ThumbNode::ConditionalBranch { condition, label } => {
                    let address = thumb_address(origin, bytes.len())?;
                    ThumbInstruction::Armv4T(arm7tdmi::ThumbInstruction::ConditionalBranch {
                        condition: *condition,
                        displacement: thumb_relative(
                            address.wrapping_add(4),
                            thumb_label(&labels, label)?,
                        )?
                        .try_into()
                        .map_err(|_| {
                            ThumbAssemblerError::BranchOutOfRange {
                                label: label.clone(),
                            }
                        })?,
                    })
                }
                ThumbNode::Branch(label) => {
                    let address = thumb_address(origin, bytes.len())?;
                    ThumbInstruction::Armv4T(arm7tdmi::ThumbInstruction::UnconditionalBranch {
                        displacement: thumb_relative(
                            address.wrapping_add(4),
                            thumb_label(&labels, label)?,
                        )?
                        .try_into()
                        .map_err(|_| {
                            ThumbAssemblerError::BranchOutOfRange {
                                label: label.clone(),
                            }
                        })?,
                    })
                }
                ThumbNode::BranchLink(label) => {
                    let address = thumb_address(origin, bytes.len())?;
                    ThumbInstruction::Armv4T(arm7tdmi::ThumbInstruction::BranchLink {
                        displacement: thumb_relative(
                            address.wrapping_add(4),
                            thumb_label(&labels, label)?,
                        )?,
                    })
                }
                ThumbNode::BranchLinkExchange(label) => {
                    let address = thumb_address(origin, bytes.len())?;
                    let pc = address.wrapping_add(4) & !3;
                    ThumbInstruction::BranchLinkExchangeImmediate {
                        displacement: thumb_relative(pc, thumb_label(&labels, label)?)?,
                    }
                }
            };
            if bytes.len() & 1 != 0 {
                return Err(ThumbAssemblerError::UnalignedInstruction {
                    offset: bytes.len(),
                });
            }
            let address = thumb_address(origin, bytes.len())?;
            let encoded = encode_thumb_bytes(&instruction)?;
            spans.push(InstructionSpan {
                offset: bytes.len(),
                address: u64::from(address),
                bytes: InstructionBytes::new(encoded.clone())
                    .expect("Thumb instruction is non-empty"),
                instruction,
            });
            bytes.extend_from_slice(&encoded);
        }
        verify_thumb(&bytes, &spans)?;
        Ok(ThumbProgram {
            origin,
            bytes,
            labels,
            spans,
        })
    }
}

fn collect_thumb_labels(
    nodes: &[ThumbNode],
    origin: u32,
) -> Result<(BTreeMap<String, u32>, usize), ThumbAssemblerError> {
    let mut labels = BTreeMap::new();
    let mut offset = 0usize;
    for node in nodes {
        if let ThumbNode::Label(label) = node {
            let address = thumb_address(origin, offset)?;
            if labels.insert(label.clone(), address).is_some() {
                return Err(ThumbAssemblerError::DuplicateLabel {
                    label: label.clone(),
                });
            }
        } else {
            offset = offset
                .checked_add(match node {
                    ThumbNode::Instruction(instruction) => instruction.encoded_len(),
                    ThumbNode::ConditionalBranch { .. } | ThumbNode::Branch(_) => 2,
                    ThumbNode::BranchLink(_) | ThumbNode::BranchLinkExchange(_) => 4,
                    ThumbNode::Data(data) => data.len(),
                    ThumbNode::Label(_) => unreachable!(),
                })
                .ok_or(ThumbAssemblerError::AddressOverflow { origin })?;
            thumb_address(origin, offset)?;
        }
    }
    Ok((labels, offset))
}

fn verify_thumb(
    bytes: &[u8],
    spans: &[InstructionSpan<ThumbInstruction>],
) -> Result<(), ThumbAssemblerError> {
    for span in spans {
        let len = span.bytes.len().get();
        let source = bytes.get(span.offset..span.offset + len).ok_or(
            ThumbAssemblerError::VerificationFailed {
                offset: span.offset,
            },
        )?;
        let decoded = decode_thumb_bytes(source)?;
        if decoded != span.instruction || encode_thumb_bytes(&decoded)? != source {
            return Err(ThumbAssemblerError::VerificationFailed {
                offset: span.offset,
            });
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThumbProgram {
    origin: u32,
    bytes: Vec<u8>,
    labels: BTreeMap<String, u32>,
    spans: Vec<InstructionSpan<ThumbInstruction>>,
}

impl ThumbProgram {
    pub const fn origin(&self) -> u32 {
        self.origin
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn label_address(&self, label: &str) -> Option<u32> {
        self.labels.get(label).copied()
    }

    pub fn instruction_spans(&self) -> &[InstructionSpan<ThumbInstruction>] {
        &self.spans
    }
}

#[derive(Debug, Error)]
pub enum ThumbAssemblerError {
    #[error("Thumb assembly origin {origin:#x} is not halfword-aligned")]
    UnalignedOrigin { origin: u32 },
    #[error("Thumb instruction at output offset {offset:#x} is not halfword-aligned")]
    UnalignedInstruction { offset: usize },
    #[error("duplicate label {label}")]
    DuplicateLabel { label: String },
    #[error("undefined label {label}")]
    UndefinedLabel { label: String },
    #[error("assembled address exceeds the 32-bit ARM address space from origin {origin:#x}")]
    AddressOverflow { origin: u32 },
    #[error("relative target {target:#x} cannot be represented from Thumb PC {pc:#x}")]
    RelativeAddressOutOfRange { pc: u32, target: u32 },
    #[error("branch to {label} is outside the selected Thumb instruction range")]
    BranchOutOfRange { label: String },
    #[error("assembled Thumb instruction at output offset {offset:#x} failed byte verification")]
    VerificationFailed { offset: usize },
    #[error(transparent)]
    Decode(#[from] DecodeError),
    #[error(transparent)]
    Encode(#[from] EncodeError),
}

fn arm_address(origin: u32, offset: usize) -> Result<u32, ArmAssemblerError> {
    origin
        .checked_add(
            u32::try_from(offset).map_err(|_| ArmAssemblerError::AddressOverflow { origin })?,
        )
        .ok_or(ArmAssemblerError::AddressOverflow { origin })
}

fn arm_label(labels: &BTreeMap<String, u32>, label: &str) -> Result<u32, ArmAssemblerError> {
    labels
        .get(label)
        .copied()
        .ok_or_else(|| ArmAssemblerError::UndefinedLabel {
            label: label.to_owned(),
        })
}

fn arm_relative(pc: u32, target: u32) -> Result<i32, ArmAssemblerError> {
    i32::try_from(i64::from(target) - i64::from(pc))
        .map_err(|_| ArmAssemblerError::RelativeAddressOutOfRange { pc, target })
}

fn thumb_address(origin: u32, offset: usize) -> Result<u32, ThumbAssemblerError> {
    origin
        .checked_add(
            u32::try_from(offset).map_err(|_| ThumbAssemblerError::AddressOverflow { origin })?,
        )
        .ok_or(ThumbAssemblerError::AddressOverflow { origin })
}

fn thumb_label(labels: &BTreeMap<String, u32>, label: &str) -> Result<u32, ThumbAssemblerError> {
    labels
        .get(label)
        .copied()
        .ok_or_else(|| ThumbAssemblerError::UndefinedLabel {
            label: label.to_owned(),
        })
}

fn thumb_relative(pc: u32, target: u32) -> Result<i32, ThumbAssemblerError> {
    i32::try_from(i64::from(target) - i64::from(pc))
        .map_err(|_| ThumbAssemblerError::RelativeAddressOutOfRange { pc, target })
}
