use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Semantic width selected by an instruction-family prefix.
pub enum OperandSize {
    /// The `Byte` operand size value.
    Byte,
    /// The `Word` operand size value.
    Word,
    /// The `Long` operand size value.
    Long,
}

impl OperandSize {
    /// Returns the operand width in bytes.
    pub const fn byte_len(self) -> usize {
        match self {
            Self::Byte => 1,
            Self::Word => 2,
            Self::Long => 4,
        }
    }
}

/// One of the eight registers in the currently selected register bank.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CurrentRegister(u8);

impl CurrentRegister {
    /// Constructs a current-bank register from an index below eight.
    pub const fn new(index: u8) -> Result<Self, RegisterError> {
        if index < 8 {
            Ok(Self(index))
        } else {
            Err(RegisterError::CurrentIndexOutOfRange { index })
        }
    }

    /// Returns the three-bit current-bank index.
    pub const fn index(self) -> u8 {
        self.0
    }

    pub(crate) const fn from_index(index: u8) -> Self {
        Self(index)
    }
}

/// One mapped extended general-register code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GeneralRegister(u8);

impl GeneralRegister {
    /// Constructs a register when `code` is mapped by the TLCS-900/H table.
    pub const fn new(code: u8) -> Result<Self, RegisterError> {
        if is_mapped_register_code(code) {
            Ok(Self(code))
        } else {
            Err(RegisterError::UnmappedCode { code })
        }
    }

    /// Returns the architectural register code.
    pub const fn code(self) -> u8 {
        self.0
    }

    /// Returns whether the code is aligned for `size` in the register map.
    pub const fn supports(self, size: OperandSize) -> bool {
        match size {
            OperandSize::Byte => true,
            OperandSize::Word => self.0 & 1 == 0,
            OperandSize::Long => self.0 & 3 == 0,
        }
    }
}

const fn is_mapped_register_code(code: u8) -> bool {
    code <= 0x3f || code >= 0xd0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
/// Failure to construct a mapped TLCS-900/H register selector.
pub enum RegisterError {
    #[error("TLCS-900/H current register index {index} is outside 0..7")]
    /// The `CurrentIndexOutOfRange` register error value.
    CurrentIndexOutOfRange {
        /// Rejected current-bank register index.
        index: u8,
    },
    #[error("TLCS-900/H general register code {code:#04x} is not mapped")]
    /// The `UnmappedCode` register error value.
    UnmappedCode {
        /// Rejected extended general-register code.
        code: u8,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Current-bank or extended general-register encoding.
pub enum RegisterEncoding {
    /// The `Current` register encoding value.
    Current(CurrentRegister),
    /// The `Extended` register encoding value.
    Extended(GeneralRegister),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
/// One of the sixteen encoded condition predicates.
pub enum Condition {
    /// The `False` condition value.
    False = 0x0,
    /// The `LessThan` condition value.
    LessThan = 0x1,
    /// The `LessOrEqual` condition value.
    LessOrEqual = 0x2,
    /// The `UnsignedLessOrEqual` condition value.
    UnsignedLessOrEqual = 0x3,
    /// The `ParityEvenOrOverflow` condition value.
    ParityEvenOrOverflow = 0x4,
    /// The `Minus` condition value.
    Minus = 0x5,
    /// The `Zero` condition value.
    Zero = 0x6,
    /// The `Carry` condition value.
    Carry = 0x7,
    /// The `True` condition value.
    True = 0x8,
    /// The `GreaterOrEqual` condition value.
    GreaterOrEqual = 0x9,
    /// The `GreaterThan` condition value.
    GreaterThan = 0xa,
    /// The `UnsignedGreaterThan` condition value.
    UnsignedGreaterThan = 0xb,
    /// The `ParityOddOrNoOverflow` condition value.
    ParityOddOrNoOverflow = 0xc,
    /// The `Plus` condition value.
    Plus = 0xd,
    /// The `NotZero` condition value.
    NotZero = 0xe,
    /// The `NotCarry` condition value.
    NotCarry = 0xf,
}

impl Condition {
    pub(crate) const fn from_code(code: u8) -> Self {
        match code {
            0x0 => Self::False,
            0x1 => Self::LessThan,
            0x2 => Self::LessOrEqual,
            0x3 => Self::UnsignedLessOrEqual,
            0x4 => Self::ParityEvenOrOverflow,
            0x5 => Self::Minus,
            0x6 => Self::Zero,
            0x7 => Self::Carry,
            0x8 => Self::True,
            0x9 => Self::GreaterOrEqual,
            0xa => Self::GreaterThan,
            0xb => Self::UnsignedGreaterThan,
            0xc => Self::ParityOddOrNoOverflow,
            0xd => Self::Plus,
            0xe => Self::NotZero,
            0xf => Self::NotCarry,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
/// Byte step used by pre-decrement, post-increment, and modulo operations.
pub enum Step {
    /// The `One` address step value.
    One = 0,
    /// The `Two` address step value.
    Two = 1,
    /// The `Four` address step value.
    Four = 2,
}

impl Step {
    /// Returns the address-update distance in bytes.
    pub const fn bytes(self) -> u8 {
        match self {
            Self::One => 1,
            Self::Two => 2,
            Self::Four => 4,
        }
    }

    pub(crate) const fn from_code(code: u8) -> Option<Self> {
        Some(match code {
            0 => Self::One,
            1 => Self::Two,
            2 => Self::Four,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A typed TLCS-900/H source or destination memory addressing mode.
pub enum MemoryAddress {
    /// The `CurrentIndirect` memory-address mode value.
    CurrentIndirect(CurrentRegister),
    /// The `CurrentDisplacement` memory-address mode value.
    CurrentDisplacement {
        /// Base register.
        base: CurrentRegister,
        /// Signed displacement.
        displacement: i8,
    },
    /// The `Direct8` memory-address mode value.
    Direct8(u8),
    /// The `Direct16` memory-address mode value.
    Direct16(u16),
    /// The `Direct24` memory-address mode value.
    Direct24(u32),
    /// The `Register` memory-address mode value.
    Register(GeneralRegister),
    /// The `RegisterDisplacement` memory-address mode value.
    RegisterDisplacement {
        /// Base register.
        base: GeneralRegister,
        /// Signed displacement.
        displacement: i16,
    },
    /// The `IndexedByte` memory-address mode value.
    IndexedByte {
        /// Base register.
        base: GeneralRegister,
        /// Rejected or selected register index.
        index: GeneralRegister,
    },
    /// The `IndexedWord` memory-address mode value.
    IndexedWord {
        /// Base register.
        base: GeneralRegister,
        /// Rejected or selected register index.
        index: GeneralRegister,
    },
    /// The `PcRelative` memory-address mode value.
    PcRelative(i16),
    /// The `PreDecrement` memory-address mode value.
    PreDecrement {
        /// General register.
        register: GeneralRegister,
        /// Address update step.
        step: Step,
    },
    /// The `PostIncrement` memory-address mode value.
    PostIncrement {
        /// General register.
        register: GeneralRegister,
        /// Address update step.
        step: Step,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Arithmetic or logical operation selected by a sub-opcode field.
pub enum ArithmeticOperation {
    /// The `Add` arithmetic operation value.
    Add,
    /// The `AddCarry` arithmetic operation value.
    AddCarry,
    /// The `Subtract` arithmetic operation value.
    Subtract,
    /// The `SubtractCarry` arithmetic operation value.
    SubtractCarry,
    /// The `And` arithmetic operation value.
    And,
    /// The `Xor` arithmetic operation value.
    Xor,
    /// The `Or` arithmetic operation value.
    Or,
    /// The `Compare` arithmetic operation value.
    Compare,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Rotate or shift operation selected by a sub-opcode field.
pub enum ShiftOperation {
    /// The `RotateLeftCircular` shift operation value.
    RotateLeftCircular,
    /// The `RotateRightCircular` shift operation value.
    RotateRightCircular,
    /// The `RotateLeftCarry` shift operation value.
    RotateLeftCarry,
    /// The `RotateRightCarry` shift operation value.
    RotateRightCarry,
    /// The `ShiftLeftArithmetic` shift operation value.
    ShiftLeftArithmetic,
    /// The `ShiftRightArithmetic` shift operation value.
    ShiftRightArithmetic,
    /// The `ShiftLeftLogical` shift operation value.
    ShiftLeftLogical,
    /// The `ShiftRightLogical` shift operation value.
    ShiftRightLogical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Carry or bit operation selected by a sub-opcode field.
pub enum BitOperation {
    /// The `AndCarry` bit operation value.
    AndCarry,
    /// The `OrCarry` bit operation value.
    OrCarry,
    /// The `XorCarry` bit operation value.
    XorCarry,
    /// The `LoadCarry` bit operation value.
    LoadCarry,
    /// The `StoreCarry` bit operation value.
    StoreCarry,
    /// The `Reset` bit operation value.
    Reset,
    /// The `Set` bit operation value.
    Set,
    /// The `Change` bit operation value.
    Change,
    /// The `Test` bit operation value.
    Test,
    /// The `TestAndSet` bit operation value.
    TestAndSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// An instruction encoded directly in the primary opcode map.
pub enum MainInstruction {
    /// The `Nop` primary instruction value.
    Nop,
    /// The `PushStatus` primary instruction value.
    PushStatus,
    /// The `PopStatus` primary instruction value.
    PopStatus,
    /// The `Halt` primary instruction value.
    Halt,
    /// The `EnableInterrupt` primary instruction value.
    EnableInterrupt(u8),
    /// The `DisableInterrupt` primary instruction value.
    DisableInterrupt,
    /// The `ReturnFromInterrupt` primary instruction value.
    ReturnFromInterrupt,
    /// The `LoadDirect8Byte` primary instruction value.
    LoadDirect8Byte {
        /// Direct address.
        address: u8,
        /// Immediate value.
        value: u8,
    },
    /// The `PushImmediateByte` primary instruction value.
    PushImmediateByte(u8),
    /// The `LoadDirect8Word` primary instruction value.
    LoadDirect8Word {
        /// Direct address.
        address: u8,
        /// Immediate value.
        value: u16,
    },
    /// The `PushImmediateWord` primary instruction value.
    PushImmediateWord(u16),
    /// The `IncrementRegisterFilePointer` primary instruction value.
    IncrementRegisterFilePointer,
    /// The `DecrementRegisterFilePointer` primary instruction value.
    DecrementRegisterFilePointer,
    /// The `Return` primary instruction value.
    Return,
    /// The `ReturnAndDeallocate` primary instruction value.
    ReturnAndDeallocate(u16),
    /// The `ResetCarry` primary instruction value.
    ResetCarry,
    /// The `SetCarry` primary instruction value.
    SetCarry,
    /// The `ComplementCarry` primary instruction value.
    ComplementCarry,
    /// The `ZeroToCarry` primary instruction value.
    ZeroToCarry,
    /// The `PushAccumulator` primary instruction value.
    PushAccumulator,
    /// The `PopAccumulator` primary instruction value.
    PopAccumulator,
    /// The `ExchangeFlags` primary instruction value.
    ExchangeFlags,
    /// The `LoadRegisterFilePointer` primary instruction value.
    LoadRegisterFilePointer(u8),
    /// The `PushFlags` primary instruction value.
    PushFlags,
    /// The `PopFlags` primary instruction value.
    PopFlags,
    /// The `JumpDirect16` primary instruction value.
    JumpDirect16(u16),
    /// The `JumpDirect24` primary instruction value.
    JumpDirect24(u32),
    /// The `CallDirect16` primary instruction value.
    CallDirect16(u16),
    /// The `CallDirect24` primary instruction value.
    CallDirect24(u32),
    /// The `CallRelative` primary instruction value.
    CallRelative(i16),
    /// The `LoadCurrentImmediate` primary instruction value.
    LoadCurrentImmediate {
        /// Destination operand.
        destination: CurrentRegister,
        /// Semantic operand size.
        size: OperandSize,
        /// Immediate value.
        value: u32,
    },
    /// The `PushCurrent` primary instruction value.
    PushCurrent {
        /// Source operand.
        source: CurrentRegister,
        /// Semantic operand size.
        size: OperandSize,
    },
    /// The `PopCurrent` primary instruction value.
    PopCurrent {
        /// Destination operand.
        destination: CurrentRegister,
        /// Semantic operand size.
        size: OperandSize,
    },
    /// The `JumpRelative` primary instruction value.
    JumpRelative {
        /// Encoded branch condition.
        condition: Condition,
        /// Signed displacement.
        displacement: i8,
    },
    /// The `JumpRelativeLong` primary instruction value.
    JumpRelativeLong {
        /// Encoded branch condition.
        condition: Condition,
        /// Signed displacement.
        displacement: i16,
    },
    /// The `LoadExtract` primary instruction value.
    LoadExtract {
        /// Direct address.
        address: u8,
        /// Immediate value.
        value: u8,
        /// Preserved filler bytes.
        filler: [u8; 3],
    },
    /// The `SoftwareInterrupt` primary instruction value.
    SoftwareInterrupt(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A secondary instruction following a register-family prefix.
pub enum RegisterInstruction {
    /// The `LoadImmediate` register-family instruction value.
    LoadImmediate(u32),
    /// The `Push` register-family instruction value.
    Push,
    /// The `Pop` register-family instruction value.
    Pop,
    /// The `Complement` register-family instruction value.
    Complement,
    /// The `Negate` register-family instruction value.
    Negate,
    /// The `MultiplyImmediate` register-family instruction value.
    MultiplyImmediate {
        /// Whether the operation uses signed arithmetic.
        signed: bool,
        /// Immediate value.
        value: u32,
    },
    /// The `DivideImmediate` register-family instruction value.
    DivideImmediate {
        /// Whether the operation uses signed arithmetic.
        signed: bool,
        /// Immediate value.
        value: u32,
    },
    /// The `Link` register-family instruction value.
    Link(i16),
    /// The `Unlink` register-family instruction value.
    Unlink,
    /// The `BitSearchForward` register-family instruction value.
    BitSearchForward,
    /// The `BitSearchBackward` register-family instruction value.
    BitSearchBackward,
    /// The `DecimalAdjust` register-family instruction value.
    DecimalAdjust,
    /// The `ExtendZero` register-family instruction value.
    ExtendZero,
    /// The `ExtendSign` register-family instruction value.
    ExtendSign,
    /// The `PointerAdjust` register-family instruction value.
    PointerAdjust,
    /// The `Mirror` register-family instruction value.
    Mirror,
    /// The `MultiplyAccumulate` register-family instruction value.
    MultiplyAccumulate,
    /// The `DecrementJump` register-family instruction value.
    DecrementJump(i8),
    /// The `CarryImmediate` register-family instruction value.
    CarryImmediate {
        /// Selected sub-operation.
        operation: BitOperation,
        /// Bit index.
        bit: u8,
    },
    /// The `CarryAccumulator` register-family instruction value.
    CarryAccumulator(BitOperation),
    /// The `LoadControl` register-family instruction value.
    LoadControl {
        /// Control-register selector.
        control_code: u8,
    },
    /// The `StoreControl` register-family instruction value.
    StoreControl {
        /// Control-register selector.
        control_code: u8,
    },
    /// The `BitImmediate` register-family instruction value.
    BitImmediate {
        /// Selected sub-operation.
        operation: BitOperation,
        /// Bit index.
        bit: u8,
    },
    /// The `ModuloIncrement` register-family instruction value.
    ModuloIncrement {
        /// Address update step.
        step: Step,
        /// Modulo boundary.
        modulus: u16,
    },
    /// The `ModuloDecrement` register-family instruction value.
    ModuloDecrement {
        /// Address update step.
        step: Step,
        /// Modulo boundary.
        modulus: u16,
    },
    /// The `MultiplyCurrent` register-family instruction value.
    MultiplyCurrent {
        /// Whether the operation uses signed arithmetic.
        signed: bool,
        /// Source operand.
        source: CurrentRegister,
    },
    /// The `DivideCurrent` register-family instruction value.
    DivideCurrent {
        /// Whether the operation uses signed arithmetic.
        signed: bool,
        /// Source operand.
        source: CurrentRegister,
    },
    /// The `Increment` register-family instruction value.
    Increment(u8),
    /// The `Decrement` register-family instruction value.
    Decrement(u8),
    /// The `SetCondition` register-family instruction value.
    SetCondition(Condition),
    /// The `ArithmeticCurrent` register-family instruction value.
    ArithmeticCurrent {
        /// Selected sub-operation.
        operation: ArithmeticOperation,
        /// Other current-bank register.
        other: CurrentRegister,
    },
    /// The `LoadFromCurrent` register-family instruction value.
    LoadFromCurrent(CurrentRegister),
    /// The `LoadToCurrent` register-family instruction value.
    LoadToCurrent(CurrentRegister),
    /// The `LoadSmallImmediate` register-family instruction value.
    LoadSmallImmediate(u8),
    /// The `ExchangeCurrent` register-family instruction value.
    ExchangeCurrent(CurrentRegister),
    /// The `ArithmeticImmediate` register-family instruction value.
    ArithmeticImmediate {
        /// Selected sub-operation.
        operation: ArithmeticOperation,
        /// Immediate value.
        value: u32,
    },
    /// The `CompareSmallImmediate` register-family instruction value.
    CompareSmallImmediate(u8),
    /// The `ShiftImmediate` register-family instruction value.
    ShiftImmediate {
        /// Selected sub-operation.
        operation: ShiftOperation,
        /// Shift count.
        count: u8,
    },
    /// The `ShiftAccumulator` register-family instruction value.
    ShiftAccumulator(ShiftOperation),
}

/// Register-family prefix operands and selected secondary instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegisterOperation {
    /// Operand size selected by the register prefix.
    pub size: OperandSize,
    /// Current-bank or extended register selector.
    pub register: RegisterEncoding,
    /// Register-family secondary instruction.
    pub instruction: RegisterInstruction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A secondary instruction following a source-memory prefix.
pub enum SourceMemoryInstruction {
    /// The `Push` source-memory instruction value.
    Push,
    /// The `RotateLeftDigit` source-memory instruction value.
    RotateLeftDigit,
    /// The `RotateRightDigit` source-memory instruction value.
    RotateRightDigit,
    /// The `LoadIncrement` source-memory instruction value.
    LoadIncrement,
    /// The `LoadIncrementRepeat` source-memory instruction value.
    LoadIncrementRepeat,
    /// The `LoadDecrement` source-memory instruction value.
    LoadDecrement,
    /// The `LoadDecrementRepeat` source-memory instruction value.
    LoadDecrementRepeat,
    /// The `CompareIncrement` source-memory instruction value.
    CompareIncrement,
    /// The `CompareIncrementRepeat` source-memory instruction value.
    CompareIncrementRepeat,
    /// The `CompareDecrement` source-memory instruction value.
    CompareDecrement,
    /// The `CompareDecrementRepeat` source-memory instruction value.
    CompareDecrementRepeat,
    /// The `LoadToDirect16` source-memory instruction value.
    LoadToDirect16(u16),
    /// The `LoadToCurrent` source-memory instruction value.
    LoadToCurrent(CurrentRegister),
    /// The `ExchangeCurrent` source-memory instruction value.
    ExchangeCurrent(CurrentRegister),
    /// The `ArithmeticImmediate` source-memory instruction value.
    ArithmeticImmediate {
        /// Selected sub-operation.
        operation: ArithmeticOperation,
        /// Immediate value.
        value: u32,
    },
    /// The `MultiplyCurrent` source-memory instruction value.
    MultiplyCurrent {
        /// Whether the operation uses signed arithmetic.
        signed: bool,
        /// Destination operand.
        destination: CurrentRegister,
    },
    /// The `DivideCurrent` source-memory instruction value.
    DivideCurrent {
        /// Whether the operation uses signed arithmetic.
        signed: bool,
        /// Destination operand.
        destination: CurrentRegister,
    },
    /// The `Increment` source-memory instruction value.
    Increment(u8),
    /// The `Decrement` source-memory instruction value.
    Decrement(u8),
    /// The `Shift` source-memory instruction value.
    Shift(ShiftOperation),
    /// The `ArithmeticCurrent` source-memory instruction value.
    ArithmeticCurrent {
        /// Selected sub-operation.
        operation: ArithmeticOperation,
        /// Current-bank register.
        current: CurrentRegister,
        /// Whether the memory operand is the destination.
        destination_is_memory: bool,
    },
}

/// Source-memory prefix address and selected secondary instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceMemoryOperation {
    /// Operand size selected by the memory prefix.
    pub size: OperandSize,
    /// Source memory addressing mode.
    pub source: MemoryAddress,
    /// Source-memory secondary instruction.
    pub instruction: SourceMemoryInstruction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A secondary instruction following a destination-memory prefix.
pub enum DestinationMemoryInstruction {
    /// The `LoadImmediateByte` destination-memory instruction value.
    LoadImmediateByte(u8),
    /// The `LoadImmediateWord` destination-memory instruction value.
    LoadImmediateWord(u16),
    /// The `PopByte` destination-memory instruction value.
    PopByte,
    /// The `PopWord` destination-memory instruction value.
    PopWord,
    /// The `LoadFromDirect16Byte` destination-memory instruction value.
    LoadFromDirect16Byte(u16),
    /// The `LoadFromDirect16Word` destination-memory instruction value.
    LoadFromDirect16Word(u16),
    /// The `LoadAddressWord` destination-memory instruction value.
    LoadAddressWord(CurrentRegister),
    /// The `LoadAddressLong` destination-memory instruction value.
    LoadAddressLong(CurrentRegister),
    /// The `CarryAccumulator` destination-memory instruction value.
    CarryAccumulator(BitOperation),
    /// The `LoadFromCurrent` destination-memory instruction value.
    LoadFromCurrent {
        /// Semantic operand size.
        size: OperandSize,
        /// Source operand.
        source: CurrentRegister,
    },
    /// The `BitImmediate` destination-memory instruction value.
    BitImmediate {
        /// Selected sub-operation.
        operation: BitOperation,
        /// Bit index.
        bit: u8,
    },
    /// The `Jump` destination-memory instruction value.
    Jump(Condition),
    /// The `Call` destination-memory instruction value.
    Call(Condition),
    /// The `Return` destination-memory instruction value.
    Return(Condition),
}

/// Destination-memory prefix address and selected secondary instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DestinationMemoryOperation {
    /// Destination memory addressing mode.
    pub destination: MemoryAddress,
    /// Destination-memory secondary instruction.
    pub instruction: DestinationMemoryInstruction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A complete typed TLCS-900/H instruction.
pub enum Instruction {
    /// The `Main` instruction family value.
    Main(MainInstruction),
    /// The `Register` instruction family value.
    Register(RegisterOperation),
    /// The `SourceMemory` instruction family value.
    SourceMemory(SourceMemoryOperation),
    /// The `DestinationMemory` instruction family value.
    DestinationMemory(DestinationMemoryOperation),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Operand-free identity of a primary-map instruction.
pub enum MainForm {
    /// The `Nop` primary form value.
    Nop,
    /// The `PushStatus` primary form value.
    PushStatus,
    /// The `PopStatus` primary form value.
    PopStatus,
    /// The `Halt` primary form value.
    Halt,
    /// The `EnableInterrupt` primary form value.
    EnableInterrupt,
    /// The `DisableInterrupt` primary form value.
    DisableInterrupt,
    /// The `ReturnFromInterrupt` primary form value.
    ReturnFromInterrupt,
    /// The `LoadDirect8Byte` primary form value.
    LoadDirect8Byte,
    /// The `PushImmediateByte` primary form value.
    PushImmediateByte,
    /// The `LoadDirect8Word` primary form value.
    LoadDirect8Word,
    /// The `PushImmediateWord` primary form value.
    PushImmediateWord,
    /// The `IncrementRegisterFilePointer` primary form value.
    IncrementRegisterFilePointer,
    /// The `DecrementRegisterFilePointer` primary form value.
    DecrementRegisterFilePointer,
    /// The `Return` primary form value.
    Return,
    /// The `ReturnAndDeallocate` primary form value.
    ReturnAndDeallocate,
    /// The `ResetCarry` primary form value.
    ResetCarry,
    /// The `SetCarry` primary form value.
    SetCarry,
    /// The `ComplementCarry` primary form value.
    ComplementCarry,
    /// The `ZeroToCarry` primary form value.
    ZeroToCarry,
    /// The `PushAccumulator` primary form value.
    PushAccumulator,
    /// The `PopAccumulator` primary form value.
    PopAccumulator,
    /// The `ExchangeFlags` primary form value.
    ExchangeFlags,
    /// The `LoadRegisterFilePointer` primary form value.
    LoadRegisterFilePointer,
    /// The `PushFlags` primary form value.
    PushFlags,
    /// The `PopFlags` primary form value.
    PopFlags,
    /// The `JumpDirect16` primary form value.
    JumpDirect16,
    /// The `JumpDirect24` primary form value.
    JumpDirect24,
    /// The `CallDirect16` primary form value.
    CallDirect16,
    /// The `CallDirect24` primary form value.
    CallDirect24,
    /// The `CallRelative` primary form value.
    CallRelative,
    /// The `LoadCurrentImmediate` primary form value.
    LoadCurrentImmediate(OperandSize),
    /// The `PushCurrent` primary form value.
    PushCurrent(OperandSize),
    /// The `PopCurrent` primary form value.
    PopCurrent(OperandSize),
    /// The `JumpRelative` primary form value.
    JumpRelative,
    /// The `JumpRelativeLong` primary form value.
    JumpRelativeLong,
    /// The `LoadExtract` primary form value.
    LoadExtract,
    /// The `SoftwareInterrupt` primary form value.
    SoftwareInterrupt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Operand-free identity of a register-family instruction.
pub enum RegisterForm {
    /// The `LoadImmediate` register form value.
    LoadImmediate,
    /// The `Push` register form value.
    Push,
    /// The `Pop` register form value.
    Pop,
    /// The `Complement` register form value.
    Complement,
    /// The `Negate` register form value.
    Negate,
    /// The `MultiplyImmediate` register form value.
    MultiplyImmediate,
    /// The `MultiplySignedImmediate` register form value.
    MultiplySignedImmediate,
    /// The `DivideImmediate` register form value.
    DivideImmediate,
    /// The `DivideSignedImmediate` register form value.
    DivideSignedImmediate,
    /// The `Link` register form value.
    Link,
    /// The `Unlink` register form value.
    Unlink,
    /// The `BitSearchForward` register form value.
    BitSearchForward,
    /// The `BitSearchBackward` register form value.
    BitSearchBackward,
    /// The `DecimalAdjust` register form value.
    DecimalAdjust,
    /// The `ExtendZero` register form value.
    ExtendZero,
    /// The `ExtendSign` register form value.
    ExtendSign,
    /// The `PointerAdjust` register form value.
    PointerAdjust,
    /// The `Mirror` register form value.
    Mirror,
    /// The `MultiplyAccumulate` register form value.
    MultiplyAccumulate,
    /// The `DecrementJump` register form value.
    DecrementJump,
    /// The `CarryImmediate` register form value.
    CarryImmediate(BitOperation),
    /// The `CarryAccumulator` register form value.
    CarryAccumulator(BitOperation),
    /// The `LoadControl` register form value.
    LoadControl,
    /// The `StoreControl` register form value.
    StoreControl,
    /// The `BitImmediate` register form value.
    BitImmediate(BitOperation),
    /// The `ModuloIncrement` register form value.
    ModuloIncrement(Step),
    /// The `ModuloDecrement` register form value.
    ModuloDecrement(Step),
    /// The `MultiplyCurrent` register form value.
    MultiplyCurrent,
    /// The `MultiplySignedCurrent` register form value.
    MultiplySignedCurrent,
    /// The `DivideCurrent` register form value.
    DivideCurrent,
    /// The `DivideSignedCurrent` register form value.
    DivideSignedCurrent,
    /// The `Increment` register form value.
    Increment,
    /// The `Decrement` register form value.
    Decrement,
    /// The `SetCondition` register form value.
    SetCondition,
    /// The `ArithmeticCurrent` register form value.
    ArithmeticCurrent(ArithmeticOperation),
    /// The `LoadFromCurrent` register form value.
    LoadFromCurrent,
    /// The `LoadToCurrent` register form value.
    LoadToCurrent,
    /// The `LoadSmallImmediate` register form value.
    LoadSmallImmediate,
    /// The `ExchangeCurrent` register form value.
    ExchangeCurrent,
    /// The `ArithmeticImmediate` register form value.
    ArithmeticImmediate(ArithmeticOperation),
    /// The `CompareSmallImmediate` register form value.
    CompareSmallImmediate,
    /// The `ShiftImmediate` register form value.
    ShiftImmediate(ShiftOperation),
    /// The `ShiftAccumulator` register form value.
    ShiftAccumulator(ShiftOperation),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Operand-free identity of a source-memory instruction.
pub enum SourceMemoryForm {
    /// The `Push` source-memory form value.
    Push,
    /// The `RotateLeftDigit` source-memory form value.
    RotateLeftDigit,
    /// The `RotateRightDigit` source-memory form value.
    RotateRightDigit,
    /// The `LoadIncrement` source-memory form value.
    LoadIncrement,
    /// The `LoadIncrementRepeat` source-memory form value.
    LoadIncrementRepeat,
    /// The `LoadDecrement` source-memory form value.
    LoadDecrement,
    /// The `LoadDecrementRepeat` source-memory form value.
    LoadDecrementRepeat,
    /// The `CompareIncrement` source-memory form value.
    CompareIncrement,
    /// The `CompareIncrementRepeat` source-memory form value.
    CompareIncrementRepeat,
    /// The `CompareDecrement` source-memory form value.
    CompareDecrement,
    /// The `CompareDecrementRepeat` source-memory form value.
    CompareDecrementRepeat,
    /// The `LoadToDirect16` source-memory form value.
    LoadToDirect16,
    /// The `LoadToCurrent` source-memory form value.
    LoadToCurrent,
    /// The `ExchangeCurrent` source-memory form value.
    ExchangeCurrent,
    /// The `ArithmeticImmediate` source-memory form value.
    ArithmeticImmediate(ArithmeticOperation),
    /// The `MultiplyCurrent` source-memory form value.
    MultiplyCurrent,
    /// The `MultiplySignedCurrent` source-memory form value.
    MultiplySignedCurrent,
    /// The `DivideCurrent` source-memory form value.
    DivideCurrent,
    /// The `DivideSignedCurrent` source-memory form value.
    DivideSignedCurrent,
    /// The `Increment` source-memory form value.
    Increment,
    /// The `Decrement` source-memory form value.
    Decrement,
    /// The `Shift` source-memory form value.
    Shift(ShiftOperation),
    /// The `ArithmeticCurrent` source-memory form value.
    ArithmeticCurrent {
        /// Selected sub-operation.
        operation: ArithmeticOperation,
        /// Whether the memory operand is the destination.
        destination_is_memory: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Operand-free identity of a destination-memory instruction.
pub enum DestinationMemoryForm {
    /// The `LoadImmediateByte` destination-memory form value.
    LoadImmediateByte,
    /// The `LoadImmediateWord` destination-memory form value.
    LoadImmediateWord,
    /// The `PopByte` destination-memory form value.
    PopByte,
    /// The `PopWord` destination-memory form value.
    PopWord,
    /// The `LoadFromDirect16Byte` destination-memory form value.
    LoadFromDirect16Byte,
    /// The `LoadFromDirect16Word` destination-memory form value.
    LoadFromDirect16Word,
    /// The `LoadAddressWord` destination-memory form value.
    LoadAddressWord,
    /// The `LoadAddressLong` destination-memory form value.
    LoadAddressLong,
    /// The `LoadAddressRelativeWord` destination-memory form value.
    LoadAddressRelativeWord,
    /// The `LoadAddressRelativeLong` destination-memory form value.
    LoadAddressRelativeLong,
    /// The `CarryAccumulator` destination-memory form value.
    CarryAccumulator(BitOperation),
    /// The `LoadFromCurrent` destination-memory form value.
    LoadFromCurrent(OperandSize),
    /// The `BitImmediate` destination-memory form value.
    BitImmediate(BitOperation),
    /// The `Jump` destination-memory form value.
    Jump,
    /// The `Call` destination-memory form value.
    Call,
    /// The `Return` destination-memory form value.
    Return,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Stable family and operand-size identity of an instruction.
pub enum InstructionForm {
    /// The `Main` instruction form value.
    Main(MainForm),
    /// The `Register` instruction form value.
    Register {
        /// Semantic operand size.
        size: OperandSize,
        /// Operand-free family form.
        form: RegisterForm,
    },
    /// The `SourceMemory` instruction form value.
    SourceMemory {
        /// Semantic operand size.
        size: OperandSize,
        /// Operand-free family form.
        form: SourceMemoryForm,
    },
    /// The `DestinationMemory` instruction form value.
    DestinationMemory(DestinationMemoryForm),
}

impl Instruction {
    /// Returns the stable family and operand-size form.
    pub const fn form(self) -> InstructionForm {
        match self {
            Self::Main(instruction) => InstructionForm::Main(instruction.form()),
            Self::Register(operation) => InstructionForm::Register {
                size: operation.size,
                form: operation.instruction.form(),
            },
            Self::SourceMemory(operation) => InstructionForm::SourceMemory {
                size: operation.size,
                form: operation.instruction.form(),
            },
            Self::DestinationMemory(operation) => {
                InstructionForm::DestinationMemory(operation.form())
            }
        }
    }
}

impl MainInstruction {
    const fn form(self) -> MainForm {
        match self {
            Self::Nop => MainForm::Nop,
            Self::PushStatus => MainForm::PushStatus,
            Self::PopStatus => MainForm::PopStatus,
            Self::Halt => MainForm::Halt,
            Self::EnableInterrupt(_) => MainForm::EnableInterrupt,
            Self::DisableInterrupt => MainForm::DisableInterrupt,
            Self::ReturnFromInterrupt => MainForm::ReturnFromInterrupt,
            Self::LoadDirect8Byte { .. } => MainForm::LoadDirect8Byte,
            Self::PushImmediateByte(_) => MainForm::PushImmediateByte,
            Self::LoadDirect8Word { .. } => MainForm::LoadDirect8Word,
            Self::PushImmediateWord(_) => MainForm::PushImmediateWord,
            Self::IncrementRegisterFilePointer => MainForm::IncrementRegisterFilePointer,
            Self::DecrementRegisterFilePointer => MainForm::DecrementRegisterFilePointer,
            Self::Return => MainForm::Return,
            Self::ReturnAndDeallocate(_) => MainForm::ReturnAndDeallocate,
            Self::ResetCarry => MainForm::ResetCarry,
            Self::SetCarry => MainForm::SetCarry,
            Self::ComplementCarry => MainForm::ComplementCarry,
            Self::ZeroToCarry => MainForm::ZeroToCarry,
            Self::PushAccumulator => MainForm::PushAccumulator,
            Self::PopAccumulator => MainForm::PopAccumulator,
            Self::ExchangeFlags => MainForm::ExchangeFlags,
            Self::LoadRegisterFilePointer(_) => MainForm::LoadRegisterFilePointer,
            Self::PushFlags => MainForm::PushFlags,
            Self::PopFlags => MainForm::PopFlags,
            Self::JumpDirect16(_) => MainForm::JumpDirect16,
            Self::JumpDirect24(_) => MainForm::JumpDirect24,
            Self::CallDirect16(_) => MainForm::CallDirect16,
            Self::CallDirect24(_) => MainForm::CallDirect24,
            Self::CallRelative(_) => MainForm::CallRelative,
            Self::LoadCurrentImmediate { size, .. } => MainForm::LoadCurrentImmediate(size),
            Self::PushCurrent { size, .. } => MainForm::PushCurrent(size),
            Self::PopCurrent { size, .. } => MainForm::PopCurrent(size),
            Self::JumpRelative { .. } => MainForm::JumpRelative,
            Self::JumpRelativeLong { .. } => MainForm::JumpRelativeLong,
            Self::LoadExtract { .. } => MainForm::LoadExtract,
            Self::SoftwareInterrupt(_) => MainForm::SoftwareInterrupt,
        }
    }
}

impl RegisterInstruction {
    const fn form(self) -> RegisterForm {
        match self {
            Self::LoadImmediate(_) => RegisterForm::LoadImmediate,
            Self::Push => RegisterForm::Push,
            Self::Pop => RegisterForm::Pop,
            Self::Complement => RegisterForm::Complement,
            Self::Negate => RegisterForm::Negate,
            Self::MultiplyImmediate { signed: false, .. } => RegisterForm::MultiplyImmediate,
            Self::MultiplyImmediate { signed: true, .. } => RegisterForm::MultiplySignedImmediate,
            Self::DivideImmediate { signed: false, .. } => RegisterForm::DivideImmediate,
            Self::DivideImmediate { signed: true, .. } => RegisterForm::DivideSignedImmediate,
            Self::Link(_) => RegisterForm::Link,
            Self::Unlink => RegisterForm::Unlink,
            Self::BitSearchForward => RegisterForm::BitSearchForward,
            Self::BitSearchBackward => RegisterForm::BitSearchBackward,
            Self::DecimalAdjust => RegisterForm::DecimalAdjust,
            Self::ExtendZero => RegisterForm::ExtendZero,
            Self::ExtendSign => RegisterForm::ExtendSign,
            Self::PointerAdjust => RegisterForm::PointerAdjust,
            Self::Mirror => RegisterForm::Mirror,
            Self::MultiplyAccumulate => RegisterForm::MultiplyAccumulate,
            Self::DecrementJump(_) => RegisterForm::DecrementJump,
            Self::CarryImmediate { operation, .. } => RegisterForm::CarryImmediate(operation),
            Self::CarryAccumulator(operation) => RegisterForm::CarryAccumulator(operation),
            Self::LoadControl { .. } => RegisterForm::LoadControl,
            Self::StoreControl { .. } => RegisterForm::StoreControl,
            Self::BitImmediate { operation, .. } => RegisterForm::BitImmediate(operation),
            Self::ModuloIncrement { step, .. } => RegisterForm::ModuloIncrement(step),
            Self::ModuloDecrement { step, .. } => RegisterForm::ModuloDecrement(step),
            Self::MultiplyCurrent { signed: false, .. } => RegisterForm::MultiplyCurrent,
            Self::MultiplyCurrent { signed: true, .. } => RegisterForm::MultiplySignedCurrent,
            Self::DivideCurrent { signed: false, .. } => RegisterForm::DivideCurrent,
            Self::DivideCurrent { signed: true, .. } => RegisterForm::DivideSignedCurrent,
            Self::Increment(_) => RegisterForm::Increment,
            Self::Decrement(_) => RegisterForm::Decrement,
            Self::SetCondition(_) => RegisterForm::SetCondition,
            Self::ArithmeticCurrent { operation, .. } => RegisterForm::ArithmeticCurrent(operation),
            Self::LoadFromCurrent(_) => RegisterForm::LoadFromCurrent,
            Self::LoadToCurrent(_) => RegisterForm::LoadToCurrent,
            Self::LoadSmallImmediate(_) => RegisterForm::LoadSmallImmediate,
            Self::ExchangeCurrent(_) => RegisterForm::ExchangeCurrent,
            Self::ArithmeticImmediate { operation, .. } => {
                RegisterForm::ArithmeticImmediate(operation)
            }
            Self::CompareSmallImmediate(_) => RegisterForm::CompareSmallImmediate,
            Self::ShiftImmediate { operation, .. } => RegisterForm::ShiftImmediate(operation),
            Self::ShiftAccumulator(operation) => RegisterForm::ShiftAccumulator(operation),
        }
    }
}

impl SourceMemoryInstruction {
    const fn form(self) -> SourceMemoryForm {
        match self {
            Self::Push => SourceMemoryForm::Push,
            Self::RotateLeftDigit => SourceMemoryForm::RotateLeftDigit,
            Self::RotateRightDigit => SourceMemoryForm::RotateRightDigit,
            Self::LoadIncrement => SourceMemoryForm::LoadIncrement,
            Self::LoadIncrementRepeat => SourceMemoryForm::LoadIncrementRepeat,
            Self::LoadDecrement => SourceMemoryForm::LoadDecrement,
            Self::LoadDecrementRepeat => SourceMemoryForm::LoadDecrementRepeat,
            Self::CompareIncrement => SourceMemoryForm::CompareIncrement,
            Self::CompareIncrementRepeat => SourceMemoryForm::CompareIncrementRepeat,
            Self::CompareDecrement => SourceMemoryForm::CompareDecrement,
            Self::CompareDecrementRepeat => SourceMemoryForm::CompareDecrementRepeat,
            Self::LoadToDirect16(_) => SourceMemoryForm::LoadToDirect16,
            Self::LoadToCurrent(_) => SourceMemoryForm::LoadToCurrent,
            Self::ExchangeCurrent(_) => SourceMemoryForm::ExchangeCurrent,
            Self::ArithmeticImmediate { operation, .. } => {
                SourceMemoryForm::ArithmeticImmediate(operation)
            }
            Self::MultiplyCurrent { signed: false, .. } => SourceMemoryForm::MultiplyCurrent,
            Self::MultiplyCurrent { signed: true, .. } => SourceMemoryForm::MultiplySignedCurrent,
            Self::DivideCurrent { signed: false, .. } => SourceMemoryForm::DivideCurrent,
            Self::DivideCurrent { signed: true, .. } => SourceMemoryForm::DivideSignedCurrent,
            Self::Increment(_) => SourceMemoryForm::Increment,
            Self::Decrement(_) => SourceMemoryForm::Decrement,
            Self::Shift(operation) => SourceMemoryForm::Shift(operation),
            Self::ArithmeticCurrent {
                operation,
                destination_is_memory,
                ..
            } => SourceMemoryForm::ArithmeticCurrent {
                operation,
                destination_is_memory,
            },
        }
    }
}

impl DestinationMemoryOperation {
    const fn form(self) -> DestinationMemoryForm {
        match self.instruction {
            DestinationMemoryInstruction::LoadImmediateByte(_) => {
                DestinationMemoryForm::LoadImmediateByte
            }
            DestinationMemoryInstruction::LoadImmediateWord(_) => {
                DestinationMemoryForm::LoadImmediateWord
            }
            DestinationMemoryInstruction::PopByte => DestinationMemoryForm::PopByte,
            DestinationMemoryInstruction::PopWord => DestinationMemoryForm::PopWord,
            DestinationMemoryInstruction::LoadFromDirect16Byte(_) => {
                DestinationMemoryForm::LoadFromDirect16Byte
            }
            DestinationMemoryInstruction::LoadFromDirect16Word(_) => {
                DestinationMemoryForm::LoadFromDirect16Word
            }
            DestinationMemoryInstruction::LoadAddressWord(_) => {
                if matches!(self.destination, MemoryAddress::PcRelative(_)) {
                    DestinationMemoryForm::LoadAddressRelativeWord
                } else {
                    DestinationMemoryForm::LoadAddressWord
                }
            }
            DestinationMemoryInstruction::LoadAddressLong(_) => {
                if matches!(self.destination, MemoryAddress::PcRelative(_)) {
                    DestinationMemoryForm::LoadAddressRelativeLong
                } else {
                    DestinationMemoryForm::LoadAddressLong
                }
            }
            DestinationMemoryInstruction::CarryAccumulator(operation) => {
                DestinationMemoryForm::CarryAccumulator(operation)
            }
            DestinationMemoryInstruction::LoadFromCurrent { size, .. } => {
                DestinationMemoryForm::LoadFromCurrent(size)
            }
            DestinationMemoryInstruction::BitImmediate { operation, .. } => {
                DestinationMemoryForm::BitImmediate(operation)
            }
            DestinationMemoryInstruction::Jump(_) => DestinationMemoryForm::Jump,
            DestinationMemoryInstruction::Call(_) => DestinationMemoryForm::Call,
            DestinationMemoryInstruction::Return(_) => DestinationMemoryForm::Return,
        }
    }
}
