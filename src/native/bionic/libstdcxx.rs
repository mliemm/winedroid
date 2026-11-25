use std::sync::atomic::{AtomicU32, Ordering};

use super::super::bridge::CallContext;
use super::export::{data_in_libstdcxx, in_libstdcxx};
use super::fatal::fatal;

const CONSTRUCTION_NOT_YET_STARTED: u32 = 0;
const CONSTRUCTION_COMPLETE: u32 = 1;
const CONSTRUCTION_UNDERWAY: u32 = 0x100;

fn allocate(context: &mut CallContext, array: bool, nothrow: bool) {
    // cite: bionic new.cpp#operator
    let size = context.arg_u64(0) as usize;
    let pointer = unsafe { libc::malloc(size) };
    if pointer.is_null() && !nothrow {
        fatal(None, &format!("new{} failed to allocate {size} bytes", if array { "[]" } else { "" }));
    }
    context.ret_ptr(pointer as usize);
}

fn guard(context: &CallContext) -> &'static AtomicU32 {
    unsafe { &*(context.arg_ptr(0) as *const AtomicU32) }
}

fn acquire(context: &mut CallContext) {
    // cite: bionic __cxa_guard.cpp#__cxa_guard_acquire
    let state = guard(context);
    loop {
        match state.compare_exchange(CONSTRUCTION_NOT_YET_STARTED, CONSTRUCTION_UNDERWAY, Ordering::Acquire, Ordering::Acquire) {
            Ok(_) => return context.ret_i32(1),
            Err(CONSTRUCTION_COMPLETE) => return context.ret_i32(0),
            Err(_) => std::thread::yield_now(),
        }
    }
}

pub fn register() {
    data_in_libstdcxx("_ZSt7nothrow", 1);
    for (name, array, nothrow) in [("_Znwm", false, false), ("_Znam", true, false), ("_ZnwmRKSt9nothrow_t", false, true), ("_ZnamRKSt9nothrow_t", true, true)] {
        in_libstdcxx(name, move |context| allocate(context, array, nothrow));
    }
    for name in ["_ZdlPv", "_ZdaPv", "_ZdlPvRKSt9nothrow_t", "_ZdaPvRKSt9nothrow_t"] {
        in_libstdcxx(name, |context| {
            // cite: bionic new.cpp#operator
            unsafe { libc::free(context.arg_ptr(0) as *mut libc::c_void) };
        });
    }
    in_libstdcxx("__cxa_guard_acquire", acquire);
    in_libstdcxx("__cxa_guard_release", |context| {
        // cite: bionic __cxa_guard.cpp#__cxa_guard_release
        guard(context).store(CONSTRUCTION_COMPLETE, Ordering::Release);
    });
    in_libstdcxx("__cxa_guard_abort", |context| {
        // cite: bionic __cxa_guard.cpp#__cxa_guard_abort
        guard(context).store(CONSTRUCTION_NOT_YET_STARTED, Ordering::Release);
    });
    in_libstdcxx("__cxa_pure_virtual", |_| {
        // cite: bionic __cxa_pure_virtual.cpp#__cxa_pure_virtual
        fatal(None, "Pure virtual function called. Are you calling virtual methods from a destructor?")
    });
}
