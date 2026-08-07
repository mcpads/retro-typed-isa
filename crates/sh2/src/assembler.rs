use std::collections::BTreeMap;

use thiserror::Error;

use crate::encoder::{EncodeError, VerifyError, encode_word, verify_placed_program};
use crate::instruction::Instruction;

/// Label-resolving assembler for typed SH-2 instructions.
///
/// Branch displacements are resolved after placement. Delayed branches must
/// have a following instruction that is legal in an SH-2 delay slot.
#[derive(Debug, Clone, Default)]
pub struct Assembler {
    nodes: Vec<Node>,
}

impl Assembler {
    /// Creates an empty assembler program.
    pub fn new() -> Self {
        Self::default()
    }

    /// Declares `name` at the current instruction address.
    pub fn label(&mut self, name: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::Label(name.into()));
        self
    }

    /// Appends one semantic instruction.
    pub fn emit(&mut self, instruction: Instruction) -> &mut Self {
        self.nodes.push(Node::Instruction(instruction));
        self
    }

    /// Appends all semantic instructions from `instructions`.
    pub fn emit_all(&mut self, instructions: impl IntoIterator<Item = Instruction>) -> &mut Self {
        self.nodes
            .extend(instructions.into_iter().map(Node::Instruction));
        self
    }

    /// Appends `bt label`.
    pub fn bt(&mut self, label: impl Into<String>) -> &mut Self {
        self.branch(LabelBranch::Bt, label)
    }

    /// Appends `bf label`.
    pub fn bf(&mut self, label: impl Into<String>) -> &mut Self {
        self.branch(LabelBranch::Bf, label)
    }

    /// Appends delayed `bt/s label`.
    pub fn bts(&mut self, label: impl Into<String>) -> &mut Self {
        self.branch(LabelBranch::Bts, label)
    }

    /// Appends delayed `bf/s label`.
    pub fn bfs(&mut self, label: impl Into<String>) -> &mut Self {
        self.branch(LabelBranch::Bfs, label)
    }

    /// Appends unconditional delayed `bra label`.
    pub fn bra(&mut self, label: impl Into<String>) -> &mut Self {
        self.branch(LabelBranch::Bra, label)
    }

    /// Appends delayed subroutine branch `bsr label`.
    pub fn bsr(&mut self, label: impl Into<String>) -> &mut Self {
        self.branch(LabelBranch::Bsr, label)
    }

    fn branch(&mut self, branch: LabelBranch, label: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::Branch {
            branch,
            label: label.into(),
        });
        self
    }

    /// Resolves labels and encodes the complete program at `origin`.
    ///
    /// Branch alignment and reach, delay-slot legality, and semantic byte
    /// round trips are checked after placement.
    pub fn assemble(&self, origin: u32) -> Result<AssembledProgram, AssemblerError> {
        if origin & 1 != 0 {
            return Err(AssemblerError::UnalignedOrigin { origin });
        }
        let labels = self.collect_labels(origin)?;
        let mut instructions = Vec::new();
        for (index, node) in self
            .nodes
            .iter()
            .filter(|node| !matches!(node, Node::Label(_)))
            .enumerate()
        {
            let address = address_for(origin, index)?;
            let instruction = match node {
                Node::Label(_) => unreachable!("labels were filtered"),
                Node::Instruction(instruction) => *instruction,
                Node::Branch { branch, label } => {
                    let target =
                        *labels
                            .get(label)
                            .ok_or_else(|| AssemblerError::UndefinedLabel {
                                label: label.clone(),
                            })?;
                    branch.resolve(address, target)?
                }
            };
            instructions.push(instruction);
        }

        for (index, instruction) in instructions.iter().enumerate() {
            if !instruction.has_delay_slot() {
                continue;
            }
            let branch_address = address_for(origin, index)?;
            let Some(delay_slot) = instructions.get(index + 1) else {
                return Err(AssemblerError::MissingDelaySlot { branch_address });
            };
            if delay_slot.is_illegal_delay_slot() {
                return Err(AssemblerError::IllegalDelaySlot {
                    branch_address,
                    slot_address: address_for(origin, index + 1)?,
                    instruction: *delay_slot,
                });
            }
        }

        let mut words = Vec::with_capacity(instructions.len());
        let mut bytes = Vec::with_capacity(instructions.len() * 2);
        for instruction in &instructions {
            let word = encode_word(instruction).map_err(AssemblerError::Encode)?;
            words.push(word);
            bytes.extend_from_slice(&word.to_be_bytes());
        }

        let decoded = verify_placed_program(&bytes, origin).map_err(AssemblerError::Verify)?;
        for (index, (expected, actual)) in instructions.iter().zip(&decoded).enumerate() {
            if expected != actual {
                return Err(AssemblerError::SemanticRoundTripMismatch {
                    address: address_for(origin, index)?,
                    expected: *expected,
                    actual: *actual,
                });
            }
        }

        Ok(AssembledProgram {
            origin,
            bytes,
            words,
            instructions,
        })
    }

    fn collect_labels(&self, origin: u32) -> Result<BTreeMap<String, u32>, AssemblerError> {
        let mut labels = BTreeMap::new();
        let mut instruction_index = 0usize;
        for node in &self.nodes {
            if let Node::Label(label) = node {
                let address = address_for(origin, instruction_index)?;
                if labels.insert(label.clone(), address).is_some() {
                    return Err(AssemblerError::DuplicateLabel {
                        label: label.clone(),
                    });
                }
            } else {
                instruction_index =
                    instruction_index
                        .checked_add(1)
                        .ok_or(AssemblerError::AddressOverflow {
                            origin,
                            instruction_index,
                        })?;
            }
        }
        address_for(origin, instruction_index)?;
        Ok(labels)
    }
}

fn address_for(origin: u32, instruction_index: usize) -> Result<u32, AssemblerError> {
    let offset = u32::try_from(instruction_index)
        .ok()
        .and_then(|index| index.checked_mul(2))
        .ok_or(AssemblerError::AddressOverflow {
            origin,
            instruction_index,
        })?;
    origin
        .checked_add(offset)
        .ok_or(AssemblerError::AddressOverflow {
            origin,
            instruction_index,
        })
}

#[derive(Debug, Clone)]
enum Node {
    Label(String),
    Instruction(Instruction),
    Branch { branch: LabelBranch, label: String },
}

#[derive(Debug, Clone, Copy)]
enum LabelBranch {
    Bt,
    Bf,
    Bts,
    Bfs,
    Bra,
    Bsr,
}

impl LabelBranch {
    fn resolve(self, address: u32, target: u32) -> Result<Instruction, AssemblerError> {
        if target & 1 != 0 {
            return Err(AssemblerError::UnalignedBranchTarget { address, target });
        }
        let base = address
            .checked_add(4)
            .ok_or(AssemblerError::AddressOverflow {
                origin: address,
                instruction_index: 2,
            })?;
        let byte_delta = i64::from(target) - i64::from(base);
        debug_assert_eq!(byte_delta & 1, 0);
        let displacement = byte_delta / 2;
        match self {
            Self::Bt | Self::Bf | Self::Bts | Self::Bfs => {
                let disp =
                    i8::try_from(displacement).map_err(|_| AssemblerError::BranchOutOfRange {
                        address,
                        target,
                        bits: 8,
                    })?;
                Ok(match self {
                    Self::Bt => Instruction::Bt { disp },
                    Self::Bf => Instruction::Bf { disp },
                    Self::Bts => Instruction::Bts { disp },
                    Self::Bfs => Instruction::Bfs { disp },
                    _ => unreachable!(),
                })
            }
            Self::Bra | Self::Bsr => {
                if !(-2048..=2047).contains(&displacement) {
                    return Err(AssemblerError::BranchOutOfRange {
                        address,
                        target,
                        bits: 12,
                    });
                }
                let disp = displacement as i16;
                Ok(match self {
                    Self::Bra => Instruction::Bra { disp },
                    Self::Bsr => Instruction::Bsr { disp },
                    _ => unreachable!(),
                })
            }
        }
    }
}

/// Immutable placed program with semantic instructions and encoded forms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledProgram {
    origin: u32,
    bytes: Vec<u8>,
    words: Vec<u16>,
    instructions: Vec<Instruction>,
}

impl AssembledProgram {
    /// Returns the requested program origin.
    pub fn origin(&self) -> u32 {
        self.origin
    }

    /// Returns the big-endian encoded bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consumes the program and returns its bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Returns encoded instruction words in placement order.
    pub fn words(&self) -> &[u16] {
        &self.words
    }

    /// Returns semantic instructions in placement order.
    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }
}

/// Failure to resolve, place, encode, or verify an assembler program.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AssemblerError {
    /// The requested origin is not two-byte aligned.
    #[error("SH-2 assembly origin 0x{origin:08X} is not two-byte aligned")]
    UnalignedOrigin {
        /// Rejected origin address.
        origin: u32,
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
    /// The origin plus an instruction offset overflows 32 bits.
    #[error("SH-2 address overflow from 0x{origin:08X} at instruction {instruction_index}")]
    AddressOverflow {
        /// Program origin involved in the overflow.
        origin: u32,
        /// Zero-based instruction index involved in the overflow.
        instruction_index: usize,
    },
    /// A branch target is not two-byte aligned.
    #[error("branch at 0x{address:08X} has unaligned target 0x{target:08X}")]
    UnalignedBranchTarget {
        /// Address of the branch instruction.
        address: u32,
        /// Rejected target address.
        target: u32,
    },
    /// A branch target cannot be represented by the selected displacement field.
    #[error("{bits}-bit branch at 0x{address:08X} cannot reach 0x{target:08X}")]
    BranchOutOfRange {
        /// Address of the branch instruction.
        address: u32,
        /// Requested target address.
        target: u32,
        /// Width of the signed displacement field.
        bits: u8,
    },
    /// A delayed branch is the last instruction in the program.
    #[error("delayed branch at 0x{branch_address:08X} has no delay-slot instruction")]
    MissingDelaySlot {
        /// Address of the delayed branch.
        branch_address: u32,
    },
    /// An instruction forbidden by SH-2 occupies a delay slot.
    #[error(
        "instruction {instruction} at 0x{slot_address:08X} is illegal in the delay slot of 0x{branch_address:08X}"
    )]
    IllegalDelaySlot {
        /// Address of the delayed branch.
        branch_address: u32,
        /// Address of the invalid delay-slot instruction.
        slot_address: u32,
        /// Invalid delay-slot instruction.
        instruction: Instruction,
    },
    /// Encoding a semantic instruction failed.
    #[error(transparent)]
    Encode(EncodeError),
    /// Post-placement decode or byte verification failed.
    #[error(transparent)]
    Verify(VerifyError),
    /// Decoded semantics differ from the instructions supplied to the assembler.
    #[error(
        "SH-2 semantic round-trip mismatch at 0x{address:08X}: expected {expected}, got {actual}"
    )]
    SemanticRoundTripMismatch {
        /// Address of the mismatched instruction.
        address: u32,
        /// Instruction supplied to the assembler.
        expected: Instruction,
        /// Instruction decoded from assembled bytes.
        actual: Instruction,
    },
}
