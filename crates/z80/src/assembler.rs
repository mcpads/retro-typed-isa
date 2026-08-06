use std::collections::BTreeMap;

use thiserror::Error;

use crate::encoder::{EncodeError, VerifyError, encode_bytes, verify_placed_program};
use crate::instruction::{Condition, Instruction};

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

    pub fn emit_all(&mut self, instructions: impl IntoIterator<Item = Instruction>) -> &mut Self {
        self.nodes
            .extend(instructions.into_iter().map(Node::Instruction));
        self
    }

    pub fn jr(&mut self, label: impl Into<String>) -> &mut Self {
        self.relative(RelativeBranch::Jr, label)
    }

    pub fn jr_cond(&mut self, condition: Condition, label: impl Into<String>) -> &mut Self {
        self.relative(RelativeBranch::JrCond(condition), label)
    }

    pub fn djnz(&mut self, label: impl Into<String>) -> &mut Self {
        self.relative(RelativeBranch::Djnz, label)
    }

    pub fn jp(&mut self, label: impl Into<String>) -> &mut Self {
        self.absolute(AbsoluteBranch::Jp, label)
    }

    pub fn jp_cond(&mut self, condition: Condition, label: impl Into<String>) -> &mut Self {
        self.absolute(AbsoluteBranch::JpCond(condition), label)
    }

    pub fn call(&mut self, label: impl Into<String>) -> &mut Self {
        self.absolute(AbsoluteBranch::Call, label)
    }

    pub fn call_cond(&mut self, condition: Condition, label: impl Into<String>) -> &mut Self {
        self.absolute(AbsoluteBranch::CallCond(condition), label)
    }

    fn relative(&mut self, branch: RelativeBranch, label: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::Relative {
            branch,
            label: label.into(),
        });
        self
    }

    fn absolute(&mut self, branch: AbsoluteBranch, label: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::Absolute {
            branch,
            label: label.into(),
        });
        self
    }

    pub fn assemble(&self, origin: u16) -> Result<AssembledProgram, AssemblerError> {
        let (labels, length) = self.collect_labels(origin)?;
        let mut instructions = Vec::new();
        let mut bytes = Vec::with_capacity(length);
        let mut offset = 0usize;

        for node in &self.nodes {
            let instruction = match node {
                Node::Label(_) => continue,
                Node::Instruction(instruction) => *instruction,
                Node::Relative { branch, label } => {
                    let target = label_address(&labels, label)?;
                    branch.resolve(origin, offset, target)?
                }
                Node::Absolute { branch, label } => branch.resolve(label_address(&labels, label)?),
            };
            let encoded = encode_bytes(&instruction).map_err(AssemblerError::Encode)?;
            offset = offset
                .checked_add(encoded.len())
                .ok_or(AssemblerError::AddressOverflow { origin, length })?;
            bytes.extend_from_slice(&encoded);
            instructions.push(instruction);
        }

        let decoded = verify_placed_program(&bytes, origin).map_err(AssemblerError::Verify)?;
        if decoded != instructions {
            return Err(AssemblerError::SemanticRoundTripMismatch);
        }
        Ok(AssembledProgram {
            origin,
            bytes,
            instructions,
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
            let length = match node {
                Node::Instruction(instruction) => encode_bytes(instruction)
                    .map_err(AssemblerError::Encode)?
                    .len(),
                Node::Relative { .. } => 2,
                Node::Absolute { .. } => 3,
                Node::Label(_) => unreachable!(),
            };
            offset = offset
                .checked_add(length)
                .ok_or(AssemblerError::AddressOverflow {
                    origin,
                    length: usize::MAX,
                })?;
            if usize::from(origin) + offset > 0x1_0000 {
                return Err(AssemblerError::AddressOverflow {
                    origin,
                    length: offset,
                });
            }
        }
        Ok((labels, offset))
    }
}

fn placed_address(origin: u16, offset: usize) -> Result<u16, AssemblerError> {
    let address = usize::from(origin) + offset;
    u16::try_from(address).map_err(|_| AssemblerError::AddressOverflow {
        origin,
        length: offset,
    })
}

fn label_address(labels: &BTreeMap<String, u16>, label: &str) -> Result<u16, AssemblerError> {
    labels
        .get(label)
        .copied()
        .ok_or_else(|| AssemblerError::UndefinedLabel {
            label: label.to_owned(),
        })
}

#[derive(Debug, Clone)]
enum Node {
    Label(String),
    Instruction(Instruction),
    Relative {
        branch: RelativeBranch,
        label: String,
    },
    Absolute {
        branch: AbsoluteBranch,
        label: String,
    },
}

#[derive(Debug, Clone, Copy)]
enum RelativeBranch {
    Jr,
    JrCond(Condition),
    Djnz,
}

impl RelativeBranch {
    fn resolve(
        self,
        origin: u16,
        offset: usize,
        target: u16,
    ) -> Result<Instruction, AssemblerError> {
        let address = placed_address(origin, offset)?;
        let next = usize::from(address) + 2;
        let displacement = i32::from(target) - i32::try_from(next).expect("Z80 address plus two");
        let displacement = i8::try_from(displacement)
            .map_err(|_| AssemblerError::RelativeBranchOutOfRange { address, target })?;
        Ok(match self {
            Self::Jr => Instruction::Jr(displacement),
            Self::JrCond(condition) => Instruction::JrCond(condition, displacement),
            Self::Djnz => Instruction::Djnz(displacement),
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum AbsoluteBranch {
    Jp,
    JpCond(Condition),
    Call,
    CallCond(Condition),
}

impl AbsoluteBranch {
    fn resolve(self, target: u16) -> Instruction {
        match self {
            Self::Jp => Instruction::Jp(target),
            Self::JpCond(condition) => Instruction::JpCond(condition, target),
            Self::Call => Instruction::Call(target),
            Self::CallCond(condition) => Instruction::CallCond(condition, target),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledProgram {
    origin: u16,
    bytes: Vec<u8>,
    instructions: Vec<Instruction>,
}

impl AssembledProgram {
    pub fn origin(&self) -> u16 {
        self.origin
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AssemblerError {
    #[error("duplicate label `{label}`")]
    DuplicateLabel { label: String },
    #[error("undefined label `{label}`")]
    UndefinedLabel { label: String },
    #[error("Z80 program from 0x{origin:04X} with {length} bytes exceeds its address space")]
    AddressOverflow { origin: u16, length: usize },
    #[error("relative branch at 0x{address:04X} cannot reach 0x{target:04X}")]
    RelativeBranchOutOfRange { address: u16, target: u16 },
    #[error(transparent)]
    Encode(EncodeError),
    #[error(transparent)]
    Verify(VerifyError),
    #[error("Z80 semantic round-trip mismatch")]
    SemanticRoundTripMismatch,
}
