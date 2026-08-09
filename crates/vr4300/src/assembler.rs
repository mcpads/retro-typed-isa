use std::collections::BTreeMap;

use thiserror::Error;
use typed_isa_core::{InstructionBytes, InstructionSpan};

use crate::{
    Coprocessor, CoprocessorBranchCondition, EncodeError, EqualityCondition, GeneralRegister,
    Instruction, SignCondition, StatusFr, encode_bytes,
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
    CoprocessorBranch {
        coprocessor: Coprocessor,
        condition: CoprocessorBranchCondition,
        likely: bool,
        label: String,
    },
    Jump {
        link: bool,
        label: String,
    },
    Data(Vec<u8>),
}

#[derive(Debug, Clone, Default)]
/// A checked VR4300 assembler with symbolic control-flow targets and explicit data nodes.
pub struct Assembler {
    nodes: Vec<Node>,
}

impl Assembler {
    /// Creates an empty assembly unit.
    pub fn new() -> Self {
        Self::default()
    }

    /// Declares a label at the current output position.
    pub fn label(&mut self, name: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::Label(name.into()));
        self
    }

    /// Appends an already-typed instruction.
    pub fn emit(&mut self, instruction: Instruction) -> &mut Self {
        self.nodes.push(Node::Instruction(instruction));
        self
    }

    /// Appends a symbolic integer equality branch.
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

    /// Appends a symbolic signed-zero branch.
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

    /// Appends a symbolic coprocessor-condition branch.
    pub fn coprocessor_branch_to_label(
        &mut self,
        coprocessor: Coprocessor,
        condition: CoprocessorBranchCondition,
        likely: bool,
        label: impl Into<String>,
    ) -> &mut Self {
        self.nodes.push(Node::CoprocessorBranch {
            coprocessor,
            condition,
            likely,
            label: label.into(),
        });
        self
    }

    /// Appends a symbolic direct jump, optionally recording a link address.
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

    /// Resolves labels and lowers instructions at a four-byte-aligned origin
    /// under the externally established CP0 Status.FR profile.
    ///
    /// Control-flow instructions retain absolute targets in the typed model;
    /// lowering checks branch reach and the MIPS jump-region rule at placement time.
    pub fn assemble(
        &self,
        origin: crate::CodeLocation,
        status_fr: StatusFr,
    ) -> Result<AssembledProgram, AssemblerError> {
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
                Node::Instruction(instruction) => *instruction,
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
                    target: label_location(&labels, label)?,
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
                    target: label_location(&labels, label)?,
                },
                Node::CoprocessorBranch {
                    coprocessor,
                    condition,
                    likely,
                    label,
                } => Instruction::CoprocessorBranch {
                    coprocessor: *coprocessor,
                    condition: *condition,
                    likely: *likely,
                    target: label_location(&labels, label)?,
                },
                Node::Jump { link, label } => Instruction::Jump {
                    link: *link,
                    target: label_location(&labels, label)?,
                },
            };
            if bytes.len() & 3 != 0 {
                return Err(AssemblerError::UnalignedInstruction {
                    offset: bytes.len(),
                });
            }
            let address = placed_address(origin, bytes.len())?;
            let encoded = encode_bytes(&instruction, address, status_fr)?;
            let offset = bytes.len();
            bytes.extend_from_slice(&encoded);
            instruction_spans.push(InstructionSpan {
                offset,
                location: address,
                bytes: InstructionBytes::new(encoded.to_vec())
                    .expect("VR4300 instruction bytes are non-empty"),
                instruction,
            });
        }

        Ok(AssembledProgram {
            origin,
            status_fr,
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

fn label_location(labels: &BTreeMap<String, u64>, label: &str) -> Result<u64, AssemblerError> {
    labels
        .get(label)
        .copied()
        .ok_or_else(|| AssemblerError::UndefinedLabel {
            label: label.to_owned(),
        })
}

fn placed_address(origin: u64, offset: usize) -> Result<u64, AssemblerError> {
    origin
        .checked_add(u64::try_from(offset).map_err(|_| AssemblerError::AddressOverflow { origin })?)
        .ok_or(AssemblerError::AddressOverflow { origin })
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Assembled big-endian bytes and their label and instruction placement metadata.
pub struct AssembledProgram {
    origin: u64,
    status_fr: StatusFr,
    bytes: Vec<u8>,
    labels: BTreeMap<String, u64>,
    instruction_spans: Vec<InstructionSpan<crate::CodeLocation, Instruction>>,
}

impl AssembledProgram {
    /// Returns the address assigned to the first output byte.
    pub const fn origin(&self) -> crate::CodeLocation {
        self.origin
    }

    /// Returns the explicit CP0 Status.FR profile used for lowering.
    pub const fn status_fr(&self) -> StatusFr {
        self.status_fr
    }

    /// Borrows the complete output, including explicitly declared data.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consumes the program and returns its complete byte stream.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Returns the resolved address of a declared label.
    pub fn label_location(&self, name: &str) -> Option<crate::CodeLocation> {
        self.labels.get(name).copied()
    }

    /// Returns placements for executable instructions only.
    pub fn instruction_spans(&self) -> &[InstructionSpan<crate::CodeLocation, Instruction>] {
        &self.instruction_spans
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
/// Failure returned while placing or lowering a VR4300 assembly unit.
pub enum AssemblerError {
    #[error("VR4300 assembly origin {origin:#018x} is not four-byte aligned")]
    /// The output origin is not four-byte aligned.
    UnalignedOrigin {
        /// The rejected origin.
        origin: u64,
    },
    #[error("VR4300 instruction at output offset {offset:#x} is not four-byte aligned")]
    /// Explicit data left a following instruction at an unaligned offset.
    UnalignedInstruction {
        /// The offending output offset.
        offset: usize,
    },
    #[error("duplicate label `{label}`")]
    /// A label name was declared more than once.
    DuplicateLabel {
        /// The duplicate label.
        label: String,
    },
    #[error("undefined label `{label}`")]
    /// A symbolic target references an undeclared label.
    UndefinedLabel {
        /// The missing label.
        label: String,
    },
    #[error("VR4300 assembly from {origin:#018x} exceeds the address space")]
    /// Output placement exceeds the represented address space.
    AddressOverflow {
        /// The assembly origin used for placement.
        origin: u64,
    },
    #[error(transparent)]
    /// A placed typed instruction could not be encoded.
    Encode(#[from] EncodeError),
}
