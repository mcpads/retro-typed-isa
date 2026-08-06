use std::fmt;

use crate::instruction::Instruction;

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // === Data transfer ===
            Instruction::MovImm { imm, rn } => write!(f, "MOV     #{},{}", imm, rn),
            Instruction::MovWPcRel { disp, rn } => {
                write!(f, "MOV.W   @({},PC),{}", (*disp as u16) * 2, rn)
            }
            Instruction::MovLPcRel { disp, rn } => {
                write!(f, "MOV.L   @({},PC),{}", (*disp as u16) * 4, rn)
            }
            Instruction::MovReg { rm, rn } => write!(f, "MOV     {},{}", rm, rn),
            Instruction::MovBStore { rm, rn } => write!(f, "MOV.B   {},@{}", rm, rn),
            Instruction::MovWStore { rm, rn } => write!(f, "MOV.W   {},@{}", rm, rn),
            Instruction::MovLStore { rm, rn } => write!(f, "MOV.L   {},@{}", rm, rn),
            Instruction::MovBLoad { rm, rn } => write!(f, "MOV.B   @{},{}", rm, rn),
            Instruction::MovWLoad { rm, rn } => write!(f, "MOV.W   @{},{}", rm, rn),
            Instruction::MovLLoad { rm, rn } => write!(f, "MOV.L   @{},{}", rm, rn),
            Instruction::MovBStorePreDec { rm, rn } => {
                write!(f, "MOV.B   {},@-{}", rm, rn)
            }
            Instruction::MovWStorePreDec { rm, rn } => {
                write!(f, "MOV.W   {},@-{}", rm, rn)
            }
            Instruction::MovLStorePreDec { rm, rn } => {
                write!(f, "MOV.L   {},@-{}", rm, rn)
            }
            Instruction::MovBLoadPostInc { rm, rn } => {
                write!(f, "MOV.B   @{}+,{}", rm, rn)
            }
            Instruction::MovWLoadPostInc { rm, rn } => {
                write!(f, "MOV.W   @{}+,{}", rm, rn)
            }
            Instruction::MovLLoadPostInc { rm, rn } => {
                write!(f, "MOV.L   @{}+,{}", rm, rn)
            }
            Instruction::MovBR0LoadIndexed { rm, rn } => {
                write!(f, "MOV.B   @(R0,{}),{}", rm, rn)
            }
            Instruction::MovWR0LoadIndexed { rm, rn } => {
                write!(f, "MOV.W   @(R0,{}),{}", rm, rn)
            }
            Instruction::MovLR0LoadIndexed { rm, rn } => {
                write!(f, "MOV.L   @(R0,{}),{}", rm, rn)
            }
            Instruction::MovBR0StoreIndexed { rm, rn } => {
                write!(f, "MOV.B   {},@(R0,{})", rm, rn)
            }
            Instruction::MovWR0StoreIndexed { rm, rn } => {
                write!(f, "MOV.W   {},@(R0,{})", rm, rn)
            }
            Instruction::MovLR0StoreIndexed { rm, rn } => {
                write!(f, "MOV.L   {},@(R0,{})", rm, rn)
            }
            Instruction::MovBDispStore { disp, rn } => {
                write!(f, "MOV.B   R0,@({},{})", disp, rn)
            }
            Instruction::MovWDispStore { disp, rn } => {
                write!(f, "MOV.W   R0,@({},{})", (*disp as u16) * 2, rn)
            }
            Instruction::MovLDispStore { disp, rm, rn } => {
                write!(f, "MOV.L   {},@({},{})", rm, (*disp as u16) * 4, rn)
            }
            Instruction::MovBDispLoad { disp, rm } => {
                write!(f, "MOV.B   @({},{}),R0", disp, rm)
            }
            Instruction::MovWDispLoad { disp, rm } => {
                write!(f, "MOV.W   @({},{}),R0", (*disp as u16) * 2, rm)
            }
            Instruction::MovLDispLoad { disp, rm, rn } => {
                write!(f, "MOV.L   @({},{}),{}", (*disp as u16) * 4, rm, rn)
            }
            Instruction::MovBGbrStore { disp } => {
                write!(f, "MOV.B   R0,@({},GBR)", disp)
            }
            Instruction::MovBGbrLoad { disp } => write!(f, "MOV.B   @({},GBR),R0", disp),
            Instruction::MovWGbrStore { disp } => {
                write!(f, "MOV.W   R0,@({},GBR)", (*disp as u16) * 2)
            }
            Instruction::MovWGbrLoad { disp } => {
                write!(f, "MOV.W   @({},GBR),R0", (*disp as u16) * 2)
            }
            Instruction::MovLGbrStore { disp } => {
                write!(f, "MOV.L   R0,@({},GBR)", (*disp as u16) * 4)
            }
            Instruction::MovLGbrLoad { disp } => {
                write!(f, "MOV.L   @({},GBR),R0", (*disp as u16) * 4)
            }
            Instruction::Mova { disp } => {
                write!(f, "MOVA    @({},PC),R0", (*disp as u16) * 4)
            }
            Instruction::MovT { rn } => write!(f, "MOVT    {}", rn),

            // === Arithmetic ===
            Instruction::Add { rm, rn } => write!(f, "ADD     {},{}", rm, rn),
            Instruction::AddImm { imm, rn } => write!(f, "ADD     #{},{}", imm, rn),
            Instruction::Addc { rm, rn } => write!(f, "ADDC    {},{}", rm, rn),
            Instruction::Addv { rm, rn } => write!(f, "ADDV    {},{}", rm, rn),
            Instruction::Sub { rm, rn } => write!(f, "SUB     {},{}", rm, rn),
            Instruction::Subc { rm, rn } => write!(f, "SUBC    {},{}", rm, rn),
            Instruction::Subv { rm, rn } => write!(f, "SUBV    {},{}", rm, rn),
            Instruction::Neg { rm, rn } => write!(f, "NEG     {},{}", rm, rn),
            Instruction::Negc { rm, rn } => write!(f, "NEGC    {},{}", rm, rn),
            Instruction::Dt { rn } => write!(f, "DT      {}", rn),
            Instruction::MulL { rm, rn } => write!(f, "MUL.L   {},{}", rm, rn),
            Instruction::MulS { rm, rn } => write!(f, "MULS.W  {},{}", rm, rn),
            Instruction::MulU { rm, rn } => write!(f, "MULU.W  {},{}", rm, rn),
            Instruction::DmulS { rm, rn } => write!(f, "DMULS.L {},{}", rm, rn),
            Instruction::DmulU { rm, rn } => write!(f, "DMULU.L {},{}", rm, rn),
            Instruction::Div0S { rm, rn } => write!(f, "DIV0S   {},{}", rm, rn),
            Instruction::Div0U => write!(f, "DIV0U"),
            Instruction::Div1 { rm, rn } => write!(f, "DIV1    {},{}", rm, rn),
            Instruction::MacL { rm, rn } => write!(f, "MAC.L   @{}+,@{}+", rm, rn),
            Instruction::MacW { rm, rn } => write!(f, "MAC.W   @{}+,@{}+", rm, rn),

            // === Logical ===
            Instruction::And { rm, rn } => write!(f, "AND     {},{}", rm, rn),
            Instruction::AndImm { imm } => write!(f, "AND     #{},R0", imm),
            Instruction::AndB { imm } => write!(f, "AND.B   #{},@(R0,GBR)", imm),
            Instruction::Or { rm, rn } => write!(f, "OR      {},{}", rm, rn),
            Instruction::OrImm { imm } => write!(f, "OR      #{},R0", imm),
            Instruction::OrB { imm } => write!(f, "OR.B    #{},@(R0,GBR)", imm),
            Instruction::Xor { rm, rn } => write!(f, "XOR     {},{}", rm, rn),
            Instruction::XorImm { imm } => write!(f, "XOR     #{},R0", imm),
            Instruction::XorB { imm } => write!(f, "XOR.B   #{},@(R0,GBR)", imm),
            Instruction::Not { rm, rn } => write!(f, "NOT     {},{}", rm, rn),
            Instruction::Tst { rm, rn } => write!(f, "TST     {},{}", rm, rn),
            Instruction::TstImm { imm } => write!(f, "TST     #{},R0", imm),
            Instruction::TstB { imm } => write!(f, "TST.B   #{},@(R0,GBR)", imm),

            // === Shifts ===
            Instruction::Shll { rn } => write!(f, "SHLL    {}", rn),
            Instruction::Shlr { rn } => write!(f, "SHLR    {}", rn),
            Instruction::Shal { rn } => write!(f, "SHAL    {}", rn),
            Instruction::Shar { rn } => write!(f, "SHAR    {}", rn),
            Instruction::Shll2 { rn } => write!(f, "SHLL2   {}", rn),
            Instruction::Shlr2 { rn } => write!(f, "SHLR2   {}", rn),
            Instruction::Shll8 { rn } => write!(f, "SHLL8   {}", rn),
            Instruction::Shlr8 { rn } => write!(f, "SHLR8   {}", rn),
            Instruction::Shll16 { rn } => write!(f, "SHLL16  {}", rn),
            Instruction::Shlr16 { rn } => write!(f, "SHLR16  {}", rn),
            Instruction::Rotl { rn } => write!(f, "ROTL    {}", rn),
            Instruction::Rotr { rn } => write!(f, "ROTR    {}", rn),
            Instruction::Rotcl { rn } => write!(f, "ROTCL   {}", rn),
            Instruction::Rotcr { rn } => write!(f, "ROTCR   {}", rn),
            Instruction::Shad { rm, rn } => write!(f, "SHAD    {},{}", rm, rn),
            Instruction::Shld { rm, rn } => write!(f, "SHLD    {},{}", rm, rn),

            // === Sign/zero extension ===
            Instruction::ExtsB { rm, rn } => write!(f, "EXTS.B  {},{}", rm, rn),
            Instruction::ExtsW { rm, rn } => write!(f, "EXTS.W  {},{}", rm, rn),
            Instruction::ExtuB { rm, rn } => write!(f, "EXTU.B  {},{}", rm, rn),
            Instruction::ExtuW { rm, rn } => write!(f, "EXTU.W  {},{}", rm, rn),
            Instruction::SwapB { rm, rn } => write!(f, "SWAP.B  {},{}", rm, rn),
            Instruction::SwapW { rm, rn } => write!(f, "SWAP.W  {},{}", rm, rn),
            Instruction::Xtrct { rm, rn } => write!(f, "XTRCT   {},{}", rm, rn),

            // === Comparisons ===
            Instruction::CmpEq { rm, rn } => write!(f, "CMP/EQ  {},{}", rm, rn),
            Instruction::CmpEqImm { imm } => write!(f, "CMP/EQ  #{},R0", imm),
            Instruction::CmpGe { rm, rn } => write!(f, "CMP/GE  {},{}", rm, rn),
            Instruction::CmpGt { rm, rn } => write!(f, "CMP/GT  {},{}", rm, rn),
            Instruction::CmpHi { rm, rn } => write!(f, "CMP/HI  {},{}", rm, rn),
            Instruction::CmpHs { rm, rn } => write!(f, "CMP/HS  {},{}", rm, rn),
            Instruction::CmpPl { rn } => write!(f, "CMP/PL  {}", rn),
            Instruction::CmpPz { rn } => write!(f, "CMP/PZ  {}", rn),
            Instruction::CmpStr { rm, rn } => write!(f, "CMP/STR {},{}", rm, rn),

            // === Branches ===
            Instruction::Bt { disp } => {
                write!(f, "BT      {}", (*disp as i32) * 2)
            }
            Instruction::Bf { disp } => {
                write!(f, "BF      {}", (*disp as i32) * 2)
            }
            Instruction::Bts { disp } => {
                write!(f, "BT/S    {}", (*disp as i32) * 2)
            }
            Instruction::Bfs { disp } => {
                write!(f, "BF/S    {}", (*disp as i32) * 2)
            }
            Instruction::Bra { disp } => {
                write!(f, "BRA     {}", (*disp as i32) * 2)
            }
            Instruction::Bsr { disp } => {
                write!(f, "BSR     {}", (*disp as i32) * 2)
            }
            Instruction::Jmp { rm } => write!(f, "JMP     @{}", rm),
            Instruction::Jsr { rm } => write!(f, "JSR     @{}", rm),
            Instruction::Rts => write!(f, "RTS"),
            Instruction::Rte => write!(f, "RTE"),

            // === System register transfer ===
            Instruction::LdcGbr { rm } => write!(f, "LDC     {},GBR", rm),
            Instruction::LdcVbr { rm } => write!(f, "LDC     {},VBR", rm),
            Instruction::LdcSr { rm } => write!(f, "LDC     {},SR", rm),
            Instruction::LdcLGbr { rm } => write!(f, "LDC.L   @{}+,GBR", rm),
            Instruction::LdcLVbr { rm } => write!(f, "LDC.L   @{}+,VBR", rm),
            Instruction::LdcLSr { rm } => write!(f, "LDC.L   @{}+,SR", rm),
            Instruction::StcGbr { rn } => write!(f, "STC     GBR,{}", rn),
            Instruction::StcVbr { rn } => write!(f, "STC     VBR,{}", rn),
            Instruction::StcSr { rn } => write!(f, "STC     SR,{}", rn),
            Instruction::StcLGbr { rn } => write!(f, "STC.L   GBR,@-{}", rn),
            Instruction::StcLVbr { rn } => write!(f, "STC.L   VBR,@-{}", rn),
            Instruction::StcLSr { rn } => write!(f, "STC.L   SR,@-{}", rn),
            Instruction::LdsMach { rm } => write!(f, "LDS     {},MACH", rm),
            Instruction::LdsMacl { rm } => write!(f, "LDS     {},MACL", rm),
            Instruction::LdsPr { rm } => write!(f, "LDS     {},PR", rm),
            Instruction::LdsLMach { rm } => write!(f, "LDS.L   @{}+,MACH", rm),
            Instruction::LdsLMacl { rm } => write!(f, "LDS.L   @{}+,MACL", rm),
            Instruction::LdsLPr { rm } => write!(f, "LDS.L   @{}+,PR", rm),
            Instruction::StsMach { rn } => write!(f, "STS     MACH,{}", rn),
            Instruction::StsMacl { rn } => write!(f, "STS     MACL,{}", rn),
            Instruction::StsPr { rn } => write!(f, "STS     PR,{}", rn),
            Instruction::StsLMach { rn } => write!(f, "STS.L   MACH,@-{}", rn),
            Instruction::StsLMacl { rn } => write!(f, "STS.L   MACL,@-{}", rn),
            Instruction::StsLPr { rn } => write!(f, "STS.L   PR,@-{}", rn),

            // === Special ===
            Instruction::Nop => write!(f, "NOP"),
            Instruction::Sleep => write!(f, "SLEEP"),
            Instruction::SetT => write!(f, "SETT"),
            Instruction::ClrT => write!(f, "CLRT"),
            Instruction::ClrMac => write!(f, "CLRMAC"),
            Instruction::Trapa { imm } => write!(f, "TRAPA   #{}", imm),
            Instruction::Bsrf { rm } => write!(f, "BSRF    {}", rm),
            Instruction::Braf { rm } => write!(f, "BRAF    {}", rm),
            Instruction::Tas { rn } => write!(f, "TAS.B   @{}", rn),
            Instruction::LdcSsr { rm } => write!(f, "LDC     {},SSR", rm),
            Instruction::LdcSpc { rm } => write!(f, "LDC     {},SPC", rm),
            Instruction::LdcDbr { rm } => write!(f, "LDC     {},DBR", rm),
            Instruction::LdcBank { rm, bank } => write!(f, "LDC     {},{}", rm, bank),
            Instruction::LdcLSsr { rm } => write!(f, "LDC.L   @{}+,SSR", rm),
            Instruction::LdcLSpc { rm } => write!(f, "LDC.L   @{}+,SPC", rm),
            Instruction::LdcLDbr { rm } => write!(f, "LDC.L   @{}+,DBR", rm),
            Instruction::LdcLBank { rm, bank } => {
                write!(f, "LDC.L   @{}+,{}", rm, bank)
            }
            Instruction::StcSsr { rn } => write!(f, "STC     SSR,{}", rn),
            Instruction::StcSpc { rn } => write!(f, "STC     SPC,{}", rn),
            Instruction::StcSgr { rn } => write!(f, "STC     SGR,{}", rn),
            Instruction::StcDbr { rn } => write!(f, "STC     DBR,{}", rn),
            Instruction::StcBank { bank, rn } => write!(f, "STC     {},{}", bank, rn),
            Instruction::StcLSsr { rn } => write!(f, "STC.L   SSR,@-{}", rn),
            Instruction::StcLSpc { rn } => write!(f, "STC.L   SPC,@-{}", rn),
            Instruction::StcLSgr { rn } => write!(f, "STC.L   SGR,@-{}", rn),
            Instruction::StcLDbr { rn } => write!(f, "STC.L   DBR,@-{}", rn),
            Instruction::StcLBank { bank, rn } => {
                write!(f, "STC.L   {},@-{}", bank, rn)
            }
            Instruction::LdsFpul { rm } => write!(f, "LDS     {},FPUL", rm),
            Instruction::LdsFpscr { rm } => write!(f, "LDS     {},FPSCR", rm),
            Instruction::LdsLFpul { rm } => write!(f, "LDS.L   @{}+,FPUL", rm),
            Instruction::LdsLFpscr { rm } => write!(f, "LDS.L   @{}+,FPSCR", rm),
            Instruction::StsFpul { rn } => write!(f, "STS     FPUL,{}", rn),
            Instruction::StsFpscr { rn } => write!(f, "STS     FPSCR,{}", rn),
            Instruction::StsLFpul { rn } => write!(f, "STS.L   FPUL,@-{}", rn),
            Instruction::StsLFpscr { rn } => write!(f, "STS.L   FPSCR,@-{}", rn),
            Instruction::ClrS => write!(f, "CLRS"),
            Instruction::SetS => write!(f, "SETS"),
            Instruction::Ldtlb => write!(f, "LDTLB"),
            Instruction::Pref { rn } => write!(f, "PREF    @{}", rn),
            Instruction::Ocbi { rn } => write!(f, "OCBI    @{}", rn),
            Instruction::Ocbp { rn } => write!(f, "OCBP    @{}", rn),
            Instruction::Ocbwb { rn } => write!(f, "OCBWB   @{}", rn),
            Instruction::MovcaL { rn } => write!(f, "MOVCA.L R0,@{}", rn),
            Instruction::Fldi0 { frn } => write!(f, "FLDI0   {}", frn),
            Instruction::Fldi1 { frn } => write!(f, "FLDI1   {}", frn),
            Instruction::Fmov { frm, frn } => write!(f, "FMOV    {},{}", frm, frn),
            Instruction::FmovLoad { rm, frn } => write!(f, "FMOV.S  @{},{}", rm, frn),
            Instruction::FmovLoadIndexed { rm, frn } => {
                write!(f, "FMOV.S  @(R0,{}),{}", rm, frn)
            }
            Instruction::FmovLoadPostInc { rm, frn } => {
                write!(f, "FMOV.S  @{}+,{}", rm, frn)
            }
            Instruction::FmovStore { frm, rn } => write!(f, "FMOV.S  {},@{}", frm, rn),
            Instruction::FmovStoreIndexed { frm, rn } => {
                write!(f, "FMOV.S  {},@(R0,{})", frm, rn)
            }
            Instruction::FmovStorePreDec { frm, rn } => {
                write!(f, "FMOV.S  {},@-{}", frm, rn)
            }
            Instruction::Flds { frm } => write!(f, "FLDS    {},FPUL", frm),
            Instruction::Fsts { frn } => write!(f, "FSTS    FPUL,{}", frn),
            Instruction::Fabs { frn } => write!(f, "FABS    {}", frn),
            Instruction::Fadd { frm, frn } => write!(f, "FADD    {},{}", frm, frn),
            Instruction::FcmpEq { frm, frn } => write!(f, "FCMP/EQ {},{}", frm, frn),
            Instruction::FcmpGt { frm, frn } => write!(f, "FCMP/GT {},{}", frm, frn),
            Instruction::Fdiv { frm, frn } => write!(f, "FDIV    {},{}", frm, frn),
            Instruction::Float { frn } => write!(f, "FLOAT   FPUL,{}", frn),
            Instruction::Fmac { frm, frn } => write!(f, "FMAC    FR0,{},{}", frm, frn),
            Instruction::Fmul { frm, frn } => write!(f, "FMUL    {},{}", frm, frn),
            Instruction::Fneg { frn } => write!(f, "FNEG    {}", frn),
            Instruction::Fsqrt { frn } => write!(f, "FSQRT   {}", frn),
            Instruction::Fsub { frm, frn } => write!(f, "FSUB    {},{}", frm, frn),
            Instruction::Ftrc { frm } => write!(f, "FTRC    {},FPUL", frm),
            Instruction::FmovPair {
                source,
                destination,
            } => write!(f, "FMOV    {},{}", source, destination),
            Instruction::FmovPairLoad { rm, destination } => {
                write!(f, "FMOV    @{},{}", rm, destination)
            }
            Instruction::FmovPairLoadIndexed { rm, destination } => {
                write!(f, "FMOV    @(R0,{}),{}", rm, destination)
            }
            Instruction::FmovPairLoadPostInc { rm, destination } => {
                write!(f, "FMOV    @{}+,{}", rm, destination)
            }
            Instruction::FmovPairStore { source, rn } => {
                write!(f, "FMOV    {},@{}", source, rn)
            }
            Instruction::FmovPairStoreIndexed { source, rn } => {
                write!(f, "FMOV    {},@(R0,{})", source, rn)
            }
            Instruction::FmovPairStorePreDec { source, rn } => {
                write!(f, "FMOV    {},@-{}", source, rn)
            }
            Instruction::FabsDouble { drn } => write!(f, "FABS    {}", drn),
            Instruction::FaddDouble { drm, drn } => write!(f, "FADD    {},{}", drm, drn),
            Instruction::FcmpEqDouble { drm, drn } => {
                write!(f, "FCMP/EQ {},{}", drm, drn)
            }
            Instruction::FcmpGtDouble { drm, drn } => {
                write!(f, "FCMP/GT {},{}", drm, drn)
            }
            Instruction::FdivDouble { drm, drn } => write!(f, "FDIV    {},{}", drm, drn),
            Instruction::Fcnvds { drm } => write!(f, "FCNVDS  {},FPUL", drm),
            Instruction::Fcnvsd { drn } => write!(f, "FCNVSD  FPUL,{}", drn),
            Instruction::FloatDouble { drn } => write!(f, "FLOAT   FPUL,{}", drn),
            Instruction::FmulDouble { drm, drn } => write!(f, "FMUL    {},{}", drm, drn),
            Instruction::FnegDouble { drn } => write!(f, "FNEG    {}", drn),
            Instruction::FsqrtDouble { drn } => write!(f, "FSQRT   {}", drn),
            Instruction::FsubDouble { drm, drn } => write!(f, "FSUB    {},{}", drm, drn),
            Instruction::FtrcDouble { drm } => write!(f, "FTRC    {},FPUL", drm),
            Instruction::Fipr { fvm, fvn } => write!(f, "FIPR    {},{}", fvm, fvn),
            Instruction::Ftrv { fvn } => write!(f, "FTRV    XMTRX,{}", fvn),
            Instruction::Frchg => write!(f, "FRCHG"),
            Instruction::Fschg => write!(f, "FSCHG"),
        }
    }
}
