use std::collections::BTreeMap;

use thiserror::Error;

use crate::codec::{DecodeError, decode_word};
use crate::encoder::{EncodeError, encode_word};
use crate::instruction::{FpscrState, Instruction, Register};

#[derive(Debug, Clone, Default)]
pub struct Assembler {
    nodes: Vec<Node>,
}

impl Assembler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn label(&mut self, name: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::Label(name.into()));
        self
    }

    pub fn emit(&mut self, instruction: Instruction) -> &mut Self {
        self.nodes.push(Node::Instruction(instruction));
        self
    }

    pub fn emit_all(&mut self, instructions: impl IntoIterator<Item = Instruction>) -> &mut Self {
        self.nodes
            .extend(instructions.into_iter().map(Node::Instruction));
        self
    }

    /// Declare the FPSCR.PR/SZ interpretation for following instructions.
    ///
    /// This is a static assembly annotation and emits no instruction bytes.
    pub fn assume_fpscr(&mut self, state: FpscrState) -> &mut Self {
        self.nodes.push(Node::Fpscr(state));
        self
    }

    /// Emit a MOV.W reference to a typed 16-bit literal in the next pool.
    pub fn mov_w_literal(&mut self, value: u16, rn: Register) -> &mut Self {
        self.nodes.push(Node::WordLiteral { value, rn });
        self
    }

    /// Emit a MOV.L reference to a typed 32-bit literal in the next pool.
    pub fn mov_l_literal(&mut self, value: u32, rn: Register) -> &mut Self {
        self.nodes.push(Node::LongLiteral { value, rn });
        self
    }

    /// Place all pending typed literals at the current position.
    pub fn emit_literal_pool(&mut self) -> &mut Self {
        self.nodes.push(Node::LiteralPool);
        self
    }

    pub fn bt(&mut self, label: impl Into<String>) -> &mut Self {
        self.branch(LabelBranch::Bt, label)
    }

    pub fn bf(&mut self, label: impl Into<String>) -> &mut Self {
        self.branch(LabelBranch::Bf, label)
    }

    pub fn bts(&mut self, label: impl Into<String>) -> &mut Self {
        self.branch(LabelBranch::Bts, label)
    }

    pub fn bfs(&mut self, label: impl Into<String>) -> &mut Self {
        self.branch(LabelBranch::Bfs, label)
    }

    pub fn bra(&mut self, label: impl Into<String>) -> &mut Self {
        self.branch(LabelBranch::Bra, label)
    }

    pub fn bsr(&mut self, label: impl Into<String>) -> &mut Self {
        self.branch(LabelBranch::Bsr, label)
    }

    fn branch(&mut self, branch: LabelBranch, label: impl Into<String>) -> &mut Self {
        self.nodes.push(Node::Branch {
            branch,
            label: label.into(),
        });
        self
    }

    pub fn assemble(
        &self,
        origin: u32,
        initial_fpscr: FpscrState,
    ) -> Result<AssembledProgram, AssemblerError> {
        if origin & 1 != 0 {
            return Err(AssemblerError::UnalignedOrigin { origin });
        }
        let layout = self.layout(origin)?;
        let mut fpscr = initial_fpscr;
        let mut bytes = Vec::with_capacity(layout.total_len);
        let mut words = Vec::new();
        let mut instructions = Vec::new();
        let mut placed_instructions = Vec::new();
        let mut literals = Vec::new();

        for (node_index, node) in self.nodes.iter().enumerate() {
            match node {
                Node::Label(_) => {}
                Node::Fpscr(state) => fpscr = *state,
                Node::Instruction(instruction) => append_instruction(
                    *instruction,
                    fpscr,
                    origin,
                    layout.node_offsets[node_index].expect("instruction offset"),
                    &mut bytes,
                    &mut words,
                    &mut instructions,
                    &mut placed_instructions,
                )?,
                Node::Branch { branch, label } => {
                    let offset = layout.node_offsets[node_index].expect("branch offset");
                    let address = address_at(origin, offset)?;
                    let target = *layout.labels.get(label).ok_or_else(|| {
                        AssemblerError::UndefinedLabel {
                            label: label.clone(),
                        }
                    })?;
                    append_instruction(
                        branch.resolve(address, target)?,
                        fpscr,
                        origin,
                        offset,
                        &mut bytes,
                        &mut words,
                        &mut instructions,
                        &mut placed_instructions,
                    )?;
                }
                Node::WordLiteral { rn, .. } => {
                    let offset = layout.node_offsets[node_index].expect("literal load offset");
                    let instruction_address = address_at(origin, offset)?;
                    let literal_offset = layout.literal_offsets[&node_index];
                    let literal_address = address_at(origin, literal_offset)?;
                    let displacement = literal_displacement(
                        instruction_address,
                        literal_address,
                        2,
                        instruction_address
                            .checked_add(4)
                            .ok_or(AssemblerError::AddressOverflow { origin, offset })?,
                    )?;
                    append_instruction(
                        Instruction::MovWPcRel {
                            disp: displacement,
                            rn: *rn,
                        },
                        fpscr,
                        origin,
                        offset,
                        &mut bytes,
                        &mut words,
                        &mut instructions,
                        &mut placed_instructions,
                    )?;
                }
                Node::LongLiteral { rn, .. } => {
                    let offset = layout.node_offsets[node_index].expect("literal load offset");
                    let instruction_address = address_at(origin, offset)?;
                    let literal_offset = layout.literal_offsets[&node_index];
                    let literal_address = address_at(origin, literal_offset)?;
                    let base = (instruction_address & !3)
                        .checked_add(4)
                        .ok_or(AssemblerError::AddressOverflow { origin, offset })?;
                    let displacement =
                        literal_displacement(instruction_address, literal_address, 4, base)?;
                    append_instruction(
                        Instruction::MovLPcRel {
                            disp: displacement,
                            rn: *rn,
                        },
                        fpscr,
                        origin,
                        offset,
                        &mut bytes,
                        &mut words,
                        &mut instructions,
                        &mut placed_instructions,
                    )?;
                }
                Node::LiteralPool => append_pool(
                    origin,
                    &layout.pools[&node_index],
                    &mut bytes,
                    &mut literals,
                )?,
            }
        }
        append_pool(
            origin,
            &layout.pools[&self.nodes.len()],
            &mut bytes,
            &mut literals,
        )?;
        debug_assert_eq!(bytes.len(), layout.total_len);

        for placed in &placed_instructions {
            if !placed.instruction.has_delay_slot() {
                continue;
            }
            let slot_address =
                placed
                    .address
                    .checked_add(2)
                    .ok_or(AssemblerError::AddressOverflow {
                        origin,
                        offset: usize::MAX,
                    })?;
            let Some(delay_slot) = placed_instructions
                .iter()
                .find(|candidate| candidate.address == slot_address)
            else {
                return Err(AssemblerError::MissingDelaySlot {
                    branch_address: placed.address,
                });
            };
            if delay_slot.instruction.is_illegal_delay_slot() {
                return Err(AssemblerError::IllegalDelaySlot {
                    branch_address: placed.address,
                    slot_address,
                    instruction: delay_slot.instruction,
                });
            }
        }

        Ok(AssembledProgram {
            origin,
            bytes,
            words,
            instructions,
            placed_instructions,
            literals,
        })
    }

    fn layout(&self, origin: u32) -> Result<Layout, AssemblerError> {
        let mut labels = BTreeMap::new();
        let mut node_offsets = vec![None; self.nodes.len()];
        let mut literal_offsets = BTreeMap::new();
        let mut pools = BTreeMap::new();
        let mut pending_words = Vec::new();
        let mut pending_longs = Vec::new();
        let mut offset = 0usize;

        for (node_index, node) in self.nodes.iter().enumerate() {
            match node {
                Node::Label(label) => {
                    let address = address_at(origin, offset)?;
                    if labels.insert(label.clone(), address).is_some() {
                        return Err(AssemblerError::DuplicateLabel {
                            label: label.clone(),
                        });
                    }
                }
                Node::Fpscr(_) => {}
                Node::Instruction(_) | Node::Branch { .. } => {
                    node_offsets[node_index] = Some(offset);
                    offset = add_offset(origin, offset, 2)?;
                }
                Node::WordLiteral { value, .. } => {
                    node_offsets[node_index] = Some(offset);
                    pending_words.push((node_index, *value));
                    offset = add_offset(origin, offset, 2)?;
                }
                Node::LongLiteral { value, .. } => {
                    node_offsets[node_index] = Some(offset);
                    pending_longs.push((node_index, *value));
                    offset = add_offset(origin, offset, 2)?;
                }
                Node::LiteralPool => {
                    let (next, pool) = plan_pool(
                        origin,
                        offset,
                        &mut pending_longs,
                        &mut pending_words,
                        &mut literal_offsets,
                    )?;
                    offset = next;
                    pools.insert(node_index, pool);
                }
            }
        }
        let (total_len, final_pool) = plan_pool(
            origin,
            offset,
            &mut pending_longs,
            &mut pending_words,
            &mut literal_offsets,
        )?;
        pools.insert(self.nodes.len(), final_pool);
        address_at(origin, total_len)?;

        Ok(Layout {
            labels,
            node_offsets,
            literal_offsets,
            pools,
            total_len,
        })
    }
}

#[derive(Debug, Clone)]
enum Node {
    Label(String),
    Fpscr(FpscrState),
    Instruction(Instruction),
    Branch { branch: LabelBranch, label: String },
    WordLiteral { value: u16, rn: Register },
    LongLiteral { value: u32, rn: Register },
    LiteralPool,
}

#[derive(Debug)]
struct Layout {
    labels: BTreeMap<String, u32>,
    node_offsets: Vec<Option<usize>>,
    literal_offsets: BTreeMap<usize, usize>,
    pools: BTreeMap<usize, PoolPlan>,
    total_len: usize,
}

#[derive(Debug, Default)]
struct PoolPlan {
    start: usize,
    end: usize,
    longs: Vec<(usize, u32)>,
    words: Vec<(usize, u16)>,
}

fn plan_pool(
    origin: u32,
    start: usize,
    pending_longs: &mut Vec<(usize, u32)>,
    pending_words: &mut Vec<(usize, u16)>,
    literal_offsets: &mut BTreeMap<usize, usize>,
) -> Result<(usize, PoolPlan), AssemblerError> {
    let mut offset = start;
    if !pending_longs.is_empty() {
        while address_at(origin, offset)? & 3 != 0 {
            offset = add_offset(origin, offset, 1)?;
        }
    }
    let mut plan = PoolPlan {
        start,
        end: start,
        longs: Vec::new(),
        words: Vec::new(),
    };
    for (node_index, value) in pending_longs.drain(..) {
        literal_offsets.insert(node_index, offset);
        plan.longs.push((offset, value));
        offset = add_offset(origin, offset, 4)?;
    }
    for (node_index, value) in pending_words.drain(..) {
        literal_offsets.insert(node_index, offset);
        plan.words.push((offset, value));
        offset = add_offset(origin, offset, 2)?;
    }
    plan.end = offset;
    Ok((offset, plan))
}

fn append_pool(
    origin: u32,
    plan: &PoolPlan,
    bytes: &mut Vec<u8>,
    literals: &mut Vec<PlacedLiteral>,
) -> Result<(), AssemblerError> {
    debug_assert_eq!(bytes.len(), plan.start);
    let first_data_offset = plan
        .longs
        .first()
        .map(|(offset, _)| *offset)
        .or_else(|| plan.words.first().map(|(offset, _)| *offset))
        .unwrap_or(plan.end);
    bytes.resize(first_data_offset, 0);
    for (offset, value) in &plan.longs {
        debug_assert_eq!(bytes.len(), *offset);
        bytes.extend_from_slice(&value.to_le_bytes());
        literals.push(PlacedLiteral {
            address: address_at(origin, *offset)?,
            value: LiteralValue::Long(*value),
        });
    }
    for (offset, value) in &plan.words {
        debug_assert_eq!(bytes.len(), *offset);
        bytes.extend_from_slice(&value.to_le_bytes());
        literals.push(PlacedLiteral {
            address: address_at(origin, *offset)?,
            value: LiteralValue::Word(*value),
        });
    }
    debug_assert_eq!(bytes.len(), plan.end);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn append_instruction(
    instruction: Instruction,
    fpscr: FpscrState,
    origin: u32,
    offset: usize,
    bytes: &mut Vec<u8>,
    words: &mut Vec<u16>,
    instructions: &mut Vec<Instruction>,
    placed_instructions: &mut Vec<PlacedInstruction>,
) -> Result<(), AssemblerError> {
    debug_assert_eq!(bytes.len(), offset);
    let address = address_at(origin, offset)?;
    let word = encode_word(&instruction).map_err(AssemblerError::Encode)?;
    let decoded = decode_word(word, fpscr).map_err(AssemblerError::Decode)?;
    if instruction != decoded {
        return Err(AssemblerError::SemanticRoundTripMismatch {
            address,
            expected: instruction,
            actual: decoded,
        });
    }
    words.push(word);
    bytes.extend_from_slice(&word.to_le_bytes());
    instructions.push(instruction);
    placed_instructions.push(PlacedInstruction {
        address,
        fpscr,
        instruction,
    });
    Ok(())
}

fn literal_displacement(
    instruction_address: u32,
    literal_address: u32,
    scale: u32,
    base: u32,
) -> Result<u8, AssemblerError> {
    let Some(distance) = literal_address.checked_sub(base) else {
        return Err(AssemblerError::LiteralBeforeInstruction {
            instruction_address,
            literal_address,
        });
    };
    if distance % scale != 0 {
        return Err(AssemblerError::UnalignedLiteral {
            instruction_address,
            literal_address,
            alignment: scale,
        });
    }
    u8::try_from(distance / scale).map_err(|_| AssemblerError::LiteralOutOfRange {
        instruction_address,
        literal_address,
    })
}

fn add_offset(origin: u32, offset: usize, amount: usize) -> Result<usize, AssemblerError> {
    let next = offset
        .checked_add(amount)
        .ok_or(AssemblerError::AddressOverflow { origin, offset })?;
    address_at(origin, next)?;
    Ok(next)
}

fn address_at(origin: u32, offset: usize) -> Result<u32, AssemblerError> {
    let offset_u32 =
        u32::try_from(offset).map_err(|_| AssemblerError::AddressOverflow { origin, offset })?;
    origin
        .checked_add(offset_u32)
        .ok_or(AssemblerError::AddressOverflow { origin, offset })
}

#[derive(Debug, Clone, Copy)]
enum LabelBranch {
    Bt,
    Bf,
    Bts,
    Bfs,
    Bra,
    Bsr,
}

impl LabelBranch {
    fn resolve(self, address: u32, target: u32) -> Result<Instruction, AssemblerError> {
        if target & 1 != 0 {
            return Err(AssemblerError::UnalignedBranchTarget { address, target });
        }
        let base = address
            .checked_add(4)
            .ok_or(AssemblerError::AddressOverflow {
                origin: address,
                offset: 4,
            })?;
        let byte_delta = i64::from(target) - i64::from(base);
        debug_assert_eq!(byte_delta & 1, 0);
        let displacement = byte_delta / 2;
        match self {
            Self::Bt | Self::Bf | Self::Bts | Self::Bfs => {
                let disp =
                    i8::try_from(displacement).map_err(|_| AssemblerError::BranchOutOfRange {
                        address,
                        target,
                        bits: 8,
                    })?;
                Ok(match self {
                    Self::Bt => Instruction::Bt { disp },
                    Self::Bf => Instruction::Bf { disp },
                    Self::Bts => Instruction::Bts { disp },
                    Self::Bfs => Instruction::Bfs { disp },
                    _ => unreachable!(),
                })
            }
            Self::Bra | Self::Bsr => {
                if !(-2048..=2047).contains(&displacement) {
                    return Err(AssemblerError::BranchOutOfRange {
                        address,
                        target,
                        bits: 12,
                    });
                }
                let disp = displacement as i16;
                Ok(match self {
                    Self::Bra => Instruction::Bra { disp },
                    Self::Bsr => Instruction::Bsr { disp },
                    _ => unreachable!(),
                })
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacedInstruction {
    address: u32,
    fpscr: FpscrState,
    instruction: Instruction,
}

impl PlacedInstruction {
    pub fn address(&self) -> u32 {
        self.address
    }

    pub fn fpscr(&self) -> FpscrState {
        self.fpscr
    }

    pub fn instruction(&self) -> Instruction {
        self.instruction
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiteralValue {
    Word(u16),
    Long(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacedLiteral {
    address: u32,
    value: LiteralValue,
}

impl PlacedLiteral {
    pub fn address(&self) -> u32 {
        self.address
    }

    pub fn value(&self) -> LiteralValue {
        self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssembledProgram {
    origin: u32,
    bytes: Vec<u8>,
    words: Vec<u16>,
    instructions: Vec<Instruction>,
    placed_instructions: Vec<PlacedInstruction>,
    literals: Vec<PlacedLiteral>,
}

impl AssembledProgram {
    pub fn origin(&self) -> u32 {
        self.origin
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Encoded instruction words only; literal-pool data is exposed separately.
    pub fn words(&self) -> &[u16] {
        &self.words
    }

    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }

    pub fn placed_instructions(&self) -> &[PlacedInstruction] {
        &self.placed_instructions
    }

    pub fn literals(&self) -> &[PlacedLiteral] {
        &self.literals
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AssemblerError {
    #[error("SH-4 assembly origin 0x{origin:08X} is not two-byte aligned")]
    UnalignedOrigin { origin: u32 },
    #[error("duplicate label `{label}`")]
    DuplicateLabel { label: String },
    #[error("undefined label `{label}`")]
    UndefinedLabel { label: String },
    #[error("SH-4 address overflow from 0x{origin:08X} at byte offset {offset}")]
    AddressOverflow { origin: u32, offset: usize },
    #[error("branch at 0x{address:08X} has unaligned target 0x{target:08X}")]
    UnalignedBranchTarget { address: u32, target: u32 },
    #[error("{bits}-bit branch at 0x{address:08X} cannot reach 0x{target:08X}")]
    BranchOutOfRange { address: u32, target: u32, bits: u8 },
    #[error("delayed branch at 0x{branch_address:08X} has no delay-slot instruction")]
    MissingDelaySlot { branch_address: u32 },
    #[error(
        "instruction {instruction} at 0x{slot_address:08X} is illegal in the delay slot of 0x{branch_address:08X}"
    )]
    IllegalDelaySlot {
        branch_address: u32,
        slot_address: u32,
        instruction: Instruction,
    },
    #[error("literal at 0x{literal_address:08X} is before its load at 0x{instruction_address:08X}")]
    LiteralBeforeInstruction {
        instruction_address: u32,
        literal_address: u32,
    },
    #[error(
        "literal at 0x{literal_address:08X} for load at 0x{instruction_address:08X} is not {alignment}-byte aligned"
    )]
    UnalignedLiteral {
        instruction_address: u32,
        literal_address: u32,
        alignment: u32,
    },
    #[error(
        "literal at 0x{literal_address:08X} is out of range for load at 0x{instruction_address:08X}"
    )]
    LiteralOutOfRange {
        instruction_address: u32,
        literal_address: u32,
    },
    #[error(transparent)]
    Encode(EncodeError),
    #[error(transparent)]
    Decode(DecodeError),
    #[error(
        "SH-4 semantic round-trip mismatch at 0x{address:08X}: expected {expected}, got {actual}"
    )]
    SemanticRoundTripMismatch {
        address: u32,
        expected: Instruction,
        actual: Instruction,
    },
}
