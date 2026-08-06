use std::fmt;

/// One of the 32 MIPS general-purpose registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Register(u8);

impl Register {
    pub const ZERO: Self = Self(0);
    pub const AT: Self = Self(1);
    pub const V0: Self = Self(2);
    pub const V1: Self = Self(3);
    pub const A0: Self = Self(4);
    pub const A1: Self = Self(5);
    pub const A2: Self = Self(6);
    pub const A3: Self = Self(7);
    pub const T0: Self = Self(8);
    pub const T1: Self = Self(9);
    pub const T2: Self = Self(10);
    pub const T3: Self = Self(11);
    pub const T4: Self = Self(12);
    pub const T5: Self = Self(13);
    pub const T6: Self = Self(14);
    pub const T7: Self = Self(15);
    pub const S0: Self = Self(16);
    pub const S1: Self = Self(17);
    pub const S2: Self = Self(18);
    pub const S3: Self = Self(19);
    pub const S4: Self = Self(20);
    pub const S5: Self = Self(21);
    pub const S6: Self = Self(22);
    pub const S7: Self = Self(23);
    pub const T8: Self = Self(24);
    pub const T9: Self = Self(25);
    pub const K0: Self = Self(26);
    pub const K1: Self = Self(27);
    pub const GP: Self = Self(28);
    pub const SP: Self = Self(29);
    pub const FP: Self = Self(30);
    pub const RA: Self = Self(31);

    pub fn new(index: u8) -> Option<Self> {
        (index < 32).then_some(Self(index))
    }

    pub const fn index(self) -> u8 {
        self.0
    }

    pub(crate) const fn from_field(index: u8) -> Self {
        debug_assert!(index < 32);
        Self(index)
    }

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GteOpcode {
    Rtps,
    Nclip,
    Op,
    Dpcs,
    Intpl,
    Mvmva,
    Ncds,
    Cdp,
    Ncdt,
    Nccs,
    Cc,
    Ncs,
    Nct,
    Sqr,
    Dcpl,
    Dpct,
    Avsz3,
    Avsz4,
    Rtpt,
    Gpf,
    Gpl,
    Ncct,
}

impl GteOpcode {
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
    pub opcode: GteOpcode,
    pub fake: u8,
    pub shift_fraction: bool,
    pub matrix: u8,
    pub vector: u8,
    pub control_vector: u8,
    pub limit_unsigned: bool,
}

macro_rules! define_instruction_forms {
    ($($form:ident),+ $(,)?) => {
        /// Semantic instruction forms included in the PlayStation R3000A profile.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum InstructionForm {
            $($form),+,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    Add {
        rd: Register,
        rs: Register,
        rt: Register,
    },
    Addu {
        rd: Register,
        rs: Register,
        rt: Register,
    },
    Sub {
        rd: Register,
        rs: Register,
        rt: Register,
    },
    Subu {
        rd: Register,
        rs: Register,
        rt: Register,
    },
    And {
        rd: Register,
        rs: Register,
        rt: Register,
    },
    Or {
        rd: Register,
        rs: Register,
        rt: Register,
    },
    Xor {
        rd: Register,
        rs: Register,
        rt: Register,
    },
    Nor {
        rd: Register,
        rs: Register,
        rt: Register,
    },
    Slt {
        rd: Register,
        rs: Register,
        rt: Register,
    },
    Sltu {
        rd: Register,
        rs: Register,
        rt: Register,
    },

    Addi {
        rt: Register,
        rs: Register,
        immediate: i16,
    },
    Addiu {
        rt: Register,
        rs: Register,
        immediate: i16,
    },
    Andi {
        rt: Register,
        rs: Register,
        immediate: u16,
    },
    Ori {
        rt: Register,
        rs: Register,
        immediate: u16,
    },
    Xori {
        rt: Register,
        rs: Register,
        immediate: u16,
    },
    Lui {
        rt: Register,
        immediate: u16,
    },
    Slti {
        rt: Register,
        rs: Register,
        immediate: i16,
    },
    Sltiu {
        rt: Register,
        rs: Register,
        immediate: i16,
    },

    Sll {
        rd: Register,
        rt: Register,
        shift: u8,
    },
    Srl {
        rd: Register,
        rt: Register,
        shift: u8,
    },
    Sra {
        rd: Register,
        rt: Register,
        shift: u8,
    },
    Sllv {
        rd: Register,
        rt: Register,
        rs: Register,
    },
    Srlv {
        rd: Register,
        rt: Register,
        rs: Register,
    },
    Srav {
        rd: Register,
        rt: Register,
        rs: Register,
    },

    Mult {
        rs: Register,
        rt: Register,
    },
    Multu {
        rs: Register,
        rt: Register,
    },
    Div {
        rs: Register,
        rt: Register,
    },
    Divu {
        rs: Register,
        rt: Register,
    },
    Mfhi {
        rd: Register,
    },
    Mthi {
        rs: Register,
    },
    Mflo {
        rd: Register,
    },
    Mtlo {
        rs: Register,
    },

    Lb {
        rt: Register,
        base: Register,
        offset: i16,
    },
    Lbu {
        rt: Register,
        base: Register,
        offset: i16,
    },
    Lh {
        rt: Register,
        base: Register,
        offset: i16,
    },
    Lhu {
        rt: Register,
        base: Register,
        offset: i16,
    },
    Lw {
        rt: Register,
        base: Register,
        offset: i16,
    },
    Lwl {
        rt: Register,
        base: Register,
        offset: i16,
    },
    Lwr {
        rt: Register,
        base: Register,
        offset: i16,
    },
    Sb {
        rt: Register,
        base: Register,
        offset: i16,
    },
    Sh {
        rt: Register,
        base: Register,
        offset: i16,
    },
    Sw {
        rt: Register,
        base: Register,
        offset: i16,
    },
    Swl {
        rt: Register,
        base: Register,
        offset: i16,
    },
    Swr {
        rt: Register,
        base: Register,
        offset: i16,
    },

    Beq {
        rs: Register,
        rt: Register,
        target: u32,
    },
    Bne {
        rs: Register,
        rt: Register,
        target: u32,
    },
    Blez {
        rs: Register,
        target: u32,
    },
    Bgtz {
        rs: Register,
        target: u32,
    },
    Bltz {
        rs: Register,
        target: u32,
    },
    Bgez {
        rs: Register,
        target: u32,
    },
    Bltzal {
        rs: Register,
        target: u32,
    },
    Bgezal {
        rs: Register,
        target: u32,
    },
    J {
        target: u32,
    },
    Jal {
        target: u32,
    },
    Jr {
        rs: Register,
    },
    Jalr {
        rd: Register,
        rs: Register,
    },

    Syscall {
        code: u32,
    },
    Break {
        code: u32,
    },
    Mfc0 {
        rt: Register,
        rd: u8,
    },
    Mtc0 {
        rt: Register,
        rd: u8,
    },
    Rfe,
    Mfc2 {
        rt: Register,
        rd: u8,
    },
    Mtc2 {
        rt: Register,
        rd: u8,
    },
    Cfc2 {
        rt: Register,
        rd: u8,
    },
    Ctc2 {
        rt: Register,
        rd: u8,
    },
    Lwc2 {
        rt: u8,
        base: Register,
        offset: i16,
    },
    Swc2 {
        rt: u8,
        base: Register,
        offset: i16,
    },
    Gte(GteCommand),
}

impl Instruction {
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

    pub const fn nop() -> Self {
        Self::Sll {
            rd: Register::ZERO,
            rt: Register::ZERO,
            shift: 0,
        }
    }

    pub fn control_flow(&self) -> ControlFlow {
        match self {
            Self::J { target } => ControlFlow::Jump { target: *target },
            Self::Jal { target } => ControlFlow::Call { target: *target },
            Self::Jr { rs } if *rs == Register::RA => ControlFlow::Return,
            Self::Jr { .. } => ControlFlow::IndirectJump,
            Self::Jalr { .. } => ControlFlow::IndirectCall,
            Self::Beq { rs, rt, target } if *rs == Register::ZERO && *rt == Register::ZERO => {
                ControlFlow::Branch { target: *target }
            }
            Self::Beq { target, .. }
            | Self::Bne { target, .. }
            | Self::Blez { target, .. }
            | Self::Bgtz { target, .. }
            | Self::Bltz { target, .. }
            | Self::Bgez { target, .. } => ControlFlow::ConditionalBranch { target: *target },
            Self::Bltzal { target, .. } | Self::Bgezal { target, .. } => {
                ControlFlow::Call { target: *target }
            }
            _ => ControlFlow::FallThrough,
        }
    }

    pub fn has_delay_slot(&self) -> bool {
        !matches!(self.control_flow(), ControlFlow::FallThrough)
    }

    /// Return the general-purpose register written by this instruction, when
    /// the instruction has at most one explicit GPR destination.
    ///
    /// Link instructions report `$ra` (or the explicit `JALR` destination).
    /// Conditional link branches report `$ra` even though the write depends on
    /// the branch condition; callers doing constant propagation must therefore
    /// conservatively invalidate it.
    pub fn written_gpr(&self) -> Option<Register> {
        match self {
            Self::Add { rd, .. }
            | Self::Addu { rd, .. }
            | Self::Sub { rd, .. }
            | Self::Subu { rd, .. }
            | Self::And { rd, .. }
            | Self::Or { rd, .. }
            | Self::Xor { rd, .. }
            | Self::Nor { rd, .. }
            | Self::Slt { rd, .. }
            | Self::Sltu { rd, .. }
            | Self::Sll { rd, .. }
            | Self::Srl { rd, .. }
            | Self::Sra { rd, .. }
            | Self::Sllv { rd, .. }
            | Self::Srlv { rd, .. }
            | Self::Srav { rd, .. }
            | Self::Mfhi { rd }
            | Self::Mflo { rd }
            | Self::Jalr { rd, .. } => Some(*rd),
            Self::Addi { rt, .. }
            | Self::Addiu { rt, .. }
            | Self::Andi { rt, .. }
            | Self::Ori { rt, .. }
            | Self::Xori { rt, .. }
            | Self::Lui { rt, .. }
            | Self::Slti { rt, .. }
            | Self::Sltiu { rt, .. }
            | Self::Lb { rt, .. }
            | Self::Lbu { rt, .. }
            | Self::Lh { rt, .. }
            | Self::Lhu { rt, .. }
            | Self::Lw { rt, .. }
            | Self::Lwl { rt, .. }
            | Self::Lwr { rt, .. }
            | Self::Mfc0 { rt, .. }
            | Self::Mfc2 { rt, .. }
            | Self::Cfc2 { rt, .. } => Some(*rt),
            Self::Jal { .. } | Self::Bltzal { .. } | Self::Bgezal { .. } => Some(Register::RA),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlFlow {
    FallThrough,
    Branch { target: u32 },
    ConditionalBranch { target: u32 },
    Jump { target: u32 },
    Call { target: u32 },
    Return,
    IndirectJump,
    IndirectCall,
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
