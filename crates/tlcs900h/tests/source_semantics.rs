use tlcs900h::{
    AnalysisPoint, Boundary, Condition, CurrentBank, CurrentRegister, DestinationMemoryInstruction,
    DestinationMemoryOperation, EffectiveAddress, GeneralRegister, ImplicitState, Instruction,
    Location, MainInstruction, MemoryAddress, MemoryLocation, OperandSize, Reference,
    RegisterLocation, SourceMemoryInstruction, SourceMemoryOperation, Tlcs900H,
};
use typed_isa_core::{
    AccessKind, ControlAction, ControlBoundary, ControlTarget, ReferenceKind, StaticSemantics,
    StorageRelation,
};

fn semantics(instruction: Instruction, location: u32) -> tlcs900h::InstructionSemantics {
    Tlcs900H::semantics(&instruction, &AnalysisPoint::new(location)).unwrap()
}

#[test]
fn appendix_c_relative_branch_uses_the_post_instruction_pc() {
    let result = semantics(
        Instruction::Main(MainInstruction::JumpRelative {
            condition: Condition::True,
            displacement: 1,
        }),
        0x1000,
    );
    assert!(matches!(
        result.control_flow.action,
        ControlAction::Transfer {
            target: ControlTarget::Direct(AnalysisPoint {
                location: 0x1003,
                ..
            })
        }
    ));
    assert_eq!(result.control_flow.fallthrough, None);
    assert!(result.static_references.is_empty());
}

#[test]
fn appendix_c_false_branch_has_no_spurious_transfer() {
    let result = semantics(
        Instruction::Main(MainInstruction::JumpRelative {
            condition: Condition::False,
            displacement: 1,
        }),
        0x1000,
    );
    assert!(matches!(
        result.control_flow.action,
        ControlAction::Continue
    ));
    assert_eq!(result.control_flow.fallthrough.unwrap().location, 0x1002);
}

#[test]
fn resolved_memory_jump_uses_only_the_control_flow_channel() {
    let result = semantics(
        Instruction::DestinationMemory(DestinationMemoryOperation {
            destination: MemoryAddress::PcRelative(4),
            instruction: DestinationMemoryInstruction::Jump(Condition::True),
        }),
        0x1000,
    );
    let ControlAction::Transfer {
        target: ControlTarget::Direct(target),
    } = result.control_flow.action
    else {
        panic!("expected a resolved direct transfer")
    };
    assert_eq!(target.location, 0x1009);
    assert!(result.static_references.is_empty());
}

#[test]
fn appendix_c_halt_is_a_resumable_wait_boundary() {
    let result = semantics(Instruction::Main(MainInstruction::Halt), 0x1000);
    assert!(matches!(
        result.control_flow.action,
        ControlAction::Boundary(ControlBoundary::Wait {
            kind: Boundary::Halt,
            resume: AnalysisPoint {
                location: 0x1001,
                ..
            }
        })
    ));
}

#[test]
fn stack_push_names_stack_state_without_fabricating_a_general_register() {
    let result = semantics(
        Instruction::Main(MainInstruction::PushImmediateWord(0x1234)),
        0x1000,
    );
    assert!(result.location_accesses.iter().any(|access| {
        access.location == Location::Implicit(ImplicitState::StackPointer)
            && access.kind == AccessKind::Write
    }));
    assert!(result.location_accesses.iter().any(|access| {
        access.location
            == Location::Memory(MemoryLocation {
                address: EffectiveAddress::Stack,
                byte_len: 2,
            })
            && access.kind == AccessKind::Write
    }));
}

#[test]
fn pc_relative_memory_exposes_a_resolved_external_reference() {
    let result = semantics(
        Instruction::SourceMemory(SourceMemoryOperation {
            size: OperandSize::Word,
            source: tlcs900h::MemoryAddress::PcRelative(4),
            instruction: SourceMemoryInstruction::Push,
        }),
        0x1000,
    );
    assert!(result.static_references.iter().any(|reference| {
        matches!(reference.target, Reference::Memory { byte_len: 2, .. })
            && reference.kind == ReferenceKind::Read
    }));
}

#[test]
fn extended_register_ranges_report_partial_overlap() {
    let byte = Location::Register(RegisterLocation::Extended {
        register: GeneralRegister::new(0).unwrap(),
        size: OperandSize::Byte,
    });
    let word = Location::Register(RegisterLocation::Extended {
        register: GeneralRegister::new(0).unwrap(),
        size: OperandSize::Word,
    });
    assert_eq!(
        Tlcs900H::storage_relation(&byte, &word),
        StorageRelation::Overlapping
    );
}

#[test]
fn unresolved_current_bank_does_not_claim_disjointness_from_extended_storage() {
    let current = Location::Register(RegisterLocation::Current {
        bank: CurrentBank::Unknown,
        register: CurrentRegister::new(0).unwrap(),
        size: OperandSize::Long,
    });
    let extended = Location::Register(RegisterLocation::Extended {
        register: GeneralRegister::new(0).unwrap(),
        size: OperandSize::Long,
    });
    assert_eq!(
        Tlcs900H::storage_relation(&current, &extended),
        StorageRelation::Unknown
    );
}
