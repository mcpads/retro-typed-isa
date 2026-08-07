use std::collections::BTreeMap;

use typed_isa_core::{InstructionBytes, InstructionSpan};

use crate::{CodeMode, DecodeError, EncodeError, Instruction, decode_bytes, encode_bytes};

#[derive(Debug, Clone)]
enum Node {
    Label(String),
    Instruction(Instruction),
    Data(Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct Assembler {
    code_mode: CodeMode,
    nodes: Vec<Node>,
}

impl Assembler {
    pub const fn new(code_mode: CodeMode) -> Self {
        Self {
            code_mode,
            nodes: Vec::new(),
        }
    }

    pub const fn code_mode(&self) -> CodeMode {
        self.code_mode
    }

    pub fn label(&mut self, name: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::Label(name.into()));
        self
    }

    pub fn emit(&mut self, instruction: Instruction) -> &mut Self {
        self.nodes.push(Node::Instruction(instruction));
        self
    }

    /// Emit declared non-executable data without interpreting it as an instruction.
    pub fn data(&mut self, bytes: impl Into<Vec<u8>>) -> &mut Self {
        self.nodes.push(Node::Data(bytes.into()));
        self
    }

    pub fn assemble(&self, origin: u64) -> Result<AssembledProgram, AssemblerError> {
        let mut labels = BTreeMap::new();
        let mut bytes = Vec::new();
        let mut instruction_spans = Vec::new();

        for node in &self.nodes {
            match node {
                Node::Label(label) => {
                    let address = placed_address(origin, bytes.len())?;
                    if labels.insert(label.clone(), address).is_some() {
                        return Err(AssemblerError::DuplicateLabel {
                            label: label.clone(),
                        });
                    }
                }
                Node::Data(data) => bytes.extend_from_slice(data),
                Node::Instruction(instruction) => {
                    if instruction.code_mode() != self.code_mode {
                        return Err(AssemblerError::CodeModeMismatch {
                            assembler: self.code_mode,
                            instruction: instruction.code_mode(),
                        });
                    }
                    let address = placed_address(origin, bytes.len())?;
                    let encoded = encode_bytes(instruction)?;
                    let decoded = decode_bytes(&encoded, self.code_mode)?;
                    if decoded.instruction() != instruction || decoded.source_bytes() != encoded {
                        return Err(AssemblerError::VerificationFailed {
                            offset: bytes.len(),
                        });
                    }
                    let offset = bytes.len();
                    bytes.extend_from_slice(&encoded);
                    instruction_spans.push(InstructionSpan {
                        offset,
                        address,
                        bytes: InstructionBytes::new(encoded)
                            .expect("encoded Intel 80386 instructions are non-empty"),
                        instruction: instruction.clone(),
                    });
                }
            }
        }

        Ok(AssembledProgram {
            origin,
            code_mode: self.code_mode,
            bytes,
            labels,
            instruction_spans,
        })
    }
}

fn placed_address(origin: u64, offset: usize) -> Result<u64, AssemblerError> {
    origin
        .checked_add(u64::try_from(offset).map_err(|_| AssemblerError::AddressOverflow { origin })?)
        .ok_or(AssemblerError::AddressOverflow { origin })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledProgram {
    origin: u64,
    code_mode: CodeMode,
    bytes: Vec<u8>,
    labels: BTreeMap<String, u64>,
    instruction_spans: Vec<InstructionSpan<Instruction>>,
}

impl AssembledProgram {
    pub const fn origin(&self) -> u64 {
        self.origin
    }

    pub const fn code_mode(&self) -> CodeMode {
        self.code_mode
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn label_address(&self, name: &str) -> Option<u64> {
        self.labels.get(name).copied()
    }

    pub fn instruction_spans(&self) -> &[InstructionSpan<Instruction>] {
        &self.instruction_spans
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AssemblerError {
    #[error("duplicate label `{label}`")]
    DuplicateLabel { label: String },
    #[error("Intel 80386 assembly from {origin:#018x} exceeds the address space")]
    AddressOverflow { origin: u64 },
    #[error(
        "assembler code mode {assembler:?} does not match instruction code mode {instruction:?}"
    )]
    CodeModeMismatch {
        assembler: CodeMode,
        instruction: CodeMode,
    },
    #[error(transparent)]
    Encode(#[from] EncodeError),
    #[error(transparent)]
    Decode(#[from] DecodeError),
    #[error("Intel 80386 instruction verification failed at output offset {offset:#x}")]
    VerificationFailed { offset: usize },
}
