use crate::{Mnemonic, OperandSize, Prefix, SegmentRegister};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryOpcodeClass {
    Instruction,
    Prefix(Prefix),
    SecondaryEscape,
    Reserved,
}

/// Manual-derived classification of every native V30 primary opcode byte.
pub const fn classify_primary_opcode(opcode: u8) -> PrimaryOpcodeClass {
    match opcode {
        0x0f => PrimaryOpcodeClass::SecondaryEscape,
        0x26 => PrimaryOpcodeClass::Prefix(Prefix::Segment(SegmentRegister::ES)),
        0x2e => PrimaryOpcodeClass::Prefix(Prefix::Segment(SegmentRegister::CS)),
        0x36 => PrimaryOpcodeClass::Prefix(Prefix::Segment(SegmentRegister::SS)),
        0x3e => PrimaryOpcodeClass::Prefix(Prefix::Segment(SegmentRegister::DS)),
        0x64 => PrimaryOpcodeClass::Prefix(Prefix::Repnc),
        0x65 => PrimaryOpcodeClass::Prefix(Prefix::Repc),
        0xf0 => PrimaryOpcodeClass::Prefix(Prefix::BusLock),
        0xf2 => PrimaryOpcodeClass::Prefix(Prefix::Repne),
        0xf3 => PrimaryOpcodeClass::Prefix(Prefix::Rep),
        0x63 | 0xd6 | 0xf1 => PrimaryOpcodeClass::Reserved,
        _ => PrimaryOpcodeClass::Instruction,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecondaryOperandEncoding {
    None,
    RegisterOrMemory8,
    RegisterOrMemoryBit {
        size: OperandSize,
        immediate_index: bool,
    },
    RegisterPair8,
    RegisterImmediate4,
    Immediate8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecondaryOpcodeClass {
    Instruction {
        mnemonic: Mnemonic,
        operands: SecondaryOperandEncoding,
    },
    Reserved,
}

/// Manual-derived classification of the V30 `0x0F` secondary opcode byte.
pub const fn classify_secondary_opcode(secondary: u8) -> SecondaryOpcodeClass {
    use Mnemonic::{Add4s, Brkem, Clr1, Cmp4s, Ext, Ins, Not1, Rol4, Ror4, Set1, Sub4s, Test1};
    use OperandSize::{Byte, Word};
    use SecondaryOpcodeClass::{Instruction, Reserved};
    use SecondaryOperandEncoding::{
        Immediate8, None, RegisterImmediate4, RegisterOrMemory8, RegisterOrMemoryBit, RegisterPair8,
    };

    match secondary {
        0x10 => Instruction {
            mnemonic: Test1,
            operands: RegisterOrMemoryBit {
                size: Byte,
                immediate_index: false,
            },
        },
        0x11 => Instruction {
            mnemonic: Test1,
            operands: RegisterOrMemoryBit {
                size: Word,
                immediate_index: false,
            },
        },
        0x12 => Instruction {
            mnemonic: Clr1,
            operands: RegisterOrMemoryBit {
                size: Byte,
                immediate_index: false,
            },
        },
        0x13 => Instruction {
            mnemonic: Clr1,
            operands: RegisterOrMemoryBit {
                size: Word,
                immediate_index: false,
            },
        },
        0x14 => Instruction {
            mnemonic: Set1,
            operands: RegisterOrMemoryBit {
                size: Byte,
                immediate_index: false,
            },
        },
        0x15 => Instruction {
            mnemonic: Set1,
            operands: RegisterOrMemoryBit {
                size: Word,
                immediate_index: false,
            },
        },
        0x16 => Instruction {
            mnemonic: Not1,
            operands: RegisterOrMemoryBit {
                size: Byte,
                immediate_index: false,
            },
        },
        0x17 => Instruction {
            mnemonic: Not1,
            operands: RegisterOrMemoryBit {
                size: Word,
                immediate_index: false,
            },
        },
        0x18 => Instruction {
            mnemonic: Test1,
            operands: RegisterOrMemoryBit {
                size: Byte,
                immediate_index: true,
            },
        },
        0x19 => Instruction {
            mnemonic: Test1,
            operands: RegisterOrMemoryBit {
                size: Word,
                immediate_index: true,
            },
        },
        0x1a => Instruction {
            mnemonic: Clr1,
            operands: RegisterOrMemoryBit {
                size: Byte,
                immediate_index: true,
            },
        },
        0x1b => Instruction {
            mnemonic: Clr1,
            operands: RegisterOrMemoryBit {
                size: Word,
                immediate_index: true,
            },
        },
        0x1c => Instruction {
            mnemonic: Set1,
            operands: RegisterOrMemoryBit {
                size: Byte,
                immediate_index: true,
            },
        },
        0x1d => Instruction {
            mnemonic: Set1,
            operands: RegisterOrMemoryBit {
                size: Word,
                immediate_index: true,
            },
        },
        0x1e => Instruction {
            mnemonic: Not1,
            operands: RegisterOrMemoryBit {
                size: Byte,
                immediate_index: true,
            },
        },
        0x1f => Instruction {
            mnemonic: Not1,
            operands: RegisterOrMemoryBit {
                size: Word,
                immediate_index: true,
            },
        },
        0x20 => Instruction {
            mnemonic: Add4s,
            operands: None,
        },
        0x22 => Instruction {
            mnemonic: Sub4s,
            operands: None,
        },
        0x26 => Instruction {
            mnemonic: Cmp4s,
            operands: None,
        },
        0x28 => Instruction {
            mnemonic: Rol4,
            operands: RegisterOrMemory8,
        },
        0x2a => Instruction {
            mnemonic: Ror4,
            operands: RegisterOrMemory8,
        },
        0x31 => Instruction {
            mnemonic: Ins,
            operands: RegisterPair8,
        },
        0x33 => Instruction {
            mnemonic: Ext,
            operands: RegisterPair8,
        },
        0x39 => Instruction {
            mnemonic: Ins,
            operands: RegisterImmediate4,
        },
        0x3b => Instruction {
            mnemonic: Ext,
            operands: RegisterImmediate4,
        },
        0xff => Instruction {
            mnemonic: Brkem,
            operands: Immediate8,
        },
        _ => Reserved,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModRmEncodingClass {
    Valid,
    Reserved,
    NotUsed,
}

/// Classifies the ModR/M byte for primary opcodes that use one.
pub const fn classify_primary_modrm(opcode: u8, modrm: u8) -> ModRmEncodingClass {
    use ModRmEncodingClass::{NotUsed, Reserved, Valid};

    let mode = modrm >> 6;
    let selector = (modrm >> 3) & 7;
    match opcode {
        0x00..=0x03
        | 0x08..=0x0b
        | 0x10..=0x13
        | 0x18..=0x1b
        | 0x20..=0x23
        | 0x28..=0x2b
        | 0x30..=0x33
        | 0x38..=0x3b
        | 0x66
        | 0x67
        | 0x69
        | 0x6b
        | 0x80..=0x8b
        | 0xd8..=0xdf => Valid,
        0x62 | 0x8d | 0xc4 | 0xc5 => {
            if mode == 3 {
                Reserved
            } else {
                Valid
            }
        }
        0x8c => {
            if selector <= 3 {
                Valid
            } else {
                Reserved
            }
        }
        0x8e => {
            if selector <= 3 && selector != 1 {
                Valid
            } else {
                Reserved
            }
        }
        0x8f | 0xc6 | 0xc7 => {
            if selector == 0 {
                Valid
            } else {
                Reserved
            }
        }
        0xc0 | 0xc1 | 0xd0..=0xd3 => {
            if selector == 6 {
                Reserved
            } else {
                Valid
            }
        }
        0xf6 | 0xf7 => {
            if selector == 1 {
                Reserved
            } else {
                Valid
            }
        }
        0xfe => {
            if selector <= 1 {
                Valid
            } else {
                Reserved
            }
        }
        0xff => {
            if selector == 7 || mode == 3 && matches!(selector, 3 | 5) {
                Reserved
            } else {
                Valid
            }
        }
        _ => NotUsed,
    }
}

/// Classifies the ModR/M byte for known `0x0F` secondary opcodes.
pub const fn classify_secondary_modrm(secondary: u8, modrm: u8) -> ModRmEncodingClass {
    use ModRmEncodingClass::{NotUsed, Reserved, Valid};

    let mode = modrm >> 6;
    let selector = (modrm >> 3) & 7;
    match secondary {
        0x10..=0x1f | 0x28 | 0x2a => {
            if selector == 0 {
                Valid
            } else {
                Reserved
            }
        }
        0x31 | 0x33 => {
            if mode == 3 {
                Valid
            } else {
                Reserved
            }
        }
        0x39 | 0x3b => {
            if mode == 3 && selector == 0 {
                Valid
            } else {
                Reserved
            }
        }
        _ => NotUsed,
    }
}
