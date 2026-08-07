use std::collections::BTreeSet;

use sh2::{DecodedWord, InstructionForm, classify_word, encode_word};

// Normative source: Hitachi, SuperH RISC Engine SH-1/SH-2 Programming
// Manual, 3 September 1996, Appendix A "Instruction List". Each row below
// transcribes one SH-2 instruction-code pattern from that table. A mask bit is
// one where the manual prints a literal zero or one; operand fields are zero.
//
// https://antime.kapsi.fi/sega/files/h12p0.pdf

#[derive(Clone, Copy)]
struct ManualPattern {
    mask: u16,
    value: u16,
    form: InstructionForm,
}

macro_rules! pattern {
    ($mask:expr, $value:expr, $form:ident) => {
        ManualPattern {
            mask: $mask,
            value: $value,
            form: InstructionForm::$form,
        }
    };
}

const MANUAL_PATTERNS: [ManualPattern; 142] = [
    pattern!(0xF000, 0xE000, MovImm),
    pattern!(0xF000, 0x9000, MovWPcRel),
    pattern!(0xF000, 0xD000, MovLPcRel),
    pattern!(0xF00F, 0x6003, MovReg),
    pattern!(0xF00F, 0x2000, MovBStore),
    pattern!(0xF00F, 0x2001, MovWStore),
    pattern!(0xF00F, 0x2002, MovLStore),
    pattern!(0xF00F, 0x6000, MovBLoad),
    pattern!(0xF00F, 0x6001, MovWLoad),
    pattern!(0xF00F, 0x6002, MovLLoad),
    pattern!(0xF00F, 0x2004, MovBStorePreDec),
    pattern!(0xF00F, 0x2005, MovWStorePreDec),
    pattern!(0xF00F, 0x2006, MovLStorePreDec),
    pattern!(0xF00F, 0x6004, MovBLoadPostInc),
    pattern!(0xF00F, 0x6005, MovWLoadPostInc),
    pattern!(0xF00F, 0x6006, MovLLoadPostInc),
    pattern!(0xF00F, 0x000C, MovBR0LoadIndexed),
    pattern!(0xF00F, 0x000D, MovWR0LoadIndexed),
    pattern!(0xF00F, 0x000E, MovLR0LoadIndexed),
    pattern!(0xF00F, 0x0004, MovBR0StoreIndexed),
    pattern!(0xF00F, 0x0005, MovWR0StoreIndexed),
    pattern!(0xF00F, 0x0006, MovLR0StoreIndexed),
    pattern!(0xFF00, 0x8000, MovBDispStore),
    pattern!(0xFF00, 0x8100, MovWDispStore),
    pattern!(0xF000, 0x1000, MovLDispStore),
    pattern!(0xFF00, 0x8400, MovBDispLoad),
    pattern!(0xFF00, 0x8500, MovWDispLoad),
    pattern!(0xF000, 0x5000, MovLDispLoad),
    pattern!(0xFF00, 0xC000, MovBGbrStore),
    pattern!(0xFF00, 0xC400, MovBGbrLoad),
    pattern!(0xFF00, 0xC100, MovWGbrStore),
    pattern!(0xFF00, 0xC500, MovWGbrLoad),
    pattern!(0xFF00, 0xC200, MovLGbrStore),
    pattern!(0xFF00, 0xC600, MovLGbrLoad),
    pattern!(0xFF00, 0xC700, Mova),
    pattern!(0xF0FF, 0x0029, MovT),
    pattern!(0xF00F, 0x300C, Add),
    pattern!(0xF000, 0x7000, AddImm),
    pattern!(0xF00F, 0x300E, Addc),
    pattern!(0xF00F, 0x300F, Addv),
    pattern!(0xF00F, 0x3008, Sub),
    pattern!(0xF00F, 0x300A, Subc),
    pattern!(0xF00F, 0x300B, Subv),
    pattern!(0xF00F, 0x600B, Neg),
    pattern!(0xF00F, 0x600A, Negc),
    pattern!(0xF0FF, 0x4010, Dt),
    pattern!(0xF00F, 0x0007, MulL),
    pattern!(0xF00F, 0x200F, MulS),
    pattern!(0xF00F, 0x200E, MulU),
    pattern!(0xF00F, 0x300D, DmulS),
    pattern!(0xF00F, 0x3005, DmulU),
    pattern!(0xF00F, 0x2007, Div0S),
    pattern!(0xFFFF, 0x0019, Div0U),
    pattern!(0xF00F, 0x3004, Div1),
    pattern!(0xF00F, 0x000F, MacL),
    pattern!(0xF00F, 0x400F, MacW),
    pattern!(0xF00F, 0x2009, And),
    pattern!(0xFF00, 0xC900, AndImm),
    pattern!(0xFF00, 0xCD00, AndB),
    pattern!(0xF00F, 0x200B, Or),
    pattern!(0xFF00, 0xCB00, OrImm),
    pattern!(0xFF00, 0xCF00, OrB),
    pattern!(0xF00F, 0x200A, Xor),
    pattern!(0xFF00, 0xCA00, XorImm),
    pattern!(0xFF00, 0xCE00, XorB),
    pattern!(0xF00F, 0x6007, Not),
    pattern!(0xF00F, 0x2008, Tst),
    pattern!(0xFF00, 0xC800, TstImm),
    pattern!(0xFF00, 0xCC00, TstB),
    pattern!(0xF0FF, 0x4000, Shll),
    pattern!(0xF0FF, 0x4001, Shlr),
    pattern!(0xF0FF, 0x4020, Shal),
    pattern!(0xF0FF, 0x4021, Shar),
    pattern!(0xF0FF, 0x4008, Shll2),
    pattern!(0xF0FF, 0x4009, Shlr2),
    pattern!(0xF0FF, 0x4018, Shll8),
    pattern!(0xF0FF, 0x4019, Shlr8),
    pattern!(0xF0FF, 0x4028, Shll16),
    pattern!(0xF0FF, 0x4029, Shlr16),
    pattern!(0xF0FF, 0x4004, Rotl),
    pattern!(0xF0FF, 0x4005, Rotr),
    pattern!(0xF0FF, 0x4024, Rotcl),
    pattern!(0xF0FF, 0x4025, Rotcr),
    pattern!(0xF00F, 0x600E, ExtsB),
    pattern!(0xF00F, 0x600F, ExtsW),
    pattern!(0xF00F, 0x600C, ExtuB),
    pattern!(0xF00F, 0x600D, ExtuW),
    pattern!(0xF00F, 0x6008, SwapB),
    pattern!(0xF00F, 0x6009, SwapW),
    pattern!(0xF00F, 0x200D, Xtrct),
    pattern!(0xF00F, 0x3000, CmpEq),
    pattern!(0xFF00, 0x8800, CmpEqImm),
    pattern!(0xF00F, 0x3003, CmpGe),
    pattern!(0xF00F, 0x3007, CmpGt),
    pattern!(0xF00F, 0x3006, CmpHi),
    pattern!(0xF00F, 0x3002, CmpHs),
    pattern!(0xF0FF, 0x4015, CmpPl),
    pattern!(0xF0FF, 0x4011, CmpPz),
    pattern!(0xF00F, 0x200C, CmpStr),
    pattern!(0xFF00, 0x8900, Bt),
    pattern!(0xFF00, 0x8B00, Bf),
    pattern!(0xFF00, 0x8D00, Bts),
    pattern!(0xFF00, 0x8F00, Bfs),
    pattern!(0xF000, 0xA000, Bra),
    pattern!(0xF000, 0xB000, Bsr),
    pattern!(0xF0FF, 0x402B, Jmp),
    pattern!(0xF0FF, 0x400B, Jsr),
    pattern!(0xFFFF, 0x000B, Rts),
    pattern!(0xFFFF, 0x002B, Rte),
    pattern!(0xF0FF, 0x401E, LdcGbr),
    pattern!(0xF0FF, 0x402E, LdcVbr),
    pattern!(0xF0FF, 0x400E, LdcSr),
    pattern!(0xF0FF, 0x4017, LdcLGbr),
    pattern!(0xF0FF, 0x4027, LdcLVbr),
    pattern!(0xF0FF, 0x4007, LdcLSr),
    pattern!(0xF0FF, 0x0012, StcGbr),
    pattern!(0xF0FF, 0x0022, StcVbr),
    pattern!(0xF0FF, 0x0002, StcSr),
    pattern!(0xF0FF, 0x4013, StcLGbr),
    pattern!(0xF0FF, 0x4023, StcLVbr),
    pattern!(0xF0FF, 0x4003, StcLSr),
    pattern!(0xF0FF, 0x400A, LdsMach),
    pattern!(0xF0FF, 0x401A, LdsMacl),
    pattern!(0xF0FF, 0x402A, LdsPr),
    pattern!(0xF0FF, 0x4006, LdsLMach),
    pattern!(0xF0FF, 0x4016, LdsLMacl),
    pattern!(0xF0FF, 0x4026, LdsLPr),
    pattern!(0xF0FF, 0x000A, StsMach),
    pattern!(0xF0FF, 0x001A, StsMacl),
    pattern!(0xF0FF, 0x002A, StsPr),
    pattern!(0xF0FF, 0x4002, StsLMach),
    pattern!(0xF0FF, 0x4012, StsLMacl),
    pattern!(0xF0FF, 0x4022, StsLPr),
    pattern!(0xFFFF, 0x0009, Nop),
    pattern!(0xFFFF, 0x001B, Sleep),
    pattern!(0xFFFF, 0x0018, SetT),
    pattern!(0xFFFF, 0x0008, ClrT),
    pattern!(0xFFFF, 0x0028, ClrMac),
    pattern!(0xFF00, 0xC300, Trapa),
    pattern!(0xF0FF, 0x0003, Bsrf),
    pattern!(0xF0FF, 0x0023, Braf),
    pattern!(0xF0FF, 0x401B, Tas),
];

#[test]
fn every_word_matches_exactly_one_manual_pattern_or_is_reserved() {
    let mut manual_forms = BTreeSet::new();
    let mut documented_words = 0usize;

    for word in u16::MIN..=u16::MAX {
        let matches: Vec<_> = MANUAL_PATTERNS
            .iter()
            .filter(|pattern| word & pattern.mask == pattern.value)
            .collect();

        match (matches.as_slice(), classify_word(word)) {
            ([expected], DecodedWord::Instruction(instruction)) => {
                documented_words += 1;
                manual_forms.insert(expected.form);
                assert_eq!(instruction.form(), expected.form, "word 0x{word:04X}");
                assert_eq!(encode_word(&instruction), Ok(word), "word 0x{word:04X}");
            }
            ([], DecodedWord::Unknown { word: actual }) => assert_eq!(actual, word),
            ([], DecodedWord::Instruction(instruction)) => {
                panic!("reserved manual word 0x{word:04X} decoded as {instruction:?}")
            }
            ([expected], DecodedWord::Unknown { .. }) => {
                panic!(
                    "manual pattern {:04X}/{:04X} rejected word 0x{word:04X}",
                    expected.value, expected.mask
                )
            }
            _ => panic!("manual patterns overlap at word 0x{word:04X}"),
        }
    }

    assert_eq!(documented_words, 53_752);
    assert_eq!(manual_forms, InstructionForm::all().collect());
}
