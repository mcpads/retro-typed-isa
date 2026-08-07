use arm7tdmi::{
    AddressingMode2, AddressingMode3, Condition, Coprocessor, CoprocessorOpcode,
    CoprocessorRegister, Register,
};

/// Half of a 32-bit register selected by an enhanced-DSP multiply instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HalfwordSelection {
    /// Least-significant 16 bits.
    Bottom,
    /// Most-significant 16 bits.
    Top,
}

impl HalfwordSelection {
    pub(crate) const fn from_bit(value: bool) -> Self {
        if value { Self::Top } else { Self::Bottom }
    }
}

/// Signed saturating arithmetic operation introduced by ARMv5TE.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum SaturatingOperation {
    /// Saturating addition (`QADD`).
    Add = 0,
    /// Saturating subtraction (`QSUB`).
    Subtract = 1,
    /// Saturating doubling of the second operand followed by addition (`QDADD`).
    DoubleAndAdd = 2,
    /// Saturating doubling of the second operand followed by subtraction (`QDSUB`).
    DoubleAndSubtract = 3,
}

impl SaturatingOperation {
    pub(crate) const fn from_field(value: u8) -> Self {
        match value {
            0 => Self::Add,
            1 => Self::Subtract,
            2 => Self::DoubleAndAdd,
            3 => Self::DoubleAndSubtract,
            _ => unreachable!(),
        }
    }
}

/// One ARM-state instruction accepted by the ARM946E-S profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmInstruction {
    /// An ARMv4T instruction inherited without changing its semantic identity.
    Armv4T(arm7tdmi::ArmInstruction),
    /// ARM breakpoint with a split 16-bit comment field.
    Breakpoint {
        /// Comment value embedded in the instruction word.
        comment: u16,
    },
    /// Unconditional immediate branch with link and exchange to Thumb state.
    BranchLinkExchangeImmediate {
        /// Signed byte displacement from the architectural ARM PC.
        displacement: i32,
    },
    /// Conditional branch with link and exchange through a register.
    BranchLinkExchangeRegister {
        /// Condition under which the branch is taken.
        condition: Condition,
        /// Register containing the target address and destination state bit.
        target: Register,
    },
    /// Count leading zero bits in a register (`CLZ`).
    CountLeadingZeros {
        /// Condition under which the instruction executes.
        condition: Condition,
        /// General register receiving the count.
        destination: Register,
        /// General register whose leading zeroes are counted.
        source: Register,
    },
    /// Load or store a consecutive register pair as a doubleword.
    DoublewordTransfer {
        /// Condition under which the instruction executes.
        condition: Condition,
        /// `true` for load and `false` for store.
        load: bool,
        /// First general register in the consecutive pair.
        first_register: Register,
        /// Address calculation and write-back semantics.
        address: AddressingMode3,
    },
    /// Unconditional ARMv5 coprocessor data operation (`CDP2`).
    CoprocessorDataOperation2 {
        /// Selected coprocessor.
        coprocessor: Coprocessor,
        /// Four-bit primary coprocessor operation.
        opcode: CoprocessorOpcode,
        /// Three-bit secondary operation field.
        info: u8,
        /// Coprocessor destination register.
        destination: CoprocessorRegister,
        /// First coprocessor source register.
        first: CoprocessorRegister,
        /// Second coprocessor source register.
        second: CoprocessorRegister,
    },
    /// Unconditional ARM/coprocessor register transfer (`MCR2` or `MRC2`).
    CoprocessorRegisterTransfer2 {
        /// `true` for coprocessor-to-ARM load and `false` for ARM-to-coprocessor store.
        load: bool,
        /// Selected coprocessor.
        coprocessor: Coprocessor,
        /// Three-bit primary coprocessor operation.
        opcode: u8,
        /// Three-bit secondary operation field.
        info: u8,
        /// General register transferred to or from the coprocessor.
        arm_register: Register,
        /// First coprocessor register selector.
        first: CoprocessorRegister,
        /// Second coprocessor register selector.
        second: CoprocessorRegister,
    },
    /// Unconditional coprocessor memory transfer (`LDC2` or `STC2`).
    CoprocessorDataTransfer2 {
        /// `true` for load and `false` for store.
        load: bool,
        /// Whether the coprocessor-specific long-transfer bit is set.
        long: bool,
        /// Selected coprocessor.
        coprocessor: Coprocessor,
        /// Coprocessor register transferred.
        register: CoprocessorRegister,
        /// General register used as the address base.
        base: Register,
        /// Unsigned word-scaled offset field.
        offset_words: u8,
        /// `true` to add the offset and `false` to subtract it.
        add: bool,
        /// Pre/post indexing and write-back selection.
        index: arm7tdmi::IndexMode,
    },
    /// Transfer two ARM registers to or from one coprocessor register.
    CoprocessorDoubleRegisterTransfer {
        /// Condition under which the instruction executes.
        condition: Condition,
        /// `true` for coprocessor-to-ARM load and `false` for ARM-to-coprocessor store.
        load: bool,
        /// Selected coprocessor.
        coprocessor: Coprocessor,
        /// Four-bit coprocessor operation.
        opcode: CoprocessorOpcode,
        /// First general register transferred.
        first_arm_register: Register,
        /// Second general register transferred.
        second_arm_register: Register,
        /// Coprocessor register transferred.
        coprocessor_register: CoprocessorRegister,
    },
    /// Preload data hint (`PLD`) using an address-mode-2 calculation.
    PreloadData {
        /// Address whose data may be preloaded.
        address: AddressingMode2,
    },
    /// Signed saturating add, subtract, or doubled-operand arithmetic.
    SaturatingArithmetic {
        /// Condition under which the instruction executes.
        condition: Condition,
        /// Saturating operation selected by the opcode.
        operation: SaturatingOperation,
        /// General register receiving the saturated result.
        destination: Register,
        /// First arithmetic source register.
        first: Register,
        /// Second arithmetic source register, optionally doubled by the operation.
        second: Register,
    },
    /// Multiply selected signed halfwords and add a 32-bit register (`SMLAxy`).
    SignedHalfwordMultiplyAccumulate {
        /// Condition under which the instruction executes.
        condition: Condition,
        /// General register receiving the accumulated result.
        destination: Register,
        /// First multiply source register.
        first: Register,
        /// Half selected from the first source register.
        first_half: HalfwordSelection,
        /// Second multiply source register.
        second: Register,
        /// Half selected from the second source register.
        second_half: HalfwordSelection,
        /// General register added to the product.
        addend: Register,
    },
    /// Multiply selected signed halfwords and accumulate into 64 bits (`SMLALxy`).
    SignedHalfwordMultiplyAccumulateLong {
        /// Condition under which the instruction executes.
        condition: Condition,
        /// General register holding the low result and addend word.
        destination_low: Register,
        /// General register holding the high result and addend word.
        destination_high: Register,
        /// First multiply source register.
        first: Register,
        /// Half selected from the first source register.
        first_half: HalfwordSelection,
        /// Second multiply source register.
        second: Register,
        /// Half selected from the second source register.
        second_half: HalfwordSelection,
    },
    /// Multiply a signed word by a selected halfword and accumulate (`SMLAWy`).
    SignedWordHalfwordMultiplyAccumulate {
        /// Condition under which the instruction executes.
        condition: Condition,
        /// General register receiving the accumulated high-word result.
        destination: Register,
        /// Signed 32-bit multiply source register.
        word: Register,
        /// Register supplying the selected signed halfword.
        halfword: Register,
        /// Half selected from `halfword`.
        half: HalfwordSelection,
        /// General register added to the scaled product.
        addend: Register,
    },
    /// Multiply two selected signed halfwords (`SMULxy`).
    SignedHalfwordMultiply {
        /// Condition under which the instruction executes.
        condition: Condition,
        /// General register receiving the product.
        destination: Register,
        /// First multiply source register.
        first: Register,
        /// Half selected from the first source register.
        first_half: HalfwordSelection,
        /// Second multiply source register.
        second: Register,
        /// Half selected from the second source register.
        second_half: HalfwordSelection,
    },
    /// Multiply a signed word by a selected halfword (`SMULWy`).
    SignedWordHalfwordMultiply {
        /// Condition under which the instruction executes.
        condition: Condition,
        /// General register receiving the high-word result.
        destination: Register,
        /// Signed 32-bit multiply source register.
        word: Register,
        /// Register supplying the selected signed halfword.
        halfword: Register,
        /// Half selected from `halfword`.
        half: HalfwordSelection,
    },
}

/// Stable ARM-state instruction identity with register and immediate values removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArmInstructionForm {
    /// Form inherited from the ARMv4T profile.
    Armv4T(arm7tdmi::ArmInstructionForm),
    /// Form of [`ArmInstruction::Breakpoint`].
    Breakpoint,
    /// Form of [`ArmInstruction::BranchLinkExchangeImmediate`].
    BranchLinkExchangeImmediate,
    /// Form of [`ArmInstruction::BranchLinkExchangeRegister`].
    BranchLinkExchangeRegister,
    /// Form of [`ArmInstruction::CountLeadingZeros`].
    CountLeadingZeros,
    /// Doubleword transfer form preserving its load/store selector.
    DoublewordTransfer {
        /// `true` for load and `false` for store.
        load: bool,
    },
    /// Form of [`ArmInstruction::CoprocessorDataOperation2`].
    CoprocessorDataOperation2,
    /// Unconditional coprocessor register-transfer form.
    CoprocessorRegisterTransfer2 {
        /// `true` for coprocessor-to-ARM load and `false` for store.
        load: bool,
    },
    /// Unconditional coprocessor data-transfer form.
    CoprocessorDataTransfer2 {
        /// `true` for load and `false` for store.
        load: bool,
    },
    /// Coprocessor double-register-transfer form.
    CoprocessorDoubleRegisterTransfer {
        /// `true` for coprocessor-to-ARM load and `false` for store.
        load: bool,
    },
    /// Form of [`ArmInstruction::PreloadData`].
    PreloadData,
    /// Saturating arithmetic form preserving its operation selector.
    SaturatingArithmetic(SaturatingOperation),
    /// Signed halfword multiply-accumulate form preserving both half selectors.
    SignedHalfwordMultiplyAccumulate {
        /// Half selected from the first multiply source.
        first_half: HalfwordSelection,
        /// Half selected from the second multiply source.
        second_half: HalfwordSelection,
    },
    /// Long signed halfword multiply-accumulate form.
    SignedHalfwordMultiplyAccumulateLong {
        /// Half selected from the first multiply source.
        first_half: HalfwordSelection,
        /// Half selected from the second multiply source.
        second_half: HalfwordSelection,
    },
    /// Word-by-halfword multiply-accumulate form preserving the half selector.
    SignedWordHalfwordMultiplyAccumulate(HalfwordSelection),
    /// Signed halfword multiply form preserving both half selectors.
    SignedHalfwordMultiply {
        /// Half selected from the first multiply source.
        first_half: HalfwordSelection,
        /// Half selected from the second multiply source.
        second_half: HalfwordSelection,
    },
    /// Word-by-halfword multiply form preserving the half selector.
    SignedWordHalfwordMultiply(HalfwordSelection),
}

impl ArmInstruction {
    /// Return the stable form identity with operand values removed.
    pub const fn form(self) -> ArmInstructionForm {
        match self {
            Self::Armv4T(instruction) => ArmInstructionForm::Armv4T(instruction.form()),
            Self::Breakpoint { .. } => ArmInstructionForm::Breakpoint,
            Self::BranchLinkExchangeImmediate { .. } => {
                ArmInstructionForm::BranchLinkExchangeImmediate
            }
            Self::BranchLinkExchangeRegister { .. } => {
                ArmInstructionForm::BranchLinkExchangeRegister
            }
            Self::CountLeadingZeros { .. } => ArmInstructionForm::CountLeadingZeros,
            Self::DoublewordTransfer { load, .. } => {
                ArmInstructionForm::DoublewordTransfer { load }
            }
            Self::CoprocessorDataOperation2 { .. } => ArmInstructionForm::CoprocessorDataOperation2,
            Self::CoprocessorRegisterTransfer2 { load, .. } => {
                ArmInstructionForm::CoprocessorRegisterTransfer2 { load }
            }
            Self::CoprocessorDataTransfer2 { load, .. } => {
                ArmInstructionForm::CoprocessorDataTransfer2 { load }
            }
            Self::CoprocessorDoubleRegisterTransfer { load, .. } => {
                ArmInstructionForm::CoprocessorDoubleRegisterTransfer { load }
            }
            Self::PreloadData { .. } => ArmInstructionForm::PreloadData,
            Self::SaturatingArithmetic { operation, .. } => {
                ArmInstructionForm::SaturatingArithmetic(operation)
            }
            Self::SignedHalfwordMultiplyAccumulate {
                first_half,
                second_half,
                ..
            } => ArmInstructionForm::SignedHalfwordMultiplyAccumulate {
                first_half,
                second_half,
            },
            Self::SignedHalfwordMultiplyAccumulateLong {
                first_half,
                second_half,
                ..
            } => ArmInstructionForm::SignedHalfwordMultiplyAccumulateLong {
                first_half,
                second_half,
            },
            Self::SignedWordHalfwordMultiplyAccumulate { half, .. } => {
                ArmInstructionForm::SignedWordHalfwordMultiplyAccumulate(half)
            }
            Self::SignedHalfwordMultiply {
                first_half,
                second_half,
                ..
            } => ArmInstructionForm::SignedHalfwordMultiply {
                first_half,
                second_half,
            },
            Self::SignedWordHalfwordMultiply { half, .. } => {
                ArmInstructionForm::SignedWordHalfwordMultiply(half)
            }
        }
    }
}

/// One Thumb-state instruction accepted by the ARM946E-S profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThumbInstruction {
    /// A Thumb instruction inherited from the ARMv4T profile.
    Armv4T(arm7tdmi::ThumbInstruction),
    /// Thumb breakpoint instruction.
    Breakpoint {
        /// Eight-bit comment embedded in the instruction.
        comment: u8,
    },
    /// Branch with link and exchange through a high register.
    BranchLinkExchangeRegister {
        /// Register containing the target address and destination state bit.
        target: Register,
    },
    /// Long branch with link and exchange to ARM state.
    BranchLinkExchangeImmediate {
        /// Signed byte displacement from the word-aligned architectural Thumb PC.
        displacement: i32,
    },
}

/// Stable Thumb-state instruction identity with literal values removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ThumbInstructionForm {
    /// Form inherited from the ARMv4T profile.
    Armv4T(arm7tdmi::ThumbInstructionForm),
    /// Form of [`ThumbInstruction::Breakpoint`].
    Breakpoint,
    /// Form of [`ThumbInstruction::BranchLinkExchangeRegister`].
    BranchLinkExchangeRegister,
    /// Form of [`ThumbInstruction::BranchLinkExchangeImmediate`].
    BranchLinkExchangeImmediate,
}

impl ThumbInstruction {
    /// Return the stable form identity with literal values removed.
    pub const fn form(self) -> ThumbInstructionForm {
        match self {
            Self::Armv4T(instruction) => ThumbInstructionForm::Armv4T(instruction.form()),
            Self::Breakpoint { .. } => ThumbInstructionForm::Breakpoint,
            Self::BranchLinkExchangeRegister { .. } => {
                ThumbInstructionForm::BranchLinkExchangeRegister
            }
            Self::BranchLinkExchangeImmediate { .. } => {
                ThumbInstructionForm::BranchLinkExchangeImmediate
            }
        }
    }

    /// Return the complete encoded length in Thumb-state bytes.
    pub const fn encoded_len(self) -> usize {
        match self {
            Self::Armv4T(instruction) => instruction.encoded_len(),
            Self::BranchLinkExchangeImmediate { .. } => 4,
            Self::Breakpoint { .. } | Self::BranchLinkExchangeRegister { .. } => 2,
        }
    }
}
