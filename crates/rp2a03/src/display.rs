//! Conventional 6502-style instruction formatting.

use core::fmt::Write;

use crate::{AddressingMode, Instruction, Operand};

/// Format one placed instruction as `MNEMONIC OPERAND`.
pub fn format_instruction(instruction: Instruction, pc: u16) -> String {
    let mnemonic = instruction.mnemonic().to_string();
    let operand = format_operand(instruction, pc);
    if operand.is_empty() {
        mnemonic
    } else {
        format!("{mnemonic} {operand}")
    }
}

/// Format bytes as an uppercase, space-separated column.
pub fn format_bytes(bytes: &[u8], slot_bytes: usize) -> String {
    let mut output = String::new();
    for (index, byte) in bytes.iter().enumerate() {
        if index != 0 {
            output.push(' ');
        }
        write!(output, "{byte:02X}").expect("writing to String cannot fail");
    }
    let width = slot_bytes.saturating_mul(3).saturating_sub(1);
    if output.len() < width {
        output.extend(core::iter::repeat_n(' ', width - output.len()));
    }
    output
}

fn format_operand(instruction: Instruction, pc: u16) -> String {
    match (instruction.addressing_mode(), instruction.operand()) {
        (AddressingMode::Implied, Operand::None) => String::new(),
        (AddressingMode::Accumulator, Operand::None) => "A".to_owned(),
        (AddressingMode::Immediate, Operand::Byte(value)) => format!("#${value:02X}"),
        (AddressingMode::ZeroPage, Operand::Byte(address)) => format!("${address:02X}"),
        (AddressingMode::ZeroPageX, Operand::Byte(address)) => format!("${address:02X},X"),
        (AddressingMode::ZeroPageY, Operand::Byte(address)) => format!("${address:02X},Y"),
        (AddressingMode::Relative, Operand::Relative(displacement)) => {
            let next = instruction.fallthrough_address(pc);
            format!("${:04X}", next.wrapping_add(displacement as i16 as u16))
        }
        (AddressingMode::Absolute, Operand::Word(address)) => format!("${address:04X}"),
        (AddressingMode::AbsoluteX, Operand::Word(address)) => format!("${address:04X},X"),
        (AddressingMode::AbsoluteY, Operand::Word(address)) => format!("${address:04X},Y"),
        (AddressingMode::AbsoluteIndirect, Operand::Word(address)) => format!("(${address:04X})"),
        (AddressingMode::ZeroPageIndexedIndirectX, Operand::Byte(address)) => {
            format!("(${address:02X},X)")
        }
        (AddressingMode::ZeroPageIndirectIndexedY, Operand::Byte(address)) => {
            format!("(${address:02X}),Y")
        }
        _ => unreachable!("Instruction construction validates operand shape"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode_bytes;

    #[test]
    fn formats_indexed_and_relative_operands() {
        assert_eq!(
            format_instruction(decode_bytes(&[0xBD, 0x34, 0x12]).unwrap(), 0),
            "LDA $1234,X"
        );
        assert_eq!(
            format_instruction(decode_bytes(&[0xD0, 0xFC]).unwrap(), 0x8000),
            "BNE $7FFE"
        );
    }
}
