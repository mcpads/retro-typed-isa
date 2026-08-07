use std::fmt;

/// One of the sixteen SH-4 general-purpose registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Register(u8);

impl Register {
    /// The r0 register or complete form set.
    pub const R0: Self = Self(0);
    /// The r1 register or complete form set.
    pub const R1: Self = Self(1);
    /// The r2 register or complete form set.
    pub const R2: Self = Self(2);
    /// The r3 register or complete form set.
    pub const R3: Self = Self(3);
    /// The r4 register or complete form set.
    pub const R4: Self = Self(4);
    /// The r5 register or complete form set.
    pub const R5: Self = Self(5);
    /// The r6 register or complete form set.
    pub const R6: Self = Self(6);
    /// The r7 register or complete form set.
    pub const R7: Self = Self(7);
    /// The r8 register or complete form set.
    pub const R8: Self = Self(8);
    /// The r9 register or complete form set.
    pub const R9: Self = Self(9);
    /// The r10 register or complete form set.
    pub const R10: Self = Self(10);
    /// The r11 register or complete form set.
    pub const R11: Self = Self(11);
    /// The r12 register or complete form set.
    pub const R12: Self = Self(12);
    /// The r13 register or complete form set.
    pub const R13: Self = Self(13);
    /// The r14 register or complete form set.
    pub const R14: Self = Self(14);
    /// The r15 register or complete form set.
    pub const R15: Self = Self(15);
    /// The sp register or complete form set.
    pub const SP: Self = Self::R15;

    /// Constructs the value after checking register alignment or records explicit FPSCR state.
    pub const fn new(index: u8) -> Option<Self> {
        if index < 16 { Some(Self(index)) } else { None }
    }

    /// Returns the encoded register index.
    pub const fn index(self) -> u8 {
        self.0
    }

    /// Performs the from field operation.
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
    /// The r0 register or complete form set.
    pub const R0: Self = Self(0);
    /// The r1 register or complete form set.
    pub const R1: Self = Self(1);
    /// The r2 register or complete form set.
    pub const R2: Self = Self(2);
    /// The r3 register or complete form set.
    pub const R3: Self = Self(3);
    /// The r4 register or complete form set.
    pub const R4: Self = Self(4);
    /// The r5 register or complete form set.
    pub const R5: Self = Self(5);
    /// The r6 register or complete form set.
    pub const R6: Self = Self(6);
    /// The r7 register or complete form set.
    pub const R7: Self = Self(7);

    /// Constructs the value after checking register alignment or records explicit FPSCR state.
    pub const fn new(index: u8) -> Option<Self> {
        if index < 8 { Some(Self(index)) } else { None }
    }

    /// Returns the encoded register index.
    pub const fn index(self) -> u8 {
        self.0
    }

    /// Performs the from field operation.
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
    /// Constructs the value after checking register alignment or records explicit FPSCR state.
    pub const fn new(index: u8) -> Option<Self> {
        if index < 16 { Some(Self(index)) } else { None }
    }

    /// Returns the encoded register index.
    pub const fn index(self) -> u8 {
        self.0
    }

    /// Performs the from field operation.
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
    /// Constructs the value after checking register alignment or records explicit FPSCR state.
    pub const fn new(index: u8) -> Option<Self> {
        if index < 16 && index & 1 == 0 {
            Some(Self(index))
        } else {
            None
        }
    }

    /// Returns the encoded register index.
    pub const fn index(self) -> u8 {
        self.0
    }

    /// Performs the from field operation.
    pub(crate) const fn from_field(field: u8) -> Self {
        debug_assert!(field < 8);
        Self(field * 2)
    }

    /// Performs the field operation.
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
    /// Constructs the value after checking register alignment or records explicit FPSCR state.
    pub const fn new(index: u8) -> Option<Self> {
        if index < 16 && index & 1 == 0 {
            Some(Self(index))
        } else {
            None
        }
    }

    /// Returns the encoded register index.
    pub const fn index(self) -> u8 {
        self.0
    }

    /// Performs the from field operation.
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
    /// The double form.
    Double(DoubleRegister),
    /// The extended form.
    Extended(ExtendedDoubleRegister),
}

impl FpuPairRegister {
    /// Performs the from selector operation.
    pub(crate) const fn from_selector(selector: u8) -> Self {
        debug_assert!(selector < 16);
        let field = selector >> 1;
        if selector & 1 == 0 {
            Self::Double(DoubleRegister::from_field(field))
        } else {
            Self::Extended(ExtendedDoubleRegister::from_field(field))
        }
    }

    /// Performs the selector operation.
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
    /// Constructs the value after checking register alignment or records explicit FPSCR state.
    pub const fn new(index: u8) -> Option<Self> {
        if index < 16 && index & 3 == 0 {
            Some(Self(index))
        } else {
            None
        }
    }

    /// Returns the encoded register index.
    pub const fn index(self) -> u8 {
        self.0
    }

    /// Performs the from field operation.
    pub(crate) const fn from_field(field: u8) -> Self {
        debug_assert!(field < 4);
        Self(field * 4)
    }

    /// Performs the field operation.
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
    /// The single form.
    Single,
    /// The double form.
    Double,
}

/// FPSCR.SZ, which selects 32- or 64-bit FMOV register interpretation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum FpuTransferSize {
    #[default]
    /// The single form.
    Single,
    /// The pair form.
    Pair,
}

/// The FPSCR bits that affect SH-4 instruction decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct FpscrState {
    precision: FpuPrecision,
    transfer_size: FpuTransferSize,
}

impl FpscrState {
    /// Constructs the value after checking register alignment or records explicit FPSCR state.
    pub const fn new(precision: FpuPrecision, transfer_size: FpuTransferSize) -> Self {
        Self {
            precision,
            transfer_size,
        }
    }

    /// Returns the FPSCR.PR arithmetic precision.
    pub const fn precision(self) -> FpuPrecision {
        self.precision
    }

    /// Returns the FPSCR.SZ transfer width.
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
        /// The imm operand.
        imm: i8,
        /// The rn operand.
        rn: Register,
    },
    /// MOV.W @(disp,PC), Rn (9ndd)
    MovWPcRel {
        /// The disp operand.
        disp: u8,
        /// The rn operand.
        rn: Register,
    },
    /// MOV.L @(disp,PC), Rn (Dndd)
    MovLPcRel {
        /// The disp operand.
        disp: u8,
        /// The rn operand.
        rn: Register,
    },
    /// MOV Rm, Rn (6nm3)
    MovReg {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// MOV.B Rm, @Rn (2nm0)
    MovBStore {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// MOV.W Rm, @Rn (2nm1)
    MovWStore {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// MOV.L Rm, @Rn (2nm2)
    MovLStore {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// MOV.B @Rm, Rn (6nm0)
    MovBLoad {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// MOV.W @Rm, Rn (6nm1)
    MovWLoad {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// MOV.L @Rm, Rn (6nm2)
    MovLLoad {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// MOV.B Rm, @-Rn (2nm4)
    MovBStorePreDec {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// MOV.W Rm, @-Rn (2nm5)
    MovWStorePreDec {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// MOV.L Rm, @-Rn (2nm6)
    MovLStorePreDec {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// MOV.B @Rm+, Rn (6nm4)
    MovBLoadPostInc {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// MOV.W @Rm+, Rn (6nm5)
    MovWLoadPostInc {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// MOV.L @Rm+, Rn (6nm6)
    MovLLoadPostInc {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// MOV.B @(R0,Rm), Rn (0nmC)
    MovBR0LoadIndexed {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// MOV.W @(R0,Rm), Rn (0nmD)
    MovWR0LoadIndexed {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// MOV.L @(R0,Rm), Rn (0nmE)
    MovLR0LoadIndexed {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// MOV.B Rm, @(R0,Rn) (0nm4)
    MovBR0StoreIndexed {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// MOV.W Rm, @(R0,Rn) (0nm5)
    MovWR0StoreIndexed {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// MOV.L Rm, @(R0,Rn) (0nm6)
    MovLR0StoreIndexed {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// MOV.B R0, @(disp,Rn) (80nd)
    MovBDispStore {
        /// The disp operand.
        disp: u8,
        /// The rn operand.
        rn: Register,
    },
    /// MOV.W R0, @(disp,Rn) (81nd)
    MovWDispStore {
        /// The disp operand.
        disp: u8,
        /// The rn operand.
        rn: Register,
    },
    /// MOV.L Rm, @(disp,Rn) (1nmd)
    MovLDispStore {
        /// The disp operand.
        disp: u8,
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// MOV.B @(disp,Rm), R0 (84md)
    MovBDispLoad {
        /// The disp operand.
        disp: u8,
        /// The rm operand.
        rm: Register,
    },
    /// MOV.W @(disp,Rm), R0 (85md)
    MovWDispLoad {
        /// The disp operand.
        disp: u8,
        /// The rm operand.
        rm: Register,
    },
    /// MOV.L @(disp,Rm), Rn (5nmd)
    MovLDispLoad {
        /// The disp operand.
        disp: u8,
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// MOV.B R0, @(disp,GBR) (C0dd)
    MovBGbrStore {
        /// The disp operand.
        disp: u8,
    },
    /// MOV.B @(disp,GBR), R0 (C4dd)
    MovBGbrLoad {
        /// The disp operand.
        disp: u8,
    },
    /// MOV.W R0, @(disp,GBR) (C1dd)
    MovWGbrStore {
        /// The disp operand.
        disp: u8,
    },
    /// MOV.W @(disp,GBR), R0 (C5dd)
    MovWGbrLoad {
        /// The disp operand.
        disp: u8,
    },
    /// MOV.L R0, @(disp,GBR) (C2dd)
    MovLGbrStore {
        /// The disp operand.
        disp: u8,
    },
    /// MOV.L @(disp,GBR), R0 (C6dd)
    MovLGbrLoad {
        /// The disp operand.
        disp: u8,
    },
    /// MOVA @(disp,PC), R0 (C7dd)
    Mova {
        /// The disp operand.
        disp: u8,
    },
    /// MOVT Rn (0n29)
    MovT {
        /// The rn operand.
        rn: Register,
    },

    // === Arithmetic ===
    /// ADD Rm, Rn (3nmc)
    Add {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// ADD #imm, Rn (7nii)
    AddImm {
        /// The imm operand.
        imm: i8,
        /// The rn operand.
        rn: Register,
    },
    /// ADDC Rm, Rn (3nme)
    Addc {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// ADDV Rm, Rn (3nmf)
    Addv {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// SUB Rm, Rn (3nm8)
    Sub {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// SUBC Rm, Rn (3nma)
    Subc {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// SUBV Rm, Rn (3nmb)
    Subv {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// NEG Rm, Rn (6nmb)
    Neg {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// NEGC Rm, Rn (6nma)
    Negc {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// DT Rn (4n10)
    Dt {
        /// The rn operand.
        rn: Register,
    },
    /// MUL.L Rm, Rn (0nm7)
    MulL {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// MULS.W Rm, Rn (2nmf)
    MulS {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// MULU.W Rm, Rn (2nme)
    MulU {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// DMULS.L Rm, Rn (3nmd)
    DmulS {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// DMULU.L Rm, Rn (3nm5)
    DmulU {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// DIV0S Rm, Rn (2nm7)
    Div0S {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// DIV0U (0019)
    Div0U,
    /// DIV1 Rm, Rn (3nm4)
    Div1 {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// MAC.L @Rm+, @Rn+ (0nmf)
    MacL {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// MAC.W @Rm+, @Rn+ (4nmf)
    MacW {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },

    // === Logical ===
    /// AND Rm, Rn (2nm9)
    And {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// AND #imm, R0 (C9ii)
    AndImm {
        /// The imm operand.
        imm: u8,
    },
    /// AND.B #imm, @(R0,GBR) (CDii)
    AndB {
        /// The imm operand.
        imm: u8,
    },
    /// OR Rm, Rn (2nmb)
    Or {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// OR #imm, R0 (CBii)
    OrImm {
        /// The imm operand.
        imm: u8,
    },
    /// OR.B #imm, @(R0,GBR) (CFii)
    OrB {
        /// The imm operand.
        imm: u8,
    },
    /// XOR Rm, Rn (2nma)
    Xor {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// XOR #imm, R0 (CAii)
    XorImm {
        /// The imm operand.
        imm: u8,
    },
    /// XOR.B #imm, @(R0,GBR) (CEii)
    XorB {
        /// The imm operand.
        imm: u8,
    },
    /// NOT Rm, Rn (6nm7)
    Not {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// TST Rm, Rn (2nm8)
    Tst {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// TST #imm, R0 (C8ii)
    TstImm {
        /// The imm operand.
        imm: u8,
    },
    /// TST.B #imm, @(R0,GBR) (CCii)
    TstB {
        /// The imm operand.
        imm: u8,
    },

    // === Shifts ===
    /// SHLL Rn (4n00)
    Shll {
        /// The rn operand.
        rn: Register,
    },
    /// SHLR Rn (4n01)
    Shlr {
        /// The rn operand.
        rn: Register,
    },
    /// SHAL Rn (4n20)
    Shal {
        /// The rn operand.
        rn: Register,
    },
    /// SHAR Rn (4n21)
    Shar {
        /// The rn operand.
        rn: Register,
    },
    /// SHLL2 Rn (4n08)
    Shll2 {
        /// The rn operand.
        rn: Register,
    },
    /// SHLR2 Rn (4n09)
    Shlr2 {
        /// The rn operand.
        rn: Register,
    },
    /// SHLL8 Rn (4n18)
    Shll8 {
        /// The rn operand.
        rn: Register,
    },
    /// SHLR8 Rn (4n19)
    Shlr8 {
        /// The rn operand.
        rn: Register,
    },
    /// SHLL16 Rn (4n28)
    Shll16 {
        /// The rn operand.
        rn: Register,
    },
    /// SHLR16 Rn (4n29)
    Shlr16 {
        /// The rn operand.
        rn: Register,
    },
    /// ROTL Rn (4n04)
    Rotl {
        /// The rn operand.
        rn: Register,
    },
    /// ROTR Rn (4n05)
    Rotr {
        /// The rn operand.
        rn: Register,
    },
    /// ROTCL Rn (4n24)
    Rotcl {
        /// The rn operand.
        rn: Register,
    },
    /// ROTCR Rn (4n25)
    Rotcr {
        /// The rn operand.
        rn: Register,
    },
    /// SHAD Rm, Rn (4nmC)
    Shad {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// SHLD Rm, Rn (4nmD)
    Shld {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },

    // === Sign/zero extension ===
    /// EXTS.B Rm, Rn (6nme)
    ExtsB {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// EXTS.W Rm, Rn (6nmf)
    ExtsW {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// EXTU.B Rm, Rn (6nmc)
    ExtuB {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// EXTU.W Rm, Rn (6nmd)
    ExtuW {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// SWAP.B Rm, Rn (6nm8)
    SwapB {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// SWAP.W Rm, Rn (6nm9)
    SwapW {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// XTRCT Rm, Rn (2nmd)
    Xtrct {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },

    // === Comparisons (set T-bit) ===
    /// CMP/EQ Rm, Rn (3nm0)
    CmpEq {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// CMP/EQ #imm, R0 (88ii)
    CmpEqImm {
        /// The imm operand.
        imm: i8,
    },
    /// CMP/GE Rm, Rn (3nm3)
    CmpGe {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// CMP/GT Rm, Rn (3nm7)
    CmpGt {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// CMP/HI Rm, Rn (3nm6)
    CmpHi {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// CMP/HS Rm, Rn (3nm2)
    CmpHs {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },
    /// CMP/PL Rn (4n15)
    CmpPl {
        /// The rn operand.
        rn: Register,
    },
    /// CMP/PZ Rn (4n11)
    CmpPz {
        /// The rn operand.
        rn: Register,
    },
    /// CMP/STR Rm, Rn (2nmc)
    CmpStr {
        /// The rm operand.
        rm: Register,
        /// The rn operand.
        rn: Register,
    },

    // === Branches ===
    /// BT disp (89dd) -- no delay slot
    Bt {
        /// The disp operand.
        disp: i8,
    },
    /// BF disp (8Bdd) -- no delay slot
    Bf {
        /// The disp operand.
        disp: i8,
    },
    /// BT/S disp (8Ddd) -- delay slot
    Bts {
        /// The disp operand.
        disp: i8,
    },
    /// BF/S disp (8Fdd) -- delay slot
    Bfs {
        /// The disp operand.
        disp: i8,
    },
    /// BRA disp (Addd) -- 12-bit signed
    Bra {
        /// The disp operand.
        disp: i16,
    },
    /// BSR disp (Bddd) -- 12-bit signed
    Bsr {
        /// The disp operand.
        disp: i16,
    },
    /// JMP @Rm (4n2b)
    Jmp {
        /// The rm operand.
        rm: Register,
    },
    /// JSR @Rm (4n0b)
    Jsr {
        /// The rm operand.
        rm: Register,
    },
    /// RTS (000B)
    Rts,
    /// RTE (002B)
    Rte,

    // === System register transfer ===
    /// LDC Rm, GBR (4n1e)
    LdcGbr {
        /// The rm operand.
        rm: Register,
    },
    /// LDC Rm, VBR (4n2e)
    LdcVbr {
        /// The rm operand.
        rm: Register,
    },
    /// LDC Rm, SR (4n0e)
    LdcSr {
        /// The rm operand.
        rm: Register,
    },
    /// LDC.L @Rm+, GBR (4n17)
    LdcLGbr {
        /// The rm operand.
        rm: Register,
    },
    /// LDC.L @Rm+, VBR (4n27)
    LdcLVbr {
        /// The rm operand.
        rm: Register,
    },
    /// LDC.L @Rm+, SR (4n07)
    LdcLSr {
        /// The rm operand.
        rm: Register,
    },
    /// STC GBR, Rn (0n12)
    StcGbr {
        /// The rn operand.
        rn: Register,
    },
    /// STC VBR, Rn (0n22)
    StcVbr {
        /// The rn operand.
        rn: Register,
    },
    /// STC SR, Rn (0n02)
    StcSr {
        /// The rn operand.
        rn: Register,
    },
    /// STC.L GBR, @-Rn (4n13)
    StcLGbr {
        /// The rn operand.
        rn: Register,
    },
    /// STC.L VBR, @-Rn (4n23)
    StcLVbr {
        /// The rn operand.
        rn: Register,
    },
    /// STC.L SR, @-Rn (4n03)
    StcLSr {
        /// The rn operand.
        rn: Register,
    },
    /// LDC Rm, SSR (4n3E)
    LdcSsr {
        /// The rm operand.
        rm: Register,
    },
    /// LDC Rm, SPC (4n4E)
    LdcSpc {
        /// The rm operand.
        rm: Register,
    },
    /// LDC Rm, DBR (4nFA)
    LdcDbr {
        /// The rm operand.
        rm: Register,
    },
    /// LDC Rm, Rn_BANK (4n8E through 4nFE)
    LdcBank {
        /// The rm operand.
        rm: Register,
        /// The bank operand.
        bank: BankedRegister,
    },
    /// LDC.L @Rm+, SSR (4n37)
    LdcLSsr {
        /// The rm operand.
        rm: Register,
    },
    /// LDC.L @Rm+, SPC (4n47)
    LdcLSpc {
        /// The rm operand.
        rm: Register,
    },
    /// LDC.L @Rm+, DBR (4nF6)
    LdcLDbr {
        /// The rm operand.
        rm: Register,
    },
    /// LDC.L @Rm+, Rn_BANK (4n87 through 4nF7)
    LdcLBank {
        /// The rm operand.
        rm: Register,
        /// The bank operand.
        bank: BankedRegister,
    },
    /// STC SSR, Rn (0n32)
    StcSsr {
        /// The rn operand.
        rn: Register,
    },
    /// STC SPC, Rn (0n42)
    StcSpc {
        /// The rn operand.
        rn: Register,
    },
    /// STC SGR, Rn (0n3A)
    StcSgr {
        /// The rn operand.
        rn: Register,
    },
    /// STC DBR, Rn (0nFA)
    StcDbr {
        /// The rn operand.
        rn: Register,
    },
    /// STC Rm_BANK, Rn (0n82 through 0nF2)
    StcBank {
        /// The bank operand.
        bank: BankedRegister,
        /// The rn operand.
        rn: Register,
    },
    /// STC.L SSR, @-Rn (4n33)
    StcLSsr {
        /// The rn operand.
        rn: Register,
    },
    /// STC.L SPC, @-Rn (4n43)
    StcLSpc {
        /// The rn operand.
        rn: Register,
    },
    /// STC.L SGR, @-Rn (4n32)
    StcLSgr {
        /// The rn operand.
        rn: Register,
    },
    /// STC.L DBR, @-Rn (4nF2)
    StcLDbr {
        /// The rn operand.
        rn: Register,
    },
    /// STC.L Rm_BANK, @-Rn (4n83 through 4nF3)
    StcLBank {
        /// The bank operand.
        bank: BankedRegister,
        /// The rn operand.
        rn: Register,
    },
    /// LDS Rm, MACH (4n0a)
    LdsMach {
        /// The rm operand.
        rm: Register,
    },
    /// LDS Rm, MACL (4n1a)
    LdsMacl {
        /// The rm operand.
        rm: Register,
    },
    /// LDS Rm, PR (4n2a)
    LdsPr {
        /// The rm operand.
        rm: Register,
    },
    /// LDS.L @Rm+, MACH (4n06)
    LdsLMach {
        /// The rm operand.
        rm: Register,
    },
    /// LDS.L @Rm+, MACL (4n16)
    LdsLMacl {
        /// The rm operand.
        rm: Register,
    },
    /// LDS.L @Rm+, PR (4n26)
    LdsLPr {
        /// The rm operand.
        rm: Register,
    },
    /// STS MACH, Rn (0n0a)
    StsMach {
        /// The rn operand.
        rn: Register,
    },
    /// STS MACL, Rn (0n1a)
    StsMacl {
        /// The rn operand.
        rn: Register,
    },
    /// STS PR, Rn (0n2a)
    StsPr {
        /// The rn operand.
        rn: Register,
    },
    /// STS.L MACH, @-Rn (4n02)
    StsLMach {
        /// The rn operand.
        rn: Register,
    },
    /// STS.L MACL, @-Rn (4n12)
    StsLMacl {
        /// The rn operand.
        rn: Register,
    },
    /// STS.L PR, @-Rn (4n22)
    StsLPr {
        /// The rn operand.
        rn: Register,
    },
    /// LDS Rm, FPUL (4n5A)
    LdsFpul {
        /// The rm operand.
        rm: Register,
    },
    /// LDS Rm, FPSCR (4n6A)
    LdsFpscr {
        /// The rm operand.
        rm: Register,
    },
    /// LDS.L @Rm+, FPUL (4n56)
    LdsLFpul {
        /// The rm operand.
        rm: Register,
    },
    /// LDS.L @Rm+, FPSCR (4n66)
    LdsLFpscr {
        /// The rm operand.
        rm: Register,
    },
    /// STS FPUL, Rn (0n5A)
    StsFpul {
        /// The rn operand.
        rn: Register,
    },
    /// STS FPSCR, Rn (0n6A)
    StsFpscr {
        /// The rn operand.
        rn: Register,
    },
    /// STS.L FPUL, @-Rn (4n52)
    StsLFpul {
        /// The rn operand.
        rn: Register,
    },
    /// STS.L FPSCR, @-Rn (4n62)
    StsLFpscr {
        /// The rn operand.
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
        /// The imm operand.
        imm: u8,
    },
    /// BSRF Rm (0n03) -- Branch to Subroutine Far: PC + 4 + Rm → PC, PC + 4 → PR
    Bsrf {
        /// The rm operand.
        rm: Register,
    },
    /// BRAF Rm (0n23) -- Branch Far: PC + 4 + Rm → PC
    Braf {
        /// The rm operand.
        rm: Register,
    },
    /// TAS.B @Rn (4n1B) -- Test And Set byte at @Rn
    Tas {
        /// The rn operand.
        rn: Register,
    },

    // === Cache operations ===
    /// The pref form.
    Pref {
        /// The rn operand.
        rn: Register,
    },
    /// The ocbi form.
    Ocbi {
        /// The rn operand.
        rn: Register,
    },
    /// The ocbp form.
    Ocbp {
        /// The rn operand.
        rn: Register,
    },
    /// The ocbwb form.
    Ocbwb {
        /// The rn operand.
        rn: Register,
    },
    /// The movca l form.
    MovcaL {
        /// The rn operand.
        rn: Register,
    },

    // === Single-precision floating-point operations (FPSCR.PR = 0) ===
    /// The fldi0 form.
    Fldi0 {
        /// The frn operand.
        frn: FloatRegister,
    },
    /// The fldi1 form.
    Fldi1 {
        /// The frn operand.
        frn: FloatRegister,
    },
    /// The fmov form.
    Fmov {
        /// The frm operand.
        frm: FloatRegister,
        /// The frn operand.
        frn: FloatRegister,
    },
    /// The fmov load form.
    FmovLoad {
        /// The rm operand.
        rm: Register,
        /// The frn operand.
        frn: FloatRegister,
    },
    /// The fmov load indexed form.
    FmovLoadIndexed {
        /// The rm operand.
        rm: Register,
        /// The frn operand.
        frn: FloatRegister,
    },
    /// The fmov load post inc form.
    FmovLoadPostInc {
        /// The rm operand.
        rm: Register,
        /// The frn operand.
        frn: FloatRegister,
    },
    /// The fmov store form.
    FmovStore {
        /// The frm operand.
        frm: FloatRegister,
        /// The rn operand.
        rn: Register,
    },
    /// The fmov store indexed form.
    FmovStoreIndexed {
        /// The frm operand.
        frm: FloatRegister,
        /// The rn operand.
        rn: Register,
    },
    /// The fmov store pre dec form.
    FmovStorePreDec {
        /// The frm operand.
        frm: FloatRegister,
        /// The rn operand.
        rn: Register,
    },
    /// The flds form.
    Flds {
        /// The frm operand.
        frm: FloatRegister,
    },
    /// The fsts form.
    Fsts {
        /// The frn operand.
        frn: FloatRegister,
    },
    /// The fabs form.
    Fabs {
        /// The frn operand.
        frn: FloatRegister,
    },
    /// The fadd form.
    Fadd {
        /// The frm operand.
        frm: FloatRegister,
        /// The frn operand.
        frn: FloatRegister,
    },
    /// The fcmp eq form.
    FcmpEq {
        /// The frm operand.
        frm: FloatRegister,
        /// The frn operand.
        frn: FloatRegister,
    },
    /// The fcmp gt form.
    FcmpGt {
        /// The frm operand.
        frm: FloatRegister,
        /// The frn operand.
        frn: FloatRegister,
    },
    /// The fdiv form.
    Fdiv {
        /// The frm operand.
        frm: FloatRegister,
        /// The frn operand.
        frn: FloatRegister,
    },
    /// The float form.
    Float {
        /// The frn operand.
        frn: FloatRegister,
    },
    /// The fmac form.
    Fmac {
        /// The frm operand.
        frm: FloatRegister,
        /// The frn operand.
        frn: FloatRegister,
    },
    /// The fmul form.
    Fmul {
        /// The frm operand.
        frm: FloatRegister,
        /// The frn operand.
        frn: FloatRegister,
    },
    /// The fneg form.
    Fneg {
        /// The frn operand.
        frn: FloatRegister,
    },
    /// The fsqrt form.
    Fsqrt {
        /// The frn operand.
        frn: FloatRegister,
    },
    /// The fsub form.
    Fsub {
        /// The frm operand.
        frm: FloatRegister,
        /// The frn operand.
        frn: FloatRegister,
    },
    /// The ftrc form.
    Ftrc {
        /// The frm operand.
        frm: FloatRegister,
    },

    // === Pair transfers (FPSCR.SZ = 1) ===
    /// The fmov pair form.
    FmovPair {
        /// The underlying codec failure.
        source: FpuPairRegister,
        /// The destination operand.
        destination: FpuPairRegister,
    },
    /// The fmov pair load form.
    FmovPairLoad {
        /// The rm operand.
        rm: Register,
        /// The destination operand.
        destination: FpuPairRegister,
    },
    /// The fmov pair load indexed form.
    FmovPairLoadIndexed {
        /// The rm operand.
        rm: Register,
        /// The destination operand.
        destination: FpuPairRegister,
    },
    /// The fmov pair load post inc form.
    FmovPairLoadPostInc {
        /// The rm operand.
        rm: Register,
        /// The destination operand.
        destination: FpuPairRegister,
    },
    /// The fmov pair store form.
    FmovPairStore {
        /// The underlying codec failure.
        source: FpuPairRegister,
        /// The rn operand.
        rn: Register,
    },
    /// The fmov pair store indexed form.
    FmovPairStoreIndexed {
        /// The underlying codec failure.
        source: FpuPairRegister,
        /// The rn operand.
        rn: Register,
    },
    /// The fmov pair store pre dec form.
    FmovPairStorePreDec {
        /// The underlying codec failure.
        source: FpuPairRegister,
        /// The rn operand.
        rn: Register,
    },

    // === Double-precision operations (FPSCR.PR = 1) ===
    /// The fabs double form.
    FabsDouble {
        /// The drn operand.
        drn: DoubleRegister,
    },
    /// The fadd double form.
    FaddDouble {
        /// The drm operand.
        drm: DoubleRegister,
        /// The drn operand.
        drn: DoubleRegister,
    },
    /// The fcmp eq double form.
    FcmpEqDouble {
        /// The drm operand.
        drm: DoubleRegister,
        /// The drn operand.
        drn: DoubleRegister,
    },
    /// The fcmp gt double form.
    FcmpGtDouble {
        /// The drm operand.
        drm: DoubleRegister,
        /// The drn operand.
        drn: DoubleRegister,
    },
    /// The fdiv double form.
    FdivDouble {
        /// The drm operand.
        drm: DoubleRegister,
        /// The drn operand.
        drn: DoubleRegister,
    },
    /// The fcnvds form.
    Fcnvds {
        /// The drm operand.
        drm: DoubleRegister,
    },
    /// The fcnvsd form.
    Fcnvsd {
        /// The drn operand.
        drn: DoubleRegister,
    },
    /// The float double form.
    FloatDouble {
        /// The drn operand.
        drn: DoubleRegister,
    },
    /// The fmul double form.
    FmulDouble {
        /// The drm operand.
        drm: DoubleRegister,
        /// The drn operand.
        drn: DoubleRegister,
    },
    /// The fneg double form.
    FnegDouble {
        /// The drn operand.
        drn: DoubleRegister,
    },
    /// The fsqrt double form.
    FsqrtDouble {
        /// The drn operand.
        drn: DoubleRegister,
    },
    /// The fsub double form.
    FsubDouble {
        /// The drm operand.
        drm: DoubleRegister,
        /// The drn operand.
        drn: DoubleRegister,
    },
    /// The ftrc double form.
    FtrcDouble {
        /// The drm operand.
        drm: DoubleRegister,
    },

    // === Graphics acceleration and FPSCR bank/size controls ===
    /// The fipr form.
    Fipr {
        /// The fvm operand.
        fvm: VectorRegister,
        /// The fvn operand.
        fvn: VectorRegister,
    },
    /// The ftrv form.
    Ftrv {
        /// The fvn operand.
        fvn: VectorRegister,
    },
    /// The frchg form.
    Frchg,
    /// The fschg form.
    Fschg,
}

/// Operand-free semantic forms in the complete SH-4 profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InstructionForm {
    /// The mov imm form.
    MovImm,
    /// The mov wpc rel form.
    MovWPcRel,
    /// The mov lpc rel form.
    MovLPcRel,
    /// The mov reg form.
    MovReg,
    /// The mov bstore form.
    MovBStore,
    /// The mov wstore form.
    MovWStore,
    /// The mov lstore form.
    MovLStore,
    /// The mov bload form.
    MovBLoad,
    /// The mov wload form.
    MovWLoad,
    /// The mov lload form.
    MovLLoad,
    /// The mov bstore pre dec form.
    MovBStorePreDec,
    /// The mov wstore pre dec form.
    MovWStorePreDec,
    /// The mov lstore pre dec form.
    MovLStorePreDec,
    /// The mov bload post inc form.
    MovBLoadPostInc,
    /// The mov wload post inc form.
    MovWLoadPostInc,
    /// The mov lload post inc form.
    MovLLoadPostInc,
    /// The mov br0 load indexed form.
    MovBR0LoadIndexed,
    /// The mov wr0 load indexed form.
    MovWR0LoadIndexed,
    /// The mov lr0 load indexed form.
    MovLR0LoadIndexed,
    /// The mov br0 store indexed form.
    MovBR0StoreIndexed,
    /// The mov wr0 store indexed form.
    MovWR0StoreIndexed,
    /// The mov lr0 store indexed form.
    MovLR0StoreIndexed,
    /// The mov bdisp store form.
    MovBDispStore,
    /// The mov wdisp store form.
    MovWDispStore,
    /// The mov ldisp store form.
    MovLDispStore,
    /// The mov bdisp load form.
    MovBDispLoad,
    /// The mov wdisp load form.
    MovWDispLoad,
    /// The mov ldisp load form.
    MovLDispLoad,
    /// The mov bgbr store form.
    MovBGbrStore,
    /// The mov bgbr load form.
    MovBGbrLoad,
    /// The mov wgbr store form.
    MovWGbrStore,
    /// The mov wgbr load form.
    MovWGbrLoad,
    /// The mov lgbr store form.
    MovLGbrStore,
    /// The mov lgbr load form.
    MovLGbrLoad,
    /// The mova form.
    Mova,
    /// The mov t form.
    MovT,
    /// The add form.
    Add,
    /// The add imm form.
    AddImm,
    /// The addc form.
    Addc,
    /// The addv form.
    Addv,
    /// The sub form.
    Sub,
    /// The subc form.
    Subc,
    /// The subv form.
    Subv,
    /// The neg form.
    Neg,
    /// The negc form.
    Negc,
    /// The dt form.
    Dt,
    /// The mul l form.
    MulL,
    /// The mul s form.
    MulS,
    /// The mul u form.
    MulU,
    /// The dmul s form.
    DmulS,
    /// The dmul u form.
    DmulU,
    /// The div0 s form.
    Div0S,
    /// The div0 u form.
    Div0U,
    /// The div1 form.
    Div1,
    /// The mac l form.
    MacL,
    /// The mac w form.
    MacW,
    /// The and form.
    And,
    /// The and imm form.
    AndImm,
    /// The and b form.
    AndB,
    /// The or form.
    Or,
    /// The or imm form.
    OrImm,
    /// The or b form.
    OrB,
    /// The xor form.
    Xor,
    /// The xor imm form.
    XorImm,
    /// The xor b form.
    XorB,
    /// The not form.
    Not,
    /// The tst form.
    Tst,
    /// The tst imm form.
    TstImm,
    /// The tst b form.
    TstB,
    /// The shll form.
    Shll,
    /// The shlr form.
    Shlr,
    /// The shal form.
    Shal,
    /// The shar form.
    Shar,
    /// The shll2 form.
    Shll2,
    /// The shlr2 form.
    Shlr2,
    /// The shll8 form.
    Shll8,
    /// The shlr8 form.
    Shlr8,
    /// The shll16 form.
    Shll16,
    /// The shlr16 form.
    Shlr16,
    /// The rotl form.
    Rotl,
    /// The rotr form.
    Rotr,
    /// The rotcl form.
    Rotcl,
    /// The rotcr form.
    Rotcr,
    /// The exts b form.
    ExtsB,
    /// The exts w form.
    ExtsW,
    /// The extu b form.
    ExtuB,
    /// The extu w form.
    ExtuW,
    /// The swap b form.
    SwapB,
    /// The swap w form.
    SwapW,
    /// The xtrct form.
    Xtrct,
    /// The cmp eq form.
    CmpEq,
    /// The cmp eq imm form.
    CmpEqImm,
    /// The cmp ge form.
    CmpGe,
    /// The cmp gt form.
    CmpGt,
    /// The cmp hi form.
    CmpHi,
    /// The cmp hs form.
    CmpHs,
    /// The cmp pl form.
    CmpPl,
    /// The cmp pz form.
    CmpPz,
    /// The cmp str form.
    CmpStr,
    /// The bt form.
    Bt,
    /// The bf form.
    Bf,
    /// The bts form.
    Bts,
    /// The bfs form.
    Bfs,
    /// The bra form.
    Bra,
    /// The bsr form.
    Bsr,
    /// The jmp form.
    Jmp,
    /// The jsr form.
    Jsr,
    /// The rts form.
    Rts,
    /// The rte form.
    Rte,
    /// The ldc gbr form.
    LdcGbr,
    /// The ldc vbr form.
    LdcVbr,
    /// The ldc sr form.
    LdcSr,
    /// The ldc lgbr form.
    LdcLGbr,
    /// The ldc lvbr form.
    LdcLVbr,
    /// The ldc lsr form.
    LdcLSr,
    /// The stc gbr form.
    StcGbr,
    /// The stc vbr form.
    StcVbr,
    /// The stc sr form.
    StcSr,
    /// The stc lgbr form.
    StcLGbr,
    /// The stc lvbr form.
    StcLVbr,
    /// The stc lsr form.
    StcLSr,
    /// The lds mach form.
    LdsMach,
    /// The lds macl form.
    LdsMacl,
    /// The lds pr form.
    LdsPr,
    /// The lds lmach form.
    LdsLMach,
    /// The lds lmacl form.
    LdsLMacl,
    /// The lds lpr form.
    LdsLPr,
    /// The sts mach form.
    StsMach,
    /// The sts macl form.
    StsMacl,
    /// The sts pr form.
    StsPr,
    /// The sts lmach form.
    StsLMach,
    /// The sts lmacl form.
    StsLMacl,
    /// The sts lpr form.
    StsLPr,
    /// The nop form.
    Nop,
    /// The sleep form.
    Sleep,
    /// The set t form.
    SetT,
    /// The clr t form.
    ClrT,
    /// The clr mac form.
    ClrMac,
    /// The trapa form.
    Trapa,
    /// The bsrf form.
    Bsrf,
    /// The braf form.
    Braf,
    /// The tas form.
    Tas,
    /// The shad form.
    Shad,
    /// The shld form.
    Shld,
    /// The ldc ssr form.
    LdcSsr,
    /// The ldc spc form.
    LdcSpc,
    /// The ldc dbr form.
    LdcDbr,
    /// The ldc bank form.
    LdcBank,
    /// The ldc lssr form.
    LdcLSsr,
    /// The ldc lspc form.
    LdcLSpc,
    /// The ldc ldbr form.
    LdcLDbr,
    /// The ldc lbank form.
    LdcLBank,
    /// The stc ssr form.
    StcSsr,
    /// The stc spc form.
    StcSpc,
    /// The stc sgr form.
    StcSgr,
    /// The stc dbr form.
    StcDbr,
    /// The stc bank form.
    StcBank,
    /// The stc lssr form.
    StcLSsr,
    /// The stc lspc form.
    StcLSpc,
    /// The stc lsgr form.
    StcLSgr,
    /// The stc ldbr form.
    StcLDbr,
    /// The stc lbank form.
    StcLBank,
    /// The lds fpul form.
    LdsFpul,
    /// The lds fpscr form.
    LdsFpscr,
    /// The lds lfpul form.
    LdsLFpul,
    /// The lds lfpscr form.
    LdsLFpscr,
    /// The sts fpul form.
    StsFpul,
    /// The sts fpscr form.
    StsFpscr,
    /// The sts lfpul form.
    StsLFpul,
    /// The sts lfpscr form.
    StsLFpscr,
    /// The clr s form.
    ClrS,
    /// The set s form.
    SetS,
    /// The ldtlb form.
    Ldtlb,
    /// The pref form.
    Pref,
    /// The ocbi form.
    Ocbi,
    /// The ocbp form.
    Ocbp,
    /// The ocbwb form.
    Ocbwb,
    /// The movca l form.
    MovcaL,
    /// The fldi0 form.
    Fldi0,
    /// The fldi1 form.
    Fldi1,
    /// The fmov form.
    Fmov,
    /// The fmov load form.
    FmovLoad,
    /// The fmov load indexed form.
    FmovLoadIndexed,
    /// The fmov load post inc form.
    FmovLoadPostInc,
    /// The fmov store form.
    FmovStore,
    /// The fmov store indexed form.
    FmovStoreIndexed,
    /// The fmov store pre dec form.
    FmovStorePreDec,
    /// The flds form.
    Flds,
    /// The fsts form.
    Fsts,
    /// The fabs form.
    Fabs,
    /// The fadd form.
    Fadd,
    /// The fcmp eq form.
    FcmpEq,
    /// The fcmp gt form.
    FcmpGt,
    /// The fdiv form.
    Fdiv,
    /// The float form.
    Float,
    /// The fmac form.
    Fmac,
    /// The fmul form.
    Fmul,
    /// The fneg form.
    Fneg,
    /// The fsqrt form.
    Fsqrt,
    /// The fsub form.
    Fsub,
    /// The ftrc form.
    Ftrc,
    /// The fmov pair form.
    FmovPair,
    /// The fmov pair load form.
    FmovPairLoad,
    /// The fmov pair load indexed form.
    FmovPairLoadIndexed,
    /// The fmov pair load post inc form.
    FmovPairLoadPostInc,
    /// The fmov pair store form.
    FmovPairStore,
    /// The fmov pair store indexed form.
    FmovPairStoreIndexed,
    /// The fmov pair store pre dec form.
    FmovPairStorePreDec,
    /// The fabs double form.
    FabsDouble,
    /// The fadd double form.
    FaddDouble,
    /// The fcmp eq double form.
    FcmpEqDouble,
    /// The fcmp gt double form.
    FcmpGtDouble,
    /// The fdiv double form.
    FdivDouble,
    /// The fcnvds form.
    Fcnvds,
    /// The fcnvsd form.
    Fcnvsd,
    /// The float double form.
    FloatDouble,
    /// The fmul double form.
    FmulDouble,
    /// The fneg double form.
    FnegDouble,
    /// The fsqrt double form.
    FsqrtDouble,
    /// The fsub double form.
    FsubDouble,
    /// The ftrc double form.
    FtrcDouble,
    /// The fipr form.
    Fipr,
    /// The ftrv form.
    Ftrv,
    /// The frchg form.
    Frchg,
    /// The fschg form.
    Fschg,
}

impl InstructionForm {
    /// The all register or complete form set.
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
    /// The fall through form.
    FallThrough {
        /// The next operand.
        next: u32,
    },
    /// The branch form.
    Branch {
        /// The requested branch target.
        target: u32,
        /// The sequential successor address.
        fallthrough: Option<u32>,
        /// The delay slot operand.
        delay_slot: Option<u32>,
    },
    /// The call form.
    Call {
        /// The requested branch target.
        target: Option<u32>,
        /// The return address operand.
        return_address: u32,
        /// The delay slot operand.
        delay_slot: u32,
    },
    /// The jump form.
    Jump {
        /// The requested branch target.
        target: Option<u32>,
        /// The delay slot operand.
        delay_slot: u32,
    },
    /// The return form.
    Return {
        /// The delay slot operand.
        delay_slot: u32,
    },
    /// The exception return form.
    ExceptionReturn {
        /// The delay slot operand.
        delay_slot: u32,
    },
    /// The interrupt form.
    Interrupt,
}
