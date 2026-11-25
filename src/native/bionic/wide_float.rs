use std::ffi::CString;

use super::super::bridge::CallContext;
use super::ctype::{wide_class_member, wide_class_of};
use super::export::in_libc;
use super::guest_mem::{read_value, write_value};

const NUMERIC: &[u8] = b"-+0123456789.xXeEpP()nNaAiIfFtTyY";

fn convert(context: &mut CallContext, single: bool) {
    // cite: bionic wcstod.cpp#wcstod
    let (string, end) = (context.arg_ptr(0), context.arg_ptr(1));
    let space = wide_class_of(b"space");
    let mut start = string;
    while wide_class_member(space, read_value::<u32>(start) as i32) != 0 {
        start += 4;
    }
    let mut ascii = Vec::new();
    loop {
        let character = read_value::<u32>(start + ascii.len() * 4);
        if character == 0 || character >= 128 || !NUMERIC.contains(&(character as u8)) {
            break;
        }
        ascii.push(character as u8);
    }
    let text = CString::new(ascii).unwrap_or_default();
    let mut ascii_end: *mut libc::c_char = std::ptr::null_mut();
    unsafe { *libc::__error() = 0 };
    if single {
        context.ret_f32(unsafe { libc::strtof(text.as_ptr(), &mut ascii_end) });
    } else {
        context.ret_f64(unsafe { libc::strtod(text.as_ptr(), &mut ascii_end) });
    }
    let host_errno = unsafe { *libc::__error() };
    if host_errno != 0 {
        context.set_errno_from_host(host_errno);
    }
    let consumed = ascii_end as usize - text.as_ptr() as usize;
    if end != 0 {
        write_value::<usize>(end, if consumed == 0 { string } else { start + consumed * 4 });
    }
}

pub fn register() {
    for name in ["wcstod", "wcstod_l"] {
        in_libc(name, |context| convert(context, false));
    }
    for name in ["wcstof", "wcstof_l"] {
        in_libc(name, |context| convert(context, true));
    }
}
