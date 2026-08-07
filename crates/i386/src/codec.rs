use crate::{
    Address16Base, AddressSize, BinaryOperation, BitOperation, CodeMode, Condition,
    ControlRegister, CoprocessorOperation, CoprocessorRegister, DebugRegister, DescriptorOperation,
    Displacement, EffectiveAddress, FarPointer, GeneralRegister, Immediate, InstructionForm,
    LoopCondition, MemoryOperand, MemoryWidth, ModRmSelectorClass, Operand, OperandSize, Operation,
    Prefix, PrefixSequence, PrimaryOpcodeClass, Register8, Register16, Register32,
    RelativeDisplacement, Scale, ScaledIndex, SecondaryOpcodeClass, SegmentRegister,
    ShiftOperation, StringOperation, TestRegister, UnaryOperation, classify_primary_modrm_selector,
    classify_primary_opcode, classify_secondary_modrm_selector, classify_secondary_opcode,
};

/// One validated 80386 instruction with an exact selected encoding.
///
/// `code_mode` records the code-segment D-bit default. Prefix order and exact
/// displacement/immediate widths are retained so encoding is byte-stable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    code_mode: CodeMode,
    prefixes: PrefixSequence,
    operation: Operation,
    operands: Vec<Operand>,
    encoding: Encoding,
}

impl Instruction {
    /// Constructs an encodable instruction from semantic components.
    ///
    /// Invalid operation/operand combinations and invalid prefix use fail
    /// before an [`Instruction`] is returned.
    pub fn new(
        code_mode: CodeMode,
        prefixes: PrefixSequence,
        operation: Operation,
        operands: Vec<Operand>,
    ) -> Result<Self, InstructionConstructionError> {
        construct_instruction(code_mode, prefixes, operation, operands)
    }

    /// Returns the code-segment mode used to interpret default sizes.
    pub const fn code_mode(&self) -> CodeMode {
        self.code_mode
    }

    /// Returns the validated prefix sequence in encoded order.
    pub fn prefixes(&self) -> &PrefixSequence {
        &self.prefixes
    }

    /// Returns the semantic operation.
    pub const fn operation(&self) -> Operation {
        self.operation
    }

    /// Returns typed operands in syntax order.
    pub fn operands(&self) -> &[Operand] {
        &self.operands
    }

    /// Returns the stable operation and operand-kind signature.
    pub fn form(&self) -> InstructionForm {
        InstructionForm::new(self.operation, &self.operands)
    }
}

/// Failure to construct an encodable instruction form.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InstructionConstructionError {
    /// The operation, operand list, sizes, or prefixes do not name a valid form.
    #[error("operation and operands do not name an encodable Intel 80386 instruction form")]
    InvalidForm,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OpcodeEncoding {
    Primary(u8),
    Secondary(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueWidth {
    Byte,
    Word,
    Doubleword,
}

impl ValueWidth {
    const fn byte_len(self) -> usize {
        match self {
            Self::Byte => 1,
            Self::Word => 2,
            Self::Doubleword => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TailEncoding {
    None,
    Value {
        operand: usize,
        width: ValueWidth,
    },
    Values {
        first_operand: usize,
        first_width: ValueWidth,
        second_operand: usize,
        second_width: ValueWidth,
    },
    FarPointer {
        operand: usize,
    },
    MemoryOffset {
        operand: usize,
        width: ValueWidth,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Encoding {
    opcode: OpcodeEncoding,
    modrm: Option<ModRmEncoding>,
    tail: TailEncoding,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModRmEncoding {
    byte: u8,
    sib: Option<u8>,
    displacement: Displacement,
}

/// One decoded instruction together with the exact bytes it consumed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedInstruction {
    instruction: Instruction,
    source_bytes: Vec<u8>,
}

impl DecodedInstruction {
    /// Borrows the semantic instruction.
    pub const fn instruction(&self) -> &Instruction {
        &self.instruction
    }

    /// Consumes the wrapper and returns the semantic instruction.
    pub fn into_instruction(self) -> Instruction {
        self.instruction
    }

    /// Returns the exact source prefix and instruction bytes.
    pub fn source_bytes(&self) -> &[u8] {
        &self.source_bytes
    }

    /// Returns the variable instruction length in bytes.
    pub fn byte_len(&self) -> usize {
        self.source_bytes.len()
    }
}

/// Failure to decode one variable-length instruction.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    /// The byte stream ended before the selected form was complete.
    #[error("input bytes truncated at byte {offset}: needed {needed} more byte(s)")]
    Truncated {
        /// Byte offset where decoding stopped.
        offset: usize,
        /// Minimum additional byte count required.
        needed: usize,
    },
    /// Prefixes and instruction bytes exceed the architectural 15-byte limit.
    #[error("instruction exceeds the architectural 15-byte limit")]
    InstructionLengthExceeded,
    /// The primary opcode byte is undefined in the 80386 map.
    #[error("undefined Intel 80386 primary opcode {opcode:#04X}")]
    UndefinedPrimaryOpcode {
        /// Undefined primary opcode byte.
        opcode: u8,
    },
    /// The opcode byte after `0F` is undefined in the 80386 map.
    #[error("undefined Intel 80386 secondary opcode 0F {opcode:02X}")]
    UndefinedSecondaryOpcode {
        /// Undefined secondary opcode byte.
        opcode: u8,
    },
    /// A ModR/M opcode-extension selector is reserved for its group.
    #[error("reserved Intel 80386 ModR/M selector /{selector} for opcode {opcode}")]
    ReservedModRmSelector {
        /// Primary or secondary opcode label.
        opcode: String,
        /// Reserved three-bit `reg` selector.
        selector: u8,
    },
    /// A recognized prefix is invalid for the decoded operation or operands.
    #[error("instruction prefix is undefined for the decoded Intel 80386 instruction form")]
    InvalidPrefix,
    /// ModR/M, SIB, register, displacement, or immediate fields are inconsistent.
    #[error("invalid Intel 80386 operand encoding")]
    InvalidOperand,
}

/// Failure to encode a validated instruction.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EncodeError {
    /// A typed operand no longer matches the field selected during construction.
    #[error("typed operand {operand} does not match its Intel 80386 encoding field")]
    OperandMismatch {
        /// Zero-based operand position.
        operand: usize,
    },
    /// The instruction belongs to a different code-segment profile.
    #[error("instruction code mode {actual:?} does not match profile code mode {expected:?}")]
    CodeModeMismatch {
        /// Mode selected by the protocol profile.
        expected: CodeMode,
        /// Mode captured by the instruction.
        actual: CodeMode,
    },
    /// Prefixes and instruction bytes exceed the architectural 15-byte limit.
    #[error("instruction exceeds the architectural 15-byte limit")]
    InstructionLengthExceeded,
}

fn construct_instruction(
    code_mode: CodeMode,
    prefixes: PrefixSequence,
    operation: Operation,
    operands: Vec<Operand>,
) -> Result<Instruction, InstructionConstructionError> {
    let operand_size = prefixes.effective_operand_size(code_mode);
    let address_size = prefixes.effective_address_size(code_mode);
    let segment = prefixes.effective_segment();
    let invalid = || InstructionConstructionError::InvalidForm;

    let encoding = match operation {
        Operation::Binary(binary) => {
            construct_binary(binary, &operands, operand_size, address_size, segment)?
        }
        Operation::Unary(unary) => {
            construct_unary(unary, &operands, operand_size, address_size, segment)?
        }
        Operation::Shift(shift) => {
            construct_shift(shift, &operands, operand_size, address_size, segment)?
        }
        Operation::Bit(bit) => construct_bit(bit, &operands, operand_size, address_size, segment)?,
        Operation::Descriptor(descriptor) => {
            construct_descriptor(descriptor, &operands, operand_size, address_size, segment)?
        }
        Operation::Move => construct_move(&operands, operand_size, address_size, segment)?,
        Operation::MoveSignExtend | Operation::MoveZeroExtend => {
            construct_extend_move(operation, &operands, operand_size, address_size, segment)?
        }
        Operation::MultiplySigned => {
            construct_signed_multiply(&operands, operand_size, address_size, segment)?
        }
        Operation::Exchange => construct_exchange(&operands, operand_size, address_size, segment)?,
        Operation::LoadEffectiveAddress => {
            construct_lea(&operands, operand_size, address_size, segment)?
        }
        Operation::LoadPointer(pointer_segment) => construct_load_pointer(
            pointer_segment,
            &operands,
            operand_size,
            address_size,
            segment,
        )?,
        Operation::Push => construct_push(&operands, operand_size, address_size, segment)?,
        Operation::Pop => construct_pop(&operands, operand_size, address_size, segment)?,
        Operation::Bound => construct_bound(&operands, operand_size, address_size, segment)?,
        Operation::AdjustRequestedPrivilege => construct_arpl(&operands, address_size, segment)?,
        Operation::Call => {
            construct_call_or_jump(true, &operands, operand_size, address_size, segment)?
        }
        Operation::Jump => {
            construct_call_or_jump(false, &operands, operand_size, address_size, segment)?
        }
        Operation::ConditionalJump(condition) => {
            construct_conditional_jump(condition, &operands, operand_size)?
        }
        Operation::Loop(condition) => construct_loop(condition, &operands)?,
        Operation::JumpCounterZero => construct_relative_primary(0xe3, &operands)?,
        Operation::ReturnNear => construct_return(0xc2, 0xc3, &operands)?,
        Operation::ReturnFar => construct_return(0xca, 0xcb, &operands)?,
        Operation::Enter => construct_enter(&operands)?,
        Operation::Interrupt => construct_interrupt(&operands)?,
        Operation::Input | Operation::Output => {
            construct_port_io(operation, &operands, operand_size)?
        }
        Operation::String {
            operation: string,
            width,
        } => construct_string(string, width, operand_size, &operands)?,
        Operation::SetCondition(condition) => {
            construct_set_condition(condition, &operands, address_size, segment)?
        }
        Operation::AsciiAdjustAfterMultiply => construct_byte_immediate(0xd4, &operands)?,
        Operation::AsciiAdjustBeforeDivision => construct_byte_immediate(0xd5, &operands)?,
        Operation::LoadMachineStatusWord => {
            construct_status_word(false, &operands, address_size, segment)?
        }
        Operation::StoreMachineStatusWord => {
            construct_status_word(true, &operands, address_size, segment)?
        }
        Operation::CoprocessorEscape(coprocessor) => {
            construct_coprocessor(coprocessor, &operands, address_size, segment)?
        }
        _ => {
            if !operands.is_empty() {
                return Err(invalid());
            }
            let opcode = fixed_opcode(operation).ok_or_else(invalid)?;
            Encoding {
                opcode,
                modrm: None,
                tail: TailEncoding::None,
            }
        }
    };

    if !prefixes_are_valid(&prefixes, operation, &operands) {
        return Err(InstructionConstructionError::InvalidForm);
    }

    Ok(Instruction {
        code_mode,
        prefixes,
        operation,
        operands,
        encoding,
    })
}

fn primary_encoding(opcode: u8, modrm: Option<ModRmEncoding>, tail: TailEncoding) -> Encoding {
    Encoding {
        opcode: OpcodeEncoding::Primary(opcode),
        modrm,
        tail,
    }
}

fn secondary_encoding(opcode: u8, modrm: Option<ModRmEncoding>, tail: TailEncoding) -> Encoding {
    Encoding {
        opcode: OpcodeEncoding::Secondary(opcode),
        modrm,
        tail,
    }
}

fn general_register(operand: &Operand) -> Option<GeneralRegister> {
    match operand {
        Operand::GeneralRegister(register) => Some(*register),
        _ => None,
    }
}

fn immediate_width(immediate: Immediate) -> ValueWidth {
    match immediate {
        Immediate::Byte(_) | Immediate::SignExtendedByte(_) => ValueWidth::Byte,
        Immediate::Word(_) => ValueWidth::Word,
        Immediate::Doubleword(_) => ValueWidth::Doubleword,
    }
}

fn construct_modrm(
    operand: &Operand,
    reg: u8,
    memory_width: MemoryWidth,
    register_size: Option<OperandSize>,
    address_size: AddressSize,
    segment: Option<SegmentRegister>,
) -> Result<ModRmEncoding, InstructionConstructionError> {
    let invalid = || InstructionConstructionError::InvalidForm;
    if reg > 7 {
        return Err(invalid());
    }
    match operand {
        Operand::GeneralRegister(register) if register_size == Some(register.size()) => {
            Ok(ModRmEncoding {
                byte: 0xc0 | reg << 3 | register.index(),
                sib: None,
                displacement: Displacement::None,
            })
        }
        Operand::Memory(memory)
            if memory.width() == memory_width
                && memory.address().address_size() == address_size
                && memory.segment() == segment =>
        {
            construct_memory_modrm(memory.address(), reg)
        }
        _ => Err(invalid()),
    }
}

fn construct_memory_modrm(
    address: EffectiveAddress,
    reg: u8,
) -> Result<ModRmEncoding, InstructionConstructionError> {
    let invalid = || InstructionConstructionError::InvalidForm;
    match address {
        EffectiveAddress::Bits16 { base, displacement } => {
            let rm = match base {
                Address16Base::BxSi => 0,
                Address16Base::BxDi => 1,
                Address16Base::BpSi => 2,
                Address16Base::BpDi => 3,
                Address16Base::Si => 4,
                Address16Base::Di => 5,
                Address16Base::Bp | Address16Base::Direct => 6,
                Address16Base::Bx => 7,
            };
            let mode = match (base, displacement) {
                (Address16Base::Direct, Displacement::AbsoluteWord(_)) => 0,
                (Address16Base::Direct, _) => return Err(invalid()),
                (_, Displacement::None) => 0,
                (_, Displacement::Byte(_)) => 1,
                (_, Displacement::Word(_)) => 2,
                _ => return Err(invalid()),
            };
            Ok(ModRmEncoding {
                byte: mode << 6 | reg << 3 | rm,
                sib: None,
                displacement,
            })
        }
        EffectiveAddress::Bits32 {
            base,
            index,
            displacement,
            uses_sib,
        } => {
            let mode = match (base, displacement) {
                (None, Displacement::AbsoluteDoubleword(_)) => 0,
                (None, _) => return Err(invalid()),
                (Some(_), Displacement::None) => 0,
                (Some(_), Displacement::Byte(_)) => 1,
                (Some(_), Displacement::Doubleword(_)) => 2,
                _ => return Err(invalid()),
            };
            if uses_sib {
                let index_field = index.map_or(4, |index| index.register().index());
                let scale = index.map_or(Scale::One, ScaledIndex::scale);
                let base_field = base.map_or(5, Register32::index);
                Ok(ModRmEncoding {
                    byte: mode << 6 | reg << 3 | 4,
                    sib: Some(scale.field() << 6 | index_field << 3 | base_field),
                    displacement,
                })
            } else {
                if index.is_some() {
                    return Err(invalid());
                }
                let rm = base.map_or(5, Register32::index);
                Ok(ModRmEncoding {
                    byte: mode << 6 | reg << 3 | rm,
                    sib: None,
                    displacement,
                })
            }
        }
    }
}

fn binary_selector(operation: BinaryOperation) -> Option<u8> {
    Some(match operation {
        BinaryOperation::Add => 0,
        BinaryOperation::Or => 1,
        BinaryOperation::AddWithCarry => 2,
        BinaryOperation::SubtractWithBorrow => 3,
        BinaryOperation::And => 4,
        BinaryOperation::Subtract => 5,
        BinaryOperation::ExclusiveOr => 6,
        BinaryOperation::Compare => 7,
        BinaryOperation::Test => return None,
    })
}

fn construct_binary(
    operation: BinaryOperation,
    operands: &[Operand],
    operand_size: OperandSize,
    address_size: AddressSize,
    segment: Option<SegmentRegister>,
) -> Result<Encoding, InstructionConstructionError> {
    let invalid = || InstructionConstructionError::InvalidForm;
    let [left, right] = operands else {
        return Err(invalid());
    };
    if let Operand::Immediate(immediate) = right {
        let size = match left {
            Operand::GeneralRegister(register) => register.size(),
            Operand::Memory(memory) => match memory.width() {
                MemoryWidth::Byte => OperandSize::Byte,
                MemoryWidth::Word => OperandSize::Word,
                MemoryWidth::Doubleword => OperandSize::Doubleword,
                _ => return Err(invalid()),
            },
            _ => return Err(invalid()),
        };
        if size != OperandSize::Byte && size != operand_size {
            return Err(invalid());
        }
        let (opcode, selector) = if operation == BinaryOperation::Test {
            (
                if size == OperandSize::Byte {
                    0xf6
                } else {
                    0xf7
                },
                0,
            )
        } else {
            (
                match (size, immediate) {
                    (OperandSize::Byte, Immediate::Byte(_)) => 0x80,
                    (
                        OperandSize::Word | OperandSize::Doubleword,
                        Immediate::SignExtendedByte(_),
                    ) => 0x83,
                    (OperandSize::Word, Immediate::Word(_))
                    | (OperandSize::Doubleword, Immediate::Doubleword(_)) => 0x81,
                    _ => return Err(invalid()),
                },
                binary_selector(operation).ok_or_else(invalid)?,
            )
        };
        let width = immediate_width(*immediate);
        if (opcode == 0xf6 && width != ValueWidth::Byte)
            || (opcode == 0xf7 && width != value_width(size))
        {
            return Err(invalid());
        }
        let modrm = construct_modrm(
            left,
            selector,
            MemoryWidth::from(size),
            Some(size),
            address_size,
            segment,
        )?;
        return Ok(primary_encoding(
            opcode,
            Some(modrm),
            TailEncoding::Value { operand: 1, width },
        ));
    }

    let register_right = general_register(right);
    let register_left = general_register(left);
    let (rm, register, direction) = if let Some(register) = register_right {
        (left, register, 0)
    } else if let Some(register) = register_left {
        (right, register, 2)
    } else {
        return Err(invalid());
    };
    let size = register.size();
    if size != OperandSize::Byte && size != operand_size {
        return Err(invalid());
    }
    let opcode = if operation == BinaryOperation::Test {
        if direction != 0 {
            return Err(invalid());
        }
        if size == OperandSize::Byte {
            0x84
        } else {
            0x85
        }
    } else {
        let selector = binary_selector(operation).ok_or_else(invalid)?;
        selector << 3 | direction | u8::from(size != OperandSize::Byte)
    };
    let modrm = construct_modrm(
        rm,
        register.index(),
        MemoryWidth::from(size),
        Some(size),
        address_size,
        segment,
    )?;
    Ok(primary_encoding(opcode, Some(modrm), TailEncoding::None))
}

fn unary_selector(operation: UnaryOperation) -> u8 {
    match operation {
        UnaryOperation::Increment => 0,
        UnaryOperation::Decrement => 1,
        UnaryOperation::Not => 2,
        UnaryOperation::Negate => 3,
        UnaryOperation::MultiplyUnsigned => 4,
        UnaryOperation::MultiplySigned => 5,
        UnaryOperation::DivideUnsigned => 6,
        UnaryOperation::DivideSigned => 7,
    }
}

fn sized_rm_operand(operand: &Operand) -> Option<OperandSize> {
    match operand {
        Operand::GeneralRegister(register) => Some(register.size()),
        Operand::Memory(memory) => match memory.width() {
            MemoryWidth::Byte => Some(OperandSize::Byte),
            MemoryWidth::Word => Some(OperandSize::Word),
            MemoryWidth::Doubleword => Some(OperandSize::Doubleword),
            _ => None,
        },
        _ => None,
    }
}

fn construct_unary(
    operation: UnaryOperation,
    operands: &[Operand],
    operand_size: OperandSize,
    address_size: AddressSize,
    segment: Option<SegmentRegister>,
) -> Result<Encoding, InstructionConstructionError> {
    let [operand] = operands else {
        return Err(InstructionConstructionError::InvalidForm);
    };
    let size = sized_rm_operand(operand).ok_or(InstructionConstructionError::InvalidForm)?;
    if size != OperandSize::Byte && size != operand_size {
        return Err(InstructionConstructionError::InvalidForm);
    }
    let opcode = if matches!(
        operation,
        UnaryOperation::Increment | UnaryOperation::Decrement
    ) {
        if size == OperandSize::Byte {
            0xfe
        } else {
            0xff
        }
    } else if size == OperandSize::Byte {
        0xf6
    } else {
        0xf7
    };
    let modrm = construct_modrm(
        operand,
        unary_selector(operation),
        MemoryWidth::from(size),
        Some(size),
        address_size,
        segment,
    )?;
    Ok(primary_encoding(opcode, Some(modrm), TailEncoding::None))
}

fn shift_selector(operation: ShiftOperation) -> Option<u8> {
    Some(match operation {
        ShiftOperation::RotateLeft => 0,
        ShiftOperation::RotateRight => 1,
        ShiftOperation::RotateThroughCarryLeft => 2,
        ShiftOperation::RotateThroughCarryRight => 3,
        ShiftOperation::ShiftLeft => 4,
        ShiftOperation::ShiftRightLogical => 5,
        ShiftOperation::ShiftRightArithmetic => 7,
        ShiftOperation::DoubleShiftLeft | ShiftOperation::DoubleShiftRight => return None,
    })
}

fn construct_shift(
    operation: ShiftOperation,
    operands: &[Operand],
    operand_size: OperandSize,
    address_size: AddressSize,
    segment: Option<SegmentRegister>,
) -> Result<Encoding, InstructionConstructionError> {
    if matches!(
        operation,
        ShiftOperation::DoubleShiftLeft | ShiftOperation::DoubleShiftRight
    ) {
        let [destination, source, count] = operands else {
            return Err(InstructionConstructionError::InvalidForm);
        };
        let source = general_register(source).ok_or(InstructionConstructionError::InvalidForm)?;
        if source.size() != operand_size {
            return Err(InstructionConstructionError::InvalidForm);
        }
        let modrm = construct_modrm(
            destination,
            source.index(),
            MemoryWidth::from(operand_size),
            Some(operand_size),
            address_size,
            segment,
        )?;
        let base = if operation == ShiftOperation::DoubleShiftLeft {
            0xa4
        } else {
            0xac
        };
        return match count {
            Operand::Immediate(Immediate::Byte(_)) => Ok(secondary_encoding(
                base,
                Some(modrm),
                TailEncoding::Value {
                    operand: 2,
                    width: ValueWidth::Byte,
                },
            )),
            Operand::GeneralRegister(GeneralRegister::Byte(Register8::Cl)) => Ok(
                secondary_encoding(base + 1, Some(modrm), TailEncoding::None),
            ),
            _ => Err(InstructionConstructionError::InvalidForm),
        };
    }

    let [destination, count] = operands else {
        return Err(InstructionConstructionError::InvalidForm);
    };
    let size = sized_rm_operand(destination).ok_or(InstructionConstructionError::InvalidForm)?;
    if size != OperandSize::Byte && size != operand_size {
        return Err(InstructionConstructionError::InvalidForm);
    }
    let selector = shift_selector(operation).ok_or(InstructionConstructionError::InvalidForm)?;
    let (opcode_base, tail) = match count {
        Operand::Immediate(Immediate::Byte(1)) => (0xd0, TailEncoding::None),
        Operand::Immediate(Immediate::Byte(_)) => (
            0xc0,
            TailEncoding::Value {
                operand: 1,
                width: ValueWidth::Byte,
            },
        ),
        Operand::GeneralRegister(GeneralRegister::Byte(Register8::Cl)) => {
            (0xd2, TailEncoding::None)
        }
        _ => return Err(InstructionConstructionError::InvalidForm),
    };
    let opcode = opcode_base + u8::from(size != OperandSize::Byte);
    let modrm = construct_modrm(
        destination,
        selector,
        MemoryWidth::from(size),
        Some(size),
        address_size,
        segment,
    )?;
    Ok(primary_encoding(opcode, Some(modrm), tail))
}

fn construct_bit(
    operation: BitOperation,
    operands: &[Operand],
    operand_size: OperandSize,
    address_size: AddressSize,
    segment: Option<SegmentRegister>,
) -> Result<Encoding, InstructionConstructionError> {
    let invalid = || InstructionConstructionError::InvalidForm;
    let [left, right] = operands else {
        return Err(invalid());
    };
    if matches!(
        operation,
        BitOperation::ScanForward | BitOperation::ScanReverse
    ) {
        let destination = general_register(left).ok_or_else(invalid)?;
        if destination.size() != operand_size {
            return Err(invalid());
        }
        let modrm = construct_modrm(
            right,
            destination.index(),
            MemoryWidth::from(operand_size),
            Some(operand_size),
            address_size,
            segment,
        )?;
        return Ok(secondary_encoding(
            if operation == BitOperation::ScanForward {
                0xbc
            } else {
                0xbd
            },
            Some(modrm),
            TailEncoding::None,
        ));
    }

    match right {
        Operand::GeneralRegister(register) if register.size() == operand_size => {
            let opcode = match operation {
                BitOperation::Test => 0xa3,
                BitOperation::TestAndSet => 0xab,
                BitOperation::TestAndReset => 0xb3,
                BitOperation::TestAndComplement => 0xbb,
                _ => return Err(invalid()),
            };
            let modrm = construct_modrm(
                left,
                register.index(),
                MemoryWidth::from(operand_size),
                Some(operand_size),
                address_size,
                segment,
            )?;
            Ok(secondary_encoding(opcode, Some(modrm), TailEncoding::None))
        }
        Operand::Immediate(Immediate::Byte(_)) => {
            let selector = match operation {
                BitOperation::Test => 4,
                BitOperation::TestAndSet => 5,
                BitOperation::TestAndReset => 6,
                BitOperation::TestAndComplement => 7,
                _ => return Err(invalid()),
            };
            let modrm = construct_modrm(
                left,
                selector,
                MemoryWidth::from(operand_size),
                Some(operand_size),
                address_size,
                segment,
            )?;
            Ok(secondary_encoding(
                0xba,
                Some(modrm),
                TailEncoding::Value {
                    operand: 1,
                    width: ValueWidth::Byte,
                },
            ))
        }
        _ => Err(invalid()),
    }
}

fn construct_descriptor(
    operation: DescriptorOperation,
    operands: &[Operand],
    operand_size: OperandSize,
    address_size: AddressSize,
    segment: Option<SegmentRegister>,
) -> Result<Encoding, InstructionConstructionError> {
    let invalid = || InstructionConstructionError::InvalidForm;
    match operation {
        DescriptorOperation::LoadAccessRights | DescriptorOperation::LoadSegmentLimit => {
            let [destination, source] = operands else {
                return Err(invalid());
            };
            let destination = general_register(destination).ok_or_else(invalid)?;
            if destination.size() != operand_size {
                return Err(invalid());
            }
            let modrm = construct_modrm(
                source,
                destination.index(),
                MemoryWidth::Word,
                Some(OperandSize::Word),
                address_size,
                segment,
            )?;
            Ok(secondary_encoding(
                if operation == DescriptorOperation::LoadAccessRights {
                    0x02
                } else {
                    0x03
                },
                Some(modrm),
                TailEncoding::None,
            ))
        }
        DescriptorOperation::StoreGlobalDescriptorTable
        | DescriptorOperation::StoreInterruptDescriptorTable
        | DescriptorOperation::LoadGlobalDescriptorTable
        | DescriptorOperation::LoadInterruptDescriptorTable => {
            let [operand] = operands else {
                return Err(invalid());
            };
            let selector = match operation {
                DescriptorOperation::StoreGlobalDescriptorTable => 0,
                DescriptorOperation::StoreInterruptDescriptorTable => 1,
                DescriptorOperation::LoadGlobalDescriptorTable => 2,
                DescriptorOperation::LoadInterruptDescriptorTable => 3,
                _ => unreachable!(),
            };
            let modrm = construct_modrm(
                operand,
                selector,
                MemoryWidth::PseudoDescriptor,
                None,
                address_size,
                segment,
            )?;
            Ok(secondary_encoding(0x01, Some(modrm), TailEncoding::None))
        }
        _ => {
            let [operand] = operands else {
                return Err(invalid());
            };
            let selector = match operation {
                DescriptorOperation::StoreLocalDescriptorTable => 0,
                DescriptorOperation::StoreTaskRegister => 1,
                DescriptorOperation::LoadLocalDescriptorTable => 2,
                DescriptorOperation::LoadTaskRegister => 3,
                DescriptorOperation::VerifyRead => 4,
                DescriptorOperation::VerifyWrite => 5,
                _ => return Err(invalid()),
            };
            let modrm = construct_modrm(
                operand,
                selector,
                MemoryWidth::Word,
                Some(OperandSize::Word),
                address_size,
                segment,
            )?;
            Ok(secondary_encoding(0x00, Some(modrm), TailEncoding::None))
        }
    }
}

fn construct_move(
    operands: &[Operand],
    operand_size: OperandSize,
    address_size: AddressSize,
    segment: Option<SegmentRegister>,
) -> Result<Encoding, InstructionConstructionError> {
    let invalid = || InstructionConstructionError::InvalidForm;
    let [destination, source] = operands else {
        return Err(invalid());
    };

    if let Operand::Immediate(immediate) = source {
        let size = sized_rm_operand(destination).ok_or_else(invalid)?;
        if size != OperandSize::Byte && size != operand_size {
            return Err(invalid());
        }
        if immediate_width(*immediate) != value_width(size)
            || matches!(immediate, Immediate::SignExtendedByte(_))
        {
            return Err(invalid());
        }
        let modrm = construct_modrm(
            destination,
            0,
            MemoryWidth::from(size),
            Some(size),
            address_size,
            segment,
        )?;
        return Ok(primary_encoding(
            if size == OperandSize::Byte {
                0xc6
            } else {
                0xc7
            },
            Some(modrm),
            TailEncoding::Value {
                operand: 1,
                width: value_width(size),
            },
        ));
    }

    if let Operand::SegmentRegister(segment_register) = source {
        let modrm = construct_modrm(
            destination,
            segment_register.modrm_index(),
            MemoryWidth::Word,
            Some(OperandSize::Word),
            address_size,
            segment,
        )?;
        return Ok(primary_encoding(0x8c, Some(modrm), TailEncoding::None));
    }
    if let Operand::SegmentRegister(segment_register) = destination {
        if *segment_register == SegmentRegister::Cs {
            return Err(invalid());
        }
        let modrm = construct_modrm(
            source,
            segment_register.modrm_index(),
            MemoryWidth::Word,
            Some(OperandSize::Word),
            address_size,
            segment,
        )?;
        return Ok(primary_encoding(0x8e, Some(modrm), TailEncoding::None));
    }

    if let Some((opcode, special_index)) = special_register_move(destination, source) {
        let general = if matches!(destination, Operand::GeneralRegister(_)) {
            destination
        } else {
            source
        };
        let register = general_register(general).ok_or_else(invalid)?;
        if register.size() != OperandSize::Doubleword {
            return Err(invalid());
        }
        let modrm = construct_modrm(
            general,
            special_index,
            MemoryWidth::Doubleword,
            Some(OperandSize::Doubleword),
            AddressSize::Bits32,
            None,
        )?;
        return Ok(secondary_encoding(opcode, Some(modrm), TailEncoding::None));
    }

    let register_source = general_register(source);
    let register_destination = general_register(destination);
    let (rm, register, direction) = if let Some(register) = register_source {
        (destination, register, 0)
    } else if let Some(register) = register_destination {
        (source, register, 2)
    } else {
        return Err(invalid());
    };
    let size = register.size();
    if size != OperandSize::Byte && size != operand_size {
        return Err(invalid());
    }
    let modrm = construct_modrm(
        rm,
        register.index(),
        MemoryWidth::from(size),
        Some(size),
        address_size,
        segment,
    )?;
    Ok(primary_encoding(
        0x88 | direction | u8::from(size != OperandSize::Byte),
        Some(modrm),
        TailEncoding::None,
    ))
}

fn special_register_move(destination: &Operand, source: &Operand) -> Option<(u8, u8)> {
    match (destination, source) {
        (Operand::GeneralRegister(_), Operand::ControlRegister(register)) => {
            Some((0x20, register.index()))
        }
        (Operand::GeneralRegister(_), Operand::DebugRegister(register)) => {
            Some((0x21, register.index()))
        }
        (Operand::ControlRegister(register), Operand::GeneralRegister(_)) => {
            Some((0x22, register.index()))
        }
        (Operand::DebugRegister(register), Operand::GeneralRegister(_)) => {
            Some((0x23, register.index()))
        }
        (Operand::GeneralRegister(_), Operand::TestRegister(register)) => {
            Some((0x24, register.index()))
        }
        (Operand::TestRegister(register), Operand::GeneralRegister(_)) => {
            Some((0x26, register.index()))
        }
        _ => None,
    }
}

fn construct_extend_move(
    operation: Operation,
    operands: &[Operand],
    operand_size: OperandSize,
    address_size: AddressSize,
    segment: Option<SegmentRegister>,
) -> Result<Encoding, InstructionConstructionError> {
    let [destination, source] = operands else {
        return Err(InstructionConstructionError::InvalidForm);
    };
    let destination =
        general_register(destination).ok_or(InstructionConstructionError::InvalidForm)?;
    if destination.size() != operand_size {
        return Err(InstructionConstructionError::InvalidForm);
    }
    let source_size = sized_rm_operand(source).ok_or(InstructionConstructionError::InvalidForm)?;
    if !matches!(source_size, OperandSize::Byte | OperandSize::Word) {
        return Err(InstructionConstructionError::InvalidForm);
    }
    if source_size == OperandSize::Word && operand_size != OperandSize::Doubleword {
        return Err(InstructionConstructionError::InvalidForm);
    }
    let opcode = match (operation, source_size) {
        (Operation::MoveZeroExtend, OperandSize::Byte) => 0xb6,
        (Operation::MoveZeroExtend, OperandSize::Word) => 0xb7,
        (Operation::MoveSignExtend, OperandSize::Byte) => 0xbe,
        (Operation::MoveSignExtend, OperandSize::Word) => 0xbf,
        _ => return Err(InstructionConstructionError::InvalidForm),
    };
    let modrm = construct_modrm(
        source,
        destination.index(),
        MemoryWidth::from(source_size),
        Some(source_size),
        address_size,
        segment,
    )?;
    Ok(secondary_encoding(opcode, Some(modrm), TailEncoding::None))
}

fn construct_signed_multiply(
    operands: &[Operand],
    operand_size: OperandSize,
    address_size: AddressSize,
    segment: Option<SegmentRegister>,
) -> Result<Encoding, InstructionConstructionError> {
    let invalid = || InstructionConstructionError::InvalidForm;
    let (destination, source, immediate) = match operands {
        [destination, source] => (destination, source, None),
        [destination, source, Operand::Immediate(immediate)] => {
            (destination, source, Some(*immediate))
        }
        _ => return Err(invalid()),
    };
    let destination = general_register(destination).ok_or_else(invalid)?;
    if destination.size() != operand_size {
        return Err(invalid());
    }
    let modrm = construct_modrm(
        source,
        destination.index(),
        MemoryWidth::from(operand_size),
        Some(operand_size),
        address_size,
        segment,
    )?;
    match immediate {
        None => Ok(secondary_encoding(0xaf, Some(modrm), TailEncoding::None)),
        Some(Immediate::SignExtendedByte(_)) => Ok(primary_encoding(
            0x6b,
            Some(modrm),
            TailEncoding::Value {
                operand: 2,
                width: ValueWidth::Byte,
            },
        )),
        Some(Immediate::Word(_)) if operand_size == OperandSize::Word => Ok(primary_encoding(
            0x69,
            Some(modrm),
            TailEncoding::Value {
                operand: 2,
                width: ValueWidth::Word,
            },
        )),
        Some(Immediate::Doubleword(_)) if operand_size == OperandSize::Doubleword => {
            Ok(primary_encoding(
                0x69,
                Some(modrm),
                TailEncoding::Value {
                    operand: 2,
                    width: ValueWidth::Doubleword,
                },
            ))
        }
        _ => Err(invalid()),
    }
}

fn construct_exchange(
    operands: &[Operand],
    operand_size: OperandSize,
    address_size: AddressSize,
    segment: Option<SegmentRegister>,
) -> Result<Encoding, InstructionConstructionError> {
    let [left, right] = operands else {
        return Err(InstructionConstructionError::InvalidForm);
    };
    let register = general_register(right).ok_or(InstructionConstructionError::InvalidForm)?;
    let size = register.size();
    if size != OperandSize::Byte && size != operand_size {
        return Err(InstructionConstructionError::InvalidForm);
    }
    let modrm = construct_modrm(
        left,
        register.index(),
        MemoryWidth::from(size),
        Some(size),
        address_size,
        segment,
    )?;
    Ok(primary_encoding(
        if size == OperandSize::Byte {
            0x86
        } else {
            0x87
        },
        Some(modrm),
        TailEncoding::None,
    ))
}

fn construct_lea(
    operands: &[Operand],
    operand_size: OperandSize,
    address_size: AddressSize,
    segment: Option<SegmentRegister>,
) -> Result<Encoding, InstructionConstructionError> {
    let [destination, source] = operands else {
        return Err(InstructionConstructionError::InvalidForm);
    };
    let destination =
        general_register(destination).ok_or(InstructionConstructionError::InvalidForm)?;
    if destination.size() != operand_size {
        return Err(InstructionConstructionError::InvalidForm);
    }
    let modrm = construct_modrm(
        source,
        destination.index(),
        MemoryWidth::Address,
        None,
        address_size,
        segment,
    )?;
    Ok(primary_encoding(0x8d, Some(modrm), TailEncoding::None))
}

fn construct_load_pointer(
    pointer_segment: SegmentRegister,
    operands: &[Operand],
    operand_size: OperandSize,
    address_size: AddressSize,
    segment: Option<SegmentRegister>,
) -> Result<Encoding, InstructionConstructionError> {
    let [destination, source] = operands else {
        return Err(InstructionConstructionError::InvalidForm);
    };
    let destination =
        general_register(destination).ok_or(InstructionConstructionError::InvalidForm)?;
    if destination.size() != operand_size {
        return Err(InstructionConstructionError::InvalidForm);
    }
    let modrm = construct_modrm(
        source,
        destination.index(),
        far_pointer_width_for_construction(operand_size)?,
        None,
        address_size,
        segment,
    )?;
    match pointer_segment {
        SegmentRegister::Es => Ok(primary_encoding(0xc4, Some(modrm), TailEncoding::None)),
        SegmentRegister::Ds => Ok(primary_encoding(0xc5, Some(modrm), TailEncoding::None)),
        SegmentRegister::Ss => Ok(secondary_encoding(0xb2, Some(modrm), TailEncoding::None)),
        SegmentRegister::Fs => Ok(secondary_encoding(0xb4, Some(modrm), TailEncoding::None)),
        SegmentRegister::Gs => Ok(secondary_encoding(0xb5, Some(modrm), TailEncoding::None)),
        SegmentRegister::Cs => Err(InstructionConstructionError::InvalidForm),
    }
}

fn far_pointer_width_for_construction(
    operand_size: OperandSize,
) -> Result<MemoryWidth, InstructionConstructionError> {
    match operand_size {
        OperandSize::Word => Ok(MemoryWidth::FarPointerWord),
        OperandSize::Doubleword => Ok(MemoryWidth::FarPointerDoubleword),
        OperandSize::Byte => Err(InstructionConstructionError::InvalidForm),
    }
}

fn construct_push(
    operands: &[Operand],
    operand_size: OperandSize,
    address_size: AddressSize,
    segment: Option<SegmentRegister>,
) -> Result<Encoding, InstructionConstructionError> {
    let [operand] = operands else {
        return Err(InstructionConstructionError::InvalidForm);
    };
    match operand {
        Operand::SegmentRegister(segment) => {
            let opcode = match segment {
                SegmentRegister::Es => OpcodeEncoding::Primary(0x06),
                SegmentRegister::Cs => OpcodeEncoding::Primary(0x0e),
                SegmentRegister::Ss => OpcodeEncoding::Primary(0x16),
                SegmentRegister::Ds => OpcodeEncoding::Primary(0x1e),
                SegmentRegister::Fs => OpcodeEncoding::Secondary(0xa0),
                SegmentRegister::Gs => OpcodeEncoding::Secondary(0xa8),
            };
            Ok(Encoding {
                opcode,
                modrm: None,
                tail: TailEncoding::None,
            })
        }
        Operand::Immediate(Immediate::SignExtendedByte(_)) => Ok(primary_encoding(
            0x6a,
            None,
            TailEncoding::Value {
                operand: 0,
                width: ValueWidth::Byte,
            },
        )),
        Operand::Immediate(Immediate::Word(_)) if operand_size == OperandSize::Word => {
            Ok(primary_encoding(
                0x68,
                None,
                TailEncoding::Value {
                    operand: 0,
                    width: ValueWidth::Word,
                },
            ))
        }
        Operand::Immediate(Immediate::Doubleword(_)) if operand_size == OperandSize::Doubleword => {
            Ok(primary_encoding(
                0x68,
                None,
                TailEncoding::Value {
                    operand: 0,
                    width: ValueWidth::Doubleword,
                },
            ))
        }
        _ => {
            let modrm = construct_modrm(
                operand,
                6,
                MemoryWidth::from(operand_size),
                Some(operand_size),
                address_size,
                segment,
            )?;
            Ok(primary_encoding(0xff, Some(modrm), TailEncoding::None))
        }
    }
}

fn construct_pop(
    operands: &[Operand],
    operand_size: OperandSize,
    address_size: AddressSize,
    segment: Option<SegmentRegister>,
) -> Result<Encoding, InstructionConstructionError> {
    let [operand] = operands else {
        return Err(InstructionConstructionError::InvalidForm);
    };
    match operand {
        Operand::SegmentRegister(segment) => {
            let opcode = match segment {
                SegmentRegister::Es => OpcodeEncoding::Primary(0x07),
                SegmentRegister::Cs => return Err(InstructionConstructionError::InvalidForm),
                SegmentRegister::Ss => OpcodeEncoding::Primary(0x17),
                SegmentRegister::Ds => OpcodeEncoding::Primary(0x1f),
                SegmentRegister::Fs => OpcodeEncoding::Secondary(0xa1),
                SegmentRegister::Gs => OpcodeEncoding::Secondary(0xa9),
            };
            Ok(Encoding {
                opcode,
                modrm: None,
                tail: TailEncoding::None,
            })
        }
        _ => {
            let modrm = construct_modrm(
                operand,
                0,
                MemoryWidth::from(operand_size),
                Some(operand_size),
                address_size,
                segment,
            )?;
            Ok(primary_encoding(0x8f, Some(modrm), TailEncoding::None))
        }
    }
}

fn construct_bound(
    operands: &[Operand],
    operand_size: OperandSize,
    address_size: AddressSize,
    segment: Option<SegmentRegister>,
) -> Result<Encoding, InstructionConstructionError> {
    let [destination, source] = operands else {
        return Err(InstructionConstructionError::InvalidForm);
    };
    let destination =
        general_register(destination).ok_or(InstructionConstructionError::InvalidForm)?;
    if destination.size() != operand_size {
        return Err(InstructionConstructionError::InvalidForm);
    }
    let width = match operand_size {
        OperandSize::Word => MemoryWidth::BoundPairWord,
        OperandSize::Doubleword => MemoryWidth::BoundPairDoubleword,
        OperandSize::Byte => return Err(InstructionConstructionError::InvalidForm),
    };
    let modrm = construct_modrm(
        source,
        destination.index(),
        width,
        None,
        address_size,
        segment,
    )?;
    Ok(primary_encoding(0x62, Some(modrm), TailEncoding::None))
}

fn construct_arpl(
    operands: &[Operand],
    address_size: AddressSize,
    segment: Option<SegmentRegister>,
) -> Result<Encoding, InstructionConstructionError> {
    let [destination, source] = operands else {
        return Err(InstructionConstructionError::InvalidForm);
    };
    let source = general_register(source).ok_or(InstructionConstructionError::InvalidForm)?;
    if source.size() != OperandSize::Word {
        return Err(InstructionConstructionError::InvalidForm);
    }
    let modrm = construct_modrm(
        destination,
        source.index(),
        MemoryWidth::Word,
        Some(OperandSize::Word),
        address_size,
        segment,
    )?;
    Ok(primary_encoding(0x63, Some(modrm), TailEncoding::None))
}

fn construct_call_or_jump(
    call: bool,
    operands: &[Operand],
    operand_size: OperandSize,
    address_size: AddressSize,
    segment: Option<SegmentRegister>,
) -> Result<Encoding, InstructionConstructionError> {
    let [operand] = operands else {
        return Err(InstructionConstructionError::InvalidForm);
    };
    match operand {
        Operand::Relative(displacement) => {
            let width = match displacement {
                RelativeDisplacement::Byte(_) if !call => ValueWidth::Byte,
                RelativeDisplacement::Word(_) if operand_size == OperandSize::Word => {
                    ValueWidth::Word
                }
                RelativeDisplacement::Doubleword(_) if operand_size == OperandSize::Doubleword => {
                    ValueWidth::Doubleword
                }
                _ => return Err(InstructionConstructionError::InvalidForm),
            };
            Ok(primary_encoding(
                if call {
                    0xe8
                } else if width == ValueWidth::Byte {
                    0xeb
                } else {
                    0xe9
                },
                None,
                TailEncoding::Value { operand: 0, width },
            ))
        }
        Operand::FarPointer(pointer) => {
            let valid = matches!(
                (operand_size, pointer),
                (OperandSize::Word, FarPointer::Word { .. })
                    | (OperandSize::Doubleword, FarPointer::Doubleword { .. })
            );
            if !valid {
                return Err(InstructionConstructionError::InvalidForm);
            }
            Ok(primary_encoding(
                if call { 0x9a } else { 0xea },
                None,
                TailEncoding::FarPointer { operand: 0 },
            ))
        }
        _ => {
            let far = matches!(
                operand,
                Operand::Memory(memory)
                    if matches!(
                        memory.width(),
                        MemoryWidth::FarPointerWord | MemoryWidth::FarPointerDoubleword
                    )
            );
            let width = if far {
                far_pointer_width_for_construction(operand_size)?
            } else {
                MemoryWidth::from(operand_size)
            };
            let modrm = construct_modrm(
                operand,
                match (call, far) {
                    (true, false) => 2,
                    (true, true) => 3,
                    (false, false) => 4,
                    (false, true) => 5,
                },
                width,
                if far { None } else { Some(operand_size) },
                address_size,
                segment,
            )?;
            Ok(primary_encoding(0xff, Some(modrm), TailEncoding::None))
        }
    }
}

fn construct_conditional_jump(
    condition: Condition,
    operands: &[Operand],
    operand_size: OperandSize,
) -> Result<Encoding, InstructionConstructionError> {
    let [Operand::Relative(displacement)] = operands else {
        return Err(InstructionConstructionError::InvalidForm);
    };
    match displacement {
        RelativeDisplacement::Byte(_) => Ok(primary_encoding(
            0x70 | condition.field(),
            None,
            TailEncoding::Value {
                operand: 0,
                width: ValueWidth::Byte,
            },
        )),
        RelativeDisplacement::Word(_) if operand_size == OperandSize::Word => {
            Ok(secondary_encoding(
                0x80 | condition.field(),
                None,
                TailEncoding::Value {
                    operand: 0,
                    width: ValueWidth::Word,
                },
            ))
        }
        RelativeDisplacement::Doubleword(_) if operand_size == OperandSize::Doubleword => {
            Ok(secondary_encoding(
                0x80 | condition.field(),
                None,
                TailEncoding::Value {
                    operand: 0,
                    width: ValueWidth::Doubleword,
                },
            ))
        }
        _ => Err(InstructionConstructionError::InvalidForm),
    }
}

fn construct_loop(
    condition: LoopCondition,
    operands: &[Operand],
) -> Result<Encoding, InstructionConstructionError> {
    construct_relative_primary(
        match condition {
            LoopCondition::NotEqual => 0xe0,
            LoopCondition::Equal => 0xe1,
            LoopCondition::Always => 0xe2,
        },
        operands,
    )
}

fn construct_relative_primary(
    opcode: u8,
    operands: &[Operand],
) -> Result<Encoding, InstructionConstructionError> {
    if !matches!(operands, [Operand::Relative(RelativeDisplacement::Byte(_))]) {
        return Err(InstructionConstructionError::InvalidForm);
    }
    Ok(primary_encoding(
        opcode,
        None,
        TailEncoding::Value {
            operand: 0,
            width: ValueWidth::Byte,
        },
    ))
}

fn construct_return(
    immediate_opcode: u8,
    fixed_opcode: u8,
    operands: &[Operand],
) -> Result<Encoding, InstructionConstructionError> {
    match operands {
        [] => Ok(primary_encoding(fixed_opcode, None, TailEncoding::None)),
        [Operand::Immediate(Immediate::Word(_))] => Ok(primary_encoding(
            immediate_opcode,
            None,
            TailEncoding::Value {
                operand: 0,
                width: ValueWidth::Word,
            },
        )),
        _ => Err(InstructionConstructionError::InvalidForm),
    }
}

fn construct_enter(operands: &[Operand]) -> Result<Encoding, InstructionConstructionError> {
    if !matches!(
        operands,
        [
            Operand::Immediate(Immediate::Word(_)),
            Operand::Immediate(Immediate::Byte(_))
        ]
    ) {
        return Err(InstructionConstructionError::InvalidForm);
    }
    Ok(primary_encoding(
        0xc8,
        None,
        TailEncoding::Values {
            first_operand: 0,
            first_width: ValueWidth::Word,
            second_operand: 1,
            second_width: ValueWidth::Byte,
        },
    ))
}

fn construct_interrupt(operands: &[Operand]) -> Result<Encoding, InstructionConstructionError> {
    if !matches!(operands, [Operand::Immediate(Immediate::Byte(_))]) {
        return Err(InstructionConstructionError::InvalidForm);
    }
    Ok(primary_encoding(
        0xcd,
        None,
        TailEncoding::Value {
            operand: 0,
            width: ValueWidth::Byte,
        },
    ))
}

fn construct_port_io(
    operation: Operation,
    operands: &[Operand],
    operand_size: OperandSize,
) -> Result<Encoding, InstructionConstructionError> {
    let [first, second] = operands else {
        return Err(InstructionConstructionError::InvalidForm);
    };
    let (accumulator, port) = if operation == Operation::Input {
        (first, second)
    } else {
        (second, first)
    };
    let accumulator =
        general_register(accumulator).ok_or(InstructionConstructionError::InvalidForm)?;
    let size = accumulator.size();
    if accumulator.index() != 0 || (size != OperandSize::Byte && size != operand_size) {
        return Err(InstructionConstructionError::InvalidForm);
    }
    let wide = u8::from(size != OperandSize::Byte);
    match port {
        Operand::Immediate(Immediate::Byte(_)) => Ok(primary_encoding(
            if operation == Operation::Input {
                0xe4 + wide
            } else {
                0xe6 + wide
            },
            None,
            TailEncoding::Value {
                operand: if operation == Operation::Input { 1 } else { 0 },
                width: ValueWidth::Byte,
            },
        )),
        Operand::GeneralRegister(GeneralRegister::Word(Register16::Dx)) => Ok(primary_encoding(
            if operation == Operation::Input {
                0xec + wide
            } else {
                0xee + wide
            },
            None,
            TailEncoding::None,
        )),
        _ => Err(InstructionConstructionError::InvalidForm),
    }
}

fn construct_string(
    operation: StringOperation,
    width: OperandSize,
    operand_size: OperandSize,
    operands: &[Operand],
) -> Result<Encoding, InstructionConstructionError> {
    if !operands.is_empty() || (width != OperandSize::Byte && width != operand_size) {
        return Err(InstructionConstructionError::InvalidForm);
    }
    let base = match operation {
        StringOperation::Move => 0xa4,
        StringOperation::Compare => 0xa6,
        StringOperation::Store => 0xaa,
        StringOperation::Load => 0xac,
        StringOperation::Scan => 0xae,
        StringOperation::Input => 0x6c,
        StringOperation::Output => 0x6e,
    };
    Ok(primary_encoding(
        base + u8::from(width != OperandSize::Byte),
        None,
        TailEncoding::None,
    ))
}

fn construct_set_condition(
    condition: Condition,
    operands: &[Operand],
    address_size: AddressSize,
    segment: Option<SegmentRegister>,
) -> Result<Encoding, InstructionConstructionError> {
    let [destination] = operands else {
        return Err(InstructionConstructionError::InvalidForm);
    };
    let modrm = construct_modrm(
        destination,
        0,
        MemoryWidth::Byte,
        Some(OperandSize::Byte),
        address_size,
        segment,
    )?;
    Ok(secondary_encoding(
        0x90 | condition.field(),
        Some(modrm),
        TailEncoding::None,
    ))
}

fn construct_byte_immediate(
    opcode: u8,
    operands: &[Operand],
) -> Result<Encoding, InstructionConstructionError> {
    if !matches!(operands, [Operand::Immediate(Immediate::Byte(_))]) {
        return Err(InstructionConstructionError::InvalidForm);
    }
    Ok(primary_encoding(
        opcode,
        None,
        TailEncoding::Value {
            operand: 0,
            width: ValueWidth::Byte,
        },
    ))
}

fn construct_status_word(
    store: bool,
    operands: &[Operand],
    address_size: AddressSize,
    segment: Option<SegmentRegister>,
) -> Result<Encoding, InstructionConstructionError> {
    let [operand] = operands else {
        return Err(InstructionConstructionError::InvalidForm);
    };
    let modrm = construct_modrm(
        operand,
        if store { 4 } else { 6 },
        MemoryWidth::Word,
        Some(OperandSize::Word),
        address_size,
        segment,
    )?;
    Ok(secondary_encoding(0x01, Some(modrm), TailEncoding::None))
}

fn construct_coprocessor(
    operation: CoprocessorOperation,
    operands: &[Operand],
    address_size: AddressSize,
    segment: Option<SegmentRegister>,
) -> Result<Encoding, InstructionConstructionError> {
    let [operand] = operands else {
        return Err(InstructionConstructionError::InvalidForm);
    };
    let opcode = 0xd8 + (operation.selector() >> 3);
    let reg = operation.selector() & 7;
    let modrm = match operand {
        Operand::CoprocessorRegister(register) => ModRmEncoding {
            byte: 0xc0 | reg << 3 | register.index(),
            sib: None,
            displacement: Displacement::None,
        },
        _ => construct_modrm(
            operand,
            reg,
            MemoryWidth::Coprocessor,
            None,
            address_size,
            segment,
        )?,
    };
    Ok(primary_encoding(opcode, Some(modrm), TailEncoding::None))
}

fn fixed_opcode(operation: Operation) -> Option<OpcodeEncoding> {
    Some(match operation {
        Operation::PushAll => OpcodeEncoding::Primary(0x60),
        Operation::PopAll => OpcodeEncoding::Primary(0x61),
        Operation::PushFlags => OpcodeEncoding::Primary(0x9c),
        Operation::PopFlags => OpcodeEncoding::Primary(0x9d),
        Operation::ConvertByteToWordOrDoubleword => OpcodeEncoding::Primary(0x98),
        Operation::ConvertWordOrDoubleword => OpcodeEncoding::Primary(0x99),
        Operation::DecimalAdjustAfterAddition => OpcodeEncoding::Primary(0x27),
        Operation::DecimalAdjustAfterSubtraction => OpcodeEncoding::Primary(0x2f),
        Operation::AsciiAdjustAfterAddition => OpcodeEncoding::Primary(0x37),
        Operation::AsciiAdjustAfterSubtraction => OpcodeEncoding::Primary(0x3f),
        Operation::Translate => OpcodeEncoding::Primary(0xd7),
        Operation::StoreAhIntoFlags => OpcodeEncoding::Primary(0x9e),
        Operation::LoadFlagsIntoAh => OpcodeEncoding::Primary(0x9f),
        Operation::ClearCarry => OpcodeEncoding::Primary(0xf8),
        Operation::SetCarry => OpcodeEncoding::Primary(0xf9),
        Operation::ComplementCarry => OpcodeEncoding::Primary(0xf5),
        Operation::ClearDirection => OpcodeEncoding::Primary(0xfc),
        Operation::SetDirection => OpcodeEncoding::Primary(0xfd),
        Operation::ClearInterrupt => OpcodeEncoding::Primary(0xfa),
        Operation::SetInterrupt => OpcodeEncoding::Primary(0xfb),
        Operation::ClearTaskSwitched => OpcodeEncoding::Secondary(0x06),
        Operation::Halt => OpcodeEncoding::Primary(0xf4),
        Operation::NoOperation => OpcodeEncoding::Primary(0x90),
        Operation::Wait => OpcodeEncoding::Primary(0x9b),
        Operation::Leave => OpcodeEncoding::Primary(0xc9),
        Operation::InterruptOnOverflow => OpcodeEncoding::Primary(0xce),
        Operation::InterruptReturn => OpcodeEncoding::Primary(0xcf),
        _ => return None,
    })
}

/// Decodes the first 80386 instruction using `code_mode` defaults.
///
/// The returned wrapper reports the exact number of source bytes consumed.
pub fn decode_bytes(source: &[u8], code_mode: CodeMode) -> Result<DecodedInstruction, DecodeError> {
    let mut cursor = Cursor::new(source);
    let prefixes = decode_prefixes(&mut cursor)?;
    let opcode = cursor.read_u8()?;
    let (operation, operands, encoding) = match classify_primary_opcode(opcode) {
        PrimaryOpcodeClass::Instruction => {
            decode_primary_instruction(opcode, &prefixes, code_mode, &mut cursor)?
        }
        PrimaryOpcodeClass::SecondaryEscape => {
            let secondary = cursor.read_u8()?;
            if classify_secondary_opcode(secondary) == SecondaryOpcodeClass::Undefined {
                return Err(DecodeError::UndefinedSecondaryOpcode { opcode: secondary });
            }
            decode_secondary_instruction(secondary, &prefixes, code_mode, &mut cursor)?
        }
        PrimaryOpcodeClass::Undefined => {
            return Err(DecodeError::UndefinedPrimaryOpcode { opcode });
        }
        PrimaryOpcodeClass::Prefix(_) => unreachable!("prefix parser consumed every prefix byte"),
    };
    if cursor.offset > 15 {
        return Err(DecodeError::InstructionLengthExceeded);
    }
    if !prefixes_are_valid(&prefixes, operation, &operands) {
        return Err(DecodeError::InvalidPrefix);
    }
    let source_bytes = source[..cursor.offset].to_vec();
    Ok(DecodedInstruction {
        instruction: Instruction {
            code_mode,
            prefixes,
            operation,
            operands,
            encoding,
        },
        source_bytes,
    })
}

fn prefixes_are_valid(
    prefixes: &PrefixSequence,
    operation: Operation,
    operands: &[Operand],
) -> bool {
    let has_lock = prefixes.as_slice().contains(&Prefix::Lock);
    let has_repeat_not_equal = prefixes.as_slice().contains(&Prefix::RepeatNotEqual);
    let has_repeat = prefixes.as_slice().contains(&Prefix::Repeat);

    if has_lock {
        let first_is_memory = matches!(operands.first(), Some(Operand::Memory(_)));
        let lockable = match operation {
            Operation::Bit(
                BitOperation::Test
                | BitOperation::TestAndSet
                | BitOperation::TestAndReset
                | BitOperation::TestAndComplement,
            ) => first_is_memory,
            Operation::Exchange => operands
                .iter()
                .any(|operand| matches!(operand, Operand::Memory(_))),
            Operation::Binary(
                BinaryOperation::Add
                | BinaryOperation::Or
                | BinaryOperation::AddWithCarry
                | BinaryOperation::SubtractWithBorrow
                | BinaryOperation::And
                | BinaryOperation::Subtract
                | BinaryOperation::ExclusiveOr,
            ) => first_is_memory,
            Operation::Unary(
                UnaryOperation::Not
                | UnaryOperation::Negate
                | UnaryOperation::Increment
                | UnaryOperation::Decrement,
            ) => first_is_memory,
            _ => false,
        };
        if !lockable {
            return false;
        }
    }

    if has_repeat_not_equal
        && !matches!(
            operation,
            Operation::String {
                operation: StringOperation::Compare | StringOperation::Scan,
                ..
            }
        )
    {
        return false;
    }
    if has_repeat && !matches!(operation, Operation::String { .. }) {
        return false;
    }
    true
}

/// Encodes one validated instruction, preserving its selected exact form.
pub fn encode_bytes(instruction: &Instruction) -> Result<Vec<u8>, EncodeError> {
    let mut bytes = instruction
        .prefixes
        .as_slice()
        .iter()
        .copied()
        .map(prefix_byte)
        .collect::<Vec<_>>();
    match instruction.encoding.opcode {
        OpcodeEncoding::Primary(opcode) => bytes.push(opcode),
        OpcodeEncoding::Secondary(opcode) => bytes.extend([0x0f, opcode]),
    }
    if let Some(modrm) = &instruction.encoding.modrm {
        encode_modrm(modrm, &mut bytes);
    }
    encode_tail(
        &instruction.encoding.tail,
        &instruction.operands,
        &mut bytes,
    )?;
    if bytes.len() > 15 {
        return Err(EncodeError::InstructionLengthExceeded);
    }
    Ok(bytes)
}

type DecodedParts = (Operation, Vec<Operand>, Encoding);

fn decode_primary_instruction(
    opcode: u8,
    prefixes: &PrefixSequence,
    code_mode: CodeMode,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let operand_size = prefixes.effective_operand_size(code_mode);
    let address_size = prefixes.effective_address_size(code_mode);
    let variable_register = register_for_size(opcode & 7, operand_size)?;
    let accumulator = register_for_size(0, operand_size)?;

    if let Some(operation) = modrm_binary_operation(opcode) {
        let size = if opcode & 1 == 0 {
            OperandSize::Byte
        } else {
            operand_size
        };
        let parsed = decode_modrm(
            cursor,
            address_size,
            prefixes.effective_segment(),
            MemoryWidth::from(size),
            Some(size),
        )?;
        let register = Operand::GeneralRegister(register_for_size(parsed.reg, size)?);
        let operands = if opcode & 2 == 0 {
            vec![parsed.rm, register]
        } else {
            vec![register, parsed.rm]
        };
        return Ok((
            Operation::Binary(operation),
            operands,
            Encoding {
                opcode: OpcodeEncoding::Primary(opcode),
                modrm: Some(parsed.encoding),
                tail: TailEncoding::None,
            },
        ));
    }

    if matches!(opcode, 0x88..=0x8b) {
        let size = if opcode & 1 == 0 {
            OperandSize::Byte
        } else {
            operand_size
        };
        let parsed = decode_modrm(
            cursor,
            address_size,
            prefixes.effective_segment(),
            MemoryWidth::from(size),
            Some(size),
        )?;
        let register = Operand::GeneralRegister(register_for_size(parsed.reg, size)?);
        let operands = if opcode & 2 == 0 {
            vec![parsed.rm, register]
        } else {
            vec![register, parsed.rm]
        };
        return Ok((
            Operation::Move,
            operands,
            Encoding {
                opcode: OpcodeEncoding::Primary(opcode),
                modrm: Some(parsed.encoding),
                tail: TailEncoding::None,
            },
        ));
    }

    if let Some(operation) = accumulator_binary_operation(opcode) {
        let width = if opcode & 1 == 0 {
            ValueWidth::Byte
        } else {
            value_width(operand_size)
        };
        let immediate = read_immediate(cursor, width)?;
        let register = if width == ValueWidth::Byte {
            Operand::GeneralRegister(GeneralRegister::Byte(Register8::Al))
        } else {
            Operand::GeneralRegister(accumulator)
        };
        return Ok((
            Operation::Binary(operation),
            vec![register, Operand::Immediate(immediate)],
            Encoding {
                opcode: OpcodeEncoding::Primary(opcode),
                modrm: None,
                tail: TailEncoding::Value { operand: 1, width },
            },
        ));
    }

    let fixed = |operation, operands| {
        Ok((
            operation,
            operands,
            Encoding {
                opcode: OpcodeEncoding::Primary(opcode),
                modrm: None,
                tail: TailEncoding::None,
            },
        ))
    };
    match opcode {
        0x06 => fixed(
            Operation::Push,
            vec![Operand::SegmentRegister(SegmentRegister::Es)],
        ),
        0x07 => fixed(
            Operation::Pop,
            vec![Operand::SegmentRegister(SegmentRegister::Es)],
        ),
        0x0e => fixed(
            Operation::Push,
            vec![Operand::SegmentRegister(SegmentRegister::Cs)],
        ),
        0x16 => fixed(
            Operation::Push,
            vec![Operand::SegmentRegister(SegmentRegister::Ss)],
        ),
        0x17 => fixed(
            Operation::Pop,
            vec![Operand::SegmentRegister(SegmentRegister::Ss)],
        ),
        0x1e => fixed(
            Operation::Push,
            vec![Operand::SegmentRegister(SegmentRegister::Ds)],
        ),
        0x1f => fixed(
            Operation::Pop,
            vec![Operand::SegmentRegister(SegmentRegister::Ds)],
        ),
        0x27 => fixed(Operation::DecimalAdjustAfterAddition, vec![]),
        0x2f => fixed(Operation::DecimalAdjustAfterSubtraction, vec![]),
        0x37 => fixed(Operation::AsciiAdjustAfterAddition, vec![]),
        0x3f => fixed(Operation::AsciiAdjustAfterSubtraction, vec![]),
        0x40..=0x47 => fixed(
            Operation::Unary(UnaryOperation::Increment),
            vec![Operand::GeneralRegister(variable_register)],
        ),
        0x48..=0x4f => fixed(
            Operation::Unary(UnaryOperation::Decrement),
            vec![Operand::GeneralRegister(variable_register)],
        ),
        0x50..=0x57 => fixed(
            Operation::Push,
            vec![Operand::GeneralRegister(variable_register)],
        ),
        0x58..=0x5f => fixed(
            Operation::Pop,
            vec![Operand::GeneralRegister(variable_register)],
        ),
        0x60 => fixed(Operation::PushAll, vec![]),
        0x61 => fixed(Operation::PopAll, vec![]),
        0x62 => decode_bound(opcode, prefixes, operand_size, address_size, cursor),
        0x63 => decode_arpl(opcode, prefixes, address_size, cursor),
        0x68 => decode_single_immediate(opcode, Operation::Push, value_width(operand_size), cursor),
        0x69 | 0x6b => {
            decode_imultiply_immediate(opcode, prefixes, operand_size, address_size, cursor)
        }
        0x6a => {
            let immediate =
                Operand::Immediate(Immediate::SignExtendedByte(cursor.read_u8()? as i8));
            Ok((
                Operation::Push,
                vec![immediate],
                Encoding {
                    opcode: OpcodeEncoding::Primary(opcode),
                    modrm: None,
                    tail: TailEncoding::Value {
                        operand: 0,
                        width: ValueWidth::Byte,
                    },
                },
            ))
        }
        0x6c => fixed(
            Operation::String {
                operation: StringOperation::Input,
                width: OperandSize::Byte,
            },
            vec![],
        ),
        0x6d => fixed(
            Operation::String {
                operation: StringOperation::Input,
                width: operand_size,
            },
            vec![],
        ),
        0x6e => fixed(
            Operation::String {
                operation: StringOperation::Output,
                width: OperandSize::Byte,
            },
            vec![],
        ),
        0x6f => fixed(
            Operation::String {
                operation: StringOperation::Output,
                width: operand_size,
            },
            vec![],
        ),
        0x70..=0x7f => decode_relative(
            opcode,
            Operation::ConditionalJump(
                Condition::from_field(opcode & 0xf).expect("condition field is four bits"),
            ),
            ValueWidth::Byte,
            cursor,
        ),
        0x80 | 0x81 | 0x83 => {
            decode_immediate_group(opcode, prefixes, operand_size, address_size, cursor)
        }
        0x84 | 0x85 => decode_test_modrm(opcode, prefixes, operand_size, address_size, cursor),
        0x86 | 0x87 => decode_exchange_modrm(opcode, prefixes, operand_size, address_size, cursor),
        0x8c | 0x8e => decode_segment_move(opcode, prefixes, address_size, cursor),
        0x8d => decode_lea(opcode, prefixes, operand_size, address_size, cursor),
        0x8f => decode_pop_modrm(opcode, prefixes, operand_size, address_size, cursor),
        0x90 => fixed(Operation::NoOperation, vec![]),
        0x91..=0x97 => fixed(
            Operation::Exchange,
            vec![
                Operand::GeneralRegister(accumulator),
                Operand::GeneralRegister(variable_register),
            ],
        ),
        0x98 => fixed(Operation::ConvertByteToWordOrDoubleword, vec![]),
        0x99 => fixed(Operation::ConvertWordOrDoubleword, vec![]),
        0x9a => decode_far_pointer(opcode, Operation::Call, operand_size, cursor),
        0x9b => fixed(Operation::Wait, vec![]),
        0x9c => fixed(Operation::PushFlags, vec![]),
        0x9d => fixed(Operation::PopFlags, vec![]),
        0x9e => fixed(Operation::StoreAhIntoFlags, vec![]),
        0x9f => fixed(Operation::LoadFlagsIntoAh, vec![]),
        0xa0..=0xa3 => decode_memory_offset(opcode, prefixes, operand_size, address_size, cursor),
        0xa4 => fixed(
            Operation::String {
                operation: StringOperation::Move,
                width: OperandSize::Byte,
            },
            vec![],
        ),
        0xa5 => fixed(
            Operation::String {
                operation: StringOperation::Move,
                width: operand_size,
            },
            vec![],
        ),
        0xa6 => fixed(
            Operation::String {
                operation: StringOperation::Compare,
                width: OperandSize::Byte,
            },
            vec![],
        ),
        0xa7 => fixed(
            Operation::String {
                operation: StringOperation::Compare,
                width: operand_size,
            },
            vec![],
        ),
        0xa8 => decode_accumulator_test(opcode, OperandSize::Byte, cursor),
        0xa9 => decode_accumulator_test(opcode, operand_size, cursor),
        0xaa => fixed(
            Operation::String {
                operation: StringOperation::Store,
                width: OperandSize::Byte,
            },
            vec![],
        ),
        0xab => fixed(
            Operation::String {
                operation: StringOperation::Store,
                width: operand_size,
            },
            vec![],
        ),
        0xac => fixed(
            Operation::String {
                operation: StringOperation::Load,
                width: OperandSize::Byte,
            },
            vec![],
        ),
        0xad => fixed(
            Operation::String {
                operation: StringOperation::Load,
                width: operand_size,
            },
            vec![],
        ),
        0xae => fixed(
            Operation::String {
                operation: StringOperation::Scan,
                width: OperandSize::Byte,
            },
            vec![],
        ),
        0xaf => fixed(
            Operation::String {
                operation: StringOperation::Scan,
                width: operand_size,
            },
            vec![],
        ),
        0xb0..=0xb7 => decode_register_immediate(opcode, OperandSize::Byte, cursor),
        0xb8..=0xbf => decode_register_immediate(opcode, operand_size, cursor),
        0xc0 | 0xc1 => decode_shift_group(
            opcode,
            prefixes,
            operand_size,
            address_size,
            ShiftCountEncoding::Immediate,
            cursor,
        ),
        0xc2 => decode_single_immediate(opcode, Operation::ReturnNear, ValueWidth::Word, cursor),
        0xc3 => fixed(Operation::ReturnNear, vec![]),
        0xc4 => decode_load_pointer(
            opcode,
            prefixes,
            operand_size,
            address_size,
            SegmentRegister::Es,
            cursor,
        ),
        0xc5 => decode_load_pointer(
            opcode,
            prefixes,
            operand_size,
            address_size,
            SegmentRegister::Ds,
            cursor,
        ),
        0xc6 | 0xc7 => {
            decode_move_immediate_modrm(opcode, prefixes, operand_size, address_size, cursor)
        }
        0xc8 => decode_enter(opcode, cursor),
        0xc9 => fixed(Operation::Leave, vec![]),
        0xca => decode_single_immediate(opcode, Operation::ReturnFar, ValueWidth::Word, cursor),
        0xcb => fixed(Operation::ReturnFar, vec![]),
        0xcc => fixed(
            Operation::Interrupt,
            vec![Operand::Immediate(Immediate::Byte(3))],
        ),
        0xcd => decode_single_immediate(opcode, Operation::Interrupt, ValueWidth::Byte, cursor),
        0xce => fixed(Operation::InterruptOnOverflow, vec![]),
        0xcf => fixed(Operation::InterruptReturn, vec![]),
        0xd4 => decode_single_immediate(
            opcode,
            Operation::AsciiAdjustAfterMultiply,
            ValueWidth::Byte,
            cursor,
        ),
        0xd5 => decode_single_immediate(
            opcode,
            Operation::AsciiAdjustBeforeDivision,
            ValueWidth::Byte,
            cursor,
        ),
        0xd7 => fixed(Operation::Translate, vec![]),
        0xd0 | 0xd1 => decode_shift_group(
            opcode,
            prefixes,
            operand_size,
            address_size,
            ShiftCountEncoding::One,
            cursor,
        ),
        0xd2 | 0xd3 => decode_shift_group(
            opcode,
            prefixes,
            operand_size,
            address_size,
            ShiftCountEncoding::Cl,
            cursor,
        ),
        0xd8..=0xdf => decode_coprocessor_escape(opcode, prefixes, address_size, cursor),
        0xe0 => decode_relative(
            opcode,
            Operation::Loop(LoopCondition::NotEqual),
            ValueWidth::Byte,
            cursor,
        ),
        0xe1 => decode_relative(
            opcode,
            Operation::Loop(LoopCondition::Equal),
            ValueWidth::Byte,
            cursor,
        ),
        0xe2 => decode_relative(
            opcode,
            Operation::Loop(LoopCondition::Always),
            ValueWidth::Byte,
            cursor,
        ),
        0xe3 => decode_relative(opcode, Operation::JumpCounterZero, ValueWidth::Byte, cursor),
        0xe4 | 0xe5 => decode_port_immediate(opcode, Operation::Input, operand_size, cursor),
        0xe6 | 0xe7 => decode_port_immediate(opcode, Operation::Output, operand_size, cursor),
        0xe8 => decode_relative(opcode, Operation::Call, value_width(operand_size), cursor),
        0xe9 => decode_relative(opcode, Operation::Jump, value_width(operand_size), cursor),
        0xea => decode_far_pointer(opcode, Operation::Jump, operand_size, cursor),
        0xeb => decode_relative(opcode, Operation::Jump, ValueWidth::Byte, cursor),
        0xec | 0xed => {
            let accumulator = if opcode & 1 == 0 {
                GeneralRegister::Byte(Register8::Al)
            } else {
                accumulator
            };
            fixed(
                Operation::Input,
                vec![
                    Operand::GeneralRegister(accumulator),
                    Operand::GeneralRegister(GeneralRegister::Word(Register16::Dx)),
                ],
            )
        }
        0xee | 0xef => {
            let accumulator = if opcode & 1 == 0 {
                GeneralRegister::Byte(Register8::Al)
            } else {
                accumulator
            };
            fixed(
                Operation::Output,
                vec![
                    Operand::GeneralRegister(GeneralRegister::Word(Register16::Dx)),
                    Operand::GeneralRegister(accumulator),
                ],
            )
        }
        0xf4 => fixed(Operation::Halt, vec![]),
        0xf5 => fixed(Operation::ComplementCarry, vec![]),
        0xf6 | 0xf7 => decode_unary_group(opcode, prefixes, operand_size, address_size, cursor),
        0xf8 => fixed(Operation::ClearCarry, vec![]),
        0xf9 => fixed(Operation::SetCarry, vec![]),
        0xfa => fixed(Operation::ClearInterrupt, vec![]),
        0xfb => fixed(Operation::SetInterrupt, vec![]),
        0xfc => fixed(Operation::ClearDirection, vec![]),
        0xfd => fixed(Operation::SetDirection, vec![]),
        0xfe | 0xff => {
            decode_inc_dec_control_group(opcode, prefixes, operand_size, address_size, cursor)
        }
        _ => unreachable!("every defined primary opcode has an explicit decoder"),
    }
}

fn decode_secondary_instruction(
    opcode: u8,
    prefixes: &PrefixSequence,
    code_mode: CodeMode,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let operand_size = prefixes.effective_operand_size(code_mode);
    let address_size = prefixes.effective_address_size(code_mode);
    let fixed = |operation, operands| {
        Ok((
            operation,
            operands,
            Encoding {
                opcode: OpcodeEncoding::Secondary(opcode),
                modrm: None,
                tail: TailEncoding::None,
            },
        ))
    };
    match opcode {
        0x00 => decode_descriptor_group6(opcode, prefixes, address_size, cursor),
        0x01 => decode_descriptor_group7(opcode, prefixes, address_size, cursor),
        0x02 => decode_descriptor_register(
            opcode,
            prefixes,
            operand_size,
            address_size,
            DescriptorOperation::LoadAccessRights,
            cursor,
        ),
        0x03 => decode_descriptor_register(
            opcode,
            prefixes,
            operand_size,
            address_size,
            DescriptorOperation::LoadSegmentLimit,
            cursor,
        ),
        0x06 => fixed(Operation::ClearTaskSwitched, vec![]),
        0x20..=0x24 | 0x26 => decode_special_register_move(opcode, cursor),
        0x80..=0x8f => decode_secondary_relative(
            opcode,
            Operation::ConditionalJump(
                Condition::from_field(opcode & 0xf).expect("condition field is four bits"),
            ),
            value_width(operand_size),
            cursor,
        ),
        0xa0 => fixed(
            Operation::Push,
            vec![Operand::SegmentRegister(SegmentRegister::Fs)],
        ),
        0xa1 => fixed(
            Operation::Pop,
            vec![Operand::SegmentRegister(SegmentRegister::Fs)],
        ),
        0xa8 => fixed(
            Operation::Push,
            vec![Operand::SegmentRegister(SegmentRegister::Gs)],
        ),
        0xa9 => fixed(
            Operation::Pop,
            vec![Operand::SegmentRegister(SegmentRegister::Gs)],
        ),
        0x90..=0x9f => decode_set_condition(opcode, prefixes, address_size, cursor),
        0xa3 | 0xab | 0xb3 | 0xbb => {
            decode_bit_register(opcode, prefixes, operand_size, address_size, cursor)
        }
        0xa4 | 0xa5 | 0xac | 0xad => {
            decode_double_shift(opcode, prefixes, operand_size, address_size, cursor)
        }
        0xaf => decode_two_operand_imultiply(opcode, prefixes, operand_size, address_size, cursor),
        0xb2 => decode_secondary_load_pointer(
            opcode,
            prefixes,
            operand_size,
            address_size,
            SegmentRegister::Ss,
            cursor,
        ),
        0xb4 => decode_secondary_load_pointer(
            opcode,
            prefixes,
            operand_size,
            address_size,
            SegmentRegister::Fs,
            cursor,
        ),
        0xb5 => decode_secondary_load_pointer(
            opcode,
            prefixes,
            operand_size,
            address_size,
            SegmentRegister::Gs,
            cursor,
        ),
        0xb6 | 0xb7 | 0xbe | 0xbf => {
            decode_extend_move(opcode, prefixes, operand_size, address_size, cursor)
        }
        0xba => decode_bit_immediate_group(opcode, prefixes, operand_size, address_size, cursor),
        0xbc | 0xbd => decode_bit_scan(opcode, prefixes, operand_size, address_size, cursor),
        _ => unreachable!("every defined secondary opcode has an explicit decoder"),
    }
}

fn accumulator_binary_operation(opcode: u8) -> Option<BinaryOperation> {
    if opcode & 7 < 4 || opcode & 7 > 5 || opcode > 0x3d {
        return None;
    }
    Some(match opcode & 0x38 {
        0x00 => BinaryOperation::Add,
        0x08 => BinaryOperation::Or,
        0x10 => BinaryOperation::AddWithCarry,
        0x18 => BinaryOperation::SubtractWithBorrow,
        0x20 => BinaryOperation::And,
        0x28 => BinaryOperation::Subtract,
        0x30 => BinaryOperation::ExclusiveOr,
        0x38 => BinaryOperation::Compare,
        _ => return None,
    })
}

fn modrm_binary_operation(opcode: u8) -> Option<BinaryOperation> {
    if opcode & 7 > 3 || opcode > 0x3b {
        return None;
    }
    Some(match opcode & 0x38 {
        0x00 => BinaryOperation::Add,
        0x08 => BinaryOperation::Or,
        0x10 => BinaryOperation::AddWithCarry,
        0x18 => BinaryOperation::SubtractWithBorrow,
        0x20 => BinaryOperation::And,
        0x28 => BinaryOperation::Subtract,
        0x30 => BinaryOperation::ExclusiveOr,
        0x38 => BinaryOperation::Compare,
        _ => return None,
    })
}

fn primary_modrm_parts(
    opcode: u8,
    operation: Operation,
    operands: Vec<Operand>,
    parsed: ParsedModRm,
    tail: TailEncoding,
) -> Result<DecodedParts, DecodeError> {
    Ok((
        operation,
        operands,
        Encoding {
            opcode: OpcodeEncoding::Primary(opcode),
            modrm: Some(parsed.encoding),
            tail,
        },
    ))
}

fn secondary_modrm_parts(
    opcode: u8,
    operation: Operation,
    operands: Vec<Operand>,
    parsed: ParsedModRm,
    tail: TailEncoding,
) -> Result<DecodedParts, DecodeError> {
    Ok((
        operation,
        operands,
        Encoding {
            opcode: OpcodeEncoding::Secondary(opcode),
            modrm: Some(parsed.encoding),
            tail,
        },
    ))
}

fn next_modrm_selector(cursor: &Cursor<'_>) -> Result<u8, DecodeError> {
    cursor
        .peek()
        .map(|byte| (byte >> 3) & 7)
        .ok_or(DecodeError::Truncated {
            offset: cursor.offset,
            needed: 1,
        })
}

fn require_primary_selector(opcode: u8, selector: u8) -> Result<(), DecodeError> {
    if classify_primary_modrm_selector(opcode, selector) == ModRmSelectorClass::Reserved {
        return Err(DecodeError::ReservedModRmSelector {
            opcode: format!("{opcode:02X}"),
            selector,
        });
    }
    Ok(())
}

fn require_secondary_selector(opcode: u8, selector: u8) -> Result<(), DecodeError> {
    if classify_secondary_modrm_selector(opcode, selector) == ModRmSelectorClass::Reserved {
        return Err(DecodeError::ReservedModRmSelector {
            opcode: format!("0F {opcode:02X}"),
            selector,
        });
    }
    Ok(())
}

fn decode_bound(
    opcode: u8,
    prefixes: &PrefixSequence,
    operand_size: OperandSize,
    address_size: AddressSize,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let width = match operand_size {
        OperandSize::Word => MemoryWidth::BoundPairWord,
        OperandSize::Doubleword => MemoryWidth::BoundPairDoubleword,
        OperandSize::Byte => return Err(DecodeError::InvalidOperand),
    };
    let parsed = decode_modrm(
        cursor,
        address_size,
        prefixes.effective_segment(),
        width,
        None,
    )?;
    let register = Operand::GeneralRegister(register_for_size(parsed.reg, operand_size)?);
    let memory = parsed.rm.clone();
    primary_modrm_parts(
        opcode,
        Operation::Bound,
        vec![register, memory],
        parsed,
        TailEncoding::None,
    )
}

fn decode_arpl(
    opcode: u8,
    prefixes: &PrefixSequence,
    address_size: AddressSize,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let parsed = decode_modrm(
        cursor,
        address_size,
        prefixes.effective_segment(),
        MemoryWidth::Word,
        Some(OperandSize::Word),
    )?;
    let register = Operand::GeneralRegister(register_for_size(parsed.reg, OperandSize::Word)?);
    let rm = parsed.rm.clone();
    primary_modrm_parts(
        opcode,
        Operation::AdjustRequestedPrivilege,
        vec![rm, register],
        parsed,
        TailEncoding::None,
    )
}

fn decode_imultiply_immediate(
    opcode: u8,
    prefixes: &PrefixSequence,
    operand_size: OperandSize,
    address_size: AddressSize,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let parsed = decode_modrm(
        cursor,
        address_size,
        prefixes.effective_segment(),
        MemoryWidth::from(operand_size),
        Some(operand_size),
    )?;
    let register = Operand::GeneralRegister(register_for_size(parsed.reg, operand_size)?);
    let (immediate, width) = if opcode == 0x6b {
        (
            Immediate::SignExtendedByte(cursor.read_u8()? as i8),
            ValueWidth::Byte,
        )
    } else {
        let width = value_width(operand_size);
        (read_immediate(cursor, width)?, width)
    };
    let rm = parsed.rm.clone();
    primary_modrm_parts(
        opcode,
        Operation::MultiplySigned,
        vec![register, rm, Operand::Immediate(immediate)],
        parsed,
        TailEncoding::Value { operand: 2, width },
    )
}

fn group1_operation(selector: u8) -> Result<BinaryOperation, DecodeError> {
    Ok(match selector {
        0 => BinaryOperation::Add,
        1 => BinaryOperation::Or,
        2 => BinaryOperation::AddWithCarry,
        3 => BinaryOperation::SubtractWithBorrow,
        4 => BinaryOperation::And,
        5 => BinaryOperation::Subtract,
        6 => BinaryOperation::ExclusiveOr,
        7 => BinaryOperation::Compare,
        _ => return Err(DecodeError::InvalidOperand),
    })
}

fn decode_immediate_group(
    opcode: u8,
    prefixes: &PrefixSequence,
    operand_size: OperandSize,
    address_size: AddressSize,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let selector = next_modrm_selector(cursor)?;
    require_primary_selector(opcode, selector)?;
    let size = if opcode == 0x80 {
        OperandSize::Byte
    } else {
        operand_size
    };
    let parsed = decode_modrm(
        cursor,
        address_size,
        prefixes.effective_segment(),
        MemoryWidth::from(size),
        Some(size),
    )?;
    let (immediate, width) = if opcode == 0x83 {
        (
            Immediate::SignExtendedByte(cursor.read_u8()? as i8),
            ValueWidth::Byte,
        )
    } else {
        let width = value_width(size);
        (read_immediate(cursor, width)?, width)
    };
    let rm = parsed.rm.clone();
    primary_modrm_parts(
        opcode,
        Operation::Binary(group1_operation(selector)?),
        vec![rm, Operand::Immediate(immediate)],
        parsed,
        TailEncoding::Value { operand: 1, width },
    )
}

fn decode_test_modrm(
    opcode: u8,
    prefixes: &PrefixSequence,
    operand_size: OperandSize,
    address_size: AddressSize,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let size = if opcode == 0x84 {
        OperandSize::Byte
    } else {
        operand_size
    };
    let parsed = decode_modrm(
        cursor,
        address_size,
        prefixes.effective_segment(),
        MemoryWidth::from(size),
        Some(size),
    )?;
    let register = Operand::GeneralRegister(register_for_size(parsed.reg, size)?);
    let rm = parsed.rm.clone();
    primary_modrm_parts(
        opcode,
        Operation::Binary(BinaryOperation::Test),
        vec![rm, register],
        parsed,
        TailEncoding::None,
    )
}

fn decode_exchange_modrm(
    opcode: u8,
    prefixes: &PrefixSequence,
    operand_size: OperandSize,
    address_size: AddressSize,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let size = if opcode == 0x86 {
        OperandSize::Byte
    } else {
        operand_size
    };
    let parsed = decode_modrm(
        cursor,
        address_size,
        prefixes.effective_segment(),
        MemoryWidth::from(size),
        Some(size),
    )?;
    let register = Operand::GeneralRegister(register_for_size(parsed.reg, size)?);
    let rm = parsed.rm.clone();
    primary_modrm_parts(
        opcode,
        Operation::Exchange,
        vec![rm, register],
        parsed,
        TailEncoding::None,
    )
}

fn decode_segment_move(
    opcode: u8,
    prefixes: &PrefixSequence,
    address_size: AddressSize,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let parsed = decode_modrm(
        cursor,
        address_size,
        prefixes.effective_segment(),
        MemoryWidth::Word,
        Some(OperandSize::Word),
    )?;
    let segment =
        SegmentRegister::from_modrm_index(parsed.reg).ok_or(DecodeError::InvalidOperand)?;
    if opcode == 0x8e && segment == SegmentRegister::Cs {
        return Err(DecodeError::InvalidOperand);
    }
    let rm = parsed.rm.clone();
    let segment = Operand::SegmentRegister(segment);
    let operands = if opcode == 0x8c {
        vec![rm, segment]
    } else {
        vec![segment, rm]
    };
    primary_modrm_parts(
        opcode,
        Operation::Move,
        operands,
        parsed,
        TailEncoding::None,
    )
}

fn decode_lea(
    opcode: u8,
    prefixes: &PrefixSequence,
    operand_size: OperandSize,
    address_size: AddressSize,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let parsed = decode_modrm(
        cursor,
        address_size,
        prefixes.effective_segment(),
        MemoryWidth::Address,
        None,
    )?;
    let register = Operand::GeneralRegister(register_for_size(parsed.reg, operand_size)?);
    let memory = parsed.rm.clone();
    primary_modrm_parts(
        opcode,
        Operation::LoadEffectiveAddress,
        vec![register, memory],
        parsed,
        TailEncoding::None,
    )
}

fn decode_pop_modrm(
    opcode: u8,
    prefixes: &PrefixSequence,
    operand_size: OperandSize,
    address_size: AddressSize,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let selector = next_modrm_selector(cursor)?;
    require_primary_selector(opcode, selector)?;
    let parsed = decode_modrm(
        cursor,
        address_size,
        prefixes.effective_segment(),
        MemoryWidth::from(operand_size),
        Some(operand_size),
    )?;
    let rm = parsed.rm.clone();
    primary_modrm_parts(opcode, Operation::Pop, vec![rm], parsed, TailEncoding::None)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShiftCountEncoding {
    One,
    Cl,
    Immediate,
}

fn shift_operation(selector: u8) -> Result<ShiftOperation, DecodeError> {
    Ok(match selector {
        0 => ShiftOperation::RotateLeft,
        1 => ShiftOperation::RotateRight,
        2 => ShiftOperation::RotateThroughCarryLeft,
        3 => ShiftOperation::RotateThroughCarryRight,
        4 => ShiftOperation::ShiftLeft,
        5 => ShiftOperation::ShiftRightLogical,
        7 => ShiftOperation::ShiftRightArithmetic,
        _ => return Err(DecodeError::InvalidOperand),
    })
}

fn decode_shift_group(
    opcode: u8,
    prefixes: &PrefixSequence,
    operand_size: OperandSize,
    address_size: AddressSize,
    count_encoding: ShiftCountEncoding,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let selector = next_modrm_selector(cursor)?;
    require_primary_selector(opcode, selector)?;
    let size = if opcode & 1 == 0 {
        OperandSize::Byte
    } else {
        operand_size
    };
    let parsed = decode_modrm(
        cursor,
        address_size,
        prefixes.effective_segment(),
        MemoryWidth::from(size),
        Some(size),
    )?;
    let (count, tail) = match count_encoding {
        ShiftCountEncoding::One => (Operand::Immediate(Immediate::Byte(1)), TailEncoding::None),
        ShiftCountEncoding::Cl => (
            Operand::GeneralRegister(GeneralRegister::Byte(Register8::Cl)),
            TailEncoding::None,
        ),
        ShiftCountEncoding::Immediate => (
            Operand::Immediate(Immediate::Byte(cursor.read_u8()?)),
            TailEncoding::Value {
                operand: 1,
                width: ValueWidth::Byte,
            },
        ),
    };
    let rm = parsed.rm.clone();
    primary_modrm_parts(
        opcode,
        Operation::Shift(shift_operation(selector)?),
        vec![rm, count],
        parsed,
        tail,
    )
}

fn far_pointer_width(operand_size: OperandSize) -> Result<MemoryWidth, DecodeError> {
    match operand_size {
        OperandSize::Word => Ok(MemoryWidth::FarPointerWord),
        OperandSize::Doubleword => Ok(MemoryWidth::FarPointerDoubleword),
        OperandSize::Byte => Err(DecodeError::InvalidOperand),
    }
}

fn decode_load_pointer(
    opcode: u8,
    prefixes: &PrefixSequence,
    operand_size: OperandSize,
    address_size: AddressSize,
    segment: SegmentRegister,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let parsed = decode_modrm(
        cursor,
        address_size,
        prefixes.effective_segment(),
        far_pointer_width(operand_size)?,
        None,
    )?;
    let register = Operand::GeneralRegister(register_for_size(parsed.reg, operand_size)?);
    let memory = parsed.rm.clone();
    primary_modrm_parts(
        opcode,
        Operation::LoadPointer(segment),
        vec![register, memory],
        parsed,
        TailEncoding::None,
    )
}

fn decode_move_immediate_modrm(
    opcode: u8,
    prefixes: &PrefixSequence,
    operand_size: OperandSize,
    address_size: AddressSize,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let selector = next_modrm_selector(cursor)?;
    require_primary_selector(opcode, selector)?;
    let size = if opcode == 0xc6 {
        OperandSize::Byte
    } else {
        operand_size
    };
    let parsed = decode_modrm(
        cursor,
        address_size,
        prefixes.effective_segment(),
        MemoryWidth::from(size),
        Some(size),
    )?;
    let width = value_width(size);
    let immediate = Operand::Immediate(read_immediate(cursor, width)?);
    let rm = parsed.rm.clone();
    primary_modrm_parts(
        opcode,
        Operation::Move,
        vec![rm, immediate],
        parsed,
        TailEncoding::Value { operand: 1, width },
    )
}

fn decode_coprocessor_escape(
    opcode: u8,
    prefixes: &PrefixSequence,
    address_size: AddressSize,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let modrm = cursor.peek().ok_or(DecodeError::Truncated {
        offset: cursor.offset,
        needed: 1,
    })?;
    let parsed = decode_modrm(
        cursor,
        address_size,
        prefixes.effective_segment(),
        MemoryWidth::Coprocessor,
        Some(OperandSize::Word),
    )?;
    let rm = if modrm >> 6 == 3 {
        Operand::CoprocessorRegister(
            CoprocessorRegister::new(modrm & 7).ok_or(DecodeError::InvalidOperand)?,
        )
    } else {
        parsed.rm.clone()
    };
    let operation = CoprocessorOperation::new(((opcode - 0xd8) << 3) | parsed.reg)
        .ok_or(DecodeError::InvalidOperand)?;
    primary_modrm_parts(
        opcode,
        Operation::CoprocessorEscape(operation),
        vec![rm],
        parsed,
        TailEncoding::None,
    )
}

fn decode_unary_group(
    opcode: u8,
    prefixes: &PrefixSequence,
    operand_size: OperandSize,
    address_size: AddressSize,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let selector = next_modrm_selector(cursor)?;
    require_primary_selector(opcode, selector)?;
    let size = if opcode == 0xf6 {
        OperandSize::Byte
    } else {
        operand_size
    };
    let parsed = decode_modrm(
        cursor,
        address_size,
        prefixes.effective_segment(),
        MemoryWidth::from(size),
        Some(size),
    )?;
    let rm = parsed.rm.clone();
    if selector == 0 {
        let width = value_width(size);
        let immediate = Operand::Immediate(read_immediate(cursor, width)?);
        return primary_modrm_parts(
            opcode,
            Operation::Binary(BinaryOperation::Test),
            vec![rm, immediate],
            parsed,
            TailEncoding::Value { operand: 1, width },
        );
    }
    let operation = match selector {
        2 => UnaryOperation::Not,
        3 => UnaryOperation::Negate,
        4 => UnaryOperation::MultiplyUnsigned,
        5 => UnaryOperation::MultiplySigned,
        6 => UnaryOperation::DivideUnsigned,
        7 => UnaryOperation::DivideSigned,
        _ => return Err(DecodeError::InvalidOperand),
    };
    primary_modrm_parts(
        opcode,
        Operation::Unary(operation),
        vec![rm],
        parsed,
        TailEncoding::None,
    )
}

fn decode_inc_dec_control_group(
    opcode: u8,
    prefixes: &PrefixSequence,
    operand_size: OperandSize,
    address_size: AddressSize,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let selector = next_modrm_selector(cursor)?;
    require_primary_selector(opcode, selector)?;
    let size = if opcode == 0xfe {
        OperandSize::Byte
    } else {
        operand_size
    };
    let memory_width = match (opcode, selector) {
        (0xff, 3 | 5) => far_pointer_width(operand_size)?,
        _ => MemoryWidth::from(size),
    };
    let register_size = if opcode == 0xff && matches!(selector, 3 | 5) {
        None
    } else {
        Some(size)
    };
    let parsed = decode_modrm(
        cursor,
        address_size,
        prefixes.effective_segment(),
        memory_width,
        register_size,
    )?;
    let operation = match selector {
        0 => Operation::Unary(UnaryOperation::Increment),
        1 => Operation::Unary(UnaryOperation::Decrement),
        2 | 3 => Operation::Call,
        4 | 5 => Operation::Jump,
        6 => Operation::Push,
        _ => return Err(DecodeError::InvalidOperand),
    };
    let rm = parsed.rm.clone();
    primary_modrm_parts(opcode, operation, vec![rm], parsed, TailEncoding::None)
}

fn decode_descriptor_group6(
    opcode: u8,
    prefixes: &PrefixSequence,
    address_size: AddressSize,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let selector = next_modrm_selector(cursor)?;
    require_secondary_selector(opcode, selector)?;
    let parsed = decode_modrm(
        cursor,
        address_size,
        prefixes.effective_segment(),
        MemoryWidth::Word,
        Some(OperandSize::Word),
    )?;
    let operation = match selector {
        0 => DescriptorOperation::StoreLocalDescriptorTable,
        1 => DescriptorOperation::StoreTaskRegister,
        2 => DescriptorOperation::LoadLocalDescriptorTable,
        3 => DescriptorOperation::LoadTaskRegister,
        4 => DescriptorOperation::VerifyRead,
        5 => DescriptorOperation::VerifyWrite,
        _ => return Err(DecodeError::InvalidOperand),
    };
    let rm = parsed.rm.clone();
    secondary_modrm_parts(
        opcode,
        Operation::Descriptor(operation),
        vec![rm],
        parsed,
        TailEncoding::None,
    )
}

fn decode_descriptor_group7(
    opcode: u8,
    prefixes: &PrefixSequence,
    address_size: AddressSize,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let selector = next_modrm_selector(cursor)?;
    require_secondary_selector(opcode, selector)?;
    let memory_only = selector <= 3;
    let parsed = decode_modrm(
        cursor,
        address_size,
        prefixes.effective_segment(),
        if memory_only {
            MemoryWidth::PseudoDescriptor
        } else {
            MemoryWidth::Word
        },
        if memory_only {
            None
        } else {
            Some(OperandSize::Word)
        },
    )?;
    let operation = match selector {
        0 => Operation::Descriptor(DescriptorOperation::StoreGlobalDescriptorTable),
        1 => Operation::Descriptor(DescriptorOperation::StoreInterruptDescriptorTable),
        2 => Operation::Descriptor(DescriptorOperation::LoadGlobalDescriptorTable),
        3 => Operation::Descriptor(DescriptorOperation::LoadInterruptDescriptorTable),
        4 => Operation::StoreMachineStatusWord,
        6 => Operation::LoadMachineStatusWord,
        _ => return Err(DecodeError::InvalidOperand),
    };
    let rm = parsed.rm.clone();
    secondary_modrm_parts(opcode, operation, vec![rm], parsed, TailEncoding::None)
}

fn decode_descriptor_register(
    opcode: u8,
    prefixes: &PrefixSequence,
    operand_size: OperandSize,
    address_size: AddressSize,
    operation: DescriptorOperation,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let parsed = decode_modrm(
        cursor,
        address_size,
        prefixes.effective_segment(),
        MemoryWidth::Word,
        Some(OperandSize::Word),
    )?;
    let destination = Operand::GeneralRegister(register_for_size(parsed.reg, operand_size)?);
    let source = parsed.rm.clone();
    secondary_modrm_parts(
        opcode,
        Operation::Descriptor(operation),
        vec![destination, source],
        parsed,
        TailEncoding::None,
    )
}

fn decode_special_register_move(
    opcode: u8,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let raw = cursor.peek().ok_or(DecodeError::Truncated {
        offset: cursor.offset,
        needed: 1,
    })?;
    if raw >> 6 != 3 {
        return Err(DecodeError::InvalidOperand);
    }
    let parsed = decode_modrm(
        cursor,
        AddressSize::Bits32,
        None,
        MemoryWidth::Doubleword,
        Some(OperandSize::Doubleword),
    )?;
    let general = parsed.rm.clone();
    let special = match opcode {
        0x20 | 0x22 => Operand::ControlRegister(
            ControlRegister::from_index(parsed.reg).ok_or(DecodeError::InvalidOperand)?,
        ),
        0x21 | 0x23 => Operand::DebugRegister(
            DebugRegister::from_index(parsed.reg).ok_or(DecodeError::InvalidOperand)?,
        ),
        0x24 | 0x26 => Operand::TestRegister(
            TestRegister::from_index(parsed.reg).ok_or(DecodeError::InvalidOperand)?,
        ),
        _ => return Err(DecodeError::InvalidOperand),
    };
    let operands = if matches!(opcode, 0x20 | 0x21 | 0x24) {
        vec![general, special]
    } else {
        vec![special, general]
    };
    secondary_modrm_parts(
        opcode,
        Operation::Move,
        operands,
        parsed,
        TailEncoding::None,
    )
}

fn decode_set_condition(
    opcode: u8,
    prefixes: &PrefixSequence,
    address_size: AddressSize,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let parsed = decode_modrm(
        cursor,
        address_size,
        prefixes.effective_segment(),
        MemoryWidth::Byte,
        Some(OperandSize::Byte),
    )?;
    let destination = parsed.rm.clone();
    secondary_modrm_parts(
        opcode,
        Operation::SetCondition(
            Condition::from_field(opcode & 0xf).ok_or(DecodeError::InvalidOperand)?,
        ),
        vec![destination],
        parsed,
        TailEncoding::None,
    )
}

fn bit_operation_for_opcode(opcode: u8) -> Result<BitOperation, DecodeError> {
    Ok(match opcode {
        0xa3 => BitOperation::Test,
        0xab => BitOperation::TestAndSet,
        0xb3 => BitOperation::TestAndReset,
        0xbb => BitOperation::TestAndComplement,
        _ => return Err(DecodeError::InvalidOperand),
    })
}

fn decode_bit_register(
    opcode: u8,
    prefixes: &PrefixSequence,
    operand_size: OperandSize,
    address_size: AddressSize,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let parsed = decode_modrm(
        cursor,
        address_size,
        prefixes.effective_segment(),
        MemoryWidth::from(operand_size),
        Some(operand_size),
    )?;
    let bit = Operand::GeneralRegister(register_for_size(parsed.reg, operand_size)?);
    let rm = parsed.rm.clone();
    secondary_modrm_parts(
        opcode,
        Operation::Bit(bit_operation_for_opcode(opcode)?),
        vec![rm, bit],
        parsed,
        TailEncoding::None,
    )
}

fn decode_double_shift(
    opcode: u8,
    prefixes: &PrefixSequence,
    operand_size: OperandSize,
    address_size: AddressSize,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let parsed = decode_modrm(
        cursor,
        address_size,
        prefixes.effective_segment(),
        MemoryWidth::from(operand_size),
        Some(operand_size),
    )?;
    let source = Operand::GeneralRegister(register_for_size(parsed.reg, operand_size)?);
    let (count, tail) = if opcode & 1 == 0 {
        (
            Operand::Immediate(Immediate::Byte(cursor.read_u8()?)),
            TailEncoding::Value {
                operand: 2,
                width: ValueWidth::Byte,
            },
        )
    } else {
        (
            Operand::GeneralRegister(GeneralRegister::Byte(Register8::Cl)),
            TailEncoding::None,
        )
    };
    let operation = if matches!(opcode, 0xa4 | 0xa5) {
        ShiftOperation::DoubleShiftLeft
    } else {
        ShiftOperation::DoubleShiftRight
    };
    let destination = parsed.rm.clone();
    secondary_modrm_parts(
        opcode,
        Operation::Shift(operation),
        vec![destination, source, count],
        parsed,
        tail,
    )
}

fn decode_two_operand_imultiply(
    opcode: u8,
    prefixes: &PrefixSequence,
    operand_size: OperandSize,
    address_size: AddressSize,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let parsed = decode_modrm(
        cursor,
        address_size,
        prefixes.effective_segment(),
        MemoryWidth::from(operand_size),
        Some(operand_size),
    )?;
    let destination = Operand::GeneralRegister(register_for_size(parsed.reg, operand_size)?);
    let source = parsed.rm.clone();
    secondary_modrm_parts(
        opcode,
        Operation::MultiplySigned,
        vec![destination, source],
        parsed,
        TailEncoding::None,
    )
}

fn decode_secondary_load_pointer(
    opcode: u8,
    prefixes: &PrefixSequence,
    operand_size: OperandSize,
    address_size: AddressSize,
    segment: SegmentRegister,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let parsed = decode_modrm(
        cursor,
        address_size,
        prefixes.effective_segment(),
        far_pointer_width(operand_size)?,
        None,
    )?;
    let destination = Operand::GeneralRegister(register_for_size(parsed.reg, operand_size)?);
    let source = parsed.rm.clone();
    secondary_modrm_parts(
        opcode,
        Operation::LoadPointer(segment),
        vec![destination, source],
        parsed,
        TailEncoding::None,
    )
}

fn decode_extend_move(
    opcode: u8,
    prefixes: &PrefixSequence,
    operand_size: OperandSize,
    address_size: AddressSize,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let source_size = if opcode & 1 == 0 {
        OperandSize::Byte
    } else {
        OperandSize::Word
    };
    if source_size == OperandSize::Word && operand_size != OperandSize::Doubleword {
        return Err(DecodeError::InvalidOperand);
    }
    let parsed = decode_modrm(
        cursor,
        address_size,
        prefixes.effective_segment(),
        MemoryWidth::from(source_size),
        Some(source_size),
    )?;
    let destination = Operand::GeneralRegister(register_for_size(parsed.reg, operand_size)?);
    let source = parsed.rm.clone();
    secondary_modrm_parts(
        opcode,
        if opcode < 0xbe {
            Operation::MoveZeroExtend
        } else {
            Operation::MoveSignExtend
        },
        vec![destination, source],
        parsed,
        TailEncoding::None,
    )
}

fn decode_bit_immediate_group(
    opcode: u8,
    prefixes: &PrefixSequence,
    operand_size: OperandSize,
    address_size: AddressSize,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let selector = next_modrm_selector(cursor)?;
    require_secondary_selector(opcode, selector)?;
    let parsed = decode_modrm(
        cursor,
        address_size,
        prefixes.effective_segment(),
        MemoryWidth::from(operand_size),
        Some(operand_size),
    )?;
    let operation = match selector {
        4 => BitOperation::Test,
        5 => BitOperation::TestAndSet,
        6 => BitOperation::TestAndReset,
        7 => BitOperation::TestAndComplement,
        _ => return Err(DecodeError::InvalidOperand),
    };
    let bit = Operand::Immediate(Immediate::Byte(cursor.read_u8()?));
    let rm = parsed.rm.clone();
    secondary_modrm_parts(
        opcode,
        Operation::Bit(operation),
        vec![rm, bit],
        parsed,
        TailEncoding::Value {
            operand: 1,
            width: ValueWidth::Byte,
        },
    )
}

fn decode_bit_scan(
    opcode: u8,
    prefixes: &PrefixSequence,
    operand_size: OperandSize,
    address_size: AddressSize,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let parsed = decode_modrm(
        cursor,
        address_size,
        prefixes.effective_segment(),
        MemoryWidth::from(operand_size),
        Some(operand_size),
    )?;
    let destination = Operand::GeneralRegister(register_for_size(parsed.reg, operand_size)?);
    let source = parsed.rm.clone();
    secondary_modrm_parts(
        opcode,
        Operation::Bit(if opcode == 0xbc {
            BitOperation::ScanForward
        } else {
            BitOperation::ScanReverse
        }),
        vec![destination, source],
        parsed,
        TailEncoding::None,
    )
}

fn decode_single_immediate(
    opcode: u8,
    operation: Operation,
    width: ValueWidth,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let immediate = read_immediate(cursor, width)?;
    Ok((
        operation,
        vec![Operand::Immediate(immediate)],
        Encoding {
            opcode: OpcodeEncoding::Primary(opcode),
            modrm: None,
            tail: TailEncoding::Value { operand: 0, width },
        },
    ))
}

fn decode_relative(
    opcode: u8,
    operation: Operation,
    width: ValueWidth,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let displacement = read_relative(cursor, width)?;
    Ok((
        operation,
        vec![Operand::Relative(displacement)],
        Encoding {
            opcode: OpcodeEncoding::Primary(opcode),
            modrm: None,
            tail: TailEncoding::Value { operand: 0, width },
        },
    ))
}

fn decode_secondary_relative(
    opcode: u8,
    operation: Operation,
    width: ValueWidth,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let displacement = read_relative(cursor, width)?;
    Ok((
        operation,
        vec![Operand::Relative(displacement)],
        Encoding {
            opcode: OpcodeEncoding::Secondary(opcode),
            modrm: None,
            tail: TailEncoding::Value { operand: 0, width },
        },
    ))
}

fn decode_far_pointer(
    opcode: u8,
    operation: Operation,
    operand_size: OperandSize,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let pointer = match operand_size {
        OperandSize::Word => FarPointer::Word {
            offset: cursor.read_u16()?,
            segment: cursor.read_u16()?,
        },
        OperandSize::Doubleword => FarPointer::Doubleword {
            offset: cursor.read_u32()?,
            segment: cursor.read_u16()?,
        },
        OperandSize::Byte => return Err(DecodeError::InvalidOperand),
    };
    Ok((
        operation,
        vec![Operand::FarPointer(pointer)],
        Encoding {
            opcode: OpcodeEncoding::Primary(opcode),
            modrm: None,
            tail: TailEncoding::FarPointer { operand: 0 },
        },
    ))
}

fn decode_register_immediate(
    opcode: u8,
    operand_size: OperandSize,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let register = register_for_size(opcode & 7, operand_size)?;
    let width = value_width(operand_size);
    let immediate = read_immediate(cursor, width)?;
    Ok((
        Operation::Move,
        vec![
            Operand::GeneralRegister(register),
            Operand::Immediate(immediate),
        ],
        Encoding {
            opcode: OpcodeEncoding::Primary(opcode),
            modrm: None,
            tail: TailEncoding::Value { operand: 1, width },
        },
    ))
}

fn decode_accumulator_test(
    opcode: u8,
    operand_size: OperandSize,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let register = register_for_size(0, operand_size)?;
    let width = value_width(operand_size);
    let immediate = read_immediate(cursor, width)?;
    Ok((
        Operation::Binary(BinaryOperation::Test),
        vec![
            Operand::GeneralRegister(register),
            Operand::Immediate(immediate),
        ],
        Encoding {
            opcode: OpcodeEncoding::Primary(opcode),
            modrm: None,
            tail: TailEncoding::Value { operand: 1, width },
        },
    ))
}

fn decode_enter(opcode: u8, cursor: &mut Cursor<'_>) -> Result<DecodedParts, DecodeError> {
    let allocation = Operand::Immediate(Immediate::Word(cursor.read_u16()?));
    let nesting = Operand::Immediate(Immediate::Byte(cursor.read_u8()?));
    Ok((
        Operation::Enter,
        vec![allocation, nesting],
        Encoding {
            opcode: OpcodeEncoding::Primary(opcode),
            modrm: None,
            tail: TailEncoding::Values {
                first_operand: 0,
                first_width: ValueWidth::Word,
                second_operand: 1,
                second_width: ValueWidth::Byte,
            },
        },
    ))
}

fn decode_port_immediate(
    opcode: u8,
    operation: Operation,
    operand_size: OperandSize,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let port = Operand::Immediate(Immediate::Byte(cursor.read_u8()?));
    let register = if opcode & 1 == 0 {
        GeneralRegister::Byte(Register8::Al)
    } else {
        register_for_size(0, operand_size)?
    };
    let operands = if operation == Operation::Input {
        vec![Operand::GeneralRegister(register), port]
    } else {
        vec![port, Operand::GeneralRegister(register)]
    };
    Ok((
        operation,
        operands,
        Encoding {
            opcode: OpcodeEncoding::Primary(opcode),
            modrm: None,
            tail: TailEncoding::Value {
                operand: if operation == Operation::Input { 1 } else { 0 },
                width: ValueWidth::Byte,
            },
        },
    ))
}

fn decode_memory_offset(
    opcode: u8,
    prefixes: &PrefixSequence,
    operand_size: OperandSize,
    address_size: AddressSize,
    cursor: &mut Cursor<'_>,
) -> Result<DecodedParts, DecodeError> {
    let memory_width = if opcode & 1 == 0 {
        OperandSize::Byte
    } else {
        operand_size
    };
    let (address, tail_width) = match address_size {
        AddressSize::Bits16 => (
            EffectiveAddress::bits16(
                Address16Base::Direct,
                Displacement::AbsoluteWord(cursor.read_u16()?),
            )
            .map_err(|_| DecodeError::InvalidOperand)?,
            ValueWidth::Word,
        ),
        AddressSize::Bits32 => (
            EffectiveAddress::bits32(
                None,
                None,
                Displacement::AbsoluteDoubleword(cursor.read_u32()?),
                false,
            )
            .map_err(|_| DecodeError::InvalidOperand)?,
            ValueWidth::Doubleword,
        ),
    };
    let memory = Operand::Memory(MemoryOperand::new(
        MemoryWidth::from(memory_width),
        prefixes.effective_segment(),
        address,
    ));
    let register = Operand::GeneralRegister(register_for_size(0, memory_width)?);
    let operands = if opcode & 2 == 0 {
        vec![register, memory]
    } else {
        vec![memory, register]
    };
    Ok((
        Operation::Move,
        operands,
        Encoding {
            opcode: OpcodeEncoding::Primary(opcode),
            modrm: None,
            tail: TailEncoding::MemoryOffset {
                operand: if opcode & 2 == 0 { 1 } else { 0 },
                width: tail_width,
            },
        },
    ))
}

fn decode_prefixes(cursor: &mut Cursor<'_>) -> Result<PrefixSequence, DecodeError> {
    let mut prefixes = Vec::new();
    loop {
        let Some(byte) = cursor.peek() else {
            return Err(DecodeError::Truncated {
                offset: cursor.offset,
                needed: 1,
            });
        };
        let PrimaryOpcodeClass::Prefix(prefix) = classify_primary_opcode(byte) else {
            break;
        };
        if prefixes.len() == 14 {
            return Err(DecodeError::InstructionLengthExceeded);
        }
        cursor.read_u8()?;
        prefixes.push(prefix);
    }
    PrefixSequence::new(prefixes).map_err(|_| DecodeError::InstructionLengthExceeded)
}

fn prefix_byte(prefix: Prefix) -> u8 {
    match prefix {
        Prefix::Segment(SegmentRegister::Es) => 0x26,
        Prefix::Segment(SegmentRegister::Cs) => 0x2e,
        Prefix::Segment(SegmentRegister::Ss) => 0x36,
        Prefix::Segment(SegmentRegister::Ds) => 0x3e,
        Prefix::Segment(SegmentRegister::Fs) => 0x64,
        Prefix::Segment(SegmentRegister::Gs) => 0x65,
        Prefix::OperandSizeOverride => 0x66,
        Prefix::AddressSizeOverride => 0x67,
        Prefix::Lock => 0xf0,
        Prefix::RepeatNotEqual => 0xf2,
        Prefix::Repeat => 0xf3,
    }
}

fn register_for_size(index: u8, size: OperandSize) -> Result<GeneralRegister, DecodeError> {
    match size {
        OperandSize::Byte => Register8::from_index(index)
            .map(GeneralRegister::Byte)
            .ok_or(DecodeError::InvalidOperand),
        OperandSize::Word => Register16::from_index(index)
            .map(GeneralRegister::Word)
            .ok_or(DecodeError::InvalidOperand),
        OperandSize::Doubleword => Register32::from_index(index)
            .map(GeneralRegister::Doubleword)
            .ok_or(DecodeError::InvalidOperand),
    }
}

const fn value_width(size: OperandSize) -> ValueWidth {
    match size {
        OperandSize::Byte => ValueWidth::Byte,
        OperandSize::Word => ValueWidth::Word,
        OperandSize::Doubleword => ValueWidth::Doubleword,
    }
}

fn read_immediate(cursor: &mut Cursor<'_>, width: ValueWidth) -> Result<Immediate, DecodeError> {
    Ok(match width {
        ValueWidth::Byte => Immediate::Byte(cursor.read_u8()?),
        ValueWidth::Word => Immediate::Word(cursor.read_u16()?),
        ValueWidth::Doubleword => Immediate::Doubleword(cursor.read_u32()?),
    })
}

fn read_relative(
    cursor: &mut Cursor<'_>,
    width: ValueWidth,
) -> Result<RelativeDisplacement, DecodeError> {
    Ok(match width {
        ValueWidth::Byte => RelativeDisplacement::Byte(cursor.read_u8()? as i8),
        ValueWidth::Word => RelativeDisplacement::Word(cursor.read_u16()? as i16),
        ValueWidth::Doubleword => RelativeDisplacement::Doubleword(cursor.read_u32()? as i32),
    })
}

fn encode_tail(
    tail: &TailEncoding,
    operands: &[Operand],
    bytes: &mut Vec<u8>,
) -> Result<(), EncodeError> {
    match *tail {
        TailEncoding::None => Ok(()),
        TailEncoding::Value { operand, width } => {
            encode_operand_value(operands.get(operand), width, operand, bytes)
        }
        TailEncoding::Values {
            first_operand,
            first_width,
            second_operand,
            second_width,
        } => {
            encode_operand_value(
                operands.get(first_operand),
                first_width,
                first_operand,
                bytes,
            )?;
            encode_operand_value(
                operands.get(second_operand),
                second_width,
                second_operand,
                bytes,
            )
        }
        TailEncoding::FarPointer { operand } => match operands.get(operand) {
            Some(Operand::FarPointer(FarPointer::Word { offset, segment })) => {
                bytes.extend(offset.to_le_bytes());
                bytes.extend(segment.to_le_bytes());
                Ok(())
            }
            Some(Operand::FarPointer(FarPointer::Doubleword { offset, segment })) => {
                bytes.extend(offset.to_le_bytes());
                bytes.extend(segment.to_le_bytes());
                Ok(())
            }
            _ => Err(EncodeError::OperandMismatch { operand }),
        },
        TailEncoding::MemoryOffset { operand, width } => {
            let Some(Operand::Memory(memory)) = operands.get(operand) else {
                return Err(EncodeError::OperandMismatch { operand });
            };
            let displacement = match memory.address() {
                EffectiveAddress::Bits16 {
                    base: Address16Base::Direct,
                    displacement: Displacement::AbsoluteWord(value),
                } if width == ValueWidth::Word => value as u32,
                EffectiveAddress::Bits32 {
                    base: None,
                    index: None,
                    displacement: Displacement::AbsoluteDoubleword(value),
                    uses_sib: false,
                } if width == ValueWidth::Doubleword => value,
                _ => return Err(EncodeError::OperandMismatch { operand }),
            };
            encode_unsigned(displacement, width, bytes);
            Ok(())
        }
    }
}

fn encode_operand_value(
    operand: Option<&Operand>,
    width: ValueWidth,
    index: usize,
    bytes: &mut Vec<u8>,
) -> Result<(), EncodeError> {
    let value = match operand {
        Some(Operand::Immediate(Immediate::Byte(value))) if width == ValueWidth::Byte => {
            u32::from(*value)
        }
        Some(Operand::Immediate(Immediate::Word(value))) if width == ValueWidth::Word => {
            u32::from(*value)
        }
        Some(Operand::Immediate(Immediate::Doubleword(value)))
            if width == ValueWidth::Doubleword =>
        {
            *value
        }
        Some(Operand::Immediate(Immediate::SignExtendedByte(value)))
            if width == ValueWidth::Byte =>
        {
            *value as u8 as u32
        }
        Some(Operand::Relative(RelativeDisplacement::Byte(value))) if width == ValueWidth::Byte => {
            *value as u8 as u32
        }
        Some(Operand::Relative(RelativeDisplacement::Word(value))) if width == ValueWidth::Word => {
            *value as u16 as u32
        }
        Some(Operand::Relative(RelativeDisplacement::Doubleword(value)))
            if width == ValueWidth::Doubleword =>
        {
            *value as u32
        }
        _ => return Err(EncodeError::OperandMismatch { operand: index }),
    };
    encode_unsigned(value, width, bytes);
    Ok(())
}

fn encode_unsigned(value: u32, width: ValueWidth, bytes: &mut Vec<u8>) {
    let encoded = value.to_le_bytes();
    bytes.extend(&encoded[..width.byte_len()]);
}

fn encode_modrm(modrm: &ModRmEncoding, bytes: &mut Vec<u8>) {
    bytes.push(modrm.byte);
    if let Some(sib) = modrm.sib {
        bytes.push(sib);
    }
    match modrm.displacement {
        Displacement::None => {}
        Displacement::Byte(value) => bytes.push(value as u8),
        Displacement::Word(value) => bytes.extend(value.to_le_bytes()),
        Displacement::Doubleword(value) => bytes.extend(value.to_le_bytes()),
        Displacement::AbsoluteWord(value) => bytes.extend(value.to_le_bytes()),
        Displacement::AbsoluteDoubleword(value) => bytes.extend(value.to_le_bytes()),
    }
}

struct ParsedModRm {
    reg: u8,
    rm: Operand,
    encoding: ModRmEncoding,
}

fn decode_modrm(
    cursor: &mut Cursor<'_>,
    address_size: AddressSize,
    segment: Option<SegmentRegister>,
    memory_width: MemoryWidth,
    register_size: Option<OperandSize>,
) -> Result<ParsedModRm, DecodeError> {
    let byte = cursor.read_u8()?;
    let mode = byte >> 6;
    let reg = (byte >> 3) & 7;
    let rm_field = byte & 7;
    if mode == 3 {
        let size = register_size.ok_or(DecodeError::InvalidOperand)?;
        return Ok(ParsedModRm {
            reg,
            rm: Operand::GeneralRegister(register_for_size(rm_field, size)?),
            encoding: ModRmEncoding {
                byte,
                sib: None,
                displacement: Displacement::None,
            },
        });
    }

    let (address, sib, displacement) = match address_size {
        AddressSize::Bits16 => decode_address16(cursor, mode, rm_field)?,
        AddressSize::Bits32 => decode_address32(cursor, mode, rm_field)?,
    };
    Ok(ParsedModRm {
        reg,
        rm: Operand::Memory(MemoryOperand::new(memory_width, segment, address)),
        encoding: ModRmEncoding {
            byte,
            sib,
            displacement,
        },
    })
}

fn decode_address16(
    cursor: &mut Cursor<'_>,
    mode: u8,
    rm: u8,
) -> Result<(EffectiveAddress, Option<u8>, Displacement), DecodeError> {
    let (base, displacement) = if mode == 0 && rm == 6 {
        (
            Address16Base::Direct,
            Displacement::AbsoluteWord(cursor.read_u16()?),
        )
    } else {
        let base = match rm {
            0 => Address16Base::BxSi,
            1 => Address16Base::BxDi,
            2 => Address16Base::BpSi,
            3 => Address16Base::BpDi,
            4 => Address16Base::Si,
            5 => Address16Base::Di,
            6 => Address16Base::Bp,
            7 => Address16Base::Bx,
            _ => return Err(DecodeError::InvalidOperand),
        };
        let displacement = match mode {
            0 => Displacement::None,
            1 => Displacement::Byte(cursor.read_u8()? as i8),
            2 => Displacement::Word(cursor.read_u16()? as i16),
            _ => return Err(DecodeError::InvalidOperand),
        };
        (base, displacement)
    };
    let address =
        EffectiveAddress::bits16(base, displacement).map_err(|_| DecodeError::InvalidOperand)?;
    Ok((address, None, displacement))
}

fn decode_address32(
    cursor: &mut Cursor<'_>,
    mode: u8,
    rm: u8,
) -> Result<(EffectiveAddress, Option<u8>, Displacement), DecodeError> {
    if rm == 4 {
        let sib = cursor.read_u8()?;
        let scale = Scale::from_field(sib >> 6).ok_or(DecodeError::InvalidOperand)?;
        let index_field = (sib >> 3) & 7;
        let base_field = sib & 7;
        let index = if index_field == 4 {
            None
        } else {
            Some(
                ScaledIndex::new(
                    Register32::from_index(index_field).ok_or(DecodeError::InvalidOperand)?,
                    scale,
                )
                .map_err(|_| DecodeError::InvalidOperand)?,
            )
        };
        let no_base = mode == 0 && base_field == 5;
        let base = if no_base {
            None
        } else {
            Some(Register32::from_index(base_field).ok_or(DecodeError::InvalidOperand)?)
        };
        let displacement = if no_base {
            Displacement::AbsoluteDoubleword(cursor.read_u32()?)
        } else {
            match mode {
                0 => Displacement::None,
                1 => Displacement::Byte(cursor.read_u8()? as i8),
                2 => Displacement::Doubleword(cursor.read_u32()? as i32),
                _ => return Err(DecodeError::InvalidOperand),
            }
        };
        let address = EffectiveAddress::bits32(base, index, displacement, true)
            .map_err(|_| DecodeError::InvalidOperand)?;
        return Ok((address, Some(sib), displacement));
    }

    let no_base = mode == 0 && rm == 5;
    let base = if no_base {
        None
    } else {
        Some(Register32::from_index(rm).ok_or(DecodeError::InvalidOperand)?)
    };
    let displacement = if no_base {
        Displacement::AbsoluteDoubleword(cursor.read_u32()?)
    } else {
        match mode {
            0 => Displacement::None,
            1 => Displacement::Byte(cursor.read_u8()? as i8),
            2 => Displacement::Doubleword(cursor.read_u32()? as i32),
            _ => return Err(DecodeError::InvalidOperand),
        }
    };
    let address = EffectiveAddress::bits32(base, None, displacement, false)
        .map_err(|_| DecodeError::InvalidOperand)?;
    Ok((address, None, displacement))
}

struct Cursor<'a> {
    source: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    const fn new(source: &'a [u8]) -> Self {
        Self { source, offset: 0 }
    }

    fn peek(&self) -> Option<u8> {
        self.source.get(self.offset).copied()
    }

    fn read_u8(&mut self) -> Result<u8, DecodeError> {
        let value = self.peek().ok_or(DecodeError::Truncated {
            offset: self.offset,
            needed: 1,
        })?;
        self.offset += 1;
        Ok(value)
    }

    fn read_u16(&mut self) -> Result<u16, DecodeError> {
        let bytes = self.read_array::<2>()?;
        Ok(u16::from_le_bytes(bytes))
    }

    fn read_u32(&mut self) -> Result<u32, DecodeError> {
        let bytes = self.read_array::<4>()?;
        Ok(u32::from_le_bytes(bytes))
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {
        let end = self.offset.saturating_add(N);
        let source = self
            .source
            .get(self.offset..end)
            .ok_or(DecodeError::Truncated {
                offset: self.offset,
                needed: end.saturating_sub(self.source.len()),
            })?;
        self.offset = end;
        Ok(source.try_into().expect("slice length was checked"))
    }
}
