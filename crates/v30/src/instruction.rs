//! 8086/V30 ISA의 핵심 타입 정의.

/// 8086 8비트 GPR. 인덱스는 ModR/M r/m / reg 필드의 하위 3비트 (0..=7).
/// `pub u8`이지만 디코더는 `b & 0x07`로 마스킹해 항상 0..=7로 제한해야 한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reg8(pub(crate) u8);

impl Reg8 {
    pub const AL: Reg8 = Reg8(0);
    pub const CL: Reg8 = Reg8(1);
    pub const DL: Reg8 = Reg8(2);
    pub const BL: Reg8 = Reg8(3);
    pub const AH: Reg8 = Reg8(4);
    pub const CH: Reg8 = Reg8(5);
    pub const DH: Reg8 = Reg8(6);
    pub const BH: Reg8 = Reg8(7);

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
            // 8 이상은 디코더 버그를 의미 — 표시용 fallback.
            _ => "??",
        }
    }

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

/// 8086 16비트 GPR. 인덱스는 ModR/M r/m / reg 필드의 하위 3비트 (0..=7).
/// `pub u8`이지만 디코더는 `b & 0x07`로 마스킹해 항상 0..=7로 제한해야 한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reg16(pub(crate) u8);

impl Reg16 {
    pub const AX: Reg16 = Reg16(0);
    pub const CX: Reg16 = Reg16(1);
    pub const DX: Reg16 = Reg16(2);
    pub const BX: Reg16 = Reg16(3);
    pub const SP: Reg16 = Reg16(4);
    pub const BP: Reg16 = Reg16(5);
    pub const SI: Reg16 = Reg16(6);
    pub const DI: Reg16 = Reg16(7);

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
            // 8 이상은 디코더 버그를 의미 — 표시용 fallback.
            _ => "??",
        }
    }

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
pub struct RegisterIndexError {
    width: u8,
    index: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sreg {
    ES,
    CS,
    SS,
    DS,
}

impl Sreg {
    pub fn name(self) -> &'static str {
        match self {
            Sreg::ES => "es",
            Sreg::CS => "cs",
            Sreg::SS => "ss",
            Sreg::DS => "ds",
        }
    }

    /// ModR/M의 reg 필드(0-7) 또는 segment override prefix 인코딩에서 Sreg로 변환.
    /// 8086에서 4-7은 미정의 — 디코더가 잘못된 인코딩으로 처리해야 하므로 `None`.
    pub fn from_index(i: u8) -> Option<Sreg> {
        match i {
            0 => Some(Sreg::ES),
            1 => Some(Sreg::CS),
            2 => Some(Sreg::SS),
            3 => Some(Sreg::DS),
            _ => None,
        }
    }

    pub fn to_index(self) -> u8 {
        match self {
            Sreg::ES => 0,
            Sreg::CS => 1,
            Sreg::SS => 2,
            Sreg::DS => 3,
        }
    }
}

/// 8086 메모리 어드레싱 base — ModR/M r/m 필드의 8가지 패턴 + 특수 [disp16].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemBase {
    BxSi,
    BxDi,
    BpSi,
    BpDi,
    Si,
    Di,
    Bp,
    Bx,
    Direct, // mod=00 r/m=110: [absolute16] (BP가 아님)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpSize {
    Byte,
    Word,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignedImmediate {
    Byte(i8),
    Word(i16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BitFieldLength(u8);

impl BitFieldLength {
    pub fn selector(self) -> u8 {
        self.0
    }

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
pub struct BitFieldLengthError {
    selector: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitIndex {
    Cl,
    Immediate(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EscapeOpcode(u8);

impl EscapeOpcode {
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
pub struct EscapeOpcodeError {
    value: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fpo2Opcode(u8);

impl Fpo2Opcode {
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
pub struct Fpo2OpcodeError {
    value: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExternalRegister(u8);

impl ExternalRegister {
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
pub struct ExternalRegisterError {
    value: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FloatingPointOperand {
    Register(ExternalRegister),
    Memory(Mem),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectiveAddressDisplacement {
    Absolute(u16),
    Signed(i16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EffectiveAddressError {
    #[error("direct V30 addressing requires an absolute 16-bit address")]
    DirectRequiresAbsolute,
    #[error("register-based V30 addressing requires a signed displacement")]
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

    pub fn segment(self) -> Option<Sreg> {
        self.seg
    }

    pub fn base(self) -> MemBase {
        self.base
    }

    pub fn displacement(self) -> EffectiveAddressDisplacement {
        if self.base == MemBase::Direct {
            EffectiveAddressDisplacement::Absolute(self.disp as u16)
        } else {
            EffectiveAddressDisplacement::Signed(self.disp)
        }
    }

    pub fn size(self) -> OpSize {
        self.size
    }

    pub fn with_segment(mut self, segment: Option<Sreg>) -> Self {
        self.seg = segment;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    Reg8(Reg8),
    Reg16(Reg16),
    Sreg(Sreg),
    Imm8(u8),
    Imm16(u16),
    Mem(Mem),
    /// 8비트 분기 displacement
    Rel8(i8),
    /// 16비트 분기 displacement
    Rel16(i16),
    /// far pointer
    Far {
        seg: u16,
        off: u16,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cond {
    O,
    No,
    B,
    Nb,
    E,
    Ne,
    Be,
    A,
    S,
    Ns,
    P,
    Np,
    L,
    Ge,
    Le,
    G,
    Cxz,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LoopCondition {
    /// LOOPNE/LOOPNZ: decrement CX and branch while CX != 0 and ZF == 0.
    NotEqual,
    /// LOOPE/LOOPZ: decrement CX and branch while CX != 0 and ZF == 1.
    Equal,
    /// LOOP: decrement CX and branch while CX != 0.
    Always,
}

impl Cond {
    /// Jcc opcode의 low nibble (`0x0..=0xF`)을 Cond로. `0x10` 이상은 `None`.
    /// JCXZ(opcode `0xE3`)는 별도 처리 — 본 함수는 `Cond::Cxz`를 반환하지 않는다.
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

    /// Jcc opcode의 low nibble로 변환 (Jcc 16개만 의미 있음).
    /// `Cxz`는 별도 opcode `0xE3`로 인코딩되므로 `None`.
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
pub enum JmpTarget {
    Rel8(i8),
    Rel16(i16),
    Far {
        seg: u16,
        off: u16,
    },
    /// 0xFF /4 — JMP near r/m16. r/m operand에서 16비트 IP만 로드 (CS 불변).
    IndirectNear(Operand),
    /// 0xFF /5 — JMP far m16:16. **메모리 operand 전용** — 32비트 far pointer (off, seg)를
    /// 로드해 CS:IP에 적재. reg form (mod=11)은 spec-undefined이므로 디코더는 `Unknown`으로
    /// 떨어진다 (near와 의미 다름 — silent normalization 금지).
    IndirectFar(Mem),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallTarget {
    Rel16(i16),
    Far {
        seg: u16,
        off: u16,
    },
    /// 0xFF /2 — CALL near r/m16. r/m operand에서 16비트 IP만 로드.
    IndirectNear(Operand),
    /// 0xFF /3 — CALL far m16:16. **메모리 operand 전용** — 32비트 far pointer (off, seg)를
    /// 로드해 CS:IP에 적재. reg form (mod=11)은 spec-undefined → `Unknown`.
    IndirectFar(Mem),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShiftCount {
    One,
    Cl,
    Imm8(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortAddr {
    Imm8(u8),
    Dx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegOff {
    pub seg: u16,
    pub off: u16,
}

impl SegOff {
    pub fn linear(self) -> u32 {
        ((self.seg as u32) << 4) + self.off as u32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Prefixes {
    pub seg_override: Option<Sreg>,
    pub bus_lock: bool,
    /// V30 (uPD70116) 확장 prefix REPC (0x65) — repeat while carry. 8086에는 없음.
    pub repc: bool,
    /// V30 확장 prefix REPNC (0x64) — repeat while not carry.
    pub repnc: bool,
}

/// Native V30 명령의 typed 의미. `Unknown`은 내부 linear sweep과 반입 회귀용이며
/// 공개 byte lowering과 checked assembler에서는 거부된다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstructionKind {
    // 데이터 이동
    Mov {
        dest: Operand,
        src: Operand,
    },
    Lea {
        dest: Reg16,
        src: Mem,
    },
    Les {
        dest: Reg16,
        src: Mem,
    },
    Lds {
        dest: Reg16,
        src: Mem,
    },
    Xchg {
        a: Operand,
        b: Operand,
    },
    Push {
        src: Operand,
    },
    Pop {
        dest: Operand,
    },
    Pusha,
    Popa,
    Bound {
        index: Reg16,
        bounds: Mem,
    },

    // 산술/논리
    Add {
        dest: Operand,
        src: Operand,
    },
    Adc {
        dest: Operand,
        src: Operand,
    },
    Sub {
        dest: Operand,
        src: Operand,
    },
    Sbb {
        dest: Operand,
        src: Operand,
    },
    Cmp {
        a: Operand,
        b: Operand,
    },
    Test {
        a: Operand,
        b: Operand,
    },
    And {
        dest: Operand,
        src: Operand,
    },
    Or {
        dest: Operand,
        src: Operand,
    },
    Xor {
        dest: Operand,
        src: Operand,
    },
    Inc {
        dest: Operand,
    },
    Dec {
        dest: Operand,
    },
    Neg {
        dest: Operand,
    },
    Not {
        dest: Operand,
    },
    Mul {
        src: Operand,
    },
    Imul {
        src: Operand,
    },
    ImulImmediate {
        dest: Reg16,
        src: Operand,
        immediate: SignedImmediate,
    },
    Div {
        src: Operand,
    },
    Idiv {
        src: Operand,
    },

    // 시프트/로테이트
    Shl {
        dest: Operand,
        count: ShiftCount,
    },
    Shr {
        dest: Operand,
        count: ShiftCount,
    },
    Sar {
        dest: Operand,
        count: ShiftCount,
    },
    Rol {
        dest: Operand,
        count: ShiftCount,
    },
    Ror {
        dest: Operand,
        count: ShiftCount,
    },
    Rcl {
        dest: Operand,
        count: ShiftCount,
    },
    Rcr {
        dest: Operand,
        count: ShiftCount,
    },

    // 분기
    Jcc {
        cond: Cond,
        target: i8,
    },
    Loop {
        condition: LoopCondition,
        target: i8,
    },
    Jmp {
        target: JmpTarget,
    },
    Call {
        target: CallTarget,
    },
    Ret {
        pop: u16,
    },
    Retf {
        pop: u16,
    },

    // 인터럽트
    Int {
        vector: u8,
    },
    Into,
    Iret,

    // I/O
    InAl {
        port: PortAddr,
    },
    InAx {
        port: PortAddr,
    },
    OutAl {
        port: PortAddr,
    },
    OutAx {
        port: PortAddr,
    },
    Insb,
    Insw,
    Outsb,
    Outsw,

    // String
    Movsb,
    Movsw,
    Lodsb,
    Lodsw,
    Stosb,
    Stosw,
    Cmpsb,
    Cmpsw,
    Scasb,
    Scasw,
    /// REP/REPE prefix가 붙은 string 명령
    Rep(Box<InstructionKind>),
    /// REPNE prefix가 붙은 string 명령
    Repne(Box<InstructionKind>),
    Repc(Box<InstructionKind>),
    Repnc(Box<InstructionKind>),
    BusLock(Box<InstructionKind>),
    SegmentOverride {
        segment: Sreg,
        instruction: Box<InstructionKind>,
    },

    // 기타
    Nop,
    Hlt,
    Xlat,

    // 플래그 명령 (Intel SDM Vol 2A 부록 B; 모두 single-byte, no operand).
    /// 0x9B — `WAIT` / `FWAIT` (FPU와의 동기 — 8086 단독에서는 NOP과 유사).
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
    Aaa,
    Aas,
    Daa,
    Das,
    Aam {
        base: u8,
    },
    Aad {
        base: u8,
    },
    Enter {
        frame_size: u16,
        nesting_level: u8,
    },
    Leave,
    Esc {
        operation: EscapeOpcode,
        operand: FloatingPointOperand,
    },
    Fpo2 {
        operation: Fpo2Opcode,
        operand: FloatingPointOperand,
    },

    // V30 (NEC uPD70116) 확장 — `0x0F` secondary opcode prefix.
    // ─ 출처: NEC uPD70116 (V30) 데이터시트 §3 "Instruction Set Extensions",
    //   「16ビット マイクロプロセッサ V30」(NEC, 1985) Appendix A. 8086 비호환 V30 검출
    //   분기가 PC-9801 게임 코드에 흔하므로 디코더 완결성을 위해 추가.
    /// `0F 20` — 4-bit packed BCD string add (DS:SI ← DS:SI op ES:DI, length CL).
    Add4s,
    /// `0F 22` — 4-bit packed BCD string subtract.
    Sub4s,
    /// `0F 26` — 4-bit packed BCD string compare.
    Cmp4s,
    /// `0F 28 /r` — ROL4 r/m8.
    Rol4 {
        rm: Operand,
    },
    /// `0F 2A /r` — ROR4 r/m8.
    Ror4 {
        rm: Operand,
    },
    /// `0F 31 /r` — INS reg8,reg8.
    Ins {
        bit_offset: Reg8,
        length: Reg8,
    },
    /// `0F 33 /r` — EXT reg8,reg8.
    Ext {
        bit_offset: Reg8,
        length: Reg8,
    },
    /// `0F 39 /0 ib` — INS reg8,imm4.
    InsImm {
        bit_offset: Reg8,
        length: BitFieldLength,
    },
    /// `0F 3B /0 ib` — EXT reg8,imm4.
    ExtImm {
        bit_offset: Reg8,
        length: BitFieldLength,
    },
    Test1 {
        target: Operand,
        bit: BitIndex,
    },
    Clr1 {
        target: Operand,
        bit: BitIndex,
    },
    Set1 {
        target: Operand,
        bit: BitIndex,
    },
    Not1 {
        target: Operand,
        bit: BitIndex,
    },
    /// `0F FF ib` — BRKEM imm8, the native static-analysis boundary.
    Brkem {
        vector: u8,
    },

    // 미분류 — round-trip 보존
    /// 디코더가 인식하지 못한 raw 바이트. `emit`이 그대로 반환해 round-trip을 보존한다.
    /// `Instruction`에 wrap될 때 `Instruction::bytes`가 권위 사본이며 `raw`는 그것의 미러.
    Unknown {
        raw: Vec<u8>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    pub cs_ip: SegOff,
    pub bytes: Vec<u8>,
    pub kind: InstructionKind,
    pub prefixes: Prefixes,
}
