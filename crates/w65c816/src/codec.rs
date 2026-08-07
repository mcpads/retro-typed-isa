//! Bidirectional instruction codec for the declared W65C816 profile.

use core::fmt;

use crate::isa::{AddressingMode, InstructionForm, Mnemonic, OPCODE_TABLE, WidthState, opcode_for};

/// Typed operand values serialized according to the instruction form and width state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    /// Implied or accumulator form with no encoded operand bytes.
    None,
    /// One unsigned byte.
    Byte(u8),
    /// One little-endian 16-bit word.
    Word(u16),
    /// One little-endian 24-bit address held in a `u32`.
    Long(u32),
    /// Signed 8-bit post-instruction displacement.
    Relative8(i8),
    /// Signed 16-bit post-instruction displacement.
    Relative16(i16),
    /// Encoded byte order is destination bank followed by source bank.
    BlockMove {
        /// Destination data-bank byte encoded first.
        destination_bank: u8,
        /// Source data-bank byte encoded second.
        source_bank: u8,
    },
}

impl Operand {
    fn kind(&self) -> &'static str {
        match self {
            Self::None => "no operand",
            Self::Byte(_) => "8-bit operand",
            Self::Word(_) => "16-bit operand",
            Self::Long(_) => "24-bit operand",
            Self::Relative8(_) => "8-bit relative operand",
            Self::Relative16(_) => "16-bit relative operand",
            Self::BlockMove { .. } => "block-move banks",
        }
    }
}

/// One typed W65C816 instruction and its concrete operand value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    /// Operation mnemonic.
    pub mnemonic: Mnemonic,
    /// Operand interpretation selected by the opcode.
    pub mode: AddressingMode,
    /// Typed operand value matching `mode` and the active width state.
    pub operand: Operand,
}

impl Instruction {
    /// Construct a typed instruction; operand/profile validation occurs at lowering.
    pub const fn new(mnemonic: Mnemonic, mode: AddressingMode, operand: Operand) -> Self {
        Self {
            mnemonic,
            mode,
            operand,
        }
    }

    /// Return the semantic form with the operand value removed.
    pub const fn form(&self) -> InstructionForm {
        InstructionForm::new(self.mnemonic, self.mode)
    }

    /// Return the complete encoded length under an explicit width state.
    pub fn encoded_len(&self, widths: WidthState) -> Result<usize, EncodeError> {
        let opcode = opcode_for(self.mnemonic, self.mode).ok_or(EncodeError::UnknownEncoding {
            mnemonic: self.mnemonic,
            mode: self.mode,
        })?;
        validate_operand(self, widths)?;
        Ok(1 + OPCODE_TABLE[usize::from(opcode)].operand_len(widths))
    }
}

/// One typed instruction decoded from the beginning of a source slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedInstruction {
    /// Exact physical opcode byte.
    pub opcode: u8,
    /// Typed instruction and operand value.
    pub instruction: Instruction,
    /// Exact number of source bytes consumed.
    pub len: usize,
}

/// A fail-closed reason that source bytes do not contain one complete instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// The source ends before the width-dependent instruction is complete.
    Truncated {
        /// Complete encoded length required under the supplied width state.
        expected: usize,
        /// Number of source bytes actually available.
        found: usize,
    },
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { expected, found } => {
                write!(
                    f,
                    "truncated instruction: need {expected} byte(s), found {found}"
                )
            }
        }
    }
}

impl std::error::Error for DecodeError {}

/// A fail-closed reason that a typed instruction cannot be lowered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodeError {
    /// No official opcode encodes the requested mnemonic and addressing mode.
    UnknownEncoding {
        /// Requested operation mnemonic.
        mnemonic: Mnemonic,
        /// Requested operand interpretation.
        mode: AddressingMode,
    },
    /// The operand type does not match the form and current width state.
    OperandMismatch {
        /// Requested operation mnemonic.
        mnemonic: Mnemonic,
        /// Requested operand interpretation.
        mode: AddressingMode,
        /// Human-readable operand category required by the profile.
        expected: &'static str,
        /// Human-readable category of the supplied operand.
        found: &'static str,
    },
    /// A purported 24-bit address has bits set above bit 23.
    LongOperandOutOfRange(u32),
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownEncoding { mnemonic, mode } => {
                write!(f, "no W65C816 encoding for {mnemonic} {mode:?}")
            }
            Self::OperandMismatch {
                mnemonic,
                mode,
                expected,
                found,
            } => write!(
                f,
                "invalid operand for {mnemonic} {mode:?}: expected {expected}, found {found}"
            ),
            Self::LongOperandOutOfRange(value) => {
                write!(f, "24-bit operand is out of range: 0x{value:X}")
            }
        }
    }
}

impl std::error::Error for EncodeError {}

/// Decode one instruction from the start of a W65C816 byte slice.
pub fn decode_bytes(bytes: &[u8], widths: WidthState) -> Result<DecodedInstruction, DecodeError> {
    let Some(&opcode) = bytes.first() else {
        return Err(DecodeError::Truncated {
            expected: 1,
            found: 0,
        });
    };
    let info = OPCODE_TABLE[usize::from(opcode)];
    let operand_len = info.operand_len(widths);
    let len = 1 + operand_len;
    if bytes.len() < len {
        return Err(DecodeError::Truncated {
            expected: len,
            found: bytes.len(),
        });
    }
    let raw = &bytes[1..len];
    let operand = decode_operand(info.mode, operand_len, raw);
    Ok(DecodedInstruction {
        opcode,
        instruction: Instruction::new(info.mnemonic, info.mode, operand),
        len,
    })
}

/// Encode one instruction in W65C816 byte order.
pub fn encode_bytes(instruction: &Instruction, widths: WidthState) -> Result<Vec<u8>, EncodeError> {
    let opcode =
        opcode_for(instruction.mnemonic, instruction.mode).ok_or(EncodeError::UnknownEncoding {
            mnemonic: instruction.mnemonic,
            mode: instruction.mode,
        })?;
    validate_operand(instruction, widths)?;

    let mut output = Vec::with_capacity(1 + OPCODE_TABLE[usize::from(opcode)].operand_len(widths));
    output.push(opcode);
    match instruction.operand {
        Operand::None => {}
        Operand::Byte(value) => output.push(value),
        Operand::Word(value) => output.extend_from_slice(&value.to_le_bytes()),
        Operand::Long(value) => {
            output.extend_from_slice(&[value as u8, (value >> 8) as u8, (value >> 16) as u8]);
        }
        Operand::Relative8(value) => output.push(value as u8),
        Operand::Relative16(value) => output.extend_from_slice(&value.to_le_bytes()),
        Operand::BlockMove {
            destination_bank,
            source_bank,
        } => output.extend_from_slice(&[destination_bank, source_bank]),
    }
    Ok(output)
}

// Compatibility names for the existing consumer. New code uses the explicit
// byte-lowering names shared by every profile.
#[doc(hidden)]
/// Compatibility wrapper for [`decode_bytes`].
pub fn decode(bytes: &[u8], widths: WidthState) -> Result<DecodedInstruction, DecodeError> {
    decode_bytes(bytes, widths)
}

#[doc(hidden)]
/// Compatibility wrapper for [`encode_bytes`].
pub fn encode(instruction: &Instruction, widths: WidthState) -> Result<Vec<u8>, EncodeError> {
    encode_bytes(instruction, widths)
}

fn decode_operand(mode: AddressingMode, operand_len: usize, raw: &[u8]) -> Operand {
    use AddressingMode::*;
    match mode {
        Implied | Accumulator => Operand::None,
        Relative8 => Operand::Relative8(raw[0] as i8),
        Relative16 => Operand::Relative16(i16::from_le_bytes([raw[0], raw[1]])),
        BlockMove => Operand::BlockMove {
            destination_bank: raw[0],
            source_bank: raw[1],
        },
        AbsoluteLong | AbsoluteLongX => {
            Operand::Long(u32::from(raw[0]) | (u32::from(raw[1]) << 8) | (u32::from(raw[2]) << 16))
        }
        Immediate if operand_len == 2 => Operand::Word(u16::from_le_bytes([raw[0], raw[1]])),
        Absolute
        | AbsoluteX
        | AbsoluteY
        | AbsoluteIndirect
        | AbsoluteIndexedIndirectX
        | AbsoluteIndirectLong => Operand::Word(u16::from_le_bytes([raw[0], raw[1]])),
        Immediate
        | ImmediateByte
        | DirectPage
        | DirectPageX
        | DirectPageY
        | DirectPageIndirect
        | DirectPageIndirectLong
        | DirectPageIndexedIndirectX
        | DirectPageIndirectIndexedY
        | DirectPageIndirectLongIndexedY
        | StackRelative
        | StackRelativeIndirectIndexedY => Operand::Byte(raw[0]),
    }
}

fn validate_operand(instruction: &Instruction, widths: WidthState) -> Result<(), EncodeError> {
    use AddressingMode::*;
    let expected_len = opcode_for(instruction.mnemonic, instruction.mode)
        .map(|opcode| OPCODE_TABLE[usize::from(opcode)].operand_len(widths))
        .ok_or(EncodeError::UnknownEncoding {
            mnemonic: instruction.mnemonic,
            mode: instruction.mode,
        })?;

    let valid = match instruction.mode {
        Implied | Accumulator => matches!(instruction.operand, Operand::None),
        Relative8 => matches!(instruction.operand, Operand::Relative8(_)),
        Relative16 => matches!(instruction.operand, Operand::Relative16(_)),
        BlockMove => matches!(instruction.operand, Operand::BlockMove { .. }),
        AbsoluteLong | AbsoluteLongX => matches!(instruction.operand, Operand::Long(_)),
        Immediate if expected_len == 2 => matches!(instruction.operand, Operand::Word(_)),
        Absolute
        | AbsoluteX
        | AbsoluteY
        | AbsoluteIndirect
        | AbsoluteIndexedIndirectX
        | AbsoluteIndirectLong => matches!(instruction.operand, Operand::Word(_)),
        Immediate
        | ImmediateByte
        | DirectPage
        | DirectPageX
        | DirectPageY
        | DirectPageIndirect
        | DirectPageIndirectLong
        | DirectPageIndexedIndirectX
        | DirectPageIndirectIndexedY
        | DirectPageIndirectLongIndexedY
        | StackRelative
        | StackRelativeIndirectIndexedY => matches!(instruction.operand, Operand::Byte(_)),
    };

    if !valid {
        return Err(EncodeError::OperandMismatch {
            mnemonic: instruction.mnemonic,
            mode: instruction.mode,
            expected: expected_operand_kind(instruction.mode, expected_len),
            found: instruction.operand.kind(),
        });
    }
    if let Operand::Long(value) = instruction.operand {
        if value > 0xFF_FFFF {
            return Err(EncodeError::LongOperandOutOfRange(value));
        }
    }
    Ok(())
}

fn expected_operand_kind(mode: AddressingMode, operand_len: usize) -> &'static str {
    use AddressingMode::*;
    match mode {
        Implied | Accumulator => "no operand",
        Relative8 => "8-bit relative operand",
        Relative16 => "16-bit relative operand",
        BlockMove => "destination/source bank pair",
        AbsoluteLong | AbsoluteLongX => "24-bit operand",
        Immediate if operand_len == 2 => "16-bit operand",
        Absolute
        | AbsoluteX
        | AbsoluteY
        | AbsoluteIndirect
        | AbsoluteIndexedIndirectX
        | AbsoluteIndirectLong => "16-bit operand",
        _ => "8-bit operand",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIDTH_STATES: [WidthState; 4] = [
        WidthState::M8_X8,
        WidthState::M8_X16,
        WidthState::M16_X8,
        WidthState::M16_X16,
    ];

    #[test]
    fn every_opcode_roundtrips_in_every_width_state() {
        for widths in WIDTH_STATES {
            for opcode in 0_u8..=u8::MAX {
                let operand_len = OPCODE_TABLE[usize::from(opcode)].operand_len(widths);
                let mut raw = vec![opcode];
                raw.extend((0..operand_len).map(|index| 0x41 + index as u8));
                let decoded = decode_bytes(&raw, widths).unwrap();
                assert_eq!(decoded.len, raw.len(), "opcode 0x{opcode:02X}");
                assert_eq!(encode_bytes(&decoded.instruction, widths).unwrap(), raw);
            }
        }
    }

    #[test]
    fn encoder_and_decoder_agree_on_dynamic_immediate_width() {
        let lda8 = Instruction::new(
            Mnemonic::Lda,
            AddressingMode::Immediate,
            Operand::Byte(0x7F),
        );
        assert_eq!(
            encode_bytes(&lda8, WidthState::M8_X16).unwrap(),
            [0xA9, 0x7F]
        );
        assert!(encode_bytes(&lda8, WidthState::M16_X8).is_err());

        let ldx16 = Instruction::new(
            Mnemonic::Ldx,
            AddressingMode::Immediate,
            Operand::Word(0x1234),
        );
        assert_eq!(
            encode_bytes(&ldx16, WidthState::M8_X16).unwrap(),
            [0xA2, 0x34, 0x12]
        );
        assert!(encode_bytes(&ldx16, WidthState::M16_X8).is_err());
    }

    #[test]
    fn block_move_byte_order_is_explicit() {
        let mvn = Instruction::new(
            Mnemonic::Mvn,
            AddressingMode::BlockMove,
            Operand::BlockMove {
                destination_bank: 0x7E,
                source_bank: 0x12,
            },
        );
        assert_eq!(
            encode_bytes(&mvn, WidthState::M8_X8).unwrap(),
            [0x54, 0x7E, 0x12]
        );
    }

    #[test]
    fn decode_rejects_truncated_input() {
        assert_eq!(
            decode_bytes(&[0xAF, 0x34], WidthState::M8_X8),
            Err(DecodeError::Truncated {
                expected: 4,
                found: 2,
            })
        );
    }
}
