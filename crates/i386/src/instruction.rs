#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CodeMode {
    Code16,
    Code32,
}

impl CodeMode {
    pub const fn default_operand_size(self) -> OperandSize {
        match self {
            Self::Code16 => OperandSize::Word,
            Self::Code32 => OperandSize::Doubleword,
        }
    }

    pub const fn default_address_size(self) -> AddressSize {
        match self {
            Self::Code16 => AddressSize::Bits16,
            Self::Code32 => AddressSize::Bits32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperandSize {
    Byte,
    Word,
    Doubleword,
}

impl OperandSize {
    pub const fn byte_len(self) -> usize {
        match self {
            Self::Byte => 1,
            Self::Word => 2,
            Self::Doubleword => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AddressSize {
    Bits16,
    Bits32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Register8 {
    Al,
    Cl,
    Dl,
    Bl,
    Ah,
    Ch,
    Dh,
    Bh,
}

impl Register8 {
    pub const fn from_index(index: u8) -> Option<Self> {
        Some(match index {
            0 => Self::Al,
            1 => Self::Cl,
            2 => Self::Dl,
            3 => Self::Bl,
            4 => Self::Ah,
            5 => Self::Ch,
            6 => Self::Dh,
            7 => Self::Bh,
            _ => return None,
        })
    }

    pub const fn index(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Register16 {
    Ax,
    Cx,
    Dx,
    Bx,
    Sp,
    Bp,
    Si,
    Di,
}

impl Register16 {
    pub const fn from_index(index: u8) -> Option<Self> {
        Some(match index {
            0 => Self::Ax,
            1 => Self::Cx,
            2 => Self::Dx,
            3 => Self::Bx,
            4 => Self::Sp,
            5 => Self::Bp,
            6 => Self::Si,
            7 => Self::Di,
            _ => return None,
        })
    }

    pub const fn index(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Register32 {
    Eax,
    Ecx,
    Edx,
    Ebx,
    Esp,
    Ebp,
    Esi,
    Edi,
}

impl Register32 {
    pub const fn from_index(index: u8) -> Option<Self> {
        Some(match index {
            0 => Self::Eax,
            1 => Self::Ecx,
            2 => Self::Edx,
            3 => Self::Ebx,
            4 => Self::Esp,
            5 => Self::Ebp,
            6 => Self::Esi,
            7 => Self::Edi,
            _ => return None,
        })
    }

    pub const fn index(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GeneralRegister {
    Byte(Register8),
    Word(Register16),
    Doubleword(Register32),
}

impl GeneralRegister {
    pub const fn size(self) -> OperandSize {
        match self {
            Self::Byte(_) => OperandSize::Byte,
            Self::Word(_) => OperandSize::Word,
            Self::Doubleword(_) => OperandSize::Doubleword,
        }
    }

    pub const fn index(self) -> u8 {
        match self {
            Self::Byte(register) => register.index(),
            Self::Word(register) => register.index(),
            Self::Doubleword(register) => register.index(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SegmentRegister {
    Es,
    Cs,
    Ss,
    Ds,
    Fs,
    Gs,
}

impl SegmentRegister {
    pub const fn from_modrm_index(index: u8) -> Option<Self> {
        Some(match index {
            0 => Self::Es,
            1 => Self::Cs,
            2 => Self::Ss,
            3 => Self::Ds,
            4 => Self::Fs,
            5 => Self::Gs,
            _ => return None,
        })
    }

    pub const fn modrm_index(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlRegister {
    Cr0,
    Cr2,
    Cr3,
}

impl ControlRegister {
    pub const fn from_index(index: u8) -> Option<Self> {
        match index {
            0 => Some(Self::Cr0),
            2 => Some(Self::Cr2),
            3 => Some(Self::Cr3),
            _ => None,
        }
    }

    pub const fn index(self) -> u8 {
        match self {
            Self::Cr0 => 0,
            Self::Cr2 => 2,
            Self::Cr3 => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DebugRegister {
    Dr0,
    Dr1,
    Dr2,
    Dr3,
    Dr6,
    Dr7,
}

impl DebugRegister {
    pub const fn from_index(index: u8) -> Option<Self> {
        Some(match index {
            0 => Self::Dr0,
            1 => Self::Dr1,
            2 => Self::Dr2,
            3 => Self::Dr3,
            6 => Self::Dr6,
            7 => Self::Dr7,
            _ => return None,
        })
    }

    pub const fn index(self) -> u8 {
        match self {
            Self::Dr0 => 0,
            Self::Dr1 => 1,
            Self::Dr2 => 2,
            Self::Dr3 => 3,
            Self::Dr6 => 6,
            Self::Dr7 => 7,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TestRegister {
    Tr6,
    Tr7,
}

impl TestRegister {
    pub const fn from_index(index: u8) -> Option<Self> {
        Some(match index {
            6 => Self::Tr6,
            7 => Self::Tr7,
            _ => return None,
        })
    }

    pub const fn index(self) -> u8 {
        match self {
            Self::Tr6 => 6,
            Self::Tr7 => 7,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Displacement {
    None,
    Byte(i8),
    Word(i16),
    Doubleword(i32),
    AbsoluteWord(u16),
    AbsoluteDoubleword(u32),
}

impl Displacement {
    pub const fn byte_len(self) -> usize {
        match self {
            Self::None => 0,
            Self::Byte(_) => 1,
            Self::Word(_) | Self::AbsoluteWord(_) => 2,
            Self::Doubleword(_) | Self::AbsoluteDoubleword(_) => 4,
        }
    }

    pub const fn signed_value(self) -> Option<i32> {
        match self {
            Self::None => Some(0),
            Self::Byte(value) => Some(value as i32),
            Self::Word(value) => Some(value as i32),
            Self::Doubleword(value) => Some(value),
            Self::AbsoluteWord(_) | Self::AbsoluteDoubleword(_) => None,
        }
    }

    pub const fn absolute_value(self) -> Option<u32> {
        match self {
            Self::AbsoluteWord(value) => Some(value as u32),
            Self::AbsoluteDoubleword(value) => Some(value),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Address16Base {
    BxSi,
    BxDi,
    BpSi,
    BpDi,
    Si,
    Di,
    Bp,
    Bx,
    Direct,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scale {
    One,
    Two,
    Four,
    Eight,
}

impl Scale {
    pub const fn factor(self) -> u8 {
        1 << self as u8
    }

    pub const fn from_field(field: u8) -> Option<Self> {
        Some(match field {
            0 => Self::One,
            1 => Self::Two,
            2 => Self::Four,
            3 => Self::Eight,
            _ => return None,
        })
    }

    pub const fn field(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScaledIndex {
    register: Register32,
    scale: Scale,
}

impl ScaledIndex {
    pub fn new(register: Register32, scale: Scale) -> Result<Self, AddressError> {
        if register == Register32::Esp {
            return Err(AddressError::StackPointerCannotBeIndex);
        }
        Ok(Self { register, scale })
    }

    pub const fn register(self) -> Register32 {
        self.register
    }

    pub const fn scale(self) -> Scale {
        self.scale
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectiveAddress {
    Bits16 {
        base: Address16Base,
        displacement: Displacement,
    },
    Bits32 {
        base: Option<Register32>,
        index: Option<ScaledIndex>,
        displacement: Displacement,
        uses_sib: bool,
    },
}

impl EffectiveAddress {
    pub fn bits16(base: Address16Base, displacement: Displacement) -> Result<Self, AddressError> {
        match (base, displacement) {
            (Address16Base::Direct, Displacement::AbsoluteWord(_)) => {}
            (Address16Base::Direct, _) => return Err(AddressError::Direct16RequiresWord),
            (Address16Base::Bp, Displacement::None) => {
                return Err(AddressError::BasePointerRequiresDisplacement);
            }
            (_, Displacement::None | Displacement::Byte(_) | Displacement::Word(_)) => {}
            (
                _,
                Displacement::Doubleword(_)
                | Displacement::AbsoluteWord(_)
                | Displacement::AbsoluteDoubleword(_),
            ) => {
                return Err(AddressError::Based16RequiresSignedDisplacement);
            }
        }
        Ok(Self::Bits16 { base, displacement })
    }

    pub fn bits32(
        base: Option<Register32>,
        index: Option<ScaledIndex>,
        displacement: Displacement,
        uses_sib: bool,
    ) -> Result<Self, AddressError> {
        if base.is_none() && !matches!(displacement, Displacement::AbsoluteDoubleword(_)) {
            return Err(AddressError::NoBase32RequiresDoubleword);
        }
        if index.is_some() && !uses_sib {
            return Err(AddressError::Index32RequiresSib);
        }
        if base == Some(Register32::Esp) && !uses_sib {
            return Err(AddressError::StackPointerBaseRequiresSib);
        }
        if base == Some(Register32::Ebp) && matches!(displacement, Displacement::None) {
            return Err(AddressError::BasePointerRequiresDisplacement);
        }
        if base.is_some()
            && matches!(
                displacement,
                Displacement::Word(_)
                    | Displacement::AbsoluteWord(_)
                    | Displacement::AbsoluteDoubleword(_)
            )
        {
            return Err(AddressError::Based32RequiresByteOrDoublewordDisplacement);
        }
        Ok(Self::Bits32 {
            base,
            index,
            displacement,
            uses_sib,
        })
    }

    pub const fn address_size(self) -> AddressSize {
        match self {
            Self::Bits16 { .. } => AddressSize::Bits16,
            Self::Bits32 { .. } => AddressSize::Bits32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AddressError {
    #[error("ESP has no SIB index encoding")]
    StackPointerCannotBeIndex,
    #[error("direct 16-bit addressing requires a 16-bit displacement")]
    Direct16RequiresWord,
    #[error("based 16-bit addressing requires a signed byte or word displacement")]
    Based16RequiresSignedDisplacement,
    #[error("32-bit addressing without a base requires a 32-bit displacement")]
    NoBase32RequiresDoubleword,
    #[error("a 32-bit index requires a SIB encoding")]
    Index32RequiresSib,
    #[error("an ESP base requires a SIB encoding")]
    StackPointerBaseRequiresSib,
    #[error("a BP or EBP base requires an encoded displacement")]
    BasePointerRequiresDisplacement,
    #[error("based 32-bit addressing requires a signed byte or doubleword displacement")]
    Based32RequiresByteOrDoublewordDisplacement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryWidth {
    Byte,
    Word,
    Doubleword,
    BoundPairWord,
    BoundPairDoubleword,
    FarPointerWord,
    FarPointerDoubleword,
    PseudoDescriptor,
    Address,
    Coprocessor,
}

impl From<OperandSize> for MemoryWidth {
    fn from(value: OperandSize) -> Self {
        match value {
            OperandSize::Byte => Self::Byte,
            OperandSize::Word => Self::Word,
            OperandSize::Doubleword => Self::Doubleword,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryOperand {
    width: MemoryWidth,
    segment: Option<SegmentRegister>,
    address: EffectiveAddress,
}

impl MemoryOperand {
    pub const fn new(
        width: MemoryWidth,
        segment: Option<SegmentRegister>,
        address: EffectiveAddress,
    ) -> Self {
        Self {
            width,
            segment,
            address,
        }
    }

    pub const fn width(self) -> MemoryWidth {
        self.width
    }

    pub const fn segment(self) -> Option<SegmentRegister> {
        self.segment
    }

    pub const fn address(self) -> EffectiveAddress {
        self.address
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Immediate {
    Byte(u8),
    Word(u16),
    Doubleword(u32),
    SignExtendedByte(i8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RelativeDisplacement {
    Byte(i8),
    Word(i16),
    Doubleword(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FarPointer {
    Word { offset: u16, segment: u16 },
    Doubleword { offset: u32, segment: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoprocessorOperation(u8);

impl CoprocessorOperation {
    pub fn new(selector: u8) -> Option<Self> {
        (selector < 64).then_some(Self(selector))
    }

    pub const fn selector(self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoprocessorRegister(u8);

impl CoprocessorRegister {
    pub fn new(index: u8) -> Option<Self> {
        (index < 8).then_some(Self(index))
    }

    pub const fn index(self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Operand {
    GeneralRegister(GeneralRegister),
    SegmentRegister(SegmentRegister),
    ControlRegister(ControlRegister),
    DebugRegister(DebugRegister),
    TestRegister(TestRegister),
    Memory(MemoryOperand),
    Immediate(Immediate),
    Relative(RelativeDisplacement),
    FarPointer(FarPointer),
    CoprocessorRegister(CoprocessorRegister),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Condition {
    Overflow,
    NotOverflow,
    Below,
    NotBelow,
    Equal,
    NotEqual,
    BelowOrEqual,
    Above,
    Sign,
    NotSign,
    Parity,
    NotParity,
    Less,
    GreaterOrEqual,
    LessOrEqual,
    Greater,
}

impl Condition {
    pub const fn from_field(field: u8) -> Option<Self> {
        Some(match field {
            0 => Self::Overflow,
            1 => Self::NotOverflow,
            2 => Self::Below,
            3 => Self::NotBelow,
            4 => Self::Equal,
            5 => Self::NotEqual,
            6 => Self::BelowOrEqual,
            7 => Self::Above,
            8 => Self::Sign,
            9 => Self::NotSign,
            10 => Self::Parity,
            11 => Self::NotParity,
            12 => Self::Less,
            13 => Self::GreaterOrEqual,
            14 => Self::LessOrEqual,
            15 => Self::Greater,
            _ => return None,
        })
    }

    pub const fn field(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOperation {
    Add,
    AddWithCarry,
    Subtract,
    SubtractWithBorrow,
    Compare,
    And,
    Or,
    ExclusiveOr,
    Test,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOperation {
    Increment,
    Decrement,
    Negate,
    Not,
    MultiplyUnsigned,
    MultiplySigned,
    DivideUnsigned,
    DivideSigned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShiftOperation {
    RotateLeft,
    RotateRight,
    RotateThroughCarryLeft,
    RotateThroughCarryRight,
    ShiftLeft,
    ShiftRightLogical,
    ShiftRightArithmetic,
    DoubleShiftLeft,
    DoubleShiftRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LoopCondition {
    NotEqual,
    Equal,
    Always,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StringOperation {
    Move,
    Compare,
    Store,
    Load,
    Scan,
    Input,
    Output,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BitOperation {
    Test,
    TestAndSet,
    TestAndReset,
    TestAndComplement,
    ScanForward,
    ScanReverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DescriptorOperation {
    LoadAccessRights,
    LoadSegmentLimit,
    VerifyRead,
    VerifyWrite,
    StoreGlobalDescriptorTable,
    StoreInterruptDescriptorTable,
    LoadGlobalDescriptorTable,
    LoadInterruptDescriptorTable,
    StoreLocalDescriptorTable,
    StoreTaskRegister,
    LoadLocalDescriptorTable,
    LoadTaskRegister,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operation {
    Binary(BinaryOperation),
    Unary(UnaryOperation),
    Shift(ShiftOperation),
    Bit(BitOperation),
    Descriptor(DescriptorOperation),
    Move,
    MoveSignExtend,
    MoveZeroExtend,
    MultiplySigned,
    Exchange,
    LoadEffectiveAddress,
    LoadPointer(SegmentRegister),
    Push,
    Pop,
    PushAll,
    PopAll,
    PushFlags,
    PopFlags,
    Bound,
    AdjustRequestedPrivilege,
    Call,
    Jump,
    ConditionalJump(Condition),
    Loop(LoopCondition),
    JumpCounterZero,
    ReturnNear,
    ReturnFar,
    Enter,
    Leave,
    Interrupt,
    InterruptOnOverflow,
    InterruptReturn,
    Input,
    Output,
    String {
        operation: StringOperation,
        width: OperandSize,
    },
    SetCondition(Condition),
    ConvertByteToWordOrDoubleword,
    ConvertWordOrDoubleword,
    DecimalAdjustAfterAddition,
    DecimalAdjustAfterSubtraction,
    AsciiAdjustAfterAddition,
    AsciiAdjustAfterSubtraction,
    AsciiAdjustAfterMultiply,
    AsciiAdjustBeforeDivision,
    Translate,
    StoreAhIntoFlags,
    LoadFlagsIntoAh,
    ClearCarry,
    SetCarry,
    ComplementCarry,
    ClearDirection,
    SetDirection,
    ClearInterrupt,
    SetInterrupt,
    ClearTaskSwitched,
    Halt,
    NoOperation,
    Wait,
    LoadMachineStatusWord,
    StoreMachineStatusWord,
    CoprocessorEscape(CoprocessorOperation),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Prefix {
    Segment(SegmentRegister),
    OperandSizeOverride,
    AddressSizeOverride,
    Lock,
    RepeatNotEqual,
    Repeat,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct PrefixSequence(Vec<Prefix>);

impl PrefixSequence {
    pub fn new(prefixes: Vec<Prefix>) -> Result<Self, PrefixError> {
        if prefixes.len() > 14 {
            return Err(PrefixError::InstructionLengthExceeded);
        }
        Ok(Self(prefixes))
    }

    pub fn as_slice(&self) -> &[Prefix] {
        &self.0
    }

    pub fn effective_operand_size(&self, mode: CodeMode) -> OperandSize {
        let has_override = self.0.contains(&Prefix::OperandSizeOverride);
        if has_override {
            match mode.default_operand_size() {
                OperandSize::Word => OperandSize::Doubleword,
                OperandSize::Doubleword => OperandSize::Word,
                OperandSize::Byte => unreachable!("code mode never defaults to byte operands"),
            }
        } else {
            mode.default_operand_size()
        }
    }

    pub fn effective_address_size(&self, mode: CodeMode) -> AddressSize {
        let has_override = self.0.contains(&Prefix::AddressSizeOverride);
        if has_override {
            match mode.default_address_size() {
                AddressSize::Bits16 => AddressSize::Bits32,
                AddressSize::Bits32 => AddressSize::Bits16,
            }
        } else {
            mode.default_address_size()
        }
    }

    pub fn effective_segment(&self) -> Option<SegmentRegister> {
        self.0.iter().rev().find_map(|prefix| match prefix {
            Prefix::Segment(segment) => Some(*segment),
            _ => None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PrefixError {
    #[error("prefix sequence leaves no room inside the 15-byte instruction limit")]
    InstructionLengthExceeded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperandKind {
    GeneralRegister(OperandSize),
    SegmentRegister,
    ControlRegister,
    DebugRegister,
    TestRegister,
    Memory {
        width: MemoryWidth,
        address_size: AddressSize,
    },
    Immediate,
    Relative,
    FarPointer,
    CoprocessorRegister,
}

impl Operand {
    pub const fn kind(&self) -> OperandKind {
        match self {
            Self::GeneralRegister(register) => OperandKind::GeneralRegister(register.size()),
            Self::SegmentRegister(_) => OperandKind::SegmentRegister,
            Self::ControlRegister(_) => OperandKind::ControlRegister,
            Self::DebugRegister(_) => OperandKind::DebugRegister,
            Self::TestRegister(_) => OperandKind::TestRegister,
            Self::Memory(memory) => OperandKind::Memory {
                width: memory.width(),
                address_size: memory.address().address_size(),
            },
            Self::Immediate(_) => OperandKind::Immediate,
            Self::Relative(_) => OperandKind::Relative,
            Self::FarPointer(_) => OperandKind::FarPointer,
            Self::CoprocessorRegister(_) => OperandKind::CoprocessorRegister,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InstructionForm {
    operation: Operation,
    operands: Vec<OperandKind>,
}

impl InstructionForm {
    pub(crate) fn new(operation: Operation, operands: &[Operand]) -> Self {
        Self {
            operation,
            operands: operands.iter().map(Operand::kind).collect(),
        }
    }

    pub const fn operation(&self) -> Operation {
        self.operation
    }

    pub fn operands(&self) -> &[OperandKind] {
        &self.operands
    }
}
