//! Lossless byte codec for the RP2A03 selector profile.

use thiserror::Error;

use crate::instruction::{
    AddressingMode, InstructionForm, Mnemonic, OPCODE_TABLE, canonical_opcode, opcode_is_documented,
};

/// Typed operand values. The opcode form determines their interpretation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    /// An implied or accumulator form has no encoded operand bytes.
    None,
    /// One unsigned byte used by immediate and zero-page forms.
    Byte(u8),
    /// One little-endian word used by absolute forms.
    Word(u16),
    /// One signed displacement relative to the post-instruction address.
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

    /// Return the exact physical opcode selector preserved by this instruction.
    pub const fn opcode(self) -> u8 {
        self.opcode
    }

    /// Return the semantic mnemonic/addressing form selected by the opcode.
    pub const fn form(self) -> InstructionForm {
        OPCODE_TABLE[self.opcode as usize]
    }

    /// Return the operation mnemonic selected by the opcode.
    pub const fn mnemonic(self) -> Mnemonic {
        self.form().mnemonic
    }

    /// Return the operand interpretation selected by the opcode.
    pub const fn addressing_mode(self) -> AddressingMode {
        self.form().mode
    }

    /// Return the typed operand value associated with the instruction.
    pub const fn operand(self) -> Operand {
        self.operand
    }

    /// Return the complete encoded length, including the opcode byte.
    pub const fn encoded_len(self) -> usize {
        self.form().encoded_len()
    }

    /// Return whether the exact selector belongs to the documented 151-opcode set.
    pub const fn opcode_is_documented(self) -> bool {
        opcode_is_documented(self.opcode)
    }

    /// Return the sequential code location immediately following this instruction.
    ///
    /// RP2A03 program-counter arithmetic wraps in the 16-bit address space.
    pub fn fallthrough_location(self, location: crate::CodeLocation) -> crate::CodeLocation {
        location.wrapping_add(self.encoded_len() as u16)
    }

    /// Classify static control transfer at the supplied instruction address.
    pub fn control_flow(self, location: crate::CodeLocation) -> crate::ControlFlow {
        crate::semantics::semantics(&self, location).control_flow
    }
}

/// A fail-closed reason that one instruction could not be decoded.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// The source slice contains no opcode byte.
    #[error("instruction input is empty")]
    Empty,
    /// The source ends before the operand required by the opcode is complete.
    #[error("truncated opcode {opcode:#04x}: expected {expected} byte(s), found {found}")]
    Truncated {
        /// Opcode whose operand bytes are incomplete.
        opcode: u8,
        /// Complete encoded length required for this opcode.
        expected: usize,
        /// Number of source bytes actually available.
        found: usize,
    },
}

/// A fail-closed reason that a typed instruction could not be encoded.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EncodeError {
    /// No physical selector implements the requested mnemonic and mode pair.
    #[error("no RP2A03 encoding for {mnemonic} {mode:?}")]
    UnknownEncoding {
        /// Requested operation mnemonic.
        mnemonic: Mnemonic,
        /// Requested operand interpretation.
        mode: AddressingMode,
    },
    /// The supplied operand type does not match the exact opcode form.
    #[error(
        "invalid operand for opcode {opcode:#04x} ({mnemonic} {mode:?}): expected {expected}, found {found}"
    )]
    OperandMismatch {
        /// Exact opcode selector being constructed or lowered.
        opcode: u8,
        /// Mnemonic selected by the opcode.
        mnemonic: Mnemonic,
        /// Addressing mode selected by the opcode.
        mode: AddressingMode,
        /// Human-readable operand category required by the form.
        expected: &'static str,
        /// Human-readable category of the supplied operand.
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
        use typed_isa_core::{ControlAction, ControlBoundary, ControlTarget};

        let branch = decode_bytes(&[0xD0, 0xFC]).unwrap();
        assert_eq!(
            branch.control_flow(0x8000),
            crate::ControlFlow {
                action: ControlAction::Transfer {
                    target: ControlTarget::Direct(0x7FFE),
                },
                fallthrough: Some(0x8002),
                delay_slot: None,
            }
        );
        let jump = decode_bytes(&[0x4C, 0x34, 0x12]).unwrap();
        assert_eq!(
            jump.control_flow(0x8000),
            crate::ControlFlow {
                action: ControlAction::Transfer {
                    target: ControlTarget::Direct(0x1234),
                },
                fallthrough: None,
                delay_slot: None,
            }
        );
        assert_eq!(
            decode_bytes(&[0x02]).unwrap().control_flow(0),
            crate::ControlFlow {
                action: ControlAction::Boundary(ControlBoundary::Stop {
                    kind: crate::Boundary::JammedUntilReset,
                }),
                fallthrough: None,
                delay_slot: None,
            }
        );
    }
}
