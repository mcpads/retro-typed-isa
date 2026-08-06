//! Two-pass, label-aware W65C816 assembly with placement verification.

use std::collections::BTreeMap;
use std::fmt;

use crate::codec::{DecodeError, EncodeError, Instruction, Operand, decode_bytes, encode_bytes};
use crate::isa::{AddressingMode, Mnemonic, OPCODE_TABLE, WidthState, opcode_for};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuMode {
    Emulation,
    Native(WidthState),
}

impl CpuMode {
    pub const NATIVE_M8_X8: Self = Self::Native(WidthState::M8_X8);
    pub const NATIVE_M8_X16: Self = Self::Native(WidthState::M8_X16);
    pub const NATIVE_M16_X8: Self = Self::Native(WidthState::M16_X8);
    pub const NATIVE_M16_X16: Self = Self::Native(WidthState::M16_X16);

    pub const fn widths(self) -> WidthState {
        match self {
            Self::Emulation => WidthState::M8_X8,
            Self::Native(widths) => widths,
        }
    }

    fn apply_rep(&mut self, mask: u8) {
        if let Self::Native(widths) = self {
            widths.apply_rep(mask);
        }
    }

    fn apply_sep(&mut self, mask: u8) {
        if let Self::Native(widths) = self {
            widths.apply_sep(mask);
        }
    }
}

#[derive(Debug, Clone)]
enum Statement {
    Label(String),
    Instruction(Instruction),
    LabelReference {
        mnemonic: Mnemonic,
        mode: AddressingMode,
        label: String,
    },
    Data(Vec<u8>),
    AssumeMode(CpuMode),
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

    /// Emit an instruction whose operand is resolved from a label.
    ///
    /// Relative, 16-bit absolute, and 24-bit absolute modes are supported.
    /// Direct-page labels are intentionally rejected because their base is a
    /// runtime register rather than a property of code placement.
    pub fn emit_label_ref(
        &mut self,
        mnemonic: Mnemonic,
        mode: AddressingMode,
        label: impl Into<String>,
    ) -> &mut Self {
        self.statements.push(Statement::LabelReference {
            mnemonic,
            mode,
            label: label.into(),
        });
        self
    }

    /// Emit declared inline data. This is not an instruction fallback.
    pub fn data(&mut self, bytes: impl Into<Vec<u8>>) -> &mut Self {
        self.statements.push(Statement::Data(bytes.into()));
        self
    }

    /// Restore an explicit CPU mode after a transition that cannot be inferred
    /// statically, such as XCE or a native-mode PLP/RTI.
    pub fn assume_mode(&mut self, mode: CpuMode) -> &mut Self {
        self.statements.push(Statement::AssumeMode(mode));
        self
    }

    pub fn assemble(
        &self,
        origin: u32,
        initial_mode: CpuMode,
    ) -> Result<AssembledProgram, AssemblerError> {
        if origin > 0xFF_FFFF {
            return Err(AssemblerError::OriginOutOfRange(origin));
        }
        let (labels, total_len) = self.first_pass(origin, initial_mode)?;
        ensure_bank_local(origin, total_len)?;

        let mut bytes = Vec::with_capacity(total_len);
        let mut instructions = Vec::new();
        let mut mode = Some(initial_mode);

        for statement in &self.statements {
            match statement {
                Statement::Label(_) => {}
                Statement::Data(data) => bytes.extend_from_slice(data),
                Statement::AssumeMode(assumed) => mode = Some(*assumed),
                Statement::Instruction(instruction) => {
                    let current_mode = require_mode(mode, bytes.len())?;
                    let encoded = encode_bytes(instruction, current_mode.widths())?;
                    let offset = bytes.len();
                    bytes.extend_from_slice(&encoded);
                    instructions.push(InstructionSpan {
                        offset,
                        len: encoded.len(),
                        widths: current_mode.widths(),
                        instruction: instruction.clone(),
                    });
                    mode = transition_mode(current_mode, instruction);
                }
                Statement::LabelReference {
                    mnemonic,
                    mode: addressing_mode,
                    label,
                } => {
                    let current_mode = require_mode(mode, bytes.len())?;
                    let offset = bytes.len();
                    let instruction = resolve_label_reference(
                        origin,
                        offset,
                        *mnemonic,
                        *addressing_mode,
                        label,
                        &labels,
                        current_mode.widths(),
                    )?;
                    let encoded = encode_bytes(&instruction, current_mode.widths())?;
                    bytes.extend_from_slice(&encoded);
                    instructions.push(InstructionSpan {
                        offset,
                        len: encoded.len(),
                        widths: current_mode.widths(),
                        instruction,
                    });
                    mode = transition_mode(current_mode, &instructions.last().unwrap().instruction);
                }
            }
        }

        debug_assert_eq!(bytes.len(), total_len);
        verify_instruction_spans(&bytes, &instructions)?;
        Ok(AssembledProgram {
            origin,
            bytes,
            labels,
            instructions,
        })
    }

    fn first_pass(
        &self,
        origin: u32,
        initial_mode: CpuMode,
    ) -> Result<(BTreeMap<String, u32>, usize), AssemblerError> {
        let mut labels = BTreeMap::new();
        let mut offset = 0_usize;
        let mut mode = Some(initial_mode);

        for statement in &self.statements {
            match statement {
                Statement::Label(name) => {
                    if (origin & 0xFFFF) as usize + offset >= 0x1_0000 {
                        return Err(AssemblerError::CrossesBankBoundary {
                            origin,
                            len: offset,
                        });
                    }
                    let address = origin
                        .checked_add(offset as u32)
                        .ok_or(AssemblerError::AddressOverflow)?;
                    if address > 0xFF_FFFF {
                        return Err(AssemblerError::AddressOverflow);
                    }
                    if labels.insert(name.clone(), address).is_some() {
                        return Err(AssemblerError::DuplicateLabel(name.clone()));
                    }
                }
                Statement::Data(data) => {
                    offset = offset
                        .checked_add(data.len())
                        .ok_or(AssemblerError::AddressOverflow)?;
                }
                Statement::AssumeMode(assumed) => mode = Some(*assumed),
                Statement::Instruction(instruction) => {
                    let current_mode = require_mode(mode, offset)?;
                    offset = offset
                        .checked_add(instruction.encoded_len(current_mode.widths())?)
                        .ok_or(AssemblerError::AddressOverflow)?;
                    mode = transition_mode(current_mode, instruction);
                }
                Statement::LabelReference {
                    mnemonic,
                    mode: addressing_mode,
                    ..
                } => {
                    let current_mode = require_mode(mode, offset)?;
                    let opcode = opcode_for(*mnemonic, *addressing_mode).ok_or(
                        EncodeError::UnknownEncoding {
                            mnemonic: *mnemonic,
                            mode: *addressing_mode,
                        },
                    )?;
                    ensure_label_mode(*addressing_mode)?;
                    offset = offset
                        .checked_add(
                            1 + OPCODE_TABLE[usize::from(opcode)]
                                .operand_len(current_mode.widths()),
                        )
                        .ok_or(AssemblerError::AddressOverflow)?;
                }
            }
        }
        Ok((labels, offset))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionSpan {
    pub offset: usize,
    pub len: usize,
    pub widths: WidthState,
    pub instruction: Instruction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledProgram {
    origin: u32,
    bytes: Vec<u8>,
    labels: BTreeMap<String, u32>,
    instructions: Vec<InstructionSpan>,
}

impl AssembledProgram {
    pub fn origin(&self) -> u32 {
        self.origin
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn label_address(&self, name: &str) -> Option<u32> {
        self.labels.get(name).copied()
    }

    pub fn instruction_spans(&self) -> &[InstructionSpan] {
        &self.instructions
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssemblerError {
    Decode(DecodeError),
    Encode(EncodeError),
    OriginOutOfRange(u32),
    AddressOverflow,
    CrossesBankBoundary {
        origin: u32,
        len: usize,
    },
    DuplicateLabel(String),
    UndefinedLabel(String),
    UnsupportedLabelMode(AddressingMode),
    RelativeOutOfRange {
        label: String,
        displacement: i64,
        bits: u8,
    },
    BankMismatch {
        label: String,
        origin_bank: u8,
        target_bank: u8,
    },
    CpuModeUnknown {
        offset: usize,
    },
    VerificationFailed {
        offset: usize,
    },
}

impl fmt::Display for AssemblerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(error) => error.fmt(f),
            Self::Encode(error) => error.fmt(f),
            Self::OriginOutOfRange(origin) => {
                write!(
                    f,
                    "assembly origin exceeds 24-bit address space: 0x{origin:X}"
                )
            }
            Self::AddressOverflow => write!(f, "assembled address overflow"),
            Self::CrossesBankBoundary { origin, len } => write!(
                f,
                "assembled block crosses a 64 KiB bank: origin 0x{origin:06X}, length {len}"
            ),
            Self::DuplicateLabel(label) => write!(f, "duplicate label {label:?}"),
            Self::UndefinedLabel(label) => write!(f, "undefined label {label:?}"),
            Self::UnsupportedLabelMode(mode) => {
                write!(f, "labels are not supported for {mode:?}")
            }
            Self::RelativeOutOfRange {
                label,
                displacement,
                bits,
            } => write!(
                f,
                "{bits}-bit branch to {label:?} is out of range: {displacement}"
            ),
            Self::BankMismatch {
                label,
                origin_bank,
                target_bank,
            } => write!(
                f,
                "16-bit reference to {label:?} crosses banks: 0x{origin_bank:02X} -> 0x{target_bank:02X}"
            ),
            Self::CpuModeUnknown { offset } => write!(
                f,
                "CPU mode is unknown at code offset 0x{offset:X}; add assume_mode after XCE or a status restore"
            ),
            Self::VerificationFailed { offset } => {
                write!(
                    f,
                    "assembled instruction failed decode verification at 0x{offset:X}"
                )
            }
        }
    }
}

impl std::error::Error for AssemblerError {}

impl From<DecodeError> for AssemblerError {
    fn from(value: DecodeError) -> Self {
        Self::Decode(value)
    }
}

impl From<EncodeError> for AssemblerError {
    fn from(value: EncodeError) -> Self {
        Self::Encode(value)
    }
}

fn ensure_bank_local(origin: u32, len: usize) -> Result<(), AssemblerError> {
    let bank_offset = (origin & 0xFFFF) as usize;
    if bank_offset
        .checked_add(len)
        .is_none_or(|end| end > 0x1_0000)
    {
        return Err(AssemblerError::CrossesBankBoundary { origin, len });
    }
    if origin
        .checked_add(len.saturating_sub(1) as u32)
        .is_none_or(|end| end > 0xFF_FFFF)
    {
        return Err(AssemblerError::AddressOverflow);
    }
    Ok(())
}

fn require_mode(mode: Option<CpuMode>, offset: usize) -> Result<CpuMode, AssemblerError> {
    mode.ok_or(AssemblerError::CpuModeUnknown { offset })
}

pub(crate) fn transition_mode(current: CpuMode, instruction: &Instruction) -> Option<CpuMode> {
    let mut mode = current;
    match (instruction.mnemonic, &instruction.operand) {
        (Mnemonic::Rep, Operand::Byte(mask)) => mode.apply_rep(*mask),
        (Mnemonic::Sep, Operand::Byte(mask)) => mode.apply_sep(*mask),
        (Mnemonic::Xce, _) => return None,
        (Mnemonic::Plp | Mnemonic::Rti, _) if matches!(mode, CpuMode::Native(_)) => return None,
        _ => {}
    }
    Some(mode)
}

fn ensure_label_mode(mode: AddressingMode) -> Result<(), AssemblerError> {
    use AddressingMode::*;
    if matches!(
        mode,
        Relative8
            | Relative16
            | Absolute
            | AbsoluteX
            | AbsoluteY
            | AbsoluteIndirect
            | AbsoluteIndexedIndirectX
            | AbsoluteIndirectLong
            | AbsoluteLong
            | AbsoluteLongX
    ) {
        Ok(())
    } else {
        Err(AssemblerError::UnsupportedLabelMode(mode))
    }
}

fn resolve_label_reference(
    origin: u32,
    offset: usize,
    mnemonic: Mnemonic,
    mode: AddressingMode,
    label: &str,
    labels: &BTreeMap<String, u32>,
    widths: WidthState,
) -> Result<Instruction, AssemblerError> {
    ensure_label_mode(mode)?;
    let target = labels
        .get(label)
        .copied()
        .ok_or_else(|| AssemblerError::UndefinedLabel(label.to_owned()))?;
    let opcode =
        opcode_for(mnemonic, mode).ok_or(EncodeError::UnknownEncoding { mnemonic, mode })?;
    let instruction_len = 1 + OPCODE_TABLE[usize::from(opcode)].operand_len(widths);
    let next = i64::from(origin) + offset as i64 + instruction_len as i64;

    use AddressingMode::*;
    let operand = match mode {
        Relative8 => {
            let displacement = i64::from(target) - next;
            let value =
                i8::try_from(displacement).map_err(|_| AssemblerError::RelativeOutOfRange {
                    label: label.to_owned(),
                    displacement,
                    bits: 8,
                })?;
            Operand::Relative8(value)
        }
        Relative16 => {
            let displacement = i64::from(target) - next;
            let value =
                i16::try_from(displacement).map_err(|_| AssemblerError::RelativeOutOfRange {
                    label: label.to_owned(),
                    displacement,
                    bits: 16,
                })?;
            Operand::Relative16(value)
        }
        Absolute
        | AbsoluteX
        | AbsoluteY
        | AbsoluteIndirect
        | AbsoluteIndexedIndirectX
        | AbsoluteIndirectLong => {
            let origin_bank = (origin >> 16) as u8;
            let target_bank = (target >> 16) as u8;
            if origin_bank != target_bank {
                return Err(AssemblerError::BankMismatch {
                    label: label.to_owned(),
                    origin_bank,
                    target_bank,
                });
            }
            Operand::Word(target as u16)
        }
        AbsoluteLong | AbsoluteLongX => Operand::Long(target),
        _ => return Err(AssemblerError::UnsupportedLabelMode(mode)),
    };
    Ok(Instruction::new(mnemonic, mode, operand))
}

fn verify_instruction_spans(
    bytes: &[u8],
    instructions: &[InstructionSpan],
) -> Result<(), AssemblerError> {
    for span in instructions {
        let decoded = decode_bytes(&bytes[span.offset..], span.widths)?;
        if decoded.len != span.len || decoded.instruction != span.instruction {
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

    fn implied(mnemonic: Mnemonic) -> Instruction {
        Instruction::new(mnemonic, AddressingMode::Implied, Operand::None)
    }

    #[test]
    fn resolves_relative_and_absolute_labels_then_verifies_output() {
        let mut program = Assembler::new();
        program
            .emit_label_ref(Mnemonic::Jsr, AddressingMode::Absolute, "sub")
            .emit_label_ref(Mnemonic::Bne, AddressingMode::Relative8, "done")
            .label("sub")
            .emit(implied(Mnemonic::Rts))
            .label("done")
            .emit(implied(Mnemonic::Rtl));

        let assembled = program.assemble(0x80_D000, CpuMode::NATIVE_M8_X8).unwrap();
        assert_eq!(
            assembled.bytes(),
            [0x20, 0x05, 0xD0, 0xD0, 0x01, 0x60, 0x6B]
        );
        assert_eq!(assembled.label_address("sub"), Some(0x80_D005));
        assert_eq!(assembled.instruction_spans().len(), 4);
    }

    #[test]
    fn rep_and_sep_track_effective_widths_in_native_mode() {
        let mut program = Assembler::new();
        program
            .emit(Instruction::new(
                Mnemonic::Rep,
                AddressingMode::ImmediateByte,
                Operand::Byte(0x20),
            ))
            .emit(Instruction::new(
                Mnemonic::Lda,
                AddressingMode::Immediate,
                Operand::Word(0x1234),
            ))
            .emit(Instruction::new(
                Mnemonic::Sep,
                AddressingMode::ImmediateByte,
                Operand::Byte(0x20),
            ))
            .emit(Instruction::new(
                Mnemonic::Lda,
                AddressingMode::Immediate,
                Operand::Byte(0x56),
            ));
        assert_eq!(
            program
                .assemble(0x80_8000, CpuMode::NATIVE_M8_X8)
                .unwrap()
                .bytes(),
            [0xC2, 0x20, 0xA9, 0x34, 0x12, 0xE2, 0x20, 0xA9, 0x56]
        );
        assert!(program.assemble(0x80_8000, CpuMode::Emulation).is_err());
    }

    #[test]
    fn uncertain_mode_transition_requires_an_explicit_assumption() {
        let mut rejected = Assembler::new();
        rejected
            .emit(implied(Mnemonic::Xce))
            .emit(implied(Mnemonic::Nop));
        assert!(matches!(
            rejected.assemble(0x80_8000, CpuMode::Emulation),
            Err(AssemblerError::CpuModeUnknown { offset: 1 })
        ));

        let mut accepted = Assembler::new();
        accepted
            .emit(implied(Mnemonic::Xce))
            .assume_mode(CpuMode::NATIVE_M8_X8)
            .emit(implied(Mnemonic::Nop));
        assert_eq!(
            accepted
                .assemble(0x80_8000, CpuMode::Emulation)
                .unwrap()
                .bytes(),
            [0xFB, 0xEA]
        );
    }

    #[test]
    fn rejects_out_of_range_branches_and_bank_crossing() {
        let mut branch = Assembler::new();
        branch.emit_label_ref(Mnemonic::Bne, AddressingMode::Relative8, "far");
        branch
            .data(vec![0; 128])
            .label("far")
            .emit(implied(Mnemonic::Nop));
        assert!(matches!(
            branch.assemble(0x80_8000, CpuMode::NATIVE_M8_X8),
            Err(AssemblerError::RelativeOutOfRange { bits: 8, .. })
        ));

        let mut crossing = Assembler::new();
        crossing.data(vec![0; 2]);
        assert!(matches!(
            crossing.assemble(0x80_FFFF, CpuMode::NATIVE_M8_X8),
            Err(AssemblerError::CrossesBankBoundary { .. })
        ));

        let mut end_label = Assembler::new();
        end_label.emit(implied(Mnemonic::Nop)).label("wrapped");
        assert!(matches!(
            end_label.assemble(0x80_FFFF, CpuMode::NATIVE_M8_X8),
            Err(AssemblerError::CrossesBankBoundary { .. })
        ));
    }

    #[test]
    fn every_profile_opcode_can_be_assembled_without_raw_instruction_bytes() {
        for widths in [
            WidthState::M8_X8,
            WidthState::M8_X16,
            WidthState::M16_X8,
            WidthState::M16_X16,
        ] {
            for opcode in 0_u8..=u8::MAX {
                let operand_len = OPCODE_TABLE[usize::from(opcode)].operand_len(widths);
                let mut raw = vec![opcode];
                raw.extend((0..operand_len).map(|index| 0x51 + index as u8));
                let instruction = decode_bytes(&raw, widths).unwrap().instruction;
                let mut program = Assembler::new();
                program.emit(instruction);
                assert_eq!(
                    program
                        .assemble(0x80_8000, CpuMode::Native(widths))
                        .unwrap()
                        .bytes(),
                    raw,
                    "opcode 0x{opcode:02X}, widths {widths:?}"
                );
            }
        }
    }
}
