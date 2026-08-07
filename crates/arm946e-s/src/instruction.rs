use arm7tdmi::{
    AddressingMode2, AddressingMode3, Condition, Coprocessor, CoprocessorOpcode,
    CoprocessorRegister, Register,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HalfwordSelection {
    Bottom,
    Top,
}

impl HalfwordSelection {
    pub(crate) const fn from_bit(value: bool) -> Self {
        if value { Self::Top } else { Self::Bottom }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum SaturatingOperation {
    Add = 0,
    Subtract = 1,
    DoubleAndAdd = 2,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmInstruction {
    Armv4T(arm7tdmi::ArmInstruction),
    Breakpoint {
        comment: u16,
    },
    BranchLinkExchangeImmediate {
        displacement: i32,
    },
    BranchLinkExchangeRegister {
        condition: Condition,
        target: Register,
    },
    CountLeadingZeros {
        condition: Condition,
        destination: Register,
        source: Register,
    },
    DoublewordTransfer {
        condition: Condition,
        load: bool,
        first_register: Register,
        address: AddressingMode3,
    },
    CoprocessorDataOperation2 {
        coprocessor: Coprocessor,
        opcode: CoprocessorOpcode,
        info: u8,
        destination: CoprocessorRegister,
        first: CoprocessorRegister,
        second: CoprocessorRegister,
    },
    CoprocessorRegisterTransfer2 {
        load: bool,
        coprocessor: Coprocessor,
        opcode: u8,
        info: u8,
        arm_register: Register,
        first: CoprocessorRegister,
        second: CoprocessorRegister,
    },
    CoprocessorDataTransfer2 {
        load: bool,
        long: bool,
        coprocessor: Coprocessor,
        register: CoprocessorRegister,
        base: Register,
        offset_words: u8,
        add: bool,
        index: arm7tdmi::IndexMode,
    },
    CoprocessorDoubleRegisterTransfer {
        condition: Condition,
        load: bool,
        coprocessor: Coprocessor,
        opcode: CoprocessorOpcode,
        first_arm_register: Register,
        second_arm_register: Register,
        coprocessor_register: CoprocessorRegister,
    },
    PreloadData {
        address: AddressingMode2,
    },
    SaturatingArithmetic {
        condition: Condition,
        operation: SaturatingOperation,
        destination: Register,
        first: Register,
        second: Register,
    },
    SignedHalfwordMultiplyAccumulate {
        condition: Condition,
        destination: Register,
        first: Register,
        first_half: HalfwordSelection,
        second: Register,
        second_half: HalfwordSelection,
        addend: Register,
    },
    SignedHalfwordMultiplyAccumulateLong {
        condition: Condition,
        destination_low: Register,
        destination_high: Register,
        first: Register,
        first_half: HalfwordSelection,
        second: Register,
        second_half: HalfwordSelection,
    },
    SignedWordHalfwordMultiplyAccumulate {
        condition: Condition,
        destination: Register,
        word: Register,
        halfword: Register,
        half: HalfwordSelection,
        addend: Register,
    },
    SignedHalfwordMultiply {
        condition: Condition,
        destination: Register,
        first: Register,
        first_half: HalfwordSelection,
        second: Register,
        second_half: HalfwordSelection,
    },
    SignedWordHalfwordMultiply {
        condition: Condition,
        destination: Register,
        word: Register,
        halfword: Register,
        half: HalfwordSelection,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArmInstructionForm {
    Armv4T(arm7tdmi::ArmInstructionForm),
    Breakpoint,
    BranchLinkExchangeImmediate,
    BranchLinkExchangeRegister,
    CountLeadingZeros,
    DoublewordTransfer {
        load: bool,
    },
    CoprocessorDataOperation2,
    CoprocessorRegisterTransfer2 {
        load: bool,
    },
    CoprocessorDataTransfer2 {
        load: bool,
    },
    CoprocessorDoubleRegisterTransfer {
        load: bool,
    },
    PreloadData,
    SaturatingArithmetic(SaturatingOperation),
    SignedHalfwordMultiplyAccumulate {
        first_half: HalfwordSelection,
        second_half: HalfwordSelection,
    },
    SignedHalfwordMultiplyAccumulateLong {
        first_half: HalfwordSelection,
        second_half: HalfwordSelection,
    },
    SignedWordHalfwordMultiplyAccumulate(HalfwordSelection),
    SignedHalfwordMultiply {
        first_half: HalfwordSelection,
        second_half: HalfwordSelection,
    },
    SignedWordHalfwordMultiply(HalfwordSelection),
}

impl ArmInstruction {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThumbInstruction {
    Armv4T(arm7tdmi::ThumbInstruction),
    Breakpoint { comment: u8 },
    BranchLinkExchangeRegister { target: Register },
    BranchLinkExchangeImmediate { displacement: i32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ThumbInstructionForm {
    Armv4T(arm7tdmi::ThumbInstructionForm),
    Breakpoint,
    BranchLinkExchangeRegister,
    BranchLinkExchangeImmediate,
}

impl ThumbInstruction {
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

    pub const fn encoded_len(self) -> usize {
        match self {
            Self::Armv4T(instruction) => instruction.encoded_len(),
            Self::BranchLinkExchangeImmediate { .. } => 4,
            Self::Breakpoint { .. } | Self::BranchLinkExchangeRegister { .. } => 2,
        }
    }
}
