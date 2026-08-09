//! Two-pass, label-aware W65C816 assembly with placement verification.

use std::collections::BTreeMap;
use std::fmt;

use typed_isa_core::InstructionBytes;

use crate::CodeLocation;
use crate::codec::{DecodeError, EncodeError, Instruction, Operand, decode_bytes, encode_bytes};
use crate::isa::{AddressingMode, Mnemonic, OPCODE_TABLE, WidthState, opcode_for};

/// CPU mode needed to determine M/X-dependent immediate widths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuMode {
    /// Emulation mode, where accumulator and index widths are both eight bits.
    Emulation,
    /// Native mode with explicit accumulator and index widths.
    Native(WidthState),
}

impl CpuMode {
    /// Native mode with eight-bit accumulator and index registers.
    pub const NATIVE_M8_X8: Self = Self::Native(WidthState::M8_X8);
    /// Native mode with eight-bit accumulator and sixteen-bit index registers.
    pub const NATIVE_M8_X16: Self = Self::Native(WidthState::M8_X16);
    /// Native mode with sixteen-bit accumulator and eight-bit index registers.
    pub const NATIVE_M16_X8: Self = Self::Native(WidthState::M16_X8);
    /// Native mode with sixteen-bit accumulator and index registers.
    pub const NATIVE_M16_X16: Self = Self::Native(WidthState::M16_X16);

    /// Return the effective accumulator and index widths for this mode.
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

/// Two-pass builder for typed W65C816 code, labels, explicit data, and mode assumptions.
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

    /// Append one typed instruction interpreted under the current CPU mode.
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

    /// Resolve labels, lower typed code, and verify every instruction span.
    ///
    /// The complete output must remain within one 64 KiB program bank. Immediate
    /// widths follow `initial_mode`, REP/SEP transitions, and explicit assumptions.
    pub fn assemble(
        &self,
        origin: CodeLocation,
        initial_mode: CpuMode,
    ) -> Result<AssembledProgram, AssemblerError> {
        let origin_address = origin.to_u32();
        let (labels, total_len) = self.first_pass(origin_address, initial_mode)?;
        ensure_bank_local(origin_address, total_len)?;

        let mut bytes = Vec::with_capacity(total_len);
        let mut instructions = Vec::new();
        let mut instruction_width_states = Vec::new();
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
                    let location = location_at(origin, offset)?;
                    bytes.extend_from_slice(&encoded);
                    instructions.push(InstructionSpan {
                        offset,
                        location,
                        bytes: InstructionBytes::new(encoded)
                            .expect("encoded W65C816 instructions are non-empty"),
                        instruction: instruction.clone(),
                    });
                    instruction_width_states.push(InstructionWidthState {
                        offset,
                        location,
                        widths: current_mode.widths(),
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
                        origin_address,
                        offset,
                        *mnemonic,
                        *addressing_mode,
                        label,
                        &labels,
                        current_mode.widths(),
                    )?;
                    let encoded = encode_bytes(&instruction, current_mode.widths())?;
                    let location = location_at(origin, offset)?;
                    bytes.extend_from_slice(&encoded);
                    instructions.push(InstructionSpan {
                        offset,
                        location,
                        bytes: InstructionBytes::new(encoded)
                            .expect("encoded W65C816 instructions are non-empty"),
                        instruction,
                    });
                    instruction_width_states.push(InstructionWidthState {
                        offset,
                        location,
                        widths: current_mode.widths(),
                    });
                    mode = transition_mode(current_mode, &instructions.last().unwrap().instruction);
                }
            }
        }

        debug_assert_eq!(bytes.len(), total_len);
        verify_instruction_spans(&bytes, &instructions, &instruction_width_states)?;
        Ok(AssembledProgram {
            origin,
            bytes,
            labels: labels
                .into_iter()
                .map(|(name, address)| (name, unpack_location(address)))
                .collect(),
            instructions,
            instruction_width_states,
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

/// Workspace-wide instruction-placement evidence specialized for W65C816.
pub type InstructionSpan = typed_isa_core::InstructionSpan<CodeLocation, Instruction>;

/// W65C816 width state attached to one assembled instruction location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstructionWidthState {
    /// Byte offset relative to the beginning of the assembled program.
    pub offset: usize,
    /// Banked architectural code location governed by `widths`.
    pub location: CodeLocation,
    /// Accumulator and index widths used to lower the instruction.
    pub widths: WidthState,
}

/// Checked single-bank output with resolved labels and verified instruction spans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledProgram {
    origin: CodeLocation,
    bytes: Vec<u8>,
    labels: BTreeMap<String, CodeLocation>,
    instructions: Vec<InstructionSpan>,
    instruction_width_states: Vec<InstructionWidthState>,
}

impl AssembledProgram {
    /// Return the banked code location assigned to the first output byte.
    pub fn origin(&self) -> CodeLocation {
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

    /// Return the resolved code location of a defined label, if present.
    pub fn label_location(&self, name: &str) -> Option<CodeLocation> {
        self.labels.get(name).copied()
    }

    /// Borrow the locations and identities of executable instructions only.
    pub fn instruction_spans(&self) -> &[InstructionSpan] {
        &self.instructions
    }

    /// Borrow the W65C816-specific M/X state paired with each instruction span.
    pub fn instruction_width_states(&self) -> &[InstructionWidthState] {
        &self.instruction_width_states
    }
}

/// A fail-closed reason that W65C816 assembly did not produce a program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssemblerError {
    /// Post-placement decoding failed.
    Decode(DecodeError),
    /// Typed instruction construction or lowering failed.
    Encode(EncodeError),
    /// Address calculation overflowed the host or 24-bit address range.
    AddressOverflow,
    /// The complete output crosses a 64 KiB program-bank boundary.
    CrossesBankBoundary {
        /// Address assigned to the first output byte.
        origin: u32,
        /// Complete or intermediate output length that does not fit.
        len: usize,
    },
    /// A label name was defined more than once.
    DuplicateLabel(String),
    /// A label reference names no definition in the program.
    UndefinedLabel(String),
    /// A label was requested for a runtime-relative or otherwise unsupported mode.
    UnsupportedLabelMode(AddressingMode),
    /// A relative label target does not fit the selected displacement width.
    RelativeOutOfRange {
        /// Target label name.
        label: String,
        /// Required byte displacement from the post-instruction address.
        displacement: i64,
        /// Signed encoded displacement width.
        bits: u8,
    },
    /// A 16-bit address form targets a different program bank.
    BankMismatch {
        /// Target label name.
        label: String,
        /// Program bank containing the instruction.
        origin_bank: u8,
        /// Program bank containing the target.
        target_bank: u8,
    },
    /// A preceding status operation made immediate width state unknowable.
    CpuModeUnknown {
        /// Offset requiring a new explicit mode assumption.
        offset: usize,
    },
    /// An emitted instruction changed identity during byte round-trip verification.
    VerificationFailed {
        /// Byte offset of the inconsistent instruction span.
        offset: usize,
    },
}

impl fmt::Display for AssemblerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(error) => error.fmt(f),
            Self::Encode(error) => error.fmt(f),
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

fn unpack_location(address: u32) -> CodeLocation {
    CodeLocation::new((address >> 16) as u8, address as u16)
}

fn location_at(origin: CodeLocation, offset: usize) -> Result<CodeLocation, AssemblerError> {
    let address = origin
        .to_u32()
        .checked_add(u32::try_from(offset).map_err(|_| AssemblerError::AddressOverflow)?)
        .ok_or(AssemblerError::AddressOverflow)?;
    if address >> 16 != u32::from(origin.bank()) {
        return Err(AssemblerError::CrossesBankBoundary {
            origin: origin.to_u32(),
            len: offset,
        });
    }
    Ok(unpack_location(address))
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
    width_states: &[InstructionWidthState],
) -> Result<(), AssemblerError> {
    if instructions.len() != width_states.len() {
        return Err(AssemblerError::VerificationFailed { offset: 0 });
    }
    for (span, width_state) in instructions.iter().zip(width_states) {
        let decoded = decode_bytes(&bytes[span.offset..], width_state.widths)?;
        if span.offset != width_state.offset
            || span.location != width_state.location
            || decoded.len != span.bytes.len().get()
            || decoded.instruction != span.instruction
            || &bytes[span.offset..span.offset + decoded.len] != span.bytes.as_slice()
        {
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

    fn location(address: u32) -> CodeLocation {
        unpack_location(address)
    }

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

        let assembled = program
            .assemble(location(0x80_D000), CpuMode::NATIVE_M8_X8)
            .unwrap();
        assert_eq!(
            assembled.bytes(),
            [0x20, 0x05, 0xD0, 0xD0, 0x01, 0x60, 0x6B]
        );
        assert_eq!(assembled.label_location("sub"), Some(location(0x80_D005)));
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
                .assemble(location(0x80_8000), CpuMode::NATIVE_M8_X8)
                .unwrap()
                .bytes(),
            [0xC2, 0x20, 0xA9, 0x34, 0x12, 0xE2, 0x20, 0xA9, 0x56]
        );
        assert!(
            program
                .assemble(location(0x80_8000), CpuMode::Emulation)
                .is_err()
        );
    }

    #[test]
    fn uncertain_mode_transition_requires_an_explicit_assumption() {
        let mut rejected = Assembler::new();
        rejected
            .emit(implied(Mnemonic::Xce))
            .emit(implied(Mnemonic::Nop));
        assert!(matches!(
            rejected.assemble(location(0x80_8000), CpuMode::Emulation),
            Err(AssemblerError::CpuModeUnknown { offset: 1 })
        ));

        let mut accepted = Assembler::new();
        accepted
            .emit(implied(Mnemonic::Xce))
            .assume_mode(CpuMode::NATIVE_M8_X8)
            .emit(implied(Mnemonic::Nop));
        assert_eq!(
            accepted
                .assemble(location(0x80_8000), CpuMode::Emulation)
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
            branch.assemble(location(0x80_8000), CpuMode::NATIVE_M8_X8),
            Err(AssemblerError::RelativeOutOfRange { bits: 8, .. })
        ));

        let mut crossing = Assembler::new();
        crossing.data(vec![0; 2]);
        assert!(matches!(
            crossing.assemble(location(0x80_FFFF), CpuMode::NATIVE_M8_X8),
            Err(AssemblerError::CrossesBankBoundary { .. })
        ));

        let mut end_label = Assembler::new();
        end_label.emit(implied(Mnemonic::Nop)).label("wrapped");
        assert!(matches!(
            end_label.assemble(location(0x80_FFFF), CpuMode::NATIVE_M8_X8),
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
                        .assemble(location(0x80_8000), CpuMode::Native(widths))
                        .unwrap()
                        .bytes(),
                    raw,
                    "opcode 0x{opcode:02X}, widths {widths:?}"
                );
            }
        }
    }
}
