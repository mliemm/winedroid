use std::arch::global_asm;
use std::cell::Cell;
use std::sync::OnceLock;

use super::super::guest_call::{abort_active_call, current_serial, only_signal_handlers_above};
use super::super::{signal_router, signal_state};
use super::export::in_libc_raw;
use super::fatal::fatal;
use super::signal_handlers::set_signal_mask;

// cite: bionic setjmp.S#_JB_SIGFLAG, _JB_SIGMASK, _JB_X30_SP, _JB_CHECKSUM
const FLAG: usize = 0;
const MASK: usize = 1;
const STACK_POINTER: usize = 3;
const CHECKSUM: usize = 24;
const FRAME: usize = 25;

#[derive(Clone, Copy)]
struct Jump {
    words: [u64; CHECKSUM],
    frame: u64,
    value: i32,
}

static COOKIE: OnceLock<u64> = OnceLock::new();

fn cookie() -> u64 {
    // cite: bionic setjmp_cookie.cpp#__libc_init_setjmp_cookie
    *COOKIE.get_or_init(|| {
        let mut value = 0u64;
        unsafe {
            libc::arc4random_buf(
                &mut value as *mut u64 as *mut libc::c_void,
                std::mem::size_of::<u64>(),
            )
        };
        value & !1
    })
}

thread_local! {
    static PENDING: Cell<Option<Jump>> = const { Cell::new(None) };
}

// cite: bionic setjmp.S#setjmp, _setjmp, sigsetjmp, siglongjmp, m_calculate_checksum
global_asm!(
    ".text",
    ".p2align 2",
    ".globl _spk_setjmp",
    "_spk_setjmp:",
    "mov w1, #1",
    "b _spk_sigsetjmp",
    ".globl _spk_setjmp_unmasked",
    "_spk_setjmp_unmasked:",
    "mov w1, #0",
    ".globl _spk_sigsetjmp",
    "_spk_sigsetjmp:",
    "mov x9, sp",
    "stp x30, x9, [x0, #16]",
    "stp x28, x29, [x0, #32]",
    "stp x26, x27, [x0, #48]",
    "stp x24, x25, [x0, #64]",
    "stp x22, x23, [x0, #80]",
    "stp x20, x21, [x0, #96]",
    "stp xzr, x19, [x0, #112]",
    "stp d14, d15, [x0, #128]",
    "stp d12, d13, [x0, #144]",
    "stp d10, d11, [x0, #160]",
    "stp d8, d9, [x0, #176]",
    "b {record}",
    ".globl _spk_longjmp_restore",
    "_spk_longjmp_restore:",
    "ldp x30, x9, [x0, #16]",
    "ldp x28, x29, [x0, #32]",
    "ldp x26, x27, [x0, #48]",
    "ldp x24, x25, [x0, #64]",
    "ldp x22, x23, [x0, #80]",
    "ldp x20, x21, [x0, #96]",
    "ldr x19, [x0, #120]",
    "ldp d14, d15, [x0, #128]",
    "ldp d12, d13, [x0, #144]",
    "ldp d10, d11, [x0, #160]",
    "ldp d8, d9, [x0, #176]",
    "mov sp, x9",
    "cmp w1, #0",
    "csinc w0, w1, wzr, ne",
    "ret",
    record = sym record,
);

extern "C" {
    fn spk_setjmp();
    fn spk_setjmp_unmasked();
    fn spk_sigsetjmp();
    fn spk_longjmp_restore(words: *const u64, value: i32) -> !;
}

fn checksum(words: &[u64; CHECKSUM]) -> u64 {
    words.iter().fold(0, |sum, word| sum ^ word)
}

extern "C" fn record(env: *mut u64, save_mask: i32) -> i32 {
    // cite: bionic setjmp_cookie.cpp#__bionic_setjmp_cookie_get
    let outer = signal_state::set_in_guest(false);
    if save_mask & !1 != 0 {
        fatal(None, &format!("unexpected sigflag value: {save_mask}"));
    }
    unsafe {
        env.add(FLAG).write_unaligned(cookie() | save_mask as u64);
        if save_mask != 0 {
            env.add(MASK).write_unaligned(signal_state::mask());
        }
        let words = env.cast::<[u64; CHECKSUM]>().read_unaligned();
        env.add(CHECKSUM).write_unaligned(checksum(&words));
        env.add(FRAME).write_unaligned(current_serial());
    }
    signal_state::set_in_guest(outer);
    0
}

extern "C" fn long_jump(env: *const u64, value: i32) -> ! {
    // cite: bionic setjmp.S#siglongjmp
    // cite: bionic setjmp_cookie.cpp#__bionic_setjmp_checksum_mismatch
    let outer = signal_state::set_in_guest(false);
    let (words, sum, frame) = unsafe {
        (
            env.cast::<[u64; CHECKSUM]>().read_unaligned(),
            env.add(CHECKSUM).read_unaligned(),
            env.add(FRAME).read_unaligned(),
        )
    };
    if checksum(&words) != sum {
        fatal(None, "setjmp checksum mismatch");
    }
    if words[FLAG] & 1 != 0 {
        set_signal_mask(words[MASK]);
        signal_router::deliver_pending();
        settle();
    }
    if words[FLAG] & !1 != cookie() {
        // cite: bionic setjmp_cookie.cpp#__bionic_setjmp_cookie_check
        fatal(None, "setjmp cookie mismatch");
    }
    signal_state::set_in_guest(outer);
    if !only_signal_handlers_above(frame) {
        abort_active_call("siglongjmp target is not in this guest call or a signal handler above it; next missing primitive: siglongjmp across a host call frame".to_string());
    }
    PENDING.set(Some(Jump {
        words,
        frame,
        value,
    }));
    settle();
    std::process::abort()
}

pub fn jumping() -> bool {
    PENDING.get().is_some()
}

pub fn resume_stack_pointer() -> Option<u64> {
    PENDING
        .get()
        .filter(|jump| jump.frame == current_serial())
        .map(|jump| jump.words[STACK_POINTER])
}

pub extern "C" fn settle() {
    let Some(jump) = PENDING.get() else { return };
    signal_state::set_in_guest(false);
    if jump.frame != current_serial() {
        abort_active_call("siglongjmp leaves a signal handler".to_string());
    }
    PENDING.set(None);
    signal_state::set_in_guest(true);
    unsafe { spk_longjmp_restore(jump.words.as_ptr(), jump.value) }
}

pub fn register() {
    in_libc_raw("setjmp", spk_setjmp as unsafe extern "C" fn() as usize);
    in_libc_raw(
        "_setjmp",
        spk_setjmp_unmasked as unsafe extern "C" fn() as usize,
    );
    in_libc_raw(
        "sigsetjmp",
        spk_sigsetjmp as unsafe extern "C" fn() as usize,
    );
    for name in ["siglongjmp", "longjmp", "_longjmp"] {
        in_libc_raw(
            name,
            long_jump as extern "C" fn(*const u64, i32) -> ! as usize,
        );
    }
}
