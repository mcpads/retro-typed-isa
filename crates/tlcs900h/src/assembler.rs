use std::collections::BTreeMap;

use thiserror::Error;

use crate::{
    Condition, DecodeError, EncodeError, Instruction, MainInstruction, decode_bytes, encode_bytes,
};

#[derive(Debug, Clone)]
enum Node {
    Label(String),
    Instruction(Instruction),
    JumpRelative { condition: Condition, label: String },
    JumpRelativeLong { condition: Condition, label: String },
    CallRelative(String),
    JumpDirect(String),
    CallDirect(String),
    Data(Vec<u8>),
}

/// Label-resolving assembler for typed TLCS-900/H instructions.
///
/// Relative and direct targets are resolved after variable-length placement;
/// declared data is retained without being interpreted as code.
#[derive(Debug, Clone, Default)]
pub struct Assembler {
    nodes: Vec<Node>,
}

impl Assembler {
    /// Creates an empty assembler program.
    pub fn new() -> Self {
        Self::default()
    }

    /// Declares `name` at the current byte address.
    pub fn label(&mut self, name: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::Label(name.into()));
        self
    }

    /// Appends one typed instruction.
    pub fn emit(&mut self, instruction: Instruction) -> &mut Self {
        self.nodes.push(Node::Instruction(instruction));
        self
    }

    /// Appends an eight-bit conditional relative jump to `label`.
    pub fn jump_relative_to_label(
        &mut self,
        condition: Condition,
        label: impl Into<String>,
    ) -> &mut Self {
        self.nodes.push(Node::JumpRelative {
            condition,
            label: label.into(),
        });
        self
    }

    /// Appends a sixteen-bit conditional relative jump to `label`.
    pub fn jump_relative_long_to_label(
        &mut self,
        condition: Condition,
        label: impl Into<String>,
    ) -> &mut Self {
        self.nodes.push(Node::JumpRelativeLong {
            condition,
            label: label.into(),
        });
        self
    }

    /// Appends a sixteen-bit PC-relative call to `label`.
    pub fn call_relative_to_label(&mut self, label: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::CallRelative(label.into()));
        self
    }

    /// Appends a 24-bit direct jump to `label`.
    pub fn jump_to_label(&mut self, label: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::JumpDirect(label.into()));
        self
    }

    /// Appends a 24-bit direct call to `label`.
    pub fn call_to_label(&mut self, label: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::CallDirect(label.into()));
        self
    }

    /// Emits declared non-executable data without treating it as an instruction.
    pub fn data(&mut self, bytes: impl Into<Vec<u8>>) -> &mut Self {
        self.nodes.push(Node::Data(bytes.into()));
        self
    }

    /// Resolves labels and encodes the complete program at `origin`.
    ///
    /// Every placed address must remain in the 24-bit address space, and each
    /// instruction span is decoded and re-encoded before success.
    pub fn assemble(&self, origin: u32) -> Result<AssembledProgram, AssemblerError> {
        require_address(origin)?;
        let (labels, byte_len) = self.collect_labels(origin)?;
        let mut bytes = Vec::with_capacity(byte_len);
        let mut instruction_spans = Vec::new();

        for node in &self.nodes {
            let address = placed_address(origin, bytes.len())?;
            let instruction = match node {
                Node::Label(_) => continue,
                Node::Data(data) => {
                    bytes.extend_from_slice(data);
                    continue;
                }
                Node::Instruction(instruction) => *instruction,
                Node::JumpRelative { condition, label } => {
                    Instruction::Main(MainInstruction::JumpRelative {
                        condition: *condition,
                        displacement: i8::try_from(relative_displacement(
                            address,
                            2,
                            label_address(&labels, label)?,
                        )?)
                        .map_err(|_| {
                            AssemblerError::RelativeJumpOutOfRange {
                                label: label.clone(),
                                width: 8,
                            }
                        })?,
                    })
                }
                Node::JumpRelativeLong { condition, label } => {
                    Instruction::Main(MainInstruction::JumpRelativeLong {
                        condition: *condition,
                        displacement: i16::try_from(relative_displacement(
                            address,
                            3,
                            label_address(&labels, label)?,
                        )?)
                        .map_err(|_| {
                            AssemblerError::RelativeJumpOutOfRange {
                                label: label.clone(),
                                width: 16,
                            }
                        })?,
                    })
                }
                Node::CallRelative(label) => Instruction::Main(MainInstruction::CallRelative(
                    i16::try_from(relative_displacement(
                        address,
                        3,
                        label_address(&labels, label)?,
                    )?)
                    .map_err(|_| AssemblerError::RelativeCallOutOfRange {
                        label: label.clone(),
                    })?,
                )),
                Node::JumpDirect(label) => Instruction::Main(MainInstruction::JumpDirect24(
                    label_address(&labels, label)?,
                )),
                Node::CallDirect(label) => Instruction::Main(MainInstruction::CallDirect24(
                    label_address(&labels, label)?,
                )),
            };

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
            let length = match node {
                Node::Instruction(instruction) => encode_bytes(instruction)?.len(),
                Node::JumpRelative { .. } => 2,
                Node::JumpRelativeLong { .. } | Node::CallRelative(_) => 3,
                Node::JumpDirect(_) | Node::CallDirect(_) => 4,
                Node::Data(data) => data.len(),
                Node::Label(_) => unreachable!(),
            };
            offset = offset
                .checked_add(length)
                .ok_or(AssemblerError::AddressOverflow { origin })?;
            placed_address(origin, offset)?;
        }
        Ok((labels, offset))
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

fn require_address(address: u32) -> Result<(), AssemblerError> {
    if address <= 0x00ff_ffff {
        Ok(())
    } else {
        Err(AssemblerError::AddressOutOfRange { address })
    }
}

fn placed_address(origin: u32, offset: usize) -> Result<u32, AssemblerError> {
    let offset = u32::try_from(offset).map_err(|_| AssemblerError::AddressOverflow { origin })?;
    let address = origin
        .checked_add(offset)
        .ok_or(AssemblerError::AddressOverflow { origin })?;
    require_address(address)?;
    Ok(address)
}

fn relative_displacement(
    address: u32,
    instruction_len: u32,
    target: u32,
) -> Result<i32, AssemblerError> {
    let base = address
        .checked_add(instruction_len)
        .ok_or(AssemblerError::AddressOverflow { origin: address })?;
    i32::try_from(i64::from(target) - i64::from(base))
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

/// Location and typed identity of one executable instruction in assembled bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionSpan {
    /// Byte offset from the program origin.
    pub offset: usize,
    /// Absolute 24-bit instruction address.
    pub address: u32,
    /// Semantic instruction placed at the address.
    pub instruction: Instruction,
    /// Encoded variable instruction length.
    pub byte_len: usize,
}

/// Immutable bytes, labels, and typed instruction spans from assembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledProgram {
    origin: u32,
    bytes: Vec<u8>,
    labels: BTreeMap<String, u32>,
    instruction_spans: Vec<InstructionSpan>,
}

impl AssembledProgram {
    /// Returns the requested program origin.
    pub const fn origin(&self) -> u32 {
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

    /// Returns the resolved address of `name`, if declared.
    pub fn label_address(&self, name: &str) -> Option<u32> {
        self.labels.get(name).copied()
    }

    /// Returns spans for executable instructions only.
    pub fn instruction_spans(&self) -> &[InstructionSpan] {
        &self.instruction_spans
    }
}

/// Failure to resolve, place, encode, or verify an assembler program.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AssemblerError {
    /// A placed address exceeds the 24-bit target address space.
    #[error("TLCS-900/H address {address:#010x} is outside the 24-bit address space")]
    AddressOutOfRange {
        /// Rejected address.
        address: u32,
    },
    /// A label name is declared more than once.
    #[error("duplicate label `{label}`")]
    DuplicateLabel {
        /// Duplicated label name.
        label: String,
    },
    /// A control-flow node refers to an undeclared label.
    #[error("undefined label `{label}`")]
    UndefinedLabel {
        /// Missing label name.
        label: String,
    },
    /// The origin plus output length overflows before address validation.
    #[error("TLCS-900/H assembly from {origin:#010x} exceeds the address space")]
    AddressOverflow {
        /// Program origin involved in the overflow.
        origin: u32,
    },
    /// Computing a relative displacement exceeds the intermediate signed range.
    #[error("relative address from {address:#010x} to {target:#010x} exceeds signed 32-bit range")]
    RelativeAddressOutOfRange {
        /// Address of the control-flow instruction.
        address: u32,
        /// Requested target address.
        target: u32,
    },
    /// A relative jump target cannot be represented by its selected width.
    #[error("TLCS-900/H {width}-bit relative jump cannot reach label `{label}`")]
    RelativeJumpOutOfRange {
        /// Target label name.
        label: String,
        /// Signed displacement width.
        width: u8,
    },
    /// A relative call target cannot be represented by its signed 16-bit field.
    #[error("TLCS-900/H relative call cannot reach label `{label}`")]
    RelativeCallOutOfRange {
        /// Target label name.
        label: String,
    },
    /// Encoding a typed instruction failed.
    #[error(transparent)]
    Encode(#[from] EncodeError),
    /// Verification decoding failed.
    #[error(transparent)]
    Decode(#[from] DecodeError),
    /// An instruction span did not decode and re-encode exactly.
    #[error("TLCS-900/H instruction verification failed at output offset {offset:#x}")]
    VerificationFailed {
        /// Byte offset of the mismatched instruction.
        offset: usize,
    },
}
