use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Register(u8);

impl Register {
    pub const ZERO: Self = Self(0);
    pub const LINK: Self = Self(31);

    pub const fn new(index: u8) -> Result<Self, RegisterOutOfRange> {
        if index < 32 {
            Ok(Self(index))
        } else {
            Err(RegisterOutOfRange { index })
        }
    }

    pub const fn index(self) -> u8 {
        self.0
    }

    pub(crate) const fn from_field(field: u16) -> Self {
        Self(field as u8)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("V810 general register index {index} is outside 0..31")]
pub struct RegisterOutOfRange {
    pub index: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum SystemRegister {
    Eipc = 0,
    Eipsw = 1,
    Fepc = 2,
    Fepsw = 3,
    Ecr = 4,
    Psw = 5,
    Pir = 6,
    Tkcw = 7,
    Chcw = 24,
    Adtre = 25,
}

impl SystemRegister {
    pub const fn id(self) -> u8 {
        self as u8
    }

    pub const fn is_writable(self) -> bool {
        !matches!(self, Self::Ecr | Self::Pir | Self::Tkcw)
    }

    pub(crate) const fn from_id(id: u8) -> Option<Self> {
        Some(match id {
            0 => Self::Eipc,
            1 => Self::Eipsw,
            2 => Self::Fepc,
            3 => Self::Fepsw,
            4 => Self::Ecr,
            5 => Self::Psw,
            6 => Self::Pir,
            7 => Self::Tkcw,
            24 => Self::Chcw,
            25 => Self::Adtre,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum RegisterOperation {
    Move = 0x00,
    Add = 0x01,
    Subtract = 0x02,
    Compare = 0x03,
    ShiftLeft = 0x04,
    ShiftRight = 0x05,
    ShiftRightArithmetic = 0x07,
    Multiply = 0x08,
    Divide = 0x09,
    MultiplyUnsigned = 0x0a,
    DivideUnsigned = 0x0b,
    Or = 0x0c,
    And = 0x0d,
    Xor = 0x0e,
    Not = 0x0f,
}

impl RegisterOperation {
    pub(crate) const fn from_opcode(opcode: u8) -> Option<Self> {
        Some(match opcode {
            0x00 => Self::Move,
            0x01 => Self::Add,
            0x02 => Self::Subtract,
            0x03 => Self::Compare,
            0x04 => Self::ShiftLeft,
            0x05 => Self::ShiftRight,
            0x07 => Self::ShiftRightArithmetic,
            0x08 => Self::Multiply,
            0x09 => Self::Divide,
            0x0a => Self::MultiplyUnsigned,
            0x0b => Self::DivideUnsigned,
            0x0c => Self::Or,
            0x0d => Self::And,
            0x0e => Self::Xor,
            0x0f => Self::Not,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ImmediateOperation {
    Move = 0x10,
    Add = 0x11,
    Compare = 0x13,
}

impl ImmediateOperation {
    pub(crate) const fn from_opcode(opcode: u8) -> Option<Self> {
        Some(match opcode {
            0x10 => Self::Move,
            0x11 => Self::Add,
            0x13 => Self::Compare,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShiftOperation {
    Left,
    Right,
    RightArithmetic,
}

impl ShiftOperation {
    pub(crate) const fn immediate_opcode(self) -> u8 {
        match self {
            Self::Left => 0x14,
            Self::Right => 0x15,
            Self::RightArithmetic => 0x17,
        }
    }

    pub(crate) const fn from_immediate_opcode(opcode: u8) -> Option<Self> {
        Some(match opcode {
            0x14 => Self::Left,
            0x15 => Self::Right,
            0x17 => Self::RightArithmetic,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Condition {
    Overflow = 0x0,
    CarryOrLower = 0x1,
    ZeroOrEqual = 0x2,
    NotHigher = 0x3,
    Negative = 0x4,
    Always = 0x5,
    LessThan = 0x6,
    LessOrEqual = 0x7,
    NoOverflow = 0x8,
    NoCarryOrNotLower = 0x9,
    NotZeroOrNotEqual = 0xa,
    Higher = 0xb,
    Positive = 0xc,
    Never = 0xd,
    GreaterOrEqual = 0xe,
    GreaterThan = 0xf,
}

impl Condition {
    pub(crate) const fn from_field(field: u8) -> Self {
        match field {
            0x0 => Self::Overflow,
            0x1 => Self::CarryOrLower,
            0x2 => Self::ZeroOrEqual,
            0x3 => Self::NotHigher,
            0x4 => Self::Negative,
            0x5 => Self::Always,
            0x6 => Self::LessThan,
            0x7 => Self::LessOrEqual,
            0x8 => Self::NoOverflow,
            0x9 => Self::NoCarryOrNotLower,
            0xa => Self::NotZeroOrNotEqual,
            0xb => Self::Higher,
            0xc => Self::Positive,
            0xd => Self::Never,
            0xe => Self::GreaterOrEqual,
            0xf => Self::GreaterThan,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum BitStringOperation {
    SearchZeroUp = 0x00,
    SearchZeroDown = 0x01,
    SearchOneUp = 0x02,
    SearchOneDown = 0x03,
    Or = 0x08,
    And = 0x09,
    Xor = 0x0a,
    Move = 0x0b,
    OrNot = 0x0c,
    AndNot = 0x0d,
    XorNot = 0x0e,
    Not = 0x0f,
}

impl BitStringOperation {
    pub(crate) const fn from_subopcode(value: u8) -> Option<Self> {
        Some(match value {
            0x00 => Self::SearchZeroUp,
            0x01 => Self::SearchZeroDown,
            0x02 => Self::SearchOneUp,
            0x03 => Self::SearchOneDown,
            0x08 => Self::Or,
            0x09 => Self::And,
            0x0a => Self::Xor,
            0x0b => Self::Move,
            0x0c => Self::OrNot,
            0x0d => Self::AndNot,
            0x0e => Self::XorNot,
            0x0f => Self::Not,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum FloatingOperation {
    Compare = 0x00,
    ConvertWordToFloat = 0x02,
    ConvertFloatToWord = 0x03,
    Add = 0x04,
    Subtract = 0x05,
    Multiply = 0x06,
    Divide = 0x07,
    TruncateFloatToWord = 0x0b,
}

impl FloatingOperation {
    pub(crate) const fn from_subopcode(value: u8) -> Option<Self> {
        Some(match value {
            0x00 => Self::Compare,
            0x02 => Self::ConvertWordToFloat,
            0x03 => Self::ConvertFloatToWord,
            0x04 => Self::Add,
            0x05 => Self::Subtract,
            0x06 => Self::Multiply,
            0x07 => Self::Divide,
            0x0b => Self::TruncateFloatToWord,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum LogicalImmediateOperation {
    Or = 0x2c,
    And = 0x2d,
    Xor = 0x2e,
    MoveHigh = 0x2f,
}

impl LogicalImmediateOperation {
    pub(crate) const fn from_opcode(value: u8) -> Option<Self> {
        Some(match value {
            0x2c => Self::Or,
            0x2d => Self::And,
            0x2e => Self::Xor,
            0x2f => Self::MoveHigh,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum MemoryOperation {
    LoadByte = 0x30,
    LoadHalfword = 0x31,
    LoadWord = 0x33,
    StoreByte = 0x34,
    StoreHalfword = 0x35,
    StoreWord = 0x37,
    InputByte = 0x38,
    InputHalfword = 0x39,
    CompareExchange = 0x3a,
    InputWord = 0x3b,
    OutputByte = 0x3c,
    OutputHalfword = 0x3d,
    OutputWord = 0x3f,
}

impl MemoryOperation {
    pub(crate) const fn from_opcode(value: u8) -> Option<Self> {
        Some(match value {
            0x30 => Self::LoadByte,
            0x31 => Self::LoadHalfword,
            0x33 => Self::LoadWord,
            0x34 => Self::StoreByte,
            0x35 => Self::StoreHalfword,
            0x37 => Self::StoreWord,
            0x38 => Self::InputByte,
            0x39 => Self::InputHalfword,
            0x3a => Self::CompareExchange,
            0x3b => Self::InputWord,
            0x3c => Self::OutputByte,
            0x3d => Self::OutputHalfword,
            0x3f => Self::OutputWord,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    Register {
        operation: RegisterOperation,
        source: Register,
        destination: Register,
    },
    JumpRegister(Register),
    Immediate {
        operation: ImmediateOperation,
        immediate: i8,
        destination: Register,
    },
    ShiftImmediate {
        operation: ShiftOperation,
        amount: u8,
        destination: Register,
    },
    SetFlag {
        condition: Condition,
        destination: Register,
    },
    Trap(u8),
    ReturnFromInterrupt,
    Halt,
    LoadSystemRegister {
        source: Register,
        destination: SystemRegister,
    },
    StoreSystemRegister {
        source: SystemRegister,
        destination: Register,
    },
    BitString(BitStringOperation),
    Nop,
    Branch {
        condition: Condition,
        displacement: i16,
    },
    AddImmediate {
        immediate: i16,
        source: Register,
        destination: Register,
    },
    MoveAddress {
        immediate: i16,
        source: Register,
        destination: Register,
    },
    LogicalImmediate {
        operation: LogicalImmediateOperation,
        immediate: u16,
        source: Register,
        destination: Register,
    },
    JumpRelative(i32),
    JumpAndLink(i32),
    Memory {
        operation: MemoryOperation,
        displacement: i16,
        base: Register,
        register: Register,
    },
    Floating {
        operation: FloatingOperation,
        source: Register,
        destination: Register,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InstructionForm {
    Register(RegisterOperation),
    JumpRegister,
    Immediate(ImmediateOperation),
    ShiftImmediate(ShiftOperation),
    SetFlag,
    Trap,
    ReturnFromInterrupt,
    Halt,
    LoadSystemRegister,
    StoreSystemRegister,
    BitString(BitStringOperation),
    Nop,
    Branch(Condition),
    AddImmediate,
    MoveAddress,
    LogicalImmediate(LogicalImmediateOperation),
    JumpRelative,
    JumpAndLink,
    Memory(MemoryOperation),
    Floating(FloatingOperation),
}

impl Instruction {
    pub const fn form(self) -> InstructionForm {
        match self {
            Self::Register { operation, .. } => InstructionForm::Register(operation),
            Self::JumpRegister(_) => InstructionForm::JumpRegister,
            Self::Immediate { operation, .. } => InstructionForm::Immediate(operation),
            Self::ShiftImmediate { operation, .. } => InstructionForm::ShiftImmediate(operation),
            Self::SetFlag { .. } => InstructionForm::SetFlag,
            Self::Trap(_) => InstructionForm::Trap,
            Self::ReturnFromInterrupt => InstructionForm::ReturnFromInterrupt,
            Self::Halt => InstructionForm::Halt,
            Self::LoadSystemRegister { .. } => InstructionForm::LoadSystemRegister,
            Self::StoreSystemRegister { .. } => InstructionForm::StoreSystemRegister,
            Self::BitString(operation) => InstructionForm::BitString(operation),
            Self::Nop => InstructionForm::Nop,
            Self::Branch { condition, .. } => InstructionForm::Branch(condition),
            Self::AddImmediate { .. } => InstructionForm::AddImmediate,
            Self::MoveAddress { .. } => InstructionForm::MoveAddress,
            Self::LogicalImmediate { operation, .. } => {
                InstructionForm::LogicalImmediate(operation)
            }
            Self::JumpRelative(_) => InstructionForm::JumpRelative,
            Self::JumpAndLink(_) => InstructionForm::JumpAndLink,
            Self::Memory { operation, .. } => InstructionForm::Memory(operation),
            Self::Floating { operation, .. } => InstructionForm::Floating(operation),
        }
    }

    pub const fn encoded_len(self) -> usize {
        match self {
            Self::AddImmediate { .. }
            | Self::MoveAddress { .. }
            | Self::LogicalImmediate { .. }
            | Self::JumpRelative(_)
            | Self::JumpAndLink(_)
            | Self::Memory { .. }
            | Self::Floating { .. } => 4,
            _ => 2,
        }
    }
}
