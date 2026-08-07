use std::{collections::HashSet, sync::OnceLock};

use crate::{decode_bytes, instruction::*};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A prefix whose meaning is part of an instruction form.
pub enum SemanticPrefix {
    /// The segment form.
    Segment(Sreg),
    /// The rep form.
    Rep,
    /// The repne form.
    Repne,
    /// The repc form.
    Repc,
    /// The repnc form.
    Repnc,
    /// The bus lock form.
    BusLock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// The semantic V30 instruction family used by static analysis.
pub enum Mnemonic {
    /// The mov form.
    Mov,
    /// The lea form.
    Lea,
    /// The les form.
    Les,
    /// The lds form.
    Lds,
    /// The xchg form.
    Xchg,
    /// The push form.
    Push,
    /// The pop form.
    Pop,
    /// The pusha form.
    Pusha,
    /// The popa form.
    Popa,
    /// The bound form.
    Bound,
    /// The add form.
    Add,
    /// The adc form.
    Adc,
    /// The sub form.
    Sub,
    /// The sbb form.
    Sbb,
    /// The cmp form.
    Cmp,
    /// The test form.
    Test,
    /// The and form.
    And,
    /// The or form.
    Or,
    /// The xor form.
    Xor,
    /// The inc form.
    Inc,
    /// The dec form.
    Dec,
    /// The neg form.
    Neg,
    /// The not form.
    Not,
    /// The mul form.
    Mul,
    /// The imul form.
    Imul,
    /// The div form.
    Div,
    /// The idiv form.
    Idiv,
    /// The shl form.
    Shl,
    /// The shr form.
    Shr,
    /// The sar form.
    Sar,
    /// The rol form.
    Rol,
    /// The ror form.
    Ror,
    /// The rcl form.
    Rcl,
    /// The rcr form.
    Rcr,
    /// The jcc form.
    Jcc(Cond),
    /// The loop form.
    Loop(LoopCondition),
    /// The jmp form.
    Jmp,
    /// The call form.
    Call,
    /// The ret form.
    Ret,
    /// The retf form.
    Retf,
    /// The int form.
    Int,
    /// The into form.
    Into,
    /// The iret form.
    Iret,
    /// The in form.
    In,
    /// The out form.
    Out,
    /// The insb form.
    Insb,
    /// The insw form.
    Insw,
    /// The outsb form.
    Outsb,
    /// The outsw form.
    Outsw,
    /// The movsb form.
    Movsb,
    /// The movsw form.
    Movsw,
    /// The lodsb form.
    Lodsb,
    /// The lodsw form.
    Lodsw,
    /// The stosb form.
    Stosb,
    /// The stosw form.
    Stosw,
    /// The cmpsb form.
    Cmpsb,
    /// The cmpsw form.
    Cmpsw,
    /// The scasb form.
    Scasb,
    /// The scasw form.
    Scasw,
    /// The nop form.
    Nop,
    /// The hlt form.
    Hlt,
    /// The xlat form.
    Xlat,
    /// The wait form.
    Wait,
    /// The pushf form.
    Pushf,
    /// The popf form.
    Popf,
    /// The sahf form.
    Sahf,
    /// The lahf form.
    Lahf,
    /// The cmc form.
    Cmc,
    /// The clc form.
    Clc,
    /// The stc form.
    Stc,
    /// The cli form.
    Cli,
    /// The sti form.
    Sti,
    /// The cld form.
    Cld,
    /// The std form.
    Std,
    /// The cbw form.
    Cbw,
    /// The cwd form.
    Cwd,
    /// The aaa form.
    Aaa,
    /// The aas form.
    Aas,
    /// The daa form.
    Daa,
    /// The das form.
    Das,
    /// The aam form.
    Aam,
    /// The aad form.
    Aad,
    /// The enter form.
    Enter,
    /// The leave form.
    Leave,
    /// The esc form.
    Esc(EscapeOpcode),
    /// The fpo2 form.
    Fpo2(Fpo2Opcode),
    /// The add4s form.
    Add4s,
    /// The sub4s form.
    Sub4s,
    /// The cmp4s form.
    Cmp4s,
    /// The rol4 form.
    Rol4,
    /// The ror4 form.
    Ror4,
    /// The ins form.
    Ins,
    /// The ext form.
    Ext,
    /// The test1 form.
    Test1,
    /// The clr1 form.
    Clr1,
    /// The set1 form.
    Set1,
    /// The not1 form.
    Not1,
    /// The brkem form.
    Brkem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// The static shape of one instruction operand.
pub enum OperandForm {
    /// The register8 form.
    Register8,
    /// The register16 form.
    Register16,
    /// The segment register form.
    SegmentRegister,
    /// The accumulator8 form.
    Accumulator8,
    /// The accumulator16 form.
    Accumulator16,
    /// The immediate8 form.
    Immediate8,
    /// The immediate16 form.
    Immediate16,
    /// The signed immediate8 form.
    SignedImmediate8,
    /// The signed immediate16 form.
    SignedImmediate16,
    /// The memory form.
    Memory {
        /// The operand width.
        size: OpSize,
        /// The optional segment override.
        segment: Option<Sreg>,
        /// The effective-address base.
        base: MemBase,
    },
    /// The relative8 form.
    Relative8,
    /// The relative16 form.
    Relative16,
    /// The far pointer form.
    FarPointer,
    /// The far memory pointer form.
    FarMemoryPointer {
        /// The optional segment override.
        segment: Option<Sreg>,
        /// The effective-address base.
        base: MemBase,
    },
    /// The port immediate8 form.
    PortImmediate8,
    /// The port dx form.
    PortDx,
    /// The shift one form.
    ShiftOne,
    /// The shift cl form.
    ShiftCl,
    /// The shift immediate8 form.
    ShiftImmediate8,
    /// The bit index cl form.
    BitIndexCl,
    /// The bit index immediate8 form.
    BitIndexImmediate8,
    /// The bit index immediate16 form.
    BitIndexImmediate16,
    /// The bit field length immediate4 form.
    BitFieldLengthImmediate4,
    /// The external register form.
    ExternalRegister,
    /// The external memory form.
    ExternalMemory {
        /// The optional segment override.
        segment: Option<Sreg>,
        /// The effective-address base.
        base: MemBase,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// An operand-independent native V30 instruction form.
pub struct InstructionForm {
    prefixes: Vec<SemanticPrefix>,
    mnemonic: Mnemonic,
    operands: Vec<OperandForm>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A canonical typed representative and byte encoding for one instruction form.
pub struct CanonicalInstruction {
    form: InstructionForm,
    instruction: InstructionKind,
    bytes: Vec<u8>,
}

impl CanonicalInstruction {
    /// Returns the canonical instruction form.
    pub fn form(&self) -> &InstructionForm {
        &self.form
    }

    /// Returns the canonical typed representative.
    pub fn instruction(&self) -> &InstructionKind {
        &self.instruction
    }

    /// Returns the canonical representative bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl InstructionForm {
    /// Returns the complete supported instruction-form census.
    pub fn all() -> &'static [Self] {
        &form_catalog().forms
    }

    /// Returns one canonical typed and byte representative per supported form.
    pub fn canonical_instructions() -> &'static [CanonicalInstruction] {
        &form_catalog().canonical_instructions
    }

    /// Classifies a typed instruction into its static form.
    pub fn of(instruction: &InstructionKind) -> Option<Self> {
        crate::encode_bytes(instruction).ok()?;
        form_with_prefixes(instruction, Vec::new())
    }

    /// Returns the semantic prefixes.
    pub fn prefixes(&self) -> &[SemanticPrefix] {
        &self.prefixes
    }

    /// Returns the semantic instruction family.
    pub fn mnemonic(&self) -> Mnemonic {
        self.mnemonic
    }

    /// Returns the static operand forms.
    pub fn operands(&self) -> &[OperandForm] {
        &self.operands
    }
}

struct FormCatalog {
    forms: Vec<InstructionForm>,
    canonical_instructions: Vec<CanonicalInstruction>,
}

fn form_catalog() -> &'static FormCatalog {
    static CATALOG: OnceLock<FormCatalog> = OnceLock::new();
    CATALOG.get_or_init(enumerate_canonical_forms)
}

fn enumerate_canonical_forms() -> FormCatalog {
    let mut catalog = FormCatalog {
        forms: Vec::new(),
        canonical_instructions: Vec::new(),
    };
    let mut known_forms = HashSet::new();
    let mut bytes = [0_u8; 15];

    enumerate_selector_space(&mut catalog, &mut known_forms, &mut bytes, &[]);
    enumerate_selector_space(&mut catalog, &mut known_forms, &mut bytes, &[0xf0]);
    for segment in [0x26, 0x2e, 0x36, 0x3e] {
        enumerate_selector_space(&mut catalog, &mut known_forms, &mut bytes, &[segment]);
        enumerate_selector_space(&mut catalog, &mut known_forms, &mut bytes, &[0xf0, segment]);
    }

    catalog
}

fn enumerate_selector_space(
    catalog: &mut FormCatalog,
    known_forms: &mut HashSet<InstructionForm>,
    bytes: &mut [u8; 15],
    prefixes: &[u8],
) {
    for opcode in u8::MIN..=u8::MAX {
        for selector in u8::MIN..=u8::MAX {
            bytes.fill(0);
            bytes[..prefixes.len()].copy_from_slice(prefixes);
            bytes[prefixes.len()] = opcode;
            bytes[prefixes.len() + 1] = selector;
            insert_decoded_form(catalog, known_forms, bytes);
        }
    }

    for secondary in u8::MIN..=u8::MAX {
        for modrm in u8::MIN..=u8::MAX {
            bytes.fill(0);
            bytes[..prefixes.len()].copy_from_slice(prefixes);
            bytes[prefixes.len()] = 0x0f;
            bytes[prefixes.len() + 1] = secondary;
            bytes[prefixes.len() + 2] = modrm;
            insert_decoded_form(catalog, known_forms, bytes);
        }
    }
}

fn insert_decoded_form(
    catalog: &mut FormCatalog,
    known_forms: &mut HashSet<InstructionForm>,
    bytes: &[u8],
) {
    let Ok(decoded) = decode_bytes(bytes) else {
        return;
    };
    let Some(form) = InstructionForm::of(&decoded.instruction) else {
        return;
    };
    if known_forms.insert(form.clone()) {
        let canonical_bytes = crate::encode_bytes(&decoded.instruction)
            .expect("decoded instruction already passed the public encoding boundary");
        catalog.forms.push(form.clone());
        catalog.canonical_instructions.push(CanonicalInstruction {
            form,
            instruction: decoded.instruction,
            bytes: canonical_bytes,
        });
    }
}

fn form_with_prefixes(
    instruction: &InstructionKind,
    mut prefixes: Vec<SemanticPrefix>,
) -> Option<InstructionForm> {
    match instruction {
        InstructionKind::Rep(inner) => {
            prefixes.push(SemanticPrefix::Rep);
            return form_with_prefixes(inner, prefixes);
        }
        InstructionKind::Repne(inner) => {
            prefixes.push(SemanticPrefix::Repne);
            return form_with_prefixes(inner, prefixes);
        }
        InstructionKind::Repc(inner) => {
            prefixes.push(SemanticPrefix::Repc);
            return form_with_prefixes(inner, prefixes);
        }
        InstructionKind::Repnc(inner) => {
            prefixes.push(SemanticPrefix::Repnc);
            return form_with_prefixes(inner, prefixes);
        }
        InstructionKind::BusLock(inner) => {
            prefixes.push(SemanticPrefix::BusLock);
            return form_with_prefixes(inner, prefixes);
        }
        InstructionKind::SegmentOverride {
            segment,
            instruction,
        } => {
            prefixes.push(SemanticPrefix::Segment(*segment));
            return form_with_prefixes(instruction, prefixes);
        }
        _ => {}
    }

    let (mnemonic, operands) = match instruction {
        InstructionKind::Mov { dest, src } => {
            (Mnemonic::Mov, vec![operand_form(dest), operand_form(src)])
        }
        InstructionKind::Lea { src, .. } => (
            Mnemonic::Lea,
            vec![OperandForm::Register16, memory_form(src)],
        ),
        InstructionKind::Les { src, .. } => (
            Mnemonic::Les,
            vec![OperandForm::Register16, far_memory_form(src)],
        ),
        InstructionKind::Lds { src, .. } => (
            Mnemonic::Lds,
            vec![OperandForm::Register16, far_memory_form(src)],
        ),
        InstructionKind::Xchg { a, b } => (Mnemonic::Xchg, vec![operand_form(a), operand_form(b)]),
        InstructionKind::Push { src } => (Mnemonic::Push, vec![operand_form(src)]),
        InstructionKind::Pop { dest } => (Mnemonic::Pop, vec![operand_form(dest)]),
        InstructionKind::Pusha => (Mnemonic::Pusha, vec![]),
        InstructionKind::Popa => (Mnemonic::Popa, vec![]),
        InstructionKind::Bound { bounds, .. } => (
            Mnemonic::Bound,
            vec![OperandForm::Register16, memory_form(bounds)],
        ),
        InstructionKind::Add { dest, src } => binary(Mnemonic::Add, dest, src),
        InstructionKind::Adc { dest, src } => binary(Mnemonic::Adc, dest, src),
        InstructionKind::Sub { dest, src } => binary(Mnemonic::Sub, dest, src),
        InstructionKind::Sbb { dest, src } => binary(Mnemonic::Sbb, dest, src),
        InstructionKind::Cmp { a, b } => binary(Mnemonic::Cmp, a, b),
        InstructionKind::Test { a, b } => binary(Mnemonic::Test, a, b),
        InstructionKind::And { dest, src } => binary(Mnemonic::And, dest, src),
        InstructionKind::Or { dest, src } => binary(Mnemonic::Or, dest, src),
        InstructionKind::Xor { dest, src } => binary(Mnemonic::Xor, dest, src),
        InstructionKind::Inc { dest } => (Mnemonic::Inc, vec![operand_form(dest)]),
        InstructionKind::Dec { dest } => (Mnemonic::Dec, vec![operand_form(dest)]),
        InstructionKind::Neg { dest } => (Mnemonic::Neg, vec![operand_form(dest)]),
        InstructionKind::Not { dest } => (Mnemonic::Not, vec![operand_form(dest)]),
        InstructionKind::Mul { src } => (Mnemonic::Mul, vec![operand_form(src)]),
        InstructionKind::Imul { src } => (Mnemonic::Imul, vec![operand_form(src)]),
        InstructionKind::ImulImmediate { src, immediate, .. } => (
            Mnemonic::Imul,
            vec![
                OperandForm::Register16,
                operand_form(src),
                signed_immediate_form(*immediate),
            ],
        ),
        InstructionKind::Div { src } => (Mnemonic::Div, vec![operand_form(src)]),
        InstructionKind::Idiv { src } => (Mnemonic::Idiv, vec![operand_form(src)]),
        InstructionKind::Shl { dest, count } => shift(Mnemonic::Shl, dest, *count),
        InstructionKind::Shr { dest, count } => shift(Mnemonic::Shr, dest, *count),
        InstructionKind::Sar { dest, count } => shift(Mnemonic::Sar, dest, *count),
        InstructionKind::Rol { dest, count } => shift(Mnemonic::Rol, dest, *count),
        InstructionKind::Ror { dest, count } => shift(Mnemonic::Ror, dest, *count),
        InstructionKind::Rcl { dest, count } => shift(Mnemonic::Rcl, dest, *count),
        InstructionKind::Rcr { dest, count } => shift(Mnemonic::Rcr, dest, *count),
        InstructionKind::Jcc { cond, .. } => (Mnemonic::Jcc(*cond), vec![OperandForm::Relative8]),
        InstructionKind::Loop { condition, .. } => {
            (Mnemonic::Loop(*condition), vec![OperandForm::Relative8])
        }
        InstructionKind::Jmp { target } => (Mnemonic::Jmp, vec![jump_target_form(target)]),
        InstructionKind::Call { target } => (Mnemonic::Call, vec![call_target_form(target)]),
        InstructionKind::Ret { pop } => (
            Mnemonic::Ret,
            if *pop == 0 {
                vec![]
            } else {
                vec![OperandForm::Immediate16]
            },
        ),
        InstructionKind::Retf { pop } => (
            Mnemonic::Retf,
            if *pop == 0 {
                vec![]
            } else {
                vec![OperandForm::Immediate16]
            },
        ),
        InstructionKind::Int { .. } => (Mnemonic::Int, vec![OperandForm::Immediate8]),
        InstructionKind::Into => (Mnemonic::Into, vec![]),
        InstructionKind::Iret => (Mnemonic::Iret, vec![]),
        InstructionKind::InAl { port } => (
            Mnemonic::In,
            vec![OperandForm::Accumulator8, port_form(*port)],
        ),
        InstructionKind::InAx { port } => (
            Mnemonic::In,
            vec![OperandForm::Accumulator16, port_form(*port)],
        ),
        InstructionKind::OutAl { port } => (
            Mnemonic::Out,
            vec![port_form(*port), OperandForm::Accumulator8],
        ),
        InstructionKind::OutAx { port } => (
            Mnemonic::Out,
            vec![port_form(*port), OperandForm::Accumulator16],
        ),
        InstructionKind::Insb => (Mnemonic::Insb, vec![]),
        InstructionKind::Insw => (Mnemonic::Insw, vec![]),
        InstructionKind::Outsb => (Mnemonic::Outsb, vec![]),
        InstructionKind::Outsw => (Mnemonic::Outsw, vec![]),
        InstructionKind::Movsb => (Mnemonic::Movsb, vec![]),
        InstructionKind::Movsw => (Mnemonic::Movsw, vec![]),
        InstructionKind::Lodsb => (Mnemonic::Lodsb, vec![]),
        InstructionKind::Lodsw => (Mnemonic::Lodsw, vec![]),
        InstructionKind::Stosb => (Mnemonic::Stosb, vec![]),
        InstructionKind::Stosw => (Mnemonic::Stosw, vec![]),
        InstructionKind::Cmpsb => (Mnemonic::Cmpsb, vec![]),
        InstructionKind::Cmpsw => (Mnemonic::Cmpsw, vec![]),
        InstructionKind::Scasb => (Mnemonic::Scasb, vec![]),
        InstructionKind::Scasw => (Mnemonic::Scasw, vec![]),
        InstructionKind::Nop => (Mnemonic::Nop, vec![]),
        InstructionKind::Hlt => (Mnemonic::Hlt, vec![]),
        InstructionKind::Xlat => (Mnemonic::Xlat, vec![]),
        InstructionKind::Wait => (Mnemonic::Wait, vec![]),
        InstructionKind::Pushf => (Mnemonic::Pushf, vec![]),
        InstructionKind::Popf => (Mnemonic::Popf, vec![]),
        InstructionKind::Sahf => (Mnemonic::Sahf, vec![]),
        InstructionKind::Lahf => (Mnemonic::Lahf, vec![]),
        InstructionKind::Cmc => (Mnemonic::Cmc, vec![]),
        InstructionKind::Clc => (Mnemonic::Clc, vec![]),
        InstructionKind::Stc => (Mnemonic::Stc, vec![]),
        InstructionKind::Cli => (Mnemonic::Cli, vec![]),
        InstructionKind::Sti => (Mnemonic::Sti, vec![]),
        InstructionKind::Cld => (Mnemonic::Cld, vec![]),
        InstructionKind::Std => (Mnemonic::Std, vec![]),
        InstructionKind::Cbw => (Mnemonic::Cbw, vec![]),
        InstructionKind::Cwd => (Mnemonic::Cwd, vec![]),
        InstructionKind::Aaa => (Mnemonic::Aaa, vec![]),
        InstructionKind::Aas => (Mnemonic::Aas, vec![]),
        InstructionKind::Daa => (Mnemonic::Daa, vec![]),
        InstructionKind::Das => (Mnemonic::Das, vec![]),
        InstructionKind::Aam { .. } => (Mnemonic::Aam, vec![OperandForm::Immediate8]),
        InstructionKind::Aad { .. } => (Mnemonic::Aad, vec![OperandForm::Immediate8]),
        InstructionKind::Enter { .. } => (
            Mnemonic::Enter,
            vec![OperandForm::Immediate16, OperandForm::Immediate8],
        ),
        InstructionKind::Leave => (Mnemonic::Leave, vec![]),
        InstructionKind::Esc { operation, operand } => (
            Mnemonic::Esc(*operation),
            vec![floating_operand_form(operand)],
        ),
        InstructionKind::Fpo2 { operation, operand } => (
            Mnemonic::Fpo2(*operation),
            vec![floating_operand_form(operand)],
        ),
        InstructionKind::Add4s => (Mnemonic::Add4s, vec![]),
        InstructionKind::Sub4s => (Mnemonic::Sub4s, vec![]),
        InstructionKind::Cmp4s => (Mnemonic::Cmp4s, vec![]),
        InstructionKind::Rol4 { rm } => (Mnemonic::Rol4, vec![operand_form(rm)]),
        InstructionKind::Ror4 { rm } => (Mnemonic::Ror4, vec![operand_form(rm)]),
        InstructionKind::Ins { .. } => (
            Mnemonic::Ins,
            vec![OperandForm::Register8, OperandForm::Register8],
        ),
        InstructionKind::Ext { .. } => (
            Mnemonic::Ext,
            vec![OperandForm::Register8, OperandForm::Register8],
        ),
        InstructionKind::InsImm { .. } => (
            Mnemonic::Ins,
            vec![
                OperandForm::Register8,
                OperandForm::BitFieldLengthImmediate4,
            ],
        ),
        InstructionKind::ExtImm { .. } => (
            Mnemonic::Ext,
            vec![
                OperandForm::Register8,
                OperandForm::BitFieldLengthImmediate4,
            ],
        ),
        InstructionKind::Test1 { target, bit } => bit_form(Mnemonic::Test1, target, *bit),
        InstructionKind::Clr1 { target, bit } => bit_form(Mnemonic::Clr1, target, *bit),
        InstructionKind::Set1 { target, bit } => bit_form(Mnemonic::Set1, target, *bit),
        InstructionKind::Not1 { target, bit } => bit_form(Mnemonic::Not1, target, *bit),
        InstructionKind::Brkem { .. } => (Mnemonic::Brkem, vec![OperandForm::Immediate8]),
        InstructionKind::Unknown { .. } => return None,
        InstructionKind::Rep(_)
        | InstructionKind::Repne(_)
        | InstructionKind::Repc(_)
        | InstructionKind::Repnc(_)
        | InstructionKind::BusLock(_)
        | InstructionKind::SegmentOverride { .. } => {
            unreachable!("prefix wrappers returned above")
        }
    };
    Some(InstructionForm {
        prefixes,
        mnemonic,
        operands,
    })
}

fn binary(mnemonic: Mnemonic, left: &Operand, right: &Operand) -> (Mnemonic, Vec<OperandForm>) {
    (mnemonic, vec![operand_form(left), operand_form(right)])
}

fn shift(mnemonic: Mnemonic, target: &Operand, count: ShiftCount) -> (Mnemonic, Vec<OperandForm>) {
    (
        mnemonic,
        vec![operand_form(target), shift_count_form(count)],
    )
}

fn bit_form(mnemonic: Mnemonic, target: &Operand, index: BitIndex) -> (Mnemonic, Vec<OperandForm>) {
    let index = match index {
        BitIndex::Cl => OperandForm::BitIndexCl,
        BitIndex::Immediate(_) => match operand_form(target) {
            OperandForm::Register8
            | OperandForm::Memory {
                size: OpSize::Byte, ..
            } => OperandForm::BitIndexImmediate8,
            _ => OperandForm::BitIndexImmediate16,
        },
    };
    (mnemonic, vec![operand_form(target), index])
}

fn operand_form(operand: &Operand) -> OperandForm {
    match operand {
        Operand::Reg8(_) => OperandForm::Register8,
        Operand::Reg16(_) => OperandForm::Register16,
        Operand::Sreg(_) => OperandForm::SegmentRegister,
        Operand::Imm8(_) => OperandForm::Immediate8,
        Operand::Imm16(_) => OperandForm::Immediate16,
        Operand::Mem(memory) => memory_form(memory),
        Operand::Rel8(_) => OperandForm::Relative8,
        Operand::Rel16(_) => OperandForm::Relative16,
        Operand::Far { .. } => OperandForm::FarPointer,
    }
}

fn signed_immediate_form(immediate: SignedImmediate) -> OperandForm {
    match immediate {
        SignedImmediate::Byte(_) => OperandForm::SignedImmediate8,
        SignedImmediate::Word(_) => OperandForm::SignedImmediate16,
    }
}

fn shift_count_form(count: ShiftCount) -> OperandForm {
    match count {
        ShiftCount::One => OperandForm::ShiftOne,
        ShiftCount::Cl => OperandForm::ShiftCl,
        ShiftCount::Imm8(_) => OperandForm::ShiftImmediate8,
    }
}

fn port_form(port: PortAddr) -> OperandForm {
    match port {
        PortAddr::Imm8(_) => OperandForm::PortImmediate8,
        PortAddr::Dx => OperandForm::PortDx,
    }
}

fn jump_target_form(target: &JmpTarget) -> OperandForm {
    match target {
        JmpTarget::Rel8(_) => OperandForm::Relative8,
        JmpTarget::Rel16(_) => OperandForm::Relative16,
        JmpTarget::Far { .. } => OperandForm::FarPointer,
        JmpTarget::IndirectNear(operand) => operand_form(operand),
        JmpTarget::IndirectFar(memory) => far_memory_form(memory),
    }
}

fn call_target_form(target: &CallTarget) -> OperandForm {
    match target {
        CallTarget::Rel16(_) => OperandForm::Relative16,
        CallTarget::Far { .. } => OperandForm::FarPointer,
        CallTarget::IndirectNear(operand) => operand_form(operand),
        CallTarget::IndirectFar(memory) => far_memory_form(memory),
    }
}

fn floating_operand_form(operand: &FloatingPointOperand) -> OperandForm {
    match operand {
        FloatingPointOperand::Register(_) => OperandForm::ExternalRegister,
        FloatingPointOperand::Memory(memory) => OperandForm::ExternalMemory {
            segment: memory.segment(),
            base: memory.base(),
        },
    }
}

fn memory_form(memory: &Mem) -> OperandForm {
    OperandForm::Memory {
        size: memory.size(),
        segment: memory.segment(),
        base: memory.base(),
    }
}

fn far_memory_form(memory: &Mem) -> OperandForm {
    OperandForm::FarMemoryPointer {
        segment: memory.segment(),
        base: memory.base(),
    }
}
