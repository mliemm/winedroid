use super::abi_types::EINVAL;
use super::ctype::{wide_class_member, wide_class_of, wide_lower, wide_upper};
use super::export::in_libc;
use super::guest_mem::{cstr_bytes, read_value, write_value};

const TRANSLATE_LOWER: u64 = 1;
const TRANSLATE_UPPER: u64 = 2;

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

fn compare_folded(left: usize, right: usize, limit: usize) -> i32 {
    // cite: bionic wcsncasecmp.c#wcsncasecmp
    if limit == 0 {
        return 0;
    }
    let mut remaining = limit;
    let mut index = 0;
    while at(left, index) != 0 {
        let (a, b) = (wide_lower(at(left, index)), wide_lower(at(right, index)));
        if a != b {
            return a.wrapping_sub(b) as i32;
        }
        remaining = remaining.wrapping_sub(1);
        if remaining == 0 {
            return 0;
        }
        index += 1;
    }
    0u32.wrapping_sub(at(right, index)) as i32
}

fn search(haystack: usize, needle: usize) -> usize {
    // cite: bionic wcsstr.c#wcsstr
    let wanted = at(needle, 0);
    if wanted == 0 {
        return haystack;
    }
    let rest = length(needle + 4);
    let mut index = 0;
    loop {
        let current = at(haystack, index);
        if current == 0 {
            return 0;
        }
        if current == wanted && compare(haystack + (index + 1) * 4, needle + 4, rest) == 0 {
            return haystack + index * 4;
        }
        index += 1;
    }
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

fn translate(character: u32, kind: u64) -> Option<u32> {
    // cite: bionic wctype.cpp#towctrans
    match kind {
        TRANSLATE_LOWER => Some(wide_lower(character)),
        TRANSLATE_UPPER => Some(wide_upper(character)),
        _ => None,
    }
}

fn register_classes() {
    in_libc("wctype", |context| context.ret_u64(wide_class_of(cstr_bytes(context.arg_ptr(0)))));
    in_libc("wctype_l", |context| context.ret_u64(wide_class_of(cstr_bytes(context.arg_ptr(0)))));
    in_libc("iswctype", |context| context.ret_i32(wide_class_member(context.arg_u64(1), context.arg_i32(0))));
    in_libc("iswctype_l", |context| context.ret_i32(wide_class_member(context.arg_u64(1), context.arg_i32(0))));
    for name in ["wctrans", "wctrans_l"] {
        in_libc(name, |context| {
            // cite: bionic wctype.cpp#wctrans
            context.ret_u64(match cstr_bytes(context.arg_ptr(0)) {
                b"tolower" => TRANSLATE_LOWER,
                b"toupper" => TRANSLATE_UPPER,
                _ => 0,
            });
        });
    }
    for name in ["towctrans", "towctrans_l"] {
        in_libc(name, |context| match translate(context.arg_u32(0), context.arg_u64(1)) {
            Some(value) => context.ret_u32(value),
            None => {
                context.set_errno(EINVAL);
                context.ret_u32(0);
            }
        });
    }
}

fn register_search() {
    in_libc("wcsstr", |context| context.ret_ptr(search(context.arg_ptr(0), context.arg_ptr(1))));
    in_libc("wcscasecmp", |context| context.ret_i32(compare_folded(context.arg_ptr(0), context.arg_ptr(1), usize::MAX)));
    in_libc("wcscasecmp_l", |context| context.ret_i32(compare_folded(context.arg_ptr(0), context.arg_ptr(1), usize::MAX)));
    in_libc("wcsncasecmp", |context| context.ret_i32(compare_folded(context.arg_ptr(0), context.arg_ptr(1), context.arg_u64(2) as usize)));
    in_libc("wcsncasecmp_l", |context| context.ret_i32(compare_folded(context.arg_ptr(0), context.arg_ptr(1), context.arg_u64(2) as usize)));
    for name in ["wcsxfrm", "wcsxfrm_l"] {
        in_libc(name, |context| {
            // cite: bionic wcsxfrm.c#wcsxfrm
            let (destination, source, capacity) = (context.arg_ptr(0), context.arg_ptr(1), context.arg_u64(2) as usize);
            let total = length(source);
            if capacity != 0 {
                copy_terminated(destination, source, total.min(capacity - 1));
                write_value::<u32>(destination + total.min(capacity - 1) * 4, 0);
            }
            context.ret_u64(total as u64);
        });
    }
}

pub fn register() {
    register_classes();
    register_search();
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
    in_libc("wcsncat", |context| {
        // cite: bionic wcsncat.c#wcsncat
        let (destination, source, limit) = (context.arg_ptr(0), context.arg_ptr(1), context.arg_u64(2) as usize);
        let mut end = destination + length(destination) * 4;
        let mut index = 0;
        while index < limit && at(source, index) != 0 {
            write_value::<u32>(end, at(source, index));
            end += 4;
            index += 1;
        }
        write_value::<u32>(end, 0);
        context.ret_ptr(destination);
    });
    in_libc("wcstok", |context| {
        // cite: bionic wcstok.c#wcstok
        context.ret_ptr(tokenize(context.arg_ptr(0), context.arg_ptr(1), context.arg_ptr(2)));
    });
}

fn is_delimiter(delim: usize, wanted: u32) -> bool {
    let mut index = 0;
    loop {
        let value = at(delim, index);
        index += 1;
        if value == 0 {
            return false;
        }
        if value == wanted {
            return true;
        }
    }
}

fn ends_token(delim: usize, wanted: u32) -> bool {
    let mut index = 0;
    loop {
        let value = at(delim, index);
        index += 1;
        if value == wanted {
            return true;
        }
        if value == 0 {
            return false;
        }
    }
}

fn tokenize(start: usize, delim: usize, last: usize) -> usize {
    let mut cursor = if start != 0 { start } else { read_value::<usize>(last) };
    if cursor == 0 {
        return 0;
    }
    let mut character;
    loop {
        character = at(cursor, 0);
        cursor += 4;
        if !is_delimiter(delim, character) {
            break;
        }
    }
    if character == 0 {
        write_value::<usize>(last, 0);
        return 0;
    }
    let token = cursor - 4;
    loop {
        character = at(cursor, 0);
        cursor += 4;
        if ends_token(delim, character) {
            let next = if character == 0 {
                0
            } else {
                write_value::<u32>(cursor - 4, 0);
                cursor
            };
            write_value::<usize>(last, next);
            return token;
        }
    }
}
