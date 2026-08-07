use crate::InstructionForm;

/// One of the 32 128-bit EE Core general-purpose registers.
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

/// One of the 32 EE Core floating-point registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FloatingRegister(u8);

impl FloatingRegister {
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

/// One of the 32 128-bit VU0 floating-point registers exposed through COP2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VectorFloatingRegister(u8);

impl VectorFloatingRegister {
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

/// One of the 16 VU0 integer registers used by macro calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VectorIntegerRegister(u8);

impl VectorIntegerRegister {
    pub const fn new(index: u8) -> Option<Self> {
        if index < 16 { Some(Self(index)) } else { None }
    }

    pub const fn index(self) -> u8 {
        self.0
    }

    pub(crate) const fn from_field(index: u8) -> Option<Self> {
        if index < 16 { Some(Self(index)) } else { None }
    }
}

/// A non-reserved VU0 register addressable through CFC2 or CTC2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VectorControlRegister(u8);

impl VectorControlRegister {
    pub const STATUS: Self = Self(16);
    pub const MAC: Self = Self(17);
    pub const CLIPPING: Self = Self(18);
    pub const RANDOM: Self = Self(20);
    pub const IMMEDIATE: Self = Self(21);
    pub const QUOTIENT: Self = Self(22);
    pub const TASK_PROGRAM_COUNTER: Self = Self(26);
    pub const CALL_ADDRESS_ZERO: Self = Self(27);
    pub const FORCE_BREAK_RESET: Self = Self(28);
    pub const VPU_STATUS: Self = Self(29);
    pub const CALL_ADDRESS_ONE: Self = Self(31);

    pub const fn new(index: u8) -> Option<Self> {
        match index {
            0..=18 | 20..=22 | 26..=29 | 31 => Some(Self(index)),
            _ => None,
        }
    }

    pub const fn index(self) -> u8 {
        self.0
    }
}

/// The VU fields selected by a macro calculation's `dest` bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VectorDestinationMask(u8);

impl VectorDestinationMask {
    pub const NONE: Self = Self(0);
    pub const XYZW: Self = Self(0x0f);
    pub const XYZ: Self = Self(0x0e);

    pub const fn new(bits: u8) -> Option<Self> {
        if bits <= 0x0f { Some(Self(bits)) } else { None }
    }

    pub const fn bits(self) -> u8 {
        self.0
    }

    pub(crate) const fn from_field(bits: u8) -> Self {
        debug_assert!(bits <= 0x0f);
        Self(bits)
    }
}

/// One x, y, z, or w field of a VU floating-point register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VectorLane {
    X,
    Y,
    Z,
    W,
}

impl VectorLane {
    pub const fn field(self) -> u8 {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
            Self::W => 3,
        }
    }

    pub const fn destination_bits(self) -> u8 {
        match self {
            Self::X => 0b1000,
            Self::Y => 0b0100,
            Self::Z => 0b0010,
            Self::W => 0b0001,
        }
    }

    pub(crate) const fn from_field(field: u8) -> Self {
        match field & 3 {
            0 => Self::X,
            1 => Self::Y,
            2 => Self::Z,
            _ => Self::W,
        }
    }

    pub(crate) const fn from_destination_bits(bits: u8) -> Option<Self> {
        match bits {
            0b1000 => Some(Self::X),
            0b0100 => Some(Self::Y),
            0b0010 => Some(Self::Z),
            0b0001 => Some(Self::W),
            _ => None,
        }
    }
}

/// Whether a COP2 transfer waits for a preceding VCALLMS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VectorInterlock {
    Without,
    With,
}

impl VectorInterlock {
    pub const fn bit(self) -> u8 {
        match self {
            Self::Without => 0,
            Self::With => 1,
        }
    }

    pub(crate) const fn from_bit(bit: u8) -> Self {
        if bit & 1 == 0 {
            Self::Without
        } else {
            Self::With
        }
    }
}

/// The signed five-bit immediate accepted by VIADDI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VectorImmediate5(i8);

impl VectorImmediate5 {
    pub const fn new(value: i8) -> Option<Self> {
        if value >= -16 && value <= 15 {
            Some(Self(value))
        } else {
            None
        }
    }

    pub const fn value(self) -> i8 {
        self.0
    }

    pub(crate) const fn field(self) -> u8 {
        (self.0 as u8) & 0x1f
    }

    pub(crate) const fn from_field(field: u8) -> Self {
        Self(((field & 0x1f) << 3) as i8 >> 3)
    }
}

/// A byte address in VU0 MicroMem accepted by VCALLMS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VectorMicroAddress(u32);

impl VectorMicroAddress {
    pub const fn new(byte_address: u32) -> Option<Self> {
        if byte_address <= 0x0003_fff8 && byte_address & 7 == 0 {
            Some(Self(byte_address))
        } else {
            None
        }
    }

    pub const fn byte_address(self) -> u32 {
        self.0
    }

    pub(crate) const fn field(self) -> u16 {
        (self.0 / 8) as u16
    }

    pub(crate) const fn from_field(field: u16) -> Self {
        debug_assert!(field <= 0x7fff);
        Self(field as u32 * 8)
    }
}

/// A COP0 register implemented by the EE Core User's Manual.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum SystemControlRegister {
    Index = 0,
    Random = 1,
    EntryLow0 = 2,
    EntryLow1 = 3,
    Context = 4,
    PageMask = 5,
    Wired = 6,
    BadVirtualAddress = 8,
    Count = 9,
    EntryHigh = 10,
    Compare = 11,
    Status = 12,
    Cause = 13,
    ExceptionProgramCounter = 14,
    ProcessorRevision = 15,
    Config = 16,
    BadPhysicalAddress = 23,
    Debug = 24,
    Performance = 25,
    TagLow = 28,
    TagHigh = 29,
    ErrorExceptionProgramCounter = 30,
}

impl SystemControlRegister {
    pub const fn new(index: u8) -> Option<Self> {
        Some(match index {
            0 => Self::Index,
            1 => Self::Random,
            2 => Self::EntryLow0,
            3 => Self::EntryLow1,
            4 => Self::Context,
            5 => Self::PageMask,
            6 => Self::Wired,
            8 => Self::BadVirtualAddress,
            9 => Self::Count,
            10 => Self::EntryHigh,
            11 => Self::Compare,
            12 => Self::Status,
            13 => Self::Cause,
            14 => Self::ExceptionProgramCounter,
            15 => Self::ProcessorRevision,
            16 => Self::Config,
            23 => Self::BadPhysicalAddress,
            24 => Self::Debug,
            25 => Self::Performance,
            28 => Self::TagLow,
            29 => Self::TagHigh,
            30 => Self::ErrorExceptionProgramCounter,
            _ => return None,
        })
    }

    pub const fn index(self) -> u8 {
        self as u8
    }
}

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

    pub(crate) const fn from_field(index: u8) -> Option<Self> {
        match index {
            0 => Some(Self::ImplementationRevision),
            31 => Some(Self::ControlStatus),
            _ => None,
        }
    }
}

macro_rules! bounded_field {
    ($name:ident, $inner:ty, $maximum:expr) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name($inner);

        impl $name {
            pub const fn new(value: $inner) -> Option<Self> {
                if value <= $maximum {
                    Some(Self(value))
                } else {
                    None
                }
            }

            pub const fn value(self) -> $inner {
                self.0
            }

            pub(crate) const fn from_field(value: $inner) -> Self {
                debug_assert!(value <= $maximum);
                Self(value)
            }
        }
    };
}

bounded_field!(ShiftAmount, u8, 31);
bounded_field!(TrapCode, u16, 0x03ff);
bounded_field!(SystemCode, u32, 0x000f_ffff);
bounded_field!(SyncType, u8, 31);
bounded_field!(PrefetchHint, u8, 31);
bounded_field!(PerformanceCounter, u8, 1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operands {
    None,
    ThreeGeneral {
        destination: GeneralRegister,
        left: GeneralRegister,
        right: GeneralRegister,
    },
    TwoGeneral {
        destination: GeneralRegister,
        source: GeneralRegister,
    },
    GeneralPair {
        left: GeneralRegister,
        right: GeneralRegister,
    },
    GeneralDestination {
        destination: GeneralRegister,
    },
    GeneralSource {
        source: GeneralRegister,
    },
    SignedImmediate {
        destination: GeneralRegister,
        source: GeneralRegister,
        immediate: i16,
    },
    UnsignedImmediate {
        destination: GeneralRegister,
        source: GeneralRegister,
        immediate: u16,
    },
    LoadUpper {
        destination: GeneralRegister,
        immediate: u16,
    },
    ShiftImmediate {
        destination: GeneralRegister,
        source: GeneralRegister,
        amount: ShiftAmount,
    },
    ShiftVariable {
        destination: GeneralRegister,
        source: GeneralRegister,
        amount: GeneralRegister,
    },
    Multiply {
        destination: Option<GeneralRegister>,
        left: GeneralRegister,
        right: GeneralRegister,
    },
    Jump {
        target: u32,
    },
    JumpRegister {
        source: GeneralRegister,
        link: Option<GeneralRegister>,
    },
    BranchTwo {
        left: GeneralRegister,
        right: GeneralRegister,
        target: u32,
    },
    BranchOne {
        source: GeneralRegister,
        target: u32,
    },
    Branch {
        target: u32,
    },
    MemoryGeneral {
        register: GeneralRegister,
        base: GeneralRegister,
        offset: i16,
    },
    Prefetch {
        hint: PrefetchHint,
        base: GeneralRegister,
        offset: i16,
    },
    Sync {
        sync_type: SyncType,
    },
    System {
        code: SystemCode,
    },
    TrapGeneral {
        left: GeneralRegister,
        right: GeneralRegister,
        code: TrapCode,
    },
    TrapImmediate {
        source: GeneralRegister,
        immediate: i16,
    },
    SourceImmediate {
        source: GeneralRegister,
        immediate: u16,
    },
    Cache {
        base: GeneralRegister,
        offset: i16,
    },
    SystemControl {
        general: GeneralRegister,
        control: SystemControlRegister,
    },
    Performance {
        general: GeneralRegister,
        counter: PerformanceCounter,
    },
    FloatingTransfer {
        general: GeneralRegister,
        floating: FloatingRegister,
    },
    FloatingControl {
        general: GeneralRegister,
        control: FloatingControlRegister,
    },
    FloatingThree {
        destination: FloatingRegister,
        left: FloatingRegister,
        right: FloatingRegister,
    },
    FloatingTwo {
        destination: FloatingRegister,
        source: FloatingRegister,
    },
    FloatingAccumulatorPair {
        left: FloatingRegister,
        right: FloatingRegister,
    },
    FloatingCompare {
        left: FloatingRegister,
        right: FloatingRegister,
    },
    FloatingMemory {
        floating: FloatingRegister,
        base: GeneralRegister,
        offset: i16,
    },
    VectorControlTransfer {
        general: GeneralRegister,
        control: VectorControlRegister,
        interlock: VectorInterlock,
    },
    VectorFloatingTransfer {
        general: GeneralRegister,
        vector: VectorFloatingRegister,
        interlock: VectorInterlock,
    },
    VectorMemory {
        vector: VectorFloatingRegister,
        base: GeneralRegister,
        offset: i16,
    },
    VectorThree {
        destination_mask: VectorDestinationMask,
        destination: VectorFloatingRegister,
        left: VectorFloatingRegister,
        right: VectorFloatingRegister,
    },
    VectorBroadcast {
        destination_mask: VectorDestinationMask,
        destination: VectorFloatingRegister,
        left: VectorFloatingRegister,
        right: VectorFloatingRegister,
        broadcast: VectorLane,
    },
    VectorTwo {
        destination_mask: VectorDestinationMask,
        destination: VectorFloatingRegister,
        source: VectorFloatingRegister,
    },
    VectorAccumulatorPair {
        destination_mask: VectorDestinationMask,
        left: VectorFloatingRegister,
        right: VectorFloatingRegister,
    },
    VectorAccumulatorSource {
        destination_mask: VectorDestinationMask,
        source: VectorFloatingRegister,
    },
    VectorAccumulatorBroadcast {
        destination_mask: VectorDestinationMask,
        left: VectorFloatingRegister,
        right: VectorFloatingRegister,
        broadcast: VectorLane,
    },
    VectorIntegerThree {
        destination: VectorIntegerRegister,
        left: VectorIntegerRegister,
        right: VectorIntegerRegister,
    },
    VectorIntegerImmediate {
        destination: VectorIntegerRegister,
        source: VectorIntegerRegister,
        immediate: VectorImmediate5,
    },
    VectorIntegerLoad {
        lane: VectorLane,
        destination: VectorIntegerRegister,
        base: VectorIntegerRegister,
    },
    VectorIntegerStore {
        destination_mask: VectorDestinationMask,
        source: VectorIntegerRegister,
        base: VectorIntegerRegister,
    },
    VectorMemoryUpdate {
        destination_mask: VectorDestinationMask,
        vector: VectorFloatingRegister,
        base: VectorIntegerRegister,
    },
    VectorFieldPair {
        left: VectorFloatingRegister,
        left_lane: VectorLane,
        right: VectorFloatingRegister,
        right_lane: VectorLane,
    },
    VectorFieldSource {
        source: VectorFloatingRegister,
        lane: VectorLane,
    },
    VectorIntegerToFloating {
        destination_mask: VectorDestinationMask,
        destination: VectorFloatingRegister,
        source: VectorIntegerRegister,
    },
    VectorFloatingToInteger {
        destination: VectorIntegerRegister,
        source: VectorFloatingRegister,
        lane: VectorLane,
    },
    VectorRandomDestination {
        destination_mask: VectorDestinationMask,
        destination: VectorFloatingRegister,
    },
    VectorPairXyz {
        left: VectorFloatingRegister,
        right: VectorFloatingRegister,
    },
    VectorThreeXyz {
        destination: VectorFloatingRegister,
        left: VectorFloatingRegister,
        right: VectorFloatingRegister,
    },
    VectorMicroCall {
        target: VectorMicroAddress,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    form: InstructionForm,
    operands: Operands,
}

impl Instruction {
    pub fn new(form: InstructionForm, operands: Operands) -> Result<Self, ConstructionError> {
        crate::codec::validate_operands(form, &operands)?;
        Ok(Self { form, operands })
    }

    pub const fn form(&self) -> InstructionForm {
        self.form
    }

    pub const fn operands(&self) -> &Operands {
        &self.operands
    }

    pub(crate) const fn from_decoded(form: InstructionForm, operands: Operands) -> Self {
        Self { form, operands }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConstructionError {
    #[error("operands do not match R5900 instruction {mnemonic}")]
    OperandShapeMismatch { mnemonic: &'static str },
    #[error("CTC1 only accepts FCR31")]
    InvalidFloatingControlWrite,
}
