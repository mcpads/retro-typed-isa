use r5900::{DecodeError, decode_word, encode_word};

const ADDRESS: u64 = 0x0010_0000;

// One word for every chapter-2 instruction section in Sony's EE Core
// Instruction Set Manual 6.0. These values come from the manual's encoding
// diagrams and appendix opcode maps, not from the production descriptor table.
const MANUAL_CPU_FORMS: &[(&str, u32)] = &[
    ("add", 0x0000_0020),
    ("addi", 0x2000_0000),
    ("addiu", 0x2400_0000),
    ("addu", 0x0000_0021),
    ("and", 0x0000_0024),
    ("andi", 0x3000_0000),
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
    ("dadd", 0x0000_002c),
    ("daddi", 0x6000_0000),
    ("daddiu", 0x6400_0000),
    ("daddu", 0x0000_002d),
    ("div", 0x0000_001a),
    ("divu", 0x0000_001b),
    ("dsll", 0x0000_0038),
    ("dsll32", 0x0000_003c),
    ("dsllv", 0x0000_0014),
    ("dsra", 0x0000_003b),
    ("dsra32", 0x0000_003f),
    ("dsrav", 0x0000_0017),
    ("dsrl", 0x0000_003a),
    ("dsrl32", 0x0000_003e),
    ("dsrlv", 0x0000_0016),
    ("dsub", 0x0000_002e),
    ("dsubu", 0x0000_002f),
    ("j", 0x0800_0000),
    ("jal", 0x0c00_0000),
    ("jalr", 0x0000_0009),
    ("jr", 0x0000_0008),
    ("lb", 0x8000_0000),
    ("lbu", 0x9000_0000),
    ("ld", 0xdc00_0000),
    ("ldl", 0x6800_0000),
    ("ldr", 0x6c00_0000),
    ("lh", 0x8400_0000),
    ("lhu", 0x9400_0000),
    ("lui", 0x3c00_0000),
    ("lw", 0x8c00_0000),
    ("lwl", 0x8800_0000),
    ("lwr", 0x9800_0000),
    ("lwu", 0x9c00_0000),
    ("mfhi", 0x0000_0010),
    ("mflo", 0x0000_0012),
    ("movn", 0x0000_000b),
    ("movz", 0x0000_000a),
    ("mthi", 0x0000_0011),
    ("mtlo", 0x0000_0013),
    ("mult", 0x0000_0018),
    ("multu", 0x0000_0019),
    ("nor", 0x0000_0027),
    ("or", 0x0000_0025),
    ("ori", 0x3400_0000),
    ("pref", 0xcc00_0000),
    ("sb", 0xa000_0000),
    ("sd", 0xfc00_0000),
    ("sdl", 0xb000_0000),
    ("sdr", 0xb400_0000),
    ("sh", 0xa400_0000),
    ("sll", 0x0000_0000),
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
    ("swl", 0xa800_0000),
    ("swr", 0xb800_0000),
    ("sync", 0x0000_000f),
    ("syscall", 0x0000_000c),
    ("teq", 0x0000_0034),
    ("teqi", 0x040c_0000),
    ("tge", 0x0000_0030),
    ("tgei", 0x0408_0000),
    ("tgeiu", 0x0409_0000),
    ("tgeu", 0x0000_0031),
    ("tlt", 0x0000_0032),
    ("tlti", 0x040a_0000),
    ("tltiu", 0x040b_0000),
    ("tltu", 0x0000_0033),
    ("tne", 0x0000_0036),
    ("tnei", 0x040e_0000),
    ("xor", 0x0000_0026),
    ("xori", 0x3800_0000),
];

// Chapter 3 has separate MULT and MULTU sections for their destination-register
// extensions. Nonzero rd samples distinguish those sections from chapter 2.
const MANUAL_EE_FORMS: &[(&str, u32)] = &[
    ("div1", 0x7000_001a),
    ("divu1", 0x7000_001b),
    ("lq", 0x7800_0000),
    ("madd", 0x7000_0000),
    ("madd1", 0x7000_0020),
    ("maddu", 0x7000_0001),
    ("maddu1", 0x7000_0021),
    ("mfhi1", 0x7000_0010),
    ("mflo1", 0x7000_0012),
    ("mfsa", 0x0000_0028),
    ("mthi1", 0x7000_0011),
    ("mtlo1", 0x7000_0013),
    ("mtsa", 0x0000_0029),
    ("mtsab", 0x0418_0000),
    ("mtsah", 0x0419_0000),
    ("mult", 0x0000_0818),
    ("mult1", 0x7000_0018),
    ("multu", 0x0000_0819),
    ("multu1", 0x7000_0019),
    ("pabsh", 0x7000_0168),
    ("pabsw", 0x7000_0068),
    ("paddb", 0x7000_0208),
    ("paddh", 0x7000_0108),
    ("paddsb", 0x7000_0608),
    ("paddsh", 0x7000_0508),
    ("paddsw", 0x7000_0408),
    ("paddub", 0x7000_0628),
    ("padduh", 0x7000_0528),
    ("padduw", 0x7000_0428),
    ("paddw", 0x7000_0008),
    ("padsbh", 0x7000_0128),
    ("pand", 0x7000_0489),
    ("pceqb", 0x7000_02a8),
    ("pceqh", 0x7000_01a8),
    ("pceqw", 0x7000_00a8),
    ("pcgtb", 0x7000_0288),
    ("pcgth", 0x7000_0188),
    ("pcgtw", 0x7000_0088),
    ("pcpyh", 0x7000_06e9),
    ("pcpyld", 0x7000_0389),
    ("pcpyud", 0x7000_03a9),
    ("pdivbw", 0x7000_0749),
    ("pdivuw", 0x7000_0369),
    ("pdivw", 0x7000_0349),
    ("pexch", 0x7000_06a9),
    ("pexcw", 0x7000_07a9),
    ("pexeh", 0x7000_0689),
    ("pexew", 0x7000_0789),
    ("pext5", 0x7000_0788),
    ("pextlb", 0x7000_0688),
    ("pextlh", 0x7000_0588),
    ("pextlw", 0x7000_0488),
    ("pextub", 0x7000_06a8),
    ("pextuh", 0x7000_05a8),
    ("pextuw", 0x7000_04a8),
    ("phmadh", 0x7000_0449),
    ("phmsbh", 0x7000_0549),
    ("pinteh", 0x7000_02a9),
    ("pinth", 0x7000_0289),
    ("plzcw", 0x7000_0004),
    ("pmaddh", 0x7000_0409),
    ("pmadduw", 0x7000_0029),
    ("pmaddw", 0x7000_0009),
    ("pmaxh", 0x7000_01c8),
    ("pmaxw", 0x7000_00c8),
    ("pmfhi", 0x7000_0209),
    ("pmfhl.lh", 0x7000_00f0),
    ("pmfhl.lw", 0x7000_0030),
    ("pmfhl.sh", 0x7000_0130),
    ("pmfhl.slw", 0x7000_00b0),
    ("pmfhl.uw", 0x7000_0070),
    ("pmflo", 0x7000_0249),
    ("pminh", 0x7000_01e8),
    ("pminw", 0x7000_00e8),
    ("pmsubh", 0x7000_0509),
    ("pmsubw", 0x7000_0109),
    ("pmthi", 0x7000_0229),
    ("pmthl.lw", 0x7000_0031),
    ("pmtlo", 0x7000_0269),
    ("pmulth", 0x7000_0709),
    ("pmultuw", 0x7000_0329),
    ("pmultw", 0x7000_0309),
    ("pnor", 0x7000_04e9),
    ("por", 0x7000_04a9),
    ("ppac5", 0x7000_07c8),
    ("ppacb", 0x7000_06c8),
    ("ppach", 0x7000_05c8),
    ("ppacw", 0x7000_04c8),
    ("prevh", 0x7000_06c9),
    ("prot3w", 0x7000_07c9),
    ("psllh", 0x7000_0034),
    ("psllvw", 0x7000_0089),
    ("psllw", 0x7000_003c),
    ("psrah", 0x7000_0037),
    ("psravw", 0x7000_00e9),
    ("psraw", 0x7000_003f),
    ("psrlh", 0x7000_0036),
    ("psrlvw", 0x7000_00c9),
    ("psrlw", 0x7000_003e),
    ("psubb", 0x7000_0248),
    ("psubh", 0x7000_0148),
    ("psubsb", 0x7000_0648),
    ("psubsh", 0x7000_0548),
    ("psubsw", 0x7000_0448),
    ("psubub", 0x7000_0668),
    ("psubuh", 0x7000_0568),
    ("psubuw", 0x7000_0468),
    ("psubw", 0x7000_0048),
    ("pxor", 0x7000_04c9),
    ("qfsrv", 0x7000_06e8),
    ("sq", 0x7c00_0000),
];

const MANUAL_COP0_FORMS: &[(&str, u32)] = &[
    ("bc0f", 0x4100_0000),
    ("bc0fl", 0x4102_0000),
    ("bc0t", 0x4101_0000),
    ("bc0tl", 0x4103_0000),
    ("cache.bfh", 0xbc0c_0000),
    ("cache.bhinbt", 0xbc0a_0000),
    ("cache.bxlbt", 0xbc02_0000),
    ("cache.bxsbt", 0xbc06_0000),
    ("cache.dhin", 0xbc1a_0000),
    ("cache.dhwbin", 0xbc18_0000),
    ("cache.dhwoin", 0xbc1c_0000),
    ("cache.dxin", 0xbc16_0000),
    ("cache.dxldt", 0xbc11_0000),
    ("cache.dxltg", 0xbc10_0000),
    ("cache.dxsdt", 0xbc13_0000),
    ("cache.dxstg", 0xbc12_0000),
    ("cache.dxwbin", 0xbc14_0000),
    ("cache.ifl", 0xbc0e_0000),
    ("cache.ihin", 0xbc0b_0000),
    ("cache.ixin", 0xbc07_0000),
    ("cache.ixldt", 0xbc01_0000),
    ("cache.ixltg", 0xbc00_0000),
    ("cache.ixsdt", 0xbc05_0000),
    ("cache.ixstg", 0xbc04_0000),
    ("di", 0x4200_0039),
    ("ei", 0x4200_0038),
    ("eret", 0x4200_0018),
    ("mfbpc", 0x4000_c000),
    ("mfc0", 0x4000_0000),
    ("mfdab", 0x4000_c004),
    ("mfdabm", 0x4000_c005),
    ("mfdvb", 0x4000_c006),
    ("mfdvbm", 0x4000_c007),
    ("mfiab", 0x4000_c002),
    ("mfiabm", 0x4000_c003),
    ("mfpc", 0x4000_c801),
    ("mfps", 0x4000_c800),
    ("mtbpc", 0x4080_c000),
    ("mtc0", 0x4080_0000),
    ("mtdab", 0x4080_c004),
    ("mtdabm", 0x4080_c005),
    ("mtdvb", 0x4080_c006),
    ("mtdvbm", 0x4080_c007),
    ("mtiab", 0x4080_c002),
    ("mtiabm", 0x4080_c003),
    ("mtpc", 0x4080_c801),
    ("mtps", 0x4080_c800),
    ("tlbp", 0x4200_0008),
    ("tlbr", 0x4200_0001),
    ("tlbwi", 0x4200_0002),
    ("tlbwr", 0x4200_0006),
];

const MANUAL_FPU_FORMS: &[(&str, u32)] = &[
    ("abs.s", 0x4600_0005),
    ("add.s", 0x4600_0000),
    ("adda.s", 0x4600_0018),
    ("bc1f", 0x4500_0000),
    ("bc1fl", 0x4502_0000),
    ("bc1t", 0x4501_0000),
    ("bc1tl", 0x4503_0000),
    ("c.eq.s", 0x4600_0032),
    ("c.f.s", 0x4600_0030),
    ("c.le.s", 0x4600_0036),
    ("c.lt.s", 0x4600_0034),
    ("cfc1", 0x4440_0000),
    ("ctc1", 0x44c0_f800),
    ("cvt.s.w", 0x4680_0020),
    ("cvt.w.s", 0x4600_0024),
    ("div.s", 0x4600_0003),
    ("lwc1", 0xc400_0000),
    ("madd.s", 0x4600_001c),
    ("madda.s", 0x4600_001e),
    ("max.s", 0x4600_0028),
    ("mfc1", 0x4400_0000),
    ("min.s", 0x4600_0029),
    ("mov.s", 0x4600_0006),
    ("msub.s", 0x4600_001d),
    ("msuba.s", 0x4600_001f),
    ("mtc1", 0x4480_0000),
    ("mul.s", 0x4600_0002),
    ("mula.s", 0x4600_001a),
    ("neg.s", 0x4600_0007),
    ("rsqrt.s", 0x4600_0016),
    ("sqrt.s", 0x4600_0004),
    ("sub.s", 0x4600_0001),
    ("suba.s", 0x4600_0019),
    ("swc1", 0xe400_0000),
];

// One word for every chapter-6 macro-mode instruction section in Sony's VU
// User's Manual 6.0. The section list and words are independent of the
// production descriptor catalog above.
const MANUAL_VU_MACRO_FORMS: &[(&str, u32)] = &[
    ("bc2f", 0x4900_0000),
    ("bc2fl", 0x4902_0000),
    ("bc2t", 0x4901_0000),
    ("bc2tl", 0x4903_0000),
    ("cfc2", 0x4840_0000),
    ("ctc2", 0x48c0_0000),
    ("lqc2", 0xd800_0000),
    ("qmfc2", 0x4820_0000),
    ("qmtc2", 0x48a0_0000),
    ("sqc2", 0xf800_0000),
    ("vabs", 0x4a00_01fd),
    ("vadd", 0x4a00_0028),
    ("vaddi", 0x4a00_0022),
    ("vaddq", 0x4a00_0020),
    ("vadd.bc", 0x4a00_0000),
    ("vadda", 0x4a00_02bc),
    ("vaddai", 0x4a00_023e),
    ("vaddaq", 0x4a00_023c),
    ("vadda.bc", 0x4a00_003c),
    ("vcallms", 0x4a00_0038),
    ("vcallmsr", 0x4a00_d839),
    ("vclip", 0x4bc0_01ff),
    ("vdiv", 0x4a00_03bc),
    ("vftoi0", 0x4a00_017c),
    ("vftoi4", 0x4a00_017d),
    ("vftoi12", 0x4a00_017e),
    ("vftoi15", 0x4a00_017f),
    ("viadd", 0x4a00_0030),
    ("viaddi", 0x4a00_0032),
    ("viand", 0x4a00_0034),
    ("vilwr", 0x4b00_03fe),
    ("vior", 0x4a00_0035),
    ("visub", 0x4a00_0031),
    ("viswr", 0x4a00_03ff),
    ("vitof0", 0x4a00_013c),
    ("vitof4", 0x4a00_013d),
    ("vitof12", 0x4a00_013e),
    ("vitof15", 0x4a00_013f),
    ("vlqd", 0x4a00_037e),
    ("vlqi", 0x4a00_037c),
    ("vmadd", 0x4a00_0029),
    ("vmaddi", 0x4a00_0023),
    ("vmaddq", 0x4a00_0021),
    ("vmadd.bc", 0x4a00_0008),
    ("vmadda", 0x4a00_02bd),
    ("vmaddai", 0x4a00_023f),
    ("vmaddaq", 0x4a00_023d),
    ("vmadda.bc", 0x4a00_00bc),
    ("vmax", 0x4a00_002b),
    ("vmaxi", 0x4a00_001d),
    ("vmax.bc", 0x4a00_0010),
    ("vmfir", 0x4a00_03fd),
    ("vmini", 0x4a00_002f),
    ("vminii", 0x4a00_001f),
    ("vmini.bc", 0x4a00_0014),
    ("vmove", 0x4a00_033c),
    ("vmr32", 0x4a00_033d),
    ("vmsub", 0x4a00_002d),
    ("vmsubi", 0x4a00_0027),
    ("vmsubq", 0x4a00_0025),
    ("vmsub.bc", 0x4a00_000c),
    ("vmsuba", 0x4a00_02fd),
    ("vmsubai", 0x4a00_027f),
    ("vmsubaq", 0x4a00_027d),
    ("vmsuba.bc", 0x4a00_00fc),
    ("vmtir", 0x4a00_03fc),
    ("vmul", 0x4a00_002a),
    ("vmuli", 0x4a00_001e),
    ("vmulq", 0x4a00_001c),
    ("vmul.bc", 0x4a00_0018),
    ("vmula", 0x4a00_02be),
    ("vmulai", 0x4a00_01fe),
    ("vmulaq", 0x4a00_01fc),
    ("vmula.bc", 0x4a00_01bc),
    ("vnop", 0x4a00_02ff),
    ("vopmula", 0x4bc0_02fe),
    ("vopmsub", 0x4bc0_002e),
    ("vrget", 0x4a00_043d),
    ("vrinit", 0x4a00_043e),
    ("vrnext", 0x4a00_043c),
    ("vrsqrt", 0x4a00_03be),
    ("vrxor", 0x4a00_043f),
    ("vsqd", 0x4a00_037f),
    ("vsqi", 0x4a00_037d),
    ("vsqrt", 0x4a00_03bd),
    ("vsub", 0x4a00_002c),
    ("vsubi", 0x4a00_0026),
    ("vsubq", 0x4a00_0024),
    ("vsub.bc", 0x4a00_0004),
    ("vsuba", 0x4a00_02fc),
    ("vsubai", 0x4a00_027e),
    ("vsubaq", 0x4a00_027c),
    ("vsuba.bc", 0x4a00_007c),
    ("vwaitq", 0x4a00_03bf),
];

#[test]
fn every_source_listed_form_decodes_and_reencodes_exactly() {
    assert_eq!(MANUAL_CPU_FORMS.len(), 104);
    assert_eq!(MANUAL_EE_FORMS.len(), 111);
    assert_eq!(MANUAL_COP0_FORMS.len(), 51);
    assert_eq!(MANUAL_FPU_FORMS.len(), 34);
    assert_eq!(MANUAL_VU_MACRO_FORMS.len(), 94);
    assert_eq!(
        MANUAL_CPU_FORMS.len()
            + MANUAL_EE_FORMS.len()
            + MANUAL_COP0_FORMS.len()
            + MANUAL_FPU_FORMS.len()
            + MANUAL_VU_MACRO_FORMS.len(),
        394
    );

    for &(mnemonic, word) in MANUAL_CPU_FORMS
        .iter()
        .chain(MANUAL_EE_FORMS)
        .chain(MANUAL_COP0_FORMS)
        .chain(MANUAL_FPU_FORMS)
        .chain(MANUAL_VU_MACRO_FORMS)
    {
        let instruction = decode_word(word, ADDRESS)
            .unwrap_or_else(|error| panic!("{mnemonic} {word:#010x}: {error}"));
        assert_eq!(instruction.form().mnemonic(), mnemonic);
        assert_eq!(
            encode_word(&instruction, ADDRESS).unwrap(),
            word,
            "{mnemonic}"
        );
    }
}

#[test]
fn reserved_top_level_and_register_selectors_fail_closed() {
    for word in [
        0x4c00_0000, // Reserved top-level opcode.
        0xc000_0000, // Unsupported LL.
        0xe000_0000, // Unsupported SC.
        0x4000_3800, // Reserved COP0 register 7.
        0x4000_8800, // Reserved COP0 register 17.
        0x4000_f800, // Reserved COP0 register 31.
        0x4440_0800, // CFC1 only accepts FCR0 and FCR31.
        0x44c0_0000, // CTC1 only accepts FCR31.
        0x4000_c802, // MFPS only accepts performance control register zero.
    ] {
        assert!(matches!(
            decode_word(word, ADDRESS),
            Err(DecodeError::ReservedWord { .. })
        ));
    }
}

#[test]
fn appendix_selector_maps_reject_unassigned_entries() {
    let valid_special = [
        0x00_u8, 0x02, 0x03, 0x04, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0f, 0x10,
        0x11, 0x12, 0x13, 0x14, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x20, 0x21, 0x22, 0x23, 0x24,
        0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f, 0x30, 0x31, 0x32, 0x33,
        0x34, 0x36, 0x38, 0x3a, 0x3b, 0x3c, 0x3e, 0x3f,
    ];
    for function in 0_u8..64 {
        let word = u32::from(function);
        if valid_special.contains(&function) {
            assert!(decode_word(word, ADDRESS).is_ok(), "SPECIAL {function:#x}");
        } else {
            assert!(matches!(
                decode_word(word, ADDRESS),
                Err(DecodeError::ReservedWord { .. })
            ));
        }
    }

    let valid_regimm = [
        0x00_u8, 0x01, 0x02, 0x03, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0e, 0x10, 0x11, 0x12, 0x13,
        0x18, 0x19,
    ];
    for selector in 0_u8..32 {
        let word = 0x0400_0000 | u32::from(selector) << 16;
        if valid_regimm.contains(&selector) {
            assert!(decode_word(word, ADDRESS).is_ok(), "REGIMM {selector:#x}");
        } else {
            assert!(matches!(
                decode_word(word, ADDRESS),
                Err(DecodeError::ReservedWord { .. })
            ));
        }
    }
}
