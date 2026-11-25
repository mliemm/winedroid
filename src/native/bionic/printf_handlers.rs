use super::super::bridge::CallContext;
use super::super::va_list::GuestVaList;
use super::abi_types::SSIZE_MAX;
use super::export::in_libc;
use super::fortify_check::check_access;
use super::guest_mem::{cstr_bytes, write_cstr_truncated};
use super::printf::format;

fn render(context: &mut CallContext, format_pointer: usize, mut args: GuestVaList) -> Option<Vec<u8>> {
    match format(cstr_bytes(format_pointer), &mut args) {
        Ok(text) => Some(text),
        Err(message) => {
            context.fail(format!("{} {message}", context.symbol()));
            None
        }
    }
}

fn into_bounded(context: &mut CallContext, destination: usize, capacity: u64, format_pointer: usize, args: GuestVaList) {
    // cite: bionic stdio.cpp#vsnprintf
    if let Some(text) = render(context, format_pointer, args) {
        write_cstr_truncated(destination, capacity as usize, &text);
        context.ret_i32(text.len() as i32);
    }
}

fn into_unbounded(context: &mut CallContext, destination: usize, format_pointer: usize, args: GuestVaList) {
    if let Some(text) = render(context, format_pointer, args) {
        write_cstr_truncated(destination, text.len() + 1, &text);
        context.ret_i32(text.len() as i32);
    }
}

fn checked_unbounded(context: &mut CallContext, destination: usize, compiler_size: u64, format_pointer: usize, args: GuestVaList) {
    // cite: bionic fortify.cpp#__vsprintf_chk
    let Some(text) = render(context, format_pointer, args) else { return };
    let limit = if compiler_size == u64::MAX { SSIZE_MAX } else { compiler_size };
    write_cstr_truncated(destination, text.len() + 1, &text);
    check_access("vsprintf", "write into", text.len() as u64 + 1, limit);
    context.ret_i32(text.len() as i32);
}

fn guest_list(context: &mut CallContext, index: usize) -> Option<GuestVaList<'static>> {
    match context.va_list_arg(index) {
        Ok(list) => Some(list),
        Err(error) => {
            context.fail(error.to_string());
            None
        }
    }
}

fn register_formatted() {
    in_libc("snprintf", |context| {
        let regs = *context.regs();
        let args = GuestVaList::from_regs(&regs, 3, 0);
        into_bounded(context, context.arg_ptr(0), context.arg_u64(1), context.arg_ptr(2), args);
    });
    in_libc("sprintf", |context| {
        let regs = *context.regs();
        let args = GuestVaList::from_regs(&regs, 2, 0);
        into_unbounded(context, context.arg_ptr(0), context.arg_ptr(1), args);
    });
    in_libc("vsnprintf", |context| {
        if let Some(args) = guest_list(context, 3) {
            into_bounded(context, context.arg_ptr(0), context.arg_u64(1), context.arg_ptr(2), args);
        }
    });
    in_libc("vsprintf", |context| {
        if let Some(args) = guest_list(context, 2) {
            into_unbounded(context, context.arg_ptr(0), context.arg_ptr(1), args);
        }
    });
}

fn register_checked() {
    in_libc("__vsnprintf_chk", |context| {
        // cite: bionic fortify.cpp#__vsnprintf_chk
        check_access("vsnprintf", "write into", context.arg_u64(1), context.arg_u64(3));
        if let Some(args) = guest_list(context, 5) {
            into_bounded(context, context.arg_ptr(0), context.arg_u64(1), context.arg_ptr(4), args);
        }
    });
    in_libc("__snprintf_chk", |context| {
        check_access("vsnprintf", "write into", context.arg_u64(1), context.arg_u64(3));
        let regs = *context.regs();
        let args = GuestVaList::from_regs(&regs, 5, 0);
        into_bounded(context, context.arg_ptr(0), context.arg_u64(1), context.arg_ptr(4), args);
    });
    in_libc("__vsprintf_chk", |context| {
        if let Some(args) = guest_list(context, 4) {
            checked_unbounded(context, context.arg_ptr(0), context.arg_u64(2), context.arg_ptr(3), args);
        }
    });
    in_libc("__sprintf_chk", |context| {
        let regs = *context.regs();
        let args = GuestVaList::from_regs(&regs, 4, 0);
        checked_unbounded(context, context.arg_ptr(0), context.arg_u64(2), context.arg_ptr(3), args);
    });
}

pub fn register() {
    register_formatted();
    register_checked();
}
