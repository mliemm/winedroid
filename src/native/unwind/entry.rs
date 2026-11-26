use std::arch::global_asm;

use super::super::host_log;
use super::super::signal_state;
use super::registers::UnwindRegs;
use super::walk::{self, Outcome};

global_asm!(
    ".text",
    ".p2align 4",
    ".globl _spk_unwind_raise",
    "_spk_unwind_raise:",
    "sub sp, sp, #416",
    "str x9, [sp, #400]",
    "mov x9, #0",
    "str x9, [sp, #408]",
    "ldr x9, [sp, #400]",
    "b Lspk_unwind_body",
    ".globl _spk_unwind_resume",
    "_spk_unwind_resume:",
    "sub sp, sp, #416",
    "str x9, [sp, #400]",
    "mov x9, #1",
    "str x9, [sp, #408]",
    "ldr x9, [sp, #400]",
    "b Lspk_unwind_body",
    ".globl _spk_unwind_forced",
    "_spk_unwind_forced:",
    "sub sp, sp, #416",
    "str x9, [sp, #400]",
    "mov x9, #2",
    "str x9, [sp, #408]",
    "ldr x9, [sp, #400]",
    "b Lspk_unwind_body",
    ".globl _spk_unwind_backtrace",
    "_spk_unwind_backtrace:",
    "sub sp, sp, #416",
    "str x9, [sp, #400]",
    "mov x9, #3",
    "str x9, [sp, #408]",
    "ldr x9, [sp, #400]",
    "b Lspk_unwind_body",
    "Lspk_unwind_body:",
    "stp q8, q9, [sp, #0]",
    "stp q10, q11, [sp, #32]",
    "stp q12, q13, [sp, #64]",
    "stp q14, q15, [sp, #96]",
    "stp x0, x1, [sp, #128]",
    "stp x2, x3, [sp, #144]",
    "stp x4, x5, [sp, #160]",
    "stp x6, x7, [sp, #176]",
    "stp x8, x9, [sp, #192]",
    "stp x10, x11, [sp, #208]",
    "stp x12, x13, [sp, #224]",
    "stp x14, x15, [sp, #240]",
    "stp x16, x17, [sp, #256]",
    "stp x18, x19, [sp, #272]",
    "stp x20, x21, [sp, #288]",
    "stp x22, x23, [sp, #304]",
    "stp x24, x25, [sp, #320]",
    "stp x26, x27, [sp, #336]",
    "stp x28, x29, [sp, #352]",
    "str x30, [sp, #368]",
    "add x9, sp, #416",
    "str x9, [sp, #376]",
    "str x30, [sp, #384]",
    "mov x0, sp",
    "ldr x1, [sp, #408]",
    "bl {dispatch}",
    "ldr x16, [sp, #384]",
    "ldr x17, [sp, #376]",
    "ldp q8, q9, [sp, #0]",
    "ldp q10, q11, [sp, #32]",
    "ldp q12, q13, [sp, #64]",
    "ldp q14, q15, [sp, #96]",
    "ldp x0, x1, [sp, #128]",
    "ldp x2, x3, [sp, #144]",
    "ldp x4, x5, [sp, #160]",
    "ldp x6, x7, [sp, #176]",
    "ldp x8, x9, [sp, #192]",
    "ldp x10, x11, [sp, #208]",
    "ldp x12, x13, [sp, #224]",
    "ldp x14, x15, [sp, #240]",
    "ldp x18, x19, [sp, #272]",
    "ldp x20, x21, [sp, #288]",
    "ldp x22, x23, [sp, #304]",
    "ldp x24, x25, [sp, #320]",
    "ldp x26, x27, [sp, #336]",
    "ldp x28, x29, [sp, #352]",
    "ldr x30, [sp, #368]",
    "mov sp, x17",
    "br x16",
    dispatch = sym dispatch_entry,
);

extern "C" {
    pub fn spk_unwind_raise();
    pub fn spk_unwind_resume();
    pub fn spk_unwind_forced();
    pub fn spk_unwind_backtrace();
}

extern "C" fn dispatch_entry(block: *mut UnwindRegs, selector: u64) {
    let start = unsafe { std::ptr::read(block) };
    signal_state::set_in_guest(false);
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        walk::dispatch(selector, &start)
    }));
    match outcome {
        Ok(Outcome::Return(code)) => unsafe { (*block).x[0] = code as u64 },
        Ok(Outcome::Install(updated)) => unsafe { std::ptr::write(block, updated) },
        Err(_) => {
            host_log::line(&format!("host panic while unwinding from selector {selector}; next missing primitive: unwinding across a host frame"));
            std::process::abort();
        }
    }
    signal_state::set_in_guest(true);
}
