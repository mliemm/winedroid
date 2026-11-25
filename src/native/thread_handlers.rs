use std::sync::Once;

use super::bridge::{register_data, register_handler, CallContext};
use super::guest_thread::attach_current;
use super::thread_block::block;

fn thread_record(context: &mut CallContext) -> Option<*mut super::pthread_internal::PthreadInternal> {
    match attach_current() {
        Ok(record) => Some(record),
        Err(error) => {
            context.fail(error.to_string());
            None
        }
    }
}

pub fn register_thread_handlers() {
    static ONCE: Once = Once::new();
    ONCE.call_once(register_all_thread_handlers);
}

fn register_all_thread_handlers() {
    register_handler("pthread_self", |context| {
        if let Some(record) = thread_record(context) {
            context.ret_ptr(record as usize);
        }
    });
    register_handler("pthread_equal", |context| {
        let equal = context.arg_u64(0) == context.arg_u64(1);
        context.ret_i32(i32::from(equal));
    });
    register_handler("gettid", |context| {
        if let Some(record) = thread_record(context) {
            context.ret_i32(unsafe { (*record).tid });
        }
    });
    register_handler("getpid", |context| {
        if let Some(record) = thread_record(context) {
            // cite: bionic getpid.cpp#__get_cached_pid
            let cached = unsafe { (*record).cached_pid };
            let pid = if cached != 0 { cached as i32 } else { unsafe { libc::getpid() } };
            context.ret_i32(pid);
        }
    });
    register_handler("__errno", |context| {
        // cite: bionic __errno.cpp#__errno
        context.ret_ptr(unsafe { libc::__error() } as usize);
    });
    let guard = register_data("__stack_chk_guard", 8);
    unsafe { *(guard as *mut u64) = block().canary() };
}
