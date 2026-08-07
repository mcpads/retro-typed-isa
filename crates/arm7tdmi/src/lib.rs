//! Static ARM7TDMI (ARMv4T) instruction model.
//!
//! The profile follows the ARM and Thumb instruction summaries in ARM DDI
//! 0210C, revision r4p1. ARM state and Thumb state are separate protocol
//! profiles because instruction width and decoding state are externally known.
//! Reserved and architecturally unpredictable encodings fail closed.

mod assembler;
mod codec;
mod instruction;

pub use assembler::{
    ArmAssembler, ArmAssemblerError, ArmProgram, ThumbAssembler, ThumbAssemblerError, ThumbProgram,
};
pub use codec::{
    Arm7TdmiArm, Arm7TdmiThumb, DecodeError, EncodeError, decode_arm_bytes, decode_thumb_bytes,
    encode_arm_bytes, encode_thumb_bytes,
};
pub use instruction::*;
