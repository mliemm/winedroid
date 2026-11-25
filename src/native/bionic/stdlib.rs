use super::abi_types::EINVAL;
use super::export::in_libc;

fn parsed_float(context: &mut super::super::bridge::CallContext, convert: impl FnOnce(&mut super::super::bridge::CallContext)) {
    unsafe { *libc::__error() = 0 };
    convert(context);
    let host_errno = unsafe { *libc::__error() };
    if host_errno != 0 {
        context.set_errno_from_host(host_errno);
    }
}

fn register_integers() {
    in_libc("abs", |context| context.ret_i32(context.arg_i32(0).wrapping_abs()));
    in_libc("labs", |context| context.ret_i64((context.arg_u64(0) as i64).wrapping_abs()));
    in_libc("llabs", |context| context.ret_i64((context.arg_u64(0) as i64).wrapping_abs()));
    in_libc("imaxabs", |context| context.ret_i64((context.arg_u64(0) as i64).wrapping_abs()));
    in_libc("div", |context| {
        // cite: bionic div.c#div
        let (numerator, denominator) = (context.arg_i32(0), context.arg_i32(1));
        let quotient = if denominator == 0 { 0 } else { numerator.wrapping_div(denominator) };
        let remainder = numerator.wrapping_sub(quotient.wrapping_mul(denominator));
        context.ret_u64(u64::from(quotient as u32) | (u64::from(remainder as u32) << 32));
    });
    for name in ["ldiv", "lldiv", "imaxdiv"] {
        in_libc(name, |context| {
            // cite: bionic ldiv.c#ldiv
            let (numerator, denominator) = (context.arg_u64(0) as i64, context.arg_u64(1) as i64);
            let quotient = if denominator == 0 { 0 } else { numerator.wrapping_div(denominator) };
            let remainder = numerator.wrapping_sub(quotient.wrapping_mul(denominator));
            context.ret_pair(quotient as u64, remainder as u64);
        });
    }
}

fn register_floats() {
    in_libc("strtod", |context| {
        parsed_float(context, |context| {
            let value = unsafe { libc::strtod(context.arg_ptr(0) as *const _, context.arg_ptr(1) as *mut *mut libc::c_char) };
            context.ret_f64(value);
        })
    });
    in_libc("strtod_l", |context| {
        // cite: bionic stdlib_l.cpp#strtod_l
        parsed_float(context, |context| {
            let value = unsafe { libc::strtod(context.arg_ptr(0) as *const _, context.arg_ptr(1) as *mut *mut libc::c_char) };
            context.ret_f64(value);
        })
    });
    in_libc("strtof", |context| {
        parsed_float(context, |context| {
            let value = unsafe { libc::strtof(context.arg_ptr(0) as *const _, context.arg_ptr(1) as *mut *mut libc::c_char) };
            context.ret_f32(value);
        })
    });
    in_libc("atof", |context| {
        parsed_float(context, |context| {
            let value = unsafe { libc::strtod(context.arg_ptr(0) as *const _, std::ptr::null_mut()) };
            context.ret_f64(value);
        })
    });
}

fn register_random() {
    in_libc("arc4random", |context| context.ret_u32(random_word()));
    in_libc("arc4random_buf", |context| fill_random(context.arg_ptr(0), context.arg_u64(1) as usize));
    in_libc("arc4random_uniform", |context| {
        let bound = context.arg_u32(0);
        let value = if bound < 2 { 0 } else { uniform(bound) };
        context.ret_u32(value);
    });
    in_libc("getentropy", |context| {
        let (buffer, length) = (context.arg_ptr(0), context.arg_u64(1) as usize);
        if length > 256 {
            context.set_errno(EINVAL);
            context.ret_i32(-1);
            return;
        }
        fill_random(buffer, length);
        context.ret_i32(0);
    });
    in_libc("getrandom", |context| {
        const GRND_NONBLOCK: u32 = 1;
        const GRND_RANDOM: u32 = 2;
        const LIMIT: usize = 33_554_431;
        if context.arg_u32(2) & !(GRND_NONBLOCK | GRND_RANDOM) != 0 {
            context.set_errno(EINVAL);
            context.ret_i64(-1);
            return;
        }
        let length = (context.arg_u64(1) as usize).min(LIMIT);
        fill_random(context.arg_ptr(0), length);
        context.ret_i64(length as i64);
    });
}

fn fill_random(buffer: usize, length: usize) {
    let mut written = 0;
    while written < length {
        let chunk = (length - written).min(256);
        unsafe { libc::getentropy((buffer + written) as *mut libc::c_void, chunk) };
        written += chunk;
    }
}

fn random_word() -> u32 {
    let mut word = 0u32;
    fill_random(&mut word as *mut u32 as usize, 4);
    word
}

fn uniform(bound: u32) -> u32 {
    // cite: bionic arc4random_uniform.c#arc4random_uniform
    let minimum = bound.wrapping_neg() % bound;
    loop {
        let candidate = random_word();
        if candidate >= minimum {
            return candidate % bound;
        }
    }
}

pub fn register() {
    register_integers();
    register_floats();
    register_random();
}
