/// An 8-bit register or the byte at address HL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ByteOperand {
    /// B register.
    B,
    /// C register.
    C,
    /// D register.
    D,
    /// E register.
    E,
    /// H register.
    H,
    /// L register.
    L,
    /// Byte in memory addressed by HL.
    IndirectHl,
    /// Accumulator register.
    A,
}

impl ByteOperand {
    pub(crate) const fn from_field(field: u8) -> Self {
        match field {
            0 => Self::B,
            1 => Self::C,
            2 => Self::D,
            3 => Self::E,
            4 => Self::H,
            5 => Self::L,
            6 => Self::IndirectHl,
            7 => Self::A,
            _ => unreachable!(),
        }
    }

    /// Return the three-bit selector encoded for this operand.
    pub const fn field(self) -> u8 {
        match self {
            Self::B => 0,
            Self::C => 1,
            Self::D => 2,
            Self::E => 3,
            Self::H => 4,
            Self::L => 5,
            Self::IndirectHl => 6,
            Self::A => 7,
        }
    }
}

/// A 16-bit register selected by the SM83 `rp` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Register16 {
    /// BC register pair.
    Bc,
    /// DE register pair.
    De,
    /// HL register pair.
    Hl,
    /// Stack pointer.
    Sp,
}

impl Register16 {
    pub(crate) const fn from_field(field: u8) -> Self {
        match field {
            0 => Self::Bc,
            1 => Self::De,
            2 => Self::Hl,
            3 => Self::Sp,
            _ => unreachable!(),
        }
    }

    /// Return the two-bit `rp` selector encoded for this register.
    pub const fn field(self) -> u8 {
        match self {
            Self::Bc => 0,
            Self::De => 1,
            Self::Hl => 2,
            Self::Sp => 3,
        }
    }
}

/// A 16-bit register selected by PUSH and POP instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StackRegister {
    /// BC register pair.
    Bc,
    /// DE register pair.
    De,
    /// HL register pair.
    Hl,
    /// Accumulator and flags register pair.
    Af,
}

impl StackRegister {
    pub(crate) const fn from_field(field: u8) -> Self {
        match field {
            0 => Self::Bc,
            1 => Self::De,
            2 => Self::Hl,
            3 => Self::Af,
            _ => unreachable!(),
        }
    }

    /// Return the two-bit stack-register selector.
    pub const fn field(self) -> u8 {
        match self {
            Self::Bc => 0,
            Self::De => 1,
            Self::Hl => 2,
            Self::Af => 3,
        }
    }
}

/// The four implicit address updates used by accumulator loads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IndirectAddress {
    /// Memory addressed by BC without updating the pair.
    Bc,
    /// Memory addressed by DE without updating the pair.
    De,
    /// Memory addressed by HL, then increment HL.
    HlIncrement,
    /// Memory addressed by HL, then decrement HL.
    HlDecrement,
}

impl IndirectAddress {
    pub(crate) const fn from_field(field: u8) -> Self {
        match field {
            0 => Self::Bc,
            1 => Self::De,
            2 => Self::HlIncrement,
            3 => Self::HlDecrement,
            _ => unreachable!(),
        }
    }

    /// Return the two-bit indirect-address selector.
    pub const fn field(self) -> u8 {
        match self {
            Self::Bc => 0,
            Self::De => 1,
            Self::HlIncrement => 2,
            Self::HlDecrement => 3,
        }
    }
}

/// Status-flag condition accepted by conditional control flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Condition {
    /// Zero flag is clear.
    Nz,
    /// Zero flag is set.
    Z,
    /// Carry flag is clear.
    Nc,
    /// Carry flag is set.
    C,
}

impl Condition {
    pub(crate) const fn from_field(field: u8) -> Self {
        match field {
            0 => Self::Nz,
            1 => Self::Z,
            2 => Self::Nc,
            3 => Self::C,
            _ => unreachable!(),
        }
    }

    /// Return the two-bit condition selector.
    pub const fn field(self) -> u8 {
        match self {
            Self::Nz => 0,
            Self::Z => 1,
            Self::Nc => 2,
            Self::C => 3,
        }
    }
}

/// Accumulator ALU operation selected by an instruction family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AluOperation {
    /// Add without carry input.
    Add,
    /// Add with carry input.
    Adc,
    /// Subtract without carry input.
    Sub,
    /// Subtract with carry input.
    Sbc,
    /// Bitwise AND.
    And,
    /// Bitwise exclusive OR.
    Xor,
    /// Bitwise OR.
    Or,
    /// Compare by subtraction without retaining the result.
    Cp,
}

impl AluOperation {
    pub(crate) const fn from_field(field: u8) -> Self {
        match field {
            0 => Self::Add,
            1 => Self::Adc,
            2 => Self::Sub,
            3 => Self::Sbc,
            4 => Self::And,
            5 => Self::Xor,
            6 => Self::Or,
            7 => Self::Cp,
            _ => unreachable!(),
        }
    }

    /// Return the three-bit ALU-operation selector.
    pub const fn field(self) -> u8 {
        match self {
            Self::Add => 0,
            Self::Adc => 1,
            Self::Sub => 2,
            Self::Sbc => 3,
            Self::And => 4,
            Self::Xor => 5,
            Self::Or => 6,
            Self::Cp => 7,
        }
    }
}

/// Rotate, shift, or nibble-swap operation in the CB-prefixed space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RotateShiftOperation {
    /// Rotate left circular.
    Rlc,
    /// Rotate right circular.
    Rrc,
    /// Rotate left through carry.
    Rl,
    /// Rotate right through carry.
    Rr,
    /// Arithmetic shift left.
    Sla,
    /// Arithmetic shift right, retaining the sign bit.
    Sra,
    /// Exchange the upper and lower nibbles.
    Swap,
    /// Logical shift right, inserting zero at bit 7.
    Srl,
}

impl RotateShiftOperation {
    pub(crate) const fn from_field(field: u8) -> Self {
        match field {
            0 => Self::Rlc,
            1 => Self::Rrc,
            2 => Self::Rl,
            3 => Self::Rr,
            4 => Self::Sla,
            5 => Self::Sra,
            6 => Self::Swap,
            7 => Self::Srl,
            _ => unreachable!(),
        }
    }

    /// Return the three-bit rotate/shift selector.
    pub const fn field(self) -> u8 {
        match self {
            Self::Rlc => 0,
            Self::Rrc => 1,
            Self::Rl => 2,
            Self::Rr => 3,
            Self::Sla => 4,
            Self::Sra => 5,
            Self::Swap => 6,
            Self::Srl => 7,
        }
    }
}

/// One encodable bit position in a CB-prefixed bit operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum BitIndex {
    /// Bit 0.
    B0 = 0,
    /// Bit 1.
    B1 = 1,
    /// Bit 2.
    B2 = 2,
    /// Bit 3.
    B3 = 3,
    /// Bit 4.
    B4 = 4,
    /// Bit 5.
    B5 = 5,
    /// Bit 6.
    B6 = 6,
    /// Bit 7.
    B7 = 7,
}

impl BitIndex {
    pub(crate) const fn from_field(field: u8) -> Self {
        match field {
            0 => Self::B0,
            1 => Self::B1,
            2 => Self::B2,
            3 => Self::B3,
            4 => Self::B4,
            5 => Self::B5,
            6 => Self::B6,
            7 => Self::B7,
            _ => unreachable!(),
        }
    }

    /// Return the numeric bit position.
    pub const fn value(self) -> u8 {
        self as u8
    }
}

/// One of the eight fixed call targets accepted by `RST`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum RestartVector {
    /// Address `0x00`.
    H00 = 0x00,
    /// Address `0x08`.
    H08 = 0x08,
    /// Address `0x10`.
    H10 = 0x10,
    /// Address `0x18`.
    H18 = 0x18,
    /// Address `0x20`.
    H20 = 0x20,
    /// Address `0x28`.
    H28 = 0x28,
    /// Address `0x30`.
    H30 = 0x30,
    /// Address `0x38`.
    H38 = 0x38,
}

impl RestartVector {
    pub(crate) const fn from_field(field: u8) -> Self {
        match field {
            0 => Self::H00,
            1 => Self::H08,
            2 => Self::H10,
            3 => Self::H18,
            4 => Self::H20,
            5 => Self::H28,
            6 => Self::H30,
            7 => Self::H38,
            _ => unreachable!(),
        }
    }

    /// Return the fixed call target address.
    pub const fn value(self) -> u8 {
        self as u8
    }

    /// Return the three-bit selector encoded in the base opcode.
    pub const fn field(self) -> u8 {
        self.value() >> 3
    }
}

/// Every encodable SM83 instruction with invalid operand combinations excluded by type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Instruction {
    /// Perform no operation.
    Nop,
    /// Load a literal word into a 16-bit register.
    Load16Immediate(Register16, u16),
    /// Store the accumulator through BC, DE, HL+, or HL-.
    LoadIndirectFromA(IndirectAddress),
    /// Load the accumulator through BC, DE, HL+, or HL-.
    LoadAFromIndirect(IndirectAddress),
    /// Increment a 16-bit register.
    Increment16(Register16),
    /// Decrement a 16-bit register.
    Decrement16(Register16),
    /// Increment an 8-bit register or the byte at HL.
    Increment8(ByteOperand),
    /// Decrement an 8-bit register or the byte at HL.
    Decrement8(ByteOperand),
    /// Load a literal byte into an 8-bit register or the byte at HL.
    Load8Immediate(ByteOperand, u8),
    /// Copy one 8-bit register or HL-addressed byte to another.
    Load8(ByteOperand, ByteOperand),
    /// Store the stack pointer to a literal little-endian address.
    LoadAddressFromSp(u16),
    /// Add a 16-bit register to HL.
    AddHl(Register16),
    /// Rotate the accumulator left circular without the CB prefix.
    RotateLeftCircularA,
    /// Rotate the accumulator right circular without the CB prefix.
    RotateRightCircularA,
    /// Rotate the accumulator left through carry without the CB prefix.
    RotateLeftA,
    /// Rotate the accumulator right through carry without the CB prefix.
    RotateRightA,
    /// Enter the stopped low-power state; the required padding byte is encoded.
    Stop,
    /// Jump by a signed displacement relative to the next instruction.
    JumpRelative(i8),
    /// Conditionally jump by a signed displacement.
    JumpRelativeIf(Condition, i8),
    /// Decimal-adjust the accumulator after binary-coded-decimal arithmetic.
    DecimalAdjustA,
    /// Complement every accumulator bit.
    ComplementA,
    /// Set the carry flag.
    SetCarryFlag,
    /// Complement the carry flag.
    ComplementCarryFlag,
    /// Enter the halted state.
    Halt,
    /// Apply an accumulator ALU operation to a register or the byte at HL.
    Alu8(AluOperation, ByteOperand),
    /// Return from a subroutine when a status condition holds.
    ReturnIf(Condition),
    /// Pop a register pair from the stack.
    Pop(StackRegister),
    /// Jump to a literal address when a status condition holds.
    JumpIf(Condition, u16),
    /// Jump to a literal address.
    Jump(u16),
    /// Call a literal address when a status condition holds.
    CallIf(Condition, u16),
    /// Push a register pair onto the stack.
    Push(StackRegister),
    /// Apply an accumulator ALU operation to a literal byte.
    AluImmediate(AluOperation, u8),
    /// Call one fixed restart vector.
    Restart(RestartVector),
    /// Return from a subroutine.
    Return,
    /// Return from an interrupt and enable interrupts.
    ReturnFromInterrupt,
    /// Call a literal address.
    Call(u16),
    /// Store the accumulator at address `0xFF00 + immediate`.
    LoadHighAddressFromA(u8),
    /// Store the accumulator at address `0xFF00 + C`.
    LoadHighAddressCFromA,
    /// Add a signed byte to the stack pointer.
    AddSp(i8),
    /// Jump to the address in HL.
    JumpHl,
    /// Store the accumulator at a literal address.
    LoadAddressFromA(u16),
    /// Load the accumulator from address `0xFF00 + immediate`.
    LoadAFromHighAddress(u8),
    /// Load the accumulator from address `0xFF00 + C`.
    LoadAFromHighAddressC,
    /// Disable maskable interrupts.
    DisableInterrupts,
    /// Load HL from the stack pointer plus a signed byte.
    LoadHlFromSpPlus(i8),
    /// Copy HL to the stack pointer.
    LoadSpFromHl,
    /// Load the accumulator from a literal address.
    LoadAFromAddress(u16),
    /// Enable maskable interrupts after the architectural delay.
    EnableInterrupts,
    /// Apply one CB-prefixed rotate or shift operation.
    RotateShift(RotateShiftOperation, ByteOperand),
    /// Test one bit in a register or the byte at HL.
    Bit(BitIndex, ByteOperand),
    /// Reset one bit in a register or the byte at HL.
    ResetBit(BitIndex, ByteOperand),
    /// Set one bit in a register or the byte at HL.
    SetBit(BitIndex, ByteOperand),
}

/// Semantic instruction identity with literal values removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InstructionForm {
    /// Literal-free form of [`Instruction::Nop`].
    Nop,
    /// Literal-free form of [`Instruction::Load16Immediate`].
    Load16Immediate(Register16),
    /// Form of [`Instruction::LoadIndirectFromA`].
    LoadIndirectFromA(IndirectAddress),
    /// Form of [`Instruction::LoadAFromIndirect`].
    LoadAFromIndirect(IndirectAddress),
    /// Form of [`Instruction::Increment16`].
    Increment16(Register16),
    /// Form of [`Instruction::Decrement16`].
    Decrement16(Register16),
    /// Form of [`Instruction::Increment8`].
    Increment8(ByteOperand),
    /// Form of [`Instruction::Decrement8`].
    Decrement8(ByteOperand),
    /// Literal-free form of [`Instruction::Load8Immediate`].
    Load8Immediate(ByteOperand),
    /// Form of [`Instruction::Load8`].
    Load8(ByteOperand, ByteOperand),
    /// Literal-free form of [`Instruction::LoadAddressFromSp`].
    LoadAddressFromSp,
    /// Form of [`Instruction::AddHl`].
    AddHl(Register16),
    /// Form of [`Instruction::RotateLeftCircularA`].
    RotateLeftCircularA,
    /// Form of [`Instruction::RotateRightCircularA`].
    RotateRightCircularA,
    /// Form of [`Instruction::RotateLeftA`].
    RotateLeftA,
    /// Form of [`Instruction::RotateRightA`].
    RotateRightA,
    /// Form of [`Instruction::Stop`].
    Stop,
    /// Literal-free form of [`Instruction::JumpRelative`].
    JumpRelative,
    /// Literal-free form of [`Instruction::JumpRelativeIf`].
    JumpRelativeIf(Condition),
    /// Form of [`Instruction::DecimalAdjustA`].
    DecimalAdjustA,
    /// Form of [`Instruction::ComplementA`].
    ComplementA,
    /// Form of [`Instruction::SetCarryFlag`].
    SetCarryFlag,
    /// Form of [`Instruction::ComplementCarryFlag`].
    ComplementCarryFlag,
    /// Form of [`Instruction::Halt`].
    Halt,
    /// Form of [`Instruction::Alu8`].
    Alu8(AluOperation, ByteOperand),
    /// Form of [`Instruction::ReturnIf`].
    ReturnIf(Condition),
    /// Form of [`Instruction::Pop`].
    Pop(StackRegister),
    /// Literal-free form of [`Instruction::JumpIf`].
    JumpIf(Condition),
    /// Literal-free form of [`Instruction::Jump`].
    Jump,
    /// Literal-free form of [`Instruction::CallIf`].
    CallIf(Condition),
    /// Form of [`Instruction::Push`].
    Push(StackRegister),
    /// Literal-free form of [`Instruction::AluImmediate`].
    AluImmediate(AluOperation),
    /// Form of [`Instruction::Restart`].
    Restart(RestartVector),
    /// Form of [`Instruction::Return`].
    Return,
    /// Form of [`Instruction::ReturnFromInterrupt`].
    ReturnFromInterrupt,
    /// Literal-free form of [`Instruction::Call`].
    Call,
    /// Literal-free form of [`Instruction::LoadHighAddressFromA`].
    LoadHighAddressFromA,
    /// Form of [`Instruction::LoadHighAddressCFromA`].
    LoadHighAddressCFromA,
    /// Literal-free form of [`Instruction::AddSp`].
    AddSp,
    /// Form of [`Instruction::JumpHl`].
    JumpHl,
    /// Literal-free form of [`Instruction::LoadAddressFromA`].
    LoadAddressFromA,
    /// Literal-free form of [`Instruction::LoadAFromHighAddress`].
    LoadAFromHighAddress,
    /// Form of [`Instruction::LoadAFromHighAddressC`].
    LoadAFromHighAddressC,
    /// Form of [`Instruction::DisableInterrupts`].
    DisableInterrupts,
    /// Literal-free form of [`Instruction::LoadHlFromSpPlus`].
    LoadHlFromSpPlus,
    /// Form of [`Instruction::LoadSpFromHl`].
    LoadSpFromHl,
    /// Literal-free form of [`Instruction::LoadAFromAddress`].
    LoadAFromAddress,
    /// Form of [`Instruction::EnableInterrupts`].
    EnableInterrupts,
    /// Form of [`Instruction::RotateShift`].
    RotateShift(RotateShiftOperation, ByteOperand),
    /// Form of [`Instruction::Bit`].
    Bit(BitIndex, ByteOperand),
    /// Form of [`Instruction::ResetBit`].
    ResetBit(BitIndex, ByteOperand),
    /// Form of [`Instruction::SetBit`].
    SetBit(BitIndex, ByteOperand),
}

impl Instruction {
    /// Return the semantic instruction identity with literal values removed.
    pub const fn form(self) -> InstructionForm {
        match self {
            Self::Nop => InstructionForm::Nop,
            Self::Load16Immediate(register, _) => InstructionForm::Load16Immediate(register),
            Self::LoadIndirectFromA(address) => InstructionForm::LoadIndirectFromA(address),
            Self::LoadAFromIndirect(address) => InstructionForm::LoadAFromIndirect(address),
            Self::Increment16(register) => InstructionForm::Increment16(register),
            Self::Decrement16(register) => InstructionForm::Decrement16(register),
            Self::Increment8(operand) => InstructionForm::Increment8(operand),
            Self::Decrement8(operand) => InstructionForm::Decrement8(operand),
            Self::Load8Immediate(operand, _) => InstructionForm::Load8Immediate(operand),
            Self::Load8(destination, source) => InstructionForm::Load8(destination, source),
            Self::LoadAddressFromSp(_) => InstructionForm::LoadAddressFromSp,
            Self::AddHl(register) => InstructionForm::AddHl(register),
            Self::RotateLeftCircularA => InstructionForm::RotateLeftCircularA,
            Self::RotateRightCircularA => InstructionForm::RotateRightCircularA,
            Self::RotateLeftA => InstructionForm::RotateLeftA,
            Self::RotateRightA => InstructionForm::RotateRightA,
            Self::Stop => InstructionForm::Stop,
            Self::JumpRelative(_) => InstructionForm::JumpRelative,
            Self::JumpRelativeIf(condition, _) => InstructionForm::JumpRelativeIf(condition),
            Self::DecimalAdjustA => InstructionForm::DecimalAdjustA,
            Self::ComplementA => InstructionForm::ComplementA,
            Self::SetCarryFlag => InstructionForm::SetCarryFlag,
            Self::ComplementCarryFlag => InstructionForm::ComplementCarryFlag,
            Self::Halt => InstructionForm::Halt,
            Self::Alu8(operation, operand) => InstructionForm::Alu8(operation, operand),
            Self::ReturnIf(condition) => InstructionForm::ReturnIf(condition),
            Self::Pop(register) => InstructionForm::Pop(register),
            Self::JumpIf(condition, _) => InstructionForm::JumpIf(condition),
            Self::Jump(_) => InstructionForm::Jump,
            Self::CallIf(condition, _) => InstructionForm::CallIf(condition),
            Self::Push(register) => InstructionForm::Push(register),
            Self::AluImmediate(operation, _) => InstructionForm::AluImmediate(operation),
            Self::Restart(vector) => InstructionForm::Restart(vector),
            Self::Return => InstructionForm::Return,
            Self::ReturnFromInterrupt => InstructionForm::ReturnFromInterrupt,
            Self::Call(_) => InstructionForm::Call,
            Self::LoadHighAddressFromA(_) => InstructionForm::LoadHighAddressFromA,
            Self::LoadHighAddressCFromA => InstructionForm::LoadHighAddressCFromA,
            Self::AddSp(_) => InstructionForm::AddSp,
            Self::JumpHl => InstructionForm::JumpHl,
            Self::LoadAddressFromA(_) => InstructionForm::LoadAddressFromA,
            Self::LoadAFromHighAddress(_) => InstructionForm::LoadAFromHighAddress,
            Self::LoadAFromHighAddressC => InstructionForm::LoadAFromHighAddressC,
            Self::DisableInterrupts => InstructionForm::DisableInterrupts,
            Self::LoadHlFromSpPlus(_) => InstructionForm::LoadHlFromSpPlus,
            Self::LoadSpFromHl => InstructionForm::LoadSpFromHl,
            Self::LoadAFromAddress(_) => InstructionForm::LoadAFromAddress,
            Self::EnableInterrupts => InstructionForm::EnableInterrupts,
            Self::RotateShift(operation, operand) => {
                InstructionForm::RotateShift(operation, operand)
            }
            Self::Bit(bit, operand) => InstructionForm::Bit(bit, operand),
            Self::ResetBit(bit, operand) => InstructionForm::ResetBit(bit, operand),
            Self::SetBit(bit, operand) => InstructionForm::SetBit(bit, operand),
        }
    }

    /// Return the complete encoded length, including a CB prefix when present.
    pub const fn encoded_len(self) -> usize {
        match self {
            Self::Load16Immediate(_, _)
            | Self::LoadAddressFromSp(_)
            | Self::JumpIf(_, _)
            | Self::Jump(_)
            | Self::CallIf(_, _)
            | Self::Call(_)
            | Self::LoadAddressFromA(_)
            | Self::LoadAFromAddress(_) => 3,
            Self::Stop
            | Self::JumpRelative(_)
            | Self::JumpRelativeIf(_, _)
            | Self::Load8Immediate(_, _)
            | Self::AluImmediate(_, _)
            | Self::LoadHighAddressFromA(_)
            | Self::AddSp(_)
            | Self::LoadAFromHighAddress(_)
            | Self::LoadHlFromSpPlus(_)
            | Self::RotateShift(_, _)
            | Self::Bit(_, _)
            | Self::ResetBit(_, _)
            | Self::SetBit(_, _) => 2,
            _ => 1,
        }
    }
}
