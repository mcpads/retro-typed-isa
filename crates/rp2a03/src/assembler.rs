//! Two-pass RP2A03 assembly with typed code/data separation.

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
    Data(Vec<u8>),
    LabelWord(String),
}

/// Two-pass builder for typed RP2A03 instructions, labels, and explicit data.
#[derive(Debug, Clone, Default)]
pub struct Assembler {
    statements: Vec<Statement>,
}

impl Assembler {
    /// Construct an empty assembly program.
    pub fn new() -> Self {
        Self::default()
    }

    /// Define a label at the current byte position.
    pub fn label(&mut self, name: impl Into<String>) -> &mut Self {
        self.statements.push(Statement::Label(name.into()));
        self
    }

    /// Append one fully typed instruction.
    pub fn emit(&mut self, instruction: Instruction) -> &mut Self {
        self.statements.push(Statement::Instruction(instruction));
        self
    }

    /// Append a sequence of fully typed instructions in iteration order.
    pub fn emit_all(&mut self, instructions: impl IntoIterator<Item = Instruction>) -> &mut Self {
        self.statements
            .extend(instructions.into_iter().map(Statement::Instruction));
        self
    }

    /// Emit a relative or 16-bit-address form resolved from a label.
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

    /// Emit explicitly declared inline data, never executable fallback bytes.
    pub fn data(&mut self, bytes: impl Into<Vec<u8>>) -> &mut Self {
        self.statements.push(Statement::Data(bytes.into()));
        self
    }

    /// Append one literal little-endian word as explicitly declared data.
    pub fn word(&mut self, value: u16) -> &mut Self {
        self.data(value.to_le_bytes().to_vec())
    }

    /// Append the resolved little-endian address of a named label as data.
    pub fn label_word(&mut self, label: impl Into<String>) -> &mut Self {
        self.statements.push(Statement::LabelWord(label.into()));
        self
    }

    /// Resolve labels, lower typed code, and verify every instruction span.
    ///
    /// The complete output must fit between `origin` and the end of the 16-bit
    /// address space. Explicit data remains outside the instruction-span list.
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
                    bytes.extend_from_slice(&label_address(&labels, label)?.to_le_bytes());
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
                    let operand = if *mode == AddressingMode::Relative {
                        let pc = address_for_offset(origin, bytes.len())?;
                        Operand::Relative(relative_displacement(pc, target, label)?)
                    } else {
                        Operand::Word(target)
                    };
                    emit_instruction(
                        &mut bytes,
                        &mut instructions,
                        Instruction::new(*mnemonic, *mode, operand)?,
                    )?;
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
    instructions: Vec<InstructionSpan>,
}

impl AssembledProgram {
    /// Return the 16-bit address assigned to the first output byte.
    pub fn origin(&self) -> u16 {
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
        &self.instructions
    }
}

/// A fail-closed reason that RP2A03 assembly did not produce a program.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
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
    /// Applying an addend to a label leaves the 16-bit address space.
    #[error("label `{label}` plus addend {addend} is outside the 16-bit address space")]
    LabelAddendOutOfRange {
        /// Label whose resolved address was adjusted.
        label: String,
        /// Signed byte-address adjustment that could not be represented.
        addend: i32,
    },
    /// A label was requested for a form without a relative or 16-bit address operand.
    #[error("label references are not supported for {mode:?}")]
    UnsupportedLabelMode {
        /// Addressing mode that cannot carry a label address.
        mode: AddressingMode,
    },
    /// A relative branch target is outside the signed 8-bit displacement range.
    #[error("relative branch to `{label}` has out-of-range displacement {displacement}")]
    RelativeOutOfRange {
        /// Label targeted by the branch.
        label: String,
        /// Required displacement from the post-instruction address.
        displacement: i32,
    },
    /// The complete output would extend beyond the 16-bit address space.
    #[error("assembly at {origin:#06x} with {byte_len} byte(s) exceeds the 16-bit address space")]
    AddressOverflow {
        /// Address assigned to the first output byte.
        origin: u16,
        /// Complete or intermediate output length that does not fit.
        byte_len: usize,
    },
    /// Typed instruction construction or lowering failed.
    #[error("instruction encoding failed: {0}")]
    Encode(#[from] EncodeError),
    /// Post-placement decoding of an instruction span failed.
    #[error("post-placement decode failed: {0}")]
    Decode(#[from] DecodeError),
    /// An emitted instruction changed identity during byte round-trip verification.
    #[error("assembled instruction differs after byte round-trip at offset {offset:#x}")]
    VerificationFailed {
        /// Byte offset of the inconsistent instruction span.
        offset: usize,
    },
}

fn statement_len(statement: &Statement) -> Result<usize, AssemblerError> {
    match statement {
        Statement::Label(_) => Ok(0),
        Statement::Instruction(instruction) => Ok(instruction.encoded_len()),
        Statement::Data(data) => Ok(data.len()),
        Statement::LabelWord(_) => Ok(2),
        Statement::LabelReference { mnemonic, mode, .. } => {
            ensure_label_mode(*mode)?;
            let form = InstructionForm::new(*mnemonic, *mode);
            canonical_opcode(form)
                .map(|_| form.encoded_len())
                .ok_or(AssemblerError::Encode(EncodeError::UnknownEncoding {
                    mnemonic: *mnemonic,
                    mode: *mode,
                }))
        }
    }
}

fn ensure_label_mode(mode: AddressingMode) -> Result<(), AssemblerError> {
    if matches!(
        mode,
        AddressingMode::Relative
            | AddressingMode::Absolute
            | AddressingMode::AbsoluteX
            | AddressingMode::AbsoluteY
            | AddressingMode::AbsoluteIndirect
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

fn relative_displacement(pc: u16, target: u16, label: &str) -> Result<i8, AssemblerError> {
    let next = i32::from(pc) + 2;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn instruction(mnemonic: Mnemonic, mode: AddressingMode, operand: Operand) -> Instruction {
        Instruction::new(mnemonic, mode, operand).unwrap()
    }

    #[test]
    fn labels_and_typed_data_are_placed() {
        let mut assembler = Assembler::new();
        assembler
            .label("loop")
            .emit(instruction(
                Mnemonic::Lda,
                AddressingMode::Immediate,
                Operand::Byte(1),
            ))
            .emit_label_ref(Mnemonic::Bne, AddressingMode::Relative, "loop")
            .word(0x1234)
            .label_word("loop");

        let program = assembler.assemble(0x8000).unwrap();
        assert_eq!(
            program.bytes(),
            &[0xA9, 0x01, 0xD0, 0xFC, 0x34, 0x12, 0x00, 0x80]
        );
        assert_eq!(program.label_address("loop"), Some(0x8000));
        assert_eq!(program.instruction_spans().len(), 2);
    }

    #[test]
    fn undefined_and_duplicate_labels_fail_closed() {
        let mut undefined = Assembler::new();
        undefined.emit_label_ref(Mnemonic::Jmp, AddressingMode::Absolute, "missing");
        assert!(matches!(
            undefined.assemble(0),
            Err(AssemblerError::UndefinedLabel { .. })
        ));

        let mut duplicate = Assembler::new();
        duplicate.label("same").label("same");
        assert!(matches!(
            duplicate.assemble(0),
            Err(AssemblerError::DuplicateLabel { .. })
        ));
    }

    #[test]
    fn relative_range_and_address_space_are_checked() {
        let mut branch = Assembler::new();
        branch
            .emit_label_ref(Mnemonic::Bne, AddressingMode::Relative, "far")
            .data(vec![0; 128])
            .label("far");
        assert!(matches!(
            branch.assemble(0),
            Err(AssemblerError::RelativeOutOfRange { .. })
        ));

        let mut overflow = Assembler::new();
        overflow.emit(instruction(
            Mnemonic::Jmp,
            AddressingMode::Absolute,
            Operand::Word(0),
        ));
        assert!(matches!(
            overflow.assemble(0xFFFE),
            Err(AssemblerError::AddressOverflow { .. })
        ));
    }
}
