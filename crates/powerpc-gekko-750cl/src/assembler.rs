use std::collections::BTreeMap;
use std::marker::PhantomData;

use thiserror::Error;
use typed_isa_core::{InstructionSpan, TypedIsa};

use crate::{
    BranchOptions, ConditionRegisterBit, EncodeError, Instruction, InstructionForm,
    InstructionModifiers, Operand, PowerPc750Cl, PowerPcGekko,
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

/// Label-resolving assembler for a selected PowerPC processor profile.
///
/// Instructions remain typed until final placement. Raw data may be inserted,
/// but any following instruction must still begin at a four-byte boundary.
#[derive(Debug, Clone)]
pub struct ProfileAssembler<P> {
    nodes: Vec<Node>,
    profile: PhantomData<P>,
}

/// PowerPC 750CL assembler using the full 750CL SPR map.
pub type Assembler = ProfileAssembler<PowerPc750Cl>;
/// IBM Gekko assembler that rejects 750CL-only SPRs.
pub type GekkoAssembler = ProfileAssembler<PowerPcGekko>;

impl<P> Default for ProfileAssembler<P> {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            profile: PhantomData,
        }
    }
}

impl<P> ProfileAssembler<P>
where
    P: TypedIsa<Instruction = Instruction, Form = InstructionForm, EncodeError = EncodeError>,
{
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

    /// Appends an unconditional relative branch to `label`.
    ///
    /// When `link` is true, the encoded branch updates the link register.
    pub fn branch_to_label(&mut self, link: bool, label: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::Branch {
            link,
            label: label.into(),
        });
        self
    }

    /// Appends a conditional relative branch to `label`.
    ///
    /// The branch-options and condition-bit fields are retained as typed
    /// operands; displacement range is checked after labels are placed.
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

    /// Emits declared non-executable data without interpreting it as an instruction.
    pub fn data(&mut self, bytes: impl Into<Vec<u8>>) -> &mut Self {
        self.nodes.push(Node::Data(bytes.into()));
        self
    }

    /// Resolves labels and encodes the program at `origin`.
    ///
    /// The origin and every instruction placement must be four-byte aligned,
    /// and every resolved address must remain within the 32-bit address space.
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
            let encoded = P::encode_one(&instruction, address)?;
            let offset = bytes.len();
            bytes.extend_from_slice(encoded.as_slice());
            instruction_spans.push(InstructionSpan {
                offset,
                address,
                bytes: encoded,
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

/// Immutable bytes, labels, and typed instruction spans from assembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledProgram {
    origin: u64,
    bytes: Vec<u8>,
    labels: BTreeMap<String, u64>,
    instruction_spans: Vec<InstructionSpan<Instruction>>,
}

impl AssembledProgram {
    /// Returns the requested program origin.
    pub const fn origin(&self) -> u64 {
        self.origin
    }

    /// Returns the complete assembled byte stream, including declared data.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consumes the result and returns its byte stream.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Returns the resolved absolute address of `name`, if declared.
    pub fn label_address(&self, name: &str) -> Option<u64> {
        self.labels.get(name).copied()
    }

    /// Returns spans for executable instructions only.
    ///
    /// Declared data is present in [`Self::bytes`] but has no instruction span.
    pub fn instruction_spans(&self) -> &[InstructionSpan<Instruction>] {
        &self.instruction_spans
    }
}

/// Failure to place, resolve, or encode an assembler program.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AssemblerError {
    /// The requested program origin is not four-byte aligned.
    #[error("PowerPC assembly origin {origin:#018x} is not four-byte aligned")]
    UnalignedOrigin {
        /// Rejected origin address.
        origin: u64,
    },
    /// Declared data leaves a following instruction unaligned.
    #[error("PowerPC instruction at output offset {offset:#x} is not four-byte aligned")]
    UnalignedInstruction {
        /// Byte offset of the unaligned instruction.
        offset: usize,
    },
    /// A label name is declared more than once.
    #[error("duplicate label `{label}`")]
    DuplicateLabel {
        /// Duplicated label name.
        label: String,
    },
    /// A branch refers to an undeclared label.
    #[error("undefined label `{label}`")]
    UndefinedLabel {
        /// Missing label name.
        label: String,
    },
    /// Output length cannot be added to the origin without overflow.
    #[error("PowerPC assembly from {origin:#018x} exceeds the address space")]
    AddressOverflow {
        /// Program origin involved in the overflow.
        origin: u64,
    },
    /// A resolved label address exceeds the 32-bit target address space.
    #[error("PowerPC label address {address:#018x} exceeds the 32-bit address space")]
    AddressOutOfRange {
        /// Resolved address that cannot be represented.
        address: u64,
    },
    /// An instruction failed profile validation or encoding.
    #[error(transparent)]
    Encode(#[from] EncodeError),
}
