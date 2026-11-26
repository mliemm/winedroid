use std::sync::atomic::{AtomicU32, Ordering};

use super::super::bridge::CallContext;
use super::super::futex::{self, MATCH_ANY};
use super::export::{data_in_libstdcxx, in_libstdcxx};
use super::fatal::fatal;

const CONSTRUCTION_NOT_YET_STARTED: u32 = 0;
const CONSTRUCTION_COMPLETE: u32 = 1;
const CONSTRUCTION_UNDERWAY_WITHOUT_WAITER: u32 = 0x100;
const CONSTRUCTION_UNDERWAY_WITH_WAITER: u32 = 0x200;
const MAX_WAITERS: i32 = i32::MAX;

fn allocate(context: &mut CallContext, array: bool, nothrow: bool) {
    // cite: bionic new.cpp#operator
    let size = context.arg_u64(0) as usize;
    let pointer = unsafe { libc::malloc(size) };
    if pointer.is_null() && !nothrow {
        fatal(
            None,
            &format!(
                "new{} failed to allocate {size} bytes",
                if array { "[]" } else { "" }
            ),
        );
    }
    context.ret_ptr(pointer as usize);
}

fn guard(context: &CallContext) -> &'static AtomicU32 {
    unsafe { &*(context.arg_ptr(0) as *const AtomicU32) }
}

fn acquire(context: &mut CallContext) {
    // cite: bionic __cxa_guard.cpp#__cxa_guard_acquire
    let state = guard(context);
    let mut current = state.load(Ordering::Acquire);
    loop {
        match current {
            CONSTRUCTION_COMPLETE => return context.ret_i32(0),
            CONSTRUCTION_NOT_YET_STARTED => {
                match state.compare_exchange_weak(
                    current,
                    CONSTRUCTION_UNDERWAY_WITHOUT_WAITER,
                    Ordering::Acquire,
                    Ordering::Acquire,
                ) {
                    Ok(_) => return context.ret_i32(1),
                    Err(actual) => {
                        current = actual;
                        continue;
                    }
                }
            }
            CONSTRUCTION_UNDERWAY_WITHOUT_WAITER => {
                match state.compare_exchange_weak(
                    current,
                    CONSTRUCTION_UNDERWAY_WITH_WAITER,
                    Ordering::Acquire,
                    Ordering::Acquire,
                ) {
                    Ok(_) => {}
                    Err(actual) => {
                        current = actual;
                        continue;
                    }
                }
            }
            _ => {}
        }
        futex::wait(
            context.arg_ptr(0),
            CONSTRUCTION_UNDERWAY_WITH_WAITER,
            MATCH_ANY,
            None,
        );
        current = state.load(Ordering::Acquire);
    }
}

fn release(context: &mut CallContext, final_state: u32) {
    // cite: bionic __cxa_guard.cpp#__cxa_guard_release, __cxa_guard_abort
    let previous = guard(context).swap(final_state, Ordering::Release);
    if previous == CONSTRUCTION_UNDERWAY_WITH_WAITER {
        futex::wake(context.arg_ptr(0), MAX_WAITERS, MATCH_ANY);
    }
}

pub fn register() {
    data_in_libstdcxx("_ZSt7nothrow", 1);
    for (name, array, nothrow) in [
        ("_Znwm", false, false),
        ("_Znam", true, false),
        ("_ZnwmRKSt9nothrow_t", false, true),
        ("_ZnamRKSt9nothrow_t", true, true),
    ] {
        in_libstdcxx(name, move |context| allocate(context, array, nothrow));
    }
    for name in [
        "_ZdlPv",
        "_ZdaPv",
        "_ZdlPvRKSt9nothrow_t",
        "_ZdaPvRKSt9nothrow_t",
    ] {
        in_libstdcxx(name, |context| {
            // cite: bionic new.cpp#operator
            unsafe { libc::free(context.arg_ptr(0) as *mut libc::c_void) };
        });
    }
    in_libstdcxx("__cxa_guard_acquire", acquire);
    in_libstdcxx("__cxa_guard_release", |context| {
        release(context, CONSTRUCTION_COMPLETE)
    });
    in_libstdcxx("__cxa_guard_abort", |context| {
        release(context, CONSTRUCTION_NOT_YET_STARTED)
    });
    in_libstdcxx("__cxa_pure_virtual", |_| {
        // cite: bionic __cxa_pure_virtual.cpp#__cxa_pure_virtual
        fatal(
            None,
            "Pure virtual function called. Are you calling virtual methods from a destructor?",
        )
    });
}
