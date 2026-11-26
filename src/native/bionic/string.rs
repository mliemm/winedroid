use std::ffi::c_char;
use std::sync::Mutex;

use super::export::in_libc;
use super::guest_mem::{cstr_bytes, write_bytes, write_value};

pub mod host {
    use std::ffi::c_char;

    extern "C" {
        pub fn strlcpy(destination: *mut c_char, source: *const c_char, size: usize) -> usize;
        pub fn strsep(cursor: *mut *mut c_char, delimiters: *const c_char) -> *mut c_char;
    }
}

fn text(pointer: usize) -> *const c_char {
    pointer as *const c_char
}

pub fn compare_ascii_fold(left: &[u8], right: &[u8], limit: usize) -> i32 {
    // cite: bionic strcasecmp.c#strcasecmp, strncasecmp
    for index in 0..limit {
        let a = left.get(index).copied().unwrap_or(0).to_ascii_lowercase();
        let b = right.get(index).copied().unwrap_or(0).to_ascii_lowercase();
        if a != b {
            return i32::from(a) - i32::from(b);
        }
        if a == 0 {
            break;
        }
    }
    0
}

fn find_fold(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    // cite: bionic strcasestr.c#strcasestr
    if needle.is_empty() {
        return Some(0);
    }
    haystack
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle))
}

fn memrchr(base: usize, byte: u8, length: usize) -> usize {
    // cite: bionic memrchr.c#memrchr
    (0..length)
        .rev()
        .find(|index| unsafe { *((base + index) as *const u8) } == byte)
        .map_or(0, |index| base + index)
}

fn strchrnul(base: usize, byte: u8) -> usize {
    // cite: bionic strchrnul.cpp#strchrnul
    let bytes = cstr_bytes(base);
    base + bytes
        .iter()
        .position(|candidate| *candidate == byte)
        .unwrap_or(bytes.len())
}

pub fn strlcat(destination: usize, source: usize, size: usize) -> usize {
    // cite: bionic strlcat.c#strlcat
    let source_len = cstr_bytes(source).len();
    let mut used = 0;
    while used < size && unsafe { *((destination + used) as *const u8) } != 0 {
        used += 1;
    }
    if used == size {
        return size + source_len;
    }
    let copied = source_len.min(size - used - 1);
    write_bytes(destination + used, &cstr_bytes(source)[..copied]);
    write_value::<u8>(destination + used + copied, 0);
    used + source_len
}

static TOKEN_STATE: Mutex<usize> = Mutex::new(0);

fn byte_at(address: usize) -> u8 {
    unsafe { *(address as *const u8) }
}

fn strtok(start: usize, delimiters: usize) -> usize {
    // cite: bionic strtok.c#strtok
    let mut last = TOKEN_STATE.lock().expect("strtok state");
    let mut cursor = if start != 0 { start } else { *last };
    if cursor == 0 {
        return 0;
    }
    let separators = cstr_bytes(delimiters);
    while byte_at(cursor) != 0 && separators.contains(&byte_at(cursor)) {
        cursor += 1;
    }
    if byte_at(cursor) == 0 {
        *last = 0;
        return 0;
    }
    let token = cursor;
    while byte_at(cursor) != 0 && !separators.contains(&byte_at(cursor)) {
        cursor += 1;
    }
    if byte_at(cursor) == 0 {
        *last = 0;
    } else {
        write_value::<u8>(cursor, 0);
        *last = cursor + 1;
    }
    token
}

fn register_memory() {
    in_libc("memcpy", |context| {
        unsafe {
            libc::memmove(
                context.arg_ptr(0) as *mut _,
                context.arg_ptr(1) as *const _,
                context.arg_u64(2) as usize,
            )
        };
        context.ret_ptr(context.arg_ptr(0));
    });
    in_libc("memmove", |context| {
        unsafe {
            libc::memmove(
                context.arg_ptr(0) as *mut _,
                context.arg_ptr(1) as *const _,
                context.arg_u64(2) as usize,
            )
        };
        context.ret_ptr(context.arg_ptr(0));
    });
    in_libc("mempcpy", |context| {
        let length = context.arg_u64(2) as usize;
        unsafe {
            libc::memmove(
                context.arg_ptr(0) as *mut _,
                context.arg_ptr(1) as *const _,
                length,
            )
        };
        context.ret_ptr(context.arg_ptr(0) + length);
    });
    in_libc("memset", |context| {
        unsafe {
            libc::memset(
                context.arg_ptr(0) as *mut _,
                context.arg_i32(1),
                context.arg_u64(2) as usize,
            )
        };
        context.ret_ptr(context.arg_ptr(0));
    });
    in_libc("memcmp", |context| {
        let result = unsafe {
            libc::memcmp(
                context.arg_ptr(0) as *const _,
                context.arg_ptr(1) as *const _,
                context.arg_u64(2) as usize,
            )
        };
        context.ret_i32(result);
    });
    in_libc("memchr", |context| {
        let found = unsafe {
            libc::memchr(
                context.arg_ptr(0) as *const _,
                context.arg_i32(1),
                context.arg_u64(2) as usize,
            )
        };
        context.ret_ptr(found as usize);
    });
    in_libc("memrchr", |context| {
        context.ret_ptr(memrchr(
            context.arg_ptr(0),
            context.arg_i32(1) as u8,
            context.arg_u64(2) as usize,
        ))
    });
    in_libc("memmem", |context| {
        let found = unsafe {
            libc::memmem(
                context.arg_ptr(0) as *const _,
                context.arg_u64(1) as usize,
                context.arg_ptr(2) as *const _,
                context.arg_u64(3) as usize,
            )
        };
        context.ret_ptr(found as usize);
    });
    in_libc("memccpy", |context| {
        let found = unsafe {
            libc::memccpy(
                context.arg_ptr(0) as *mut _,
                context.arg_ptr(1) as *const _,
                context.arg_i32(2),
                context.arg_u64(3) as usize,
            )
        };
        context.ret_ptr(found as usize);
    });
}

fn register_length_and_compare() {
    in_libc("strlen", |context| {
        context.ret_u64(cstr_bytes(context.arg_ptr(0)).len() as u64)
    });
    in_libc("strnlen", |context| {
        context.ret_u64(unsafe {
            libc::strnlen(text(context.arg_ptr(0)), context.arg_u64(1) as usize)
        } as u64)
    });
    in_libc("strcmp", |context| {
        context.ret_i32(unsafe { libc::strcmp(text(context.arg_ptr(0)), text(context.arg_ptr(1))) })
    });
    in_libc("strncmp", |context| {
        context.ret_i32(unsafe {
            libc::strncmp(
                text(context.arg_ptr(0)),
                text(context.arg_ptr(1)),
                context.arg_u64(2) as usize,
            )
        })
    });
    in_libc("strcoll", |context| {
        context.ret_i32(unsafe { libc::strcmp(text(context.arg_ptr(0)), text(context.arg_ptr(1))) })
    });
    in_libc("strcoll_l", |context| {
        // cite: bionic string_l.cpp#strcoll_l
        context
            .ret_i32(unsafe { libc::strcmp(text(context.arg_ptr(0)), text(context.arg_ptr(1))) });
    });
    in_libc("strcasecmp", |context| {
        let (left, right) = (
            cstr_bytes(context.arg_ptr(0)),
            cstr_bytes(context.arg_ptr(1)),
        );
        context.ret_i32(compare_ascii_fold(
            left,
            right,
            left.len().max(right.len()) + 1,
        ));
    });
    in_libc("strncasecmp", |context| {
        let (left, right) = (
            cstr_bytes(context.arg_ptr(0)),
            cstr_bytes(context.arg_ptr(1)),
        );
        context.ret_i32(compare_ascii_fold(
            left,
            right,
            (context.arg_u64(2) as usize).min(left.len().max(right.len()) + 1),
        ));
    });
    in_libc("strxfrm", |context| {
        let source = cstr_bytes(context.arg_ptr(1));
        let capacity = context.arg_u64(2) as usize;
        if capacity > 0 {
            let copied = source.len().min(capacity - 1);
            write_bytes(context.arg_ptr(0), &source[..copied]);
            write_value::<u8>(context.arg_ptr(0) + copied, 0);
        }
        context.ret_u64(source.len() as u64);
    });
    in_libc("strxfrm_l", |context| {
        // cite: bionic string_l.cpp#strxfrm_l
        let source = cstr_bytes(context.arg_ptr(1));
        let capacity = context.arg_u64(2) as usize;
        if capacity > 0 {
            let copied = source.len().min(capacity - 1);
            write_bytes(context.arg_ptr(0), &source[..copied]);
            write_value::<u8>(context.arg_ptr(0) + copied, 0);
        }
        context.ret_u64(source.len() as u64);
    });
}

fn register_search() {
    in_libc("strchr", |context| {
        context
            .ret_ptr(unsafe { libc::strchr(text(context.arg_ptr(0)), context.arg_i32(1)) } as usize)
    });
    in_libc("strrchr", |context| {
        context.ret_ptr(
            unsafe { libc::strrchr(text(context.arg_ptr(0)), context.arg_i32(1)) } as usize,
        )
    });
    in_libc("strchrnul", |context| {
        context.ret_ptr(strchrnul(context.arg_ptr(0), context.arg_i32(1) as u8))
    });
    in_libc("strstr", |context| {
        context.ret_ptr(
            unsafe { libc::strstr(text(context.arg_ptr(0)), text(context.arg_ptr(1))) } as usize,
        )
    });
    in_libc("strcasestr", |context| {
        let haystack = context.arg_ptr(0);
        let found = find_fold(cstr_bytes(haystack), cstr_bytes(context.arg_ptr(1)));
        context.ret_ptr(found.map_or(0, |offset| haystack + offset));
    });
    in_libc("strspn", |context| {
        context.ret_u64(
            unsafe { libc::strspn(text(context.arg_ptr(0)), text(context.arg_ptr(1))) } as u64,
        )
    });
    in_libc("strcspn", |context| {
        context.ret_u64(
            unsafe { libc::strcspn(text(context.arg_ptr(0)), text(context.arg_ptr(1))) } as u64,
        )
    });
    in_libc("strpbrk", |context| {
        context.ret_ptr(
            unsafe { libc::strpbrk(text(context.arg_ptr(0)), text(context.arg_ptr(1))) } as usize,
        )
    });
    in_libc("strtok", |context| {
        context.ret_ptr(strtok(context.arg_ptr(0), context.arg_ptr(1)))
    });
    in_libc("strtok_r", |context| {
        let token = unsafe {
            libc::strtok_r(
                context.arg_ptr(0) as *mut c_char,
                text(context.arg_ptr(1)),
                context.arg_ptr(2) as *mut *mut c_char,
            )
        };
        context.ret_ptr(token as usize);
    });
    in_libc("strsep", |context| {
        let token = unsafe {
            host::strsep(
                context.arg_ptr(0) as *mut *mut c_char,
                text(context.arg_ptr(1)),
            )
        };
        context.ret_ptr(token as usize);
    });
}

fn register_copy() {
    in_libc("strcpy", |context| {
        unsafe { libc::strcpy(context.arg_ptr(0) as *mut c_char, text(context.arg_ptr(1))) };
        context.ret_ptr(context.arg_ptr(0));
    });
    in_libc("stpcpy", |context| {
        let length = cstr_bytes(context.arg_ptr(1)).len();
        unsafe {
            libc::memmove(
                context.arg_ptr(0) as *mut _,
                context.arg_ptr(1) as *const _,
                length + 1,
            )
        };
        context.ret_ptr(context.arg_ptr(0) + length);
    });
    in_libc("strncpy", |context| {
        unsafe {
            libc::strncpy(
                context.arg_ptr(0) as *mut c_char,
                text(context.arg_ptr(1)),
                context.arg_u64(2) as usize,
            )
        };
        context.ret_ptr(context.arg_ptr(0));
    });
    in_libc("stpncpy", |context| {
        let end = unsafe {
            libc::stpncpy(
                context.arg_ptr(0) as *mut c_char,
                text(context.arg_ptr(1)),
                context.arg_u64(2) as usize,
            )
        };
        context.ret_ptr(end as usize);
    });
    in_libc("strcat", |context| {
        unsafe { libc::strcat(context.arg_ptr(0) as *mut c_char, text(context.arg_ptr(1))) };
        context.ret_ptr(context.arg_ptr(0));
    });
    in_libc("strncat", |context| {
        unsafe {
            libc::strncat(
                context.arg_ptr(0) as *mut c_char,
                text(context.arg_ptr(1)),
                context.arg_u64(2) as usize,
            )
        };
        context.ret_ptr(context.arg_ptr(0));
    });
    in_libc("strlcpy", |context| {
        let length = unsafe {
            host::strlcpy(
                context.arg_ptr(0) as *mut c_char,
                text(context.arg_ptr(1)),
                context.arg_u64(2) as usize,
            )
        };
        context.ret_u64(length as u64);
    });
    in_libc("strlcat", |context| {
        context.ret_u64(strlcat(
            context.arg_ptr(0),
            context.arg_ptr(1),
            context.arg_u64(2) as usize,
        ) as u64)
    });
    in_libc("strdup", |context| {
        let copy = unsafe { libc::strdup(text(context.arg_ptr(0))) };
        context.ret_ptr(copy as usize);
    });
    in_libc("strndup", |context| {
        let copy = unsafe { libc::strndup(text(context.arg_ptr(0)), context.arg_u64(1) as usize) };
        context.ret_ptr(copy as usize);
    });
}

pub fn register() {
    register_memory();
    register_length_and_compare();
    register_search();
    register_copy();
}
