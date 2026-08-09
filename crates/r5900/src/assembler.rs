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

/// Label-resolving assembler for typed R5900 instructions.
///
/// Branch and jump targets are resolved after placement. Declared data is
/// preserved verbatim and may not leave a following instruction unaligned.
#[derive(Debug, Clone, Default)]
pub struct Assembler {
    nodes: Vec<Node>,
}

impl Assembler {
    /// Creates an empty assembler program.
    pub fn new() -> Self {
        Self::default()
    }

    /// Declares `name` at the current output address.
    pub fn label(&mut self, name: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::Label(name.into()));
        self
    }

    /// Appends an already validated instruction.
    pub fn emit(&mut self, instruction: Instruction) -> &mut Self {
        self.nodes.push(Node::Instruction(instruction));
        self
    }

    /// Appends a two-register conditional branch to `label`.
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

    /// Appends a one-register conditional branch to `label`.
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

    /// Appends a COP0, COP1, or COP2 conditional branch to `label`.
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

    /// Appends a direct jump or jump-and-link to `label`.
    pub fn jump_to_label(&mut self, form: InstructionForm, label: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::Jump {
            form,
            label: label.into(),
        });
        self
    }

    /// Emits declared non-executable data without treating it as an instruction.
    pub fn data(&mut self, bytes: impl Into<Vec<u8>>) -> &mut Self {
        self.nodes.push(Node::Data(bytes.into()));
        self
    }

    /// Resolves labels and encodes the program at `origin`.
    ///
    /// The origin and each instruction placement must be four-byte aligned and
    /// the complete output must fit the 32-bit EE address space.
    pub fn assemble(
        &self,
        origin: crate::CodeLocation,
    ) -> Result<AssembledProgram, AssemblerError> {
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
                        target: label_location(&labels, label)?,
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
                        target: label_location(&labels, label)?,
                    },
                )?,
                Node::Branch { form, label } => Instruction::new(
                    *form,
                    Operands::Branch {
                        target: label_location(&labels, label)?,
                    },
                )?,
                Node::Jump { form, label } => Instruction::new(
                    *form,
                    Operands::Jump {
                        target: label_location(&labels, label)?,
                    },
                )?,
            };

            if bytes.len() & 3 != 0 {
                return Err(AssemblerError::UnalignedInstruction {
                    offset: bytes.len(),
                });
            }
            let location = placed_location(origin, bytes.len())?;
            let encoded = encode_bytes(&instruction, location)?;
            let offset = bytes.len();
            bytes.extend_from_slice(&encoded);
            instruction_spans.push(InstructionSpan {
                offset,
                location,
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
        origin: crate::CodeLocation,
    ) -> Result<(BTreeMap<String, u32>, usize), AssemblerError> {
        let mut labels = BTreeMap::new();
        let mut offset = 0usize;
        for node in &self.nodes {
            if let Node::Label(label) = node {
                let location = placed_location(origin, offset)?;
                if labels.insert(label.clone(), location).is_some() {
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

/// Immutable bytes, labels, and typed instruction spans from assembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledProgram {
    origin: crate::CodeLocation,
    bytes: Vec<u8>,
    labels: BTreeMap<String, u32>,
    instruction_spans: Vec<InstructionSpan<crate::CodeLocation, Instruction>>,
}

impl AssembledProgram {
    /// Returns the requested program origin.
    pub const fn origin(&self) -> crate::CodeLocation {
        self.origin
    }

    /// Returns the complete byte stream, including declared data.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consumes the result and returns its byte stream.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Returns every resolved label address.
    pub fn labels(&self) -> &BTreeMap<String, u32> {
        &self.labels
    }

    /// Returns spans for executable instructions only.
    pub fn instruction_spans(&self) -> &[InstructionSpan<crate::CodeLocation, Instruction>] {
        &self.instruction_spans
    }
}

/// Failure to place, resolve, construct, or encode an assembler program.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AssemblerError {
    /// The requested origin is not four-byte aligned.
    #[error("R5900 origin {origin:#x} is not four-byte aligned")]
    UnalignedOrigin {
        /// Rejected origin address.
        origin: crate::CodeLocation,
    },
    /// A label name is declared more than once.
    #[error("duplicate label {label}")]
    DuplicateLabel {
        /// Duplicated label name.
        label: String,
    },
    /// A control-flow node refers to an undeclared label.
    #[error("undefined label {label}")]
    UndefinedLabel {
        /// Missing label name.
        label: String,
    },
    /// Declared data leaves a following instruction unaligned.
    #[error("instruction at output offset {offset:#x} is not four-byte aligned")]
    UnalignedInstruction {
        /// Byte offset of the unaligned instruction.
        offset: usize,
    },
    /// A placed address or the program extent exceeds the address space.
    #[error("address overflow from origin {origin:#x} at output offset {offset:#x}")]
    AddressOverflow {
        /// Program origin involved in the overflow.
        origin: crate::CodeLocation,
        /// Output offset involved in the overflow.
        offset: usize,
    },
    /// A label-resolved instruction failed typed construction.
    #[error(transparent)]
    Construction(#[from] crate::ConstructionError),
    /// An instruction failed final placement or encoding.
    #[error(transparent)]
    Encode(#[from] EncodeError),
}

fn validate_origin(origin: crate::CodeLocation) -> Result<(), AssemblerError> {
    if origin & 3 != 0 {
        return Err(AssemblerError::UnalignedOrigin { origin });
    }
    Ok(())
}

fn placed_location(
    origin: crate::CodeLocation,
    offset: usize,
) -> Result<crate::CodeLocation, AssemblerError> {
    let placed_offset =
        u32::try_from(offset).map_err(|_| AssemblerError::AddressOverflow { origin, offset })?;
    origin
        .checked_add(placed_offset)
        .ok_or(AssemblerError::AddressOverflow { origin, offset })
}

fn validate_extent(origin: crate::CodeLocation, byte_len: usize) -> Result<(), AssemblerError> {
    let byte_len_u64 = u64::try_from(byte_len).map_err(|_| AssemblerError::AddressOverflow {
        origin,
        offset: byte_len,
    })?;
    let end =
        u64::from(origin)
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

fn label_location(labels: &BTreeMap<String, u32>, label: &str) -> Result<u32, AssemblerError> {
    labels
        .get(label)
        .copied()
        .ok_or_else(|| AssemblerError::UndefinedLabel {
            label: label.to_owned(),
        })
}
