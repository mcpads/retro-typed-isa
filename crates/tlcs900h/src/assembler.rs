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

    pub fn call_relative_to_label(&mut self, label: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::CallRelative(label.into()));
        self
    }

    pub fn jump_to_label(&mut self, label: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::JumpDirect(label.into()));
        self
    }

    pub fn call_to_label(&mut self, label: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::CallDirect(label.into()));
        self
    }

    /// Emit declared non-executable data without treating it as an instruction.
    pub fn data(&mut self, bytes: impl Into<Vec<u8>>) -> &mut Self {
        self.nodes.push(Node::Data(bytes.into()));
        self
    }

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionSpan {
    pub offset: usize,
    pub address: u32,
    pub instruction: Instruction,
    pub byte_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledProgram {
    origin: u32,
    bytes: Vec<u8>,
    labels: BTreeMap<String, u32>,
    instruction_spans: Vec<InstructionSpan>,
}

impl AssembledProgram {
    pub const fn origin(&self) -> u32 {
        self.origin
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn label_address(&self, name: &str) -> Option<u32> {
        self.labels.get(name).copied()
    }

    pub fn instruction_spans(&self) -> &[InstructionSpan] {
        &self.instruction_spans
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AssemblerError {
    #[error("TLCS-900/H address {address:#010x} is outside the 24-bit address space")]
    AddressOutOfRange { address: u32 },
    #[error("duplicate label `{label}`")]
    DuplicateLabel { label: String },
    #[error("undefined label `{label}`")]
    UndefinedLabel { label: String },
    #[error("TLCS-900/H assembly from {origin:#010x} exceeds the address space")]
    AddressOverflow { origin: u32 },
    #[error("relative address from {address:#010x} to {target:#010x} exceeds signed 32-bit range")]
    RelativeAddressOutOfRange { address: u32, target: u32 },
    #[error("TLCS-900/H {width}-bit relative jump cannot reach label `{label}`")]
    RelativeJumpOutOfRange { label: String, width: u8 },
    #[error("TLCS-900/H relative call cannot reach label `{label}`")]
    RelativeCallOutOfRange { label: String },
    #[error(transparent)]
    Encode(#[from] EncodeError),
    #[error(transparent)]
    Decode(#[from] DecodeError),
    #[error("TLCS-900/H instruction verification failed at output offset {offset:#x}")]
    VerificationFailed { offset: usize },
}
