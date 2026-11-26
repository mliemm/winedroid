use std::ffi::CString;

use super::super::guest_context;
use super::fd_table::{self, FdEntry, FdKind};
use super::fs_result::from_host;
use super::linux_abi::open;

pub fn anonymous_file() -> i64 {
    let Some(context) = guest_context::current() else {
        return -2;
    };
    let template = context.temp_dir.join("wd-anon-XXXXXX");
    let Ok(template) = CString::new(template.as_os_str().as_encoded_bytes()) else {
        return -22;
    };
    let mut bytes = template.into_bytes_with_nul();
    let fd = unsafe { libc::mkstemp(bytes.as_mut_ptr() as *mut libc::c_char) };
    if fd < 0 {
        return from_host();
    }
    unsafe { libc::unlink(bytes.as_ptr() as *const libc::c_char) };
    fd_table::adopt(
        fd,
        FdEntry::new(
            FdKind::Host,
            context.temp_dir.join("anon").display().to_string(),
            open::RDWR,
        ),
    );
    i64::from(fd)
}

pub fn placeholder_fd() -> i64 {
    let fd = unsafe { libc::open(c"/dev/null".as_ptr(), libc::O_RDONLY | libc::O_CLOEXEC) };
    if fd < 0 {
        from_host()
    } else {
        i64::from(fd)
    }
}

pub fn random_suffix() -> [u8; 6] {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut suffix = [0u8; 6];
    for slot in suffix.iter_mut() {
        *slot = ALPHABET[unsafe { libc::arc4random_uniform(ALPHABET.len() as u32) } as usize];
    }
    suffix
}
