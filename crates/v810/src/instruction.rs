use thiserror::Error;

/// One of the V810's 32 general registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Register(u8);

impl Register {
    /// Register zero, which supplies a constant zero value.
    pub const ZERO: Self = Self(0);
    /// Conventional link register used by `JAL`.
    pub const LINK: Self = Self(31);

    /// Construct a register from an architectural index in `0..32`.
    pub const fn new(index: u8) -> Result<Self, RegisterOutOfRange> {
        if index < 32 {
            Ok(Self(index))
        } else {
            Err(RegisterOutOfRange { index })
        }
    }

    /// Return the five-bit architectural register index.
    pub const fn index(self) -> u8 {
        self.0
    }

    pub(crate) const fn from_field(field: u16) -> Self {
        Self(field as u8)
    }
}

/// Error returned when constructing a general register outside `0..32`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("V810 general register index {index} is outside 0..31")]
pub struct RegisterOutOfRange {
    /// Rejected register index.
    pub index: u8,
}

/// Architecturally defined V810 system register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum SystemRegister {
    /// Exception/interrupt program counter.
    Eipc = 0,
    /// Exception/interrupt program status word.
    Eipsw = 1,
    /// Duplexed-exception program counter.
    Fepc = 2,
    /// Duplexed-exception program status word.
    Fepsw = 3,
    /// Exception cause register.
    Ecr = 4,
    /// Program status word.
    Psw = 5,
    /// Processor ID register.
    Pir = 6,
    /// Task-control word.
    Tkcw = 7,
    /// Cache-control word.
    Chcw = 24,
    /// Address-trap register.
    Adtre = 25,
}

impl SystemRegister {
    /// Return the five-bit system-register identifier.
    pub const fn id(self) -> u8 {
        self as u8
    }

    /// Return whether `LDSR` may write this system register.
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

/// Two-register integer operation selected by the primary opcode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum RegisterOperation {
    /// Copy the source register.
    Move = 0x00,
    /// Add signed integer operands.
    Add = 0x01,
    /// Subtract the source from the destination.
    Subtract = 0x02,
    /// Compare by subtraction without retaining the result.
    Compare = 0x03,
    /// Logical shift left by a register count.
    ShiftLeft = 0x04,
    /// Logical shift right by a register count.
    ShiftRight = 0x05,
    /// Arithmetic shift right by a register count.
    ShiftRightArithmetic = 0x07,
    /// Signed multiplication.
    Multiply = 0x08,
    /// Signed division.
    Divide = 0x09,
    /// Unsigned multiplication.
    MultiplyUnsigned = 0x0a,
    /// Unsigned division.
    DivideUnsigned = 0x0b,
    /// Bitwise OR.
    Or = 0x0c,
    /// Bitwise AND.
    And = 0x0d,
    /// Bitwise exclusive OR.
    Xor = 0x0e,
    /// Bitwise complement of the source.
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

/// Signed five-bit immediate operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ImmediateOperation {
    /// Move a sign-extended immediate.
    Move = 0x10,
    /// Add a sign-extended immediate.
    Add = 0x11,
    /// Compare against a sign-extended immediate.
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

/// Shift operation with an immediate count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShiftOperation {
    /// Logical shift left.
    Left,
    /// Logical shift right.
    Right,
    /// Arithmetic shift right.
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

/// Status condition selected by branches and `SETF`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Condition {
    /// Overflow flag is set.
    Overflow = 0x0,
    /// Carry flag is set, or unsigned comparison is lower.
    CarryOrLower = 0x1,
    /// Zero flag is set, or values compare equal.
    ZeroOrEqual = 0x2,
    /// Unsigned comparison is not higher.
    NotHigher = 0x3,
    /// Sign flag is set.
    Negative = 0x4,
    /// Condition always holds.
    Always = 0x5,
    /// Signed comparison is less than.
    LessThan = 0x6,
    /// Signed comparison is less than or equal.
    LessOrEqual = 0x7,
    /// Overflow flag is clear.
    NoOverflow = 0x8,
    /// Carry flag is clear, or unsigned comparison is not lower.
    NoCarryOrNotLower = 0x9,
    /// Zero flag is clear, or values compare unequal.
    NotZeroOrNotEqual = 0xa,
    /// Unsigned comparison is higher.
    Higher = 0xb,
    /// Sign flag is clear.
    Positive = 0xc,
    /// Condition never holds.
    Never = 0xd,
    /// Signed comparison is greater than or equal.
    GreaterOrEqual = 0xe,
    /// Signed comparison is greater than.
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

/// Search or Boolean operation over V810 bit strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum BitStringOperation {
    /// Search upward for a zero bit.
    SearchZeroUp = 0x00,
    /// Search downward for a zero bit.
    SearchZeroDown = 0x01,
    /// Search upward for a one bit.
    SearchOneUp = 0x02,
    /// Search downward for a one bit.
    SearchOneDown = 0x03,
    /// OR source bits into destination bits.
    Or = 0x08,
    /// AND source bits into destination bits.
    And = 0x09,
    /// Exclusive-OR source bits into destination bits.
    Xor = 0x0a,
    /// Copy source bits to destination bits.
    Move = 0x0b,
    /// OR complemented source bits into destination bits.
    OrNot = 0x0c,
    /// AND complemented source bits into destination bits.
    AndNot = 0x0d,
    /// Exclusive-OR complemented source bits into destination bits.
    XorNot = 0x0e,
    /// Copy complemented source bits to destination bits.
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

/// Single-precision floating-point operation in the extension subopcode space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum FloatingOperation {
    /// Compare two single-precision values.
    Compare = 0x00,
    /// Convert a signed word to single precision.
    ConvertWordToFloat = 0x02,
    /// Convert single precision to a rounded signed word.
    ConvertFloatToWord = 0x03,
    /// Add two single-precision values.
    Add = 0x04,
    /// Subtract two single-precision values.
    Subtract = 0x05,
    /// Multiply two single-precision values.
    Multiply = 0x06,
    /// Divide two single-precision values.
    Divide = 0x07,
    /// Convert single precision to a truncated signed word.
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

/// Unsigned 16-bit logical immediate operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum LogicalImmediateOperation {
    /// Bitwise OR with a zero-extended immediate.
    Or = 0x2c,
    /// Bitwise AND with a zero-extended immediate.
    And = 0x2d,
    /// Bitwise exclusive OR with a zero-extended immediate.
    Xor = 0x2e,
    /// Move the immediate into the high halfword.
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

/// Memory or I/O transfer operation selected by a format-VI opcode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum MemoryOperation {
    /// Sign-extending byte load.
    LoadByte = 0x30,
    /// Sign-extending halfword load.
    LoadHalfword = 0x31,
    /// Word load.
    LoadWord = 0x33,
    /// Byte store.
    StoreByte = 0x34,
    /// Halfword store.
    StoreHalfword = 0x35,
    /// Word store.
    StoreWord = 0x37,
    /// Byte input from the I/O address space.
    InputByte = 0x38,
    /// Halfword input from the I/O address space.
    InputHalfword = 0x39,
    /// Atomic compare-and-exchange word operation.
    CompareExchange = 0x3a,
    /// Word input from the I/O address space.
    InputWord = 0x3b,
    /// Byte output to the I/O address space.
    OutputByte = 0x3c,
    /// Halfword output to the I/O address space.
    OutputHalfword = 0x3d,
    /// Word output to the I/O address space.
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

/// One typed V810 instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    /// Two-register integer operation.
    Register {
        /// Operation selected by the primary opcode.
        operation: RegisterOperation,
        /// First source register.
        source: Register,
        /// Source/destination register updated by applicable operations.
        destination: Register,
    },
    /// Jump to the address held in a register.
    JumpRegister(Register),
    /// Signed five-bit immediate operation.
    Immediate {
        /// Operation selected by the primary opcode.
        operation: ImmediateOperation,
        /// Sign-extended five-bit value.
        immediate: i8,
        /// Destination or comparison register.
        destination: Register,
    },
    /// Shift a register by an immediate five-bit count.
    ShiftImmediate {
        /// Shift direction and signedness.
        operation: ShiftOperation,
        /// Shift count encoded in five bits.
        amount: u8,
        /// Register shifted in place.
        destination: Register,
    },
    /// Materialize a status condition as zero or one.
    SetFlag {
        /// Condition evaluated against the program status word.
        condition: Condition,
        /// Register receiving zero or one.
        destination: Register,
    },
    /// Enter one of the encoded software-trap vectors.
    Trap(u8),
    /// Return from an exception or interrupt (`RETI`).
    ReturnFromInterrupt,
    /// Halt instruction execution.
    Halt,
    /// Load a general-register value into a writable system register.
    LoadSystemRegister {
        /// General register supplying the value.
        source: Register,
        /// Writable system register receiving the value.
        destination: SystemRegister,
    },
    /// Store a system-register value into a general register.
    StoreSystemRegister {
        /// System register supplying the value.
        source: SystemRegister,
        /// General register receiving the value.
        destination: Register,
    },
    /// Execute a bit-string search or Boolean operation.
    BitString(BitStringOperation),
    /// Perform no operation.
    Nop,
    /// Conditional nine-bit PC-relative branch.
    Branch {
        /// Status condition controlling the branch.
        condition: Condition,
        /// Signed byte displacement from the current instruction address.
        displacement: i16,
    },
    /// Add a signed 16-bit immediate to a source register.
    AddImmediate {
        /// Signed immediate addend.
        immediate: i16,
        /// General register supplying the base value.
        source: Register,
        /// General register receiving the sum.
        destination: Register,
    },
    /// Form a 32-bit address from a register and signed immediate (`MOVEA`).
    MoveAddress {
        /// Signed immediate address component.
        immediate: i16,
        /// General register supplying the base address.
        source: Register,
        /// General register receiving the address.
        destination: Register,
    },
    /// Apply a logical operation with an unsigned 16-bit immediate.
    LogicalImmediate {
        /// Logical or high-half move operation.
        operation: LogicalImmediateOperation,
        /// Unsigned immediate bits.
        immediate: u16,
        /// General register supplying the source value.
        source: Register,
        /// General register receiving the result.
        destination: Register,
    },
    /// Unconditional 26-bit PC-relative jump.
    JumpRelative(i32),
    /// PC-relative jump that writes the return address to the link register.
    JumpAndLink(i32),
    /// Memory or I/O transfer through a base register and displacement.
    Memory {
        /// Transfer width, direction, and address-space operation.
        operation: MemoryOperation,
        /// Signed 16-bit byte displacement.
        displacement: i16,
        /// General register supplying the base address.
        base: Register,
        /// General register loaded, stored, or exchanged.
        register: Register,
    },
    /// Single-precision floating-point or conversion operation.
    Floating {
        /// Floating-point operation selected by the subopcode.
        operation: FloatingOperation,
        /// Source general register containing operand bits.
        source: Register,
        /// Destination general register containing operand/result bits.
        destination: Register,
    },
}

/// Stable instruction identity with register and immediate values removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InstructionForm {
    /// Form of [`Instruction::Register`].
    Register(RegisterOperation),
    /// Form of [`Instruction::JumpRegister`].
    JumpRegister,
    /// Form of [`Instruction::Immediate`].
    Immediate(ImmediateOperation),
    /// Form of [`Instruction::ShiftImmediate`].
    ShiftImmediate(ShiftOperation),
    /// Form of [`Instruction::SetFlag`].
    SetFlag,
    /// Form of [`Instruction::Trap`].
    Trap,
    /// Form of [`Instruction::ReturnFromInterrupt`].
    ReturnFromInterrupt,
    /// Form of [`Instruction::Halt`].
    Halt,
    /// Form of [`Instruction::LoadSystemRegister`].
    LoadSystemRegister,
    /// Form of [`Instruction::StoreSystemRegister`].
    StoreSystemRegister,
    /// Form of [`Instruction::BitString`].
    BitString(BitStringOperation),
    /// Form of [`Instruction::Nop`].
    Nop,
    /// Branch form preserving the condition selector.
    Branch(Condition),
    /// Form of [`Instruction::AddImmediate`].
    AddImmediate,
    /// Form of [`Instruction::MoveAddress`].
    MoveAddress,
    /// Form of [`Instruction::LogicalImmediate`].
    LogicalImmediate(LogicalImmediateOperation),
    /// Form of [`Instruction::JumpRelative`].
    JumpRelative,
    /// Form of [`Instruction::JumpAndLink`].
    JumpAndLink,
    /// Form of [`Instruction::Memory`].
    Memory(MemoryOperation),
    /// Form of [`Instruction::Floating`].
    Floating(FloatingOperation),
}

impl Instruction {
    /// Return the stable form identity with operand values removed.
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

    /// Return the complete encoded length in bytes.
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
