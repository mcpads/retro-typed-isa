//! Bidirectional instruction codec for the declared W65C816 profile.

use core::fmt;

use crate::isa::{AddressingMode, InstructionForm, Mnemonic, OPCODE_TABLE, WidthState, opcode_for};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    None,
    Byte(u8),
    Word(u16),
    Long(u32),
    Relative8(i8),
    Relative16(i16),
    /// Encoded byte order is destination bank followed by source bank.
    BlockMove {
        destination_bank: u8,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    pub mnemonic: Mnemonic,
    pub mode: AddressingMode,
    pub operand: Operand,
}

impl Instruction {
    pub const fn new(mnemonic: Mnemonic, mode: AddressingMode, operand: Operand) -> Self {
        Self {
            mnemonic,
            mode,
            operand,
        }
    }

    pub const fn form(&self) -> InstructionForm {
        InstructionForm::new(self.mnemonic, self.mode)
    }

    pub fn encoded_len(&self, widths: WidthState) -> Result<usize, EncodeError> {
        let opcode = opcode_for(self.mnemonic, self.mode).ok_or(EncodeError::UnknownEncoding {
            mnemonic: self.mnemonic,
            mode: self.mode,
        })?;
        validate_operand(self, widths)?;
        Ok(1 + OPCODE_TABLE[usize::from(opcode)].operand_len(widths))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedInstruction {
    pub opcode: u8,
    pub instruction: Instruction,
    pub len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    Truncated { expected: usize, found: usize },
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodeError {
    UnknownEncoding {
        mnemonic: Mnemonic,
        mode: AddressingMode,
    },
    OperandMismatch {
        mnemonic: Mnemonic,
        mode: AddressingMode,
        expected: &'static str,
        found: &'static str,
    },
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
pub fn decode(bytes: &[u8], widths: WidthState) -> Result<DecodedInstruction, DecodeError> {
    decode_bytes(bytes, widths)
}

#[doc(hidden)]
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
