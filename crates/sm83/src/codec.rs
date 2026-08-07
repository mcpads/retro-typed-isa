use thiserror::Error;

use crate::instruction::{
    AluOperation, BitIndex, ByteOperand, Condition, IndirectAddress, Instruction, InstructionForm,
    Register16, RestartVector, RotateShiftOperation, StackRegister,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseOpcodeClass {
    Instruction,
    Prefix,
    Illegal,
}

/// Classify one selector in the 256-entry base opcode space.
pub const fn classify_base_opcode(opcode: u8) -> BaseOpcodeClass {
    match opcode {
        0xcb => BaseOpcodeClass::Prefix,
        0xd3 | 0xdb | 0xdd | 0xe3 | 0xe4 | 0xeb | 0xec | 0xed | 0xf4 | 0xfc | 0xfd => {
            BaseOpcodeClass::Illegal
        }
        _ => BaseOpcodeClass::Instruction,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DecodeError {
    #[error("truncated SM83 instruction: needed {needed} byte(s), got {got}")]
    UnexpectedEnd { needed: usize, got: usize },
    #[error("illegal SM83 base opcode {opcode:#04x}")]
    IllegalOpcode { opcode: u8 },
    #[error("SM83 STOP requires a zero padding byte, got {padding:#04x}")]
    InvalidStopPadding { padding: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodedInstruction {
    instruction: Instruction,
    length: usize,
}

impl DecodedInstruction {
    pub const fn instruction(&self) -> &Instruction {
        &self.instruction
    }

    pub const fn into_instruction(self) -> Instruction {
        self.instruction
    }

    pub const fn length(&self) -> usize {
        self.length
    }
}

fn read_u8(bytes: &[u8], offset: usize) -> Result<u8, DecodeError> {
    bytes
        .get(offset)
        .copied()
        .ok_or(DecodeError::UnexpectedEnd {
            needed: offset + 1,
            got: bytes.len(),
        })
}

fn read_u16_le(bytes: &[u8], offset: usize) -> Result<u16, DecodeError> {
    Ok(u16::from_le_bytes([
        read_u8(bytes, offset)?,
        read_u8(bytes, offset + 1)?,
    ]))
}

/// Decode one instruction from the start of `bytes`.
pub fn decode_bytes(bytes: &[u8]) -> Result<DecodedInstruction, DecodeError> {
    let opcode = read_u8(bytes, 0)?;
    if classify_base_opcode(opcode) == BaseOpcodeClass::Illegal {
        return Err(DecodeError::IllegalOpcode { opcode });
    }

    let (instruction, length) = match opcode {
        0xcb => (decode_cb(read_u8(bytes, 1)?), 2),
        0x00 => (Instruction::Nop, 1),
        opcode if opcode & 0xcf == 0x01 => (
            Instruction::Load16Immediate(
                Register16::from_field((opcode >> 4) & 0x03),
                read_u16_le(bytes, 1)?,
            ),
            3,
        ),
        opcode if opcode & 0xcf == 0x02 => (
            Instruction::LoadIndirectFromA(IndirectAddress::from_field((opcode >> 4) & 0x03)),
            1,
        ),
        opcode if opcode & 0xcf == 0x03 => (
            Instruction::Increment16(Register16::from_field((opcode >> 4) & 0x03)),
            1,
        ),
        opcode if opcode & 0xc7 == 0x04 => (
            Instruction::Increment8(ByteOperand::from_field((opcode >> 3) & 0x07)),
            1,
        ),
        opcode if opcode & 0xc7 == 0x05 => (
            Instruction::Decrement8(ByteOperand::from_field((opcode >> 3) & 0x07)),
            1,
        ),
        opcode if opcode & 0xc7 == 0x06 => (
            Instruction::Load8Immediate(
                ByteOperand::from_field((opcode >> 3) & 0x07),
                read_u8(bytes, 1)?,
            ),
            2,
        ),
        0x07 => (Instruction::RotateLeftCircularA, 1),
        0x08 => (Instruction::LoadAddressFromSp(read_u16_le(bytes, 1)?), 3),
        opcode if opcode & 0xcf == 0x09 => (
            Instruction::AddHl(Register16::from_field((opcode >> 4) & 0x03)),
            1,
        ),
        opcode if opcode & 0xcf == 0x0a => (
            Instruction::LoadAFromIndirect(IndirectAddress::from_field((opcode >> 4) & 0x03)),
            1,
        ),
        opcode if opcode & 0xcf == 0x0b => (
            Instruction::Decrement16(Register16::from_field((opcode >> 4) & 0x03)),
            1,
        ),
        0x0f => (Instruction::RotateRightCircularA, 1),
        0x10 => {
            let padding = read_u8(bytes, 1)?;
            if padding != 0 {
                return Err(DecodeError::InvalidStopPadding { padding });
            }
            (Instruction::Stop, 2)
        }
        0x17 => (Instruction::RotateLeftA, 1),
        0x18 => (Instruction::JumpRelative(read_u8(bytes, 1)? as i8), 2),
        0x1f => (Instruction::RotateRightA, 1),
        opcode if opcode & 0xe7 == 0x20 => (
            Instruction::JumpRelativeIf(
                Condition::from_field((opcode >> 3) & 0x03),
                read_u8(bytes, 1)? as i8,
            ),
            2,
        ),
        0x27 => (Instruction::DecimalAdjustA, 1),
        0x2f => (Instruction::ComplementA, 1),
        0x37 => (Instruction::SetCarryFlag, 1),
        0x3f => (Instruction::ComplementCarryFlag, 1),
        0x40..=0x7f if opcode != 0x76 => (
            Instruction::Load8(
                ByteOperand::from_field((opcode >> 3) & 0x07),
                ByteOperand::from_field(opcode & 0x07),
            ),
            1,
        ),
        0x76 => (Instruction::Halt, 1),
        0x80..=0xbf => (
            Instruction::Alu8(
                AluOperation::from_field((opcode >> 3) & 0x07),
                ByteOperand::from_field(opcode & 0x07),
            ),
            1,
        ),
        opcode if opcode & 0xe7 == 0xc0 => (
            Instruction::ReturnIf(Condition::from_field((opcode >> 3) & 0x03)),
            1,
        ),
        opcode if opcode & 0xcf == 0xc1 => (
            Instruction::Pop(StackRegister::from_field((opcode >> 4) & 0x03)),
            1,
        ),
        opcode if opcode & 0xe7 == 0xc2 => (
            Instruction::JumpIf(
                Condition::from_field((opcode >> 3) & 0x03),
                read_u16_le(bytes, 1)?,
            ),
            3,
        ),
        0xc3 => (Instruction::Jump(read_u16_le(bytes, 1)?), 3),
        opcode if opcode & 0xe7 == 0xc4 => (
            Instruction::CallIf(
                Condition::from_field((opcode >> 3) & 0x03),
                read_u16_le(bytes, 1)?,
            ),
            3,
        ),
        opcode if opcode & 0xcf == 0xc5 => (
            Instruction::Push(StackRegister::from_field((opcode >> 4) & 0x03)),
            1,
        ),
        opcode if opcode & 0xc7 == 0xc6 => (
            Instruction::AluImmediate(
                AluOperation::from_field((opcode >> 3) & 0x07),
                read_u8(bytes, 1)?,
            ),
            2,
        ),
        opcode if opcode & 0xc7 == 0xc7 => (
            Instruction::Restart(RestartVector::from_field((opcode >> 3) & 0x07)),
            1,
        ),
        0xc9 => (Instruction::Return, 1),
        0xcd => (Instruction::Call(read_u16_le(bytes, 1)?), 3),
        0xd9 => (Instruction::ReturnFromInterrupt, 1),
        0xe0 => (Instruction::LoadHighAddressFromA(read_u8(bytes, 1)?), 2),
        0xe2 => (Instruction::LoadHighAddressCFromA, 1),
        0xe8 => (Instruction::AddSp(read_u8(bytes, 1)? as i8), 2),
        0xe9 => (Instruction::JumpHl, 1),
        0xea => (Instruction::LoadAddressFromA(read_u16_le(bytes, 1)?), 3),
        0xf0 => (Instruction::LoadAFromHighAddress(read_u8(bytes, 1)?), 2),
        0xf2 => (Instruction::LoadAFromHighAddressC, 1),
        0xf3 => (Instruction::DisableInterrupts, 1),
        0xf8 => (Instruction::LoadHlFromSpPlus(read_u8(bytes, 1)? as i8), 2),
        0xf9 => (Instruction::LoadSpFromHl, 1),
        0xfa => (Instruction::LoadAFromAddress(read_u16_le(bytes, 1)?), 3),
        0xfb => (Instruction::EnableInterrupts, 1),
        _ => unreachable!("all legal SM83 base selectors are classified"),
    };

    Ok(DecodedInstruction {
        instruction,
        length,
    })
}

fn decode_cb(opcode: u8) -> Instruction {
    let group = opcode >> 6;
    let operation = (opcode >> 3) & 0x07;
    let operand = ByteOperand::from_field(opcode & 0x07);
    match group {
        0 => Instruction::RotateShift(RotateShiftOperation::from_field(operation), operand),
        1 => Instruction::Bit(BitIndex::from_field(operation), operand),
        2 => Instruction::ResetBit(BitIndex::from_field(operation), operand),
        3 => Instruction::SetBit(BitIndex::from_field(operation), operand),
        _ => unreachable!(),
    }
}

/// Lower one typed SM83 instruction to canonical machine code.
pub fn encode_bytes(instruction: &Instruction) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(instruction.encoded_len());
    match *instruction {
        Instruction::Nop => bytes.push(0x00),
        Instruction::Load16Immediate(register, value) => {
            bytes.push(0x01 | register.field() << 4);
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        Instruction::LoadIndirectFromA(address) => bytes.push(0x02 | address.field() << 4),
        Instruction::LoadAFromIndirect(address) => bytes.push(0x0a | address.field() << 4),
        Instruction::Increment16(register) => bytes.push(0x03 | register.field() << 4),
        Instruction::Decrement16(register) => bytes.push(0x0b | register.field() << 4),
        Instruction::Increment8(operand) => bytes.push(0x04 | operand.field() << 3),
        Instruction::Decrement8(operand) => bytes.push(0x05 | operand.field() << 3),
        Instruction::Load8Immediate(operand, value) => {
            bytes.extend([0x06 | operand.field() << 3, value]);
        }
        Instruction::Load8(destination, source) => {
            bytes.push(0x40 | destination.field() << 3 | source.field());
        }
        Instruction::LoadAddressFromSp(address) => {
            bytes.push(0x08);
            bytes.extend_from_slice(&address.to_le_bytes());
        }
        Instruction::AddHl(register) => bytes.push(0x09 | register.field() << 4),
        Instruction::RotateLeftCircularA => bytes.push(0x07),
        Instruction::RotateRightCircularA => bytes.push(0x0f),
        Instruction::RotateLeftA => bytes.push(0x17),
        Instruction::RotateRightA => bytes.push(0x1f),
        Instruction::Stop => bytes.extend([0x10, 0x00]),
        Instruction::JumpRelative(displacement) => bytes.extend([0x18, displacement as u8]),
        Instruction::JumpRelativeIf(condition, displacement) => {
            bytes.extend([0x20 | condition.field() << 3, displacement as u8]);
        }
        Instruction::DecimalAdjustA => bytes.push(0x27),
        Instruction::ComplementA => bytes.push(0x2f),
        Instruction::SetCarryFlag => bytes.push(0x37),
        Instruction::ComplementCarryFlag => bytes.push(0x3f),
        Instruction::Halt => bytes.push(0x76),
        Instruction::Alu8(operation, operand) => {
            bytes.push(0x80 | operation.field() << 3 | operand.field());
        }
        Instruction::ReturnIf(condition) => bytes.push(0xc0 | condition.field() << 3),
        Instruction::Pop(register) => bytes.push(0xc1 | register.field() << 4),
        Instruction::JumpIf(condition, address) => {
            bytes.push(0xc2 | condition.field() << 3);
            bytes.extend_from_slice(&address.to_le_bytes());
        }
        Instruction::Jump(address) => {
            bytes.push(0xc3);
            bytes.extend_from_slice(&address.to_le_bytes());
        }
        Instruction::CallIf(condition, address) => {
            bytes.push(0xc4 | condition.field() << 3);
            bytes.extend_from_slice(&address.to_le_bytes());
        }
        Instruction::Push(register) => bytes.push(0xc5 | register.field() << 4),
        Instruction::AluImmediate(operation, value) => {
            bytes.extend([0xc6 | operation.field() << 3, value]);
        }
        Instruction::Restart(vector) => bytes.push(0xc7 | vector.field() << 3),
        Instruction::Return => bytes.push(0xc9),
        Instruction::ReturnFromInterrupt => bytes.push(0xd9),
        Instruction::Call(address) => {
            bytes.push(0xcd);
            bytes.extend_from_slice(&address.to_le_bytes());
        }
        Instruction::LoadHighAddressFromA(offset) => bytes.extend([0xe0, offset]),
        Instruction::LoadHighAddressCFromA => bytes.push(0xe2),
        Instruction::AddSp(displacement) => bytes.extend([0xe8, displacement as u8]),
        Instruction::JumpHl => bytes.push(0xe9),
        Instruction::LoadAddressFromA(address) => {
            bytes.push(0xea);
            bytes.extend_from_slice(&address.to_le_bytes());
        }
        Instruction::LoadAFromHighAddress(offset) => bytes.extend([0xf0, offset]),
        Instruction::LoadAFromHighAddressC => bytes.push(0xf2),
        Instruction::DisableInterrupts => bytes.push(0xf3),
        Instruction::LoadHlFromSpPlus(displacement) => bytes.extend([0xf8, displacement as u8]),
        Instruction::LoadSpFromHl => bytes.push(0xf9),
        Instruction::LoadAFromAddress(address) => {
            bytes.push(0xfa);
            bytes.extend_from_slice(&address.to_le_bytes());
        }
        Instruction::EnableInterrupts => bytes.push(0xfb),
        Instruction::RotateShift(operation, operand) => {
            bytes.extend([0xcb, operation.field() << 3 | operand.field()]);
        }
        Instruction::Bit(bit, operand) => {
            bytes.extend([0xcb, 0x40 | bit.value() << 3 | operand.field()]);
        }
        Instruction::ResetBit(bit, operand) => {
            bytes.extend([0xcb, 0x80 | bit.value() << 3 | operand.field()]);
        }
        Instruction::SetBit(bit, operand) => {
            bytes.extend([0xcb, 0xc0 | bit.value() << 3 | operand.field()]);
        }
    }
    bytes
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalInstruction {
    instruction: Instruction,
    bytes: Vec<u8>,
}

impl CanonicalInstruction {
    pub const fn instruction(&self) -> &Instruction {
        &self.instruction
    }

    pub const fn form(&self) -> InstructionForm {
        self.instruction.form()
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Return one canonical typed vector for every valid base and CB selector.
pub fn canonical_instructions() -> Vec<CanonicalInstruction> {
    let mut result = Vec::with_capacity(500);
    for opcode in u8::MIN..=u8::MAX {
        if classify_base_opcode(opcode) != BaseOpcodeClass::Instruction {
            continue;
        }
        let probe = [opcode, 0, 0];
        let decoded = decode_bytes(&probe).expect("classified base selector must decode");
        let instruction = decoded.into_instruction();
        result.push(CanonicalInstruction {
            bytes: encode_bytes(&instruction),
            instruction,
        });
    }
    for opcode in u8::MIN..=u8::MAX {
        let instruction = decode_bytes(&[0xcb, opcode])
            .expect("every CB selector must decode")
            .into_instruction();
        result.push(CanonicalInstruction {
            bytes: encode_bytes(&instruction),
            instruction,
        });
    }
    result
}
