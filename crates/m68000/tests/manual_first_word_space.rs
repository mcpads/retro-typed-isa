use m68000::{DecodedWord, classify_word};

// Normative source: Motorola, M68000 8-/16-/32-Bit Microprocessors User's
// Manual, Ninth Edition, sections 2.2 and 8. The functions below transcribe
// the instruction-format diagrams and their permitted effective-address
// categories. No production instruction, form, decoder, or encoder catalog is
// used to decide whether a first word is defined.
//
// https://www.nxp.com/docs/en/reference-manual/MC68000UM.pdf

fn ea_mode(field: u16) -> u16 {
    (field >> 3) & 7
}

fn ea_register(field: u16) -> u16 {
    field & 7
}

fn ea_is_valid(field: u16) -> bool {
    ea_mode(field) != 7 || ea_register(field) <= 4
}

fn ea_is_immediate(field: u16) -> bool {
    ea_mode(field) == 7 && ea_register(field) == 4
}

fn ea_is_data(field: u16) -> bool {
    ea_is_valid(field) && ea_mode(field) != 1
}

fn ea_is_memory_alterable(field: u16) -> bool {
    matches!(ea_mode(field), 2..=6) || (ea_mode(field) == 7 && matches!(ea_register(field), 0 | 1))
}

fn ea_is_data_alterable(field: u16) -> bool {
    ea_mode(field) == 0 || ea_is_memory_alterable(field)
}

fn ea_is_alterable(field: u16) -> bool {
    ea_mode(field) == 1 || ea_is_data_alterable(field)
}

fn ea_is_control(field: u16) -> bool {
    matches!(ea_mode(field), 2 | 5 | 6) || (ea_mode(field) == 7 && ea_register(field) <= 3)
}

fn source_ea(word: u16) -> u16 {
    word & 0x3F
}

fn move_destination_ea(word: u16) -> u16 {
    ((word >> 3) & 0x38) | ((word >> 9) & 7)
}

fn standard_size_is_valid(word: u16) -> bool {
    (word >> 6) & 3 != 3
}

fn group_0_is_documented(word: u16) -> bool {
    if matches!(word, 0x003C | 0x007C | 0x023C | 0x027C | 0x0A3C | 0x0A7C) {
        return true;
    }
    if word & 0xF138 == 0x0108 {
        return true;
    }

    let ea = source_ea(word);
    if word & 0xF100 == 0x0100 {
        return if word & 0x00C0 == 0 {
            ea_is_data(ea)
        } else {
            ea_is_data_alterable(ea)
        };
    }
    if word & 0xFF00 == 0x0800 {
        return if word & 0x00C0 == 0 {
            ea_is_data(ea) && !ea_is_immediate(ea)
        } else {
            ea_is_data_alterable(ea)
        };
    }

    standard_size_is_valid(word)
        && matches!(
            word & 0xFF00,
            0x0000 | 0x0200 | 0x0400 | 0x0600 | 0x0A00 | 0x0C00
        )
        && ea_is_data_alterable(ea)
}

fn move_group_is_documented(word: u16, byte: bool) -> bool {
    let source = source_ea(word);
    let destination = move_destination_ea(word);
    if !ea_is_valid(source) || !ea_is_valid(destination) {
        return false;
    }
    if byte && ea_mode(source) == 1 {
        return false;
    }
    if ea_mode(destination) == 1 {
        !byte
    } else {
        ea_is_data_alterable(destination)
    }
}

fn movem_ea_is_documented(word: u16) -> bool {
    let ea = source_ea(word);
    if word & 0x0400 == 0 {
        matches!(ea_mode(ea), 2 | 4 | 5 | 6)
            || (ea_mode(ea) == 7 && matches!(ea_register(ea), 0 | 1))
    } else {
        matches!(ea_mode(ea), 2 | 3 | 5 | 6) || (ea_mode(ea) == 7 && ea_register(ea) <= 3)
    }
}

fn group_4_is_documented(word: u16) -> bool {
    if matches!(
        word,
        0x4AFC | 0x4E70 | 0x4E71 | 0x4E72 | 0x4E73 | 0x4E75 | 0x4E76 | 0x4E77
    ) || word & 0xFFF0 == 0x4E40
        || word & 0xFFF8 == 0x4E50
        || word & 0xFFF8 == 0x4E58
        || word & 0xFFF0 == 0x4E60
    {
        return true;
    }

    let ea = source_ea(word);
    if word & 0xFFC0 == 0x4E80 || word & 0xFFC0 == 0x4EC0 {
        return ea_is_control(ea);
    }
    if word & 0xFFC0 == 0x40C0 {
        return ea_is_data_alterable(ea);
    }
    if word & 0xFFC0 == 0x44C0 || word & 0xFFC0 == 0x46C0 {
        return ea_is_data(ea);
    }
    if word & 0xF1C0 == 0x4180 {
        return ea_is_data(ea);
    }
    if word & 0xF1C0 == 0x41C0 {
        return ea_is_control(ea);
    }
    if word & 0xFFC0 == 0x4800 {
        return ea_is_data_alterable(ea);
    }
    if word & 0xFFF8 == 0x4840 || word & 0xFFF8 == 0x4880 || word & 0xFFF8 == 0x48C0 {
        return true;
    }
    if word & 0xFFC0 == 0x4840 {
        return ea_is_control(ea);
    }
    if word & 0xFB80 == 0x4880 {
        return movem_ea_is_documented(word);
    }
    if word & 0xFFC0 == 0x4AC0 {
        return ea_is_data_alterable(ea);
    }

    standard_size_is_valid(word)
        && matches!(word & 0xFF00, 0x4000 | 0x4200 | 0x4400 | 0x4600 | 0x4A00)
        && ea_is_data_alterable(ea)
}

fn group_5_is_documented(word: u16) -> bool {
    let ea = source_ea(word);
    if word & 0x00C0 == 0x00C0 {
        return word & 0x0038 == 0x0008 || ea_is_data_alterable(ea);
    }
    standard_size_is_valid(word)
        && ea_is_alterable(ea)
        && !((word & 0x00C0 == 0) && ea_mode(ea) == 1)
}

fn binary_data_is_documented(word: u16, add_or_sub: bool) -> bool {
    let opmode = (word >> 6) & 7;
    if matches!(opmode, 3 | 7) {
        return false;
    }
    let ea = source_ea(word);
    if opmode & 4 == 0 {
        if add_or_sub {
            ea_is_valid(ea) && !(opmode == 0 && ea_mode(ea) == 1)
        } else {
            ea_is_data(ea)
        }
    } else {
        ea_is_memory_alterable(ea)
    }
}

fn group_8_is_documented(word: u16) -> bool {
    if word & 0xF1F0 == 0x8100 {
        return true;
    }
    let opmode = (word >> 6) & 7;
    if matches!(opmode, 3 | 7) {
        ea_is_data(source_ea(word))
    } else {
        binary_data_is_documented(word, false)
    }
}

fn add_sub_group_is_documented(word: u16) -> bool {
    if (word & 0xF130 == 0x9100 || word & 0xF130 == 0xD100) && word & 0x00C0 != 0x00C0 {
        return true;
    }
    let opmode = (word >> 6) & 7;
    if matches!(opmode, 3 | 7) {
        ea_is_valid(source_ea(word))
    } else {
        binary_data_is_documented(word, true)
    }
}

fn group_b_is_documented(word: u16) -> bool {
    if word & 0xF138 == 0xB108 && word & 0x00C0 != 0x00C0 {
        return true;
    }
    let opmode = (word >> 6) & 7;
    let ea = source_ea(word);
    if matches!(opmode, 3 | 7) {
        ea_is_valid(ea)
    } else if opmode & 4 == 0 {
        ea_is_valid(ea) && !(opmode == 0 && ea_mode(ea) == 1)
    } else {
        ea_is_data_alterable(ea)
    }
}

fn group_c_is_documented(word: u16) -> bool {
    if word & 0xF1F0 == 0xC100 || matches!(word & 0xF1F8, 0xC140 | 0xC148 | 0xC188) {
        return true;
    }
    let opmode = (word >> 6) & 7;
    if matches!(opmode, 3 | 7) {
        ea_is_data(source_ea(word))
    } else {
        binary_data_is_documented(word, false)
    }
}

fn group_e_is_documented(word: u16) -> bool {
    if word & 0x00C0 == 0x00C0 {
        word & 0x0800 == 0 && ea_is_memory_alterable(source_ea(word))
    } else {
        standard_size_is_valid(word)
    }
}

fn manual_first_word_is_documented(word: u16) -> bool {
    match word >> 12 {
        0x0 => group_0_is_documented(word),
        0x1 => move_group_is_documented(word, true),
        0x2 | 0x3 => move_group_is_documented(word, false),
        0x4 => group_4_is_documented(word),
        0x5 => group_5_is_documented(word),
        0x6 => true,
        0x7 => word & 0x0100 == 0,
        0x8 => group_8_is_documented(word),
        0x9 | 0xD => add_sub_group_is_documented(word),
        0xA | 0xF => false,
        0xB => group_b_is_documented(word),
        0xC => group_c_is_documented(word),
        0xE => group_e_is_documented(word),
        _ => unreachable!(),
    }
}

#[test]
fn every_first_word_matches_the_mc68000_manual_space() {
    let mut manual_documented = 0usize;

    for word in u16::MIN..=u16::MAX {
        let source_defined = manual_first_word_is_documented(word);
        if source_defined {
            manual_documented += 1;
        }
        assert_eq!(
            matches!(classify_word(word), DecodedWord::Instruction(_)),
            source_defined,
            "word 0x{word:04X}"
        );
    }

    assert_eq!(manual_documented, 45_816);
}
