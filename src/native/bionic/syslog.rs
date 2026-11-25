use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

use super::env::program_name;
use super::export::in_libc;
use super::guest_mem::cstr_bytes;
use super::log_sink::{emit, LogRecord};
use super::printf::format;
use super::super::bridge::CallContext;

const LOG_PRIMASK: i32 = 7;
const LOG_ERR: i32 = 3;
const LOG_WARNING: i32 = 4;
const LOG_INFO: i32 = 6;
const ANDROID_LOG_DEBUG: i32 = 3;
const ANDROID_LOG_INFO: i32 = 4;
const ANDROID_LOG_WARN: i32 = 5;
const ANDROID_LOG_ERROR: i32 = 6;
const DEFAULT_MASK: i32 = 0xff;
const LINE_LIMIT: usize = 1023;

static TAG: AtomicUsize = AtomicUsize::new(0);
static MASK: AtomicI32 = AtomicI32::new(DEFAULT_MASK);

fn emit_line(priority: i32, text: &[u8]) {
    // cite: bionic syslog.cpp#vsyslog
    if MASK.load(Ordering::SeqCst) & (1 << (priority & LOG_PRIMASK)) == 0 {
        return;
    }
    let stored = TAG.load(Ordering::SeqCst);
    let tag = if stored == 0 { program_name() } else { String::from_utf8_lossy(cstr_bytes(stored)).into_owned() };
    let priority = priority & LOG_PRIMASK;
    let android = if priority <= LOG_ERR {
        ANDROID_LOG_ERROR
    } else if priority == LOG_WARNING {
        ANDROID_LOG_WARN
    } else if priority <= LOG_INFO {
        ANDROID_LOG_INFO
    } else {
        ANDROID_LOG_DEBUG
    };
    let mut message = String::from_utf8_lossy(text).into_owned();
    message.truncate(LINE_LIMIT);
    emit(LogRecord { buffer: 0, priority: android, tag, file: None, line: 0, message });
}

fn syslog(context: &mut CallContext) {
    // cite: bionic syslog.cpp#syslog
    let priority = context.arg_i32(0);
    let mut args = context.varargs(2, 0);
    match format(cstr_bytes(context.arg_ptr(1)), &mut args) {
        Ok(text) => emit_line(priority, &text),
        Err(message) => context.fail(format!("{} {message}", context.symbol())),
    }
}

fn vsyslog(context: &mut CallContext) {
    // cite: bionic syslog.cpp#vsyslog
    let priority = context.arg_i32(0);
    match context.va_list_arg(2) {
        Ok(mut args) => match format(cstr_bytes(context.arg_ptr(1)), &mut args) {
            Ok(text) => emit_line(priority, &text),
            Err(message) => context.fail(format!("{} {message}", context.symbol())),
        },
        Err(error) => context.fail(error.to_string()),
    }
}

pub fn register() {
    in_libc("openlog", |context| {
        // cite: bionic syslog.cpp#openlog
        TAG.store(context.arg_ptr(0), Ordering::SeqCst);
    });
    in_libc("closelog", |_| {
        // cite: bionic syslog.cpp#closelog
        TAG.store(0, Ordering::SeqCst);
    });
    in_libc("setlogmask", |context| {
        // cite: bionic syslog.cpp#setlogmask
        let requested = context.arg_i32(0);
        let previous = MASK.load(Ordering::SeqCst);
        if requested != 0 {
            MASK.store(requested, Ordering::SeqCst);
        }
        context.ret_i32(previous);
    });
    in_libc("syslog", syslog);
    in_libc("vsyslog", vsyslog);
}
