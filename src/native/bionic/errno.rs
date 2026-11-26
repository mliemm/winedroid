use std::cell::RefCell;
use std::ffi::CString;
use std::sync::OnceLock;

use super::super::bridge::CallContext;
use super::abi_types::ERANGE;
use super::export::in_libc;
use super::guest_mem::write_cstr_truncated;
use super::strerror_table::{COUNT, TABLE};

const STRERROR_BUFFER: usize = 255;

static NAMES: OnceLock<Vec<Option<CString>>> = OnceLock::new();

thread_local! {
    static BUFFER: RefCell<[u8; STRERROR_BUFFER + 1]> = const { RefCell::new([0; STRERROR_BUFFER + 1]) };
}

pub fn set_guest_errno(linux_code: i32) {
    unsafe { *libc::__error() = linux_code };
}

pub fn guest_errno() -> i32 {
    unsafe { *libc::__error() }
}

pub fn message(error: i32) -> Option<&'static str> {
    // cite: bionic strerror.cpp#__sys_error_strings
    usize::try_from(error)
        .ok()
        .filter(|index| *index < COUNT)
        .and_then(|index| TABLE[index])
}

fn unknown(error: i32) -> String {
    format!("Unknown error {error}")
}

fn text_for(error: i32) -> String {
    message(error)
        .map(str::to_string)
        .unwrap_or_else(|| unknown(error))
}

fn names() -> &'static Vec<Option<CString>> {
    NAMES.get_or_init(|| {
        TABLE
            .iter()
            .map(|entry| entry.map(|text| CString::new(text).expect("no interior nul")))
            .collect()
    })
}

fn strerror_r(error: i32, buffer: usize, length: usize) -> i32 {
    // cite: bionic strerror.cpp#strerror_r
    let text = text_for(error);
    let needed = write_cstr_truncated(buffer, length, text.as_bytes());
    if needed >= length {
        ERANGE
    } else {
        0
    }
}

fn strerror(error: i32) -> usize {
    // cite: bionic strerror.cpp#strerror
    if message(error).is_some() {
        return names()[error as usize]
            .as_ref()
            .map_or(0, |name| name.as_ptr() as usize);
    }
    BUFFER.with(|buffer| {
        let mut buffer = buffer.borrow_mut();
        let text = unknown(error);
        let address = buffer.as_mut_ptr() as usize;
        write_cstr_truncated(address, STRERROR_BUFFER, text.as_bytes());
        address
    })
}

fn handle_strerror_r(context: &mut CallContext) {
    let result = strerror_r(
        context.arg_i32(0),
        context.arg_ptr(1),
        context.arg_u64(2) as usize,
    );
    context.ret_i32(result);
}

pub fn register() {
    in_libc("strerror", |context| {
        context.ret_ptr(strerror(context.arg_i32(0)))
    });
    in_libc("strerror_l", |context| {
        context.ret_ptr(strerror(context.arg_i32(0)))
    });
    in_libc("strerror_r", handle_strerror_r);
    in_libc("__gnu_strerror_r", |context| {
        // cite: bionic strerror.cpp#__gnu_strerror_r
        strerror_r(
            context.arg_i32(0),
            context.arg_ptr(1),
            context.arg_u64(2) as usize,
        );
        context.ret_ptr(context.arg_ptr(1));
    });
}
