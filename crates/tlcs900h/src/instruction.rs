use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OperandSize {
    Byte,
    Word,
    Long,
}

impl OperandSize {
    pub const fn byte_len(self) -> usize {
        match self {
            Self::Byte => 1,
            Self::Word => 2,
            Self::Long => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CurrentRegister(u8);

impl CurrentRegister {
    pub const fn new(index: u8) -> Result<Self, RegisterError> {
        if index < 8 {
            Ok(Self(index))
        } else {
            Err(RegisterError::CurrentIndexOutOfRange { index })
        }
    }

    pub const fn index(self) -> u8 {
        self.0
    }

    pub(crate) const fn from_index(index: u8) -> Self {
        Self(index)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GeneralRegister(u8);

impl GeneralRegister {
    pub const fn new(code: u8) -> Result<Self, RegisterError> {
        if is_mapped_register_code(code) {
            Ok(Self(code))
        } else {
            Err(RegisterError::UnmappedCode { code })
        }
    }

    pub const fn code(self) -> u8 {
        self.0
    }

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
pub enum RegisterError {
    #[error("TLCS-900/H current register index {index} is outside 0..7")]
    CurrentIndexOutOfRange { index: u8 },
    #[error("TLCS-900/H general register code {code:#04x} is not mapped")]
    UnmappedCode { code: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RegisterEncoding {
    Current(CurrentRegister),
    Extended(GeneralRegister),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Condition {
    False = 0x0,
    LessThan = 0x1,
    LessOrEqual = 0x2,
    UnsignedLessOrEqual = 0x3,
    ParityEvenOrOverflow = 0x4,
    Minus = 0x5,
    Zero = 0x6,
    Carry = 0x7,
    True = 0x8,
    GreaterOrEqual = 0x9,
    GreaterThan = 0xa,
    UnsignedGreaterThan = 0xb,
    ParityOddOrNoOverflow = 0xc,
    Plus = 0xd,
    NotZero = 0xe,
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
pub enum Step {
    One = 0,
    Two = 1,
    Four = 2,
}

impl Step {
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
pub enum MemoryAddress {
    CurrentIndirect(CurrentRegister),
    CurrentDisplacement {
        base: CurrentRegister,
        displacement: i8,
    },
    Direct8(u8),
    Direct16(u16),
    Direct24(u32),
    Register(GeneralRegister),
    RegisterDisplacement {
        base: GeneralRegister,
        displacement: i16,
    },
    IndexedByte {
        base: GeneralRegister,
        index: GeneralRegister,
    },
    IndexedWord {
        base: GeneralRegister,
        index: GeneralRegister,
    },
    PcRelative(i16),
    PreDecrement {
        register: GeneralRegister,
        step: Step,
    },
    PostIncrement {
        register: GeneralRegister,
        step: Step,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArithmeticOperation {
    Add,
    AddCarry,
    Subtract,
    SubtractCarry,
    And,
    Xor,
    Or,
    Compare,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShiftOperation {
    RotateLeftCircular,
    RotateRightCircular,
    RotateLeftCarry,
    RotateRightCarry,
    ShiftLeftArithmetic,
    ShiftRightArithmetic,
    ShiftLeftLogical,
    ShiftRightLogical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BitOperation {
    AndCarry,
    OrCarry,
    XorCarry,
    LoadCarry,
    StoreCarry,
    Reset,
    Set,
    Change,
    Test,
    TestAndSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainInstruction {
    Nop,
    PushStatus,
    PopStatus,
    Halt,
    EnableInterrupt(u8),
    DisableInterrupt,
    ReturnFromInterrupt,
    LoadDirect8Byte {
        address: u8,
        value: u8,
    },
    PushImmediateByte(u8),
    LoadDirect8Word {
        address: u8,
        value: u16,
    },
    PushImmediateWord(u16),
    IncrementRegisterFilePointer,
    DecrementRegisterFilePointer,
    Return,
    ReturnAndDeallocate(u16),
    ResetCarry,
    SetCarry,
    ComplementCarry,
    ZeroToCarry,
    PushAccumulator,
    PopAccumulator,
    ExchangeFlags,
    LoadRegisterFilePointer(u8),
    PushFlags,
    PopFlags,
    JumpDirect16(u16),
    JumpDirect24(u32),
    CallDirect16(u16),
    CallDirect24(u32),
    CallRelative(i16),
    LoadCurrentImmediate {
        destination: CurrentRegister,
        size: OperandSize,
        value: u32,
    },
    PushCurrent {
        source: CurrentRegister,
        size: OperandSize,
    },
    PopCurrent {
        destination: CurrentRegister,
        size: OperandSize,
    },
    JumpRelative {
        condition: Condition,
        displacement: i8,
    },
    JumpRelativeLong {
        condition: Condition,
        displacement: i16,
    },
    LoadExtract {
        address: u8,
        value: u8,
        filler: [u8; 3],
    },
    SoftwareInterrupt(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterInstruction {
    LoadImmediate(u32),
    Push,
    Pop,
    Complement,
    Negate,
    MultiplyImmediate {
        signed: bool,
        value: u32,
    },
    DivideImmediate {
        signed: bool,
        value: u32,
    },
    Link(i16),
    Unlink,
    BitSearchForward,
    BitSearchBackward,
    DecimalAdjust,
    ExtendZero,
    ExtendSign,
    PointerAdjust,
    Mirror,
    MultiplyAccumulate,
    DecrementJump(i8),
    CarryImmediate {
        operation: BitOperation,
        bit: u8,
    },
    CarryAccumulator(BitOperation),
    LoadControl {
        control_code: u8,
    },
    StoreControl {
        control_code: u8,
    },
    BitImmediate {
        operation: BitOperation,
        bit: u8,
    },
    ModuloIncrement {
        step: Step,
        modulus: u16,
    },
    ModuloDecrement {
        step: Step,
        modulus: u16,
    },
    MultiplyCurrent {
        signed: bool,
        source: CurrentRegister,
    },
    DivideCurrent {
        signed: bool,
        source: CurrentRegister,
    },
    Increment(u8),
    Decrement(u8),
    SetCondition(Condition),
    ArithmeticCurrent {
        operation: ArithmeticOperation,
        other: CurrentRegister,
    },
    LoadFromCurrent(CurrentRegister),
    LoadToCurrent(CurrentRegister),
    LoadSmallImmediate(u8),
    ExchangeCurrent(CurrentRegister),
    ArithmeticImmediate {
        operation: ArithmeticOperation,
        value: u32,
    },
    CompareSmallImmediate(u8),
    ShiftImmediate {
        operation: ShiftOperation,
        count: u8,
    },
    ShiftAccumulator(ShiftOperation),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegisterOperation {
    pub size: OperandSize,
    pub register: RegisterEncoding,
    pub instruction: RegisterInstruction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceMemoryInstruction {
    Push,
    RotateLeftDigit,
    RotateRightDigit,
    LoadIncrement,
    LoadIncrementRepeat,
    LoadDecrement,
    LoadDecrementRepeat,
    CompareIncrement,
    CompareIncrementRepeat,
    CompareDecrement,
    CompareDecrementRepeat,
    LoadToDirect16(u16),
    LoadToCurrent(CurrentRegister),
    ExchangeCurrent(CurrentRegister),
    ArithmeticImmediate {
        operation: ArithmeticOperation,
        value: u32,
    },
    MultiplyCurrent {
        signed: bool,
        destination: CurrentRegister,
    },
    DivideCurrent {
        signed: bool,
        destination: CurrentRegister,
    },
    Increment(u8),
    Decrement(u8),
    Shift(ShiftOperation),
    ArithmeticCurrent {
        operation: ArithmeticOperation,
        current: CurrentRegister,
        destination_is_memory: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceMemoryOperation {
    pub size: OperandSize,
    pub source: MemoryAddress,
    pub instruction: SourceMemoryInstruction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestinationMemoryInstruction {
    LoadImmediateByte(u8),
    LoadImmediateWord(u16),
    PopByte,
    PopWord,
    LoadFromDirect16Byte(u16),
    LoadFromDirect16Word(u16),
    LoadAddressWord(CurrentRegister),
    LoadAddressLong(CurrentRegister),
    CarryAccumulator(BitOperation),
    LoadFromCurrent {
        size: OperandSize,
        source: CurrentRegister,
    },
    BitImmediate {
        operation: BitOperation,
        bit: u8,
    },
    Jump(Condition),
    Call(Condition),
    Return(Condition),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DestinationMemoryOperation {
    pub destination: MemoryAddress,
    pub instruction: DestinationMemoryInstruction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    Main(MainInstruction),
    Register(RegisterOperation),
    SourceMemory(SourceMemoryOperation),
    DestinationMemory(DestinationMemoryOperation),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MainForm {
    Nop,
    PushStatus,
    PopStatus,
    Halt,
    EnableInterrupt,
    DisableInterrupt,
    ReturnFromInterrupt,
    LoadDirect8Byte,
    PushImmediateByte,
    LoadDirect8Word,
    PushImmediateWord,
    IncrementRegisterFilePointer,
    DecrementRegisterFilePointer,
    Return,
    ReturnAndDeallocate,
    ResetCarry,
    SetCarry,
    ComplementCarry,
    ZeroToCarry,
    PushAccumulator,
    PopAccumulator,
    ExchangeFlags,
    LoadRegisterFilePointer,
    PushFlags,
    PopFlags,
    JumpDirect16,
    JumpDirect24,
    CallDirect16,
    CallDirect24,
    CallRelative,
    LoadCurrentImmediate(OperandSize),
    PushCurrent(OperandSize),
    PopCurrent(OperandSize),
    JumpRelative,
    JumpRelativeLong,
    LoadExtract,
    SoftwareInterrupt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RegisterForm {
    LoadImmediate,
    Push,
    Pop,
    Complement,
    Negate,
    MultiplyImmediate,
    MultiplySignedImmediate,
    DivideImmediate,
    DivideSignedImmediate,
    Link,
    Unlink,
    BitSearchForward,
    BitSearchBackward,
    DecimalAdjust,
    ExtendZero,
    ExtendSign,
    PointerAdjust,
    Mirror,
    MultiplyAccumulate,
    DecrementJump,
    CarryImmediate(BitOperation),
    CarryAccumulator(BitOperation),
    LoadControl,
    StoreControl,
    BitImmediate(BitOperation),
    ModuloIncrement(Step),
    ModuloDecrement(Step),
    MultiplyCurrent,
    MultiplySignedCurrent,
    DivideCurrent,
    DivideSignedCurrent,
    Increment,
    Decrement,
    SetCondition,
    ArithmeticCurrent(ArithmeticOperation),
    LoadFromCurrent,
    LoadToCurrent,
    LoadSmallImmediate,
    ExchangeCurrent,
    ArithmeticImmediate(ArithmeticOperation),
    CompareSmallImmediate,
    ShiftImmediate(ShiftOperation),
    ShiftAccumulator(ShiftOperation),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceMemoryForm {
    Push,
    RotateLeftDigit,
    RotateRightDigit,
    LoadIncrement,
    LoadIncrementRepeat,
    LoadDecrement,
    LoadDecrementRepeat,
    CompareIncrement,
    CompareIncrementRepeat,
    CompareDecrement,
    CompareDecrementRepeat,
    LoadToDirect16,
    LoadToCurrent,
    ExchangeCurrent,
    ArithmeticImmediate(ArithmeticOperation),
    MultiplyCurrent,
    MultiplySignedCurrent,
    DivideCurrent,
    DivideSignedCurrent,
    Increment,
    Decrement,
    Shift(ShiftOperation),
    ArithmeticCurrent {
        operation: ArithmeticOperation,
        destination_is_memory: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DestinationMemoryForm {
    LoadImmediateByte,
    LoadImmediateWord,
    PopByte,
    PopWord,
    LoadFromDirect16Byte,
    LoadFromDirect16Word,
    LoadAddressWord,
    LoadAddressLong,
    LoadAddressRelativeWord,
    LoadAddressRelativeLong,
    CarryAccumulator(BitOperation),
    LoadFromCurrent(OperandSize),
    BitImmediate(BitOperation),
    Jump,
    Call,
    Return,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InstructionForm {
    Main(MainForm),
    Register {
        size: OperandSize,
        form: RegisterForm,
    },
    SourceMemory {
        size: OperandSize,
        form: SourceMemoryForm,
    },
    DestinationMemory(DestinationMemoryForm),
}

impl Instruction {
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
