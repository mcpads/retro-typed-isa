//! Motorola M68000 Family Programmer's Reference Manual (1992), CLR pp. 4-73–74,
//! MOVE from SR p. 4-125, and Scc pp. 4-172–173 explicitly require a destination
//! memory read before the write on MC68000. Register destinations do not read
//! their old value. These tests enumerate the manual's data-alterable modes,
//! operation sizes, and condition selectors independently of the form census.
//!
//! https://www.nxp.com/docs/en/reference-manual/M68000PRM.pdf

use m68000::{
    AddressRegister, AddressRegisterLocation, AnalysisPoint, CodeLocation, Condition, DataRegister,
    EffectiveAddress, IndexRegister, IndexSize, Instruction, Location, Mc68000, MemoryAddress,
    MemoryLocation, OperandSize, PrivilegeMode, Reference, Register,
};
use typed_isa_core::{
    AccessKind, AccessOccurrence, LocationAccess, ReferenceKind, StaticReference, StaticSemantics,
};

const SIZES: [(OperandSize, u8); 3] = [
    (OperandSize::Byte, 1),
    (OperandSize::Word, 2),
    (OperandSize::Long, 4),
];

const CONDITIONS: [Condition; 16] = [
    Condition::True,
    Condition::False,
    Condition::High,
    Condition::LowOrSame,
    Condition::CarryClear,
    Condition::CarrySet,
    Condition::NotEqual,
    Condition::Equal,
    Condition::OverflowClear,
    Condition::OverflowSet,
    Condition::Plus,
    Condition::Minus,
    Condition::GreaterOrEqual,
    Condition::LessThan,
    Condition::GreaterThan,
    Condition::LessOrEqual,
];

fn point() -> AnalysisPoint {
    AnalysisPoint::new(CodeLocation::new(0x1000), PrivilegeMode::Supervisor)
}

fn memory_destinations() -> [(EffectiveAddress, MemoryAddress); 7] {
    let base = AddressRegisterLocation::Register(AddressRegister::A0);
    let index = IndexRegister {
        register: Register::Data(DataRegister::D1),
        size: IndexSize::Word,
    };
    [
        (
            EffectiveAddress::AddressIndirect(AddressRegister::A0),
            MemoryAddress::AddressRegister(base),
        ),
        (
            EffectiveAddress::AddressPostincrement(AddressRegister::A0),
            MemoryAddress::Postincrement(base),
        ),
        (
            EffectiveAddress::AddressPredecrement(AddressRegister::A0),
            MemoryAddress::Predecrement(base),
        ),
        (
            EffectiveAddress::AddressDisplacement {
                register: AddressRegister::A0,
                displacement: -8,
            },
            MemoryAddress::Displacement {
                base,
                displacement: -8,
            },
        ),
        (
            EffectiveAddress::AddressIndex {
                register: AddressRegister::A0,
                index,
                displacement: 6,
            },
            MemoryAddress::Indexed {
                base,
                index,
                displacement: 6,
            },
        ),
        (
            EffectiveAddress::AbsoluteWord(0xff00),
            MemoryAddress::Absolute(0xffff_ff00),
        ),
        (
            EffectiveAddress::AbsoluteLong(0x00c0_0000),
            MemoryAddress::Absolute(0x00c0_0000),
        ),
    ]
}

fn assert_destination_read_write(instruction: Instruction, address: MemoryAddress, byte_len: u8) {
    let semantics = Mc68000::semantics(&instruction, &point()).unwrap();
    let memory = MemoryLocation {
        address,
        element_byte_len: byte_len,
        element_count: 1,
        stride: byte_len,
    };
    let accesses: Vec<_> = semantics
        .location_accesses
        .iter()
        .filter(|access| matches!(access.location, Location::Memory(_)))
        .cloned()
        .collect();
    assert_eq!(accesses.len(), 2, "{instruction:?}");
    // This is an unordered access summary, not a bus-cycle sequence.
    for kind in [AccessKind::Read, AccessKind::Write] {
        assert!(
            accesses.contains(&LocationAccess {
                location: Location::Memory(memory),
                kind,
                occurrence: AccessOccurrence::Always,
            }),
            "{instruction:?}: missing {kind:?} of {memory:?}",
        );
    }
    if let MemoryAddress::Absolute(address) = address {
        assert_eq!(semantics.static_references.len(), 2, "{instruction:?}");
        for kind in [ReferenceKind::Read, ReferenceKind::Write] {
            assert!(semantics.static_references.contains(&StaticReference {
                target: Reference::Memory { address, byte_len },
                kind,
                occurrence: AccessOccurrence::Always,
            }));
        }
    } else {
        assert!(semantics.static_references.is_empty());
    }
}

#[test]
fn clr_reads_every_memory_destination_at_the_operation_width() {
    for (destination, address) in memory_destinations() {
        for (size, byte_len) in SIZES {
            assert_destination_read_write(
                Instruction::Clear { size, destination },
                address,
                byte_len,
            );
        }
    }
}

#[test]
fn scc_reads_every_memory_destination_for_every_condition() {
    for (destination, address) in memory_destinations() {
        for condition in CONDITIONS {
            assert_destination_read_write(
                Instruction::SetCondition {
                    condition,
                    destination,
                },
                address,
                1,
            );
        }
    }
}

#[test]
fn move_from_sr_reads_every_memory_destination_as_a_word() {
    for (destination, address) in memory_destinations() {
        assert_destination_read_write(Instruction::MoveFromStatus { destination }, address, 2);
    }
}

#[test]
fn register_destinations_do_not_read_the_previous_result() {
    for register in [
        DataRegister::D0,
        DataRegister::D1,
        DataRegister::D2,
        DataRegister::D3,
        DataRegister::D4,
        DataRegister::D5,
        DataRegister::D6,
        DataRegister::D7,
    ] {
        let destination = EffectiveAddress::DataRegister(register);
        let instructions = SIZES
            .map(|(size, byte_len)| (Instruction::Clear { size, destination }, byte_len))
            .into_iter()
            .chain([(Instruction::MoveFromStatus { destination }, 2)])
            .chain(CONDITIONS.map(|condition| {
                (
                    Instruction::SetCondition {
                        condition,
                        destination,
                    },
                    1,
                )
            }));
        for (instruction, byte_len) in instructions {
            let semantics = Mc68000::semantics(&instruction, &point()).unwrap();
            let accesses: Vec<_> = semantics
                .location_accesses
                .iter()
                .filter(|access| {
                    matches!(
                        access.location,
                        Location::DataRegister { .. } | Location::Memory(_)
                    )
                })
                .cloned()
                .collect();
            assert_eq!(
                accesses,
                [LocationAccess {
                    location: Location::DataRegister { register, byte_len },
                    kind: AccessKind::Write,
                    occurrence: AccessOccurrence::Always,
                }],
                "{instruction:?}",
            );
            assert!(semantics.static_references.is_empty());
        }
    }
}

#[test]
fn ordinary_move_does_not_gain_a_destination_read() {
    // MC68000 User's Manual, section 8, Tables 8-2 and 8-3: MOVE stores do
    // not have the extra destination operand fetch required by CLR/Scc/SR.
    // https://www.nxp.com/docs/en/reference-manual/MC68000UM.pdf
    for (destination, address) in memory_destinations() {
        for (size, byte_len) in SIZES {
            let instruction = Instruction::Move {
                size,
                source: EffectiveAddress::DataRegister(DataRegister::D2),
                destination,
            };
            let semantics = Mc68000::semantics(&instruction, &point()).unwrap();
            let accesses: Vec<_> = semantics
                .location_accesses
                .iter()
                .filter(|access| matches!(access.location, Location::Memory(_)))
                .cloned()
                .collect();
            assert_eq!(
                accesses,
                [LocationAccess {
                    location: Location::Memory(MemoryLocation {
                        address,
                        element_byte_len: byte_len,
                        element_count: 1,
                        stride: byte_len,
                    }),
                    kind: AccessKind::Write,
                    occurrence: AccessOccurrence::Always,
                }],
                "{instruction:?}",
            );
            assert!(
                semantics
                    .static_references
                    .iter()
                    .all(|reference| { reference.kind == ReferenceKind::Write })
            );
        }
    }
}
