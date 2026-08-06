//! Two-pass HuC6280 assembly with typed code/data separation.

use std::collections::BTreeMap;

use thiserror::Error;

use crate::codec::{DecodeError, EncodeError, Instruction, Operand, decode_bytes, encode_bytes};
use crate::instruction::{AddressingMode, InstructionForm, Mnemonic, canonical_opcode};

#[derive(Debug, Clone)]
enum Statement {
    Label(String),
    Instruction(Instruction),
    LabelReference {
        mnemonic: Mnemonic,
        mode: AddressingMode,
        label: String,
        addend: i32,
    },
    ZeroPageBranch {
        mnemonic: Mnemonic,
        address: u8,
        label: String,
    },
    BlockTransferFromLabel {
        mnemonic: Mnemonic,
        source: String,
        destination: u16,
        length: u16,
    },
    Data(Vec<u8>),
    LabelWord(String),
}

#[derive(Debug, Clone, Default)]
pub struct Assembler {
    statements: Vec<Statement>,
}

impl Assembler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn label(&mut self, name: impl Into<String>) -> &mut Self {
        self.statements.push(Statement::Label(name.into()));
        self
    }

    pub fn emit(&mut self, instruction: Instruction) -> &mut Self {
        self.statements.push(Statement::Instruction(instruction));
        self
    }

    pub fn emit_all(&mut self, instructions: impl IntoIterator<Item = Instruction>) -> &mut Self {
        self.statements
            .extend(instructions.into_iter().map(Statement::Instruction));
        self
    }

    /// Emit an instruction whose relative or 16-bit address is resolved from
    /// a label during placement.
    pub fn emit_label_ref(
        &mut self,
        mnemonic: Mnemonic,
        mode: AddressingMode,
        label: impl Into<String>,
    ) -> &mut Self {
        self.emit_label_ref_offset(mnemonic, mode, label, 0)
    }

    /// Emit a label reference with a checked signed byte-address addend.
    pub fn emit_label_ref_offset(
        &mut self,
        mnemonic: Mnemonic,
        mode: AddressingMode,
        label: impl Into<String>,
        addend: i32,
    ) -> &mut Self {
        self.statements.push(Statement::LabelReference {
            mnemonic,
            mode,
            label: label.into(),
            addend,
        });
        self
    }

    /// Emit BBRn/BBSn with a concrete zero-page address and a label target.
    pub fn emit_zero_page_branch(
        &mut self,
        mnemonic: Mnemonic,
        address: u8,
        label: impl Into<String>,
    ) -> &mut Self {
        self.statements.push(Statement::ZeroPageBranch {
            mnemonic,
            address,
            label: label.into(),
        });
        self
    }

    /// Emit a block-transfer instruction whose source address is a label.
    pub fn emit_block_transfer_from_label(
        &mut self,
        mnemonic: Mnemonic,
        source: impl Into<String>,
        destination: u16,
        length: u16,
    ) -> &mut Self {
        self.statements.push(Statement::BlockTransferFromLabel {
            mnemonic,
            source: source.into(),
            destination,
            length,
        });
        self
    }

    /// Emit declared inline data. This is not an instruction fallback.
    pub fn data(&mut self, bytes: impl Into<Vec<u8>>) -> &mut Self {
        self.statements.push(Statement::Data(bytes.into()));
        self
    }

    /// Emit a literal little-endian data word.
    pub fn word(&mut self, value: u16) -> &mut Self {
        self.data(value.to_le_bytes().to_vec())
    }

    /// Emit a label address as a little-endian data word.
    pub fn label_word(&mut self, label: impl Into<String>) -> &mut Self {
        self.statements.push(Statement::LabelWord(label.into()));
        self
    }

    pub fn assemble(&self, origin: u16) -> Result<AssembledProgram, AssemblerError> {
        let (labels, byte_len) = self.collect_labels(origin)?;
        ensure_address_span(origin, byte_len)?;

        let mut bytes = Vec::with_capacity(byte_len);
        let mut instructions = Vec::new();
        for statement in &self.statements {
            match statement {
                Statement::Label(_) => {}
                Statement::Data(data) => bytes.extend_from_slice(data),
                Statement::LabelWord(label) => {
                    let address = label_address(&labels, label)?;
                    bytes.extend_from_slice(&address.to_le_bytes());
                }
                Statement::Instruction(instruction) => {
                    emit_instruction(&mut bytes, &mut instructions, *instruction)?;
                }
                Statement::LabelReference {
                    mnemonic,
                    mode,
                    label,
                    addend,
                } => {
                    ensure_label_mode(*mode)?;
                    let target =
                        checked_label_addend(label_address(&labels, label)?, *addend, label)?;
                    let pc = address_for_offset(origin, bytes.len())?;
                    let operand = if *mode == AddressingMode::Relative {
                        Operand::Relative(relative_displacement(
                            pc,
                            InstructionForm::new(*mnemonic, *mode).encoded_len(),
                            target,
                            label,
                        )?)
                    } else {
                        Operand::Word(target)
                    };
                    let instruction = Instruction::new(*mnemonic, *mode, operand)?;
                    emit_instruction(&mut bytes, &mut instructions, instruction)?;
                }
                Statement::ZeroPageBranch {
                    mnemonic,
                    address,
                    label,
                } => {
                    let target = label_address(&labels, label)?;
                    let pc = address_for_offset(origin, bytes.len())?;
                    let displacement = relative_displacement(pc, 3, target, label)?;
                    let instruction = Instruction::new(
                        *mnemonic,
                        AddressingMode::ZeroPageRelative,
                        Operand::ZeroPageRelative {
                            address: *address,
                            displacement,
                        },
                    )?;
                    emit_instruction(&mut bytes, &mut instructions, instruction)?;
                }
                Statement::BlockTransferFromLabel {
                    mnemonic,
                    source,
                    destination,
                    length,
                } => {
                    let source = label_address(&labels, source)?;
                    let instruction = Instruction::new(
                        *mnemonic,
                        AddressingMode::BlockTransfer,
                        Operand::BlockTransfer {
                            source,
                            destination: *destination,
                            length: *length,
                        },
                    )?;
                    emit_instruction(&mut bytes, &mut instructions, instruction)?;
                }
            }
        }

        debug_assert_eq!(bytes.len(), byte_len);
        verify_instruction_spans(&bytes, &instructions)?;
        Ok(AssembledProgram {
            origin,
            bytes,
            labels,
            instructions,
        })
    }

    fn collect_labels(
        &self,
        origin: u16,
    ) -> Result<(BTreeMap<String, u16>, usize), AssemblerError> {
        let mut labels = BTreeMap::new();
        let mut offset = 0usize;
        for statement in &self.statements {
            match statement {
                Statement::Label(label) => {
                    let address = address_for_offset(origin, offset)?;
                    if labels.insert(label.clone(), address).is_some() {
                        return Err(AssemblerError::DuplicateLabel {
                            label: label.clone(),
                        });
                    }
                }
                _ => {
                    offset = offset.checked_add(statement_len(statement)?).ok_or(
                        AssemblerError::AddressOverflow {
                            origin,
                            byte_len: usize::MAX,
                        },
                    )?;
                    ensure_address_span(origin, offset)?;
                }
            }
        }
        Ok((labels, offset))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstructionSpan {
    pub offset: usize,
    pub len: usize,
    pub instruction: Instruction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledProgram {
    origin: u16,
    bytes: Vec<u8>,
    labels: BTreeMap<String, u16>,
    instructions: Vec<InstructionSpan>,
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

    pub fn label_address(&self, name: &str) -> Option<u16> {
        self.labels.get(name).copied()
    }

    pub fn instruction_spans(&self) -> &[InstructionSpan] {
        &self.instructions
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AssemblerError {
    #[error("duplicate label `{label}`")]
    DuplicateLabel { label: String },
    #[error("undefined label `{label}`")]
    UndefinedLabel { label: String },
    #[error("label `{label}` plus addend {addend} is outside the 16-bit address space")]
    LabelAddendOutOfRange { label: String, addend: i32 },
    #[error("label references are not supported for {mode:?}")]
    UnsupportedLabelMode { mode: AddressingMode },
    #[error("relative branch to `{label}` has out-of-range displacement {displacement}")]
    RelativeOutOfRange { label: String, displacement: i32 },
    #[error("assembly at {origin:#06x} with {byte_len} byte(s) exceeds the 16-bit address space")]
    AddressOverflow { origin: u16, byte_len: usize },
    #[error("instruction encoding failed: {0}")]
    Encode(#[from] EncodeError),
    #[error("post-placement decode failed: {0}")]
    Decode(#[from] DecodeError),
    #[error("assembled instruction differs after byte round-trip at offset {offset:#x}")]
    VerificationFailed { offset: usize },
}

fn statement_len(statement: &Statement) -> Result<usize, AssemblerError> {
    match statement {
        Statement::Label(_) => Ok(0),
        Statement::Instruction(instruction) => Ok(instruction.encoded_len()),
        Statement::Data(data) => Ok(data.len()),
        Statement::LabelWord(_) => Ok(2),
        Statement::LabelReference { mnemonic, mode, .. } => {
            ensure_label_mode(*mode)?;
            ensure_form(*mnemonic, *mode)
        }
        Statement::ZeroPageBranch { mnemonic, .. } => {
            ensure_form(*mnemonic, AddressingMode::ZeroPageRelative)
        }
        Statement::BlockTransferFromLabel { mnemonic, .. } => {
            ensure_form(*mnemonic, AddressingMode::BlockTransfer)
        }
    }
}

fn ensure_form(mnemonic: Mnemonic, mode: AddressingMode) -> Result<usize, AssemblerError> {
    let form = InstructionForm::new(mnemonic, mode);
    canonical_opcode(form)
        .map(|_| form.encoded_len())
        .ok_or(AssemblerError::Encode(EncodeError::UnknownEncoding {
            mnemonic,
            mode,
        }))
}

fn ensure_label_mode(mode: AddressingMode) -> Result<(), AssemblerError> {
    if matches!(
        mode,
        AddressingMode::Absolute
            | AddressingMode::AbsoluteX
            | AddressingMode::AbsoluteY
            | AddressingMode::AbsoluteIndirect
            | AddressingMode::AbsoluteIndexedIndirectX
            | AddressingMode::Relative
    ) {
        Ok(())
    } else {
        Err(AssemblerError::UnsupportedLabelMode { mode })
    }
}

fn emit_instruction(
    bytes: &mut Vec<u8>,
    instructions: &mut Vec<InstructionSpan>,
    instruction: Instruction,
) -> Result<(), AssemblerError> {
    let encoded = encode_bytes(&instruction)?;
    let offset = bytes.len();
    let len = encoded.len();
    bytes.extend_from_slice(&encoded);
    instructions.push(InstructionSpan {
        offset,
        len,
        instruction,
    });
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

fn checked_label_addend(address: u16, addend: i32, label: &str) -> Result<u16, AssemblerError> {
    let value = i64::from(address) + i64::from(addend);
    u16::try_from(value).map_err(|_| AssemblerError::LabelAddendOutOfRange {
        label: label.to_owned(),
        addend,
    })
}

fn relative_displacement(
    pc: u16,
    instruction_len: usize,
    target: u16,
    label: &str,
) -> Result<i8, AssemblerError> {
    let next = i32::from(pc) + instruction_len as i32;
    let displacement = i32::from(target) - next;
    i8::try_from(displacement).map_err(|_| AssemblerError::RelativeOutOfRange {
        label: label.to_owned(),
        displacement,
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

fn address_for_offset(origin: u16, offset: usize) -> Result<u16, AssemblerError> {
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

fn verify_instruction_spans(
    bytes: &[u8],
    instructions: &[InstructionSpan],
) -> Result<(), AssemblerError> {
    for span in instructions {
        let encoded = bytes.get(span.offset..span.offset + span.len).ok_or(
            AssemblerError::VerificationFailed {
                offset: span.offset,
            },
        )?;
        let decoded = decode_bytes(encoded)?;
        if decoded != span.instruction || encode_bytes(&decoded)? != encoded {
            return Err(AssemblerError::VerificationFailed {
                offset: span.offset,
            });
        }
    }
    Ok(())
}
