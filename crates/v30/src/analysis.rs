use crate::{
    CallTarget, Condition, DecodeError, Instruction, InstructionAddress, JmpTarget, LoopCondition,
    decode_bytes, encode_bytes,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlFlow {
    FallThrough {
        next: InstructionAddress,
    },
    Branch {
        target: InstructionAddress,
        fallthrough: InstructionAddress,
    },
    Jump {
        target: Option<InstructionAddress>,
    },
    Call {
        target: Option<InstructionAddress>,
        fallthrough: InstructionAddress,
    },
    Return,
    Interrupt {
        vector: u8,
    },
    ConditionalInterrupt {
        vector: u8,
        fallthrough: InstructionAddress,
    },
    Halt,
    ExternalModeTransfer {
        vector: u8,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedInstruction {
    pub address: InstructionAddress,
    pub offset: usize,
    pub byte_len: usize,
    pub instruction: Instruction,
    pub control_flow: ControlFlow,
}

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
pub enum ProgramVerificationError {
    #[error("program at {origin:?} with {byte_len} bytes crosses the 64 KiB code segment")]
    CodeSegmentBoundary {
        origin: InstructionAddress,
        byte_len: usize,
    },
    #[error("decode failed at {address:?}: {source}")]
    Decode {
        address: InstructionAddress,
        source: DecodeError,
    },
    #[error("encode failed at {address:?}: {source}")]
    Encode {
        address: InstructionAddress,
        source: crate::EncodeError,
    },
    #[error("instruction at {address:?} is not in canonical generated-code form")]
    NonCanonicalEncoding { address: InstructionAddress },
}

#[allow(dead_code)]
fn _keep_public_condition_names(_: Condition, _: LoopCondition) {}
