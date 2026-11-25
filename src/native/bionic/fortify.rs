use super::export::in_libc;
use super::fatal::fortify_fatal;
use super::fortify_check::{check_access, check_count, check_fd_set};
use super::guest_mem::{cstr_bytes, read_value, write_value};
use super::super::bridge::CallContext;

fn byte(address: usize) -> u8 {
    read_value::<u8>(address)
}

fn strlen_chk(source: usize, capacity: u64) -> usize {
    // cite: bionic fortify.cpp#__strlen_chk
    let length = cstr_bytes(source).len();
    if length as u64 >= capacity {
        fortify_fatal("strlen: detected read past end of buffer");
    }
    length
}

fn strchr_chk(mut cursor: usize, ch: i32, mut remaining: u64) -> usize {
    // cite: bionic fortify.cpp#__strchr_chk
    loop {
        if remaining == 0 {
            fortify_fatal("strchr: prevented read past end of buffer");
        }
        if byte(cursor) == ch as u8 {
            return cursor;
        }
        if byte(cursor) == 0 {
            return 0;
        }
        cursor += 1;
        remaining -= 1;
    }
}

fn strrchr_chk(mut cursor: usize, ch: i32, mut remaining: u64) -> usize {
    // cite: bionic fortify.cpp#__strrchr_chk
    let mut saved = 0;
    loop {
        if remaining == 0 {
            fortify_fatal("strrchr: prevented read past end of buffer");
        }
        if byte(cursor) == ch as u8 {
            saved = cursor;
        }
        if byte(cursor) == 0 {
            return saved;
        }
        cursor += 1;
        remaining -= 1;
    }
}

fn strcat_chk(destination: usize, source: usize, capacity: u64) -> usize {
    // cite: bionic fortify.cpp#__strcat_chk
    let used = strlen_chk(destination, capacity);
    let mut room = capacity - used as u64;
    let (mut to, mut from) = (destination + used, source);
    loop {
        let value = byte(from);
        write_value::<u8>(to, value);
        to += 1;
        from += 1;
        if value == 0 {
            return destination;
        }
        room -= 1;
        if room == 0 {
            fortify_fatal(&format!("strcat: prevented write past end of {room}-byte buffer"));
        }
    }
}

fn strncat_chk(destination: usize, source: usize, mut limit: u64, capacity: u64) -> usize {
    // cite: bionic fortify.cpp#__strncat_chk
    if limit == 0 {
        return destination;
    }
    let used = strlen_chk(destination, capacity);
    let (mut to, mut from, mut room) = (destination + used, source, capacity - used as u64);
    while byte(from) != 0 {
        write_value::<u8>(to, byte(from));
        to += 1;
        from += 1;
        limit -= 1;
        room -= 1;
        if room == 0 {
            fortify_fatal("strncat: prevented write past end of buffer");
        }
        if limit == 0 {
            break;
        }
    }
    write_value::<u8>(to, 0);
    destination
}

fn bounded_copy(function: &str, destination: usize, source: usize, mut count: u64, destination_len: u64, source_len: u64) -> usize {
    // cite: bionic fortify.cpp#__strncpy_chk2, __stpncpy_chk2
    check_access(function, "write into", count, destination_len);
    let (mut to, mut from) = (destination, source);
    while count != 0 {
        if (from - source) as u64 >= source_len {
            fortify_fatal(&format!("{function}: detected read past end of {source_len}-byte buffer"));
        }
        let value = byte(from);
        write_value::<u8>(to, value);
        to += 1;
        from += 1;
        count -= 1;
        if value == 0 {
            while count != 0 {
                write_value::<u8>(to, 0);
                to += 1;
                count -= 1;
            }
            break;
        }
    }
    to
}

fn register_copies() {
    in_libc("__memcpy_chk", |context| {
        // cite: bionic fortify.cpp#__memcpy_chk_fail
        let (count, capacity) = (context.arg_u64(2), context.arg_u64(3));
        check_count("memcpy", "count", count);
        check_access("memcpy", "write into", count, capacity);
        unsafe { libc::memmove(context.arg_ptr(0) as *mut _, context.arg_ptr(1) as *const _, count as usize) };
        context.ret_ptr(context.arg_ptr(0));
    });
    in_libc("__memmove_chk", |context| {
        // cite: bionic fortify.cpp#__memmove_chk
        let (count, capacity) = (context.arg_u64(2), context.arg_u64(3));
        check_access("memmove", "write into", count, capacity);
        unsafe { libc::memmove(context.arg_ptr(0) as *mut _, context.arg_ptr(1) as *const _, count as usize) };
        context.ret_ptr(context.arg_ptr(0));
    });
    in_libc("__mempcpy_chk", |context| {
        // cite: bionic fortify.cpp#__mempcpy_chk
        let (count, capacity) = (context.arg_u64(2), context.arg_u64(3));
        check_count("mempcpy", "count", count);
        check_access("mempcpy", "write into", count, capacity);
        unsafe { libc::memmove(context.arg_ptr(0) as *mut _, context.arg_ptr(1) as *const _, count as usize) };
        context.ret_ptr(context.arg_ptr(0) + count as usize);
    });
    in_libc("__memset_chk", |context| {
        // cite: bionic fortify.cpp#__memset_chk_fail
        let (count, capacity) = (context.arg_u64(2), context.arg_u64(3));
        check_count("memset", "count", count);
        check_access("memset", "write into", count, capacity);
        unsafe { libc::memset(context.arg_ptr(0) as *mut _, context.arg_i32(1), count as usize) };
        context.ret_ptr(context.arg_ptr(0));
    });
    in_libc("__memchr_chk", |context| {
        // cite: bionic fortify.cpp#__memchr_chk
        check_access("memchr", "read from", context.arg_u64(2), context.arg_u64(3));
        let found = unsafe { libc::memchr(context.arg_ptr(0) as *const _, context.arg_i32(1), context.arg_u64(2) as usize) };
        context.ret_ptr(found as usize);
    });
    in_libc("__strcpy_chk", |context| {
        // cite: bionic fortify.cpp#__strcpy_chk
        let needed = cstr_bytes(context.arg_ptr(1)).len() as u64 + 1;
        check_access("strcpy", "write into", needed, context.arg_u64(2));
        unsafe { libc::memmove(context.arg_ptr(0) as *mut _, context.arg_ptr(1) as *const _, needed as usize) };
        context.ret_ptr(context.arg_ptr(0));
    });
    in_libc("__stpcpy_chk", |context| {
        // cite: bionic fortify.cpp#__stpcpy_chk
        let needed = cstr_bytes(context.arg_ptr(1)).len() as u64 + 1;
        check_access("stpcpy", "write into", needed, context.arg_u64(2));
        unsafe { libc::memmove(context.arg_ptr(0) as *mut _, context.arg_ptr(1) as *const _, needed as usize) };
        context.ret_ptr(context.arg_ptr(0) + needed as usize - 1);
    });
    in_libc("__strcat_chk", |context| context.ret_ptr(strcat_chk(context.arg_ptr(0), context.arg_ptr(1), context.arg_u64(2))));
    in_libc("__strncat_chk", |context| context.ret_ptr(strncat_chk(context.arg_ptr(0), context.arg_ptr(1), context.arg_u64(2), context.arg_u64(3))));
    in_libc("__strncpy_chk", |context| {
        // cite: bionic fortify.cpp#__strncpy_chk
        check_access("strncpy", "write into", context.arg_u64(2), context.arg_u64(3));
        unsafe { libc::strncpy(context.arg_ptr(0) as *mut _, context.arg_ptr(1) as *const _, context.arg_u64(2) as usize) };
        context.ret_ptr(context.arg_ptr(0));
    });
    in_libc("__strncpy_chk2", |context| {
        bounded_copy("strncpy", context.arg_ptr(0), context.arg_ptr(1), context.arg_u64(2), context.arg_u64(3), context.arg_u64(4));
        context.ret_ptr(context.arg_ptr(0));
    });
    in_libc("__stpncpy_chk", |context| {
        // cite: bionic fortify.cpp#__stpncpy_chk
        check_access("stpncpy", "write into", context.arg_u64(2), context.arg_u64(3));
        let end = unsafe { libc::stpncpy(context.arg_ptr(0) as *mut _, context.arg_ptr(1) as *const _, context.arg_u64(2) as usize) };
        context.ret_ptr(end as usize);
    });
    in_libc("__stpncpy_chk2", |context| {
        bounded_copy("stpncpy", context.arg_ptr(0), context.arg_ptr(1), context.arg_u64(2), context.arg_u64(3), context.arg_u64(4));
        context.ret_ptr(context.arg_ptr(0));
    });
    in_libc("__strlcpy_chk", |context| {
        // cite: bionic fortify.cpp#__strlcpy_chk
        check_access("strlcpy", "write into", context.arg_u64(2), context.arg_u64(3));
        let length = unsafe { super::string::host::strlcpy(context.arg_ptr(0) as *mut _, context.arg_ptr(1) as *const _, context.arg_u64(2) as usize) };
        context.ret_u64(length as u64);
    });
    in_libc("__strlcat_chk", |context| {
        // cite: bionic fortify.cpp#__strlcat_chk
        check_access("strlcat", "write into", context.arg_u64(2), context.arg_u64(3));
        context.ret_u64(super::string::strlcat(context.arg_ptr(0), context.arg_ptr(1), context.arg_u64(2) as usize) as u64);
    });
}

fn register_scans() {
    in_libc("__strlen_chk", |context| context.ret_u64(strlen_chk(context.arg_ptr(0), context.arg_u64(1)) as u64));
    in_libc("__strchr_chk", |context| context.ret_ptr(strchr_chk(context.arg_ptr(0), context.arg_i32(1), context.arg_u64(2))));
    in_libc("__strrchr_chk", |context| context.ret_ptr(strrchr_chk(context.arg_ptr(0), context.arg_i32(1), context.arg_u64(2))));
}

fn fd_set_bit(context: &CallContext) -> (usize, u64) {
    let fd = context.arg_i32(0) as usize;
    (context.arg_ptr(1) + (fd / 64) * 8, 1u64 << (fd % 64))
}

fn register_descriptors() {
    in_libc("__FD_SET_chk", |context| {
        check_fd_set("FD_SET", context.arg_i32(0), context.arg_u64(2));
        let (address, mask) = fd_set_bit(context);
        write_value::<u64>(address, read_value::<u64>(address) | mask);
    });
    in_libc("__FD_CLR_chk", |context| {
        check_fd_set("FD_CLR", context.arg_i32(0), context.arg_u64(2));
        let (address, mask) = fd_set_bit(context);
        write_value::<u64>(address, read_value::<u64>(address) & !mask);
    });
    in_libc("__FD_ISSET_chk", |context| {
        check_fd_set("FD_ISSET", context.arg_i32(0), context.arg_u64(2));
        let (address, mask) = fd_set_bit(context);
        context.ret_i32(i32::from(read_value::<u64>(address) & mask != 0));
    });
    in_libc("__read_chk", |context| {
        // cite: bionic fortify.cpp#__read_chk
        let (count, capacity) = (context.arg_u64(2), context.arg_u64(3));
        check_count("read", "count", count);
        check_access("read", "write into", count, capacity);
        let read = unsafe { libc::read(context.arg_i32(0), context.arg_ptr(1) as *mut _, count as usize) };
        if read < 0 {
            context.set_errno_from_host(unsafe { *libc::__error() });
        }
        context.ret_i64(read as i64);
    });
    in_libc("__write_chk", |context| {
        let (count, capacity) = (context.arg_u64(2), context.arg_u64(3));
        check_count("write", "count", count);
        check_access("write", "read from", count, capacity);
        let written = unsafe { libc::write(context.arg_i32(0), context.arg_ptr(1) as *const _, count as usize) };
        if written < 0 {
            context.set_errno_from_host(unsafe { *libc::__error() });
        }
        context.ret_i64(written as i64);
    });
}

pub fn register() {
    register_copies();
    register_scans();
    register_descriptors();
}
