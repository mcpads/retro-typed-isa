use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Register(u8);

impl Register {
    pub const PC: Self = Self(15);
    pub const LINK: Self = Self(14);
    pub const SP: Self = Self(13);

    pub const fn new(index: u8) -> Result<Self, FieldOutOfRange> {
        if index < 16 {
            Ok(Self(index))
        } else {
            Err(FieldOutOfRange::new("register", index as u32, 15))
        }
    }

    pub const fn index(self) -> u8 {
        self.0
    }

    pub(crate) const fn from_field(value: u32) -> Self {
        Self(value as u8)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LowRegister(u8);

impl LowRegister {
    pub const fn new(index: u8) -> Result<Self, FieldOutOfRange> {
        if index < 8 {
            Ok(Self(index))
        } else {
            Err(FieldOutOfRange::new("low register", index as u32, 7))
        }
    }

    pub const fn index(self) -> u8 {
        self.0
    }

    pub const fn as_register(self) -> Register {
        Register(self.0)
    }

    pub(crate) const fn from_field(value: u16) -> Self {
        Self(value as u8)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{field} value {value} is outside 0..={maximum}")]
pub struct FieldOutOfRange {
    pub field: &'static str,
    pub value: u32,
    pub maximum: u32,
}

impl FieldOutOfRange {
    pub const fn new(field: &'static str, value: u32, maximum: u32) -> Self {
        Self {
            field,
            value,
            maximum,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Condition {
    Equal = 0x0,
    NotEqual = 0x1,
    CarrySet = 0x2,
    CarryClear = 0x3,
    Negative = 0x4,
    PositiveOrZero = 0x5,
    Overflow = 0x6,
    NoOverflow = 0x7,
    UnsignedHigher = 0x8,
    UnsignedLowerOrSame = 0x9,
    SignedGreaterOrEqual = 0xa,
    SignedLessThan = 0xb,
    SignedGreaterThan = 0xc,
    SignedLessOrEqual = 0xd,
    Always = 0xe,
}

impl Condition {
    pub(crate) const fn from_field(value: u8) -> Option<Self> {
        Some(match value {
            0x0 => Self::Equal,
            0x1 => Self::NotEqual,
            0x2 => Self::CarrySet,
            0x3 => Self::CarryClear,
            0x4 => Self::Negative,
            0x5 => Self::PositiveOrZero,
            0x6 => Self::Overflow,
            0x7 => Self::NoOverflow,
            0x8 => Self::UnsignedHigher,
            0x9 => Self::UnsignedLowerOrSame,
            0xa => Self::SignedGreaterOrEqual,
            0xb => Self::SignedLessThan,
            0xc => Self::SignedGreaterThan,
            0xd => Self::SignedLessOrEqual,
            0xe => Self::Always,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum DataOperation {
    And = 0x0,
    ExclusiveOr = 0x1,
    Subtract = 0x2,
    ReverseSubtract = 0x3,
    Add = 0x4,
    AddWithCarry = 0x5,
    SubtractWithCarry = 0x6,
    ReverseSubtractWithCarry = 0x7,
    Test = 0x8,
    TestEquivalence = 0x9,
    Compare = 0xa,
    CompareNegative = 0xb,
    Or = 0xc,
    Move = 0xd,
    BitClear = 0xe,
    MoveNot = 0xf,
}

impl DataOperation {
    pub const fn is_test(self) -> bool {
        matches!(
            self,
            Self::Test | Self::TestEquivalence | Self::Compare | Self::CompareNegative
        )
    }

    pub const fn ignores_first_register(self) -> bool {
        matches!(self, Self::Move | Self::MoveNot)
    }

    pub(crate) const fn from_field(value: u8) -> Self {
        match value {
            0x0 => Self::And,
            0x1 => Self::ExclusiveOr,
            0x2 => Self::Subtract,
            0x3 => Self::ReverseSubtract,
            0x4 => Self::Add,
            0x5 => Self::AddWithCarry,
            0x6 => Self::SubtractWithCarry,
            0x7 => Self::ReverseSubtractWithCarry,
            0x8 => Self::Test,
            0x9 => Self::TestEquivalence,
            0xa => Self::Compare,
            0xb => Self::CompareNegative,
            0xc => Self::Or,
            0xd => Self::Move,
            0xe => Self::BitClear,
            0xf => Self::MoveNot,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ShiftKind {
    LogicalLeft = 0,
    LogicalRight = 1,
    ArithmeticRight = 2,
    RotateRight = 3,
}

impl ShiftKind {
    pub(crate) const fn from_field(value: u8) -> Self {
        match value {
            0 => Self::LogicalLeft,
            1 => Self::LogicalRight,
            2 => Self::ArithmeticRight,
            3 => Self::RotateRight,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RotatedImmediate {
    value: u8,
    rotate_right: u8,
}

impl RotatedImmediate {
    pub const fn new(value: u8, rotate_right: u8) -> Result<Self, FieldOutOfRange> {
        if rotate_right <= 30 && rotate_right & 1 == 0 {
            Ok(Self {
                value,
                rotate_right,
            })
        } else {
            Err(FieldOutOfRange::new(
                "immediate rotate-right amount",
                rotate_right as u32,
                30,
            ))
        }
    }

    pub const fn value(self) -> u8 {
        self.value
    }

    pub const fn rotate_right(self) -> u8 {
        self.rotate_right
    }

    pub const fn resolved_value(self) -> u32 {
        (self.value as u32).rotate_right(self.rotate_right as u32)
    }

    pub(crate) const fn from_fields(value: u8, rotate: u8) -> Self {
        Self {
            value,
            rotate_right: rotate * 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ImmediateShift {
    LogicalLeft(u8),
    LogicalRight(u8),
    ArithmeticRight(u8),
    RotateRight(u8),
    RotateRightExtended,
}

impl ImmediateShift {
    pub const fn new(kind: ShiftKind, amount: u8) -> Result<Self, FieldOutOfRange> {
        match (kind, amount) {
            (ShiftKind::LogicalLeft, 0..=31) => Ok(Self::LogicalLeft(amount)),
            (ShiftKind::LogicalRight, 1..=32) => Ok(Self::LogicalRight(amount)),
            (ShiftKind::ArithmeticRight, 1..=32) => Ok(Self::ArithmeticRight(amount)),
            (ShiftKind::RotateRight, 1..=31) => Ok(Self::RotateRight(amount)),
            _ => Err(FieldOutOfRange::new("shift amount", amount as u32, 32)),
        }
    }

    pub(crate) const fn from_fields(kind: ShiftKind, encoded_amount: u8) -> Self {
        match (kind, encoded_amount) {
            (ShiftKind::LogicalLeft, value) => Self::LogicalLeft(value),
            (ShiftKind::LogicalRight, 0) => Self::LogicalRight(32),
            (ShiftKind::LogicalRight, value) => Self::LogicalRight(value),
            (ShiftKind::ArithmeticRight, 0) => Self::ArithmeticRight(32),
            (ShiftKind::ArithmeticRight, value) => Self::ArithmeticRight(value),
            (ShiftKind::RotateRight, 0) => Self::RotateRightExtended,
            (ShiftKind::RotateRight, value) => Self::RotateRight(value),
        }
    }

    pub(crate) const fn fields(self) -> (ShiftKind, u8) {
        match self {
            Self::LogicalLeft(value) => (ShiftKind::LogicalLeft, value),
            Self::LogicalRight(32) => (ShiftKind::LogicalRight, 0),
            Self::LogicalRight(value) => (ShiftKind::LogicalRight, value),
            Self::ArithmeticRight(32) => (ShiftKind::ArithmeticRight, 0),
            Self::ArithmeticRight(value) => (ShiftKind::ArithmeticRight, value),
            Self::RotateRight(value) => (ShiftKind::RotateRight, value),
            Self::RotateRightExtended => (ShiftKind::RotateRight, 0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Operand2 {
    Immediate(RotatedImmediate),
    Register {
        register: Register,
        shift: ImmediateShift,
    },
    RegisterShift {
        register: Register,
        kind: ShiftKind,
        amount: Register,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IndexMode {
    Offset,
    PreIndexed,
    PostIndexed,
    PostIndexedUser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AddressOffset {
    Immediate(u16),
    Register {
        register: Register,
        shift: ImmediateShift,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AddressingMode2 {
    pub base: Register,
    pub offset: AddressOffset,
    pub add: bool,
    pub index: IndexMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HalfwordOffset {
    Immediate(u8),
    Register(Register),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AddressingMode3 {
    pub base: Register,
    pub offset: HalfwordOffset,
    pub add: bool,
    pub index: IndexMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TransferWidth {
    Word,
    Byte,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HalfwordTransferKind {
    UnsignedHalfword,
    SignedByte,
    SignedHalfword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum BlockAddressing {
    DecrementAfter = 0,
    IncrementAfter = 1,
    DecrementBefore = 2,
    IncrementBefore = 3,
}

impl BlockAddressing {
    pub(crate) const fn from_fields(pre_index: bool, add: bool) -> Self {
        match (pre_index, add) {
            (false, false) => Self::DecrementAfter,
            (false, true) => Self::IncrementAfter,
            (true, false) => Self::DecrementBefore,
            (true, true) => Self::IncrementBefore,
        }
    }

    pub(crate) const fn fields(self) -> (bool, bool) {
        match self {
            Self::DecrementAfter => (false, false),
            Self::IncrementAfter => (false, true),
            Self::DecrementBefore => (true, false),
            Self::IncrementBefore => (true, true),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RegisterList(u16);

impl RegisterList {
    pub const fn new(bits: u16) -> Result<Self, EmptyRegisterList> {
        if bits == 0 {
            Err(EmptyRegisterList)
        } else {
            Ok(Self(bits))
        }
    }

    pub const fn bits(self) -> u16 {
        self.0
    }

    pub(crate) const fn from_bits(bits: u16) -> Self {
        Self(bits)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("register list cannot be empty")]
pub struct EmptyRegisterList;

macro_rules! four_bit_field {
    ($name:ident, $label:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u8);

        impl $name {
            pub const fn new(value: u8) -> Result<Self, FieldOutOfRange> {
                if value < 16 {
                    Ok(Self(value))
                } else {
                    Err(FieldOutOfRange::new($label, value as u32, 15))
                }
            }

            pub const fn value(self) -> u8 {
                self.0
            }

            pub(crate) const fn from_field(value: u32) -> Self {
                Self(value as u8)
            }
        }
    };
}

four_bit_field!(Coprocessor, "coprocessor number");
four_bit_field!(CoprocessorRegister, "coprocessor register");
four_bit_field!(CoprocessorOpcode, "coprocessor opcode");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StatusRegister {
    Current,
    Saved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StatusMask(u8);

impl StatusMask {
    pub const CONTROL: Self = Self(0b0001);
    pub const EXTENSION: Self = Self(0b0010);
    pub const STATUS: Self = Self(0b0100);
    pub const FLAGS: Self = Self(0b1000);

    pub const fn new(bits: u8) -> Result<Self, FieldOutOfRange> {
        if bits != 0 && bits < 16 {
            Ok(Self(bits))
        } else {
            Err(FieldOutOfRange::new("status field mask", bits as u32, 15))
        }
    }

    pub const fn bits(self) -> u8 {
        self.0
    }

    pub(crate) const fn from_bits(bits: u8) -> Self {
        Self(bits)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StatusSource {
    Register(Register),
    Immediate(RotatedImmediate),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmInstruction {
    DataProcessing {
        condition: Condition,
        operation: DataOperation,
        set_flags: bool,
        destination: Register,
        first: Register,
        second: Operand2,
    },
    Multiply {
        condition: Condition,
        accumulate: bool,
        set_flags: bool,
        destination: Register,
        multiplicand: Register,
        multiplier: Register,
        addend: Register,
    },
    MultiplyLong {
        condition: Condition,
        signed: bool,
        accumulate: bool,
        set_flags: bool,
        destination_low: Register,
        destination_high: Register,
        multiplicand: Register,
        multiplier: Register,
    },
    BranchExchange {
        condition: Condition,
        target: Register,
    },
    Branch {
        condition: Condition,
        link: bool,
        displacement: i32,
    },
    SingleTransfer {
        condition: Condition,
        load: bool,
        width: TransferWidth,
        register: Register,
        address: AddressingMode2,
    },
    HalfwordTransfer {
        condition: Condition,
        load: bool,
        kind: HalfwordTransferKind,
        register: Register,
        address: AddressingMode3,
    },
    BlockTransfer {
        condition: Condition,
        load: bool,
        addressing: BlockAddressing,
        write_back: bool,
        user_registers: bool,
        base: Register,
        registers: RegisterList,
    },
    Swap {
        condition: Condition,
        width: TransferWidth,
        destination: Register,
        source: Register,
        base: Register,
    },
    MoveStatusToRegister {
        condition: Condition,
        source: StatusRegister,
        destination: Register,
    },
    MoveToStatusRegister {
        condition: Condition,
        destination: StatusRegister,
        mask: StatusMask,
        source: StatusSource,
    },
    CoprocessorDataOperation {
        condition: Condition,
        coprocessor: Coprocessor,
        opcode: CoprocessorOpcode,
        info: u8,
        destination: CoprocessorRegister,
        first: CoprocessorRegister,
        second: CoprocessorRegister,
    },
    CoprocessorRegisterTransfer {
        condition: Condition,
        load: bool,
        coprocessor: Coprocessor,
        opcode: u8,
        info: u8,
        arm_register: Register,
        first: CoprocessorRegister,
        second: CoprocessorRegister,
    },
    CoprocessorDataTransfer {
        condition: Condition,
        load: bool,
        long: bool,
        coprocessor: Coprocessor,
        register: CoprocessorRegister,
        base: Register,
        offset_words: u8,
        add: bool,
        index: IndexMode,
    },
    SoftwareInterrupt {
        condition: Condition,
        comment: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArmInstructionForm {
    DataProcessing(DataOperation),
    Multiply,
    MultiplyAccumulate,
    MultiplyLong {
        signed: bool,
        accumulate: bool,
    },
    BranchExchange,
    Branch,
    BranchLink,
    SingleTransfer {
        load: bool,
        width: TransferWidth,
    },
    HalfwordTransfer {
        load: bool,
        kind: HalfwordTransferKind,
    },
    BlockTransfer {
        load: bool,
    },
    Swap(TransferWidth),
    MoveStatusToRegister(StatusRegister),
    MoveToStatusRegister {
        destination: StatusRegister,
        immediate: bool,
    },
    CoprocessorDataOperation,
    CoprocessorRegisterTransfer {
        load: bool,
    },
    CoprocessorDataTransfer {
        load: bool,
    },
    SoftwareInterrupt,
}

impl ArmInstruction {
    pub const fn form(self) -> ArmInstructionForm {
        match self {
            Self::DataProcessing { operation, .. } => ArmInstructionForm::DataProcessing(operation),
            Self::Multiply { accumulate, .. } => {
                if accumulate {
                    ArmInstructionForm::MultiplyAccumulate
                } else {
                    ArmInstructionForm::Multiply
                }
            }
            Self::MultiplyLong {
                signed, accumulate, ..
            } => ArmInstructionForm::MultiplyLong { signed, accumulate },
            Self::BranchExchange { .. } => ArmInstructionForm::BranchExchange,
            Self::Branch { link, .. } => {
                if link {
                    ArmInstructionForm::BranchLink
                } else {
                    ArmInstructionForm::Branch
                }
            }
            Self::SingleTransfer { load, width, .. } => {
                ArmInstructionForm::SingleTransfer { load, width }
            }
            Self::HalfwordTransfer { load, kind, .. } => {
                ArmInstructionForm::HalfwordTransfer { load, kind }
            }
            Self::BlockTransfer { load, .. } => ArmInstructionForm::BlockTransfer { load },
            Self::Swap { width, .. } => ArmInstructionForm::Swap(width),
            Self::MoveStatusToRegister { source, .. } => {
                ArmInstructionForm::MoveStatusToRegister(source)
            }
            Self::MoveToStatusRegister {
                destination,
                source,
                ..
            } => ArmInstructionForm::MoveToStatusRegister {
                destination,
                immediate: matches!(source, StatusSource::Immediate(_)),
            },
            Self::CoprocessorDataOperation { .. } => ArmInstructionForm::CoprocessorDataOperation,
            Self::CoprocessorRegisterTransfer { load, .. } => {
                ArmInstructionForm::CoprocessorRegisterTransfer { load }
            }
            Self::CoprocessorDataTransfer { load, .. } => {
                ArmInstructionForm::CoprocessorDataTransfer { load }
            }
            Self::SoftwareInterrupt { .. } => ArmInstructionForm::SoftwareInterrupt,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ThumbAluOperation {
    And = 0x0,
    ExclusiveOr = 0x1,
    LogicalShiftLeft = 0x2,
    LogicalShiftRight = 0x3,
    ArithmeticShiftRight = 0x4,
    AddWithCarry = 0x5,
    SubtractWithCarry = 0x6,
    RotateRight = 0x7,
    Test = 0x8,
    Negate = 0x9,
    Compare = 0xa,
    CompareNegative = 0xb,
    Or = 0xc,
    Multiply = 0xd,
    BitClear = 0xe,
    MoveNot = 0xf,
}

impl ThumbAluOperation {
    pub(crate) const fn from_field(value: u8) -> Self {
        match value {
            0x0 => Self::And,
            0x1 => Self::ExclusiveOr,
            0x2 => Self::LogicalShiftLeft,
            0x3 => Self::LogicalShiftRight,
            0x4 => Self::ArithmeticShiftRight,
            0x5 => Self::AddWithCarry,
            0x6 => Self::SubtractWithCarry,
            0x7 => Self::RotateRight,
            0x8 => Self::Test,
            0x9 => Self::Negate,
            0xa => Self::Compare,
            0xb => Self::CompareNegative,
            0xc => Self::Or,
            0xd => Self::Multiply,
            0xe => Self::BitClear,
            0xf => Self::MoveNot,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ThumbRegisterList(u8);

impl ThumbRegisterList {
    pub const EMPTY: Self = Self(0);

    pub const fn new(bits: u8) -> Result<Self, EmptyRegisterList> {
        if bits == 0 {
            Err(EmptyRegisterList)
        } else {
            Ok(Self(bits))
        }
    }

    pub const fn bits(self) -> u8 {
        self.0
    }

    pub(crate) const fn from_bits(bits: u8) -> Self {
        Self(bits)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThumbInstruction {
    ShiftImmediate {
        kind: ShiftKind,
        destination: LowRegister,
        source: LowRegister,
        amount: u8,
    },
    AddSubtract {
        subtract: bool,
        destination: LowRegister,
        source: LowRegister,
        operand: ThumbAddOperand,
    },
    Immediate {
        operation: ThumbImmediateOperation,
        register: LowRegister,
        immediate: u8,
    },
    Alu {
        operation: ThumbAluOperation,
        destination: LowRegister,
        source: LowRegister,
    },
    HighRegister {
        operation: ThumbHighOperation,
        destination: Register,
        source: Register,
    },
    BranchExchange {
        source: Register,
    },
    PcRelativeLoad {
        destination: LowRegister,
        offset: u16,
    },
    RegisterTransfer {
        operation: ThumbTransferOperation,
        destination: LowRegister,
        base: LowRegister,
        offset: LowRegister,
    },
    ImmediateTransfer {
        load: bool,
        width: ThumbTransferWidth,
        destination: LowRegister,
        base: LowRegister,
        offset: u8,
    },
    SpRelativeTransfer {
        load: bool,
        register: LowRegister,
        offset: u16,
    },
    LoadAddress {
        from_sp: bool,
        destination: LowRegister,
        offset: u16,
    },
    AddStackPointer {
        subtract: bool,
        offset: u16,
    },
    Push {
        registers: ThumbRegisterList,
        link: bool,
    },
    Pop {
        registers: ThumbRegisterList,
        program_counter: bool,
    },
    MultipleTransfer {
        load: bool,
        base: LowRegister,
        registers: ThumbRegisterList,
    },
    ConditionalBranch {
        condition: Condition,
        displacement: i16,
    },
    UnconditionalBranch {
        displacement: i16,
    },
    BranchLink {
        displacement: i32,
    },
    SoftwareInterrupt {
        comment: u8,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ThumbAddOperand {
    Register(LowRegister),
    Immediate(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ThumbImmediateOperation {
    Move = 0,
    Compare = 1,
    Add = 2,
    Subtract = 3,
}

impl ThumbImmediateOperation {
    pub(crate) const fn from_field(value: u8) -> Self {
        match value {
            0 => Self::Move,
            1 => Self::Compare,
            2 => Self::Add,
            3 => Self::Subtract,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ThumbHighOperation {
    Add = 0,
    Compare = 1,
    Move = 2,
}

impl ThumbHighOperation {
    pub(crate) const fn from_field(value: u8) -> Option<Self> {
        Some(match value {
            0 => Self::Add,
            1 => Self::Compare,
            2 => Self::Move,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ThumbTransferOperation {
    StoreWord,
    StoreHalfword,
    StoreByte,
    LoadSignedByte,
    LoadWord,
    LoadHalfword,
    LoadByte,
    LoadSignedHalfword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ThumbTransferWidth {
    Word,
    Halfword,
    Byte,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ThumbInstructionForm {
    ShiftImmediate(ShiftKind),
    AddSubtract {
        subtract: bool,
        immediate: bool,
    },
    Immediate(ThumbImmediateOperation),
    Alu(ThumbAluOperation),
    HighRegister(ThumbHighOperation),
    BranchExchange,
    PcRelativeLoad,
    RegisterTransfer(ThumbTransferOperation),
    ImmediateTransfer {
        load: bool,
        width: ThumbTransferWidth,
    },
    SpRelativeTransfer {
        load: bool,
    },
    LoadAddress {
        from_sp: bool,
    },
    AddStackPointer {
        subtract: bool,
    },
    Push {
        link: bool,
    },
    Pop {
        program_counter: bool,
    },
    MultipleTransfer {
        load: bool,
    },
    ConditionalBranch,
    UnconditionalBranch,
    BranchLink,
    SoftwareInterrupt,
}

impl ThumbInstruction {
    pub const fn form(self) -> ThumbInstructionForm {
        match self {
            Self::ShiftImmediate { kind, .. } => ThumbInstructionForm::ShiftImmediate(kind),
            Self::AddSubtract {
                subtract, operand, ..
            } => ThumbInstructionForm::AddSubtract {
                subtract,
                immediate: matches!(operand, ThumbAddOperand::Immediate(_)),
            },
            Self::Immediate { operation, .. } => ThumbInstructionForm::Immediate(operation),
            Self::Alu { operation, .. } => ThumbInstructionForm::Alu(operation),
            Self::HighRegister { operation, .. } => ThumbInstructionForm::HighRegister(operation),
            Self::BranchExchange { .. } => ThumbInstructionForm::BranchExchange,
            Self::PcRelativeLoad { .. } => ThumbInstructionForm::PcRelativeLoad,
            Self::RegisterTransfer { operation, .. } => {
                ThumbInstructionForm::RegisterTransfer(operation)
            }
            Self::ImmediateTransfer { load, width, .. } => {
                ThumbInstructionForm::ImmediateTransfer { load, width }
            }
            Self::SpRelativeTransfer { load, .. } => {
                ThumbInstructionForm::SpRelativeTransfer { load }
            }
            Self::LoadAddress { from_sp, .. } => ThumbInstructionForm::LoadAddress { from_sp },
            Self::AddStackPointer { subtract, .. } => {
                ThumbInstructionForm::AddStackPointer { subtract }
            }
            Self::Push { link, .. } => ThumbInstructionForm::Push { link },
            Self::Pop {
                program_counter, ..
            } => ThumbInstructionForm::Pop { program_counter },
            Self::MultipleTransfer { load, .. } => ThumbInstructionForm::MultipleTransfer { load },
            Self::ConditionalBranch { .. } => ThumbInstructionForm::ConditionalBranch,
            Self::UnconditionalBranch { .. } => ThumbInstructionForm::UnconditionalBranch,
            Self::BranchLink { .. } => ThumbInstructionForm::BranchLink,
            Self::SoftwareInterrupt { .. } => ThumbInstructionForm::SoftwareInterrupt,
        }
    }

    pub const fn encoded_len(self) -> usize {
        if matches!(self, Self::BranchLink { .. }) {
            4
        } else {
            2
        }
    }
}
