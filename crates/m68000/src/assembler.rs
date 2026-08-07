use std::collections::BTreeMap;

use thiserror::Error;

use crate::{
    BranchDisplacement, BranchKind, Condition, DataRegister, DecodeError, EncodeError, Instruction,
    decode_bytes, encode_bytes,
};

#[derive(Debug, Clone, PartialEq, Eq)]
/// A canonically encoded instruction and its absolute placement address.
pub struct PlacedInstruction {
    /// The absolute instruction address.
    pub address: u32,
    /// The typed instruction.
    pub instruction: Instruction,
    /// The exact canonical big-endian instruction bytes.
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A verified MC68000 program and its per-instruction placements.
pub struct AssembledProgram {
    /// The first output address.
    pub origin: u32,
    /// The exact canonical big-endian instruction bytes.
    pub bytes: Vec<u8>,
    /// The verified instruction placements.
    pub instructions: Vec<PlacedInstruction>,
}

impl AssembledProgram {
    /// Returns the first output address.
    pub const fn origin(&self) -> u32 {
        self.origin
    }

    /// Borrows the complete program bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consumes the program and returns its complete byte stream.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Returns the verified instruction placements.
    pub fn instructions(&self) -> &[PlacedInstruction] {
        &self.instructions
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
/// Failure returned while resolving, encoding, or verifying an assembly unit.
pub enum AssemblerError {
    #[error("MC68000 program origin 0x{origin:08X} is not word-aligned")]
    /// The program origin is not word-aligned.
    UnalignedOrigin {
        /// The rejected origin.
        origin: u32,
    },
    #[error("duplicate label {label:?}")]
    /// A label name was declared more than once.
    DuplicateLabel {
        /// The duplicate label.
        label: String,
    },
    #[error("undefined label {label:?}")]
    /// A symbolic branch references an undeclared label.
    UndefinedLabel {
        /// The missing label.
        label: String,
    },
    #[error(
        "branch at 0x{address:08X} cannot reach 0x{target:08X} with a signed 16-bit displacement"
    )]
    /// A symbolic branch target does not fit a signed word displacement.
    BranchOutOfRange {
        /// The branch instruction address.
        address: u32,
        /// The requested target address.
        target: u32,
    },
    #[error("address overflow from 0x{origin:08X} at node {node_index}")]
    /// Program placement exceeds the 32-bit address space.
    AddressOverflow {
        /// The assembly origin.
        origin: u32,
        /// The node being placed when overflow occurred.
        node_index: usize,
    },
    #[error(transparent)]
    /// A typed instruction could not be encoded.
    Encode(#[from] EncodeError),
    #[error("decode failed at 0x{address:08X}: {source}")]
    /// Verification could not decode an instruction from the emitted stream.
    Decode {
        /// The absolute instruction address.
        address: u32,
        #[source]
        /// The underlying decode failure.
        source: DecodeError,
    },
    #[error(
        "semantic byte round trip changed instruction at 0x{address:08X}: expected {expected:?}, decoded {actual:?}"
    )]
    /// Emitted bytes decoded to a different typed instruction.
    SemanticRoundTripMismatch {
        /// The absolute instruction address.
        address: u32,
        /// The instruction submitted to the assembler.
        expected: Instruction,
        /// The instruction recovered from emitted bytes.
        actual: Instruction,
    },
    #[error("instruction at 0x{address:08X} is not canonically encoded")]
    /// Input bytes decode but are not the canonical lowering of that typed instruction.
    NonCanonicalEncoding {
        /// The absolute instruction address.
        address: u32,
        /// The canonical encoding.
        canonical: Vec<u8>,
        /// The instruction recovered from emitted bytes.
        actual: Vec<u8>,
    },
}

#[derive(Debug, Clone, Default)]
/// A checked MC68000 assembler with symbolic branches.
pub struct Assembler {
    nodes: Vec<Node>,
}

impl Assembler {
    /// Creates an empty assembly unit.
    pub fn new() -> Self {
        Self::default()
    }

    /// Declares a label at the current output address.
    pub fn label(&mut self, name: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::Label(name.into()));
        self
    }

    /// Appends an already-typed instruction.
    pub fn emit(&mut self, instruction: Instruction) -> &mut Self {
        self.nodes.push(Node::Instruction(instruction));
        self
    }

    /// Appends typed instructions in iteration order.
    pub fn emit_all(&mut self, instructions: impl IntoIterator<Item = Instruction>) -> &mut Self {
        self.nodes
            .extend(instructions.into_iter().map(Node::Instruction));
        self
    }

    /// Emit a word-displacement `BRA` to a label.
    pub fn bra(&mut self, label: impl Into<String>) -> &mut Self {
        self.branch(BranchKind::Always, label)
    }

    /// Emit a word-displacement `BSR` to a label.
    pub fn bsr(&mut self, label: impl Into<String>) -> &mut Self {
        self.branch(BranchKind::Subroutine, label)
    }

    /// Emit a word-displacement conditional branch to a label.
    pub fn bcc(&mut self, condition: Condition, label: impl Into<String>) -> &mut Self {
        self.branch(BranchKind::Conditional(condition), label)
    }

    fn branch(&mut self, kind: BranchKind, label: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::Branch {
            kind,
            label: label.into(),
        });
        self
    }

    /// Emit a `DBcc` to a label.
    pub fn dbcc(
        &mut self,
        condition: Condition,
        register: DataRegister,
        label: impl Into<String>,
    ) -> &mut Self {
        self.nodes.push(Node::DecrementBranch {
            condition,
            register,
            label: label.into(),
        });
        self
    }

    /// Resolves labels, encodes every instruction, and verifies the complete placed byte stream.
    pub fn assemble(&self, origin: u32) -> Result<AssembledProgram, AssemblerError> {
        if origin & 1 != 0 {
            return Err(AssemblerError::UnalignedOrigin { origin });
        }
        let labels = self.collect_labels(origin)?;
        let mut bytes = Vec::new();
        let mut expected = Vec::new();
        let mut address = origin;
        for (node_index, node) in self.nodes.iter().enumerate() {
            let instruction = match node {
                Node::Label(_) => continue,
                Node::Instruction(instruction) => *instruction,
                Node::Branch { kind, label } => Instruction::Branch {
                    kind: *kind,
                    displacement: BranchDisplacement::Word(resolve_displacement(
                        address, label, &labels,
                    )?),
                },
                Node::DecrementBranch {
                    condition,
                    register,
                    label,
                } => Instruction::DecrementBranch {
                    condition: *condition,
                    register: *register,
                    displacement: resolve_displacement(address, label, &labels)?,
                },
            };
            let encoded = encode_bytes(&instruction)?;
            expected.push(PlacedInstruction {
                address,
                instruction,
                bytes: encoded.clone(),
            });
            bytes.extend_from_slice(&encoded);
            address = add_length(origin, address, encoded.len(), node_index)?;
        }

        let actual = verify_placed_program(&bytes, origin)?;
        for (expected, actual) in expected.iter().zip(&actual) {
            if expected.instruction != actual.instruction {
                return Err(AssemblerError::SemanticRoundTripMismatch {
                    address: expected.address,
                    expected: expected.instruction,
                    actual: actual.instruction,
                });
            }
        }
        Ok(AssembledProgram {
            origin,
            bytes,
            instructions: actual,
        })
    }

    fn collect_labels(&self, origin: u32) -> Result<BTreeMap<String, u32>, AssemblerError> {
        let mut labels = BTreeMap::new();
        let mut address = origin;
        for (node_index, node) in self.nodes.iter().enumerate() {
            match node {
                Node::Label(label) => {
                    if labels.insert(label.clone(), address).is_some() {
                        return Err(AssemblerError::DuplicateLabel {
                            label: label.clone(),
                        });
                    }
                }
                Node::Instruction(instruction) => {
                    let len = encode_bytes(instruction)?.len();
                    address = add_length(origin, address, len, node_index)?;
                }
                Node::Branch { .. } | Node::DecrementBranch { .. } => {
                    address = add_length(origin, address, 4, node_index)?;
                }
            }
        }
        Ok(labels)
    }
}

#[derive(Debug, Clone)]
enum Node {
    Label(String),
    Instruction(Instruction),
    Branch {
        kind: BranchKind,
        label: String,
    },
    DecrementBranch {
        condition: Condition,
        register: DataRegister,
        label: String,
    },
}

fn resolve_displacement(
    address: u32,
    label: &str,
    labels: &BTreeMap<String, u32>,
) -> Result<i16, AssemblerError> {
    let target = *labels
        .get(label)
        .ok_or_else(|| AssemblerError::UndefinedLabel {
            label: label.to_owned(),
        })?;
    let base = address
        .checked_add(2)
        .ok_or(AssemblerError::AddressOverflow {
            origin: address,
            node_index: 0,
        })?;
    let displacement = i64::from(target) - i64::from(base);
    i16::try_from(displacement).map_err(|_| AssemblerError::BranchOutOfRange { address, target })
}

fn add_length(
    origin: u32,
    address: u32,
    byte_len: usize,
    node_index: usize,
) -> Result<u32, AssemblerError> {
    let byte_len = u32::try_from(byte_len)
        .map_err(|_| AssemblerError::AddressOverflow { origin, node_index })?;
    address
        .checked_add(byte_len)
        .ok_or(AssemblerError::AddressOverflow { origin, node_index })
}

/// Decode and canonically re-encode an entire placed instruction stream.
pub fn verify_placed_program(
    bytes: &[u8],
    origin: u32,
) -> Result<Vec<PlacedInstruction>, AssemblerError> {
    if origin & 1 != 0 {
        return Err(AssemblerError::UnalignedOrigin { origin });
    }
    let mut placed = Vec::new();
    let mut offset = 0usize;
    while offset < bytes.len() {
        let offset_u32 = u32::try_from(offset).map_err(|_| AssemblerError::AddressOverflow {
            origin,
            node_index: placed.len(),
        })?;
        let address = origin
            .checked_add(offset_u32)
            .ok_or(AssemblerError::AddressOverflow {
                origin,
                node_index: placed.len(),
            })?;
        let decoded = decode_bytes(&bytes[offset..])
            .map_err(|source| AssemblerError::Decode { address, source })?;
        let encoded = encode_bytes(&decoded.instruction)?;
        let actual = &bytes[offset..offset + decoded.byte_len];
        if encoded != actual {
            return Err(AssemblerError::NonCanonicalEncoding {
                address,
                canonical: encoded,
                actual: actual.to_vec(),
            });
        }
        placed.push(PlacedInstruction {
            address,
            instruction: decoded.instruction,
            bytes: encoded,
        });
        offset += decoded.byte_len;
    }
    Ok(placed)
}
