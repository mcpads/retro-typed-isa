use std::fmt;

use crate::VfpuInstruction;

/// One of the 32 Allegrex general-purpose registers.
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

macro_rules! define_five_bit_register {
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

define_five_bit_register!(
    SystemControlRegister,
    "One five-bit Allegrex CP0 register field."
);
define_five_bit_register!(FloatingRegister, "One of the 32 Allegrex FPU registers.");

/// An FPU control register accepted by CFC1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum FloatingControlRegister {
    ImplementationRevision = 0,
    ControlStatus = 31,
}

impl FloatingControlRegister {
    pub const fn index(self) -> u8 {
        self as u8
    }

    pub(crate) const fn from_cfc1_field(index: u8) -> Option<Self> {
        match index {
            0 => Some(Self::ImplementationRevision),
            31 => Some(Self::ControlStatus),
            _ => None,
        }
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
    Maximum,
    Minimum,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ImmediateArithmeticOperation {
    Add,
    AddUnsigned,
    SetLessThan,
    SetLessThanUnsigned,
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
    RotateRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MultiplyDivideOperation {
    Multiply,
    MultiplyUnsigned,
    Divide,
    DivideUnsigned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AccumulateOperation {
    MultiplyAdd,
    MultiplyAddUnsigned,
    MultiplySubtract,
    MultiplySubtractUnsigned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SpecialRegister {
    Hi,
    Lo,
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
pub enum ConditionalMoveCondition {
    Zero,
    Nonzero,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MemoryOperation {
    LoadByte,
    LoadByteUnsigned,
    LoadHalfword,
    LoadHalfwordUnsigned,
    LoadWord,
    LoadWordLeft,
    LoadWordRight,
    StoreByte,
    StoreHalfword,
    StoreWord,
    StoreWordLeft,
    StoreWordRight,
    LoadLinked,
    StoreConditional,
}

/// The fourteen CACHE function codes assigned by the Allegrex manual.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum CacheOperation {
    IndexInvalidateInstruction = 4,
    IndexUnlockInstruction = 6,
    HitInvalidateInstruction = 8,
    FillInstruction = 10,
    FillAndLockInstruction = 11,
    IndexWritebackInvalidateData = 20,
    IndexUnlockData = 22,
    CreateDirtyExclusiveData = 24,
    HitInvalidateData = 25,
    HitWritebackData = 26,
    HitWritebackInvalidateData = 27,
    CreateDirtyExclusiveAndLockData = 28,
    FillData = 30,
    FillAndLockData = 31,
}

impl CacheOperation {
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

    pub const fn code(self) -> u8 {
        self as u8
    }

    pub(crate) fn from_field(value: u8) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|operation| operation.code() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BitCountOperation {
    LeadingZeros,
    LeadingOnes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BitFieldOperation {
    Extract,
    Insert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ByteTransformOperation {
    SignExtendByte,
    SignExtendHalfword,
    ReverseBits,
    SwapBytesWithinHalfwords,
    SwapBytesWithinWord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FloatingBranchCondition {
    False,
    True,
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
pub enum FloatingRoundingOperation {
    Round,
    Truncate,
    Ceiling,
    Floor,
}

/// The sixteen comparison predicates assigned by the Allegrex FPU manual.
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
    NotGreaterLessOrEqual = 9,
    SignalingEqual = 10,
    NotGreaterOrLess = 11,
    LessThan = 12,
    NotGreaterThanOrEqual = 13,
    LessThanOrEqual = 14,
    NotGreaterThan = 15,
}

impl FloatingComparePredicate {
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

    pub const fn code(self) -> u8 {
        self as u8
    }

    pub(crate) fn from_field(value: u8) -> Self {
        Self::ALL[value as usize]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InstructionForm {
    Nop,
    ThreeRegister(ThreeRegisterOperation),
    ImmediateArithmetic(ImmediateArithmeticOperation),
    LogicalImmediate(LogicalImmediateOperation),
    LoadUpperImmediate,
    ShiftImmediate(ShiftOperation),
    ShiftVariable(ShiftOperation),
    MultiplyDivide(MultiplyDivideOperation),
    Accumulate(AccumulateOperation),
    MoveFromSpecial(SpecialRegister),
    MoveToSpecial(SpecialRegister),
    ConditionalMove(ConditionalMoveCondition),
    BitCount(BitCountOperation),
    BitField(BitFieldOperation),
    ByteTransform(ByteTransformOperation),
    JumpRegister,
    JumpAndLinkRegister,
    Jump,
    JumpAndLink,
    EqualityBranch(EqualityCondition),
    SignBranch(SignCondition),
    Memory(MemoryOperation),
    Cache(CacheOperation),
    SystemCall,
    Break,
    Synchronize,
    ExceptionReturn,
    MoveFromSystemControl,
    MoveToSystemControl,
    MoveFromFloating,
    MoveToFloating,
    MoveFromFloatingControl,
    MoveToFloatingControl,
    FloatingBranch(FloatingBranchCondition),
    FloatingMemoryLoad,
    FloatingMemoryStore,
    FloatingBinary(FloatingBinaryOperation),
    FloatingUnary(FloatingUnaryOperation),
    ConvertWordToSingle,
    ConvertSingleToWord,
    FloatingRoundToWord(FloatingRoundingOperation),
    FloatingCompare(FloatingComparePredicate),
    Vfpu(crate::VfpuForm),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    Nop,
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
    Accumulate {
        operation: AccumulateOperation,
        left: GeneralRegister,
        right: GeneralRegister,
    },
    MoveFromSpecial {
        source: SpecialRegister,
        destination: GeneralRegister,
    },
    MoveToSpecial {
        destination: SpecialRegister,
        source: GeneralRegister,
    },
    ConditionalMove {
        condition: ConditionalMoveCondition,
        destination: GeneralRegister,
        source: GeneralRegister,
        condition_register: GeneralRegister,
    },
    BitCount {
        operation: BitCountOperation,
        destination: GeneralRegister,
        source: GeneralRegister,
    },
    BitField {
        operation: BitFieldOperation,
        destination: GeneralRegister,
        source: GeneralRegister,
        position: u8,
        size: u8,
    },
    ByteTransform {
        operation: ByteTransformOperation,
        destination: GeneralRegister,
        source: GeneralRegister,
    },
    JumpRegister {
        source: GeneralRegister,
        link: Option<GeneralRegister>,
    },
    Jump {
        link: bool,
        target: u32,
    },
    EqualityBranch {
        condition: EqualityCondition,
        likely: bool,
        left: GeneralRegister,
        right: GeneralRegister,
        target: u32,
    },
    SignBranch {
        condition: SignCondition,
        likely: bool,
        link: bool,
        source: GeneralRegister,
        target: u32,
    },
    Memory {
        operation: MemoryOperation,
        register: GeneralRegister,
        base: GeneralRegister,
        offset: i16,
    },
    Cache {
        operation: CacheOperation,
        base: GeneralRegister,
        offset: i16,
    },
    SystemCall {
        code: u32,
    },
    Break {
        code: u32,
    },
    Synchronize,
    ExceptionReturn,
    SystemControlTransfer {
        to_control: bool,
        general: GeneralRegister,
        control: SystemControlRegister,
    },
    FloatingTransfer {
        to_floating: bool,
        general: GeneralRegister,
        floating: FloatingRegister,
    },
    FloatingControlTransfer {
        to_control: bool,
        general: GeneralRegister,
        control: FloatingControlRegister,
    },
    FloatingBranch {
        condition: FloatingBranchCondition,
        likely: bool,
        target: u32,
    },
    FloatingMemory {
        store: bool,
        floating: FloatingRegister,
        base: GeneralRegister,
        offset: i16,
    },
    FloatingBinary {
        operation: FloatingBinaryOperation,
        destination: FloatingRegister,
        left: FloatingRegister,
        right: FloatingRegister,
    },
    FloatingUnary {
        operation: FloatingUnaryOperation,
        destination: FloatingRegister,
        source: FloatingRegister,
    },
    ConvertWordToSingle {
        destination: FloatingRegister,
        source: FloatingRegister,
    },
    ConvertSingleToWord {
        destination: FloatingRegister,
        source: FloatingRegister,
    },
    FloatingRoundToWord {
        operation: FloatingRoundingOperation,
        destination: FloatingRegister,
        source: FloatingRegister,
    },
    FloatingCompare {
        predicate: FloatingComparePredicate,
        left: FloatingRegister,
        right: FloatingRegister,
    },
    Vfpu(VfpuInstruction),
}

impl Instruction {
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
