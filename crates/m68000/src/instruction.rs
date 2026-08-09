use std::collections::BTreeSet;

use crate::classify_word;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
/// One of the eight MC68000 data registers.
pub enum DataRegister {
    /// The d0 form.
    D0,
    /// The d1 form.
    D1,
    /// The d2 form.
    D2,
    /// The d3 form.
    D3,
    /// The d4 form.
    D4,
    /// The d5 form.
    D5,
    /// The d6 form.
    D6,
    /// The d7 form.
    D7,
}

impl DataRegister {
    /// The complete all set or constant.
    pub const ALL: [Self; 8] = [
        Self::D0,
        Self::D1,
        Self::D2,
        Self::D3,
        Self::D4,
        Self::D5,
        Self::D6,
        Self::D7,
    ];

    pub(crate) const fn bits(self) -> u16 {
        self as u16
    }

    pub(crate) const fn from_bits(bits: u16) -> Self {
        Self::ALL[(bits & 7) as usize]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
/// One of the eight MC68000 address registers.
pub enum AddressRegister {
    /// The a0 form.
    A0,
    /// The a1 form.
    A1,
    /// The a2 form.
    A2,
    /// The a3 form.
    A3,
    /// The a4 form.
    A4,
    /// The a5 form.
    A5,
    /// The a6 form.
    A6,
    /// The a7 form.
    A7,
}

impl AddressRegister {
    /// The complete all set or constant.
    pub const ALL: [Self; 8] = [
        Self::A0,
        Self::A1,
        Self::A2,
        Self::A3,
        Self::A4,
        Self::A5,
        Self::A6,
        Self::A7,
    ];

    pub(crate) const fn bits(self) -> u16 {
        self as u16
    }

    pub(crate) const fn from_bits(bits: u16) -> Self {
        Self::ALL[(bits & 7) as usize]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A data or address register usable by a register mask or index operand.
pub enum Register {
    /// The data form.
    Data(DataRegister),
    /// The address form.
    Address(AddressRegister),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The byte, word, or long-word width of an integer operand.
pub enum OperandSize {
    /// The byte form.
    Byte,
    /// The word form.
    Word,
    /// The long form.
    Long,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The word or long-word width of an address-register operation.
pub enum AddressSize {
    /// The word form.
    Word,
    /// The long form.
    Long,
}

impl From<AddressSize> for OperandSize {
    fn from(size: AddressSize) -> Self {
        match size {
            AddressSize::Word => Self::Word,
            AddressSize::Long => Self::Long,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// An immediate value whose Rust variant preserves its encoded width.
pub enum Immediate {
    /// The byte form.
    Byte(u8),
    /// The word form.
    Word(u16),
    /// The long form.
    Long(u32),
}

impl Immediate {
    /// Returns the encoded width carried by this immediate.
    pub const fn size(self) -> OperandSize {
        match self {
            Self::Byte(_) => OperandSize::Byte,
            Self::Word(_) => OperandSize::Word,
            Self::Long(_) => OperandSize::Long,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The word or long-word interpretation of a brief-extension index register.
pub enum IndexSize {
    /// The word form.
    Word,
    /// The long form.
    Long,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The register and width stored in an MC68000 brief index extension.
pub struct IndexRegister {
    /// The selected register.
    pub register: Register,
    /// The operand or index width.
    pub size: IndexSize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A complete typed MC68000 effective address, including extension values.
pub enum EffectiveAddress {
    /// The data register form.
    DataRegister(DataRegister),
    /// The address register form.
    AddressRegister(AddressRegister),
    /// The address indirect form.
    AddressIndirect(AddressRegister),
    /// The address postincrement form.
    AddressPostincrement(AddressRegister),
    /// The address predecrement form.
    AddressPredecrement(AddressRegister),
    /// The address displacement form.
    AddressDisplacement {
        /// The selected register.
        register: AddressRegister,
        /// The encoded signed displacement.
        displacement: i16,
    },
    /// The address index form.
    AddressIndex {
        /// The selected register.
        register: AddressRegister,
        /// The brief-extension index register.
        index: IndexRegister,
        /// The encoded signed displacement.
        displacement: i8,
    },
    /// The absolute word form.
    AbsoluteWord(u16),
    /// The absolute long form.
    AbsoluteLong(u32),
    /// The program counter displacement form.
    ProgramCounterDisplacement(i16),
    /// The program counter index form.
    ProgramCounterIndex {
        /// The brief-extension index register.
        index: IndexRegister,
        /// The encoded signed displacement.
        displacement: i8,
    },
    /// The immediate form.
    Immediate(Immediate),
}

impl EffectiveAddress {
    /// Returns the structural addressing mode.
    pub const fn mode(self) -> AddressingMode {
        match self {
            Self::DataRegister(_) => AddressingMode::DataRegisterDirect,
            Self::AddressRegister(_) => AddressingMode::AddressRegisterDirect,
            Self::AddressIndirect(_) => AddressingMode::AddressRegisterIndirect,
            Self::AddressPostincrement(_) => AddressingMode::AddressRegisterIndirectPostincrement,
            Self::AddressPredecrement(_) => AddressingMode::AddressRegisterIndirectPredecrement,
            Self::AddressDisplacement { .. } => AddressingMode::AddressRegisterIndirectDisplacement,
            Self::AddressIndex { .. } => AddressingMode::AddressRegisterIndirectIndex,
            Self::AbsoluteWord(_) => AddressingMode::AbsoluteShort,
            Self::AbsoluteLong(_) => AddressingMode::AbsoluteLong,
            Self::ProgramCounterDisplacement(_) => {
                AddressingMode::ProgramCounterRelativeDisplacement
            }
            Self::ProgramCounterIndex { .. } => AddressingMode::ProgramCounterRelativeIndex,
            Self::Immediate(_) => AddressingMode::Immediate,
        }
    }

    /// Returns the static-analysis operand classification.
    pub const fn operand_kind(self) -> OperandKind {
        match self {
            Self::DataRegister(_) => OperandKind::DataRegister,
            Self::AddressRegister(_) => OperandKind::AddressRegister,
            Self::AddressIndirect(_) => OperandKind::AddressIndirect,
            Self::AddressPostincrement(_) => OperandKind::AddressPostincrement,
            Self::AddressPredecrement(_) => OperandKind::AddressPredecrement,
            Self::AddressDisplacement { .. } => OperandKind::AddressDisplacement,
            Self::AddressIndex { .. } => OperandKind::AddressIndex,
            Self::AbsoluteWord(_) => OperandKind::AbsoluteWord,
            Self::AbsoluteLong(_) => OperandKind::AbsoluteLong,
            Self::ProgramCounterDisplacement(_) => OperandKind::ProgramCounterDisplacement,
            Self::ProgramCounterIndex { .. } => OperandKind::ProgramCounterIndex,
            Self::Immediate(_) => OperandKind::Immediate,
        }
    }
}

/// The fourteen addressing modes documented for the MC68000 profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AddressingMode {
    /// The data register direct form.
    DataRegisterDirect,
    /// The address register direct form.
    AddressRegisterDirect,
    /// The address register indirect form.
    AddressRegisterIndirect,
    /// The address register indirect postincrement form.
    AddressRegisterIndirectPostincrement,
    /// The address register indirect predecrement form.
    AddressRegisterIndirectPredecrement,
    /// The address register indirect displacement form.
    AddressRegisterIndirectDisplacement,
    /// The address register indirect index form.
    AddressRegisterIndirectIndex,
    /// The absolute short form.
    AbsoluteShort,
    /// The absolute long form.
    AbsoluteLong,
    /// The program counter relative displacement form.
    ProgramCounterRelativeDisplacement,
    /// The program counter relative index form.
    ProgramCounterRelativeIndex,
    /// The immediate form.
    Immediate,
    /// The quick immediate form.
    QuickImmediate,
    /// The implied form.
    Implied,
}

impl AddressingMode {
    /// The complete all set or constant.
    pub const ALL: [Self; 14] = [
        Self::DataRegisterDirect,
        Self::AddressRegisterDirect,
        Self::AddressRegisterIndirect,
        Self::AddressRegisterIndirectPostincrement,
        Self::AddressRegisterIndirectPredecrement,
        Self::AddressRegisterIndirectDisplacement,
        Self::AddressRegisterIndirectIndex,
        Self::AbsoluteShort,
        Self::AbsoluteLong,
        Self::ProgramCounterRelativeDisplacement,
        Self::ProgramCounterRelativeIndex,
        Self::Immediate,
        Self::QuickImmediate,
        Self::Implied,
    ];
}

/// Operand-location classes retained by [`InstructionForm`].
///
/// Unlike [`AddressingMode`], this also distinguishes fixed architectural
/// operands whose identity changes the instruction form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OperandKind {
    /// The data register form.
    DataRegister,
    /// The address register form.
    AddressRegister,
    /// The address indirect form.
    AddressIndirect,
    /// The address postincrement form.
    AddressPostincrement,
    /// The address predecrement form.
    AddressPredecrement,
    /// The address displacement form.
    AddressDisplacement,
    /// The address index form.
    AddressIndex,
    /// The absolute word form.
    AbsoluteWord,
    /// The absolute long form.
    AbsoluteLong,
    /// The program counter displacement form.
    ProgramCounterDisplacement,
    /// The program counter index form.
    ProgramCounterIndex,
    /// The immediate form.
    Immediate,
    /// The quick immediate form.
    QuickImmediate,
    /// The register list form.
    RegisterList,
    /// The user stack pointer form.
    UserStackPointer,
    /// The condition code register form.
    ConditionCodeRegister,
    /// The status register form.
    StatusRegister,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Selects whether a data register is the source or destination of an ALU form.
pub enum DataDirection {
    /// The to data register form.
    ToDataRegister,
    /// The to effective address form.
    ToEffectiveAddress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The two data registers used by decimal and extended arithmetic.
pub enum RegisterPair {
    /// The data form.
    Data {
        /// The source operand.
        source: DataRegister,
        /// The destination operand.
        destination: DataRegister,
    },
    /// The address predecrement form.
    AddressPredecrement {
        /// The source operand.
        source: AddressRegister,
        /// The destination operand.
        destination: AddressRegister,
    },
}

impl RegisterPair {
    /// Returns the source operand classification for this register pair.
    pub const fn source_operand_kind(self) -> OperandKind {
        match self {
            Self::Data { .. } => OperandKind::DataRegister,
            Self::AddressPredecrement { .. } => OperandKind::AddressPredecrement,
        }
    }

    /// Returns the destination operand classification for this register pair.
    pub const fn destination_operand_kind(self) -> OperandKind {
        self.source_operand_kind()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
/// An MC68000 condition-code selector.
pub enum Condition {
    /// The true form.
    True,
    /// The false form.
    False,
    /// The high form.
    High,
    /// The low or same form.
    LowOrSame,
    /// The carry clear form.
    CarryClear,
    /// The carry set form.
    CarrySet,
    /// The not equal form.
    NotEqual,
    /// The equal form.
    Equal,
    /// The overflow clear form.
    OverflowClear,
    /// The overflow set form.
    OverflowSet,
    /// The plus form.
    Plus,
    /// The minus form.
    Minus,
    /// The greater or equal form.
    GreaterOrEqual,
    /// The less than form.
    LessThan,
    /// The greater than form.
    GreaterThan,
    /// The less or equal form.
    LessOrEqual,
}

impl Condition {
    /// The complete all set or constant.
    pub const ALL: [Self; 16] = [
        Self::True,
        Self::False,
        Self::High,
        Self::LowOrSame,
        Self::CarryClear,
        Self::CarrySet,
        Self::NotEqual,
        Self::Equal,
        Self::OverflowClear,
        Self::OverflowSet,
        Self::Plus,
        Self::Minus,
        Self::GreaterOrEqual,
        Self::LessThan,
        Self::GreaterThan,
        Self::LessOrEqual,
    ];

    pub(crate) const fn bits(self) -> u16 {
        self as u16
    }

    pub(crate) const fn from_bits(bits: u16) -> Self {
        Self::ALL[(bits & 0x0f) as usize]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The unconditional, subroutine, or conditional branch family.
pub enum BranchKind {
    /// The always form.
    Always,
    /// The subroutine form.
    Subroutine,
    /// The conditional form.
    Conditional(Condition),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A branch displacement with an explicit byte or word encoding width.
pub enum BranchDisplacement {
    /// The byte form.
    Byte(i8),
    /// The word form.
    Word(i16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A bit-test or bit-modification operation.
pub enum BitOperation {
    /// The test form.
    Test,
    /// The change form.
    Change,
    /// The clear form.
    Clear,
    /// The set form.
    Set,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The immediate or data-register source of a bit number.
pub enum BitNumber {
    /// The data register form.
    DataRegister(DataRegister),
    /// The immediate form.
    Immediate(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Selects signed or unsigned integer division.
pub enum DivisionSign {
    /// The unsigned form.
    Unsigned,
    /// The signed form.
    Signed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Selects signed or unsigned integer multiplication.
pub enum MultiplicationSign {
    /// The unsigned form.
    Unsigned,
    /// The signed form.
    Signed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A legal register-pair class for `EXG`.
pub enum ExchangeRegisters {
    /// The data form.
    Data {
        /// The source operand.
        source: DataRegister,
        /// The destination operand.
        destination: DataRegister,
    },
    /// The address form.
    Address {
        /// The source operand.
        source: AddressRegister,
        /// The destination operand.
        destination: AddressRegister,
    },
    /// The data address form.
    DataAddress {
        /// The data-register operand.
        data: DataRegister,
        /// The absolute address.
        address: AddressRegister,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The byte, word, or long-word size used by sign extension.
pub enum ExtendSize {
    /// The byte to word form.
    ByteToWord,
    /// The word to long form.
    WordToLong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A shift or rotate operation.
pub enum ShiftOperation {
    /// The arithmetic form.
    Arithmetic,
    /// The logical form.
    Logical,
    /// The rotate with extend form.
    RotateWithExtend,
    /// The rotate form.
    Rotate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The left or right direction of a shift or rotate.
pub enum ShiftDirection {
    /// The right form.
    Right,
    /// The left form.
    Left,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The immediate or register source of a shift count.
pub enum ShiftCount {
    /// The immediate form.
    Immediate(u8),
    /// The data register form.
    DataRegister(DataRegister),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Selects the direction of a `MOVEM` transfer.
pub enum MoveMultipleDirection {
    /// The registers to memory form.
    RegistersToMemory,
    /// The memory to registers form.
    MemoryToRegisters,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Selects the direction of a `MOVEP` transfer.
pub enum MovePeripheralDirection {
    /// The memory to register form.
    MemoryToRegister,
    /// The register to memory form.
    RegisterToMemory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Selects the direction of a user-stack-pointer transfer.
pub enum UserStackPointerDirection {
    /// The address register to user stack pointer form.
    AddressRegisterToUserStackPointer,
    /// The user stack pointer to address register form.
    UserStackPointerToAddressRegister,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
/// A sixteen-bit data/address register mask used by `MOVEM`.
pub struct RegisterMask(u16);

impl RegisterMask {
    /// The complete empty set or constant.
    pub const EMPTY: Self = Self(0);

    /// Constructs a register mask from its raw sixteen-bit representation.
    pub const fn from_bits(bits: u16) -> Self {
        Self(bits)
    }

    /// Returns the raw sixteen-bit register mask.
    pub const fn bits(self) -> u16 {
        self.0
    }

    /// Returns whether the mask contains the selected register.
    pub fn contains(self, register: Register) -> bool {
        let bit = match register {
            Register::Data(register) => register.bits(),
            Register::Address(register) => 8 + register.bits(),
        };
        self.0 & (1 << bit) != 0
    }

    /// Adds the selected register to the mask.
    pub fn insert(&mut self, register: Register) {
        let bit = match register {
            Register::Data(register) => register.bits(),
            Register::Address(register) => 8 + register.bits(),
        };
        self.0 |= 1 << bit;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A complete typed instruction admitted by the MC68000 profile.
pub enum Instruction {
    /// The add decimal form.
    AddDecimal {
        /// The operands operand.
        operands: RegisterPair,
    },
    /// The add form.
    Add {
        /// The operand or index width.
        size: OperandSize,
        /// The selected transfer or data direction.
        direction: DataDirection,
        /// The data register operand.
        data_register: DataRegister,
        /// The effective address operand.
        effective_address: EffectiveAddress,
    },
    /// The add address form.
    AddAddress {
        /// The operand or index width.
        size: AddressSize,
        /// The source operand.
        source: EffectiveAddress,
        /// The destination operand.
        destination: AddressRegister,
    },
    /// The add immediate form.
    AddImmediate {
        /// The operand or index width.
        size: OperandSize,
        /// The typed immediate operand.
        immediate: Immediate,
        /// The destination operand.
        destination: EffectiveAddress,
    },
    /// The add quick form.
    AddQuick {
        /// The operand or index width.
        size: OperandSize,
        /// The typed immediate operand.
        immediate: u8,
        /// The destination operand.
        destination: EffectiveAddress,
    },
    /// The add extend form.
    AddExtend {
        /// The operand or index width.
        size: OperandSize,
        /// The operands operand.
        operands: RegisterPair,
    },
    /// The and form.
    And {
        /// The operand or index width.
        size: OperandSize,
        /// The selected transfer or data direction.
        direction: DataDirection,
        /// The data register operand.
        data_register: DataRegister,
        /// The effective address operand.
        effective_address: EffectiveAddress,
    },
    /// The and immediate form.
    AndImmediate {
        /// The operand or index width.
        size: OperandSize,
        /// The typed immediate operand.
        immediate: Immediate,
        /// The destination operand.
        destination: EffectiveAddress,
    },
    /// The and immediate to condition code form.
    AndImmediateToConditionCode(u8),
    /// The and immediate to status form.
    AndImmediateToStatus(u16),
    /// The shift register form.
    ShiftRegister {
        /// The selected arithmetic, bit, shift, or rotate operation.
        operation: ShiftOperation,
        /// The selected transfer or data direction.
        direction: ShiftDirection,
        /// The operand or index width.
        size: OperandSize,
        /// The immediate or register shift count.
        count: ShiftCount,
        /// The destination operand.
        destination: DataRegister,
    },
    /// The shift memory form.
    ShiftMemory {
        /// The selected arithmetic, bit, shift, or rotate operation.
        operation: ShiftOperation,
        /// The selected transfer or data direction.
        direction: ShiftDirection,
        /// The destination operand.
        destination: EffectiveAddress,
    },
    /// The branch form.
    Branch {
        /// The selected branch or operand kind.
        kind: BranchKind,
        /// The encoded signed displacement.
        displacement: BranchDisplacement,
    },
    /// The bit form.
    Bit {
        /// The selected arithmetic, bit, shift, or rotate operation.
        operation: BitOperation,
        /// The bit number operand.
        bit_number: BitNumber,
        /// The destination operand.
        destination: EffectiveAddress,
    },
    /// The check form.
    Check {
        /// The source operand.
        source: EffectiveAddress,
        /// The destination operand.
        destination: DataRegister,
    },
    /// The clear form.
    Clear {
        /// The operand or index width.
        size: OperandSize,
        /// The destination operand.
        destination: EffectiveAddress,
    },
    /// The compare form.
    Compare {
        /// The operand or index width.
        size: OperandSize,
        /// The source operand.
        source: EffectiveAddress,
        /// The destination operand.
        destination: DataRegister,
    },
    /// The compare address form.
    CompareAddress {
        /// The operand or index width.
        size: AddressSize,
        /// The source operand.
        source: EffectiveAddress,
        /// The destination operand.
        destination: AddressRegister,
    },
    /// The compare immediate form.
    CompareImmediate {
        /// The operand or index width.
        size: OperandSize,
        /// The typed immediate operand.
        immediate: Immediate,
        /// The destination operand.
        destination: EffectiveAddress,
    },
    /// The compare memory form.
    CompareMemory {
        /// The operand or index width.
        size: OperandSize,
        /// The source operand.
        source: AddressRegister,
        /// The destination operand.
        destination: AddressRegister,
    },
    /// The decrement branch form.
    DecrementBranch {
        /// The condition-code selector.
        condition: Condition,
        /// The selected register.
        register: DataRegister,
        /// The encoded signed displacement.
        displacement: i16,
    },
    /// The divide form.
    Divide {
        /// Whether signed or unsigned arithmetic is selected.
        sign: DivisionSign,
        /// The source operand.
        source: EffectiveAddress,
        /// The destination operand.
        destination: DataRegister,
    },
    /// The exclusive or form.
    ExclusiveOr {
        /// The operand or index width.
        size: OperandSize,
        /// The source operand.
        source: DataRegister,
        /// The destination operand.
        destination: EffectiveAddress,
    },
    /// The exclusive or immediate form.
    ExclusiveOrImmediate {
        /// The operand or index width.
        size: OperandSize,
        /// The typed immediate operand.
        immediate: Immediate,
        /// The destination operand.
        destination: EffectiveAddress,
    },
    /// The exclusive or immediate to condition code form.
    ExclusiveOrImmediateToConditionCode(u8),
    /// The exclusive or immediate to status form.
    ExclusiveOrImmediateToStatus(u16),
    /// The exchange form.
    Exchange {
        /// The register mask.
        registers: ExchangeRegisters,
    },
    /// The extend form.
    Extend {
        /// The operand or index width.
        size: ExtendSize,
        /// The selected register.
        register: DataRegister,
    },
    /// The illegal form.
    Illegal,
    /// The jump form.
    Jump {
        /// The computed control-flow target.
        target: EffectiveAddress,
    },
    /// The jump subroutine form.
    JumpSubroutine {
        /// The computed control-flow target.
        target: EffectiveAddress,
    },
    /// The load effective address form.
    LoadEffectiveAddress {
        /// The source operand.
        source: EffectiveAddress,
        /// The destination operand.
        destination: AddressRegister,
    },
    /// The link form.
    Link {
        /// The selected register.
        register: AddressRegister,
        /// The encoded signed displacement.
        displacement: i16,
    },
    /// The move form.
    Move {
        /// The operand or index width.
        size: OperandSize,
        /// The source operand.
        source: EffectiveAddress,
        /// The destination operand.
        destination: EffectiveAddress,
    },
    /// The move address form.
    MoveAddress {
        /// The operand or index width.
        size: AddressSize,
        /// The source operand.
        source: EffectiveAddress,
        /// The destination operand.
        destination: AddressRegister,
    },
    /// The move to condition code form.
    MoveToConditionCode {
        /// The source operand.
        source: EffectiveAddress,
    },
    /// The move from status form.
    MoveFromStatus {
        /// The destination operand.
        destination: EffectiveAddress,
    },
    /// The move to status form.
    MoveToStatus {
        /// The source operand.
        source: EffectiveAddress,
    },
    /// The move user stack pointer form.
    MoveUserStackPointer {
        /// The selected transfer or data direction.
        direction: UserStackPointerDirection,
        /// The selected register.
        register: AddressRegister,
    },
    /// The move multiple form.
    MoveMultiple {
        /// The operand or index width.
        size: AddressSize,
        /// The selected transfer or data direction.
        direction: MoveMultipleDirection,
        /// The register mask.
        registers: RegisterMask,
        /// The effective address operand.
        effective_address: EffectiveAddress,
    },
    /// The move peripheral form.
    MovePeripheral {
        /// The operand or index width.
        size: AddressSize,
        /// The selected transfer or data direction.
        direction: MovePeripheralDirection,
        /// The data register operand.
        data_register: DataRegister,
        /// The address register operand.
        address_register: AddressRegister,
        /// The encoded signed displacement.
        displacement: i16,
    },
    /// The move quick form.
    MoveQuick {
        /// The typed immediate operand.
        immediate: i8,
        /// The destination operand.
        destination: DataRegister,
    },
    /// The multiply form.
    Multiply {
        /// Whether signed or unsigned arithmetic is selected.
        sign: MultiplicationSign,
        /// The source operand.
        source: EffectiveAddress,
        /// The destination operand.
        destination: DataRegister,
    },
    /// The negate decimal form.
    NegateDecimal {
        /// The destination operand.
        destination: EffectiveAddress,
    },
    /// The negate form.
    Negate {
        /// The operand or index width.
        size: OperandSize,
        /// The destination operand.
        destination: EffectiveAddress,
    },
    /// The negate extend form.
    NegateExtend {
        /// The operand or index width.
        size: OperandSize,
        /// The destination operand.
        destination: EffectiveAddress,
    },
    /// The nop form.
    Nop,
    /// The not form.
    Not {
        /// The operand or index width.
        size: OperandSize,
        /// The destination operand.
        destination: EffectiveAddress,
    },
    /// The or form.
    Or {
        /// The operand or index width.
        size: OperandSize,
        /// The selected transfer or data direction.
        direction: DataDirection,
        /// The data register operand.
        data_register: DataRegister,
        /// The effective address operand.
        effective_address: EffectiveAddress,
    },
    /// The or immediate form.
    OrImmediate {
        /// The operand or index width.
        size: OperandSize,
        /// The typed immediate operand.
        immediate: Immediate,
        /// The destination operand.
        destination: EffectiveAddress,
    },
    /// The or immediate to condition code form.
    OrImmediateToConditionCode(u8),
    /// The or immediate to status form.
    OrImmediateToStatus(u16),
    /// The push effective address form.
    PushEffectiveAddress {
        /// The source operand.
        source: EffectiveAddress,
    },
    /// The reset form.
    Reset,
    /// The return from exception form.
    ReturnFromException,
    /// The return and restore form.
    ReturnAndRestore,
    /// The return from subroutine form.
    ReturnFromSubroutine,
    /// The subtract decimal form.
    SubtractDecimal {
        /// The operands operand.
        operands: RegisterPair,
    },
    /// The set condition form.
    SetCondition {
        /// The condition-code selector.
        condition: Condition,
        /// The destination operand.
        destination: EffectiveAddress,
    },
    /// The stop form.
    Stop {
        /// The status operand.
        status: u16,
    },
    /// The subtract form.
    Subtract {
        /// The operand or index width.
        size: OperandSize,
        /// The selected transfer or data direction.
        direction: DataDirection,
        /// The data register operand.
        data_register: DataRegister,
        /// The effective address operand.
        effective_address: EffectiveAddress,
    },
    /// The subtract address form.
    SubtractAddress {
        /// The operand or index width.
        size: AddressSize,
        /// The source operand.
        source: EffectiveAddress,
        /// The destination operand.
        destination: AddressRegister,
    },
    /// The subtract immediate form.
    SubtractImmediate {
        /// The operand or index width.
        size: OperandSize,
        /// The typed immediate operand.
        immediate: Immediate,
        /// The destination operand.
        destination: EffectiveAddress,
    },
    /// The subtract quick form.
    SubtractQuick {
        /// The operand or index width.
        size: OperandSize,
        /// The typed immediate operand.
        immediate: u8,
        /// The destination operand.
        destination: EffectiveAddress,
    },
    /// The subtract extend form.
    SubtractExtend {
        /// The operand or index width.
        size: OperandSize,
        /// The operands operand.
        operands: RegisterPair,
    },
    /// The swap form.
    Swap {
        /// The selected register.
        register: DataRegister,
    },
    /// The test and set form.
    TestAndSet {
        /// The destination operand.
        destination: EffectiveAddress,
    },
    /// The trap form.
    Trap {
        /// The trap vector number.
        vector: u8,
    },
    /// The trap on overflow form.
    TrapOnOverflow,
    /// The test form.
    Test {
        /// The operand or index width.
        size: OperandSize,
        /// The source operand.
        source: EffectiveAddress,
    },
    /// The unlink form.
    Unlink {
        /// The selected register.
        register: AddressRegister,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The semantic instruction family used by static analysis.
pub enum Mnemonic {
    /// The abcd form.
    Abcd,
    /// The add form.
    Add,
    /// The adda form.
    Adda,
    /// The addi form.
    Addi,
    /// The addq form.
    Addq,
    /// The addx form.
    Addx,
    /// The and form.
    And,
    /// The andi form.
    Andi,
    /// The asl form.
    Asl,
    /// The asr form.
    Asr,
    /// The bcc form.
    Bcc,
    /// The bchg form.
    Bchg,
    /// The bclr form.
    Bclr,
    /// The bra form.
    Bra,
    /// The bset form.
    Bset,
    /// The bsr form.
    Bsr,
    /// The btst form.
    Btst,
    /// The chk form.
    Chk,
    /// The clr form.
    Clr,
    /// The cmp form.
    Cmp,
    /// The cmpa form.
    Cmpa,
    /// The cmpi form.
    Cmpi,
    /// The cmpm form.
    Cmpm,
    /// The dbcc form.
    Dbcc,
    /// The divs form.
    Divs,
    /// The divu form.
    Divu,
    /// The eor form.
    Eor,
    /// The eori form.
    Eori,
    /// The exg form.
    Exg,
    /// The ext form.
    Ext,
    /// The illegal form.
    Illegal,
    /// The jmp form.
    Jmp,
    /// The jsr form.
    Jsr,
    /// The lea form.
    Lea,
    /// The link form.
    Link,
    /// The lsl form.
    Lsl,
    /// The lsr form.
    Lsr,
    /// The move form.
    Move,
    /// The movea form.
    Movea,
    /// The movem form.
    Movem,
    /// The movep form.
    Movep,
    /// The moveq form.
    Moveq,
    /// The muls form.
    Muls,
    /// The mulu form.
    Mulu,
    /// The nbcd form.
    Nbcd,
    /// The neg form.
    Neg,
    /// The negx form.
    Negx,
    /// The nop form.
    Nop,
    /// The not form.
    Not,
    /// The or form.
    Or,
    /// The ori form.
    Ori,
    /// The pea form.
    Pea,
    /// The reset form.
    Reset,
    /// The rol form.
    Rol,
    /// The ror form.
    Ror,
    /// The roxl form.
    Roxl,
    /// The roxr form.
    Roxr,
    /// The rte form.
    Rte,
    /// The rtr form.
    Rtr,
    /// The rts form.
    Rts,
    /// The sbcd form.
    Sbcd,
    /// The scc form.
    Scc,
    /// The stop form.
    Stop,
    /// The sub form.
    Sub,
    /// The suba form.
    Suba,
    /// The subi form.
    Subi,
    /// The subq form.
    Subq,
    /// The subx form.
    Subx,
    /// The swap form.
    Swap,
    /// The tas form.
    Tas,
    /// The trap form.
    Trap,
    /// The trapv form.
    Trapv,
    /// The tst form.
    Tst,
    /// The unlk form.
    Unlk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// An operand-independent MC68000 form with size and operand classifications.
pub struct InstructionForm {
    /// The semantic instruction family.
    pub mnemonic: Mnemonic,
    /// The operand or index width.
    pub size: Option<OperandSize>,
    /// The source operand.
    pub source: Option<OperandKind>,
    /// The destination operand.
    pub destination: Option<OperandKind>,
}

impl InstructionForm {
    /// Returns the complete set of supported instruction forms.
    pub fn all() -> Vec<Self> {
        let mut forms = BTreeSet::new();
        for word in u16::MIN..=u16::MAX {
            if let crate::DecodedWord::Instruction(form) = classify_word(word) {
                forms.insert(form);
            }
        }
        forms.into_iter().collect()
    }
}

impl Instruction {
    /// Returns the operand-independent instruction form.
    pub fn form(&self) -> InstructionForm {
        let form = |mnemonic, size, source, destination| InstructionForm {
            mnemonic,
            size,
            source,
            destination,
        };
        let ea = |address: EffectiveAddress| Some(address.operand_kind());
        let pair = |operands: RegisterPair| {
            (
                Some(operands.source_operand_kind()),
                Some(operands.destination_operand_kind()),
            )
        };

        match *self {
            Self::AddDecimal { operands } => {
                let (source, destination) = pair(operands);
                form(Mnemonic::Abcd, Some(OperandSize::Byte), source, destination)
            }
            Self::SubtractDecimal { operands } => {
                let (source, destination) = pair(operands);
                form(Mnemonic::Sbcd, Some(OperandSize::Byte), source, destination)
            }
            Self::Add {
                size,
                direction,
                effective_address,
                ..
            }
            | Self::Subtract {
                size,
                direction,
                effective_address,
                ..
            }
            | Self::And {
                size,
                direction,
                effective_address,
                ..
            }
            | Self::Or {
                size,
                direction,
                effective_address,
                ..
            } => {
                let mnemonic = match self {
                    Self::Add { .. } => Mnemonic::Add,
                    Self::Subtract { .. } => Mnemonic::Sub,
                    Self::And { .. } => Mnemonic::And,
                    Self::Or { .. } => Mnemonic::Or,
                    _ => unreachable!(),
                };
                let (source, destination) = match direction {
                    DataDirection::ToDataRegister => {
                        (ea(effective_address), Some(OperandKind::DataRegister))
                    }
                    DataDirection::ToEffectiveAddress => {
                        (Some(OperandKind::DataRegister), ea(effective_address))
                    }
                };
                form(mnemonic, Some(size), source, destination)
            }
            Self::AddAddress { size, source, .. }
            | Self::SubtractAddress { size, source, .. }
            | Self::CompareAddress { size, source, .. } => form(
                match self {
                    Self::AddAddress { .. } => Mnemonic::Adda,
                    Self::SubtractAddress { .. } => Mnemonic::Suba,
                    Self::CompareAddress { .. } => Mnemonic::Cmpa,
                    _ => unreachable!(),
                },
                Some(size.into()),
                ea(source),
                Some(OperandKind::AddressRegister),
            ),
            Self::AddImmediate {
                size, destination, ..
            }
            | Self::SubtractImmediate {
                size, destination, ..
            }
            | Self::AndImmediate {
                size, destination, ..
            }
            | Self::OrImmediate {
                size, destination, ..
            }
            | Self::ExclusiveOrImmediate {
                size, destination, ..
            }
            | Self::CompareImmediate {
                size, destination, ..
            } => form(
                match self {
                    Self::AddImmediate { .. } => Mnemonic::Addi,
                    Self::SubtractImmediate { .. } => Mnemonic::Subi,
                    Self::AndImmediate { .. } => Mnemonic::Andi,
                    Self::OrImmediate { .. } => Mnemonic::Ori,
                    Self::ExclusiveOrImmediate { .. } => Mnemonic::Eori,
                    Self::CompareImmediate { .. } => Mnemonic::Cmpi,
                    _ => unreachable!(),
                },
                Some(size),
                Some(OperandKind::Immediate),
                ea(destination),
            ),
            Self::AddQuick {
                size, destination, ..
            }
            | Self::SubtractQuick {
                size, destination, ..
            } => form(
                if matches!(self, Self::AddQuick { .. }) {
                    Mnemonic::Addq
                } else {
                    Mnemonic::Subq
                },
                Some(size),
                Some(OperandKind::QuickImmediate),
                ea(destination),
            ),
            Self::AddExtend { size, operands } | Self::SubtractExtend { size, operands } => {
                let (source, destination) = pair(operands);
                form(
                    if matches!(self, Self::AddExtend { .. }) {
                        Mnemonic::Addx
                    } else {
                        Mnemonic::Subx
                    },
                    Some(size),
                    source,
                    destination,
                )
            }
            Self::AndImmediateToConditionCode(_)
            | Self::OrImmediateToConditionCode(_)
            | Self::ExclusiveOrImmediateToConditionCode(_) => form(
                match self {
                    Self::AndImmediateToConditionCode(_) => Mnemonic::Andi,
                    Self::OrImmediateToConditionCode(_) => Mnemonic::Ori,
                    Self::ExclusiveOrImmediateToConditionCode(_) => Mnemonic::Eori,
                    _ => unreachable!(),
                },
                Some(OperandSize::Byte),
                Some(OperandKind::Immediate),
                Some(OperandKind::ConditionCodeRegister),
            ),
            Self::AndImmediateToStatus(_)
            | Self::OrImmediateToStatus(_)
            | Self::ExclusiveOrImmediateToStatus(_) => form(
                match self {
                    Self::AndImmediateToStatus(_) => Mnemonic::Andi,
                    Self::OrImmediateToStatus(_) => Mnemonic::Ori,
                    Self::ExclusiveOrImmediateToStatus(_) => Mnemonic::Eori,
                    _ => unreachable!(),
                },
                Some(OperandSize::Word),
                Some(OperandKind::Immediate),
                Some(OperandKind::StatusRegister),
            ),
            Self::ShiftRegister {
                operation,
                direction,
                size,
                count,
                ..
            } => form(
                shift_mnemonic(operation, direction),
                Some(size),
                Some(match count {
                    ShiftCount::Immediate(_) => OperandKind::QuickImmediate,
                    ShiftCount::DataRegister(_) => OperandKind::DataRegister,
                }),
                Some(OperandKind::DataRegister),
            ),
            Self::ShiftMemory {
                operation,
                direction,
                destination,
            } => form(
                shift_mnemonic(operation, direction),
                Some(OperandSize::Word),
                None,
                ea(destination),
            ),
            Self::Branch { kind, displacement } => form(
                match kind {
                    BranchKind::Always => Mnemonic::Bra,
                    BranchKind::Subroutine => Mnemonic::Bsr,
                    BranchKind::Conditional(_) => Mnemonic::Bcc,
                },
                Some(match displacement {
                    BranchDisplacement::Byte(_) => OperandSize::Byte,
                    BranchDisplacement::Word(_) => OperandSize::Word,
                }),
                Some(OperandKind::Immediate),
                None,
            ),
            Self::Bit {
                operation,
                bit_number,
                destination,
            } => form(
                bit_mnemonic(operation),
                Some(
                    if matches!(destination, EffectiveAddress::DataRegister(_)) {
                        OperandSize::Long
                    } else {
                        OperandSize::Byte
                    },
                ),
                Some(match bit_number {
                    BitNumber::DataRegister(_) => OperandKind::DataRegister,
                    BitNumber::Immediate(_) => OperandKind::Immediate,
                }),
                ea(destination),
            ),
            Self::Check { source, .. } => form(
                Mnemonic::Chk,
                Some(OperandSize::Word),
                ea(source),
                Some(OperandKind::DataRegister),
            ),
            Self::Clear { size, destination } => {
                form(Mnemonic::Clr, Some(size), None, ea(destination))
            }
            Self::Compare { size, source, .. } => form(
                Mnemonic::Cmp,
                Some(size),
                ea(source),
                Some(OperandKind::DataRegister),
            ),
            Self::CompareMemory { size, .. } => form(
                Mnemonic::Cmpm,
                Some(size),
                Some(OperandKind::AddressPostincrement),
                Some(OperandKind::AddressPostincrement),
            ),
            Self::DecrementBranch { .. } => form(
                Mnemonic::Dbcc,
                Some(OperandSize::Word),
                Some(OperandKind::DataRegister),
                Some(OperandKind::Immediate),
            ),
            Self::Divide { sign, source, .. } => form(
                match sign {
                    DivisionSign::Unsigned => Mnemonic::Divu,
                    DivisionSign::Signed => Mnemonic::Divs,
                },
                Some(OperandSize::Word),
                ea(source),
                Some(OperandKind::DataRegister),
            ),
            Self::ExclusiveOr {
                size, destination, ..
            } => form(
                Mnemonic::Eor,
                Some(size),
                Some(OperandKind::DataRegister),
                ea(destination),
            ),
            Self::Exchange { registers } => form(
                Mnemonic::Exg,
                Some(OperandSize::Long),
                Some(match registers {
                    ExchangeRegisters::Data { .. } | ExchangeRegisters::DataAddress { .. } => {
                        OperandKind::DataRegister
                    }
                    ExchangeRegisters::Address { .. } => OperandKind::AddressRegister,
                }),
                Some(match registers {
                    ExchangeRegisters::Data { .. } => OperandKind::DataRegister,
                    ExchangeRegisters::Address { .. } | ExchangeRegisters::DataAddress { .. } => {
                        OperandKind::AddressRegister
                    }
                }),
            ),
            Self::Extend { size, .. } => form(
                Mnemonic::Ext,
                Some(match size {
                    ExtendSize::ByteToWord => OperandSize::Word,
                    ExtendSize::WordToLong => OperandSize::Long,
                }),
                Some(OperandKind::DataRegister),
                Some(OperandKind::DataRegister),
            ),
            Self::Illegal => form(Mnemonic::Illegal, None, None, None),
            Self::Jump { target } | Self::JumpSubroutine { target } => form(
                if matches!(self, Self::Jump { .. }) {
                    Mnemonic::Jmp
                } else {
                    Mnemonic::Jsr
                },
                None,
                ea(target),
                None,
            ),
            Self::LoadEffectiveAddress { source, .. } => form(
                Mnemonic::Lea,
                Some(OperandSize::Long),
                ea(source),
                Some(OperandKind::AddressRegister),
            ),
            Self::Link { .. } => form(
                Mnemonic::Link,
                Some(OperandSize::Word),
                Some(OperandKind::AddressRegister),
                Some(OperandKind::Immediate),
            ),
            Self::Move {
                size,
                source,
                destination,
            } => form(Mnemonic::Move, Some(size), ea(source), ea(destination)),
            Self::MoveAddress { size, source, .. } => form(
                Mnemonic::Movea,
                Some(size.into()),
                ea(source),
                Some(OperandKind::AddressRegister),
            ),
            Self::MoveToConditionCode { source } => form(
                Mnemonic::Move,
                Some(OperandSize::Word),
                ea(source),
                Some(OperandKind::ConditionCodeRegister),
            ),
            Self::MoveFromStatus { destination } => form(
                Mnemonic::Move,
                Some(OperandSize::Word),
                Some(OperandKind::StatusRegister),
                ea(destination),
            ),
            Self::MoveToStatus { source } => form(
                Mnemonic::Move,
                Some(OperandSize::Word),
                ea(source),
                Some(OperandKind::StatusRegister),
            ),
            Self::MoveUserStackPointer { direction, .. } => {
                let (source, destination) = match direction {
                    UserStackPointerDirection::AddressRegisterToUserStackPointer => (
                        Some(OperandKind::AddressRegister),
                        Some(OperandKind::UserStackPointer),
                    ),
                    UserStackPointerDirection::UserStackPointerToAddressRegister => (
                        Some(OperandKind::UserStackPointer),
                        Some(OperandKind::AddressRegister),
                    ),
                };
                form(Mnemonic::Move, Some(OperandSize::Long), source, destination)
            }
            Self::MoveMultiple {
                size,
                direction,
                effective_address,
                ..
            } => {
                let (source, destination) = match direction {
                    MoveMultipleDirection::RegistersToMemory => {
                        (Some(OperandKind::RegisterList), ea(effective_address))
                    }
                    MoveMultipleDirection::MemoryToRegisters => {
                        (ea(effective_address), Some(OperandKind::RegisterList))
                    }
                };
                form(Mnemonic::Movem, Some(size.into()), source, destination)
            }
            Self::MovePeripheral {
                size, direction, ..
            } => {
                let (source, destination) = match direction {
                    MovePeripheralDirection::MemoryToRegister => (
                        Some(OperandKind::AddressDisplacement),
                        Some(OperandKind::DataRegister),
                    ),
                    MovePeripheralDirection::RegisterToMemory => (
                        Some(OperandKind::DataRegister),
                        Some(OperandKind::AddressDisplacement),
                    ),
                };
                form(Mnemonic::Movep, Some(size.into()), source, destination)
            }
            Self::MoveQuick { .. } => form(
                Mnemonic::Moveq,
                Some(OperandSize::Long),
                Some(OperandKind::QuickImmediate),
                Some(OperandKind::DataRegister),
            ),
            Self::Multiply { sign, source, .. } => form(
                match sign {
                    MultiplicationSign::Unsigned => Mnemonic::Mulu,
                    MultiplicationSign::Signed => Mnemonic::Muls,
                },
                Some(OperandSize::Word),
                ea(source),
                Some(OperandKind::DataRegister),
            ),
            Self::NegateDecimal { destination } => form(
                Mnemonic::Nbcd,
                Some(OperandSize::Byte),
                None,
                ea(destination),
            ),
            Self::Negate { size, destination }
            | Self::NegateExtend { size, destination }
            | Self::Not { size, destination } => form(
                match self {
                    Self::Negate { .. } => Mnemonic::Neg,
                    Self::NegateExtend { .. } => Mnemonic::Negx,
                    Self::Not { .. } => Mnemonic::Not,
                    _ => unreachable!(),
                },
                Some(size),
                None,
                ea(destination),
            ),
            Self::Nop => form(Mnemonic::Nop, None, None, None),
            Self::PushEffectiveAddress { source } => {
                form(Mnemonic::Pea, Some(OperandSize::Long), ea(source), None)
            }
            Self::Reset => form(Mnemonic::Reset, None, None, None),
            Self::ReturnFromException => form(Mnemonic::Rte, None, None, None),
            Self::ReturnAndRestore => form(Mnemonic::Rtr, None, None, None),
            Self::ReturnFromSubroutine => form(Mnemonic::Rts, None, None, None),
            Self::SetCondition { destination, .. } => form(
                Mnemonic::Scc,
                Some(OperandSize::Byte),
                None,
                ea(destination),
            ),
            Self::Stop { .. } => form(
                Mnemonic::Stop,
                Some(OperandSize::Word),
                Some(OperandKind::Immediate),
                Some(OperandKind::StatusRegister),
            ),
            Self::Swap { .. } => form(
                Mnemonic::Swap,
                Some(OperandSize::Word),
                Some(OperandKind::DataRegister),
                Some(OperandKind::DataRegister),
            ),
            Self::TestAndSet { destination } => form(
                Mnemonic::Tas,
                Some(OperandSize::Byte),
                None,
                ea(destination),
            ),
            Self::Trap { .. } => form(
                Mnemonic::Trap,
                None,
                Some(OperandKind::QuickImmediate),
                None,
            ),
            Self::TrapOnOverflow => form(Mnemonic::Trapv, None, None, None),
            Self::Test { size, source } => form(Mnemonic::Tst, Some(size), ea(source), None),
            Self::Unlink { .. } => form(
                Mnemonic::Unlk,
                None,
                Some(OperandKind::AddressRegister),
                None,
            ),
        }
    }
}

fn shift_mnemonic(operation: ShiftOperation, direction: ShiftDirection) -> Mnemonic {
    match (operation, direction) {
        (ShiftOperation::Arithmetic, ShiftDirection::Left) => Mnemonic::Asl,
        (ShiftOperation::Arithmetic, ShiftDirection::Right) => Mnemonic::Asr,
        (ShiftOperation::Logical, ShiftDirection::Left) => Mnemonic::Lsl,
        (ShiftOperation::Logical, ShiftDirection::Right) => Mnemonic::Lsr,
        (ShiftOperation::RotateWithExtend, ShiftDirection::Left) => Mnemonic::Roxl,
        (ShiftOperation::RotateWithExtend, ShiftDirection::Right) => Mnemonic::Roxr,
        (ShiftOperation::Rotate, ShiftDirection::Left) => Mnemonic::Rol,
        (ShiftOperation::Rotate, ShiftDirection::Right) => Mnemonic::Ror,
    }
}

fn bit_mnemonic(operation: BitOperation) -> Mnemonic {
    match operation {
        BitOperation::Test => Mnemonic::Btst,
        BitOperation::Change => Mnemonic::Bchg,
        BitOperation::Clear => Mnemonic::Bclr,
        BitOperation::Set => Mnemonic::Bset,
    }
}
