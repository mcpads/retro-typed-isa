use i386::{
    Address16Base, AddressError, AddressSize, CodeMode, Displacement, EffectiveAddress,
    OperandSize, Prefix, PrefixSequence, Register32, Scale, ScaledIndex,
};

#[test]
fn code_mode_and_size_overrides_select_the_effective_static_sizes() {
    let overrides = PrefixSequence::new(vec![
        Prefix::OperandSizeOverride,
        Prefix::AddressSizeOverride,
    ])
    .unwrap();
    assert_eq!(
        overrides.effective_operand_size(CodeMode::Code16),
        OperandSize::Doubleword
    );
    assert_eq!(
        overrides.effective_address_size(CodeMode::Code16),
        AddressSize::Bits32
    );
    assert_eq!(
        overrides.effective_operand_size(CodeMode::Code32),
        OperandSize::Word
    );
    assert_eq!(
        overrides.effective_address_size(CodeMode::Code32),
        AddressSize::Bits16
    );
}

#[test]
fn duplicate_size_prefixes_do_not_cancel_the_override() {
    let prefixes = PrefixSequence::new(vec![
        Prefix::OperandSizeOverride,
        Prefix::OperandSizeOverride,
        Prefix::AddressSizeOverride,
        Prefix::AddressSizeOverride,
    ])
    .unwrap();
    assert_eq!(
        prefixes.effective_operand_size(CodeMode::Code16),
        OperandSize::Doubleword
    );
    assert_eq!(
        prefixes.effective_address_size(CodeMode::Code16),
        AddressSize::Bits32
    );
}

#[test]
fn effective_addresses_reject_selector_combinations_without_i386_encodings() {
    assert_eq!(
        EffectiveAddress::bits16(Address16Base::Direct, Displacement::None),
        Err(AddressError::Direct16RequiresWord)
    );
    assert_eq!(
        ScaledIndex::new(Register32::Esp, Scale::Two),
        Err(AddressError::StackPointerCannotBeIndex)
    );
    assert_eq!(
        EffectiveAddress::bits16(Address16Base::Bp, Displacement::None),
        Err(AddressError::BasePointerRequiresDisplacement)
    );
    assert_eq!(
        EffectiveAddress::bits32(Some(Register32::Ebp), None, Displacement::None, false,),
        Err(AddressError::BasePointerRequiresDisplacement)
    );
    assert!(
        EffectiveAddress::bits32(None, None, Displacement::AbsoluteDoubleword(0x1234), true,)
            .is_ok(),
        "SIB base=5/index=4 encodes an alternate disp32-only address"
    );
}
