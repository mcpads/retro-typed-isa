use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// An ARM architectural register numbered `r0` through `r15`.
pub struct Register(u8);

impl Register {
    /// The program counter, `r15`.
    pub const PC: Self = Self(15);
    /// The link register, `r14`.
    pub const LINK: Self = Self(14);
    /// The stack pointer, `r13`.
    pub const SP: Self = Self(13);

    /// Constructs the value after checking its architectural encoding range.
    pub const fn new(index: u8) -> Result<Self, FieldOutOfRange> {
        if index < 16 {
            Ok(Self(index))
        } else {
            Err(FieldOutOfRange::new("register", index as u32, 15))
        }
    }

    /// Returns the architectural register number.
    pub const fn index(self) -> u8 {
        self.0
    }

    pub(crate) const fn from_field(value: u32) -> Self {
        Self(value as u8)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A Thumb low register numbered `r0` through `r7`.
pub struct LowRegister(u8);

impl LowRegister {
    /// Constructs the value after checking its architectural encoding range.
    pub const fn new(index: u8) -> Result<Self, FieldOutOfRange> {
        if index < 8 {
            Ok(Self(index))
        } else {
            Err(FieldOutOfRange::new("low register", index as u32, 7))
        }
    }

    /// Returns the architectural register number.
    pub const fn index(self) -> u8 {
        self.0
    }

    /// Widens this low register to the corresponding architectural register.
    pub const fn as_register(self) -> Register {
        Register(self.0)
    }

    pub(crate) const fn from_field(value: u16) -> Self {
        Self(value as u8)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{field} value {value} is outside 0..={maximum}")]
/// Describes a typed instruction field rejected by a checked constructor.
pub struct FieldOutOfRange {
    /// The field name.
    pub field: &'static str,
    /// The rejected value.
    pub value: u32,
    /// The greatest encodable value.
    pub maximum: u32,
}

impl FieldOutOfRange {
    /// Constructs the value after checking its architectural encoding range.
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
/// An ARM condition code; `Always` is the only unconditional encoding admitted by this profile.
pub enum Condition {
    /// The equal form.
    Equal = 0x0,
    /// The not equal form.
    NotEqual = 0x1,
    /// The carry set form.
    CarrySet = 0x2,
    /// The carry clear form.
    CarryClear = 0x3,
    /// The negative form.
    Negative = 0x4,
    /// The positive or zero form.
    PositiveOrZero = 0x5,
    /// The overflow form.
    Overflow = 0x6,
    /// The no overflow form.
    NoOverflow = 0x7,
    /// The unsigned higher form.
    UnsignedHigher = 0x8,
    /// The unsigned lower or same form.
    UnsignedLowerOrSame = 0x9,
    /// The signed greater or equal form.
    SignedGreaterOrEqual = 0xa,
    /// The signed less than form.
    SignedLessThan = 0xb,
    /// The signed greater than form.
    SignedGreaterThan = 0xc,
    /// The signed less or equal form.
    SignedLessOrEqual = 0xd,
    /// The always form.
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
/// An ARM data-processing opcode.
pub enum DataOperation {
    /// The and form.
    And = 0x0,
    /// The exclusive or form.
    ExclusiveOr = 0x1,
    /// The subtract form.
    Subtract = 0x2,
    /// The reverse subtract form.
    ReverseSubtract = 0x3,
    /// The add form.
    Add = 0x4,
    /// The add with carry form.
    AddWithCarry = 0x5,
    /// The subtract with carry form.
    SubtractWithCarry = 0x6,
    /// The reverse subtract with carry form.
    ReverseSubtractWithCarry = 0x7,
    /// The test form.
    Test = 0x8,
    /// The test equivalence form.
    TestEquivalence = 0x9,
    /// The compare form.
    Compare = 0xa,
    /// The compare negative form.
    CompareNegative = 0xb,
    /// The or form.
    Or = 0xc,
    /// The move form.
    Move = 0xd,
    /// The bit clear form.
    BitClear = 0xe,
    /// The move not form.
    MoveNot = 0xf,
}

impl DataOperation {
    /// Returns whether this operation updates flags without writing a destination register.
    pub const fn is_test(self) -> bool {
        matches!(
            self,
            Self::Test | Self::TestEquivalence | Self::Compare | Self::CompareNegative
        )
    }

    /// Returns whether the encoding ignores the first source-register field.
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
/// A barrel-shifter operation shared by ARM and Thumb encodings.
pub enum ShiftKind {
    /// The logical left form.
    LogicalLeft = 0,
    /// The logical right form.
    LogicalRight = 1,
    /// The arithmetic right form.
    ArithmeticRight = 2,
    /// The rotate right form.
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
/// An ARM immediate represented as an eight-bit value rotated right by an even amount.
pub struct RotatedImmediate {
    value: u8,
    rotate_right: u8,
}

impl RotatedImmediate {
    /// Constructs the value after checking its architectural encoding range.
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

    /// Returns the unrotated encoded value.
    pub const fn value(self) -> u8 {
        self.value
    }

    /// Returns the rotation amount in bits.
    pub const fn rotate_right(self) -> u8 {
        self.rotate_right
    }

    /// Returns the 32-bit immediate value represented by this encoding.
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
/// A checked ARM immediate shift, including the encoded `RRX` special case.
pub enum ImmediateShift {
    /// The logical left form.
    LogicalLeft(u8),
    /// The logical right form.
    LogicalRight(u8),
    /// The arithmetic right form.
    ArithmeticRight(u8),
    /// The rotate right form.
    RotateRight(u8),
    /// The rotate right extended form.
    RotateRightExtended,
}

impl ImmediateShift {
    /// Constructs the value after checking its architectural encoding range.
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
/// The typed second operand of an ARM data-processing instruction.
pub enum Operand2 {
    /// The immediate form.
    Immediate(RotatedImmediate),
    /// The register form.
    Register {
        /// The register operand.
        register: Register,
        /// The shift operand.
        shift: ImmediateShift,
    },
    /// The register shift form.
    RegisterShift {
        /// The register operand.
        register: Register,
        /// The selected operation or transfer kind.
        kind: ShiftKind,
        /// The shift amount.
        amount: Register,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The address-index and write-back mode of an ARM memory operand.
pub enum IndexMode {
    /// The offset form.
    Offset,
    /// The pre indexed form.
    PreIndexed,
    /// The post indexed form.
    PostIndexed,
    /// The post indexed user form.
    PostIndexedUser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The immediate or shifted-register offset used by ARM addressing mode 2.
pub enum AddressOffset {
    /// The immediate form.
    Immediate(u16),
    /// The register form.
    Register {
        /// The register operand.
        register: Register,
        /// The shift operand.
        shift: ImmediateShift,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A typed ARM single-data-transfer address.
pub struct AddressingMode2 {
    /// The base register.
    pub base: Register,
    /// The encoded address or displacement offset.
    pub offset: AddressOffset,
    /// Whether the offset is added rather than subtracted.
    pub add: bool,
    /// The indexing and write-back mode.
    pub index: IndexMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The immediate or register offset used by ARM addressing mode 3.
pub enum HalfwordOffset {
    /// The immediate form.
    Immediate(u8),
    /// The register form.
    Register(Register),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A typed ARM halfword and signed-transfer address.
pub struct AddressingMode3 {
    /// The base register.
    pub base: Register,
    /// The encoded address or displacement offset.
    pub offset: HalfwordOffset,
    /// Whether the offset is added rather than subtracted.
    pub add: bool,
    /// The indexing and write-back mode.
    pub index: IndexMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The width selected by an ARM single transfer or swap.
pub enum TransferWidth {
    /// The word form.
    Word,
    /// The byte form.
    Byte,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The data interpretation selected by an ARM halfword or signed transfer.
pub enum HalfwordTransferKind {
    /// The unsigned halfword form.
    UnsignedHalfword,
    /// The signed byte form.
    SignedByte,
    /// The signed halfword form.
    SignedHalfword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
/// The increment/decrement and before/after mode of an ARM block transfer.
pub enum BlockAddressing {
    /// The decrement after form.
    DecrementAfter = 0,
    /// The increment after form.
    IncrementAfter = 1,
    /// The decrement before form.
    DecrementBefore = 2,
    /// The increment before form.
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
/// A non-empty ARM register mask for block transfers.
pub struct RegisterList(u16);

impl RegisterList {
    /// Constructs the value after checking its architectural encoding range.
    pub const fn new(bits: u16) -> Result<Self, EmptyRegisterList> {
        if bits == 0 {
            Err(EmptyRegisterList)
        } else {
            Ok(Self(bits))
        }
    }

    /// Returns the encoded register or status-field mask.
    pub const fn bits(self) -> u16 {
        self.0
    }

    pub(crate) const fn from_bits(bits: u16) -> Self {
        Self(bits)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("register list cannot be empty")]
/// Returned when a block-transfer register mask contains no registers.
pub struct EmptyRegisterList;

macro_rules! four_bit_field {
    ($name:ident, $label:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        /// A typed $name value.
        pub struct $name(u8);

        impl $name {
            /// Constructs the value after checking its architectural encoding range.
            pub const fn new(value: u8) -> Result<Self, FieldOutOfRange> {
                if value < 16 {
                    Ok(Self(value))
                } else {
                    Err(FieldOutOfRange::new($label, value as u32, 15))
                }
            }

            /// Returns the unrotated encoded value.
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
/// Selects the current program status register or saved program status register.
pub enum StatusRegister {
    /// The current form.
    Current,
    /// The saved form.
    Saved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A non-empty field mask for an ARM status-register write.
pub struct StatusMask(u8);

impl StatusMask {
    /// The control-byte status field.
    pub const CONTROL: Self = Self(0b0001);
    /// The extension-byte status field.
    pub const EXTENSION: Self = Self(0b0010);
    /// The status-byte field.
    pub const STATUS: Self = Self(0b0100);
    /// The condition-flags field.
    pub const FLAGS: Self = Self(0b1000);

    /// Constructs the value after checking its architectural encoding range.
    pub const fn new(bits: u8) -> Result<Self, FieldOutOfRange> {
        if bits != 0 && bits < 16 {
            Ok(Self(bits))
        } else {
            Err(FieldOutOfRange::new("status field mask", bits as u32, 15))
        }
    }

    /// Returns the encoded register or status-field mask.
    pub const fn bits(self) -> u8 {
        self.0
    }

    pub(crate) const fn from_bits(bits: u8) -> Self {
        Self(bits)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The register or rotated-immediate source of a status-register write.
pub enum StatusSource {
    /// The register form.
    Register(Register),
    /// The immediate form.
    Immediate(RotatedImmediate),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A complete typed ARM-state instruction admitted by the ARM7TDMI profile.
pub enum ArmInstruction {
    /// The data processing form.
    DataProcessing {
        /// The condition under which the instruction executes.
        condition: Condition,
        /// The selected operation.
        operation: DataOperation,
        /// Whether the instruction updates condition flags.
        set_flags: bool,
        /// The destination register or status register.
        destination: Register,
        /// The first source operand.
        first: Register,
        /// The second source operand.
        second: Operand2,
    },
    /// The multiply form.
    Multiply {
        /// The condition under which the instruction executes.
        condition: Condition,
        /// Whether the instruction adds an accumulator.
        accumulate: bool,
        /// Whether the instruction updates condition flags.
        set_flags: bool,
        /// The destination register or status register.
        destination: Register,
        /// The multiplicand register.
        multiplicand: Register,
        /// The multiplier register.
        multiplier: Register,
        /// The accumulator register.
        addend: Register,
    },
    /// The multiply long form.
    MultiplyLong {
        /// The condition under which the instruction executes.
        condition: Condition,
        /// Whether operands are interpreted as signed values.
        signed: bool,
        /// Whether the instruction adds an accumulator.
        accumulate: bool,
        /// Whether the instruction updates condition flags.
        set_flags: bool,
        /// The register receiving the low result word.
        destination_low: Register,
        /// The register receiving the high result word.
        destination_high: Register,
        /// The multiplicand register.
        multiplicand: Register,
        /// The multiplier register.
        multiplier: Register,
    },
    /// The branch exchange form.
    BranchExchange {
        /// The condition under which the instruction executes.
        condition: Condition,
        /// The branch target register.
        target: Register,
    },
    /// The branch form.
    Branch {
        /// The condition under which the instruction executes.
        condition: Condition,
        /// Whether the instruction records a return address.
        link: bool,
        /// The byte displacement relative to the architecturally visible program counter.
        displacement: i32,
    },
    /// The single transfer form.
    SingleTransfer {
        /// The condition under which the instruction executes.
        condition: Condition,
        /// Whether the transfer loads rather than stores.
        load: bool,
        /// The transfer width.
        width: TransferWidth,
        /// The register operand.
        register: Register,
        /// The typed memory address.
        address: AddressingMode2,
    },
    /// The halfword transfer form.
    HalfwordTransfer {
        /// The condition under which the instruction executes.
        condition: Condition,
        /// Whether the transfer loads rather than stores.
        load: bool,
        /// The selected operation or transfer kind.
        kind: HalfwordTransferKind,
        /// The register operand.
        register: Register,
        /// The typed memory address.
        address: AddressingMode3,
    },
    /// The block transfer form.
    BlockTransfer {
        /// The condition under which the instruction executes.
        condition: Condition,
        /// Whether the transfer loads rather than stores.
        load: bool,
        /// The block addressing mode.
        addressing: BlockAddressing,
        /// Whether the updated base address is written back.
        write_back: bool,
        /// Whether the transfer uses the user-mode register bank.
        user_registers: bool,
        /// The base register.
        base: Register,
        /// The transferred register mask.
        registers: RegisterList,
    },
    /// The swap form.
    Swap {
        /// The condition under which the instruction executes.
        condition: Condition,
        /// The transfer width.
        width: TransferWidth,
        /// The destination register or status register.
        destination: Register,
        /// The source register or value.
        source: Register,
        /// The base register.
        base: Register,
    },
    /// The move status to register form.
    MoveStatusToRegister {
        /// The condition under which the instruction executes.
        condition: Condition,
        /// The source register or value.
        source: StatusRegister,
        /// The destination register or status register.
        destination: Register,
    },
    /// The move to status register form.
    MoveToStatusRegister {
        /// The condition under which the instruction executes.
        condition: Condition,
        /// The destination register or status register.
        destination: StatusRegister,
        /// The status fields written by the instruction.
        mask: StatusMask,
        /// The source register or value.
        source: StatusSource,
    },
    /// The coprocessor data operation form.
    CoprocessorDataOperation {
        /// The condition under which the instruction executes.
        condition: Condition,
        /// The selected coprocessor number.
        coprocessor: Coprocessor,
        /// The coprocessor operation field.
        opcode: CoprocessorOpcode,
        /// The coprocessor information field.
        info: u8,
        /// The destination register or status register.
        destination: CoprocessorRegister,
        /// The first source operand.
        first: CoprocessorRegister,
        /// The second source operand.
        second: CoprocessorRegister,
    },
    /// The coprocessor register transfer form.
    CoprocessorRegisterTransfer {
        /// The condition under which the instruction executes.
        condition: Condition,
        /// Whether the transfer loads rather than stores.
        load: bool,
        /// The selected coprocessor number.
        coprocessor: Coprocessor,
        /// The coprocessor operation field.
        opcode: u8,
        /// The coprocessor information field.
        info: u8,
        /// The ARM register transferred to or from the coprocessor.
        arm_register: Register,
        /// The first source operand.
        first: CoprocessorRegister,
        /// The second source operand.
        second: CoprocessorRegister,
    },
    /// The coprocessor data transfer form.
    CoprocessorDataTransfer {
        /// The condition under which the instruction executes.
        condition: Condition,
        /// Whether the transfer loads rather than stores.
        load: bool,
        /// Whether the coprocessor transfer uses its long form.
        long: bool,
        /// The selected coprocessor number.
        coprocessor: Coprocessor,
        /// The register operand.
        register: CoprocessorRegister,
        /// The base register.
        base: Register,
        /// The address displacement measured in words.
        offset_words: u8,
        /// Whether the offset is added rather than subtracted.
        add: bool,
        /// The indexing and write-back mode.
        index: IndexMode,
    },
    /// The software interrupt form.
    SoftwareInterrupt {
        /// The condition under which the instruction executes.
        condition: Condition,
        /// The software-interrupt comment field.
        comment: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The operand-independent ARM instruction form used by static analysis.
pub enum ArmInstructionForm {
    /// The data processing form.
    DataProcessing(DataOperation),
    /// The multiply form.
    Multiply,
    /// The multiply accumulate form.
    MultiplyAccumulate,
    /// The multiply long form.
    MultiplyLong {
        /// Whether operands are interpreted as signed values.
        signed: bool,
        /// Whether the instruction adds an accumulator.
        accumulate: bool,
    },
    /// The branch exchange form.
    BranchExchange,
    /// The branch form.
    Branch,
    /// The branch link form.
    BranchLink,
    /// The single transfer form.
    SingleTransfer {
        /// Whether the transfer loads rather than stores.
        load: bool,
        /// The transfer width.
        width: TransferWidth,
    },
    /// The halfword transfer form.
    HalfwordTransfer {
        /// Whether the transfer loads rather than stores.
        load: bool,
        /// The selected operation or transfer kind.
        kind: HalfwordTransferKind,
    },
    /// The block transfer form.
    BlockTransfer {
        /// Whether the transfer loads rather than stores.
        load: bool,
    },
    /// The swap form.
    Swap(TransferWidth),
    /// The move status to register form.
    MoveStatusToRegister(StatusRegister),
    /// The move to status register form.
    MoveToStatusRegister {
        /// The destination register or status register.
        destination: StatusRegister,
        /// Whether the form uses an immediate operand.
        immediate: bool,
    },
    /// The coprocessor data operation form.
    CoprocessorDataOperation,
    /// The coprocessor register transfer form.
    CoprocessorRegisterTransfer {
        /// Whether the transfer loads rather than stores.
        load: bool,
    },
    /// The coprocessor data transfer form.
    CoprocessorDataTransfer {
        /// Whether the transfer loads rather than stores.
        load: bool,
    },
    /// The software interrupt form.
    SoftwareInterrupt,
}

impl ArmInstruction {
    /// Returns the operand-independent instruction form.
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
/// A Thumb register ALU opcode.
pub enum ThumbAluOperation {
    /// The and form.
    And = 0x0,
    /// The exclusive or form.
    ExclusiveOr = 0x1,
    /// The logical shift left form.
    LogicalShiftLeft = 0x2,
    /// The logical shift right form.
    LogicalShiftRight = 0x3,
    /// The arithmetic shift right form.
    ArithmeticShiftRight = 0x4,
    /// The add with carry form.
    AddWithCarry = 0x5,
    /// The subtract with carry form.
    SubtractWithCarry = 0x6,
    /// The rotate right form.
    RotateRight = 0x7,
    /// The test form.
    Test = 0x8,
    /// The negate form.
    Negate = 0x9,
    /// The compare form.
    Compare = 0xa,
    /// The compare negative form.
    CompareNegative = 0xb,
    /// The or form.
    Or = 0xc,
    /// The multiply form.
    Multiply = 0xd,
    /// The bit clear form.
    BitClear = 0xe,
    /// The move not form.
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
/// A Thumb low-register mask used by push, pop, and multiple transfer.
pub struct ThumbRegisterList(u8);

impl ThumbRegisterList {
    /// An empty low-register mask, usable only where the encoding supplies an extra register.
    pub const EMPTY: Self = Self(0);

    /// Constructs the value after checking its architectural encoding range.
    pub const fn new(bits: u8) -> Result<Self, EmptyRegisterList> {
        if bits == 0 {
            Err(EmptyRegisterList)
        } else {
            Ok(Self(bits))
        }
    }

    /// Returns the encoded register or status-field mask.
    pub const fn bits(self) -> u8 {
        self.0
    }

    pub(crate) const fn from_bits(bits: u8) -> Self {
        Self(bits)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A complete typed Thumb-state instruction admitted by the ARM7TDMI profile.
pub enum ThumbInstruction {
    /// The shift immediate form.
    ShiftImmediate {
        /// The selected operation or transfer kind.
        kind: ShiftKind,
        /// The destination register or status register.
        destination: LowRegister,
        /// The source register or value.
        source: LowRegister,
        /// The shift amount.
        amount: u8,
    },
    /// The add subtract form.
    AddSubtract {
        /// Whether subtraction is selected.
        subtract: bool,
        /// The destination register or status register.
        destination: LowRegister,
        /// The source register or value.
        source: LowRegister,
        /// The typed add/subtract operand.
        operand: ThumbAddOperand,
    },
    /// The immediate form.
    Immediate {
        /// The selected operation.
        operation: ThumbImmediateOperation,
        /// The register operand.
        register: LowRegister,
        /// Whether the form uses an immediate operand.
        immediate: u8,
    },
    /// The alu form.
    Alu {
        /// The selected operation.
        operation: ThumbAluOperation,
        /// The destination register or status register.
        destination: LowRegister,
        /// The source register or value.
        source: LowRegister,
    },
    /// The high register form.
    HighRegister {
        /// The selected operation.
        operation: ThumbHighOperation,
        /// The destination register or status register.
        destination: Register,
        /// The source register or value.
        source: Register,
    },
    /// The branch exchange form.
    BranchExchange {
        /// The source register or value.
        source: Register,
    },
    /// The pc relative load form.
    PcRelativeLoad {
        /// The destination register or status register.
        destination: LowRegister,
        /// The encoded address or displacement offset.
        offset: u16,
    },
    /// The register transfer form.
    RegisterTransfer {
        /// The selected operation.
        operation: ThumbTransferOperation,
        /// The destination register or status register.
        destination: LowRegister,
        /// The base register.
        base: LowRegister,
        /// The encoded address or displacement offset.
        offset: LowRegister,
    },
    /// The immediate transfer form.
    ImmediateTransfer {
        /// Whether the transfer loads rather than stores.
        load: bool,
        /// The transfer width.
        width: ThumbTransferWidth,
        /// The destination register or status register.
        destination: LowRegister,
        /// The base register.
        base: LowRegister,
        /// The encoded address or displacement offset.
        offset: u8,
    },
    /// The sp relative transfer form.
    SpRelativeTransfer {
        /// Whether the transfer loads rather than stores.
        load: bool,
        /// The register operand.
        register: LowRegister,
        /// The encoded address or displacement offset.
        offset: u16,
    },
    /// The load address form.
    LoadAddress {
        /// Whether the address is based on the stack pointer rather than the program counter.
        from_sp: bool,
        /// The destination register or status register.
        destination: LowRegister,
        /// The encoded address or displacement offset.
        offset: u16,
    },
    /// The add stack pointer form.
    AddStackPointer {
        /// Whether subtraction is selected.
        subtract: bool,
        /// The encoded address or displacement offset.
        offset: u16,
    },
    /// The push form.
    Push {
        /// The transferred register mask.
        registers: ThumbRegisterList,
        /// Whether the instruction records a return address.
        link: bool,
    },
    /// The pop form.
    Pop {
        /// The transferred register mask.
        registers: ThumbRegisterList,
        /// Whether the pop list includes the program counter.
        program_counter: bool,
    },
    /// The multiple transfer form.
    MultipleTransfer {
        /// Whether the transfer loads rather than stores.
        load: bool,
        /// The base register.
        base: LowRegister,
        /// The transferred register mask.
        registers: ThumbRegisterList,
    },
    /// The conditional branch form.
    ConditionalBranch {
        /// The condition under which the instruction executes.
        condition: Condition,
        /// The byte displacement relative to the architecturally visible program counter.
        displacement: i16,
    },
    /// The unconditional branch form.
    UnconditionalBranch {
        /// The byte displacement relative to the architecturally visible program counter.
        displacement: i16,
    },
    /// The branch link form.
    BranchLink {
        /// The byte displacement relative to the architecturally visible program counter.
        displacement: i32,
    },
    /// The software interrupt form.
    SoftwareInterrupt {
        /// The software-interrupt comment field.
        comment: u8,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The register or three-bit immediate operand of Thumb add/subtract.
pub enum ThumbAddOperand {
    /// The register form.
    Register(LowRegister),
    /// The immediate form.
    Immediate(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
/// A Thumb eight-bit-immediate data operation.
pub enum ThumbImmediateOperation {
    /// The move form.
    Move = 0,
    /// The compare form.
    Compare = 1,
    /// The add form.
    Add = 2,
    /// The subtract form.
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
/// A Thumb high-register operation.
pub enum ThumbHighOperation {
    /// The add form.
    Add = 0,
    /// The compare form.
    Compare = 1,
    /// The move form.
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
/// A Thumb register-offset load/store operation.
pub enum ThumbTransferOperation {
    /// The store word form.
    StoreWord,
    /// The store halfword form.
    StoreHalfword,
    /// The store byte form.
    StoreByte,
    /// The load signed byte form.
    LoadSignedByte,
    /// The load word form.
    LoadWord,
    /// The load halfword form.
    LoadHalfword,
    /// The load byte form.
    LoadByte,
    /// The load signed halfword form.
    LoadSignedHalfword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The width of a Thumb immediate-offset load/store.
pub enum ThumbTransferWidth {
    /// The word form.
    Word,
    /// The halfword form.
    Halfword,
    /// The byte form.
    Byte,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The operand-independent Thumb instruction form used by static analysis.
pub enum ThumbInstructionForm {
    /// The shift immediate form.
    ShiftImmediate(ShiftKind),
    /// The add subtract form.
    AddSubtract {
        /// Whether subtraction is selected.
        subtract: bool,
        /// Whether the form uses an immediate operand.
        immediate: bool,
    },
    /// The immediate form.
    Immediate(ThumbImmediateOperation),
    /// The alu form.
    Alu(ThumbAluOperation),
    /// The high register form.
    HighRegister(ThumbHighOperation),
    /// The branch exchange form.
    BranchExchange,
    /// The pc relative load form.
    PcRelativeLoad,
    /// The register transfer form.
    RegisterTransfer(ThumbTransferOperation),
    /// The immediate transfer form.
    ImmediateTransfer {
        /// Whether the transfer loads rather than stores.
        load: bool,
        /// The transfer width.
        width: ThumbTransferWidth,
    },
    /// The sp relative transfer form.
    SpRelativeTransfer {
        /// Whether the transfer loads rather than stores.
        load: bool,
    },
    /// The load address form.
    LoadAddress {
        /// Whether the address is based on the stack pointer rather than the program counter.
        from_sp: bool,
    },
    /// The add stack pointer form.
    AddStackPointer {
        /// Whether subtraction is selected.
        subtract: bool,
    },
    /// The push form.
    Push {
        /// Whether the instruction records a return address.
        link: bool,
    },
    /// The pop form.
    Pop {
        /// Whether the pop list includes the program counter.
        program_counter: bool,
    },
    /// The multiple transfer form.
    MultipleTransfer {
        /// Whether the transfer loads rather than stores.
        load: bool,
    },
    /// The conditional branch form.
    ConditionalBranch,
    /// The unconditional branch form.
    UnconditionalBranch,
    /// The branch link form.
    BranchLink,
    /// The software interrupt form.
    SoftwareInterrupt,
}

impl ThumbInstruction {
    /// Returns the operand-independent instruction form.
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

    /// Returns the encoded instruction length in bytes.
    pub const fn encoded_len(self) -> usize {
        if matches!(self, Self::BranchLink { .. }) {
            4
        } else {
            2
        }
    }
}
