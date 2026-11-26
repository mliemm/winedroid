use std::sync::OnceLock;

use super::export::{data_in_libc, in_libc};
use super::guest_mem::write_value;

const UPPER: u8 = 0x01;
const LOWER: u8 = 0x02;
const DIGIT: u8 = 0x04;
const SPACE: u8 = 0x08;
const PUNCT: u8 = 0x10;
const CONTROL: u8 = 0x20;
const HEX: u8 = 0x40;
const BLANK: u8 = 0x80;
const TABLE_ENTRIES: usize = 257;

static TABLE: OnceLock<usize> = OnceLock::new();

pub fn class_flags(byte: u8) -> u8 {
    // cite: bionic ctype_.c#_C_ctype_
    match byte {
        b'\t'..=b'\r' => CONTROL | SPACE,
        0..=31 | 127 => CONTROL,
        b' ' => SPACE | BLANK,
        b'0'..=b'9' => DIGIT,
        b'A'..=b'F' => UPPER | HEX,
        b'G'..=b'Z' => UPPER,
        b'a'..=b'f' => LOWER | HEX,
        b'g'..=b'z' => LOWER,
        33..=126 => PUNCT,
        _ => 0,
    }
}

fn table() -> usize {
    *TABLE.get_or_init(|| {
        let mut entries = vec![0u8; TABLE_ENTRIES];
        for byte in 0..=255u8 {
            entries[usize::from(byte) + 1] = class_flags(byte);
        }
        Box::leak(entries.into_boxed_slice()).as_ptr() as usize
    })
}

fn flags_of(character: i32) -> u8 {
    if (-1..=255).contains(&character) {
        unsafe { *((table() + (character + 1) as usize) as *const u8) }
    } else {
        0
    }
}

fn is_alpha(character: i32) -> bool {
    // cite: bionic ctype_inlines.h#isalpha
    ((character | 32).wrapping_sub(i32::from(b'a')) as u32) < 26
}

fn is_digit(character: i32) -> bool {
    (character.wrapping_sub(i32::from(b'0')) as u32) < 10
}

fn is_graph(character: i32) -> bool {
    (character.wrapping_sub(33) as u32) < 94
}

fn is_hex(character: i32) -> bool {
    is_digit(character) || ((character | 32).wrapping_sub(i32::from(b'a')) as u32) < 6
}

fn is_space(character: i32) -> bool {
    character == i32::from(b' ') || (character.wrapping_sub(9) as u32) < 5
}

fn registered(names: &[&str], predicate: fn(i32) -> i32) {
    for name in names {
        in_libc(name, move |context| {
            context.ret_i32(predicate(context.arg_i32(0)))
        });
    }
}

fn flag(value: bool) -> i32 {
    i32::from(value)
}

fn lower(character: i32) -> i32 {
    if (i32::from(b'A')..=i32::from(b'Z')).contains(&character) {
        character + 32
    } else {
        character
    }
}

fn upper(character: i32) -> i32 {
    if (i32::from(b'a')..=i32::from(b'z')).contains(&character) {
        character - 32
    } else {
        character
    }
}

const CLASSES: [(&str, fn(i32) -> i32); 12] = [
    ("alnum", |c| {
        i32::from(flags_of(c) & (UPPER | LOWER | DIGIT))
    }),
    ("alpha", |c| flag(is_alpha(c))),
    ("blank", |c| {
        flag(c == i32::from(b' ') || c == i32::from(b'\t'))
    }),
    ("cntrl", |c| flag((c as u32) < 32 || c == 127)),
    ("digit", |c| flag(is_digit(c))),
    ("graph", |c| flag(is_graph(c))),
    ("lower", |c| {
        flag(((c.wrapping_sub(i32::from(b'a'))) as u32) < 26)
    }),
    ("print", |c| flag((c.wrapping_sub(32) as u32) < 95)),
    ("punct", |c| i32::from(flags_of(c) & PUNCT)),
    ("space", |c| flag(is_space(c))),
    ("upper", |c| {
        flag(((c.wrapping_sub(i32::from(b'A'))) as u32) < 26)
    }),
    ("xdigit", |c| flag(is_hex(c))),
];

pub fn wide_class_member(class: u64, character: i32) -> i32 {
    // cite: bionic wctype.cpp#iswctype
    match class {
        1..=12 => CLASSES[class as usize - 1].1(character),
        _ => 0,
    }
}

pub fn wide_class_of(name: &[u8]) -> u64 {
    // cite: bionic wctype.cpp#wctype
    CLASSES
        .iter()
        .position(|(candidate, _)| candidate.as_bytes() == name)
        .map_or(0, |index| index as u64 + 1)
}

pub fn wide_lower(character: u32) -> u32 {
    // cite: bionic wctype.cpp#towlower
    lower(character as i32) as u32
}

pub fn wide_upper(character: u32) -> u32 {
    // cite: bionic wctype.cpp#towupper
    upper(character as i32) as u32
}

pub fn register() {
    let cell = data_in_libc("_ctype_", 8);
    write_value::<usize>(cell, table());
    for (name, predicate) in CLASSES {
        for prefix in ["is", "isw"] {
            in_libc(&format!("{prefix}{name}"), move |context| {
                context.ret_i32(predicate(context.arg_i32(0)))
            });
            in_libc(&format!("{prefix}{name}_l"), move |context| {
                context.ret_i32(predicate(context.arg_i32(0)))
            });
        }
    }
    registered(&["isascii"], |c| flag((c as u32) < 0x80));
    registered(&["toascii"], |c| c & 0x7f);
    registered(&["tolower", "tolower_l", "towlower", "towlower_l"], lower);
    registered(&["toupper", "toupper_l", "towupper", "towupper_l"], upper);
    registered(&["_tolower"], |c| c | 0x20);
    registered(&["_toupper"], |c| c ^ 0x20);
}
