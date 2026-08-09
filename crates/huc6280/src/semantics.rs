//! Complete static meaning for the Hudson HuC6280 profile.

use typed_isa_core::{
    AccessKind, AccessOccurrence, ControlAction, ControlBoundary, ControlTarget, LocationAccess,
    ReferenceKind, StaticReference, StorageRelation,
};

use crate::{AddressingMode, CodeLocation, Instruction, Mnemonic, Operand};

/// Statically known value of the HuC6280 memory-operation flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TState {
    /// The T flag is clear, so accumulator operations use A.
    Clear,
    /// The T flag is set, so the next supported operation uses zero-page X.
    Set,
    /// The T flag was restored from runtime storage and either target is possible.
    Unknown,
}

/// Code location and semantic state required to interpret one instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnalysisPoint {
    /// Wrapping 16-bit program-counter location.
    pub location: CodeLocation,
    /// Memory-operation state at this location.
    pub t_state: TState,
}

impl AnalysisPoint {
    /// Construct an analysis point with explicit T state.
    pub const fn new(location: CodeLocation, t_state: TState) -> Self {
        Self { location, t_state }
    }
}

/// Pointer storage used to obtain a runtime control or data address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointerAddress {
    /// Two pointer bytes in logical zero page, optionally indexed by X.
    ZeroPage {
        /// Encoded low-byte base.
        base: u8,
        /// Whether X is added within zero page before the pointer is read.
        indexed_by_x: bool,
    },
    /// Two pointer bytes at an absolute address, optionally indexed by X.
    Absolute {
        /// Encoded pointer base.
        base: u16,
        /// Whether X is added before the pointer is read.
        indexed_by_x: bool,
    },
}

/// Runtime-derived HuC6280 control destination described statically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndirectTarget {
    /// JMP destination loaded through the specified pointer storage.
    JumpPointer(PointerAddress),
    /// Destination restored by a dedicated stack-return instruction.
    StackReturn(ReturnAddressRule),
    /// Handler destination loaded from the maskable-interrupt vector.
    InterruptVector {
        /// Address of the vector's low byte.
        address: u16,
    },
}

/// Stack-supplied return-address rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReturnAddressRule {
    /// RTS restores a saved address and advances to its following byte.
    Subroutine,
    /// RTI restores status and the interrupted program counter.
    Interrupt,
}

/// HuC6280 synchronous boundary kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Boundary {
    /// BRK software interrupt.
    Break,
}

/// HuC6280 programmer-visible register storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegisterLocation {
    /// Accumulator A.
    Accumulator,
    /// Index register X.
    X,
    /// Index register Y.
    Y,
    /// Stack pointer S.
    StackPointer,
}

/// Individually meaningful HuC6280 status-register storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusLocation {
    /// Negative flag N.
    Negative,
    /// Overflow flag V.
    Overflow,
    /// Memory-operation flag T.
    MemoryOperation,
    /// Break flag B.
    Break,
    /// Decimal-mode flag D.
    Decimal,
    /// Interrupt-disable flag I.
    InterruptDisable,
    /// Zero flag Z.
    Zero,
    /// Carry flag C.
    Carry,
}

/// Register used by an indexed address rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndexRegister {
    /// Index register X.
    X,
    /// Index register Y.
    Y,
}

/// Address progression used by a block transfer or static reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AddressPattern {
    /// Advance one address after each byte.
    Increment,
    /// Decrement one address after each byte.
    Decrement,
    /// Alternate between the initial address and its following address.
    Alternating,
    /// Reuse the initial address for every byte.
    Fixed,
    /// Advance while wrapping within the 256-byte logical zero page.
    ZeroPageIncrement,
}

/// Symbolic HuC6280 memory address established by typed operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryAddress {
    /// Logical zero-page address, with optional runtime indexing.
    ZeroPage {
        /// Encoded low-byte base.
        base: u8,
        /// Runtime index applied within zero page.
        index: Option<IndexRegister>,
    },
    /// Absolute logical address, with optional runtime indexing.
    Absolute {
        /// Encoded address base.
        base: u16,
        /// Runtime index added to the base.
        index: Option<IndexRegister>,
    },
    /// Address obtained by reading a pointer and applying an optional post-index.
    Indirect {
        /// Pointer storage read by the addressing mode.
        pointer: PointerAddress,
        /// Index applied after pointer dereference.
        post_index: Option<IndexRegister>,
    },
    /// Runtime stack storage in logical page 0x21.
    Stack,
    /// Memory stream selected by a block-transfer operand.
    Block {
        /// Initial logical address.
        initial: u16,
        /// Per-byte address progression.
        pattern: AddressPattern,
    },
    /// Maskable-interrupt vector storage.
    InterruptVector,
}

/// One symbolic HuC6280 memory access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryLocation {
    /// Address rule for the first accessed byte.
    pub address: MemoryAddress,
    /// Number of bytes accessed by this architectural operation.
    pub byte_len: u32,
}

/// HuC6280 fixed hardware-port destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HardwarePort {
    /// ST0 destination selected by A1:A0 = 00.
    Selector0,
    /// ST1 destination selected by A1:A0 = 10.
    Selector1,
    /// ST2 destination selected by A1:A0 = 11.
    Selector2,
}

/// Architectural HuC6280 storage location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Location {
    /// Programmer-visible register storage.
    Register(RegisterLocation),
    /// Individually meaningful status-register storage.
    Status(StatusLocation),
    /// One of the eight memory-page registers.
    MemoryPageRegister(u8),
    /// Processor clock-speed selection state.
    ClockSpeed,
    /// Symbolic logical memory storage.
    Memory(MemoryLocation),
    /// Fixed hardware port written by ST0, ST1, or ST2.
    HardwarePort(HardwarePort),
}

/// Statically resolved non-control HuC6280 reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reference {
    /// Logical memory bytes with a source-defined address progression.
    Memory {
        /// Initial logical address.
        address: u16,
        /// Number of referenced bytes.
        byte_len: u32,
        /// Per-byte address progression.
        pattern: AddressPattern,
    },
    /// Fixed hardware-port destination.
    HardwarePort(HardwarePort),
}

/// This exact profile has no structurally invalid analysis point.
pub type SemanticError = core::convert::Infallible;

/// Shared control-flow result specialized with HuC6280 vocabulary.
pub type ControlFlow = typed_isa_core::ControlFlow<AnalysisPoint, IndirectTarget, Boundary>;

/// Shared instruction-semantics result specialized with HuC6280 vocabulary.
pub type InstructionSemantics = typed_isa_core::InstructionSemantics<
    AnalysisPoint,
    IndirectTarget,
    Boundary,
    Location,
    Reference,
>;

pub(crate) fn semantics(instruction: &Instruction, at: AnalysisPoint) -> InstructionSemantics {
    let next_location = instruction.fallthrough_location(at.location);
    let mut accesses = Vec::new();
    let mut references = Vec::new();
    add_instruction_accesses(instruction, at.t_state, &mut accesses, &mut references);
    InstructionSemantics {
        control_flow: control_flow(instruction, at, next_location),
        location_accesses: accesses,
        static_references: references,
    }
}

fn point(location: u16, t_state: TState) -> AnalysisPoint {
    AnalysisPoint { location, t_state }
}

fn control_flow(instruction: &Instruction, at: AnalysisPoint, next: u16) -> ControlFlow {
    use Mnemonic::*;
    let clear_next = point(next, TState::Clear);
    match instruction.mnemonic() {
        Bpl | Bmi | Bvc | Bvs | Bcc | Bcs | Bne | Beq | Bbr { .. } | Bbs { .. } => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Direct(point(
                    relative_target(instruction, next),
                    TState::Clear,
                )),
            },
            fallthrough: Some(clear_next),
            delay_slot: None,
        },
        Bra => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Direct(point(
                    relative_target(instruction, next),
                    TState::Clear,
                )),
            },
            fallthrough: None,
            delay_slot: None,
        },
        Bsr => ControlFlow {
            action: ControlAction::LinkedTransfer {
                target: ControlTarget::Direct(point(
                    relative_target(instruction, next),
                    TState::Clear,
                )),
                return_site: clear_next,
            },
            fallthrough: None,
            delay_slot: None,
        },
        Jsr => ControlFlow {
            action: ControlAction::LinkedTransfer {
                target: ControlTarget::Direct(point(absolute_target(instruction), TState::Clear)),
                return_site: clear_next,
            },
            fallthrough: None,
            delay_slot: None,
        },
        Jmp if instruction.addressing_mode() == AddressingMode::Absolute => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Direct(point(absolute_target(instruction), TState::Clear)),
            },
            fallthrough: None,
            delay_slot: None,
        },
        Jmp => ControlFlow {
            action: ControlAction::Transfer {
                target: ControlTarget::Indirect(IndirectTarget::JumpPointer(
                    pointer_address(instruction).expect("validated indirect JMP has a pointer"),
                )),
            },
            fallthrough: None,
            delay_slot: None,
        },
        Rts => ControlFlow {
            action: ControlAction::Return {
                target: IndirectTarget::StackReturn(ReturnAddressRule::Subroutine),
            },
            fallthrough: None,
            delay_slot: None,
        },
        Rti => ControlFlow {
            action: ControlAction::ExceptionReturn {
                target: IndirectTarget::StackReturn(ReturnAddressRule::Interrupt),
            },
            fallthrough: None,
            delay_slot: None,
        },
        Brk => ControlFlow {
            action: ControlAction::Boundary(ControlBoundary::Trap {
                kind: Boundary::Break,
                target: Some(ControlTarget::Indirect(IndirectTarget::InterruptVector {
                    address: 0xfff6,
                })),
                resume: Some(point(at.location.wrapping_add(2), TState::Unknown)),
            }),
            fallthrough: None,
            delay_slot: None,
        },
        Set => ControlFlow {
            action: ControlAction::Continue,
            fallthrough: Some(point(next, TState::Set)),
            delay_slot: None,
        },
        Plp => ControlFlow {
            action: ControlAction::Continue,
            fallthrough: Some(point(next, TState::Unknown)),
            delay_slot: None,
        },
        _ => ControlFlow {
            action: ControlAction::Continue,
            fallthrough: Some(clear_next),
            delay_slot: None,
        },
    }
}

fn relative_target(instruction: &Instruction, next: u16) -> u16 {
    let displacement = match instruction.operand() {
        Operand::Relative(displacement) | Operand::ZeroPageRelative { displacement, .. } => {
            displacement
        }
        _ => unreachable!("validated branch has a relative operand"),
    };
    next.wrapping_add(displacement as i16 as u16)
}

fn absolute_target(instruction: &Instruction) -> u16 {
    let Operand::Word(target) = instruction.operand() else {
        unreachable!("validated absolute control instruction has a word operand")
    };
    target
}

fn add_instruction_accesses(
    instruction: &Instruction,
    t_state: TState,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    use Mnemonic::*;
    use RegisterLocation::{Accumulator, StackPointer, X, Y};
    use StatusLocation::{Carry, Decimal, InterruptDisable, Negative, Overflow, Zero};

    match instruction.mnemonic() {
        Adc | And | Eor | Ora => {
            add_t_selected_target(instruction, t_state, accesses, references);
            if instruction.mnemonic() == Adc {
                read_status(accesses, Carry);
                read_status(accesses, Decimal);
                write_statuses(accesses, &[Negative, Zero, Carry]);
                write_status(accesses, Overflow, AccessOccurrence::Conditional);
            } else {
                write_statuses(accesses, &[Negative, Zero]);
            }
        }
        Sbc => {
            read_register(accesses, Accumulator);
            add_operand_access(instruction, AccessKind::Read, accesses, references);
            read_status(accesses, Carry);
            read_status(accesses, Decimal);
            write_register(accesses, Accumulator);
            write_statuses(accesses, &[Negative, Zero, Carry]);
            write_status(accesses, Overflow, AccessOccurrence::Conditional);
        }
        Cmp | Cpx | Cpy => {
            read_register(
                accesses,
                match instruction.mnemonic() {
                    Cmp => Accumulator,
                    Cpx => X,
                    Cpy => Y,
                    _ => unreachable!(),
                },
            );
            add_operand_access(instruction, AccessKind::Read, accesses, references);
            write_statuses(accesses, &[Negative, Zero, Carry]);
        }
        Bit => {
            read_register(accesses, Accumulator);
            add_operand_access(instruction, AccessKind::Read, accesses, references);
            write_statuses(accesses, &[Negative, Overflow, Zero]);
        }
        Trb | Tsb => {
            read_register(accesses, Accumulator);
            add_operand_access(instruction, AccessKind::Read, accesses, references);
            add_operand_access(instruction, AccessKind::Write, accesses, references);
            write_statuses(accesses, &[Negative, Overflow, Zero]);
        }
        Tst => {
            add_operand_access(instruction, AccessKind::Read, accesses, references);
            write_statuses(accesses, &[Negative, Overflow, Zero]);
        }
        Asl | Lsr | Rol | Ror | Dec | Inc => {
            add_target_access(instruction, AccessKind::Read, accesses, references);
            if matches!(instruction.mnemonic(), Rol | Ror) {
                read_status(accesses, Carry);
            }
            add_target_access(instruction, AccessKind::Write, accesses, references);
            if matches!(instruction.mnemonic(), Asl | Lsr | Rol | Ror) {
                write_statuses(accesses, &[Negative, Zero, Carry]);
            } else {
                write_statuses(accesses, &[Negative, Zero]);
            }
        }
        Rmb { .. } | Smb { .. } => {
            add_operand_access(instruction, AccessKind::Read, accesses, references);
            add_operand_access(instruction, AccessKind::Write, accesses, references);
        }
        Lda | Ldx | Ldy => {
            add_operand_access(instruction, AccessKind::Read, accesses, references);
            write_register(
                accesses,
                match instruction.mnemonic() {
                    Lda => Accumulator,
                    Ldx => X,
                    Ldy => Y,
                    _ => unreachable!(),
                },
            );
            write_statuses(accesses, &[Negative, Zero]);
        }
        Sta | Stx | Sty => {
            read_register(
                accesses,
                match instruction.mnemonic() {
                    Sta => Accumulator,
                    Stx => X,
                    Sty => Y,
                    _ => unreachable!(),
                },
            );
            add_operand_access(instruction, AccessKind::Write, accesses, references);
        }
        Stz => add_operand_access(instruction, AccessKind::Write, accesses, references),
        Bcc | Bcs => read_status(accesses, Carry),
        Beq | Bne => read_status(accesses, Zero),
        Bmi | Bpl => read_status(accesses, Negative),
        Bvc | Bvs => read_status(accesses, Overflow),
        Bbr { .. } | Bbs { .. } => {
            add_operand_access(instruction, AccessKind::Read, accesses, references)
        }
        Brk => {
            add_stack_access(accesses, AccessKind::Write, 3);
            read_all_status(accesses);
            write_statuses(
                accesses,
                &[StatusLocation::Break, Decimal, InterruptDisable],
            );
            access(
                accesses,
                Location::Memory(MemoryLocation {
                    address: MemoryAddress::InterruptVector,
                    byte_len: 2,
                }),
                AccessKind::Read,
                AccessOccurrence::Always,
            );
            add_memory_reference(
                references,
                0xfff6,
                2,
                AddressPattern::Increment,
                ReferenceKind::Read,
                AccessOccurrence::Always,
            );
        }
        Clc | Sec => write_statuses(accesses, &[Carry]),
        Cld | Sed => write_statuses(accesses, &[Decimal]),
        Cli | Sei => write_statuses(accesses, &[InterruptDisable]),
        Clv => write_statuses(accesses, &[Overflow]),
        Cla => write_register(accesses, Accumulator),
        Clx => write_register(accesses, X),
        Cly => write_register(accesses, Y),
        Dex | Inx => {
            read_register(accesses, X);
            write_register(accesses, X);
            write_statuses(accesses, &[Negative, Zero]);
        }
        Dey | Iny => {
            read_register(accesses, Y);
            write_register(accesses, Y);
            write_statuses(accesses, &[Negative, Zero]);
        }
        Jmp => {
            if instruction.addressing_mode() != AddressingMode::Absolute {
                add_pointer_read(instruction, accesses, references);
            }
        }
        Jsr | Bsr => add_stack_access(accesses, AccessKind::Write, 2),
        Rts => add_stack_access(accesses, AccessKind::Read, 2),
        Rti => {
            add_stack_access(accesses, AccessKind::Read, 3);
            write_all_status(accesses);
        }
        Pha | Phx | Phy => {
            read_register(
                accesses,
                match instruction.mnemonic() {
                    Pha => Accumulator,
                    Phx => X,
                    Phy => Y,
                    _ => unreachable!(),
                },
            );
            add_stack_access(accesses, AccessKind::Write, 1);
        }
        Php => {
            read_all_status(accesses);
            add_stack_access(accesses, AccessKind::Write, 1);
        }
        Pla | Plx | Ply => {
            add_stack_access(accesses, AccessKind::Read, 1);
            write_register(
                accesses,
                match instruction.mnemonic() {
                    Pla => Accumulator,
                    Plx => X,
                    Ply => Y,
                    _ => unreachable!(),
                },
            );
            write_statuses(accesses, &[Negative, Zero]);
        }
        Plp => {
            add_stack_access(accesses, AccessKind::Read, 1);
            write_all_status(accesses);
        }
        Sax | Say | Sxy => {
            let pair = match instruction.mnemonic() {
                Sax => [Accumulator, X],
                Say => [Accumulator, Y],
                Sxy => [X, Y],
                _ => unreachable!(),
            };
            for register in pair {
                read_register(accesses, register);
                write_register(accesses, register);
            }
        }
        Tax | Tay | Tsx | Txa | Tya => {
            let (source, destination) = match instruction.mnemonic() {
                Tax => (Accumulator, X),
                Tay => (Accumulator, Y),
                Tsx => (StackPointer, X),
                Txa => (X, Accumulator),
                Tya => (Y, Accumulator),
                _ => unreachable!(),
            };
            read_register(accesses, source);
            write_register(accesses, destination);
            write_statuses(accesses, &[Negative, Zero]);
        }
        Txs => {
            read_register(accesses, X);
            write_register(accesses, StackPointer);
        }
        Tam | Tma => add_mpr_accesses(instruction, accesses),
        Csh | Csl => access(
            accesses,
            Location::ClockSpeed,
            AccessKind::Write,
            AccessOccurrence::Always,
        ),
        St0 | St1 | St2 => {
            let port = match instruction.mnemonic() {
                St0 => HardwarePort::Selector0,
                St1 => HardwarePort::Selector1,
                St2 => HardwarePort::Selector2,
                _ => unreachable!(),
            };
            access(
                accesses,
                Location::HardwarePort(port),
                AccessKind::Write,
                AccessOccurrence::Always,
            );
            references.push(StaticReference {
                target: Reference::HardwarePort(port),
                kind: ReferenceKind::Write,
                occurrence: AccessOccurrence::Always,
            });
        }
        Tai | Tdd | Tia | Tii | Tin => {
            add_block_transfer_accesses(instruction, accesses, references)
        }
        Bra | Nop | Set => {}
    }

    match instruction.mnemonic() {
        Set => write_statuses(accesses, &[StatusLocation::MemoryOperation]),
        Plp | Rti => {}
        _ => write_statuses(accesses, &[StatusLocation::MemoryOperation]),
    }
}

fn add_t_selected_target(
    instruction: &Instruction,
    t_state: TState,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    read_status(accesses, StatusLocation::MemoryOperation);
    add_operand_access(instruction, AccessKind::Read, accesses, references);
    let zero_page_x = Location::Memory(MemoryLocation {
        address: MemoryAddress::ZeroPage {
            base: 0,
            index: Some(IndexRegister::X),
        },
        byte_len: 1,
    });
    match t_state {
        TState::Clear => {
            read_register(accesses, RegisterLocation::Accumulator);
            write_register(accesses, RegisterLocation::Accumulator);
        }
        TState::Set => {
            read_register(accesses, RegisterLocation::X);
            access(
                accesses,
                zero_page_x,
                AccessKind::Read,
                AccessOccurrence::Always,
            );
            access(
                accesses,
                zero_page_x,
                AccessKind::Write,
                AccessOccurrence::Always,
            );
        }
        TState::Unknown => {
            access(
                accesses,
                Location::Register(RegisterLocation::Accumulator),
                AccessKind::Read,
                AccessOccurrence::Conditional,
            );
            access(
                accesses,
                Location::Register(RegisterLocation::Accumulator),
                AccessKind::Write,
                AccessOccurrence::Conditional,
            );
            access(
                accesses,
                Location::Register(RegisterLocation::X),
                AccessKind::Read,
                AccessOccurrence::Conditional,
            );
            access(
                accesses,
                zero_page_x,
                AccessKind::Read,
                AccessOccurrence::Conditional,
            );
            access(
                accesses,
                zero_page_x,
                AccessKind::Write,
                AccessOccurrence::Conditional,
            );
        }
    }
}

fn add_target_access(
    instruction: &Instruction,
    kind: AccessKind,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    if instruction.addressing_mode() == AddressingMode::Accumulator {
        access(
            accesses,
            Location::Register(RegisterLocation::Accumulator),
            kind,
            AccessOccurrence::Always,
        );
    } else {
        add_operand_access(instruction, kind, accesses, references);
    }
}

fn add_operand_access(
    instruction: &Instruction,
    kind: AccessKind,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    add_index_read(instruction.addressing_mode(), accesses);
    if pointer_address(instruction).is_some() {
        add_pointer_read(instruction, accesses, references);
    }
    let Some(memory) = data_memory_location(instruction) else {
        return;
    };
    access(
        accesses,
        Location::Memory(memory),
        kind,
        AccessOccurrence::Always,
    );
    if let Some((address, pattern)) = direct_reference(memory) {
        add_memory_reference(
            references,
            address,
            memory.byte_len,
            pattern,
            match kind {
                AccessKind::Read => ReferenceKind::Read,
                AccessKind::Write => ReferenceKind::Write,
            },
            AccessOccurrence::Always,
        );
    }
}

fn add_index_read(mode: AddressingMode, accesses: &mut Vec<LocationAccess<Location>>) {
    use AddressingMode::*;
    match mode {
        ZeroPageX
        | AbsoluteX
        | ZeroPageIndexedIndirectX
        | AbsoluteIndexedIndirectX
        | ImmediateZeroPageX
        | ImmediateAbsoluteX => read_register(accesses, RegisterLocation::X),
        ZeroPageY | AbsoluteY | ZeroPageIndirectIndexedY => {
            read_register(accesses, RegisterLocation::Y)
        }
        _ => {}
    }
}

fn data_memory_location(instruction: &Instruction) -> Option<MemoryLocation> {
    use AddressingMode::*;
    let address = match (instruction.addressing_mode(), instruction.operand()) {
        (ZeroPage, Operand::Byte(base))
        | (ZeroPageRelative, Operand::ZeroPageRelative { address: base, .. })
        | (ImmediateZeroPage, Operand::ImmediateZeroPage { address: base, .. }) => {
            MemoryAddress::ZeroPage { base, index: None }
        }
        (ZeroPageX, Operand::Byte(base))
        | (ImmediateZeroPageX, Operand::ImmediateZeroPage { address: base, .. }) => {
            MemoryAddress::ZeroPage {
                base,
                index: Some(IndexRegister::X),
            }
        }
        (ZeroPageY, Operand::Byte(base)) => MemoryAddress::ZeroPage {
            base,
            index: Some(IndexRegister::Y),
        },
        (Absolute, Operand::Word(base))
        | (ImmediateAbsolute, Operand::ImmediateAbsolute { address: base, .. }) => {
            MemoryAddress::Absolute { base, index: None }
        }
        (AbsoluteX, Operand::Word(base))
        | (ImmediateAbsoluteX, Operand::ImmediateAbsolute { address: base, .. }) => {
            MemoryAddress::Absolute {
                base,
                index: Some(IndexRegister::X),
            }
        }
        (AbsoluteY, Operand::Word(base)) => MemoryAddress::Absolute {
            base,
            index: Some(IndexRegister::Y),
        },
        (ZeroPageIndexedIndirectX, _) | (ZeroPageIndirect, _) | (ZeroPageIndirectIndexedY, _) => {
            MemoryAddress::Indirect {
                pointer: pointer_address(instruction).expect("indirect mode has a pointer"),
                post_index: (instruction.addressing_mode() == ZeroPageIndirectIndexedY)
                    .then_some(IndexRegister::Y),
            }
        }
        _ => return None,
    };
    Some(MemoryLocation {
        address,
        byte_len: 1,
    })
}

fn pointer_address(instruction: &Instruction) -> Option<PointerAddress> {
    use AddressingMode::*;
    match (instruction.addressing_mode(), instruction.operand()) {
        (ZeroPageIndirect, Operand::Byte(base))
        | (ZeroPageIndirectIndexedY, Operand::Byte(base)) => Some(PointerAddress::ZeroPage {
            base,
            indexed_by_x: false,
        }),
        (ZeroPageIndexedIndirectX, Operand::Byte(base)) => Some(PointerAddress::ZeroPage {
            base,
            indexed_by_x: true,
        }),
        (AbsoluteIndirect, Operand::Word(base)) => Some(PointerAddress::Absolute {
            base,
            indexed_by_x: false,
        }),
        (AbsoluteIndexedIndirectX, Operand::Word(base)) => Some(PointerAddress::Absolute {
            base,
            indexed_by_x: true,
        }),
        _ => None,
    }
}

fn add_pointer_read(
    instruction: &Instruction,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    let pointer = pointer_address(instruction).expect("indirect mode has pointer storage");
    let (memory, reference) = match pointer {
        PointerAddress::ZeroPage { base, indexed_by_x } => (
            MemoryLocation {
                address: MemoryAddress::ZeroPage {
                    base,
                    index: indexed_by_x.then_some(IndexRegister::X),
                },
                byte_len: 2,
            },
            (!indexed_by_x)
                .then_some((0x2000 | u16::from(base), AddressPattern::ZeroPageIncrement)),
        ),
        PointerAddress::Absolute { base, indexed_by_x } => (
            MemoryLocation {
                address: MemoryAddress::Absolute {
                    base,
                    index: indexed_by_x.then_some(IndexRegister::X),
                },
                byte_len: 2,
            },
            (!indexed_by_x).then_some((base, AddressPattern::Increment)),
        ),
    };
    access(
        accesses,
        Location::Memory(memory),
        AccessKind::Read,
        AccessOccurrence::Always,
    );
    if let Some((address, pattern)) = reference {
        add_memory_reference(
            references,
            address,
            2,
            pattern,
            ReferenceKind::Read,
            AccessOccurrence::Always,
        );
    }
}

fn direct_reference(memory: MemoryLocation) -> Option<(u16, AddressPattern)> {
    match memory.address {
        MemoryAddress::ZeroPage { base, index: None } => {
            Some((0x2000 | u16::from(base), AddressPattern::ZeroPageIncrement))
        }
        MemoryAddress::Absolute { base, index: None } => Some((base, AddressPattern::Increment)),
        MemoryAddress::InterruptVector => Some((0xfff6, AddressPattern::Increment)),
        _ => None,
    }
}

fn add_mpr_accesses(instruction: &Instruction, accesses: &mut Vec<LocationAccess<Location>>) {
    let Operand::Byte(mask) = instruction.operand() else {
        unreachable!("validated TAM/TMA has an immediate mask")
    };
    if instruction.mnemonic() == Mnemonic::Tam {
        read_register(accesses, RegisterLocation::Accumulator);
    }
    for index in 0..8 {
        if mask & (1 << index) != 0 {
            access(
                accesses,
                Location::MemoryPageRegister(index),
                if instruction.mnemonic() == Mnemonic::Tam {
                    AccessKind::Write
                } else {
                    AccessKind::Read
                },
                AccessOccurrence::Always,
            );
        }
    }
    if instruction.mnemonic() == Mnemonic::Tma {
        write_register(accesses, RegisterLocation::Accumulator);
    }
}

fn add_block_transfer_accesses(
    instruction: &Instruction,
    accesses: &mut Vec<LocationAccess<Location>>,
    references: &mut Vec<StaticReference<Reference>>,
) {
    let Operand::BlockTransfer {
        source,
        destination,
        length,
    } = instruction.operand()
    else {
        unreachable!("validated block transfer has three words")
    };
    let transfer_len = if length == 0 {
        65_536
    } else {
        u32::from(length)
    };
    let (source_pattern, destination_pattern) = match instruction.mnemonic() {
        Mnemonic::Tai => (AddressPattern::Alternating, AddressPattern::Increment),
        Mnemonic::Tdd => (AddressPattern::Decrement, AddressPattern::Decrement),
        Mnemonic::Tia => (AddressPattern::Increment, AddressPattern::Alternating),
        Mnemonic::Tii => (AddressPattern::Increment, AddressPattern::Increment),
        Mnemonic::Tin => (AddressPattern::Increment, AddressPattern::Fixed),
        _ => unreachable!(),
    };
    for register in [
        RegisterLocation::Accumulator,
        RegisterLocation::X,
        RegisterLocation::Y,
    ] {
        read_register(accesses, register);
        write_register(accesses, register);
    }
    add_stack_access(accesses, AccessKind::Write, 3);
    add_stack_access(accesses, AccessKind::Read, 3);
    for (initial, pattern, kind) in [
        (source, source_pattern, AccessKind::Read),
        (destination, destination_pattern, AccessKind::Write),
    ] {
        access(
            accesses,
            Location::Memory(MemoryLocation {
                address: MemoryAddress::Block { initial, pattern },
                byte_len: transfer_len,
            }),
            kind,
            AccessOccurrence::Always,
        );
        add_memory_reference(
            references,
            initial,
            transfer_len,
            pattern,
            match kind {
                AccessKind::Read => ReferenceKind::Read,
                AccessKind::Write => ReferenceKind::Write,
            },
            AccessOccurrence::Always,
        );
    }
}

fn add_stack_access(accesses: &mut Vec<LocationAccess<Location>>, kind: AccessKind, byte_len: u32) {
    read_register(accesses, RegisterLocation::StackPointer);
    write_register(accesses, RegisterLocation::StackPointer);
    access(
        accesses,
        Location::Memory(MemoryLocation {
            address: MemoryAddress::Stack,
            byte_len,
        }),
        kind,
        AccessOccurrence::Always,
    );
}

fn add_memory_reference(
    references: &mut Vec<StaticReference<Reference>>,
    address: u16,
    byte_len: u32,
    pattern: AddressPattern,
    kind: ReferenceKind,
    occurrence: AccessOccurrence,
) {
    references.push(StaticReference {
        target: Reference::Memory {
            address,
            byte_len,
            pattern,
        },
        kind,
        occurrence,
    });
}

fn read_register(accesses: &mut Vec<LocationAccess<Location>>, register: RegisterLocation) {
    access(
        accesses,
        Location::Register(register),
        AccessKind::Read,
        AccessOccurrence::Always,
    );
}

fn write_register(accesses: &mut Vec<LocationAccess<Location>>, register: RegisterLocation) {
    access(
        accesses,
        Location::Register(register),
        AccessKind::Write,
        AccessOccurrence::Always,
    );
}

fn read_status(accesses: &mut Vec<LocationAccess<Location>>, status: StatusLocation) {
    access(
        accesses,
        Location::Status(status),
        AccessKind::Read,
        AccessOccurrence::Always,
    );
}

fn write_statuses(accesses: &mut Vec<LocationAccess<Location>>, statuses: &[StatusLocation]) {
    for status in statuses {
        write_status(accesses, *status, AccessOccurrence::Always);
    }
}

fn write_status(
    accesses: &mut Vec<LocationAccess<Location>>,
    status: StatusLocation,
    occurrence: AccessOccurrence,
) {
    access(
        accesses,
        Location::Status(status),
        AccessKind::Write,
        occurrence,
    );
}

fn read_all_status(accesses: &mut Vec<LocationAccess<Location>>) {
    for status in status_locations() {
        read_status(accesses, status);
    }
}

fn write_all_status(accesses: &mut Vec<LocationAccess<Location>>) {
    write_statuses(accesses, &status_locations());
}

const fn status_locations() -> [StatusLocation; 8] {
    use StatusLocation::*;
    [
        Negative,
        Overflow,
        MemoryOperation,
        Break,
        Decimal,
        InterruptDisable,
        Zero,
        Carry,
    ]
}

fn access(
    accesses: &mut Vec<LocationAccess<Location>>,
    location: Location,
    kind: AccessKind,
    occurrence: AccessOccurrence,
) {
    accesses.push(LocationAccess {
        location,
        kind,
        occurrence,
    });
}

pub(crate) fn storage_relation(left: &Location, right: &Location) -> StorageRelation {
    if left == right {
        return StorageRelation::Same;
    }
    match (left, right) {
        (Location::Memory(left), Location::Memory(right)) => memory_relation(*left, *right),
        (Location::Memory(_), _) | (_, Location::Memory(_)) => StorageRelation::Disjoint,
        _ => StorageRelation::Disjoint,
    }
}

fn memory_relation(left: MemoryLocation, right: MemoryLocation) -> StorageRelation {
    let Some(left_bytes) = small_known_bytes(left) else {
        return StorageRelation::Unknown;
    };
    let Some(right_bytes) = small_known_bytes(right) else {
        return StorageRelation::Unknown;
    };
    if left_bytes == right_bytes {
        StorageRelation::Same
    } else if left_bytes
        .iter()
        .any(|left_byte| right_bytes.contains(left_byte))
    {
        StorageRelation::Overlapping
    } else {
        StorageRelation::Disjoint
    }
}

fn small_known_bytes(memory: MemoryLocation) -> Option<Vec<u16>> {
    if memory.byte_len > 8 {
        return None;
    }
    let (start, pattern) = direct_reference(memory)?;
    Some(
        (0..memory.byte_len)
            .map(|offset| match pattern {
                AddressPattern::Increment => start.wrapping_add(offset as u16),
                AddressPattern::Decrement => start.wrapping_sub(offset as u16),
                AddressPattern::Alternating => start.wrapping_add((offset & 1) as u16),
                AddressPattern::Fixed => start,
                AddressPattern::ZeroPageIncrement => {
                    0x2000 | u16::from((start as u8).wrapping_add(offset as u8))
                }
            })
            .collect(),
    )
}
