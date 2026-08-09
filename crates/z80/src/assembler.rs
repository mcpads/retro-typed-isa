use std::collections::BTreeMap;

use thiserror::Error;
use typed_isa_core::InstructionBytes;

use crate::encoder::{EncodeError, VerifyError, encode_bytes, verify_placed_program};
use crate::instruction::{Condition, Instruction};

/// Two-pass builder for typed Z80 code and checked label references.
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
    pub fn jr(&mut self, label: impl Into<String>) -> &mut Self {
        self.relative(RelativeBranch::Jr, label)
    }

    /// Append a conditional relative jump to a named label.
    pub fn jr_cond(&mut self, condition: Condition, label: impl Into<String>) -> &mut Self {
        self.relative(RelativeBranch::JrCond(condition), label)
    }

    /// Append a decrement-B-and-relative-jump instruction to a named label.
    pub fn djnz(&mut self, label: impl Into<String>) -> &mut Self {
        self.relative(RelativeBranch::Djnz, label)
    }

    /// Append an unconditional absolute jump to a named label.
    pub fn jp(&mut self, label: impl Into<String>) -> &mut Self {
        self.absolute(AbsoluteBranch::Jp, label)
    }

    /// Append a conditional absolute jump to a named label.
    pub fn jp_cond(&mut self, condition: Condition, label: impl Into<String>) -> &mut Self {
        self.absolute(AbsoluteBranch::JpCond(condition), label)
    }

    /// Append an unconditional call to a named label.
    pub fn call(&mut self, label: impl Into<String>) -> &mut Self {
        self.absolute(AbsoluteBranch::Call, label)
    }

    /// Append a conditional call to a named label.
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

    /// Resolve labels, lower typed instructions, and verify the complete program.
    ///
    /// The output must fit between `origin` and the end of the 16-bit address space.
    pub fn assemble(
        &self,
        origin: crate::CodeLocation,
    ) -> Result<AssembledProgram, AssemblerError> {
        let (labels, length) = self.collect_labels(origin)?;
        let mut instructions = Vec::new();
        let mut instruction_spans = Vec::new();
        let mut bytes = Vec::with_capacity(length);
        let mut offset = 0usize;

        for node in &self.nodes {
            let instruction = match node {
                Node::Label(_) => continue,
                Node::Instruction(instruction) => *instruction,
                Node::Relative { branch, label } => {
                    let target = label_location(&labels, label)?;
                    branch.resolve(origin, offset, target)?
                }
                Node::Absolute { branch, label } => branch.resolve(label_location(&labels, label)?),
            };
            let encoded = encode_bytes(&instruction).map_err(AssemblerError::Encode)?;
            let location = placed_address(origin, offset)?;
            instruction_spans.push(InstructionSpan {
                offset,
                location,
                bytes: InstructionBytes::new(encoded.clone())
                    .expect("encoded Z80 instructions are non-empty"),
                instruction,
            });
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

fn label_location(labels: &BTreeMap<String, u16>, label: &str) -> Result<u16, AssemblerError> {
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

/// Checked assembled bytes and their sequential typed instruction identities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledProgram {
    origin: u16,
    bytes: Vec<u8>,
    instructions: Vec<Instruction>,
    instruction_spans: Vec<InstructionSpan>,
}

/// Workspace-wide instruction-placement evidence specialized for Z80.
pub type InstructionSpan = typed_isa_core::InstructionSpan<crate::CodeLocation, Instruction>;

impl AssembledProgram {
    /// Return the address assigned to the first output byte.
    pub fn origin(&self) -> crate::CodeLocation {
        self.origin
    }

    /// Borrow the complete assembled byte sequence.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consume the program and return its complete byte sequence.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Borrow the sequential typed instructions used to produce the bytes.
    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }

    /// Borrow exact bytes, locations, and typed identities for each instruction.
    pub fn instruction_spans(&self) -> &[InstructionSpan] {
        &self.instruction_spans
    }
}

/// A fail-closed reason that Z80 assembly did not produce a program.
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
    /// Origin plus output length exceeds the 16-bit address space.
    #[error("Z80 program from 0x{origin:04X} with {length} bytes exceeds its address space")]
    AddressOverflow {
        /// Address assigned to the first output byte.
        origin: u16,
        /// Complete or intermediate output length that does not fit.
        length: usize,
    },
    /// A relative target lies outside the signed 8-bit displacement range.
    #[error("relative branch at 0x{address:04X} cannot reach 0x{target:04X}")]
    RelativeBranchOutOfRange {
        /// Address of the relative branch instruction.
        address: u16,
        /// Resolved target address.
        target: u16,
    },
    /// Typed instruction lowering failed.
    #[error(transparent)]
    Encode(EncodeError),
    /// Sequential decode/re-encode verification failed.
    #[error(transparent)]
    Verify(VerifyError),
    /// Verified bytes decoded to different semantic instruction identities.
    #[error("Z80 semantic round-trip mismatch")]
    SemanticRoundTripMismatch,
}
