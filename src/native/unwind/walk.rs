use std::ffi::c_void;

use super::super::call_assign::CallBuilder;
use super::super::bionic::guest_mem::read_value;
use super::cfi::{self, CfaRule, Rule, RuleSet};
use super::dwarf_expr::{self, AddressReader};
use super::fde::{self, Found};
use super::registers::{register_value, UnwindContext, UnwindRegs, SHADOW_CALL_STACK_REGISTER, STACK_POINTER_REGISTER, VECTOR_FIRST_REGISTER, VECTOR_REGISTER_COUNT};
use super::{
    refuse, URC_CONTINUE_UNWIND, URC_END_OF_STACK, URC_FATAL_PHASE1_ERROR, URC_FATAL_PHASE2_ERROR, URC_HANDLER_FOUND, URC_INSTALL_CONTEXT, URC_NO_REASON,
    UA_CLEANUP_PHASE, UA_FORCE_UNWIND, UA_HANDLER_FRAME, UA_SEARCH_PHASE, UNWIND_VERSION,
};

extern "C" {
    fn pthread_self() -> *mut c_void;
    fn pthread_get_stackaddr_np(thread: *mut c_void) -> *mut c_void;
    fn pthread_get_stacksize_np(thread: *mut c_void) -> usize;
}

const ALIGNMENT: u64 = 16;

pub struct Space {
    pub stack_low: u64,
    pub stack_high: u64,
}

impl Space {
    pub fn current() -> Result<Self, String> {
        let thread = unsafe { pthread_self() };
        let high = unsafe { pthread_get_stackaddr_np(thread) } as u64;
        let size = unsafe { pthread_get_stacksize_np(thread) } as u64;
        if high == 0 || size == 0 {
            return Err("the current thread has no readable stack bounds; next missing primitive: thread stack bounds".to_string());
        }
        Ok(Self { stack_low: high - size, stack_high: high })
    }

    fn contains(&self, address: u64, len: u64) -> bool {
        address >= self.stack_low && address.checked_add(len).is_some_and(|end| end <= self.stack_high)
    }
}

impl AddressReader for Space {
    fn read_u64(&self, address: u64) -> Result<u64, String> {
        if !self.contains(address, 8) {
            return Err(format!("unwind read of {address:#x} is outside the thread stack {:#x}..{:#x}", self.stack_low, self.stack_high));
        }
        Ok(read_value::<u64>(address as usize))
    }
}

pub enum Step {
    Next,
    EndOfStack,
}

enum Phase {
    Install(UnwindRegs),
    End,
    Fatal,
}

pub enum Outcome {
    Return(i32),
    Install(UnwindRegs),
}

fn frame_at(space: &Space, regs: &UnwindRegs) -> Result<Option<(Found, RuleSet, u64)>, String> {
    if regs.pc == 0 {
        return Ok(None);
    }
    let Some(found) = fde::lookup(regs.pc)? else { return Ok(None) };
    let rules = cfi::execute(&found.memory, &found.fde, regs.pc)?;
    let cfa = compute_cfa(space, &found, &rules, regs)?;
    guard(space, regs, cfa, &found)?;
    Ok(Some((found, rules, cfa)))
}

fn compute_cfa(space: &Space, found: &Found, rules: &RuleSet, regs: &UnwindRegs) -> Result<u64, String> {
    match rules.cfa {
        Some(CfaRule::Register(register, offset)) => {
            let value = register_value(regs, u32::from(register)).ok_or_else(|| {
                format!(
                    "the CFA of {:#x} in {} uses unknown register {register}; next missing primitive: DWARF register {register}",
                    regs.pc, found.name
                )
            })?;
            Ok(value.wrapping_add(offset as u64))
        }
        Some(CfaRule::Expression(start, length)) => dwarf_expr::evaluate(space, &found.memory, (start, length), regs),
        None => Err(format!("no CFA rule for {:#x} in {}; next missing primitive: DWARF CFA rule", regs.pc, found.name)),
    }
}

pub(crate) fn guard(space: &Space, regs: &UnwindRegs, cfa: u64, found: &Found) -> Result<(), String> {
    if cfa < regs.sp {
        return Err(format!(
            "the CFA {cfa:#x} of {:#x} in {} is below its stack pointer {:#x}; next missing primitive: unwind stack direction",
            regs.pc, found.name, regs.sp
        ));
    }
    if cfa % ALIGNMENT != 0 {
        return Err(format!("the CFA {cfa:#x} of {:#x} in {} is not 16 byte aligned; next missing primitive: unwind stack alignment", regs.pc, found.name));
    }
    if !space.contains(cfa, 0) {
        return Err(format!(
            "the CFA {cfa:#x} of {:#x} in {} is outside the thread stack {:#x}..{:#x}; next missing primitive: unwind stack bounds",
            regs.pc, found.name, space.stack_low, space.stack_high
        ));
    }
    Ok(())
}

struct Restorer<'a> {
    space: &'a Space,
    found: &'a Found,
    rules: &'a RuleSet,
    cfa: u64,
    regs: &'a UnwindRegs,
    resolved: [Option<Option<u64>>; 31],
    visiting: [bool; 31],
}

impl Restorer<'_> {
    fn value(&mut self, register: u32) -> Result<Option<u64>, String> {
        if register == STACK_POINTER_REGISTER {
            return Ok(Some(self.cfa));
        }
        let index = register as usize;
        if index >= self.resolved.len() {
            return Ok(None);
        }
        if let Some(value) = self.resolved[index] {
            return Ok(value);
        }
        if self.visiting[index] {
            return Err(format!(
                "unwind register {register} restores from itself in {}; next missing primitive: DWARF register rule cycle",
                self.found.name
            ));
        }
        self.visiting[index] = true;
        let value = match self.rules.rule(register) {
            Rule::SameValue => Some(self.regs.x[index]),
            Rule::Undefined => None,
            rule => self.rule_value(rule)?,
        };
        self.visiting[index] = false;
        self.resolved[index] = Some(value);
        Ok(value)
    }

    fn rule_value(&mut self, rule: Rule) -> Result<Option<u64>, String> {
        let factor = self.found.fde.cie.data_align;
        match rule {
            Rule::Undefined | Rule::SameValue => Ok(None),
            Rule::Offset(offset) => Ok(Some(self.read(self.cfa.wrapping_add(offset.wrapping_mul(factor) as u64))?)),
            Rule::ValueOffset(offset) => Ok(Some(self.cfa.wrapping_add(offset.wrapping_mul(factor) as u64))),
            Rule::Register(other) => self.value(u32::from(other)),
            Rule::Expression(start, length) => {
                let address = dwarf_expr::evaluate(self.space, &self.found.memory, (start, length), self.regs)?;
                Ok(Some(self.read(address)?))
            }
            Rule::ValueExpression(start, length) => Ok(Some(dwarf_expr::evaluate(self.space, &self.found.memory, (start, length), self.regs)?)),
        }
    }

    fn read(&self, address: u64) -> Result<u64, String> {
        self.space.read_u64(address)
    }
}

fn restore(space: &Space, found: &Found, rules: &RuleSet, regs: &UnwindRegs, cfa: u64) -> Result<Option<UnwindRegs>, String> {
    let return_register = found.fde.cie.return_register;
    if return_register >= 31 {
        return Err(format!(
            "the return address register {return_register} of {} is not a general register; next missing primitive: DWARF return address register",
            found.name
        ));
    }
    if rules.rule(return_register) == Rule::Undefined {
        return Ok(None);
    }
    let mut restorer = Restorer { space, found, rules, cfa, regs, resolved: [None; 31], visiting: [false; 31] };
    let mut next = *regs;
    next.x[SHADOW_CALL_STACK_REGISTER] = regs.x[SHADOW_CALL_STACK_REGISTER];
    for register in 0..31u32 {
        if register as usize == SHADOW_CALL_STACK_REGISTER || rules.rule(register) == Rule::Undefined {
            continue;
        }
        if let Some(value) = restorer.value(register)? {
            next.x[register as usize] = value;
        }
    }
    for slot in 0..VECTOR_REGISTER_COUNT {
        let rule = rules.rule(VECTOR_FIRST_REGISTER + slot as u32);
        if matches!(rule, Rule::Undefined | Rule::SameValue) {
            continue;
        }
        if let Some(value) = restorer.rule_value(rule)? {
            next.q[slot] = u128::from(value);
        }
    }
    let Some(return_address) = restorer.value(return_register)? else { return Ok(None) };
    next.sp = cfa;
    next.pc = return_address;
    Ok(Some(next))
}

fn step(space: &Space, regs: &mut UnwindRegs, budget: &mut usize) -> Result<Step, String> {
    let Some((found, rules, cfa)) = frame_at(space, regs)? else { return Ok(Step::EndOfStack) };
    let Some(next) = restore(space, &found, &rules, regs, cfa)? else { return Ok(Step::EndOfStack) };
    if *budget == 0 {
        return Err("unwind frame budget exceeded; next missing primitive: deeper unwind frame budget".to_string());
    }
    *budget -= 1;
    *regs = next;
    Ok(Step::Next)
}

fn personality_for(found: &Found) -> Result<u64, String> {
    let personality = found.fde.cie.personality;
    if personality == 0 {
        return Ok(0);
    }
    if let Some(image) = found.memory.image() {
        let vaddr = personality.wrapping_sub(image.mapped.load_bias() as u64);
        if image.mapped.contains_vaddr(vaddr, 1) && image.foreign_unwinder {
            return Err(format!(
                "personality {personality:#x} in {} uses its own unwinder; next missing primitive: unwinding through a personality that uses its own unwinder",
                found.name
            ));
        }
    }
    Ok(personality)
}

fn make_context(found: &Found, regs: &UnwindRegs, cfa: u64, actions: i32) -> UnwindContext {
    let mut context = UnwindContext::new(*regs);
    context.cfa = cfa;
    context.region_start = found.fde.initial_location;
    context.lsda = found.fde.lsda;
    context.fde = found.fde.address as u64;
    context.personality = found.fde.cie.personality;
    context.handler_frame = actions & UA_HANDLER_FRAME != 0;
    context
}

fn call_personality(context: &mut UnwindContext, exception: u64, actions: i32) -> Result<i32, String> {
    let outcome = CallBuilder::new(context.personality as usize)
        .int(UNWIND_VERSION as u64)
        .int(actions as u64)
        .int(read_value::<u64>(exception as usize))
        .ptr(exception as usize)
        .ptr((context as *mut UnwindContext) as usize)
        .invoke()
        .map_err(|error| error.to_string())?;
    Ok(outcome.i32())
}

fn call_stop(stop: u64, exception: u64, frame: &UnwindContext, argument: u64) -> Result<i32, String> {
    let outcome = CallBuilder::new(stop as usize)
        .int(1)
        .int((UA_FORCE_UNWIND | UA_CLEANUP_PHASE) as u64)
        .int(read_value::<u64>(exception as usize))
        .ptr(exception as usize)
        .ptr((&*frame as *const UnwindContext) as usize)
        .ptr(argument as usize)
        .invoke()
        .map_err(|error| error.to_string())?;
    Ok(outcome.i32())
}

fn cleanup(space: &Space, start: &UnwindRegs, exception: u64, handler: Option<usize>, stop: Option<(u64, u64)>) -> Result<Phase, String> {
    let mut cursor = *start;
    let mut budget = super::FRAME_BUDGET;
    let mut index = 0usize;
    loop {
        let Some((found, rules, cfa)) = frame_at(space, &cursor)? else { return Ok(Phase::End) };
        let _ = rules;
        if let Some((function, argument)) = stop {
            let frame = make_context(&found, &cursor, cfa, UA_FORCE_UNWIND | UA_CLEANUP_PHASE);
            match call_stop(function, exception, &frame, argument)? {
                URC_NO_REASON => {}
                URC_END_OF_STACK => return Ok(Phase::End),
                _ => return Ok(Phase::Fatal),
            }
        }
        let actions = UA_CLEANUP_PHASE | if stop.is_some() { UA_FORCE_UNWIND } else { 0 } | if handler == Some(index) { UA_HANDLER_FRAME } else { 0 };
        if personality_for(&found)? != 0 {
            let mut context = make_context(&found, &cursor, cfa, actions);
            match call_personality(&mut context, exception, actions)? {
                URC_INSTALL_CONTEXT => return Ok(Phase::Install(context.regs)),
                URC_CONTINUE_UNWIND => {}
                other => {
                    return Err(format!(
                        "the personality in {} returned {other} during the cleanup phase; next missing primitive: unwind personality result {other}",
                        found.name
                    ))
                }
            }
        }
        if handler.is_some_and(|handler| index >= handler) {
            return Ok(Phase::Fatal);
        }
        match step(space, &mut cursor, &mut budget)? {
            Step::Next => {}
            Step::EndOfStack => return Ok(Phase::End),
        }
        index += 1;
    }
}

pub fn dispatch(selector: u64, regs: &UnwindRegs) -> Outcome {
    match run(selector, regs) {
        Ok(outcome) => outcome,
        Err(message) => refuse(&message),
    }
}

fn run(selector: u64, regs: &UnwindRegs) -> Result<Outcome, String> {
    let space = Space::current()?;
    match selector {
        0 => raise(&space, regs),
        1 => resume(&space, regs),
        2 => forced(&space, regs),
        3 => backtrace(&space, regs),
        other => Err(format!("unknown unwind entry selector {other}; next missing primitive: unwind entry")),
    }
}

fn raise(space: &Space, start: &UnwindRegs) -> Result<Outcome, String> {
    let exception = start.x[0];
    let mut cursor = *start;
    let mut budget = super::FRAME_BUDGET;
    let mut index = 0usize;
    let mut handler = None;
    loop {
        let Some((found, _rules, cfa)) = frame_at(space, &cursor)? else { break };
        if personality_for(&found)? != 0 {
            let mut context = make_context(&found, &cursor, cfa, UA_SEARCH_PHASE);
            match call_personality(&mut context, exception, UA_SEARCH_PHASE)? {
                URC_HANDLER_FOUND => {
                    handler = Some(index);
                    break;
                }
                URC_CONTINUE_UNWIND => {}
                URC_FATAL_PHASE1_ERROR => return Ok(Outcome::Return(URC_FATAL_PHASE1_ERROR)),
                other => {
                    return Err(format!(
                        "the personality in {} returned {other} during the search phase; next missing primitive: unwind personality result {other}",
                        found.name
                    ))
                }
            }
        }
        match step(space, &mut cursor, &mut budget)? {
            Step::Next => {}
            Step::EndOfStack => break,
        }
        index += 1;
    }
    let Some(handler) = handler else { return Ok(Outcome::Return(URC_END_OF_STACK)) };
    match cleanup(space, start, exception, Some(handler), None)? {
        Phase::Install(regs) => Ok(Outcome::Install(regs)),
        Phase::End | Phase::Fatal => Ok(Outcome::Return(URC_FATAL_PHASE2_ERROR)),
    }
}

fn resume(space: &Space, start: &UnwindRegs) -> Result<Outcome, String> {
    let exception = start.x[0];
    let mut cursor = *start;
    let mut budget = super::FRAME_BUDGET;
    match step(space, &mut cursor, &mut budget)? {
        Step::Next => {}
        Step::EndOfStack => return Err("an exception reached the end of the stack during a resume; next missing primitive: uncaught exception".to_string()),
    }
    match cleanup(space, &cursor, exception, None, None)? {
        Phase::Install(regs) => Ok(Outcome::Install(regs)),
        Phase::End | Phase::Fatal => Err("an exception reached the end of the stack during a resume; next missing primitive: uncaught exception".to_string()),
    }
}

fn forced(space: &Space, start: &UnwindRegs) -> Result<Outcome, String> {
    let exception = start.x[0];
    let (stop, argument) = (start.x[1], start.x[2]);
    match cleanup(space, start, exception, None, Some((stop, argument)))? {
        Phase::Install(regs) => Ok(Outcome::Install(regs)),
        Phase::End => Ok(Outcome::Return(URC_END_OF_STACK)),
        Phase::Fatal => Ok(Outcome::Return(URC_FATAL_PHASE2_ERROR)),
    }
}

fn backtrace(space: &Space, start: &UnwindRegs) -> Result<Outcome, String> {
    let trace = start.x[0];
    let argument = start.x[1];
    let mut cursor = *start;
    let mut budget = super::FRAME_BUDGET;
    loop {
        let Some((found, _rules, cfa)) = frame_at(space, &cursor)? else { return Ok(Outcome::Return(URC_END_OF_STACK)) };
        let mut context = make_context(&found, &cursor, cfa, 0);
        let outcome = CallBuilder::new(trace as usize)
            .ptr((&mut context as *mut UnwindContext) as usize)
            .ptr(argument as usize)
            .invoke()
            .map_err(|error| error.to_string())?;
        let result = outcome.i32();
        if result != 0 {
            return Ok(Outcome::Return(result));
        }
        match step(space, &mut cursor, &mut budget)? {
            Step::Next => {}
            Step::EndOfStack => return Ok(Outcome::Return(URC_END_OF_STACK)),
        }
    }
}
