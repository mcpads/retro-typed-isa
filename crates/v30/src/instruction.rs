//! Core types for the 8086/V30 ISA profile.

/// An 8086 8-bit GPR indexed by the low three bits of a ModR/M r/m or reg field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reg8(pub(crate) u8);

impl Reg8 {
    /// The al register or complete selector set.
    pub const AL: Reg8 = Reg8(0);
    /// The cl register or complete selector set.
    pub const CL: Reg8 = Reg8(1);
    /// The dl register or complete selector set.
    pub const DL: Reg8 = Reg8(2);
    /// The bl register or complete selector set.
    pub const BL: Reg8 = Reg8(3);
    /// The ah register or complete selector set.
    pub const AH: Reg8 = Reg8(4);
    /// The ch register or complete selector set.
    pub const CH: Reg8 = Reg8(5);
    /// The dh register or complete selector set.
    pub const DH: Reg8 = Reg8(6);
    /// The bh register or complete selector set.
    pub const BH: Reg8 = Reg8(7);

    /// Returns the conventional assembly register or condition name.
    pub fn name(self) -> &'static str {
        match self.0 {
            0 => "al",
            1 => "cl",
            2 => "dl",
            3 => "bl",
            4 => "ah",
            5 => "ch",
            6 => "dh",
            7 => "bh",
            // An index above seven indicates a decoder bug; keep formatting total.
            _ => "??",
        }
    }

    /// Returns the encoded register index.
    pub fn index(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for Reg8 {
    type Error = RegisterIndexError;

    fn try_from(index: u8) -> Result<Self, Self::Error> {
        (index < 8)
            .then_some(Self(index))
            .ok_or(RegisterIndexError { width: 8, index })
    }
}

/// An 8086 16-bit GPR indexed by the low three bits of a ModR/M r/m or reg field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reg16(pub(crate) u8);

impl Reg16 {
    /// The ax register or complete selector set.
    pub const AX: Reg16 = Reg16(0);
    /// The cx register or complete selector set.
    pub const CX: Reg16 = Reg16(1);
    /// The dx register or complete selector set.
    pub const DX: Reg16 = Reg16(2);
    /// The bx register or complete selector set.
    pub const BX: Reg16 = Reg16(3);
    /// The sp register or complete selector set.
    pub const SP: Reg16 = Reg16(4);
    /// The bp register or complete selector set.
    pub const BP: Reg16 = Reg16(5);
    /// The si register or complete selector set.
    pub const SI: Reg16 = Reg16(6);
    /// The di register or complete selector set.
    pub const DI: Reg16 = Reg16(7);

    /// Returns the conventional assembly register or condition name.
    pub fn name(self) -> &'static str {
        match self.0 {
            0 => "ax",
            1 => "cx",
            2 => "dx",
            3 => "bx",
            4 => "sp",
            5 => "bp",
            6 => "si",
            7 => "di",
            // An index above seven indicates a decoder bug; keep formatting total.
            _ => "??",
        }
    }

    /// Returns the encoded register index.
    pub fn index(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for Reg16 {
    type Error = RegisterIndexError;

    fn try_from(index: u8) -> Result<Self, Self::Error> {
        (index < 8)
            .then_some(Self(index))
            .ok_or(RegisterIndexError { width: 16, index })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{index} is not a V30 {width}-bit register index")]
/// Returned when a numeric register index exceeds three bits.
pub struct RegisterIndexError {
    width: u8,
    index: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A native V30 segment register.
pub enum Sreg {
    /// The es form.
    ES,
    /// The cs form.
    CS,
    /// The ss form.
    SS,
    /// The ds form.
    DS,
}

impl Sreg {
    /// Returns the conventional assembly register or condition name.
    pub fn name(self) -> &'static str {
        match self {
            Sreg::ES => "es",
            Sreg::CS => "cs",
            Sreg::SS => "ss",
            Sreg::DS => "ds",
        }
    }

    /// Convert a ModR/M reg field or segment-override selector to a segment register.
    /// Selectors four through seven are undefined on 8086 and return `None`.
    pub fn from_index(i: u8) -> Option<Sreg> {
        match i {
            0 => Some(Sreg::ES),
            1 => Some(Sreg::CS),
            2 => Some(Sreg::SS),
            3 => Some(Sreg::DS),
            _ => None,
        }
    }

    /// Returns the encoded selector index.
    pub fn to_index(self) -> u8 {
        match self {
            Sreg::ES => 0,
            Sreg::CS => 1,
            Sreg::SS => 2,
            Sreg::DS => 3,
        }
    }
}

/// The eight 8086 ModR/M address bases plus the special absolute `[disp16]` form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemBase {
    /// The bx si form.
    BxSi,
    /// The bx di form.
    BxDi,
    /// The bp si form.
    BpSi,
    /// The bp di form.
    BpDi,
    /// The si form.
    Si,
    /// The di form.
    Di,
    /// The bp form.
    Bp,
    /// The bx form.
    Bx,
    /// The direct form.
    Direct, // mod=00 r/m=110: [absolute16], not BP.
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// The byte or word width of a native V30 operand.
pub enum OpSize {
    /// The byte form.
    Byte,
    /// The word form.
    Word,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// An immediate value with explicit encoded width.
pub enum SignedImmediate {
    /// The byte form.
    Byte(i8),
    /// The word form.
    Word(i16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A checked V30 bit-field length selector representing one through sixteen bits.
pub struct BitFieldLength(u8);

impl BitFieldLength {
    /// Returns the encoded selector field.
    pub fn selector(self) -> u8 {
        self.0
    }

    /// Returns the represented bit count.
    pub fn bit_count(self) -> u8 {
        self.0 + 1
    }
}

impl TryFrom<u8> for BitFieldLength {
    type Error = BitFieldLengthError;

    fn try_from(selector: u8) -> Result<Self, Self::Error> {
        (selector < 16)
            .then_some(Self(selector))
            .ok_or(BitFieldLengthError { selector })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{selector} is not a 4-bit V30 bit-field length selector")]
/// Returned when a bit-field length lies outside one through sixteen bits.
pub struct BitFieldLengthError {
    selector: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// An immediate or register-supplied bit index.
pub enum BitIndex {
    /// The cl form.
    Cl,
    /// The immediate form.
    Immediate(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A checked five-bit ESC operation selector.
pub struct EscapeOpcode(u8);

impl EscapeOpcode {
    /// Returns the checked opcode or register field.
    pub fn value(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for EscapeOpcode {
    type Error = EscapeOpcodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        (value < 64)
            .then_some(Self(value))
            .ok_or(EscapeOpcodeError { value })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{value} is not a 6-bit V30 ESC operation")]
/// Returned when an ESC selector exceeds five bits.
pub struct EscapeOpcodeError {
    value: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A checked six-bit FPO2 operation selector.
pub struct Fpo2Opcode(u8);

impl Fpo2Opcode {
    /// Returns the checked opcode or register field.
    pub fn value(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for Fpo2Opcode {
    type Error = Fpo2OpcodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        (value < 16)
            .then_some(Self(value))
            .ok_or(Fpo2OpcodeError { value })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{value} is not a 4-bit V30 FPO2 operation")]
/// Returned when an FPO2 selector exceeds six bits.
pub struct Fpo2OpcodeError {
    value: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A checked three-bit external-processor register selector.
pub struct ExternalRegister(u8);

impl ExternalRegister {
    /// Returns the encoded register index.
    pub fn index(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for ExternalRegister {
    type Error = ExternalRegisterError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        (value < 8)
            .then_some(Self(value))
            .ok_or(ExternalRegisterError { value })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{value} is not a 3-bit external floating-point register selector")]
/// Returned when an external register index exceeds three bits.
pub struct ExternalRegisterError {
    value: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// An external-processor register or memory operand.
pub enum FloatingPointOperand {
    /// The register form.
    Register(ExternalRegister),
    /// The memory form.
    Memory(Mem),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// The absent, signed-byte, or signed-word displacement of an effective address.
pub enum EffectiveAddressDisplacement {
    /// The absolute form.
    Absolute(u16),
    /// The signed form.
    Signed(i16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
/// Failure returned when a typed effective address cannot be represented.
pub enum EffectiveAddressError {
    #[error("direct V30 addressing requires an absolute 16-bit address")]
    /// The direct requires absolute form.
    DirectRequiresAbsolute,
    #[error("register-based V30 addressing requires a signed displacement")]
    /// The based requires signed form.
    BasedRequiresSigned,
}

/// 8086/V30 memory addressing with a typed public displacement boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mem {
    pub(crate) seg: Option<Sreg>,
    pub(crate) base: MemBase,
    pub(crate) disp: i16,
    pub(crate) size: OpSize,
}

impl Mem {
    /// Constructs the value after checking its architectural encoding constraints.
    pub fn new(
        segment: Option<Sreg>,
        base: MemBase,
        displacement: EffectiveAddressDisplacement,
        size: OpSize,
    ) -> Result<Self, EffectiveAddressError> {
        let disp = match (base, displacement) {
            (MemBase::Direct, EffectiveAddressDisplacement::Absolute(address)) => address as i16,
            (MemBase::Direct, EffectiveAddressDisplacement::Signed(_)) => {
                return Err(EffectiveAddressError::DirectRequiresAbsolute);
            }
            (_, EffectiveAddressDisplacement::Signed(displacement)) => displacement,
            (_, EffectiveAddressDisplacement::Absolute(_)) => {
                return Err(EffectiveAddressError::BasedRequiresSigned);
            }
        };
        Ok(Self {
            seg: segment,
            base,
            disp,
            size,
        })
    }

    /// Returns the effective segment override.
    pub fn segment(self) -> Option<Sreg> {
        self.seg
    }

    /// Returns the effective-address base.
    pub fn base(self) -> MemBase {
        self.base
    }

    /// Returns the typed displacement.
    pub fn displacement(self) -> EffectiveAddressDisplacement {
        if self.base == MemBase::Direct {
            EffectiveAddressDisplacement::Absolute(self.disp as u16)
        } else {
            EffectiveAddressDisplacement::Signed(self.disp)
        }
    }

    /// Returns the operand width.
    pub fn size(self) -> OpSize {
        self.size
    }

    /// Returns the memory operand with the supplied segment override.
    pub fn with_segment(mut self, segment: Option<Sreg>) -> Self {
        self.seg = segment;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A typed register, memory, or immediate instruction operand.
pub enum Operand {
    /// The reg8 form.
    Reg8(Reg8),
    /// The reg16 form.
    Reg16(Reg16),
    /// The sreg form.
    Sreg(Sreg),
    /// The imm8 form.
    Imm8(u8),
    /// The imm16 form.
    Imm16(u16),
    /// The mem form.
    Mem(Mem),
    /// An 8-bit branch displacement.
    Rel8(i8),
    /// A 16-bit branch displacement.
    Rel16(i16),
    /// A far pointer.
    Far {
        /// The segment value.
        seg: u16,
        /// The wrapping sixteen-bit offset.
        off: u16,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A native V30 condition-code selector.
pub enum Cond {
    /// The o form.
    O,
    /// The no form.
    No,
    /// The b form.
    B,
    /// The nb form.
    Nb,
    /// The e form.
    E,
    /// The ne form.
    Ne,
    /// The be form.
    Be,
    /// The a form.
    A,
    /// The s form.
    S,
    /// The ns form.
    Ns,
    /// The p form.
    P,
    /// The np form.
    Np,
    /// The l form.
    L,
    /// The ge form.
    Ge,
    /// The le form.
    Le,
    /// The g form.
    G,
    /// The cxz form.
    Cxz,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A loop-family control-flow condition.
pub enum LoopCondition {
    /// LOOPNE/LOOPNZ: decrement CX and branch while CX != 0 and ZF == 0.
    NotEqual,
    /// LOOPE/LOOPZ: decrement CX and branch while CX != 0 and ZF == 1.
    Equal,
    /// LOOP: decrement CX and branch while CX != 0.
    Always,
}

impl Cond {
    /// Convert the low nibble of a Jcc opcode to a condition.
    /// JCXZ uses opcode `0xE3` and is intentionally handled separately.
    pub fn from_jcc_index(i: u8) -> Option<Cond> {
        match i {
            0x0 => Some(Cond::O),
            0x1 => Some(Cond::No),
            0x2 => Some(Cond::B),
            0x3 => Some(Cond::Nb),
            0x4 => Some(Cond::E),
            0x5 => Some(Cond::Ne),
            0x6 => Some(Cond::Be),
            0x7 => Some(Cond::A),
            0x8 => Some(Cond::S),
            0x9 => Some(Cond::Ns),
            0xA => Some(Cond::P),
            0xB => Some(Cond::Np),
            0xC => Some(Cond::L),
            0xD => Some(Cond::Ge),
            0xE => Some(Cond::Le),
            0xF => Some(Cond::G),
            _ => None,
        }
    }

    /// Convert one of the 16 Jcc conditions to its opcode low nibble.
    /// `Cxz` uses the separate `0xE3` opcode and returns `None`.
    pub fn to_jcc_index(self) -> Option<u8> {
        Some(match self {
            Cond::O => 0x0,
            Cond::No => 0x1,
            Cond::B => 0x2,
            Cond::Nb => 0x3,
            Cond::E => 0x4,
            Cond::Ne => 0x5,
            Cond::Be => 0x6,
            Cond::A => 0x7,
            Cond::S => 0x8,
            Cond::Ns => 0x9,
            Cond::P => 0xA,
            Cond::Np => 0xB,
            Cond::L => 0xC,
            Cond::Ge => 0xD,
            Cond::Le => 0xE,
            Cond::G => 0xF,
            Cond::Cxz => return None,
        })
    }

    /// Returns the conventional assembly register or condition name.
    pub fn name(self) -> &'static str {
        match self {
            Cond::O => "o",
            Cond::No => "no",
            Cond::B => "b",
            Cond::Nb => "nb",
            Cond::E => "e",
            Cond::Ne => "ne",
            Cond::Be => "be",
            Cond::A => "a",
            Cond::S => "s",
            Cond::Ns => "ns",
            Cond::P => "p",
            Cond::Np => "np",
            Cond::L => "l",
            Cond::Ge => "ge",
            Cond::Le => "le",
            Cond::G => "g",
            Cond::Cxz => "cxz",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A short, near, or far jump target representation.
pub enum JmpTarget {
    /// The rel8 form.
    Rel8(i8),
    /// The rel16 form.
    Rel16(i16),
    /// The far form.
    Far {
        /// The segment value.
        seg: u16,
        /// The wrapping sixteen-bit offset.
        off: u16,
    },
    /// 0xFF /4: near JMP through r/m16; loads IP and leaves CS unchanged.
    IndirectNear(Operand),
    /// 0xFF /5: far JMP through a memory-only m16:16 pointer.
    /// Register mode is undefined and decodes as `Unknown` rather than a near jump.
    IndirectFar(Mem),
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A near or far call target representation.
pub enum CallTarget {
    /// The rel16 form.
    Rel16(i16),
    /// The far form.
    Far {
        /// The segment value.
        seg: u16,
        /// The wrapping sixteen-bit offset.
        off: u16,
    },
    /// 0xFF /2: near CALL through r/m16; loads only IP.
    IndirectNear(Operand),
    /// 0xFF /3: far CALL through a memory-only m16:16 pointer.
    /// Register mode is undefined and decodes as `Unknown`.
    IndirectFar(Mem),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// The implicit-one, immediate, or `CL` shift-count source.
pub enum ShiftCount {
    /// The one form.
    One,
    /// The cl form.
    Cl,
    /// The imm8 form.
    Imm8(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// An immediate or `DX` I/O port address.
pub enum PortAddr {
    /// The imm8 form.
    Imm8(u8),
    /// The dx form.
    Dx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A segmented 16:16 native-mode instruction address.
pub struct SegOff {
    /// The segment value.
    pub seg: u16,
    /// The wrapping sixteen-bit offset.
    pub off: u16,
}

impl SegOff {
    /// Returns the 20-bit real-mode linear address before physical wrapping.
    pub fn linear(self) -> u32 {
        ((self.seg as u32) << 4) + self.off as u32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// Semantic prefixes attached to a decoded internal instruction.
pub struct Prefixes {
    /// The effective segment override.
    pub seg_override: Option<Sreg>,
    /// Whether the bus-lock prefix is present.
    pub bus_lock: bool,
    /// V30 REPC prefix (`0x65`): repeat while carry; absent from 8086.
    pub repc: bool,
    /// V30 REPNC prefix (`0x64`): repeat while carry is clear.
    pub repnc: bool,
}

/// Typed semantics for native V30 instructions.
/// `Unknown` supports internal linear sweeps and import regression only; public
/// byte lowering and the checked assembler reject it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstructionKind {
    // Data movement.
    /// The mov form.
    Mov {
        /// The dest operand.
        dest: Operand,
        /// The src operand.
        src: Operand,
    },
    /// The lea form.
    Lea {
        /// The dest operand.
        dest: Reg16,
        /// The src operand.
        src: Mem,
    },
    /// The les form.
    Les {
        /// The dest operand.
        dest: Reg16,
        /// The src operand.
        src: Mem,
    },
    /// The lds form.
    Lds {
        /// The dest operand.
        dest: Reg16,
        /// The src operand.
        src: Mem,
    },
    /// The xchg form.
    Xchg {
        /// The a operand.
        a: Operand,
        /// The b operand.
        b: Operand,
    },
    /// The push form.
    Push {
        /// The src operand.
        src: Operand,
    },
    /// The pop form.
    Pop {
        /// The dest operand.
        dest: Operand,
    },
    /// The pusha form.
    Pusha,
    /// The popa form.
    Popa,
    /// The bound form.
    Bound {
        /// The index operand.
        index: Reg16,
        /// The bounds operand.
        bounds: Mem,
    },

    // Arithmetic and logic.
    /// The add form.
    Add {
        /// The dest operand.
        dest: Operand,
        /// The src operand.
        src: Operand,
    },
    /// The adc form.
    Adc {
        /// The dest operand.
        dest: Operand,
        /// The src operand.
        src: Operand,
    },
    /// The sub form.
    Sub {
        /// The dest operand.
        dest: Operand,
        /// The src operand.
        src: Operand,
    },
    /// The sbb form.
    Sbb {
        /// The dest operand.
        dest: Operand,
        /// The src operand.
        src: Operand,
    },
    /// The cmp form.
    Cmp {
        /// The a operand.
        a: Operand,
        /// The b operand.
        b: Operand,
    },
    /// The test form.
    Test {
        /// The a operand.
        a: Operand,
        /// The b operand.
        b: Operand,
    },
    /// The and form.
    And {
        /// The dest operand.
        dest: Operand,
        /// The src operand.
        src: Operand,
    },
    /// The or form.
    Or {
        /// The dest operand.
        dest: Operand,
        /// The src operand.
        src: Operand,
    },
    /// The xor form.
    Xor {
        /// The dest operand.
        dest: Operand,
        /// The src operand.
        src: Operand,
    },
    /// The inc form.
    Inc {
        /// The dest operand.
        dest: Operand,
    },
    /// The dec form.
    Dec {
        /// The dest operand.
        dest: Operand,
    },
    /// The neg form.
    Neg {
        /// The dest operand.
        dest: Operand,
    },
    /// The not form.
    Not {
        /// The dest operand.
        dest: Operand,
    },
    /// The mul form.
    Mul {
        /// The src operand.
        src: Operand,
    },
    /// The imul form.
    Imul {
        /// The src operand.
        src: Operand,
    },
    /// The imul immediate form.
    ImulImmediate {
        /// The dest operand.
        dest: Reg16,
        /// The src operand.
        src: Operand,
        /// The immediate operand.
        immediate: SignedImmediate,
    },
    /// The div form.
    Div {
        /// The src operand.
        src: Operand,
    },
    /// The idiv form.
    Idiv {
        /// The src operand.
        src: Operand,
    },

    // Shifts and rotates.
    /// The shl form.
    Shl {
        /// The dest operand.
        dest: Operand,
        /// The count operand.
        count: ShiftCount,
    },
    /// The shr form.
    Shr {
        /// The dest operand.
        dest: Operand,
        /// The count operand.
        count: ShiftCount,
    },
    /// The sar form.
    Sar {
        /// The dest operand.
        dest: Operand,
        /// The count operand.
        count: ShiftCount,
    },
    /// The rol form.
    Rol {
        /// The dest operand.
        dest: Operand,
        /// The count operand.
        count: ShiftCount,
    },
    /// The ror form.
    Ror {
        /// The dest operand.
        dest: Operand,
        /// The count operand.
        count: ShiftCount,
    },
    /// The rcl form.
    Rcl {
        /// The dest operand.
        dest: Operand,
        /// The count operand.
        count: ShiftCount,
    },
    /// The rcr form.
    Rcr {
        /// The dest operand.
        dest: Operand,
        /// The count operand.
        count: ShiftCount,
    },

    // Control flow.
    /// The jcc form.
    Jcc {
        /// The cond operand.
        cond: Cond,
        /// The resolved target address.
        target: i8,
    },
    /// The loop form.
    Loop {
        /// The selected condition.
        condition: LoopCondition,
        /// The resolved target address.
        target: i8,
    },
    /// The jmp form.
    Jmp {
        /// The resolved target address.
        target: JmpTarget,
    },
    /// The call form.
    Call {
        /// The resolved target address.
        target: CallTarget,
    },
    /// The ret form.
    Ret {
        /// The pop operand.
        pop: u16,
    },
    /// The retf form.
    Retf {
        /// The pop operand.
        pop: u16,
    },

    // Interrupts.
    /// The int form.
    Int {
        /// The vector operand.
        vector: u8,
    },
    /// The into form.
    Into,
    /// The iret form.
    Iret,

    // I/O
    /// The in al form.
    InAl {
        /// The port operand.
        port: PortAddr,
    },
    /// The in ax form.
    InAx {
        /// The port operand.
        port: PortAddr,
    },
    /// The out al form.
    OutAl {
        /// The port operand.
        port: PortAddr,
    },
    /// The out ax form.
    OutAx {
        /// The port operand.
        port: PortAddr,
    },
    /// The insb form.
    Insb,
    /// The insw form.
    Insw,
    /// The outsb form.
    Outsb,
    /// The outsw form.
    Outsw,

    // String
    /// The movsb form.
    Movsb,
    /// The movsw form.
    Movsw,
    /// The lodsb form.
    Lodsb,
    /// The lodsw form.
    Lodsw,
    /// The stosb form.
    Stosb,
    /// The stosw form.
    Stosw,
    /// The cmpsb form.
    Cmpsb,
    /// The cmpsw form.
    Cmpsw,
    /// The scasb form.
    Scasb,
    /// The scasw form.
    Scasw,
    /// A string instruction with REP or REPE.
    Rep(Box<InstructionKind>),
    /// A string instruction with REPNE.
    Repne(Box<InstructionKind>),
    /// The repc form.
    Repc(Box<InstructionKind>),
    /// The repnc form.
    Repnc(Box<InstructionKind>),
    /// The bus lock form.
    BusLock(Box<InstructionKind>),
    /// The segment override form.
    SegmentOverride {
        /// The optional segment override.
        segment: Sreg,
        /// The typed instruction.
        instruction: Box<InstructionKind>,
    },

    // Miscellaneous instructions.
    /// The nop form.
    Nop,
    /// The hlt form.
    Hlt,
    /// The xlat form.
    Xlat,

    // Single-byte, operand-free flag instructions from Intel SDM Volume 2A, Appendix B.
    /// 0x9B: `WAIT` / `FWAIT`, synchronizing with an external FPU.
    Wait,
    /// 0x9C — `PUSHF` (push FLAGS to stack).
    Pushf,
    /// 0x9D — `POPF` (pop stack to FLAGS).
    Popf,
    /// 0x9E — `SAHF` (store AH to FLAGS low byte).
    Sahf,
    /// 0x9F — `LAHF` (load FLAGS low byte to AH).
    Lahf,
    /// 0xF5 — `CMC` (complement carry flag).
    Cmc,
    /// 0xF8 — `CLC` (clear carry flag).
    Clc,
    /// 0xF9 — `STC` (set carry flag).
    Stc,
    /// 0xFA — `CLI` (clear interrupt-enable flag).
    Cli,
    /// 0xFB — `STI` (set interrupt-enable flag).
    Sti,
    /// 0xFC — `CLD` (clear direction flag).
    Cld,
    /// 0xFD — `STD` (set direction flag).
    Std,

    /// 0x98 — `CBW` (sign-extend AL → AX).
    Cbw,
    /// 0x99 — `CWD` (sign-extend AX → DX:AX).
    Cwd,
    /// The aaa form.
    Aaa,
    /// The aas form.
    Aas,
    /// The daa form.
    Daa,
    /// The das form.
    Das,
    /// The aam form.
    Aam {
        /// The effective-address base.
        base: u8,
    },
    /// The aad form.
    Aad {
        /// The effective-address base.
        base: u8,
    },
    /// The enter form.
    Enter {
        /// The frame size operand.
        frame_size: u16,
        /// The nesting level operand.
        nesting_level: u8,
    },
    /// The leave form.
    Leave,
    /// The esc form.
    Esc {
        /// The operation operand.
        operation: EscapeOpcode,
        /// The operand operand.
        operand: FloatingPointOperand,
    },
    /// The fpo2 form.
    Fpo2 {
        /// The operation operand.
        operation: Fpo2Opcode,
        /// The operand operand.
        operand: FloatingPointOperand,
    },

    // V30 extensions under the `0x0F` secondary opcode prefix.
    // Sources: NEC uPD70116 data sheet section 3 and NEC's 1985 V30 manual,
    // Appendix A. The native V30 detection branch is included as part of the ISA.
    /// `0F 20` — 4-bit packed BCD string add (DS:SI ← DS:SI op ES:DI, length CL).
    Add4s,
    /// `0F 22` — 4-bit packed BCD string subtract.
    Sub4s,
    /// `0F 26` — 4-bit packed BCD string compare.
    Cmp4s,
    /// `0F 28 /r` — ROL4 r/m8.
    Rol4 {
        /// The rm operand.
        rm: Operand,
    },
    /// `0F 2A /r` — ROR4 r/m8.
    Ror4 {
        /// The rm operand.
        rm: Operand,
    },
    /// `0F 31 /r` — INS reg8,reg8.
    Ins {
        /// The bit offset operand.
        bit_offset: Reg8,
        /// The length operand.
        length: Reg8,
    },
    /// `0F 33 /r` — EXT reg8,reg8.
    Ext {
        /// The bit offset operand.
        bit_offset: Reg8,
        /// The length operand.
        length: Reg8,
    },
    /// `0F 39 /0 ib` — INS reg8,imm4.
    InsImm {
        /// The bit offset operand.
        bit_offset: Reg8,
        /// The length operand.
        length: BitFieldLength,
    },
    /// `0F 3B /0 ib` — EXT reg8,imm4.
    ExtImm {
        /// The bit offset operand.
        bit_offset: Reg8,
        /// The length operand.
        length: BitFieldLength,
    },
    /// The test1 form.
    Test1 {
        /// The resolved target address.
        target: Operand,
        /// The bit operand.
        bit: BitIndex,
    },
    /// The clr1 form.
    Clr1 {
        /// The resolved target address.
        target: Operand,
        /// The bit operand.
        bit: BitIndex,
    },
    /// The set1 form.
    Set1 {
        /// The resolved target address.
        target: Operand,
        /// The bit operand.
        bit: BitIndex,
    },
    /// The not1 form.
    Not1 {
        /// The resolved target address.
        target: Operand,
        /// The bit operand.
        bit: BitIndex,
    },
    /// `0F FF ib` — BRKEM imm8, the native static-analysis boundary.
    Brkem {
        /// The vector operand.
        vector: u8,
    },

    // Unclassified input retained only for import round trips.
    /// Raw bytes not recognized by the permissive internal decoder.
    /// `Instruction::bytes` is authoritative; `raw` mirrors it.
    Unknown {
        /// The raw operand.
        raw: Vec<u8>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A decoded internal instruction with address, bytes, kind, and prefixes.
pub struct Instruction {
    /// The segmented instruction address.
    pub cs_ip: SegOff,
    /// The exact instruction or program bytes.
    pub bytes: Vec<u8>,
    /// The typed instruction kind.
    pub kind: InstructionKind,
    /// The preserved semantic prefix sequence.
    pub prefixes: Prefixes,
}
