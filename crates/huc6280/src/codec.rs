//! Lossless byte codec for the declared HuC6280 profile.

use thiserror::Error;

use crate::instruction::{
    AddressingMode, ControlFlow, InstructionForm, Mnemonic, OPCODE_TABLE, canonical_opcode,
    opcode_is_documented,
};

/// Typed operand values. The instruction form determines how each value is
/// interpreted and serialized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    None,
    Byte(u8),
    Word(u16),
    Relative(i8),
    ZeroPageRelative {
        address: u8,
        displacement: i8,
    },
    BlockTransfer {
        source: u16,
        destination: u16,
        length: u16,
    },
    ImmediateZeroPage {
        immediate: u8,
        address: u8,
    },
    ImmediateAbsolute {
        immediate: u8,
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
/// The exact opcode selector is retained so undocumented one-byte NOP aliases
/// can decode and re-encode without byte canonicalization.
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

    /// Construct the exact hardware encoding selected by `opcode`.
    ///
    /// Unlike arbitrary byte emission, this validates the typed operand
    /// against the opcode's declared form before succeeding.
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

    pub fn control_flow(self, pc: u16) -> ControlFlow {
        use Mnemonic::*;

        let next = self.fallthrough_address(pc);
        match self.mnemonic() {
            Bpl | Bmi | Bvc | Bvs | Bcc | Bcs | Bne | Beq | Bbr { .. } | Bbs { .. } => {
                ControlFlow::Branch {
                    target: self.relative_target(next),
                    fallthrough: Some(next),
                }
            }
            Bra => ControlFlow::Branch {
                target: self.relative_target(next),
                fallthrough: None,
            },
            Bsr => ControlFlow::Call {
                target: self.relative_target(next),
                return_address: next,
            },
            Jsr => ControlFlow::Call {
                target: self.absolute_target(),
                return_address: next,
            },
            Jmp => match self.addressing_mode() {
                AddressingMode::Absolute => ControlFlow::Jump {
                    target: Some(self.absolute_target()),
                },
                AddressingMode::AbsoluteIndirect | AddressingMode::AbsoluteIndexedIndirectX => {
                    ControlFlow::Jump { target: None }
                }
                _ => unreachable!("validated JMP has an unsupported addressing mode"),
            },
            Rti | Rts => ControlFlow::Return,
            Brk => ControlFlow::Interrupt,
            _ => ControlFlow::FallThrough { next },
        }
    }

    fn relative_target(self, next: u16) -> u16 {
        let displacement = match self.operand {
            Operand::Relative(displacement) | Operand::ZeroPageRelative { displacement, .. } => {
                displacement
            }
            _ => unreachable!("validated relative instruction has a non-relative operand"),
        };
        next.wrapping_add(displacement as i16 as u16)
    }

    fn absolute_target(self) -> u16 {
        match self.operand {
            Operand::Word(target) => target,
            _ => unreachable!("validated absolute control flow has a non-word operand"),
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DecodeError {
    #[error("truncated opcode {opcode:#04x}: expected {expected} byte(s), found {found}")]
    Truncated {
        opcode: u8,
        expected: usize,
        found: usize,
    },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EncodeError {
    #[error("no HuC6280 encoding for {mnemonic} {mode:?}")]
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
    #[error("bit-indexed mnemonic has bit {bit}; expected 0..=7")]
    InvalidBitIndex { bit: u8 },
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
