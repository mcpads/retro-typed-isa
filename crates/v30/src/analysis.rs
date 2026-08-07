use crate::{
    CallTarget, Condition, DecodeError, Instruction, InstructionAddress, JmpTarget, LoopCondition,
    decode_bytes, encode_bytes,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// The statically determined control-flow effect of a native V30 instruction.
pub enum ControlFlow {
    /// The fall through form.
    FallThrough {
        /// The next operand.
        next: InstructionAddress,
    },
    /// The branch form.
    Branch {
        /// The resolved target address.
        target: InstructionAddress,
        /// The fallthrough operand.
        fallthrough: InstructionAddress,
    },
    /// The jump form.
    Jump {
        /// The resolved target address.
        target: Option<InstructionAddress>,
    },
    /// The call form.
    Call {
        /// The resolved target address.
        target: Option<InstructionAddress>,
        /// The fallthrough operand.
        fallthrough: InstructionAddress,
    },
    /// The return form.
    Return,
    /// The interrupt form.
    Interrupt {
        /// The vector operand.
        vector: u8,
    },
    /// The conditional interrupt form.
    ConditionalInterrupt {
        /// The vector operand.
        vector: u8,
        /// The fallthrough operand.
        fallthrough: InstructionAddress,
    },
    /// The halt form.
    Halt,
    /// The external mode transfer form.
    ExternalModeTransfer {
        /// The vector operand.
        vector: u8,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A verified instruction, its segment-relative offset, and control-flow effect.
pub struct PlacedInstruction {
    /// The segmented instruction address.
    pub address: InstructionAddress,
    /// The byte offset or encoded displacement.
    pub offset: usize,
    /// The encoded instruction length in bytes.
    pub byte_len: usize,
    /// The typed instruction.
    pub instruction: Instruction,
    /// The static control-flow effect.
    pub control_flow: ControlFlow,
}

/// Strictly decodes and verifies a complete placed program.
pub fn verify_placed_program(
    bytes: &[u8],
    origin: InstructionAddress,
) -> Result<Vec<PlacedInstruction>, ProgramVerificationError> {
    ensure_segment_capacity(origin, bytes.len())?;
    let mut instructions = Vec::new();
    let mut offset = 0usize;
    while offset < bytes.len() {
        let address = InstructionAddress {
            seg: origin.seg,
            off: origin.off + offset as u16,
        };
        let decoded = decode_bytes(&bytes[offset..])
            .map_err(|source| ProgramVerificationError::Decode { address, source })?;
        let instruction_bytes = &bytes[offset..offset + decoded.byte_len];
        let reencoded = encode_bytes(&decoded.instruction)
            .map_err(|source| ProgramVerificationError::Encode { address, source })?;
        if reencoded != instruction_bytes {
            return Err(ProgramVerificationError::NonCanonicalEncoding { address });
        }
        let next = advance(address, decoded.byte_len);
        let control_flow = control_flow(&decoded.instruction, next);
        instructions.push(PlacedInstruction {
            address,
            offset,
            byte_len: decoded.byte_len,
            instruction: decoded.instruction,
            control_flow,
        });
        offset += decoded.byte_len;
    }
    Ok(instructions)
}

/// Computes static control flow using the supplied next-instruction address.
pub fn control_flow(instruction: &Instruction, next: InstructionAddress) -> ControlFlow {
    match instruction {
        Instruction::Jcc { target, .. } => ControlFlow::Branch {
            target: relative_target(next, i32::from(*target)),
            fallthrough: next,
        },
        Instruction::Loop { target, .. } => ControlFlow::Branch {
            target: relative_target(next, i32::from(*target)),
            fallthrough: next,
        },
        Instruction::Jmp { target } => ControlFlow::Jump {
            target: jump_target(target, next),
        },
        Instruction::Call { target } => ControlFlow::Call {
            target: call_target(target, next),
            fallthrough: next,
        },
        Instruction::Ret { .. } | Instruction::Retf { .. } | Instruction::Iret => {
            ControlFlow::Return
        }
        Instruction::Int { vector } => ControlFlow::Interrupt { vector: *vector },
        Instruction::Into => ControlFlow::ConditionalInterrupt {
            vector: 4,
            fallthrough: next,
        },
        Instruction::Hlt => ControlFlow::Halt,
        Instruction::Brkem { vector } => ControlFlow::ExternalModeTransfer { vector: *vector },
        Instruction::Rep(instruction)
        | Instruction::Repne(instruction)
        | Instruction::Repc(instruction)
        | Instruction::Repnc(instruction)
        | Instruction::BusLock(instruction)
        | Instruction::SegmentOverride { instruction, .. } => control_flow(instruction, next),
        _ => ControlFlow::FallThrough { next },
    }
}

fn jump_target(target: &JmpTarget, next: InstructionAddress) -> Option<InstructionAddress> {
    match target {
        JmpTarget::Rel8(displacement) => Some(relative_target(next, i32::from(*displacement))),
        JmpTarget::Rel16(displacement) => Some(relative_target(next, i32::from(*displacement))),
        JmpTarget::Far { seg, off } => Some(InstructionAddress {
            seg: *seg,
            off: *off,
        }),
        JmpTarget::IndirectNear(_) | JmpTarget::IndirectFar(_) => None,
    }
}

fn call_target(target: &CallTarget, next: InstructionAddress) -> Option<InstructionAddress> {
    match target {
        CallTarget::Rel16(displacement) => Some(relative_target(next, i32::from(*displacement))),
        CallTarget::Far { seg, off } => Some(InstructionAddress {
            seg: *seg,
            off: *off,
        }),
        CallTarget::IndirectNear(_) | CallTarget::IndirectFar(_) => None,
    }
}

fn relative_target(next: InstructionAddress, displacement: i32) -> InstructionAddress {
    InstructionAddress {
        seg: next.seg,
        off: (i32::from(next.off) + displacement) as u16,
    }
}

fn advance(address: InstructionAddress, len: usize) -> InstructionAddress {
    InstructionAddress {
        seg: address.seg,
        off: address.off + len as u16,
    }
}

pub(crate) fn ensure_segment_capacity(
    origin: InstructionAddress,
    byte_len: usize,
) -> Result<(), ProgramVerificationError> {
    if usize::from(origin.off) + byte_len > 0x1_0000 {
        return Err(ProgramVerificationError::CodeSegmentBoundary { origin, byte_len });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
/// Failure returned while decoding and verifying a placed native V30 byte stream.
pub enum ProgramVerificationError {
    #[error("program at {origin:?} with {byte_len} bytes crosses the 64 KiB code segment")]
    /// The code segment boundary form.
    CodeSegmentBoundary {
        /// The first instruction address.
        origin: InstructionAddress,
        /// The encoded instruction length in bytes.
        byte_len: usize,
    },
    #[error("decode failed at {address:?}: {source}")]
    /// The decode form.
    Decode {
        /// The segmented instruction address.
        address: InstructionAddress,
        /// The underlying decode or encode failure.
        source: DecodeError,
    },
    #[error("encode failed at {address:?}: {source}")]
    /// The encode form.
    Encode {
        /// The segmented instruction address.
        address: InstructionAddress,
        /// The underlying decode or encode failure.
        source: crate::EncodeError,
    },
    #[error("instruction at {address:?} is not in canonical generated-code form")]
    /// The non canonical encoding form.
    NonCanonicalEncoding {
        /// The affected instruction address.
        address: InstructionAddress,
    },
}

#[allow(dead_code)]
fn _keep_public_condition_names(_: Condition, _: LoopCondition) {}
