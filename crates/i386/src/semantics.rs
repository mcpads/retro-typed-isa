//! Complete static meaning shared by the Intel 80386 code-size profiles.

use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlBoundary, ControlTarget, LocationAccess,
    ReferenceKind, StaticReference, StorageRelation,
};

use crate::{
    Address16Base, AddressSize, BinaryOperation, BitOperation, CodeLocation, CodeMode, Condition,
    ControlRegister, CoprocessorOperation, DebugRegister, DescriptorOperation, Displacement,
    EffectiveAddress, EncodeError, FarPointer, GeneralRegister, I386Profile, Immediate,
    Instruction, MemoryOperand, MemoryWidth, Operand, OperandSize, Operation, Prefix, Register8,
    Register16, Register32, RelativeDisplacement, SegmentRegister, ShiftOperation, StringOperation,
    TestRegister, UnaryOperation,
};

/// Interrupt-recognition state relevant to the successor of `sti` and SS loads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InterruptState {
    /// Maskable interrupts are recognized at ordinary instruction boundaries.
    Enabled,
    /// IF is clear.
    Disabled,
    /// `sti` has set IF, but recognition starts after the following instruction.
    EnableAfterNextInstruction,
    /// Loading SS inhibits recognition through the following instruction.
    InhibitedAfterStackSegmentLoad,
    /// A FLAGS restore made the state unavailable to static analysis.
    Unknown,
}

/// Placed 80386 instruction and descriptor-derived context needed by static analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnalysisPoint {
    /// Segmented architectural instruction location.
    pub location: CodeLocation,
    /// Known interrupt-recognition state.
    pub interrupt_state: InterruptState,
    /// Stack-address size selected by the SS descriptor's B bit.
    pub stack_address_size: AddressSize,
}

impl AnalysisPoint {
    /// Constructs an 80386 static-analysis point.
    pub const fn new(
        location: CodeLocation,
        interrupt_state: InterruptState,
        stack_address_size: AddressSize,
    ) -> Self {
        Self {
            location,
            interrupt_state,
            stack_address_size,
        }
    }
}

/// Runtime-derived 80386 control destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndirectTarget {
    /// Near offset read through a register or memory operand.
    Near(Operand),
    /// Far selector and offset read from memory.
    Far(EffectiveAddress),
    /// Near return offset popped from the current stack.
    NearReturn(OperandSize),
    /// Far return selector and offset popped from the current stack.
    FarReturn(OperandSize),
    /// EFLAGS plus CS:EIP restored by `iret`.
    InterruptReturn(OperandSize),
    /// Handler selected from the interrupt descriptor table.
    InterruptVector(u8),
}

/// Synchronous boundary or wait state defined by the 80386 profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Boundary {
    /// Divide error, vector zero.
    DivideError,
    /// BOUND range exception, vector five.
    BoundRange,
    /// Software interrupt with the encoded vector.
    Interrupt(u8),
    /// Conditional overflow interrupt, vector four.
    Overflow,
    /// Halt until an enabled interrupt, NMI, or reset.
    Halt,
    /// Synchronization with the external numeric coprocessor.
    NumericProcessorWait,
}

/// Individually meaningful EFLAGS bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Flag {
    /// Carry flag.
    Carry,
    /// Parity flag.
    Parity,
    /// Auxiliary-carry flag.
    AuxiliaryCarry,
    /// Zero flag.
    Zero,
    /// Sign flag.
    Sign,
    /// Trap flag.
    Trap,
    /// Interrupt-enable flag.
    InterruptEnable,
    /// Direction flag.
    Direction,
    /// Overflow flag.
    Overflow,
    /// Nested-task flag.
    NestedTask,
    /// Resume flag.
    Resume,
    /// Virtual-8086-mode flag.
    Virtual8086,
}

/// Segment identity retained for a symbolic memory location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SegmentSelection {
    /// Runtime value of a segment register.
    Register(SegmentRegister),
    /// The current code selector known from placement.
    CurrentCode(u16),
}

/// Symbolic offset rule within a selected segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OffsetAddress {
    /// Explicit ModR/M effective address.
    Effective(EffectiveAddress),
    /// Current stack top selected by the SS descriptor.
    Stack(AddressSize),
    /// Source index selected by the instruction address size.
    StringSource(AddressSize),
    /// Destination index selected by the instruction address size.
    StringDestination(AddressSize),
    /// `BX + zero-extended AL` used by `xlat`.
    Translate,
}

/// Symbolic segmented 80386 memory storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryLocation {
    /// Segment source.
    pub segment: SegmentSelection,
    /// Offset expression.
    pub offset: OffsetAddress,
    /// Exact operand class or an instruction-defined contiguous byte extent.
    pub extent: MemoryExtent,
}

/// Extent of one statically described 80386 memory access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryExtent {
    /// Memory-object class carried by an explicit typed operand.
    Operand(MemoryWidth),
    /// Contiguous byte count established by an implicit access rule.
    KnownBytes(u8),
}

/// Stable implicit 80386 state not named by an ordinary instruction operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImplicitState {
    /// Global descriptor table register.
    GlobalDescriptorTable,
    /// Interrupt descriptor table register.
    InterruptDescriptorTable,
    /// Local descriptor table register.
    LocalDescriptorTable,
    /// Task register.
    TaskRegister,
    /// Descriptor and privilege checks used by protected-mode operations.
    DescriptorEnvironment,
    /// External numeric-coprocessor state.
    Coprocessor,
    /// Bus lock asserted by the LOCK prefix.
    BusLock,
}

/// Symbolic I/O port selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortLocation {
    /// Immediate eight-bit port number.
    Immediate(u8),
    /// Runtime port number in DX.
    DataRegister,
}

/// 80386 architectural storage or externally visible resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Location {
    /// Typed byte, word, or doubleword view of a general register.
    GeneralRegister(GeneralRegister),
    /// Segment register.
    SegmentRegister(SegmentRegister),
    /// Control register.
    ControlRegister(ControlRegister),
    /// Debug register.
    DebugRegister(DebugRegister),
    /// Test register.
    TestRegister(TestRegister),
    /// Whole EFLAGS storage.
    Flags,
    /// One EFLAGS bit.
    Flag(Flag),
    /// Symbolic segmented memory.
    Memory(MemoryLocation),
    /// Symbolic I/O port and transfer width.
    Port {
        /// Port-number rule.
        port: PortLocation,
        /// Transfer width in bytes.
        byte_len: u8,
    },
    /// Stable implicit state.
    Implicit(ImplicitState),
}

/// Statically resolved 80386 reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reference {
    /// Direct offset within a segment selected by an architectural register.
    Memory {
        /// Segment source.
        segment: SegmentSelection,
        /// Direct segment offset.
        offset: u32,
        /// Semantic width or object class.
        width: MemoryWidth,
    },
    /// Direct effective offset produced without claiming a data access.
    Offset(u32),
}

/// Failure to form sound profile-specific static facts.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SemanticError {
    /// The instruction cannot be encoded by the selected code-size profile.
    #[error(transparent)]
    Encode(#[from] EncodeError),
}

/// Shared control-flow result specialized with 80386 vocabulary.
pub type ControlFlow = typed_isa_core::ControlFlow<AnalysisPoint, IndirectTarget, Boundary>;

/// Shared instruction-semantics result specialized with 80386 vocabulary.
pub type InstructionSemantics = typed_isa_core::InstructionSemantics<
    AnalysisPoint,
    IndirectTarget,
    Boundary,
    Location,
    Reference,
>;

pub(crate) fn semantics<Profile: I386Profile>(
    instruction: &Instruction,
    at: AnalysisPoint,
) -> Result<InstructionSemantics, SemanticError> {
    if instruction.code_mode() != Profile::CODE_MODE {
        return Err(EncodeError::CodeModeMismatch {
            expected: Profile::CODE_MODE,
            actual: instruction.code_mode(),
        }
        .into());
    }
    let byte_len = crate::encode_bytes(instruction)?.len();
    let next = successor(
        instruction,
        at,
        advance(at.location, byte_len, instruction.code_mode()),
    );
    let mut accesses = Vec::new();
    let mut references = Vec::new();
    derive_accesses(instruction, at, &mut accesses, &mut references);
    Ok(InstructionSemantics {
        control_flow: control_flow(instruction, at, next),
        location_accesses: accesses,
        static_references: references,
    })
}

fn advance(location: CodeLocation, byte_len: usize, code_mode: CodeMode) -> CodeLocation {
    let offset = location.offset().wrapping_add(byte_len as u32);
    CodeLocation::new(
        location.selector(),
        if code_mode == CodeMode::Code16 {
            offset & 0xffff
        } else {
            offset
        },
    )
}

fn successor(
    instruction: &Instruction,
    mut at: AnalysisPoint,
    location: CodeLocation,
) -> AnalysisPoint {
    let mut state = match at.interrupt_state {
        InterruptState::EnableAfterNextInstruction
        | InterruptState::InhibitedAfterStackSegmentLoad => InterruptState::Enabled,
        state => state,
    };
    state = match instruction.operation() {
        Operation::ClearInterrupt => InterruptState::Disabled,
        Operation::SetInterrupt => InterruptState::EnableAfterNextInstruction,
        Operation::PopFlags | Operation::InterruptReturn => InterruptState::Unknown,
        Operation::Move | Operation::Pop
            if matches!(
                instruction.operands().first(),
                Some(Operand::SegmentRegister(SegmentRegister::Ss))
            ) =>
        {
            InterruptState::InhibitedAfterStackSegmentLoad
        }
        _ => state,
    };
    at.location = location;
    at.interrupt_state = state;
    at
}

fn control_flow(instruction: &Instruction, at: AnalysisPoint, next: AnalysisPoint) -> ControlFlow {
    let operation = instruction.operation();
    match operation {
        Operation::Call => ControlFlow {
            action: ControlAction::LinkedTransfer {
                target: control_target(
                    instruction.operands().first().unwrap(),
                    next,
                    instruction.code_mode(),
                ),
                return_site: next,
            },
            fallthrough: None,
            delay_slot: None,
        },
        Operation::Jump => ControlFlow {
            action: ControlAction::Transfer {
                target: control_target(
                    instruction.operands().first().unwrap(),
                    next,
                    instruction.code_mode(),
                ),
            },
            fallthrough: None,
            delay_slot: None,
        },
        Operation::ConditionalJump(_) | Operation::Loop(_) | Operation::JumpCounterZero => {
            ControlFlow {
                action: ControlAction::Transfer {
                    target: control_target(
                        instruction.operands().first().unwrap(),
                        next,
                        instruction.code_mode(),
                    ),
                },
                fallthrough: Some(next),
                delay_slot: None,
            }
        }
        Operation::ReturnNear => ControlFlow {
            action: ControlAction::Return {
                target: IndirectTarget::NearReturn(effective_operand_size(instruction)),
            },
            fallthrough: None,
            delay_slot: None,
        },
        Operation::ReturnFar => ControlFlow {
            action: ControlAction::Return {
                target: IndirectTarget::FarReturn(effective_operand_size(instruction)),
            },
            fallthrough: None,
            delay_slot: None,
        },
        Operation::Interrupt => trap_flow(
            Boundary::Interrupt(interrupt_vector(instruction)),
            interrupt_vector(instruction),
            next,
            next,
            false,
        ),
        Operation::InterruptOnOverflow => trap_flow(Boundary::Overflow, 4, next, next, true),
        Operation::InterruptReturn => ControlFlow {
            action: ControlAction::ExceptionReturn {
                target: IndirectTarget::InterruptReturn(effective_operand_size(instruction)),
            },
            fallthrough: None,
            delay_slot: None,
        },
        Operation::Bound => trap_flow(Boundary::BoundRange, 5, at, next, true),
        Operation::Unary(UnaryOperation::DivideUnsigned | UnaryOperation::DivideSigned) => {
            trap_flow(Boundary::DivideError, 0, at, next, true)
        }
        Operation::AsciiAdjustAfterMultiply if immediate_byte(instruction) == Some(0) => {
            trap_flow(Boundary::DivideError, 0, at, next, false)
        }
        Operation::Halt => ControlFlow {
            action: ControlAction::Boundary(ControlBoundary::Wait {
                kind: Boundary::Halt,
                resume: next,
            }),
            fallthrough: None,
            delay_slot: None,
        },
        Operation::Wait => ControlFlow {
            action: ControlAction::Boundary(ControlBoundary::Wait {
                kind: Boundary::NumericProcessorWait,
                resume: next,
            }),
            fallthrough: None,
            delay_slot: None,
        },
        _ => ControlFlow {
            action: ControlAction::Continue,
            fallthrough: Some(next),
            delay_slot: None,
        },
    }
}

fn trap_flow(
    kind: Boundary,
    vector: u8,
    resume: AnalysisPoint,
    next: AnalysisPoint,
    conditional: bool,
) -> ControlFlow {
    ControlFlow {
        action: ControlAction::Boundary(ControlBoundary::Trap {
            kind,
            target: Some(ControlTarget::Indirect(IndirectTarget::InterruptVector(
                vector,
            ))),
            resume: Some(resume),
        }),
        fallthrough: conditional.then_some(next),
        delay_slot: None,
    }
}

fn control_target(
    operand: &Operand,
    next: AnalysisPoint,
    code_mode: CodeMode,
) -> ControlTarget<AnalysisPoint, IndirectTarget> {
    match operand {
        Operand::Relative(displacement) => ControlTarget::Direct(AnalysisPoint {
            location: relative_target(next.location, *displacement, code_mode),
            ..next
        }),
        Operand::FarPointer(FarPointer::Word { offset, segment }) => {
            ControlTarget::Direct(AnalysisPoint {
                location: CodeLocation::new(*segment, u32::from(*offset)),
                ..next
            })
        }
        Operand::FarPointer(FarPointer::Doubleword { offset, segment }) => {
            ControlTarget::Direct(AnalysisPoint {
                location: CodeLocation::new(*segment, *offset),
                ..next
            })
        }
        Operand::Memory(memory)
            if matches!(
                memory.width(),
                MemoryWidth::FarPointerWord | MemoryWidth::FarPointerDoubleword
            ) =>
        {
            ControlTarget::Indirect(IndirectTarget::Far(memory.address()))
        }
        operand => ControlTarget::Indirect(IndirectTarget::Near(operand.clone())),
    }
}

fn relative_target(
    next: CodeLocation,
    displacement: RelativeDisplacement,
    code_mode: CodeMode,
) -> CodeLocation {
    let signed = match displacement {
        RelativeDisplacement::Byte(value) => i32::from(value),
        RelativeDisplacement::Word(value) => i32::from(value),
        RelativeDisplacement::Doubleword(value) => value,
    };
    let offset = next.offset().wrapping_add_signed(signed);
    let offset = if matches!(displacement, RelativeDisplacement::Word(_))
        || matches!(displacement, RelativeDisplacement::Byte(_)) && code_mode == CodeMode::Code16
    {
        offset & 0xffff
    } else {
        offset
    };
    CodeLocation::new(next.selector(), offset)
}

fn derive_accesses(
    instruction: &Instruction,
    at: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    if instruction.prefixes().as_slice().contains(&Prefix::Lock) {
        write(
            accesses,
            Location::Implicit(ImplicitState::BusLock),
            AccessOccurrence::Always,
        );
    }
    match instruction.operation() {
        Operation::Binary(operation) => {
            binary_accesses(instruction, operation, at, accesses, references)
        }
        Operation::Unary(operation) => {
            unary_accesses(instruction, operation, at, accesses, references)
        }
        Operation::Shift(operation) => {
            shift_accesses(instruction, operation, at, accesses, references)
        }
        Operation::Bit(operation) => bit_accesses(instruction, operation, at, accesses, references),
        Operation::Descriptor(operation) => {
            descriptor_accesses(instruction, operation, at, accesses, references)
        }
        Operation::Move | Operation::MoveSignExtend | Operation::MoveZeroExtend => {
            read_operand(
                &instruction.operands()[1],
                at,
                AccessOccurrence::Always,
                accesses,
                references,
            );
            write_operand(
                &instruction.operands()[0],
                at,
                AccessOccurrence::Always,
                accesses,
                references,
            );
        }
        Operation::MultiplySigned => {
            for operand in &instruction.operands()[1..] {
                read_operand(operand, at, AccessOccurrence::Always, accesses, references);
            }
            write_operand(
                &instruction.operands()[0],
                at,
                AccessOccurrence::Always,
                accesses,
                references,
            );
            write_arithmetic_flags(accesses, AccessOccurrence::Always);
        }
        Operation::Exchange => {
            for operand in instruction.operands() {
                read_write_operand(operand, at, AccessOccurrence::Always, accesses, references);
            }
        }
        Operation::LoadEffectiveAddress => {
            let Operand::Memory(memory) = instruction.operands()[1] else {
                unreachable!()
            };
            address_inputs(memory, false, AccessOccurrence::Always, accesses);
            add_direct_reference(
                memory,
                at,
                ReferenceKind::Address,
                AccessOccurrence::Always,
                references,
            );
            write_operand(
                &instruction.operands()[0],
                at,
                AccessOccurrence::Always,
                accesses,
                references,
            );
        }
        Operation::LoadPointer(segment) => {
            read_operand(
                &instruction.operands()[1],
                at,
                AccessOccurrence::Always,
                accesses,
                references,
            );
            write_operand(
                &instruction.operands()[0],
                at,
                AccessOccurrence::Always,
                accesses,
                references,
            );
            write(
                accesses,
                Location::SegmentRegister(segment),
                AccessOccurrence::Always,
            );
        }
        Operation::Push => {
            read_operand(
                &instruction.operands()[0],
                at,
                AccessOccurrence::Always,
                accesses,
                references,
            );
            push_stack(
                at,
                effective_operand_size(instruction),
                AccessOccurrence::Always,
                accesses,
            );
        }
        Operation::Pop => {
            pop_stack(
                at,
                effective_operand_size(instruction),
                AccessOccurrence::Always,
                accesses,
            );
            write_operand(
                &instruction.operands()[0],
                at,
                AccessOccurrence::Always,
                accesses,
                references,
            );
        }
        Operation::PushAll => push_all(instruction, at, accesses),
        Operation::PopAll => pop_all(instruction, at, accesses),
        Operation::PushFlags => {
            read(accesses, Location::Flags, AccessOccurrence::Always);
            push_stack(
                at,
                effective_operand_size(instruction),
                AccessOccurrence::Always,
                accesses,
            );
        }
        Operation::PopFlags => {
            pop_stack(
                at,
                effective_operand_size(instruction),
                AccessOccurrence::Always,
                accesses,
            );
            write(accesses, Location::Flags, AccessOccurrence::Always);
        }
        Operation::Bound => {
            read_operand(
                &instruction.operands()[0],
                at,
                AccessOccurrence::Always,
                accesses,
                references,
            );
            read_operand(
                &instruction.operands()[1],
                at,
                AccessOccurrence::Always,
                accesses,
                references,
            );
            exception_entry(AccessOccurrence::Conditional, accesses);
            push_interrupt_frame(instruction, at, AccessOccurrence::Conditional, accesses);
        }
        Operation::AdjustRequestedPrivilege => {
            read_write_operand(
                &instruction.operands()[0],
                at,
                AccessOccurrence::Always,
                accesses,
                references,
            );
            read_operand(
                &instruction.operands()[1],
                at,
                AccessOccurrence::Always,
                accesses,
                references,
            );
            write_flag(accesses, Flag::Zero, AccessOccurrence::Always);
        }
        Operation::Call => {
            read_control_operand(
                instruction.operands().first().unwrap(),
                at,
                accesses,
                references,
            );
            let far = is_far_control_operand(instruction.operands().first().unwrap());
            stack_transfer(at, effective_operand_size(instruction), far, true, accesses);
        }
        Operation::Jump => {
            read_control_operand(
                instruction.operands().first().unwrap(),
                at,
                accesses,
                references,
            );
        }
        Operation::ConditionalJump(condition) => {
            read_condition(accesses, condition, AccessOccurrence::Always);
        }
        Operation::Loop(condition) => loop_accesses(instruction, condition, accesses),
        Operation::JumpCounterZero => {
            read_counter(instruction, accesses, AccessOccurrence::Always);
        }
        Operation::ReturnNear => pop_stack(
            at,
            effective_operand_size(instruction),
            AccessOccurrence::Always,
            accesses,
        ),
        Operation::ReturnFar => stack_transfer(
            at,
            effective_operand_size(instruction),
            true,
            false,
            accesses,
        ),
        Operation::Enter => enter_accesses(instruction, at, accesses),
        Operation::Leave => leave_accesses(instruction, at, accesses),
        Operation::Interrupt => {
            exception_entry(AccessOccurrence::Always, accesses);
            push_interrupt_frame(instruction, at, AccessOccurrence::Always, accesses);
        }
        Operation::InterruptOnOverflow => {
            read_flag(accesses, Flag::Overflow, AccessOccurrence::Always);
            exception_entry(AccessOccurrence::Conditional, accesses);
            push_interrupt_frame(instruction, at, AccessOccurrence::Conditional, accesses);
        }
        Operation::InterruptReturn => {
            let size = effective_operand_size(instruction);
            pop_stack_bytes(
                at,
                (size.byte_len() * 3) as u8,
                AccessOccurrence::Always,
                accesses,
            );
            write(
                accesses,
                Location::SegmentRegister(SegmentRegister::Cs),
                AccessOccurrence::Always,
            );
            write(accesses, Location::Flags, AccessOccurrence::Always);
        }
        Operation::Input | Operation::Output => port_accesses(instruction, at, accesses),
        Operation::String { operation, width } => {
            string_accesses(instruction, operation, width, at, accesses)
        }
        Operation::SetCondition(condition) => {
            read_condition(accesses, condition, AccessOccurrence::Always);
            write_operand(
                &instruction.operands()[0],
                at,
                AccessOccurrence::Always,
                accesses,
                references,
            );
        }
        Operation::ConvertByteToWordOrDoubleword => convert_accumulator(instruction, accesses),
        Operation::ConvertWordOrDoubleword => convert_data_register(instruction, accesses),
        Operation::DecimalAdjustAfterAddition | Operation::DecimalAdjustAfterSubtraction => {
            decimal_adjust(accesses)
        }
        Operation::AsciiAdjustAfterAddition | Operation::AsciiAdjustAfterSubtraction => {
            ascii_adjust(accesses)
        }
        Operation::AsciiAdjustAfterMultiply | Operation::AsciiAdjustBeforeDivision => {
            ascii_multiply_divide(instruction, accesses);
            if instruction.operation() == Operation::AsciiAdjustAfterMultiply
                && immediate_byte(instruction) == Some(0)
            {
                exception_entry(AccessOccurrence::Always, accesses);
                push_interrupt_frame(instruction, at, AccessOccurrence::Always, accesses);
            }
        }
        Operation::Translate => translate_accesses(instruction, at, accesses),
        Operation::StoreAhIntoFlags => store_ah_into_flags(accesses),
        Operation::LoadFlagsIntoAh => load_flags_into_ah(accesses),
        Operation::ClearCarry | Operation::SetCarry => {
            write_flag(accesses, Flag::Carry, AccessOccurrence::Always)
        }
        Operation::ComplementCarry => {
            read_write_flag(accesses, Flag::Carry, AccessOccurrence::Always)
        }
        Operation::ClearDirection | Operation::SetDirection => {
            write_flag(accesses, Flag::Direction, AccessOccurrence::Always)
        }
        Operation::ClearInterrupt | Operation::SetInterrupt => {
            write_flag(accesses, Flag::InterruptEnable, AccessOccurrence::Always)
        }
        Operation::ClearTaskSwitched => {
            read_write(accesses, Location::ControlRegister(ControlRegister::Cr0))
        }
        Operation::LoadMachineStatusWord => {
            read_operand(
                &instruction.operands()[0],
                at,
                AccessOccurrence::Always,
                accesses,
                references,
            );
            read_write(accesses, Location::ControlRegister(ControlRegister::Cr0));
        }
        Operation::StoreMachineStatusWord => {
            read(
                accesses,
                Location::ControlRegister(ControlRegister::Cr0),
                AccessOccurrence::Always,
            );
            write_operand(
                &instruction.operands()[0],
                at,
                AccessOccurrence::Always,
                accesses,
                references,
            );
        }
        Operation::CoprocessorEscape(operation) => {
            coprocessor_accesses(instruction, operation, at, accesses, references)
        }
        Operation::Halt | Operation::NoOperation | Operation::Wait => {}
    }
}

fn binary_accesses(
    instruction: &Instruction,
    operation: BinaryOperation,
    at: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    let [destination, source] = instruction.operands() else {
        unreachable!()
    };
    read_operand(
        destination,
        at,
        AccessOccurrence::Always,
        accesses,
        references,
    );
    read_operand(source, at, AccessOccurrence::Always, accesses, references);
    if !matches!(operation, BinaryOperation::Compare | BinaryOperation::Test) {
        write_operand(
            destination,
            at,
            AccessOccurrence::Always,
            accesses,
            references,
        );
    }
    if matches!(
        operation,
        BinaryOperation::AddWithCarry | BinaryOperation::SubtractWithBorrow
    ) {
        read_flag(accesses, Flag::Carry, AccessOccurrence::Always);
    }
    if matches!(
        operation,
        BinaryOperation::And
            | BinaryOperation::Or
            | BinaryOperation::ExclusiveOr
            | BinaryOperation::Test
    ) {
        write_logic_flags(accesses, AccessOccurrence::Always);
    } else {
        write_arithmetic_flags(accesses, AccessOccurrence::Always);
    }
}

fn unary_accesses(
    instruction: &Instruction,
    operation: UnaryOperation,
    at: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    let operand = &instruction.operands()[0];
    read_operand(operand, at, AccessOccurrence::Always, accesses, references);
    match operation {
        UnaryOperation::Increment | UnaryOperation::Decrement => {
            write_operand(operand, at, AccessOccurrence::Always, accesses, references);
            write_inc_dec_flags(accesses, AccessOccurrence::Always);
        }
        UnaryOperation::Negate => {
            write_operand(operand, at, AccessOccurrence::Always, accesses, references);
            write_arithmetic_flags(accesses, AccessOccurrence::Always);
        }
        UnaryOperation::Not => {
            write_operand(operand, at, AccessOccurrence::Always, accesses, references)
        }
        UnaryOperation::MultiplyUnsigned | UnaryOperation::MultiplySigned => {
            implicit_multiply(operand_width(operand), accesses);
            write_arithmetic_flags(accesses, AccessOccurrence::Always);
        }
        UnaryOperation::DivideUnsigned | UnaryOperation::DivideSigned => {
            implicit_divide(operand_width(operand), accesses);
            write_arithmetic_flags(accesses, AccessOccurrence::Conditional);
            exception_entry(AccessOccurrence::Conditional, accesses);
            push_interrupt_frame(instruction, at, AccessOccurrence::Conditional, accesses);
        }
    }
}

fn shift_accesses(
    instruction: &Instruction,
    operation: ShiftOperation,
    at: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    let destination = &instruction.operands()[0];
    read_operand(
        destination,
        at,
        AccessOccurrence::Always,
        accesses,
        references,
    );
    for operand in &instruction.operands()[1..] {
        read_operand(operand, at, AccessOccurrence::Always, accesses, references);
    }
    let Some(occurrence) = shift_occurrence(instruction) else {
        return;
    };
    write_operand(destination, at, occurrence, accesses, references);
    if matches!(
        operation,
        ShiftOperation::RotateThroughCarryLeft | ShiftOperation::RotateThroughCarryRight
    ) {
        read_flag(accesses, Flag::Carry, occurrence);
    }
    write_shift_flags(accesses, operation, occurrence);
}

fn bit_accesses(
    instruction: &Instruction,
    operation: BitOperation,
    at: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    let [left, right] = instruction.operands() else {
        unreachable!()
    };
    if matches!(
        operation,
        BitOperation::ScanForward | BitOperation::ScanReverse
    ) {
        read_operand(right, at, AccessOccurrence::Always, accesses, references);
        write_operand(
            left,
            at,
            AccessOccurrence::Conditional,
            accesses,
            references,
        );
        write_flag(accesses, Flag::Zero, AccessOccurrence::Always);
        return;
    }
    read_operand(left, at, AccessOccurrence::Always, accesses, references);
    read_operand(right, at, AccessOccurrence::Always, accesses, references);
    if operation != BitOperation::Test {
        write_operand(left, at, AccessOccurrence::Always, accesses, references);
    }
    write_flag(accesses, Flag::Carry, AccessOccurrence::Always);
}

fn descriptor_accesses(
    instruction: &Instruction,
    operation: DescriptorOperation,
    at: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    use DescriptorOperation::*;
    match operation {
        StoreGlobalDescriptorTable | StoreInterruptDescriptorTable => {
            read(
                accesses,
                Location::Implicit(if operation == StoreGlobalDescriptorTable {
                    ImplicitState::GlobalDescriptorTable
                } else {
                    ImplicitState::InterruptDescriptorTable
                }),
                AccessOccurrence::Always,
            );
            write_operand(
                &instruction.operands()[0],
                at,
                AccessOccurrence::Always,
                accesses,
                references,
            );
        }
        LoadGlobalDescriptorTable | LoadInterruptDescriptorTable => {
            read_operand(
                &instruction.operands()[0],
                at,
                AccessOccurrence::Always,
                accesses,
                references,
            );
            write(
                accesses,
                Location::Implicit(if operation == LoadGlobalDescriptorTable {
                    ImplicitState::GlobalDescriptorTable
                } else {
                    ImplicitState::InterruptDescriptorTable
                }),
                AccessOccurrence::Always,
            );
        }
        StoreLocalDescriptorTable | StoreTaskRegister => {
            read(
                accesses,
                Location::Implicit(if operation == StoreLocalDescriptorTable {
                    ImplicitState::LocalDescriptorTable
                } else {
                    ImplicitState::TaskRegister
                }),
                AccessOccurrence::Always,
            );
            write_operand(
                &instruction.operands()[0],
                at,
                AccessOccurrence::Always,
                accesses,
                references,
            );
        }
        LoadLocalDescriptorTable | LoadTaskRegister => {
            read_operand(
                &instruction.operands()[0],
                at,
                AccessOccurrence::Always,
                accesses,
                references,
            );
            write(
                accesses,
                Location::Implicit(if operation == LoadLocalDescriptorTable {
                    ImplicitState::LocalDescriptorTable
                } else {
                    ImplicitState::TaskRegister
                }),
                AccessOccurrence::Always,
            );
        }
        LoadAccessRights | LoadSegmentLimit => {
            read_operand(
                &instruction.operands()[1],
                at,
                AccessOccurrence::Always,
                accesses,
                references,
            );
            read(
                accesses,
                Location::Implicit(ImplicitState::DescriptorEnvironment),
                AccessOccurrence::Always,
            );
            write_operand(
                &instruction.operands()[0],
                at,
                AccessOccurrence::Conditional,
                accesses,
                references,
            );
            write_flag(accesses, Flag::Zero, AccessOccurrence::Always);
        }
        VerifyRead | VerifyWrite => {
            read_operand(
                &instruction.operands()[0],
                at,
                AccessOccurrence::Always,
                accesses,
                references,
            );
            read(
                accesses,
                Location::Implicit(ImplicitState::DescriptorEnvironment),
                AccessOccurrence::Always,
            );
            write_flag(accesses, Flag::Zero, AccessOccurrence::Always);
        }
    }
}

fn read_operand(
    operand: &Operand,
    at: AnalysisPoint,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    match operand {
        Operand::GeneralRegister(register) => {
            read(accesses, Location::GeneralRegister(*register), occurrence)
        }
        Operand::SegmentRegister(register) => {
            read(accesses, Location::SegmentRegister(*register), occurrence)
        }
        Operand::ControlRegister(register) => {
            read(accesses, Location::ControlRegister(*register), occurrence)
        }
        Operand::DebugRegister(register) => {
            read(accesses, Location::DebugRegister(*register), occurrence)
        }
        Operand::TestRegister(register) => {
            read(accesses, Location::TestRegister(*register), occurrence)
        }
        Operand::CoprocessorRegister(_) => {}
        Operand::Memory(memory) => memory_access(
            *memory,
            at,
            AccessKind::Read,
            occurrence,
            accesses,
            references,
        ),
        Operand::Immediate(_) | Operand::Relative(_) | Operand::FarPointer(_) => {}
    }
}

fn write_operand(
    operand: &Operand,
    at: AnalysisPoint,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    match operand {
        Operand::GeneralRegister(register) => {
            write(accesses, Location::GeneralRegister(*register), occurrence)
        }
        Operand::SegmentRegister(register) => {
            write(accesses, Location::SegmentRegister(*register), occurrence)
        }
        Operand::ControlRegister(register) => {
            write(accesses, Location::ControlRegister(*register), occurrence)
        }
        Operand::DebugRegister(register) => {
            write(accesses, Location::DebugRegister(*register), occurrence)
        }
        Operand::TestRegister(register) => {
            write(accesses, Location::TestRegister(*register), occurrence)
        }
        Operand::CoprocessorRegister(_) => {}
        Operand::Memory(memory) => memory_access(
            *memory,
            at,
            AccessKind::Write,
            occurrence,
            accesses,
            references,
        ),
        Operand::Immediate(_) | Operand::Relative(_) | Operand::FarPointer(_) => {}
    }
}

fn read_write_operand(
    operand: &Operand,
    at: AnalysisPoint,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    read_operand(operand, at, occurrence, accesses, references);
    write_operand(operand, at, occurrence, accesses, references);
}

fn memory_access(
    memory: MemoryOperand,
    at: AnalysisPoint,
    kind: AccessKind,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    address_inputs(memory, true, occurrence, accesses);
    access(
        accesses,
        Location::Memory(memory_location(memory, at)),
        kind,
        occurrence,
    );
    add_direct_reference(
        memory,
        at,
        match kind {
            AccessKind::Read => ReferenceKind::Read,
            AccessKind::Write => ReferenceKind::Write,
        },
        occurrence,
        references,
    );
}

fn memory_location(memory: MemoryOperand, at: AnalysisPoint) -> MemoryLocation {
    let segment = memory
        .segment()
        .unwrap_or_else(|| default_segment(memory.address()));
    MemoryLocation {
        segment: segment_selection(segment, at),
        offset: OffsetAddress::Effective(memory.address()),
        extent: MemoryExtent::Operand(memory.width()),
    }
}

fn address_inputs(
    memory: MemoryOperand,
    read_segment: bool,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    let segment = memory
        .segment()
        .unwrap_or_else(|| default_segment(memory.address()));
    if read_segment {
        read(accesses, Location::SegmentRegister(segment), occurrence);
    }
    match memory.address() {
        EffectiveAddress::Bits16 { base, .. } => {
            for register in address16_registers(base) {
                read(
                    accesses,
                    Location::GeneralRegister(GeneralRegister::Word(*register)),
                    occurrence,
                );
            }
        }
        EffectiveAddress::Bits32 { base, index, .. } => {
            if let Some(base) = base {
                read(
                    accesses,
                    Location::GeneralRegister(GeneralRegister::Doubleword(base)),
                    occurrence,
                );
            }
            if let Some(index) = index {
                read(
                    accesses,
                    Location::GeneralRegister(GeneralRegister::Doubleword(index.register())),
                    occurrence,
                );
            }
        }
    }
}

fn address16_registers(base: Address16Base) -> &'static [Register16] {
    use Address16Base::*;
    match base {
        BxSi => &[Register16::Bx, Register16::Si],
        BxDi => &[Register16::Bx, Register16::Di],
        BpSi => &[Register16::Bp, Register16::Si],
        BpDi => &[Register16::Bp, Register16::Di],
        Si => &[Register16::Si],
        Di => &[Register16::Di],
        Bp => &[Register16::Bp],
        Bx => &[Register16::Bx],
        Direct => &[],
    }
}

fn default_segment(address: EffectiveAddress) -> SegmentRegister {
    match address {
        EffectiveAddress::Bits16 {
            base: Address16Base::Bp | Address16Base::BpSi | Address16Base::BpDi,
            ..
        }
        | EffectiveAddress::Bits32 {
            base: Some(Register32::Ebp | Register32::Esp),
            ..
        } => SegmentRegister::Ss,
        _ => SegmentRegister::Ds,
    }
}

fn add_direct_reference(
    memory: MemoryOperand,
    at: AnalysisPoint,
    kind: ReferenceKind,
    occurrence: AccessOccurrence,
    references: &mut Vec<StaticReference<Reference>>,
) {
    let offset = match memory.address() {
        EffectiveAddress::Bits16 {
            base: Address16Base::Direct,
            displacement: Displacement::AbsoluteWord(value),
        } => u32::from(value),
        EffectiveAddress::Bits32 {
            base: None,
            index: None,
            displacement: Displacement::AbsoluteDoubleword(value),
            ..
        } => value,
        _ => return,
    };
    add_reference(
        references,
        StaticReference {
            target: if kind == ReferenceKind::Address {
                Reference::Offset(offset)
            } else {
                Reference::Memory {
                    segment: segment_selection(
                        memory
                            .segment()
                            .unwrap_or_else(|| default_segment(memory.address())),
                        at,
                    ),
                    offset,
                    width: memory.width(),
                }
            },
            kind,
            occurrence,
        },
    );
}

fn segment_selection(segment: SegmentRegister, at: AnalysisPoint) -> SegmentSelection {
    if segment == SegmentRegister::Cs {
        SegmentSelection::CurrentCode(at.location.selector())
    } else {
        SegmentSelection::Register(segment)
    }
}

fn effective_operand_size(instruction: &Instruction) -> OperandSize {
    instruction
        .prefixes()
        .effective_operand_size(instruction.code_mode())
}

fn effective_address_size(instruction: &Instruction) -> AddressSize {
    instruction
        .prefixes()
        .effective_address_size(instruction.code_mode())
}

fn read_control_operand(
    operand: &Operand,
    at: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    match operand {
        Operand::Relative(_) | Operand::FarPointer(_) => {}
        _ => read_operand(operand, at, AccessOccurrence::Always, accesses, references),
    }
}

fn is_far_control_operand(operand: &Operand) -> bool {
    match operand {
        Operand::FarPointer(_) => true,
        Operand::Memory(memory) => matches!(
            memory.width(),
            MemoryWidth::FarPointerWord | MemoryWidth::FarPointerDoubleword
        ),
        _ => false,
    }
}

fn stack_pointer(at: AnalysisPoint) -> GeneralRegister {
    match at.stack_address_size {
        AddressSize::Bits16 => GeneralRegister::Word(Register16::Sp),
        AddressSize::Bits32 => GeneralRegister::Doubleword(Register32::Esp),
    }
}

fn stack_memory(at: AnalysisPoint, width: MemoryWidth) -> Location {
    Location::Memory(MemoryLocation {
        segment: SegmentSelection::Register(SegmentRegister::Ss),
        offset: OffsetAddress::Stack(at.stack_address_size),
        extent: MemoryExtent::Operand(width),
    })
}

fn push_stack(
    at: AnalysisPoint,
    size: OperandSize,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read(
        accesses,
        Location::GeneralRegister(stack_pointer(at)),
        occurrence,
    );
    write(
        accesses,
        Location::GeneralRegister(stack_pointer(at)),
        occurrence,
    );
    read(
        accesses,
        Location::SegmentRegister(SegmentRegister::Ss),
        occurrence,
    );
    write(
        accesses,
        stack_memory(at, MemoryWidth::from(size)),
        occurrence,
    );
}

fn push_stack_bytes(
    at: AnalysisPoint,
    byte_len: u8,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read(
        accesses,
        Location::GeneralRegister(stack_pointer(at)),
        occurrence,
    );
    write(
        accesses,
        Location::GeneralRegister(stack_pointer(at)),
        occurrence,
    );
    read(
        accesses,
        Location::SegmentRegister(SegmentRegister::Ss),
        occurrence,
    );
    write(
        accesses,
        Location::Memory(MemoryLocation {
            segment: SegmentSelection::Register(SegmentRegister::Ss),
            offset: OffsetAddress::Stack(at.stack_address_size),
            extent: MemoryExtent::KnownBytes(byte_len),
        }),
        occurrence,
    );
}

fn pop_stack(
    at: AnalysisPoint,
    size: OperandSize,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read(
        accesses,
        Location::GeneralRegister(stack_pointer(at)),
        occurrence,
    );
    write(
        accesses,
        Location::GeneralRegister(stack_pointer(at)),
        occurrence,
    );
    read(
        accesses,
        Location::SegmentRegister(SegmentRegister::Ss),
        occurrence,
    );
    read(
        accesses,
        stack_memory(at, MemoryWidth::from(size)),
        occurrence,
    );
}

fn pop_stack_bytes(
    at: AnalysisPoint,
    byte_len: u8,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read(
        accesses,
        Location::GeneralRegister(stack_pointer(at)),
        occurrence,
    );
    write(
        accesses,
        Location::GeneralRegister(stack_pointer(at)),
        occurrence,
    );
    read(
        accesses,
        Location::SegmentRegister(SegmentRegister::Ss),
        occurrence,
    );
    read(
        accesses,
        Location::Memory(MemoryLocation {
            segment: SegmentSelection::Register(SegmentRegister::Ss),
            offset: OffsetAddress::Stack(at.stack_address_size),
            extent: MemoryExtent::KnownBytes(byte_len),
        }),
        occurrence,
    );
}

fn stack_transfer(
    at: AnalysisPoint,
    size: OperandSize,
    far: bool,
    push: bool,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    let count = if far { 2 } else { 1 };
    for _ in 0..count {
        if push {
            push_stack(at, size, AccessOccurrence::Always, accesses);
        } else {
            pop_stack(at, size, AccessOccurrence::Always, accesses);
        }
    }
}

fn push_all(
    instruction: &Instruction,
    at: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    let size = effective_operand_size(instruction);
    for index in 0..8 {
        read(
            accesses,
            Location::GeneralRegister(register_for_size(index, size)),
            AccessOccurrence::Always,
        );
    }
    for _ in 0..8 {
        push_stack(at, size, AccessOccurrence::Always, accesses);
    }
}

fn pop_all(
    instruction: &Instruction,
    at: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    let size = effective_operand_size(instruction);
    for _ in 0..8 {
        pop_stack(at, size, AccessOccurrence::Always, accesses);
    }
    for index in [0, 1, 2, 3, 5, 6, 7] {
        write(
            accesses,
            Location::GeneralRegister(register_for_size(index, size)),
            AccessOccurrence::Always,
        );
    }
}

fn register_for_size(index: u8, size: OperandSize) -> GeneralRegister {
    match size {
        OperandSize::Byte => GeneralRegister::Byte(Register8::from_index(index).unwrap()),
        OperandSize::Word => GeneralRegister::Word(Register16::from_index(index).unwrap()),
        OperandSize::Doubleword => {
            GeneralRegister::Doubleword(Register32::from_index(index).unwrap())
        }
    }
}

fn operand_width(operand: &Operand) -> OperandSize {
    match operand {
        Operand::GeneralRegister(register) => register.size(),
        Operand::Memory(memory) => match memory.width() {
            MemoryWidth::Byte => OperandSize::Byte,
            MemoryWidth::Word => OperandSize::Word,
            MemoryWidth::Doubleword => OperandSize::Doubleword,
            _ => OperandSize::Word,
        },
        _ => OperandSize::Word,
    }
}

fn implicit_multiply(size: OperandSize, accesses: &mut Vec<LocationAccess<Location>>) {
    read(
        accesses,
        Location::GeneralRegister(register_for_size(0, size)),
        AccessOccurrence::Always,
    );
    match size {
        OperandSize::Byte => write(
            accesses,
            Location::GeneralRegister(GeneralRegister::Word(Register16::Ax)),
            AccessOccurrence::Always,
        ),
        OperandSize::Word | OperandSize::Doubleword => {
            write(
                accesses,
                Location::GeneralRegister(register_for_size(0, size)),
                AccessOccurrence::Always,
            );
            write(
                accesses,
                Location::GeneralRegister(register_for_size(2, size)),
                AccessOccurrence::Always,
            );
        }
    }
}

fn implicit_divide(size: OperandSize, accesses: &mut Vec<LocationAccess<Location>>) {
    match size {
        OperandSize::Byte => {
            read(
                accesses,
                Location::GeneralRegister(GeneralRegister::Word(Register16::Ax)),
                AccessOccurrence::Always,
            );
            write(
                accesses,
                Location::GeneralRegister(GeneralRegister::Word(Register16::Ax)),
                AccessOccurrence::Conditional,
            );
        }
        OperandSize::Word | OperandSize::Doubleword => {
            for index in [0, 2] {
                read(
                    accesses,
                    Location::GeneralRegister(register_for_size(index, size)),
                    AccessOccurrence::Always,
                );
                write(
                    accesses,
                    Location::GeneralRegister(register_for_size(index, size)),
                    AccessOccurrence::Conditional,
                );
            }
        }
    }
}

fn shift_occurrence(instruction: &Instruction) -> Option<AccessOccurrence> {
    match instruction.operands().last() {
        Some(Operand::Immediate(Immediate::Byte(value))) if *value & 0x1f == 0 => None,
        Some(Operand::GeneralRegister(GeneralRegister::Byte(Register8::Cl))) => {
            Some(AccessOccurrence::Conditional)
        }
        _ => Some(AccessOccurrence::Always),
    }
}

fn loop_accesses(
    instruction: &Instruction,
    condition: crate::LoopCondition,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    read_counter(instruction, accesses, AccessOccurrence::Always);
    write_counter(instruction, accesses, AccessOccurrence::Always);
    if condition != crate::LoopCondition::Always {
        read_flag(accesses, Flag::Zero, AccessOccurrence::Always);
    }
}

fn counter_register(instruction: &Instruction) -> GeneralRegister {
    match effective_address_size(instruction) {
        AddressSize::Bits16 => GeneralRegister::Word(Register16::Cx),
        AddressSize::Bits32 => GeneralRegister::Doubleword(Register32::Ecx),
    }
}

fn read_counter(
    instruction: &Instruction,
    accesses: &mut Vec<LocationAccess<Location>>,
    occurrence: AccessOccurrence,
) {
    read(
        accesses,
        Location::GeneralRegister(counter_register(instruction)),
        occurrence,
    );
}

fn write_counter(
    instruction: &Instruction,
    accesses: &mut Vec<LocationAccess<Location>>,
    occurrence: AccessOccurrence,
) {
    write(
        accesses,
        Location::GeneralRegister(counter_register(instruction)),
        occurrence,
    );
}

fn enter_accesses(
    instruction: &Instruction,
    at: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    let size = effective_operand_size(instruction);
    let bp = register_for_size(5, size);
    read(
        accesses,
        Location::GeneralRegister(bp),
        AccessOccurrence::Always,
    );
    push_stack(at, size, AccessOccurrence::Always, accesses);
    write(
        accesses,
        Location::GeneralRegister(bp),
        AccessOccurrence::Always,
    );
    if immediate_byte(instruction).is_some_and(|level| level & 0x1f != 0) {
        read(
            accesses,
            stack_memory(at, MemoryWidth::from(size)),
            AccessOccurrence::Conditional,
        );
        write(
            accesses,
            stack_memory(at, MemoryWidth::from(size)),
            AccessOccurrence::Conditional,
        );
    }
}

fn leave_accesses(
    instruction: &Instruction,
    at: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    let size = effective_operand_size(instruction);
    let bp = register_for_size(5, size);
    read(
        accesses,
        Location::GeneralRegister(bp),
        AccessOccurrence::Always,
    );
    write(
        accesses,
        Location::GeneralRegister(stack_pointer(at)),
        AccessOccurrence::Always,
    );
    pop_stack(at, size, AccessOccurrence::Always, accesses);
    write(
        accesses,
        Location::GeneralRegister(bp),
        AccessOccurrence::Always,
    );
}

fn interrupt_vector(instruction: &Instruction) -> u8 {
    immediate_byte(instruction).unwrap_or(3)
}

fn immediate_byte(instruction: &Instruction) -> Option<u8> {
    instruction
        .operands()
        .iter()
        .find_map(|operand| match operand {
            Operand::Immediate(Immediate::Byte(value)) => Some(*value),
            _ => None,
        })
}

fn exception_entry(occurrence: AccessOccurrence, accesses: &mut Vec<LocationAccess<Location>>) {
    read(accesses, Location::Flags, occurrence);
    read(
        accesses,
        Location::SegmentRegister(SegmentRegister::Cs),
        occurrence,
    );
    write_flag(accesses, Flag::Trap, occurrence);
    write_flag(accesses, Flag::InterruptEnable, occurrence);
    read(
        accesses,
        Location::Implicit(ImplicitState::InterruptDescriptorTable),
        occurrence,
    );
}

fn push_interrupt_frame(
    instruction: &Instruction,
    at: AnalysisPoint,
    occurrence: AccessOccurrence,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    let size = effective_operand_size(instruction);
    push_stack_bytes(at, (size.byte_len() * 3) as u8, occurrence, accesses);
}

fn port_accesses(
    instruction: &Instruction,
    at: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    let operation = instruction.operation();
    let operands = instruction.operands();
    let (data, port) = if operation == Operation::Input {
        (&operands[0], &operands[1])
    } else {
        (&operands[1], &operands[0])
    };
    let width = operand_width(data).byte_len() as u8;
    let port = match port {
        Operand::Immediate(Immediate::Byte(value)) => PortLocation::Immediate(*value),
        Operand::GeneralRegister(GeneralRegister::Word(Register16::Dx)) => {
            read(
                accesses,
                Location::GeneralRegister(GeneralRegister::Word(Register16::Dx)),
                AccessOccurrence::Always,
            );
            PortLocation::DataRegister
        }
        _ => unreachable!(),
    };
    if operation == Operation::Input {
        read(
            accesses,
            Location::Port {
                port,
                byte_len: width,
            },
            AccessOccurrence::Always,
        );
        write_operand(
            data,
            at,
            AccessOccurrence::Always,
            accesses,
            &mut Vec::new(),
        );
    } else {
        read_operand(
            data,
            at,
            AccessOccurrence::Always,
            accesses,
            &mut Vec::new(),
        );
        write(
            accesses,
            Location::Port {
                port,
                byte_len: width,
            },
            AccessOccurrence::Always,
        );
    }
}

fn string_accesses(
    instruction: &Instruction,
    operation: StringOperation,
    width: OperandSize,
    at: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    let address_size = effective_address_size(instruction);
    let occurrence = if instruction
        .prefixes()
        .as_slice()
        .iter()
        .any(|prefix| matches!(prefix, Prefix::Repeat | Prefix::RepeatNotEqual))
    {
        read_counter(instruction, accesses, AccessOccurrence::Always);
        write_counter(instruction, accesses, AccessOccurrence::Conditional);
        AccessOccurrence::Conditional
    } else {
        AccessOccurrence::Always
    };
    read_flag(accesses, Flag::Direction, occurrence);
    let source_segment = instruction
        .prefixes()
        .effective_segment()
        .unwrap_or(SegmentRegister::Ds);
    let source = Location::Memory(MemoryLocation {
        segment: SegmentSelection::Register(source_segment),
        offset: OffsetAddress::StringSource(address_size),
        extent: MemoryExtent::Operand(MemoryWidth::from(width)),
    });
    let destination = Location::Memory(MemoryLocation {
        segment: SegmentSelection::Register(SegmentRegister::Es),
        offset: OffsetAddress::StringDestination(address_size),
        extent: MemoryExtent::Operand(MemoryWidth::from(width)),
    });
    let source_index = string_index(address_size, true);
    let destination_index = string_index(address_size, false);
    let accumulator = register_for_size(0, width);
    if matches!(
        operation,
        StringOperation::Move
            | StringOperation::Compare
            | StringOperation::Load
            | StringOperation::Output
    ) {
        read(
            accesses,
            Location::SegmentRegister(source_segment),
            occurrence,
        );
    }
    if matches!(
        operation,
        StringOperation::Move
            | StringOperation::Compare
            | StringOperation::Store
            | StringOperation::Scan
            | StringOperation::Input
    ) {
        read(
            accesses,
            Location::SegmentRegister(SegmentRegister::Es),
            occurrence,
        );
    }
    match operation {
        StringOperation::Move => {
            read(accesses, source, occurrence);
            write(accesses, destination, occurrence);
            read_write_occurrence(
                accesses,
                Location::GeneralRegister(source_index),
                occurrence,
            );
            read_write_occurrence(
                accesses,
                Location::GeneralRegister(destination_index),
                occurrence,
            );
        }
        StringOperation::Compare => {
            read(accesses, source, occurrence);
            read(accesses, destination, occurrence);
            read_write_occurrence(
                accesses,
                Location::GeneralRegister(source_index),
                occurrence,
            );
            read_write_occurrence(
                accesses,
                Location::GeneralRegister(destination_index),
                occurrence,
            );
            write_arithmetic_flags(accesses, occurrence);
        }
        StringOperation::Store => {
            read(accesses, Location::GeneralRegister(accumulator), occurrence);
            write(accesses, destination, occurrence);
            read_write_occurrence(
                accesses,
                Location::GeneralRegister(destination_index),
                occurrence,
            );
        }
        StringOperation::Load => {
            read(accesses, source, occurrence);
            write(accesses, Location::GeneralRegister(accumulator), occurrence);
            read_write_occurrence(
                accesses,
                Location::GeneralRegister(source_index),
                occurrence,
            );
        }
        StringOperation::Scan => {
            read(accesses, Location::GeneralRegister(accumulator), occurrence);
            read(accesses, destination, occurrence);
            read_write_occurrence(
                accesses,
                Location::GeneralRegister(destination_index),
                occurrence,
            );
            write_arithmetic_flags(accesses, occurrence);
        }
        StringOperation::Input => {
            read(
                accesses,
                Location::GeneralRegister(GeneralRegister::Word(Register16::Dx)),
                occurrence,
            );
            read(
                accesses,
                Location::Port {
                    port: PortLocation::DataRegister,
                    byte_len: width.byte_len() as u8,
                },
                occurrence,
            );
            write(accesses, destination, occurrence);
            read_write_occurrence(
                accesses,
                Location::GeneralRegister(destination_index),
                occurrence,
            );
        }
        StringOperation::Output => {
            read(
                accesses,
                Location::GeneralRegister(GeneralRegister::Word(Register16::Dx)),
                occurrence,
            );
            read(accesses, source, occurrence);
            write(
                accesses,
                Location::Port {
                    port: PortLocation::DataRegister,
                    byte_len: width.byte_len() as u8,
                },
                occurrence,
            );
            read_write_occurrence(
                accesses,
                Location::GeneralRegister(source_index),
                occurrence,
            );
        }
    }
    if matches!(operation, StringOperation::Compare | StringOperation::Scan)
        && instruction
            .prefixes()
            .as_slice()
            .iter()
            .any(|prefix| matches!(prefix, Prefix::Repeat | Prefix::RepeatNotEqual))
    {
        read_flag(accesses, Flag::Zero, AccessOccurrence::Always);
    }
    let _ = at;
}

fn string_index(size: AddressSize, source: bool) -> GeneralRegister {
    match (size, source) {
        (AddressSize::Bits16, true) => GeneralRegister::Word(Register16::Si),
        (AddressSize::Bits16, false) => GeneralRegister::Word(Register16::Di),
        (AddressSize::Bits32, true) => GeneralRegister::Doubleword(Register32::Esi),
        (AddressSize::Bits32, false) => GeneralRegister::Doubleword(Register32::Edi),
    }
}

fn convert_accumulator(instruction: &Instruction, accesses: &mut Vec<LocationAccess<Location>>) {
    match effective_operand_size(instruction) {
        OperandSize::Word => {
            read(
                accesses,
                Location::GeneralRegister(GeneralRegister::Byte(Register8::Al)),
                AccessOccurrence::Always,
            );
            write(
                accesses,
                Location::GeneralRegister(GeneralRegister::Word(Register16::Ax)),
                AccessOccurrence::Always,
            );
        }
        OperandSize::Doubleword => {
            read(
                accesses,
                Location::GeneralRegister(GeneralRegister::Word(Register16::Ax)),
                AccessOccurrence::Always,
            );
            write(
                accesses,
                Location::GeneralRegister(GeneralRegister::Doubleword(Register32::Eax)),
                AccessOccurrence::Always,
            );
        }
        OperandSize::Byte => unreachable!(),
    }
}

fn convert_data_register(instruction: &Instruction, accesses: &mut Vec<LocationAccess<Location>>) {
    let size = effective_operand_size(instruction);
    read(
        accesses,
        Location::GeneralRegister(register_for_size(0, size)),
        AccessOccurrence::Always,
    );
    write(
        accesses,
        Location::GeneralRegister(register_for_size(2, size)),
        AccessOccurrence::Always,
    );
}

fn decimal_adjust(accesses: &mut Vec<LocationAccess<Location>>) {
    read_write(
        accesses,
        Location::GeneralRegister(GeneralRegister::Byte(Register8::Al)),
    );
    read_write_flag(accesses, Flag::AuxiliaryCarry, AccessOccurrence::Always);
    read_write_flag(accesses, Flag::Carry, AccessOccurrence::Always);
    for flag in [Flag::Parity, Flag::Zero, Flag::Sign] {
        write_flag(accesses, flag, AccessOccurrence::Always);
    }
}

fn ascii_adjust(accesses: &mut Vec<LocationAccess<Location>>) {
    read_write(
        accesses,
        Location::GeneralRegister(GeneralRegister::Word(Register16::Ax)),
    );
    read_write_flag(accesses, Flag::AuxiliaryCarry, AccessOccurrence::Always);
    read_write_flag(accesses, Flag::Carry, AccessOccurrence::Always);
}

fn ascii_multiply_divide(instruction: &Instruction, accesses: &mut Vec<LocationAccess<Location>>) {
    read_write(
        accesses,
        Location::GeneralRegister(GeneralRegister::Word(Register16::Ax)),
    );
    if instruction.operation() == Operation::AsciiAdjustAfterMultiply
        && immediate_byte(instruction) != Some(0)
    {
        write_logic_flags(accesses, AccessOccurrence::Always);
    }
}

fn translate_accesses(
    instruction: &Instruction,
    at: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
) {
    let segment = instruction
        .prefixes()
        .effective_segment()
        .unwrap_or(SegmentRegister::Ds);
    let base = match effective_address_size(instruction) {
        AddressSize::Bits16 => GeneralRegister::Word(Register16::Bx),
        AddressSize::Bits32 => GeneralRegister::Doubleword(Register32::Ebx),
    };
    for register in [base, GeneralRegister::Byte(Register8::Al)] {
        read(
            accesses,
            Location::GeneralRegister(register),
            AccessOccurrence::Always,
        );
    }
    read(
        accesses,
        Location::SegmentRegister(segment),
        AccessOccurrence::Always,
    );
    read(
        accesses,
        Location::Memory(MemoryLocation {
            segment: SegmentSelection::Register(segment),
            offset: OffsetAddress::Translate,
            extent: MemoryExtent::Operand(MemoryWidth::Byte),
        }),
        AccessOccurrence::Always,
    );
    write(
        accesses,
        Location::GeneralRegister(GeneralRegister::Byte(Register8::Al)),
        AccessOccurrence::Always,
    );
    let _ = at;
}

fn store_ah_into_flags(accesses: &mut Vec<LocationAccess<Location>>) {
    read(
        accesses,
        Location::GeneralRegister(GeneralRegister::Byte(Register8::Ah)),
        AccessOccurrence::Always,
    );
    for flag in [
        Flag::Sign,
        Flag::Zero,
        Flag::AuxiliaryCarry,
        Flag::Parity,
        Flag::Carry,
    ] {
        write_flag(accesses, flag, AccessOccurrence::Always);
    }
}

fn load_flags_into_ah(accesses: &mut Vec<LocationAccess<Location>>) {
    for flag in [
        Flag::Sign,
        Flag::Zero,
        Flag::AuxiliaryCarry,
        Flag::Parity,
        Flag::Carry,
    ] {
        read_flag(accesses, flag, AccessOccurrence::Always);
    }
    write(
        accesses,
        Location::GeneralRegister(GeneralRegister::Byte(Register8::Ah)),
        AccessOccurrence::Always,
    );
}

fn coprocessor_accesses(
    instruction: &Instruction,
    operation: CoprocessorOperation,
    at: AnalysisPoint,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    read_write(accesses, Location::Implicit(ImplicitState::Coprocessor));
    let operand = &instruction.operands()[0];
    if let Operand::Memory(_) = operand {
        if coprocessor_memory_writes(operation) {
            write_operand(operand, at, AccessOccurrence::Always, accesses, references);
        } else {
            read_operand(operand, at, AccessOccurrence::Always, accesses, references);
        }
    } else {
        read_write_operand(operand, at, AccessOccurrence::Always, accesses, references);
    }
}

fn coprocessor_memory_writes(operation: CoprocessorOperation) -> bool {
    matches!(
        operation.selector(),
        10 | 11 | 14 | 15 | 26 | 27 | 30 | 31 | 42 | 43 | 46 | 47 | 58 | 59 | 62 | 63
    )
}

fn read_condition(
    accesses: &mut Vec<LocationAccess<Location>>,
    condition: Condition,
    occurrence: AccessOccurrence,
) {
    use Condition::*;
    let flags: &[Flag] = match condition {
        Overflow | NotOverflow => &[Flag::Overflow],
        Below | NotBelow => &[Flag::Carry],
        Equal | NotEqual => &[Flag::Zero],
        BelowOrEqual | Above => &[Flag::Carry, Flag::Zero],
        Sign | NotSign => &[Flag::Sign],
        Parity | NotParity => &[Flag::Parity],
        Less | GreaterOrEqual => &[Flag::Sign, Flag::Overflow],
        LessOrEqual | Greater => &[Flag::Zero, Flag::Sign, Flag::Overflow],
    };
    for flag in flags {
        read_flag(accesses, *flag, occurrence);
    }
}

fn write_arithmetic_flags(
    accesses: &mut Vec<LocationAccess<Location>>,
    occurrence: AccessOccurrence,
) {
    for flag in [
        Flag::Carry,
        Flag::Parity,
        Flag::AuxiliaryCarry,
        Flag::Zero,
        Flag::Sign,
        Flag::Overflow,
    ] {
        write_flag(accesses, flag, occurrence);
    }
}

fn write_logic_flags(accesses: &mut Vec<LocationAccess<Location>>, occurrence: AccessOccurrence) {
    for flag in [
        Flag::Carry,
        Flag::Parity,
        Flag::AuxiliaryCarry,
        Flag::Zero,
        Flag::Sign,
        Flag::Overflow,
    ] {
        write_flag(accesses, flag, occurrence);
    }
}

fn write_inc_dec_flags(accesses: &mut Vec<LocationAccess<Location>>, occurrence: AccessOccurrence) {
    for flag in [
        Flag::Parity,
        Flag::AuxiliaryCarry,
        Flag::Zero,
        Flag::Sign,
        Flag::Overflow,
    ] {
        write_flag(accesses, flag, occurrence);
    }
}

fn write_shift_flags(
    accesses: &mut Vec<LocationAccess<Location>>,
    operation: ShiftOperation,
    occurrence: AccessOccurrence,
) {
    let flags: &[Flag] = if matches!(
        operation,
        ShiftOperation::RotateLeft
            | ShiftOperation::RotateRight
            | ShiftOperation::RotateThroughCarryLeft
            | ShiftOperation::RotateThroughCarryRight
    ) {
        &[Flag::Carry, Flag::Overflow]
    } else {
        &[
            Flag::Carry,
            Flag::Parity,
            Flag::AuxiliaryCarry,
            Flag::Zero,
            Flag::Sign,
            Flag::Overflow,
        ]
    };
    for flag in flags {
        write_flag(accesses, *flag, occurrence);
    }
}

fn read_flag(
    accesses: &mut Vec<LocationAccess<Location>>,
    flag: Flag,
    occurrence: AccessOccurrence,
) {
    read(accesses, Location::Flag(flag), occurrence);
}

fn write_flag(
    accesses: &mut Vec<LocationAccess<Location>>,
    flag: Flag,
    occurrence: AccessOccurrence,
) {
    write(accesses, Location::Flag(flag), occurrence);
}

fn read_write_flag(
    accesses: &mut Vec<LocationAccess<Location>>,
    flag: Flag,
    occurrence: AccessOccurrence,
) {
    read_flag(accesses, flag, occurrence);
    write_flag(accesses, flag, occurrence);
}

fn read_write(accesses: &mut Vec<LocationAccess<Location>>, location: Location) {
    read(accesses, location, AccessOccurrence::Always);
    write(accesses, location, AccessOccurrence::Always);
}

fn read_write_occurrence(
    accesses: &mut Vec<LocationAccess<Location>>,
    location: Location,
    occurrence: AccessOccurrence,
) {
    read(accesses, location, occurrence);
    write(accesses, location, occurrence);
}

fn read(
    accesses: &mut Vec<LocationAccess<Location>>,
    location: Location,
    occurrence: AccessOccurrence,
) {
    access(accesses, location, AccessKind::Read, occurrence);
}

fn write(
    accesses: &mut Vec<LocationAccess<Location>>,
    location: Location,
    occurrence: AccessOccurrence,
) {
    access(accesses, location, AccessKind::Write, occurrence);
}

fn access(
    accesses: &mut Vec<LocationAccess<Location>>,
    location: Location,
    kind: AccessKind,
    occurrence: AccessOccurrence,
) {
    let candidate = LocationAccess {
        location,
        kind,
        occurrence,
    };
    if !accesses.contains(&candidate) {
        accesses.push(candidate);
    }
}

fn add_reference(
    references: &mut Vec<StaticReference<Reference>>,
    candidate: StaticReference<Reference>,
) {
    if !references.contains(&candidate) {
        references.push(candidate);
    }
}

pub(crate) fn storage_relation(left: &Location, right: &Location) -> StorageRelation {
    use StorageRelation::*;
    if left == right {
        return Same;
    }
    match (left, right) {
        (Location::GeneralRegister(left), Location::GeneralRegister(right)) => {
            if register_views_overlap(*left, *right) {
                Overlapping
            } else {
                Disjoint
            }
        }
        (Location::Flags, Location::Flag(_)) | (Location::Flag(_), Location::Flags) => Overlapping,
        (Location::Memory(left), Location::Memory(right)) => {
            if left.segment == right.segment && left.offset == right.offset {
                Overlapping
            } else {
                Unknown
            }
        }
        _ => Disjoint,
    }
}

fn register_views_overlap(left: GeneralRegister, right: GeneralRegister) -> bool {
    let (left_parent, left_start, left_len) = register_span(left);
    let (right_parent, right_start, right_len) = register_span(right);
    left_parent == right_parent
        && left_start < right_start + right_len
        && right_start < left_start + left_len
}

fn register_span(register: GeneralRegister) -> (u8, u8, u8) {
    match register {
        GeneralRegister::Byte(register) => {
            let index = register.index();
            if index < 4 {
                (index, 0, 1)
            } else {
                (index - 4, 1, 1)
            }
        }
        GeneralRegister::Word(register) => (register.index(), 0, 2),
        GeneralRegister::Doubleword(register) => (register.index(), 0, 4),
    }
}
