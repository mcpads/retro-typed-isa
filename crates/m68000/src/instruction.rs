use std::collections::BTreeSet;

use crate::classify_word;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum DataRegister {
    D0,
    D1,
    D2,
    D3,
    D4,
    D5,
    D6,
    D7,
}

impl DataRegister {
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
pub enum AddressRegister {
    A0,
    A1,
    A2,
    A3,
    A4,
    A5,
    A6,
    A7,
}

impl AddressRegister {
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
pub enum Register {
    Data(DataRegister),
    Address(AddressRegister),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OperandSize {
    Byte,
    Word,
    Long,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AddressSize {
    Word,
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
pub enum Immediate {
    Byte(u8),
    Word(u16),
    Long(u32),
}

impl Immediate {
    pub const fn size(self) -> OperandSize {
        match self {
            Self::Byte(_) => OperandSize::Byte,
            Self::Word(_) => OperandSize::Word,
            Self::Long(_) => OperandSize::Long,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IndexSize {
    Word,
    Long,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndexRegister {
    pub register: Register,
    pub size: IndexSize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EffectiveAddress {
    DataRegister(DataRegister),
    AddressRegister(AddressRegister),
    AddressIndirect(AddressRegister),
    AddressPostincrement(AddressRegister),
    AddressPredecrement(AddressRegister),
    AddressDisplacement {
        register: AddressRegister,
        displacement: i16,
    },
    AddressIndex {
        register: AddressRegister,
        index: IndexRegister,
        displacement: i8,
    },
    AbsoluteWord(u16),
    AbsoluteLong(u32),
    ProgramCounterDisplacement(i16),
    ProgramCounterIndex {
        index: IndexRegister,
        displacement: i8,
    },
    Immediate(Immediate),
}

impl EffectiveAddress {
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
    DataRegisterDirect,
    AddressRegisterDirect,
    AddressRegisterIndirect,
    AddressRegisterIndirectPostincrement,
    AddressRegisterIndirectPredecrement,
    AddressRegisterIndirectDisplacement,
    AddressRegisterIndirectIndex,
    AbsoluteShort,
    AbsoluteLong,
    ProgramCounterRelativeDisplacement,
    ProgramCounterRelativeIndex,
    Immediate,
    QuickImmediate,
    Implied,
}

impl AddressingMode {
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
    DataRegister,
    AddressRegister,
    AddressIndirect,
    AddressPostincrement,
    AddressPredecrement,
    AddressDisplacement,
    AddressIndex,
    AbsoluteWord,
    AbsoluteLong,
    ProgramCounterDisplacement,
    ProgramCounterIndex,
    Immediate,
    QuickImmediate,
    RegisterList,
    UserStackPointer,
    ConditionCodeRegister,
    StatusRegister,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DataDirection {
    ToDataRegister,
    ToEffectiveAddress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RegisterPair {
    Data {
        source: DataRegister,
        destination: DataRegister,
    },
    AddressPredecrement {
        source: AddressRegister,
        destination: AddressRegister,
    },
}

impl RegisterPair {
    pub const fn source_operand_kind(self) -> OperandKind {
        match self {
            Self::Data { .. } => OperandKind::DataRegister,
            Self::AddressPredecrement { .. } => OperandKind::AddressPredecrement,
        }
    }

    pub const fn destination_operand_kind(self) -> OperandKind {
        self.source_operand_kind()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Condition {
    True,
    False,
    High,
    LowOrSame,
    CarryClear,
    CarrySet,
    NotEqual,
    Equal,
    OverflowClear,
    OverflowSet,
    Plus,
    Minus,
    GreaterOrEqual,
    LessThan,
    GreaterThan,
    LessOrEqual,
}

impl Condition {
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
pub enum BranchKind {
    Always,
    Subroutine,
    Conditional(Condition),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BranchDisplacement {
    Byte(i8),
    Word(i16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BitOperation {
    Test,
    Change,
    Clear,
    Set,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BitNumber {
    DataRegister(DataRegister),
    Immediate(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DivisionSign {
    Unsigned,
    Signed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MultiplicationSign {
    Unsigned,
    Signed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExchangeRegisters {
    Data {
        source: DataRegister,
        destination: DataRegister,
    },
    Address {
        source: AddressRegister,
        destination: AddressRegister,
    },
    DataAddress {
        data: DataRegister,
        address: AddressRegister,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExtendSize {
    ByteToWord,
    WordToLong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShiftOperation {
    Arithmetic,
    Logical,
    RotateWithExtend,
    Rotate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShiftDirection {
    Right,
    Left,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShiftCount {
    Immediate(u8),
    DataRegister(DataRegister),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MoveMultipleDirection {
    RegistersToMemory,
    MemoryToRegisters,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MovePeripheralDirection {
    MemoryToRegister,
    RegisterToMemory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UserStackPointerDirection {
    AddressRegisterToUserStackPointer,
    UserStackPointerToAddressRegister,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct RegisterMask(u16);

impl RegisterMask {
    pub const EMPTY: Self = Self(0);

    pub const fn from_bits(bits: u16) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u16 {
        self.0
    }

    pub fn contains(self, register: Register) -> bool {
        let bit = match register {
            Register::Data(register) => register.bits(),
            Register::Address(register) => 8 + register.bits(),
        };
        self.0 & (1 << bit) != 0
    }

    pub fn insert(&mut self, register: Register) {
        let bit = match register {
            Register::Data(register) => register.bits(),
            Register::Address(register) => 8 + register.bits(),
        };
        self.0 |= 1 << bit;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    AddDecimal {
        operands: RegisterPair,
    },
    Add {
        size: OperandSize,
        direction: DataDirection,
        data_register: DataRegister,
        effective_address: EffectiveAddress,
    },
    AddAddress {
        size: AddressSize,
        source: EffectiveAddress,
        destination: AddressRegister,
    },
    AddImmediate {
        size: OperandSize,
        immediate: Immediate,
        destination: EffectiveAddress,
    },
    AddQuick {
        size: OperandSize,
        immediate: u8,
        destination: EffectiveAddress,
    },
    AddExtend {
        size: OperandSize,
        operands: RegisterPair,
    },
    And {
        size: OperandSize,
        direction: DataDirection,
        data_register: DataRegister,
        effective_address: EffectiveAddress,
    },
    AndImmediate {
        size: OperandSize,
        immediate: Immediate,
        destination: EffectiveAddress,
    },
    AndImmediateToConditionCode(u8),
    AndImmediateToStatus(u16),
    ShiftRegister {
        operation: ShiftOperation,
        direction: ShiftDirection,
        size: OperandSize,
        count: ShiftCount,
        destination: DataRegister,
    },
    ShiftMemory {
        operation: ShiftOperation,
        direction: ShiftDirection,
        destination: EffectiveAddress,
    },
    Branch {
        kind: BranchKind,
        displacement: BranchDisplacement,
    },
    Bit {
        operation: BitOperation,
        bit_number: BitNumber,
        destination: EffectiveAddress,
    },
    Check {
        source: EffectiveAddress,
        destination: DataRegister,
    },
    Clear {
        size: OperandSize,
        destination: EffectiveAddress,
    },
    Compare {
        size: OperandSize,
        source: EffectiveAddress,
        destination: DataRegister,
    },
    CompareAddress {
        size: AddressSize,
        source: EffectiveAddress,
        destination: AddressRegister,
    },
    CompareImmediate {
        size: OperandSize,
        immediate: Immediate,
        destination: EffectiveAddress,
    },
    CompareMemory {
        size: OperandSize,
        source: AddressRegister,
        destination: AddressRegister,
    },
    DecrementBranch {
        condition: Condition,
        register: DataRegister,
        displacement: i16,
    },
    Divide {
        sign: DivisionSign,
        source: EffectiveAddress,
        destination: DataRegister,
    },
    ExclusiveOr {
        size: OperandSize,
        source: DataRegister,
        destination: EffectiveAddress,
    },
    ExclusiveOrImmediate {
        size: OperandSize,
        immediate: Immediate,
        destination: EffectiveAddress,
    },
    ExclusiveOrImmediateToConditionCode(u8),
    ExclusiveOrImmediateToStatus(u16),
    Exchange {
        registers: ExchangeRegisters,
    },
    Extend {
        size: ExtendSize,
        register: DataRegister,
    },
    Illegal,
    Jump {
        target: EffectiveAddress,
    },
    JumpSubroutine {
        target: EffectiveAddress,
    },
    LoadEffectiveAddress {
        source: EffectiveAddress,
        destination: AddressRegister,
    },
    Link {
        register: AddressRegister,
        displacement: i16,
    },
    Move {
        size: OperandSize,
        source: EffectiveAddress,
        destination: EffectiveAddress,
    },
    MoveAddress {
        size: AddressSize,
        source: EffectiveAddress,
        destination: AddressRegister,
    },
    MoveToConditionCode {
        source: EffectiveAddress,
    },
    MoveFromStatus {
        destination: EffectiveAddress,
    },
    MoveToStatus {
        source: EffectiveAddress,
    },
    MoveUserStackPointer {
        direction: UserStackPointerDirection,
        register: AddressRegister,
    },
    MoveMultiple {
        size: AddressSize,
        direction: MoveMultipleDirection,
        registers: RegisterMask,
        effective_address: EffectiveAddress,
    },
    MovePeripheral {
        size: AddressSize,
        direction: MovePeripheralDirection,
        data_register: DataRegister,
        address_register: AddressRegister,
        displacement: i16,
    },
    MoveQuick {
        immediate: i8,
        destination: DataRegister,
    },
    Multiply {
        sign: MultiplicationSign,
        source: EffectiveAddress,
        destination: DataRegister,
    },
    NegateDecimal {
        destination: EffectiveAddress,
    },
    Negate {
        size: OperandSize,
        destination: EffectiveAddress,
    },
    NegateExtend {
        size: OperandSize,
        destination: EffectiveAddress,
    },
    Nop,
    Not {
        size: OperandSize,
        destination: EffectiveAddress,
    },
    Or {
        size: OperandSize,
        direction: DataDirection,
        data_register: DataRegister,
        effective_address: EffectiveAddress,
    },
    OrImmediate {
        size: OperandSize,
        immediate: Immediate,
        destination: EffectiveAddress,
    },
    OrImmediateToConditionCode(u8),
    OrImmediateToStatus(u16),
    PushEffectiveAddress {
        source: EffectiveAddress,
    },
    Reset,
    ReturnFromException,
    ReturnAndRestore,
    ReturnFromSubroutine,
    SubtractDecimal {
        operands: RegisterPair,
    },
    SetCondition {
        condition: Condition,
        destination: EffectiveAddress,
    },
    Stop {
        status: u16,
    },
    Subtract {
        size: OperandSize,
        direction: DataDirection,
        data_register: DataRegister,
        effective_address: EffectiveAddress,
    },
    SubtractAddress {
        size: AddressSize,
        source: EffectiveAddress,
        destination: AddressRegister,
    },
    SubtractImmediate {
        size: OperandSize,
        immediate: Immediate,
        destination: EffectiveAddress,
    },
    SubtractQuick {
        size: OperandSize,
        immediate: u8,
        destination: EffectiveAddress,
    },
    SubtractExtend {
        size: OperandSize,
        operands: RegisterPair,
    },
    Swap {
        register: DataRegister,
    },
    TestAndSet {
        destination: EffectiveAddress,
    },
    Trap {
        vector: u8,
    },
    TrapOnOverflow,
    Test {
        size: OperandSize,
        source: EffectiveAddress,
    },
    Unlink {
        register: AddressRegister,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Mnemonic {
    Abcd,
    Add,
    Adda,
    Addi,
    Addq,
    Addx,
    And,
    Andi,
    Asl,
    Asr,
    Bcc,
    Bchg,
    Bclr,
    Bra,
    Bset,
    Bsr,
    Btst,
    Chk,
    Clr,
    Cmp,
    Cmpa,
    Cmpi,
    Cmpm,
    Dbcc,
    Divs,
    Divu,
    Eor,
    Eori,
    Exg,
    Ext,
    Illegal,
    Jmp,
    Jsr,
    Lea,
    Link,
    Lsl,
    Lsr,
    Move,
    Movea,
    Movem,
    Movep,
    Moveq,
    Muls,
    Mulu,
    Nbcd,
    Neg,
    Negx,
    Nop,
    Not,
    Or,
    Ori,
    Pea,
    Reset,
    Rol,
    Ror,
    Roxl,
    Roxr,
    Rte,
    Rtr,
    Rts,
    Sbcd,
    Scc,
    Stop,
    Sub,
    Suba,
    Subi,
    Subq,
    Subx,
    Swap,
    Tas,
    Trap,
    Trapv,
    Tst,
    Unlk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstructionForm {
    pub mnemonic: Mnemonic,
    pub size: Option<OperandSize>,
    pub source: Option<OperandKind>,
    pub destination: Option<OperandKind>,
}

impl InstructionForm {
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

    pub fn control_flow(&self, address: u32) -> Result<ControlFlow, crate::EncodeError> {
        let byte_len = u32::try_from(crate::encode_bytes(self)?.len())
            .expect("an MC68000 instruction length fits in u32");
        let next = address.wrapping_add(byte_len);
        let branch_base = address.wrapping_add(2);
        Ok(match *self {
            Self::Branch { kind, displacement } => {
                let displacement = match displacement {
                    BranchDisplacement::Byte(value) => i32::from(value),
                    BranchDisplacement::Word(value) => i32::from(value),
                };
                let target = add_signed(branch_base, displacement);
                match kind {
                    BranchKind::Always => ControlFlow::Branch {
                        target,
                        fallthrough: None,
                    },
                    BranchKind::Subroutine => ControlFlow::Call {
                        target: Some(target),
                        return_address: next,
                    },
                    BranchKind::Conditional(_) => ControlFlow::Branch {
                        target,
                        fallthrough: Some(next),
                    },
                }
            }
            Self::DecrementBranch { displacement, .. } => ControlFlow::Branch {
                target: add_signed(branch_base, i32::from(displacement)),
                fallthrough: Some(next),
            },
            Self::Jump { target } => ControlFlow::Jump {
                target: direct_target(target, branch_base),
            },
            Self::JumpSubroutine { target } => ControlFlow::Call {
                target: direct_target(target, branch_base),
                return_address: next,
            },
            Self::ReturnFromException => ControlFlow::ExceptionReturn,
            Self::ReturnAndRestore | Self::ReturnFromSubroutine => ControlFlow::Return,
            Self::Illegal | Self::Trap { .. } => ControlFlow::Interrupt,
            Self::TrapOnOverflow => ControlFlow::ConditionalInterrupt { fallthrough: next },
            Self::Stop { .. } => ControlFlow::Stop,
            _ => ControlFlow::FallThrough { next },
        })
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

fn add_signed(base: u32, displacement: i32) -> u32 {
    base.wrapping_add(displacement as u32)
}

fn direct_target(target: EffectiveAddress, pc_base: u32) -> Option<u32> {
    match target {
        EffectiveAddress::AbsoluteWord(address) => Some(i32::from(address as i16) as u32),
        EffectiveAddress::AbsoluteLong(address) => Some(address),
        EffectiveAddress::ProgramCounterDisplacement(displacement) => {
            Some(add_signed(pc_base, i32::from(displacement)))
        }
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlFlow {
    FallThrough {
        next: u32,
    },
    Branch {
        target: u32,
        fallthrough: Option<u32>,
    },
    Call {
        target: Option<u32>,
        return_address: u32,
    },
    Jump {
        target: Option<u32>,
    },
    Return,
    ExceptionReturn,
    Interrupt,
    ConditionalInterrupt {
        fallthrough: u32,
    },
    Stop,
}
