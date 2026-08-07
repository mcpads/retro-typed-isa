use std::fmt;

use crate::VfpuInstruction;

/// One of the 32 Allegrex general-purpose registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GeneralRegister(u8);

impl GeneralRegister {
    /// The zero register or complete selector set.
    pub const ZERO: Self = Self(0);
    /// The at register or complete selector set.
    pub const AT: Self = Self(1);
    /// The v0 register or complete selector set.
    pub const V0: Self = Self(2);
    /// The v1 register or complete selector set.
    pub const V1: Self = Self(3);
    /// The a0 register or complete selector set.
    pub const A0: Self = Self(4);
    /// The a1 register or complete selector set.
    pub const A1: Self = Self(5);
    /// The a2 register or complete selector set.
    pub const A2: Self = Self(6);
    /// The a3 register or complete selector set.
    pub const A3: Self = Self(7);
    /// The t0 register or complete selector set.
    pub const T0: Self = Self(8);
    /// The t1 register or complete selector set.
    pub const T1: Self = Self(9);
    /// The t2 register or complete selector set.
    pub const T2: Self = Self(10);
    /// The t3 register or complete selector set.
    pub const T3: Self = Self(11);
    /// The t4 register or complete selector set.
    pub const T4: Self = Self(12);
    /// The t5 register or complete selector set.
    pub const T5: Self = Self(13);
    /// The t6 register or complete selector set.
    pub const T6: Self = Self(14);
    /// The t7 register or complete selector set.
    pub const T7: Self = Self(15);
    /// The s0 register or complete selector set.
    pub const S0: Self = Self(16);
    /// The s1 register or complete selector set.
    pub const S1: Self = Self(17);
    /// The s2 register or complete selector set.
    pub const S2: Self = Self(18);
    /// The s3 register or complete selector set.
    pub const S3: Self = Self(19);
    /// The s4 register or complete selector set.
    pub const S4: Self = Self(20);
    /// The s5 register or complete selector set.
    pub const S5: Self = Self(21);
    /// The s6 register or complete selector set.
    pub const S6: Self = Self(22);
    /// The s7 register or complete selector set.
    pub const S7: Self = Self(23);
    /// The t8 register or complete selector set.
    pub const T8: Self = Self(24);
    /// The t9 register or complete selector set.
    pub const T9: Self = Self(25);
    /// The k0 register or complete selector set.
    pub const K0: Self = Self(26);
    /// The k1 register or complete selector set.
    pub const K1: Self = Self(27);
    /// The gp register or complete selector set.
    pub const GP: Self = Self(28);
    /// The sp register or complete selector set.
    pub const SP: Self = Self(29);
    /// The fp register or complete selector set.
    pub const FP: Self = Self(30);
    /// The ra register or complete selector set.
    pub const RA: Self = Self(31);

    /// Constructs the value after checking its encoding and operand-layout constraints.
    pub const fn new(index: u8) -> Option<Self> {
        if index < 32 { Some(Self(index)) } else { None }
    }

    /// Returns the encoded register index.
    pub const fn index(self) -> u8 {
        self.0
    }

    /// Performs the from field operation.
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

macro_rules! define_five_bit_register {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u8);

        impl $name {
            /// Constructs the value after checking its encoding and operand-layout constraints.
            pub const fn new(index: u8) -> Option<Self> {
                if index < 32 { Some(Self(index)) } else { None }
            }

            /// Returns the encoded register index.
            pub const fn index(self) -> u8 {
                self.0
            }

            /// Performs the from field operation.
            pub(crate) const fn from_field(index: u8) -> Self {
                debug_assert!(index < 32);
                Self(index)
            }
        }
    };
}

define_five_bit_register!(
    SystemControlRegister,
    "One five-bit Allegrex CP0 register field."
);
define_five_bit_register!(FloatingRegister, "One of the 32 Allegrex FPU registers.");

/// An FPU control register accepted by CFC1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum FloatingControlRegister {
    /// The implementation revision form.
    ImplementationRevision = 0,
    /// The control status form.
    ControlStatus = 31,
}

impl FloatingControlRegister {
    /// Returns the encoded register index.
    pub const fn index(self) -> u8 {
        self as u8
    }

    /// Performs the from cfc1 field operation.
    pub(crate) const fn from_cfc1_field(index: u8) -> Option<Self> {
        match index {
            0 => Some(Self::ImplementationRevision),
            31 => Some(Self::ControlStatus),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The three register operation alternatives.
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
    /// The maximum form.
    Maximum,
    /// The minimum form.
    Minimum,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The immediate arithmetic operation alternatives.
pub enum ImmediateArithmeticOperation {
    /// The add form.
    Add,
    /// The add unsigned form.
    AddUnsigned,
    /// The set less than form.
    SetLessThan,
    /// The set less than unsigned form.
    SetLessThanUnsigned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The logical immediate operation alternatives.
pub enum LogicalImmediateOperation {
    /// The and form.
    And,
    /// The or form.
    Or,
    /// The exclusive or form.
    ExclusiveOr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The shift operation alternatives.
pub enum ShiftOperation {
    /// The left logical form.
    LeftLogical,
    /// The right logical form.
    RightLogical,
    /// The right arithmetic form.
    RightArithmetic,
    /// The rotate right form.
    RotateRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The multiply divide operation alternatives.
pub enum MultiplyDivideOperation {
    /// The multiply form.
    Multiply,
    /// The multiply unsigned form.
    MultiplyUnsigned,
    /// The divide form.
    Divide,
    /// The divide unsigned form.
    DivideUnsigned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The accumulate operation alternatives.
pub enum AccumulateOperation {
    /// The multiply add form.
    MultiplyAdd,
    /// The multiply add unsigned form.
    MultiplyAddUnsigned,
    /// The multiply subtract form.
    MultiplySubtract,
    /// The multiply subtract unsigned form.
    MultiplySubtractUnsigned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The special register alternatives.
pub enum SpecialRegister {
    /// The hi form.
    Hi,
    /// The lo form.
    Lo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The equality condition alternatives.
pub enum EqualityCondition {
    /// The equal form.
    Equal,
    /// The not equal form.
    NotEqual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The sign condition alternatives.
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
/// The conditional move condition alternatives.
pub enum ConditionalMoveCondition {
    /// The zero form.
    Zero,
    /// The nonzero form.
    Nonzero,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The memory operation alternatives.
pub enum MemoryOperation {
    /// The load byte form.
    LoadByte,
    /// The load byte unsigned form.
    LoadByteUnsigned,
    /// The load halfword form.
    LoadHalfword,
    /// The load halfword unsigned form.
    LoadHalfwordUnsigned,
    /// The load word form.
    LoadWord,
    /// The load word left form.
    LoadWordLeft,
    /// The load word right form.
    LoadWordRight,
    /// The store byte form.
    StoreByte,
    /// The store halfword form.
    StoreHalfword,
    /// The store word form.
    StoreWord,
    /// The store word left form.
    StoreWordLeft,
    /// The store word right form.
    StoreWordRight,
    /// The load linked form.
    LoadLinked,
    /// The store conditional form.
    StoreConditional,
}

/// The fourteen CACHE function codes assigned by the Allegrex manual.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum CacheOperation {
    /// The index invalidate instruction form.
    IndexInvalidateInstruction = 4,
    /// The index unlock instruction form.
    IndexUnlockInstruction = 6,
    /// The hit invalidate instruction form.
    HitInvalidateInstruction = 8,
    /// The fill instruction form.
    FillInstruction = 10,
    /// The fill and lock instruction form.
    FillAndLockInstruction = 11,
    /// The index writeback invalidate data form.
    IndexWritebackInvalidateData = 20,
    /// The index unlock data form.
    IndexUnlockData = 22,
    /// The create dirty exclusive data form.
    CreateDirtyExclusiveData = 24,
    /// The hit invalidate data form.
    HitInvalidateData = 25,
    /// The hit writeback data form.
    HitWritebackData = 26,
    /// The hit writeback invalidate data form.
    HitWritebackInvalidateData = 27,
    /// The create dirty exclusive and lock data form.
    CreateDirtyExclusiveAndLockData = 28,
    /// The fill data form.
    FillData = 30,
    /// The fill and lock data form.
    FillAndLockData = 31,
}

impl CacheOperation {
    /// The all register or complete selector set.
    pub const ALL: [Self; 14] = [
        Self::IndexInvalidateInstruction,
        Self::IndexUnlockInstruction,
        Self::HitInvalidateInstruction,
        Self::FillInstruction,
        Self::FillAndLockInstruction,
        Self::IndexWritebackInvalidateData,
        Self::IndexUnlockData,
        Self::CreateDirtyExclusiveData,
        Self::HitInvalidateData,
        Self::HitWritebackData,
        Self::HitWritebackInvalidateData,
        Self::CreateDirtyExclusiveAndLockData,
        Self::FillData,
        Self::FillAndLockData,
    ];

    /// Returns the encoded selector code.
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// Performs the from field operation.
    pub(crate) fn from_field(value: u8) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|operation| operation.code() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The bit count operation alternatives.
pub enum BitCountOperation {
    /// The leading zeros form.
    LeadingZeros,
    /// The leading ones form.
    LeadingOnes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The bit field operation alternatives.
pub enum BitFieldOperation {
    /// The extract form.
    Extract,
    /// The insert form.
    Insert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The byte transform operation alternatives.
pub enum ByteTransformOperation {
    /// The sign extend byte form.
    SignExtendByte,
    /// The sign extend halfword form.
    SignExtendHalfword,
    /// The reverse bits form.
    ReverseBits,
    /// The swap bytes within halfwords form.
    SwapBytesWithinHalfwords,
    /// The swap bytes within word form.
    SwapBytesWithinWord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The floating branch condition alternatives.
pub enum FloatingBranchCondition {
    /// The false form.
    False,
    /// The true form.
    True,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The floating binary operation alternatives.
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
/// The floating unary operation alternatives.
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
/// The floating rounding operation alternatives.
pub enum FloatingRoundingOperation {
    /// The round form.
    Round,
    /// The truncate form.
    Truncate,
    /// The ceiling form.
    Ceiling,
    /// The floor form.
    Floor,
}

/// The sixteen comparison predicates assigned by the Allegrex FPU manual.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
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
    /// The not greater less or equal form.
    NotGreaterLessOrEqual = 9,
    /// The signaling equal form.
    SignalingEqual = 10,
    /// The not greater or less form.
    NotGreaterOrLess = 11,
    /// The less than form.
    LessThan = 12,
    /// The not greater than or equal form.
    NotGreaterThanOrEqual = 13,
    /// The less than or equal form.
    LessThanOrEqual = 14,
    /// The not greater than form.
    NotGreaterThan = 15,
}

impl FloatingComparePredicate {
    /// The all register or complete selector set.
    pub const ALL: [Self; 16] = [
        Self::False,
        Self::Unordered,
        Self::Equal,
        Self::UnorderedOrEqual,
        Self::OrderedLessThan,
        Self::UnorderedOrLessThan,
        Self::OrderedLessThanOrEqual,
        Self::UnorderedOrLessThanOrEqual,
        Self::SignalingFalse,
        Self::NotGreaterLessOrEqual,
        Self::SignalingEqual,
        Self::NotGreaterOrLess,
        Self::LessThan,
        Self::NotGreaterThanOrEqual,
        Self::LessThanOrEqual,
        Self::NotGreaterThan,
    ];

    /// Returns the encoded selector code.
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// Performs the from field operation.
    pub(crate) fn from_field(value: u8) -> Self {
        Self::ALL[value as usize]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The operand-independent Allegrex instruction form used by static analysis.
pub enum InstructionForm {
    /// The nop form.
    Nop,
    /// The three register form.
    ThreeRegister(ThreeRegisterOperation),
    /// The immediate arithmetic form.
    ImmediateArithmetic(ImmediateArithmeticOperation),
    /// The logical immediate form.
    LogicalImmediate(LogicalImmediateOperation),
    /// The load upper immediate form.
    LoadUpperImmediate,
    /// The shift immediate form.
    ShiftImmediate(ShiftOperation),
    /// The shift variable form.
    ShiftVariable(ShiftOperation),
    /// The multiply divide form.
    MultiplyDivide(MultiplyDivideOperation),
    /// The accumulate form.
    Accumulate(AccumulateOperation),
    /// The move from special form.
    MoveFromSpecial(SpecialRegister),
    /// The move to special form.
    MoveToSpecial(SpecialRegister),
    /// The conditional move form.
    ConditionalMove(ConditionalMoveCondition),
    /// The bit count form.
    BitCount(BitCountOperation),
    /// The bit field form.
    BitField(BitFieldOperation),
    /// The byte transform form.
    ByteTransform(ByteTransformOperation),
    /// The jump register form.
    JumpRegister,
    /// The jump and link register form.
    JumpAndLinkRegister,
    /// The jump form.
    Jump,
    /// The jump and link form.
    JumpAndLink,
    /// The equality branch form.
    EqualityBranch(EqualityCondition),
    /// The sign branch form.
    SignBranch(SignCondition),
    /// The memory form.
    Memory(MemoryOperation),
    /// The cache form.
    Cache(CacheOperation),
    /// The system call form.
    SystemCall,
    /// The break form.
    Break,
    /// The synchronize form.
    Synchronize,
    /// The exception return form.
    ExceptionReturn,
    /// The move from system control form.
    MoveFromSystemControl,
    /// The move to system control form.
    MoveToSystemControl,
    /// The move from floating form.
    MoveFromFloating,
    /// The move to floating form.
    MoveToFloating,
    /// The move from floating control form.
    MoveFromFloatingControl,
    /// The move to floating control form.
    MoveToFloatingControl,
    /// The floating branch form.
    FloatingBranch(FloatingBranchCondition),
    /// The floating memory load form.
    FloatingMemoryLoad,
    /// The floating memory store form.
    FloatingMemoryStore,
    /// The floating binary form.
    FloatingBinary(FloatingBinaryOperation),
    /// The floating unary form.
    FloatingUnary(FloatingUnaryOperation),
    /// The convert word to single form.
    ConvertWordToSingle,
    /// The convert single to word form.
    ConvertSingleToWord,
    /// The floating round to word form.
    FloatingRoundToWord(FloatingRoundingOperation),
    /// The floating compare form.
    FloatingCompare(FloatingComparePredicate),
    /// The vfpu form.
    Vfpu(crate::VfpuForm),
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A complete typed instruction admitted by the Allegrex profile.
pub enum Instruction {
    /// The nop form.
    Nop,
    /// The three register form.
    ThreeRegister {
        /// The selected operation.
        operation: ThreeRegisterOperation,
        /// The destination operand.
        destination: GeneralRegister,
        /// The left source register.
        left: GeneralRegister,
        /// The right source register.
        right: GeneralRegister,
    },
    /// The immediate arithmetic form.
    ImmediateArithmetic {
        /// The selected operation.
        operation: ImmediateArithmeticOperation,
        /// The destination operand.
        destination: GeneralRegister,
        /// The underlying codec failure.
        source: GeneralRegister,
        /// The immediate operand.
        immediate: i16,
    },
    /// The logical immediate form.
    LogicalImmediate {
        /// The selected operation.
        operation: LogicalImmediateOperation,
        /// The destination operand.
        destination: GeneralRegister,
        /// The underlying codec failure.
        source: GeneralRegister,
        /// The immediate operand.
        immediate: u16,
    },
    /// The load upper immediate form.
    LoadUpperImmediate {
        /// The destination operand.
        destination: GeneralRegister,
        /// The immediate operand.
        immediate: u16,
    },
    /// The shift immediate form.
    ShiftImmediate {
        /// The selected operation.
        operation: ShiftOperation,
        /// The destination operand.
        destination: GeneralRegister,
        /// The underlying codec failure.
        source: GeneralRegister,
        /// The shift amount.
        amount: u8,
    },
    /// The shift variable form.
    ShiftVariable {
        /// The selected operation.
        operation: ShiftOperation,
        /// The destination operand.
        destination: GeneralRegister,
        /// The underlying codec failure.
        source: GeneralRegister,
        /// The amount register operand.
        amount_register: GeneralRegister,
    },
    /// The multiply divide form.
    MultiplyDivide {
        /// The selected operation.
        operation: MultiplyDivideOperation,
        /// The left source register.
        left: GeneralRegister,
        /// The right source register.
        right: GeneralRegister,
    },
    /// The accumulate form.
    Accumulate {
        /// The selected operation.
        operation: AccumulateOperation,
        /// The left source register.
        left: GeneralRegister,
        /// The right source register.
        right: GeneralRegister,
    },
    /// The move from special form.
    MoveFromSpecial {
        /// The underlying codec failure.
        source: SpecialRegister,
        /// The destination operand.
        destination: GeneralRegister,
    },
    /// The move to special form.
    MoveToSpecial {
        /// The destination operand.
        destination: SpecialRegister,
        /// The underlying codec failure.
        source: GeneralRegister,
    },
    /// The conditional move form.
    ConditionalMove {
        /// The selected branch condition.
        condition: ConditionalMoveCondition,
        /// The destination operand.
        destination: GeneralRegister,
        /// The underlying codec failure.
        source: GeneralRegister,
        /// The condition register operand.
        condition_register: GeneralRegister,
    },
    /// The bit count form.
    BitCount {
        /// The selected operation.
        operation: BitCountOperation,
        /// The destination operand.
        destination: GeneralRegister,
        /// The underlying codec failure.
        source: GeneralRegister,
    },
    /// The bit field form.
    BitField {
        /// The selected operation.
        operation: BitFieldOperation,
        /// The destination operand.
        destination: GeneralRegister,
        /// The underlying codec failure.
        source: GeneralRegister,
        /// The position operand.
        position: u8,
        /// The vector size.
        size: u8,
    },
    /// The byte transform form.
    ByteTransform {
        /// The selected operation.
        operation: ByteTransformOperation,
        /// The destination operand.
        destination: GeneralRegister,
        /// The underlying codec failure.
        source: GeneralRegister,
    },
    /// The jump register form.
    JumpRegister {
        /// The underlying codec failure.
        source: GeneralRegister,
        /// Whether a return address is recorded.
        link: Option<GeneralRegister>,
    },
    /// The jump form.
    Jump {
        /// Whether a return address is recorded.
        link: bool,
        /// The requested control-flow target.
        target: u32,
    },
    /// The equality branch form.
    EqualityBranch {
        /// The selected branch condition.
        condition: EqualityCondition,
        /// Whether branch-likely semantics are selected.
        likely: bool,
        /// The left source register.
        left: GeneralRegister,
        /// The right source register.
        right: GeneralRegister,
        /// The requested control-flow target.
        target: u32,
    },
    /// The sign branch form.
    SignBranch {
        /// The selected branch condition.
        condition: SignCondition,
        /// Whether branch-likely semantics are selected.
        likely: bool,
        /// Whether a return address is recorded.
        link: bool,
        /// The underlying codec failure.
        source: GeneralRegister,
        /// The requested control-flow target.
        target: u32,
    },
    /// The memory form.
    Memory {
        /// The selected operation.
        operation: MemoryOperation,
        /// The selected register.
        register: GeneralRegister,
        /// The base operand.
        base: GeneralRegister,
        /// The output byte offset.
        offset: i16,
    },
    /// The cache form.
    Cache {
        /// The selected operation.
        operation: CacheOperation,
        /// The base operand.
        base: GeneralRegister,
        /// The output byte offset.
        offset: i16,
    },
    /// The system call form.
    SystemCall {
        /// The rejected code field.
        code: u32,
    },
    /// The break form.
    Break {
        /// The rejected code field.
        code: u32,
    },
    /// The synchronize form.
    Synchronize,
    /// The exception return form.
    ExceptionReturn,
    /// The system control transfer form.
    SystemControlTransfer {
        /// The to control operand.
        to_control: bool,
        /// The general operand.
        general: GeneralRegister,
        /// The control operand.
        control: SystemControlRegister,
    },
    /// The floating transfer form.
    FloatingTransfer {
        /// The to floating operand.
        to_floating: bool,
        /// The general operand.
        general: GeneralRegister,
        /// The floating operand.
        floating: FloatingRegister,
    },
    /// The floating control transfer form.
    FloatingControlTransfer {
        /// The to control operand.
        to_control: bool,
        /// The general operand.
        general: GeneralRegister,
        /// The control operand.
        control: FloatingControlRegister,
    },
    /// The floating branch form.
    FloatingBranch {
        /// The selected branch condition.
        condition: FloatingBranchCondition,
        /// Whether branch-likely semantics are selected.
        likely: bool,
        /// The requested control-flow target.
        target: u32,
    },
    /// The floating memory form.
    FloatingMemory {
        /// The store operand.
        store: bool,
        /// The floating operand.
        floating: FloatingRegister,
        /// The base operand.
        base: GeneralRegister,
        /// The output byte offset.
        offset: i16,
    },
    /// The floating binary form.
    FloatingBinary {
        /// The selected operation.
        operation: FloatingBinaryOperation,
        /// The destination operand.
        destination: FloatingRegister,
        /// The left source register.
        left: FloatingRegister,
        /// The right source register.
        right: FloatingRegister,
    },
    /// The floating unary form.
    FloatingUnary {
        /// The selected operation.
        operation: FloatingUnaryOperation,
        /// The destination operand.
        destination: FloatingRegister,
        /// The underlying codec failure.
        source: FloatingRegister,
    },
    /// The convert word to single form.
    ConvertWordToSingle {
        /// The destination operand.
        destination: FloatingRegister,
        /// The underlying codec failure.
        source: FloatingRegister,
    },
    /// The convert single to word form.
    ConvertSingleToWord {
        /// The destination operand.
        destination: FloatingRegister,
        /// The underlying codec failure.
        source: FloatingRegister,
    },
    /// The floating round to word form.
    FloatingRoundToWord {
        /// The selected operation.
        operation: FloatingRoundingOperation,
        /// The destination operand.
        destination: FloatingRegister,
        /// The underlying codec failure.
        source: FloatingRegister,
    },
    /// The floating compare form.
    FloatingCompare {
        /// The predicate operand.
        predicate: FloatingComparePredicate,
        /// The left source register.
        left: FloatingRegister,
        /// The right source register.
        right: FloatingRegister,
    },
    /// The vfpu form.
    Vfpu(VfpuInstruction),
}

impl Instruction {
    /// Returns the operand-independent instruction form.
    pub const fn form(&self) -> InstructionForm {
        match self {
            Self::Nop => InstructionForm::Nop,
            Self::ThreeRegister { operation, .. } => InstructionForm::ThreeRegister(*operation),
            Self::ImmediateArithmetic { operation, .. } => {
                InstructionForm::ImmediateArithmetic(*operation)
            }
            Self::LogicalImmediate { operation, .. } => {
                InstructionForm::LogicalImmediate(*operation)
            }
            Self::LoadUpperImmediate { .. } => InstructionForm::LoadUpperImmediate,
            Self::ShiftImmediate { operation, .. } => InstructionForm::ShiftImmediate(*operation),
            Self::ShiftVariable { operation, .. } => InstructionForm::ShiftVariable(*operation),
            Self::MultiplyDivide { operation, .. } => InstructionForm::MultiplyDivide(*operation),
            Self::Accumulate { operation, .. } => InstructionForm::Accumulate(*operation),
            Self::MoveFromSpecial { source, .. } => InstructionForm::MoveFromSpecial(*source),
            Self::MoveToSpecial { destination, .. } => InstructionForm::MoveToSpecial(*destination),
            Self::ConditionalMove { condition, .. } => InstructionForm::ConditionalMove(*condition),
            Self::BitCount { operation, .. } => InstructionForm::BitCount(*operation),
            Self::BitField { operation, .. } => InstructionForm::BitField(*operation),
            Self::ByteTransform { operation, .. } => InstructionForm::ByteTransform(*operation),
            Self::JumpRegister { link: None, .. } => InstructionForm::JumpRegister,
            Self::JumpRegister { link: Some(_), .. } => InstructionForm::JumpAndLinkRegister,
            Self::Jump { link: false, .. } => InstructionForm::Jump,
            Self::Jump { link: true, .. } => InstructionForm::JumpAndLink,
            Self::EqualityBranch { condition, .. } => InstructionForm::EqualityBranch(*condition),
            Self::SignBranch { condition, .. } => InstructionForm::SignBranch(*condition),
            Self::Memory { operation, .. } => InstructionForm::Memory(*operation),
            Self::Cache { operation, .. } => InstructionForm::Cache(*operation),
            Self::SystemCall { .. } => InstructionForm::SystemCall,
            Self::Break { .. } => InstructionForm::Break,
            Self::Synchronize => InstructionForm::Synchronize,
            Self::ExceptionReturn => InstructionForm::ExceptionReturn,
            Self::SystemControlTransfer {
                to_control: false, ..
            } => InstructionForm::MoveFromSystemControl,
            Self::SystemControlTransfer {
                to_control: true, ..
            } => InstructionForm::MoveToSystemControl,
            Self::FloatingTransfer {
                to_floating: false, ..
            } => InstructionForm::MoveFromFloating,
            Self::FloatingTransfer {
                to_floating: true, ..
            } => InstructionForm::MoveToFloating,
            Self::FloatingControlTransfer {
                to_control: false, ..
            } => InstructionForm::MoveFromFloatingControl,
            Self::FloatingControlTransfer {
                to_control: true, ..
            } => InstructionForm::MoveToFloatingControl,
            Self::FloatingBranch { condition, .. } => InstructionForm::FloatingBranch(*condition),
            Self::FloatingMemory { store: false, .. } => InstructionForm::FloatingMemoryLoad,
            Self::FloatingMemory { store: true, .. } => InstructionForm::FloatingMemoryStore,
            Self::FloatingBinary { operation, .. } => InstructionForm::FloatingBinary(*operation),
            Self::FloatingUnary { operation, .. } => InstructionForm::FloatingUnary(*operation),
            Self::ConvertWordToSingle { .. } => InstructionForm::ConvertWordToSingle,
            Self::ConvertSingleToWord { .. } => InstructionForm::ConvertSingleToWord,
            Self::FloatingRoundToWord { operation, .. } => {
                InstructionForm::FloatingRoundToWord(*operation)
            }
            Self::FloatingCompare { predicate, .. } => InstructionForm::FloatingCompare(*predicate),
            Self::Vfpu(instruction) => InstructionForm::Vfpu(instruction.form()),
        }
    }
}
