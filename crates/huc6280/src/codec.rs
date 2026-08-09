//! Lossless byte codec for the declared HuC6280 profile.

use thiserror::Error;

use crate::instruction::{
    AddressingMode, InstructionForm, Mnemonic, canonical_opcode, instruction_form,
    opcode_is_documented,
};

/// Typed operand values. The instruction form determines how each value is
/// interpreted and serialized.
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
    /// Zero-page bit-test address followed by a relative branch displacement.
    ZeroPageRelative {
        /// Zero-page byte address tested by the instruction.
        address: u8,
        /// Signed displacement from the post-instruction address.
        displacement: i8,
    },
    /// Source, destination, and byte count for a HuC6280 block transfer.
    BlockTransfer {
        /// Initial 16-bit source address.
        source: u16,
        /// Initial 16-bit destination address.
        destination: u16,
        /// Number of bytes transferred; zero retains the hardware-defined value.
        length: u16,
    },
    /// Immediate test mask paired with a zero-page address.
    ImmediateZeroPage {
        /// Literal mask used by the test.
        immediate: u8,
        /// Zero-page base address, optionally indexed by the instruction form.
        address: u8,
    },
    /// Immediate test mask paired with an absolute address.
    ImmediateAbsolute {
        /// Literal mask used by the test.
        immediate: u8,
        /// Absolute base address, optionally indexed by the instruction form.
        address: u16,
    },
}

impl Operand {
    fn kind(self) -> &'static str {
        match self {
            Self::None => "no operand",
            Self::Byte(_) => "8-bit operand",
            Self::Word(_) => "16-bit operand",
            Self::Relative(_) => "8-bit relative operand",
            Self::ZeroPageRelative { .. } => "zero-page and relative operands",
            Self::BlockTransfer { .. } => "block-transfer operands",
            Self::ImmediateZeroPage { .. } => "immediate and zero-page operands",
            Self::ImmediateAbsolute { .. } => "immediate and absolute operands",
        }
    }
}

/// One typed HuC6280 instruction.
///
/// The exact documented opcode selector is retained for byte-exact lowering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Instruction {
    opcode: u8,
    operand: Operand,
}

impl Instruction {
    /// Construct a canonical documented encoding for a semantic form.
    pub fn new(
        mnemonic: Mnemonic,
        mode: AddressingMode,
        operand: Operand,
    ) -> Result<Self, EncodeError> {
        validate_bit_index(mnemonic)?;
        let form = InstructionForm::new(mnemonic, mode);
        let opcode =
            canonical_opcode(form).ok_or(EncodeError::UnknownEncoding { mnemonic, mode })?;
        let instruction = Self { opcode, operand };
        validate_operand(&instruction)?;
        Ok(instruction)
    }

    /// Construct the exact documented encoding selected by `opcode`.
    ///
    /// Unlike arbitrary byte emission, this validates the typed operand
    /// against the opcode's declared form before succeeding.
    pub fn from_opcode(opcode: u8, operand: Operand) -> Result<Self, EncodeError> {
        if !opcode_is_documented(opcode) {
            return Err(EncodeError::UndocumentedOpcode { opcode });
        }
        let instruction = Self { opcode, operand };
        validate_operand(&instruction)?;
        Ok(instruction)
    }

    /// Return the exact documented opcode selector retained by this instruction.
    pub const fn opcode(self) -> u8 {
        self.opcode
    }

    /// Return the semantic mnemonic/addressing form selected by the opcode.
    pub const fn form(self) -> InstructionForm {
        match instruction_form(self.opcode) {
            Some(form) => form,
            None => panic!("Instruction contains an undocumented HuC6280 opcode"),
        }
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

    /// Return whether the selector is assigned by Hudson's software manual.
    pub const fn opcode_is_documented(self) -> bool {
        opcode_is_documented(self.opcode)
    }

    /// Return the sequential code location immediately following this instruction.
    ///
    /// HuC6280 program-counter arithmetic wraps in the 16-bit address space.
    pub fn fallthrough_location(self, location: crate::CodeLocation) -> crate::CodeLocation {
        location.wrapping_add(self.encoded_len() as u16)
    }
}

/// A fail-closed reason that one instruction could not be decoded.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// The selector is one of the 22 slots left undefined by Hudson's manual.
    #[error("opcode {opcode:#04x} is undefined by the HuC6280 software manual")]
    UndocumentedOpcode {
        /// Rejected opcode selector.
        opcode: u8,
    },
    /// The source ends before the complete documented instruction is available.
    #[error("truncated opcode {opcode:#04x}: expected {expected} byte(s), found {found}")]
    Truncated {
        /// Opcode whose operands are incomplete, or zero when no opcode exists.
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
    /// The requested exact selector is not assigned by Hudson's manual.
    #[error("opcode {opcode:#04x} is undefined by the HuC6280 software manual")]
    UndocumentedOpcode {
        /// Rejected opcode selector.
        opcode: u8,
    },
    /// No documented selector implements the requested mnemonic and mode pair.
    #[error("no HuC6280 encoding for {mnemonic} {mode:?}")]
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
    /// An RMB, SMB, BBR, or BBS mnemonic carries an unencodable bit number.
    #[error("bit-indexed mnemonic has bit {bit}; expected 0..=7")]
    InvalidBitIndex {
        /// Rejected bit number.
        bit: u8,
    },
}

/// Decode one instruction from the start of a byte slice.
pub fn decode_bytes(bytes: &[u8]) -> Result<Instruction, DecodeError> {
    let Some(&opcode) = bytes.first() else {
        return Err(DecodeError::Truncated {
            opcode: 0,
            expected: 1,
            found: 0,
        });
    };
    let Some(form) = instruction_form(opcode) else {
        return Err(DecodeError::UndocumentedOpcode { opcode });
    };
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

/// Encode one typed instruction in HuC6280 byte order.
pub fn encode_bytes(instruction: &Instruction) -> Result<Vec<u8>, EncodeError> {
    validate_operand(instruction)?;
    let mut bytes = Vec::with_capacity(instruction.encoded_len());
    bytes.push(instruction.opcode);
    match instruction.operand {
        Operand::None => {}
        Operand::Byte(value) => bytes.push(value),
        Operand::Word(value) => bytes.extend_from_slice(&value.to_le_bytes()),
        Operand::Relative(displacement) => bytes.push(displacement as u8),
        Operand::ZeroPageRelative {
            address,
            displacement,
        } => {
            bytes.push(address);
            bytes.push(displacement as u8);
        }
        Operand::BlockTransfer {
            source,
            destination,
            length,
        } => {
            bytes.extend_from_slice(&source.to_le_bytes());
            bytes.extend_from_slice(&destination.to_le_bytes());
            bytes.extend_from_slice(&length.to_le_bytes());
        }
        Operand::ImmediateZeroPage { immediate, address } => {
            bytes.push(immediate);
            bytes.push(address);
        }
        Operand::ImmediateAbsolute { immediate, address } => {
            bytes.push(immediate);
            bytes.extend_from_slice(&address.to_le_bytes());
        }
    }
    Ok(bytes)
}

fn decode_operand(mode: AddressingMode, bytes: &[u8]) -> Operand {
    use AddressingMode::*;
    match mode {
        Implied | Accumulator => Operand::None,
        Immediate
        | ZeroPage
        | ZeroPageX
        | ZeroPageY
        | ZeroPageIndexedIndirectX
        | ZeroPageIndirectIndexedY
        | ZeroPageIndirect => Operand::Byte(bytes[0]),
        Absolute | AbsoluteX | AbsoluteY | AbsoluteIndirect | AbsoluteIndexedIndirectX => {
            Operand::Word(u16::from_le_bytes([bytes[0], bytes[1]]))
        }
        Relative => Operand::Relative(bytes[0] as i8),
        ZeroPageRelative => Operand::ZeroPageRelative {
            address: bytes[0],
            displacement: bytes[1] as i8,
        },
        BlockTransfer => Operand::BlockTransfer {
            source: u16::from_le_bytes([bytes[0], bytes[1]]),
            destination: u16::from_le_bytes([bytes[2], bytes[3]]),
            length: u16::from_le_bytes([bytes[4], bytes[5]]),
        },
        ImmediateZeroPage | ImmediateZeroPageX => Operand::ImmediateZeroPage {
            immediate: bytes[0],
            address: bytes[1],
        },
        ImmediateAbsolute | ImmediateAbsoluteX => Operand::ImmediateAbsolute {
            immediate: bytes[0],
            address: u16::from_le_bytes([bytes[1], bytes[2]]),
        },
    }
}

fn validate_bit_index(mnemonic: Mnemonic) -> Result<(), EncodeError> {
    let bit = match mnemonic {
        Mnemonic::Rmb { bit }
        | Mnemonic::Smb { bit }
        | Mnemonic::Bbr { bit }
        | Mnemonic::Bbs { bit } => bit,
        _ => return Ok(()),
    };
    if bit > 7 {
        return Err(EncodeError::InvalidBitIndex { bit });
    }
    Ok(())
}

fn validate_operand(instruction: &Instruction) -> Result<(), EncodeError> {
    let form = instruction.form();
    validate_bit_index(form.mnemonic)?;
    let valid = match form.mode {
        AddressingMode::Implied | AddressingMode::Accumulator => {
            matches!(instruction.operand, Operand::None)
        }
        AddressingMode::Immediate
        | AddressingMode::ZeroPage
        | AddressingMode::ZeroPageX
        | AddressingMode::ZeroPageY
        | AddressingMode::ZeroPageIndexedIndirectX
        | AddressingMode::ZeroPageIndirectIndexedY
        | AddressingMode::ZeroPageIndirect => matches!(instruction.operand, Operand::Byte(_)),
        AddressingMode::Absolute
        | AddressingMode::AbsoluteX
        | AddressingMode::AbsoluteY
        | AddressingMode::AbsoluteIndirect
        | AddressingMode::AbsoluteIndexedIndirectX => {
            matches!(instruction.operand, Operand::Word(_))
        }
        AddressingMode::Relative => matches!(instruction.operand, Operand::Relative(_)),
        AddressingMode::ZeroPageRelative => {
            matches!(instruction.operand, Operand::ZeroPageRelative { .. })
        }
        AddressingMode::BlockTransfer => {
            matches!(instruction.operand, Operand::BlockTransfer { .. })
        }
        AddressingMode::ImmediateZeroPage | AddressingMode::ImmediateZeroPageX => {
            matches!(instruction.operand, Operand::ImmediateZeroPage { .. })
        }
        AddressingMode::ImmediateAbsolute | AddressingMode::ImmediateAbsoluteX => {
            matches!(instruction.operand, Operand::ImmediateAbsolute { .. })
        }
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
        | AddressingMode::ZeroPageIndirectIndexedY
        | AddressingMode::ZeroPageIndirect => "8-bit operand",
        AddressingMode::Absolute
        | AddressingMode::AbsoluteX
        | AddressingMode::AbsoluteY
        | AddressingMode::AbsoluteIndirect
        | AddressingMode::AbsoluteIndexedIndirectX => "16-bit operand",
        AddressingMode::Relative => "8-bit relative operand",
        AddressingMode::ZeroPageRelative => "zero-page and relative operands",
        AddressingMode::BlockTransfer => "block-transfer operands",
        AddressingMode::ImmediateZeroPage | AddressingMode::ImmediateZeroPageX => {
            "immediate and zero-page operands"
        }
        AddressingMode::ImmediateAbsolute | AddressingMode::ImmediateAbsoluteX => {
            "immediate and absolute operands"
        }
    }
}
