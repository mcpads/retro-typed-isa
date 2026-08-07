use crate::InstructionForm;

/// One of the 32 128-bit EE Core general-purpose registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GeneralRegister(u8);

impl GeneralRegister {
    /// Constant-zero register (`$zero`).
    pub const ZERO: Self = Self(0);
    /// Assembler-temporary register (`$at`).
    pub const AT: Self = Self(1);
    /// Function-result register 0 (`$v0`).
    pub const V0: Self = Self(2);
    /// Function-result register 1 (`$v1`).
    pub const V1: Self = Self(3);
    /// Argument register 0 (`$a0`).
    pub const A0: Self = Self(4);
    /// Argument register 1 (`$a1`).
    pub const A1: Self = Self(5);
    /// Argument register 2 (`$a2`).
    pub const A2: Self = Self(6);
    /// Argument register 3 (`$a3`).
    pub const A3: Self = Self(7);
    /// Caller-saved temporary register 0 (`$t0`).
    pub const T0: Self = Self(8);
    /// Caller-saved temporary register 1 (`$t1`).
    pub const T1: Self = Self(9);
    /// Caller-saved temporary register 2 (`$t2`).
    pub const T2: Self = Self(10);
    /// Caller-saved temporary register 3 (`$t3`).
    pub const T3: Self = Self(11);
    /// Caller-saved temporary register 4 (`$t4`).
    pub const T4: Self = Self(12);
    /// Caller-saved temporary register 5 (`$t5`).
    pub const T5: Self = Self(13);
    /// Caller-saved temporary register 6 (`$t6`).
    pub const T6: Self = Self(14);
    /// Caller-saved temporary register 7 (`$t7`).
    pub const T7: Self = Self(15);
    /// Callee-saved register 0 (`$s0`).
    pub const S0: Self = Self(16);
    /// Callee-saved register 1 (`$s1`).
    pub const S1: Self = Self(17);
    /// Callee-saved register 2 (`$s2`).
    pub const S2: Self = Self(18);
    /// Callee-saved register 3 (`$s3`).
    pub const S3: Self = Self(19);
    /// Callee-saved register 4 (`$s4`).
    pub const S4: Self = Self(20);
    /// Callee-saved register 5 (`$s5`).
    pub const S5: Self = Self(21);
    /// Callee-saved register 6 (`$s6`).
    pub const S6: Self = Self(22);
    /// Callee-saved register 7 (`$s7`).
    pub const S7: Self = Self(23);
    /// Caller-saved temporary register 8 (`$t8`).
    pub const T8: Self = Self(24);
    /// Caller-saved temporary register 9 (`$t9`).
    pub const T9: Self = Self(25);
    /// Kernel-reserved register 0 (`$k0`).
    pub const K0: Self = Self(26);
    /// Kernel-reserved register 1 (`$k1`).
    pub const K1: Self = Self(27);
    /// Global-pointer register (`$gp`).
    pub const GP: Self = Self(28);
    /// Stack-pointer register (`$sp`).
    pub const SP: Self = Self(29);
    /// Frame-pointer register (`$fp`).
    pub const FP: Self = Self(30);
    /// Return-address register (`$ra`).
    pub const RA: Self = Self(31);

    /// Constructs a register from a five-bit index.
    pub const fn new(index: u8) -> Option<Self> {
        if index < 32 { Some(Self(index)) } else { None }
    }

    /// Returns the architectural register index.
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
    /// Constructs a floating-point register from a five-bit index.
    pub const fn new(index: u8) -> Option<Self> {
        if index < 32 { Some(Self(index)) } else { None }
    }

    /// Returns the architectural register index.
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
    /// Constructs a VU floating-point register from a five-bit index.
    pub const fn new(index: u8) -> Option<Self> {
        if index < 32 { Some(Self(index)) } else { None }
    }

    /// Returns the architectural register index.
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
    /// Constructs a VU integer register from a four-bit index.
    pub const fn new(index: u8) -> Option<Self> {
        if index < 16 { Some(Self(index)) } else { None }
    }

    /// Returns the architectural register index.
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
    /// VU status register (`STATUS`).
    pub const STATUS: Self = Self(16);
    /// VU MAC flag register (`MAC`).
    pub const MAC: Self = Self(17);
    /// VU clipping flag register (`CLIPPING`).
    pub const CLIPPING: Self = Self(18);
    /// VU random-number register (`R`).
    pub const RANDOM: Self = Self(20);
    /// VU immediate scalar register (`I`).
    pub const IMMEDIATE: Self = Self(21);
    /// VU quotient scalar register (`Q`).
    pub const QUOTIENT: Self = Self(22);
    /// VU task program-counter register (`TPC`).
    pub const TASK_PROGRAM_COUNTER: Self = Self(26);
    /// VU call-address register zero (`CMSAR0`).
    pub const CALL_ADDRESS_ZERO: Self = Self(27);
    /// VU force-break/reset register (`FBRST`).
    pub const FORCE_BREAK_RESET: Self = Self(28);
    /// Vector-processing-unit status register (`VPU-STAT`).
    pub const VPU_STATUS: Self = Self(29);
    /// VU call-address register one (`CMSAR1`).
    pub const CALL_ADDRESS_ONE: Self = Self(31);

    /// Constructs a control register when the index is not reserved.
    pub const fn new(index: u8) -> Option<Self> {
        match index {
            0..=18 | 20..=22 | 26..=29 | 31 => Some(Self(index)),
            _ => None,
        }
    }

    /// Returns the architectural control-register index.
    pub const fn index(self) -> u8 {
        self.0
    }
}

/// The VU fields selected by a macro calculation's `dest` bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VectorDestinationMask(u8);

impl VectorDestinationMask {
    /// Selects no destination lanes.
    pub const NONE: Self = Self(0);
    /// Selects all four destination lanes.
    pub const XYZW: Self = Self(0x0f);
    /// Selects the x, y, and z destination lanes.
    pub const XYZ: Self = Self(0x0e);

    /// Constructs a mask from the low four `dest` bits.
    pub const fn new(bits: u8) -> Option<Self> {
        if bits <= 0x0f { Some(Self(bits)) } else { None }
    }

    /// Returns the low four `dest` bits.
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
    /// The x lane.
    X,
    /// The y lane.
    Y,
    /// The z lane.
    Z,
    /// The w lane.
    W,
}

impl VectorLane {
    /// Returns the two-bit lane selector used by broadcast and field forms.
    pub const fn field(self) -> u8 {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
            Self::W => 3,
        }
    }

    /// Returns this lane's bit in a VU destination mask.
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
    /// Encode the non-interlocked COP2 transfer form.
    Without,
    /// Encode the interlocked COP2 transfer form.
    With,
}

impl VectorInterlock {
    /// Returns the instruction's interlock bit.
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
    /// Constructs an immediate in the inclusive range -16 through 15.
    pub const fn new(value: i8) -> Option<Self> {
        if value >= -16 && value <= 15 {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Returns the sign-extended immediate value.
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
    /// Constructs an eight-byte-aligned VU0 MicroMem byte address.
    pub const fn new(byte_address: u32) -> Option<Self> {
        if byte_address <= 0x0003_fff8 && byte_address & 7 == 0 {
            Some(Self(byte_address))
        } else {
            None
        }
    }

    /// Returns the semantic byte address.
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
    /// TLB index register.
    Index = 0,
    /// Pseudo-random TLB entry selector.
    Random = 1,
    /// TLB entry-low register zero.
    EntryLow0 = 2,
    /// TLB entry-low register one.
    EntryLow1 = 3,
    /// Virtual-address context register.
    Context = 4,
    /// TLB page-mask register.
    PageMask = 5,
    /// Wired TLB boundary register.
    Wired = 6,
    /// Bad virtual address register.
    BadVirtualAddress = 8,
    /// Processor count register.
    Count = 9,
    /// TLB entry-high register.
    EntryHigh = 10,
    /// Processor timer compare register.
    Compare = 11,
    /// Processor status register.
    Status = 12,
    /// Exception cause register.
    Cause = 13,
    /// Exception program-counter register.
    ExceptionProgramCounter = 14,
    /// Processor revision identifier register.
    ProcessorRevision = 15,
    /// Processor configuration register.
    Config = 16,
    /// Bad physical address register.
    BadPhysicalAddress = 23,
    /// Debug control register.
    Debug = 24,
    /// Performance-counter control register.
    Performance = 25,
    /// Cache tag-low register.
    TagLow = 28,
    /// Cache tag-high register.
    TagHigh = 29,
    /// Error-exception program-counter register.
    ErrorExceptionProgramCounter = 30,
}

impl SystemControlRegister {
    /// Constructs a register when the COP0 index is implemented by the EE Core.
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

    /// Returns the architectural COP0 register index.
    pub const fn index(self) -> u8 {
        self as u8
    }
}

/// An FPU control register accepted by CFC1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum FloatingControlRegister {
    /// FPU implementation and revision register (FCR0).
    ImplementationRevision = 0,
    /// FPU control and status register (FCR31).
    ControlStatus = 31,
}

impl FloatingControlRegister {
    /// Returns the architectural FCR index.
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
    ($name:ident, $inner:ty, $maximum:expr, $description:literal) => {
        #[doc = $description]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name($inner);

        impl $name {
            /// Constructs a value when it fits the architectural field.
            pub const fn new(value: $inner) -> Option<Self> {
                if value <= $maximum {
                    Some(Self(value))
                } else {
                    None
                }
            }

            /// Returns the unencoded field value.
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

bounded_field!(ShiftAmount, u8, 31, "A five-bit immediate shift amount.");
bounded_field!(TrapCode, u16, 0x03ff, "A ten-bit register-trap code.");
bounded_field!(
    SystemCode,
    u32,
    0x000f_ffff,
    "A 20-bit `syscall` or `break` code."
);
bounded_field!(SyncType, u8, 31, "The five-bit `sync` subtype field.");
bounded_field!(
    PrefetchHint,
    u8,
    31,
    "The five-bit implementation-defined `pref` hint field."
);
bounded_field!(
    PerformanceCounter,
    u8,
    1,
    "One of the two EE Core performance counters."
);

/// Typed operand shapes for the complete R5900 instruction catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operands {
    /// No explicit operands.
    None,
    /// Three general registers: one destination and two inputs.
    ThreeGeneral {
        /// Destination register.
        destination: GeneralRegister,
        /// Left input register.
        left: GeneralRegister,
        /// Right input register.
        right: GeneralRegister,
    },
    /// One general-register destination and one source.
    TwoGeneral {
        /// Destination register.
        destination: GeneralRegister,
        /// Source register.
        source: GeneralRegister,
    },
    /// Two general-register inputs without an explicit destination.
    GeneralPair {
        /// Left input register.
        left: GeneralRegister,
        /// Right input register.
        right: GeneralRegister,
    },
    /// One general-register destination.
    GeneralDestination {
        /// Destination register.
        destination: GeneralRegister,
    },
    /// One general-register source.
    GeneralSource {
        /// Source register.
        source: GeneralRegister,
    },
    /// A general-register destination, source, and signed immediate.
    SignedImmediate {
        /// Destination register.
        destination: GeneralRegister,
        /// Source register.
        source: GeneralRegister,
        /// Immediate value.
        immediate: i16,
    },
    /// A general-register destination, source, and unsigned immediate.
    UnsignedImmediate {
        /// Destination register.
        destination: GeneralRegister,
        /// Source register.
        source: GeneralRegister,
        /// Immediate value.
        immediate: u16,
    },
    /// A general-register destination and upper-half immediate.
    LoadUpper {
        /// Destination register.
        destination: GeneralRegister,
        /// Immediate value.
        immediate: u16,
    },
    /// A general-register destination, source, and immediate shift amount.
    ShiftImmediate {
        /// Destination register.
        destination: GeneralRegister,
        /// Source register.
        source: GeneralRegister,
        /// Shift amount or register supplying it.
        amount: ShiftAmount,
    },
    /// A general-register destination, source, and register-provided shift amount.
    ShiftVariable {
        /// Destination register.
        destination: GeneralRegister,
        /// Source register.
        source: GeneralRegister,
        /// Shift amount or register supplying it.
        amount: GeneralRegister,
    },
    /// Two general-register inputs and an optional destination.
    Multiply {
        /// Destination register.
        destination: Option<GeneralRegister>,
        /// Left input register.
        left: GeneralRegister,
        /// Right input register.
        right: GeneralRegister,
    },
    /// An absolute direct-jump target.
    Jump {
        /// Absolute control-flow or microprogram target.
        target: u32,
    },
    /// A register-indirect jump with an optional link destination.
    JumpRegister {
        /// Source register.
        source: GeneralRegister,
        /// Optional link destination register.
        link: Option<GeneralRegister>,
    },
    /// Two compared registers and an absolute branch target.
    BranchTwo {
        /// Left input register.
        left: GeneralRegister,
        /// Right input register.
        right: GeneralRegister,
        /// Absolute control-flow or microprogram target.
        target: u32,
    },
    /// One tested register and an absolute branch target.
    BranchOne {
        /// Source register.
        source: GeneralRegister,
        /// Absolute control-flow or microprogram target.
        target: u32,
    },
    /// A coprocessor condition branch target.
    Branch {
        /// Absolute control-flow or microprogram target.
        target: u32,
    },
    /// A general register and base-plus-displacement memory address.
    MemoryGeneral {
        /// Transferred general register.
        register: GeneralRegister,
        /// Base address register.
        base: GeneralRegister,
        /// Signed byte displacement.
        offset: i16,
    },
    /// A prefetch hint and base-plus-displacement memory address.
    Prefetch {
        /// Implementation-defined prefetch hint.
        hint: PrefetchHint,
        /// Base address register.
        base: GeneralRegister,
        /// Signed byte displacement.
        offset: i16,
    },
    /// A memory-ordering subtype.
    Sync {
        /// Memory-ordering subtype.
        sync_type: SyncType,
    },
    /// A system-call or break code.
    System {
        /// Embedded system or trap code.
        code: SystemCode,
    },
    /// Two compared registers and a trap code.
    TrapGeneral {
        /// Left input register.
        left: GeneralRegister,
        /// Right input register.
        right: GeneralRegister,
        /// Embedded system or trap code.
        code: TrapCode,
    },
    /// One compared register and a signed immediate.
    TrapImmediate {
        /// Source register.
        source: GeneralRegister,
        /// Immediate value.
        immediate: i16,
    },
    /// One source register and an unsigned immediate.
    SourceImmediate {
        /// Source register.
        source: GeneralRegister,
        /// Immediate value.
        immediate: u16,
    },
    /// A cache operation's base-plus-displacement address.
    Cache {
        /// Base address register.
        base: GeneralRegister,
        /// Signed byte displacement.
        offset: i16,
    },
    /// A transfer between a general register and COP0 register.
    SystemControl {
        /// General-register side of the transfer.
        general: GeneralRegister,
        /// Control-register selector.
        control: SystemControlRegister,
    },
    /// A transfer between a general register and performance counter.
    Performance {
        /// General-register side of the transfer.
        general: GeneralRegister,
        /// Performance-counter selector.
        counter: PerformanceCounter,
    },
    /// A transfer between a general register and floating-point register.
    FloatingTransfer {
        /// General-register side of the transfer.
        general: GeneralRegister,
        /// Floating-point-register side of the transfer.
        floating: FloatingRegister,
    },
    /// A transfer between a general register and FPU control register.
    FloatingControl {
        /// General-register side of the transfer.
        general: GeneralRegister,
        /// Control-register selector.
        control: FloatingControlRegister,
    },
    /// One floating-point destination and two floating-point inputs.
    FloatingThree {
        /// Destination register.
        destination: FloatingRegister,
        /// Left input register.
        left: FloatingRegister,
        /// Right input register.
        right: FloatingRegister,
    },
    /// One floating-point destination and one source.
    FloatingTwo {
        /// Destination register.
        destination: FloatingRegister,
        /// Source register.
        source: FloatingRegister,
    },
    /// Two floating-point inputs for an accumulator operation.
    FloatingAccumulatorPair {
        /// Left input register.
        left: FloatingRegister,
        /// Right input register.
        right: FloatingRegister,
    },
    /// Two floating-point inputs for a comparison.
    FloatingCompare {
        /// Left input register.
        left: FloatingRegister,
        /// Right input register.
        right: FloatingRegister,
    },
    /// A floating-point register and base-plus-displacement memory address.
    FloatingMemory {
        /// Floating-point-register side of the transfer.
        floating: FloatingRegister,
        /// Base address register.
        base: GeneralRegister,
        /// Signed byte displacement.
        offset: i16,
    },
    /// A general/VU-control-register transfer with explicit interlock selection.
    VectorControlTransfer {
        /// General-register side of the transfer.
        general: GeneralRegister,
        /// Control-register selector.
        control: VectorControlRegister,
        /// Whether the COP2 transfer uses the interlocked encoding.
        interlock: VectorInterlock,
    },
    /// A general/VU-floating-register transfer with explicit interlock selection.
    VectorFloatingTransfer {
        /// General-register side of the transfer.
        general: GeneralRegister,
        /// VU floating-point-register side of the operation.
        vector: VectorFloatingRegister,
        /// Whether the COP2 transfer uses the interlocked encoding.
        interlock: VectorInterlock,
    },
    /// A VU floating register and base-plus-displacement memory address.
    VectorMemory {
        /// VU floating-point-register side of the operation.
        vector: VectorFloatingRegister,
        /// Base address register.
        base: GeneralRegister,
        /// Signed byte displacement.
        offset: i16,
    },
    /// A masked VU destination and two VU floating-point inputs.
    VectorThree {
        /// VU destination lanes written by the operation.
        destination_mask: VectorDestinationMask,
        /// Destination register.
        destination: VectorFloatingRegister,
        /// Left input register.
        left: VectorFloatingRegister,
        /// Right input register.
        right: VectorFloatingRegister,
    },
    /// A masked VU destination, two inputs, and a broadcast lane.
    VectorBroadcast {
        /// VU destination lanes written by the operation.
        destination_mask: VectorDestinationMask,
        /// Destination register.
        destination: VectorFloatingRegister,
        /// Left input register.
        left: VectorFloatingRegister,
        /// Right input register.
        right: VectorFloatingRegister,
        /// VU source lane broadcast across the calculation.
        broadcast: VectorLane,
    },
    /// A masked VU destination and one VU floating-point source.
    VectorTwo {
        /// VU destination lanes written by the operation.
        destination_mask: VectorDestinationMask,
        /// Destination register.
        destination: VectorFloatingRegister,
        /// Source register.
        source: VectorFloatingRegister,
    },
    /// A destination mask and two VU accumulator inputs.
    VectorAccumulatorPair {
        /// VU destination lanes written by the operation.
        destination_mask: VectorDestinationMask,
        /// Left input register.
        left: VectorFloatingRegister,
        /// Right input register.
        right: VectorFloatingRegister,
    },
    /// A destination mask and one VU accumulator source.
    VectorAccumulatorSource {
        /// VU destination lanes written by the operation.
        destination_mask: VectorDestinationMask,
        /// Source register.
        source: VectorFloatingRegister,
    },
    /// A destination mask, two VU inputs, and an accumulator broadcast lane.
    VectorAccumulatorBroadcast {
        /// VU destination lanes written by the operation.
        destination_mask: VectorDestinationMask,
        /// Left input register.
        left: VectorFloatingRegister,
        /// Right input register.
        right: VectorFloatingRegister,
        /// VU source lane broadcast across the calculation.
        broadcast: VectorLane,
    },
    /// One VU integer destination and two VU integer inputs.
    VectorIntegerThree {
        /// Destination register.
        destination: VectorIntegerRegister,
        /// Left input register.
        left: VectorIntegerRegister,
        /// Right input register.
        right: VectorIntegerRegister,
    },
    /// VU integer destination and source registers with a signed five-bit immediate.
    VectorIntegerImmediate {
        /// Destination register.
        destination: VectorIntegerRegister,
        /// Source register.
        source: VectorIntegerRegister,
        /// Immediate value.
        immediate: VectorImmediate5,
    },
    /// A VU integer load lane, destination, and base register.
    VectorIntegerLoad {
        /// Selected VU register lane.
        lane: VectorLane,
        /// Destination register.
        destination: VectorIntegerRegister,
        /// Base address register.
        base: VectorIntegerRegister,
    },
    /// A masked VU integer store source and base register.
    VectorIntegerStore {
        /// VU destination lanes written by the operation.
        destination_mask: VectorDestinationMask,
        /// Source register.
        source: VectorIntegerRegister,
        /// Base address register.
        base: VectorIntegerRegister,
    },
    /// A masked VU floating register and updating VU integer base.
    VectorMemoryUpdate {
        /// VU destination lanes written by the operation.
        destination_mask: VectorDestinationMask,
        /// VU floating-point-register side of the operation.
        vector: VectorFloatingRegister,
        /// Base address register.
        base: VectorIntegerRegister,
    },
    /// Two selected fields from two VU floating-point registers.
    VectorFieldPair {
        /// Left input register.
        left: VectorFloatingRegister,
        /// Selected lane of the left input.
        left_lane: VectorLane,
        /// Right input register.
        right: VectorFloatingRegister,
        /// Selected lane of the right input.
        right_lane: VectorLane,
    },
    /// One selected field from a VU floating-point register.
    VectorFieldSource {
        /// Source register.
        source: VectorFloatingRegister,
        /// Selected VU register lane.
        lane: VectorLane,
    },
    /// A masked VU floating destination and VU integer source.
    VectorIntegerToFloating {
        /// VU destination lanes written by the operation.
        destination_mask: VectorDestinationMask,
        /// Destination register.
        destination: VectorFloatingRegister,
        /// Source register.
        source: VectorIntegerRegister,
    },
    /// A VU integer destination and selected floating-point source lane.
    VectorFloatingToInteger {
        /// Destination register.
        destination: VectorIntegerRegister,
        /// Source register.
        source: VectorFloatingRegister,
        /// Selected VU register lane.
        lane: VectorLane,
    },
    /// A masked destination for a VU random-number operation.
    VectorRandomDestination {
        /// VU destination lanes written by the operation.
        destination_mask: VectorDestinationMask,
        /// Destination register.
        destination: VectorFloatingRegister,
    },
    /// Two VU floating-point inputs whose x, y, and z lanes are implied.
    VectorPairXyz {
        /// Left input register.
        left: VectorFloatingRegister,
        /// Right input register.
        right: VectorFloatingRegister,
    },
    /// One VU destination and two inputs whose x, y, and z lanes are implied.
    VectorThreeXyz {
        /// Destination register.
        destination: VectorFloatingRegister,
        /// Left input register.
        left: VectorFloatingRegister,
        /// Right input register.
        right: VectorFloatingRegister,
    },
    /// An aligned VU0 MicroMem call target.
    VectorMicroCall {
        /// Absolute control-flow or microprogram target.
        target: VectorMicroAddress,
    },
}

/// One validated R5900 instruction and its typed operand shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    form: InstructionForm,
    operands: Operands,
}

impl Instruction {
    /// Constructs an instruction when `operands` match the selected form.
    ///
    /// This also enforces form-specific restrictions such as the FCR31-only
    /// write target of `ctc1`.
    pub fn new(form: InstructionForm, operands: Operands) -> Result<Self, ConstructionError> {
        crate::codec::validate_operands(form, &operands)?;
        Ok(Self { form, operands })
    }

    /// Returns the executable instruction form.
    pub const fn form(&self) -> InstructionForm {
        self.form
    }

    /// Returns the form-specific typed operands.
    pub const fn operands(&self) -> &Operands {
        &self.operands
    }

    pub(crate) const fn from_decoded(form: InstructionForm, operands: Operands) -> Self {
        Self { form, operands }
    }
}

/// Failure to pair an instruction form with a valid typed operand shape.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConstructionError {
    /// The operand shape is not accepted by the selected form.
    #[error("operands do not match R5900 instruction {mnemonic}")]
    OperandShapeMismatch {
        /// Mnemonic of the selected instruction form.
        mnemonic: &'static str,
    },
    /// `ctc1` attempts to write an FPU control register other than FCR31.
    #[error("CTC1 only accepts FCR31")]
    InvalidFloatingControlWrite,
}
