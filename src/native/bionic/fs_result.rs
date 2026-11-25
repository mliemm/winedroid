use super::super::bridge::CallContext;
use super::super::host_log;
use super::errno_map::linux_from_darwin;

const MAX_ERRNO: i64 = 4095;
const EIO: i32 = 5;

pub fn host_errno() -> i32 {
    unsafe { *libc::__error() }
}

pub fn linux_errno_of(darwin: i32) -> i32 {
    match linux_from_darwin(darwin) {
        Some(linux) => linux,
        None => {
            host_log::line(&format!("FIXME: host errno {darwin} has no Linux errno; next missing primitive: errno {darwin}"));
            EIO
        }
    }
}

pub fn from_host() -> i64 {
    -i64::from(linux_errno_of(host_errno()))
}

pub fn check(result: i64) -> i64 {
    if result < 0 {
        from_host()
    } else {
        result
    }
}

pub fn missing(what: &str, reason: &str) {
    host_log::line(&format!("FIXME: missing-primitive {what}: {reason}; next missing primitive: {what}"));
}

pub fn is_error(result: i64) -> bool {
    (-MAX_ERRNO..0).contains(&result)
}

pub fn finish_int(context: &mut CallContext, result: i64) {
    if is_error(result) {
        context.set_errno((-result) as i32);
        context.ret_i32(-1);
    } else {
        context.ret_i32(result as i32);
    }
}

pub fn finish_long(context: &mut CallContext, result: i64) {
    if is_error(result) {
        context.set_errno((-result) as i32);
        context.ret_i64(-1);
    } else {
        context.ret_i64(result);
    }
}

pub fn finish_zero_or_errno(context: &mut CallContext, result: i64) {
    finish_int(context, result);
}

pub fn finish_code(context: &mut CallContext, result: i64) {
    if is_error(result) {
        context.ret_i32(-(result as i32));
    } else {
        context.ret_i32(0);
    }
}
