use powerpc_gekko_750cl::{
    AssemblerError, DecodeError, EncodeError, FloatingRegister, GekkoAssembler, GeneralRegister,
    Instruction, InstructionForm, InstructionModifiers, Operand, PowerPc750Cl, PowerPcGekko,
    SpecialPurposeRegister, decode_750cl_word, decode_gekko_word, encode_750cl_word,
};
use typed_isa_core::TypedIsa;

#[test]
fn ps_merge10_detail_encoding_beats_the_appendix_typo() {
    // IBM Appendix B Table B-1 misprints the selector. The instruction detail
    // on page 583, the timing table on page 242, and Table B-4 all agree on
    // XO 592 and the word below.
    let instruction = Instruction::with_modifiers(
        InstructionForm::PsMerge10,
        InstructionModifiers::default(),
        [
            Operand::FloatingRegister(FloatingRegister::new(2).unwrap()),
            Operand::FloatingRegister(FloatingRegister::new(3).unwrap()),
            Operand::FloatingRegister(FloatingRegister::new(4).unwrap()),
        ],
    )
    .unwrap();

    assert_eq!(encode_750cl_word(&instruction, 0), Ok(0x1043_24a0));
}

#[test]
fn dcba_is_not_admitted_to_the_gekko_compatible_profile() {
    // Dolphin commit 66d8a892 retains selector 758 in a generic dispatch
    // table, but identifies it as not a Gekko instruction. devkitPro binutils
    // commit 55113f8e also excludes dcba from PPC750 targets.
    let word = 0x7c00_05ec;
    assert_eq!(
        decode_750cl_word(word, 0),
        Err(DecodeError::ReservedWord { word })
    );
}

#[test]
fn gekko_rejects_the_registers_added_by_750cl() {
    // IBM Gekko Tables 2-52/2-53 and IBM 750CL Tables 2-55/2-56 differ by
    // four extra IBAT/DBAT pairs, HID4, and the two thermal calibration SPRs.
    const ADDED_BY_750CL: [SpecialPurposeRegister; 19] = [
        SpecialPurposeRegister::InstructionBat4Upper,
        SpecialPurposeRegister::InstructionBat4Lower,
        SpecialPurposeRegister::InstructionBat5Upper,
        SpecialPurposeRegister::InstructionBat5Lower,
        SpecialPurposeRegister::InstructionBat6Upper,
        SpecialPurposeRegister::InstructionBat6Lower,
        SpecialPurposeRegister::InstructionBat7Upper,
        SpecialPurposeRegister::InstructionBat7Lower,
        SpecialPurposeRegister::DataBat4Upper,
        SpecialPurposeRegister::DataBat4Lower,
        SpecialPurposeRegister::DataBat5Upper,
        SpecialPurposeRegister::DataBat5Lower,
        SpecialPurposeRegister::DataBat6Upper,
        SpecialPurposeRegister::DataBat6Lower,
        SpecialPurposeRegister::DataBat7Upper,
        SpecialPurposeRegister::DataBat7Lower,
        SpecialPurposeRegister::HardwareImplementation4,
        SpecialPurposeRegister::ThermalDataCacheLower,
        SpecialPurposeRegister::ThermalDataCacheUpper,
    ];

    let destination = Operand::GeneralRegister(GeneralRegister::new(3).unwrap());
    for register in ADDED_BY_750CL {
        let instruction = Instruction::new(
            InstructionForm::Mfspr,
            [destination, Operand::SpecialPurposeRegister(register)],
        )
        .unwrap();
        let word = PowerPc750Cl::encode_one(&instruction, &0)
            .unwrap()
            .as_slice()
            .try_into()
            .map(u32::from_be_bytes)
            .unwrap();

        assert_eq!(
            PowerPcGekko::encode_one(&instruction, &0),
            Err(EncodeError::GekkoUnavailableSpecialPurposeRegister { register })
        );
        assert_eq!(
            decode_gekko_word(word, 0),
            Err(DecodeError::GekkoUnavailableSpecialPurposeRegister { register })
        );
    }
}

#[test]
fn gekko_assembler_enforces_the_selected_processor_profile() {
    let instruction = Instruction::new(
        InstructionForm::Mfspr,
        [
            Operand::GeneralRegister(GeneralRegister::new(3).unwrap()),
            Operand::SpecialPurposeRegister(SpecialPurposeRegister::HardwareImplementation4),
        ],
    )
    .unwrap();
    let mut assembler = GekkoAssembler::new();
    assembler.emit(instruction);

    assert_eq!(
        assembler.assemble(0),
        Err(AssemblerError::Encode(
            EncodeError::GekkoUnavailableSpecialPurposeRegister {
                register: SpecialPurposeRegister::HardwareImplementation4,
            }
        ))
    );
}
