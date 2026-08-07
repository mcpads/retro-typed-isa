use std::collections::BTreeSet;

use powerpc_gekko_750cl::{
    DECLARED_FORM_COUNT, InstructionForm, PowerPc750Cl, PowerPcGekko, decode_750cl_word,
    encode_750cl_word,
};
use typed_isa_core::TypedIsa;

// IBM Gekko User's Manual Appendix A and IBM 750CL User's Manual Appendix A
// Table A-1 plus Appendix B Table B-1. Both manuals define this same selector
// set. Gekko/750CL SPR-map differences are covered separately.
// These canonical selector words are intentionally independent of the
// production form iterator and decoder tables.
//
// Table B-1 misprints ps_merge10 as binary 1001010101. The instruction's own
// page 583 gives x'1000 04A0', while the timing table on page 242 and Table B-4
// both give XO 592 (binary 1001010000); the canonical word below follows those
// three agreeing locations.
const SOURCE_CANONICAL_WORDS: [u32; 222] = [
    0x0c000000, 0x100007ec, 0x1001004c, 0x1000000c, 0x1001004e, 0x1000000e, 0x10000210, 0x1000002a,
    0x10000040, 0x100000c0, 0x10000000, 0x10000080, 0x10000024, 0x1000003a, 0x1000001c, 0x1000001e,
    0x10000420, 0x10000460, 0x100004a0, 0x100004e0, 0x10000090, 0x10000038, 0x10000032, 0x10000018,
    0x1000001a, 0x10000110, 0x10000050, 0x1000003e, 0x1000003c, 0x10000030, 0x10000034, 0x1000002e,
    0x10000028, 0x10000014, 0x10000016, 0x1c000000, 0x20000000, 0x28000000, 0x2c000000, 0x30000000,
    0x34000000, 0x38000000, 0x3c000000, 0x40000000, 0x44000002, 0x48000000, 0x4e800420, 0x4c000020,
    0x4c000202, 0x4c000102, 0x4c000242, 0x4c0001c2, 0x4c000042, 0x4c000382, 0x4c000342, 0x4c000182,
    0x4c00012c, 0x4c000000, 0x4c000064, 0x50000000, 0x54000000, 0x5c000000, 0x60000000, 0x64000000,
    0x68000000, 0x6c000000, 0x70000000, 0x74000000, 0x7c000214, 0x7c000014, 0x7c000114, 0x7c0001d4,
    0x7c000194, 0x7c000038, 0x7c000078, 0x7c000000, 0x7c000040, 0x7c000034, 0x7c0000ac, 0x7c0003ac,
    0x7c00006c, 0x7c00022c, 0x7c0001ec, 0x7c0007ec, 0x7c0003d6, 0x7c000396, 0x7c00026c, 0x7c00036c,
    0x7c0006ac, 0x7c000238, 0x7c000774, 0x7c000734, 0x7c0007ac, 0x7c4100ee, 0x7c0000ae, 0x7c0104ee,
    0x7c0004ae, 0x7c01046e, 0x7c00042e, 0x7c4102ee, 0x7c0002ae, 0x7c00062c, 0x7c41026e, 0x7c00022e,
    0x7c4104aa, 0x7c411c2a, 0x7c000028, 0x7c00042c, 0x7c41006e, 0x7c00002e, 0x7c000400, 0x7c000026,
    0x7c0000a6, 0x7c0102a6, 0x7c0004a6, 0x7c000526, 0x7c0c42e6, 0x7c000120, 0x7c000124, 0x7c0103a6,
    0x7c0001a4, 0x7c0001e4, 0x7c000096, 0x7c000016, 0x7c0001d6, 0x7c0003b8, 0x7c0000d0, 0x7c0000f8,
    0x7c000378, 0x7c000338, 0x7c000030, 0x7c000630, 0x7c000670, 0x7c000430, 0x7c0101ee, 0x7c0001ae,
    0x7c0105ee, 0x7c0005ae, 0x7c0007ae, 0x7c01056e, 0x7c00052e, 0x7c00072c, 0x7c01036e, 0x7c00032e,
    0x7c0005aa, 0x7c00052a, 0x7c00052c, 0x7c00012d, 0x7c01016e, 0x7c00012e, 0x7c000050, 0x7c000010,
    0x7c000110, 0x7c0001d0, 0x7c000190, 0x7c0004ac, 0x7c000264, 0x7c00046c, 0x7c000008, 0x7c000278,
    0x80000000, 0x84410000, 0x88000000, 0x8c410000, 0x90000000, 0x94010000, 0x98000000, 0x9c010000,
    0xa0000000, 0xa4410000, 0xa8000000, 0xac410000, 0xb0000000, 0xb4010000, 0xb8410000, 0xbc000000,
    0xc0000000, 0xc4010000, 0xc8000000, 0xcc010000, 0xd0000000, 0xd4010000, 0xd8000000, 0xdc010000,
    0xe0000000, 0xe4010000, 0xec00002a, 0xec000024, 0xec00003a, 0xec000038, 0xec000032, 0xec00003e,
    0xec00003c, 0xec000030, 0xec000028, 0xf0000000, 0xf4010000, 0xfc000210, 0xfc00002a, 0xfc000040,
    0xfc000000, 0xfc00001c, 0xfc00001e, 0xfc000024, 0xfc00003a, 0xfc000090, 0xfc000038, 0xfc000032,
    0xfc000110, 0xfc000050, 0xfc00003e, 0xfc00003c, 0xfc000018, 0xfc000034, 0xfc00002e, 0xfc000028,
    0xfc000080, 0xfc00048e, 0xfc00008c, 0xfc00004c, 0xfc00058e, 0xfc00010c,
];

// Counts read from the same two IBM tables by six-bit primary opcode. The
// denominator is not computed from InstructionForm::ALL.
const SOURCE_PRIMARY_FORM_COUNTS: [usize; 64] = [
    0, 0, 0, 1, 34, 0, 0, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 13, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 0,
    92, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 9, 1, 1,
    0, 25,
];

#[test]
fn ibm_source_tables_cover_every_declared_form_for_both_profiles() {
    let source_forms = SOURCE_CANONICAL_WORDS
        .into_iter()
        .map(|word| {
            let instruction = decode_750cl_word(word, 0).unwrap();
            assert_eq!(encode_750cl_word(&instruction, 0), Ok(word));
            let bytes = word.to_be_bytes();
            assert_eq!(
                PowerPcGekko::decode_one(&bytes, 0)
                    .unwrap()
                    .into_instruction(),
                instruction
            );
            assert_eq!(
                PowerPc750Cl::decode_one(&bytes, 0)
                    .unwrap()
                    .into_instruction(),
                instruction
            );
            instruction.form()
        })
        .collect::<BTreeSet<_>>();
    let declared_forms = InstructionForm::ALL.into_iter().collect::<BTreeSet<_>>();

    assert_eq!(DECLARED_FORM_COUNT, 222);
    assert_eq!(source_forms.len(), 222);
    assert_eq!(source_forms, declared_forms);
}

#[test]
fn source_primary_opcode_distribution_is_preserved() {
    let mut actual = [0usize; 64];
    for word in SOURCE_CANONICAL_WORDS {
        actual[(word >> 26) as usize] += 1;
    }
    assert_eq!(actual, SOURCE_PRIMARY_FORM_COUNTS);
}

#[test]
fn independent_mpc750_boundary_exposes_the_750cl_extension() {
    // MPC750UM section 2.3.1.3 lists primary opcode 4 as illegal. IBM 750CL
    // Appendix B assigns 34 forms there and four D-form quantized operations
    // to primary opcodes 56, 57, 60, and 61.
    assert_eq!(SOURCE_PRIMARY_FORM_COUNTS[4], 34);
    assert_eq!(SOURCE_PRIMARY_FORM_COUNTS[56], 1);
    assert_eq!(SOURCE_PRIMARY_FORM_COUNTS[57], 1);
    assert_eq!(SOURCE_PRIMARY_FORM_COUNTS[60], 1);
    assert_eq!(SOURCE_PRIMARY_FORM_COUNTS[61], 1);
}
