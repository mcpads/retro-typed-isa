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
        }
    }
}
