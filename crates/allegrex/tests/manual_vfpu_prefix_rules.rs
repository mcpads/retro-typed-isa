use std::collections::BTreeMap;

use allegrex::{VfpuForm, VfpuPrefixRules, VfpuPrefixUse};

// Normative source: Sony Computer Entertainment, VFPU User's Manual,
// PSP Hardware Manual Release 1.0.0, section 8.5. These groups transcribe
// every row of the manual's prefix-validity table independently of the
// production operation grouping.

const fn rules(
    source: VfpuPrefixUse,
    target: VfpuPrefixUse,
    destination: VfpuPrefixUse,
) -> VfpuPrefixRules {
    VfpuPrefixRules {
        source,
        target,
        destination,
    }
}

const MANUAL_PREFIX_RULE_GROUPS: &[(VfpuPrefixRules, &[&str])] = &[
    (
        rules(
            VfpuPrefixUse::NoEffect,
            VfpuPrefixUse::NoEffect,
            VfpuPrefixUse::NoEffect,
        ),
        &[
            "bvfl", "bvf", "bvtl", "bvt", "lv.q", "lv.s", "mfvc", "mfv", "mtvc", "mtv", "sv.q,wb",
            "sv.q", "sv.s", "svl.q", "svr.q", "vflush", "vmfvc", "vmtvc", "vnop", "vrnds.s",
            "vsync2", "vsync",
        ],
    ),
    (
        rules(
            VfpuPrefixUse::NoEffect,
            VfpuPrefixUse::NoEffect,
            VfpuPrefixUse::Overwrite,
        ),
        &["vpfxd"],
    ),
    (
        rules(
            VfpuPrefixUse::NoEffect,
            VfpuPrefixUse::NoEffect,
            VfpuPrefixUse::Prohibited,
        ),
        &[
            "vrndf1.p", "vrndf1.q", "vrndf1.t", "vrndf2.p", "vrndf2.q", "vrndf2.t", "vrndi.p",
            "vrndi.q", "vrndi.t",
        ],
    ),
    (
        rules(
            VfpuPrefixUse::NoEffect,
            VfpuPrefixUse::NoEffect,
            VfpuPrefixUse::Valid,
        ),
        &[
            "vcst.p", "vcst.q", "vcst.s", "vcst.t", "vfim.s", "viim.s", "vrndf1.s", "vrndf2.s",
            "vrndi.s",
        ],
    ),
    (
        rules(
            VfpuPrefixUse::NoEffect,
            VfpuPrefixUse::Overwrite,
            VfpuPrefixUse::NoEffect,
        ),
        &["vpfxt"],
    ),
    (
        rules(
            VfpuPrefixUse::OnlySwizzle,
            VfpuPrefixUse::NoEffect,
            VfpuPrefixUse::OnlyWriteMask,
        ),
        &[
            "vi2c.q", "vi2s.p", "vi2s.q", "vi2uc.q", "vi2us.p", "vi2us.q",
        ],
    ),
    (
        rules(
            VfpuPrefixUse::OnlySwizzle,
            VfpuPrefixUse::NoEffect,
            VfpuPrefixUse::Prohibited,
        ),
        &["vt4444.q", "vt5551.q", "vt5650.q"],
    ),
    (
        rules(
            VfpuPrefixUse::OnlySwizzle,
            VfpuPrefixUse::NoEffect,
            VfpuPrefixUse::Valid,
        ),
        &[
            "vabs.p", "vabs.q", "vabs.s", "vabs.t", "vi2f.p", "vi2f.q", "vi2f.s", "vi2f.t",
            "vneg.p", "vneg.q", "vneg.s", "vneg.t",
        ],
    ),
    (
        rules(
            VfpuPrefixUse::Overwrite,
            VfpuPrefixUse::NoEffect,
            VfpuPrefixUse::NoEffect,
        ),
        &["vpfxs"],
    ),
    (
        rules(
            VfpuPrefixUse::Prohibited,
            VfpuPrefixUse::NoEffect,
            VfpuPrefixUse::OnlyWriteMask,
        ),
        &[
            "vc2i.s",
            "vs2i.p",
            "vs2i.s",
            "vuc2ifs.s",
            "vus2i.p",
            "vus2i.s",
        ],
    ),
    (
        rules(
            VfpuPrefixUse::Prohibited,
            VfpuPrefixUse::NoEffect,
            VfpuPrefixUse::Prohibited,
        ),
        &[
            "vasin.p", "vasin.q", "vasin.t", "vcos.p", "vcos.q", "vcos.t", "vexp2.p", "vexp2.q",
            "vexp2.t", "vlog2.p", "vlog2.q", "vlog2.t", "vmidt.p", "vmidt.q", "vmidt.t", "vmmov.p",
            "vmmov.q", "vmmov.t", "vmone.p", "vmone.q", "vmone.t", "vmzero.p", "vmzero.q",
            "vmzero.t", "vnrcp.p", "vnrcp.q", "vnrcp.t", "vnsin.p", "vnsin.q", "vnsin.t", "vrcp.p",
            "vrcp.q", "vrcp.t", "vrexp2.p", "vrexp2.q", "vrexp2.t", "vrot.p", "vrot.q", "vrot.t",
            "vrsq.p", "vrsq.q", "vrsq.t", "vsin.p", "vsin.q", "vsin.t", "vsqrt.p", "vsqrt.q",
            "vsqrt.t",
        ],
    ),
    (
        rules(
            VfpuPrefixUse::Prohibited,
            VfpuPrefixUse::NoEffect,
            VfpuPrefixUse::Valid,
        ),
        &[
            "vh2f.p", "vh2f.s", "vidt.p", "vidt.q", "vnrcp.s", "vnsin.s", "vone.p", "vone.q",
            "vone.s", "vone.t", "vrexp2.s", "vzero.p", "vzero.q", "vzero.s", "vzero.t",
        ],
    ),
    (
        rules(
            VfpuPrefixUse::Prohibited,
            VfpuPrefixUse::Prohibited,
            VfpuPrefixUse::Prohibited,
        ),
        &[
            "vcrsp.t", "vhtfm2.p", "vhtfm3.t", "vhtfm4.q", "vmmul.p", "vmmul.q", "vmmul.t",
            "vmscl.p", "vmscl.q", "vmscl.t", "vqmul.q", "vsocp.p", "vsocp.s", "vtfm2.p", "vtfm3.t",
            "vtfm4.q",
        ],
    ),
    (
        rules(
            VfpuPrefixUse::Prohibited,
            VfpuPrefixUse::Prohibited,
            VfpuPrefixUse::Valid,
        ),
        &[
            "vbfy1.p", "vbfy1.q", "vbfy2.q", "vcrs.t", "vocp.p", "vocp.q", "vocp.s", "vocp.t",
            "vsrt1.q", "vsrt2.q", "vsrt3.q", "vsrt4.q",
        ],
    ),
    (
        rules(
            VfpuPrefixUse::Prohibited,
            VfpuPrefixUse::Valid,
            VfpuPrefixUse::Valid,
        ),
        &["vhdp.p", "vhdp.q", "vhdp.t"],
    ),
    (
        rules(
            VfpuPrefixUse::Valid,
            VfpuPrefixUse::NoEffect,
            VfpuPrefixUse::OnlyWriteMask,
        ),
        &[
            "vf2h.p", "vf2h.q", "vf2id.p", "vf2id.q", "vf2id.s", "vf2id.t", "vf2in.p", "vf2in.q",
            "vf2in.s", "vf2in.t", "vf2iu.p", "vf2iu.q", "vf2iu.s", "vf2iu.t", "vf2iz.p", "vf2iz.q",
            "vf2iz.s", "vf2iz.t",
        ],
    ),
    (
        rules(
            VfpuPrefixUse::Valid,
            VfpuPrefixUse::NoEffect,
            VfpuPrefixUse::Prohibited,
        ),
        &[
            "vsat0.p", "vsat0.q", "vsat0.s", "vsat0.t", "vsat1.p", "vsat1.q", "vsat1.s", "vsat1.t",
        ],
    ),
    (
        rules(
            VfpuPrefixUse::Valid,
            VfpuPrefixUse::NoEffect,
            VfpuPrefixUse::Valid,
        ),
        &[
            "vasin.s", "vcos.s", "vexp2.s", "vlgb.s", "vlog2.s", "vmov.p", "vmov.q", "vmov.s",
            "vmov.t", "vrcp.s", "vrsq.s", "vsbz.s", "vsin.s", "vsqrt.s", "vwbn.s",
        ],
    ),
    (
        rules(
            VfpuPrefixUse::Valid,
            VfpuPrefixUse::Prohibited,
            VfpuPrefixUse::Prohibited,
        ),
        &[
            "vcmovf.p", "vcmovf.q", "vcmovf.s", "vcmovf.t", "vcmovt.p", "vcmovt.q", "vcmovt.s",
            "vcmovt.t",
        ],
    ),
    (
        rules(
            VfpuPrefixUse::Valid,
            VfpuPrefixUse::Prohibited,
            VfpuPrefixUse::Valid,
        ),
        &[
            "vavg.p", "vavg.q", "vavg.t", "vdet.p", "vfad.p", "vfad.q", "vfad.t", "vscl.p",
            "vscl.q", "vscl.t", "vsgn.p", "vsgn.q", "vsgn.s", "vsgn.t",
        ],
    ),
    (
        rules(
            VfpuPrefixUse::Valid,
            VfpuPrefixUse::Valid,
            VfpuPrefixUse::NoEffect,
        ),
        &["vcmp.p", "vcmp.q", "vcmp.s", "vcmp.t"],
    ),
    (
        rules(
            VfpuPrefixUse::Valid,
            VfpuPrefixUse::Valid,
            VfpuPrefixUse::Valid,
        ),
        &[
            "vadd.p", "vadd.q", "vadd.s", "vadd.t", "vdiv.s", "vdot.p", "vdot.q", "vdot.t",
            "vmax.p", "vmax.q", "vmax.s", "vmax.t", "vmin.p", "vmin.q", "vmin.s", "vmin.t",
            "vmul.p", "vmul.q", "vmul.s", "vmul.t", "vsbn.s", "vscmp.p", "vscmp.q", "vscmp.s",
            "vscmp.t", "vsge.p", "vsge.q", "vsge.s", "vsge.t", "vslt.p", "vslt.q", "vslt.s",
            "vslt.t", "vsub.p", "vsub.q", "vsub.s", "vsub.t",
        ],
    ),
];

#[test]
fn every_manual_vfpu_form_has_its_source_prefix_rules() {
    let mut expected = BTreeMap::new();
    for (rules, mnemonics) in MANUAL_PREFIX_RULE_GROUPS {
        for mnemonic in *mnemonics {
            assert!(expected.insert(*mnemonic, *rules).is_none(), "{mnemonic}");
        }
    }
    assert_eq!(expected.len(), 268);

    let actual: BTreeMap<_, _> = VfpuForm::all()
        .map(|form| (form.mnemonic(), form.prefix_rules()))
        .collect();
    assert_eq!(actual, expected);
}
