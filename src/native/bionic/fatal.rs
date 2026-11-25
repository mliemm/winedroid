use std::sync::OnceLock;

use super::super::{signal_numbers, signal_router, signal_state};
use super::export::in_libc;
use super::guest_mem::{cstr_lossy, optional_cstr};
use super::log_sink::{emit, LogRecord};

const MESSAGE_LIMIT: usize = 1023;
const LOG_FATAL: i32 = 7;

static ABORT_MESSAGE: OnceLock<String> = OnceLock::new();

pub fn set_abort_message(message: &str) {
    // cite: bionic android_set_abort_message.cpp#android_set_abort_message
    let _ = ABORT_MESSAGE.set(message.to_string());
}

pub fn abort_message() -> Option<&'static str> {
    ABORT_MESSAGE.get().map(String::as_str)
}

pub fn guest_abort() -> ! {
    // cite: bionic abort.cpp#abort
    const SIGABRT: i32 = 6;
    if signal_state::action_of(SIGABRT).has_handler() {
        signal_state::set_mask(signal_state::mask() & !signal_numbers::bit(SIGABRT));
        signal_router::apply_darwin_mask(signal_state::mask());
        signal_state::mark_thread_directed(SIGABRT);
        signal_state::raise_pending(SIGABRT);
        signal_router::deliver_pending();
        signal_state::publish_action(SIGABRT, signal_state::DEFAULT_ACTION);
        let _ = signal_router::install(SIGABRT, &signal_state::DEFAULT_ACTION);
    }
    unsafe { libc::abort() }
}

pub fn fatal(prefix: Option<&str>, message: &str) -> ! {
    // cite: bionic async_safe_log.cpp#async_safe_fatal_va_list
    let mut text = match prefix {
        Some(prefix) => format!("{prefix}: {message}"),
        None => message.to_string(),
    };
    while text.len() > MESSAGE_LIMIT {
        text.pop();
    }
    let line = format!("{text}\n");
    unsafe { libc::write(2, line.as_ptr() as *const libc::c_void, line.len()) };
    emit(LogRecord { buffer: 0, priority: LOG_FATAL, tag: "libc".to_string(), file: None, line: 0, message: text.clone() });
    set_abort_message(&text);
    guest_abort()
}

pub fn fortify_fatal(message: &str) -> ! {
    // cite: bionic bionic_fortify.h#__fortify_fatal
    fatal(Some("FORTIFY"), message)
}

pub fn register() {
    in_libc("abort", |_| guest_abort());
    in_libc("__assert", |context| {
        // cite: bionic assert.cpp#__assert
        let message = format!("{}:{}: assertion \"{}\" failed", cstr_lossy(context.arg_ptr(0)), context.arg_i32(1), cstr_lossy(context.arg_ptr(2)));
        fatal(None, &message)
    });
    in_libc("__assert2", |context| {
        // cite: bionic assert.cpp#__assert2
        let message = format!(
            "{}:{}: {}: assertion \"{}\" failed",
            cstr_lossy(context.arg_ptr(0)),
            context.arg_i32(1),
            cstr_lossy(context.arg_ptr(2)),
            cstr_lossy(context.arg_ptr(3))
        );
        fatal(None, &message)
    });
    in_libc("android_set_abort_message", |context| {
        let message = optional_cstr(context.arg_ptr(0)).map_or_else(|| "(null)".to_string(), |bytes| String::from_utf8_lossy(bytes).into_owned());
        set_abort_message(&message);
    });
    in_libc("__stack_chk_fail", |_| {
        // cite: bionic __stack_chk_fail.cpp#__stack_chk_fail
        fatal(None, "stack corruption detected (-fstack-protector)")
    });
}
