use std::ffi::CString;
use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};
use std::sync::Mutex;

use super::super::bridge::CallContext;
use super::super::call_assign::CallBuilder;
use super::super::va_list::GuestVaList;
use super::abi_types::LOG_MESSAGE_SIZE;
use super::env::program_name;
use super::export::in_liblog;
use super::fatal::{guest_abort, set_abort_message};
use super::guest_mem::{cstr_bytes, cstr_lossy, optional_cstr, read_value};
use super::log_sink::{emit, write_stderr, LogRecord};
use super::printf::format;
use super::props::get as property;

const PRIORITY_DEFAULT: i32 = 1;
const PRIORITY_VERBOSE: i32 = 2;
const PRIORITY_FATAL: i32 = 7;
const BUFFER_DEFAULT: i32 = 0x7FFF_FFFF;
const FORMAT_LIMIT: usize = 1023;

static LOGGER: AtomicUsize = AtomicUsize::new(0);
static ABORTER: AtomicUsize = AtomicUsize::new(0);
static MINIMUM: AtomicI32 = AtomicI32::new(PRIORITY_DEFAULT);
static DEFAULT_TAG: Mutex<Option<String>> = Mutex::new(None);

fn tag_priority(tag: &[u8]) -> Option<i32> {
    // cite: bionic async_safe_log.cpp#async_safe_write_log
    let tag = String::from_utf8_lossy(tag).into_owned();
    let lookups = [format!("log.tag.{tag}"), format!("persist.log.tag.{tag}"), "log.tag".to_string(), "persist.log.tag".to_string()];
    lookups.iter().find_map(|key| property(key)).and_then(|value| {
        let letter = value.bytes().next()?.to_ascii_uppercase();
        "XXVDIWEFS".bytes().position(|candidate| candidate == letter).map(|index| index as i32)
    })
}

pub fn is_loggable(priority: i32, tag: Option<&[u8]>, default_priority: i32) -> bool {
    let minimum = MINIMUM.load(Ordering::SeqCst);
    let set_minimum = (minimum != PRIORITY_DEFAULT).then_some(minimum);
    let from_tag = tag.and_then(tag_priority);
    let effective = match (set_minimum, from_tag) {
        (Some(left), Some(right)) => left.min(right),
        (Some(value), None) | (None, Some(value)) => value,
        (None, None) => default_priority,
    };
    priority >= effective
}

fn default_tag() -> String {
    DEFAULT_TAG.lock().expect("default tag").clone().unwrap_or_else(program_name)
}

fn buffer_is_writable(buffer: i32) -> bool {
    matches!(buffer, 0 | 1 | 3 | 4 | BUFFER_DEFAULT)
}

fn call_logger(record: &LogRecord) -> Result<(), String> {
    let logger = LOGGER.load(Ordering::SeqCst);
    if logger == 0 {
        emit(record.clone());
        return Ok(());
    }
    let tag = CString::new(record.tag.clone()).unwrap_or_default();
    let message = CString::new(record.message.clone()).unwrap_or_default();
    let mut block = [0u8; LOG_MESSAGE_SIZE];
    block[..8].copy_from_slice(&(LOG_MESSAGE_SIZE as u64).to_le_bytes());
    block[8..12].copy_from_slice(&record.buffer.to_le_bytes());
    block[12..16].copy_from_slice(&record.priority.to_le_bytes());
    block[16..24].copy_from_slice(&(tag.as_ptr() as u64).to_le_bytes());
    block[32..36].copy_from_slice(&record.line.to_le_bytes());
    block[40..48].copy_from_slice(&(message.as_ptr() as u64).to_le_bytes());
    CallBuilder::new(logger).ptr(block.as_ptr() as usize).invoke().map(|_| ()).map_err(|error| error.to_string())
}

fn write_record(buffer: i32, priority: i32, tag: Option<&[u8]>, text: &[u8]) -> Result<(), String> {
    // cite: bionic async_safe_log.cpp#async_safe_write_log
    if !buffer_is_writable(buffer) {
        return Ok(());
    }
    let tag = tag.map_or_else(default_tag, |bytes| String::from_utf8_lossy(bytes).into_owned());
    let message = String::from_utf8_lossy(text).into_owned();
    if priority == PRIORITY_FATAL {
        set_abort_message(&message);
    }
    let buffer = if buffer == BUFFER_DEFAULT { 0 } else { buffer };
    call_logger(&LogRecord { buffer, priority, tag, file: None, line: 0, message })
}

fn simple_write(context: &mut CallContext, buffer: i32, priority: i32, tag: usize, text: usize) {
    if !is_loggable(priority, optional_cstr(tag), PRIORITY_VERBOSE) {
        context.ret_i32(-1);
        return;
    }
    match write_record(buffer, priority, optional_cstr(tag), optional_cstr(text).unwrap_or(b"")) {
        Ok(()) => context.ret_i32(1),
        Err(message) => context.fail(message),
    }
}

fn formatted_write(context: &mut CallContext, buffer: i32, priority: i32, tag: usize, format_pointer: usize, mut args: GuestVaList) {
    if !is_loggable(priority, optional_cstr(tag), PRIORITY_VERBOSE) {
        context.ret_i32(-1);
        return;
    }
    let mut text = match format(cstr_bytes(format_pointer), &mut args) {
        Ok(text) => text,
        Err(message) => return context.fail(format!("{} {message}", context.symbol())),
    };
    text.truncate(FORMAT_LIMIT);
    match write_record(buffer, priority, optional_cstr(tag), &text) {
        Ok(()) => context.ret_i32(1),
        Err(message) => context.fail(message),
    }
}

fn assert_fatal(context: &mut CallContext) {
    // cite: bionic async_safe_log.cpp#async_safe_fatal_va_list
    let regs = *context.regs();
    let (condition, tag, format_pointer) = (context.arg_ptr(0), context.arg_ptr(1), context.arg_ptr(2));
    let text = if format_pointer != 0 {
        let mut args = GuestVaList::from_regs(&regs, 3, 0);
        let mut rendered = match format(cstr_bytes(format_pointer), &mut args) {
            Ok(text) => text,
            Err(message) => return context.fail(format!("{} {message}", context.symbol())),
        };
        rendered.truncate(FORMAT_LIMIT);
        rendered
    } else if condition != 0 {
        format!("Assertion failed: {}", cstr_lossy(condition)).into_bytes()
    } else {
        b"Unspecified assertion failed".to_vec()
    };
    let mut line = text.clone();
    line.push(b'\n');
    unsafe { libc::write(2, line.as_ptr() as *const libc::c_void, line.len()) };
    if let Err(message) = write_record(0, PRIORITY_FATAL, optional_cstr(tag), &text) {
        return context.fail(message);
    }
    let aborter = ABORTER.load(Ordering::SeqCst);
    if aborter != 0 {
        let message = CString::new(text).unwrap_or_default();
        if let Err(error) = CallBuilder::new(aborter).ptr(message.as_ptr() as usize).invoke() {
            return context.fail(error.to_string());
        }
    } else {
        set_abort_message(&String::from_utf8_lossy(&text));
    }
    guest_abort()
}

fn record_from_message(address: usize) -> LogRecord {
    let pointer = |offset: usize| read_value::<usize>(address + offset);
    LogRecord {
        buffer: read_value::<i32>(address + 8),
        priority: read_value::<i32>(address + 12),
        tag: if pointer(16) == 0 { "nullptr".to_string() } else { cstr_lossy(pointer(16)) },
        file: (pointer(24) != 0).then(|| cstr_lossy(pointer(24))),
        line: read_value::<u32>(address + 32),
        message: cstr_lossy(pointer(40)),
    }
}

fn register_writers() {
    in_liblog("__android_log_write", |context| simple_write(context, 0, context.arg_i32(0), context.arg_ptr(1), context.arg_ptr(2)));
    in_liblog("__android_log_buf_write", |context| simple_write(context, context.arg_i32(0), context.arg_i32(1), context.arg_ptr(2), context.arg_ptr(3)));
    in_liblog("__android_log_print", |context| {
        let regs = *context.regs();
        let args = GuestVaList::from_regs(&regs, 3, 0);
        formatted_write(context, 0, context.arg_i32(0), context.arg_ptr(1), context.arg_ptr(2), args);
    });
    in_liblog("__android_log_buf_print", |context| {
        let regs = *context.regs();
        let args = GuestVaList::from_regs(&regs, 4, 0);
        formatted_write(context, context.arg_i32(0), context.arg_i32(1), context.arg_ptr(2), context.arg_ptr(3), args);
    });
    in_liblog("__android_log_vprint", |context| match context.va_list_arg(3) {
        Ok(args) => formatted_write(context, 0, context.arg_i32(0), context.arg_ptr(1), context.arg_ptr(2), args),
        Err(error) => context.fail(error.to_string()),
    });
    in_liblog("__android_log_assert", assert_fatal);
    in_liblog("__android_log_write_log_message", |context| {
        let record = record_from_message(context.arg_ptr(0));
        let tag = (record.tag != "nullptr").then(|| record.tag.clone());
        match write_record(record.buffer, record.priority, tag.as_deref().map(str::as_bytes), record.message.as_bytes()) {
            Ok(()) => {}
            Err(message) => context.fail(message),
        }
    });
}

fn register_configuration() {
    in_liblog("__android_log_set_logger", |context| LOGGER.store(context.arg_ptr(0), Ordering::SeqCst));
    in_liblog("__android_log_logd_logger", |context| emit(record_from_message(context.arg_ptr(0))));
    in_liblog("__android_log_stderr_logger", |context| write_stderr(&record_from_message(context.arg_ptr(0))));
    in_liblog("__android_log_set_aborter", |context| ABORTER.store(context.arg_ptr(0), Ordering::SeqCst));
    in_liblog("__android_log_call_aborter", |context| {
        let aborter = ABORTER.load(Ordering::SeqCst);
        if aborter == 0 {
            set_abort_message(&cstr_lossy(context.arg_ptr(0)));
            guest_abort();
        }
        if let Err(error) = CallBuilder::new(aborter).ptr(context.arg_ptr(0)).invoke() {
            context.fail(error.to_string());
        }
    });
    in_liblog("__android_log_default_aborter", |context| {
        set_abort_message(&cstr_lossy(context.arg_ptr(0)));
        guest_abort()
    });
    in_liblog("__android_log_is_loggable", |context| {
        let loggable = is_loggable(context.arg_i32(0), optional_cstr(context.arg_ptr(1)), context.arg_i32(2));
        context.ret_i32(i32::from(loggable));
    });
    in_liblog("__android_log_is_loggable_len", |context| {
        let tag = (context.arg_ptr(1) != 0).then(|| unsafe { std::slice::from_raw_parts(context.arg_ptr(1) as *const u8, context.arg_u64(2) as usize) });
        context.ret_i32(i32::from(is_loggable(context.arg_i32(0), tag, context.arg_i32(3))));
    });
    in_liblog("__android_log_set_minimum_priority", |context| context.ret_i32(MINIMUM.swap(context.arg_i32(0), Ordering::SeqCst)));
    in_liblog("__android_log_get_minimum_priority", |context| context.ret_i32(MINIMUM.load(Ordering::SeqCst)));
    in_liblog("__android_log_set_default_tag", |context| {
        let tag = optional_cstr(context.arg_ptr(0)).map(|bytes| String::from_utf8_lossy(bytes).into_owned());
        *DEFAULT_TAG.lock().expect("default tag") = tag;
    });
}

pub fn register() {
    register_writers();
    register_configuration();
}
