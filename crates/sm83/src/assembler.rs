use std::collections::BTreeMap;

use thiserror::Error;

use crate::{Condition, DecodeError, Instruction, decode_bytes, encode_bytes};

#[derive(Debug, Clone)]
enum Node {
    Label(String),
    Instruction(Instruction),
    RelativeBranch {
        condition: Option<Condition>,
        label: String,
    },
    AbsoluteBranch {
        branch: AbsoluteBranch,
        label: String,
    },
    Data(Vec<u8>),
    LabelWord(String),
}

#[derive(Debug, Clone, Copy)]
enum AbsoluteBranch {
    Jump,
    JumpIf(Condition),
    Call,
    CallIf(Condition),
}

/// Two-pass SM83 assembler with checked label placement.
#[derive(Debug, Clone, Default)]
pub struct Assembler {
    nodes: Vec<Node>,
}

impl Assembler {
    /// Construct an empty assembly program.
    pub fn new() -> Self {
        Self::default()
    }

    /// Define a label at the current byte position.
    pub fn label(&mut self, name: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::Label(name.into()));
        self
    }

    /// Append one fully typed instruction.
    pub fn emit(&mut self, instruction: Instruction) -> &mut Self {
        self.nodes.push(Node::Instruction(instruction));
        self
    }

    /// Append a sequence of fully typed instructions in iteration order.
    pub fn emit_all(&mut self, instructions: impl IntoIterator<Item = Instruction>) -> &mut Self {
        self.nodes
            .extend(instructions.into_iter().map(Node::Instruction));
        self
    }

    /// Append an unconditional relative jump to a named label.
    pub fn jump_relative(&mut self, label: impl Into<String>) -> &mut Self {
        self.relative_branch(None, label)
    }

    /// Append a conditional relative jump to a named label.
    pub fn jump_relative_if(
        &mut self,
        condition: Condition,
        label: impl Into<String>,
    ) -> &mut Self {
        self.relative_branch(Some(condition), label)
    }

    fn relative_branch(
        &mut self,
        condition: Option<Condition>,
        label: impl Into<String>,
    ) -> &mut Self {
        self.nodes.push(Node::RelativeBranch {
            condition,
            label: label.into(),
        });
        self
    }

    /// Append an unconditional absolute jump to a named label.
    pub fn jump(&mut self, label: impl Into<String>) -> &mut Self {
        self.absolute_branch(AbsoluteBranch::Jump, label)
    }

    /// Append a conditional absolute jump to a named label.
    pub fn jump_if(&mut self, condition: Condition, label: impl Into<String>) -> &mut Self {
        self.absolute_branch(AbsoluteBranch::JumpIf(condition), label)
    }

    /// Append an unconditional call to a named label.
    pub fn call(&mut self, label: impl Into<String>) -> &mut Self {
        self.absolute_branch(AbsoluteBranch::Call, label)
    }

    /// Append a conditional call to a named label.
    pub fn call_if(&mut self, condition: Condition, label: impl Into<String>) -> &mut Self {
        self.absolute_branch(AbsoluteBranch::CallIf(condition), label)
    }

    fn absolute_branch(&mut self, branch: AbsoluteBranch, label: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::AbsoluteBranch {
            branch,
            label: label.into(),
        });
        self
    }

    /// Emit explicitly declared data, never an executable-instruction fallback.
    pub fn data(&mut self, bytes: impl Into<Vec<u8>>) -> &mut Self {
        self.nodes.push(Node::Data(bytes.into()));
        self
    }

    /// Append one literal little-endian word as explicitly declared data.
    pub fn word(&mut self, value: u16) -> &mut Self {
        self.data(value.to_le_bytes().to_vec())
    }

    /// Append the resolved little-endian address of a named label as data.
    pub fn label_word(&mut self, label: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::LabelWord(label.into()));
        self
    }

    /// Resolve labels, lower typed code, and verify every instruction span.
    ///
    /// The complete output must fit between `origin` and the end of the 16-bit
    /// address space. Explicit data remains outside the instruction-span list.
    pub fn assemble(&self, origin: u16) -> Result<AssembledProgram, AssemblerError> {
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
                Node::LabelWord(label) => {
                    bytes.extend_from_slice(&label_address(&labels, label)?.to_le_bytes());
                    continue;
                }
                Node::Instruction(instruction) => *instruction,
                Node::RelativeBranch { condition, label } => {
                    let address = placed_address(origin, bytes.len())?;
                    let target = label_address(&labels, label)?;
                    let displacement = relative_displacement(address, target, label)?;
                    match condition {
                        Some(condition) => Instruction::JumpRelativeIf(*condition, displacement),
                        None => Instruction::JumpRelative(displacement),
                    }
                }
                Node::AbsoluteBranch { branch, label } => {
                    let address = label_address(&labels, label)?;
                    match branch {
                        AbsoluteBranch::Jump => Instruction::Jump(address),
                        AbsoluteBranch::JumpIf(condition) => {
                            Instruction::JumpIf(*condition, address)
                        }
                        AbsoluteBranch::Call => Instruction::Call(address),
                        AbsoluteBranch::CallIf(condition) => {
                            Instruction::CallIf(*condition, address)
                        }
                    }
                }
            };
            let encoded = encode_bytes(&instruction);
            let offset = bytes.len();
            bytes.extend_from_slice(&encoded);
            instruction_spans.push(InstructionSpan {
                offset,
                len: encoded.len(),
                instruction,
            });
        }

        debug_assert_eq!(bytes.len(), byte_len);
        verify_instruction_spans(&bytes, &instruction_spans)?;
        Ok(AssembledProgram {
            origin,
            bytes,
            labels,
            instruction_spans,
        })
    }

    fn collect_labels(
        &self,
        origin: u16,
    ) -> Result<(BTreeMap<String, u16>, usize), AssemblerError> {
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
            offset = offset
                .checked_add(node_len(node))
                .ok_or(AssemblerError::AddressOverflow {
                    origin,
                    byte_len: usize::MAX,
                })?;
            ensure_address_span(origin, offset)?;
        }
        Ok((labels, offset))
    }
}

fn node_len(node: &Node) -> usize {
    match node {
        Node::Label(_) => 0,
        Node::Instruction(instruction) => instruction.encoded_len(),
        Node::RelativeBranch { .. } => 2,
        Node::AbsoluteBranch { .. } => 3,
        Node::Data(data) => data.len(),
        Node::LabelWord(_) => 2,
    }
}

fn placed_address(origin: u16, offset: usize) -> Result<u16, AssemblerError> {
    let address =
        usize::from(origin)
            .checked_add(offset)
            .ok_or(AssemblerError::AddressOverflow {
                origin,
                byte_len: offset,
            })?;
    u16::try_from(address).map_err(|_| AssemblerError::AddressOverflow {
        origin,
        byte_len: offset,
    })
}

fn ensure_address_span(origin: u16, byte_len: usize) -> Result<(), AssemblerError> {
    if usize::from(origin)
        .checked_add(byte_len)
        .is_none_or(|end| end > 0x1_0000)
    {
        return Err(AssemblerError::AddressOverflow { origin, byte_len });
    }
    Ok(())
}

fn label_address(labels: &BTreeMap<String, u16>, label: &str) -> Result<u16, AssemblerError> {
    labels
        .get(label)
        .copied()
        .ok_or_else(|| AssemblerError::UndefinedLabel {
            label: label.to_owned(),
        })
}

fn relative_displacement(address: u16, target: u16, label: &str) -> Result<i8, AssemblerError> {
    let next = i32::from(address) + 2;
    let displacement = i32::from(target) - next;
    i8::try_from(displacement).map_err(|_| AssemblerError::RelativeBranchOutOfRange {
        label: label.to_owned(),
        address,
        target,
    })
}

fn verify_instruction_spans(bytes: &[u8], spans: &[InstructionSpan]) -> Result<(), AssemblerError> {
    for span in spans {
        let source = bytes.get(span.offset..span.offset + span.len).ok_or(
            AssemblerError::SemanticRoundTripMismatch {
                offset: span.offset,
            },
        )?;
        let decoded = decode_bytes(source)?;
        if decoded.into_instruction() != span.instruction
            || encode_bytes(&span.instruction) != source
        {
            return Err(AssemblerError::SemanticRoundTripMismatch {
                offset: span.offset,
            });
        }
    }
    Ok(())
}

/// Location and typed identity of one instruction in assembled output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstructionSpan {
    /// Byte offset relative to the beginning of the assembled program.
    pub offset: usize,
    /// Number of bytes occupied by the instruction.
    pub len: usize,
    /// Typed instruction that produced the bytes in this span.
    pub instruction: Instruction,
}

/// Checked assembled bytes with resolved labels and verified instruction spans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledProgram {
    origin: u16,
    bytes: Vec<u8>,
    labels: BTreeMap<String, u16>,
    instruction_spans: Vec<InstructionSpan>,
}

impl AssembledProgram {
    /// Return the 16-bit address assigned to the first output byte.
    pub const fn origin(&self) -> u16 {
        self.origin
    }

    /// Borrow the complete assembled code and explicit data bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consume the program and return its complete byte sequence.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Return the resolved 16-bit address of a defined label, if present.
    pub fn label_address(&self, name: &str) -> Option<u16> {
        self.labels.get(name).copied()
    }

    /// Borrow the locations and identities of executable instructions only.
    pub fn instruction_spans(&self) -> &[InstructionSpan] {
        &self.instruction_spans
    }
}

/// A fail-closed reason that SM83 assembly did not produce a program.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AssemblerError {
    /// A label name was defined more than once.
    #[error("duplicate label `{label}`")]
    DuplicateLabel {
        /// Repeated label name.
        label: String,
    },
    /// A label reference names no definition in the program.
    #[error("undefined label `{label}`")]
    UndefinedLabel {
        /// Missing label name.
        label: String,
    },
    /// The complete output would extend beyond the 16-bit address space.
    #[error("SM83 program from {origin:#06x} with {byte_len} byte(s) exceeds its address space")]
    AddressOverflow {
        /// Address assigned to the first output byte.
        origin: u16,
        /// Complete or intermediate output length that does not fit.
        byte_len: usize,
    },
    /// A relative jump target is outside the signed 8-bit displacement range.
    #[error("relative branch at {address:#06x} cannot reach label `{label}` at {target:#06x}")]
    RelativeBranchOutOfRange {
        /// Label targeted by the jump.
        label: String,
        /// Address of the relative-jump instruction.
        address: u16,
        /// Resolved address of the target label.
        target: u16,
    },
    /// Post-placement decoding of an instruction span failed.
    #[error(transparent)]
    Decode(#[from] DecodeError),
    /// An emitted instruction changed identity during byte round-trip verification.
    #[error("SM83 semantic byte round-trip failed at output offset {offset:#x}")]
    SemanticRoundTripMismatch {
        /// Byte offset of the inconsistent instruction span.
        offset: usize,
    },
}
