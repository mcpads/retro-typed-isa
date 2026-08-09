use std::fmt;

/// One of the 32 MIPS general-purpose registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Register(u8);

impl Register {
    /// Constant-zero register (`$zero`).
    pub const ZERO: Self = Self(0);
    /// Assembler-temporary register (`$at`).
    pub const AT: Self = Self(1);
    /// Function-result register 0 (`$v0`).
    pub const V0: Self = Self(2);
    /// Function-result register 1 (`$v1`).
    pub const V1: Self = Self(3);
    /// Argument register 0 (`$a0`).
    pub const A0: Self = Self(4);
    /// Argument register 1 (`$a1`).
    pub const A1: Self = Self(5);
    /// Argument register 2 (`$a2`).
    pub const A2: Self = Self(6);
    /// Argument register 3 (`$a3`).
    pub const A3: Self = Self(7);
    /// Caller-saved temporary register 0 (`$t0`).
    pub const T0: Self = Self(8);
    /// Caller-saved temporary register 1 (`$t1`).
    pub const T1: Self = Self(9);
    /// Caller-saved temporary register 2 (`$t2`).
    pub const T2: Self = Self(10);
    /// Caller-saved temporary register 3 (`$t3`).
    pub const T3: Self = Self(11);
    /// Caller-saved temporary register 4 (`$t4`).
    pub const T4: Self = Self(12);
    /// Caller-saved temporary register 5 (`$t5`).
    pub const T5: Self = Self(13);
    /// Caller-saved temporary register 6 (`$t6`).
    pub const T6: Self = Self(14);
    /// Caller-saved temporary register 7 (`$t7`).
    pub const T7: Self = Self(15);
    /// Callee-saved register 0 (`$s0`).
    pub const S0: Self = Self(16);
    /// Callee-saved register 1 (`$s1`).
    pub const S1: Self = Self(17);
    /// Callee-saved register 2 (`$s2`).
    pub const S2: Self = Self(18);
    /// Callee-saved register 3 (`$s3`).
    pub const S3: Self = Self(19);
    /// Callee-saved register 4 (`$s4`).
    pub const S4: Self = Self(20);
    /// Callee-saved register 5 (`$s5`).
    pub const S5: Self = Self(21);
    /// Callee-saved register 6 (`$s6`).
    pub const S6: Self = Self(22);
    /// Callee-saved register 7 (`$s7`).
    pub const S7: Self = Self(23);
    /// Caller-saved temporary register 8 (`$t8`).
    pub const T8: Self = Self(24);
    /// Caller-saved temporary register 9 (`$t9`).
    pub const T9: Self = Self(25);
    /// Kernel-reserved register 0 (`$k0`).
    pub const K0: Self = Self(26);
    /// Kernel-reserved register 1 (`$k1`).
    pub const K1: Self = Self(27);
    /// Global-pointer register (`$gp`).
    pub const GP: Self = Self(28);
    /// Stack-pointer register (`$sp`).
    pub const SP: Self = Self(29);
    /// Frame-pointer register (`$fp`).
    pub const FP: Self = Self(30);
    /// Return-address register (`$ra`).
    pub const RA: Self = Self(31);

    /// Constructs a register from a five-bit index.
    pub fn new(index: u8) -> Option<Self> {
        (index < 32).then_some(Self(index))
    }

    /// Returns the architectural register index.
    pub const fn index(self) -> u8 {
        self.0
    }

    pub(crate) const fn from_field(index: u8) -> Self {
        debug_assert!(index < 32);
        Self(index)
    }

    /// Returns the conventional MIPS ABI name without the `$` prefix.
    pub const fn name(self) -> &'static str {
        match self.0 {
            0 => "zero",
            1 => "at",
            2 => "v0",
            3 => "v1",
            4 => "a0",
            5 => "a1",
            6 => "a2",
            7 => "a3",
            8 => "t0",
            9 => "t1",
            10 => "t2",
            11 => "t3",
            12 => "t4",
            13 => "t5",
            14 => "t6",
            15 => "t7",
            16 => "s0",
            17 => "s1",
            18 => "s2",
            19 => "s3",
            20 => "s4",
            21 => "s5",
            22 => "s6",
            23 => "s7",
            24 => "t8",
            25 => "t9",
            26 => "k0",
            27 => "k1",
            28 => "gp",
            29 => "sp",
            30 => "fp",
            31 => "ra",
            _ => unreachable!(),
        }
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
        write!(formatter, "${}", self.name())
    }
}

/// One of the 22 documented PlayStation geometry transformation engine commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GteOpcode {
    /// Perspective-transform one point (`RTPS`).
    Rtps,
    /// Normal clipping (`NCLIP`).
    Nclip,
    /// Outer product (`OP`).
    Op,
    /// Depth cue one color (`DPCS`).
    Dpcs,
    /// Interpolate a vector and far color (`INTPL`).
    Intpl,
    /// Matrix-vector multiplication and vector addition (`MVMVA`).
    Mvmva,
    /// Normal color depth cue one vector (`NCDS`).
    Ncds,
    /// Color depth cue (`CDP`).
    Cdp,
    /// Normal color depth cue three vectors (`NCDT`).
    Ncdt,
    /// Normal color color one vector (`NCCS`).
    Nccs,
    /// Color color (`CC`).
    Cc,
    /// Normal color one vector (`NCS`).
    Ncs,
    /// Normal color three vectors (`NCT`).
    Nct,
    /// Square vector (`SQR`).
    Sqr,
    /// Depth cue color light (`DCPL`).
    Dcpl,
    /// Depth cue three colors (`DPCT`).
    Dpct,
    /// Average of three z values (`AVSZ3`).
    Avsz3,
    /// Average of four z values (`AVSZ4`).
    Avsz4,
    /// Perspective-transform three points (`RTPT`).
    Rtpt,
    /// General-purpose interpolation without accumulation (`GPF`).
    Gpf,
    /// General-purpose interpolation with accumulation (`GPL`).
    Gpl,
    /// Normal color color three vectors (`NCCT`).
    Ncct,
}

impl GteOpcode {
    /// Complete documented GTE command denominator.
    pub const ALL: [Self; 22] = [
        Self::Rtps,
        Self::Nclip,
        Self::Op,
        Self::Dpcs,
        Self::Intpl,
        Self::Mvmva,
        Self::Ncds,
        Self::Cdp,
        Self::Ncdt,
        Self::Nccs,
        Self::Cc,
        Self::Ncs,
        Self::Nct,
        Self::Sqr,
        Self::Dcpl,
        Self::Dpct,
        Self::Avsz3,
        Self::Avsz4,
        Self::Rtpt,
        Self::Gpf,
        Self::Gpl,
        Self::Ncct,
    ];

    /// Returns the six-bit function code used for hardware dispatch.
    pub const fn function(self) -> u8 {
        match self {
            Self::Rtps => 0x01,
            Self::Nclip => 0x06,
            Self::Op => 0x0c,
            Self::Dpcs => 0x10,
            Self::Intpl => 0x11,
            Self::Mvmva => 0x12,
            Self::Ncds => 0x13,
            Self::Cdp => 0x14,
            Self::Ncdt => 0x16,
            Self::Nccs => 0x1b,
            Self::Cc => 0x1c,
            Self::Ncs => 0x1e,
            Self::Nct => 0x20,
            Self::Sqr => 0x28,
            Self::Dcpl => 0x29,
            Self::Dpct => 0x2a,
            Self::Avsz3 => 0x2d,
            Self::Avsz4 => 0x2e,
            Self::Rtpt => 0x30,
            Self::Gpf => 0x3d,
            Self::Gpl => 0x3e,
            Self::Ncct => 0x3f,
        }
    }

    pub(crate) fn from_function(function: u8) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|opcode| opcode.function() == function)
    }
}

/// Semantic fields of a PlayStation GTE command.
///
/// `fake` is Sony's conventional five-bit command number. Hardware ignores
/// it for dispatch, but preserving it makes decode/encode byte-exact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GteCommand {
    /// Semantic GTE command selected by the six-bit function field.
    pub opcode: GteOpcode,
    /// Preserved five-bit conventional command number ignored by dispatch.
    pub fake: u8,
    /// Selects the 12-bit fractional shift when set.
    pub shift_fraction: bool,
    /// Two-bit matrix selector.
    pub matrix: u8,
    /// Two-bit input-vector selector.
    pub vector: u8,
    /// Two-bit translation or control-vector selector.
    pub control_vector: u8,
    /// Enables positive-only result limiting when set.
    pub limit_unsigned: bool,
}

macro_rules! define_instruction_forms {
    ($($form:ident),+ $(,)?) => {
        /// Semantic instruction forms included in the PlayStation R3000A profile.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum InstructionForm {
            $(
                #[doc = concat!("The `", stringify!($form), "` instruction form.")]
                $form
            ),+,
            /// A GTE command form distinguished by its documented function code.
            Gte(GteOpcode),
        }

        impl InstructionForm {
            /// Forms other than GTE commands in the profile.
            pub const NON_GTE: &'static [Self] = &[$(Self::$form),+];

            /// Complete semantic-form denominator for profile-level checks.
            pub fn all() -> impl Iterator<Item = Self> {
                Self::NON_GTE
                    .iter()
                    .copied()
                    .chain(GteOpcode::ALL.into_iter().map(Self::Gte))
            }
        }
    };
}

define_instruction_forms!(
    Add, Addu, Sub, Subu, And, Or, Xor, Nor, Slt, Sltu, Addi, Addiu, Andi, Ori, Xori, Lui, Slti,
    Sltiu, Sll, Srl, Sra, Sllv, Srlv, Srav, Mult, Multu, Div, Divu, Mfhi, Mthi, Mflo, Mtlo, Lb,
    Lbu, Lh, Lhu, Lw, Lwl, Lwr, Sb, Sh, Sw, Swl, Swr, Beq, Bne, Blez, Bgtz, Bltz, Bgez, Bltzal,
    Bgezal, J, Jal, Jr, Jalr, Syscall, Break, Mfc0, Mtc0, Rfe, Mfc2, Mtc2, Cfc2, Ctc2, Lwc2, Swc2,
);

/// A semantic PlayStation R3000A instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    /// The `add` instruction.
    Add {
        /// Encoded `rd` register or coprocessor-register field.
        rd: Register,
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
    },
    /// The `addu` instruction.
    Addu {
        /// Encoded `rd` register or coprocessor-register field.
        rd: Register,
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
    },
    /// The `sub` instruction.
    Sub {
        /// Encoded `rd` register or coprocessor-register field.
        rd: Register,
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
    },
    /// The `subu` instruction.
    Subu {
        /// Encoded `rd` register or coprocessor-register field.
        rd: Register,
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
    },
    /// The `and` instruction.
    And {
        /// Encoded `rd` register or coprocessor-register field.
        rd: Register,
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
    },
    /// The `or` instruction.
    Or {
        /// Encoded `rd` register or coprocessor-register field.
        rd: Register,
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
    },
    /// The `xor` instruction.
    Xor {
        /// Encoded `rd` register or coprocessor-register field.
        rd: Register,
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
    },
    /// The `nor` instruction.
    Nor {
        /// Encoded `rd` register or coprocessor-register field.
        rd: Register,
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
    },
    /// The `slt` instruction.
    Slt {
        /// Encoded `rd` register or coprocessor-register field.
        rd: Register,
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
    },
    /// The `sltu` instruction.
    Sltu {
        /// Encoded `rd` register or coprocessor-register field.
        rd: Register,
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
    },

    /// The `addi` instruction.
    Addi {
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Immediate value.
        immediate: i16,
    },
    /// The `addiu` instruction.
    Addiu {
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Immediate value.
        immediate: i16,
    },
    /// The `andi` instruction.
    Andi {
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Immediate value.
        immediate: u16,
    },
    /// The `ori` instruction.
    Ori {
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Immediate value.
        immediate: u16,
    },
    /// The `xori` instruction.
    Xori {
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Immediate value.
        immediate: u16,
    },
    /// The `lui` instruction.
    Lui {
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Immediate value.
        immediate: u16,
    },
    /// The `slti` instruction.
    Slti {
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Immediate value.
        immediate: i16,
    },
    /// The `sltiu` instruction.
    Sltiu {
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Immediate value.
        immediate: i16,
    },

    /// The `sll` instruction.
    Sll {
        /// Encoded `rd` register or coprocessor-register field.
        rd: Register,
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Immediate shift amount; encoding requires a value below 32.
        shift: u8,
    },
    /// The `srl` instruction.
    Srl {
        /// Encoded `rd` register or coprocessor-register field.
        rd: Register,
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Immediate shift amount; encoding requires a value below 32.
        shift: u8,
    },
    /// The `sra` instruction.
    Sra {
        /// Encoded `rd` register or coprocessor-register field.
        rd: Register,
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Immediate shift amount; encoding requires a value below 32.
        shift: u8,
    },
    /// The `sllv` instruction.
    Sllv {
        /// Encoded `rd` register or coprocessor-register field.
        rd: Register,
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Encoded `rs` general-purpose register field.
        rs: Register,
    },
    /// The `srlv` instruction.
    Srlv {
        /// Encoded `rd` register or coprocessor-register field.
        rd: Register,
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Encoded `rs` general-purpose register field.
        rs: Register,
    },
    /// The `srav` instruction.
    Srav {
        /// Encoded `rd` register or coprocessor-register field.
        rd: Register,
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Encoded `rs` general-purpose register field.
        rs: Register,
    },

    /// The `mult` instruction.
    Mult {
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
    },
    /// The `multu` instruction.
    Multu {
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
    },
    /// The `div` instruction.
    Div {
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
    },
    /// The `divu` instruction.
    Divu {
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
    },
    /// The `mfhi` instruction.
    Mfhi {
        /// Encoded `rd` register or coprocessor-register field.
        rd: Register,
    },
    /// The `mthi` instruction.
    Mthi {
        /// Encoded `rs` general-purpose register field.
        rs: Register,
    },
    /// The `mflo` instruction.
    Mflo {
        /// Encoded `rd` register or coprocessor-register field.
        rd: Register,
    },
    /// The `mtlo` instruction.
    Mtlo {
        /// Encoded `rs` general-purpose register field.
        rs: Register,
    },

    /// The `lb` instruction.
    Lb {
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Base general-purpose register for the memory address.
        base: Register,
        /// Signed byte displacement from the base register.
        offset: i16,
    },
    /// The `lbu` instruction.
    Lbu {
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Base general-purpose register for the memory address.
        base: Register,
        /// Signed byte displacement from the base register.
        offset: i16,
    },
    /// The `lh` instruction.
    Lh {
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Base general-purpose register for the memory address.
        base: Register,
        /// Signed byte displacement from the base register.
        offset: i16,
    },
    /// The `lhu` instruction.
    Lhu {
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Base general-purpose register for the memory address.
        base: Register,
        /// Signed byte displacement from the base register.
        offset: i16,
    },
    /// The `lw` instruction.
    Lw {
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Base general-purpose register for the memory address.
        base: Register,
        /// Signed byte displacement from the base register.
        offset: i16,
    },
    /// The `lwl` instruction.
    Lwl {
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Base general-purpose register for the memory address.
        base: Register,
        /// Signed byte displacement from the base register.
        offset: i16,
    },
    /// The `lwr` instruction.
    Lwr {
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Base general-purpose register for the memory address.
        base: Register,
        /// Signed byte displacement from the base register.
        offset: i16,
    },
    /// The `sb` instruction.
    Sb {
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Base general-purpose register for the memory address.
        base: Register,
        /// Signed byte displacement from the base register.
        offset: i16,
    },
    /// The `sh` instruction.
    Sh {
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Base general-purpose register for the memory address.
        base: Register,
        /// Signed byte displacement from the base register.
        offset: i16,
    },
    /// The `sw` instruction.
    Sw {
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Base general-purpose register for the memory address.
        base: Register,
        /// Signed byte displacement from the base register.
        offset: i16,
    },
    /// The `swl` instruction.
    Swl {
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Base general-purpose register for the memory address.
        base: Register,
        /// Signed byte displacement from the base register.
        offset: i16,
    },
    /// The `swr` instruction.
    Swr {
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Base general-purpose register for the memory address.
        base: Register,
        /// Signed byte displacement from the base register.
        offset: i16,
    },

    /// The `beq` instruction.
    Beq {
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Absolute control-flow target resolved against the placed instruction.
        target: u32,
    },
    /// The `bne` instruction.
    Bne {
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Absolute control-flow target resolved against the placed instruction.
        target: u32,
    },
    /// The `blez` instruction.
    Blez {
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Absolute control-flow target resolved against the placed instruction.
        target: u32,
    },
    /// The `bgtz` instruction.
    Bgtz {
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Absolute control-flow target resolved against the placed instruction.
        target: u32,
    },
    /// The `bltz` instruction.
    Bltz {
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Absolute control-flow target resolved against the placed instruction.
        target: u32,
    },
    /// The `bgez` instruction.
    Bgez {
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Absolute control-flow target resolved against the placed instruction.
        target: u32,
    },
    /// The `bltzal` instruction.
    Bltzal {
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Absolute control-flow target resolved against the placed instruction.
        target: u32,
    },
    /// The `bgezal` instruction.
    Bgezal {
        /// Encoded `rs` general-purpose register field.
        rs: Register,
        /// Absolute control-flow target resolved against the placed instruction.
        target: u32,
    },
    /// The `j` instruction.
    J {
        /// Absolute control-flow target resolved against the placed instruction.
        target: u32,
    },
    /// The `jal` instruction.
    Jal {
        /// Absolute control-flow target resolved against the placed instruction.
        target: u32,
    },
    /// The `jr` instruction.
    Jr {
        /// Encoded `rs` general-purpose register field.
        rs: Register,
    },
    /// The `jalr` instruction.
    Jalr {
        /// Encoded `rd` register or coprocessor-register field.
        rd: Register,
        /// Encoded `rs` general-purpose register field.
        rs: Register,
    },

    /// The `syscall` instruction.
    Syscall {
        /// Embedded system-call or break code.
        code: u32,
    },
    /// The `break` instruction.
    Break {
        /// Embedded system-call or break code.
        code: u32,
    },
    /// The `mfc0` instruction.
    Mfc0 {
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Encoded `rd` register or coprocessor-register field.
        rd: u8,
    },
    /// The `mtc0` instruction.
    Mtc0 {
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Encoded `rd` register or coprocessor-register field.
        rd: u8,
    },
    /// The `rfe` instruction.
    Rfe,
    /// The `mfc2` instruction.
    Mfc2 {
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Encoded `rd` register or coprocessor-register field.
        rd: u8,
    },
    /// The `mtc2` instruction.
    Mtc2 {
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Encoded `rd` register or coprocessor-register field.
        rd: u8,
    },
    /// The `cfc2` instruction.
    Cfc2 {
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Encoded `rd` register or coprocessor-register field.
        rd: u8,
    },
    /// The `ctc2` instruction.
    Ctc2 {
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: Register,
        /// Encoded `rd` register or coprocessor-register field.
        rd: u8,
    },
    /// The `lwc2` instruction.
    Lwc2 {
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: u8,
        /// Base general-purpose register for the memory address.
        base: Register,
        /// Signed byte displacement from the base register.
        offset: i16,
    },
    /// The `swc2` instruction.
    Swc2 {
        /// Encoded `rt` general-purpose or coprocessor-register field.
        rt: u8,
        /// Base general-purpose register for the memory address.
        base: Register,
        /// Signed byte displacement from the base register.
        offset: i16,
    },
    /// The `GTE command` instruction.
    Gte(GteCommand),
}

impl Instruction {
    /// Returns the semantic instruction form.
    pub const fn form(&self) -> InstructionForm {
        match self {
            Self::Add { .. } => InstructionForm::Add,
            Self::Addu { .. } => InstructionForm::Addu,
            Self::Sub { .. } => InstructionForm::Sub,
            Self::Subu { .. } => InstructionForm::Subu,
            Self::And { .. } => InstructionForm::And,
            Self::Or { .. } => InstructionForm::Or,
            Self::Xor { .. } => InstructionForm::Xor,
            Self::Nor { .. } => InstructionForm::Nor,
            Self::Slt { .. } => InstructionForm::Slt,
            Self::Sltu { .. } => InstructionForm::Sltu,
            Self::Addi { .. } => InstructionForm::Addi,
            Self::Addiu { .. } => InstructionForm::Addiu,
            Self::Andi { .. } => InstructionForm::Andi,
            Self::Ori { .. } => InstructionForm::Ori,
            Self::Xori { .. } => InstructionForm::Xori,
            Self::Lui { .. } => InstructionForm::Lui,
            Self::Slti { .. } => InstructionForm::Slti,
            Self::Sltiu { .. } => InstructionForm::Sltiu,
            Self::Sll { .. } => InstructionForm::Sll,
            Self::Srl { .. } => InstructionForm::Srl,
            Self::Sra { .. } => InstructionForm::Sra,
            Self::Sllv { .. } => InstructionForm::Sllv,
            Self::Srlv { .. } => InstructionForm::Srlv,
            Self::Srav { .. } => InstructionForm::Srav,
            Self::Mult { .. } => InstructionForm::Mult,
            Self::Multu { .. } => InstructionForm::Multu,
            Self::Div { .. } => InstructionForm::Div,
            Self::Divu { .. } => InstructionForm::Divu,
            Self::Mfhi { .. } => InstructionForm::Mfhi,
            Self::Mthi { .. } => InstructionForm::Mthi,
            Self::Mflo { .. } => InstructionForm::Mflo,
            Self::Mtlo { .. } => InstructionForm::Mtlo,
            Self::Lb { .. } => InstructionForm::Lb,
            Self::Lbu { .. } => InstructionForm::Lbu,
            Self::Lh { .. } => InstructionForm::Lh,
            Self::Lhu { .. } => InstructionForm::Lhu,
            Self::Lw { .. } => InstructionForm::Lw,
            Self::Lwl { .. } => InstructionForm::Lwl,
            Self::Lwr { .. } => InstructionForm::Lwr,
            Self::Sb { .. } => InstructionForm::Sb,
            Self::Sh { .. } => InstructionForm::Sh,
            Self::Sw { .. } => InstructionForm::Sw,
            Self::Swl { .. } => InstructionForm::Swl,
            Self::Swr { .. } => InstructionForm::Swr,
            Self::Beq { .. } => InstructionForm::Beq,
            Self::Bne { .. } => InstructionForm::Bne,
            Self::Blez { .. } => InstructionForm::Blez,
            Self::Bgtz { .. } => InstructionForm::Bgtz,
            Self::Bltz { .. } => InstructionForm::Bltz,
            Self::Bgez { .. } => InstructionForm::Bgez,
            Self::Bltzal { .. } => InstructionForm::Bltzal,
            Self::Bgezal { .. } => InstructionForm::Bgezal,
            Self::J { .. } => InstructionForm::J,
            Self::Jal { .. } => InstructionForm::Jal,
            Self::Jr { .. } => InstructionForm::Jr,
            Self::Jalr { .. } => InstructionForm::Jalr,
            Self::Syscall { .. } => InstructionForm::Syscall,
            Self::Break { .. } => InstructionForm::Break,
            Self::Mfc0 { .. } => InstructionForm::Mfc0,
            Self::Mtc0 { .. } => InstructionForm::Mtc0,
            Self::Rfe => InstructionForm::Rfe,
            Self::Mfc2 { .. } => InstructionForm::Mfc2,
            Self::Mtc2 { .. } => InstructionForm::Mtc2,
            Self::Cfc2 { .. } => InstructionForm::Cfc2,
            Self::Ctc2 { .. } => InstructionForm::Ctc2,
            Self::Lwc2 { .. } => InstructionForm::Lwc2,
            Self::Swc2 { .. } => InstructionForm::Swc2,
            Self::Gte(command) => InstructionForm::Gte(command.opcode),
        }
    }

    /// Constructs the canonical all-zero `sll $zero, $zero, 0` no-op.
    pub const fn nop() -> Self {
        Self::Sll {
            rd: Register::ZERO,
            rt: Register::ZERO,
            shift: 0,
        }
    }

    /// Returns whether shared static semantics require a following delay-slot instruction.
    pub fn has_delay_slot(&self) -> bool {
        crate::semantics::semantics(self, 0)
            .expect("zero is an aligned R3000A analysis point")
            .control_flow
            .delay_slot
            .is_some()
    }

    /// Returns the general-purpose register written by this instruction, when
    /// the instruction has at most one explicit GPR destination.
    ///
    /// Link instructions report `$ra` (or the explicit `JALR` destination).
    /// Conditional link branches report `$ra` even though the write depends on
    /// the branch condition; callers doing constant propagation must therefore
    /// conservatively invalidate it.
    pub fn written_gpr(&self) -> Option<Register> {
        let semantics =
            crate::semantics::semantics(self, 0).expect("zero is an aligned R3000A analysis point");
        let mut written = semantics.location_accesses.iter().filter_map(|access| {
            if access.kind == typed_isa_core::AccessKind::Write {
                if let crate::Location::Gpr(register) = access.location {
                    return Some(register);
                }
            }
            None
        });
        let first = written.next()?;
        written.all(|register| register == first).then_some(first)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_explicit_and_implicit_gpr_destinations() {
        assert_eq!(
            Instruction::Lui {
                rt: Register::A0,
                immediate: 0x8015,
            }
            .written_gpr(),
            Some(Register::A0)
        );
        assert_eq!(
            Instruction::Addiu {
                rt: Register::A0,
                rs: Register::A0,
                immediate: -2,
            }
            .written_gpr(),
            Some(Register::A0)
        );
        assert_eq!(
            Instruction::Jal {
                target: 0x8005_08e0,
            }
            .written_gpr(),
            Some(Register::RA)
        );
        assert_eq!(
            Instruction::Sw {
                rt: Register::A0,
                base: Register::SP,
                offset: 0,
            }
            .written_gpr(),
            None
        );
    }
}
