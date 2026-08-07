use i386::{
    ModRmSelectorClass, PrimaryOpcodeClass, SecondaryOpcodeClass, classify_primary_modrm_selector,
    classify_primary_opcode, classify_secondary_modrm_selector, classify_secondary_opcode,
};

// Intel 80386 Programmer's Reference Manual, Appendix A, one-byte map.
const MANUAL_PREFIXES: [u8; 11] = [
    0x26, 0x2e, 0x36, 0x3e, 0x64, 0x65, 0x66, 0x67, 0xf0, 0xf2, 0xf3,
];
const MANUAL_UNDEFINED_PRIMARY: [u8; 3] = [0x82, 0xd6, 0xf1];

// Intel 80386 Programmer's Reference Manual, Appendix A, two-byte map. This
// list deliberately omits undocumented LOADALL and every 80486-or-later form.
const MANUAL_SECONDARY_INSTRUCTIONS: [u8; 66] = [
    0x00, 0x01, 0x02, 0x03, 0x06, 0x20, 0x21, 0x22, 0x23, 0x24, 0x26, 0x80, 0x81, 0x82, 0x83, 0x84,
    0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x8b, 0x8c, 0x8d, 0x8e, 0x8f, 0x90, 0x91, 0x92, 0x93, 0x94,
    0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0x9b, 0x9c, 0x9d, 0x9e, 0x9f, 0xa0, 0xa1, 0xa3, 0xa4, 0xa5,
    0xa8, 0xa9, 0xab, 0xac, 0xad, 0xaf, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xba, 0xbb, 0xbc, 0xbd,
    0xbe, 0xbf,
];

#[test]
fn public_primary_classification_matches_every_manual_map_cell() {
    for opcode in u8::MIN..=u8::MAX {
        let expected = if MANUAL_PREFIXES.contains(&opcode) {
            "prefix"
        } else if MANUAL_UNDEFINED_PRIMARY.contains(&opcode) {
            "undefined"
        } else if opcode == 0x0f {
            "secondary"
        } else {
            "instruction"
        };
        let actual = match classify_primary_opcode(opcode) {
            PrimaryOpcodeClass::Instruction => "instruction",
            PrimaryOpcodeClass::Prefix(_) => "prefix",
            PrimaryOpcodeClass::SecondaryEscape => "secondary",
            PrimaryOpcodeClass::Undefined => "undefined",
        };
        assert_eq!(actual, expected, "primary opcode {opcode:#04x}");
    }
}

#[test]
fn public_secondary_classification_rejects_every_cell_absent_from_the_manual() {
    for opcode in u8::MIN..=u8::MAX {
        assert_eq!(
            classify_secondary_opcode(opcode),
            if MANUAL_SECONDARY_INSTRUCTIONS.contains(&opcode) {
                SecondaryOpcodeClass::Instruction
            } else {
                SecondaryOpcodeClass::Undefined
            },
            "secondary opcode 0f {opcode:02x}"
        );
    }
}

#[test]
fn manual_modrm_groups_reject_their_unassigned_selector_fields() {
    let primary: &[(u8, &[u8])] = &[
        (0x80, &[0, 1, 2, 3, 4, 5, 6, 7]),
        (0x81, &[0, 1, 2, 3, 4, 5, 6, 7]),
        (0x83, &[0, 1, 2, 3, 4, 5, 6, 7]),
        (0x8f, &[0]),
        (0xc0, &[0, 1, 2, 3, 4, 5, 7]),
        (0xc1, &[0, 1, 2, 3, 4, 5, 7]),
        (0xc6, &[0]),
        (0xc7, &[0]),
        (0xd0, &[0, 1, 2, 3, 4, 5, 7]),
        (0xd1, &[0, 1, 2, 3, 4, 5, 7]),
        (0xd2, &[0, 1, 2, 3, 4, 5, 7]),
        (0xd3, &[0, 1, 2, 3, 4, 5, 7]),
        (0xf6, &[0, 2, 3, 4, 5, 6, 7]),
        (0xf7, &[0, 2, 3, 4, 5, 6, 7]),
        (0xfe, &[0, 1]),
        (0xff, &[0, 1, 2, 3, 4, 5, 6]),
    ];
    for &(opcode, valid) in primary {
        for selector in 0..8 {
            assert_eq!(
                classify_primary_modrm_selector(opcode, selector),
                if valid.contains(&selector) {
                    ModRmSelectorClass::Valid
                } else {
                    ModRmSelectorClass::Reserved
                },
                "primary {opcode:#04x} /{selector}"
            );
        }
    }

    let secondary: &[(u8, &[u8])] = &[
        (0x00, &[0, 1, 2, 3, 4, 5]),
        (0x01, &[0, 1, 2, 3, 4, 6]),
        (0xba, &[4, 5, 6, 7]),
    ];
    for &(opcode, valid) in secondary {
        for selector in 0..8 {
            assert_eq!(
                classify_secondary_modrm_selector(opcode, selector),
                if valid.contains(&selector) {
                    ModRmSelectorClass::Valid
                } else {
                    ModRmSelectorClass::Reserved
                },
                "secondary 0f {opcode:02x} /{selector}"
            );
        }
    }
}
