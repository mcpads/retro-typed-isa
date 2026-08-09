use std::collections::BTreeMap;

use thiserror::Error;
use typed_isa_core::InstructionBytes;

use crate::codec::{EncodeError, VerifyError, encode_word, verify_placed_program};
use crate::instruction::{Instruction, Register};

/// Label-resolving assembler for typed PlayStation R3000A instructions.
///
/// Assembly validates branch reach, jump regions, delay slots, and semantic
/// decode/encode round trips after final placement.
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

    /// Appends `beq rs, rt, label`.
    pub fn beq(&mut self, rs: Register, rt: Register, label: impl Into<String>) -> &mut Self {
        self.branch(LabelBranch::Beq { rs, rt }, label)
    }

    /// Appends `bne rs, rt, label`.
    pub fn bne(&mut self, rs: Register, rt: Register, label: impl Into<String>) -> &mut Self {
        self.branch(LabelBranch::Bne { rs, rt }, label)
    }

    /// Appends `blez rs, label`.
    pub fn blez(&mut self, rs: Register, label: impl Into<String>) -> &mut Self {
        self.branch(LabelBranch::Blez { rs }, label)
    }

    /// Appends `bgtz rs, label`.
    pub fn bgtz(&mut self, rs: Register, label: impl Into<String>) -> &mut Self {
        self.branch(LabelBranch::Bgtz { rs }, label)
    }

    /// Appends `bltz rs, label`.
    pub fn bltz(&mut self, rs: Register, label: impl Into<String>) -> &mut Self {
        self.branch(LabelBranch::Bltz { rs }, label)
    }

    /// Appends `bgez rs, label`.
    pub fn bgez(&mut self, rs: Register, label: impl Into<String>) -> &mut Self {
        self.branch(LabelBranch::Bgez { rs }, label)
    }

    /// Appends `bltzal rs, label`.
    pub fn bltzal(&mut self, rs: Register, label: impl Into<String>) -> &mut Self {
        self.branch(LabelBranch::Bltzal { rs }, label)
    }

    /// Appends `bgezal rs, label`.
    pub fn bgezal(&mut self, rs: Register, label: impl Into<String>) -> &mut Self {
        self.branch(LabelBranch::Bgezal { rs }, label)
    }

    /// Appends an unconditional direct jump to `label`.
    pub fn jump(&mut self, label: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::Jump {
            link: false,
            label: label.into(),
        });
        self
    }

    /// Appends a direct jump-and-link call to `label`.
    pub fn call(&mut self, label: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::Jump {
            link: true,
            label: label.into(),
        });
        self
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
    /// Every control-flow instruction must have a non-control-flow delay slot.
    pub fn assemble(
        &self,
        origin: crate::CodeLocation,
    ) -> Result<AssembledProgram, AssemblerError> {
        if origin & 3 != 0 {
            return Err(AssemblerError::UnalignedOrigin { origin });
        }
        let labels = self.collect_labels(origin)?;
        let mut instructions = Vec::new();
        for node in &self.nodes {
            let instruction = match node {
                Node::Label(_) => continue,
                Node::Instruction(instruction) => instruction.clone(),
                Node::Branch { branch, label } => {
                    let target =
                        *labels
                            .get(label)
                            .ok_or_else(|| AssemblerError::UndefinedLabel {
                                label: label.clone(),
                            })?;
                    branch.resolve(target)
                }
                Node::Jump { link, label } => {
                    let target =
                        *labels
                            .get(label)
                            .ok_or_else(|| AssemblerError::UndefinedLabel {
                                label: label.clone(),
                            })?;
                    if *link {
                        Instruction::Jal { target }
                    } else {
                        Instruction::J { target }
                    }
                }
            };
            instructions.push(instruction);
        }

        for (index, instruction) in instructions.iter().enumerate() {
            if !instruction.has_delay_slot() {
                continue;
            }
            let address = self.address_for(origin, index)?;
            let Some(delay_slot) = instructions.get(index + 1) else {
                return Err(AssemblerError::MissingDelaySlot { address });
            };
            if delay_slot.has_delay_slot() {
                return Err(AssemblerError::ControlFlowInDelaySlot {
                    branch_address: address,
                    slot_address: self.address_for(origin, index + 1)?,
                });
            }
        }

        let mut words = Vec::with_capacity(instructions.len());
        let mut bytes = Vec::with_capacity(instructions.len() * 4);
        let mut instruction_spans = Vec::with_capacity(instructions.len());
        for (index, instruction) in instructions.iter().enumerate() {
            let location = self.address_for(origin, index)?;
            let word = encode_word(instruction, location).map_err(AssemblerError::Encode)?;
            let encoded = word.to_le_bytes();
            words.push(word);
            instruction_spans.push(InstructionSpan {
                offset: bytes.len(),
                location,
                bytes: InstructionBytes::new(encoded.to_vec())
                    .expect("encoded R3000A instructions are non-empty"),
                instruction: instruction.clone(),
            });
            bytes.extend_from_slice(&encoded);
        }

        let decoded = verify_placed_program(&bytes, origin).map_err(AssemblerError::Verify)?;
        for (index, (expected, actual)) in instructions.iter().zip(&decoded).enumerate() {
            if expected != actual {
                return Err(AssemblerError::SemanticRoundTripMismatch {
                    address: self.address_for(origin, index)?,
                    expected: expected.clone(),
                    actual: actual.clone(),
                });
            }
        }

        Ok(AssembledProgram {
            origin,
            bytes,
            words,
            instructions,
            instruction_spans,
        })
    }

    fn collect_labels(&self, origin: u32) -> Result<BTreeMap<String, u32>, AssemblerError> {
        let mut labels = BTreeMap::new();
        let mut instruction_index = 0usize;
        for node in &self.nodes {
            if let Node::Label(label) = node {
                let address = self.address_for(origin, instruction_index)?;
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
        self.address_for(origin, instruction_index)?;
        Ok(labels)
    }

    fn address_for(&self, origin: u32, instruction_index: usize) -> Result<u32, AssemblerError> {
        let offset = u32::try_from(instruction_index)
            .ok()
            .and_then(|index| index.checked_mul(4))
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
}

#[derive(Debug, Clone)]
enum Node {
    Label(String),
    Instruction(Instruction),
    Branch { branch: LabelBranch, label: String },
    Jump { link: bool, label: String },
}

#[derive(Debug, Clone, Copy)]
enum LabelBranch {
    Beq { rs: Register, rt: Register },
    Bne { rs: Register, rt: Register },
    Blez { rs: Register },
    Bgtz { rs: Register },
    Bltz { rs: Register },
    Bgez { rs: Register },
    Bltzal { rs: Register },
    Bgezal { rs: Register },
}

impl LabelBranch {
    fn resolve(self, target: u32) -> Instruction {
        match self {
            Self::Beq { rs, rt } => Instruction::Beq { rs, rt, target },
            Self::Bne { rs, rt } => Instruction::Bne { rs, rt, target },
            Self::Blez { rs } => Instruction::Blez { rs, target },
            Self::Bgtz { rs } => Instruction::Bgtz { rs, target },
            Self::Bltz { rs } => Instruction::Bltz { rs, target },
            Self::Bgez { rs } => Instruction::Bgez { rs, target },
            Self::Bltzal { rs } => Instruction::Bltzal { rs, target },
            Self::Bgezal { rs } => Instruction::Bgezal { rs, target },
        }
    }
}

/// Immutable placed program with semantic instructions and encoded forms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledProgram {
    origin: u32,
    bytes: Vec<u8>,
    words: Vec<u32>,
    instructions: Vec<Instruction>,
    instruction_spans: Vec<InstructionSpan>,
}

/// Workspace-wide instruction-placement evidence specialized for R3000A.
pub type InstructionSpan = typed_isa_core::InstructionSpan<crate::CodeLocation, Instruction>;

impl AssembledProgram {
    /// Returns the requested program origin.
    pub fn origin(&self) -> crate::CodeLocation {
        self.origin
    }

    /// Returns the little-endian encoded bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consumes the program and returns its bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Returns encoded instruction words in placement order.
    pub fn words(&self) -> &[u32] {
        &self.words
    }

    /// Returns semantic instructions in placement order.
    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }

    /// Returns the exact bytes, location, and typed identity of each instruction.
    pub fn instruction_spans(&self) -> &[InstructionSpan] {
        &self.instruction_spans
    }
}

/// Canonical `lui` + signed `addiu` sequence for a 32-bit address.
pub fn load_address(register: Register, address: u32) -> [Instruction; 2] {
    let upper = address.wrapping_add(0x8000) >> 16;
    let lower = address as u16 as i16;
    [
        Instruction::Lui {
            rt: register,
            immediate: upper as u16,
        },
        Instruction::Addiu {
            rt: register,
            rs: register,
            immediate: lower,
        },
    ]
}

/// Failure to resolve, place, encode, or verify an assembler program.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AssemblerError {
    /// The requested origin is not four-byte aligned.
    #[error("assembly origin {origin:#010x} is not word-aligned")]
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
    /// A control-flow node refers to an undeclared label.
    #[error("undefined label `{label}`")]
    UndefinedLabel {
        /// Missing label name.
        label: String,
    },
    /// The origin plus an instruction offset overflows 32 bits.
    #[error(
        "assembly address overflows for origin {origin:#010x}, instruction {instruction_index}"
    )]
    AddressOverflow {
        /// Program origin involved in the overflow.
        origin: u32,
        /// Zero-based instruction index involved in the overflow.
        instruction_index: usize,
    },
    /// A control-flow instruction is the last instruction in the program.
    #[error("control-flow instruction at {address:#010x} has no delay slot")]
    MissingDelaySlot {
        /// Address of the control-flow instruction.
        address: u32,
    },
    /// A delay slot contains another control-flow instruction.
    #[error(
        "control-flow instruction at {branch_address:#010x} has another control-flow instruction in delay slot {slot_address:#010x}"
    )]
    ControlFlowInDelaySlot {
        /// Address of the first control-flow instruction.
        branch_address: u32,
        /// Address of the invalid delay-slot instruction.
        slot_address: u32,
    },
    /// An instruction failed final placement or encoding.
    #[error("instruction encoding failed: {0}")]
    Encode(EncodeError),
    /// Post-placement decoding or byte round-trip verification failed.
    #[error("post-placement verification failed: {0}")]
    Verify(VerifyError),
    /// Decoded semantics differ from the instructions supplied to the assembler.
    #[error(
        "semantic roundtrip mismatch at {address:#010x}: expected {expected:?}, actual {actual:?}"
    )]
    SemanticRoundTripMismatch {
        /// Address of the mismatched instruction.
        address: u32,
        /// Instruction supplied to the assembler.
        expected: Instruction,
        /// Instruction decoded from the assembled word.
        actual: Instruction,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_labels_and_signed_load_address() {
        let origin = 0x8004_0000;
        let mut assembler = Assembler::new();
        assembler.emit_all(load_address(Register::T0, 0x8004_9694));
        assembler.label("loop");
        assembler.emit(Instruction::Addiu {
            rt: Register::T0,
            rs: Register::T0,
            immediate: 1,
        });
        assembler.bne(Register::T0, Register::T1, "loop");
        assembler.emit(Instruction::nop());
        let program = assembler.assemble(origin).unwrap();
        assert_eq!(program.words()[0], 0x3c08_8005);
        assert_eq!(program.words()[1], 0x2508_9694);
        assert_eq!(program.words()[3] & 0xffff, 0xfffe);
    }

    #[test]
    fn rejects_missing_or_nested_delay_slots() {
        let origin = 0x8001_0000;
        let mut missing = Assembler::new();
        missing.emit(Instruction::Jr { rs: Register::RA });
        assert!(matches!(
            missing.assemble(origin),
            Err(AssemblerError::MissingDelaySlot { .. })
        ));

        let mut nested = Assembler::new();
        nested.emit(Instruction::Jr { rs: Register::RA });
        nested.emit(Instruction::Jr { rs: Register::RA });
        nested.emit(Instruction::nop());
        assert!(matches!(
            nested.assemble(origin),
            Err(AssemblerError::ControlFlowInDelaySlot { .. })
        ));
    }

    #[test]
    fn rejects_unaligned_assembly_origin() {
        assert!(matches!(
            Assembler::new().assemble(0x8001_0002),
            Err(AssemblerError::UnalignedOrigin {
                origin: 0x8001_0002
            })
        ));
    }
}
