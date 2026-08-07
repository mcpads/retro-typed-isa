/// An 8-bit register or the byte at address HL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ByteOperand {
    B,
    C,
    D,
    E,
    H,
    L,
    IndirectHl,
    A,
}

impl ByteOperand {
    pub(crate) const fn from_field(field: u8) -> Self {
        match field {
            0 => Self::B,
            1 => Self::C,
            2 => Self::D,
            3 => Self::E,
            4 => Self::H,
            5 => Self::L,
            6 => Self::IndirectHl,
            7 => Self::A,
            _ => unreachable!(),
        }
    }

    pub const fn field(self) -> u8 {
        match self {
            Self::B => 0,
            Self::C => 1,
            Self::D => 2,
            Self::E => 3,
            Self::H => 4,
            Self::L => 5,
            Self::IndirectHl => 6,
            Self::A => 7,
        }
    }
}

/// A 16-bit register selected by the SM83 `rp` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Register16 {
    Bc,
    De,
    Hl,
    Sp,
}

impl Register16 {
    pub(crate) const fn from_field(field: u8) -> Self {
        match field {
            0 => Self::Bc,
            1 => Self::De,
            2 => Self::Hl,
            3 => Self::Sp,
            _ => unreachable!(),
        }
    }

    pub const fn field(self) -> u8 {
        match self {
            Self::Bc => 0,
            Self::De => 1,
            Self::Hl => 2,
            Self::Sp => 3,
        }
    }
}

/// A 16-bit register selected by PUSH and POP instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StackRegister {
    Bc,
    De,
    Hl,
    Af,
}

impl StackRegister {
    pub(crate) const fn from_field(field: u8) -> Self {
        match field {
            0 => Self::Bc,
            1 => Self::De,
            2 => Self::Hl,
            3 => Self::Af,
            _ => unreachable!(),
        }
    }

    pub const fn field(self) -> u8 {
        match self {
            Self::Bc => 0,
            Self::De => 1,
            Self::Hl => 2,
            Self::Af => 3,
        }
    }
}

/// The four implicit address updates used by accumulator loads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IndirectAddress {
    Bc,
    De,
    HlIncrement,
    HlDecrement,
}

impl IndirectAddress {
    pub(crate) const fn from_field(field: u8) -> Self {
        match field {
            0 => Self::Bc,
            1 => Self::De,
            2 => Self::HlIncrement,
            3 => Self::HlDecrement,
            _ => unreachable!(),
        }
    }

    pub const fn field(self) -> u8 {
        match self {
            Self::Bc => 0,
            Self::De => 1,
            Self::HlIncrement => 2,
            Self::HlDecrement => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Condition {
    Nz,
    Z,
    Nc,
    C,
}

impl Condition {
    pub(crate) const fn from_field(field: u8) -> Self {
        match field {
            0 => Self::Nz,
            1 => Self::Z,
            2 => Self::Nc,
            3 => Self::C,
            _ => unreachable!(),
        }
    }

    pub const fn field(self) -> u8 {
        match self {
            Self::Nz => 0,
            Self::Z => 1,
            Self::Nc => 2,
            Self::C => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AluOperation {
    Add,
    Adc,
    Sub,
    Sbc,
    And,
    Xor,
    Or,
    Cp,
}

impl AluOperation {
    pub(crate) const fn from_field(field: u8) -> Self {
        match field {
            0 => Self::Add,
            1 => Self::Adc,
            2 => Self::Sub,
            3 => Self::Sbc,
            4 => Self::And,
            5 => Self::Xor,
            6 => Self::Or,
            7 => Self::Cp,
            _ => unreachable!(),
        }
    }

    pub const fn field(self) -> u8 {
        match self {
            Self::Add => 0,
            Self::Adc => 1,
            Self::Sub => 2,
            Self::Sbc => 3,
            Self::And => 4,
            Self::Xor => 5,
            Self::Or => 6,
            Self::Cp => 7,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RotateShiftOperation {
    Rlc,
    Rrc,
    Rl,
    Rr,
    Sla,
    Sra,
    Swap,
    Srl,
}

impl RotateShiftOperation {
    pub(crate) const fn from_field(field: u8) -> Self {
        match field {
            0 => Self::Rlc,
            1 => Self::Rrc,
            2 => Self::Rl,
            3 => Self::Rr,
            4 => Self::Sla,
            5 => Self::Sra,
            6 => Self::Swap,
            7 => Self::Srl,
            _ => unreachable!(),
        }
    }

    pub const fn field(self) -> u8 {
        match self {
            Self::Rlc => 0,
            Self::Rrc => 1,
            Self::Rl => 2,
            Self::Rr => 3,
            Self::Sla => 4,
            Self::Sra => 5,
            Self::Swap => 6,
            Self::Srl => 7,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum BitIndex {
    B0 = 0,
    B1 = 1,
    B2 = 2,
    B3 = 3,
    B4 = 4,
    B5 = 5,
    B6 = 6,
    B7 = 7,
}

impl BitIndex {
    pub(crate) const fn from_field(field: u8) -> Self {
        match field {
            0 => Self::B0,
            1 => Self::B1,
            2 => Self::B2,
            3 => Self::B3,
            4 => Self::B4,
            5 => Self::B5,
            6 => Self::B6,
            7 => Self::B7,
            _ => unreachable!(),
        }
    }

    pub const fn value(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum RestartVector {
    H00 = 0x00,
    H08 = 0x08,
    H10 = 0x10,
    H18 = 0x18,
    H20 = 0x20,
    H28 = 0x28,
    H30 = 0x30,
    H38 = 0x38,
}

impl RestartVector {
    pub(crate) const fn from_field(field: u8) -> Self {
        match field {
            0 => Self::H00,
            1 => Self::H08,
            2 => Self::H10,
            3 => Self::H18,
            4 => Self::H20,
            5 => Self::H28,
            6 => Self::H30,
            7 => Self::H38,
            _ => unreachable!(),
        }
    }

    pub const fn value(self) -> u8 {
        self as u8
    }

    pub const fn field(self) -> u8 {
        self.value() >> 3
    }
}

/// Every encodable SM83 instruction with invalid operand combinations excluded by type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Instruction {
    Nop,
    Load16Immediate(Register16, u16),
    LoadIndirectFromA(IndirectAddress),
    LoadAFromIndirect(IndirectAddress),
    Increment16(Register16),
    Decrement16(Register16),
    Increment8(ByteOperand),
    Decrement8(ByteOperand),
    Load8Immediate(ByteOperand, u8),
    Load8(ByteOperand, ByteOperand),
    LoadAddressFromSp(u16),
    AddHl(Register16),
    RotateLeftCircularA,
    RotateRightCircularA,
    RotateLeftA,
    RotateRightA,
    Stop,
    JumpRelative(i8),
    JumpRelativeIf(Condition, i8),
    DecimalAdjustA,
    ComplementA,
    SetCarryFlag,
    ComplementCarryFlag,
    Halt,
    Alu8(AluOperation, ByteOperand),
    ReturnIf(Condition),
    Pop(StackRegister),
    JumpIf(Condition, u16),
    Jump(u16),
    CallIf(Condition, u16),
    Push(StackRegister),
    AluImmediate(AluOperation, u8),
    Restart(RestartVector),
    Return,
    ReturnFromInterrupt,
    Call(u16),
    LoadHighAddressFromA(u8),
    LoadHighAddressCFromA,
    AddSp(i8),
    JumpHl,
    LoadAddressFromA(u16),
    LoadAFromHighAddress(u8),
    LoadAFromHighAddressC,
    DisableInterrupts,
    LoadHlFromSpPlus(i8),
    LoadSpFromHl,
    LoadAFromAddress(u16),
    EnableInterrupts,
    RotateShift(RotateShiftOperation, ByteOperand),
    Bit(BitIndex, ByteOperand),
    ResetBit(BitIndex, ByteOperand),
    SetBit(BitIndex, ByteOperand),
}

/// Semantic instruction identity with literal values removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InstructionForm {
    Nop,
    Load16Immediate(Register16),
    LoadIndirectFromA(IndirectAddress),
    LoadAFromIndirect(IndirectAddress),
    Increment16(Register16),
    Decrement16(Register16),
    Increment8(ByteOperand),
    Decrement8(ByteOperand),
    Load8Immediate(ByteOperand),
    Load8(ByteOperand, ByteOperand),
    LoadAddressFromSp,
    AddHl(Register16),
    RotateLeftCircularA,
    RotateRightCircularA,
    RotateLeftA,
    RotateRightA,
    Stop,
    JumpRelative,
    JumpRelativeIf(Condition),
    DecimalAdjustA,
    ComplementA,
    SetCarryFlag,
    ComplementCarryFlag,
    Halt,
    Alu8(AluOperation, ByteOperand),
    ReturnIf(Condition),
    Pop(StackRegister),
    JumpIf(Condition),
    Jump,
    CallIf(Condition),
    Push(StackRegister),
    AluImmediate(AluOperation),
    Restart(RestartVector),
    Return,
    ReturnFromInterrupt,
    Call,
    LoadHighAddressFromA,
    LoadHighAddressCFromA,
    AddSp,
    JumpHl,
    LoadAddressFromA,
    LoadAFromHighAddress,
    LoadAFromHighAddressC,
    DisableInterrupts,
    LoadHlFromSpPlus,
    LoadSpFromHl,
    LoadAFromAddress,
    EnableInterrupts,
    RotateShift(RotateShiftOperation, ByteOperand),
    Bit(BitIndex, ByteOperand),
    ResetBit(BitIndex, ByteOperand),
    SetBit(BitIndex, ByteOperand),
}

impl Instruction {
    pub const fn form(self) -> InstructionForm {
        match self {
            Self::Nop => InstructionForm::Nop,
            Self::Load16Immediate(register, _) => InstructionForm::Load16Immediate(register),
            Self::LoadIndirectFromA(address) => InstructionForm::LoadIndirectFromA(address),
            Self::LoadAFromIndirect(address) => InstructionForm::LoadAFromIndirect(address),
            Self::Increment16(register) => InstructionForm::Increment16(register),
            Self::Decrement16(register) => InstructionForm::Decrement16(register),
            Self::Increment8(operand) => InstructionForm::Increment8(operand),
            Self::Decrement8(operand) => InstructionForm::Decrement8(operand),
            Self::Load8Immediate(operand, _) => InstructionForm::Load8Immediate(operand),
            Self::Load8(destination, source) => InstructionForm::Load8(destination, source),
            Self::LoadAddressFromSp(_) => InstructionForm::LoadAddressFromSp,
            Self::AddHl(register) => InstructionForm::AddHl(register),
            Self::RotateLeftCircularA => InstructionForm::RotateLeftCircularA,
            Self::RotateRightCircularA => InstructionForm::RotateRightCircularA,
            Self::RotateLeftA => InstructionForm::RotateLeftA,
            Self::RotateRightA => InstructionForm::RotateRightA,
            Self::Stop => InstructionForm::Stop,
            Self::JumpRelative(_) => InstructionForm::JumpRelative,
            Self::JumpRelativeIf(condition, _) => InstructionForm::JumpRelativeIf(condition),
            Self::DecimalAdjustA => InstructionForm::DecimalAdjustA,
            Self::ComplementA => InstructionForm::ComplementA,
            Self::SetCarryFlag => InstructionForm::SetCarryFlag,
            Self::ComplementCarryFlag => InstructionForm::ComplementCarryFlag,
            Self::Halt => InstructionForm::Halt,
            Self::Alu8(operation, operand) => InstructionForm::Alu8(operation, operand),
            Self::ReturnIf(condition) => InstructionForm::ReturnIf(condition),
            Self::Pop(register) => InstructionForm::Pop(register),
            Self::JumpIf(condition, _) => InstructionForm::JumpIf(condition),
            Self::Jump(_) => InstructionForm::Jump,
            Self::CallIf(condition, _) => InstructionForm::CallIf(condition),
            Self::Push(register) => InstructionForm::Push(register),
            Self::AluImmediate(operation, _) => InstructionForm::AluImmediate(operation),
            Self::Restart(vector) => InstructionForm::Restart(vector),
            Self::Return => InstructionForm::Return,
            Self::ReturnFromInterrupt => InstructionForm::ReturnFromInterrupt,
            Self::Call(_) => InstructionForm::Call,
            Self::LoadHighAddressFromA(_) => InstructionForm::LoadHighAddressFromA,
            Self::LoadHighAddressCFromA => InstructionForm::LoadHighAddressCFromA,
            Self::AddSp(_) => InstructionForm::AddSp,
            Self::JumpHl => InstructionForm::JumpHl,
            Self::LoadAddressFromA(_) => InstructionForm::LoadAddressFromA,
            Self::LoadAFromHighAddress(_) => InstructionForm::LoadAFromHighAddress,
            Self::LoadAFromHighAddressC => InstructionForm::LoadAFromHighAddressC,
            Self::DisableInterrupts => InstructionForm::DisableInterrupts,
            Self::LoadHlFromSpPlus(_) => InstructionForm::LoadHlFromSpPlus,
            Self::LoadSpFromHl => InstructionForm::LoadSpFromHl,
            Self::LoadAFromAddress(_) => InstructionForm::LoadAFromAddress,
            Self::EnableInterrupts => InstructionForm::EnableInterrupts,
            Self::RotateShift(operation, operand) => {
                InstructionForm::RotateShift(operation, operand)
            }
            Self::Bit(bit, operand) => InstructionForm::Bit(bit, operand),
            Self::ResetBit(bit, operand) => InstructionForm::ResetBit(bit, operand),
            Self::SetBit(bit, operand) => InstructionForm::SetBit(bit, operand),
        }
    }

    pub const fn encoded_len(self) -> usize {
        match self {
            Self::Load16Immediate(_, _)
            | Self::LoadAddressFromSp(_)
            | Self::JumpIf(_, _)
            | Self::Jump(_)
            | Self::CallIf(_, _)
            | Self::Call(_)
            | Self::LoadAddressFromA(_)
            | Self::LoadAFromAddress(_) => 3,
            Self::Stop
            | Self::JumpRelative(_)
            | Self::JumpRelativeIf(_, _)
            | Self::Load8Immediate(_, _)
            | Self::AluImmediate(_, _)
            | Self::LoadHighAddressFromA(_)
            | Self::AddSp(_)
            | Self::LoadAFromHighAddress(_)
            | Self::LoadHlFromSpPlus(_)
            | Self::RotateShift(_, _)
            | Self::Bit(_, _)
            | Self::ResetBit(_, _)
            | Self::SetBit(_, _) => 2,
            _ => 1,
        }
    }
}
