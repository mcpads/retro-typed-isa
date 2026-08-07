use thiserror::Error;

use crate::instruction::{Instruction, Register};

/// Classification of every possible 16-bit word in the SH-2 encoding space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodedWord {
    /// A word assigned to a declared SH-2 instruction.
    Instruction(Instruction),
    /// A reserved or out-of-profile word.
    Unknown {
        /// Unassigned instruction word.
        word: u16,
    },
}

/// Failure to decode one word or a placed instruction stream.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DecodeError {
    /// Fewer than two bytes were available.
    #[error("SH-2 instruction requires two bytes, got {len}")]
    TruncatedInput {
        /// Supplied byte length.
        len: usize,
    },
    /// The word is reserved or outside the declared SH-2 profile.
    #[error("reserved SH-2 instruction word 0x{word:04X}")]
    UnknownWord {
        /// Rejected instruction word.
        word: u16,
    },
    /// A placed instruction address is not two-byte aligned.
    #[error("SH-2 instruction address 0x{address:08X} is not two-byte aligned")]
    UnalignedInstructionAddress {
        /// Rejected instruction address.
        address: u32,
    },
    /// A placed instruction address overflows 32 bits.
    #[error("SH-2 address overflow from 0x{origin:08X} at instruction {index}")]
    AddressOverflow {
        /// Stream origin.
        origin: u32,
        /// Zero-based instruction index.
        index: usize,
    },
}

/// Sign-extend an 8-bit value to i8.
fn sign_ext8(val: u8) -> i8 {
    val as i8
}

/// Sign-extend a 12-bit value to i16.
fn sign_ext12(val: u16) -> i16 {
    if val & 0x800 != 0 {
        (val | 0xF000) as i16
    } else {
        val as i16
    }
}

fn known(instruction: Instruction) -> DecodedWord {
    DecodedWord::Instruction(instruction)
}

/// Helper to construct an Unknown instruction.
fn unknown(opcode: u16) -> DecodedWord {
    DecodedWord::Unknown { word: opcode }
}

/// Classify one natural SH-2 instruction word, including reserved words.
pub fn classify_word(opcode: u16) -> DecodedWord {
    let n = ((opcode >> 8) & 0x0F) as u8; // Rn field (bits 8-11)
    let m = ((opcode >> 4) & 0x0F) as u8; // Rm field (bits 4-7)
    let d4 = (opcode & 0x0F) as u8; // low 4 bits
    let d8 = (opcode & 0xFF) as u8; // low 8 bits
    let d12 = opcode & 0x0FFF; // low 12 bits

    match opcode >> 12 {
        0x0 => decode_0xxx(opcode, n, m, d4),
        0x1 => {
            // MOV.L Rm, @(disp,Rn) -- 1nmd
            known(Instruction::MovLDispStore {
                disp: d4,
                rm: Register::from_field(m),
                rn: Register::from_field(n),
            })
        }
        0x2 => decode_2xxx(opcode, n, m, d4),
        0x3 => decode_3xxx(opcode, n, m, d4),
        0x4 => decode_4xxx(opcode, n, m, d4),
        0x5 => {
            // MOV.L @(disp,Rm), Rn -- 5nmd
            known(Instruction::MovLDispLoad {
                disp: d4,
                rm: Register::from_field(m),
                rn: Register::from_field(n),
            })
        }
        0x6 => decode_6xxx(n, m, d4),
        0x7 => {
            // ADD #imm, Rn -- 7nii
            known(Instruction::AddImm {
                imm: sign_ext8(d8),
                rn: Register::from_field(n),
            })
        }
        0x8 => decode_8xxx(opcode, n, d8),
        0x9 => {
            // MOV.W @(disp,PC), Rn -- 9ndd
            known(Instruction::MovWPcRel {
                disp: d8,
                rn: Register::from_field(n),
            })
        }
        0xA => {
            // BRA disp -- Addd (12-bit signed)
            known(Instruction::Bra {
                disp: sign_ext12(d12),
            })
        }
        0xB => {
            // BSR disp -- Bddd (12-bit signed)
            known(Instruction::Bsr {
                disp: sign_ext12(d12),
            })
        }
        0xC => decode_cxxx(opcode, d8),
        0xD => {
            // MOV.L @(disp,PC), Rn -- Dndd
            known(Instruction::MovLPcRel {
                disp: d8,
                rn: Register::from_field(n),
            })
        }
        0xE => {
            // MOV #imm, Rn -- Enii (8-bit sign-extended)
            known(Instruction::MovImm {
                imm: sign_ext8(d8),
                rn: Register::from_field(n),
            })
        }
        0xF => {
            // Undefined on SH-2 (SH-4 FPU instructions)
            unknown(opcode)
        }
        _ => unreachable!(),
    }
}

/// Decode 0xxx instructions: system, STC, STS, MUL.L, MAC.L, MOV R0-indexed, etc.
fn decode_0xxx(opcode: u16, n: u8, m: u8, d4: u8) -> DecodedWord {
    match d4 {
        0x2 => {
            // 0n02: STC SR, Rn
            // 0n12: STC GBR, Rn
            // 0n22: STC VBR, Rn
            match m {
                0 => known(Instruction::StcSr {
                    rn: Register::from_field(n),
                }),
                1 => known(Instruction::StcGbr {
                    rn: Register::from_field(n),
                }),
                2 => known(Instruction::StcVbr {
                    rn: Register::from_field(n),
                }),
                _ => unknown(opcode),
            }
        }
        0x3 => {
            // 0nm3: BSRF/BRAF -- standard SH-2 instructions (register-relative branch)
            // Target = PC + 4 + Rm, resolved at runtime (not statically determinable)
            match m {
                0 => known(Instruction::Bsrf {
                    rm: Register::from_field(n),
                }),
                2 => known(Instruction::Braf {
                    rm: Register::from_field(n),
                }),
                _ => unknown(opcode),
            }
        }
        0x4 => {
            // 0nm4: MOV.B Rm, @(R0,Rn)
            known(Instruction::MovBR0StoreIndexed {
                rm: Register::from_field(m),
                rn: Register::from_field(n),
            })
        }
        0x5 => {
            // 0nm5: MOV.W Rm, @(R0,Rn)
            known(Instruction::MovWR0StoreIndexed {
                rm: Register::from_field(m),
                rn: Register::from_field(n),
            })
        }
        0x6 => {
            // 0nm6: MOV.L Rm, @(R0,Rn)
            known(Instruction::MovLR0StoreIndexed {
                rm: Register::from_field(m),
                rn: Register::from_field(n),
            })
        }
        0x7 => {
            // 0nm7: MUL.L Rm, Rn
            known(Instruction::MulL {
                rm: Register::from_field(m),
                rn: Register::from_field(n),
            })
        }
        0x8 => {
            // 0x08: CLRT
            // 0x18: SETT
            // 0x28: CLRMAC
            // 0x38+: unknown
            match opcode {
                0x0008 => known(Instruction::ClrT),
                0x0018 => known(Instruction::SetT),
                0x0028 => known(Instruction::ClrMac),
                _ => unknown(opcode),
            }
        }
        0x9 => {
            // 0x09: NOP
            // 0x19: DIV0U
            // 0x29: MOVT Rn -- but actually MOVT is 0n29 format
            match opcode {
                0x0009 => known(Instruction::Nop),
                0x0019 => known(Instruction::Div0U),
                _ => {
                    // 0n29: MOVT Rn (where n != 0 potentially, but encoding is 0n29)
                    if m == 2 {
                        known(Instruction::MovT {
                            rn: Register::from_field(n),
                        })
                    } else {
                        unknown(opcode)
                    }
                }
            }
        }
        0xA => {
            // 0n0a: STS MACH, Rn
            // 0n1a: STS MACL, Rn
            // 0n2a: STS PR, Rn
            match m {
                0 => known(Instruction::StsMach {
                    rn: Register::from_field(n),
                }),
                1 => known(Instruction::StsMacl {
                    rn: Register::from_field(n),
                }),
                2 => known(Instruction::StsPr {
                    rn: Register::from_field(n),
                }),
                _ => unknown(opcode),
            }
        }
        0xB => {
            // 0x0B: RTS
            // 0x1B: SLEEP
            // 0x2B: RTE
            match opcode {
                0x000B => known(Instruction::Rts),
                0x001B => known(Instruction::Sleep),
                0x002B => known(Instruction::Rte),
                _ => unknown(opcode),
            }
        }
        0xC => {
            // 0nmC: MOV.B @(R0,Rm), Rn
            known(Instruction::MovBR0LoadIndexed {
                rm: Register::from_field(m),
                rn: Register::from_field(n),
            })
        }
        0xD => {
            // 0nmD: MOV.W @(R0,Rm), Rn
            known(Instruction::MovWR0LoadIndexed {
                rm: Register::from_field(m),
                rn: Register::from_field(n),
            })
        }
        0xE => {
            // 0nmE: MOV.L @(R0,Rm), Rn
            known(Instruction::MovLR0LoadIndexed {
                rm: Register::from_field(m),
                rn: Register::from_field(n),
            })
        }
        0xF => {
            // 0nmF: MAC.L @Rm+, @Rn+
            known(Instruction::MacL {
                rm: Register::from_field(m),
                rn: Register::from_field(n),
            })
        }
        _ => {
            // 0x0, 0x1: unknown encodings in this group
            unknown(opcode)
        }
    }
}

/// Decode 2xxx instructions: MOV stores, pre-decrement, logical, etc.
fn decode_2xxx(opcode: u16, n: u8, m: u8, d4: u8) -> DecodedWord {
    match d4 {
        0x0 => known(Instruction::MovBStore {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0x1 => known(Instruction::MovWStore {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0x2 => known(Instruction::MovLStore {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0x4 => known(Instruction::MovBStorePreDec {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0x5 => known(Instruction::MovWStorePreDec {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0x6 => known(Instruction::MovLStorePreDec {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0x7 => known(Instruction::Div0S {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0x8 => known(Instruction::Tst {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0x9 => known(Instruction::And {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0xA => known(Instruction::Xor {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0xB => known(Instruction::Or {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0xC => known(Instruction::CmpStr {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0xD => known(Instruction::Xtrct {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0xE => known(Instruction::MulU {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0xF => known(Instruction::MulS {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        _ => {
            // 0x3: undefined in 2xxx group
            unknown(opcode)
        }
    }
}

/// Decode 3xxx instructions: arithmetic, compare, div, dmul.
fn decode_3xxx(opcode: u16, n: u8, m: u8, d4: u8) -> DecodedWord {
    match d4 {
        0x0 => known(Instruction::CmpEq {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0x2 => known(Instruction::CmpHs {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0x3 => known(Instruction::CmpGe {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0x4 => known(Instruction::Div1 {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0x5 => known(Instruction::DmulU {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0x6 => known(Instruction::CmpHi {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0x7 => known(Instruction::CmpGt {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0x8 => known(Instruction::Sub {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0xA => known(Instruction::Subc {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0xB => known(Instruction::Subv {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0xC => known(Instruction::Add {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0xD => known(Instruction::DmulS {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0xE => known(Instruction::Addc {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0xF => known(Instruction::Addv {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        _ => {
            // 0x1, 0x9: undefined in 3xxx group
            unknown(opcode)
        }
    }
}

/// Decode 4xxx instructions: shifts, rotates, JSR, JMP, DT, CMP/Px, system register ops.
fn decode_4xxx(opcode: u16, n: u8, m: u8, d4: u8) -> DecodedWord {
    // 4xxx is complex: the low byte (m:d4) determines the instruction.
    let low_byte = ((m as u16) << 4) | d4 as u16;

    match low_byte as u8 {
        0x00 => known(Instruction::Shll {
            rn: Register::from_field(n),
        }),
        0x01 => known(Instruction::Shlr {
            rn: Register::from_field(n),
        }),
        0x02 => known(Instruction::StsLMach {
            rn: Register::from_field(n),
        }),
        0x03 => known(Instruction::StcLSr {
            rn: Register::from_field(n),
        }),
        0x04 => known(Instruction::Rotl {
            rn: Register::from_field(n),
        }),
        0x05 => known(Instruction::Rotr {
            rn: Register::from_field(n),
        }),
        0x06 => known(Instruction::LdsLMach {
            rm: Register::from_field(n),
        }),
        0x07 => known(Instruction::LdcLSr {
            rm: Register::from_field(n),
        }),
        0x08 => known(Instruction::Shll2 {
            rn: Register::from_field(n),
        }),
        0x09 => known(Instruction::Shlr2 {
            rn: Register::from_field(n),
        }),
        0x0A => known(Instruction::LdsMach {
            rm: Register::from_field(n),
        }),
        0x0B => known(Instruction::Jsr {
            rm: Register::from_field(n),
        }),
        0x0E => known(Instruction::LdcSr {
            rm: Register::from_field(n),
        }),
        0x10 => known(Instruction::Dt {
            rn: Register::from_field(n),
        }),
        0x11 => known(Instruction::CmpPz {
            rn: Register::from_field(n),
        }),
        0x12 => known(Instruction::StsLMacl {
            rn: Register::from_field(n),
        }),
        0x13 => known(Instruction::StcLGbr {
            rn: Register::from_field(n),
        }),
        0x15 => known(Instruction::CmpPl {
            rn: Register::from_field(n),
        }),
        0x16 => known(Instruction::LdsLMacl {
            rm: Register::from_field(n),
        }),
        0x17 => known(Instruction::LdcLGbr {
            rm: Register::from_field(n),
        }),
        0x18 => known(Instruction::Shll8 {
            rn: Register::from_field(n),
        }),
        0x19 => known(Instruction::Shlr8 {
            rn: Register::from_field(n),
        }),
        0x1A => known(Instruction::LdsMacl {
            rm: Register::from_field(n),
        }),
        0x1B => known(Instruction::Tas {
            rn: Register::from_field(n),
        }),
        0x1E => known(Instruction::LdcGbr {
            rm: Register::from_field(n),
        }),
        0x20 => known(Instruction::Shal {
            rn: Register::from_field(n),
        }),
        0x21 => known(Instruction::Shar {
            rn: Register::from_field(n),
        }),
        0x22 => known(Instruction::StsLPr {
            rn: Register::from_field(n),
        }),
        0x23 => known(Instruction::StcLVbr {
            rn: Register::from_field(n),
        }),
        0x24 => known(Instruction::Rotcl {
            rn: Register::from_field(n),
        }),
        0x25 => known(Instruction::Rotcr {
            rn: Register::from_field(n),
        }),
        0x26 => known(Instruction::LdsLPr {
            rm: Register::from_field(n),
        }),
        0x27 => known(Instruction::LdcLVbr {
            rm: Register::from_field(n),
        }),
        0x28 => known(Instruction::Shll16 {
            rn: Register::from_field(n),
        }),
        0x29 => known(Instruction::Shlr16 {
            rn: Register::from_field(n),
        }),
        0x2A => known(Instruction::LdsPr {
            rm: Register::from_field(n),
        }),
        0x2B => known(Instruction::Jmp {
            rm: Register::from_field(n),
        }),
        0x2E => known(Instruction::LdcVbr {
            rm: Register::from_field(n),
        }),
        _ => {
            // Check for MAC.W @Rm+, @Rn+ (4nmF pattern)
            if d4 == 0xF {
                known(Instruction::MacW {
                    rm: Register::from_field(m),
                    rn: Register::from_field(n),
                })
            } else {
                unknown(opcode)
            }
        }
    }
}

/// Decode 6xxx instructions: MOV loads, post-increment, extensions, swaps, etc.
fn decode_6xxx(n: u8, m: u8, d4: u8) -> DecodedWord {
    match d4 {
        0x0 => known(Instruction::MovBLoad {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0x1 => known(Instruction::MovWLoad {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0x2 => known(Instruction::MovLLoad {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0x3 => known(Instruction::MovReg {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0x4 => known(Instruction::MovBLoadPostInc {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0x5 => known(Instruction::MovWLoadPostInc {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0x6 => known(Instruction::MovLLoadPostInc {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0x7 => known(Instruction::Not {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0x8 => known(Instruction::SwapB {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0x9 => known(Instruction::SwapW {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0xA => known(Instruction::Negc {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0xB => known(Instruction::Neg {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0xC => known(Instruction::ExtuB {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0xD => known(Instruction::ExtuW {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0xE => known(Instruction::ExtsB {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        0xF => known(Instruction::ExtsW {
            rm: Register::from_field(m),
            rn: Register::from_field(n),
        }),
        _ => unreachable!(),
    }
}

/// Decode 8xxx instructions: BT, BF, BT/S, BF/S, MOV.B disp, MOV.W disp, CMP/EQ #imm.
fn decode_8xxx(opcode: u16, n: u8, d8: u8) -> DecodedWord {
    // In 8xxx group, bits 8-11 determine the sub-instruction.
    match n {
        0x0 => {
            // 80md: MOV.B R0, @(disp,Rm) -- here "n" in 80nd means Rm, d8 has disp
            // Actually encoding is 80nd where n=Rn and d=disp (4-bit each from d8)
            let rn = (d8 >> 4) & 0x0F;
            let disp = d8 & 0x0F;
            known(Instruction::MovBDispStore {
                disp,
                rn: Register::from_field(rn),
            })
        }
        0x1 => {
            // 81nd: MOV.W R0, @(disp,Rn)
            let rn = (d8 >> 4) & 0x0F;
            let disp = d8 & 0x0F;
            known(Instruction::MovWDispStore {
                disp,
                rn: Register::from_field(rn),
            })
        }
        0x4 => {
            // 84md: MOV.B @(disp,Rm), R0
            let rm = (d8 >> 4) & 0x0F;
            let disp = d8 & 0x0F;
            known(Instruction::MovBDispLoad {
                disp,
                rm: Register::from_field(rm),
            })
        }
        0x5 => {
            // 85md: MOV.W @(disp,Rm), R0
            let rm = (d8 >> 4) & 0x0F;
            let disp = d8 & 0x0F;
            known(Instruction::MovWDispLoad {
                disp,
                rm: Register::from_field(rm),
            })
        }
        0x8 => {
            // 88ii: CMP/EQ #imm, R0
            known(Instruction::CmpEqImm { imm: sign_ext8(d8) })
        }
        0x9 => {
            // 89dd: BT disp (no delay slot)
            known(Instruction::Bt {
                disp: sign_ext8(d8),
            })
        }
        0xB => {
            // 8Bdd: BF disp (no delay slot)
            known(Instruction::Bf {
                disp: sign_ext8(d8),
            })
        }
        0xD => {
            // 8Ddd: BT/S disp (with delay slot)
            known(Instruction::Bts {
                disp: sign_ext8(d8),
            })
        }
        0xF => {
            // 8Fdd: BF/S disp (with delay slot)
            known(Instruction::Bfs {
                disp: sign_ext8(d8),
            })
        }
        _ => unknown(opcode),
    }
}

/// Decode Cxxx instructions: GBR-relative MOV, MOVA, TRAPA, AND/OR/XOR/TST #imm.
fn decode_cxxx(opcode: u16, d8: u8) -> DecodedWord {
    // Upper nibble of d8 (bits 8-11, which is actually the "n" field) determines sub-instruction
    let sub = (opcode >> 8) & 0x0F;
    match sub {
        0x0 => known(Instruction::MovBGbrStore { disp: d8 }),
        0x1 => known(Instruction::MovWGbrStore { disp: d8 }),
        0x2 => known(Instruction::MovLGbrStore { disp: d8 }),
        0x3 => known(Instruction::Trapa { imm: d8 }),
        0x4 => known(Instruction::MovBGbrLoad { disp: d8 }),
        0x5 => known(Instruction::MovWGbrLoad { disp: d8 }),
        0x6 => known(Instruction::MovLGbrLoad { disp: d8 }),
        0x7 => known(Instruction::Mova { disp: d8 }),
        0x8 => known(Instruction::TstImm { imm: d8 }),
        0x9 => known(Instruction::AndImm { imm: d8 }),
        0xA => known(Instruction::XorImm { imm: d8 }),
        0xB => known(Instruction::OrImm { imm: d8 }),
        0xC => known(Instruction::TstB { imm: d8 }),
        0xD => known(Instruction::AndB { imm: d8 }),
        0xE => known(Instruction::XorB { imm: d8 }),
        0xF => known(Instruction::OrB { imm: d8 }),
        _ => unreachable!(),
    }
}

/// Decode one natural SH-2 instruction word, failing closed for reserved words.
/// Decodes one assigned SH-2 instruction word.
pub fn decode_word(word: u16) -> Result<Instruction, DecodeError> {
    match classify_word(word) {
        DecodedWord::Instruction(instruction) => Ok(instruction),
        DecodedWord::Unknown { word } => Err(DecodeError::UnknownWord { word }),
    }
}

/// Decode one instruction from the first two bytes in SH-2 big-endian order.
/// Decodes the first two bytes as one big-endian SH-2 instruction.
pub fn decode_bytes(bytes: &[u8]) -> Result<Instruction, DecodeError> {
    let Some(bytes) = bytes.get(..2) else {
        return Err(DecodeError::TruncatedInput { len: bytes.len() });
    };
    decode_word(u16::from_be_bytes(
        bytes.try_into().expect("two-byte instruction"),
    ))
}

pub(crate) fn decode_program(bytes: &[u8], origin: u32) -> Result<Vec<Instruction>, DecodeError> {
    if origin & 1 != 0 {
        return Err(DecodeError::UnalignedInstructionAddress { address: origin });
    }
    if bytes.len() % 2 != 0 {
        return Err(DecodeError::TruncatedInput { len: bytes.len() });
    }
    let mut instructions = Vec::with_capacity(bytes.len() / 2);
    for (index, chunk) in bytes.chunks_exact(2).enumerate() {
        let offset = u32::try_from(index)
            .ok()
            .and_then(|value| value.checked_mul(2))
            .ok_or(DecodeError::AddressOverflow { origin, index })?;
        origin
            .checked_add(offset)
            .ok_or(DecodeError::AddressOverflow { origin, index })?;
        instructions.push(decode_bytes(chunk)?);
    }
    Ok(instructions)
}
