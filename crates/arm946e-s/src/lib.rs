//! Static ARM946E-S (ARMv5TE) instruction model.
//!
//! The profile follows ARM DDI 0201D for the processor scope and the ARMv5TE
//! instruction tables in ARM DDI 0100I. It extends the workspace-owned ARMv4T
//! profile with the version 5 and enhanced-DSP instruction forms; it does not
//! widen the profile to ARMv5TEJ or ARMv6.

mod assembler;
mod codec;
mod instruction;

pub use assembler::{
    ArmAssembler, ArmAssemblerError, ArmProgram, ThumbAssembler, ThumbAssemblerError, ThumbProgram,
};
pub use codec::{
    Arm946EsArm, Arm946EsThumb, DecodeError, EncodeError, decode_arm_bytes, decode_thumb_bytes,
    encode_arm_bytes, encode_thumb_bytes,
};
pub use instruction::*;
