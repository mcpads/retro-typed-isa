use thiserror::Error;

use crate::codec::{DecodeError, decode_program};
use crate::instruction::{Instruction, InstructionForm, Register};

/// An SH-2 instruction cannot be represented by the declared 16-bit profile.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EncodeError {
    /// A memory displacement does not fit the selected four-bit field.
    #[error("SH-2 {form:?} displacement {displacement} exceeds its four-bit field")]
    FourBitDisplacementOutOfRange {
        /// Instruction form whose displacement was rejected.
        form: InstructionForm,
        /// Rejected displacement value.
        displacement: u8,
    },
    /// A branch displacement does not fit its signed 12-bit field.
    #[error("SH-2 branch displacement {displacement} exceeds its signed 12-bit field")]
    TwelveBitDisplacementOutOfRange {
        /// Rejected displacement in instruction words.
        displacement: i16,
    },
}

fn reg(reg: Register) -> u16 {
    u16::from(reg.index())
}

fn disp12(disp: i16) -> Result<u16, EncodeError> {
    if (-2048..=2047).contains(&disp) {
        Ok((disp as u16) & 0x0FFF)
    } else {
        Err(EncodeError::TwelveBitDisplacementOutOfRange { displacement: disp })
    }
}

fn disp4(form: InstructionForm, displacement: u8) -> Result<u16, EncodeError> {
    if displacement <= 0x0f {
        Ok(u16::from(displacement))
    } else {
        Err(EncodeError::FourBitDisplacementOutOfRange { form, displacement })
    }
}

/// Encode one instruction from the complete SH-2 profile represented by
/// [`Instruction`]. Reserved/undecodable opcodes are rejected.
pub fn encode_word(instruction: &Instruction) -> Result<u16, EncodeError> {
    use Instruction::*;

    let opcode = match instruction {
        MovImm { imm, rn } => 0xE000 | reg(*rn) << 8 | u16::from(*imm as u8),
        MovWPcRel { disp, rn } => 0x9000 | reg(*rn) << 8 | u16::from(*disp),
        MovLPcRel { disp, rn } => 0xD000 | reg(*rn) << 8 | u16::from(*disp),
        MovReg { rm, rn } => 0x6003 | reg(*rn) << 8 | reg(*rm) << 4,
        MovBStore { rm, rn } => 0x2000 | reg(*rn) << 8 | reg(*rm) << 4,
        MovWStore { rm, rn } => 0x2001 | reg(*rn) << 8 | reg(*rm) << 4,
        MovLStore { rm, rn } => 0x2002 | reg(*rn) << 8 | reg(*rm) << 4,
        MovBLoad { rm, rn } => 0x6000 | reg(*rn) << 8 | reg(*rm) << 4,
        MovWLoad { rm, rn } => 0x6001 | reg(*rn) << 8 | reg(*rm) << 4,
        MovLLoad { rm, rn } => 0x6002 | reg(*rn) << 8 | reg(*rm) << 4,
        MovBStorePreDec { rm, rn } => 0x2004 | reg(*rn) << 8 | reg(*rm) << 4,
        MovWStorePreDec { rm, rn } => 0x2005 | reg(*rn) << 8 | reg(*rm) << 4,
        MovLStorePreDec { rm, rn } => 0x2006 | reg(*rn) << 8 | reg(*rm) << 4,
        MovBLoadPostInc { rm, rn } => 0x6004 | reg(*rn) << 8 | reg(*rm) << 4,
        MovWLoadPostInc { rm, rn } => 0x6005 | reg(*rn) << 8 | reg(*rm) << 4,
        MovLLoadPostInc { rm, rn } => 0x6006 | reg(*rn) << 8 | reg(*rm) << 4,
        MovBR0LoadIndexed { rm, rn } => 0x000C | reg(*rn) << 8 | reg(*rm) << 4,
        MovWR0LoadIndexed { rm, rn } => 0x000D | reg(*rn) << 8 | reg(*rm) << 4,
        MovLR0LoadIndexed { rm, rn } => 0x000E | reg(*rn) << 8 | reg(*rm) << 4,
        MovBR0StoreIndexed { rm, rn } => 0x0004 | reg(*rn) << 8 | reg(*rm) << 4,
        MovWR0StoreIndexed { rm, rn } => 0x0005 | reg(*rn) << 8 | reg(*rm) << 4,
        MovLR0StoreIndexed { rm, rn } => 0x0006 | reg(*rn) << 8 | reg(*rm) << 4,
        MovBDispStore { disp, rn } => {
            0x8000 | reg(*rn) << 4 | disp4(InstructionForm::MovBDispStore, *disp)?
        }
        MovWDispStore { disp, rn } => {
            0x8100 | reg(*rn) << 4 | disp4(InstructionForm::MovWDispStore, *disp)?
        }
        MovLDispStore { disp, rm, rn } => {
            0x1000 | reg(*rn) << 8 | reg(*rm) << 4 | disp4(InstructionForm::MovLDispStore, *disp)?
        }
        MovBDispLoad { disp, rm } => {
            0x8400 | reg(*rm) << 4 | disp4(InstructionForm::MovBDispLoad, *disp)?
        }
        MovWDispLoad { disp, rm } => {
            0x8500 | reg(*rm) << 4 | disp4(InstructionForm::MovWDispLoad, *disp)?
        }
        MovLDispLoad { disp, rm, rn } => {
            0x5000 | reg(*rn) << 8 | reg(*rm) << 4 | disp4(InstructionForm::MovLDispLoad, *disp)?
        }
        MovBGbrStore { disp } => 0xC000 | u16::from(*disp),
        MovBGbrLoad { disp } => 0xC400 | u16::from(*disp),
        MovWGbrStore { disp } => 0xC100 | u16::from(*disp),
        MovWGbrLoad { disp } => 0xC500 | u16::from(*disp),
        MovLGbrStore { disp } => 0xC200 | u16::from(*disp),
        MovLGbrLoad { disp } => 0xC600 | u16::from(*disp),
        Mova { disp } => 0xC700 | u16::from(*disp),
        MovT { rn } => 0x0029 | reg(*rn) << 8,

        Add { rm, rn } => 0x300C | reg(*rn) << 8 | reg(*rm) << 4,
        AddImm { imm, rn } => 0x7000 | reg(*rn) << 8 | u16::from(*imm as u8),
        Addc { rm, rn } => 0x300E | reg(*rn) << 8 | reg(*rm) << 4,
        Addv { rm, rn } => 0x300F | reg(*rn) << 8 | reg(*rm) << 4,
        Sub { rm, rn } => 0x3008 | reg(*rn) << 8 | reg(*rm) << 4,
        Subc { rm, rn } => 0x300A | reg(*rn) << 8 | reg(*rm) << 4,
        Subv { rm, rn } => 0x300B | reg(*rn) << 8 | reg(*rm) << 4,
        Neg { rm, rn } => 0x600B | reg(*rn) << 8 | reg(*rm) << 4,
        Negc { rm, rn } => 0x600A | reg(*rn) << 8 | reg(*rm) << 4,
        Dt { rn } => 0x4010 | reg(*rn) << 8,
        MulL { rm, rn } => 0x0007 | reg(*rn) << 8 | reg(*rm) << 4,
        MulS { rm, rn } => 0x200F | reg(*rn) << 8 | reg(*rm) << 4,
        MulU { rm, rn } => 0x200E | reg(*rn) << 8 | reg(*rm) << 4,
        DmulS { rm, rn } => 0x300D | reg(*rn) << 8 | reg(*rm) << 4,
        DmulU { rm, rn } => 0x3005 | reg(*rn) << 8 | reg(*rm) << 4,
        Div0S { rm, rn } => 0x2007 | reg(*rn) << 8 | reg(*rm) << 4,
        Div0U => 0x0019,
        Div1 { rm, rn } => 0x3004 | reg(*rn) << 8 | reg(*rm) << 4,
        MacL { rm, rn } => 0x000F | reg(*rn) << 8 | reg(*rm) << 4,
        MacW { rm, rn } => 0x400F | reg(*rn) << 8 | reg(*rm) << 4,

        And { rm, rn } => 0x2009 | reg(*rn) << 8 | reg(*rm) << 4,
        AndImm { imm } => 0xC900 | u16::from(*imm),
        AndB { imm } => 0xCD00 | u16::from(*imm),
        Or { rm, rn } => 0x200B | reg(*rn) << 8 | reg(*rm) << 4,
        OrImm { imm } => 0xCB00 | u16::from(*imm),
        OrB { imm } => 0xCF00 | u16::from(*imm),
        Xor { rm, rn } => 0x200A | reg(*rn) << 8 | reg(*rm) << 4,
        XorImm { imm } => 0xCA00 | u16::from(*imm),
        XorB { imm } => 0xCE00 | u16::from(*imm),
        Not { rm, rn } => 0x6007 | reg(*rn) << 8 | reg(*rm) << 4,
        Tst { rm, rn } => 0x2008 | reg(*rn) << 8 | reg(*rm) << 4,
        TstImm { imm } => 0xC800 | u16::from(*imm),
        TstB { imm } => 0xCC00 | u16::from(*imm),

        Shll { rn } => 0x4000 | reg(*rn) << 8,
        Shlr { rn } => 0x4001 | reg(*rn) << 8,
        Shal { rn } => 0x4020 | reg(*rn) << 8,
        Shar { rn } => 0x4021 | reg(*rn) << 8,
        Shll2 { rn } => 0x4008 | reg(*rn) << 8,
        Shlr2 { rn } => 0x4009 | reg(*rn) << 8,
        Shll8 { rn } => 0x4018 | reg(*rn) << 8,
        Shlr8 { rn } => 0x4019 | reg(*rn) << 8,
        Shll16 { rn } => 0x4028 | reg(*rn) << 8,
        Shlr16 { rn } => 0x4029 | reg(*rn) << 8,
        Rotl { rn } => 0x4004 | reg(*rn) << 8,
        Rotr { rn } => 0x4005 | reg(*rn) << 8,
        Rotcl { rn } => 0x4024 | reg(*rn) << 8,
        Rotcr { rn } => 0x4025 | reg(*rn) << 8,

        ExtsB { rm, rn } => 0x600E | reg(*rn) << 8 | reg(*rm) << 4,
        ExtsW { rm, rn } => 0x600F | reg(*rn) << 8 | reg(*rm) << 4,
        ExtuB { rm, rn } => 0x600C | reg(*rn) << 8 | reg(*rm) << 4,
        ExtuW { rm, rn } => 0x600D | reg(*rn) << 8 | reg(*rm) << 4,
        SwapB { rm, rn } => 0x6008 | reg(*rn) << 8 | reg(*rm) << 4,
        SwapW { rm, rn } => 0x6009 | reg(*rn) << 8 | reg(*rm) << 4,
        Xtrct { rm, rn } => 0x200D | reg(*rn) << 8 | reg(*rm) << 4,

        CmpEq { rm, rn } => 0x3000 | reg(*rn) << 8 | reg(*rm) << 4,
        CmpEqImm { imm } => 0x8800 | u16::from(*imm as u8),
        CmpGe { rm, rn } => 0x3003 | reg(*rn) << 8 | reg(*rm) << 4,
        CmpGt { rm, rn } => 0x3007 | reg(*rn) << 8 | reg(*rm) << 4,
        CmpHi { rm, rn } => 0x3006 | reg(*rn) << 8 | reg(*rm) << 4,
        CmpHs { rm, rn } => 0x3002 | reg(*rn) << 8 | reg(*rm) << 4,
        CmpPl { rn } => 0x4015 | reg(*rn) << 8,
        CmpPz { rn } => 0x4011 | reg(*rn) << 8,
        CmpStr { rm, rn } => 0x200C | reg(*rn) << 8 | reg(*rm) << 4,

        Bt { disp } => 0x8900 | u16::from(*disp as u8),
        Bf { disp } => 0x8B00 | u16::from(*disp as u8),
        Bts { disp } => 0x8D00 | u16::from(*disp as u8),
        Bfs { disp } => 0x8F00 | u16::from(*disp as u8),
        Bra { disp } => 0xA000 | disp12(*disp)?,
        Bsr { disp } => 0xB000 | disp12(*disp)?,
        Jmp { rm } => 0x402B | reg(*rm) << 8,
        Jsr { rm } => 0x400B | reg(*rm) << 8,
        Rts => 0x000B,
        Rte => 0x002B,

        LdcGbr { rm } => 0x401E | reg(*rm) << 8,
        LdcVbr { rm } => 0x402E | reg(*rm) << 8,
        LdcSr { rm } => 0x400E | reg(*rm) << 8,
        LdcLGbr { rm } => 0x4017 | reg(*rm) << 8,
        LdcLVbr { rm } => 0x4027 | reg(*rm) << 8,
        LdcLSr { rm } => 0x4007 | reg(*rm) << 8,
        StcGbr { rn } => 0x0012 | reg(*rn) << 8,
        StcVbr { rn } => 0x0022 | reg(*rn) << 8,
        StcSr { rn } => 0x0002 | reg(*rn) << 8,
        StcLGbr { rn } => 0x4013 | reg(*rn) << 8,
        StcLVbr { rn } => 0x4023 | reg(*rn) << 8,
        StcLSr { rn } => 0x4003 | reg(*rn) << 8,
        LdsMach { rm } => 0x400A | reg(*rm) << 8,
        LdsMacl { rm } => 0x401A | reg(*rm) << 8,
        LdsPr { rm } => 0x402A | reg(*rm) << 8,
        LdsLMach { rm } => 0x4006 | reg(*rm) << 8,
        LdsLMacl { rm } => 0x4016 | reg(*rm) << 8,
        LdsLPr { rm } => 0x4026 | reg(*rm) << 8,
        StsMach { rn } => 0x000A | reg(*rn) << 8,
        StsMacl { rn } => 0x001A | reg(*rn) << 8,
        StsPr { rn } => 0x002A | reg(*rn) << 8,
        StsLMach { rn } => 0x4002 | reg(*rn) << 8,
        StsLMacl { rn } => 0x4012 | reg(*rn) << 8,
        StsLPr { rn } => 0x4022 | reg(*rn) << 8,

        Nop => 0x0009,
        Sleep => 0x001B,
        SetT => 0x0018,
        ClrT => 0x0008,
        ClrMac => 0x0028,
        Trapa { imm } => 0xC300 | u16::from(*imm),
        Bsrf { rm } => 0x0003 | reg(*rm) << 8,
        Braf { rm } => 0x0023 | reg(*rm) << 8,
        Tas { rn } => 0x401B | reg(*rn) << 8,
    };

    Ok(opcode)
}

/// Encode one instruction in SH-2 big-endian byte order.
pub fn encode_bytes(instruction: &Instruction) -> Result<[u8; 2], EncodeError> {
    Ok(encode_word(instruction)?.to_be_bytes())
}

/// Failure of placed-program decode and exact byte round-trip verification.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum VerifyError {
    /// Decoding the placed bytes failed.
    #[error(transparent)]
    Decode(#[from] DecodeError),
    /// Re-encoding a decoded instruction failed.
    #[error(transparent)]
    Encode(#[from] EncodeError),
    /// Re-encoding produced different bytes.
    #[error(
        "SH-2 byte round-trip mismatch at instruction {index}: expected {expected:02X?}, got {actual:02X?}"
    )]
    ByteMismatch {
        /// Zero-based instruction index.
        index: usize,
        /// Original big-endian instruction bytes.
        expected: [u8; 2],
        /// Bytes produced by semantic re-encoding.
        actual: [u8; 2],
    },
}

/// Decode and re-encode an entire placed SH-2 code byte sequence.
pub fn verify_placed_program(bytes: &[u8], origin: u32) -> Result<Vec<Instruction>, VerifyError> {
    let instructions = decode_program(bytes, origin)?;
    for (index, (chunk, instruction)) in bytes.chunks_exact(2).zip(instructions.iter()).enumerate()
    {
        let expected: [u8; 2] = chunk.try_into().expect("two-byte instruction");
        let actual = encode_bytes(instruction)?;
        if expected != actual {
            return Err(VerifyError::ByteMismatch {
                index,
                expected,
                actual,
            });
        }
    }
    Ok(instructions)
}
