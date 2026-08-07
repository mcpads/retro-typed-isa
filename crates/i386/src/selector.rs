use crate::{Prefix, SegmentRegister};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryOpcodeClass {
    Instruction,
    Prefix(Prefix),
    SecondaryEscape,
    Undefined,
}

/// Classify one byte using the Intel 80386 Appendix A one-byte opcode map.
pub const fn classify_primary_opcode(opcode: u8) -> PrimaryOpcodeClass {
    match opcode {
        0x0f => PrimaryOpcodeClass::SecondaryEscape,
        0x26 => PrimaryOpcodeClass::Prefix(Prefix::Segment(SegmentRegister::Es)),
        0x2e => PrimaryOpcodeClass::Prefix(Prefix::Segment(SegmentRegister::Cs)),
        0x36 => PrimaryOpcodeClass::Prefix(Prefix::Segment(SegmentRegister::Ss)),
        0x3e => PrimaryOpcodeClass::Prefix(Prefix::Segment(SegmentRegister::Ds)),
        0x64 => PrimaryOpcodeClass::Prefix(Prefix::Segment(SegmentRegister::Fs)),
        0x65 => PrimaryOpcodeClass::Prefix(Prefix::Segment(SegmentRegister::Gs)),
        0x66 => PrimaryOpcodeClass::Prefix(Prefix::OperandSizeOverride),
        0x67 => PrimaryOpcodeClass::Prefix(Prefix::AddressSizeOverride),
        0xf0 => PrimaryOpcodeClass::Prefix(Prefix::Lock),
        0xf2 => PrimaryOpcodeClass::Prefix(Prefix::RepeatNotEqual),
        0xf3 => PrimaryOpcodeClass::Prefix(Prefix::Repeat),
        0x82 | 0xd6 | 0xf1 => PrimaryOpcodeClass::Undefined,
        _ => PrimaryOpcodeClass::Instruction,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecondaryOpcodeClass {
    Instruction,
    Undefined,
}

/// Classify the second byte after `0F` using the 80386 two-byte opcode map.
pub const fn classify_secondary_opcode(opcode: u8) -> SecondaryOpcodeClass {
    match opcode {
        0x00 | 0x01 | 0x02 | 0x03 | 0x06 => SecondaryOpcodeClass::Instruction,
        0x20 | 0x21 | 0x22 | 0x23 | 0x24 | 0x26 => SecondaryOpcodeClass::Instruction,
        0x80..=0x9f => SecondaryOpcodeClass::Instruction,
        0xa0 | 0xa1 | 0xa3 | 0xa4 | 0xa5 | 0xa8 | 0xa9 | 0xab | 0xac | 0xad | 0xaf => {
            SecondaryOpcodeClass::Instruction
        }
        0xb2 | 0xb3 | 0xb4 | 0xb5 | 0xb6 | 0xb7 | 0xba | 0xbb | 0xbc | 0xbd | 0xbe | 0xbf => {
            SecondaryOpcodeClass::Instruction
        }
        _ => SecondaryOpcodeClass::Undefined,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModRmSelectorClass {
    Valid,
    Reserved,
    NotOpcodeExtension,
}

/// Classify primary-opcode ModR/M selector fields that extend an opcode.
pub const fn classify_primary_modrm_selector(opcode: u8, selector: u8) -> ModRmSelectorClass {
    if selector > 7 {
        return ModRmSelectorClass::Reserved;
    }
    match opcode {
        0x80 | 0x81 | 0x83 => ModRmSelectorClass::Valid,
        0x8f | 0xc6 | 0xc7 => {
            if selector == 0 {
                ModRmSelectorClass::Valid
            } else {
                ModRmSelectorClass::Reserved
            }
        }
        0xc0 | 0xc1 | 0xd0 | 0xd1 | 0xd2 | 0xd3 => {
            if selector == 6 {
                ModRmSelectorClass::Reserved
            } else {
                ModRmSelectorClass::Valid
            }
        }
        0xf6 | 0xf7 => {
            if selector == 1 {
                ModRmSelectorClass::Reserved
            } else {
                ModRmSelectorClass::Valid
            }
        }
        0xfe => {
            if selector <= 1 {
                ModRmSelectorClass::Valid
            } else {
                ModRmSelectorClass::Reserved
            }
        }
        0xff => {
            if selector <= 6 {
                ModRmSelectorClass::Valid
            } else {
                ModRmSelectorClass::Reserved
            }
        }
        _ => ModRmSelectorClass::NotOpcodeExtension,
    }
}

/// Classify two-byte-opcode ModR/M selector fields that extend an opcode.
pub const fn classify_secondary_modrm_selector(opcode: u8, selector: u8) -> ModRmSelectorClass {
    if selector > 7 {
        return ModRmSelectorClass::Reserved;
    }
    match opcode {
        0x00 => {
            if selector <= 5 {
                ModRmSelectorClass::Valid
            } else {
                ModRmSelectorClass::Reserved
            }
        }
        0x01 => {
            if matches!(selector, 0 | 1 | 2 | 3 | 4 | 6) {
                ModRmSelectorClass::Valid
            } else {
                ModRmSelectorClass::Reserved
            }
        }
        0xba => {
            if selector >= 4 {
                ModRmSelectorClass::Valid
            } else {
                ModRmSelectorClass::Reserved
            }
        }
        _ => ModRmSelectorClass::NotOpcodeExtension,
    }
}
