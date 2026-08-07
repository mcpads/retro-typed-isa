use std::fmt;

/// One of the 32 VR4300 general-purpose registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GeneralRegister(u8);

impl GeneralRegister {
    /// The constant-zero register, `$0`.
    pub const ZERO: Self = Self(0);
    /// The assembler-temporary register, `$1`.
    pub const AT: Self = Self(1);
    /// The first function-result register, `$2`.
    pub const V0: Self = Self(2);
    /// The second function-result register, `$3`.
    pub const V1: Self = Self(3);
    /// The first argument register, `$4`.
    pub const A0: Self = Self(4);
    /// The second argument register, `$5`.
    pub const A1: Self = Self(5);
    /// The third argument register, `$6`.
    pub const A2: Self = Self(6);
    /// The fourth argument register, `$7`.
    pub const A3: Self = Self(7);
    /// Temporary register `$8`.
    pub const T0: Self = Self(8);
    /// Temporary register `$9`.
    pub const T1: Self = Self(9);
    /// Temporary register `$10`.
    pub const T2: Self = Self(10);
    /// Temporary register `$11`.
    pub const T3: Self = Self(11);
    /// Temporary register `$12`.
    pub const T4: Self = Self(12);
    /// Temporary register `$13`.
    pub const T5: Self = Self(13);
    /// Temporary register `$14`.
    pub const T6: Self = Self(14);
    /// Temporary register `$15`.
    pub const T7: Self = Self(15);
    /// Saved register `$16`.
    pub const S0: Self = Self(16);
    /// Saved register `$17`.
    pub const S1: Self = Self(17);
    /// Saved register `$18`.
    pub const S2: Self = Self(18);
    /// Saved register `$19`.
    pub const S3: Self = Self(19);
    /// Saved register `$20`.
    pub const S4: Self = Self(20);
    /// Saved register `$21`.
    pub const S5: Self = Self(21);
    /// Saved register `$22`.
    pub const S6: Self = Self(22);
    /// Saved register `$23`.
    pub const S7: Self = Self(23);
    /// Temporary register `$24`.
    pub const T8: Self = Self(24);
    /// Temporary register `$25`.
    pub const T9: Self = Self(25);
    /// Kernel-reserved register `$26`.
    pub const K0: Self = Self(26);
    /// Kernel-reserved register `$27`.
    pub const K1: Self = Self(27);
    /// The global pointer, `$28`.
    pub const GP: Self = Self(28);
    /// The stack pointer, `$29`.
    pub const SP: Self = Self(29);
    /// The frame pointer, `$30`.
    pub const FP: Self = Self(30);
    /// The return-address register, `$31`.
    pub const RA: Self = Self(31);

    /// Constructs the value when the numeric field fits its five-bit encoding.
    pub const fn new(index: u8) -> Option<Self> {
        if index < 32 { Some(Self(index)) } else { None }
    }

    /// Returns the five-bit architectural register index.
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
            /// Constructs the value when the numeric field fits its five-bit encoding.
            pub const fn new(index: u8) -> Option<Self> {
                if index < 32 { Some(Self(index)) } else { None }
            }

            /// Returns the five-bit architectural register index.
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
    /// The index form.
    Index = 0,
    /// The random form.
    Random = 1,
    /// The entry lo0 form.
    EntryLo0 = 2,
    /// The entry lo1 form.
    EntryLo1 = 3,
    /// The context form.
    Context = 4,
    /// The page mask form.
    PageMask = 5,
    /// The wired form.
    Wired = 6,
    /// The bad virtual address form.
    BadVirtualAddress = 8,
    /// The count form.
    Count = 9,
    /// The entry hi form.
    EntryHi = 10,
    /// The compare form.
    Compare = 11,
    /// The status form.
    Status = 12,
    /// The cause form.
    Cause = 13,
    /// The exception program counter form.
    ExceptionProgramCounter = 14,
    /// The processor revision form.
    ProcessorRevision = 15,
    /// The config form.
    Config = 16,
    /// The load linked address form.
    LoadLinkedAddress = 17,
    /// The watch lo form.
    WatchLo = 18,
    /// The watch hi form.
    WatchHi = 19,
    /// The extended context form.
    ExtendedContext = 20,
    /// The parity error form.
    ParityError = 26,
    /// The cache error form.
    CacheError = 27,
    /// The tag lo form.
    TagLo = 28,
    /// The tag hi form.
    TagHi = 29,
    /// The error exception program counter form.
    ErrorExceptionProgramCounter = 30,
}

impl Cp0Register {
    /// Returns the five-bit architectural register index.
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
    /// The implementation revision form.
    ImplementationRevision = 0,
    /// The control status form.
    ControlStatus = 31,
}

impl FpuControlRegister {
    /// Returns the five-bit architectural register index.
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
    /// Constructs the value when the numeric field fits its five-bit encoding.
    pub const fn new(value: u8) -> Option<Self> {
        if value < 32 { Some(Self(value)) } else { None }
    }

    /// Returns the five-bit cache-operation value.
    pub const fn value(self) -> u8 {
        self.0
    }

    pub(crate) const fn from_field(value: u8) -> Self {
        debug_assert!(value < 32);
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A three-register integer or logical operation.
pub enum ThreeRegisterOperation {
    /// The add form.
    Add,
    /// The add unsigned form.
    AddUnsigned,
    /// The subtract form.
    Subtract,
    /// The subtract unsigned form.
    SubtractUnsigned,
    /// The and form.
    And,
    /// The or form.
    Or,
    /// The exclusive or form.
    ExclusiveOr,
    /// The nor form.
    Nor,
    /// The set less than form.
    SetLessThan,
    /// The set less than unsigned form.
    SetLessThanUnsigned,
    /// The doubleword add form.
    DoublewordAdd,
    /// The doubleword add unsigned form.
    DoublewordAddUnsigned,
    /// The doubleword subtract form.
    DoublewordSubtract,
    /// The doubleword subtract unsigned form.
    DoublewordSubtractUnsigned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A sign-extended-immediate arithmetic operation.
pub enum ImmediateArithmeticOperation {
    /// The add form.
    Add,
    /// The add unsigned form.
    AddUnsigned,
    /// The set less than form.
    SetLessThan,
    /// The set less than unsigned form.
    SetLessThanUnsigned,
    /// The doubleword add form.
    DoublewordAdd,
    /// The doubleword add unsigned form.
    DoublewordAddUnsigned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A zero-extended-immediate logical operation.
pub enum LogicalImmediateOperation {
    /// The and form.
    And,
    /// The or form.
    Or,
    /// The exclusive or form.
    ExclusiveOr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A 32- or 64-bit shift operation.
pub enum ShiftOperation {
    /// The left logical form.
    LeftLogical,
    /// The right logical form.
    RightLogical,
    /// The right arithmetic form.
    RightArithmetic,
    /// The doubleword left logical form.
    DoublewordLeftLogical,
    /// The doubleword right logical form.
    DoublewordRightLogical,
    /// The doubleword right arithmetic form.
    DoublewordRightArithmetic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A multiply or divide operation that writes the special result registers.
pub enum MultiplyDivideOperation {
    /// The multiply form.
    Multiply,
    /// The multiply unsigned form.
    MultiplyUnsigned,
    /// The divide form.
    Divide,
    /// The divide unsigned form.
    DivideUnsigned,
    /// The doubleword multiply form.
    DoublewordMultiply,
    /// The doubleword multiply unsigned form.
    DoublewordMultiplyUnsigned,
    /// The doubleword divide form.
    DoublewordDivide,
    /// The doubleword divide unsigned form.
    DoublewordDivideUnsigned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The special multiply/divide result register selected by a move.
pub enum SpecialRegister {
    /// The hi form.
    Hi,
    /// The lo form.
    Lo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A CPU load, store, or atomic memory operation.
pub enum MemoryOperation {
    /// The load byte form.
    LoadByte,
    /// The load halfword form.
    LoadHalfword,
    /// The load word left form.
    LoadWordLeft,
    /// The load word form.
    LoadWord,
    /// The load byte unsigned form.
    LoadByteUnsigned,
    /// The load halfword unsigned form.
    LoadHalfwordUnsigned,
    /// The load word right form.
    LoadWordRight,
    /// The load word unsigned form.
    LoadWordUnsigned,
    /// The load doubleword left form.
    LoadDoublewordLeft,
    /// The load doubleword right form.
    LoadDoublewordRight,
    /// The load linked form.
    LoadLinked,
    /// The load linked doubleword form.
    LoadLinkedDoubleword,
    /// The load doubleword form.
    LoadDoubleword,
    /// The store byte form.
    StoreByte,
    /// The store halfword form.
    StoreHalfword,
    /// The store word left form.
    StoreWordLeft,
    /// The store word form.
    StoreWord,
    /// The store doubleword left form.
    StoreDoublewordLeft,
    /// The store doubleword right form.
    StoreDoublewordRight,
    /// The store word right form.
    StoreWordRight,
    /// The store conditional form.
    StoreConditional,
    /// The store conditional doubleword form.
    StoreConditionalDoubleword,
    /// The store doubleword form.
    StoreDoubleword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The equality relation selected by an integer branch.
pub enum EqualityCondition {
    /// The equal form.
    Equal,
    /// The not equal form.
    NotEqual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The signed zero relation selected by a register-immediate branch.
pub enum SignCondition {
    /// The less than zero form.
    LessThanZero,
    /// The greater than or equal zero form.
    GreaterThanOrEqualZero,
    /// The less than or equal zero form.
    LessThanOrEqualZero,
    /// The greater than zero form.
    GreaterThanZero,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The integer relation selected by a conditional trap.
pub enum TrapCondition {
    /// The greater than or equal form.
    GreaterThanOrEqual,
    /// The greater than or equal unsigned form.
    GreaterThanOrEqualUnsigned,
    /// The less than form.
    LessThan,
    /// The less than unsigned form.
    LessThanUnsigned,
    /// The equal form.
    Equal,
    /// The not equal form.
    NotEqual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A coprocessor selector admitted by the VR4300 profile.
pub enum Coprocessor {
    /// The system control form.
    SystemControl,
    /// The floating point form.
    FloatingPoint,
    /// The coprocessor2 form.
    Coprocessor2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The coprocessor condition bit selected by a branch.
pub enum CoprocessorBranchCondition {
    /// The false form.
    False,
    /// The true form.
    True,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The width selected by a coprocessor load or store.
pub enum CoprocessorMemoryOperation {
    /// The load word form.
    LoadWord,
    /// The store word form.
    StoreWord,
    /// The load doubleword form.
    LoadDoubleword,
    /// The store doubleword form.
    StoreDoubleword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A typed FPU or CP2 register used by a coprocessor memory transfer.
pub enum CoprocessorMemoryRegister {
    /// The floating point form.
    FloatingPoint(FloatingRegister),
    /// The coprocessor2 form.
    Coprocessor2(Coprocessor2Register),
}

impl CoprocessorMemoryRegister {
    /// Returns the coprocessor selected by this typed memory register.
    pub const fn coprocessor(self) -> Coprocessor {
        match self {
            Self::FloatingPoint(_) => Coprocessor::FloatingPoint,
            Self::Coprocessor2(_) => Coprocessor::Coprocessor2,
        }
    }

    /// Returns the five-bit architectural register index.
    pub const fn index(self) -> u8 {
        match self {
            Self::FloatingPoint(register) => register.index(),
            Self::Coprocessor2(register) => register.index(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A privileged CP0 operation with a defined VR4300 encoding.
pub enum Cp0Operation {
    /// The read indexed tlb entry form.
    ReadIndexedTlbEntry,
    /// The write indexed tlb entry form.
    WriteIndexedTlbEntry,
    /// The write random tlb entry form.
    WriteRandomTlbEntry,
    /// The probe tlb form.
    ProbeTlb,
    /// The return from exception form.
    ReturnFromException,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The single- or double-precision source format of an FPU arithmetic operation.
pub enum FloatingFormat {
    /// The single form.
    Single,
    /// The double form.
    Double,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The 32- or 64-bit fixed-point FPU format.
pub enum FixedFormat {
    /// The word form.
    Word,
    /// The longword form.
    Longword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A two-source floating-point arithmetic operation.
pub enum FloatingBinaryOperation {
    /// The add form.
    Add,
    /// The subtract form.
    Subtract,
    /// The multiply form.
    Multiply,
    /// The divide form.
    Divide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A one-source floating-point arithmetic operation.
pub enum FloatingUnaryOperation {
    /// The square root form.
    SquareRoot,
    /// The absolute value form.
    AbsoluteValue,
    /// The move form.
    Move,
    /// The negate form.
    Negate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// An explicit floating-point rounding operation and integer result width.
pub enum FloatingRounding {
    /// The nearest even form.
    NearestEven,
    /// The toward zero form.
    TowardZero,
    /// The toward positive infinity form.
    TowardPositiveInfinity,
    /// The toward negative infinity form.
    TowardNegativeInfinity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
/// A VR4300 floating-point compare predicate.
pub enum FloatingComparePredicate {
    /// The false form.
    False = 0,
    /// The unordered form.
    Unordered = 1,
    /// The equal form.
    Equal = 2,
    /// The unordered or equal form.
    UnorderedOrEqual = 3,
    /// The ordered less than form.
    OrderedLessThan = 4,
    /// The unordered or less than form.
    UnorderedOrLessThan = 5,
    /// The ordered less than or equal form.
    OrderedLessThanOrEqual = 6,
    /// The unordered or less than or equal form.
    UnorderedOrLessThanOrEqual = 7,
    /// The signaling false form.
    SignalingFalse = 8,
    /// The not greater or less or equal form.
    NotGreaterOrLessOrEqual = 9,
    /// The signaling equal form.
    SignalingEqual = 10,
    /// The not greater or less form.
    NotGreaterOrLess = 11,
    /// The less than form.
    LessThan = 12,
    /// The not greater or equal form.
    NotGreaterOrEqual = 13,
    /// The less than or equal form.
    LessThanOrEqual = 14,
    /// The not greater than form.
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
/// A numeric format accepted by the VR4300 conversion instruction.
pub enum NumericFormat {
    /// The floating form.
    Floating(FloatingFormat),
    /// The fixed form.
    Fixed(FixedFormat),
}

/// One semantically typed VR4300 instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    /// The three register form.
    ThreeRegister {
        /// The selected operation.
        operation: ThreeRegisterOperation,
        /// The destination register.
        destination: GeneralRegister,
        /// The left integer or floating-point source.
        left: GeneralRegister,
        /// The right integer or floating-point source.
        right: GeneralRegister,
    },
    /// The immediate arithmetic form.
    ImmediateArithmetic {
        /// The selected operation.
        operation: ImmediateArithmeticOperation,
        /// The destination register.
        destination: GeneralRegister,
        /// The source register or operand.
        source: GeneralRegister,
        /// The encoded immediate operand.
        immediate: i16,
    },
    /// The logical immediate form.
    LogicalImmediate {
        /// The selected operation.
        operation: LogicalImmediateOperation,
        /// The destination register.
        destination: GeneralRegister,
        /// The source register or operand.
        source: GeneralRegister,
        /// The encoded immediate operand.
        immediate: u16,
    },
    /// The load upper immediate form.
    LoadUpperImmediate {
        /// The destination register.
        destination: GeneralRegister,
        /// The encoded immediate operand.
        immediate: u16,
    },
    /// The shift immediate form.
    ShiftImmediate {
        /// The selected operation.
        operation: ShiftOperation,
        /// The destination register.
        destination: GeneralRegister,
        /// The source register or operand.
        source: GeneralRegister,
        /// The shift amount or source register.
        amount: u8,
    },
    /// The shift variable form.
    ShiftVariable {
        /// The selected operation.
        operation: ShiftOperation,
        /// The destination register.
        destination: GeneralRegister,
        /// The source register or operand.
        source: GeneralRegister,
        /// The amount register operand.
        amount_register: GeneralRegister,
    },
    /// The multiply divide form.
    MultiplyDivide {
        /// The selected operation.
        operation: MultiplyDivideOperation,
        /// The left integer or floating-point source.
        left: GeneralRegister,
        /// The right integer or floating-point source.
        right: GeneralRegister,
    },
    /// The move from special form.
    MoveFromSpecial {
        /// The source register or operand.
        source: SpecialRegister,
        /// The destination register.
        destination: GeneralRegister,
    },
    /// The move to special form.
    MoveToSpecial {
        /// The source register or operand.
        source: GeneralRegister,
        /// The destination register.
        destination: SpecialRegister,
    },
    /// The memory form.
    Memory {
        /// The selected operation.
        operation: MemoryOperation,
        /// The selected architectural register.
        register: GeneralRegister,
        /// The base address register.
        base: GeneralRegister,
        /// The signed byte-address displacement.
        offset: i16,
    },
    /// The equality branch form.
    EqualityBranch {
        /// The relation or coprocessor condition tested by the instruction.
        condition: EqualityCondition,
        /// Whether the branch-likely form annuls its delay slot when not taken.
        likely: bool,
        /// The left integer or floating-point source.
        left: GeneralRegister,
        /// The right integer or floating-point source.
        right: GeneralRegister,
        /// The absolute control-flow target address.
        target: u64,
    },
    /// The sign branch form.
    SignBranch {
        /// The relation or coprocessor condition tested by the instruction.
        condition: SignCondition,
        /// Whether the branch-likely form annuls its delay slot when not taken.
        likely: bool,
        /// Whether the instruction records a return address.
        link: bool,
        /// The source register or operand.
        source: GeneralRegister,
        /// The absolute control-flow target address.
        target: u64,
    },
    /// The jump form.
    Jump {
        /// Whether the instruction records a return address.
        link: bool,
        /// The absolute control-flow target address.
        target: u64,
    },
    /// The jump register form.
    JumpRegister {
        /// Whether the instruction records a return address.
        link: Option<GeneralRegister>,
        /// The absolute control-flow target address.
        target: GeneralRegister,
    },
    /// The system call form.
    SystemCall {
        /// The system or trap code field.
        code: u32,
    },
    /// The breakpoint form.
    Breakpoint {
        /// The system or trap code field.
        code: u32,
    },
    /// The synchronize form.
    Synchronize,
    /// The trap register form.
    TrapRegister {
        /// The relation or coprocessor condition tested by the instruction.
        condition: TrapCondition,
        /// The left integer or floating-point source.
        left: GeneralRegister,
        /// The right integer or floating-point source.
        right: GeneralRegister,
        /// The system or trap code field.
        code: u16,
    },
    /// The trap immediate form.
    TrapImmediate {
        /// The relation or coprocessor condition tested by the instruction.
        condition: TrapCondition,
        /// The source register or operand.
        source: GeneralRegister,
        /// The encoded immediate operand.
        immediate: i16,
    },
    /// The move from cp0 form.
    MoveFromCp0 {
        /// The doubleword operand.
        doubleword: bool,
        /// The destination register.
        destination: GeneralRegister,
        /// The source register or operand.
        source: Cp0Register,
    },
    /// The move to cp0 form.
    MoveToCp0 {
        /// The doubleword operand.
        doubleword: bool,
        /// The source register or operand.
        source: GeneralRegister,
        /// The destination register.
        destination: Cp0Register,
    },
    /// The move from fpu form.
    MoveFromFpu {
        /// The doubleword operand.
        doubleword: bool,
        /// The destination register.
        destination: GeneralRegister,
        /// The source register or operand.
        source: FloatingRegister,
    },
    /// The move to fpu form.
    MoveToFpu {
        /// The doubleword operand.
        doubleword: bool,
        /// The source register or operand.
        source: GeneralRegister,
        /// The destination register.
        destination: FloatingRegister,
    },
    /// The move control from fpu form.
    MoveControlFromFpu {
        /// The destination register.
        destination: GeneralRegister,
        /// The source register or operand.
        source: FpuControlRegister,
    },
    /// The move control to fpu form.
    MoveControlToFpu {
        /// The source register or operand.
        source: GeneralRegister,
        /// The destination register.
        destination: FpuControlRegister,
    },
    /// The move from coprocessor2 form.
    MoveFromCoprocessor2 {
        /// The doubleword operand.
        doubleword: bool,
        /// The destination register.
        destination: GeneralRegister,
        /// The source register or operand.
        source: Coprocessor2Register,
    },
    /// The move to coprocessor2 form.
    MoveToCoprocessor2 {
        /// The doubleword operand.
        doubleword: bool,
        /// The source register or operand.
        source: GeneralRegister,
        /// The destination register.
        destination: Coprocessor2Register,
    },
    /// The move control from coprocessor2 form.
    MoveControlFromCoprocessor2 {
        /// The destination register.
        destination: GeneralRegister,
        /// The source register or operand.
        source: Coprocessor2ControlRegister,
    },
    /// The move control to coprocessor2 form.
    MoveControlToCoprocessor2 {
        /// The source register or operand.
        source: GeneralRegister,
        /// The destination register.
        destination: Coprocessor2ControlRegister,
    },
    /// The coprocessor branch form.
    CoprocessorBranch {
        /// The selected coprocessor.
        coprocessor: Coprocessor,
        /// The relation or coprocessor condition tested by the instruction.
        condition: CoprocessorBranchCondition,
        /// Whether the branch-likely form annuls its delay slot when not taken.
        likely: bool,
        /// The absolute control-flow target address.
        target: u64,
    },
    /// The coprocessor memory form.
    CoprocessorMemory {
        /// The selected operation.
        operation: CoprocessorMemoryOperation,
        /// The selected architectural register.
        register: CoprocessorMemoryRegister,
        /// The base address register.
        base: GeneralRegister,
        /// The signed byte-address displacement.
        offset: i16,
    },
    /// The cp0 form.
    Cp0(Cp0Operation),
    /// The cache form.
    Cache {
        /// The selected operation.
        operation: CacheOperation,
        /// The base address register.
        base: GeneralRegister,
        /// The signed byte-address displacement.
        offset: i16,
    },
    /// The floating binary form.
    FloatingBinary {
        /// The selected operation.
        operation: FloatingBinaryOperation,
        /// The source numeric format.
        format: FloatingFormat,
        /// The destination register.
        destination: FloatingRegister,
        /// The left integer or floating-point source.
        left: FloatingRegister,
        /// The right integer or floating-point source.
        right: FloatingRegister,
    },
    /// The floating unary form.
    FloatingUnary {
        /// The selected operation.
        operation: FloatingUnaryOperation,
        /// The source numeric format.
        format: FloatingFormat,
        /// The destination register.
        destination: FloatingRegister,
        /// The source register or operand.
        source: FloatingRegister,
    },
    /// The floating round to integer form.
    FloatingRoundToInteger {
        /// The rounding operand.
        rounding: FloatingRounding,
        /// The destination format operand.
        destination_format: FixedFormat,
        /// The source format operand.
        source_format: FloatingFormat,
        /// The destination register.
        destination: FloatingRegister,
        /// The source register or operand.
        source: FloatingRegister,
    },
    /// The floating convert form.
    FloatingConvert {
        /// The destination format operand.
        destination_format: NumericFormat,
        /// The source format operand.
        source_format: NumericFormat,
        /// The destination register.
        destination: FloatingRegister,
        /// The source register or operand.
        source: FloatingRegister,
    },
    /// The floating compare form.
    FloatingCompare {
        /// The selected floating-point comparison predicate.
        predicate: FloatingComparePredicate,
        /// The source numeric format.
        format: FloatingFormat,
        /// The left integer or floating-point source.
        left: FloatingRegister,
        /// The right integer or floating-point source.
        right: FloatingRegister,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The operand-independent instruction form used by static analysis.
pub enum InstructionForm {
    /// The three register form.
    ThreeRegister(ThreeRegisterOperation),
    /// The immediate arithmetic form.
    ImmediateArithmetic(ImmediateArithmeticOperation),
    /// The logical immediate form.
    LogicalImmediate(LogicalImmediateOperation),
    /// The load upper immediate form.
    LoadUpperImmediate,
    /// The shift immediate form.
    ShiftImmediate {
        /// The selected operation.
        operation: ShiftOperation,
        /// The high half operand.
        high_half: bool,
    },
    /// The shift variable form.
    ShiftVariable(ShiftOperation),
    /// The multiply divide form.
    MultiplyDivide(MultiplyDivideOperation),
    /// The move from special form.
    MoveFromSpecial(SpecialRegister),
    /// The move to special form.
    MoveToSpecial(SpecialRegister),
    /// The memory form.
    Memory(MemoryOperation),
    /// The equality branch form.
    EqualityBranch {
        /// The relation or coprocessor condition tested by the instruction.
        condition: EqualityCondition,
        /// Whether the branch-likely form annuls its delay slot when not taken.
        likely: bool,
    },
    /// The sign branch form.
    SignBranch {
        /// The relation or coprocessor condition tested by the instruction.
        condition: SignCondition,
        /// Whether the branch-likely form annuls its delay slot when not taken.
        likely: bool,
        /// Whether the instruction records a return address.
        link: bool,
    },
    /// The jump form.
    Jump {
        /// Whether the instruction records a return address.
        link: bool,
    },
    /// The jump register form.
    JumpRegister {
        /// Whether the instruction records a return address.
        link: bool,
    },
    /// The system call form.
    SystemCall,
    /// The breakpoint form.
    Breakpoint,
    /// The synchronize form.
    Synchronize,
    /// The trap register form.
    TrapRegister(TrapCondition),
    /// The trap immediate form.
    TrapImmediate(TrapCondition),
    /// The move from cp0 form.
    MoveFromCp0 {
        /// The doubleword operand.
        doubleword: bool,
    },
    /// The move to cp0 form.
    MoveToCp0 {
        /// The doubleword operand.
        doubleword: bool,
    },
    /// The move from fpu form.
    MoveFromFpu {
        /// The doubleword operand.
        doubleword: bool,
    },
    /// The move to fpu form.
    MoveToFpu {
        /// The doubleword operand.
        doubleword: bool,
    },
    /// The move control from fpu form.
    MoveControlFromFpu,
    /// The move control to fpu form.
    MoveControlToFpu,
    /// The move from coprocessor2 form.
    MoveFromCoprocessor2 {
        /// The doubleword operand.
        doubleword: bool,
    },
    /// The move to coprocessor2 form.
    MoveToCoprocessor2 {
        /// The doubleword operand.
        doubleword: bool,
    },
    /// The move control from coprocessor2 form.
    MoveControlFromCoprocessor2,
    /// The move control to coprocessor2 form.
    MoveControlToCoprocessor2,
    /// The coprocessor branch form.
    CoprocessorBranch {
        /// The selected coprocessor.
        coprocessor: Coprocessor,
        /// The relation or coprocessor condition tested by the instruction.
        condition: CoprocessorBranchCondition,
        /// Whether the branch-likely form annuls its delay slot when not taken.
        likely: bool,
    },
    /// The coprocessor memory form.
    CoprocessorMemory {
        /// The selected coprocessor.
        coprocessor: Coprocessor,
        /// The selected operation.
        operation: CoprocessorMemoryOperation,
    },
    /// The cp0 form.
    Cp0(Cp0Operation),
    /// The cache form.
    Cache,
    /// The floating binary form.
    FloatingBinary {
        /// The selected operation.
        operation: FloatingBinaryOperation,
        /// The source numeric format.
        format: FloatingFormat,
    },
    /// The floating unary form.
    FloatingUnary {
        /// The selected operation.
        operation: FloatingUnaryOperation,
        /// The source numeric format.
        format: FloatingFormat,
    },
    /// The floating round to integer form.
    FloatingRoundToInteger {
        /// The rounding operand.
        rounding: FloatingRounding,
        /// The destination format operand.
        destination_format: FixedFormat,
        /// The source format operand.
        source_format: FloatingFormat,
    },
    /// The floating convert form.
    FloatingConvert {
        /// The destination format operand.
        destination_format: NumericFormat,
        /// The source format operand.
        source_format: NumericFormat,
    },
    /// The floating compare form.
    FloatingCompare {
        /// The selected floating-point comparison predicate.
        predicate: FloatingComparePredicate,
        /// The source numeric format.
        format: FloatingFormat,
    },
}

impl Instruction {
    /// Returns the operand-independent instruction form.
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
