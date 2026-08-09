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

/// Placement-aware builder for typed ARM-state code and explicit data.
#[derive(Debug, Clone, Default)]
pub struct ArmAssembler {
    nodes: Vec<ArmNode>,
}

impl ArmAssembler {
    /// Construct an empty ARM-state assembly program.
    pub fn new() -> Self {
        Self::default()
    }

    /// Define a label at the current byte position.
    pub fn label(&mut self, name: impl Into<String>) -> &mut Self {
        self.nodes.push(ArmNode::Label(name.into()));
        self
    }

    /// Append one fully typed ARM-state instruction.
    pub fn emit(&mut self, instruction: ArmInstruction) -> &mut Self {
        self.nodes.push(ArmNode::Instruction(instruction));
        self
    }

    /// Append a conditional ARM branch to a named ARM-state label.
    pub fn branch_to_label(&mut self, condition: Condition, label: impl Into<String>) -> &mut Self {
        self.nodes.push(ArmNode::Branch {
            condition,
            link: false,
            label: label.into(),
        });
        self
    }

    /// Append a conditional ARM branch with link to a named ARM-state label.
    pub fn call_label(&mut self, condition: Condition, label: impl Into<String>) -> &mut Self {
        self.nodes.push(ArmNode::Branch {
            condition,
            link: true,
            label: label.into(),
        });
        self
    }

    /// Append an immediate `BLX` call to a named Thumb-state entry address.
    pub fn call_thumb_label(&mut self, label: impl Into<String>) -> &mut Self {
        self.nodes.push(ArmNode::BranchLinkExchange(label.into()));
        self
    }

    /// Emit declared non-executable data without treating it as an instruction.
    pub fn data(&mut self, bytes: impl Into<Vec<u8>>) -> &mut Self {
        self.nodes.push(ArmNode::Data(bytes.into()));
        self
    }

    /// Resolve labels, lower typed ARM code, and verify every instruction span.
    ///
    /// `origin` and every instruction position must be word-aligned. ARM branch
    /// displacements use the architectural address plus eight as their PC base.
    pub fn assemble(&self, origin: crate::CodeLocation) -> Result<ArmProgram, ArmAssemblerError> {
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
                location: address,
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
    spans: &[InstructionSpan<crate::CodeLocation, ArmInstruction>],
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

/// Checked ARM-state output with resolved labels and verified instruction spans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArmProgram {
    origin: u32,
    bytes: Vec<u8>,
    labels: BTreeMap<String, u32>,
    spans: Vec<InstructionSpan<crate::CodeLocation, ArmInstruction>>,
}

impl ArmProgram {
    /// Return the address assigned to the first output byte.
    pub const fn origin(&self) -> crate::CodeLocation {
        self.origin
    }

    /// Borrow the complete assembled code and explicit data bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Return the resolved address of a defined label, if present.
    pub fn label_location(&self, label: &str) -> Option<crate::CodeLocation> {
        self.labels.get(label).copied()
    }

    /// Borrow the locations, bytes, and identities of ARM instructions only.
    pub fn instruction_spans(&self) -> &[InstructionSpan<crate::CodeLocation, ArmInstruction>] {
        &self.spans
    }
}

/// A fail-closed reason that ARM-state assembly did not produce a program.
#[derive(Debug, Error)]
pub enum ArmAssemblerError {
    /// The requested origin is not word-aligned.
    #[error("ARM assembly origin {origin:#x} is not word-aligned")]
    UnalignedOrigin {
        /// Rejected program origin.
        origin: u32,
    },
    /// Explicit data left the following instruction at a non-word-aligned offset.
    #[error("ARM instruction at output offset {offset:#x} is not word-aligned")]
    UnalignedInstruction {
        /// Byte offset of the misaligned instruction.
        offset: usize,
    },
    /// A label name was defined more than once.
    #[error("duplicate label {label}")]
    DuplicateLabel {
        /// Repeated label name.
        label: String,
    },
    /// A label reference names no definition in the program.
    #[error("undefined label {label}")]
    UndefinedLabel {
        /// Missing label name.
        label: String,
    },
    /// Origin plus output length exceeds the 32-bit address space.
    #[error("assembled address exceeds the 32-bit ARM address space from origin {origin:#x}")]
    AddressOverflow {
        /// Address assigned to the first output byte.
        origin: u32,
    },
    /// A target difference cannot be represented as a signed ARM displacement.
    #[error("relative target {target:#x} cannot be represented from ARM PC {pc:#x}")]
    RelativeAddressOutOfRange {
        /// Architectural PC base used by the branch.
        pc: u32,
        /// Resolved target address.
        target: u32,
    },
    /// An emitted instruction changed identity during byte round-trip verification.
    #[error("assembled ARM instruction at output offset {offset:#x} failed byte verification")]
    VerificationFailed {
        /// Byte offset of the inconsistent instruction span.
        offset: usize,
    },
    /// Post-placement ARM decoding failed.
    #[error(transparent)]
    Decode(#[from] DecodeError),
    /// Typed ARM lowering failed.
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

/// Placement-aware builder for typed Thumb-state code and explicit data.
#[derive(Debug, Clone, Default)]
pub struct ThumbAssembler {
    nodes: Vec<ThumbNode>,
}

impl ThumbAssembler {
    /// Construct an empty Thumb-state assembly program.
    pub fn new() -> Self {
        Self::default()
    }

    /// Define a label at the current byte position.
    pub fn label(&mut self, name: impl Into<String>) -> &mut Self {
        self.nodes.push(ThumbNode::Label(name.into()));
        self
    }

    /// Append one fully typed Thumb-state instruction.
    pub fn emit(&mut self, instruction: ThumbInstruction) -> &mut Self {
        self.nodes.push(ThumbNode::Instruction(instruction));
        self
    }

    /// Append a conditional short branch to a named Thumb-state label.
    pub fn branch_to_label(&mut self, condition: Condition, label: impl Into<String>) -> &mut Self {
        self.nodes.push(ThumbNode::ConditionalBranch {
            condition,
            label: label.into(),
        });
        self
    }

    /// Append an unconditional short branch to a named Thumb-state label.
    pub fn jump_label(&mut self, label: impl Into<String>) -> &mut Self {
        self.nodes.push(ThumbNode::Branch(label.into()));
        self
    }

    /// Append a long branch with link to a named Thumb-state label.
    pub fn call_label(&mut self, label: impl Into<String>) -> &mut Self {
        self.nodes.push(ThumbNode::BranchLink(label.into()));
        self
    }

    /// Append an immediate `BLX` call to a named ARM-state entry address.
    pub fn call_arm_label(&mut self, label: impl Into<String>) -> &mut Self {
        self.nodes.push(ThumbNode::BranchLinkExchange(label.into()));
        self
    }

    /// Emit declared non-executable data without treating it as an instruction.
    pub fn data(&mut self, bytes: impl Into<Vec<u8>>) -> &mut Self {
        self.nodes.push(ThumbNode::Data(bytes.into()));
        self
    }

    /// Resolve labels, lower typed Thumb code, and verify every instruction span.
    ///
    /// `origin` and every instruction position must be halfword-aligned. Thumb
    /// branches use the architectural address plus four; immediate `BLX` also
    /// word-aligns that PC base.
    pub fn assemble(
        &self,
        origin: crate::CodeLocation,
    ) -> Result<ThumbProgram, ThumbAssemblerError> {
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
                location: address,
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
    spans: &[InstructionSpan<crate::CodeLocation, ThumbInstruction>],
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

/// Checked Thumb-state output with resolved labels and verified instruction spans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThumbProgram {
    origin: u32,
    bytes: Vec<u8>,
    labels: BTreeMap<String, u32>,
    spans: Vec<InstructionSpan<crate::CodeLocation, ThumbInstruction>>,
}

impl ThumbProgram {
    /// Return the address assigned to the first output byte.
    pub const fn origin(&self) -> crate::CodeLocation {
        self.origin
    }

    /// Borrow the complete assembled code and explicit data bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Return the resolved address of a defined label, if present.
    pub fn label_location(&self, label: &str) -> Option<crate::CodeLocation> {
        self.labels.get(label).copied()
    }

    /// Borrow the locations, bytes, and identities of Thumb instructions only.
    pub fn instruction_spans(&self) -> &[InstructionSpan<crate::CodeLocation, ThumbInstruction>] {
        &self.spans
    }
}

/// A fail-closed reason that Thumb-state assembly did not produce a program.
#[derive(Debug, Error)]
pub enum ThumbAssemblerError {
    /// The requested origin is not halfword-aligned.
    #[error("Thumb assembly origin {origin:#x} is not halfword-aligned")]
    UnalignedOrigin {
        /// Rejected program origin.
        origin: u32,
    },
    /// Explicit data left the following instruction at an odd byte offset.
    #[error("Thumb instruction at output offset {offset:#x} is not halfword-aligned")]
    UnalignedInstruction {
        /// Byte offset of the misaligned instruction.
        offset: usize,
    },
    /// A label name was defined more than once.
    #[error("duplicate label {label}")]
    DuplicateLabel {
        /// Repeated label name.
        label: String,
    },
    /// A label reference names no definition in the program.
    #[error("undefined label {label}")]
    UndefinedLabel {
        /// Missing label name.
        label: String,
    },
    /// Origin plus output length exceeds the 32-bit address space.
    #[error("assembled address exceeds the 32-bit ARM address space from origin {origin:#x}")]
    AddressOverflow {
        /// Address assigned to the first output byte.
        origin: u32,
    },
    /// A target difference cannot be represented as a signed Thumb displacement.
    #[error("relative target {target:#x} cannot be represented from Thumb PC {pc:#x}")]
    RelativeAddressOutOfRange {
        /// Architectural PC base used by the branch.
        pc: u32,
        /// Resolved target address.
        target: u32,
    },
    /// A target exceeds the range of the selected short Thumb branch form.
    #[error("branch to {label} is outside the selected Thumb instruction range")]
    BranchOutOfRange {
        /// Target label that cannot be reached by the selected form.
        label: String,
    },
    /// An emitted instruction changed identity during byte round-trip verification.
    #[error("assembled Thumb instruction at output offset {offset:#x} failed byte verification")]
    VerificationFailed {
        /// Byte offset of the inconsistent instruction span.
        offset: usize,
    },
    /// Post-placement Thumb decoding failed.
    #[error(transparent)]
    Decode(#[from] DecodeError),
    /// Typed Thumb lowering failed.
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
