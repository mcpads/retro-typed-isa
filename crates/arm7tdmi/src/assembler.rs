use std::collections::BTreeMap;

use thiserror::Error;
use typed_isa_core::{InstructionBytes, InstructionSpan};

use crate::{
    ArmInstruction, Condition, DecodeError, EncodeError, ThumbInstruction, decode_arm_bytes,
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
    Data(Vec<u8>),
}

#[derive(Debug, Clone, Default)]
/// A checked ARM-state assembler with symbolic branches and explicit data nodes.
pub struct ArmAssembler {
    nodes: Vec<ArmNode>,
}

impl ArmAssembler {
    /// Creates an empty ARM-state assembly unit.
    pub fn new() -> Self {
        Self::default()
    }

    /// Declares a label at the current output position.
    pub fn label(&mut self, name: impl Into<String>) -> &mut Self {
        self.nodes.push(ArmNode::Label(name.into()));
        self
    }

    /// Appends an already-typed ARM-state instruction.
    pub fn emit(&mut self, instruction: ArmInstruction) -> &mut Self {
        self.nodes.push(ArmNode::Instruction(instruction));
        self
    }

    /// Appends a symbolic conditional branch without link.
    pub fn branch_to_label(&mut self, condition: Condition, label: impl Into<String>) -> &mut Self {
        self.nodes.push(ArmNode::Branch {
            condition,
            link: false,
            label: label.into(),
        });
        self
    }

    /// Appends a symbolic conditional branch with link.
    pub fn call_label(&mut self, condition: Condition, label: impl Into<String>) -> &mut Self {
        self.nodes.push(ArmNode::Branch {
            condition,
            link: true,
            label: label.into(),
        });
        self
    }

    /// Emit declared non-executable data without treating it as an instruction.
    pub fn data(&mut self, bytes: impl Into<Vec<u8>>) -> &mut Self {
        self.nodes.push(ArmNode::Data(bytes.into()));
        self
    }

    /// Resolves labels, lowers instructions, and verifies every emitted instruction span.
    ///
    /// ARM branches are resolved against the architecturally visible `PC`, which
    /// is the instruction address plus eight bytes in ARM state.
    pub fn assemble(&self, origin: u32) -> Result<ArmProgram, ArmAssemblerError> {
        require_origin(origin, 4).map_err(|_| ArmAssemblerError::UnalignedOrigin { origin })?;
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
                    let address = placed_address(origin, bytes.len())?;
                    let target = label_address(&labels, label)?;
                    ArmInstruction::Branch {
                        condition: *condition,
                        link: *link,
                        displacement: relative_displacement(address.wrapping_add(8), target)?,
                    }
                }
            };
            if bytes.len() & 3 != 0 {
                return Err(ArmAssemblerError::UnalignedInstruction {
                    offset: bytes.len(),
                });
            }
            let address = placed_address(origin, bytes.len())?;
            let encoded = encode_arm_bytes(&instruction)?;
            let encoded_bytes =
                InstructionBytes::new(encoded.to_vec()).expect("ARM instruction is non-empty");
            spans.push(InstructionSpan {
                offset: bytes.len(),
                address: u64::from(address),
                bytes: encoded_bytes,
                instruction,
            });
            bytes.extend_from_slice(&encoded);
        }
        verify_arm_spans(&bytes, &spans)?;
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
            let address = placed_address(origin, offset)?;
            if labels.insert(label.clone(), address).is_some() {
                return Err(ArmAssemblerError::DuplicateLabel {
                    label: label.clone(),
                });
            }
        } else {
            offset = offset
                .checked_add(match node {
                    ArmNode::Instruction(_) | ArmNode::Branch { .. } => 4,
                    ArmNode::Data(data) => data.len(),
                    ArmNode::Label(_) => unreachable!(),
                })
                .ok_or(ArmAssemblerError::AddressOverflow { origin })?;
            placed_address(origin, offset)?;
        }
    }
    Ok((labels, offset))
}

fn verify_arm_spans(
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
/// Verified ARM-state output and its source-to-byte placement metadata.
pub struct ArmProgram {
    origin: u32,
    bytes: Vec<u8>,
    labels: BTreeMap<String, u32>,
    spans: Vec<InstructionSpan<ArmInstruction>>,
}

impl ArmProgram {
    /// Returns the address assigned to the first output byte.
    pub const fn origin(&self) -> u32 {
        self.origin
    }

    /// Returns the complete assembled byte stream, including explicit data.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the resolved address of a declared label.
    pub fn label_address(&self, label: &str) -> Option<u32> {
        self.labels.get(label).copied()
    }

    /// Returns placements for executable instructions only.
    pub fn instruction_spans(&self) -> &[InstructionSpan<ArmInstruction>] {
        &self.spans
    }
}

#[derive(Debug, Error)]
/// Failure returned while resolving or verifying an ARM-state assembly unit.
pub enum ArmAssemblerError {
    #[error("ARM assembly origin {origin:#x} is not word-aligned")]
    /// The output origin is not word-aligned.
    UnalignedOrigin {
        /// The rejected origin.
        origin: u32,
    },
    #[error("ARM instruction at output offset {offset:#x} is not word-aligned")]
    /// Explicit data left the following instruction at an unaligned offset.
    UnalignedInstruction {
        /// The offending output offset.
        offset: usize,
    },
    #[error("duplicate label {label}")]
    /// A label name was declared more than once.
    DuplicateLabel {
        /// The duplicate label.
        label: String,
    },
    #[error("undefined label {label}")]
    /// A symbolic branch references an undeclared label.
    UndefinedLabel {
        /// The missing label.
        label: String,
    },
    #[error("assembled address exceeds the 32-bit ARM address space from origin {origin:#x}")]
    /// Output placement exceeds the 32-bit ARM address space.
    AddressOverflow {
        /// The assembly origin used for placement.
        origin: u32,
    },
    #[error("relative target {target:#x} cannot be represented from ARM PC {pc:#x}")]
    /// A target cannot be represented by the aligned signed ARM branch field.
    RelativeAddressOutOfRange {
        /// The architecturally visible program counter.
        pc: u32,
        /// The requested target address.
        target: u32,
    },
    #[error("assembled ARM instruction at output offset {offset:#x} failed byte verification")]
    /// Re-decoding an emitted instruction did not reproduce the typed value and bytes.
    VerificationFailed {
        /// The instruction's output offset.
        offset: usize,
    },
    #[error(transparent)]
    /// Instruction decoding failed during output verification.
    Decode(#[from] DecodeError),
    #[error(transparent)]
    /// A typed instruction could not be encoded.
    Encode(#[from] EncodeError),
}

#[derive(Debug, Clone)]
enum ThumbNode {
    Label(String),
    Instruction(ThumbInstruction),
    ConditionalBranch { condition: Condition, label: String },
    Branch(String),
    BranchLink(String),
    Data(Vec<u8>),
}

#[derive(Debug, Clone, Default)]
/// A checked Thumb-state assembler with symbolic branches and explicit data nodes.
pub struct ThumbAssembler {
    nodes: Vec<ThumbNode>,
}

impl ThumbAssembler {
    /// Creates an empty Thumb-state assembly unit.
    pub fn new() -> Self {
        Self::default()
    }

    /// Declares a label at the current output position.
    pub fn label(&mut self, name: impl Into<String>) -> &mut Self {
        self.nodes.push(ThumbNode::Label(name.into()));
        self
    }

    /// Appends an already-typed Thumb-state instruction.
    pub fn emit(&mut self, instruction: ThumbInstruction) -> &mut Self {
        self.nodes.push(ThumbNode::Instruction(instruction));
        self
    }

    /// Appends a symbolic conditional branch.
    pub fn branch_to_label(&mut self, condition: Condition, label: impl Into<String>) -> &mut Self {
        self.nodes.push(ThumbNode::ConditionalBranch {
            condition,
            label: label.into(),
        });
        self
    }

    /// Appends a symbolic unconditional branch.
    pub fn jump_label(&mut self, label: impl Into<String>) -> &mut Self {
        self.nodes.push(ThumbNode::Branch(label.into()));
        self
    }

    /// Appends a symbolic long branch with link.
    pub fn call_label(&mut self, label: impl Into<String>) -> &mut Self {
        self.nodes.push(ThumbNode::BranchLink(label.into()));
        self
    }

    /// Emit declared non-executable data without treating it as an instruction.
    pub fn data(&mut self, bytes: impl Into<Vec<u8>>) -> &mut Self {
        self.nodes.push(ThumbNode::Data(bytes.into()));
        self
    }

    /// Resolves labels, lowers instructions, and verifies every emitted instruction span.
    ///
    /// Thumb branches are resolved against the architecturally visible `PC`,
    /// which is the first halfword address plus four bytes.
    pub fn assemble(&self, origin: u32) -> Result<ThumbProgram, ThumbAssemblerError> {
        require_origin(origin, 2).map_err(|_| ThumbAssemblerError::UnalignedOrigin { origin })?;
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
                    let address = placed_thumb_address(origin, bytes.len())?;
                    ThumbInstruction::ConditionalBranch {
                        condition: *condition,
                        displacement: thumb_relative_displacement(
                            address.wrapping_add(4),
                            label_address_thumb(&labels, label)?,
                        )?
                        .try_into()
                        .map_err(|_| {
                            ThumbAssemblerError::BranchOutOfRange {
                                label: label.clone(),
                            }
                        })?,
                    }
                }
                ThumbNode::Branch(label) => {
                    let address = placed_thumb_address(origin, bytes.len())?;
                    ThumbInstruction::UnconditionalBranch {
                        displacement: thumb_relative_displacement(
                            address.wrapping_add(4),
                            label_address_thumb(&labels, label)?,
                        )?
                        .try_into()
                        .map_err(|_| {
                            ThumbAssemblerError::BranchOutOfRange {
                                label: label.clone(),
                            }
                        })?,
                    }
                }
                ThumbNode::BranchLink(label) => {
                    let address = placed_thumb_address(origin, bytes.len())?;
                    ThumbInstruction::BranchLink {
                        displacement: thumb_relative_displacement(
                            address.wrapping_add(4),
                            label_address_thumb(&labels, label)?,
                        )?,
                    }
                }
            };
            if bytes.len() & 1 != 0 {
                return Err(ThumbAssemblerError::UnalignedInstruction {
                    offset: bytes.len(),
                });
            }
            let address = placed_thumb_address(origin, bytes.len())?;
            let encoded = encode_thumb_bytes(&instruction)?;
            let encoded_bytes =
                InstructionBytes::new(encoded.clone()).expect("Thumb instruction is non-empty");
            spans.push(InstructionSpan {
                offset: bytes.len(),
                address: u64::from(address),
                bytes: encoded_bytes,
                instruction,
            });
            bytes.extend_from_slice(&encoded);
        }
        verify_thumb_spans(&bytes, &spans)?;
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
            let address = placed_thumb_address(origin, offset)?;
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
                    ThumbNode::BranchLink(_) => 4,
                    ThumbNode::Data(data) => data.len(),
                    ThumbNode::Label(_) => unreachable!(),
                })
                .ok_or(ThumbAssemblerError::AddressOverflow { origin })?;
            placed_thumb_address(origin, offset)?;
        }
    }
    Ok((labels, offset))
}

fn verify_thumb_spans(
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
/// Verified Thumb-state output and its source-to-byte placement metadata.
pub struct ThumbProgram {
    origin: u32,
    bytes: Vec<u8>,
    labels: BTreeMap<String, u32>,
    spans: Vec<InstructionSpan<ThumbInstruction>>,
}

impl ThumbProgram {
    /// Returns the address assigned to the first output byte.
    pub const fn origin(&self) -> u32 {
        self.origin
    }

    /// Returns the complete assembled byte stream, including explicit data.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the resolved address of a declared label.
    pub fn label_address(&self, label: &str) -> Option<u32> {
        self.labels.get(label).copied()
    }

    /// Returns placements for executable instructions only.
    pub fn instruction_spans(&self) -> &[InstructionSpan<ThumbInstruction>] {
        &self.spans
    }
}

#[derive(Debug, Error)]
/// Failure returned while resolving or verifying a Thumb-state assembly unit.
pub enum ThumbAssemblerError {
    #[error("Thumb assembly origin {origin:#x} is not halfword-aligned")]
    /// The output origin is not halfword-aligned.
    UnalignedOrigin {
        /// The rejected origin.
        origin: u32,
    },
    #[error("Thumb instruction at output offset {offset:#x} is not halfword-aligned")]
    /// Explicit data left the following instruction at an unaligned offset.
    UnalignedInstruction {
        /// The offending output offset.
        offset: usize,
    },
    #[error("duplicate label {label}")]
    /// A label name was declared more than once.
    DuplicateLabel {
        /// The duplicate label.
        label: String,
    },
    #[error("undefined label {label}")]
    /// A symbolic branch references an undeclared label.
    UndefinedLabel {
        /// The missing label.
        label: String,
    },
    #[error("assembled address exceeds the 32-bit ARM address space from origin {origin:#x}")]
    /// Output placement exceeds the 32-bit ARM address space.
    AddressOverflow {
        /// The assembly origin used for placement.
        origin: u32,
    },
    #[error("relative target {target:#x} cannot be represented from Thumb PC {pc:#x}")]
    /// A target is not halfword-aligned or cannot be subtracted in the address space.
    RelativeAddressOutOfRange {
        /// The architecturally visible program counter.
        pc: u32,
        /// The requested target address.
        target: u32,
    },
    #[error("branch to {label} is outside the selected Thumb instruction range")]
    /// A resolved displacement does not fit the selected Thumb branch form.
    BranchOutOfRange {
        /// The out-of-range target label.
        label: String,
    },
    #[error("assembled Thumb instruction at output offset {offset:#x} failed byte verification")]
    /// Re-decoding an emitted instruction did not reproduce the typed value and bytes.
    VerificationFailed {
        /// The instruction's output offset.
        offset: usize,
    },
    #[error(transparent)]
    /// Instruction decoding failed during output verification.
    Decode(#[from] DecodeError),
    #[error(transparent)]
    /// A typed instruction could not be encoded.
    Encode(#[from] EncodeError),
}

fn require_origin(origin: u32, alignment: u32) -> Result<(), ()> {
    if origin & (alignment - 1) == 0 {
        Ok(())
    } else {
        Err(())
    }
}

fn placed_address(origin: u32, offset: usize) -> Result<u32, ArmAssemblerError> {
    origin
        .checked_add(
            u32::try_from(offset).map_err(|_| ArmAssemblerError::AddressOverflow { origin })?,
        )
        .ok_or(ArmAssemblerError::AddressOverflow { origin })
}

fn label_address(labels: &BTreeMap<String, u32>, label: &str) -> Result<u32, ArmAssemblerError> {
    labels
        .get(label)
        .copied()
        .ok_or_else(|| ArmAssemblerError::UndefinedLabel {
            label: label.to_owned(),
        })
}

fn placed_thumb_address(origin: u32, offset: usize) -> Result<u32, ThumbAssemblerError> {
    origin
        .checked_add(
            u32::try_from(offset).map_err(|_| ThumbAssemblerError::AddressOverflow { origin })?,
        )
        .ok_or(ThumbAssemblerError::AddressOverflow { origin })
}

fn label_address_thumb(
    labels: &BTreeMap<String, u32>,
    label: &str,
) -> Result<u32, ThumbAssemblerError> {
    labels
        .get(label)
        .copied()
        .ok_or_else(|| ThumbAssemblerError::UndefinedLabel {
            label: label.to_owned(),
        })
}

fn relative_displacement(pc: u32, target: u32) -> Result<i32, ArmAssemblerError> {
    i32::try_from(i64::from(target) - i64::from(pc))
        .map_err(|_| ArmAssemblerError::RelativeAddressOutOfRange { pc, target })
}

fn thumb_relative_displacement(pc: u32, target: u32) -> Result<i32, ThumbAssemblerError> {
    i32::try_from(i64::from(target) - i64::from(pc))
        .map_err(|_| ThumbAssemblerError::RelativeAddressOutOfRange { pc, target })
}
