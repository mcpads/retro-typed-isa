use std::collections::BTreeMap;

use typed_isa_core::{InstructionBytes, InstructionSpan};

use crate::{CodeMode, DecodeError, EncodeError, Instruction, decode_bytes, encode_bytes};

#[derive(Debug, Clone)]
enum Node {
    Label(String),
    Instruction(Instruction),
    Data(Vec<u8>),
}

/// Sequential assembler for one explicit 80386 code-segment mode.
///
/// Labels record placed addresses; typed instructions and declared data are
/// emitted in insertion order. Relative fixups are represented by instruction
/// operands rather than label nodes in this assembler.
#[derive(Debug, Clone)]
pub struct Assembler {
    code_mode: CodeMode,
    nodes: Vec<Node>,
}

impl Assembler {
    /// Creates an empty assembler for `code_mode`.
    pub const fn new(code_mode: CodeMode) -> Self {
        Self {
            code_mode,
            nodes: Vec::new(),
        }
    }

    /// Returns the assembler's fixed code-segment mode.
    pub const fn code_mode(&self) -> CodeMode {
        self.code_mode
    }

    /// Declares `name` at the current byte address.
    pub fn label(&mut self, name: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::Label(name.into()));
        self
    }

    /// Appends a typed instruction.
    pub fn emit(&mut self, instruction: Instruction) -> &mut Self {
        self.nodes.push(Node::Instruction(instruction));
        self
    }

    /// Emits declared non-executable data without interpreting it as an instruction.
    pub fn data(&mut self, bytes: impl Into<Vec<u8>>) -> &mut Self {
        self.nodes.push(Node::Data(bytes.into()));
        self
    }

    /// Places and verifies the complete program at `origin`.
    ///
    /// Each instruction must use the assembler's code mode and decode back to
    /// the same typed value and exact byte sequence.
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

/// Immutable bytes, labels, and typed instruction spans from assembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledProgram {
    origin: u64,
    code_mode: CodeMode,
    bytes: Vec<u8>,
    labels: BTreeMap<String, u64>,
    instruction_spans: Vec<InstructionSpan<Instruction>>,
}

impl AssembledProgram {
    /// Returns the requested program origin.
    pub const fn origin(&self) -> u64 {
        self.origin
    }

    /// Returns the code-segment mode used for every instruction.
    pub const fn code_mode(&self) -> CodeMode {
        self.code_mode
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
    pub fn label_address(&self, name: &str) -> Option<u64> {
        self.labels.get(name).copied()
    }

    /// Returns spans for executable instructions only.
    pub fn instruction_spans(&self) -> &[InstructionSpan<Instruction>] {
        &self.instruction_spans
    }
}

/// Failure to place or verify an assembler program.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AssemblerError {
    /// A label name is declared more than once.
    #[error("duplicate label `{label}`")]
    DuplicateLabel {
        /// Duplicated label name.
        label: String,
    },
    /// The origin plus output length overflows a 64-bit address.
    #[error("Intel 80386 assembly from {origin:#018x} exceeds the address space")]
    AddressOverflow {
        /// Program origin involved in the overflow.
        origin: u64,
    },
    /// An instruction was constructed for a different code-segment mode.
    #[error(
        "assembler code mode {assembler:?} does not match instruction code mode {instruction:?}"
    )]
    CodeModeMismatch {
        /// Mode selected for the assembler.
        assembler: CodeMode,
        /// Mode captured by the instruction.
        instruction: CodeMode,
    },
    /// Typed encoding failed.
    #[error(transparent)]
    Encode(#[from] EncodeError),
    /// Verification decoding failed.
    #[error(transparent)]
    Decode(#[from] DecodeError),
    /// Encoded bytes did not decode to the same typed instruction.
    #[error("Intel 80386 instruction verification failed at output offset {offset:#x}")]
    VerificationFailed {
        /// Byte offset of the mismatched instruction.
        offset: usize,
    },
}
