use crate::codec::EncodeError;

macro_rules! bounded_u8 {
    ($name:ident, $limit:expr, $description:literal) => {
        #[doc = $description]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u8);

        impl $name {
            pub const fn new(value: u8) -> Option<Self> {
                if value < $limit {
                    Some(Self(value))
                } else {
                    None
                }
            }

            pub const fn value(self) -> u8 {
                self.0
            }

            pub(crate) const fn from_field(value: u8) -> Self {
                debug_assert!(value < $limit);
                Self(value)
            }
        }
    };
}

bounded_u8!(
    GeneralRegister,
    32,
    "One of the 32 general-purpose registers."
);
bounded_u8!(
    FloatingRegister,
    32,
    "One of the 32 floating-point registers."
);
bounded_u8!(
    ConditionRegisterField,
    8,
    "One of the eight four-bit condition-register fields."
);
bounded_u8!(
    ConditionRegisterBit,
    32,
    "One of the 32 condition-register bits."
);
bounded_u8!(
    SegmentRegister,
    16,
    "One of the 16 segment-register selectors."
);
bounded_u8!(
    QuantizationRegister,
    8,
    "One of the eight paired-single graphics quantization registers."
);
bounded_u8!(BitIndex, 32, "A five-bit shift or mask bit index.");
bounded_u8!(
    BranchOptions,
    32,
    "The five-bit BO field of a conditional branch."
);
bounded_u8!(
    TrapCondition,
    32,
    "The five-bit TO field of a trap instruction."
);

/// The eight-bit condition-register field mask used by `mtcrf`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConditionRegisterMask(u8);

impl ConditionRegisterMask {
    pub const fn new(bits: u8) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u8 {
        self.0
    }
}

/// The eight-bit FPSCR field mask used by `mtfsf`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FpscrFieldMask(u8);

impl FpscrFieldMask {
    pub const fn new(bits: u8) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u8 {
        self.0
    }
}

/// The four-bit immediate used by `mtfsfi`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FpscrImmediate(u8);

impl FpscrImmediate {
    pub const fn new(value: u8) -> Option<Self> {
        if value < 16 { Some(Self(value)) } else { None }
    }

    pub const fn value(self) -> u8 {
        self.0
    }

    pub(crate) const fn from_field(value: u8) -> Self {
        debug_assert!(value < 16);
        Self(value)
    }
}

/// A Gekko or PowerPC 750CL SPR listed in the processors' programming-model tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum SpecialPurposeRegister {
    Xer = 1,
    Link = 8,
    Count = 9,
    DataStorageInterruptStatus = 18,
    DataAddress = 19,
    Decrementer = 22,
    StorageDescription = 25,
    SaveRestore0 = 26,
    SaveRestore1 = 27,
    TimeBaseLowerRead = 268,
    TimeBaseUpperRead = 269,
    General0 = 272,
    General1 = 273,
    General2 = 274,
    General3 = 275,
    ExternalAccess = 282,
    TimeBaseLowerWrite = 284,
    TimeBaseUpperWrite = 285,
    ProcessorVersion = 287,
    InstructionBat0Upper = 528,
    InstructionBat0Lower = 529,
    InstructionBat1Upper = 530,
    InstructionBat1Lower = 531,
    InstructionBat2Upper = 532,
    InstructionBat2Lower = 533,
    InstructionBat3Upper = 534,
    InstructionBat3Lower = 535,
    DataBat0Upper = 536,
    DataBat0Lower = 537,
    DataBat1Upper = 538,
    DataBat1Lower = 539,
    DataBat2Upper = 540,
    DataBat2Lower = 541,
    DataBat3Upper = 542,
    DataBat3Lower = 543,
    InstructionBat4Upper = 560,
    InstructionBat4Lower = 561,
    InstructionBat5Upper = 562,
    InstructionBat5Lower = 563,
    InstructionBat6Upper = 564,
    InstructionBat6Lower = 565,
    InstructionBat7Upper = 566,
    InstructionBat7Lower = 567,
    DataBat4Upper = 568,
    DataBat4Lower = 569,
    DataBat5Upper = 570,
    DataBat5Lower = 571,
    DataBat6Upper = 572,
    DataBat6Lower = 573,
    DataBat7Upper = 574,
    DataBat7Lower = 575,
    GraphicsQuantization0 = 912,
    GraphicsQuantization1 = 913,
    GraphicsQuantization2 = 914,
    GraphicsQuantization3 = 915,
    GraphicsQuantization4 = 916,
    GraphicsQuantization5 = 917,
    GraphicsQuantization6 = 918,
    GraphicsQuantization7 = 919,
    HardwareImplementation2 = 920,
    WritePipeAddress = 921,
    DmaUpper = 922,
    DmaLower = 923,
    UserMonitorModeControl0 = 936,
    UserPerformanceMonitorCounter1 = 937,
    UserPerformanceMonitorCounter2 = 938,
    UserSampledInstructionAddress = 939,
    UserMonitorModeControl1 = 940,
    UserPerformanceMonitorCounter3 = 941,
    UserPerformanceMonitorCounter4 = 942,
    UserSampledDataAddress = 943,
    MonitorModeControl0 = 952,
    PerformanceMonitorCounter1 = 953,
    PerformanceMonitorCounter2 = 954,
    SampledInstructionAddress = 955,
    MonitorModeControl1 = 956,
    PerformanceMonitorCounter3 = 957,
    PerformanceMonitorCounter4 = 958,
    SampledDataAddress = 959,
    HardwareImplementation0 = 1008,
    HardwareImplementation1 = 1009,
    InstructionAddressBreakpoint = 1010,
    HardwareImplementation4 = 1011,
    ThermalDataCacheLower = 1012,
    DataAddressBreakpoint = 1013,
    L2CacheControl = 1017,
    ThermalDataCacheUpper = 1018,
    InstructionCacheThrottlingControl = 1019,
    ThermalManagement1 = 1020,
    ThermalManagement2 = 1021,
    ThermalManagement3 = 1022,
}

impl SpecialPurposeRegister {
    pub const fn selector(self) -> u16 {
        self as u16
    }

    pub(crate) const fn from_field(selector: u16) -> Option<Self> {
        Some(match selector {
            1 => Self::Xer,
            8 => Self::Link,
            9 => Self::Count,
            18 => Self::DataStorageInterruptStatus,
            19 => Self::DataAddress,
            22 => Self::Decrementer,
            25 => Self::StorageDescription,
            26 => Self::SaveRestore0,
            27 => Self::SaveRestore1,
            268 => Self::TimeBaseLowerRead,
            269 => Self::TimeBaseUpperRead,
            272 => Self::General0,
            273 => Self::General1,
            274 => Self::General2,
            275 => Self::General3,
            282 => Self::ExternalAccess,
            284 => Self::TimeBaseLowerWrite,
            285 => Self::TimeBaseUpperWrite,
            287 => Self::ProcessorVersion,
            528 => Self::InstructionBat0Upper,
            529 => Self::InstructionBat0Lower,
            530 => Self::InstructionBat1Upper,
            531 => Self::InstructionBat1Lower,
            532 => Self::InstructionBat2Upper,
            533 => Self::InstructionBat2Lower,
            534 => Self::InstructionBat3Upper,
            535 => Self::InstructionBat3Lower,
            536 => Self::DataBat0Upper,
            537 => Self::DataBat0Lower,
            538 => Self::DataBat1Upper,
            539 => Self::DataBat1Lower,
            540 => Self::DataBat2Upper,
            541 => Self::DataBat2Lower,
            542 => Self::DataBat3Upper,
            543 => Self::DataBat3Lower,
            560 => Self::InstructionBat4Upper,
            561 => Self::InstructionBat4Lower,
            562 => Self::InstructionBat5Upper,
            563 => Self::InstructionBat5Lower,
            564 => Self::InstructionBat6Upper,
            565 => Self::InstructionBat6Lower,
            566 => Self::InstructionBat7Upper,
            567 => Self::InstructionBat7Lower,
            568 => Self::DataBat4Upper,
            569 => Self::DataBat4Lower,
            570 => Self::DataBat5Upper,
            571 => Self::DataBat5Lower,
            572 => Self::DataBat6Upper,
            573 => Self::DataBat6Lower,
            574 => Self::DataBat7Upper,
            575 => Self::DataBat7Lower,
            912 => Self::GraphicsQuantization0,
            913 => Self::GraphicsQuantization1,
            914 => Self::GraphicsQuantization2,
            915 => Self::GraphicsQuantization3,
            916 => Self::GraphicsQuantization4,
            917 => Self::GraphicsQuantization5,
            918 => Self::GraphicsQuantization6,
            919 => Self::GraphicsQuantization7,
            920 => Self::HardwareImplementation2,
            921 => Self::WritePipeAddress,
            922 => Self::DmaUpper,
            923 => Self::DmaLower,
            936 => Self::UserMonitorModeControl0,
            937 => Self::UserPerformanceMonitorCounter1,
            938 => Self::UserPerformanceMonitorCounter2,
            939 => Self::UserSampledInstructionAddress,
            940 => Self::UserMonitorModeControl1,
            941 => Self::UserPerformanceMonitorCounter3,
            942 => Self::UserPerformanceMonitorCounter4,
            943 => Self::UserSampledDataAddress,
            952 => Self::MonitorModeControl0,
            953 => Self::PerformanceMonitorCounter1,
            954 => Self::PerformanceMonitorCounter2,
            955 => Self::SampledInstructionAddress,
            956 => Self::MonitorModeControl1,
            957 => Self::PerformanceMonitorCounter3,
            958 => Self::PerformanceMonitorCounter4,
            959 => Self::SampledDataAddress,
            1008 => Self::HardwareImplementation0,
            1009 => Self::HardwareImplementation1,
            1010 => Self::InstructionAddressBreakpoint,
            1011 => Self::HardwareImplementation4,
            1012 => Self::ThermalDataCacheLower,
            1013 => Self::DataAddressBreakpoint,
            1017 => Self::L2CacheControl,
            1018 => Self::ThermalDataCacheUpper,
            1019 => Self::InstructionCacheThrottlingControl,
            1020 => Self::ThermalManagement1,
            1021 => Self::ThermalManagement2,
            1022 => Self::ThermalManagement3,
            _ => return None,
        })
    }

    pub(crate) const fn is_available_on_gekko(self) -> bool {
        !matches!(
            self,
            Self::InstructionBat4Upper
                | Self::InstructionBat4Lower
                | Self::InstructionBat5Upper
                | Self::InstructionBat5Lower
                | Self::InstructionBat6Upper
                | Self::InstructionBat6Lower
                | Self::InstructionBat7Upper
                | Self::InstructionBat7Lower
                | Self::DataBat4Upper
                | Self::DataBat4Lower
                | Self::DataBat5Upper
                | Self::DataBat5Lower
                | Self::DataBat6Upper
                | Self::DataBat6Lower
                | Self::DataBat7Upper
                | Self::DataBat7Lower
                | Self::HardwareImplementation4
                | Self::ThermalDataCacheLower
                | Self::ThermalDataCacheUpper
        )
    }

    pub(crate) const fn can_read(self) -> bool {
        !matches!(
            self,
            Self::TimeBaseLowerWrite
                | Self::TimeBaseUpperWrite
                | Self::UserSampledDataAddress
                | Self::SampledDataAddress
        )
    }

    pub(crate) const fn can_write(self) -> bool {
        !matches!(
            self,
            Self::TimeBaseLowerRead
                | Self::TimeBaseUpperRead
                | Self::ProcessorVersion
                | Self::UserMonitorModeControl0
                | Self::UserPerformanceMonitorCounter1
                | Self::UserPerformanceMonitorCounter2
                | Self::UserSampledInstructionAddress
                | Self::UserMonitorModeControl1
                | Self::UserPerformanceMonitorCounter3
                | Self::UserPerformanceMonitorCounter4
                | Self::ThermalDataCacheLower
                | Self::ThermalDataCacheUpper
        )
    }
}

/// A time-base register accepted by `mftb` on the 32-bit profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum TimeBaseRegister {
    Lower = 268,
    Upper = 269,
}

impl TimeBaseRegister {
    pub const fn selector(self) -> u16 {
        self as u16
    }

    pub(crate) const fn from_field(selector: u16) -> Option<Self> {
        match selector {
            268 => Some(Self::Lower),
            269 => Some(Self::Upper),
            _ => None,
        }
    }
}

/// The semantic byte count of an immediate string load or store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StringByteCount(u8);

impl StringByteCount {
    pub const fn new(bytes: u8) -> Option<Self> {
        if bytes >= 1 && bytes <= 32 {
            Some(Self(bytes))
        } else {
            None
        }
    }

    pub const fn bytes(self) -> u8 {
        self.0
    }

    pub(crate) const fn from_field(field: u8) -> Self {
        if field == 0 { Self(32) } else { Self(field) }
    }

    pub(crate) const fn field(self) -> u8 {
        if self.0 == 32 { 0 } else { self.0 }
    }
}

/// Whether a quantized paired-single memory operation transfers one or two values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PairedSingleWidth {
    Pair,
    Single,
}

impl PairedSingleWidth {
    pub(crate) const fn field(self) -> u8 {
        match self {
            Self::Pair => 0,
            Self::Single => 1,
        }
    }

    pub(crate) const fn from_field(field: u8) -> Self {
        if field == 0 { Self::Pair } else { Self::Single }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Operand {
    GeneralRegister(GeneralRegister),
    FloatingRegister(FloatingRegister),
    ConditionRegisterField(ConditionRegisterField),
    ConditionRegisterBit(ConditionRegisterBit),
    SegmentRegister(SegmentRegister),
    SpecialPurposeRegister(SpecialPurposeRegister),
    TimeBaseRegister(TimeBaseRegister),
    QuantizationRegister(QuantizationRegister),
    SignedImmediate(i16),
    UnsignedImmediate(u16),
    MemoryOffset(i16),
    BranchTarget(u32),
    ShiftOrMaskBit(BitIndex),
    BranchOptions(BranchOptions),
    TrapCondition(TrapCondition),
    StringByteCount(StringByteCount),
    ConditionRegisterMask(ConditionRegisterMask),
    FpscrFieldMask(FpscrFieldMask),
    FpscrImmediate(FpscrImmediate),
    PairedSingleWidth(PairedSingleWidth),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OperandKind {
    GeneralRegister,
    FloatingRegister,
    ConditionRegisterField,
    ConditionRegisterBit,
    SegmentRegister,
    SpecialPurposeRegister,
    TimeBaseRegister,
    QuantizationRegister,
    SignedImmediate,
    UnsignedImmediate,
    MemoryOffset,
    BranchTarget,
    ShiftOrMaskBit,
    BranchOptions,
    TrapCondition,
    StringByteCount,
    ConditionRegisterMask,
    FpscrFieldMask,
    FpscrImmediate,
    PairedSingleWidth,
}

impl Operand {
    pub const fn kind(self) -> OperandKind {
        match self {
            Self::GeneralRegister(_) => OperandKind::GeneralRegister,
            Self::FloatingRegister(_) => OperandKind::FloatingRegister,
            Self::ConditionRegisterField(_) => OperandKind::ConditionRegisterField,
            Self::ConditionRegisterBit(_) => OperandKind::ConditionRegisterBit,
            Self::SegmentRegister(_) => OperandKind::SegmentRegister,
            Self::SpecialPurposeRegister(_) => OperandKind::SpecialPurposeRegister,
            Self::TimeBaseRegister(_) => OperandKind::TimeBaseRegister,
            Self::QuantizationRegister(_) => OperandKind::QuantizationRegister,
            Self::SignedImmediate(_) => OperandKind::SignedImmediate,
            Self::UnsignedImmediate(_) => OperandKind::UnsignedImmediate,
            Self::MemoryOffset(_) => OperandKind::MemoryOffset,
            Self::BranchTarget(_) => OperandKind::BranchTarget,
            Self::ShiftOrMaskBit(_) => OperandKind::ShiftOrMaskBit,
            Self::BranchOptions(_) => OperandKind::BranchOptions,
            Self::TrapCondition(_) => OperandKind::TrapCondition,
            Self::StringByteCount(_) => OperandKind::StringByteCount,
            Self::ConditionRegisterMask(_) => OperandKind::ConditionRegisterMask,
            Self::FpscrFieldMask(_) => OperandKind::FpscrFieldMask,
            Self::FpscrImmediate(_) => OperandKind::FpscrImmediate,
            Self::PairedSingleWidth(_) => OperandKind::PairedSingleWidth,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstructionModifiers {
    pub record: bool,
    pub overflow: bool,
    pub link: bool,
    pub absolute: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    form: InstructionForm,
    modifiers: InstructionModifiers,
    operands: Vec<Operand>,
}

impl Instruction {
    pub fn new(
        form: InstructionForm,
        operands: impl Into<Vec<Operand>>,
    ) -> Result<Self, EncodeError> {
        Self::with_modifiers(form, InstructionModifiers::default(), operands)
    }

    pub fn with_modifiers(
        form: InstructionForm,
        modifiers: InstructionModifiers,
        operands: impl Into<Vec<Operand>>,
    ) -> Result<Self, EncodeError> {
        let operands = operands.into();
        let definition = form.definition();
        if operands.len() != definition.fields.len() {
            return Err(EncodeError::OperandCount {
                actual: operands.len(),
                expected: definition.fields.len(),
            });
        }
        for (index, (operand, field)) in operands.iter().zip(definition.fields).enumerate() {
            let expected = field.operand_kind();
            let actual = operand.kind();
            if actual != expected {
                return Err(EncodeError::OperandKind {
                    index,
                    expected,
                    actual,
                });
            }
        }
        let support = form.modifier_support();
        if (modifiers.record && !support.record)
            || (modifiers.overflow && !support.overflow)
            || (modifiers.link && !support.link)
            || (modifiers.absolute && !support.absolute)
        {
            return Err(EncodeError::InvalidModifiers { form, modifiers });
        }
        match (form, operands.as_slice()) {
            (InstructionForm::Mfspr, [_, Operand::SpecialPurposeRegister(register)])
                if !register.can_read() =>
            {
                return Err(EncodeError::InvalidSpecialPurposeAccess {
                    form,
                    register: *register,
                });
            }
            (InstructionForm::Mtspr, [Operand::SpecialPurposeRegister(register), _])
                if !register.can_write() =>
            {
                return Err(EncodeError::InvalidSpecialPurposeAccess {
                    form,
                    register: *register,
                });
            }
            _ => {}
        }
        validate_operand_constraints(form, &operands)?;
        Ok(Self {
            form,
            modifiers,
            operands,
        })
    }

    pub const fn form(&self) -> InstructionForm {
        self.form
    }

    pub const fn modifiers(&self) -> InstructionModifiers {
        self.modifiers
    }

    pub fn operands(&self) -> &[Operand] {
        &self.operands
    }
}

fn validate_operand_constraints(
    form: InstructionForm,
    operands: &[Operand],
) -> Result<(), EncodeError> {
    let general_register = |index: usize| match operands[index] {
        Operand::GeneralRegister(register) => register,
        _ => unreachable!("operand kinds were validated"),
    };

    if form == InstructionForm::Bcctr {
        let Operand::BranchOptions(options) = operands[0] else {
            unreachable!("operand kinds were validated");
        };
        if options.value() & 0b0_0100 == 0 {
            return Err(EncodeError::CountRegisterBranchDecrementsCount { options });
        }
    }

    let update_base_index = match form {
        InstructionForm::PsqLux
        | InstructionForm::PsqStux
        | InstructionForm::Lbzux
        | InstructionForm::Lfdux
        | InstructionForm::Lfsux
        | InstructionForm::Lhaux
        | InstructionForm::Lhzux
        | InstructionForm::Lwzux
        | InstructionForm::Stbux
        | InstructionForm::Stfdux
        | InstructionForm::Stfsux
        | InstructionForm::Sthux
        | InstructionForm::Stwux => Some(1),
        InstructionForm::Lwzu
        | InstructionForm::Lbzu
        | InstructionForm::Stwu
        | InstructionForm::Stbu
        | InstructionForm::Lhzu
        | InstructionForm::Lhau
        | InstructionForm::Sthu
        | InstructionForm::Lfsu
        | InstructionForm::Lfdu
        | InstructionForm::Stfsu
        | InstructionForm::Stfdu
        | InstructionForm::PsqLu
        | InstructionForm::PsqStu => Some(2),
        _ => None,
    };
    if let Some(base_index) = update_base_index {
        let base = general_register(base_index);
        if base.value() == 0 {
            return Err(EncodeError::ZeroUpdateBase { form });
        }
        if matches!(
            form,
            InstructionForm::Lbzux
                | InstructionForm::Lhaux
                | InstructionForm::Lhzux
                | InstructionForm::Lwzux
                | InstructionForm::Lwzu
                | InstructionForm::Lbzu
                | InstructionForm::Lhzu
                | InstructionForm::Lhau
        ) && general_register(0) == base
        {
            return Err(EncodeError::UpdateBaseAliasesDestination {
                form,
                register: base,
            });
        }
    }

    if form == InstructionForm::Lmw {
        let destination = general_register(0);
        let base = general_register(2);
        if base.value() >= destination.value() {
            return Err(EncodeError::LoadRangeIncludesAddress {
                form,
                register: base,
            });
        }
    }

    if form == InstructionForm::Lswi {
        let destination = general_register(0);
        let base = general_register(1);
        let Operand::StringByteCount(byte_count) = operands[2] else {
            unreachable!("operand kinds were validated");
        };
        let register_count = byte_count.bytes().div_ceil(4);
        if wrapping_register_range_contains(destination, register_count, base) {
            return Err(EncodeError::LoadRangeIncludesAddress {
                form,
                register: base,
            });
        }
    }

    if form == InstructionForm::Lswx {
        let destination = general_register(0);
        let base = general_register(1);
        let index = general_register(2);
        if destination == base || destination == index {
            return Err(EncodeError::StringLoadDestinationAliasesAddress {
                register: destination,
            });
        }
    }

    Ok(())
}

fn wrapping_register_range_contains(
    first: GeneralRegister,
    register_count: u8,
    candidate: GeneralRegister,
) -> bool {
    (candidate.value() + 32 - first.value()) % 32 < register_count
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Field {
    Gpr21,
    Gpr16,
    Gpr11,
    Fpr21,
    Fpr16,
    Fpr11,
    Fpr6,
    CrField23,
    CrField18,
    CrBit21,
    CrBit16,
    CrBit11,
    Signed16,
    Unsigned16,
    MemoryOffset16,
    PairedOffset12,
    BranchOptions21,
    ConditionBit16,
    BranchTarget14,
    BranchTarget24,
    ShiftAmount11,
    MaskBit6,
    MaskBit1,
    StringByteCount11,
    SpecialRegisterSplit,
    TimeBaseRegisterSplit,
    SegmentRegister16,
    ConditionMask12,
    FpscrMask17,
    FpscrImmediate12,
    TrapCondition21,
    PairedWidth15,
    QuantizationRegister12,
    PairedWidth10,
    QuantizationRegister7,
}

impl Field {
    pub(crate) const fn operand_kind(self) -> OperandKind {
        match self {
            Self::Gpr21 | Self::Gpr16 | Self::Gpr11 => OperandKind::GeneralRegister,
            Self::Fpr21 | Self::Fpr16 | Self::Fpr11 | Self::Fpr6 => OperandKind::FloatingRegister,
            Self::CrField23 | Self::CrField18 => OperandKind::ConditionRegisterField,
            Self::CrBit21 | Self::CrBit16 | Self::CrBit11 | Self::ConditionBit16 => {
                OperandKind::ConditionRegisterBit
            }
            Self::Signed16 | Self::PairedOffset12 => OperandKind::SignedImmediate,
            Self::Unsigned16 => OperandKind::UnsignedImmediate,
            Self::MemoryOffset16 => OperandKind::MemoryOffset,
            Self::BranchOptions21 => OperandKind::BranchOptions,
            Self::BranchTarget14 | Self::BranchTarget24 => OperandKind::BranchTarget,
            Self::ShiftAmount11 | Self::MaskBit6 | Self::MaskBit1 => OperandKind::ShiftOrMaskBit,
            Self::StringByteCount11 => OperandKind::StringByteCount,
            Self::SpecialRegisterSplit => OperandKind::SpecialPurposeRegister,
            Self::TimeBaseRegisterSplit => OperandKind::TimeBaseRegister,
            Self::SegmentRegister16 => OperandKind::SegmentRegister,
            Self::ConditionMask12 => OperandKind::ConditionRegisterMask,
            Self::FpscrMask17 => OperandKind::FpscrFieldMask,
            Self::FpscrImmediate12 => OperandKind::FpscrImmediate,
            Self::TrapCondition21 => OperandKind::TrapCondition,
            Self::PairedWidth15 | Self::PairedWidth10 => OperandKind::PairedSingleWidth,
            Self::QuantizationRegister12 | Self::QuantizationRegister7 => {
                OperandKind::QuantizationRegister
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Definition {
    pub mask: u32,
    pub bits: u32,
    pub fields: &'static [Field],
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ModifierSupport {
    pub record: bool,
    pub overflow: bool,
    pub link: bool,
    pub absolute: bool,
}

macro_rules! define_forms {
    ($($form:ident => ($mnemonic:literal, $mask:expr, $bits:expr, [$($field:ident),*]);)+) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum InstructionForm {
            $($form,)+
        }

        impl InstructionForm {
            pub const ALL: [Self; crate::DECLARED_FORM_COUNT] = [$(Self::$form,)+];

            pub const fn mnemonic(self) -> &'static str {
                match self {
                    $(Self::$form => $mnemonic,)+
                }
            }

            pub(crate) const fn definition(self) -> Definition {
                match self {
                    $(Self::$form => Definition {
                        mask: $mask,
                        bits: $bits,
                        fields: &[$(Field::$field),*],
                    },)+
                }
            }

            pub(crate) fn detect(word: u32) -> Option<Self> {
                Self::ALL.into_iter().find(|form| {
                    let definition = form.definition();
                    word & definition.mask == definition.bits
                })
            }
        }
    };
}

define_forms! {
    Twi => ("twi", 0xfc000000, 0x0c000000, [TrapCondition21, Gpr16, Signed16]);
    DcbzL => ("dcbz_l", 0xffe007ff, 0x100007ec, [Gpr16, Gpr11]);
    PsqLux => ("psq_lux", 0xfc00007f, 0x1000004c, [Fpr21, Gpr16, Gpr11, PairedWidth10, QuantizationRegister7]);
    PsqLx => ("psq_lx", 0xfc00007f, 0x1000000c, [Fpr21, Gpr16, Gpr11, PairedWidth10, QuantizationRegister7]);
    PsqStux => ("psq_stux", 0xfc00007f, 0x1000004e, [Fpr21, Gpr16, Gpr11, PairedWidth10, QuantizationRegister7]);
    PsqStx => ("psq_stx", 0xfc00007f, 0x1000000e, [Fpr21, Gpr16, Gpr11, PairedWidth10, QuantizationRegister7]);
    PsAbs => ("ps_abs", 0xfc1f07fe, 0x10000210, [Fpr21, Fpr11]);
    PsAdd => ("ps_add", 0xfc0007fe, 0x1000002a, [Fpr21, Fpr16, Fpr11]);
    PsCmpo0 => ("ps_cmpo0", 0xfc6007ff, 0x10000040, [CrField23, Fpr16, Fpr11]);
    PsCmpo1 => ("ps_cmpo1", 0xfc6007ff, 0x100000c0, [CrField23, Fpr16, Fpr11]);
    PsCmpu0 => ("ps_cmpu0", 0xfc6007ff, 0x10000000, [CrField23, Fpr16, Fpr11]);
    PsCmpu1 => ("ps_cmpu1", 0xfc6007ff, 0x10000080, [CrField23, Fpr16, Fpr11]);
    PsDiv => ("ps_div", 0xfc0007fe, 0x10000024, [Fpr21, Fpr16, Fpr11]);
    PsMadd => ("ps_madd", 0xfc00003e, 0x1000003a, [Fpr21, Fpr16, Fpr6, Fpr11]);
    PsMadds0 => ("ps_madds0", 0xfc00003e, 0x1000001c, [Fpr21, Fpr16, Fpr6, Fpr11]);
    PsMadds1 => ("ps_madds1", 0xfc00003e, 0x1000001e, [Fpr21, Fpr16, Fpr6, Fpr11]);
    PsMerge00 => ("ps_merge00", 0xfc0007fe, 0x10000420, [Fpr21, Fpr16, Fpr11]);
    PsMerge01 => ("ps_merge01", 0xfc0007fe, 0x10000460, [Fpr21, Fpr16, Fpr11]);
    PsMerge10 => ("ps_merge10", 0xfc0007fe, 0x100004a0, [Fpr21, Fpr16, Fpr11]);
    PsMerge11 => ("ps_merge11", 0xfc0007fe, 0x100004e0, [Fpr21, Fpr16, Fpr11]);
    PsMr => ("ps_mr", 0xfc1f07fe, 0x10000090, [Fpr21, Fpr11]);
    PsMsub => ("ps_msub", 0xfc00003e, 0x10000038, [Fpr21, Fpr16, Fpr6, Fpr11]);
    PsMul => ("ps_mul", 0xfc00f83e, 0x10000032, [Fpr21, Fpr16, Fpr6]);
    PsMuls0 => ("ps_muls0", 0xfc00f83e, 0x10000018, [Fpr21, Fpr16, Fpr6]);
    PsMuls1 => ("ps_muls1", 0xfc00f83e, 0x1000001a, [Fpr21, Fpr16, Fpr6]);
    PsNabs => ("ps_nabs", 0xfc1f07fe, 0x10000110, [Fpr21, Fpr11]);
    PsNeg => ("ps_neg", 0xfc1f07fe, 0x10000050, [Fpr21, Fpr11]);
    PsNmadd => ("ps_nmadd", 0xfc00003e, 0x1000003e, [Fpr21, Fpr16, Fpr6, Fpr11]);
    PsNmsub => ("ps_nmsub", 0xfc00003e, 0x1000003c, [Fpr21, Fpr16, Fpr6, Fpr11]);
    PsRes => ("ps_res", 0xfc1f07fe, 0x10000030, [Fpr21, Fpr11]);
    PsRsqrte => ("ps_rsqrte", 0xfc1f07fe, 0x10000034, [Fpr21, Fpr11]);
    PsSel => ("ps_sel", 0xfc00003e, 0x1000002e, [Fpr21, Fpr16, Fpr6, Fpr11]);
    PsSub => ("ps_sub", 0xfc0007fe, 0x10000028, [Fpr21, Fpr16, Fpr11]);
    PsSum0 => ("ps_sum0", 0xfc00003e, 0x10000014, [Fpr21, Fpr16, Fpr6, Fpr11]);
    PsSum1 => ("ps_sum1", 0xfc00003e, 0x10000016, [Fpr21, Fpr16, Fpr6, Fpr11]);
    Mulli => ("mulli", 0xfc000000, 0x1c000000, [Gpr21, Gpr16, Signed16]);
    Subfic => ("subfic", 0xfc000000, 0x20000000, [Gpr21, Gpr16, Signed16]);
    Cmpli => ("cmpli", 0xfc400000, 0x28000000, [CrField23, Gpr16, Unsigned16]);
    Cmpi => ("cmpi", 0xfc400000, 0x2c000000, [CrField23, Gpr16, Signed16]);
    Addic => ("addic", 0xfc000000, 0x30000000, [Gpr21, Gpr16, Signed16]);
    Addic_ => ("addic.", 0xfc000000, 0x34000000, [Gpr21, Gpr16, Signed16]);
    Addi => ("addi", 0xfc000000, 0x38000000, [Gpr21, Gpr16, Signed16]);
    Addis => ("addis", 0xfc000000, 0x3c000000, [Gpr21, Gpr16, Signed16]);
    Bc => ("bc", 0xfc000000, 0x40000000, [BranchOptions21, ConditionBit16, BranchTarget14]);
    Sc => ("sc", 0xffffffff, 0x44000002, []);
    B => ("b", 0xfc000000, 0x48000000, [BranchTarget24]);
    Bcctr => ("bcctr", 0xfc00fffe, 0x4c000420, [BranchOptions21, ConditionBit16]);
    Bclr => ("bclr", 0xfc00fffe, 0x4c000020, [BranchOptions21, ConditionBit16]);
    Crand => ("crand", 0xfc0007ff, 0x4c000202, [CrBit21, CrBit16, CrBit11]);
    Crandc => ("crandc", 0xfc0007ff, 0x4c000102, [CrBit21, CrBit16, CrBit11]);
    Creqv => ("creqv", 0xfc0007ff, 0x4c000242, [CrBit21, CrBit16, CrBit11]);
    Crnand => ("crnand", 0xfc0007ff, 0x4c0001c2, [CrBit21, CrBit16, CrBit11]);
    Crnor => ("crnor", 0xfc0007ff, 0x4c000042, [CrBit21, CrBit16, CrBit11]);
    Cror => ("cror", 0xfc0007ff, 0x4c000382, [CrBit21, CrBit16, CrBit11]);
    Crorc => ("crorc", 0xfc0007ff, 0x4c000342, [CrBit21, CrBit16, CrBit11]);
    Crxor => ("crxor", 0xfc0007ff, 0x4c000182, [CrBit21, CrBit16, CrBit11]);
    Isync => ("isync", 0xffffffff, 0x4c00012c, []);
    Mcrf => ("mcrf", 0xfc63ffff, 0x4c000000, [CrField23, CrField18]);
    Rfi => ("rfi", 0xffffffff, 0x4c000064, []);
    Rlwimi => ("rlwimi", 0xfc000000, 0x50000000, [Gpr16, Gpr21, ShiftAmount11, MaskBit6, MaskBit1]);
    Rlwinm => ("rlwinm", 0xfc000000, 0x54000000, [Gpr16, Gpr21, ShiftAmount11, MaskBit6, MaskBit1]);
    Rlwnm => ("rlwnm", 0xfc000000, 0x5c000000, [Gpr16, Gpr21, Gpr11, MaskBit6, MaskBit1]);
    Ori => ("ori", 0xfc000000, 0x60000000, [Gpr16, Gpr21, Unsigned16]);
    Oris => ("oris", 0xfc000000, 0x64000000, [Gpr16, Gpr21, Unsigned16]);
    Xori => ("xori", 0xfc000000, 0x68000000, [Gpr16, Gpr21, Unsigned16]);
    Xoris => ("xoris", 0xfc000000, 0x6c000000, [Gpr16, Gpr21, Unsigned16]);
    Andi_ => ("andi.", 0xfc000000, 0x70000000, [Gpr16, Gpr21, Unsigned16]);
    Andis_ => ("andis.", 0xfc000000, 0x74000000, [Gpr16, Gpr21, Unsigned16]);
    Add => ("add", 0xfc0003fe, 0x7c000214, [Gpr21, Gpr16, Gpr11]);
    Addc => ("addc", 0xfc0003fe, 0x7c000014, [Gpr21, Gpr16, Gpr11]);
    Adde => ("adde", 0xfc0003fe, 0x7c000114, [Gpr21, Gpr16, Gpr11]);
    Addme => ("addme", 0xfc00fbfe, 0x7c0001d4, [Gpr21, Gpr16]);
    Addze => ("addze", 0xfc00fbfe, 0x7c000194, [Gpr21, Gpr16]);
    And => ("and", 0xfc0007fe, 0x7c000038, [Gpr16, Gpr21, Gpr11]);
    Andc => ("andc", 0xfc0007fe, 0x7c000078, [Gpr16, Gpr21, Gpr11]);
    Cmp => ("cmp", 0xfc4007ff, 0x7c000000, [CrField23, Gpr16, Gpr11]);
    Cmpl => ("cmpl", 0xfc4007ff, 0x7c000040, [CrField23, Gpr16, Gpr11]);
    Cntlzw => ("cntlzw", 0xfc00fffe, 0x7c000034, [Gpr16, Gpr21]);
    Dcbf => ("dcbf", 0xffe007ff, 0x7c0000ac, [Gpr16, Gpr11]);
    Dcbi => ("dcbi", 0xffe007ff, 0x7c0003ac, [Gpr16, Gpr11]);
    Dcbst => ("dcbst", 0xffe007ff, 0x7c00006c, [Gpr16, Gpr11]);
    Dcbt => ("dcbt", 0xffe007ff, 0x7c00022c, [Gpr16, Gpr11]);
    Dcbtst => ("dcbtst", 0xffe007ff, 0x7c0001ec, [Gpr16, Gpr11]);
    Dcbz => ("dcbz", 0xffe007ff, 0x7c0007ec, [Gpr16, Gpr11]);
    Divw => ("divw", 0xfc0003fe, 0x7c0003d6, [Gpr21, Gpr16, Gpr11]);
    Divwu => ("divwu", 0xfc0003fe, 0x7c000396, [Gpr21, Gpr16, Gpr11]);
    Eciwx => ("eciwx", 0xfc0007ff, 0x7c00026c, [Gpr21, Gpr16, Gpr11]);
    Ecowx => ("ecowx", 0xfc0007ff, 0x7c00036c, [Gpr21, Gpr16, Gpr11]);
    Eieio => ("eieio", 0xffffffff, 0x7c0006ac, []);
    Eqv => ("eqv", 0xfc0007fe, 0x7c000238, [Gpr16, Gpr21, Gpr11]);
    Extsb => ("extsb", 0xfc00fffe, 0x7c000774, [Gpr16, Gpr21]);
    Extsh => ("extsh", 0xfc00fffe, 0x7c000734, [Gpr16, Gpr21]);
    Icbi => ("icbi", 0xffe007fe, 0x7c0007ac, [Gpr16, Gpr11]);
    Lbzux => ("lbzux", 0xfc0007ff, 0x7c0000ee, [Gpr21, Gpr16, Gpr11]);
    Lbzx => ("lbzx", 0xfc0007ff, 0x7c0000ae, [Gpr21, Gpr16, Gpr11]);
    Lfdux => ("lfdux", 0xfc0007ff, 0x7c0004ee, [Fpr21, Gpr16, Gpr11]);
    Lfdx => ("lfdx", 0xfc0007ff, 0x7c0004ae, [Fpr21, Gpr16, Gpr11]);
    Lfsux => ("lfsux", 0xfc0007ff, 0x7c00046e, [Fpr21, Gpr16, Gpr11]);
    Lfsx => ("lfsx", 0xfc0007ff, 0x7c00042e, [Fpr21, Gpr16, Gpr11]);
    Lhaux => ("lhaux", 0xfc0007ff, 0x7c0002ee, [Gpr21, Gpr16, Gpr11]);
    Lhax => ("lhax", 0xfc0007ff, 0x7c0002ae, [Gpr21, Gpr16, Gpr11]);
    Lhbrx => ("lhbrx", 0xfc0007ff, 0x7c00062c, [Gpr21, Gpr16, Gpr11]);
    Lhzux => ("lhzux", 0xfc0007ff, 0x7c00026e, [Gpr21, Gpr16, Gpr11]);
    Lhzx => ("lhzx", 0xfc0007ff, 0x7c00022e, [Gpr21, Gpr16, Gpr11]);
    Lswi => ("lswi", 0xfc0007ff, 0x7c0004aa, [Gpr21, Gpr16, StringByteCount11]);
    Lswx => ("lswx", 0xfc0007ff, 0x7c00042a, [Gpr21, Gpr16, Gpr11]);
    Lwarx => ("lwarx", 0xfc0007ff, 0x7c000028, [Gpr21, Gpr16, Gpr11]);
    Lwbrx => ("lwbrx", 0xfc0007ff, 0x7c00042c, [Gpr21, Gpr16, Gpr11]);
    Lwzux => ("lwzux", 0xfc0007ff, 0x7c00006e, [Gpr21, Gpr16, Gpr11]);
    Lwzx => ("lwzx", 0xfc0007ff, 0x7c00002e, [Gpr21, Gpr16, Gpr11]);
    Mcrxr => ("mcrxr", 0xfc7fffff, 0x7c000400, [CrField23]);
    Mfcr => ("mfcr", 0xfc1fffff, 0x7c000026, [Gpr21]);
    Mfmsr => ("mfmsr", 0xfc1fffff, 0x7c0000a6, [Gpr21]);
    Mfspr => ("mfspr", 0xfc0007ff, 0x7c0002a6, [Gpr21, SpecialRegisterSplit]);
    Mfsr => ("mfsr", 0xfc10ffff, 0x7c0004a6, [Gpr21, SegmentRegister16]);
    Mfsrin => ("mfsrin", 0xfc1f07ff, 0x7c000526, [Gpr21, Gpr11]);
    Mftb => ("mftb", 0xfc0007ff, 0x7c0002e6, [Gpr21, TimeBaseRegisterSplit]);
    Mtcrf => ("mtcrf", 0xfc100fff, 0x7c000120, [ConditionMask12, Gpr21]);
    Mtmsr => ("mtmsr", 0xfc1fffff, 0x7c000124, [Gpr21]);
    Mtspr => ("mtspr", 0xfc0007ff, 0x7c0003a6, [SpecialRegisterSplit, Gpr21]);
    Mtsr => ("mtsr", 0xfc10ffff, 0x7c0001a4, [SegmentRegister16, Gpr21]);
    Mtsrin => ("mtsrin", 0xfc1f07ff, 0x7c0001e4, [Gpr21, Gpr11]);
    Mulhw => ("mulhw", 0xfc0007fe, 0x7c000096, [Gpr21, Gpr16, Gpr11]);
    Mulhwu => ("mulhwu", 0xfc0007fe, 0x7c000016, [Gpr21, Gpr16, Gpr11]);
    Mullw => ("mullw", 0xfc0003fe, 0x7c0001d6, [Gpr21, Gpr16, Gpr11]);
    Nand => ("nand", 0xfc0007fe, 0x7c0003b8, [Gpr16, Gpr21, Gpr11]);
    Neg => ("neg", 0xfc00fbfe, 0x7c0000d0, [Gpr21, Gpr16]);
    Nor => ("nor", 0xfc0007fe, 0x7c0000f8, [Gpr16, Gpr21, Gpr11]);
    Or => ("or", 0xfc0007fe, 0x7c000378, [Gpr16, Gpr21, Gpr11]);
    Orc => ("orc", 0xfc0007fe, 0x7c000338, [Gpr16, Gpr21, Gpr11]);
    Slw => ("slw", 0xfc0007fe, 0x7c000030, [Gpr16, Gpr21, Gpr11]);
    Sraw => ("sraw", 0xfc0007fe, 0x7c000630, [Gpr16, Gpr21, Gpr11]);
    Srawi => ("srawi", 0xfc0007fe, 0x7c000670, [Gpr16, Gpr21, ShiftAmount11]);
    Srw => ("srw", 0xfc0007fe, 0x7c000430, [Gpr16, Gpr21, Gpr11]);
    Stbux => ("stbux", 0xfc0007ff, 0x7c0001ee, [Gpr21, Gpr16, Gpr11]);
    Stbx => ("stbx", 0xfc0007ff, 0x7c0001ae, [Gpr21, Gpr16, Gpr11]);
    Stfdux => ("stfdux", 0xfc0007ff, 0x7c0005ee, [Fpr21, Gpr16, Gpr11]);
    Stfdx => ("stfdx", 0xfc0007ff, 0x7c0005ae, [Fpr21, Gpr16, Gpr11]);
    Stfiwx => ("stfiwx", 0xfc0007ff, 0x7c0007ae, [Fpr21, Gpr16, Gpr11]);
    Stfsux => ("stfsux", 0xfc0007ff, 0x7c00056e, [Fpr21, Gpr16, Gpr11]);
    Stfsx => ("stfsx", 0xfc0007ff, 0x7c00052e, [Fpr21, Gpr16, Gpr11]);
    Sthbrx => ("sthbrx", 0xfc0007ff, 0x7c00072c, [Gpr21, Gpr16, Gpr11]);
    Sthux => ("sthux", 0xfc0007ff, 0x7c00036e, [Gpr21, Gpr16, Gpr11]);
    Sthx => ("sthx", 0xfc0007ff, 0x7c00032e, [Gpr21, Gpr16, Gpr11]);
    Stswi => ("stswi", 0xfc0007ff, 0x7c0005aa, [Gpr21, Gpr16, StringByteCount11]);
    Stswx => ("stswx", 0xfc0007ff, 0x7c00052a, [Gpr21, Gpr16, Gpr11]);
    Stwbrx => ("stwbrx", 0xfc0007ff, 0x7c00052c, [Gpr21, Gpr16, Gpr11]);
    Stwcx_ => ("stwcx.", 0xfc0007ff, 0x7c00012d, [Gpr21, Gpr16, Gpr11]);
    Stwux => ("stwux", 0xfc0007ff, 0x7c00016e, [Gpr21, Gpr16, Gpr11]);
    Stwx => ("stwx", 0xfc0007ff, 0x7c00012e, [Gpr21, Gpr16, Gpr11]);
    Subf => ("subf", 0xfc0003fe, 0x7c000050, [Gpr21, Gpr16, Gpr11]);
    Subfc => ("subfc", 0xfc0003fe, 0x7c000010, [Gpr21, Gpr16, Gpr11]);
    Subfe => ("subfe", 0xfc0003fe, 0x7c000110, [Gpr21, Gpr16, Gpr11]);
    Subfme => ("subfme", 0xfc00fbfe, 0x7c0001d0, [Gpr21, Gpr16]);
    Subfze => ("subfze", 0xfc00fbfe, 0x7c000190, [Gpr21, Gpr16]);
    Sync => ("sync", 0xffffffff, 0x7c0004ac, []);
    Tlbie => ("tlbie", 0xffff07ff, 0x7c000264, [Gpr11]);
    Tlbsync => ("tlbsync", 0xffffffff, 0x7c00046c, []);
    Tw => ("tw", 0xfc0007ff, 0x7c000008, [TrapCondition21, Gpr16, Gpr11]);
    Xor => ("xor", 0xfc0007fe, 0x7c000278, [Gpr16, Gpr21, Gpr11]);
    Lwz => ("lwz", 0xfc000000, 0x80000000, [Gpr21, MemoryOffset16, Gpr16]);
    Lwzu => ("lwzu", 0xfc000000, 0x84000000, [Gpr21, MemoryOffset16, Gpr16]);
    Lbz => ("lbz", 0xfc000000, 0x88000000, [Gpr21, MemoryOffset16, Gpr16]);
    Lbzu => ("lbzu", 0xfc000000, 0x8c000000, [Gpr21, MemoryOffset16, Gpr16]);
    Stw => ("stw", 0xfc000000, 0x90000000, [Gpr21, MemoryOffset16, Gpr16]);
    Stwu => ("stwu", 0xfc000000, 0x94000000, [Gpr21, MemoryOffset16, Gpr16]);
    Stb => ("stb", 0xfc000000, 0x98000000, [Gpr21, MemoryOffset16, Gpr16]);
    Stbu => ("stbu", 0xfc000000, 0x9c000000, [Gpr21, MemoryOffset16, Gpr16]);
    Lhz => ("lhz", 0xfc000000, 0xa0000000, [Gpr21, MemoryOffset16, Gpr16]);
    Lhzu => ("lhzu", 0xfc000000, 0xa4000000, [Gpr21, MemoryOffset16, Gpr16]);
    Lha => ("lha", 0xfc000000, 0xa8000000, [Gpr21, MemoryOffset16, Gpr16]);
    Lhau => ("lhau", 0xfc000000, 0xac000000, [Gpr21, MemoryOffset16, Gpr16]);
    Sth => ("sth", 0xfc000000, 0xb0000000, [Gpr21, MemoryOffset16, Gpr16]);
    Sthu => ("sthu", 0xfc000000, 0xb4000000, [Gpr21, MemoryOffset16, Gpr16]);
    Lmw => ("lmw", 0xfc000000, 0xb8000000, [Gpr21, MemoryOffset16, Gpr16]);
    Stmw => ("stmw", 0xfc000000, 0xbc000000, [Gpr21, MemoryOffset16, Gpr16]);
    Lfs => ("lfs", 0xfc000000, 0xc0000000, [Fpr21, MemoryOffset16, Gpr16]);
    Lfsu => ("lfsu", 0xfc000000, 0xc4000000, [Fpr21, MemoryOffset16, Gpr16]);
    Lfd => ("lfd", 0xfc000000, 0xc8000000, [Fpr21, MemoryOffset16, Gpr16]);
    Lfdu => ("lfdu", 0xfc000000, 0xcc000000, [Fpr21, MemoryOffset16, Gpr16]);
    Stfs => ("stfs", 0xfc000000, 0xd0000000, [Fpr21, MemoryOffset16, Gpr16]);
    Stfsu => ("stfsu", 0xfc000000, 0xd4000000, [Fpr21, MemoryOffset16, Gpr16]);
    Stfd => ("stfd", 0xfc000000, 0xd8000000, [Fpr21, MemoryOffset16, Gpr16]);
    Stfdu => ("stfdu", 0xfc000000, 0xdc000000, [Fpr21, MemoryOffset16, Gpr16]);
    PsqL => ("psq_l", 0xfc000000, 0xe0000000, [Fpr21, PairedOffset12, Gpr16, PairedWidth15, QuantizationRegister12]);
    PsqLu => ("psq_lu", 0xfc000000, 0xe4000000, [Fpr21, PairedOffset12, Gpr16, PairedWidth15, QuantizationRegister12]);
    Fadds => ("fadds", 0xfc0007fe, 0xec00002a, [Fpr21, Fpr16, Fpr11]);
    Fdivs => ("fdivs", 0xfc0007fe, 0xec000024, [Fpr21, Fpr16, Fpr11]);
    Fmadds => ("fmadds", 0xfc00003e, 0xec00003a, [Fpr21, Fpr16, Fpr6, Fpr11]);
    Fmsubs => ("fmsubs", 0xfc00003e, 0xec000038, [Fpr21, Fpr16, Fpr6, Fpr11]);
    Fmuls => ("fmuls", 0xfc00f83e, 0xec000032, [Fpr21, Fpr16, Fpr6]);
    Fnmadds => ("fnmadds", 0xfc00003e, 0xec00003e, [Fpr21, Fpr16, Fpr6, Fpr11]);
    Fnmsubs => ("fnmsubs", 0xfc00003e, 0xec00003c, [Fpr21, Fpr16, Fpr6, Fpr11]);
    Fres => ("fres", 0xfc1f07fe, 0xec000030, [Fpr21, Fpr11]);
    Fsubs => ("fsubs", 0xfc0007fe, 0xec000028, [Fpr21, Fpr16, Fpr11]);
    PsqSt => ("psq_st", 0xfc000000, 0xf0000000, [Fpr21, PairedOffset12, Gpr16, PairedWidth15, QuantizationRegister12]);
    PsqStu => ("psq_stu", 0xfc000000, 0xf4000000, [Fpr21, PairedOffset12, Gpr16, PairedWidth15, QuantizationRegister12]);
    Fabs => ("fabs", 0xfc1f07fe, 0xfc000210, [Fpr21, Fpr11]);
    Fadd => ("fadd", 0xfc0007fe, 0xfc00002a, [Fpr21, Fpr16, Fpr11]);
    Fcmpo => ("fcmpo", 0xfc6007ff, 0xfc000040, [CrField23, Fpr16, Fpr11]);
    Fcmpu => ("fcmpu", 0xfc6007ff, 0xfc000000, [CrField23, Fpr16, Fpr11]);
    Fctiw => ("fctiw", 0xfc1f07fe, 0xfc00001c, [Fpr21, Fpr11]);
    Fctiwz => ("fctiwz", 0xfc1f07fe, 0xfc00001e, [Fpr21, Fpr11]);
    Fdiv => ("fdiv", 0xfc0007fe, 0xfc000024, [Fpr21, Fpr16, Fpr11]);
    Fmadd => ("fmadd", 0xfc00003e, 0xfc00003a, [Fpr21, Fpr16, Fpr6, Fpr11]);
    Fmr => ("fmr", 0xfc1f07fe, 0xfc000090, [Fpr21, Fpr11]);
    Fmsub => ("fmsub", 0xfc00003e, 0xfc000038, [Fpr21, Fpr16, Fpr6, Fpr11]);
    Fmul => ("fmul", 0xfc00f83e, 0xfc000032, [Fpr21, Fpr16, Fpr6]);
    Fnabs => ("fnabs", 0xfc1f07fe, 0xfc000110, [Fpr21, Fpr11]);
    Fneg => ("fneg", 0xfc1f07fe, 0xfc000050, [Fpr21, Fpr11]);
    Fnmadd => ("fnmadd", 0xfc00003e, 0xfc00003e, [Fpr21, Fpr16, Fpr6, Fpr11]);
    Fnmsub => ("fnmsub", 0xfc00003e, 0xfc00003c, [Fpr21, Fpr16, Fpr6, Fpr11]);
    Frsp => ("frsp", 0xfc1f07fe, 0xfc000018, [Fpr21, Fpr11]);
    Frsqrte => ("frsqrte", 0xfc1f07fe, 0xfc000034, [Fpr21, Fpr11]);
    Fsel => ("fsel", 0xfc00003e, 0xfc00002e, [Fpr21, Fpr16, Fpr6, Fpr11]);
    Fsub => ("fsub", 0xfc0007fe, 0xfc000028, [Fpr21, Fpr16, Fpr11]);
    Mcrfs => ("mcrfs", 0xfc63ffff, 0xfc000080, [CrField23, CrField18]);
    Mffs => ("mffs", 0xfc1ffffe, 0xfc00048e, [Fpr21]);
    Mtfsb0 => ("mtfsb0", 0xfc1ffffe, 0xfc00008c, [CrBit21]);
    Mtfsb1 => ("mtfsb1", 0xfc1ffffe, 0xfc00004c, [CrBit21]);
    Mtfsf => ("mtfsf", 0xfe0107fe, 0xfc00058e, [FpscrMask17, Fpr11]);
    Mtfsfi => ("mtfsfi", 0xfc7f0ffe, 0xfc00010c, [CrField23, FpscrImmediate12]);
}

impl InstructionForm {
    pub(crate) const fn modifier_support(self) -> ModifierSupport {
        use InstructionForm::*;
        let record = matches!(
            self,
            PsAbs
                | PsAdd
                | PsDiv
                | PsMadd
                | PsMadds0
                | PsMadds1
                | PsMerge00
                | PsMerge01
                | PsMerge10
                | PsMerge11
                | PsMr
                | PsMsub
                | PsMul
                | PsMuls0
                | PsMuls1
                | PsNabs
                | PsNeg
                | PsNmadd
                | PsNmsub
                | PsRes
                | PsRsqrte
                | PsSel
                | PsSub
                | PsSum0
                | PsSum1
                | Rlwimi
                | Rlwinm
                | Rlwnm
                | Add
                | Addc
                | Adde
                | Addme
                | Addze
                | And
                | Andc
                | Cntlzw
                | Divw
                | Divwu
                | Eqv
                | Extsb
                | Extsh
                | Mulhw
                | Mulhwu
                | Mullw
                | Nand
                | Neg
                | Nor
                | Or
                | Orc
                | Slw
                | Sraw
                | Srawi
                | Srw
                | Subf
                | Subfc
                | Subfe
                | Subfme
                | Subfze
                | Xor
                | Fadds
                | Fdivs
                | Fmadds
                | Fmsubs
                | Fmuls
                | Fnmadds
                | Fnmsubs
                | Fres
                | Fsubs
                | Fabs
                | Fadd
                | Fctiw
                | Fctiwz
                | Fdiv
                | Fmadd
                | Fmr
                | Fmsub
                | Fmul
                | Fnabs
                | Fneg
                | Fnmadd
                | Fnmsub
                | Frsp
                | Frsqrte
                | Fsel
                | Fsub
                | Mffs
                | Mtfsb0
                | Mtfsb1
                | Mtfsf
                | Mtfsfi
        );
        let overflow = matches!(
            self,
            Add | Addc
                | Adde
                | Addme
                | Addze
                | Divw
                | Divwu
                | Mullw
                | Neg
                | Subf
                | Subfc
                | Subfe
                | Subfme
                | Subfze
        );
        let link = matches!(self, B | Bc | Bcctr | Bclr);
        let absolute = matches!(self, B | Bc);
        ModifierSupport {
            record,
            overflow,
            link,
            absolute,
        }
    }
}
