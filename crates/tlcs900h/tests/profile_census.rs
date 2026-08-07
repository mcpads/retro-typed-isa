use tlcs900h::{GeneralRegister, OperandSize, decode_bytes, encode_bytes};

fn assert_round_trip(bytes: &[u8]) -> bool {
    let Ok(decoded) = decode_bytes(bytes) else {
        return false;
    };
    let consumed = &bytes[..decoded.byte_len()];
    assert_eq!(
        encode_bytes(decoded.instruction()).unwrap(),
        consumed,
        "decoded bytes changed: {consumed:02x?}"
    );
    true
}

fn primary_example(primary: u8) -> Vec<u8> {
    let mut bytes = vec![primary];
    match primary {
        0x80..=0x87 | 0x90..=0x97 | 0xa0..=0xa7 | 0xb0..=0xb7 => bytes.push(0x20),
        0x88..=0x8f | 0x98..=0x9f | 0xa8..=0xaf | 0xb8..=0xbf => {
            bytes.extend([0, 0x20]);
        }
        0xc0..=0xc2 | 0xd0..=0xd2 | 0xe0..=0xe2 | 0xf0..=0xf2 => {
            let address_len = usize::from((primary & 3) + 1);
            bytes.extend(vec![0; address_len]);
            bytes.push(0x20);
        }
        0xc3 | 0xd3 | 0xe3 | 0xf3 => bytes.extend([0x00, 0x20]),
        0xc4 | 0xc5 | 0xd4 | 0xd5 | 0xe4 | 0xe5 | 0xf4 | 0xf5 => {
            bytes.extend([0x00, 0x20]);
        }
        0xc7 | 0xd7 | 0xe7 => bytes.extend([0xe0, 0x03, 0, 0, 0, 0]),
        0xc8..=0xcf | 0xd8..=0xdf | 0xe8..=0xef => bytes.extend([0x03, 0, 0, 0, 0]),
        0xf7 => bytes.extend([0; 5]),
        _ => bytes.extend([0; 8]),
    }
    bytes
}

fn source_primary_is_defined(primary: u8) -> bool {
    matches!(
        primary,
        0x00
            | 0x02..=0x03
            | 0x05..=0x1e
            | 0x20..=0x4f
            | 0x58..=0xbf
            | 0xc0..=0xc5
            | 0xc7..=0xd5
            | 0xd7..=0xe5
            | 0xe7..=0xf5
            | 0xf7..=0xff
    )
}

#[test]
fn appendix_c_primary_map_is_classified_directly() {
    for primary in u8::MIN..=u8::MAX {
        assert_eq!(
            assert_round_trip(&primary_example(primary)),
            source_primary_is_defined(primary),
            "primary opcode {primary:#04x} disagrees with Appendix C"
        );
    }
}

fn register_tail(secondary: u8) -> [u8; 8] {
    let mut tail = [0; 8];
    if matches!(secondary, 0x38 | 0x3c) {
        tail[0] = 1;
    } else if matches!(secondary, 0x39 | 0x3d) {
        tail[0] = 2;
    } else if matches!(secondary, 0x3a | 0x3e) {
        tail[0] = 4;
    }
    tail
}

fn source_register_secondary_is_defined(size: OperandSize, secondary: u8) -> bool {
    use OperandSize::{Byte, Long, Word};
    match secondary {
        0x03..=0x05 => true,
        0x06..=0x0b => size != Long,
        0x0c..=0x0d => size == Long,
        0x0e..=0x0f => size == Word,
        0x10 => size == Byte,
        0x12..=0x14 => size != Byte,
        0x16 | 0x19 => size == Word,
        0x1c | 0x20..=0x24 | 0x28..=0x2c | 0x30..=0x34 => size != Long,
        0x2e..=0x2f => true,
        0x38..=0x3a | 0x3c..=0x3e => size == Word,
        0x40..=0x5f => size == Word || size == Byte && secondary & 1 == 1,
        0x60..=0x6f => true,
        0x70..=0x7f => size != Long,
        0x80..=0xb7 => true,
        0xb8..=0xbf => size != Long,
        0xc0..=0xd7 => true,
        0xd8..=0xdf => size != Long,
        0xe0..=0xff => true,
        _ => false,
    }
}

#[test]
fn appendix_c_register_maps_are_classified_directly() {
    for (size, compact, extension) in [
        (OperandSize::Byte, 0xc9, 0xc7),
        (OperandSize::Word, 0xd8, 0xd7),
        (OperandSize::Long, 0xe8, 0xe7),
    ] {
        for secondary in u8::MIN..=u8::MAX {
            let expected = source_register_secondary_is_defined(size, secondary);
            let tail = register_tail(secondary);

            let mut compact_bytes = vec![compact, secondary];
            compact_bytes.extend(tail);
            assert_eq!(
                assert_round_trip(&compact_bytes),
                expected,
                "compact {size:?} secondary {secondary:#04x}"
            );

            let mut extension_bytes = vec![extension, 0xe0, secondary];
            extension_bytes.extend(tail);
            assert_eq!(
                assert_round_trip(&extension_bytes),
                expected,
                "extended {size:?} secondary {secondary:#04x}"
            );
        }
    }
}

fn source_memory_secondary_is_defined(
    size: OperandSize,
    secondary: u8,
    current_base: Option<u8>,
) -> bool {
    use OperandSize::{Byte, Long, Word};
    match secondary {
        0x04 => size != Long,
        0x06..=0x07 => size == Byte,
        0x10..=0x13 => size != Long && matches!(current_base, Some(3 | 5)),
        0x14..=0x17 => size != Long && current_base.is_some(),
        0x19 => size != Long,
        0x20..=0x27 => true,
        0x30..=0x3f => size != Long,
        0x40..=0x5f => size == Word || size == Byte && secondary & 1 == 1,
        0x60..=0x6f | 0x78..=0x7f => size != Long,
        0x80..=0xff => true,
        _ => false,
    }
}

#[test]
fn appendix_c_source_memory_maps_are_classified_directly() {
    let prefixes: &[(OperandSize, Option<u8>, &[u8])] = &[
        (OperandSize::Byte, Some(0), &[0x80]),
        (OperandSize::Byte, Some(3), &[0x83]),
        (OperandSize::Byte, None, &[0x88, 0]),
        (OperandSize::Byte, None, &[0xc0, 0]),
        (OperandSize::Word, Some(0), &[0x90]),
        (OperandSize::Word, Some(3), &[0x93]),
        (OperandSize::Word, None, &[0x98, 0]),
        (OperandSize::Word, None, &[0xd0, 0]),
        (OperandSize::Long, Some(0), &[0xa0]),
        (OperandSize::Long, None, &[0xa8, 0]),
        (OperandSize::Long, None, &[0xe0, 0]),
    ];
    for &(size, current_base, prefix) in prefixes {
        for secondary in u8::MIN..=u8::MAX {
            let mut bytes = prefix.to_vec();
            bytes.push(secondary);
            bytes.extend([0; 8]);
            assert_eq!(
                assert_round_trip(&bytes),
                source_memory_secondary_is_defined(size, secondary, current_base),
                "source-memory {size:?} secondary {secondary:#04x}"
            );
        }
    }
}

fn source_destination_secondary_is_defined(primary: u8, secondary: u8) -> bool {
    matches!(
        secondary,
        0x00 | 0x02 | 0x04 | 0x06 | 0x14 | 0x16 | 0x20..=0x2c | 0x30..=0x37
            | 0x40..=0x47 | 0x50..=0x57 | 0x60..=0x67 | 0x80..=0xef
    ) || primary == 0xb0 && secondary >= 0xf0
}

#[test]
fn appendix_c_destination_memory_maps_are_classified_directly() {
    let prefixes: &[(u8, &[u8])] = &[(0xb0, &[0xb0]), (0xb8, &[0xb8, 0]), (0xf0, &[0xf0, 0])];
    for &(primary, prefix) in prefixes {
        for secondary in u8::MIN..=u8::MAX {
            let mut bytes = prefix.to_vec();
            bytes.push(secondary);
            bytes.extend([0; 8]);
            assert_eq!(
                assert_round_trip(&bytes),
                source_destination_secondary_is_defined(primary, secondary),
                "destination-memory primary {primary:#04x}, secondary {secondary:#04x}"
            );
        }
    }
}

fn source_address_extension_is_defined(extension: u8) -> bool {
    if matches!(extension, 0x03 | 0x07 | 0x13) {
        return true;
    }
    if extension & 3 > 1 {
        return false;
    }
    let base = extension & !3;
    base <= 0x3c || base >= 0xd0
}

#[test]
fn appendix_addressing_extension_map_is_classified_directly() {
    for extension in u8::MIN..=u8::MAX {
        let mut bytes = vec![0xc3, extension];
        match extension {
            0x03 | 0x07 => bytes.extend([0xe0, 0xe0]),
            0x13 => bytes.extend([0, 0]),
            _ if extension & 3 == 1 => bytes.extend([0, 0]),
            _ => {}
        }
        bytes.push(0x20);
        bytes.extend([0; 8]);
        assert_eq!(
            assert_round_trip(&bytes),
            source_address_extension_is_defined(extension),
            "address extension {extension:#04x}"
        );
    }
}

#[test]
fn source_register_map_controls_width_alignment() {
    for code in u8::MIN..=u8::MAX {
        let register = GeneralRegister::new(code);
        let mapped = code <= 0x3f || code >= 0xd0;
        assert_eq!(register.is_ok(), mapped, "register code {code:#04x}");
        if let Ok(register) = register {
            assert_eq!(register.supports(OperandSize::Byte), mapped);
            assert_eq!(register.supports(OperandSize::Word), code & 1 == 0);
            assert_eq!(register.supports(OperandSize::Long), code & 3 == 0);
        }
    }
}
