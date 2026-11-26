use super::fde::Memory;
use super::registers::{register_value, UnwindRegs};

const DW_OP_ADDR: u8 = 0x03;
const DW_OP_DEREF: u8 = 0x06;
const DW_OP_CONST1U: u8 = 0x08;
const DW_OP_CONST1S: u8 = 0x09;
const DW_OP_CONST2U: u8 = 0x0a;
const DW_OP_CONST2S: u8 = 0x0b;
const DW_OP_CONST4U: u8 = 0x0c;
const DW_OP_CONST4S: u8 = 0x0d;
const DW_OP_CONST8U: u8 = 0x0e;
const DW_OP_CONST8S: u8 = 0x0f;
const DW_OP_CONSTU: u8 = 0x10;
const DW_OP_CONSTS: u8 = 0x11;
const DW_OP_DUP: u8 = 0x12;
const DW_OP_DROP: u8 = 0x13;
const DW_OP_PICK: u8 = 0x15;
const DW_OP_SWAP: u8 = 0x16;
const DW_OP_AND: u8 = 0x1a;
const DW_OP_MINUS: u8 = 0x1c;
const DW_OP_PLUS: u8 = 0x22;
const DW_OP_PLUS_UCONST: u8 = 0x23;
const DW_OP_SHL: u8 = 0x24;
const DW_OP_SHR: u8 = 0x25;
const DW_OP_LITERAL_BASE: u8 = 0x30;
const DW_OP_BREG_BASE: u8 = 0x70;
const DW_OP_BREGX: u8 = 0x92;
const DW_OP_NOP: u8 = 0x96;
const DW_OP_STACK_VALUE: u8 = 0x9f;
const DEPTH_LIMIT: usize = 64;

pub trait AddressReader {
    fn read_u64(&self, address: u64) -> Result<u64, String>;
}

fn push(stack: &mut Vec<u64>, value: u64) -> Result<(), String> {
    if stack.len() >= DEPTH_LIMIT {
        return Err(
            "DWARF expression stack overflow; next missing primitive: DWARF expression depth"
                .to_string(),
        );
    }
    stack.push(value);
    Ok(())
}

fn pop(stack: &mut Vec<u64>) -> Result<u64, String> {
    stack.pop().ok_or_else(|| {
        "DWARF expression stack underflow; next missing primitive: DWARF expression".to_string()
    })
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
            return Err(format!("DWARF expression LEB128 value at {:#x} is longer than 10 bytes; next missing primitive: DWARF expression", *at));
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
            return Err(format!("DWARF expression LEB128 value at {:#x} is longer than 10 bytes; next missing primitive: DWARF expression", *at));
        }
    }
}

pub fn evaluate(
    reader: &dyn AddressReader,
    memory: &Memory,
    expression: (usize, usize),
    regs: &UnwindRegs,
) -> Result<u64, String> {
    let mut stack: Vec<u64> = Vec::new();
    let mut at = expression.0;
    let end = expression.0 + expression.1;
    while at < end {
        let opcode = memory.bytes(at, 1)?[0];
        at += 1;
        match opcode {
            DW_OP_ADDR => {
                let address = memory.bytes(at, 8)?.try_into().expect("8 bytes");
                at += 8;
                push(&mut stack, u64::from_le_bytes(address))?;
            }
            DW_OP_DEREF => {
                let address = pop(&mut stack)?;
                push(&mut stack, reader.read_u64(address)?)?;
            }
            DW_OP_CONST1U => {
                let value = u64::from(memory.bytes(at, 1)?[0]);
                at += 1;
                push(&mut stack, value)?;
            }
            DW_OP_CONST1S => {
                let value = memory.bytes(at, 1)?[0] as i8 as i64 as u64;
                at += 1;
                push(&mut stack, value)?;
            }
            DW_OP_CONST2U => {
                let value = u64::from(u16::from_le_bytes(memory.bytes(at, 2)?.try_into().expect("2 bytes")));
                at += 2;
                push(&mut stack, value)?;
            }
            DW_OP_CONST2S => {
                let value = i16::from_le_bytes(memory.bytes(at, 2)?.try_into().expect("2 bytes")) as i64 as u64;
                at += 2;
                push(&mut stack, value)?;
            }
            DW_OP_CONST4U => {
                let value = u64::from(u32::from_le_bytes(memory.bytes(at, 4)?.try_into().expect("4 bytes")));
                at += 4;
                push(&mut stack, value)?;
            }
            DW_OP_CONST4S => {
                let value = i32::from_le_bytes(memory.bytes(at, 4)?.try_into().expect("4 bytes")) as i64 as u64;
                at += 4;
                push(&mut stack, value)?;
            }
            DW_OP_CONST8U => {
                let value = u64::from_le_bytes(memory.bytes(at, 8)?.try_into().expect("8 bytes"));
                at += 8;
                push(&mut stack, value)?;
            }
            DW_OP_CONST8S => {
                let value = i64::from_le_bytes(memory.bytes(at, 8)?.try_into().expect("8 bytes")) as u64;
                at += 8;
                push(&mut stack, value)?;
            }
            DW_OP_CONSTU => {
                let value = read_uleb(memory, &mut at)?;
                push(&mut stack, value)?;
            }
            DW_OP_CONSTS => {
                let value = read_sleb(memory, &mut at)? as u64;
                push(&mut stack, value)?;
            }
            DW_OP_DUP => {
                let value = *stack.last().ok_or_else(|| "DWARF expression stack underflow; next missing primitive: DWARF expression".to_string())?;
                push(&mut stack, value)?;
            }
            DW_OP_DROP => {
                pop(&mut stack)?;
            }
            DW_OP_PICK => {
                let index = u64::from(memory.bytes(at, 1)?[0]) as usize;
                at += 1;
                let value = *stack.iter().rev().nth(index).ok_or_else(|| "DWARF expression pick out of range; next missing primitive: DWARF expression".to_string())?;
                push(&mut stack, value)?;
            }
            DW_OP_SWAP => {
                let (first, second) = (pop(&mut stack)?, pop(&mut stack)?);
                push(&mut stack, first)?;
                push(&mut stack, second)?;
            }
            DW_OP_AND => {
                let (first, second) = (pop(&mut stack)?, pop(&mut stack)?);
                push(&mut stack, first & second)?;
            }
            DW_OP_MINUS => {
                let (first, second) = (pop(&mut stack)?, pop(&mut stack)?);
                push(&mut stack, second.wrapping_sub(first))?;
            }
            DW_OP_PLUS => {
                let (first, second) = (pop(&mut stack)?, pop(&mut stack)?);
                push(&mut stack, second.wrapping_add(first))?;
            }
            DW_OP_PLUS_UCONST => {
                let increment = read_uleb(memory, &mut at)?;
                let value = pop(&mut stack)?;
                push(&mut stack, value.wrapping_add(increment))?;
            }
            DW_OP_SHL => {
                let (shift, value) = (pop(&mut stack)?, pop(&mut stack)?);
                push(&mut stack, value.wrapping_shl(shift as u32 & 63))?;
            }
            DW_OP_SHR => {
                let (shift, value) = (pop(&mut stack)?, pop(&mut stack)?);
                push(&mut stack, value.wrapping_shr(shift as u32 & 63))?;
            }
            DW_OP_LITERAL_BASE..=0x4f => {
                push(&mut stack, u64::from(opcode - DW_OP_LITERAL_BASE))?;
            }
            DW_OP_BREG_BASE..=0x8f => {
                let register = u32::from(opcode - DW_OP_BREG_BASE);
                let offset = read_sleb(memory, &mut at)? as u64;
                let value = register_value(regs, register).ok_or_else(|| {
                    format!("DWARF expression names register {register}; next missing primitive: DWARF register {register}")
                })?;
                push(&mut stack, value.wrapping_add(offset))?;
            }
            DW_OP_BREGX => {
                let register = read_uleb(memory, &mut at)? as u32;
                let offset = read_sleb(memory, &mut at)? as u64;
                let value = register_value(regs, register).ok_or_else(|| {
                    format!("DWARF expression names register {register}; next missing primitive: DWARF register {register}")
                })?;
                push(&mut stack, value.wrapping_add(offset))?;
            }
            DW_OP_NOP => {}
            DW_OP_STACK_VALUE => {}
            other => {
                return Err(format!(
                    "unsupported DWARF expression opcode {other:#x} at {:#x}; next missing primitive: DWARF expression opcode {other:#x}",
                    at - 1
                ))
            }
        }
    }
    pop(&mut stack)
}
