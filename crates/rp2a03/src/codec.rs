//! Lossless byte codec for the RP2A03 selector profile.

use thiserror::Error;

use crate::instruction::{
    AddressingMode, ControlFlow, InstructionForm, Mnemonic, OPCODE_TABLE, canonical_opcode,
    opcode_is_documented,
};

/// Typed operand values. The opcode form determines their interpretation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    None,
    Byte(u8),
    Word(u16),
    Relative(i8),
}

impl Operand {
    fn kind(self) -> &'static str {
        match self {
            Self::None => "no operand",
            Self::Byte(_) => "8-bit operand",
            Self::Word(_) => "16-bit operand",
            Self::Relative(_) => "8-bit relative operand",
        }
    }
}

/// One typed RP2A03 instruction with its exact selector identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Instruction {
    opcode: u8,
    operand: Operand,
}

impl Instruction {
    /// Construct the canonical encoding for a semantic form.
    pub fn new(
        mnemonic: Mnemonic,
        mode: AddressingMode,
        operand: Operand,
    ) -> Result<Self, EncodeError> {
        let form = InstructionForm::new(mnemonic, mode);
        let opcode =
            canonical_opcode(form).ok_or(EncodeError::UnknownEncoding { mnemonic, mode })?;
        let instruction = Self { opcode, operand };
        validate_operand(&instruction)?;
        Ok(instruction)
    }

    /// Construct the exact hardware encoding selected by `opcode`.
    ///
    /// This accepts no raw byte sequence: the operand must match the opcode's
    /// declared typed form.
    pub fn from_opcode(opcode: u8, operand: Operand) -> Result<Self, EncodeError> {
        let instruction = Self { opcode, operand };
        validate_operand(&instruction)?;
        Ok(instruction)
    }

    pub const fn opcode(self) -> u8 {
        self.opcode
    }

    pub const fn form(self) -> InstructionForm {
        OPCODE_TABLE[self.opcode as usize]
    }

    pub const fn mnemonic(self) -> Mnemonic {
        self.form().mnemonic
    }

    pub const fn addressing_mode(self) -> AddressingMode {
        self.form().mode
    }

    pub const fn operand(self) -> Operand {
        self.operand
    }

    pub const fn encoded_len(self) -> usize {
        self.form().encoded_len()
    }

    pub const fn opcode_is_documented(self) -> bool {
        opcode_is_documented(self.opcode)
    }

    pub fn fallthrough_address(self, pc: u16) -> u16 {
        pc.wrapping_add(self.encoded_len() as u16)
    }

    /// Classify static control transfer at the supplied instruction address.
    pub fn control_flow(self, pc: u16) -> ControlFlow {
        use Mnemonic::*;

        let next = self.fallthrough_address(pc);
        match self.mnemonic() {
            Bcc | Bcs | Beq | Bmi | Bne | Bpl | Bvc | Bvs => ControlFlow::Branch {
                target: self.relative_target(next),
                fallthrough: Some(next),
            },
            Jmp => match self.addressing_mode() {
                AddressingMode::Absolute => ControlFlow::Jump {
                    target: Some(self.absolute_target()),
                },
                AddressingMode::AbsoluteIndirect => ControlFlow::Jump { target: None },
                _ => unreachable!("validated JMP has an unsupported addressing mode"),
            },
            Jsr => ControlFlow::Call {
                target: self.absolute_target(),
                return_address: next,
            },
            Rti | Rts => ControlFlow::Return,
            Brk => ControlFlow::Interrupt,
            Kil => ControlFlow::Stop,
            _ => ControlFlow::FallThrough { next },
        }
    }

    fn relative_target(self, next: u16) -> u16 {
        let Operand::Relative(displacement) = self.operand else {
            unreachable!("validated relative instruction has a non-relative operand")
        };
        next.wrapping_add(displacement as i16 as u16)
    }

    fn absolute_target(self) -> u16 {
        let Operand::Word(target) = self.operand else {
            unreachable!("validated absolute instruction has a non-word operand")
        };
        target
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DecodeError {
    #[error("instruction input is empty")]
    Empty,
    #[error("truncated opcode {opcode:#04x}: expected {expected} byte(s), found {found}")]
    Truncated {
        opcode: u8,
        expected: usize,
        found: usize,
    },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EncodeError {
    #[error("no RP2A03 encoding for {mnemonic} {mode:?}")]
    UnknownEncoding {
        mnemonic: Mnemonic,
        mode: AddressingMode,
    },
    #[error(
        "invalid operand for opcode {opcode:#04x} ({mnemonic} {mode:?}): expected {expected}, found {found}"
    )]
    OperandMismatch {
        opcode: u8,
        mnemonic: Mnemonic,
        mode: AddressingMode,
        expected: &'static str,
        found: &'static str,
    },
}

/// Decode one instruction from the start of a byte slice.
pub fn decode_bytes(bytes: &[u8]) -> Result<Instruction, DecodeError> {
    let Some(&opcode) = bytes.first() else {
        return Err(DecodeError::Empty);
    };
    let form = OPCODE_TABLE[opcode as usize];
    let expected = form.encoded_len();
    if bytes.len() < expected {
        return Err(DecodeError::Truncated {
            opcode,
            expected,
            found: bytes.len(),
        });
    }
    Ok(Instruction {
        opcode,
        operand: decode_operand(form.mode, &bytes[1..expected]),
    })
}

/// Lower one typed instruction to its exact RP2A03 byte sequence.
pub fn encode_bytes(instruction: &Instruction) -> Result<Vec<u8>, EncodeError> {
    validate_operand(instruction)?;
    let mut bytes = Vec::with_capacity(instruction.encoded_len());
    bytes.push(instruction.opcode);
    match instruction.operand {
        Operand::None => {}
        Operand::Byte(value) => bytes.push(value),
        Operand::Word(value) => bytes.extend_from_slice(&value.to_le_bytes()),
        Operand::Relative(displacement) => bytes.push(displacement as u8),
    }
    Ok(bytes)
}

fn decode_operand(mode: AddressingMode, bytes: &[u8]) -> Operand {
    match mode {
        AddressingMode::Implied | AddressingMode::Accumulator => Operand::None,
        AddressingMode::Immediate
        | AddressingMode::ZeroPage
        | AddressingMode::ZeroPageX
        | AddressingMode::ZeroPageY
        | AddressingMode::ZeroPageIndexedIndirectX
        | AddressingMode::ZeroPageIndirectIndexedY => Operand::Byte(bytes[0]),
        AddressingMode::Relative => Operand::Relative(bytes[0] as i8),
        AddressingMode::Absolute
        | AddressingMode::AbsoluteX
        | AddressingMode::AbsoluteY
        | AddressingMode::AbsoluteIndirect => {
            Operand::Word(u16::from_le_bytes([bytes[0], bytes[1]]))
        }
    }
}

fn validate_operand(instruction: &Instruction) -> Result<(), EncodeError> {
    let form = instruction.form();
    let valid = match form.mode {
        AddressingMode::Implied | AddressingMode::Accumulator => {
            matches!(instruction.operand, Operand::None)
        }
        AddressingMode::Immediate
        | AddressingMode::ZeroPage
        | AddressingMode::ZeroPageX
        | AddressingMode::ZeroPageY
        | AddressingMode::ZeroPageIndexedIndirectX
        | AddressingMode::ZeroPageIndirectIndexedY => {
            matches!(instruction.operand, Operand::Byte(_))
        }
        AddressingMode::Relative => matches!(instruction.operand, Operand::Relative(_)),
        AddressingMode::Absolute
        | AddressingMode::AbsoluteX
        | AddressingMode::AbsoluteY
        | AddressingMode::AbsoluteIndirect => matches!(instruction.operand, Operand::Word(_)),
    };
    if valid {
        return Ok(());
    }
    Err(EncodeError::OperandMismatch {
        opcode: instruction.opcode,
        mnemonic: form.mnemonic,
        mode: form.mode,
        expected: expected_operand(form.mode),
        found: instruction.operand.kind(),
    })
}

const fn expected_operand(mode: AddressingMode) -> &'static str {
    match mode {
        AddressingMode::Implied | AddressingMode::Accumulator => "no operand",
        AddressingMode::Immediate
        | AddressingMode::ZeroPage
        | AddressingMode::ZeroPageX
        | AddressingMode::ZeroPageY
        | AddressingMode::ZeroPageIndexedIndirectX
        | AddressingMode::ZeroPageIndirectIndexedY => "8-bit operand",
        AddressingMode::Relative => "8-bit relative operand",
        AddressingMode::Absolute
        | AddressingMode::AbsoluteX
        | AddressingMode::AbsoluteY
        | AddressingMode::AbsoluteIndirect => "16-bit operand",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_bytes(opcode: u8) -> Vec<u8> {
        let len = OPCODE_TABLE[opcode as usize].encoded_len();
        [opcode, 0x5A, 0xA5][..len].to_vec()
    }

    #[test]
    fn all_selectors_round_trip_exactly() {
        for opcode in 0..=u8::MAX {
            let bytes = sample_bytes(opcode);
            let instruction = decode_bytes(&bytes).unwrap();
            assert_eq!(instruction.opcode(), opcode);
            assert_eq!(encode_bytes(&instruction).unwrap(), bytes);
        }
    }

    #[test]
    fn documented_selector_denominator_is_151() {
        assert_eq!(
            (0..=u8::MAX).filter(|&op| opcode_is_documented(op)).count(),
            151
        );
        assert_eq!(InstructionForm::all().count(), 221);
        let lengths = (0..=u8::MAX).fold([0usize; 4], |mut counts, opcode| {
            counts[OPCODE_TABLE[opcode as usize].encoded_len()] += 1;
            counts
        });
        assert_eq!(lengths, [0, 47, 128, 81]);
    }

    #[test]
    fn unofficial_selector_families_keep_their_exact_identity() {
        let cases = [
            (0x02, Mnemonic::Kil, AddressingMode::Implied),
            (
                0x03,
                Mnemonic::Slo,
                AddressingMode::ZeroPageIndexedIndirectX,
            ),
            (0x4B, Mnemonic::Alr, AddressingMode::Immediate),
            (0x6B, Mnemonic::Arr, AddressingMode::Immediate),
            (0x8B, Mnemonic::Xaa, AddressingMode::Immediate),
            (
                0x93,
                Mnemonic::Ahx,
                AddressingMode::ZeroPageIndirectIndexedY,
            ),
            (0x9B, Mnemonic::Tas, AddressingMode::AbsoluteY),
            (0x9C, Mnemonic::Shy, AddressingMode::AbsoluteX),
            (0x9E, Mnemonic::Shx, AddressingMode::AbsoluteY),
            (0xAB, Mnemonic::Lax, AddressingMode::Immediate),
            (0xBB, Mnemonic::Las, AddressingMode::AbsoluteY),
            (0xCB, Mnemonic::Axs, AddressingMode::Immediate),
            (0xEB, Mnemonic::Sbc, AddressingMode::Immediate),
        ];
        for (opcode, mnemonic, mode) in cases {
            let form = OPCODE_TABLE[opcode as usize];
            assert_eq!(form, InstructionForm::new(mnemonic, mode));
            assert!(!opcode_is_documented(opcode));
        }
    }

    #[test]
    fn canonical_sbc_immediate_uses_documented_selector() {
        let instruction =
            Instruction::new(Mnemonic::Sbc, AddressingMode::Immediate, Operand::Byte(1)).unwrap();
        assert_eq!(instruction.opcode(), 0xE9);
        assert_eq!(
            Instruction::from_opcode(0xEB, Operand::Byte(1))
                .unwrap()
                .opcode(),
            0xEB
        );
    }

    #[test]
    fn operand_shape_is_checked() {
        let error = Instruction::from_opcode(0x4C, Operand::Byte(0)).unwrap_err();
        assert!(matches!(
            error,
            EncodeError::OperandMismatch { opcode: 0x4C, .. }
        ));
    }

    #[test]
    fn truncated_input_is_rejected() {
        assert!(matches!(
            decode_bytes(&[0x4C, 0x00]),
            Err(DecodeError::Truncated {
                opcode: 0x4C,
                expected: 3,
                found: 2
            })
        ));
    }

    #[test]
    fn control_flow_resolves_static_targets() {
        let branch = decode_bytes(&[0xD0, 0xFC]).unwrap();
        assert_eq!(
            branch.control_flow(0x8000),
            ControlFlow::Branch {
                target: 0x7FFE,
                fallthrough: Some(0x8002)
            }
        );
        let jump = decode_bytes(&[0x4C, 0x34, 0x12]).unwrap();
        assert_eq!(
            jump.control_flow(0x8000),
            ControlFlow::Jump {
                target: Some(0x1234)
            }
        );
        assert_eq!(
            decode_bytes(&[0x02]).unwrap().control_flow(0),
            ControlFlow::Stop
        );
    }
}
