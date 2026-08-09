use allegrex::{Allegrex, AnalysisPoint, DecodeError, Instruction, decode_word, encode_word};
use typed_isa_core::StaticSemantics;

const ADDRESS: allegrex::CodeLocation = 0x0880_0000;

// One word for each instruction encoding listed in the CPU manual's table of
// contents. CACHE entries are counted separately because the manual assigns
// fourteen semantic function codes.
const MANUAL_CPU_FORMS: &[(&str, u32)] = &[
    ("add", 0x0000_0020),
    ("addi", 0x2000_0000),
    ("addiu", 0x2400_0000),
    ("addu", 0x0000_0021),
    ("and", 0x0000_0024),
    ("andi", 0x3000_0000),
    ("bc1f", 0x4500_0000),
    ("bc1fl", 0x4502_0000),
    ("bc1t", 0x4501_0000),
    ("bc1tl", 0x4503_0000),
    ("beq", 0x1000_0000),
    ("beql", 0x5000_0000),
    ("bgez", 0x0401_0000),
    ("bgezal", 0x0411_0000),
    ("bgezall", 0x0413_0000),
    ("bgezl", 0x0403_0000),
    ("bgtz", 0x1c00_0000),
    ("bgtzl", 0x5c00_0000),
    ("blez", 0x1800_0000),
    ("blezl", 0x5800_0000),
    ("bltz", 0x0400_0000),
    ("bltzal", 0x0410_0000),
    ("bltzall", 0x0412_0000),
    ("bltzl", 0x0402_0000),
    ("bne", 0x1400_0000),
    ("bnel", 0x5400_0000),
    ("break", 0x0000_000d),
    ("cfc1", 0x4440_0000),
    ("ctc1", 0x44c0_f800),
    ("div", 0x0000_001a),
    ("divu", 0x0000_001b),
    ("eret", 0x4200_0018),
    ("j", 0x0800_0000),
    ("jal", 0x0c00_0000),
    ("jalr", 0x0000_0009),
    ("jr", 0x0000_0008),
    ("lb", 0x8000_0000),
    ("lbu", 0x9000_0000),
    ("lh", 0x8400_0000),
    ("lhu", 0x9400_0000),
    ("lui", 0x3c00_0000),
    ("lw", 0x8c00_0000),
    ("lwc1", 0xc400_0000),
    ("lwl", 0x8800_0000),
    ("lwr", 0x9800_0000),
    ("mfc0", 0x4000_0000),
    ("mfc1", 0x4400_0000),
    ("mfhi", 0x0000_0010),
    ("mflo", 0x0000_0012),
    ("mtc0", 0x4080_0000),
    ("mtc1", 0x4480_0000),
    ("mthi", 0x0000_0011),
    ("mtlo", 0x0000_0013),
    ("mult", 0x0000_0018),
    ("multu", 0x0000_0019),
    ("nop", 0x0000_0000),
    ("nor", 0x0000_0027),
    ("or", 0x0000_0025),
    ("ori", 0x3400_0000),
    ("sb", 0xa000_0000),
    ("sh", 0xa400_0000),
    ("sll", 0x0000_0800),
    ("sllv", 0x0000_0004),
    ("slt", 0x0000_002a),
    ("slti", 0x2800_0000),
    ("sltiu", 0x2c00_0000),
    ("sltu", 0x0000_002b),
    ("sra", 0x0000_0003),
    ("srav", 0x0000_0007),
    ("srl", 0x0000_0002),
    ("srlv", 0x0000_0006),
    ("sub", 0x0000_0022),
    ("subu", 0x0000_0023),
    ("sw", 0xac00_0000),
    ("swc1", 0xe400_0000),
    ("swl", 0xa800_0000),
    ("swr", 0xb800_0000),
    ("syscall", 0x0000_000c),
    ("xor", 0x0000_0026),
    ("xori", 0x3800_0000),
    ("clo", 0x0000_0017),
    ("clz", 0x0000_0016),
    ("madd", 0x0000_001c),
    ("maddu", 0x0000_001d),
    ("msub", 0x0000_002e),
    ("msubu", 0x0000_002f),
    ("max", 0x0000_002c),
    ("min", 0x0000_002d),
    ("movn", 0x0000_000b),
    ("movz", 0x0000_000a),
    ("ext", 0x7c00_0000),
    ("ins", 0x7c00_0004),
    ("seb", 0x7c00_0420),
    ("seh", 0x7c00_0620),
    ("bitrev", 0x7c00_0520),
    ("rotr", 0x0020_0002),
    ("rotrv", 0x0000_0046),
    ("wsbh", 0x7c00_00a0),
    ("wsbw", 0x7c00_00e0),
    ("cache index invalidate I", 0xbc04_0000),
    ("cache index unlock I", 0xbc06_0000),
    ("cache hit invalidate I", 0xbc08_0000),
    ("cache fill I", 0xbc0a_0000),
    ("cache fill with lock I", 0xbc0b_0000),
    ("cache index writeback invalidate D", 0xbc14_0000),
    ("cache index unlock D", 0xbc16_0000),
    ("cache create dirty exclusive D", 0xbc18_0000),
    ("cache hit invalidate D", 0xbc19_0000),
    ("cache hit writeback D", 0xbc1a_0000),
    ("cache hit writeback invalidate D", 0xbc1b_0000),
    ("cache create dirty exclusive with lock D", 0xbc1c_0000),
    ("cache fill D", 0xbc1e_0000),
    ("cache fill with lock D", 0xbc1f_0000),
    ("sync", 0x0000_000f),
    ("ll", 0xc000_0000),
    ("sc", 0xe000_0000),
];

const MANUAL_FPU_FORMS: &[(&str, u32)] = &[
    ("abs.s", 0x4600_0005),
    ("add.s", 0x4600_0000),
    ("bc1f", 0x4500_0000),
    ("bc1fl", 0x4502_0000),
    ("bc1t", 0x4501_0000),
    ("bc1tl", 0x4503_0000),
    ("c.f.s", 0x4600_0030),
    ("c.un.s", 0x4600_0031),
    ("c.eq.s", 0x4600_0032),
    ("c.ueq.s", 0x4600_0033),
    ("c.olt.s", 0x4600_0034),
    ("c.ult.s", 0x4600_0035),
    ("c.ole.s", 0x4600_0036),
    ("c.ule.s", 0x4600_0037),
    ("c.sf.s", 0x4600_0038),
    ("c.ngle.s", 0x4600_0039),
    ("c.seq.s", 0x4600_003a),
    ("c.ngl.s", 0x4600_003b),
    ("c.lt.s", 0x4600_003c),
    ("c.nge.s", 0x4600_003d),
    ("c.le.s", 0x4600_003e),
    ("c.ngt.s", 0x4600_003f),
    ("ceil.w.s", 0x4600_000a),
    ("cfc1", 0x4440_0000),
    ("ctc1", 0x44c0_f800),
    ("cvt.s.w", 0x4680_0020),
    ("cvt.w.s", 0x4600_0024),
    ("div.s", 0x4600_0003),
    ("floor.w.s", 0x4600_000b),
    ("lwc1", 0xc400_0000),
    ("mfc1", 0x4400_0000),
    ("mov.s", 0x4600_0006),
    ("mtc1", 0x4480_0000),
    ("mul.s", 0x4600_0002),
    ("neg.s", 0x4600_0007),
    ("round.w.s", 0x4600_0008),
    ("sqrt.s", 0x4600_0004),
    ("sub.s", 0x4600_0001),
    ("swc1", 0xe400_0000),
    ("trunc.w.s", 0x4600_0009),
];

#[test]
fn every_cpu_manual_form_decodes_and_reencodes_exactly() {
    assert_eq!(MANUAL_CPU_FORMS.len(), 116);
    assert_manual_words(MANUAL_CPU_FORMS);
}

#[test]
fn every_fpu_manual_form_decodes_and_reencodes_exactly() {
    assert_eq!(MANUAL_FPU_FORMS.len(), 40);
    assert_manual_words(MANUAL_FPU_FORMS);
}

#[test]
fn reserved_selector_classes_fail_closed() {
    for function in [
        0x01_u8, 0x05, 0x0e, 0x14, 0x15, 0x1e, 0x1f, 0x28, 0x29, 0x30, 0x3f,
    ] {
        assert_reserved(u32::from(function));
    }
    for selector in [0x04_u8, 0x05, 0x06, 0x07, 0x08, 0x0f, 0x14, 0x1f] {
        assert_reserved(0x0400_0000 | u32::from(selector) << 16);
    }
    for cache_code in [
        0_u8, 1, 2, 3, 5, 7, 9, 12, 13, 14, 15, 16, 17, 18, 19, 21, 23, 29,
    ] {
        assert_reserved(0xbc00_0000 | u32::from(cache_code) << 16);
    }
    assert_reserved(0x4620_0000); // No double-precision format.
    assert_reserved(0x4440_0800); // CFC1 only accepts FCR0 and FCR31.
    assert_reserved(0x44c0_0000); // CTC1 only accepts FCR31.
    assert_reserved(0xd060_0000); // VCST constant zero is undefined.
    assert_reserved(0x4860_0085); // Control registers 133 and 134 are reserved.
    assert_reserved(0x48e0_0087); // Control register 135 is read-only.
    assert!(decode_word(0x4860_0087, ADDRESS).is_ok());
}

#[test]
fn decoded_instruction_never_uses_an_opaque_word_fallback() {
    for &(_, word) in MANUAL_CPU_FORMS.iter().chain(MANUAL_FPU_FORMS) {
        let instruction = decode_word(word, ADDRESS).unwrap();
        assert!(!matches!(instruction, Instruction::Vfpu(_)) || word >= 0x4800_0000);
        assert_eq!(encode_word(&instruction, ADDRESS).unwrap(), word);
    }
}

fn assert_manual_words(words: &[(&str, u32)]) {
    for &(mnemonic, word) in words {
        let instruction = decode_word(word, ADDRESS)
            .unwrap_or_else(|error| panic!("{mnemonic} {word:#010x}: {error}"));
        assert_eq!(
            encode_word(&instruction, ADDRESS).unwrap(),
            word,
            "{mnemonic}"
        );
        Allegrex::semantics(&instruction, &AnalysisPoint::new(ADDRESS))
            .unwrap_or_else(|error| panic!("{mnemonic} semantic failure: {error}"));
    }
}

fn assert_reserved(word: u32) {
    assert!(
        matches!(
            decode_word(word, ADDRESS),
            Err(DecodeError::ReservedWord { .. })
        ),
        "word {word:#010x} did not fail closed"
    );
}
