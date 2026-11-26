use super::super::bridge::CallContext;
use super::super::va_list::GuestVaList;
use super::abi_types::SSIZE_MAX;
use super::export::in_libc;
use super::fortify_check::check_access;
use super::guest_mem::{cstr_bytes, read_value, write_cstr_truncated, write_value};
use super::printf::format;
use super::stdio_files;
use super::stream;

fn render(
    context: &mut CallContext,
    format_pointer: usize,
    mut args: GuestVaList,
) -> Option<Vec<u8>> {
    match format(cstr_bytes(format_pointer), &mut args) {
        Ok(text) => Some(text),
        Err(message) => {
            context.fail(format!("{} {message}", context.symbol()));
            None
        }
    }
}

fn into_bounded(
    context: &mut CallContext,
    destination: usize,
    capacity: u64,
    format_pointer: usize,
    args: GuestVaList,
) {
    // cite: bionic stdio.cpp#vsnprintf
    if let Some(text) = render(context, format_pointer, args) {
        write_cstr_truncated(destination, capacity as usize, &text);
        context.ret_i32(text.len() as i32);
    }
}

fn into_unbounded(
    context: &mut CallContext,
    destination: usize,
    format_pointer: usize,
    args: GuestVaList,
) {
    if let Some(text) = render(context, format_pointer, args) {
        write_cstr_truncated(destination, text.len() + 1, &text);
        context.ret_i32(text.len() as i32);
    }
}

fn checked_unbounded(
    context: &mut CallContext,
    destination: usize,
    compiler_size: u64,
    format_pointer: usize,
    args: GuestVaList,
) {
    // cite: bionic fortify.cpp#__vsprintf_chk
    let Some(text) = render(context, format_pointer, args) else {
        return;
    };
    let limit = if compiler_size == u64::MAX {
        SSIZE_MAX
    } else {
        compiler_size
    };
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

fn into_stream(context: &mut CallContext, file: usize, format_pointer: usize, args: GuestVaList) {
    let stream = match stream::of(file) {
        Ok(stream) => stream,
        Err(errno) => {
            context.set_errno(errno);
            context.ret_i32(-1);
            return;
        }
    };
    let Some(text) = render(context, format_pointer, args) else {
        return;
    };
    match stream::write_bytes(&stream, text.as_ptr() as usize, text.len()) {
        Ok(written) if written == text.len() => context.ret_i32(text.len() as i32),
        Ok(_) => {
            context.set_errno(super::abi_types::EIO);
            context.ret_i32(-1);
        }
        Err(errno) => {
            context.set_errno(errno);
            context.ret_i32(-1);
        }
    }
}

fn into_descriptor(
    context: &mut CallContext,
    descriptor: i32,
    format_pointer: usize,
    args: GuestVaList,
) {
    let Some(text) = render(context, format_pointer, args) else {
        return;
    };
    let mut done = 0usize;
    while done < text.len() {
        let result =
            super::fs_io::write_fd(descriptor, text.as_ptr() as usize + done, text.len() - done);
        if result < 0 {
            context.set_errno((-result) as i32);
            context.ret_i32(-1);
            return;
        }
        done += result as usize;
    }
    context.ret_i32(text.len() as i32);
}

fn into_allocated(
    context: &mut CallContext,
    out_pointer: usize,
    format_pointer: usize,
    args: GuestVaList,
) {
    if out_pointer == 0 {
        context.set_errno(super::abi_types::EFAULT);
        context.ret_i32(-1);
        return;
    }
    write_value::<usize>(out_pointer, 0);
    let Some(text) = render(context, format_pointer, args) else {
        return;
    };
    let allocation = unsafe { libc::malloc(text.len() + 1) } as usize;
    if allocation == 0 {
        context.set_errno(super::abi_types::ENOMEM);
        context.ret_i32(-1);
        return;
    }
    write_cstr_truncated(allocation, text.len() + 1, &text);
    write_value::<usize>(out_pointer, allocation);
    context.ret_i32(text.len() as i32);
}

fn wide_format(pointer: usize) -> Vec<u8> {
    let mut out = Vec::new();
    if pointer == 0 {
        return out;
    }
    let mut offset = 0usize;
    loop {
        let code = read_value::<u32>(pointer + offset);
        if code == 0 {
            break;
        }
        out.extend(
            char::from_u32(code)
                .unwrap_or('\u{fffd}')
                .to_string()
                .bytes(),
        );
        offset += 4;
    }
    out
}

fn wide_output(bytes: &[u8]) -> Vec<u32> {
    String::from_utf8_lossy(bytes)
        .chars()
        .map(|character| character as u32)
        .collect()
}

fn render_wide(
    context: &mut CallContext,
    format_pointer: usize,
    mut args: GuestVaList,
) -> Option<Vec<u8>> {
    let format = wide_format(format_pointer);
    match format_text(&format, &mut args) {
        Ok(text) => Some(text),
        Err(message) => {
            context.fail(format!("{} {message}", context.symbol()));
            None
        }
    }
}

fn format_text(format: &[u8], args: &mut GuestVaList) -> Result<Vec<u8>, String> {
    super::printf::format(format, args)
}

fn wide_into_stream(
    context: &mut CallContext,
    file: usize,
    format_pointer: usize,
    args: GuestVaList,
) {
    let Some(text) = render_wide(context, format_pointer, args) else {
        return;
    };
    let stream = match stream::of(file) {
        Ok(stream) => stream,
        Err(errno) => {
            context.set_errno(errno);
            context.ret_i32(-1);
            return;
        }
    };
    match stream::write_bytes(&stream, text.as_ptr() as usize, text.len()) {
        Ok(written) if written == text.len() => context.ret_i32(wide_output(&text).len() as i32),
        Ok(_) => {
            context.set_errno(super::abi_types::EIO);
            context.ret_i32(-1);
        }
        Err(errno) => {
            context.set_errno(errno);
            context.ret_i32(-1);
        }
    }
}

fn wide_bounded(
    context: &mut CallContext,
    destination: usize,
    capacity: usize,
    format_pointer: usize,
    args: GuestVaList,
) {
    let Some(text) = render_wide(context, format_pointer, args) else {
        return;
    };
    let units = wide_output(&text);
    if capacity == 0 {
        context.ret_i32(units.len() as i32);
        return;
    }
    if units.len() >= capacity {
        write_value::<u32>(destination, 0);
        context.ret_i32(-1);
        return;
    }
    for (index, unit) in units.iter().enumerate() {
        write_value::<u32>(destination + index * 4, *unit);
    }
    write_value::<u32>(destination + units.len() * 4, 0);
    context.ret_i32(units.len() as i32);
}

fn register_formatted() {
    in_libc("snprintf", |context| {
        let regs = *context.regs();
        let args = GuestVaList::from_regs(&regs, 3, 0);
        into_bounded(
            context,
            context.arg_ptr(0),
            context.arg_u64(1),
            context.arg_ptr(2),
            args,
        );
    });
    in_libc("sprintf", |context| {
        let regs = *context.regs();
        let args = GuestVaList::from_regs(&regs, 2, 0);
        into_unbounded(context, context.arg_ptr(0), context.arg_ptr(1), args);
    });
    in_libc("vsnprintf", |context| {
        if let Some(args) = guest_list(context, 3) {
            into_bounded(
                context,
                context.arg_ptr(0),
                context.arg_u64(1),
                context.arg_ptr(2),
                args,
            );
        }
    });
    in_libc("vsprintf", |context| {
        if let Some(args) = guest_list(context, 2) {
            into_unbounded(context, context.arg_ptr(0), context.arg_ptr(1), args);
        }
    });
    in_libc("fprintf", |context| {
        let file = context.arg_ptr(0);
        let regs = *context.regs();
        let args = GuestVaList::from_regs(&regs, 2, 0);
        into_stream(context, file, context.arg_ptr(1), args);
    });
    in_libc("vfprintf", |context| {
        let file = context.arg_ptr(0);
        if let Some(args) = guest_list(context, 2) {
            into_stream(context, file, context.arg_ptr(1), args);
        }
    });
    in_libc("printf", |context| {
        let regs = *context.regs();
        let args = GuestVaList::from_regs(&regs, 1, 0);
        into_stream(context, stdio_files::standard(1), context.arg_ptr(0), args);
    });
    in_libc("vprintf", |context| {
        if let Some(args) = guest_list(context, 1) {
            into_stream(context, stdio_files::standard(1), context.arg_ptr(0), args);
        }
    });
    in_libc("dprintf", |context| {
        let regs = *context.regs();
        let args = GuestVaList::from_regs(&regs, 2, 0);
        into_descriptor(context, context.arg_i32(0), context.arg_ptr(1), args);
    });
    in_libc("vdprintf", |context| {
        if let Some(args) = guest_list(context, 2) {
            into_descriptor(context, context.arg_i32(0), context.arg_ptr(1), args);
        }
    });
    in_libc("asprintf", |context| {
        let regs = *context.regs();
        let args = GuestVaList::from_regs(&regs, 2, 0);
        into_allocated(context, context.arg_ptr(0), context.arg_ptr(1), args);
    });
    in_libc("vasprintf", |context| {
        if let Some(args) = guest_list(context, 2) {
            into_allocated(context, context.arg_ptr(0), context.arg_ptr(1), args);
        }
    });
    in_libc("swprintf", |context| {
        let regs = *context.regs();
        let args = GuestVaList::from_regs(&regs, 3, 0);
        wide_bounded(
            context,
            context.arg_ptr(0),
            context.arg_u64(1) as usize,
            context.arg_ptr(2),
            args,
        );
    });
    in_libc("vswprintf", |context| {
        if let Some(args) = guest_list(context, 3) {
            wide_bounded(
                context,
                context.arg_ptr(0),
                context.arg_u64(1) as usize,
                context.arg_ptr(2),
                args,
            );
        }
    });
    in_libc("fwprintf", |context| {
        let file = context.arg_ptr(0);
        let regs = *context.regs();
        let args = GuestVaList::from_regs(&regs, 2, 0);
        wide_into_stream(context, file, context.arg_ptr(1), args);
    });
    in_libc("vfwprintf", |context| {
        let file = context.arg_ptr(0);
        if let Some(args) = guest_list(context, 2) {
            wide_into_stream(context, file, context.arg_ptr(1), args);
        }
    });
    in_libc("wprintf", |context| {
        let regs = *context.regs();
        let args = GuestVaList::from_regs(&regs, 1, 0);
        wide_into_stream(context, stdio_files::standard(1), context.arg_ptr(0), args);
    });
    in_libc("vwprintf", |context| {
        if let Some(args) = guest_list(context, 1) {
            wide_into_stream(context, stdio_files::standard(1), context.arg_ptr(0), args);
        }
    });
    in_libc("__swprintf_chk", |context| {
        check_access(
            "swprintf",
            "write into",
            context.arg_u64(1) * 4,
            context.arg_u64(4) * 4,
        );
        let regs = *context.regs();
        let args = GuestVaList::from_regs(&regs, 5, 0);
        wide_bounded(
            context,
            context.arg_ptr(0),
            context.arg_u64(1) as usize,
            context.arg_ptr(3),
            args,
        );
    });
    in_libc("__vswprintf_chk", |context| {
        check_access(
            "swprintf",
            "write into",
            context.arg_u64(1) * 4,
            context.arg_u64(4) * 4,
        );
        if let Some(args) = guest_list(context, 5) {
            wide_bounded(
                context,
                context.arg_ptr(0),
                context.arg_u64(1) as usize,
                context.arg_ptr(3),
                args,
            );
        }
    });
    in_libc("__fprintf_chk", |context| {
        let file = context.arg_ptr(0);
        let regs = *context.regs();
        let args = GuestVaList::from_regs(&regs, 3, 0);
        into_stream(context, file, context.arg_ptr(2), args);
    });
    in_libc("__printf_chk", |context| {
        let regs = *context.regs();
        let args = GuestVaList::from_regs(&regs, 2, 0);
        into_stream(context, stdio_files::standard(1), context.arg_ptr(1), args);
    });
    in_libc("__vfprintf_chk", |context| {
        let file = context.arg_ptr(0);
        if let Some(args) = guest_list(context, 3) {
            into_stream(context, file, context.arg_ptr(2), args);
        }
    });
}

fn register_checked() {
    in_libc("__vsnprintf_chk", |context| {
        // cite: bionic fortify.cpp#__vsnprintf_chk
        check_access(
            "vsnprintf",
            "write into",
            context.arg_u64(1),
            context.arg_u64(3),
        );
        if let Some(args) = guest_list(context, 5) {
            into_bounded(
                context,
                context.arg_ptr(0),
                context.arg_u64(1),
                context.arg_ptr(4),
                args,
            );
        }
    });
    in_libc("__snprintf_chk", |context| {
        check_access(
            "vsnprintf",
            "write into",
            context.arg_u64(1),
            context.arg_u64(3),
        );
        let regs = *context.regs();
        let args = GuestVaList::from_regs(&regs, 5, 0);
        into_bounded(
            context,
            context.arg_ptr(0),
            context.arg_u64(1),
            context.arg_ptr(4),
            args,
        );
    });
    in_libc("__vsprintf_chk", |context| {
        if let Some(args) = guest_list(context, 4) {
            checked_unbounded(
                context,
                context.arg_ptr(0),
                context.arg_u64(2),
                context.arg_ptr(3),
                args,
            );
        }
    });
    in_libc("__sprintf_chk", |context| {
        let regs = *context.regs();
        let args = GuestVaList::from_regs(&regs, 4, 0);
        checked_unbounded(
            context,
            context.arg_ptr(0),
            context.arg_u64(2),
            context.arg_ptr(3),
            args,
        );
    });
}

pub fn register() {
    register_formatted();
    register_checked();
}
