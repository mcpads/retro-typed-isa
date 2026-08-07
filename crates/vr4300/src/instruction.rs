use std::fmt;

/// One of the 32 VR4300 general-purpose registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GeneralRegister(u8);

impl GeneralRegister {
    pub const ZERO: Self = Self(0);
    pub const AT: Self = Self(1);
    pub const V0: Self = Self(2);
    pub const V1: Self = Self(3);
    pub const A0: Self = Self(4);
    pub const A1: Self = Self(5);
    pub const A2: Self = Self(6);
    pub const A3: Self = Self(7);
    pub const T0: Self = Self(8);
    pub const T1: Self = Self(9);
    pub const T2: Self = Self(10);
    pub const T3: Self = Self(11);
    pub const T4: Self = Self(12);
    pub const T5: Self = Self(13);
    pub const T6: Self = Self(14);
    pub const T7: Self = Self(15);
    pub const S0: Self = Self(16);
    pub const S1: Self = Self(17);
    pub const S2: Self = Self(18);
    pub const S3: Self = Self(19);
    pub const S4: Self = Self(20);
    pub const S5: Self = Self(21);
    pub const S6: Self = Self(22);
    pub const S7: Self = Self(23);
    pub const T8: Self = Self(24);
    pub const T9: Self = Self(25);
    pub const K0: Self = Self(26);
    pub const K1: Self = Self(27);
    pub const GP: Self = Self(28);
    pub const SP: Self = Self(29);
    pub const FP: Self = Self(30);
    pub const RA: Self = Self(31);

    pub const fn new(index: u8) -> Option<Self> {
        if index < 32 { Some(Self(index)) } else { None }
    }

    pub const fn index(self) -> u8 {
        self.0
    }

    pub(crate) const fn from_field(index: u8) -> Self {
        debug_assert!(index < 32);
        Self(index)
    }
}

impl fmt::Display for GeneralRegister {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "${}", self.0)
    }
}

macro_rules! define_numbered_register {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u8);

        impl $name {
            pub const fn new(index: u8) -> Option<Self> {
                if index < 32 { Some(Self(index)) } else { None }
            }

            pub const fn index(self) -> u8 {
                self.0
            }

            pub(crate) const fn from_field(index: u8) -> Self {
                debug_assert!(index < 32);
                Self(index)
            }
        }
    };
}

define_numbered_register!(
    FloatingRegister,
    "One of the 32 FPU general-purpose registers."
);
define_numbered_register!(
    Coprocessor2Register,
    "One of the 32 CP2 general-purpose register fields."
);
define_numbered_register!(
    Coprocessor2ControlRegister,
    "One of the 32 CP2 control-register fields."
);

/// A software-visible VR4300 CP0 register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Cp0Register {
    Index = 0,
    Random = 1,
    EntryLo0 = 2,
    EntryLo1 = 3,
    Context = 4,
    PageMask = 5,
    Wired = 6,
    BadVirtualAddress = 8,
    Count = 9,
    EntryHi = 10,
    Compare = 11,
    Status = 12,
    Cause = 13,
    ExceptionProgramCounter = 14,
    ProcessorRevision = 15,
    Config = 16,
    LoadLinkedAddress = 17,
    WatchLo = 18,
    WatchHi = 19,
    ExtendedContext = 20,
    ParityError = 26,
    CacheError = 27,
    TagLo = 28,
    TagHi = 29,
    ErrorExceptionProgramCounter = 30,
}

impl Cp0Register {
    pub const fn index(self) -> u8 {
        self as u8
    }

    pub(crate) const fn from_field(index: u8) -> Option<Self> {
        Some(match index {
            0 => Self::Index,
            1 => Self::Random,
            2 => Self::EntryLo0,
            3 => Self::EntryLo1,
            4 => Self::Context,
            5 => Self::PageMask,
            6 => Self::Wired,
            8 => Self::BadVirtualAddress,
            9 => Self::Count,
            10 => Self::EntryHi,
            11 => Self::Compare,
            12 => Self::Status,
            13 => Self::Cause,
            14 => Self::ExceptionProgramCounter,
            15 => Self::ProcessorRevision,
            16 => Self::Config,
            17 => Self::LoadLinkedAddress,
            18 => Self::WatchLo,
            19 => Self::WatchHi,
            20 => Self::ExtendedContext,
            26 => Self::ParityError,
            27 => Self::CacheError,
            28 => Self::TagLo,
            29 => Self::TagHi,
            30 => Self::ErrorExceptionProgramCounter,
            _ => return None,
        })
    }
}

/// One of the two FPU control registers defined by the VR4300 manual.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum FpuControlRegister {
    ImplementationRevision = 0,
    ControlStatus = 31,
}

impl FpuControlRegister {
    pub const fn index(self) -> u8 {
        self as u8
    }

    pub(crate) const fn from_field(index: u8) -> Option<Self> {
        match index {
            0 => Some(Self::ImplementationRevision),
            31 => Some(Self::ControlStatus),
            _ => None,
        }
    }
}

/// A five-bit cache operation selected by the CACHE instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CacheOperation(u8);

impl CacheOperation {
    pub const fn new(value: u8) -> Option<Self> {
        if value < 32 { Some(Self(value)) } else { None }
    }

    pub const fn value(self) -> u8 {
        self.0
    }

    pub(crate) const fn from_field(value: u8) -> Self {
        debug_assert!(value < 32);
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ThreeRegisterOperation {
    Add,
    AddUnsigned,
    Subtract,
    SubtractUnsigned,
    And,
    Or,
    ExclusiveOr,
    Nor,
    SetLessThan,
    SetLessThanUnsigned,
    DoublewordAdd,
    DoublewordAddUnsigned,
    DoublewordSubtract,
    DoublewordSubtractUnsigned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ImmediateArithmeticOperation {
    Add,
    AddUnsigned,
    SetLessThan,
    SetLessThanUnsigned,
    DoublewordAdd,
    DoublewordAddUnsigned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogicalImmediateOperation {
    And,
    Or,
    ExclusiveOr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShiftOperation {
    LeftLogical,
    RightLogical,
    RightArithmetic,
    DoublewordLeftLogical,
    DoublewordRightLogical,
    DoublewordRightArithmetic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MultiplyDivideOperation {
    Multiply,
    MultiplyUnsigned,
    Divide,
    DivideUnsigned,
    DoublewordMultiply,
    DoublewordMultiplyUnsigned,
    DoublewordDivide,
    DoublewordDivideUnsigned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SpecialRegister {
    Hi,
    Lo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MemoryOperation {
    LoadByte,
    LoadHalfword,
    LoadWordLeft,
    LoadWord,
    LoadByteUnsigned,
    LoadHalfwordUnsigned,
    LoadWordRight,
    LoadWordUnsigned,
    LoadDoublewordLeft,
    LoadDoublewordRight,
    LoadLinked,
    LoadLinkedDoubleword,
    LoadDoubleword,
    StoreByte,
    StoreHalfword,
    StoreWordLeft,
    StoreWord,
    StoreDoublewordLeft,
    StoreDoublewordRight,
    StoreWordRight,
    StoreConditional,
    StoreConditionalDoubleword,
    StoreDoubleword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EqualityCondition {
    Equal,
    NotEqual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SignCondition {
    LessThanZero,
    GreaterThanOrEqualZero,
    LessThanOrEqualZero,
    GreaterThanZero,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TrapCondition {
    GreaterThanOrEqual,
    GreaterThanOrEqualUnsigned,
    LessThan,
    LessThanUnsigned,
    Equal,
    NotEqual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Coprocessor {
    SystemControl,
    FloatingPoint,
    Coprocessor2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CoprocessorBranchCondition {
    False,
    True,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CoprocessorMemoryOperation {
    LoadWord,
    StoreWord,
    LoadDoubleword,
    StoreDoubleword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CoprocessorMemoryRegister {
    FloatingPoint(FloatingRegister),
    Coprocessor2(Coprocessor2Register),
}

impl CoprocessorMemoryRegister {
    pub const fn coprocessor(self) -> Coprocessor {
        match self {
            Self::FloatingPoint(_) => Coprocessor::FloatingPoint,
            Self::Coprocessor2(_) => Coprocessor::Coprocessor2,
        }
    }

    pub const fn index(self) -> u8 {
        match self {
            Self::FloatingPoint(register) => register.index(),
            Self::Coprocessor2(register) => register.index(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Cp0Operation {
    ReadIndexedTlbEntry,
    WriteIndexedTlbEntry,
    WriteRandomTlbEntry,
    ProbeTlb,
    ReturnFromException,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatingFormat {
    Single,
    Double,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FixedFormat {
    Word,
    Longword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatingBinaryOperation {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatingUnaryOperation {
    SquareRoot,
    AbsoluteValue,
    Move,
    Negate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatingRounding {
    NearestEven,
    TowardZero,
    TowardPositiveInfinity,
    TowardNegativeInfinity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum FloatingComparePredicate {
    False = 0,
    Unordered = 1,
    Equal = 2,
    UnorderedOrEqual = 3,
    OrderedLessThan = 4,
    UnorderedOrLessThan = 5,
    OrderedLessThanOrEqual = 6,
    UnorderedOrLessThanOrEqual = 7,
    SignalingFalse = 8,
    NotGreaterOrLessOrEqual = 9,
    SignalingEqual = 10,
    NotGreaterOrLess = 11,
    LessThan = 12,
    NotGreaterOrEqual = 13,
    LessThanOrEqual = 14,
    NotGreaterThan = 15,
}

impl FloatingComparePredicate {
    pub(crate) const fn from_field(value: u8) -> Self {
        match value {
            0 => Self::False,
            1 => Self::Unordered,
            2 => Self::Equal,
            3 => Self::UnorderedOrEqual,
            4 => Self::OrderedLessThan,
            5 => Self::UnorderedOrLessThan,
            6 => Self::OrderedLessThanOrEqual,
            7 => Self::UnorderedOrLessThanOrEqual,
            8 => Self::SignalingFalse,
            9 => Self::NotGreaterOrLessOrEqual,
            10 => Self::SignalingEqual,
            11 => Self::NotGreaterOrLess,
            12 => Self::LessThan,
            13 => Self::NotGreaterOrEqual,
            14 => Self::LessThanOrEqual,
            15 => Self::NotGreaterThan,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NumericFormat {
    Floating(FloatingFormat),
    Fixed(FixedFormat),
}

/// One semantically typed VR4300 instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    ThreeRegister {
        operation: ThreeRegisterOperation,
        destination: GeneralRegister,
        left: GeneralRegister,
        right: GeneralRegister,
    },
    ImmediateArithmetic {
        operation: ImmediateArithmeticOperation,
        destination: GeneralRegister,
        source: GeneralRegister,
        immediate: i16,
    },
    LogicalImmediate {
        operation: LogicalImmediateOperation,
        destination: GeneralRegister,
        source: GeneralRegister,
        immediate: u16,
    },
    LoadUpperImmediate {
        destination: GeneralRegister,
        immediate: u16,
    },
    ShiftImmediate {
        operation: ShiftOperation,
        destination: GeneralRegister,
        source: GeneralRegister,
        amount: u8,
    },
    ShiftVariable {
        operation: ShiftOperation,
        destination: GeneralRegister,
        source: GeneralRegister,
        amount_register: GeneralRegister,
    },
    MultiplyDivide {
        operation: MultiplyDivideOperation,
        left: GeneralRegister,
        right: GeneralRegister,
    },
    MoveFromSpecial {
        source: SpecialRegister,
        destination: GeneralRegister,
    },
    MoveToSpecial {
        source: GeneralRegister,
        destination: SpecialRegister,
    },
    Memory {
        operation: MemoryOperation,
        register: GeneralRegister,
        base: GeneralRegister,
        offset: i16,
    },
    EqualityBranch {
        condition: EqualityCondition,
        likely: bool,
        left: GeneralRegister,
        right: GeneralRegister,
        target: u64,
    },
    SignBranch {
        condition: SignCondition,
        likely: bool,
        link: bool,
        source: GeneralRegister,
        target: u64,
    },
    Jump {
        link: bool,
        target: u64,
    },
    JumpRegister {
        link: Option<GeneralRegister>,
        target: GeneralRegister,
    },
    SystemCall {
        code: u32,
    },
    Breakpoint {
        code: u32,
    },
    Synchronize,
    TrapRegister {
        condition: TrapCondition,
        left: GeneralRegister,
        right: GeneralRegister,
        code: u16,
    },
    TrapImmediate {
        condition: TrapCondition,
        source: GeneralRegister,
        immediate: i16,
    },
    MoveFromCp0 {
        doubleword: bool,
        destination: GeneralRegister,
        source: Cp0Register,
    },
    MoveToCp0 {
        doubleword: bool,
        source: GeneralRegister,
        destination: Cp0Register,
    },
    MoveFromFpu {
        doubleword: bool,
        destination: GeneralRegister,
        source: FloatingRegister,
    },
    MoveToFpu {
        doubleword: bool,
        source: GeneralRegister,
        destination: FloatingRegister,
    },
    MoveControlFromFpu {
        destination: GeneralRegister,
        source: FpuControlRegister,
    },
    MoveControlToFpu {
        source: GeneralRegister,
        destination: FpuControlRegister,
    },
    MoveFromCoprocessor2 {
        doubleword: bool,
        destination: GeneralRegister,
        source: Coprocessor2Register,
    },
    MoveToCoprocessor2 {
        doubleword: bool,
        source: GeneralRegister,
        destination: Coprocessor2Register,
    },
    MoveControlFromCoprocessor2 {
        destination: GeneralRegister,
        source: Coprocessor2ControlRegister,
    },
    MoveControlToCoprocessor2 {
        source: GeneralRegister,
        destination: Coprocessor2ControlRegister,
    },
    CoprocessorBranch {
        coprocessor: Coprocessor,
        condition: CoprocessorBranchCondition,
        likely: bool,
        target: u64,
    },
    CoprocessorMemory {
        operation: CoprocessorMemoryOperation,
        register: CoprocessorMemoryRegister,
        base: GeneralRegister,
        offset: i16,
    },
    Cp0(Cp0Operation),
    Cache {
        operation: CacheOperation,
        base: GeneralRegister,
        offset: i16,
    },
    FloatingBinary {
        operation: FloatingBinaryOperation,
        format: FloatingFormat,
        destination: FloatingRegister,
        left: FloatingRegister,
        right: FloatingRegister,
    },
    FloatingUnary {
        operation: FloatingUnaryOperation,
        format: FloatingFormat,
        destination: FloatingRegister,
        source: FloatingRegister,
    },
    FloatingRoundToInteger {
        rounding: FloatingRounding,
        destination_format: FixedFormat,
        source_format: FloatingFormat,
        destination: FloatingRegister,
        source: FloatingRegister,
    },
    FloatingConvert {
        destination_format: NumericFormat,
        source_format: NumericFormat,
        destination: FloatingRegister,
        source: FloatingRegister,
    },
    FloatingCompare {
        predicate: FloatingComparePredicate,
        format: FloatingFormat,
        left: FloatingRegister,
        right: FloatingRegister,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InstructionForm {
    ThreeRegister(ThreeRegisterOperation),
    ImmediateArithmetic(ImmediateArithmeticOperation),
    LogicalImmediate(LogicalImmediateOperation),
    LoadUpperImmediate,
    ShiftImmediate {
        operation: ShiftOperation,
        high_half: bool,
    },
    ShiftVariable(ShiftOperation),
    MultiplyDivide(MultiplyDivideOperation),
    MoveFromSpecial(SpecialRegister),
    MoveToSpecial(SpecialRegister),
    Memory(MemoryOperation),
    EqualityBranch {
        condition: EqualityCondition,
        likely: bool,
    },
    SignBranch {
        condition: SignCondition,
        likely: bool,
        link: bool,
    },
    Jump {
        link: bool,
    },
    JumpRegister {
        link: bool,
    },
    SystemCall,
    Breakpoint,
    Synchronize,
    TrapRegister(TrapCondition),
    TrapImmediate(TrapCondition),
    MoveFromCp0 {
        doubleword: bool,
    },
    MoveToCp0 {
        doubleword: bool,
    },
    MoveFromFpu {
        doubleword: bool,
    },
    MoveToFpu {
        doubleword: bool,
    },
    MoveControlFromFpu,
    MoveControlToFpu,
    MoveFromCoprocessor2 {
        doubleword: bool,
    },
    MoveToCoprocessor2 {
        doubleword: bool,
    },
    MoveControlFromCoprocessor2,
    MoveControlToCoprocessor2,
    CoprocessorBranch {
        coprocessor: Coprocessor,
        condition: CoprocessorBranchCondition,
        likely: bool,
    },
    CoprocessorMemory {
        coprocessor: Coprocessor,
        operation: CoprocessorMemoryOperation,
    },
    Cp0(Cp0Operation),
    Cache,
    FloatingBinary {
        operation: FloatingBinaryOperation,
        format: FloatingFormat,
    },
    FloatingUnary {
        operation: FloatingUnaryOperation,
        format: FloatingFormat,
    },
    FloatingRoundToInteger {
        rounding: FloatingRounding,
        destination_format: FixedFormat,
        source_format: FloatingFormat,
    },
    FloatingConvert {
        destination_format: NumericFormat,
        source_format: NumericFormat,
    },
    FloatingCompare {
        predicate: FloatingComparePredicate,
        format: FloatingFormat,
    },
}

impl Instruction {
    pub const fn form(self) -> InstructionForm {
        match self {
            Self::ThreeRegister { operation, .. } => InstructionForm::ThreeRegister(operation),
            Self::ImmediateArithmetic { operation, .. } => {
                InstructionForm::ImmediateArithmetic(operation)
            }
            Self::LogicalImmediate { operation, .. } => {
                InstructionForm::LogicalImmediate(operation)
            }
            Self::LoadUpperImmediate { .. } => InstructionForm::LoadUpperImmediate,
            Self::ShiftImmediate {
                operation, amount, ..
            } => InstructionForm::ShiftImmediate {
                operation,
                high_half: matches!(
                    operation,
                    ShiftOperation::DoublewordLeftLogical
                        | ShiftOperation::DoublewordRightLogical
                        | ShiftOperation::DoublewordRightArithmetic
                ) && amount >= 32,
            },
            Self::ShiftVariable { operation, .. } => InstructionForm::ShiftVariable(operation),
            Self::MultiplyDivide { operation, .. } => InstructionForm::MultiplyDivide(operation),
            Self::MoveFromSpecial { source, .. } => InstructionForm::MoveFromSpecial(source),
            Self::MoveToSpecial { destination, .. } => InstructionForm::MoveToSpecial(destination),
            Self::Memory { operation, .. } => InstructionForm::Memory(operation),
            Self::EqualityBranch {
                condition, likely, ..
            } => InstructionForm::EqualityBranch { condition, likely },
            Self::SignBranch {
                condition,
                likely,
                link,
                ..
            } => InstructionForm::SignBranch {
                condition,
                likely,
                link,
            },
            Self::Jump { link, .. } => InstructionForm::Jump { link },
            Self::JumpRegister { link, .. } => InstructionForm::JumpRegister {
                link: link.is_some(),
            },
            Self::SystemCall { .. } => InstructionForm::SystemCall,
            Self::Breakpoint { .. } => InstructionForm::Breakpoint,
            Self::Synchronize => InstructionForm::Synchronize,
            Self::TrapRegister { condition, .. } => InstructionForm::TrapRegister(condition),
            Self::TrapImmediate { condition, .. } => InstructionForm::TrapImmediate(condition),
            Self::MoveFromCp0 { doubleword, .. } => InstructionForm::MoveFromCp0 { doubleword },
            Self::MoveToCp0 { doubleword, .. } => InstructionForm::MoveToCp0 { doubleword },
            Self::MoveFromFpu { doubleword, .. } => InstructionForm::MoveFromFpu { doubleword },
            Self::MoveToFpu { doubleword, .. } => InstructionForm::MoveToFpu { doubleword },
            Self::MoveControlFromFpu { .. } => InstructionForm::MoveControlFromFpu,
            Self::MoveControlToFpu { .. } => InstructionForm::MoveControlToFpu,
            Self::MoveFromCoprocessor2 { doubleword, .. } => {
                InstructionForm::MoveFromCoprocessor2 { doubleword }
            }
            Self::MoveToCoprocessor2 { doubleword, .. } => {
                InstructionForm::MoveToCoprocessor2 { doubleword }
            }
            Self::MoveControlFromCoprocessor2 { .. } => {
                InstructionForm::MoveControlFromCoprocessor2
            }
            Self::MoveControlToCoprocessor2 { .. } => InstructionForm::MoveControlToCoprocessor2,
            Self::CoprocessorBranch {
                coprocessor,
                condition,
                likely,
                ..
            } => InstructionForm::CoprocessorBranch {
                coprocessor,
                condition,
                likely,
            },
            Self::CoprocessorMemory {
                operation,
                register,
                ..
            } => InstructionForm::CoprocessorMemory {
                coprocessor: register.coprocessor(),
                operation,
            },
            Self::Cp0(operation) => InstructionForm::Cp0(operation),
            Self::Cache { .. } => InstructionForm::Cache,
            Self::FloatingBinary {
                operation, format, ..
            } => InstructionForm::FloatingBinary { operation, format },
            Self::FloatingUnary {
                operation, format, ..
            } => InstructionForm::FloatingUnary { operation, format },
            Self::FloatingRoundToInteger {
                rounding,
                destination_format,
                source_format,
                ..
            } => InstructionForm::FloatingRoundToInteger {
                rounding,
                destination_format,
                source_format,
            },
            Self::FloatingConvert {
                destination_format,
                source_format,
                ..
            } => InstructionForm::FloatingConvert {
                destination_format,
                source_format,
            },
            Self::FloatingCompare {
                predicate, format, ..
            } => InstructionForm::FloatingCompare { predicate, format },
        }
    }
}
