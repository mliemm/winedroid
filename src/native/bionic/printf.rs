use super::super::va_list::GuestVaList;
use super::errno::{guest_errno, message};
use super::fatal::fortify_fatal;
use super::guest_mem::{cstr_bytes, cstr_bytes_bounded};

#[derive(Default, Clone, Copy)]
struct Flags {
    left: bool,
    plus: bool,
    space: bool,
    alternate: bool,
    zero: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Length {
    Char,
    Short,
    Int,
    Long,
    Double,
}

struct Spec {
    flags: Flags,
    width: usize,
    precision: Option<usize>,
    length: Length,
}

struct Cursor<'a> {
    format: &'a [u8],
    index: usize,
}

impl Cursor<'_> {
    fn peek(&self) -> Option<u8> {
        self.format.get(self.index).copied()
    }

    fn take(&mut self) -> Option<u8> {
        let value = self.peek();
        self.index += 1;
        value
    }

    fn number(&mut self) -> usize {
        let mut value = 0usize;
        while let Some(digit) = self.peek().filter(u8::is_ascii_digit) {
            value = value.saturating_mul(10).saturating_add(usize::from(digit - b'0'));
            self.index += 1;
        }
        value
    }
}

fn parse_flags(cursor: &mut Cursor) -> Flags {
    let mut flags = Flags::default();
    loop {
        match cursor.peek() {
            Some(b'-') => flags.left = true,
            Some(b'+') => flags.plus = true,
            Some(b' ') => flags.space = true,
            Some(b'#') => flags.alternate = true,
            Some(b'0') => flags.zero = true,
            Some(b'\'') => {}
            _ => return flags,
        }
        cursor.index += 1;
    }
}

fn parse_spec(cursor: &mut Cursor, args: &mut GuestVaList) -> Result<Spec, String> {
    let mut flags = parse_flags(cursor);
    let mut width = 0usize;
    if cursor.peek() == Some(b'*') {
        cursor.index += 1;
        let requested = args.next_i32();
        flags.left |= requested < 0;
        width = requested.unsigned_abs() as usize;
    } else {
        width = width.max(cursor.number());
    }
    if cursor.peek() == Some(b'$') {
        return Err("printf positional argument; next missing primitive: printf positional arguments".to_string());
    }
    let mut precision = None;
    if cursor.peek() == Some(b'.') {
        cursor.index += 1;
        if cursor.peek() == Some(b'*') {
            cursor.index += 1;
            let requested = args.next_i32();
            precision = usize::try_from(requested).ok();
        } else {
            precision = Some(cursor.number());
        }
    }
    let mut length = Length::Int;
    loop {
        match cursor.peek() {
            Some(b'h') => length = if length == Length::Short { Length::Char } else { Length::Short },
            Some(b'l' | b'j' | b'z' | b't' | b'q') => length = Length::Long,
            Some(b'L') => length = Length::Double,
            Some(b'w') => {
                cursor.index += 1;
                length = match cursor.number() {
                    8 => Length::Char,
                    16 => Length::Short,
                    32 => Length::Int,
                    64 => Length::Long,
                    bits => fortify_fatal(&format!("%w{bits} is unsupported")),
                };
                continue;
            }
            _ => break,
        }
        cursor.index += 1;
    }
    Ok(Spec { flags, width, precision, length })
}

fn pad(out: &mut Vec<u8>, count: usize, byte: u8) {
    out.extend(std::iter::repeat_n(byte, count));
}

fn finish(out: &mut Vec<u8>, spec: &Spec, prefix: &[u8], digits: &[u8], zero_pad_allowed: bool) {
    let used = prefix.len() + digits.len();
    let fill = spec.width.saturating_sub(used);
    if spec.flags.left {
        out.extend_from_slice(prefix);
        out.extend_from_slice(digits);
        pad(out, fill, b' ');
    } else if spec.flags.zero && zero_pad_allowed {
        out.extend_from_slice(prefix);
        pad(out, fill, b'0');
        out.extend_from_slice(digits);
    } else {
        pad(out, fill, b' ');
        out.extend_from_slice(prefix);
        out.extend_from_slice(digits);
    }
}

fn digits_of(mut value: u64, base: u64, upper: bool) -> Vec<u8> {
    let table: &[u8; 16] = if upper { b"0123456789ABCDEF" } else { b"0123456789abcdef" };
    let mut digits = Vec::new();
    loop {
        digits.push(table[(value % base) as usize]);
        value /= base;
        if value == 0 {
            break;
        }
    }
    digits.reverse();
    digits
}

fn integer(out: &mut Vec<u8>, spec: &Spec, negative: bool, magnitude: u64, base: u64, upper: bool, signed: bool, always_prefix: bool) {
    // cite: bionic vfprintf.cpp#vfprintf
    let mut digits = if spec.precision == Some(0) && magnitude == 0 { Vec::new() } else { digits_of(magnitude, base, upper) };
    if let Some(minimum) = spec.precision {
        while digits.len() < minimum {
            digits.insert(0, b'0');
        }
    }
    let mut prefix = Vec::new();
    if signed {
        if negative {
            prefix.push(b'-');
        } else if spec.flags.plus {
            prefix.push(b'+');
        } else if spec.flags.space {
            prefix.push(b' ');
        }
    }
    if base == 8 && spec.flags.alternate && digits.first() != Some(&b'0') {
        digits.insert(0, b'0');
    }
    if base == 16 && (always_prefix || (spec.flags.alternate && magnitude != 0)) {
        prefix.extend_from_slice(if upper { b"0X" } else { b"0x" });
    }
    if base == 2 && spec.flags.alternate && magnitude != 0 {
        prefix.extend_from_slice(if upper { b"0B" } else { b"0b" });
    }
    finish(out, spec, &prefix, &digits, spec.precision.is_none());
}

fn signed_value(args: &mut GuestVaList, length: Length) -> i64 {
    match length {
        Length::Char => i64::from(args.next_i32() as i8),
        Length::Short => i64::from(args.next_i32() as i16),
        Length::Int => i64::from(args.next_i32()),
        _ => args.next_i64(),
    }
}

fn unsigned_value(args: &mut GuestVaList, length: Length) -> u64 {
    match length {
        Length::Char => u64::from(args.next_u32() as u8),
        Length::Short => u64::from(args.next_u32() as u16),
        Length::Int => u64::from(args.next_u32()),
        _ => args.next_i64() as u64,
    }
}

fn host_float(spec: &Spec, conversion: u8, value: f64) -> Vec<u8> {
    let mut format = String::from("%");
    for (enabled, letter) in [(spec.flags.left, '-'), (spec.flags.plus, '+'), (spec.flags.space, ' '), (spec.flags.alternate, '#'), (spec.flags.zero, '0')] {
        if enabled {
            format.push(letter);
        }
    }
    format.push_str(&spec.width.to_string());
    if let Some(precision) = spec.precision {
        format.push('.');
        format.push_str(&precision.to_string());
    }
    format.push(conversion as char);
    let format = std::ffi::CString::new(format).expect("no interior nul");
    let needed = unsafe { libc::snprintf(std::ptr::null_mut(), 0, format.as_ptr(), value) };
    let mut buffer = vec![0u8; needed.max(0) as usize + 1];
    unsafe { libc::snprintf(buffer.as_mut_ptr() as *mut libc::c_char, buffer.len(), format.as_ptr(), value) };
    buffer.truncate(needed.max(0) as usize);
    buffer
}

fn text(out: &mut Vec<u8>, spec: &Spec, bytes: &[u8]) {
    let used = spec.precision.map_or(bytes.len(), |limit| bytes.len().min(limit));
    finish(out, spec, &[], &bytes[..used], true);
}

fn string(out: &mut Vec<u8>, spec: &Spec, pointer: usize) {
    // cite: bionic vfprintf.cpp#vfprintf
    match (pointer, spec.precision) {
        (0, _) => text(out, spec, b"(null)"),
        (_, Some(limit)) => text(out, spec, cstr_bytes_bounded(pointer, limit)),
        (_, None) => text(out, spec, cstr_bytes(pointer)),
    }
}

fn long_alias(spec: &mut Spec, conversion: u8) -> u8 {
    spec.length = Length::Long;
    conversion
}

pub fn format(format: &[u8], args: &mut GuestVaList) -> Result<Vec<u8>, String> {
    // cite: bionic vfprintf.cpp#vfprintf
    let mut out = Vec::with_capacity(format.len() + 16);
    let mut cursor = Cursor { format, index: 0 };
    while let Some(byte) = cursor.take() {
        if byte != b'%' {
            out.push(byte);
            continue;
        }
        let mut spec = parse_spec(&mut cursor, args)?;
        let Some(conversion) = cursor.take() else { break };
        let conversion = match conversion {
            b'D' => long_alias(&mut spec, b'd'),
            b'O' => long_alias(&mut spec, b'o'),
            b'U' => long_alias(&mut spec, b'u'),
            b'C' => long_alias(&mut spec, b'c'),
            b'S' => long_alias(&mut spec, b's'),
            other => other,
        };
        match conversion {
            b'd' | b'i' => {
                let value = signed_value(args, spec.length);
                integer(&mut out, &spec, value < 0, value.unsigned_abs(), 10, false, true, false);
            }
            b'u' => integer(&mut out, &spec, false, unsigned_value(args, spec.length), 10, false, false, false),
            b'o' => integer(&mut out, &spec, false, unsigned_value(args, spec.length), 8, false, false, false),
            b'b' => integer(&mut out, &spec, false, unsigned_value(args, spec.length), 2, false, false, false),
            b'B' => integer(&mut out, &spec, false, unsigned_value(args, spec.length), 2, true, false, false),
            b'x' => integer(&mut out, &spec, false, unsigned_value(args, spec.length), 16, false, false, false),
            b'X' => integer(&mut out, &spec, false, unsigned_value(args, spec.length), 16, true, false, false),
            b'p' => integer(&mut out, &spec, false, args.next_ptr() as u64, 16, false, false, true),
            b'c' => {
                if spec.length == Length::Long {
                    return Err("printf %lc; next missing primitive: printf wide characters".to_string());
                }
                let character = [args.next_i32() as u8];
                finish(&mut out, &spec, &[], &character, true);
            }
            b's' => {
                if spec.length == Length::Long {
                    return Err("printf %ls; next missing primitive: printf wide strings".to_string());
                }
                string(&mut out, &spec, args.next_ptr());
            }
            b'm' => {
                let message = message(guest_errno()).map_or_else(|| format!("Unknown error {}", guest_errno()), str::to_string);
                text(&mut out, &spec, message.as_bytes());
            }
            b'e' | b'E' | b'f' | b'F' | b'g' | b'G' | b'a' | b'A' => {
                if spec.length == Length::Double {
                    return Err("printf long double; next missing primitive: binary128 printf conversion".to_string());
                }
                out.extend(host_float(&spec, conversion, args.next_f64()));
            }
            b'n' => fortify_fatal("%n not allowed on Android"),
            0 => break,
            other => finish(&mut out, &spec, &[], &[other], true),
        }
    }
    Ok(out)
}
