use std::ffi::CString;
use std::sync::OnceLock;

use super::super::bridge::CallContext;
use super::abi_types::{EFAULT, EINVAL, EOVERFLOW, TIMESPEC_SIZE, TM_SIZE};
use super::civil::{from_seconds, read_tm, to_seconds, write_tm, BrokenDown};
use super::clocks::{now, resolution};
use super::export::in_libc;
use super::guest_mem::{read_value, write_bytes, write_cstr_truncated, write_value};
use super::sleep::{clock_sleep, nanosleep, sleep_micros, sleep_seconds};

const ASCTIME_BUFFER: usize = 26;
const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

static GMT_NAME: OnceLock<usize> = OnceLock::new();
static GMTIME_BUFFER: OnceLock<usize> = OnceLock::new();
static ASCTIME_STATIC: OnceLock<usize> = OnceLock::new();

fn gmt_name() -> usize {
    *GMT_NAME.get_or_init(|| Box::leak(Box::new(CString::new("GMT").expect("no nul"))).as_ptr() as usize)
}

pub fn leaked_buffer(cell: &'static OnceLock<usize>, size: usize) -> usize {
    *cell.get_or_init(|| Box::leak(vec![0u8; size].into_boxed_slice()).as_ptr() as usize)
}

fn write_timespec(address: usize, value: libc::timespec) {
    let mut block = [0u8; TIMESPEC_SIZE];
    block[..8].copy_from_slice(&value.tv_sec.to_le_bytes());
    block[8..].copy_from_slice(&value.tv_nsec.to_le_bytes());
    write_bytes(address, &block);
}

fn read_timespec(address: usize) -> libc::timespec {
    libc::timespec { tv_sec: read_value::<i64>(address), tv_nsec: read_value::<i64>(address + 8) }
}

fn clock_gettime(context: &mut CallContext) {
    // cite: bionic vdso.cpp#clock_gettime
    let (clock, destination) = (context.arg_i32(0), context.arg_ptr(1));
    if destination == 0 {
        context.set_errno(EFAULT);
        context.ret_i32(-1);
        return;
    }
    match now(clock) {
        Ok(value) => {
            write_timespec(destination, value);
            context.ret_i32(0);
        }
        Err(code) => {
            context.set_errno(code);
            context.ret_i32(-1);
        }
    }
}

fn clock_getres(context: &mut CallContext) {
    let (clock, destination) = (context.arg_i32(0), context.arg_ptr(1));
    match resolution(clock) {
        Ok(value) => {
            if destination != 0 {
                write_timespec(destination, value);
            }
            context.ret_i32(0);
        }
        Err(code) => {
            context.set_errno(code);
            context.ret_i32(-1);
        }
    }
}

fn gettimeofday(context: &mut CallContext) {
    // cite: bionic vdso.cpp#gettimeofday
    let (value, zone) = (context.arg_ptr(0), context.arg_ptr(1));
    if value != 0 {
        let current = now(0).unwrap_or(libc::timespec { tv_sec: 0, tv_nsec: 0 });
        write_value::<i64>(value, current.tv_sec);
        write_value::<i64>(value + 8, current.tv_nsec / 1000);
    }
    if zone != 0 {
        write_value::<i64>(zone, 0);
    }
    context.ret_i32(0);
}

fn sleeping(context: &mut CallContext) {
    let (request, remaining) = (context.arg_ptr(0), context.arg_ptr(1));
    let wanted = read_timespec(request);
    let mut left = wanted;
    let code = nanosleep(&wanted, &mut left);
    if remaining != 0 && code != 0 {
        write_timespec(remaining, left);
    }
    if code != 0 {
        context.set_errno_from_host(code);
        context.ret_i32(-1);
    } else {
        context.ret_i32(0);
    }
}

fn clock_nanosleep(context: &mut CallContext) {
    let (clock, flags, request, remaining) = (context.arg_i32(0), context.arg_i32(1), context.arg_ptr(2), context.arg_ptr(3));
    let mut left = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    let code = clock_sleep(clock, flags, read_timespec(request), &mut left);
    if remaining != 0 && code != 0 && flags & 1 == 0 {
        write_timespec(remaining, left);
    }
    context.ret_i32(code);
}

fn gmtime_into(context: &mut CallContext, seconds_pointer: usize, destination: usize) {
    // cite: bionic localtime.c#gmtsub
    match from_seconds(read_value::<i64>(seconds_pointer)) {
        Ok(parts) => {
            write_tm(destination, &parts, 0, 0, gmt_name());
            context.ret_ptr(destination);
        }
        Err(code) => {
            context.set_errno(code);
            context.ret_ptr(0);
        }
    }
}

fn timegm(context: &mut CallContext) {
    // cite: bionic localtime.c#timegm
    let address = context.arg_ptr(0);
    let outcome = to_seconds(&read_tm(address)).and_then(|seconds| from_seconds(seconds).map(|parts| (seconds, parts)));
    match outcome {
        Ok((seconds, parts)) => {
            write_tm(address, &parts, 0, 0, gmt_name());
            context.ret_i64(seconds);
        }
        Err(code) => {
            context.set_errno(code);
            context.ret_i64(-1);
        }
    }
}

fn year_text(parts: &BrokenDown) -> String {
    (i64::from(parts.year) + 1900).to_string()
}

fn asctime_text(parts: &BrokenDown) -> String {
    // cite: bionic asctime.c#asctime_r
    let weekday = usize::try_from(parts.weekday).ok().and_then(|index| WEEKDAYS.get(index)).copied().unwrap_or("???");
    let month = usize::try_from(parts.month).ok().and_then(|index| MONTHS.get(index)).copied().unwrap_or("???");
    let year = year_text(parts);
    let year_field = if year.len() <= 4 { format!("{year:<4}") } else { format!("    {year}") };
    format!("{weekday} {month}{:3} {:02}:{:02}:{:02} {year_field}\n", parts.day, parts.hour, parts.minute, parts.second)
}

pub fn asctime_into(buffer: usize, parts: &BrokenDown, is_static: bool) -> bool {
    let text = asctime_text(parts);
    if text.len() >= ASCTIME_BUFFER && !is_static {
        return false;
    }
    write_cstr_truncated(buffer, text.len() + 1, text.as_bytes());
    true
}

fn asctime(context: &mut CallContext, buffer: usize, is_static: bool) {
    let source = context.arg_ptr(0);
    if source == 0 {
        context.set_errno(EINVAL);
        write_cstr_truncated(buffer, ASCTIME_BUFFER, b"??? ??? ?? ??:??:?? ????\n");
        context.ret_ptr(buffer);
        return;
    }
    if asctime_into(buffer, &read_tm(source), is_static) {
        context.ret_ptr(buffer);
    } else {
        context.set_errno(EOVERFLOW);
        context.ret_ptr(0);
    }
}

pub fn register() {
    in_libc("clock_gettime", clock_gettime);
    in_libc("clock_getres", clock_getres);
    in_libc("gettimeofday", gettimeofday);
    in_libc("time", |context| {
        let current = now(0).map_or(0, |value| value.tv_sec);
        if context.arg_ptr(0) != 0 {
            write_value::<i64>(context.arg_ptr(0), current);
        }
        context.ret_i64(current);
    });
    in_libc("clock", |context| {
        // cite: bionic clock.cpp#clock
        match now(2) {
            Ok(value) => context.ret_i64(value.tv_sec * 1_000_000 + value.tv_nsec / 1000),
            Err(_) => context.ret_i64(-1),
        }
    });
    in_libc("nanosleep", sleeping);
    in_libc("clock_nanosleep", clock_nanosleep);
    in_libc("usleep", |context| context.ret_i32(sleep_micros(context.arg_u32(0))));
    in_libc("sleep", |context| context.ret_u32(sleep_seconds(context.arg_u32(0))));
    in_libc("gmtime_r", |context| gmtime_into(context, context.arg_ptr(0), context.arg_ptr(1)));
    in_libc("gmtime", |context| gmtime_into(context, context.arg_ptr(0), leaked_buffer(&GMTIME_BUFFER, TM_SIZE)));
    in_libc("timegm", timegm);
    in_libc("difftime", |context| context.ret_f64((context.arg_u64(0) as i64 as f64) - (context.arg_u64(1) as i64 as f64)));
    in_libc("asctime_r", |context| asctime(context, context.arg_ptr(1), false));
    in_libc("asctime", |context| asctime(context, leaked_buffer(&ASCTIME_STATIC, ASCTIME_BUFFER.max(80)), true));
}
