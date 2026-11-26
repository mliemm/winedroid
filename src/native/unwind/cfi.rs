use super::fde::{Fde, Memory};

pub const DWARF_REGISTER_COUNT: usize = 97;

const DW_CFA_NOP: u8 = 0x00;
const DW_CFA_SET_LOC: u8 = 0x01;
const DW_CFA_ADVANCE_LOC1: u8 = 0x02;
const DW_CFA_ADVANCE_LOC2: u8 = 0x03;
const DW_CFA_ADVANCE_LOC4: u8 = 0x04;
const DW_CFA_OFFSET_EXTENDED: u8 = 0x05;
const DW_CFA_RESTORE_EXTENDED: u8 = 0x06;
const DW_CFA_UNDEFINED: u8 = 0x07;
const DW_CFA_SAME_VALUE: u8 = 0x08;
const DW_CFA_REGISTER: u8 = 0x09;
const DW_CFA_REMEMBER_STATE: u8 = 0x0a;
const DW_CFA_RESTORE_STATE: u8 = 0x0b;
const DW_CFA_DEF_CFA: u8 = 0x0c;
const DW_CFA_DEF_CFA_REGISTER: u8 = 0x0d;
const DW_CFA_DEF_CFA_OFFSET: u8 = 0x0e;
const DW_CFA_DEF_CFA_EXPRESSION: u8 = 0x0f;
const DW_CFA_EXPRESSION: u8 = 0x10;
const DW_CFA_OFFSET_EXTENDED_SF: u8 = 0x11;
const DW_CFA_DEF_CFA_SF: u8 = 0x12;
const DW_CFA_DEF_CFA_OFFSET_SF: u8 = 0x13;
const DW_CFA_VAL_OFFSET: u8 = 0x14;
const DW_CFA_VAL_OFFSET_SF: u8 = 0x15;
const DW_CFA_VAL_EXPRESSION: u8 = 0x16;
const DW_CFA_GNU_ARGS_SIZE: u8 = 0x2e;
const DW_CFA_GNU_NEGATIVE_OFFSET_EXTENDED: u8 = 0x2f;
const DW_CFA_ADVANCE_LOC_BASE: u8 = 0x40;
const DW_CFA_OFFSET_BASE: u8 = 0x80;
const DW_CFA_RESTORE_BASE: u8 = 0xc0;
const OPCODE_NAME_MASK: u8 = 0xc0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Rule {
    Undefined,
    SameValue,
    Offset(i64),
    ValueOffset(i64),
    Register(u16),
    Expression(usize, usize),
    ValueExpression(usize, usize),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CfaRule {
    Register(u16, i64),
    Expression(usize, usize),
}

#[derive(Clone, Debug)]
pub struct RuleSet {
    pub cfa: Option<CfaRule>,
    pub registers: Vec<Rule>,
}

impl RuleSet {
    fn new() -> Self {
        Self {
            cfa: None,
            registers: vec![Rule::Undefined; DWARF_REGISTER_COUNT],
        }
    }

    pub fn rule(&self, register: u32) -> Rule {
        self.registers
            .get(register as usize)
            .copied()
            .unwrap_or(Rule::Undefined)
    }

    fn set(&mut self, register: u32, rule: Rule) -> Result<(), String> {
        let Some(slot) = self.registers.get_mut(register as usize) else {
            return Err(format!("unwind CFI names DWARF register {register}; next missing primitive: DWARF register {register}"));
        };
        *slot = rule;
        Ok(())
    }
}

struct Machine<'a> {
    memory: &'a Memory,
    cie: RuleSet,
    current: RuleSet,
    states: Vec<RuleSet>,
    location: u64,
    code_align: u64,
    pointer_encoding: u8,
    datarel: u64,
    target: u64,
}

fn read_block(memory: &Memory, at: &mut usize) -> Result<(usize, usize), String> {
    let length = read_uleb(memory, at)? as usize;
    let start = *at;
    memory
        .bytes(start, length)
        .map_err(|error| format!("{error}; next missing primitive: DWARF CFI expression"))?;
    *at += length;
    Ok((start, length))
}

fn read_uleb(memory: &Memory, at: &mut usize) -> Result<u64, String> {
    let mut value = 0u64;
    let mut shift = 0u32;
    loop {
        let byte = memory.bytes(*at, 1)?[0];
        *at += 1;
        if shift < 64 {
            value |= u64::from(byte & 0x7f) << shift;
        }
        shift += 7;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
        if shift > 70 {
            return Err(format!("DWARF CFI LEB128 value at {:#x} is longer than 10 bytes; next missing primitive: DWARF CFI LEB128", *at));
        }
    }
}

fn read_sleb(memory: &Memory, at: &mut usize) -> Result<i64, String> {
    let mut value = 0i64;
    let mut shift = 0u32;
    loop {
        let byte = memory.bytes(*at, 1)?[0];
        *at += 1;
        if shift < 64 {
            value |= i64::from(byte & 0x7f) << shift;
        }
        shift += 7;
        if byte & 0x80 == 0 {
            if shift < 64 && byte & 0x40 != 0 {
                value |= -1i64 << shift;
            }
            return Ok(value);
        }
        if shift > 70 {
            return Err(format!("DWARF CFI LEB128 value at {:#x} is longer than 10 bytes; next missing primitive: DWARF CFI LEB128", *at));
        }
    }
}

impl Machine<'_> {
    fn advance(&mut self, delta: u64) -> bool {
        let next = self
            .location
            .wrapping_add(delta.wrapping_mul(self.code_align));
        if next > self.target {
            return false;
        }
        self.location = next;
        true
    }

    fn run(&mut self, range: (usize, usize)) -> Result<(), String> {
        let mut at = range.0;
        while at < range.1 {
            let opcode = self.memory.bytes(at, 1)?[0];
            at += 1;
            match opcode {
                DW_CFA_NOP => {}
                DW_CFA_SET_LOC => {
                    let location = self.memory.encoded(&mut at, self.pointer_encoding, self.datarel)?;
                    if location > self.target {
                        return Ok(());
                    }
                    self.location = location;
                }
                DW_CFA_ADVANCE_LOC1 => {
                    let delta = u64::from(self.memory.bytes(at, 1)?[0]);
                    at += 1;
                    if !self.advance(delta) {
                        return Ok(());
                    }
                }
                DW_CFA_ADVANCE_LOC2 => {
                    let delta = u64::from(u16::from_le_bytes(self.memory.bytes(at, 2)?.try_into().expect("2 bytes")));
                    at += 2;
                    if !self.advance(delta) {
                        return Ok(());
                    }
                }
                DW_CFA_ADVANCE_LOC4 => {
                    let delta = u64::from(u32::from_le_bytes(self.memory.bytes(at, 4)?.try_into().expect("4 bytes")));
                    at += 4;
                    if !self.advance(delta) {
                        return Ok(());
                    }
                }
                DW_CFA_OFFSET_EXTENDED => {
                    let register = read_uleb(self.memory, &mut at)? as u32;
                    let factor = read_uleb(self.memory, &mut at)? as i64;
                    self.current.set(register, Rule::Offset(factor))?;
                }
                DW_CFA_RESTORE_EXTENDED => {
                    let register = read_uleb(self.memory, &mut at)? as u32;
                    let rule = self.cie.rule(register);
                    self.current.set(register, rule)?;
                }
                DW_CFA_UNDEFINED => {
                    let register = read_uleb(self.memory, &mut at)? as u32;
                    self.current.set(register, Rule::Undefined)?;
                }
                DW_CFA_SAME_VALUE => {
                    let register = read_uleb(self.memory, &mut at)? as u32;
                    self.current.set(register, Rule::SameValue)?;
                }
                DW_CFA_REGISTER => {
                    let register = read_uleb(self.memory, &mut at)? as u32;
                    let other = read_uleb(self.memory, &mut at)? as u32;
                    self.current.set(register, Rule::Register(other as u16))?;
                }
                DW_CFA_REMEMBER_STATE => self.states.push(self.current.clone()),
                DW_CFA_RESTORE_STATE => match self.states.pop() {
                    Some(state) => self.current = state,
                    None => return Err("DWARF CFI restore_state without remember_state; next missing primitive: DWARF CFI state stack".to_string()),
                },
                DW_CFA_DEF_CFA => {
                    let register = read_uleb(self.memory, &mut at)? as u32;
                    let offset = read_uleb(self.memory, &mut at)? as i64;
                    self.current.cfa = Some(CfaRule::Register(register as u16, offset));
                }
                DW_CFA_DEF_CFA_REGISTER => {
                    let register = read_uleb(self.memory, &mut at)? as u32;
                    let offset = match self.current.cfa {
                        Some(CfaRule::Register(_, offset)) => offset,
                        _ => 0,
                    };
                    self.current.cfa = Some(CfaRule::Register(register as u16, offset));
                }
                DW_CFA_DEF_CFA_OFFSET => {
                    let offset = read_uleb(self.memory, &mut at)? as i64;
                    let Some(CfaRule::Register(register, _)) = self.current.cfa else {
                        return Err("DWARF CFI def_cfa_offset without a CFA register; next missing primitive: DWARF CFI CFA rule".to_string());
                    };
                    self.current.cfa = Some(CfaRule::Register(register, offset));
                }
                DW_CFA_DEF_CFA_EXPRESSION => {
                    let block = read_block(self.memory, &mut at)?;
                    self.current.cfa = Some(CfaRule::Expression(block.0, block.1));
                }
                DW_CFA_EXPRESSION => {
                    let register = read_uleb(self.memory, &mut at)? as u32;
                    let block = read_block(self.memory, &mut at)?;
                    self.current.set(register, Rule::Expression(block.0, block.1))?;
                }
                DW_CFA_OFFSET_EXTENDED_SF => {
                    let register = read_uleb(self.memory, &mut at)? as u32;
                    let factor = read_sleb(self.memory, &mut at)?;
                    self.current.set(register, Rule::Offset(factor))?;
                }
                DW_CFA_DEF_CFA_SF => {
                    let register = read_uleb(self.memory, &mut at)? as u32;
                    let offset = read_sleb(self.memory, &mut at)?;
                    self.current.cfa = Some(CfaRule::Register(register as u16, offset));
                }
                DW_CFA_DEF_CFA_OFFSET_SF => {
                    let offset = read_sleb(self.memory, &mut at)?;
                    let Some(CfaRule::Register(register, _)) = self.current.cfa else {
                        return Err("DWARF CFI def_cfa_offset_sf without a CFA register; next missing primitive: DWARF CFI CFA rule".to_string());
                    };
                    self.current.cfa = Some(CfaRule::Register(register, offset));
                }
                DW_CFA_VAL_OFFSET => {
                    let register = read_uleb(self.memory, &mut at)? as u32;
                    let factor = read_uleb(self.memory, &mut at)? as i64;
                    self.current.set(register, Rule::ValueOffset(factor))?;
                }
                DW_CFA_VAL_OFFSET_SF => {
                    let register = read_uleb(self.memory, &mut at)? as u32;
                    let factor = read_sleb(self.memory, &mut at)?;
                    self.current.set(register, Rule::ValueOffset(factor))?;
                }
                DW_CFA_VAL_EXPRESSION => {
                    let register = read_uleb(self.memory, &mut at)? as u32;
                    let block = read_block(self.memory, &mut at)?;
                    self.current.set(register, Rule::ValueExpression(block.0, block.1))?;
                }
                DW_CFA_GNU_ARGS_SIZE => {
                    let _args_size = read_uleb(self.memory, &mut at)?;
                }
                DW_CFA_GNU_NEGATIVE_OFFSET_EXTENDED => {
                    let register = read_uleb(self.memory, &mut at)? as u32;
                    let factor = read_uleb(self.memory, &mut at)? as i64;
                    self.current.set(register, Rule::Offset(-factor))?;
                }
                other => match other & OPCODE_NAME_MASK {
                    DW_CFA_ADVANCE_LOC_BASE => {
                        let delta = u64::from(other & 0x3f);
                        if !self.advance(delta) {
                            return Ok(());
                        }
                    }
                    DW_CFA_OFFSET_BASE => {
                        let register = u32::from(other & 0x3f);
                        let factor = read_uleb(self.memory, &mut at)? as i64;
                        self.current.set(register, Rule::Offset(factor))?;
                    }
                    DW_CFA_RESTORE_BASE => {
                        let register = u32::from(other & 0x3f);
                        let rule = self.cie.rule(register);
                        self.current.set(register, rule)?;
                    }
                    _ => {
                        return Err(format!(
                            "unsupported DWARF CFI opcode {other:#x}; next missing primitive: DWARF CFI opcode {other:#x}"
                        ))
                    }
                },
            }
        }
        Ok(())
    }
}

pub fn execute(memory: &Memory, fde: &Fde, pc: u64) -> Result<RuleSet, String> {
    let mut machine = Machine {
        memory,
        cie: RuleSet::new(),
        current: RuleSet::new(),
        states: Vec::new(),
        location: 0,
        code_align: fde.cie.code_align.max(1),
        pointer_encoding: fde.cie.pointer_encoding,
        datarel: 0,
        target: u64::MAX,
    };
    machine.run(fde.cie.instructions)?;
    machine.cie = machine.current.clone();
    machine.location = fde.initial_location;
    machine.target = pc;
    machine.states.clear();
    machine.run(fde.instructions)?;
    Ok(machine.current)
}
