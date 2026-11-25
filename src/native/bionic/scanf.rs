use std::sync::Arc;

use super::super::bridge::CallContext;
use super::super::va_list::GuestVaList;
use super::abi_types::{EINVAL, ERANGE};
use super::export::in_libc;
use super::fs_result::missing;
use super::guest_mem::{cstr_bytes, write_value};
use super::stream::{self, Stream};

const EOF_VALUE: i64 = -1;

enum Source {
    Stream(Arc<Stream>),
    Memory { base: usize, length: usize },
}

struct Scanner {
    source: Source,
    position: usize,
    pushback: Vec<u8>,
    consumed: usize,
}

impl Scanner {
    fn stream(stream: Arc<Stream>) -> Self {
        Self { source: Source::Stream(stream), position: 0, pushback: Vec::new(), consumed: 0 }
    }

    fn memory(base: usize, length: usize) -> Self {
        Self { source: Source::Memory { base, length }, position: 0, pushback: Vec::new(), consumed: 0 }
    }

    fn next(&mut self) -> Option<u8> {
        if let Some(byte) = self.pushback.pop() {
            self.consumed += 1;
            return Some(byte);
        }
        let byte = match &mut self.source {
            Source::Stream(stream) => stream::read_byte(stream).ok().flatten(),
            Source::Memory { base, length } => {
                let position = self.position;
                if position >= *length {
                    None
                } else {
                    Some(unsafe { std::ptr::read((*base + position) as *const u8) })
                }
            }
        };
        if let Source::Memory { .. } = self.source {
            if byte.is_some() {
                self.position += 1;
            }
        }
        if byte.is_some() {
            self.consumed += 1;
        }
        byte
    }

    fn unread(&mut self, byte: u8) {
        self.pushback.push(byte);
        self.consumed = self.consumed.saturating_sub(1);
    }

    fn skip_space(&mut self) {
        while let Some(byte) = self.next() {
            if !byte.is_ascii_whitespace() {
                self.unread(byte);
                return;
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Conversion {
    suppressed: bool,
    width: Option<usize>,
    length: i32,
    kind: u8,
}

fn parse_conversion(cursor: &mut usize, format: &[u8]) -> Conversion {
    let mut conversion = Conversion { suppressed: false, width: None, length: 0, kind: 0 };
    if format.get(*cursor) == Some(&b'*') {
        conversion.suppressed = true;
        *cursor += 1;
    }
    let start = *cursor;
    while format.get(*cursor).is_some_and(|byte| byte.is_ascii_digit()) {
        *cursor += 1;
    }
    if *cursor > start {
        conversion.width = std::str::from_utf8(&format[start..*cursor]).ok().and_then(|text| text.parse().ok());
    }
    conversion.length = match format.get(*cursor) {
        Some(b'h') => {
            *cursor += 1;
            if format.get(*cursor) == Some(&b'h') {
                *cursor += 1;
                -2
            } else {
                -1
            }
        }
        Some(b'l') => {
            *cursor += 1;
            if format.get(*cursor) == Some(&b'l') {
                *cursor += 1;
                2
            } else {
                1
            }
        }
        Some(b'j' | b'z' | b't' | b'q') => {
            *cursor += 1;
            2
        }
        Some(b'L') => {
            *cursor += 1;
            3
        }
        _ => 0,
    };
    if let Some(byte) = format.get(*cursor) {
        conversion.kind = *byte;
        *cursor += 1;
    }
    conversion
}

fn store_integer(pointer: usize, length: i32, signed: bool, magnitude: u128, negative: bool) {
    if signed {
        let value = if negative { -(magnitude as i128) } else { magnitude as i128 };
        match length {
            -2 => write_value::<i8>(pointer, value as i8),
            -1 => write_value::<i16>(pointer, value as i16),
            0 => write_value::<i32>(pointer, value as i32),
            _ => write_value::<i64>(pointer, value as i64),
        }
    } else {
        let value = if negative { (magnitude as i128).wrapping_neg() as u128 } else { magnitude };
        match length {
            -2 => write_value::<u8>(pointer, value as u8),
            -1 => write_value::<u16>(pointer, value as u16),
            0 => write_value::<u32>(pointer, value as u32),
            _ => write_value::<u64>(pointer, value as u64),
        }
    }
}

fn scanned_integer(scanner: &mut Scanner, conversion: Conversion, pointer: usize, signed: bool) -> Option<bool> {
    scanner.skip_space();
    let limit = conversion.width.unwrap_or(usize::MAX);
    let mut negative = false;
    let mut sign_seen = false;
    let mut value: u128 = 0;
    let mut radix = match conversion.kind {
        b'o' => 8,
        b'x' | b'X' | b'p' => 16,
        b'i' => 0,
        _ => 10,
    };
    let mut digits = 0usize;
    let mut has_digit = false;
    while digits < limit {
        let Some(byte) = scanner.next() else { break };
        if !sign_seen && digits == 0 && (byte == b'+' || byte == b'-') {
            negative = byte == b'-';
            sign_seen = true;
            continue;
        }
        if radix == 0 {
            if byte == b'0' {
                let Some(following) = scanner.next() else {
                    value = 0;
                    has_digit = true;
                    break;
                };
                if following == b'x' || following == b'X' {
                    radix = 16;
                    digits += 2;
                    continue;
                }
                radix = 8;
                scanner.unread(following);
                has_digit = true;
                digits += 1;
                continue;
            }
            radix = 10;
        }
        let Some(digit) = (byte as char).to_digit(radix as u32) else {
            scanner.unread(byte);
            break;
        };
        value = value.saturating_mul(radix as u128).saturating_add(u128::from(digit));
        has_digit = true;
        digits += 1;
    }
    if !has_digit {
        return None;
    }
    if !conversion.suppressed {
        if value > u128::from(u64::MAX) {
            super::errno::set_guest_errno(ERANGE);
        }
        let value = value.min(u128::from(u64::MAX));
        if conversion.kind == b'p' {
            write_value::<u64>(pointer, value as u64);
        } else {
            store_integer(pointer, conversion.length, signed, value, negative);
        }
    }
    Some(true)
}

fn scanned_float(scanner: &mut Scanner, conversion: Conversion, pointer: usize) -> Option<bool> {
    scanner.skip_space();
    let limit = conversion.width.unwrap_or(usize::MAX);
    let mut collected = Vec::new();
    while collected.len() < limit {
        let Some(byte) = scanner.next() else { break };
        let plausible = byte.is_ascii_digit() || matches!(byte, b'+' | b'-' | b'.' | b'x' | b'X' | b'p' | b'P' | b'a'..=b'f' | b'A'..=b'F' | b'i' | b'n' | b't' | b'y');
        if !plausible {
            scanner.unread(byte);
            break;
        }
        collected.push(byte);
    }
    if collected.is_empty() {
        return None;
    }
    let mut text = collected.clone();
    text.push(0);
    let mut end: *mut libc::c_char = std::ptr::null_mut();
    let value = unsafe { libc::strtod(text.as_ptr() as *const libc::c_char, &mut end) };
    let parsed = (end as usize).saturating_sub(text.as_ptr() as usize);
    if parsed == 0 {
        for byte in collected.iter().rev() {
            scanner.unread(*byte);
        }
        return None;
    }
    for byte in collected[parsed..].iter().rev() {
        scanner.unread(*byte);
    }
    if !conversion.suppressed {
        if conversion.length == 1 {
            write_value::<f64>(pointer, value);
        } else if conversion.length == 3 {
            missing("scanf", "%Lf needs a binary128 conversion the host libc does not expose");
            return None;
        } else {
            write_value::<f32>(pointer, value as f32);
        }
    }
    Some(true)
}

fn scanned_string(scanner: &mut Scanner, conversion: Conversion, pointer: usize) -> Option<bool> {
    scanner.skip_space();
    let limit = conversion.width.unwrap_or(usize::MAX);
    let mut written = 0usize;
    while written < limit {
        let Some(byte) = scanner.next() else { break };
        if byte.is_ascii_whitespace() {
            scanner.unread(byte);
            break;
        }
        write_value::<u8>(pointer + written, byte);
        written += 1;
    }
    if written == 0 {
        return None;
    }
    write_value::<u8>(pointer + written, 0);
    Some(true)
}

fn scanset(format: &[u8], cursor: &mut usize) -> [u64; 4] {
    let mut set = [0u64; 4];
    let mut negate = false;
    if format.get(*cursor) == Some(&b'^') {
        negate = true;
        *cursor += 1;
    }
    let mut first = true;
    while let Some(byte) = format.get(*cursor) {
        if *byte == b']' && !first {
            *cursor += 1;
            break;
        }
        first = false;
        if format.get(*cursor + 1) == Some(&b'-') && format.get(*cursor + 2).is_some_and(|end| *end != b']') {
            let end = format[*cursor + 2];
            let mut character = *byte;
            while character <= end {
                set[(character / 64) as usize] |= 1u64 << (character % 64);
                character += 1;
            }
            *cursor += 3;
            continue;
        }
        set[(*byte / 64) as usize] |= 1u64 << (*byte % 64);
        *cursor += 1;
    }
    if negate {
        for word in set.iter_mut() {
            *word = !*word;
        }
        set[0] &= !(1u64 << b'\n');
    }
    set
}

fn scanned_set(scanner: &mut Scanner, set: &[u64; 4], width: Option<usize>, pointer: usize) -> Option<bool> {
    let limit = width.unwrap_or(usize::MAX);
    let mut written = 0usize;
    while written < limit {
        let Some(byte) = scanner.next() else { break };
        if set[(byte / 64) as usize] & (1u64 << (byte % 64)) == 0 {
            scanner.unread(byte);
            break;
        }
        write_value::<u8>(pointer + written, byte);
        written += 1;
    }
    if written == 0 {
        return None;
    }
    write_value::<u8>(pointer + written, 0);
    Some(true)
}

fn scanned_chars(scanner: &mut Scanner, width: Option<usize>, pointer: usize) -> Option<bool> {
    let count = width.unwrap_or(1);
    for index in 0..count {
        let Some(byte) = scanner.next() else {
            if index == 0 {
                return None;
            }
            break;
        };
        write_value::<u8>(pointer + index, byte);
    }
    Some(true)
}

fn run(format: &[u8], scanner: &mut Scanner, args: &mut GuestVaList) -> Result<i64, String> {
    let mut assigned = 0i64;
    let mut cursor = 0usize;
    while cursor < format.len() {
        let byte = format[cursor];
        cursor += 1;
        if byte.is_ascii_whitespace() {
            scanner.skip_space();
            continue;
        }
        if byte != b'%' {
            let Some(input) = scanner.next() else {
                return Ok(if assigned == 0 { EOF_VALUE } else { assigned });
            };
            if input != byte {
                scanner.unread(input);
                return Ok(assigned);
            }
            continue;
        }
        let conversion = parse_conversion(&mut cursor, format);
        if conversion.kind == 0 {
            break;
        }
        let pointer = if !conversion.suppressed && conversion.kind != b'%' { args.next_ptr() } else { 0 };
        let mut scratch = if conversion.suppressed { vec![0u8; conversion.width.unwrap_or(64).clamp(1, 1 << 20)] } else { Vec::new() };
        let target = if conversion.suppressed { scratch.as_mut_ptr() as usize } else { pointer };
        let outcome = match conversion.kind {
            b'%' => {
                let Some(input) = scanner.next() else { return Ok(if assigned == 0 { EOF_VALUE } else { assigned }) };
                if input != b'%' {
                    scanner.unread(input);
                    return Ok(assigned);
                }
                None
            }
            b'n' => {
                if !conversion.suppressed && pointer != 0 {
                    store_integer(pointer, conversion.length, true, scanner.consumed as u128, false);
                }
                None
            }
            b'd' | b'i' | b'o' | b'u' | b'x' | b'X' | b'p' => scanned_integer(scanner, conversion, target, matches!(conversion.kind, b'd' | b'i')),
            b'e' | b'E' | b'f' | b'F' | b'g' | b'G' | b'a' | b'A' => scanned_float(scanner, conversion, target),
            b's' => {
                if conversion.length >= 1 {
                    return Err(format!("scanf %ls; next missing primitive: wide scanf strings"));
                }
                scanned_string(scanner, conversion, target)
            }
            b'c' => {
                if conversion.length >= 1 {
                    return Err(format!("scanf %lc; next missing primitive: wide scanf characters"));
                }
                scanned_chars(scanner, conversion.width, target)
            }
            b'[' => {
                if conversion.length >= 1 {
                    return Err(format!("scanf %l[; next missing primitive: wide scanf sets"));
                }
                let set = scanset(format, &mut cursor);
                scanned_set(scanner, &set, conversion.width, target)
            }
            other => {
                let Some(input) = scanner.next() else { return Ok(if assigned == 0 { EOF_VALUE } else { assigned }) };
                if input != other {
                    scanner.unread(input);
                    return Ok(assigned);
                }
                None
            }
        };
        match outcome {
            Some(_) if !conversion.suppressed => assigned += 1,
            Some(_) => {}
            None if conversion.kind == b'%' || conversion.kind == b'n' => {}
            None => return Ok(assigned),
        }
    }
    Ok(assigned)
}

fn scan_stream(context: &mut CallContext, file: usize, format_pointer: usize, args: GuestVaList) {
    let stream = match stream::of(file) {
        Ok(stream) => stream,
        Err(errno) => {
            context.set_errno(errno);
            context.ret_i32(-1);
            return;
        }
    };
    let mut scanner = Scanner::stream(stream);
    let mut args = args;
    match run(cstr_bytes(format_pointer), &mut scanner, &mut args) {
        Ok(value) => context.ret_i64(value),
        Err(message) => context.fail(format!("{} {message}", context.symbol())),
    }
}

fn scan_memory(context: &mut CallContext, source: usize, format_pointer: usize, args: GuestVaList) {
    if source == 0 {
        context.set_errno(super::abi_types::EFAULT);
        context.ret_i32(-1);
        return;
    }
    let mut scanner = Scanner::memory(source, cstr_bytes(source).len());
    let mut args = args;
    match run(cstr_bytes(format_pointer), &mut scanner, &mut args) {
        Ok(value) => context.ret_i64(value),
        Err(message) => context.fail(format!("{} {message}", context.symbol())),
    }
}

fn guest_list(context: &mut CallContext, index: usize) -> Option<GuestVaList<'static>> {
    match context.va_list_arg(index) {
        Ok(list) => Some(list),
        Err(error) => {
            context.fail(error.to_string());
            None
        }
    }
}

pub fn register() {
    in_libc("scanf", |context| {
        let regs = *context.regs();
        let args = GuestVaList::from_regs(&regs, 1, 0);
        let format = context.arg_ptr(0);
        scan_stream(context, super::stdio_files::standard(0), format, args);
    });
    in_libc("fscanf", |context| {
        let file = context.arg_ptr(0);
        let format = context.arg_ptr(1);
        let regs = *context.regs();
        let args = GuestVaList::from_regs(&regs, 2, 0);
        scan_stream(context, file, format, args);
    });
    in_libc("sscanf", |context| {
        let source = context.arg_ptr(0);
        let format = context.arg_ptr(1);
        let regs = *context.regs();
        let args = GuestVaList::from_regs(&regs, 2, 0);
        scan_memory(context, source, format, args);
    });
    in_libc("vscanf", |context| {
        let format = context.arg_ptr(0);
        if let Some(args) = guest_list(context, 1) {
            scan_stream(context, super::stdio_files::standard(0), format, args);
        }
    });
    in_libc("vfscanf", |context| {
        let (file, format) = (context.arg_ptr(0), context.arg_ptr(1));
        if let Some(args) = guest_list(context, 2) {
            scan_stream(context, file, format, args);
        }
    });
    in_libc("vsscanf", |context| {
        let (source, format) = (context.arg_ptr(0), context.arg_ptr(1));
        if let Some(args) = guest_list(context, 2) {
            scan_memory(context, source, format, args);
        }
    });
    for name in ["swscanf", "fwscanf", "wscanf", "vswscanf", "vfwscanf", "vwscanf"] {
        in_libc(name, move |context: &mut CallContext| {
            missing(name, "wide scanf needs the wide character source pipeline");
            context.set_errno(EINVAL);
            context.ret_i32(-1);
        });
    }
}
