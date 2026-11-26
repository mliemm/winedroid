use std::arch::global_asm;

pub const STUB_COUNT: usize = 8192;
pub const STUB_SLOT_BYTES: usize = 8;
pub const GUEST_CALL_SIZE: usize = 336;

#[repr(C, align(16))]
#[derive(Debug)]
pub struct GuestCallFrame {
    pub function: u64,
    pub x: [u64; 8],
    pub x8: u64,
    pub q: [[u64; 2]; 8],
    pub stack_words: u64,
    pub stack: u64,
    pub ret_x: [u64; 2],
    pub ret_q: [[u64; 2]; 4],
    pub frame: u64,
    pub aborted: u64,
    pub stack_top: u64,
    pub serial: u64,
}

const _: () = assert!(std::mem::size_of::<GuestCallFrame>() == GUEST_CALL_SIZE);
const _: () = assert!(std::mem::offset_of!(GuestCallFrame, x) == 8);
const _: () = assert!(std::mem::offset_of!(GuestCallFrame, x8) == 72);
const _: () = assert!(std::mem::offset_of!(GuestCallFrame, q) == 80);
const _: () = assert!(std::mem::offset_of!(GuestCallFrame, stack_words) == 208);
const _: () = assert!(std::mem::offset_of!(GuestCallFrame, stack) == 216);
const _: () = assert!(std::mem::offset_of!(GuestCallFrame, ret_x) == 224);
const _: () = assert!(std::mem::offset_of!(GuestCallFrame, ret_q) == 240);
const _: () = assert!(std::mem::offset_of!(GuestCallFrame, frame) == 304);
const _: () = assert!(std::mem::offset_of!(GuestCallFrame, stack_top) == 320);

global_asm!(
    ".text",
    ".p2align 4",
    ".globl _spk_stub_template_start",
    "_spk_stub_template_start:",
    "Lspk_template_start:",
    ".rept 8192",
    "adr x16, .",
    "b Lspk_common",
    ".endr",
    "Lspk_common:",
    "sub sp, sp, #224",
    "stp x0, x1, [sp, #0]",
    "stp x2, x3, [sp, #16]",
    "stp x4, x5, [sp, #32]",
    "stp x6, x7, [sp, #48]",
    "add x9, sp, #224",
    "stp x8, x30, [sp, #64]",
    "str x9, [sp, #80]",
    "str x29, [sp, #88]",
    "stp q0, q1, [sp, #96]",
    "stp q2, q3, [sp, #128]",
    "stp q4, q5, [sp, #160]",
    "stp q6, q7, [sp, #192]",
    "adr x9, Lspk_template_start",
    "sub x0, x16, x9",
    "lsr x0, x0, #3",
    "mov x1, sp",
    "ldr x10, Lspk_dispatch_literal",
    "blr x10",
    "ldp q0, q1, [sp, #96]",
    "ldp q2, q3, [sp, #128]",
    "ldp q4, q5, [sp, #160]",
    "ldp q6, q7, [sp, #192]",
    "ldr x30, [sp, #72]",
    "ldp x0, x1, [sp, #0]",
    "add sp, sp, #224",
    "ret",
    ".p2align 3",
    "Lspk_dispatch_literal:",
    ".quad 0",
    ".globl _spk_stub_template_end",
    "_spk_stub_template_end:",
    ".p2align 2",
    ".globl _spk_call_guest",
    "_spk_call_guest:",
    "sub sp, sp, #160",
    "stp x29, x30, [sp, #0]",
    "stp x19, x20, [sp, #16]",
    "stp x21, x22, [sp, #32]",
    "stp x23, x24, [sp, #48]",
    "stp x25, x26, [sp, #64]",
    "stp x27, x28, [sp, #80]",
    "stp d8, d9, [sp, #96]",
    "stp d10, d11, [sp, #112]",
    "stp d12, d13, [sp, #128]",
    "stp d14, d15, [sp, #144]",
    "mov x29, sp",
    "mov x19, x0",
    "str x29, [x19, #304]",
    "ldr x14, [x19, #320]",
    "cbz x14, 3f",
    "mov sp, x14",
    "3:",
    "ldr x9, [x19, #208]",
    "ldr x10, [x19, #216]",
    "add x11, x9, #1",
    "and x11, x11, #-2",
    "sub sp, sp, x11, lsl #3",
    "mov x12, sp",
    "cbz x9, 2f",
    "1:",
    "ldr x13, [x10], #8",
    "str x13, [x12], #8",
    "subs x9, x9, #1",
    "b.ne 1b",
    "2:",
    "ldp q0, q1, [x19, #80]",
    "ldp q2, q3, [x19, #112]",
    "ldp q4, q5, [x19, #144]",
    "ldp q6, q7, [x19, #176]",
    "ldp x2, x3, [x19, #24]",
    "ldp x4, x5, [x19, #40]",
    "ldp x6, x7, [x19, #56]",
    "ldr x8, [x19, #72]",
    "ldp x0, x1, [x19, #8]",
    "ldr x16, [x19]",
    "blr x16",
    "stp x0, x1, [x19, #224]",
    "stp q0, q1, [x19, #240]",
    "stp q2, q3, [x19, #272]",
    "Lspk_call_guest_leave:",
    "mov sp, x29",
    "ldp x19, x20, [sp, #16]",
    "ldp x21, x22, [sp, #32]",
    "ldp x23, x24, [sp, #48]",
    "ldp x25, x26, [sp, #64]",
    "ldp x27, x28, [sp, #80]",
    "ldp d8, d9, [sp, #96]",
    "ldp d10, d11, [sp, #112]",
    "ldp d12, d13, [sp, #128]",
    "ldp d14, d15, [sp, #144]",
    "ldp x29, x30, [sp, #0]",
    "add sp, sp, #160",
    "ret",
    ".p2align 2",
    ".globl _spk_unwind_to",
    "_spk_unwind_to:",
    "mov x29, x0",
    "b Lspk_call_guest_leave",
);

extern "C" {
    static spk_stub_template_start: u8;
    static spk_stub_template_end: u8;
    pub fn spk_call_guest(call: *mut GuestCallFrame);
    pub fn spk_unwind_to(frame: u64) -> !;
}

pub fn template_bounds() -> (usize, usize) {
    (
        std::ptr::addr_of!(spk_stub_template_start) as usize,
        std::ptr::addr_of!(spk_stub_template_end) as usize,
    )
}
