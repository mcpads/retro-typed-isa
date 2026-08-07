#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Code-segment D-bit default used while decoding and encoding.
pub enum CodeMode {
    /// The `Code16` code-segment default-size mode value.
    Code16,
    /// The `Code32` code-segment default-size mode value.
    Code32,
}

impl CodeMode {
    /// Returns the operand size selected when no `66` prefix is present.
    pub const fn default_operand_size(self) -> OperandSize {
        match self {
            Self::Code16 => OperandSize::Word,
            Self::Code32 => OperandSize::Doubleword,
        }
    }

    /// Returns the address size selected when no `67` prefix is present.
    pub const fn default_address_size(self) -> AddressSize {
        match self {
            Self::Code16 => AddressSize::Bits16,
            Self::Code32 => AddressSize::Bits32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Semantic width of an integer operand.
pub enum OperandSize {
    /// The `Byte` operand width value.
    Byte,
    /// The `Word` operand width value.
    Word,
    /// The `Doubleword` operand width value.
    Doubleword,
}

impl OperandSize {
    /// Returns the operand width in bytes.
    pub const fn byte_len(self) -> usize {
        match self {
            Self::Byte => 1,
            Self::Word => 2,
            Self::Doubleword => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Address-calculation width selected by the code mode and prefixes.
pub enum AddressSize {
    /// The `Bits16` effective-address width value.
    Bits16,
    /// The `Bits32` effective-address width value.
    Bits32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// An encodable 80386 eight-bit general register.
pub enum Register8 {
    /// The `Al` eight-bit register value.
    Al,
    /// The `Cl` eight-bit register value.
    Cl,
    /// The `Dl` eight-bit register value.
    Dl,
    /// The `Bl` eight-bit register value.
    Bl,
    /// The `Ah` eight-bit register value.
    Ah,
    /// The `Ch` eight-bit register value.
    Ch,
    /// The `Dh` eight-bit register value.
    Dh,
    /// The `Bh` eight-bit register value.
    Bh,
}

impl Register8 {
    /// Decodes a three-bit general-register index.
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

    /// Returns the three-bit ModR/M register index.
    pub const fn index(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// An encodable 80386 sixteen-bit general register.
pub enum Register16 {
    /// The `Ax` sixteen-bit register value.
    Ax,
    /// The `Cx` sixteen-bit register value.
    Cx,
    /// The `Dx` sixteen-bit register value.
    Dx,
    /// The `Bx` sixteen-bit register value.
    Bx,
    /// The `Sp` sixteen-bit register value.
    Sp,
    /// The `Bp` sixteen-bit register value.
    Bp,
    /// The `Si` sixteen-bit register value.
    Si,
    /// The `Di` sixteen-bit register value.
    Di,
}

impl Register16 {
    /// Decodes a three-bit general-register index.
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

    /// Returns the three-bit ModR/M register index.
    pub const fn index(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// An encodable 80386 thirty-two-bit general register.
pub enum Register32 {
    /// The `Eax` thirty-two-bit register value.
    Eax,
    /// The `Ecx` thirty-two-bit register value.
    Ecx,
    /// The `Edx` thirty-two-bit register value.
    Edx,
    /// The `Ebx` thirty-two-bit register value.
    Ebx,
    /// The `Esp` thirty-two-bit register value.
    Esp,
    /// The `Ebp` thirty-two-bit register value.
    Ebp,
    /// The `Esi` thirty-two-bit register value.
    Esi,
    /// The `Edi` thirty-two-bit register value.
    Edi,
}

impl Register32 {
    /// Decodes a three-bit general-register index.
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

    /// Returns the three-bit ModR/M register index.
    pub const fn index(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A general register carrying its encoded operand width.
pub enum GeneralRegister {
    /// The `Byte` sized general register value.
    Byte(Register8),
    /// The `Word` sized general register value.
    Word(Register16),
    /// The `Doubleword` sized general register value.
    Doubleword(Register32),
}

impl GeneralRegister {
    /// Returns the register's semantic operand width.
    pub const fn size(self) -> OperandSize {
        match self {
            Self::Byte(_) => OperandSize::Byte,
            Self::Word(_) => OperandSize::Word,
            Self::Doubleword(_) => OperandSize::Doubleword,
        }
    }

    /// Returns the three-bit ModR/M register index.
    pub const fn index(self) -> u8 {
        match self {
            Self::Byte(register) => register.index(),
            Self::Word(register) => register.index(),
            Self::Doubleword(register) => register.index(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// An encodable 80386 segment register.
pub enum SegmentRegister {
    /// The `Es` segment register value.
    Es,
    /// The `Cs` segment register value.
    Cs,
    /// The `Ss` segment register value.
    Ss,
    /// The `Ds` segment register value.
    Ds,
    /// The `Fs` segment register value.
    Fs,
    /// The `Gs` segment register value.
    Gs,
}

impl SegmentRegister {
    /// Decodes a three-bit ModR/M segment-register index.
    ///
    /// Indices six and seven are reserved on the 80386.
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

    /// Returns the three-bit ModR/M segment-register index.
    pub const fn modrm_index(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A control register implemented by the 80386 profile.
pub enum ControlRegister {
    /// The `Cr0` control register value.
    Cr0,
    /// The `Cr2` control register value.
    Cr2,
    /// The `Cr3` control register value.
    Cr3,
}

impl ControlRegister {
    /// Decodes a documented 80386 control-register index.
    pub const fn from_index(index: u8) -> Option<Self> {
        match index {
            0 => Some(Self::Cr0),
            2 => Some(Self::Cr2),
            3 => Some(Self::Cr3),
            _ => None,
        }
    }

    /// Returns the architectural control-register index.
    pub const fn index(self) -> u8 {
        match self {
            Self::Cr0 => 0,
            Self::Cr2 => 2,
            Self::Cr3 => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A debug register implemented by the 80386 profile.
pub enum DebugRegister {
    /// The `Dr0` debug register value.
    Dr0,
    /// The `Dr1` debug register value.
    Dr1,
    /// The `Dr2` debug register value.
    Dr2,
    /// The `Dr3` debug register value.
    Dr3,
    /// The `Dr6` debug register value.
    Dr6,
    /// The `Dr7` debug register value.
    Dr7,
}

impl DebugRegister {
    /// Decodes a documented 80386 debug-register index.
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

    /// Returns the architectural debug-register index.
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
/// A test register implemented by the 80386 profile.
pub enum TestRegister {
    /// The `Tr6` test register value.
    Tr6,
    /// The `Tr7` test register value.
    Tr7,
}

impl TestRegister {
    /// Decodes a documented 80386 test-register index.
    pub const fn from_index(index: u8) -> Option<Self> {
        Some(match index {
            6 => Self::Tr6,
            7 => Self::Tr7,
            _ => return None,
        })
    }

    /// Returns the architectural test-register index.
    pub const fn index(self) -> u8 {
        match self {
            Self::Tr6 => 6,
            Self::Tr7 => 7,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// The exact encoded form of an effective-address displacement.
pub enum Displacement {
    /// The `None` effective-address displacement value.
    None,
    /// The `Byte` effective-address displacement value.
    Byte(i8),
    /// The `Word` effective-address displacement value.
    Word(i16),
    /// The `Doubleword` effective-address displacement value.
    Doubleword(i32),
    /// The `AbsoluteWord` effective-address displacement value.
    AbsoluteWord(u16),
    /// The `AbsoluteDoubleword` effective-address displacement value.
    AbsoluteDoubleword(u32),
}

impl Displacement {
    /// Returns the number of serialized displacement bytes.
    pub const fn byte_len(self) -> usize {
        match self {
            Self::None => 0,
            Self::Byte(_) => 1,
            Self::Word(_) | Self::AbsoluteWord(_) => 2,
            Self::Doubleword(_) | Self::AbsoluteDoubleword(_) => 4,
        }
    }

    /// Returns a sign-extended value for relative displacements.
    pub const fn signed_value(self) -> Option<i32> {
        match self {
            Self::None => Some(0),
            Self::Byte(value) => Some(value as i32),
            Self::Word(value) => Some(value as i32),
            Self::Doubleword(value) => Some(value),
            Self::AbsoluteWord(_) | Self::AbsoluteDoubleword(_) => None,
        }
    }

    /// Returns the zero-extended value of a direct address displacement.
    pub const fn absolute_value(self) -> Option<u32> {
        match self {
            Self::AbsoluteWord(value) => Some(value as u32),
            Self::AbsoluteDoubleword(value) => Some(value),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A base/index combination from the 16-bit ModR/M addressing table.
pub enum Address16Base {
    /// The `BxSi` 16-bit addressing base combination value.
    BxSi,
    /// The `BxDi` 16-bit addressing base combination value.
    BxDi,
    /// The `BpSi` 16-bit addressing base combination value.
    BpSi,
    /// The `BpDi` 16-bit addressing base combination value.
    BpDi,
    /// The `Si` 16-bit addressing base combination value.
    Si,
    /// The `Di` 16-bit addressing base combination value.
    Di,
    /// The `Bp` 16-bit addressing base combination value.
    Bp,
    /// The `Bx` 16-bit addressing base combination value.
    Bx,
    /// The `Direct` 16-bit addressing base combination value.
    Direct,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A scale factor from a 32-bit SIB byte.
pub enum Scale {
    /// The `One` SIB scale value.
    One,
    /// The `Two` SIB scale value.
    Two,
    /// The `Four` SIB scale value.
    Four,
    /// The `Eight` SIB scale value.
    Eight,
}

impl Scale {
    /// Returns the numeric SIB scale factor.
    pub const fn factor(self) -> u8 {
        1 << self as u8
    }

    /// Decodes a two-bit SIB scale field.
    pub const fn from_field(field: u8) -> Option<Self> {
        Some(match field {
            0 => Self::One,
            1 => Self::Two,
            2 => Self::Four,
            3 => Self::Eight,
            _ => return None,
        })
    }

    /// Returns the two-bit SIB scale field.
    pub const fn field(self) -> u8 {
        self as u8
    }
}

/// A validated index-register and scale pair for a SIB byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScaledIndex {
    register: Register32,
    scale: Scale,
}

impl ScaledIndex {
    /// Constructs a scaled index, rejecting `esp` because it denotes no index.
    pub fn new(register: Register32, scale: Scale) -> Result<Self, AddressError> {
        if register == Register32::Esp {
            return Err(AddressError::StackPointerCannotBeIndex);
        }
        Ok(Self { register, scale })
    }

    /// Returns the index register.
    pub const fn register(self) -> Register32 {
        self.register
    }

    /// Returns the index scale.
    pub const fn scale(self) -> Scale {
        self.scale
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A validated 16-bit or 32-bit ModR/M effective address.
pub enum EffectiveAddress {
    /// The `Bits16` validated effective address value.
    Bits16 {
        /// Base register or 16-bit addressing combination.
        base: Address16Base,
        /// Exact displacement encoding.
        displacement: Displacement,
    },
    /// The `Bits32` validated effective address value.
    Bits32 {
        /// Base register or 16-bit addressing combination.
        base: Option<Register32>,
        /// Optional scaled index register.
        index: Option<ScaledIndex>,
        /// Exact displacement encoding.
        displacement: Displacement,
        /// The `uses_sib` component.
        uses_sib: bool,
    },
}

impl EffectiveAddress {
    /// Constructs a 16-bit ModR/M address with an exact displacement encoding.
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

    /// Constructs a 32-bit ModR/M/SIB address with an exact displacement encoding.
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

    /// Returns the effective-address width.
    pub const fn address_size(self) -> AddressSize {
        match self {
            Self::Bits16 { .. } => AddressSize::Bits16,
            Self::Bits32 { .. } => AddressSize::Bits32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
/// Failure to construct a representable 80386 effective address.
pub enum AddressError {
    #[error("ESP has no SIB index encoding")]
    /// The `StackPointerCannotBeIndex` effective-address construction error value.
    StackPointerCannotBeIndex,
    #[error("direct 16-bit addressing requires a 16-bit displacement")]
    /// The `Direct16RequiresWord` effective-address construction error value.
    Direct16RequiresWord,
    #[error("based 16-bit addressing requires a signed byte or word displacement")]
    /// The `Based16RequiresSignedDisplacement` effective-address construction error value.
    Based16RequiresSignedDisplacement,
    #[error("32-bit addressing without a base requires a 32-bit displacement")]
    /// The `NoBase32RequiresDoubleword` effective-address construction error value.
    NoBase32RequiresDoubleword,
    #[error("a 32-bit index requires a SIB encoding")]
    /// The `Index32RequiresSib` effective-address construction error value.
    Index32RequiresSib,
    #[error("an ESP base requires a SIB encoding")]
    /// The `StackPointerBaseRequiresSib` effective-address construction error value.
    StackPointerBaseRequiresSib,
    #[error("a BP or EBP base requires an encoded displacement")]
    /// The `BasePointerRequiresDisplacement` effective-address construction error value.
    BasePointerRequiresDisplacement,
    #[error("based 32-bit addressing requires a signed byte or doubleword displacement")]
    /// The `Based32RequiresByteOrDoublewordDisplacement` effective-address construction error value.
    Based32RequiresByteOrDoublewordDisplacement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Semantic width or memory-object class required by an operand.
pub enum MemoryWidth {
    /// The `Byte` semantic memory width value.
    Byte,
    /// The `Word` semantic memory width value.
    Word,
    /// The `Doubleword` semantic memory width value.
    Doubleword,
    /// The `BoundPairWord` semantic memory width value.
    BoundPairWord,
    /// The `BoundPairDoubleword` semantic memory width value.
    BoundPairDoubleword,
    /// The `FarPointerWord` semantic memory width value.
    FarPointerWord,
    /// The `FarPointerDoubleword` semantic memory width value.
    FarPointerDoubleword,
    /// The `PseudoDescriptor` semantic memory width value.
    PseudoDescriptor,
    /// The `Address` semantic memory width value.
    Address,
    /// The `Coprocessor` semantic memory width value.
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

/// A memory operand with semantic width, optional segment override, and address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryOperand {
    width: MemoryWidth,
    segment: Option<SegmentRegister>,
    address: EffectiveAddress,
}

impl MemoryOperand {
    /// Constructs a memory operand from independently validated components.
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

    /// Returns the semantic memory width or object class.
    pub const fn width(self) -> MemoryWidth {
        self.width
    }

    /// Returns the explicit segment override, if any.
    pub const fn segment(self) -> Option<SegmentRegister> {
        self.segment
    }

    /// Returns the effective address.
    pub const fn address(self) -> EffectiveAddress {
        self.address
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// The exact encoded form of an immediate operand.
pub enum Immediate {
    /// The `Byte` immediate value value.
    Byte(u8),
    /// The `Word` immediate value value.
    Word(u16),
    /// The `Doubleword` immediate value value.
    Doubleword(u32),
    /// The `SignExtendedByte` immediate value value.
    SignExtendedByte(i8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// The exact encoded width of a relative control-flow displacement.
pub enum RelativeDisplacement {
    /// The `Byte` relative displacement value.
    Byte(i8),
    /// The `Word` relative displacement value.
    Word(i16),
    /// The `Doubleword` relative displacement value.
    Doubleword(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A segmented far pointer with mode-dependent offset width.
pub enum FarPointer {
    /// The `Word` far pointer value.
    Word {
        /// Sixteen-bit offset selected by a word-size form.
        offset: u16,
        /// Sixteen-bit segment selector.
        segment: u16,
    },
    /// The `Doubleword` far pointer value.
    Doubleword {
        /// Thirty-two-bit offset selected by a doubleword-size form.
        offset: u32,
        /// Sixteen-bit segment selector.
        segment: u16,
    },
}

/// A six-bit x87 escape-operation selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoprocessorOperation(u8);

impl CoprocessorOperation {
    /// Constructs a selector from values zero through 63.
    pub fn new(selector: u8) -> Option<Self> {
        (selector < 64).then_some(Self(selector))
    }

    /// Returns the six-bit selector.
    pub const fn selector(self) -> u8 {
        self.0
    }
}

/// One of the eight x87 stack-register selectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoprocessorRegister(u8);

impl CoprocessorRegister {
    /// Constructs a register from values zero through seven.
    pub fn new(index: u8) -> Option<Self> {
        (index < 8).then_some(Self(index))
    }

    /// Returns the three-bit register index.
    pub const fn index(self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// A typed semantic operand of an 80386 instruction.
pub enum Operand {
    /// The `GeneralRegister` typed instruction operand value.
    GeneralRegister(GeneralRegister),
    /// The `SegmentRegister` typed instruction operand value.
    SegmentRegister(SegmentRegister),
    /// The `ControlRegister` typed instruction operand value.
    ControlRegister(ControlRegister),
    /// The `DebugRegister` typed instruction operand value.
    DebugRegister(DebugRegister),
    /// The `TestRegister` typed instruction operand value.
    TestRegister(TestRegister),
    /// The `Memory` typed instruction operand value.
    Memory(MemoryOperand),
    /// The `Immediate` typed instruction operand value.
    Immediate(Immediate),
    /// The `Relative` typed instruction operand value.
    Relative(RelativeDisplacement),
    /// The `FarPointer` typed instruction operand value.
    FarPointer(FarPointer),
    /// The `CoprocessorRegister` typed instruction operand value.
    CoprocessorRegister(CoprocessorRegister),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// One of the sixteen 80386 condition-code predicates.
pub enum Condition {
    /// The `Overflow` condition-code predicate value.
    Overflow,
    /// The `NotOverflow` condition-code predicate value.
    NotOverflow,
    /// The `Below` condition-code predicate value.
    Below,
    /// The `NotBelow` condition-code predicate value.
    NotBelow,
    /// The `Equal` condition-code predicate value.
    Equal,
    /// The `NotEqual` condition-code predicate value.
    NotEqual,
    /// The `BelowOrEqual` condition-code predicate value.
    BelowOrEqual,
    /// The `Above` condition-code predicate value.
    Above,
    /// The `Sign` condition-code predicate value.
    Sign,
    /// The `NotSign` condition-code predicate value.
    NotSign,
    /// The `Parity` condition-code predicate value.
    Parity,
    /// The `NotParity` condition-code predicate value.
    NotParity,
    /// The `Less` condition-code predicate value.
    Less,
    /// The `GreaterOrEqual` condition-code predicate value.
    GreaterOrEqual,
    /// The `LessOrEqual` condition-code predicate value.
    LessOrEqual,
    /// The `Greater` condition-code predicate value.
    Greater,
}

impl Condition {
    /// Decodes a four-bit condition field.
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

    /// Returns the four-bit condition field.
    pub const fn field(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A binary arithmetic, logical, compare, or test operation.
pub enum BinaryOperation {
    /// The `Add` binary operation value.
    Add,
    /// The `AddWithCarry` binary operation value.
    AddWithCarry,
    /// The `Subtract` binary operation value.
    Subtract,
    /// The `SubtractWithBorrow` binary operation value.
    SubtractWithBorrow,
    /// The `Compare` binary operation value.
    Compare,
    /// The `And` binary operation value.
    And,
    /// The `Or` binary operation value.
    Or,
    /// The `ExclusiveOr` binary operation value.
    ExclusiveOr,
    /// The `Test` binary operation value.
    Test,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A unary arithmetic, logical, multiply, or divide operation.
pub enum UnaryOperation {
    /// The `Increment` unary operation value.
    Increment,
    /// The `Decrement` unary operation value.
    Decrement,
    /// The `Negate` unary operation value.
    Negate,
    /// The `Not` unary operation value.
    Not,
    /// The `MultiplyUnsigned` unary operation value.
    MultiplyUnsigned,
    /// The `MultiplySigned` unary operation value.
    MultiplySigned,
    /// The `DivideUnsigned` unary operation value.
    DivideUnsigned,
    /// The `DivideSigned` unary operation value.
    DivideSigned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A rotate, shift, or double-shift operation.
pub enum ShiftOperation {
    /// The `RotateLeft` shift or rotate operation value.
    RotateLeft,
    /// The `RotateRight` shift or rotate operation value.
    RotateRight,
    /// The `RotateThroughCarryLeft` shift or rotate operation value.
    RotateThroughCarryLeft,
    /// The `RotateThroughCarryRight` shift or rotate operation value.
    RotateThroughCarryRight,
    /// The `ShiftLeft` shift or rotate operation value.
    ShiftLeft,
    /// The `ShiftRightLogical` shift or rotate operation value.
    ShiftRightLogical,
    /// The `ShiftRightArithmetic` shift or rotate operation value.
    ShiftRightArithmetic,
    /// The `DoubleShiftLeft` shift or rotate operation value.
    DoubleShiftLeft,
    /// The `DoubleShiftRight` shift or rotate operation value.
    DoubleShiftRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// The zero-flag condition applied by a loop instruction.
pub enum LoopCondition {
    /// The `NotEqual` loop termination condition value.
    NotEqual,
    /// The `Equal` loop termination condition value.
    Equal,
    /// The `Always` loop termination condition value.
    Always,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A repeated or non-repeated string data operation.
pub enum StringOperation {
    /// The `Move` string operation value.
    Move,
    /// The `Compare` string operation value.
    Compare,
    /// The `Store` string operation value.
    Store,
    /// The `Load` string operation value.
    Load,
    /// The `Scan` string operation value.
    Scan,
    /// The `Input` string operation value.
    Input,
    /// The `Output` string operation value.
    Output,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A bit-test, bit-modify, or bit-scan operation.
pub enum BitOperation {
    /// The `Test` bit operation value.
    Test,
    /// The `TestAndSet` bit operation value.
    TestAndSet,
    /// The `TestAndReset` bit operation value.
    TestAndReset,
    /// The `TestAndComplement` bit operation value.
    TestAndComplement,
    /// The `ScanForward` bit operation value.
    ScanForward,
    /// The `ScanReverse` bit operation value.
    ScanReverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A protected-mode descriptor, selector, or task-register operation.
pub enum DescriptorOperation {
    /// The `LoadAccessRights` descriptor-table operation value.
    LoadAccessRights,
    /// The `LoadSegmentLimit` descriptor-table operation value.
    LoadSegmentLimit,
    /// The `VerifyRead` descriptor-table operation value.
    VerifyRead,
    /// The `VerifyWrite` descriptor-table operation value.
    VerifyWrite,
    /// The `StoreGlobalDescriptorTable` descriptor-table operation value.
    StoreGlobalDescriptorTable,
    /// The `StoreInterruptDescriptorTable` descriptor-table operation value.
    StoreInterruptDescriptorTable,
    /// The `LoadGlobalDescriptorTable` descriptor-table operation value.
    LoadGlobalDescriptorTable,
    /// The `LoadInterruptDescriptorTable` descriptor-table operation value.
    LoadInterruptDescriptorTable,
    /// The `StoreLocalDescriptorTable` descriptor-table operation value.
    StoreLocalDescriptorTable,
    /// The `StoreTaskRegister` descriptor-table operation value.
    StoreTaskRegister,
    /// The `LoadLocalDescriptorTable` descriptor-table operation value.
    LoadLocalDescriptorTable,
    /// The `LoadTaskRegister` descriptor-table operation value.
    LoadTaskRegister,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A semantic 80386 operation independent of opcode spelling.
pub enum Operation {
    /// The `Binary` semantic instruction operation value.
    Binary(BinaryOperation),
    /// The `Unary` semantic instruction operation value.
    Unary(UnaryOperation),
    /// The `Shift` semantic instruction operation value.
    Shift(ShiftOperation),
    /// The `Bit` semantic instruction operation value.
    Bit(BitOperation),
    /// The `Descriptor` semantic instruction operation value.
    Descriptor(DescriptorOperation),
    /// The `Move` semantic instruction operation value.
    Move,
    /// The `MoveSignExtend` semantic instruction operation value.
    MoveSignExtend,
    /// The `MoveZeroExtend` semantic instruction operation value.
    MoveZeroExtend,
    /// The `MultiplySigned` semantic instruction operation value.
    MultiplySigned,
    /// The `Exchange` semantic instruction operation value.
    Exchange,
    /// The `LoadEffectiveAddress` semantic instruction operation value.
    LoadEffectiveAddress,
    /// The `LoadPointer` semantic instruction operation value.
    LoadPointer(SegmentRegister),
    /// The `Push` semantic instruction operation value.
    Push,
    /// The `Pop` semantic instruction operation value.
    Pop,
    /// The `PushAll` semantic instruction operation value.
    PushAll,
    /// The `PopAll` semantic instruction operation value.
    PopAll,
    /// The `PushFlags` semantic instruction operation value.
    PushFlags,
    /// The `PopFlags` semantic instruction operation value.
    PopFlags,
    /// The `Bound` semantic instruction operation value.
    Bound,
    /// The `AdjustRequestedPrivilege` semantic instruction operation value.
    AdjustRequestedPrivilege,
    /// The `Call` semantic instruction operation value.
    Call,
    /// The `Jump` semantic instruction operation value.
    Jump,
    /// The `ConditionalJump` semantic instruction operation value.
    ConditionalJump(Condition),
    /// The `Loop` semantic instruction operation value.
    Loop(LoopCondition),
    /// The `JumpCounterZero` semantic instruction operation value.
    JumpCounterZero,
    /// The `ReturnNear` semantic instruction operation value.
    ReturnNear,
    /// The `ReturnFar` semantic instruction operation value.
    ReturnFar,
    /// The `Enter` semantic instruction operation value.
    Enter,
    /// The `Leave` semantic instruction operation value.
    Leave,
    /// The `Interrupt` semantic instruction operation value.
    Interrupt,
    /// The `InterruptOnOverflow` semantic instruction operation value.
    InterruptOnOverflow,
    /// The `InterruptReturn` semantic instruction operation value.
    InterruptReturn,
    /// The `Input` semantic instruction operation value.
    Input,
    /// The `Output` semantic instruction operation value.
    Output,
    /// The `String` semantic instruction operation value.
    String {
        /// Selected operation within the family.
        operation: StringOperation,
        /// Semantic operand or memory width.
        width: OperandSize,
    },
    /// The `SetCondition` semantic instruction operation value.
    SetCondition(Condition),
    /// The `ConvertByteToWordOrDoubleword` semantic instruction operation value.
    ConvertByteToWordOrDoubleword,
    /// The `ConvertWordOrDoubleword` semantic instruction operation value.
    ConvertWordOrDoubleword,
    /// The `DecimalAdjustAfterAddition` semantic instruction operation value.
    DecimalAdjustAfterAddition,
    /// The `DecimalAdjustAfterSubtraction` semantic instruction operation value.
    DecimalAdjustAfterSubtraction,
    /// The `AsciiAdjustAfterAddition` semantic instruction operation value.
    AsciiAdjustAfterAddition,
    /// The `AsciiAdjustAfterSubtraction` semantic instruction operation value.
    AsciiAdjustAfterSubtraction,
    /// The `AsciiAdjustAfterMultiply` semantic instruction operation value.
    AsciiAdjustAfterMultiply,
    /// The `AsciiAdjustBeforeDivision` semantic instruction operation value.
    AsciiAdjustBeforeDivision,
    /// The `Translate` semantic instruction operation value.
    Translate,
    /// The `StoreAhIntoFlags` semantic instruction operation value.
    StoreAhIntoFlags,
    /// The `LoadFlagsIntoAh` semantic instruction operation value.
    LoadFlagsIntoAh,
    /// The `ClearCarry` semantic instruction operation value.
    ClearCarry,
    /// The `SetCarry` semantic instruction operation value.
    SetCarry,
    /// The `ComplementCarry` semantic instruction operation value.
    ComplementCarry,
    /// The `ClearDirection` semantic instruction operation value.
    ClearDirection,
    /// The `SetDirection` semantic instruction operation value.
    SetDirection,
    /// The `ClearInterrupt` semantic instruction operation value.
    ClearInterrupt,
    /// The `SetInterrupt` semantic instruction operation value.
    SetInterrupt,
    /// The `ClearTaskSwitched` semantic instruction operation value.
    ClearTaskSwitched,
    /// The `Halt` semantic instruction operation value.
    Halt,
    /// The `NoOperation` semantic instruction operation value.
    NoOperation,
    /// The `Wait` semantic instruction operation value.
    Wait,
    /// The `LoadMachineStatusWord` semantic instruction operation value.
    LoadMachineStatusWord,
    /// The `StoreMachineStatusWord` semantic instruction operation value.
    StoreMachineStatusWord,
    /// The `CoprocessorEscape` semantic instruction operation value.
    CoprocessorEscape(CoprocessorOperation),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A semantic legacy instruction prefix.
pub enum Prefix {
    /// The `Segment` instruction prefix value.
    Segment(SegmentRegister),
    /// The `OperandSizeOverride` instruction prefix value.
    OperandSizeOverride,
    /// The `AddressSizeOverride` instruction prefix value.
    AddressSizeOverride,
    /// The `Lock` instruction prefix value.
    Lock,
    /// The `RepeatNotEqual` instruction prefix value.
    RepeatNotEqual,
    /// The `Repeat` instruction prefix value.
    Repeat,
}

/// Validated sequence of legacy prefixes in encoded order.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct PrefixSequence(Vec<Prefix>);

impl PrefixSequence {
    /// Constructs a prefix sequence within the 15-byte instruction limit.
    pub fn new(prefixes: Vec<Prefix>) -> Result<Self, PrefixError> {
        if prefixes.len() > 14 {
            return Err(PrefixError::InstructionLengthExceeded);
        }
        Ok(Self(prefixes))
    }

    /// Returns prefixes in encoded order.
    pub fn as_slice(&self) -> &[Prefix] {
        &self.0
    }

    /// Applies the operand-size override to the code-mode default.
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

    /// Applies the address-size override to the code-mode default.
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

    /// Returns the last segment override, if any.
    pub fn effective_segment(&self) -> Option<SegmentRegister> {
        self.0.iter().rev().find_map(|prefix| match prefix {
            Prefix::Segment(segment) => Some(*segment),
            _ => None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
/// Failure to construct a legal prefix sequence.
pub enum PrefixError {
    #[error("prefix sequence leaves no room inside the 15-byte instruction limit")]
    /// The `InstructionLengthExceeded` prefix-sequence error value.
    InstructionLengthExceeded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A structural operand descriptor used by an instruction form.
pub enum OperandKind {
    /// The `GeneralRegister` operand-kind descriptor value.
    GeneralRegister(OperandSize),
    /// The `SegmentRegister` operand-kind descriptor value.
    SegmentRegister,
    /// The `ControlRegister` operand-kind descriptor value.
    ControlRegister,
    /// The `DebugRegister` operand-kind descriptor value.
    DebugRegister,
    /// The `TestRegister` operand-kind descriptor value.
    TestRegister,
    /// The `Memory` operand-kind descriptor value.
    Memory {
        /// Semantic operand or memory width.
        width: MemoryWidth,
        /// Required effective-address width.
        address_size: AddressSize,
    },
    /// The `Immediate` operand-kind descriptor value.
    Immediate,
    /// The `Relative` operand-kind descriptor value.
    Relative,
    /// The `FarPointer` operand-kind descriptor value.
    FarPointer,
    /// The `CoprocessorRegister` operand-kind descriptor value.
    CoprocessorRegister,
}

impl Operand {
    /// Returns the structural descriptor of this operand.
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

/// Stable operation and operand-kind signature of an instruction.
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

    /// Returns the semantic operation.
    pub const fn operation(&self) -> Operation {
        self.operation
    }

    /// Returns operand descriptors in syntax order.
    pub fn operands(&self) -> &[OperandKind] {
        &self.operands
    }
}
