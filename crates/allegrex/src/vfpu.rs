use thiserror::Error;

use crate::GeneralRegister;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VectorSize {
    Single,
    Pair,
    Triple,
    Quad,
}

/// A seven-bit VFPU vector-register specifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VectorRegister(u8);

impl VectorRegister {
    pub const fn new(index: u8) -> Option<Self> {
        if index < 128 { Some(Self(index)) } else { None }
    }

    pub const fn index(self) -> u8 {
        self.0
    }

    pub(crate) const fn from_field(index: u8) -> Self {
        debug_assert!(index < 128);
        Self(index)
    }
}

/// A seven-bit VFPU matrix-register specifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MatrixRegister(u8);

impl MatrixRegister {
    pub const fn new(index: u8) -> Option<Self> {
        if index < 128 { Some(Self(index)) } else { None }
    }

    pub const fn index(self) -> u8 {
        self.0
    }

    pub(crate) const fn from_field(index: u8) -> Self {
        debug_assert!(index < 128);
        Self(index)
    }
}

/// One of the fourteen implemented VFPU control registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum VfpuControlRegister {
    SourcePrefix = 128,
    TargetPrefix = 129,
    DestinationPrefix = 130,
    Condition = 131,
    InternalInformation4 = 132,
    Revision = 135,
    RandomState0 = 136,
    RandomState1 = 137,
    RandomState2 = 138,
    RandomState3 = 139,
    RandomState4 = 140,
    RandomState5 = 141,
    RandomState6 = 142,
    RandomState7 = 143,
}

impl VfpuControlRegister {
    pub const fn new(index: u8) -> Option<Self> {
        Some(match index {
            128 => Self::SourcePrefix,
            129 => Self::TargetPrefix,
            130 => Self::DestinationPrefix,
            131 => Self::Condition,
            132 => Self::InternalInformation4,
            135 => Self::Revision,
            136 => Self::RandomState0,
            137 => Self::RandomState1,
            138 => Self::RandomState2,
            139 => Self::RandomState3,
            140 => Self::RandomState4,
            141 => Self::RandomState5,
            142 => Self::RandomState6,
            143 => Self::RandomState7,
            _ => return None,
        })
    }

    pub const fn index(self) -> u8 {
        self as u8
    }

    pub const fn is_writable(self) -> bool {
        !matches!(self, Self::Revision)
    }
}

/// One of the eight VFPU condition-code bits addressable by branch and move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VfpuConditionBit(u8);

impl VfpuConditionBit {
    pub const fn new(index: u8) -> Option<Self> {
        if index < 8 { Some(Self(index)) } else { None }
    }

    pub const fn index(self) -> u8 {
        self.0
    }

    pub(crate) const fn from_field(index: u8) -> Self {
        debug_assert!(index < 8);
        Self(index)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum VfpuCompareCondition {
    AlwaysFalse = 0,
    Equal = 1,
    LessThan = 2,
    LessThanOrEqual = 3,
    AlwaysTrue = 4,
    NotEqual = 5,
    GreaterThanOrEqual = 6,
    GreaterThan = 7,
    EqualZero = 8,
    EqualNan = 9,
    EqualInfinity = 10,
    EqualInfinityOrNan = 11,
    NotEqualZero = 12,
    NotEqualNan = 13,
    NotEqualInfinity = 14,
    NotEqualInfinityAndNotNan = 15,
}

impl VfpuCompareCondition {
    pub const ALL: [Self; 16] = [
        Self::AlwaysFalse,
        Self::Equal,
        Self::LessThan,
        Self::LessThanOrEqual,
        Self::AlwaysTrue,
        Self::NotEqual,
        Self::GreaterThanOrEqual,
        Self::GreaterThan,
        Self::EqualZero,
        Self::EqualNan,
        Self::EqualInfinity,
        Self::EqualInfinityOrNan,
        Self::NotEqualZero,
        Self::NotEqualNan,
        Self::NotEqualInfinity,
        Self::NotEqualInfinityAndNotNan,
    ];

    pub const fn code(self) -> u8 {
        self as u8
    }

    pub(crate) const fn from_field(value: u8) -> Self {
        Self::ALL[value as usize]
    }
}

/// A constant selector accepted by VCST. Codes 0 and 20 through 31 are undefined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VfpuConstant(u8);

impl VfpuConstant {
    pub const fn new(code: u8) -> Option<Self> {
        if code >= 1 && code <= 19 {
            Some(Self(code))
        } else {
            None
        }
    }

    pub const fn code(self) -> u8 {
        self.0
    }
}

macro_rules! define_bounded_immediate {
    ($name:ident, $maximum:expr) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u32);

        impl $name {
            pub const fn new(value: u32) -> Option<Self> {
                if value <= $maximum {
                    Some(Self(value))
                } else {
                    None
                }
            }

            pub const fn value(self) -> u32 {
                self.0
            }

            pub(crate) const fn from_field(value: u32) -> Self {
                debug_assert!(value <= $maximum);
                Self(value)
            }
        }
    };
}

define_bounded_immediate!(Immediate5, 0x1f);
define_bounded_immediate!(Immediate8, 0xff);
define_bounded_immediate!(Immediate16, 0xffff);
/// A signed VFPU load/store offset whose low two bits are zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VfpuMemoryOffset(i16);

impl VfpuMemoryOffset {
    pub const fn new(value: i16) -> Option<Self> {
        if value & 3 == 0 {
            Some(Self(value))
        } else {
            None
        }
    }

    pub const fn value(self) -> i16 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VfpuOperation {
    BranchFalse,
    BranchFalseLikely,
    BranchTrue,
    BranchTrueLikely,
    Load,
    Store,
    StoreWriteBuffer,
    StoreLeft,
    StoreRight,
    MoveFromVector,
    MoveFromControl,
    MoveToVector,
    MoveToControl,
    AbsoluteValue,
    Add,
    ArcSine,
    Average,
    Butterfly1,
    Butterfly2,
    ConvertColorToInteger,
    ConditionalMoveFalse,
    ConditionalMoveTrue,
    Compare,
    Cosine,
    CrossProduct,
    CrossProductQuaternion,
    Constant,
    Determinant,
    Divide,
    DotProduct,
    Exponential2,
    FloatToHalf,
    FloatToIntegerDown,
    FloatToIntegerNearest,
    FloatToIntegerUp,
    FloatToIntegerTruncate,
    Sum,
    FloatImmediate,
    Flush,
    HalfToFloat,
    HomogeneousDotProduct,
    HomogeneousTransform2,
    HomogeneousTransform3,
    HomogeneousTransform4,
    IntegerToColor,
    IntegerToFloat,
    IntegerToShort,
    IntegerToUnsignedColor,
    IntegerToUnsignedShort,
    Identity,
    IntegerImmediate,
    LeadingBit,
    Logarithm2,
    Maximum,
    MoveFromVfpuControl,
    MatrixIdentity,
    Minimum,
    MatrixMove,
    MatrixMultiply,
    MatrixOne,
    Move,
    MatrixScale,
    MoveToVfpuControl,
    Multiply,
    MatrixZero,
    Negate,
    NoOperation,
    NegativeReciprocal,
    NegativeSine,
    OneComplement,
    One,
    DestinationPrefix,
    SourcePrefix,
    TargetPrefix,
    QuaternionMultiply,
    Reciprocal,
    ReciprocalExponential2,
    RandomFloat1,
    RandomFloat2,
    RandomInteger,
    RandomSeed,
    Rotate,
    ReciprocalSquareRoot,
    ShortToInteger,
    SaturateZeroOne,
    SaturateNegativeOneOne,
    ScaleByExponent,
    SignBitZero,
    Scale,
    CompareThreeWay,
    SetGreaterThanOrEqual,
    Sign,
    Sine,
    SetLessThan,
    OneComplementScaled,
    SquareRoot,
    Sort1,
    Sort2,
    Sort3,
    Sort4,
    Subtract,
    Synchronize,
    SynchronizeCpu,
    ConvertTo4444,
    ConvertTo5551,
    ConvertTo5650,
    Transform2,
    Transform3,
    Transform4,
    UnsignedColorToIntegerFloatScaled,
    UnsignedShortToInteger,
    WriteBreakNumber,
    Zero,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VfpuForm {
    operation: VfpuOperation,
    size: Option<VectorSize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum VectorComponent {
    X = 0,
    Y = 1,
    Z = 2,
    W = 3,
}

impl VectorComponent {
    pub(crate) const fn from_field(value: u8) -> Self {
        match value {
            0 => Self::X,
            1 => Self::Y,
            2 => Self::Z,
            3 => Self::W,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum SourcePrefixConstant {
    Zero = 8,
    One = 9,
    Two = 10,
    OneHalf = 11,
    Three = 12,
    OneThird = 13,
    OneQuarter = 14,
    OneSixth = 15,
}

impl SourcePrefixConstant {
    pub(crate) const fn from_field(value: u8) -> Self {
        match value {
            8 => Self::Zero,
            9 => Self::One,
            10 => Self::Two,
            11 => Self::OneHalf,
            12 => Self::Three,
            13 => Self::OneThird,
            14 => Self::OneQuarter,
            15 => Self::OneSixth,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourcePrefixValue {
    Component {
        component: VectorComponent,
        absolute: bool,
    },
    Constant(SourcePrefixConstant),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourcePrefixLane {
    pub value: SourcePrefixValue,
    pub negate: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum DestinationSaturation {
    None = 0,
    ZeroToOne = 1,
    NegativeOneToOne = 3,
}

impl DestinationSaturation {
    pub(crate) const fn from_field(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::ZeroToOne),
            3 => Some(Self::NegativeOneToOne),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DestinationPrefixLane {
    pub saturation: DestinationSaturation,
    pub mask: bool,
}

impl VfpuForm {
    pub fn new(operation: VfpuOperation, size: Option<VectorSize>) -> Option<Self> {
        let form = Self { operation, size };
        descriptor_for_form(form).map(|_| form)
    }

    pub const fn operation(self) -> VfpuOperation {
        self.operation
    }

    pub const fn size(self) -> Option<VectorSize> {
        self.size
    }

    pub fn mnemonic(self) -> &'static str {
        descriptor_for_form(self)
            .expect("constructed VFPU forms have descriptors")
            .mnemonic
    }

    pub fn all() -> impl Iterator<Item = Self> {
        VFPU_DESCRIPTORS.iter().map(|descriptor| descriptor.form)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VfpuOperands {
    None,
    Branch {
        condition: VfpuConditionBit,
        target: u32,
    },
    MemoryVector {
        vector: VectorRegister,
        base: GeneralRegister,
        offset: VfpuMemoryOffset,
    },
    GeneralAndVector {
        general: GeneralRegister,
        vector: VectorRegister,
    },
    GeneralAndControl {
        general: GeneralRegister,
        control: VfpuControlRegister,
    },
    ThreeVectors {
        destination: VectorRegister,
        left: VectorRegister,
        right: VectorRegister,
    },
    TwoVectors {
        destination: VectorRegister,
        source: VectorRegister,
    },
    ConditionalMove {
        destination: VectorRegister,
        source: VectorRegister,
        condition: VfpuConditionBit,
    },
    Compare {
        condition: VfpuCompareCondition,
        left: VectorRegister,
        right: VectorRegister,
    },
    Constant {
        destination: VectorRegister,
        constant: VfpuConstant,
    },
    ImmediateAndTwoVectors {
        destination: VectorRegister,
        source: VectorRegister,
        immediate: Immediate5,
    },
    VectorImmediate16 {
        destination: VectorRegister,
        immediate: Immediate16,
    },
    DestinationVector {
        destination: VectorRegister,
    },
    ControlAndVector {
        control: VfpuControlRegister,
        vector: VectorRegister,
    },
    SourcePrefix {
        lanes: [SourcePrefixLane; 4],
    },
    DestinationPrefix {
        lanes: [DestinationPrefixLane; 4],
    },
    SourceVector {
        source: VectorRegister,
    },
    ThreeMatrices {
        destination: MatrixRegister,
        left: MatrixRegister,
        right: MatrixRegister,
    },
    TwoMatrices {
        destination: MatrixRegister,
        source: MatrixRegister,
    },
    DestinationMatrix {
        destination: MatrixRegister,
    },
    MatrixScale {
        destination: MatrixRegister,
        source: MatrixRegister,
        scale: VectorRegister,
    },
    MatrixVectorTransform {
        destination: VectorRegister,
        matrix: MatrixRegister,
        vector: VectorRegister,
    },
    Immediate8AndTwoVectors {
        destination: VectorRegister,
        source: VectorRegister,
        immediate: Immediate8,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VfpuInstruction {
    form: VfpuForm,
    operands: VfpuOperands,
}

impl VfpuInstruction {
    pub fn new(form: VfpuForm, operands: VfpuOperands) -> Result<Self, VfpuConstructionError> {
        let descriptor = descriptor_for_form(form).ok_or(VfpuConstructionError::UnknownForm)?;
        if descriptor.layout != OperandLayout::of(&operands) {
            return Err(VfpuConstructionError::OperandShapeMismatch {
                mnemonic: descriptor.mnemonic,
            });
        }
        let written_control = match (form.operation(), &operands) {
            (VfpuOperation::MoveToControl, VfpuOperands::GeneralAndControl { control, .. })
            | (VfpuOperation::MoveToVfpuControl, VfpuOperands::ControlAndVector { control, .. }) => {
                Some(*control)
            }
            _ => None,
        };
        if written_control.is_some_and(|control| !control.is_writable()) {
            return Err(VfpuConstructionError::ReadOnlyControlRegister {
                register: VfpuControlRegister::Revision,
            });
        }
        Ok(Self { form, operands })
    }

    pub const fn form(&self) -> VfpuForm {
        self.form
    }

    pub const fn operands(&self) -> &VfpuOperands {
        &self.operands
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum VfpuConstructionError {
    #[error("unknown VFPU instruction form")]
    UnknownForm,
    #[error("operands do not match VFPU instruction {mnemonic}")]
    OperandShapeMismatch { mnemonic: &'static str },
    #[error("VFPU control register {register:?} is read-only")]
    ReadOnlyControlRegister { register: VfpuControlRegister },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OperandLayout {
    None,
    Branch,
    MemoryVector,
    GeneralAndVector,
    GeneralAndControl,
    ThreeVectors,
    TwoVectors,
    ConditionalMove,
    Compare,
    Constant,
    ImmediateAndTwoVectors,
    VectorImmediate16,
    DestinationVector,
    ControlAndVector,
    SourcePrefix,
    DestinationPrefix,
    SourceVector,
    ThreeMatrices,
    TwoMatrices,
    DestinationMatrix,
    MatrixScale,
    MatrixVectorTransform,
    Immediate8AndTwoVectors,
}

impl OperandLayout {
    const fn of(operands: &VfpuOperands) -> Self {
        match operands {
            VfpuOperands::None => Self::None,
            VfpuOperands::Branch { .. } => Self::Branch,
            VfpuOperands::MemoryVector { .. } => Self::MemoryVector,
            VfpuOperands::GeneralAndVector { .. } => Self::GeneralAndVector,
            VfpuOperands::GeneralAndControl { .. } => Self::GeneralAndControl,
            VfpuOperands::ThreeVectors { .. } => Self::ThreeVectors,
            VfpuOperands::TwoVectors { .. } => Self::TwoVectors,
            VfpuOperands::ConditionalMove { .. } => Self::ConditionalMove,
            VfpuOperands::Compare { .. } => Self::Compare,
            VfpuOperands::Constant { .. } => Self::Constant,
            VfpuOperands::ImmediateAndTwoVectors { .. } => Self::ImmediateAndTwoVectors,
            VfpuOperands::VectorImmediate16 { .. } => Self::VectorImmediate16,
            VfpuOperands::DestinationVector { .. } => Self::DestinationVector,
            VfpuOperands::ControlAndVector { .. } => Self::ControlAndVector,
            VfpuOperands::SourcePrefix { .. } => Self::SourcePrefix,
            VfpuOperands::DestinationPrefix { .. } => Self::DestinationPrefix,
            VfpuOperands::SourceVector { .. } => Self::SourceVector,
            VfpuOperands::ThreeMatrices { .. } => Self::ThreeMatrices,
            VfpuOperands::TwoMatrices { .. } => Self::TwoMatrices,
            VfpuOperands::DestinationMatrix { .. } => Self::DestinationMatrix,
            VfpuOperands::MatrixScale { .. } => Self::MatrixScale,
            VfpuOperands::MatrixVectorTransform { .. } => Self::MatrixVectorTransform,
            VfpuOperands::Immediate8AndTwoVectors { .. } => Self::Immediate8AndTwoVectors,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct VfpuDescriptor {
    pub form: VfpuForm,
    pub mnemonic: &'static str,
    pub mask: u32,
    pub value: u32,
    pub layout: OperandLayout,
}

pub(crate) fn descriptor_for_form(form: VfpuForm) -> Option<&'static VfpuDescriptor> {
    VFPU_DESCRIPTORS
        .iter()
        .find(|descriptor| descriptor.form == form)
}

macro_rules! descriptor {
    ($operation:ident, None, $mnemonic:literal, $mask:literal, $value:literal, $layout:ident) => {
        VfpuDescriptor {
            form: VfpuForm {
                operation: VfpuOperation::$operation,
                size: None,
            },
            mnemonic: $mnemonic,
            mask: $mask,
            value: $value,
            layout: OperandLayout::$layout,
        }
    };
    ($operation:ident, $size:ident, $mnemonic:literal, $mask:literal, $value:literal, $layout:ident) => {
        VfpuDescriptor {
            form: VfpuForm {
                operation: VfpuOperation::$operation,
                size: Some(VectorSize::$size),
            },
            mnemonic: $mnemonic,
            mask: $mask,
            value: $value,
            layout: OperandLayout::$layout,
        }
    };
}

pub(crate) static VFPU_DESCRIPTORS: &[VfpuDescriptor] = &[
    descriptor!(BranchFalse, None, "bvf", 0xffe30000, 0x49000000, Branch),
    descriptor!(
        BranchFalseLikely,
        None,
        "bvfl",
        0xffe30000,
        0x49020000,
        Branch
    ),
    descriptor!(BranchTrue, None, "bvt", 0xffe30000, 0x49010000, Branch),
    descriptor!(
        BranchTrueLikely,
        None,
        "bvtl",
        0xffe30000,
        0x49030000,
        Branch
    ),
    descriptor!(Load, Single, "lv.s", 0xfc000000, 0xc8000000, MemoryVector),
    descriptor!(Load, Quad, "lv.q", 0xfc000002, 0xd8000000, MemoryVector),
    descriptor!(
        MoveFromVector,
        None,
        "mfv",
        0xffe0ff80,
        0x48600000,
        GeneralAndVector
    ),
    descriptor!(
        MoveFromControl,
        None,
        "mfvc",
        0xffe0ff00,
        0x48600000,
        GeneralAndControl
    ),
    descriptor!(
        MoveToVector,
        None,
        "mtv",
        0xffe0ff80,
        0x48e00000,
        GeneralAndVector
    ),
    descriptor!(
        MoveToControl,
        None,
        "mtvc",
        0xffe0ff00,
        0x48e00000,
        GeneralAndControl
    ),
    descriptor!(Store, Single, "sv.s", 0xfc000000, 0xe8000000, MemoryVector),
    descriptor!(Store, Quad, "sv.q", 0xfc000002, 0xf8000000, MemoryVector),
    descriptor!(
        StoreWriteBuffer,
        Quad,
        "sv.q,wb",
        0xfc000002,
        0xf8000002,
        MemoryVector
    ),
    descriptor!(
        StoreLeft,
        Quad,
        "svl.q",
        0xfc000002,
        0xf4000000,
        MemoryVector
    ),
    descriptor!(
        StoreRight,
        Quad,
        "svr.q",
        0xfc000002,
        0xf4000002,
        MemoryVector
    ),
    descriptor!(
        AbsoluteValue,
        Single,
        "vabs.s",
        0xffff8080,
        0xd0010000,
        TwoVectors
    ),
    descriptor!(
        AbsoluteValue,
        Pair,
        "vabs.p",
        0xffff8080,
        0xd0010080,
        TwoVectors
    ),
    descriptor!(
        AbsoluteValue,
        Triple,
        "vabs.t",
        0xffff8080,
        0xd0018000,
        TwoVectors
    ),
    descriptor!(
        AbsoluteValue,
        Quad,
        "vabs.q",
        0xffff8080,
        0xd0018080,
        TwoVectors
    ),
    descriptor!(Add, Single, "vadd.s", 0xff808080, 0x60000000, ThreeVectors),
    descriptor!(Add, Pair, "vadd.p", 0xff808080, 0x60000080, ThreeVectors),
    descriptor!(Add, Triple, "vadd.t", 0xff808080, 0x60008000, ThreeVectors),
    descriptor!(Add, Quad, "vadd.q", 0xff808080, 0x60008080, ThreeVectors),
    descriptor!(
        ArcSine, Single, "vasin.s", 0xffff8080, 0xd0170000, TwoVectors
    ),
    descriptor!(ArcSine, Pair, "vasin.p", 0xffff8080, 0xd0170080, TwoVectors),
    descriptor!(
        ArcSine, Triple, "vasin.t", 0xffff8080, 0xd0178000, TwoVectors
    ),
    descriptor!(ArcSine, Quad, "vasin.q", 0xffff8080, 0xd0178080, TwoVectors),
    descriptor!(Average, Pair, "vavg.p", 0xffff8080, 0xd0470080, TwoVectors),
    descriptor!(
        Average, Triple, "vavg.t", 0xffff8080, 0xd0478000, TwoVectors
    ),
    descriptor!(Average, Quad, "vavg.q", 0xffff8080, 0xd0478080, TwoVectors),
    descriptor!(
        Butterfly1, Pair, "vbfy1.p", 0xffff8080, 0xd0420080, TwoVectors
    ),
    descriptor!(
        Butterfly1, Quad, "vbfy1.q", 0xffff8080, 0xd0428080, TwoVectors
    ),
    descriptor!(
        Butterfly2, Quad, "vbfy2.q", 0xffff8080, 0xd0438080, TwoVectors
    ),
    descriptor!(
        ConvertColorToInteger,
        Single,
        "vc2i.s",
        0xffff8080,
        0xd0390000,
        TwoVectors
    ),
    descriptor!(
        ConditionalMoveFalse,
        Single,
        "vcmovf.s",
        0xfff88080,
        0xd2a80000,
        ConditionalMove
    ),
    descriptor!(
        ConditionalMoveFalse,
        Pair,
        "vcmovf.p",
        0xfff88080,
        0xd2a80080,
        ConditionalMove
    ),
    descriptor!(
        ConditionalMoveFalse,
        Triple,
        "vcmovf.t",
        0xfff88080,
        0xd2a88000,
        ConditionalMove
    ),
    descriptor!(
        ConditionalMoveFalse,
        Quad,
        "vcmovf.q",
        0xfff88080,
        0xd2a88080,
        ConditionalMove
    ),
    descriptor!(
        ConditionalMoveTrue,
        Single,
        "vcmovt.s",
        0xfff88080,
        0xd2a00000,
        ConditionalMove
    ),
    descriptor!(
        ConditionalMoveTrue,
        Pair,
        "vcmovt.p",
        0xfff88080,
        0xd2a00080,
        ConditionalMove
    ),
    descriptor!(
        ConditionalMoveTrue,
        Triple,
        "vcmovt.t",
        0xfff88080,
        0xd2a08000,
        ConditionalMove
    ),
    descriptor!(
        ConditionalMoveTrue,
        Quad,
        "vcmovt.q",
        0xfff88080,
        0xd2a08080,
        ConditionalMove
    ),
    descriptor!(Compare, Single, "vcmp.s", 0xff8080f0, 0x6c000000, Compare),
    descriptor!(Compare, Pair, "vcmp.p", 0xff8080f0, 0x6c000080, Compare),
    descriptor!(Compare, Triple, "vcmp.t", 0xff8080f0, 0x6c008000, Compare),
    descriptor!(Compare, Quad, "vcmp.q", 0xff8080f0, 0x6c008080, Compare),
    descriptor!(Cosine, Single, "vcos.s", 0xffff8080, 0xd0130000, TwoVectors),
    descriptor!(Cosine, Pair, "vcos.p", 0xffff8080, 0xd0130080, TwoVectors),
    descriptor!(Cosine, Triple, "vcos.t", 0xffff8080, 0xd0138000, TwoVectors),
    descriptor!(Cosine, Quad, "vcos.q", 0xffff8080, 0xd0138080, TwoVectors),
    descriptor!(
        CrossProduct,
        Triple,
        "vcrs.t",
        0xff808080,
        0x66808000,
        ThreeVectors
    ),
    descriptor!(
        CrossProductQuaternion,
        Triple,
        "vcrsp.t",
        0xff808080,
        0xf2808000,
        ThreeVectors
    ),
    descriptor!(Constant, Single, "vcst.s", 0xffe0ff80, 0xd0600000, Constant),
    descriptor!(Constant, Pair, "vcst.p", 0xffe0ff80, 0xd0600080, Constant),
    descriptor!(Constant, Triple, "vcst.t", 0xffe0ff80, 0xd0608000, Constant),
    descriptor!(Constant, Quad, "vcst.q", 0xffe0ff80, 0xd0608080, Constant),
    descriptor!(
        Determinant,
        Pair,
        "vdet.p",
        0xff808080,
        0x67000080,
        ThreeVectors
    ),
    descriptor!(
        Divide,
        Single,
        "vdiv.s",
        0xff808080,
        0x63800000,
        ThreeVectors
    ),
    descriptor!(
        DotProduct,
        Pair,
        "vdot.p",
        0xff808080,
        0x64800080,
        ThreeVectors
    ),
    descriptor!(
        DotProduct,
        Triple,
        "vdot.t",
        0xff808080,
        0x64808000,
        ThreeVectors
    ),
    descriptor!(
        DotProduct,
        Quad,
        "vdot.q",
        0xff808080,
        0x64808080,
        ThreeVectors
    ),
    descriptor!(
        Exponential2,
        Single,
        "vexp2.s",
        0xffff8080,
        0xd0140000,
        TwoVectors
    ),
    descriptor!(
        Exponential2,
        Pair,
        "vexp2.p",
        0xffff8080,
        0xd0140080,
        TwoVectors
    ),
    descriptor!(
        Exponential2,
        Triple,
        "vexp2.t",
        0xffff8080,
        0xd0148000,
        TwoVectors
    ),
    descriptor!(
        Exponential2,
        Quad,
        "vexp2.q",
        0xffff8080,
        0xd0148080,
        TwoVectors
    ),
    descriptor!(
        FloatToHalf,
        Pair,
        "vf2h.p",
        0xffff8080,
        0xd0320080,
        TwoVectors
    ),
    descriptor!(
        FloatToHalf,
        Quad,
        "vf2h.q",
        0xffff8080,
        0xd0328080,
        TwoVectors
    ),
    descriptor!(
        FloatToIntegerDown,
        Single,
        "vf2id.s",
        0xffe08080,
        0xd2600000,
        ImmediateAndTwoVectors
    ),
    descriptor!(
        FloatToIntegerDown,
        Pair,
        "vf2id.p",
        0xffe08080,
        0xd2600080,
        ImmediateAndTwoVectors
    ),
    descriptor!(
        FloatToIntegerDown,
        Triple,
        "vf2id.t",
        0xffe08080,
        0xd2608000,
        ImmediateAndTwoVectors
    ),
    descriptor!(
        FloatToIntegerDown,
        Quad,
        "vf2id.q",
        0xffe08080,
        0xd2608080,
        ImmediateAndTwoVectors
    ),
    descriptor!(
        FloatToIntegerNearest,
        Single,
        "vf2in.s",
        0xffe08080,
        0xd2000000,
        ImmediateAndTwoVectors
    ),
    descriptor!(
        FloatToIntegerNearest,
        Pair,
        "vf2in.p",
        0xffe08080,
        0xd2000080,
        ImmediateAndTwoVectors
    ),
    descriptor!(
        FloatToIntegerNearest,
        Triple,
        "vf2in.t",
        0xffe08080,
        0xd2008000,
        ImmediateAndTwoVectors
    ),
    descriptor!(
        FloatToIntegerNearest,
        Quad,
        "vf2in.q",
        0xffe08080,
        0xd2008080,
        ImmediateAndTwoVectors
    ),
    descriptor!(
        FloatToIntegerUp,
        Single,
        "vf2iu.s",
        0xffe08080,
        0xd2400000,
        ImmediateAndTwoVectors
    ),
    descriptor!(
        FloatToIntegerUp,
        Pair,
        "vf2iu.p",
        0xffe08080,
        0xd2400080,
        ImmediateAndTwoVectors
    ),
    descriptor!(
        FloatToIntegerUp,
        Triple,
        "vf2iu.t",
        0xffe08080,
        0xd2408000,
        ImmediateAndTwoVectors
    ),
    descriptor!(
        FloatToIntegerUp,
        Quad,
        "vf2iu.q",
        0xffe08080,
        0xd2408080,
        ImmediateAndTwoVectors
    ),
    descriptor!(
        FloatToIntegerTruncate,
        Single,
        "vf2iz.s",
        0xffe08080,
        0xd2200000,
        ImmediateAndTwoVectors
    ),
    descriptor!(
        FloatToIntegerTruncate,
        Pair,
        "vf2iz.p",
        0xffe08080,
        0xd2200080,
        ImmediateAndTwoVectors
    ),
    descriptor!(
        FloatToIntegerTruncate,
        Triple,
        "vf2iz.t",
        0xffe08080,
        0xd2208000,
        ImmediateAndTwoVectors
    ),
    descriptor!(
        FloatToIntegerTruncate,
        Quad,
        "vf2iz.q",
        0xffe08080,
        0xd2208080,
        ImmediateAndTwoVectors
    ),
    descriptor!(Sum, Pair, "vfad.p", 0xffff8080, 0xd0460080, TwoVectors),
    descriptor!(Sum, Triple, "vfad.t", 0xffff8080, 0xd0468000, TwoVectors),
    descriptor!(Sum, Quad, "vfad.q", 0xffff8080, 0xd0468080, TwoVectors),
    descriptor!(
        FloatImmediate,
        Single,
        "vfim.s",
        0xff800000,
        0xdf800000,
        VectorImmediate16
    ),
    descriptor!(Flush, None, "vflush", 0xffffffff, 0xffff040d, None),
    descriptor!(
        HalfToFloat,
        Single,
        "vh2f.s",
        0xffff8080,
        0xd0330000,
        TwoVectors
    ),
    descriptor!(
        HalfToFloat,
        Pair,
        "vh2f.p",
        0xffff8080,
        0xd0330080,
        TwoVectors
    ),
    descriptor!(
        HomogeneousDotProduct,
        Pair,
        "vhdp.p",
        0xff808080,
        0x66000080,
        ThreeVectors
    ),
    descriptor!(
        HomogeneousDotProduct,
        Triple,
        "vhdp.t",
        0xff808080,
        0x66008000,
        ThreeVectors
    ),
    descriptor!(
        HomogeneousDotProduct,
        Quad,
        "vhdp.q",
        0xff808080,
        0x66008080,
        ThreeVectors
    ),
    descriptor!(
        HomogeneousTransform2,
        Pair,
        "vhtfm2.p",
        0xff808080,
        0xf0800000,
        MatrixVectorTransform
    ),
    descriptor!(
        HomogeneousTransform3,
        Triple,
        "vhtfm3.t",
        0xff808080,
        0xf1000080,
        MatrixVectorTransform
    ),
    descriptor!(
        HomogeneousTransform4,
        Quad,
        "vhtfm4.q",
        0xff808080,
        0xf1808000,
        MatrixVectorTransform
    ),
    descriptor!(
        IntegerToColor,
        Quad,
        "vi2c.q",
        0xffff8080,
        0xd03d8080,
        TwoVectors
    ),
    descriptor!(
        IntegerToFloat,
        Single,
        "vi2f.s",
        0xffe08080,
        0xd2800000,
        ImmediateAndTwoVectors
    ),
    descriptor!(
        IntegerToFloat,
        Pair,
        "vi2f.p",
        0xffe08080,
        0xd2800080,
        ImmediateAndTwoVectors
    ),
    descriptor!(
        IntegerToFloat,
        Triple,
        "vi2f.t",
        0xffe08080,
        0xd2808000,
        ImmediateAndTwoVectors
    ),
    descriptor!(
        IntegerToFloat,
        Quad,
        "vi2f.q",
        0xffe08080,
        0xd2808080,
        ImmediateAndTwoVectors
    ),
    descriptor!(
        IntegerToShort,
        Pair,
        "vi2s.p",
        0xffff8080,
        0xd03f0080,
        TwoVectors
    ),
    descriptor!(
        IntegerToShort,
        Quad,
        "vi2s.q",
        0xffff8080,
        0xd03f8080,
        TwoVectors
    ),
    descriptor!(
        IntegerToUnsignedColor,
        Quad,
        "vi2uc.q",
        0xffff8080,
        0xd03c8080,
        TwoVectors
    ),
    descriptor!(
        IntegerToUnsignedShort,
        Pair,
        "vi2us.p",
        0xffff8080,
        0xd03e0080,
        TwoVectors
    ),
    descriptor!(
        IntegerToUnsignedShort,
        Quad,
        "vi2us.q",
        0xffff8080,
        0xd03e8080,
        TwoVectors
    ),
    descriptor!(
        Identity,
        Pair,
        "vidt.p",
        0xffffff80,
        0xd0030080,
        DestinationVector
    ),
    descriptor!(
        Identity,
        Quad,
        "vidt.q",
        0xffffff80,
        0xd0038080,
        DestinationVector
    ),
    descriptor!(
        IntegerImmediate,
        Single,
        "viim.s",
        0xff800000,
        0xdf000000,
        VectorImmediate16
    ),
    descriptor!(
        LeadingBit, Single, "vlgb.s", 0xffff8080, 0xd0370000, TwoVectors
    ),
    descriptor!(
        Logarithm2, Single, "vlog2.s", 0xffff8080, 0xd0150000, TwoVectors
    ),
    descriptor!(
        Logarithm2, Pair, "vlog2.p", 0xffff8080, 0xd0150080, TwoVectors
    ),
    descriptor!(
        Logarithm2, Triple, "vlog2.t", 0xffff8080, 0xd0158000, TwoVectors
    ),
    descriptor!(
        Logarithm2, Quad, "vlog2.q", 0xffff8080, 0xd0158080, TwoVectors
    ),
    descriptor!(
        Maximum,
        Single,
        "vmax.s",
        0xff808080,
        0x6d800000,
        ThreeVectors
    ),
    descriptor!(
        Maximum,
        Pair,
        "vmax.p",
        0xff808080,
        0x6d800080,
        ThreeVectors
    ),
    descriptor!(
        Maximum,
        Triple,
        "vmax.t",
        0xff808080,
        0x6d808000,
        ThreeVectors
    ),
    descriptor!(
        Maximum,
        Quad,
        "vmax.q",
        0xff808080,
        0x6d808080,
        ThreeVectors
    ),
    descriptor!(
        MoveFromVfpuControl,
        None,
        "vmfvc",
        0xffff0080,
        0xd0500000,
        ControlAndVector
    ),
    descriptor!(
        MatrixIdentity,
        Pair,
        "vmidt.p",
        0xffffff80,
        0xf3830080,
        DestinationMatrix
    ),
    descriptor!(
        MatrixIdentity,
        Triple,
        "vmidt.t",
        0xffffff80,
        0xf3838000,
        DestinationMatrix
    ),
    descriptor!(
        MatrixIdentity,
        Quad,
        "vmidt.q",
        0xffffff80,
        0xf3838080,
        DestinationMatrix
    ),
    descriptor!(
        Minimum,
        Single,
        "vmin.s",
        0xff808080,
        0x6d000000,
        ThreeVectors
    ),
    descriptor!(
        Minimum,
        Pair,
        "vmin.p",
        0xff808080,
        0x6d000080,
        ThreeVectors
    ),
    descriptor!(
        Minimum,
        Triple,
        "vmin.t",
        0xff808080,
        0x6d008000,
        ThreeVectors
    ),
    descriptor!(
        Minimum,
        Quad,
        "vmin.q",
        0xff808080,
        0x6d008080,
        ThreeVectors
    ),
    descriptor!(
        MatrixMove,
        Pair,
        "vmmov.p",
        0xffff8080,
        0xf3800080,
        TwoMatrices
    ),
    descriptor!(
        MatrixMove,
        Triple,
        "vmmov.t",
        0xffff8080,
        0xf3808000,
        TwoMatrices
    ),
    descriptor!(
        MatrixMove,
        Quad,
        "vmmov.q",
        0xffff8080,
        0xf3808080,
        TwoMatrices
    ),
    descriptor!(
        MatrixMultiply,
        Pair,
        "vmmul.p",
        0xff808080,
        0xf0000080,
        ThreeMatrices
    ),
    descriptor!(
        MatrixMultiply,
        Triple,
        "vmmul.t",
        0xff808080,
        0xf0008000,
        ThreeMatrices
    ),
    descriptor!(
        MatrixMultiply,
        Quad,
        "vmmul.q",
        0xff808080,
        0xf0008080,
        ThreeMatrices
    ),
    descriptor!(
        MatrixOne,
        Pair,
        "vmone.p",
        0xffffff80,
        0xf3870080,
        DestinationMatrix
    ),
    descriptor!(
        MatrixOne,
        Triple,
        "vmone.t",
        0xffffff80,
        0xf3878000,
        DestinationMatrix
    ),
    descriptor!(
        MatrixOne,
        Quad,
        "vmone.q",
        0xffffff80,
        0xf3878080,
        DestinationMatrix
    ),
    descriptor!(Move, Single, "vmov.s", 0xffff8080, 0xd0000000, TwoVectors),
    descriptor!(Move, Pair, "vmov.p", 0xffff8080, 0xd0000080, TwoVectors),
    descriptor!(Move, Triple, "vmov.t", 0xffff8080, 0xd0008000, TwoVectors),
    descriptor!(Move, Quad, "vmov.q", 0xffff8080, 0xd0008080, TwoVectors),
    descriptor!(
        MatrixScale,
        Pair,
        "vmscl.p",
        0xff808080,
        0xf2000080,
        MatrixScale
    ),
    descriptor!(
        MatrixScale,
        Triple,
        "vmscl.t",
        0xff808080,
        0xf2008000,
        MatrixScale
    ),
    descriptor!(
        MatrixScale,
        Quad,
        "vmscl.q",
        0xff808080,
        0xf2008080,
        MatrixScale
    ),
    descriptor!(
        MoveToVfpuControl,
        None,
        "vmtvc",
        0xffff8000,
        0xd0510000,
        ControlAndVector
    ),
    descriptor!(
        Multiply,
        Single,
        "vmul.s",
        0xff808080,
        0x64000000,
        ThreeVectors
    ),
    descriptor!(
        Multiply,
        Pair,
        "vmul.p",
        0xff808080,
        0x64000080,
        ThreeVectors
    ),
    descriptor!(
        Multiply,
        Triple,
        "vmul.t",
        0xff808080,
        0x64008000,
        ThreeVectors
    ),
    descriptor!(
        Multiply,
        Quad,
        "vmul.q",
        0xff808080,
        0x64008080,
        ThreeVectors
    ),
    descriptor!(
        MatrixZero,
        Pair,
        "vmzero.p",
        0xffffff80,
        0xf3860080,
        DestinationMatrix
    ),
    descriptor!(
        MatrixZero,
        Triple,
        "vmzero.t",
        0xffffff80,
        0xf3868000,
        DestinationMatrix
    ),
    descriptor!(
        MatrixZero,
        Quad,
        "vmzero.q",
        0xffffff80,
        0xf3868080,
        DestinationMatrix
    ),
    descriptor!(Negate, Single, "vneg.s", 0xffff8080, 0xd0020000, TwoVectors),
    descriptor!(Negate, Pair, "vneg.p", 0xffff8080, 0xd0020080, TwoVectors),
    descriptor!(Negate, Triple, "vneg.t", 0xffff8080, 0xd0028000, TwoVectors),
    descriptor!(Negate, Quad, "vneg.q", 0xffff8080, 0xd0028080, TwoVectors),
    descriptor!(NoOperation, None, "vnop", 0xffffffff, 0xffff0000, None),
    descriptor!(
        NegativeReciprocal,
        Single,
        "vnrcp.s",
        0xffff8080,
        0xd0180000,
        TwoVectors
    ),
    descriptor!(
        NegativeReciprocal,
        Pair,
        "vnrcp.p",
        0xffff8080,
        0xd0180080,
        TwoVectors
    ),
    descriptor!(
        NegativeReciprocal,
        Triple,
        "vnrcp.t",
        0xffff8080,
        0xd0188000,
        TwoVectors
    ),
    descriptor!(
        NegativeReciprocal,
        Quad,
        "vnrcp.q",
        0xffff8080,
        0xd0188080,
        TwoVectors
    ),
    descriptor!(
        NegativeSine,
        Single,
        "vnsin.s",
        0xffff8080,
        0xd01a0000,
        TwoVectors
    ),
    descriptor!(
        NegativeSine,
        Pair,
        "vnsin.p",
        0xffff8080,
        0xd01a0080,
        TwoVectors
    ),
    descriptor!(
        NegativeSine,
        Triple,
        "vnsin.t",
        0xffff8080,
        0xd01a8000,
        TwoVectors
    ),
    descriptor!(
        NegativeSine,
        Quad,
        "vnsin.q",
        0xffff8080,
        0xd01a8080,
        TwoVectors
    ),
    descriptor!(
        OneComplement,
        Single,
        "vocp.s",
        0xffff8080,
        0xd0440000,
        TwoVectors
    ),
    descriptor!(
        OneComplement,
        Pair,
        "vocp.p",
        0xffff8080,
        0xd0440080,
        TwoVectors
    ),
    descriptor!(
        OneComplement,
        Triple,
        "vocp.t",
        0xffff8080,
        0xd0448000,
        TwoVectors
    ),
    descriptor!(
        OneComplement,
        Quad,
        "vocp.q",
        0xffff8080,
        0xd0448080,
        TwoVectors
    ),
    descriptor!(
        One,
        Single,
        "vone.s",
        0xffffff80,
        0xd0070000,
        DestinationVector
    ),
    descriptor!(
        One,
        Pair,
        "vone.p",
        0xffffff80,
        0xd0070080,
        DestinationVector
    ),
    descriptor!(
        One,
        Triple,
        "vone.t",
        0xffffff80,
        0xd0078000,
        DestinationVector
    ),
    descriptor!(
        One,
        Quad,
        "vone.q",
        0xffffff80,
        0xd0078080,
        DestinationVector
    ),
    descriptor!(
        DestinationPrefix,
        None,
        "vpfxd",
        0xff000000,
        0xde000000,
        DestinationPrefix
    ),
    descriptor!(
        SourcePrefix,
        None,
        "vpfxs",
        0xff000000,
        0xdc000000,
        SourcePrefix
    ),
    descriptor!(
        TargetPrefix,
        None,
        "vpfxt",
        0xff000000,
        0xdd000000,
        SourcePrefix
    ),
    descriptor!(
        QuaternionMultiply,
        Quad,
        "vqmul.q",
        0xff808080,
        0xf2808080,
        ThreeVectors
    ),
    descriptor!(
        Reciprocal, Single, "vrcp.s", 0xffff8080, 0xd0100000, TwoVectors
    ),
    descriptor!(
        Reciprocal, Pair, "vrcp.p", 0xffff8080, 0xd0100080, TwoVectors
    ),
    descriptor!(
        Reciprocal, Triple, "vrcp.t", 0xffff8080, 0xd0108000, TwoVectors
    ),
    descriptor!(
        Reciprocal, Quad, "vrcp.q", 0xffff8080, 0xd0108080, TwoVectors
    ),
    descriptor!(
        ReciprocalExponential2,
        Single,
        "vrexp2.s",
        0xffff8080,
        0xd01c0000,
        TwoVectors
    ),
    descriptor!(
        ReciprocalExponential2,
        Pair,
        "vrexp2.p",
        0xffff8080,
        0xd01c0080,
        TwoVectors
    ),
    descriptor!(
        ReciprocalExponential2,
        Triple,
        "vrexp2.t",
        0xffff8080,
        0xd01c8000,
        TwoVectors
    ),
    descriptor!(
        ReciprocalExponential2,
        Quad,
        "vrexp2.q",
        0xffff8080,
        0xd01c8080,
        TwoVectors
    ),
    descriptor!(
        RandomFloat1,
        Single,
        "vrndf1.s",
        0xffffff80,
        0xd0220000,
        DestinationVector
    ),
    descriptor!(
        RandomFloat1,
        Pair,
        "vrndf1.p",
        0xffffff80,
        0xd0220080,
        DestinationVector
    ),
    descriptor!(
        RandomFloat1,
        Triple,
        "vrndf1.t",
        0xffffff80,
        0xd0228000,
        DestinationVector
    ),
    descriptor!(
        RandomFloat1,
        Quad,
        "vrndf1.q",
        0xffffff80,
        0xd0228080,
        DestinationVector
    ),
    descriptor!(
        RandomFloat2,
        Single,
        "vrndf2.s",
        0xffffff80,
        0xd0230000,
        DestinationVector
    ),
    descriptor!(
        RandomFloat2,
        Pair,
        "vrndf2.p",
        0xffffff80,
        0xd0230080,
        DestinationVector
    ),
    descriptor!(
        RandomFloat2,
        Triple,
        "vrndf2.t",
        0xffffff80,
        0xd0238000,
        DestinationVector
    ),
    descriptor!(
        RandomFloat2,
        Quad,
        "vrndf2.q",
        0xffffff80,
        0xd0238080,
        DestinationVector
    ),
    descriptor!(
        RandomInteger,
        Single,
        "vrndi.s",
        0xffffff80,
        0xd0210000,
        DestinationVector
    ),
    descriptor!(
        RandomInteger,
        Pair,
        "vrndi.p",
        0xffffff80,
        0xd0210080,
        DestinationVector
    ),
    descriptor!(
        RandomInteger,
        Triple,
        "vrndi.t",
        0xffffff80,
        0xd0218000,
        DestinationVector
    ),
    descriptor!(
        RandomInteger,
        Quad,
        "vrndi.q",
        0xffffff80,
        0xd0218080,
        DestinationVector
    ),
    descriptor!(
        RandomSeed,
        Single,
        "vrnds.s",
        0xffff80ff,
        0xd0200000,
        SourceVector
    ),
    descriptor!(
        Rotate,
        Pair,
        "vrot.p",
        0xffe08080,
        0xf3a00080,
        ImmediateAndTwoVectors
    ),
    descriptor!(
        Rotate,
        Triple,
        "vrot.t",
        0xffe08080,
        0xf3a08000,
        ImmediateAndTwoVectors
    ),
    descriptor!(
        Rotate,
        Quad,
        "vrot.q",
        0xffe08080,
        0xf3a08080,
        ImmediateAndTwoVectors
    ),
    descriptor!(
        ReciprocalSquareRoot,
        Single,
        "vrsq.s",
        0xffff8080,
        0xd0110000,
        TwoVectors
    ),
    descriptor!(
        ReciprocalSquareRoot,
        Pair,
        "vrsq.p",
        0xffff8080,
        0xd0110080,
        TwoVectors
    ),
    descriptor!(
        ReciprocalSquareRoot,
        Triple,
        "vrsq.t",
        0xffff8080,
        0xd0118000,
        TwoVectors
    ),
    descriptor!(
        ReciprocalSquareRoot,
        Quad,
        "vrsq.q",
        0xffff8080,
        0xd0118080,
        TwoVectors
    ),
    descriptor!(
        ShortToInteger,
        Single,
        "vs2i.s",
        0xffff8080,
        0xd03b0000,
        TwoVectors
    ),
    descriptor!(
        ShortToInteger,
        Pair,
        "vs2i.p",
        0xffff8080,
        0xd03b0080,
        TwoVectors
    ),
    descriptor!(
        SaturateZeroOne,
        Single,
        "vsat0.s",
        0xffff8080,
        0xd0040000,
        TwoVectors
    ),
    descriptor!(
        SaturateZeroOne,
        Pair,
        "vsat0.p",
        0xffff8080,
        0xd0040080,
        TwoVectors
    ),
    descriptor!(
        SaturateZeroOne,
        Triple,
        "vsat0.t",
        0xffff8080,
        0xd0048000,
        TwoVectors
    ),
    descriptor!(
        SaturateZeroOne,
        Quad,
        "vsat0.q",
        0xffff8080,
        0xd0048080,
        TwoVectors
    ),
    descriptor!(
        SaturateNegativeOneOne,
        Single,
        "vsat1.s",
        0xffff8080,
        0xd0050000,
        TwoVectors
    ),
    descriptor!(
        SaturateNegativeOneOne,
        Pair,
        "vsat1.p",
        0xffff8080,
        0xd0050080,
        TwoVectors
    ),
    descriptor!(
        SaturateNegativeOneOne,
        Triple,
        "vsat1.t",
        0xffff8080,
        0xd0058000,
        TwoVectors
    ),
    descriptor!(
        SaturateNegativeOneOne,
        Quad,
        "vsat1.q",
        0xffff8080,
        0xd0058080,
        TwoVectors
    ),
    descriptor!(
        ScaleByExponent,
        Single,
        "vsbn.s",
        0xff808080,
        0x61000000,
        ThreeVectors
    ),
    descriptor!(
        SignBitZero,
        Single,
        "vsbz.s",
        0xffff8080,
        0xd0360000,
        TwoVectors
    ),
    descriptor!(Scale, Pair, "vscl.p", 0xff808080, 0x65000080, ThreeVectors),
    descriptor!(
        Scale,
        Triple,
        "vscl.t",
        0xff808080,
        0x65008000,
        ThreeVectors
    ),
    descriptor!(Scale, Quad, "vscl.q", 0xff808080, 0x65008080, ThreeVectors),
    descriptor!(
        CompareThreeWay,
        Single,
        "vscmp.s",
        0xff808080,
        0x6e800000,
        ThreeVectors
    ),
    descriptor!(
        CompareThreeWay,
        Pair,
        "vscmp.p",
        0xff808080,
        0x6e800080,
        ThreeVectors
    ),
    descriptor!(
        CompareThreeWay,
        Triple,
        "vscmp.t",
        0xff808080,
        0x6e808000,
        ThreeVectors
    ),
    descriptor!(
        CompareThreeWay,
        Quad,
        "vscmp.q",
        0xff808080,
        0x6e808080,
        ThreeVectors
    ),
    descriptor!(
        SetGreaterThanOrEqual,
        Single,
        "vsge.s",
        0xff808080,
        0x6f000000,
        ThreeVectors
    ),
    descriptor!(
        SetGreaterThanOrEqual,
        Pair,
        "vsge.p",
        0xff808080,
        0x6f000080,
        ThreeVectors
    ),
    descriptor!(
        SetGreaterThanOrEqual,
        Triple,
        "vsge.t",
        0xff808080,
        0x6f008000,
        ThreeVectors
    ),
    descriptor!(
        SetGreaterThanOrEqual,
        Quad,
        "vsge.q",
        0xff808080,
        0x6f008080,
        ThreeVectors
    ),
    descriptor!(Sign, Single, "vsgn.s", 0xffff8080, 0xd04a0000, TwoVectors),
    descriptor!(Sign, Pair, "vsgn.p", 0xffff8080, 0xd04a0080, TwoVectors),
    descriptor!(Sign, Triple, "vsgn.t", 0xffff8080, 0xd04a8000, TwoVectors),
    descriptor!(Sign, Quad, "vsgn.q", 0xffff8080, 0xd04a8080, TwoVectors),
    descriptor!(Sine, Single, "vsin.s", 0xffff8080, 0xd0120000, TwoVectors),
    descriptor!(Sine, Pair, "vsin.p", 0xffff8080, 0xd0120080, TwoVectors),
    descriptor!(Sine, Triple, "vsin.t", 0xffff8080, 0xd0128000, TwoVectors),
    descriptor!(Sine, Quad, "vsin.q", 0xffff8080, 0xd0128080, TwoVectors),
    descriptor!(
        SetLessThan,
        Single,
        "vslt.s",
        0xff808080,
        0x6f800000,
        ThreeVectors
    ),
    descriptor!(
        SetLessThan,
        Pair,
        "vslt.p",
        0xff808080,
        0x6f800080,
        ThreeVectors
    ),
    descriptor!(
        SetLessThan,
        Triple,
        "vslt.t",
        0xff808080,
        0x6f808000,
        ThreeVectors
    ),
    descriptor!(
        SetLessThan,
        Quad,
        "vslt.q",
        0xff808080,
        0x6f808080,
        ThreeVectors
    ),
    descriptor!(
        OneComplementScaled,
        Single,
        "vsocp.s",
        0xffff8080,
        0xd0450000,
        TwoVectors
    ),
    descriptor!(
        OneComplementScaled,
        Pair,
        "vsocp.p",
        0xffff8080,
        0xd0450080,
        TwoVectors
    ),
    descriptor!(
        SquareRoot, Single, "vsqrt.s", 0xffff8080, 0xd0160000, TwoVectors
    ),
    descriptor!(
        SquareRoot, Pair, "vsqrt.p", 0xffff8080, 0xd0160080, TwoVectors
    ),
    descriptor!(
        SquareRoot, Triple, "vsqrt.t", 0xffff8080, 0xd0168000, TwoVectors
    ),
    descriptor!(
        SquareRoot, Quad, "vsqrt.q", 0xffff8080, 0xd0168080, TwoVectors
    ),
    descriptor!(Sort1, Quad, "vsrt1.q", 0xffff8080, 0xd0408080, TwoVectors),
    descriptor!(Sort2, Quad, "vsrt2.q", 0xffff8080, 0xd0418080, TwoVectors),
    descriptor!(Sort3, Quad, "vsrt3.q", 0xffff8080, 0xd0488080, TwoVectors),
    descriptor!(Sort4, Quad, "vsrt4.q", 0xffff8080, 0xd0498080, TwoVectors),
    descriptor!(
        Subtract,
        Single,
        "vsub.s",
        0xff808080,
        0x60800000,
        ThreeVectors
    ),
    descriptor!(
        Subtract,
        Pair,
        "vsub.p",
        0xff808080,
        0x60800080,
        ThreeVectors
    ),
    descriptor!(
        Subtract,
        Triple,
        "vsub.t",
        0xff808080,
        0x60808000,
        ThreeVectors
    ),
    descriptor!(
        Subtract,
        Quad,
        "vsub.q",
        0xff808080,
        0x60808080,
        ThreeVectors
    ),
    descriptor!(Synchronize, None, "vsync", 0xffffffff, 0xffff0320, None),
    descriptor!(SynchronizeCpu, None, "vsync2", 0xffffffff, 0x486000ff, None),
    descriptor!(
        ConvertTo4444,
        Quad,
        "vt4444.q",
        0xffff8080,
        0xd0598080,
        TwoVectors
    ),
    descriptor!(
        ConvertTo5551,
        Quad,
        "vt5551.q",
        0xffff8080,
        0xd05a8080,
        TwoVectors
    ),
    descriptor!(
        ConvertTo5650,
        Quad,
        "vt5650.q",
        0xffff8080,
        0xd05b8080,
        TwoVectors
    ),
    descriptor!(
        Transform2,
        Pair,
        "vtfm2.p",
        0xff808080,
        0xf0800080,
        MatrixVectorTransform
    ),
    descriptor!(
        Transform3,
        Triple,
        "vtfm3.t",
        0xff808080,
        0xf1008000,
        MatrixVectorTransform
    ),
    descriptor!(
        Transform4,
        Quad,
        "vtfm4.q",
        0xff808080,
        0xf1808080,
        MatrixVectorTransform
    ),
    descriptor!(
        UnsignedColorToIntegerFloatScaled,
        Single,
        "vuc2ifs.s",
        0xffff8080,
        0xd0380000,
        TwoVectors
    ),
    descriptor!(
        UnsignedShortToInteger,
        Single,
        "vus2i.s",
        0xffff8080,
        0xd03a0000,
        TwoVectors
    ),
    descriptor!(
        UnsignedShortToInteger,
        Pair,
        "vus2i.p",
        0xffff8080,
        0xd03a0080,
        TwoVectors
    ),
    descriptor!(
        WriteBreakNumber,
        Single,
        "vwbn.s",
        0xff008080,
        0xd3000000,
        Immediate8AndTwoVectors
    ),
    descriptor!(
        Zero,
        Single,
        "vzero.s",
        0xffffff80,
        0xd0060000,
        DestinationVector
    ),
    descriptor!(
        Zero,
        Pair,
        "vzero.p",
        0xffffff80,
        0xd0060080,
        DestinationVector
    ),
    descriptor!(
        Zero,
        Triple,
        "vzero.t",
        0xffffff80,
        0xd0068000,
        DestinationVector
    ),
    descriptor!(
        Zero,
        Quad,
        "vzero.q",
        0xffffff80,
        0xd0068080,
        DestinationVector
    ),
];
