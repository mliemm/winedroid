use super::abi_types::{ERANGE, TM_GMTOFF_OFFSET, TM_ZONE_OFFSET};
use super::civil::{read_tm, BrokenDown};
use super::export::in_libc;
use super::guest_mem::{cstr_bytes, read_value, write_cstr_truncated};
use super::tz_state::mktime_copy;

const WEEKDAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const TIME_C: &[u8] = b"%a %b %e %H:%M:%S %Y";
const TIME_X: &[u8] = b"%m/%d/%y";
const TIME_BIG_X: &[u8] = b"%H:%M:%S";
const TIME_DATE: &[u8] = b"%a %b %e %H:%M:%S %Z %Y";
const FORCE_LOWER: u8 = 1;

pub struct Tm {
    pub parts: BrokenDown,
    pub is_dst: i32,
    pub offset: i64,
    pub zone: Vec<u8>,
    pub address: usize,
}

pub fn read_full_tm(address: usize) -> Tm {
    let zone_pointer = read_value::<usize>(address + TM_ZONE_OFFSET);
    Tm {
        parts: read_tm(address),
        is_dst: read_value::<i32>(address + 32),
        offset: read_value::<i64>(address + TM_GMTOFF_OFFSET),
        zone: if zone_pointer == 0 { Vec::new() } else { cstr_bytes(zone_pointer).to_vec() },
        address,
    }
}

fn leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn add(out: &mut Vec<u8>, text: &[u8], modifier: u8) {
    // cite: bionic strftime.c#_add
    for byte in text {
        out.push(match modifier {
            FORCE_LOWER => byte.to_ascii_lowercase(),
            b'^' => byte.to_ascii_uppercase(),
            b'#' if byte.is_ascii_uppercase() => byte.to_ascii_lowercase(),
            b'#' if byte.is_ascii_lowercase() => byte.to_ascii_uppercase(),
            _ => *byte,
        });
    }
}

fn convert(out: &mut Vec<u8>, number: i64, fill: u8, width: usize) {
    // cite: bionic strftime.c#_conv
    let digits = number.to_string();
    for _ in digits.len()..width {
        out.push(fill);
    }
    out.extend_from_slice(digits.as_bytes());
}

fn shape(modifier: u8, normal: (u8, usize)) -> (u8, usize) {
    // cite: bionic strftime.c#getformat
    match modifier {
        b'_' => (b' ', normal.1),
        b'-' => (b' ', 0),
        b'0' => (b'0', normal.1),
        _ => normal,
    }
}

fn year_text(out: &mut Vec<u8>, year: i64, base: i64, top: bool, low: bool, modifier: u8) {
    // cite: bionic strftime.c#_yconv
    let mut trail = year % 100 + base % 100;
    let mut lead = year / 100 + base / 100 + trail / 100;
    trail %= 100;
    if trail < 0 && lead > 0 {
        trail += 100;
        lead -= 1;
    } else if lead < 0 && trail > 0 {
        trail -= 100;
        lead += 1;
    }
    let (fill, width) = shape(modifier, (b'0', 2));
    if top {
        if lead == 0 && trail < 0 {
            add(out, b"-0", modifier);
        } else {
            convert(out, lead, fill, width);
        }
    }
    if low {
        convert(out, trail.abs(), fill, width);
    }
}

fn iso_week(tm: &Tm, selector: u8, out: &mut Vec<u8>, modifier: u8) {
    // cite: bionic strftime.c#strftime
    let (year, mut base) = (i64::from(tm.parts.year), 1900i64);
    let (mut yday, wday) = (i64::from(tm.parts.yearday), i64::from(tm.parts.weekday));
    let week;
    loop {
        let length = if leap(year + base) { 366 } else { 365 };
        let bottom = ((yday + 11 - wday) % 7) - 3;
        let mut top = bottom - (length % 7);
        if top < -3 {
            top += 7;
        }
        top += length;
        if yday >= top {
            base += 1;
            week = 1;
            break;
        }
        if yday >= bottom {
            week = 1 + (yday - bottom) / 7;
            break;
        }
        base -= 1;
        yday += if leap(year + base) { 366 } else { 365 };
    }
    let (fill, width) = shape(modifier, (b'0', 2));
    match selector {
        b'V' => convert(out, week, fill, width),
        b'g' => year_text(out, year, base, false, true, modifier),
        _ => year_text(out, year, base, true, true, modifier),
    }
}

fn numeric(out: &mut Vec<u8>, value: i64, modifier: u8, normal: (u8, usize)) {
    let (fill, width) = shape(modifier, normal);
    convert(out, value, fill, width);
}

fn emit(format: &[u8], tm: &Tm, out: &mut Vec<u8>) {
    // cite: bionic strftime.c#_fmt
    let mut index = 0;
    while index < format.len() {
        let byte = format[index];
        index += 1;
        if byte != b'%' {
            out.push(byte);
            continue;
        }
        let mut modifier = 0u8;
        loop {
            let Some(&conversion) = format.get(index) else { return };
            index += 1;
            match conversion {
                b'E' | b'O' => continue,
                b'_' | b'-' | b'0' | b'^' | b'#' => {
                    modifier = conversion;
                    continue;
                }
                _ => {}
            }
            conversion_text(conversion, tm, out, modifier);
            break;
        }
    }
}

fn conversion_text(conversion: u8, tm: &Tm, out: &mut Vec<u8>, modifier: u8) {
    let parts = &tm.parts;
    let hour12 = if parts.hour % 12 != 0 { parts.hour % 12 } else { 12 };
    let name = |table: &[&str], index: i32, short: bool| -> Vec<u8> {
        usize::try_from(index).ok().and_then(|position| table.get(position)).map_or(b"?".to_vec(), |text| if short { text.as_bytes()[..3].to_vec() } else { text.as_bytes().to_vec() })
    };
    match conversion {
        b'A' => add(out, &name(&WEEKDAYS, parts.weekday, false), modifier),
        b'a' => add(out, &name(&WEEKDAYS, parts.weekday, true), modifier),
        b'B' => add(out, &name(&MONTHS, parts.month, false), modifier),
        b'b' | b'h' => add(out, &name(&MONTHS, parts.month, true), modifier),
        b'C' => year_text(out, i64::from(parts.year), 1900, true, false, modifier),
        b'c' => emit(TIME_C, tm, out),
        b'D' => emit(b"%m/%d/%y", tm, out),
        b'd' => numeric(out, i64::from(parts.day), modifier, (b'0', 2)),
        b'e' => numeric(out, i64::from(parts.day), modifier, (b' ', 2)),
        b'F' => emit(b"%Y-%m-%d", tm, out),
        b'H' => numeric(out, i64::from(parts.hour), modifier, (b'0', 2)),
        b'I' => numeric(out, i64::from(hour12), modifier, (b'0', 2)),
        b'j' => numeric(out, i64::from(parts.yearday) + 1, modifier, (b'0', 3)),
        b'k' => numeric(out, i64::from(parts.hour), modifier, (b' ', 2)),
        b'l' => numeric(out, i64::from(hour12), modifier, (b' ', 2)),
        b'M' => numeric(out, i64::from(parts.minute), modifier, (b'0', 2)),
        b'm' => numeric(out, i64::from(parts.month) + 1, modifier, (b'0', 2)),
        b'n' => add(out, b"\n", modifier),
        b'P' => add(out, if parts.hour >= 12 { b"PM" } else { b"AM" }, FORCE_LOWER),
        b'p' => add(out, if parts.hour >= 12 { b"PM" } else { b"AM" }, modifier),
        b'R' => emit(b"%H:%M", tm, out),
        b'r' => emit(b"%I:%M:%S %p", tm, out),
        b'S' => numeric(out, i64::from(parts.second), modifier, (b'0', 2)),
        b's' => {
            let seconds = mktime_copy(tm.address).unwrap_or(-1);
            add(out, seconds.to_string().as_bytes(), modifier);
        }
        b'T' => emit(b"%H:%M:%S", tm, out),
        b't' => add(out, b"\t", modifier),
        b'U' => numeric(out, i64::from((parts.yearday + 7 - parts.weekday) / 7), modifier, (b'0', 2)),
        b'u' => convert(out, if parts.weekday == 0 { 7 } else { i64::from(parts.weekday) }, b' ', 0),
        b'V' | b'G' | b'g' => iso_week(tm, conversion, out, modifier),
        b'v' => emit(b"%e-%b-%Y", tm, out),
        b'W' => numeric(out, i64::from((parts.yearday + 7 - if parts.weekday != 0 { parts.weekday - 1 } else { 6 }) / 7), modifier, (b'0', 2)),
        b'w' => convert(out, i64::from(parts.weekday), b' ', 0),
        b'X' => emit(TIME_BIG_X, tm, out),
        b'x' => emit(TIME_X, tm, out),
        b'y' => year_text(out, i64::from(parts.year), 1900, false, true, modifier),
        b'Y' => year_text(out, i64::from(parts.year), 1900, true, true, modifier),
        b'Z' => add(out, &safe_zone(tm), modifier),
        b'z' => {
            let negative = tm.offset < 0 || (tm.offset == 0 && safe_zone(tm).first() == Some(&b'-'));
            add(out, if negative { b"-" } else { b"+" }, modifier);
            let minutes = tm.offset.abs() / 60;
            numeric(out, (minutes / 60) * 100 + minutes % 60, modifier, (b'0', 4));
        }
        b'+' => emit(TIME_DATE, tm, out),
        other => out.push(other),
    }
}

fn safe_zone(tm: &Tm) -> Vec<u8> {
    // cite: bionic strftime.c#_safe_tm_zone
    tm.zone.clone()
}

pub fn format(format: &[u8], tm: &Tm, capacity: usize) -> Result<Vec<u8>, i32> {
    // cite: bionic strftime.c#strftime
    let mut out = Vec::new();
    emit(format, tm, &mut out);
    if out.len() >= capacity {
        return Err(ERANGE);
    }
    Ok(out)
}

pub fn register() {
    for name in ["strftime", "strftime_l"] {
        in_libc(name, |context| {
            let (buffer, capacity) = (context.arg_ptr(0), context.arg_u64(1) as usize);
            let tm = read_full_tm(context.arg_ptr(3));
            match format(cstr_bytes(context.arg_ptr(2)), &tm, capacity) {
                Ok(text) => {
                    write_cstr_truncated(buffer, text.len() + 1, &text);
                    context.ret_u64(text.len() as u64);
                }
                Err(code) => {
                    context.set_errno(code);
                    context.ret_u64(0);
                }
            }
        });
    }
}
