use i386::{
    AddressSize, AnalysisPoint, CodeLocation, CodeMode, DecodeError, I386Code16, I386Code32,
    InterruptState, PrimaryOpcodeClass, decode_bytes, encode_bytes,
};
use typed_isa_core::StaticSemantics;

fn assert_semantically_reconstructible(decoded: &i386::DecodedInstruction) {
    let instruction = decoded.instruction();
    let rebuilt = i386::Instruction::new(
        instruction.code_mode(),
        instruction.prefixes().clone(),
        instruction.operation(),
        instruction.operands().to_vec(),
    )
    .unwrap_or_else(|error| {
        panic!(
            "cannot reconstruct {:?} {:?}: {error}",
            instruction.operation(),
            instruction.operands()
        )
    });
    let bytes = encode_bytes(&rebuilt).unwrap();
    let reparsed = decode_bytes(&bytes, instruction.code_mode()).unwrap();
    assert_eq!(reparsed.instruction().operation(), instruction.operation());
    assert_eq!(reparsed.instruction().operands(), instruction.operands());

    let at = AnalysisPoint::new(
        CodeLocation::new(8, 0x1000),
        InterruptState::Enabled,
        AddressSize::Bits16,
    );
    I386Code16::semantics(instruction, &at).unwrap();

    let code32_bytes = toggle_default_size_prefixes(decoded.source_bytes());
    let code32 = decode_bytes(&code32_bytes, CodeMode::Code32).unwrap_or_else(|error| {
        panic!(
            "code32 counterpart failed for {:02x?}: {error}; transformed {:02x?}",
            decoded.source_bytes(),
            code32_bytes
        )
    });
    assert_eq!(encode_bytes(code32.instruction()).unwrap(), code32_bytes);
    let at = AnalysisPoint::new(
        CodeLocation::new(8, 0x1000),
        InterruptState::Enabled,
        AddressSize::Bits32,
    );
    I386Code32::semantics(code32.instruction(), &at).unwrap();
}

fn toggle_default_size_prefixes(bytes: &[u8]) -> Vec<u8> {
    let mut prefixes = Vec::new();
    let mut opcode = 0;
    while opcode < bytes.len()
        && matches!(
            bytes[opcode],
            0x26 | 0x2e | 0x36 | 0x3e | 0x64 | 0x65 | 0x66 | 0x67 | 0xf0 | 0xf2 | 0xf3
        )
    {
        prefixes.push(bytes[opcode]);
        opcode += 1;
    }
    for prefix in [0x66, 0x67] {
        if let Some(index) = prefixes.iter().position(|candidate| *candidate == prefix) {
            prefixes.remove(index);
        } else {
            prefixes.push(prefix);
        }
    }
    prefixes.extend_from_slice(&bytes[opcode..]);
    prefixes
}

fn primary_sample(opcode: u8) -> Vec<u8> {
    let mut bytes = vec![opcode];
    match opcode {
        0x00..=0x03
        | 0x08..=0x0b
        | 0x10..=0x13
        | 0x18..=0x1b
        | 0x20..=0x23
        | 0x28..=0x2b
        | 0x30..=0x33
        | 0x38..=0x3b
        | 0x63
        | 0x84..=0x8c
        | 0x8e..=0x8f
        | 0xd0..=0xd3
        | 0xd8..=0xdf
        | 0xfe..=0xff => bytes.push(0xc0),
        0x62 | 0x8d | 0xc4 | 0xc5 => bytes.push(0x00),
        0x69 | 0x81 => bytes.extend([0xc0, 0x00, 0x00]),
        0x6b | 0x80 | 0x83 => bytes.extend([0xc0, 0x00]),
        0x68 | 0xc2 | 0xca => bytes.extend([0x00, 0x00]),
        0x04
        | 0x0c
        | 0x14
        | 0x1c
        | 0x24
        | 0x2c
        | 0x34
        | 0x3c
        | 0x6a
        | 0x70..=0x7f
        | 0xcd
        | 0xd4
        | 0xd5
        | 0xe0..=0xe7
        | 0xeb => bytes.push(0),
        0x05 | 0x0d | 0x15 | 0x1d | 0x25 | 0x2d | 0x35 | 0x3d => bytes.extend([0, 0]),
        0x9a | 0xea => bytes.extend([0, 0, 0, 0]),
        0xa0..=0xa3 => bytes.extend([0, 0]),
        0xa8 | 0xb0..=0xb7 => bytes.push(0),
        0xa9 | 0xb8..=0xbf | 0xe8 | 0xe9 => bytes.extend([0, 0]),
        0xc0 | 0xc1 | 0xc6 => bytes.extend([0xc0, 0]),
        0xc7 => bytes.extend([0xc0, 0, 0]),
        0xc8 => bytes.extend([0, 0, 0]),
        0xf6 | 0xf7 => bytes.push(0xd0),
        _ => {}
    }
    bytes
}

fn secondary_sample(opcode: u8) -> Vec<u8> {
    let mut bytes = if matches!(opcode, 0xb7 | 0xbf) {
        vec![0x66, 0x0f, opcode]
    } else {
        vec![0x0f, opcode]
    };
    match opcode {
        0x00
        | 0x02
        | 0x03
        | 0x20..=0x23
        | 0x90..=0x9f
        | 0xa3
        | 0xa5
        | 0xab
        | 0xad
        | 0xaf
        | 0xb3
        | 0xb6
        | 0xb7
        | 0xbb..=0xbf => bytes.push(0xc0),
        0x01 | 0xba => bytes.push(0xe0),
        0x24 | 0x26 => bytes.push(0xf0),
        0x80..=0x8f => bytes.extend([0, 0]),
        0xa4 | 0xac => bytes.extend([0xc0, 0]),
        0xb2 | 0xb4 | 0xb5 => bytes.push(0x00),
        _ => {}
    }
    if opcode == 0xba {
        bytes.push(0);
    }
    bytes
}

#[test]
fn every_manual_primary_instruction_cell_has_a_decodable_source_form() {
    for opcode in u8::MIN..=u8::MAX {
        if i386::classify_primary_opcode(opcode) != PrimaryOpcodeClass::Instruction {
            continue;
        }
        let bytes = primary_sample(opcode);
        let decoded = decode_bytes(&bytes, CodeMode::Code16)
            .unwrap_or_else(|error| panic!("primary {opcode:02x}: {error}; bytes {bytes:02x?}"));
        assert_eq!(encode_bytes(decoded.instruction()).unwrap(), bytes);
        assert_semantically_reconstructible(&decoded);
    }
}

#[test]
fn every_manual_secondary_instruction_cell_has_a_decodable_source_form() {
    for opcode in u8::MIN..=u8::MAX {
        if i386::classify_secondary_opcode(opcode) != i386::SecondaryOpcodeClass::Instruction {
            continue;
        }
        let bytes = secondary_sample(opcode);
        let decoded = decode_bytes(&bytes, CodeMode::Code16).unwrap_or_else(|error| {
            panic!("secondary 0f {opcode:02x}: {error}; bytes {bytes:02x?}")
        });
        assert_eq!(encode_bytes(decoded.instruction()).unwrap(), bytes);
        assert_semantically_reconstructible(&decoded);
    }
}

#[test]
fn reserved_manual_group_selectors_are_rejected_by_the_decoder() {
    let primary: &[(u8, &[u8])] = &[
        (0x8f, &[1, 2, 3, 4, 5, 6, 7]),
        (0xc0, &[6]),
        (0xc1, &[6]),
        (0xc6, &[1, 2, 3, 4, 5, 6, 7]),
        (0xc7, &[1, 2, 3, 4, 5, 6, 7]),
        (0xd0, &[6]),
        (0xd1, &[6]),
        (0xd2, &[6]),
        (0xd3, &[6]),
        (0xf6, &[1]),
        (0xf7, &[1]),
        (0xfe, &[2, 3, 4, 5, 6, 7]),
        (0xff, &[7]),
    ];
    for &(opcode, selectors) in primary {
        for &selector in selectors {
            let bytes = [opcode, 0xc0 | selector << 3, 0, 0, 0, 0];
            assert!(matches!(
                decode_bytes(&bytes, CodeMode::Code16),
                Err(DecodeError::ReservedModRmSelector { selector: actual, .. }) if actual == selector
            ));
        }
    }

    let secondary: &[(u8, &[u8])] = &[(0x00, &[6, 7]), (0x01, &[5, 7]), (0xba, &[0, 1, 2, 3])];
    for &(opcode, selectors) in secondary {
        for &selector in selectors {
            let bytes = [0x0f, opcode, 0xc0 | selector << 3, 0];
            assert!(matches!(
                decode_bytes(&bytes, CodeMode::Code16),
                Err(DecodeError::ReservedModRmSelector { selector: actual, .. }) if actual == selector
            ));
        }
    }
}

#[test]
fn every_assigned_manual_group_selector_has_a_decodable_source_form() {
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
    for &(opcode, selectors) in primary {
        for &selector in selectors {
            let memory_only = opcode == 0xff && matches!(selector, 3 | 5);
            let modrm = if memory_only {
                selector << 3
            } else {
                0xc0 | selector << 3
            };
            let mut bytes = vec![opcode, modrm];
            match opcode {
                0x80 | 0x83 | 0xc0 | 0xc1 | 0xc6 => bytes.push(0),
                0x81 | 0xc7 => bytes.extend([0, 0]),
                0xf6 if selector == 0 => bytes.push(0),
                0xf7 if selector == 0 => bytes.extend([0, 0]),
                _ => {}
            }
            let decoded = decode_bytes(&bytes, CodeMode::Code16).unwrap_or_else(|error| {
                panic!("primary {opcode:02x} /{selector}: {error}; bytes {bytes:02x?}")
            });
            assert_eq!(encode_bytes(decoded.instruction()).unwrap(), bytes);
            assert_semantically_reconstructible(&decoded);
        }
    }

    let secondary: &[(u8, &[u8])] = &[
        (0x00, &[0, 1, 2, 3, 4, 5]),
        (0x01, &[0, 1, 2, 3, 4, 6]),
        (0xba, &[4, 5, 6, 7]),
    ];
    for &(opcode, selectors) in secondary {
        for &selector in selectors {
            let memory_only = opcode == 0x01 && selector <= 3;
            let modrm = if memory_only {
                selector << 3
            } else {
                0xc0 | selector << 3
            };
            let mut bytes = vec![0x0f, opcode, modrm];
            if opcode == 0xba {
                bytes.push(0);
            }
            let decoded = decode_bytes(&bytes, CodeMode::Code16).unwrap_or_else(|error| {
                panic!("secondary 0f {opcode:02x} /{selector}: {error}; bytes {bytes:02x?}")
            });
            assert_eq!(encode_bytes(decoded.instruction()).unwrap(), bytes);
            assert_semantically_reconstructible(&decoded);
        }
    }
}
