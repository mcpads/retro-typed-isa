use std::fmt;

/// One of the sixteen SH-4 general-purpose registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Register(u8);

impl Register {
    pub const R0: Self = Self(0);
    pub const R1: Self = Self(1);
    pub const R2: Self = Self(2);
    pub const R3: Self = Self(3);
    pub const R4: Self = Self(4);
    pub const R5: Self = Self(5);
    pub const R6: Self = Self(6);
    pub const R7: Self = Self(7);
    pub const R8: Self = Self(8);
    pub const R9: Self = Self(9);
    pub const R10: Self = Self(10);
    pub const R11: Self = Self(11);
    pub const R12: Self = Self(12);
    pub const R13: Self = Self(13);
    pub const R14: Self = Self(14);
    pub const R15: Self = Self(15);
    pub const SP: Self = Self::R15;

    pub const fn new(index: u8) -> Option<Self> {
        if index < 16 { Some(Self(index)) } else { None }
    }

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

/// One of the eight banked general-purpose registers, R0_BANK through R7_BANK.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BankedRegister(u8);

impl BankedRegister {
    pub const R0: Self = Self(0);
    pub const R1: Self = Self(1);
    pub const R2: Self = Self(2);
    pub const R3: Self = Self(3);
    pub const R4: Self = Self(4);
    pub const R5: Self = Self(5);
    pub const R6: Self = Self(6);
    pub const R7: Self = Self(7);

    pub const fn new(index: u8) -> Option<Self> {
        if index < 8 { Some(Self(index)) } else { None }
    }

    pub const fn index(self) -> u8 {
        self.0
    }

    pub(crate) const fn from_field(index: u8) -> Self {
        debug_assert!(index < 8);
        Self(index)
    }
}

impl fmt::Display for BankedRegister {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "R{}_BANK", self.0)
    }
}

/// One of the sixteen single-precision registers, FR0 through FR15.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FloatRegister(u8);

impl FloatRegister {
    pub const fn new(index: u8) -> Option<Self> {
        if index < 16 { Some(Self(index)) } else { None }
    }

    pub const fn index(self) -> u8 {
        self.0
    }

    pub(crate) const fn from_field(index: u8) -> Self {
        debug_assert!(index < 16);
        Self(index)
    }
}

impl fmt::Display for FloatRegister {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "FR{}", self.0)
    }
}

/// One of the eight even-numbered double-precision registers, DR0 through DR14.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DoubleRegister(u8);

impl DoubleRegister {
    pub const fn new(index: u8) -> Option<Self> {
        if index < 16 && index & 1 == 0 {
            Some(Self(index))
        } else {
            None
        }
    }

    pub const fn index(self) -> u8 {
        self.0
    }

    pub(crate) const fn from_field(field: u8) -> Self {
        debug_assert!(field < 8);
        Self(field * 2)
    }

    pub(crate) const fn field(self) -> u8 {
        self.0 / 2
    }
}

impl fmt::Display for DoubleRegister {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "DR{}", self.0)
    }
}

/// One of the eight even-numbered extended double registers, XD0 through XD14.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExtendedDoubleRegister(u8);

impl ExtendedDoubleRegister {
    pub const fn new(index: u8) -> Option<Self> {
        if index < 16 && index & 1 == 0 {
            Some(Self(index))
        } else {
            None
        }
    }

    pub const fn index(self) -> u8 {
        self.0
    }

    pub(crate) const fn from_field(field: u8) -> Self {
        debug_assert!(field < 8);
        Self(field * 2)
    }
}

impl fmt::Display for ExtendedDoubleRegister {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "XD{}", self.0)
    }
}

/// A 64-bit register selected by an FMOV instruction while FPSCR.SZ is one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FpuPairRegister {
    Double(DoubleRegister),
    Extended(ExtendedDoubleRegister),
}

impl FpuPairRegister {
    pub(crate) const fn from_selector(selector: u8) -> Self {
        debug_assert!(selector < 16);
        let field = selector >> 1;
        if selector & 1 == 0 {
            Self::Double(DoubleRegister::from_field(field))
        } else {
            Self::Extended(ExtendedDoubleRegister::from_field(field))
        }
    }

    pub(crate) const fn selector(self) -> u8 {
        match self {
            Self::Double(register) => register.field() << 1,
            Self::Extended(register) => (register.index() / 2) << 1 | 1,
        }
    }
}

impl fmt::Display for FpuPairRegister {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Double(register) => register.fmt(formatter),
            Self::Extended(register) => register.fmt(formatter),
        }
    }
}

/// One of the four vector registers, FV0, FV4, FV8, or FV12.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VectorRegister(u8);

impl VectorRegister {
    pub const fn new(index: u8) -> Option<Self> {
        if index < 16 && index & 3 == 0 {
            Some(Self(index))
        } else {
            None
        }
    }

    pub const fn index(self) -> u8 {
        self.0
    }

    pub(crate) const fn from_field(field: u8) -> Self {
        debug_assert!(field < 4);
        Self(field * 4)
    }

    pub(crate) const fn field(self) -> u8 {
        self.0 / 4
    }
}

impl fmt::Display for VectorRegister {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "FV{}", self.0)
    }
}

/// FPSCR.PR, which selects single- or double-precision arithmetic decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum FpuPrecision {
    #[default]
    Single,
    Double,
}

/// FPSCR.SZ, which selects 32- or 64-bit FMOV register interpretation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum FpuTransferSize {
    #[default]
    Single,
    Pair,
}

/// The FPSCR bits that affect SH-4 instruction decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct FpscrState {
    precision: FpuPrecision,
    transfer_size: FpuTransferSize,
}

impl FpscrState {
    pub const fn new(precision: FpuPrecision, transfer_size: FpuTransferSize) -> Self {
        Self {
            precision,
            transfer_size,
        }
    }

    pub const fn precision(self) -> FpuPrecision {
        self.precision
    }

    pub const fn transfer_size(self) -> FpuTransferSize {
        self.transfer_size
    }
}

/// A complete typed instruction in the declared SH-4 profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    // === Data transfer ===
    /// MOV #imm, Rn (Enii -- 8-bit sign-extended immediate)
    MovImm {
        imm: i8,
        rn: Register,
    },
    /// MOV.W @(disp,PC), Rn (9ndd)
    MovWPcRel {
        disp: u8,
        rn: Register,
    },
    /// MOV.L @(disp,PC), Rn (Dndd)
    MovLPcRel {
        disp: u8,
        rn: Register,
    },
    /// MOV Rm, Rn (6nm3)
    MovReg {
        rm: Register,
        rn: Register,
    },
    /// MOV.B Rm, @Rn (2nm0)
    MovBStore {
        rm: Register,
        rn: Register,
    },
    /// MOV.W Rm, @Rn (2nm1)
    MovWStore {
        rm: Register,
        rn: Register,
    },
    /// MOV.L Rm, @Rn (2nm2)
    MovLStore {
        rm: Register,
        rn: Register,
    },
    /// MOV.B @Rm, Rn (6nm0)
    MovBLoad {
        rm: Register,
        rn: Register,
    },
    /// MOV.W @Rm, Rn (6nm1)
    MovWLoad {
        rm: Register,
        rn: Register,
    },
    /// MOV.L @Rm, Rn (6nm2)
    MovLLoad {
        rm: Register,
        rn: Register,
    },
    /// MOV.B Rm, @-Rn (2nm4)
    MovBStorePreDec {
        rm: Register,
        rn: Register,
    },
    /// MOV.W Rm, @-Rn (2nm5)
    MovWStorePreDec {
        rm: Register,
        rn: Register,
    },
    /// MOV.L Rm, @-Rn (2nm6)
    MovLStorePreDec {
        rm: Register,
        rn: Register,
    },
    /// MOV.B @Rm+, Rn (6nm4)
    MovBLoadPostInc {
        rm: Register,
        rn: Register,
    },
    /// MOV.W @Rm+, Rn (6nm5)
    MovWLoadPostInc {
        rm: Register,
        rn: Register,
    },
    /// MOV.L @Rm+, Rn (6nm6)
    MovLLoadPostInc {
        rm: Register,
        rn: Register,
    },
    /// MOV.B @(R0,Rm), Rn (0nmC)
    MovBR0LoadIndexed {
        rm: Register,
        rn: Register,
    },
    /// MOV.W @(R0,Rm), Rn (0nmD)
    MovWR0LoadIndexed {
        rm: Register,
        rn: Register,
    },
    /// MOV.L @(R0,Rm), Rn (0nmE)
    MovLR0LoadIndexed {
        rm: Register,
        rn: Register,
    },
    /// MOV.B Rm, @(R0,Rn) (0nm4)
    MovBR0StoreIndexed {
        rm: Register,
        rn: Register,
    },
    /// MOV.W Rm, @(R0,Rn) (0nm5)
    MovWR0StoreIndexed {
        rm: Register,
        rn: Register,
    },
    /// MOV.L Rm, @(R0,Rn) (0nm6)
    MovLR0StoreIndexed {
        rm: Register,
        rn: Register,
    },
    /// MOV.B R0, @(disp,Rn) (80nd)
    MovBDispStore {
        disp: u8,
        rn: Register,
    },
    /// MOV.W R0, @(disp,Rn) (81nd)
    MovWDispStore {
        disp: u8,
        rn: Register,
    },
    /// MOV.L Rm, @(disp,Rn) (1nmd)
    MovLDispStore {
        disp: u8,
        rm: Register,
        rn: Register,
    },
    /// MOV.B @(disp,Rm), R0 (84md)
    MovBDispLoad {
        disp: u8,
        rm: Register,
    },
    /// MOV.W @(disp,Rm), R0 (85md)
    MovWDispLoad {
        disp: u8,
        rm: Register,
    },
    /// MOV.L @(disp,Rm), Rn (5nmd)
    MovLDispLoad {
        disp: u8,
        rm: Register,
        rn: Register,
    },
    /// MOV.B R0, @(disp,GBR) (C0dd)
    MovBGbrStore {
        disp: u8,
    },
    /// MOV.B @(disp,GBR), R0 (C4dd)
    MovBGbrLoad {
        disp: u8,
    },
    /// MOV.W R0, @(disp,GBR) (C1dd)
    MovWGbrStore {
        disp: u8,
    },
    /// MOV.W @(disp,GBR), R0 (C5dd)
    MovWGbrLoad {
        disp: u8,
    },
    /// MOV.L R0, @(disp,GBR) (C2dd)
    MovLGbrStore {
        disp: u8,
    },
    /// MOV.L @(disp,GBR), R0 (C6dd)
    MovLGbrLoad {
        disp: u8,
    },
    /// MOVA @(disp,PC), R0 (C7dd)
    Mova {
        disp: u8,
    },
    /// MOVT Rn (0n29)
    MovT {
        rn: Register,
    },

    // === Arithmetic ===
    /// ADD Rm, Rn (3nmc)
    Add {
        rm: Register,
        rn: Register,
    },
    /// ADD #imm, Rn (7nii)
    AddImm {
        imm: i8,
        rn: Register,
    },
    /// ADDC Rm, Rn (3nme)
    Addc {
        rm: Register,
        rn: Register,
    },
    /// ADDV Rm, Rn (3nmf)
    Addv {
        rm: Register,
        rn: Register,
    },
    /// SUB Rm, Rn (3nm8)
    Sub {
        rm: Register,
        rn: Register,
    },
    /// SUBC Rm, Rn (3nma)
    Subc {
        rm: Register,
        rn: Register,
    },
    /// SUBV Rm, Rn (3nmb)
    Subv {
        rm: Register,
        rn: Register,
    },
    /// NEG Rm, Rn (6nmb)
    Neg {
        rm: Register,
        rn: Register,
    },
    /// NEGC Rm, Rn (6nma)
    Negc {
        rm: Register,
        rn: Register,
    },
    /// DT Rn (4n10)
    Dt {
        rn: Register,
    },
    /// MUL.L Rm, Rn (0nm7)
    MulL {
        rm: Register,
        rn: Register,
    },
    /// MULS.W Rm, Rn (2nmf)
    MulS {
        rm: Register,
        rn: Register,
    },
    /// MULU.W Rm, Rn (2nme)
    MulU {
        rm: Register,
        rn: Register,
    },
    /// DMULS.L Rm, Rn (3nmd)
    DmulS {
        rm: Register,
        rn: Register,
    },
    /// DMULU.L Rm, Rn (3nm5)
    DmulU {
        rm: Register,
        rn: Register,
    },
    /// DIV0S Rm, Rn (2nm7)
    Div0S {
        rm: Register,
        rn: Register,
    },
    /// DIV0U (0019)
    Div0U,
    /// DIV1 Rm, Rn (3nm4)
    Div1 {
        rm: Register,
        rn: Register,
    },
    /// MAC.L @Rm+, @Rn+ (0nmf)
    MacL {
        rm: Register,
        rn: Register,
    },
    /// MAC.W @Rm+, @Rn+ (4nmf)
    MacW {
        rm: Register,
        rn: Register,
    },

    // === Logical ===
    /// AND Rm, Rn (2nm9)
    And {
        rm: Register,
        rn: Register,
    },
    /// AND #imm, R0 (C9ii)
    AndImm {
        imm: u8,
    },
    /// AND.B #imm, @(R0,GBR) (CDii)
    AndB {
        imm: u8,
    },
    /// OR Rm, Rn (2nmb)
    Or {
        rm: Register,
        rn: Register,
    },
    /// OR #imm, R0 (CBii)
    OrImm {
        imm: u8,
    },
    /// OR.B #imm, @(R0,GBR) (CFii)
    OrB {
        imm: u8,
    },
    /// XOR Rm, Rn (2nma)
    Xor {
        rm: Register,
        rn: Register,
    },
    /// XOR #imm, R0 (CAii)
    XorImm {
        imm: u8,
    },
    /// XOR.B #imm, @(R0,GBR) (CEii)
    XorB {
        imm: u8,
    },
    /// NOT Rm, Rn (6nm7)
    Not {
        rm: Register,
        rn: Register,
    },
    /// TST Rm, Rn (2nm8)
    Tst {
        rm: Register,
        rn: Register,
    },
    /// TST #imm, R0 (C8ii)
    TstImm {
        imm: u8,
    },
    /// TST.B #imm, @(R0,GBR) (CCii)
    TstB {
        imm: u8,
    },

    // === Shifts ===
    /// SHLL Rn (4n00)
    Shll {
        rn: Register,
    },
    /// SHLR Rn (4n01)
    Shlr {
        rn: Register,
    },
    /// SHAL Rn (4n20)
    Shal {
        rn: Register,
    },
    /// SHAR Rn (4n21)
    Shar {
        rn: Register,
    },
    /// SHLL2 Rn (4n08)
    Shll2 {
        rn: Register,
    },
    /// SHLR2 Rn (4n09)
    Shlr2 {
        rn: Register,
    },
    /// SHLL8 Rn (4n18)
    Shll8 {
        rn: Register,
    },
    /// SHLR8 Rn (4n19)
    Shlr8 {
        rn: Register,
    },
    /// SHLL16 Rn (4n28)
    Shll16 {
        rn: Register,
    },
    /// SHLR16 Rn (4n29)
    Shlr16 {
        rn: Register,
    },
    /// ROTL Rn (4n04)
    Rotl {
        rn: Register,
    },
    /// ROTR Rn (4n05)
    Rotr {
        rn: Register,
    },
    /// ROTCL Rn (4n24)
    Rotcl {
        rn: Register,
    },
    /// ROTCR Rn (4n25)
    Rotcr {
        rn: Register,
    },
    /// SHAD Rm, Rn (4nmC)
    Shad {
        rm: Register,
        rn: Register,
    },
    /// SHLD Rm, Rn (4nmD)
    Shld {
        rm: Register,
        rn: Register,
    },

    // === Sign/zero extension ===
    /// EXTS.B Rm, Rn (6nme)
    ExtsB {
        rm: Register,
        rn: Register,
    },
    /// EXTS.W Rm, Rn (6nmf)
    ExtsW {
        rm: Register,
        rn: Register,
    },
    /// EXTU.B Rm, Rn (6nmc)
    ExtuB {
        rm: Register,
        rn: Register,
    },
    /// EXTU.W Rm, Rn (6nmd)
    ExtuW {
        rm: Register,
        rn: Register,
    },
    /// SWAP.B Rm, Rn (6nm8)
    SwapB {
        rm: Register,
        rn: Register,
    },
    /// SWAP.W Rm, Rn (6nm9)
    SwapW {
        rm: Register,
        rn: Register,
    },
    /// XTRCT Rm, Rn (2nmd)
    Xtrct {
        rm: Register,
        rn: Register,
    },

    // === Comparisons (set T-bit) ===
    /// CMP/EQ Rm, Rn (3nm0)
    CmpEq {
        rm: Register,
        rn: Register,
    },
    /// CMP/EQ #imm, R0 (88ii)
    CmpEqImm {
        imm: i8,
    },
    /// CMP/GE Rm, Rn (3nm3)
    CmpGe {
        rm: Register,
        rn: Register,
    },
    /// CMP/GT Rm, Rn (3nm7)
    CmpGt {
        rm: Register,
        rn: Register,
    },
    /// CMP/HI Rm, Rn (3nm6)
    CmpHi {
        rm: Register,
        rn: Register,
    },
    /// CMP/HS Rm, Rn (3nm2)
    CmpHs {
        rm: Register,
        rn: Register,
    },
    /// CMP/PL Rn (4n15)
    CmpPl {
        rn: Register,
    },
    /// CMP/PZ Rn (4n11)
    CmpPz {
        rn: Register,
    },
    /// CMP/STR Rm, Rn (2nmc)
    CmpStr {
        rm: Register,
        rn: Register,
    },

    // === Branches ===
    /// BT disp (89dd) -- no delay slot
    Bt {
        disp: i8,
    },
    /// BF disp (8Bdd) -- no delay slot
    Bf {
        disp: i8,
    },
    /// BT/S disp (8Ddd) -- delay slot
    Bts {
        disp: i8,
    },
    /// BF/S disp (8Fdd) -- delay slot
    Bfs {
        disp: i8,
    },
    /// BRA disp (Addd) -- 12-bit signed
    Bra {
        disp: i16,
    },
    /// BSR disp (Bddd) -- 12-bit signed
    Bsr {
        disp: i16,
    },
    /// JMP @Rm (4n2b)
    Jmp {
        rm: Register,
    },
    /// JSR @Rm (4n0b)
    Jsr {
        rm: Register,
    },
    /// RTS (000B)
    Rts,
    /// RTE (002B)
    Rte,

    // === System register transfer ===
    /// LDC Rm, GBR (4n1e)
    LdcGbr {
        rm: Register,
    },
    /// LDC Rm, VBR (4n2e)
    LdcVbr {
        rm: Register,
    },
    /// LDC Rm, SR (4n0e)
    LdcSr {
        rm: Register,
    },
    /// LDC.L @Rm+, GBR (4n17)
    LdcLGbr {
        rm: Register,
    },
    /// LDC.L @Rm+, VBR (4n27)
    LdcLVbr {
        rm: Register,
    },
    /// LDC.L @Rm+, SR (4n07)
    LdcLSr {
        rm: Register,
    },
    /// STC GBR, Rn (0n12)
    StcGbr {
        rn: Register,
    },
    /// STC VBR, Rn (0n22)
    StcVbr {
        rn: Register,
    },
    /// STC SR, Rn (0n02)
    StcSr {
        rn: Register,
    },
    /// STC.L GBR, @-Rn (4n13)
    StcLGbr {
        rn: Register,
    },
    /// STC.L VBR, @-Rn (4n23)
    StcLVbr {
        rn: Register,
    },
    /// STC.L SR, @-Rn (4n03)
    StcLSr {
        rn: Register,
    },
    /// LDC Rm, SSR (4n3E)
    LdcSsr {
        rm: Register,
    },
    /// LDC Rm, SPC (4n4E)
    LdcSpc {
        rm: Register,
    },
    /// LDC Rm, DBR (4nFA)
    LdcDbr {
        rm: Register,
    },
    /// LDC Rm, Rn_BANK (4n8E through 4nFE)
    LdcBank {
        rm: Register,
        bank: BankedRegister,
    },
    /// LDC.L @Rm+, SSR (4n37)
    LdcLSsr {
        rm: Register,
    },
    /// LDC.L @Rm+, SPC (4n47)
    LdcLSpc {
        rm: Register,
    },
    /// LDC.L @Rm+, DBR (4nF6)
    LdcLDbr {
        rm: Register,
    },
    /// LDC.L @Rm+, Rn_BANK (4n87 through 4nF7)
    LdcLBank {
        rm: Register,
        bank: BankedRegister,
    },
    /// STC SSR, Rn (0n32)
    StcSsr {
        rn: Register,
    },
    /// STC SPC, Rn (0n42)
    StcSpc {
        rn: Register,
    },
    /// STC SGR, Rn (0n3A)
    StcSgr {
        rn: Register,
    },
    /// STC DBR, Rn (0nFA)
    StcDbr {
        rn: Register,
    },
    /// STC Rm_BANK, Rn (0n82 through 0nF2)
    StcBank {
        bank: BankedRegister,
        rn: Register,
    },
    /// STC.L SSR, @-Rn (4n33)
    StcLSsr {
        rn: Register,
    },
    /// STC.L SPC, @-Rn (4n43)
    StcLSpc {
        rn: Register,
    },
    /// STC.L SGR, @-Rn (4n32)
    StcLSgr {
        rn: Register,
    },
    /// STC.L DBR, @-Rn (4nF2)
    StcLDbr {
        rn: Register,
    },
    /// STC.L Rm_BANK, @-Rn (4n83 through 4nF3)
    StcLBank {
        bank: BankedRegister,
        rn: Register,
    },
    /// LDS Rm, MACH (4n0a)
    LdsMach {
        rm: Register,
    },
    /// LDS Rm, MACL (4n1a)
    LdsMacl {
        rm: Register,
    },
    /// LDS Rm, PR (4n2a)
    LdsPr {
        rm: Register,
    },
    /// LDS.L @Rm+, MACH (4n06)
    LdsLMach {
        rm: Register,
    },
    /// LDS.L @Rm+, MACL (4n16)
    LdsLMacl {
        rm: Register,
    },
    /// LDS.L @Rm+, PR (4n26)
    LdsLPr {
        rm: Register,
    },
    /// STS MACH, Rn (0n0a)
    StsMach {
        rn: Register,
    },
    /// STS MACL, Rn (0n1a)
    StsMacl {
        rn: Register,
    },
    /// STS PR, Rn (0n2a)
    StsPr {
        rn: Register,
    },
    /// STS.L MACH, @-Rn (4n02)
    StsLMach {
        rn: Register,
    },
    /// STS.L MACL, @-Rn (4n12)
    StsLMacl {
        rn: Register,
    },
    /// STS.L PR, @-Rn (4n22)
    StsLPr {
        rn: Register,
    },
    /// LDS Rm, FPUL (4n5A)
    LdsFpul {
        rm: Register,
    },
    /// LDS Rm, FPSCR (4n6A)
    LdsFpscr {
        rm: Register,
    },
    /// LDS.L @Rm+, FPUL (4n56)
    LdsLFpul {
        rm: Register,
    },
    /// LDS.L @Rm+, FPSCR (4n66)
    LdsLFpscr {
        rm: Register,
    },
    /// STS FPUL, Rn (0n5A)
    StsFpul {
        rn: Register,
    },
    /// STS FPSCR, Rn (0n6A)
    StsFpscr {
        rn: Register,
    },
    /// STS.L FPUL, @-Rn (4n52)
    StsLFpul {
        rn: Register,
    },
    /// STS.L FPSCR, @-Rn (4n62)
    StsLFpscr {
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
    /// CLRS (0048)
    ClrS,
    /// SETS (0058)
    SetS,
    /// LDTLB (0038)
    Ldtlb,
    /// TRAPA #imm (C3ii)
    Trapa {
        imm: u8,
    },
    /// BSRF Rm (0n03) -- Branch to Subroutine Far: PC + 4 + Rm → PC, PC + 4 → PR
    Bsrf {
        rm: Register,
    },
    /// BRAF Rm (0n23) -- Branch Far: PC + 4 + Rm → PC
    Braf {
        rm: Register,
    },
    /// TAS.B @Rn (4n1B) -- Test And Set byte at @Rn
    Tas {
        rn: Register,
    },

    // === Cache operations ===
    Pref {
        rn: Register,
    },
    Ocbi {
        rn: Register,
    },
    Ocbp {
        rn: Register,
    },
    Ocbwb {
        rn: Register,
    },
    MovcaL {
        rn: Register,
    },

    // === Single-precision floating-point operations (FPSCR.PR = 0) ===
    Fldi0 {
        frn: FloatRegister,
    },
    Fldi1 {
        frn: FloatRegister,
    },
    Fmov {
        frm: FloatRegister,
        frn: FloatRegister,
    },
    FmovLoad {
        rm: Register,
        frn: FloatRegister,
    },
    FmovLoadIndexed {
        rm: Register,
        frn: FloatRegister,
    },
    FmovLoadPostInc {
        rm: Register,
        frn: FloatRegister,
    },
    FmovStore {
        frm: FloatRegister,
        rn: Register,
    },
    FmovStoreIndexed {
        frm: FloatRegister,
        rn: Register,
    },
    FmovStorePreDec {
        frm: FloatRegister,
        rn: Register,
    },
    Flds {
        frm: FloatRegister,
    },
    Fsts {
        frn: FloatRegister,
    },
    Fabs {
        frn: FloatRegister,
    },
    Fadd {
        frm: FloatRegister,
        frn: FloatRegister,
    },
    FcmpEq {
        frm: FloatRegister,
        frn: FloatRegister,
    },
    FcmpGt {
        frm: FloatRegister,
        frn: FloatRegister,
    },
    Fdiv {
        frm: FloatRegister,
        frn: FloatRegister,
    },
    Float {
        frn: FloatRegister,
    },
    Fmac {
        frm: FloatRegister,
        frn: FloatRegister,
    },
    Fmul {
        frm: FloatRegister,
        frn: FloatRegister,
    },
    Fneg {
        frn: FloatRegister,
    },
    Fsqrt {
        frn: FloatRegister,
    },
    Fsub {
        frm: FloatRegister,
        frn: FloatRegister,
    },
    Ftrc {
        frm: FloatRegister,
    },

    // === Pair transfers (FPSCR.SZ = 1) ===
    FmovPair {
        source: FpuPairRegister,
        destination: FpuPairRegister,
    },
    FmovPairLoad {
        rm: Register,
        destination: FpuPairRegister,
    },
    FmovPairLoadIndexed {
        rm: Register,
        destination: FpuPairRegister,
    },
    FmovPairLoadPostInc {
        rm: Register,
        destination: FpuPairRegister,
    },
    FmovPairStore {
        source: FpuPairRegister,
        rn: Register,
    },
    FmovPairStoreIndexed {
        source: FpuPairRegister,
        rn: Register,
    },
    FmovPairStorePreDec {
        source: FpuPairRegister,
        rn: Register,
    },

    // === Double-precision operations (FPSCR.PR = 1) ===
    FabsDouble {
        drn: DoubleRegister,
    },
    FaddDouble {
        drm: DoubleRegister,
        drn: DoubleRegister,
    },
    FcmpEqDouble {
        drm: DoubleRegister,
        drn: DoubleRegister,
    },
    FcmpGtDouble {
        drm: DoubleRegister,
        drn: DoubleRegister,
    },
    FdivDouble {
        drm: DoubleRegister,
        drn: DoubleRegister,
    },
    Fcnvds {
        drm: DoubleRegister,
    },
    Fcnvsd {
        drn: DoubleRegister,
    },
    FloatDouble {
        drn: DoubleRegister,
    },
    FmulDouble {
        drm: DoubleRegister,
        drn: DoubleRegister,
    },
    FnegDouble {
        drn: DoubleRegister,
    },
    FsqrtDouble {
        drn: DoubleRegister,
    },
    FsubDouble {
        drm: DoubleRegister,
        drn: DoubleRegister,
    },
    FtrcDouble {
        drm: DoubleRegister,
    },

    // === Graphics acceleration and FPSCR bank/size controls ===
    Fipr {
        fvm: VectorRegister,
        fvn: VectorRegister,
    },
    Ftrv {
        fvn: VectorRegister,
    },
    Frchg,
    Fschg,
}

/// Operand-free semantic forms in the complete SH-4 profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InstructionForm {
    MovImm,
    MovWPcRel,
    MovLPcRel,
    MovReg,
    MovBStore,
    MovWStore,
    MovLStore,
    MovBLoad,
    MovWLoad,
    MovLLoad,
    MovBStorePreDec,
    MovWStorePreDec,
    MovLStorePreDec,
    MovBLoadPostInc,
    MovWLoadPostInc,
    MovLLoadPostInc,
    MovBR0LoadIndexed,
    MovWR0LoadIndexed,
    MovLR0LoadIndexed,
    MovBR0StoreIndexed,
    MovWR0StoreIndexed,
    MovLR0StoreIndexed,
    MovBDispStore,
    MovWDispStore,
    MovLDispStore,
    MovBDispLoad,
    MovWDispLoad,
    MovLDispLoad,
    MovBGbrStore,
    MovBGbrLoad,
    MovWGbrStore,
    MovWGbrLoad,
    MovLGbrStore,
    MovLGbrLoad,
    Mova,
    MovT,
    Add,
    AddImm,
    Addc,
    Addv,
    Sub,
    Subc,
    Subv,
    Neg,
    Negc,
    Dt,
    MulL,
    MulS,
    MulU,
    DmulS,
    DmulU,
    Div0S,
    Div0U,
    Div1,
    MacL,
    MacW,
    And,
    AndImm,
    AndB,
    Or,
    OrImm,
    OrB,
    Xor,
    XorImm,
    XorB,
    Not,
    Tst,
    TstImm,
    TstB,
    Shll,
    Shlr,
    Shal,
    Shar,
    Shll2,
    Shlr2,
    Shll8,
    Shlr8,
    Shll16,
    Shlr16,
    Rotl,
    Rotr,
    Rotcl,
    Rotcr,
    ExtsB,
    ExtsW,
    ExtuB,
    ExtuW,
    SwapB,
    SwapW,
    Xtrct,
    CmpEq,
    CmpEqImm,
    CmpGe,
    CmpGt,
    CmpHi,
    CmpHs,
    CmpPl,
    CmpPz,
    CmpStr,
    Bt,
    Bf,
    Bts,
    Bfs,
    Bra,
    Bsr,
    Jmp,
    Jsr,
    Rts,
    Rte,
    LdcGbr,
    LdcVbr,
    LdcSr,
    LdcLGbr,
    LdcLVbr,
    LdcLSr,
    StcGbr,
    StcVbr,
    StcSr,
    StcLGbr,
    StcLVbr,
    StcLSr,
    LdsMach,
    LdsMacl,
    LdsPr,
    LdsLMach,
    LdsLMacl,
    LdsLPr,
    StsMach,
    StsMacl,
    StsPr,
    StsLMach,
    StsLMacl,
    StsLPr,
    Nop,
    Sleep,
    SetT,
    ClrT,
    ClrMac,
    Trapa,
    Bsrf,
    Braf,
    Tas,
    Shad,
    Shld,
    LdcSsr,
    LdcSpc,
    LdcDbr,
    LdcBank,
    LdcLSsr,
    LdcLSpc,
    LdcLDbr,
    LdcLBank,
    StcSsr,
    StcSpc,
    StcSgr,
    StcDbr,
    StcBank,
    StcLSsr,
    StcLSpc,
    StcLSgr,
    StcLDbr,
    StcLBank,
    LdsFpul,
    LdsFpscr,
    LdsLFpul,
    LdsLFpscr,
    StsFpul,
    StsFpscr,
    StsLFpul,
    StsLFpscr,
    ClrS,
    SetS,
    Ldtlb,
    Pref,
    Ocbi,
    Ocbp,
    Ocbwb,
    MovcaL,
    Fldi0,
    Fldi1,
    Fmov,
    FmovLoad,
    FmovLoadIndexed,
    FmovLoadPostInc,
    FmovStore,
    FmovStoreIndexed,
    FmovStorePreDec,
    Flds,
    Fsts,
    Fabs,
    Fadd,
    FcmpEq,
    FcmpGt,
    Fdiv,
    Float,
    Fmac,
    Fmul,
    Fneg,
    Fsqrt,
    Fsub,
    Ftrc,
    FmovPair,
    FmovPairLoad,
    FmovPairLoadIndexed,
    FmovPairLoadPostInc,
    FmovPairStore,
    FmovPairStoreIndexed,
    FmovPairStorePreDec,
    FabsDouble,
    FaddDouble,
    FcmpEqDouble,
    FcmpGtDouble,
    FdivDouble,
    Fcnvds,
    Fcnvsd,
    FloatDouble,
    FmulDouble,
    FnegDouble,
    FsqrtDouble,
    FsubDouble,
    FtrcDouble,
    Fipr,
    Ftrv,
    Frchg,
    Fschg,
}

impl InstructionForm {
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
        Self::Shad,
        Self::Shld,
        Self::LdcSsr,
        Self::LdcSpc,
        Self::LdcDbr,
        Self::LdcBank,
        Self::LdcLSsr,
        Self::LdcLSpc,
        Self::LdcLDbr,
        Self::LdcLBank,
        Self::StcSsr,
        Self::StcSpc,
        Self::StcSgr,
        Self::StcDbr,
        Self::StcBank,
        Self::StcLSsr,
        Self::StcLSpc,
        Self::StcLSgr,
        Self::StcLDbr,
        Self::StcLBank,
        Self::LdsFpul,
        Self::LdsFpscr,
        Self::LdsLFpul,
        Self::LdsLFpscr,
        Self::StsFpul,
        Self::StsFpscr,
        Self::StsLFpul,
        Self::StsLFpscr,
        Self::ClrS,
        Self::SetS,
        Self::Ldtlb,
        Self::Pref,
        Self::Ocbi,
        Self::Ocbp,
        Self::Ocbwb,
        Self::MovcaL,
        Self::Fldi0,
        Self::Fldi1,
        Self::Fmov,
        Self::FmovLoad,
        Self::FmovLoadIndexed,
        Self::FmovLoadPostInc,
        Self::FmovStore,
        Self::FmovStoreIndexed,
        Self::FmovStorePreDec,
        Self::Flds,
        Self::Fsts,
        Self::Fabs,
        Self::Fadd,
        Self::FcmpEq,
        Self::FcmpGt,
        Self::Fdiv,
        Self::Float,
        Self::Fmac,
        Self::Fmul,
        Self::Fneg,
        Self::Fsqrt,
        Self::Fsub,
        Self::Ftrc,
        Self::FmovPair,
        Self::FmovPairLoad,
        Self::FmovPairLoadIndexed,
        Self::FmovPairLoadPostInc,
        Self::FmovPairStore,
        Self::FmovPairStoreIndexed,
        Self::FmovPairStorePreDec,
        Self::FabsDouble,
        Self::FaddDouble,
        Self::FcmpEqDouble,
        Self::FcmpGtDouble,
        Self::FdivDouble,
        Self::Fcnvds,
        Self::Fcnvsd,
        Self::FloatDouble,
        Self::FmulDouble,
        Self::FnegDouble,
        Self::FsqrtDouble,
        Self::FsubDouble,
        Self::FtrcDouble,
        Self::Fipr,
        Self::Ftrv,
        Self::Frchg,
        Self::Fschg,
    ];

    /// Complete semantic-form denominator for profile-level checks.
    pub fn all() -> impl Iterator<Item = Self> {
        Self::ALL.iter().copied()
    }
}

impl Instruction {
    /// Return the operand-free semantic form of this instruction.
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
            Self::Shad { .. } => InstructionForm::Shad,
            Self::Shld { .. } => InstructionForm::Shld,
            Self::LdcSsr { .. } => InstructionForm::LdcSsr,
            Self::LdcSpc { .. } => InstructionForm::LdcSpc,
            Self::LdcDbr { .. } => InstructionForm::LdcDbr,
            Self::LdcBank { .. } => InstructionForm::LdcBank,
            Self::LdcLSsr { .. } => InstructionForm::LdcLSsr,
            Self::LdcLSpc { .. } => InstructionForm::LdcLSpc,
            Self::LdcLDbr { .. } => InstructionForm::LdcLDbr,
            Self::LdcLBank { .. } => InstructionForm::LdcLBank,
            Self::StcSsr { .. } => InstructionForm::StcSsr,
            Self::StcSpc { .. } => InstructionForm::StcSpc,
            Self::StcSgr { .. } => InstructionForm::StcSgr,
            Self::StcDbr { .. } => InstructionForm::StcDbr,
            Self::StcBank { .. } => InstructionForm::StcBank,
            Self::StcLSsr { .. } => InstructionForm::StcLSsr,
            Self::StcLSpc { .. } => InstructionForm::StcLSpc,
            Self::StcLSgr { .. } => InstructionForm::StcLSgr,
            Self::StcLDbr { .. } => InstructionForm::StcLDbr,
            Self::StcLBank { .. } => InstructionForm::StcLBank,
            Self::LdsFpul { .. } => InstructionForm::LdsFpul,
            Self::LdsFpscr { .. } => InstructionForm::LdsFpscr,
            Self::LdsLFpul { .. } => InstructionForm::LdsLFpul,
            Self::LdsLFpscr { .. } => InstructionForm::LdsLFpscr,
            Self::StsFpul { .. } => InstructionForm::StsFpul,
            Self::StsFpscr { .. } => InstructionForm::StsFpscr,
            Self::StsLFpul { .. } => InstructionForm::StsLFpul,
            Self::StsLFpscr { .. } => InstructionForm::StsLFpscr,
            Self::ClrS => InstructionForm::ClrS,
            Self::SetS => InstructionForm::SetS,
            Self::Ldtlb => InstructionForm::Ldtlb,
            Self::Pref { .. } => InstructionForm::Pref,
            Self::Ocbi { .. } => InstructionForm::Ocbi,
            Self::Ocbp { .. } => InstructionForm::Ocbp,
            Self::Ocbwb { .. } => InstructionForm::Ocbwb,
            Self::MovcaL { .. } => InstructionForm::MovcaL,
            Self::Fldi0 { .. } => InstructionForm::Fldi0,
            Self::Fldi1 { .. } => InstructionForm::Fldi1,
            Self::Fmov { .. } => InstructionForm::Fmov,
            Self::FmovLoad { .. } => InstructionForm::FmovLoad,
            Self::FmovLoadIndexed { .. } => InstructionForm::FmovLoadIndexed,
            Self::FmovLoadPostInc { .. } => InstructionForm::FmovLoadPostInc,
            Self::FmovStore { .. } => InstructionForm::FmovStore,
            Self::FmovStoreIndexed { .. } => InstructionForm::FmovStoreIndexed,
            Self::FmovStorePreDec { .. } => InstructionForm::FmovStorePreDec,
            Self::Flds { .. } => InstructionForm::Flds,
            Self::Fsts { .. } => InstructionForm::Fsts,
            Self::Fabs { .. } => InstructionForm::Fabs,
            Self::Fadd { .. } => InstructionForm::Fadd,
            Self::FcmpEq { .. } => InstructionForm::FcmpEq,
            Self::FcmpGt { .. } => InstructionForm::FcmpGt,
            Self::Fdiv { .. } => InstructionForm::Fdiv,
            Self::Float { .. } => InstructionForm::Float,
            Self::Fmac { .. } => InstructionForm::Fmac,
            Self::Fmul { .. } => InstructionForm::Fmul,
            Self::Fneg { .. } => InstructionForm::Fneg,
            Self::Fsqrt { .. } => InstructionForm::Fsqrt,
            Self::Fsub { .. } => InstructionForm::Fsub,
            Self::Ftrc { .. } => InstructionForm::Ftrc,
            Self::FmovPair { .. } => InstructionForm::FmovPair,
            Self::FmovPairLoad { .. } => InstructionForm::FmovPairLoad,
            Self::FmovPairLoadIndexed { .. } => InstructionForm::FmovPairLoadIndexed,
            Self::FmovPairLoadPostInc { .. } => InstructionForm::FmovPairLoadPostInc,
            Self::FmovPairStore { .. } => InstructionForm::FmovPairStore,
            Self::FmovPairStoreIndexed { .. } => InstructionForm::FmovPairStoreIndexed,
            Self::FmovPairStorePreDec { .. } => InstructionForm::FmovPairStorePreDec,
            Self::FabsDouble { .. } => InstructionForm::FabsDouble,
            Self::FaddDouble { .. } => InstructionForm::FaddDouble,
            Self::FcmpEqDouble { .. } => InstructionForm::FcmpEqDouble,
            Self::FcmpGtDouble { .. } => InstructionForm::FcmpGtDouble,
            Self::FdivDouble { .. } => InstructionForm::FdivDouble,
            Self::Fcnvds { .. } => InstructionForm::Fcnvds,
            Self::Fcnvsd { .. } => InstructionForm::Fcnvsd,
            Self::FloatDouble { .. } => InstructionForm::FloatDouble,
            Self::FmulDouble { .. } => InstructionForm::FmulDouble,
            Self::FnegDouble { .. } => InstructionForm::FnegDouble,
            Self::FsqrtDouble { .. } => InstructionForm::FsqrtDouble,
            Self::FsubDouble { .. } => InstructionForm::FsubDouble,
            Self::FtrcDouble { .. } => InstructionForm::FtrcDouble,
            Self::Fipr { .. } => InstructionForm::Fipr,
            Self::Ftrv { .. } => InstructionForm::Ftrv,
            Self::Frchg => InstructionForm::Frchg,
            Self::Fschg => InstructionForm::Fschg,
        }
    }

    /// Static control-flow facts at the address where this instruction is placed.
    pub fn control_flow(&self, pc: u32) -> ControlFlow {
        let next = pc.wrapping_add(2);
        let after_delay_slot = pc.wrapping_add(4);
        match self {
            Self::Bt { .. } | Self::Bf { .. } => ControlFlow::Branch {
                target: self.branch_target(pc).expect("PC-relative branch"),
                fallthrough: Some(next),
                delay_slot: None,
            },
            Self::Bts { .. } | Self::Bfs { .. } => ControlFlow::Branch {
                target: self.branch_target(pc).expect("PC-relative branch"),
                fallthrough: Some(after_delay_slot),
                delay_slot: Some(next),
            },
            Self::Bra { .. } => ControlFlow::Branch {
                target: self.branch_target(pc).expect("PC-relative branch"),
                fallthrough: None,
                delay_slot: Some(next),
            },
            Self::Bsr { .. } => ControlFlow::Call {
                target: self.branch_target(pc),
                return_address: after_delay_slot,
                delay_slot: next,
            },
            Self::Braf { .. } | Self::Jmp { .. } => ControlFlow::Jump {
                target: None,
                delay_slot: next,
            },
            Self::Bsrf { .. } | Self::Jsr { .. } => ControlFlow::Call {
                target: None,
                return_address: after_delay_slot,
                delay_slot: next,
            },
            Self::Rts => ControlFlow::Return { delay_slot: next },
            Self::Rte => ControlFlow::ExceptionReturn { delay_slot: next },
            Self::Trapa { .. } => ControlFlow::Interrupt,
            _ => ControlFlow::FallThrough { next },
        }
    }

    /// Whether this instruction has a delay slot.
    pub fn has_delay_slot(&self) -> bool {
        matches!(
            self,
            Self::Bts { .. }
                | Self::Bfs { .. }
                | Self::Bra { .. }
                | Self::Bsr { .. }
                | Self::Braf { .. }
                | Self::Bsrf { .. }
                | Self::Jmp { .. }
                | Self::Jsr { .. }
                | Self::Rts
                | Self::Rte
        )
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
                | Self::MovWPcRel { .. }
                | Self::MovLPcRel { .. }
                | Self::Mova { .. }
        )
    }

    /// Compute the PC-relative branch target address.
    ///
    /// `pc` is the address of this instruction itself.
    /// Returns `None` for non-branch instructions or register-indirect branches.
    pub fn branch_target(&self, pc: u32) -> Option<u32> {
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
    pub fn literal_pool_addr(&self, pc: u32) -> Option<u32> {
        match self {
            Self::MovLPcRel { disp, .. } => {
                // addr = (PC & 0xFFFFFFFC) + 4 + disp * 4
                Some((pc & 0xFFFF_FFFC) + 4 + (*disp as u32) * 4)
            }
            Self::MovWPcRel { disp, .. } => {
                // addr = PC + 4 + disp * 2  (no alignment)
                Some(pc + 4 + (*disp as u32) * 2)
            }
            Self::Mova { disp } => {
                // addr = (PC & 0xFFFFFFFC) + 4 + disp * 4
                Some((pc & 0xFFFF_FFFC) + 4 + (*disp as u32) * 4)
            }
            _ => None,
        }
    }
}

/// Placement-aware static control-flow classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlFlow {
    FallThrough {
        next: u32,
    },
    Branch {
        target: u32,
        fallthrough: Option<u32>,
        delay_slot: Option<u32>,
    },
    Call {
        target: Option<u32>,
        return_address: u32,
        delay_slot: u32,
    },
    Jump {
        target: Option<u32>,
        delay_slot: u32,
    },
    Return {
        delay_slot: u32,
    },
    ExceptionReturn {
        delay_slot: u32,
    },
    Interrupt,
}
