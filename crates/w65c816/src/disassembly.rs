//! Placement-aware disassembly for caller-declared code ranges.
//!
//! This module does not discover code in arbitrary bytes. The caller supplies
//! an exact code range, its CPU origin, and the entry CPU mode. Within that
//! boundary every byte must decode as an instruction; no data fallback exists.

use core::fmt;

use crate::assembler::{CpuMode, transition_mode};
use crate::codec::{DecodeError, DecodedInstruction, Operand, decode_bytes};
use crate::isa::{AddressingMode, Mnemonic};

/// Static control-transfer classification for one placed instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlFlow {
    /// Execution continues at the next sequential address.
    FallThrough {
        /// Address immediately after the instruction.
        next: u32,
    },
    /// A conditional or unconditional relative branch.
    Branch {
        /// Direct branch target in the current program bank.
        target: u32,
        /// Address reached when the condition is false, or `None` for BRA/BRL.
        fallthrough: Option<u32>,
    },
    /// A subroutine call, possibly through an indirect pointer.
    Call {
        /// Direct static target, or `None` for an indirect call.
        target: Option<u32>,
        /// Address immediately after the call instruction.
        return_address: u32,
    },
    /// An unconditional jump, possibly through an indirect pointer.
    Jump {
        /// Direct static target, or `None` for an indirect jump.
        target: Option<u32>,
    },
    /// Control returns through a stack-supplied address.
    Return,
    /// Control enters a BRK or COP vector.
    Interrupt,
    /// Execution waits for an interrupt and then resumes sequentially.
    Wait {
        /// Sequential address at which execution resumes.
        resume_address: u32,
    },
    /// The processor clock stops until reset.
    Stop,
}

/// One instruction decoded at a caller-declared 24-bit code location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedInstruction {
    /// Byte offset relative to the declared range.
    pub offset: usize,
    /// 24-bit CPU address of the opcode byte.
    pub address: u32,
    /// Exact source bytes consumed for the instruction.
    pub raw: Vec<u8>,
    /// CPU mode used to determine immediate operand widths.
    pub mode_before: CpuMode,
    /// Statically known mode after the instruction, or `None` when unknowable.
    pub mode_after: Option<CpuMode>,
    /// Typed decode result.
    pub decoded: DecodedInstruction,
    /// Placement-aware static control-flow classification.
    pub control_flow: ControlFlow,
}

/// Complete linear decode of one caller-declared, single-bank code range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disassembly {
    /// 24-bit address assigned to the first source byte.
    pub origin: u32,
    /// Number of source bytes consumed by the declared range.
    pub byte_len: usize,
    /// CPU mode supplied for the entry instruction.
    pub initial_mode: CpuMode,
    /// Mode after the final instruction, or `None` if the final transition is unknown.
    pub final_mode: Option<CpuMode>,
    /// Sequential placed instructions covering every source byte.
    pub instructions: Vec<PlacedInstruction>,
}

/// A fail-closed reason that a declared range could not be decoded linearly.
#[derive(Debug)]
pub enum DisassemblyError {
    /// The origin exceeds the W65C816's 24-bit address space.
    OriginOutOfRange(u32),
    /// The declared range crosses a 64 KiB program-bank boundary.
    CrossesBankBoundary {
        /// Address assigned to the first source byte.
        origin: u32,
        /// Declared byte length that does not fit in the bank.
        len: usize,
    },
    /// A preceding instruction made M/X width state statically unknowable.
    CpuModeUnknown {
        /// Offset of the first instruction requiring a new mode assumption.
        offset: usize,
    },
    /// An instruction failed to decode under the current width state.
    Decode {
        /// Offset at which decoding failed.
        offset: usize,
        /// Underlying byte-decoder failure.
        source: DecodeError,
    },
}

impl fmt::Display for DisassemblyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OriginOutOfRange(origin) => {
                write!(f, "disassembly origin exceeds 24-bit space: 0x{origin:X}")
            }
            Self::CrossesBankBoundary { origin, len } => write!(
                f,
                "disassembly at 0x{origin:06X} with length 0x{len:X} crosses a program-bank boundary"
            ),
            Self::CpuModeUnknown { offset } => write!(
                f,
                "CPU mode is unknown at code offset 0x{offset:X}; split the declared range and supply a new entry mode"
            ),
            Self::Decode { offset, source } => {
                write!(f, "decode failed at code offset 0x{offset:X}: {source}")
            }
        }
    }
}

impl std::error::Error for DisassemblyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Decode { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Decode every byte in a caller-declared, single-bank code range.
///
/// REP and SEP update subsequent immediate widths. XCE and native-mode
/// PLP/RTI make the following mode unknown, so a non-empty suffix is rejected.
/// Calls are treated as returning to the next instruction with the same mode;
/// validating a callee's status-register contract remains an interprocedural
/// responsibility of the caller.
pub fn disassemble_linear(
    bytes: &[u8],
    origin: u32,
    initial_mode: CpuMode,
) -> Result<Disassembly, DisassemblyError> {
    ensure_range(origin, bytes.len())?;

    let mut instructions = Vec::new();
    let mut offset = 0;
    let mut mode = Some(initial_mode);
    while offset < bytes.len() {
        let mode_before = mode.ok_or(DisassemblyError::CpuModeUnknown { offset })?;
        let decoded = decode_bytes(&bytes[offset..], mode_before.widths())
            .map_err(|source| DisassemblyError::Decode { offset, source })?;
        let end = offset + decoded.len;
        let address = origin + offset as u32;
        let next = origin + end as u32;
        let mode_after = transition_mode(mode_before, &decoded.instruction);
        let control_flow = classify_control_flow(address, next, &decoded);
        instructions.push(PlacedInstruction {
            offset,
            address,
            raw: bytes[offset..end].to_vec(),
            mode_before,
            mode_after,
            decoded,
            control_flow,
        });
        offset = end;
        mode = mode_after;
    }

    Ok(Disassembly {
        origin,
        byte_len: bytes.len(),
        initial_mode,
        final_mode: mode,
        instructions,
    })
}

fn ensure_range(origin: u32, len: usize) -> Result<(), DisassemblyError> {
    if origin > 0xFF_FFFF {
        return Err(DisassemblyError::OriginOutOfRange(origin));
    }
    let bank_offset = (origin & 0xFFFF) as usize;
    if bank_offset
        .checked_add(len)
        .is_none_or(|end| end > 0x1_0000)
    {
        return Err(DisassemblyError::CrossesBankBoundary { origin, len });
    }
    Ok(())
}

fn classify_control_flow(address: u32, next: u32, decoded: &DecodedInstruction) -> ControlFlow {
    use Mnemonic::*;

    let mnemonic = decoded.instruction.mnemonic;
    if is_conditional_branch(mnemonic) {
        return ControlFlow::Branch {
            target: relative_target(address, next, &decoded.instruction.operand)
                .expect("branch opcode has a relative operand"),
            fallthrough: Some(next),
        };
    }
    match mnemonic {
        Bra | Brl => ControlFlow::Branch {
            target: relative_target(address, next, &decoded.instruction.operand)
                .expect("branch opcode has a relative operand"),
            fallthrough: None,
        },
        Jsr | Jsl => ControlFlow::Call {
            target: direct_target(address, decoded),
            return_address: next,
        },
        Jmp | Jml => ControlFlow::Jump {
            target: direct_target(address, decoded),
        },
        Rti | Rtl | Rts => ControlFlow::Return,
        Brk | Cop => ControlFlow::Interrupt,
        Wai => ControlFlow::Wait {
            resume_address: next,
        },
        Stp => ControlFlow::Stop,
        _ => ControlFlow::FallThrough { next },
    }
}

fn is_conditional_branch(mnemonic: Mnemonic) -> bool {
    matches!(
        mnemonic,
        Mnemonic::Bcc
            | Mnemonic::Bcs
            | Mnemonic::Beq
            | Mnemonic::Bmi
            | Mnemonic::Bne
            | Mnemonic::Bpl
            | Mnemonic::Bvc
            | Mnemonic::Bvs
    )
}

fn relative_target(address: u32, next: u32, operand: &Operand) -> Option<u32> {
    let displacement = match operand {
        Operand::Relative8(value) => i32::from(*value),
        Operand::Relative16(value) => i32::from(*value),
        _ => return None,
    };
    let bank = address & 0xFF_0000;
    let target = (next as i32 + displacement) & 0xFFFF;
    Some(bank | target as u32)
}

fn direct_target(address: u32, decoded: &DecodedInstruction) -> Option<u32> {
    match (decoded.instruction.mode, &decoded.instruction.operand) {
        (AddressingMode::Absolute, Operand::Word(target)) => {
            Some((address & 0xFF_0000) | u32::from(*target))
        }
        (AddressingMode::AbsoluteLong, Operand::Long(target)) => Some(*target),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{WidthState, encode_bytes};

    #[test]
    fn decodes_placed_m16_code_and_resolves_branch_targets() {
        let bytes = [
            0xA9, 0x01, 0x00, // LDA #$0001
            0x85, 0x02, // STA $02
            0x80, 0x02, // BRA $80:A4CA
            0x64, 0x02, // alternate-entry STZ $02
            0xA9, 0x00, 0x23, // LDA #$2300
        ];
        let output = disassemble_linear(&bytes, 0x80_A4C1, CpuMode::NATIVE_M16_X16).unwrap();

        assert_eq!(output.instructions.len(), 5);
        assert_eq!(output.instructions[0].address, 0x80_A4C1);
        assert_eq!(output.instructions[0].raw, [0xA9, 0x01, 0x00]);
        assert_eq!(
            output.instructions[2].control_flow,
            ControlFlow::Branch {
                target: 0x80_A4CA,
                fallthrough: None,
            }
        );
        assert_eq!(output.instructions[3].address, 0x80_A4C8);
        for instruction in &output.instructions {
            assert_eq!(
                encode_bytes(
                    &instruction.decoded.instruction,
                    instruction.mode_before.widths()
                )
                .unwrap(),
                instruction.raw
            );
        }
    }

    #[test]
    fn tracks_rep_and_sep_across_dynamic_immediates() {
        let bytes = [
            0xC2, 0x30, // REP #$30
            0xA9, 0x34, 0x12, // LDA #$1234
            0xA2, 0x78, 0x56, // LDX #$5678
            0xE2, 0x20, // SEP #$20
            0xA9, 0x7F, // LDA #$7F
        ];
        let output = disassemble_linear(&bytes, 0x81_8000, CpuMode::NATIVE_M8_X8).unwrap();

        assert_eq!(output.instructions.len(), 5);
        assert_eq!(
            output.instructions[1].mode_before.widths(),
            WidthState::M16_X16
        );
        assert_eq!(
            output.instructions[3].mode_after.unwrap().widths(),
            WidthState::M8_X16
        );
        assert_eq!(output.instructions[4].raw, [0xA9, 0x7F]);
    }

    #[test]
    fn refuses_to_guess_a_mode_after_ambiguous_status_transitions() {
        let xce_then_code = [0xFB, 0xA9, 0x00];
        assert!(matches!(
            disassemble_linear(&xce_then_code, 0x80_8000, CpuMode::NATIVE_M16_X16),
            Err(DisassemblyError::CpuModeUnknown { offset: 1 })
        ));

        let native_plp_then_code = [0x28, 0xA9, 0x00];
        assert!(matches!(
            disassemble_linear(&native_plp_then_code, 0x80_8000, CpuMode::NATIVE_M16_X16),
            Err(DisassemblyError::CpuModeUnknown { offset: 1 })
        ));

        let terminal_xce = disassemble_linear(&[0xFB], 0x80_8000, CpuMode::Emulation).unwrap();
        assert_eq!(terminal_xce.final_mode, None);
    }

    #[test]
    fn classifies_direct_calls_indirect_jumps_and_returns() {
        let calls = disassemble_linear(
            &[0x22, 0xCD, 0x92, 0x80, 0x6B],
            0x80_A4F3,
            CpuMode::NATIVE_M16_X16,
        )
        .unwrap();
        assert_eq!(
            calls.instructions[0].control_flow,
            ControlFlow::Call {
                target: Some(0x80_92CD),
                return_address: 0x80_A4F7,
            }
        );
        assert_eq!(calls.instructions[1].control_flow, ControlFlow::Return);

        let indirect =
            disassemble_linear(&[0x7C, 0x00, 0x90], 0x82_8000, CpuMode::Emulation).unwrap();
        assert_eq!(
            indirect.instructions[0].control_flow,
            ControlFlow::Jump { target: None }
        );
    }

    #[test]
    fn rejects_ranges_that_cross_a_program_bank() {
        assert!(matches!(
            disassemble_linear(&[0xEA, 0xEA], 0x80_FFFF, CpuMode::Emulation),
            Err(DisassemblyError::CrossesBankBoundary { .. })
        ));
    }
}
