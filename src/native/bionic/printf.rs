use super::super::va_list::GuestVaList;
use super::errno::{guest_errno, message};
use super::fatal::fortify_fatal;
use super::guest_mem::{cstr_bytes, cstr_bytes_bounded, read_value};

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

#[derive(Clone, Copy)]
struct Spec {
    flags: Flags,
    width: usize,
    precision: Option<usize>,
    length: Length,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum ValueKind {
    Int,
    Long,
    Double,
    Pointer,
}

#[derive(Clone, Copy)]
enum Value {
    Int(i32),
    Long(u64),
    Double(f64),
    Pointer(usize),
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

fn parse_length(cursor: &mut Cursor) -> Length {
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
    length
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
        return Err("printf positional argument mixed into a sequential format; next missing primitive: printf positional arguments".to_string());
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
    Ok(Spec { flags, width, precision, length: parse_length(cursor) })
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

fn encode_utf8(code: u32) -> Vec<u8> {
    match char::from_u32(code) {
        Some(character) => character.to_string().into_bytes(),
        None => "\u{fffd}".to_string().into_bytes(),
    }
}

fn wide_string(pointer: usize, precision: Option<usize>) -> Vec<u8> {
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
        let bytes = encode_utf8(code);
        if precision.is_some_and(|limit| out.len() + bytes.len() > limit) {
            break;
        }
        out.extend_from_slice(&bytes);
        offset += 4;
    }
    out
}

fn long_alias(spec: &mut Spec, conversion: u8) -> u8 {
    spec.length = Length::Long;
    conversion
}

fn value_kind(conversion: u8, length: Length) -> Option<ValueKind> {
    match conversion {
        b'd' | b'i' | b'u' | b'o' | b'x' | b'X' | b'b' | b'B' => Some(if matches!(length, Length::Char | Length::Short | Length::Int) { ValueKind::Int } else { ValueKind::Long }),
        b'c' => Some(if length == Length::Long { ValueKind::Long } else { ValueKind::Int }),
        b's' | b'p' | b'n' => Some(ValueKind::Pointer),
        b'e' | b'E' | b'f' | b'F' | b'g' | b'G' | b'a' | b'A' => Some(ValueKind::Double),
        _ => None,
    }
}

fn fetch(args: &mut GuestVaList, kind: ValueKind) -> Value {
    match kind {
        ValueKind::Int => Value::Int(args.next_i32()),
        ValueKind::Long => Value::Long(args.next_gp()),
        ValueKind::Double => Value::Double(args.next_f64()),
        ValueKind::Pointer => Value::Pointer(args.next_ptr()),
    }
}

fn signed_of(value: Value, length: Length) -> i64 {
    match value {
        Value::Int(number) => match length {
            Length::Char => i64::from(number as i8),
            Length::Short => i64::from(number as i16),
            _ => i64::from(number),
        },
        Value::Long(number) => number as i64,
        Value::Double(_) | Value::Pointer(_) => 0,
    }
}

fn unsigned_of(value: Value, length: Length) -> u64 {
    match value {
        Value::Int(number) => match length {
            Length::Char => u64::from(number as u8),
            Length::Short => u64::from(number as u16),
            _ => u64::from(number as u32),
        },
        Value::Long(number) => number,
        Value::Double(_) | Value::Pointer(_) => 0,
    }
}

fn render(out: &mut Vec<u8>, spec: &Spec, conversion: u8, value: Option<Value>) -> Result<(), String> {
    match conversion {
        b'd' | b'i' => {
            let number = signed_of(value.unwrap_or(Value::Int(0)), spec.length);
            integer(out, spec, number < 0, number.unsigned_abs(), 10, false, true, false);
        }
        b'u' => integer(out, spec, false, unsigned_of(value.unwrap_or(Value::Int(0)), spec.length), 10, false, false, false),
        b'o' => integer(out, spec, false, unsigned_of(value.unwrap_or(Value::Int(0)), spec.length), 8, false, false, false),
        b'b' => integer(out, spec, false, unsigned_of(value.unwrap_or(Value::Int(0)), spec.length), 2, false, false, false),
        b'B' => integer(out, spec, false, unsigned_of(value.unwrap_or(Value::Int(0)), spec.length), 2, true, false, false),
        b'x' => integer(out, spec, false, unsigned_of(value.unwrap_or(Value::Int(0)), spec.length), 16, false, false, false),
        b'X' => integer(out, spec, false, unsigned_of(value.unwrap_or(Value::Int(0)), spec.length), 16, true, false, false),
        b'p' => string_of_pointer(out, spec, value),
        b'c' => {
            if spec.length == Length::Long {
                let code = match value {
                    Some(Value::Long(code)) => code as u32,
                    Some(Value::Int(code)) => code as u32,
                    _ => 0,
                };
                finish(out, spec, &[], &encode_utf8(code), true);
            } else {
                let byte = match value {
                    Some(Value::Int(code)) => code as u8,
                    _ => 0,
                };
                finish(out, spec, &[], &[byte], true);
            }
        }
        b's' => match value {
            Some(Value::Pointer(pointer)) if spec.length == Length::Long => {
                let bytes = wide_string(pointer, spec.precision);
                let mut plain = *spec;
                plain.precision = None;
                text(out, &plain, &bytes);
            }
            Some(Value::Pointer(pointer)) => string(out, spec, pointer),
            _ => string(out, spec, 0),
        },
        b'm' => {
            let code = guest_errno();
            let message = message(code).map_or_else(|| format!("Unknown error {code}"), str::to_string);
            text(out, spec, message.as_bytes());
        }
        b'e' | b'E' | b'f' | b'F' | b'g' | b'G' | b'a' | b'A' => {
            if spec.length == Length::Double {
                return Err("printf long double; next missing primitive: binary128 printf conversion".to_string());
            }
            let number = match value {
                Some(Value::Double(number)) => number,
                _ => 0.0,
            };
            out.extend(host_float(spec, conversion, number));
        }
        b'n' => fortify_fatal("%n not allowed on Android"),
        0 => {}
        other => finish(out, spec, &[], &[other], true),
    }
    Ok(())
}

fn string_of_pointer(out: &mut Vec<u8>, spec: &Spec, value: Option<Value>) {
    let pointer = match value {
        Some(Value::Pointer(pointer)) => pointer,
        _ => 0,
    };
    integer(out, spec, false, pointer as u64, 16, false, false, true);
}

fn has_positional_spec(format: &[u8]) -> bool {
    let mut index = 0usize;
    while index < format.len() {
        if format[index] != b'%' {
            index += 1;
            continue;
        }
        index += 1;
        while index < format.len() && matches!(format[index], b'-' | b'+' | b' ' | b'#' | b'0' | b'\'') {
            index += 1;
        }
        let start = index;
        while index < format.len() && format[index].is_ascii_digit() {
            index += 1;
        }
        if index > start && index < format.len() && format[index] == b'$' {
            return true;
        }
        while index < format.len() && format[index] != b'%' && !format[index].is_ascii_alphabetic() && format[index] != b'%' {
            if format[index] == b'$' && index > start {
                return true;
            }
            index += 1;
        }
        index += 1;
    }
    false
}

struct Item {
    literal: Vec<u8>,
    spec: Option<Spec>,
    conversion: u8,
    width_position: Option<usize>,
    precision_position: Option<usize>,
    value_position: Option<usize>,
}

fn assign(kinds: &mut Vec<Option<ValueKind>>, position: Option<usize>, kind: ValueKind, next: &mut usize) -> Result<usize, String> {
    let position = match position {
        Some(position) => position,
        None => {
            *next += 1;
            *next
        }
    };
    if position == 0 {
        return Err("printf positional argument 0; next missing primitive: printf positional arguments".to_string());
    }
    if kinds.len() < position {
        kinds.resize(position, None);
    }
    match kinds[position - 1] {
        None => kinds[position - 1] = Some(kind),
        Some(existing) if existing == kind => {}
        Some(_) => return Err("printf positional argument reused with a different type".to_string()),
    }
    Ok(position)
}

fn read_index(cursor: &mut Cursor, format: &[u8]) -> Option<usize> {
    let start = cursor.index;
    while cursor.peek().is_some_and(|byte| byte.is_ascii_digit()) {
        cursor.index += 1;
    }
    if cursor.index > start && cursor.peek() == Some(b'$') {
        let text = std::str::from_utf8(&format[start..cursor.index]).unwrap_or("0");
        cursor.index += 1;
        return text.parse::<usize>().ok();
    }
    cursor.index = start;
    None
}

fn parse_items(format: &[u8], kinds: &mut Vec<Option<ValueKind>>, next: &mut usize) -> Result<Vec<Item>, String> {
    let mut items = Vec::new();
    let mut literal = Vec::new();
    let mut cursor = Cursor { format, index: 0 };
    while let Some(byte) = cursor.take() {
        if byte != b'%' {
            literal.push(byte);
            continue;
        }
        let value_index = read_index(&mut cursor, format);
        let flags = parse_flags(&mut cursor);
        let mut width_position = None;
        let mut fixed_width = 0usize;
        if cursor.peek() == Some(b'*') {
            cursor.index += 1;
            let explicit = read_index(&mut cursor, format);
            width_position = Some(match explicit {
                Some(position) => assign(kinds, Some(position), ValueKind::Int, next)?,
                None => assign(kinds, None, ValueKind::Int, next)?,
            });
        } else {
            fixed_width = cursor.number();
        }
        let mut precision_position = None;
        let mut fixed_precision = None;
        if cursor.peek() == Some(b'.') {
            cursor.index += 1;
            if cursor.peek() == Some(b'*') {
                cursor.index += 1;
                let explicit = read_index(&mut cursor, format);
                precision_position = Some(match explicit {
                    Some(position) => assign(kinds, Some(position), ValueKind::Int, next)?,
                    None => assign(kinds, None, ValueKind::Int, next)?,
                });
            } else {
                fixed_precision = Some(cursor.number());
            }
        }
        let length = parse_length(&mut cursor);
        let Some(conversion) = cursor.take() else { break };
        let (conversion, long) = match conversion {
            b'D' => (b'd', true),
            b'O' => (b'o', true),
            b'U' => (b'u', true),
            b'C' => (b'c', true),
            b'S' => (b's', true),
            other => (other, false),
        };
        let mut spec = Spec { flags, width: fixed_width, precision: fixed_precision, length };
        if long {
            spec.length = Length::Long;
        }
        let value_position = match value_kind(conversion, spec.length) {
            Some(kind) => Some(assign(kinds, value_index, kind, next)?),
            None => None,
        };
        if !literal.is_empty() {
            items.push(Item { literal: std::mem::take(&mut literal), spec: None, conversion: 0, width_position: None, precision_position: None, value_position: None });
        }
        items.push(Item { literal: Vec::new(), spec: Some(spec), conversion, width_position, precision_position, value_position });
    }
    if !literal.is_empty() {
        items.push(Item { literal, spec: None, conversion: 0, width_position: None, precision_position: None, value_position: None });
    }
    Ok(items)
}

fn positional_value(values: &[Value], position: usize, kind: ValueKind) -> Result<Value, String> {
    match values.get(position.wrapping_sub(1)) {
        Some(Value::Int(number)) => Ok(Value::Int(*number)),
        Some(Value::Long(number)) => Ok(Value::Long(*number)),
        Some(Value::Double(number)) => Ok(Value::Double(*number)),
        Some(Value::Pointer(pointer)) => Ok(Value::Pointer(*pointer)),
        None => Err(format!("printf positional argument {position} is out of range; next missing primitive: printf positional arguments for {kind:?}")),
    }
}

fn format_positional(format: &[u8], args: &mut GuestVaList) -> Result<Vec<u8>, String> {
    let mut kinds = Vec::new();
    let mut next = 0usize;
    let items = parse_items(format, &mut kinds, &mut next)?;
    if kinds.iter().any(Option::is_none) {
        return Err("printf positional arguments skip a position; next missing primitive: printf positional arguments".to_string());
    }
    let mut cursor = *args;
    let mut values = Vec::with_capacity(kinds.len());
    for kind in &kinds {
        values.push(fetch(&mut cursor, kind.unwrap_or(ValueKind::Int)));
    }
    let mut out = Vec::with_capacity(format.len() + 16);
    for item in items {
        let Some(mut spec) = item.spec else {
            out.extend_from_slice(&item.literal);
            continue;
        };
        if let Some(position) = item.width_position {
            let width = positional_value(&values, position, ValueKind::Int)?;
            let requested = match width {
                Value::Int(number) => number,
                _ => 0,
            };
            spec.flags.left |= requested < 0;
            spec.width = requested.unsigned_abs() as usize;
        }
        if let Some(position) = item.precision_position {
            let precision = positional_value(&values, position, ValueKind::Int)?;
            spec.precision = match precision {
                Value::Int(number) => usize::try_from(number).ok(),
                _ => None,
            };
        }
        let value = match item.value_position {
            Some(position) => Some(positional_value(&values, position, ValueKind::Int)?),
            None => None,
        };
        render(&mut out, &spec, item.conversion, value)?;
    }
    Ok(out)
}

pub fn format(format: &[u8], args: &mut GuestVaList) -> Result<Vec<u8>, String> {
    // cite: bionic vfprintf.cpp#vfprintf
    if has_positional_spec(format) {
        return format_positional(format, args);
    }
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
        let value = value_kind(conversion, spec.length).map(|kind| fetch(args, kind));
        render(&mut out, &spec, conversion, value)?;
    }
    Ok(out)
}
