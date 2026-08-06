//! PCEAS-style HuC6280 instruction formatting.

use core::fmt::Write;

use crate::{AddressingMode, Instruction, Operand};

/// Format one placed instruction as `MNEMONIC OPERANDS`.
pub fn format_instruction(instruction: Instruction, pc: u16) -> String {
    let mnemonic = instruction.mnemonic().to_string();
    let operand = format_operand(instruction, pc);
    if operand.is_empty() {
        mnemonic
    } else {
        format!("{mnemonic} {operand}")
    }
}

/// Format instruction bytes as an uppercase, space-separated column.
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
    let next = instruction.fallthrough_address(pc);
    match (instruction.addressing_mode(), instruction.operand()) {
        (AddressingMode::Implied, Operand::None) => String::new(),
        (AddressingMode::Accumulator, Operand::None) => "A".to_owned(),
        (AddressingMode::Immediate, Operand::Byte(value)) => format!("#${value:02X}"),
        (AddressingMode::ZeroPage, Operand::Byte(address)) => format!("${address:02X}"),
        (AddressingMode::ZeroPageX, Operand::Byte(address)) => format!("${address:02X},X"),
        (AddressingMode::ZeroPageY, Operand::Byte(address)) => format!("${address:02X},Y"),
        (AddressingMode::Absolute, Operand::Word(address)) => format!("${address:04X}"),
        (AddressingMode::AbsoluteX, Operand::Word(address)) => format!("${address:04X},X"),
        (AddressingMode::AbsoluteY, Operand::Word(address)) => format!("${address:04X},Y"),
        (AddressingMode::AbsoluteIndirect, Operand::Word(address)) => {
            format!("(${address:04X})")
        }
        (AddressingMode::ZeroPageIndexedIndirectX, Operand::Byte(address)) => {
            format!("(${address:02X},X)")
        }
        (AddressingMode::ZeroPageIndirectIndexedY, Operand::Byte(address)) => {
            format!("(${address:02X}),Y")
        }
        (AddressingMode::ZeroPageIndirect, Operand::Byte(address)) => {
            format!("(${address:02X})")
        }
        (AddressingMode::AbsoluteIndexedIndirectX, Operand::Word(address)) => {
            format!("(${address:04X},X)")
        }
        (AddressingMode::Relative, Operand::Relative(displacement)) => {
            format!("${:04X}", relative_target(next, displacement))
        }
        (
            AddressingMode::ZeroPageRelative,
            Operand::ZeroPageRelative {
                address,
                displacement,
            },
        ) => format!(
            "${address:02X},${:04X}",
            relative_target(next, displacement)
        ),
        (
            AddressingMode::BlockTransfer,
            Operand::BlockTransfer {
                source,
                destination,
                length,
            },
        ) => format!("${source:04X},${destination:04X},${length:04X}"),
        (AddressingMode::ImmediateZeroPage, Operand::ImmediateZeroPage { immediate, address }) => {
            format!("#${immediate:02X},${address:02X}")
        }
        (AddressingMode::ImmediateZeroPageX, Operand::ImmediateZeroPage { immediate, address }) => {
            format!("#${immediate:02X},${address:02X},X")
        }
        (AddressingMode::ImmediateAbsolute, Operand::ImmediateAbsolute { immediate, address }) => {
            format!("#${immediate:02X},${address:04X}")
        }
        (AddressingMode::ImmediateAbsoluteX, Operand::ImmediateAbsolute { immediate, address }) => {
            format!("#${immediate:02X},${address:04X},X")
        }
        _ => unreachable!("Instruction construction validates operand shape"),
    }
}

fn relative_target(next: u16, displacement: i8) -> u16 {
    next.wrapping_add(displacement as i16 as u16)
}
