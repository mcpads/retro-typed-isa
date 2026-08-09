use std::fmt;

/// One of the sixteen SH-2 general-purpose registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Register(u8);

impl Register {
    /// General-purpose register R0.
    pub const R0: Self = Self(0);
    /// General-purpose register R1.
    pub const R1: Self = Self(1);
    /// General-purpose register R2.
    pub const R2: Self = Self(2);
    /// General-purpose register R3.
    pub const R3: Self = Self(3);
    /// General-purpose register R4.
    pub const R4: Self = Self(4);
    /// General-purpose register R5.
    pub const R5: Self = Self(5);
    /// General-purpose register R6.
    pub const R6: Self = Self(6);
    /// General-purpose register R7.
    pub const R7: Self = Self(7);
    /// General-purpose register R8.
    pub const R8: Self = Self(8);
    /// General-purpose register R9.
    pub const R9: Self = Self(9);
    /// General-purpose register R10.
    pub const R10: Self = Self(10);
    /// General-purpose register R11.
    pub const R11: Self = Self(11);
    /// General-purpose register R12.
    pub const R12: Self = Self(12);
    /// General-purpose register R13.
    pub const R13: Self = Self(13);
    /// General-purpose register R14.
    pub const R14: Self = Self(14);
    /// General-purpose register R15.
    pub const R15: Self = Self(15);
    /// Stack-pointer alias for R15.
    pub const SP: Self = Self::R15;

    /// Constructs a register from a four-bit index.
    pub const fn new(index: u8) -> Option<Self> {
        if index < 16 { Some(Self(index)) } else { None }
    }

    /// Returns the architectural register index.
    pub const fn index(self) -> u8 {
        self.0
    }

    pub(crate) const fn from_field(index: u8) -> Self {
        debug_assert!(index < 16);
        Self(index)
    }
}

impl TryFrom<u8> for Register {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(value)
    }
}

impl fmt::Display for Register {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "R{}", self.0)
    }
}

/// A complete typed instruction in the declared SH-2 profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    // === Data transfer ===
    /// MOV #imm, Rn (Enii -- 8-bit sign-extended immediate)
    MovImm {
        /// Immediate value.
        imm: i8,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MOV.W @(disp,PC), Rn (9ndd)
    MovWPcRel {
        /// Encoded displacement field.
        disp: u8,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MOV.L @(disp,PC), Rn (Dndd)
    MovLPcRel {
        /// Encoded displacement field.
        disp: u8,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MOV Rm, Rn (6nm3)
    MovReg {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MOV.B Rm, @Rn (2nm0)
    MovBStore {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MOV.W Rm, @Rn (2nm1)
    MovWStore {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MOV.L Rm, @Rn (2nm2)
    MovLStore {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MOV.B @Rm, Rn (6nm0)
    MovBLoad {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MOV.W @Rm, Rn (6nm1)
    MovWLoad {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MOV.L @Rm, Rn (6nm2)
    MovLLoad {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MOV.B Rm, @-Rn (2nm4)
    MovBStorePreDec {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MOV.W Rm, @-Rn (2nm5)
    MovWStorePreDec {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MOV.L Rm, @-Rn (2nm6)
    MovLStorePreDec {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MOV.B @Rm+, Rn (6nm4)
    MovBLoadPostInc {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MOV.W @Rm+, Rn (6nm5)
    MovWLoadPostInc {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MOV.L @Rm+, Rn (6nm6)
    MovLLoadPostInc {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MOV.B @(R0,Rm), Rn (0nmC)
    MovBR0LoadIndexed {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MOV.W @(R0,Rm), Rn (0nmD)
    MovWR0LoadIndexed {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MOV.L @(R0,Rm), Rn (0nmE)
    MovLR0LoadIndexed {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MOV.B Rm, @(R0,Rn) (0nm4)
    MovBR0StoreIndexed {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MOV.W Rm, @(R0,Rn) (0nm5)
    MovWR0StoreIndexed {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MOV.L Rm, @(R0,Rn) (0nm6)
    MovLR0StoreIndexed {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MOV.B R0, @(disp,Rn) (80nd)
    MovBDispStore {
        /// Encoded displacement field.
        disp: u8,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MOV.W R0, @(disp,Rn) (81nd)
    MovWDispStore {
        /// Encoded displacement field.
        disp: u8,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MOV.L Rm, @(disp,Rn) (1nmd)
    MovLDispStore {
        /// Encoded displacement field.
        disp: u8,
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MOV.B @(disp,Rm), R0 (84md)
    MovBDispLoad {
        /// Encoded displacement field.
        disp: u8,
        /// Architectural Rm register field.
        rm: Register,
    },
    /// MOV.W @(disp,Rm), R0 (85md)
    MovWDispLoad {
        /// Encoded displacement field.
        disp: u8,
        /// Architectural Rm register field.
        rm: Register,
    },
    /// MOV.L @(disp,Rm), Rn (5nmd)
    MovLDispLoad {
        /// Encoded displacement field.
        disp: u8,
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MOV.B R0, @(disp,GBR) (C0dd)
    MovBGbrStore {
        /// Encoded displacement field.
        disp: u8,
    },
    /// MOV.B @(disp,GBR), R0 (C4dd)
    MovBGbrLoad {
        /// Encoded displacement field.
        disp: u8,
    },
    /// MOV.W R0, @(disp,GBR) (C1dd)
    MovWGbrStore {
        /// Encoded displacement field.
        disp: u8,
    },
    /// MOV.W @(disp,GBR), R0 (C5dd)
    MovWGbrLoad {
        /// Encoded displacement field.
        disp: u8,
    },
    /// MOV.L R0, @(disp,GBR) (C2dd)
    MovLGbrStore {
        /// Encoded displacement field.
        disp: u8,
    },
    /// MOV.L @(disp,GBR), R0 (C6dd)
    MovLGbrLoad {
        /// Encoded displacement field.
        disp: u8,
    },
    /// MOVA @(disp,PC), R0 (C7dd)
    Mova {
        /// Encoded displacement field.
        disp: u8,
    },
    /// MOVT Rn (0n29)
    MovT {
        /// Architectural Rn register field.
        rn: Register,
    },

    // === Arithmetic ===
    /// ADD Rm, Rn (3nmc)
    Add {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// ADD #imm, Rn (7nii)
    AddImm {
        /// Immediate value.
        imm: i8,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// ADDC Rm, Rn (3nme)
    Addc {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// ADDV Rm, Rn (3nmf)
    Addv {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// SUB Rm, Rn (3nm8)
    Sub {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// SUBC Rm, Rn (3nma)
    Subc {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// SUBV Rm, Rn (3nmb)
    Subv {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// NEG Rm, Rn (6nmb)
    Neg {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// NEGC Rm, Rn (6nma)
    Negc {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// DT Rn (4n10)
    Dt {
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MUL.L Rm, Rn (0nm7)
    MulL {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MULS.W Rm, Rn (2nmf)
    MulS {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MULU.W Rm, Rn (2nme)
    MulU {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// DMULS.L Rm, Rn (3nmd)
    DmulS {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// DMULU.L Rm, Rn (3nm5)
    DmulU {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// DIV0S Rm, Rn (2nm7)
    Div0S {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// DIV0U (0019)
    Div0U,
    /// DIV1 Rm, Rn (3nm4)
    Div1 {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MAC.L @Rm+, @Rn+ (0nmf)
    MacL {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// MAC.W @Rm+, @Rn+ (4nmf)
    MacW {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },

    // === Logical ===
    /// AND Rm, Rn (2nm9)
    And {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// AND #imm, R0 (C9ii)
    AndImm {
        /// Immediate value.
        imm: u8,
    },
    /// AND.B #imm, @(R0,GBR) (CDii)
    AndB {
        /// Immediate value.
        imm: u8,
    },
    /// OR Rm, Rn (2nmb)
    Or {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// OR #imm, R0 (CBii)
    OrImm {
        /// Immediate value.
        imm: u8,
    },
    /// OR.B #imm, @(R0,GBR) (CFii)
    OrB {
        /// Immediate value.
        imm: u8,
    },
    /// XOR Rm, Rn (2nma)
    Xor {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// XOR #imm, R0 (CAii)
    XorImm {
        /// Immediate value.
        imm: u8,
    },
    /// XOR.B #imm, @(R0,GBR) (CEii)
    XorB {
        /// Immediate value.
        imm: u8,
    },
    /// NOT Rm, Rn (6nm7)
    Not {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// TST Rm, Rn (2nm8)
    Tst {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// TST #imm, R0 (C8ii)
    TstImm {
        /// Immediate value.
        imm: u8,
    },
    /// TST.B #imm, @(R0,GBR) (CCii)
    TstB {
        /// Immediate value.
        imm: u8,
    },

    // === Shifts ===
    /// SHLL Rn (4n00)
    Shll {
        /// Architectural Rn register field.
        rn: Register,
    },
    /// SHLR Rn (4n01)
    Shlr {
        /// Architectural Rn register field.
        rn: Register,
    },
    /// SHAL Rn (4n20)
    Shal {
        /// Architectural Rn register field.
        rn: Register,
    },
    /// SHAR Rn (4n21)
    Shar {
        /// Architectural Rn register field.
        rn: Register,
    },
    /// SHLL2 Rn (4n08)
    Shll2 {
        /// Architectural Rn register field.
        rn: Register,
    },
    /// SHLR2 Rn (4n09)
    Shlr2 {
        /// Architectural Rn register field.
        rn: Register,
    },
    /// SHLL8 Rn (4n18)
    Shll8 {
        /// Architectural Rn register field.
        rn: Register,
    },
    /// SHLR8 Rn (4n19)
    Shlr8 {
        /// Architectural Rn register field.
        rn: Register,
    },
    /// SHLL16 Rn (4n28)
    Shll16 {
        /// Architectural Rn register field.
        rn: Register,
    },
    /// SHLR16 Rn (4n29)
    Shlr16 {
        /// Architectural Rn register field.
        rn: Register,
    },
    /// ROTL Rn (4n04)
    Rotl {
        /// Architectural Rn register field.
        rn: Register,
    },
    /// ROTR Rn (4n05)
    Rotr {
        /// Architectural Rn register field.
        rn: Register,
    },
    /// ROTCL Rn (4n24)
    Rotcl {
        /// Architectural Rn register field.
        rn: Register,
    },
    /// ROTCR Rn (4n25)
    Rotcr {
        /// Architectural Rn register field.
        rn: Register,
    },

    // === Sign/zero extension ===
    /// EXTS.B Rm, Rn (6nme)
    ExtsB {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// EXTS.W Rm, Rn (6nmf)
    ExtsW {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// EXTU.B Rm, Rn (6nmc)
    ExtuB {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// EXTU.W Rm, Rn (6nmd)
    ExtuW {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// SWAP.B Rm, Rn (6nm8)
    SwapB {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// SWAP.W Rm, Rn (6nm9)
    SwapW {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// XTRCT Rm, Rn (2nmd)
    Xtrct {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },

    // === Comparisons (set T-bit) ===
    /// CMP/EQ Rm, Rn (3nm0)
    CmpEq {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// CMP/EQ #imm, R0 (88ii)
    CmpEqImm {
        /// Immediate value.
        imm: i8,
    },
    /// CMP/GE Rm, Rn (3nm3)
    CmpGe {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// CMP/GT Rm, Rn (3nm7)
    CmpGt {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// CMP/HI Rm, Rn (3nm6)
    CmpHi {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// CMP/HS Rm, Rn (3nm2)
    CmpHs {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },
    /// CMP/PL Rn (4n15)
    CmpPl {
        /// Architectural Rn register field.
        rn: Register,
    },
    /// CMP/PZ Rn (4n11)
    CmpPz {
        /// Architectural Rn register field.
        rn: Register,
    },
    /// CMP/STR Rm, Rn (2nmc)
    CmpStr {
        /// Architectural Rm register field.
        rm: Register,
        /// Architectural Rn register field.
        rn: Register,
    },

    // === Branches ===
    /// BT disp (89dd) -- no delay slot
    Bt {
        /// Encoded displacement field.
        disp: i8,
    },
    /// BF disp (8Bdd) -- no delay slot
    Bf {
        /// Encoded displacement field.
        disp: i8,
    },
    /// BT/S disp (8Ddd) -- delay slot
    Bts {
        /// Encoded displacement field.
        disp: i8,
    },
    /// BF/S disp (8Fdd) -- delay slot
    Bfs {
        /// Encoded displacement field.
        disp: i8,
    },
    /// BRA disp (Addd) -- 12-bit signed
    Bra {
        /// Encoded displacement field.
        disp: i16,
    },
    /// BSR disp (Bddd) -- 12-bit signed
    Bsr {
        /// Encoded displacement field.
        disp: i16,
    },
    /// JMP @Rm (4n2b)
    Jmp {
        /// Architectural Rm register field.
        rm: Register,
    },
    /// JSR @Rm (4n0b)
    Jsr {
        /// Architectural Rm register field.
        rm: Register,
    },
    /// RTS (000B)
    Rts,
    /// RTE (002B)
    Rte,

    // === System register transfer ===
    /// LDC Rm, GBR (4n1e)
    LdcGbr {
        /// Architectural Rm register field.
        rm: Register,
    },
    /// LDC Rm, VBR (4n2e)
    LdcVbr {
        /// Architectural Rm register field.
        rm: Register,
    },
    /// LDC Rm, SR (4n0e)
    LdcSr {
        /// Architectural Rm register field.
        rm: Register,
    },
    /// LDC.L @Rm+, GBR (4n17)
    LdcLGbr {
        /// Architectural Rm register field.
        rm: Register,
    },
    /// LDC.L @Rm+, VBR (4n27)
    LdcLVbr {
        /// Architectural Rm register field.
        rm: Register,
    },
    /// LDC.L @Rm+, SR (4n07)
    LdcLSr {
        /// Architectural Rm register field.
        rm: Register,
    },
    /// STC GBR, Rn (0n12)
    StcGbr {
        /// Architectural Rn register field.
        rn: Register,
    },
    /// STC VBR, Rn (0n22)
    StcVbr {
        /// Architectural Rn register field.
        rn: Register,
    },
    /// STC SR, Rn (0n02)
    StcSr {
        /// Architectural Rn register field.
        rn: Register,
    },
    /// STC.L GBR, @-Rn (4n13)
    StcLGbr {
        /// Architectural Rn register field.
        rn: Register,
    },
    /// STC.L VBR, @-Rn (4n23)
    StcLVbr {
        /// Architectural Rn register field.
        rn: Register,
    },
    /// STC.L SR, @-Rn (4n03)
    StcLSr {
        /// Architectural Rn register field.
        rn: Register,
    },
    /// LDS Rm, MACH (4n0a)
    LdsMach {
        /// Architectural Rm register field.
        rm: Register,
    },
    /// LDS Rm, MACL (4n1a)
    LdsMacl {
        /// Architectural Rm register field.
        rm: Register,
    },
    /// LDS Rm, PR (4n2a)
    LdsPr {
        /// Architectural Rm register field.
        rm: Register,
    },
    /// LDS.L @Rm+, MACH (4n06)
    LdsLMach {
        /// Architectural Rm register field.
        rm: Register,
    },
    /// LDS.L @Rm+, MACL (4n16)
    LdsLMacl {
        /// Architectural Rm register field.
        rm: Register,
    },
    /// LDS.L @Rm+, PR (4n26)
    LdsLPr {
        /// Architectural Rm register field.
        rm: Register,
    },
    /// STS MACH, Rn (0n0a)
    StsMach {
        /// Architectural Rn register field.
        rn: Register,
    },
    /// STS MACL, Rn (0n1a)
    StsMacl {
        /// Architectural Rn register field.
        rn: Register,
    },
    /// STS PR, Rn (0n2a)
    StsPr {
        /// Architectural Rn register field.
        rn: Register,
    },
    /// STS.L MACH, @-Rn (4n02)
    StsLMach {
        /// Architectural Rn register field.
        rn: Register,
    },
    /// STS.L MACL, @-Rn (4n12)
    StsLMacl {
        /// Architectural Rn register field.
        rn: Register,
    },
    /// STS.L PR, @-Rn (4n22)
    StsLPr {
        /// Architectural Rn register field.
        rn: Register,
    },

    // === Special ===
    /// NOP (0009)
    Nop,
    /// SLEEP (001B)
    Sleep,
    /// SETT (0018)
    SetT,
    /// CLRT (0008)
    ClrT,
    /// CLRMAC (0028)
    ClrMac,
    /// TRAPA #imm (C3ii)
    Trapa {
        /// Immediate value.
        imm: u8,
    },
    /// BSRF Rm (0n03) -- Branch to Subroutine Far: PC + 4 + Rm → PC, PC + 4 → PR
    Bsrf {
        /// Architectural Rm register field.
        rm: Register,
    },
    /// BRAF Rm (0n23) -- Branch Far: PC + 4 + Rm → PC
    Braf {
        /// Architectural Rm register field.
        rm: Register,
    },
    /// TAS.B @Rn (4n1B) -- Test And Set byte at @Rn
    Tas {
        /// Architectural Rn register field.
        rn: Register,
    },
}

/// Operand-free semantic forms in the complete SH-2 profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InstructionForm {
    /// The `MovImm` instruction form.
    MovImm,
    /// The `MovWPcRel` instruction form.
    MovWPcRel,
    /// The `MovLPcRel` instruction form.
    MovLPcRel,
    /// The `MovReg` instruction form.
    MovReg,
    /// The `MovBStore` instruction form.
    MovBStore,
    /// The `MovWStore` instruction form.
    MovWStore,
    /// The `MovLStore` instruction form.
    MovLStore,
    /// The `MovBLoad` instruction form.
    MovBLoad,
    /// The `MovWLoad` instruction form.
    MovWLoad,
    /// The `MovLLoad` instruction form.
    MovLLoad,
    /// The `MovBStorePreDec` instruction form.
    MovBStorePreDec,
    /// The `MovWStorePreDec` instruction form.
    MovWStorePreDec,
    /// The `MovLStorePreDec` instruction form.
    MovLStorePreDec,
    /// The `MovBLoadPostInc` instruction form.
    MovBLoadPostInc,
    /// The `MovWLoadPostInc` instruction form.
    MovWLoadPostInc,
    /// The `MovLLoadPostInc` instruction form.
    MovLLoadPostInc,
    /// The `MovBR0LoadIndexed` instruction form.
    MovBR0LoadIndexed,
    /// The `MovWR0LoadIndexed` instruction form.
    MovWR0LoadIndexed,
    /// The `MovLR0LoadIndexed` instruction form.
    MovLR0LoadIndexed,
    /// The `MovBR0StoreIndexed` instruction form.
    MovBR0StoreIndexed,
    /// The `MovWR0StoreIndexed` instruction form.
    MovWR0StoreIndexed,
    /// The `MovLR0StoreIndexed` instruction form.
    MovLR0StoreIndexed,
    /// The `MovBDispStore` instruction form.
    MovBDispStore,
    /// The `MovWDispStore` instruction form.
    MovWDispStore,
    /// The `MovLDispStore` instruction form.
    MovLDispStore,
    /// The `MovBDispLoad` instruction form.
    MovBDispLoad,
    /// The `MovWDispLoad` instruction form.
    MovWDispLoad,
    /// The `MovLDispLoad` instruction form.
    MovLDispLoad,
    /// The `MovBGbrStore` instruction form.
    MovBGbrStore,
    /// The `MovBGbrLoad` instruction form.
    MovBGbrLoad,
    /// The `MovWGbrStore` instruction form.
    MovWGbrStore,
    /// The `MovWGbrLoad` instruction form.
    MovWGbrLoad,
    /// The `MovLGbrStore` instruction form.
    MovLGbrStore,
    /// The `MovLGbrLoad` instruction form.
    MovLGbrLoad,
    /// The `Mova` instruction form.
    Mova,
    /// The `MovT` instruction form.
    MovT,
    /// The `Add` instruction form.
    Add,
    /// The `AddImm` instruction form.
    AddImm,
    /// The `Addc` instruction form.
    Addc,
    /// The `Addv` instruction form.
    Addv,
    /// The `Sub` instruction form.
    Sub,
    /// The `Subc` instruction form.
    Subc,
    /// The `Subv` instruction form.
    Subv,
    /// The `Neg` instruction form.
    Neg,
    /// The `Negc` instruction form.
    Negc,
    /// The `Dt` instruction form.
    Dt,
    /// The `MulL` instruction form.
    MulL,
    /// The `MulS` instruction form.
    MulS,
    /// The `MulU` instruction form.
    MulU,
    /// The `DmulS` instruction form.
    DmulS,
    /// The `DmulU` instruction form.
    DmulU,
    /// The `Div0S` instruction form.
    Div0S,
    /// The `Div0U` instruction form.
    Div0U,
    /// The `Div1` instruction form.
    Div1,
    /// The `MacL` instruction form.
    MacL,
    /// The `MacW` instruction form.
    MacW,
    /// The `And` instruction form.
    And,
    /// The `AndImm` instruction form.
    AndImm,
    /// The `AndB` instruction form.
    AndB,
    /// The `Or` instruction form.
    Or,
    /// The `OrImm` instruction form.
    OrImm,
    /// The `OrB` instruction form.
    OrB,
    /// The `Xor` instruction form.
    Xor,
    /// The `XorImm` instruction form.
    XorImm,
    /// The `XorB` instruction form.
    XorB,
    /// The `Not` instruction form.
    Not,
    /// The `Tst` instruction form.
    Tst,
    /// The `TstImm` instruction form.
    TstImm,
    /// The `TstB` instruction form.
    TstB,
    /// The `Shll` instruction form.
    Shll,
    /// The `Shlr` instruction form.
    Shlr,
    /// The `Shal` instruction form.
    Shal,
    /// The `Shar` instruction form.
    Shar,
    /// The `Shll2` instruction form.
    Shll2,
    /// The `Shlr2` instruction form.
    Shlr2,
    /// The `Shll8` instruction form.
    Shll8,
    /// The `Shlr8` instruction form.
    Shlr8,
    /// The `Shll16` instruction form.
    Shll16,
    /// The `Shlr16` instruction form.
    Shlr16,
    /// The `Rotl` instruction form.
    Rotl,
    /// The `Rotr` instruction form.
    Rotr,
    /// The `Rotcl` instruction form.
    Rotcl,
    /// The `Rotcr` instruction form.
    Rotcr,
    /// The `ExtsB` instruction form.
    ExtsB,
    /// The `ExtsW` instruction form.
    ExtsW,
    /// The `ExtuB` instruction form.
    ExtuB,
    /// The `ExtuW` instruction form.
    ExtuW,
    /// The `SwapB` instruction form.
    SwapB,
    /// The `SwapW` instruction form.
    SwapW,
    /// The `Xtrct` instruction form.
    Xtrct,
    /// The `CmpEq` instruction form.
    CmpEq,
    /// The `CmpEqImm` instruction form.
    CmpEqImm,
    /// The `CmpGe` instruction form.
    CmpGe,
    /// The `CmpGt` instruction form.
    CmpGt,
    /// The `CmpHi` instruction form.
    CmpHi,
    /// The `CmpHs` instruction form.
    CmpHs,
    /// The `CmpPl` instruction form.
    CmpPl,
    /// The `CmpPz` instruction form.
    CmpPz,
    /// The `CmpStr` instruction form.
    CmpStr,
    /// The `Bt` instruction form.
    Bt,
    /// The `Bf` instruction form.
    Bf,
    /// The `Bts` instruction form.
    Bts,
    /// The `Bfs` instruction form.
    Bfs,
    /// The `Bra` instruction form.
    Bra,
    /// The `Bsr` instruction form.
    Bsr,
    /// The `Jmp` instruction form.
    Jmp,
    /// The `Jsr` instruction form.
    Jsr,
    /// The `Rts` instruction form.
    Rts,
    /// The `Rte` instruction form.
    Rte,
    /// The `LdcGbr` instruction form.
    LdcGbr,
    /// The `LdcVbr` instruction form.
    LdcVbr,
    /// The `LdcSr` instruction form.
    LdcSr,
    /// The `LdcLGbr` instruction form.
    LdcLGbr,
    /// The `LdcLVbr` instruction form.
    LdcLVbr,
    /// The `LdcLSr` instruction form.
    LdcLSr,
    /// The `StcGbr` instruction form.
    StcGbr,
    /// The `StcVbr` instruction form.
    StcVbr,
    /// The `StcSr` instruction form.
    StcSr,
    /// The `StcLGbr` instruction form.
    StcLGbr,
    /// The `StcLVbr` instruction form.
    StcLVbr,
    /// The `StcLSr` instruction form.
    StcLSr,
    /// The `LdsMach` instruction form.
    LdsMach,
    /// The `LdsMacl` instruction form.
    LdsMacl,
    /// The `LdsPr` instruction form.
    LdsPr,
    /// The `LdsLMach` instruction form.
    LdsLMach,
    /// The `LdsLMacl` instruction form.
    LdsLMacl,
    /// The `LdsLPr` instruction form.
    LdsLPr,
    /// The `StsMach` instruction form.
    StsMach,
    /// The `StsMacl` instruction form.
    StsMacl,
    /// The `StsPr` instruction form.
    StsPr,
    /// The `StsLMach` instruction form.
    StsLMach,
    /// The `StsLMacl` instruction form.
    StsLMacl,
    /// The `StsLPr` instruction form.
    StsLPr,
    /// The `Nop` instruction form.
    Nop,
    /// The `Sleep` instruction form.
    Sleep,
    /// The `SetT` instruction form.
    SetT,
    /// The `ClrT` instruction form.
    ClrT,
    /// The `ClrMac` instruction form.
    ClrMac,
    /// The `Trapa` instruction form.
    Trapa,
    /// The `Bsrf` instruction form.
    Bsrf,
    /// The `Braf` instruction form.
    Braf,
    /// The `Tas` instruction form.
    Tas,
}

impl InstructionForm {
    /// Complete semantic-form denominator in deterministic catalog order.
    pub const ALL: &'static [Self] = &[
        Self::MovImm,
        Self::MovWPcRel,
        Self::MovLPcRel,
        Self::MovReg,
        Self::MovBStore,
        Self::MovWStore,
        Self::MovLStore,
        Self::MovBLoad,
        Self::MovWLoad,
        Self::MovLLoad,
        Self::MovBStorePreDec,
        Self::MovWStorePreDec,
        Self::MovLStorePreDec,
        Self::MovBLoadPostInc,
        Self::MovWLoadPostInc,
        Self::MovLLoadPostInc,
        Self::MovBR0LoadIndexed,
        Self::MovWR0LoadIndexed,
        Self::MovLR0LoadIndexed,
        Self::MovBR0StoreIndexed,
        Self::MovWR0StoreIndexed,
        Self::MovLR0StoreIndexed,
        Self::MovBDispStore,
        Self::MovWDispStore,
        Self::MovLDispStore,
        Self::MovBDispLoad,
        Self::MovWDispLoad,
        Self::MovLDispLoad,
        Self::MovBGbrStore,
        Self::MovBGbrLoad,
        Self::MovWGbrStore,
        Self::MovWGbrLoad,
        Self::MovLGbrStore,
        Self::MovLGbrLoad,
        Self::Mova,
        Self::MovT,
        Self::Add,
        Self::AddImm,
        Self::Addc,
        Self::Addv,
        Self::Sub,
        Self::Subc,
        Self::Subv,
        Self::Neg,
        Self::Negc,
        Self::Dt,
        Self::MulL,
        Self::MulS,
        Self::MulU,
        Self::DmulS,
        Self::DmulU,
        Self::Div0S,
        Self::Div0U,
        Self::Div1,
        Self::MacL,
        Self::MacW,
        Self::And,
        Self::AndImm,
        Self::AndB,
        Self::Or,
        Self::OrImm,
        Self::OrB,
        Self::Xor,
        Self::XorImm,
        Self::XorB,
        Self::Not,
        Self::Tst,
        Self::TstImm,
        Self::TstB,
        Self::Shll,
        Self::Shlr,
        Self::Shal,
        Self::Shar,
        Self::Shll2,
        Self::Shlr2,
        Self::Shll8,
        Self::Shlr8,
        Self::Shll16,
        Self::Shlr16,
        Self::Rotl,
        Self::Rotr,
        Self::Rotcl,
        Self::Rotcr,
        Self::ExtsB,
        Self::ExtsW,
        Self::ExtuB,
        Self::ExtuW,
        Self::SwapB,
        Self::SwapW,
        Self::Xtrct,
        Self::CmpEq,
        Self::CmpEqImm,
        Self::CmpGe,
        Self::CmpGt,
        Self::CmpHi,
        Self::CmpHs,
        Self::CmpPl,
        Self::CmpPz,
        Self::CmpStr,
        Self::Bt,
        Self::Bf,
        Self::Bts,
        Self::Bfs,
        Self::Bra,
        Self::Bsr,
        Self::Jmp,
        Self::Jsr,
        Self::Rts,
        Self::Rte,
        Self::LdcGbr,
        Self::LdcVbr,
        Self::LdcSr,
        Self::LdcLGbr,
        Self::LdcLVbr,
        Self::LdcLSr,
        Self::StcGbr,
        Self::StcVbr,
        Self::StcSr,
        Self::StcLGbr,
        Self::StcLVbr,
        Self::StcLSr,
        Self::LdsMach,
        Self::LdsMacl,
        Self::LdsPr,
        Self::LdsLMach,
        Self::LdsLMacl,
        Self::LdsLPr,
        Self::StsMach,
        Self::StsMacl,
        Self::StsPr,
        Self::StsLMach,
        Self::StsLMacl,
        Self::StsLPr,
        Self::Nop,
        Self::Sleep,
        Self::SetT,
        Self::ClrT,
        Self::ClrMac,
        Self::Trapa,
        Self::Bsrf,
        Self::Braf,
        Self::Tas,
    ];

    /// Complete semantic-form denominator for profile-level checks.
    pub fn all() -> impl Iterator<Item = Self> {
        Self::ALL.iter().copied()
    }
}

impl Instruction {
    /// Returns the operand-free semantic form of this instruction.
    pub fn form(&self) -> InstructionForm {
        match self {
            Self::MovImm { .. } => InstructionForm::MovImm,
            Self::MovWPcRel { .. } => InstructionForm::MovWPcRel,
            Self::MovLPcRel { .. } => InstructionForm::MovLPcRel,
            Self::MovReg { .. } => InstructionForm::MovReg,
            Self::MovBStore { .. } => InstructionForm::MovBStore,
            Self::MovWStore { .. } => InstructionForm::MovWStore,
            Self::MovLStore { .. } => InstructionForm::MovLStore,
            Self::MovBLoad { .. } => InstructionForm::MovBLoad,
            Self::MovWLoad { .. } => InstructionForm::MovWLoad,
            Self::MovLLoad { .. } => InstructionForm::MovLLoad,
            Self::MovBStorePreDec { .. } => InstructionForm::MovBStorePreDec,
            Self::MovWStorePreDec { .. } => InstructionForm::MovWStorePreDec,
            Self::MovLStorePreDec { .. } => InstructionForm::MovLStorePreDec,
            Self::MovBLoadPostInc { .. } => InstructionForm::MovBLoadPostInc,
            Self::MovWLoadPostInc { .. } => InstructionForm::MovWLoadPostInc,
            Self::MovLLoadPostInc { .. } => InstructionForm::MovLLoadPostInc,
            Self::MovBR0LoadIndexed { .. } => InstructionForm::MovBR0LoadIndexed,
            Self::MovWR0LoadIndexed { .. } => InstructionForm::MovWR0LoadIndexed,
            Self::MovLR0LoadIndexed { .. } => InstructionForm::MovLR0LoadIndexed,
            Self::MovBR0StoreIndexed { .. } => InstructionForm::MovBR0StoreIndexed,
            Self::MovWR0StoreIndexed { .. } => InstructionForm::MovWR0StoreIndexed,
            Self::MovLR0StoreIndexed { .. } => InstructionForm::MovLR0StoreIndexed,
            Self::MovBDispStore { .. } => InstructionForm::MovBDispStore,
            Self::MovWDispStore { .. } => InstructionForm::MovWDispStore,
            Self::MovLDispStore { .. } => InstructionForm::MovLDispStore,
            Self::MovBDispLoad { .. } => InstructionForm::MovBDispLoad,
            Self::MovWDispLoad { .. } => InstructionForm::MovWDispLoad,
            Self::MovLDispLoad { .. } => InstructionForm::MovLDispLoad,
            Self::MovBGbrStore { .. } => InstructionForm::MovBGbrStore,
            Self::MovBGbrLoad { .. } => InstructionForm::MovBGbrLoad,
            Self::MovWGbrStore { .. } => InstructionForm::MovWGbrStore,
            Self::MovWGbrLoad { .. } => InstructionForm::MovWGbrLoad,
            Self::MovLGbrStore { .. } => InstructionForm::MovLGbrStore,
            Self::MovLGbrLoad { .. } => InstructionForm::MovLGbrLoad,
            Self::Mova { .. } => InstructionForm::Mova,
            Self::MovT { .. } => InstructionForm::MovT,
            Self::Add { .. } => InstructionForm::Add,
            Self::AddImm { .. } => InstructionForm::AddImm,
            Self::Addc { .. } => InstructionForm::Addc,
            Self::Addv { .. } => InstructionForm::Addv,
            Self::Sub { .. } => InstructionForm::Sub,
            Self::Subc { .. } => InstructionForm::Subc,
            Self::Subv { .. } => InstructionForm::Subv,
            Self::Neg { .. } => InstructionForm::Neg,
            Self::Negc { .. } => InstructionForm::Negc,
            Self::Dt { .. } => InstructionForm::Dt,
            Self::MulL { .. } => InstructionForm::MulL,
            Self::MulS { .. } => InstructionForm::MulS,
            Self::MulU { .. } => InstructionForm::MulU,
            Self::DmulS { .. } => InstructionForm::DmulS,
            Self::DmulU { .. } => InstructionForm::DmulU,
            Self::Div0S { .. } => InstructionForm::Div0S,
            Self::Div0U => InstructionForm::Div0U,
            Self::Div1 { .. } => InstructionForm::Div1,
            Self::MacL { .. } => InstructionForm::MacL,
            Self::MacW { .. } => InstructionForm::MacW,
            Self::And { .. } => InstructionForm::And,
            Self::AndImm { .. } => InstructionForm::AndImm,
            Self::AndB { .. } => InstructionForm::AndB,
            Self::Or { .. } => InstructionForm::Or,
            Self::OrImm { .. } => InstructionForm::OrImm,
            Self::OrB { .. } => InstructionForm::OrB,
            Self::Xor { .. } => InstructionForm::Xor,
            Self::XorImm { .. } => InstructionForm::XorImm,
            Self::XorB { .. } => InstructionForm::XorB,
            Self::Not { .. } => InstructionForm::Not,
            Self::Tst { .. } => InstructionForm::Tst,
            Self::TstImm { .. } => InstructionForm::TstImm,
            Self::TstB { .. } => InstructionForm::TstB,
            Self::Shll { .. } => InstructionForm::Shll,
            Self::Shlr { .. } => InstructionForm::Shlr,
            Self::Shal { .. } => InstructionForm::Shal,
            Self::Shar { .. } => InstructionForm::Shar,
            Self::Shll2 { .. } => InstructionForm::Shll2,
            Self::Shlr2 { .. } => InstructionForm::Shlr2,
            Self::Shll8 { .. } => InstructionForm::Shll8,
            Self::Shlr8 { .. } => InstructionForm::Shlr8,
            Self::Shll16 { .. } => InstructionForm::Shll16,
            Self::Shlr16 { .. } => InstructionForm::Shlr16,
            Self::Rotl { .. } => InstructionForm::Rotl,
            Self::Rotr { .. } => InstructionForm::Rotr,
            Self::Rotcl { .. } => InstructionForm::Rotcl,
            Self::Rotcr { .. } => InstructionForm::Rotcr,
            Self::ExtsB { .. } => InstructionForm::ExtsB,
            Self::ExtsW { .. } => InstructionForm::ExtsW,
            Self::ExtuB { .. } => InstructionForm::ExtuB,
            Self::ExtuW { .. } => InstructionForm::ExtuW,
            Self::SwapB { .. } => InstructionForm::SwapB,
            Self::SwapW { .. } => InstructionForm::SwapW,
            Self::Xtrct { .. } => InstructionForm::Xtrct,
            Self::CmpEq { .. } => InstructionForm::CmpEq,
            Self::CmpEqImm { .. } => InstructionForm::CmpEqImm,
            Self::CmpGe { .. } => InstructionForm::CmpGe,
            Self::CmpGt { .. } => InstructionForm::CmpGt,
            Self::CmpHi { .. } => InstructionForm::CmpHi,
            Self::CmpHs { .. } => InstructionForm::CmpHs,
            Self::CmpPl { .. } => InstructionForm::CmpPl,
            Self::CmpPz { .. } => InstructionForm::CmpPz,
            Self::CmpStr { .. } => InstructionForm::CmpStr,
            Self::Bt { .. } => InstructionForm::Bt,
            Self::Bf { .. } => InstructionForm::Bf,
            Self::Bts { .. } => InstructionForm::Bts,
            Self::Bfs { .. } => InstructionForm::Bfs,
            Self::Bra { .. } => InstructionForm::Bra,
            Self::Bsr { .. } => InstructionForm::Bsr,
            Self::Jmp { .. } => InstructionForm::Jmp,
            Self::Jsr { .. } => InstructionForm::Jsr,
            Self::Rts => InstructionForm::Rts,
            Self::Rte => InstructionForm::Rte,
            Self::LdcGbr { .. } => InstructionForm::LdcGbr,
            Self::LdcVbr { .. } => InstructionForm::LdcVbr,
            Self::LdcSr { .. } => InstructionForm::LdcSr,
            Self::LdcLGbr { .. } => InstructionForm::LdcLGbr,
            Self::LdcLVbr { .. } => InstructionForm::LdcLVbr,
            Self::LdcLSr { .. } => InstructionForm::LdcLSr,
            Self::StcGbr { .. } => InstructionForm::StcGbr,
            Self::StcVbr { .. } => InstructionForm::StcVbr,
            Self::StcSr { .. } => InstructionForm::StcSr,
            Self::StcLGbr { .. } => InstructionForm::StcLGbr,
            Self::StcLVbr { .. } => InstructionForm::StcLVbr,
            Self::StcLSr { .. } => InstructionForm::StcLSr,
            Self::LdsMach { .. } => InstructionForm::LdsMach,
            Self::LdsMacl { .. } => InstructionForm::LdsMacl,
            Self::LdsPr { .. } => InstructionForm::LdsPr,
            Self::LdsLMach { .. } => InstructionForm::LdsLMach,
            Self::LdsLMacl { .. } => InstructionForm::LdsLMacl,
            Self::LdsLPr { .. } => InstructionForm::LdsLPr,
            Self::StsMach { .. } => InstructionForm::StsMach,
            Self::StsMacl { .. } => InstructionForm::StsMacl,
            Self::StsPr { .. } => InstructionForm::StsPr,
            Self::StsLMach { .. } => InstructionForm::StsLMach,
            Self::StsLMacl { .. } => InstructionForm::StsLMacl,
            Self::StsLPr { .. } => InstructionForm::StsLPr,
            Self::Nop => InstructionForm::Nop,
            Self::Sleep => InstructionForm::Sleep,
            Self::SetT => InstructionForm::SetT,
            Self::ClrT => InstructionForm::ClrT,
            Self::ClrMac => InstructionForm::ClrMac,
            Self::Trapa { .. } => InstructionForm::Trapa,
            Self::Bsrf { .. } => InstructionForm::Bsrf,
            Self::Braf { .. } => InstructionForm::Braf,
            Self::Tas { .. } => InstructionForm::Tas,
        }
    }

    /// Whether shared static semantics report a delay slot.
    pub fn has_delay_slot(&self) -> bool {
        crate::semantics::semantics(self, 0)
            .expect("zero is an aligned SH-2 analysis point")
            .control_flow
            .delay_slot
            .is_some()
    }

    /// Whether the instruction is forbidden in a delayed branch's slot.
    pub fn is_illegal_delay_slot(&self) -> bool {
        matches!(
            self,
            Self::Bt { .. }
                | Self::Bf { .. }
                | Self::Bra { .. }
                | Self::Bsr { .. }
                | Self::Jmp { .. }
                | Self::Jsr { .. }
                | Self::Rts
                | Self::Rte
                | Self::Trapa { .. }
                | Self::Bts { .. }
                | Self::Bfs { .. }
                | Self::Braf { .. }
                | Self::Bsrf { .. }
        )
    }

    /// Compute the PC-relative branch target code location.
    ///
    /// `pc` is the address of this instruction itself.
    /// Returns `None` for non-branch instructions or register-indirect branches.
    pub fn branch_target(&self, location: crate::CodeLocation) -> Option<crate::CodeLocation> {
        let pc = location;
        match self {
            Self::Bt { disp } | Self::Bf { disp } | Self::Bts { disp } | Self::Bfs { disp } => {
                // target = PC + 4 + sign_extended(disp) * 2
                Some((pc as i64 + 4 + (*disp as i64) * 2) as u32)
            }
            Self::Bra { disp } | Self::Bsr { disp } => {
                // target = PC + 4 + sign_extended(disp) * 2  (12-bit signed)
                Some((pc as i64 + 4 + (*disp as i64) * 2) as u32)
            }
            _ => None,
        }
    }

    /// Compute the literal pool address for PC-relative load instructions.
    ///
    /// `pc` is the address of this instruction itself.
    pub fn literal_pool_address(&self, location: crate::CodeLocation) -> Option<u32> {
        let pc = location;
        match self {
            Self::MovLPcRel { disp, .. } => {
                // addr = (PC & 0xFFFFFFFC) + 4 + disp * 4
                Some(
                    (pc & 0xFFFF_FFFC)
                        .wrapping_add(4)
                        .wrapping_add(u32::from(*disp) * 4),
                )
            }
            Self::MovWPcRel { disp, .. } => {
                // addr = PC + 4 + disp * 2  (no alignment)
                Some(pc.wrapping_add(4).wrapping_add(u32::from(*disp) * 2))
            }
            Self::Mova { disp } => {
                // addr = (PC & 0xFFFFFFFC) + 4 + disp * 4
                Some(
                    (pc & 0xFFFF_FFFC)
                        .wrapping_add(4)
                        .wrapping_add(u32::from(*disp) * 4),
                )
            }
            _ => None,
        }
    }
}
