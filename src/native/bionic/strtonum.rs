use super::super::bridge::CallContext;
use super::abi_types::{EINVAL, ERANGE};
use super::export::in_libc;
use super::guest_mem::{read_value, write_value};

pub trait Unit: Copy {
    const SIZE: usize;
    fn load(address: usize) -> i64;
}

impl Unit for u8 {
    const SIZE: usize = 1;
    fn load(address: usize) -> i64 {
        i64::from(read_value::<u8>(address))
    }
}

impl Unit for u32 {
    const SIZE: usize = 4;
    fn load(address: usize) -> i64 {
        i64::from(read_value::<u32>(address))
    }
}

pub struct Parsed {
    pub value: u64,
    pub end: usize,
    pub errno: Option<i32>,
}

struct Prefix {
    negative: bool,
    base: u64,
    first: i64,
    cursor: usize,
}

fn is_space(unit: i64) -> bool {
    unit == i64::from(b' ') || (i64::from(b'\t')..=i64::from(b'\r')).contains(&unit)
}

fn is_digit(unit: i64) -> bool {
    (i64::from(b'0')..=i64::from(b'9')).contains(&unit)
}

fn is_hex_digit(unit: i64) -> bool {
    is_digit(unit)
        || (i64::from(b'a')..=i64::from(b'f')).contains(&unit)
        || (i64::from(b'A')..=i64::from(b'F')).contains(&unit)
}

fn digit_value(unit: i64) -> Option<u64> {
    let letter = |low: u8| unit - i64::from(low) + 10;
    if is_digit(unit) {
        Some((unit - i64::from(b'0')) as u64)
    } else if (i64::from(b'a')..=i64::from(b'z')).contains(&unit) {
        Some(letter(b'a') as u64)
    } else if (i64::from(b'A')..=i64::from(b'Z')).contains(&unit) {
        Some(letter(b'A') as u64)
    } else {
        None
    }
}

fn read_prefix<U: Unit>(start: usize, base: i32) -> Prefix {
    // cite: bionic strtol.cpp#StrToI, StrToU
    let mut base = base as u64;
    let mut cursor = start;
    let mut unit;
    loop {
        unit = U::load(cursor);
        cursor += U::SIZE;
        if !is_space(unit) {
            break;
        }
    }
    let mut negative = false;
    if unit == i64::from(b'-') {
        negative = true;
        unit = U::load(cursor);
        cursor += U::SIZE;
    } else if unit == i64::from(b'+') {
        unit = U::load(cursor);
        cursor += U::SIZE;
    }
    if unit == i64::from(b'0') {
        let marker = U::load(cursor);
        let is_hex = marker == i64::from(b'x') || marker == i64::from(b'X');
        let is_binary = marker == i64::from(b'b') || marker == i64::from(b'B');
        if (is_hex && (base == 0 || base == 16) && is_hex_digit(U::load(cursor + U::SIZE)))
            || (is_binary && (base == 0 || base == 2) && is_digit(U::load(cursor + U::SIZE)))
        {
            unit = U::load(cursor + U::SIZE);
            cursor += 2 * U::SIZE;
            base = if is_hex { 16 } else { 2 };
        }
    }
    if base == 0 {
        base = if unit == i64::from(b'0') { 8 } else { 10 };
    }
    Prefix {
        negative,
        base,
        first: unit,
        cursor,
    }
}

fn invalid_base(base: i32) -> bool {
    base < 0 || base == 1 || base > 36
}

pub fn parse_signed<U: Unit>(start: usize, base: i32) -> Parsed {
    // cite: bionic strtol.cpp#StrToI
    if invalid_base(base) {
        return Parsed {
            value: 0,
            end: start,
            errno: Some(EINVAL),
        };
    }
    let prefix = read_prefix::<U>(start, base);
    let base = prefix.base as i64;
    let cutoff = i64::MIN / base;
    let cutlim = -(i64::MIN % base);
    let (mut unit, mut cursor) = (prefix.first, prefix.cursor);
    let (mut any, mut accumulator, mut errno) = (0i32, 0i64, None);
    loop {
        let Some(digit) = digit_value(unit).filter(|digit| (*digit as i64) < base) else {
            break;
        };
        let digit = digit as i64;
        if any >= 0 {
            if accumulator < cutoff || (accumulator == cutoff && digit > cutlim) {
                any = -1;
                accumulator = i64::MIN;
                errno = Some(ERANGE);
            } else {
                any = 1;
                accumulator = accumulator.wrapping_mul(base).wrapping_sub(digit);
            }
        }
        unit = U::load(cursor);
        cursor += U::SIZE;
    }
    let end = if any != 0 { cursor - U::SIZE } else { start };
    if !prefix.negative {
        if accumulator == i64::MIN {
            errno = Some(ERANGE);
            accumulator = i64::MAX;
        } else {
            accumulator = -accumulator;
        }
    }
    Parsed {
        value: accumulator as u64,
        end,
        errno,
    }
}

pub fn parse_unsigned<U: Unit>(start: usize, base: i32) -> Parsed {
    // cite: bionic strtol.cpp#StrToU
    if invalid_base(base) {
        return Parsed {
            value: 0,
            end: start,
            errno: Some(EINVAL),
        };
    }
    let prefix = read_prefix::<U>(start, base);
    let base = prefix.base;
    let cutoff = u64::MAX / base;
    let cutlim = u64::MAX % base;
    let (mut unit, mut cursor) = (prefix.first, prefix.cursor);
    let (mut any, mut accumulator, mut errno) = (0i32, 0u64, None);
    loop {
        let Some(digit) = digit_value(unit).filter(|digit| *digit < base) else {
            break;
        };
        if any >= 0 {
            if accumulator > cutoff || (accumulator == cutoff && digit > cutlim) {
                any = -1;
                accumulator = u64::MAX;
                errno = Some(ERANGE);
            } else {
                any = 1;
                accumulator = accumulator.wrapping_mul(base).wrapping_add(digit);
            }
        }
        unit = U::load(cursor);
        cursor += U::SIZE;
    }
    if prefix.negative && any > 0 {
        accumulator = accumulator.wrapping_neg();
    }
    Parsed {
        value: accumulator,
        end: if any != 0 { cursor - U::SIZE } else { start },
        errno,
    }
}

fn finish(context: &mut CallContext, parsed: Parsed) {
    let end_pointer = context.arg_ptr(1);
    if end_pointer != 0 {
        write_value::<usize>(end_pointer, parsed.end);
    }
    if let Some(code) = parsed.errno {
        context.set_errno(code);
    }
    context.ret_u64(parsed.value);
}

fn register_signed<U: Unit>(names: &[&str]) {
    for name in names {
        in_libc(name, |context| {
            let parsed = parse_signed::<U>(context.arg_ptr(0), context.arg_i32(2));
            finish(context, parsed);
        });
    }
}

fn register_unsigned<U: Unit>(names: &[&str]) {
    for name in names {
        in_libc(name, |context| {
            let parsed = parse_unsigned::<U>(context.arg_ptr(0), context.arg_i32(2));
            finish(context, parsed);
        });
    }
}

pub fn register() {
    register_signed::<u8>(&["strtol", "strtoll", "strtoimax", "strtol_l", "strtoll_l"]);
    register_unsigned::<u8>(&[
        "strtoul",
        "strtoull",
        "strtoumax",
        "strtoul_l",
        "strtoull_l",
    ]);
    register_signed::<u32>(&["wcstol", "wcstoll", "wcstoimax"]);
    register_unsigned::<u32>(&["wcstoul", "wcstoull", "wcstoumax"]);
    for name in ["atoi", "atol", "atoll"] {
        in_libc(name, |context| {
            // cite: bionic strtol.cpp#atoi, atol, atoll
            let parsed = parse_signed::<u8>(context.arg_ptr(0), 10);
            if let Some(code) = parsed.errno {
                context.set_errno(code);
            }
            context.ret_u64(parsed.value);
        });
    }
}
