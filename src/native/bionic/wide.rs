use super::export::in_libc;
use super::guest_mem::{read_value, write_value};

fn at(base: usize, index: usize) -> u32 {
    read_value::<u32>(base + index * 4)
}

fn length(base: usize) -> usize {
    // cite: bionic wcslen.c#wcslen
    let mut count = 0;
    while at(base, count) != 0 {
        count += 1;
    }
    count
}

fn compare(left: usize, right: usize, limit: usize) -> i32 {
    // cite: bionic wcscmp.c#wcscmp
    for index in 0..limit {
        let (a, b) = (at(left, index), at(right, index));
        if a != b {
            return a.wrapping_sub(b) as i32;
        }
        if a == 0 {
            break;
        }
    }
    0
}

fn find(base: usize, wanted: u32, last: bool) -> usize {
    // cite: bionic wcschr.c#wcschr
    let mut found = 0;
    let mut index = 0;
    loop {
        let value = at(base, index);
        if value == wanted {
            found = base + index * 4;
            if !last {
                return found;
            }
        }
        if value == 0 {
            return found;
        }
        index += 1;
    }
}

fn copy_terminated(destination: usize, source: usize, limit: usize) {
    // cite: bionic wcsncpy.c#wcsncpy
    let mut ended = false;
    for index in 0..limit {
        let value = if ended { 0 } else { at(source, index) };
        ended |= value == 0;
        write_value::<u32>(destination + index * 4, value);
    }
}

pub fn register() {
    in_libc("wcslen", |context| context.ret_u64(length(context.arg_ptr(0)) as u64));
    in_libc("wcsnlen", |context| {
        let (base, limit) = (context.arg_ptr(0), context.arg_u64(1) as usize);
        context.ret_u64((0..limit).find(|index| at(base, *index) == 0).unwrap_or(limit) as u64);
    });
    in_libc("wcscmp", |context| context.ret_i32(compare(context.arg_ptr(0), context.arg_ptr(1), usize::MAX)));
    in_libc("wcsncmp", |context| context.ret_i32(compare(context.arg_ptr(0), context.arg_ptr(1), context.arg_u64(2) as usize)));
    in_libc("wcscoll", |context| context.ret_i32(compare(context.arg_ptr(0), context.arg_ptr(1), usize::MAX)));
    in_libc("wcschr", |context| context.ret_ptr(find(context.arg_ptr(0), context.arg_u32(1), false)));
    in_libc("wcsrchr", |context| context.ret_ptr(find(context.arg_ptr(0), context.arg_u32(1), true)));
    in_libc("wcscpy", |context| {
        let source = context.arg_ptr(1);
        copy_terminated(context.arg_ptr(0), source, length(source) + 1);
        context.ret_ptr(context.arg_ptr(0));
    });
    in_libc("wcsncpy", |context| {
        copy_terminated(context.arg_ptr(0), context.arg_ptr(1), context.arg_u64(2) as usize);
        context.ret_ptr(context.arg_ptr(0));
    });
    in_libc("wcscat", |context| {
        let (destination, source) = (context.arg_ptr(0), context.arg_ptr(1));
        copy_terminated(destination + length(destination) * 4, source, length(source) + 1);
        context.ret_ptr(destination);
    });
    in_libc("wmemcpy", |context| {
        unsafe { std::ptr::copy(context.arg_ptr(1) as *const u8, context.arg_ptr(0) as *mut u8, context.arg_u64(2) as usize * 4) };
        context.ret_ptr(context.arg_ptr(0));
    });
    in_libc("wmemmove", |context| {
        unsafe { std::ptr::copy(context.arg_ptr(1) as *const u8, context.arg_ptr(0) as *mut u8, context.arg_u64(2) as usize * 4) };
        context.ret_ptr(context.arg_ptr(0));
    });
    in_libc("wmemset", |context| {
        let (destination, value) = (context.arg_ptr(0), context.arg_u32(1));
        for index in 0..context.arg_u64(2) as usize {
            write_value::<u32>(destination + index * 4, value);
        }
        context.ret_ptr(destination);
    });
    in_libc("wmemchr", |context| {
        let (base, wanted) = (context.arg_ptr(0), context.arg_u32(1));
        context.ret_ptr((0..context.arg_u64(2) as usize).find(|index| at(base, *index) == wanted).map_or(0, |index| base + index * 4));
    });
    in_libc("wmemcmp", |context| {
        // cite: bionic wmemcmp.c#wmemcmp
        let (left, right) = (context.arg_ptr(0), context.arg_ptr(1));
        let different = (0..context.arg_u64(2) as usize).find(|index| at(left, *index) != at(right, *index));
        context.ret_i32(different.map_or(0, |index| if at(left, index) > at(right, index) { 1 } else { -1 }));
    });
}
