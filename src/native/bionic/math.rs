use super::export::{data_in_libm, in_libc, in_libm};
use super::guest_mem::write_value;

const FP_INFINITE: i32 = 1;
const FP_NAN: i32 = 2;
const FP_NORMAL: i32 = 4;
const FP_SUBNORMAL: i32 = 8;
const FP_ZERO: i32 = 0x10;
const ILOGB_ZERO: i32 = -0x7FFF_FFFF;
const ILOGB_NAN: i32 = 0x7FFF_FFFF;

static SIGNGAM: std::sync::OnceLock<usize> = std::sync::OnceLock::new();

macro_rules! host_functions {
    ($($name:ident($($argument:ident: $kind:ty),*) -> $result:ty;)*) => {
        mod host {
            extern "C" {
                $(pub fn $name($($argument: $kind),*) -> $result;)*
            }
        }
    };
}

host_functions! {
    acos(x: f64) -> f64; asin(x: f64) -> f64; atan(x: f64) -> f64; cos(x: f64) -> f64; sin(x: f64) -> f64; tan(x: f64) -> f64;
    acosh(x: f64) -> f64; asinh(x: f64) -> f64; atanh(x: f64) -> f64; cosh(x: f64) -> f64; sinh(x: f64) -> f64; tanh(x: f64) -> f64;
    exp(x: f64) -> f64; exp2(x: f64) -> f64; expm1(x: f64) -> f64; log(x: f64) -> f64; log10(x: f64) -> f64; log1p(x: f64) -> f64;
    log2(x: f64) -> f64; logb(x: f64) -> f64; cbrt(x: f64) -> f64; fabs(x: f64) -> f64; sqrt(x: f64) -> f64; erf(x: f64) -> f64;
    erfc(x: f64) -> f64; tgamma(x: f64) -> f64; ceil(x: f64) -> f64; floor(x: f64) -> f64; nearbyint(x: f64) -> f64; rint(x: f64) -> f64;
    round(x: f64) -> f64; trunc(x: f64) -> f64; j0(x: f64) -> f64; j1(x: f64) -> f64; y0(x: f64) -> f64; y1(x: f64) -> f64;
    acosf(x: f32) -> f32; asinf(x: f32) -> f32; atanf(x: f32) -> f32; cosf(x: f32) -> f32; sinf(x: f32) -> f32; tanf(x: f32) -> f32;
    acoshf(x: f32) -> f32; asinhf(x: f32) -> f32; atanhf(x: f32) -> f32; coshf(x: f32) -> f32; sinhf(x: f32) -> f32; tanhf(x: f32) -> f32;
    expf(x: f32) -> f32; exp2f(x: f32) -> f32; expm1f(x: f32) -> f32; logf(x: f32) -> f32; log10f(x: f32) -> f32; log1pf(x: f32) -> f32;
    log2f(x: f32) -> f32; logbf(x: f32) -> f32; cbrtf(x: f32) -> f32; fabsf(x: f32) -> f32; sqrtf(x: f32) -> f32; erff(x: f32) -> f32;
    erfcf(x: f32) -> f32; tgammaf(x: f32) -> f32; ceilf(x: f32) -> f32; floorf(x: f32) -> f32; nearbyintf(x: f32) -> f32; rintf(x: f32) -> f32;
    roundf(x: f32) -> f32; truncf(x: f32) -> f32;
    atan2(x: f64, y: f64) -> f64; hypot(x: f64, y: f64) -> f64; pow(x: f64, y: f64) -> f64; fmod(x: f64, y: f64) -> f64;
    remainder(x: f64, y: f64) -> f64; copysign(x: f64, y: f64) -> f64; nextafter(x: f64, y: f64) -> f64; fdim(x: f64, y: f64) -> f64;
    fmax(x: f64, y: f64) -> f64; fmin(x: f64, y: f64) -> f64;
    atan2f(x: f32, y: f32) -> f32; hypotf(x: f32, y: f32) -> f32; powf(x: f32, y: f32) -> f32; fmodf(x: f32, y: f32) -> f32;
    remainderf(x: f32, y: f32) -> f32; copysignf(x: f32, y: f32) -> f32; nextafterf(x: f32, y: f32) -> f32; fdimf(x: f32, y: f32) -> f32;
    fmaxf(x: f32, y: f32) -> f32; fminf(x: f32, y: f32) -> f32;
    fma(x: f64, y: f64, z: f64) -> f64; fmaf(x: f32, y: f32, z: f32) -> f32;
    lrint(x: f64) -> i64; llrint(x: f64) -> i64; lround(x: f64) -> i64; llround(x: f64) -> i64;
    lrintf(x: f32) -> i64; llrintf(x: f32) -> i64; lroundf(x: f32) -> i64; llroundf(x: f32) -> i64;
    ldexp(x: f64, e: i32) -> f64; scalbn(x: f64, e: i32) -> f64; scalbln(x: f64, e: i64) -> f64;
    ldexpf(x: f32, e: i32) -> f32; scalbnf(x: f32, e: i32) -> f32; scalblnf(x: f32, e: i64) -> f32;
    frexp(x: f64, e: *mut i32) -> f64; frexpf(x: f32, e: *mut i32) -> f32;
    modf(x: f64, i: *mut f64) -> f64; modff(x: f32, i: *mut f32) -> f32;
    ilogb(x: f64) -> i32;
    remquo(x: f64, y: f64, q: *mut i32) -> f64; remquof(x: f32, y: f32, q: *mut i32) -> f32;
    lgamma_r(x: f64, s: *mut i32) -> f64; lgammaf_r(x: f32, s: *mut i32) -> f32;
    jn(n: i32, x: f64) -> f64; yn(n: i32, x: f64) -> f64;
}

macro_rules! unary_f64 {
    ($($name:ident),*) => { $( in_libm(stringify!($name), |context| context.ret_f64(unsafe { host::$name(context.arg_f64(0)) })); )* };
}

macro_rules! unary_f32 {
    ($($name:ident),*) => { $( in_libm(stringify!($name), |context| context.ret_f32(unsafe { host::$name(context.arg_f32(0)) })); )* };
}

macro_rules! binary_f64 {
    ($($name:ident),*) => { $( in_libm(stringify!($name), |context| context.ret_f64(unsafe { host::$name(context.arg_f64(0), context.arg_f64(1)) })); )* };
}

macro_rules! binary_f32 {
    ($($name:ident),*) => { $( in_libm(stringify!($name), |context| context.ret_f32(unsafe { host::$name(context.arg_f32(0), context.arg_f32(1)) })); )* };
}

macro_rules! to_integer {
    ($($name:ident: $argument:ident),*) => { $( in_libm(stringify!($name), |context| context.ret_i64(unsafe { host::$name(context.$argument(0)) })); )* };
}

fn ilogb_bionic(value: f64) -> i32 {
    // cite: bionic math.h#FP_ILOGB0, FP_ILOGBNAN
    if value == 0.0 {
        ILOGB_ZERO
    } else if value.is_nan() || value.is_infinite() {
        ILOGB_NAN
    } else {
        unsafe { host::ilogb(value) }
    }
}

fn nan_words(text: &[u8], words: usize) -> Vec<u32> {
    let body = text
        .strip_prefix(b"0x")
        .or_else(|| text.strip_prefix(b"0X"))
        .unwrap_or(text);
    let digits: Vec<u32> = body
        .iter()
        .map_while(|byte| (*byte as char).to_digit(16))
        .collect();
    let mut array = vec![0u32; words];
    for (position, digit) in digits.iter().rev().enumerate() {
        let (word, shift) = (position / 8, (position % 8) * 4);
        if word >= words {
            break;
        }
        array[word] |= digit << shift;
    }
    array
}

fn classify(value: f64) -> i32 {
    // cite: bionic libc.map.txt#__fpclassify
    if value.is_nan() {
        FP_NAN
    } else if value.is_infinite() {
        FP_INFINITE
    } else if value == 0.0 {
        FP_ZERO
    } else if value.is_subnormal() {
        FP_SUBNORMAL
    } else {
        FP_NORMAL
    }
}

fn register_classification() {
    in_libc("__fpclassify", |context| {
        context.ret_i32(classify(context.arg_f64(0)))
    });
    in_libc("__fpclassifyd", |context| {
        context.ret_i32(classify(context.arg_f64(0)))
    });
    in_libc("__fpclassifyf", |context| {
        context.ret_i32(classify(f64::from(context.arg_f32(0))))
    });
    for name in ["isnan", "__isnan"] {
        in_libc(name, |context| {
            context.ret_i32(i32::from(context.arg_f64(0).is_nan()))
        });
    }
    for name in ["isnanf", "__isnanf"] {
        in_libc(name, |context| {
            context.ret_i32(i32::from(context.arg_f32(0).is_nan()))
        });
    }
    for name in ["isinf", "__isinf"] {
        in_libc(name, |context| {
            context.ret_i32(i32::from(context.arg_f64(0).is_infinite()))
        });
    }
    for name in ["isinff", "__isinff"] {
        in_libc(name, |context| {
            context.ret_i32(i32::from(context.arg_f32(0).is_infinite()))
        });
    }
    for name in ["isfinite", "__isfinite"] {
        in_libc(name, |context| {
            context.ret_i32(i32::from(context.arg_f64(0).is_finite()))
        });
    }
    for name in ["isfinitef", "__isfinitef"] {
        in_libc(name, |context| {
            context.ret_i32(i32::from(context.arg_f32(0).is_finite()))
        });
    }
    in_libc("ldexp", |context| {
        context.ret_f64(unsafe { host::ldexp(context.arg_f64(0), context.arg_i32(0)) })
    });
}

fn register_adapted() {
    let _ = SIGNGAM.set(data_in_libm("signgam", 4));
    in_libm("ilogb", |context| {
        context.ret_i32(ilogb_bionic(context.arg_f64(0)))
    });
    in_libm("ilogbf", |context| {
        context.ret_i32(ilogb_bionic(f64::from(context.arg_f32(0))))
    });
    in_libm("nan", |context| {
        // cite: bionic math.h#nan
        let words = nan_words(
            super::guest_mem::optional_cstr(context.arg_ptr(0)).unwrap_or(b""),
            2,
        );
        let bits = (u64::from(words[1] | 0x7FF8_0000) << 32) | u64::from(words[0]);
        context.ret_f64(f64::from_bits(bits));
    });
    in_libm("nanf", |context| {
        let words = nan_words(
            super::guest_mem::optional_cstr(context.arg_ptr(0)).unwrap_or(b""),
            1,
        );
        context.ret_f32(f32::from_bits(words[0] | 0x7FC0_0000));
    });
    for name in ["lgamma", "gamma"] {
        in_libm(name, |context| {
            let mut sign = 0i32;
            let value = unsafe { host::lgamma_r(context.arg_f64(0), &mut sign) };
            write_value::<i32>(signgam_address(), sign);
            context.ret_f64(value);
        });
    }
    for name in ["lgammaf", "gammaf"] {
        in_libm(name, |context| {
            let mut sign = 0i32;
            let value = unsafe { host::lgammaf_r(context.arg_f32(0), &mut sign) };
            write_value::<i32>(signgam_address(), sign);
            context.ret_f32(value);
        });
    }
    for name in ["lgamma_r", "gamma_r"] {
        in_libm(name, |context| {
            let mut sign = 0i32;
            let value = unsafe { host::lgamma_r(context.arg_f64(0), &mut sign) };
            write_value::<i32>(context.arg_ptr(0), sign);
            context.ret_f64(value);
        });
    }
    for name in ["lgammaf_r", "gammaf_r"] {
        in_libm(name, |context| {
            let mut sign = 0i32;
            let value = unsafe { host::lgammaf_r(context.arg_f32(0), &mut sign) };
            write_value::<i32>(context.arg_ptr(0), sign);
            context.ret_f32(value);
        });
    }
    in_libm("sincos", |context| {
        write_value::<f64>(context.arg_ptr(0), unsafe { host::sin(context.arg_f64(0)) });
        write_value::<f64>(context.arg_ptr(1), unsafe { host::cos(context.arg_f64(0)) });
    });
    in_libm("sincosf", |context| {
        write_value::<f32>(context.arg_ptr(0), unsafe {
            host::sinf(context.arg_f32(0))
        });
        write_value::<f32>(context.arg_ptr(1), unsafe {
            host::cosf(context.arg_f32(0))
        });
    });
    in_libm("drem", |context| {
        context.ret_f64(unsafe { host::remainder(context.arg_f64(0), context.arg_f64(1)) })
    });
    in_libm("dremf", |context| {
        context.ret_f32(unsafe { host::remainderf(context.arg_f32(0), context.arg_f32(1)) })
    });
    in_libm("finite", |context| {
        context.ret_i32(i32::from(context.arg_f64(0).is_finite()))
    });
    in_libm("finitef", |context| {
        context.ret_i32(i32::from(context.arg_f32(0).is_finite()))
    });
    in_libm("__signbit", |context| {
        context.ret_i32(i32::from(context.arg_f64(0).is_sign_negative()))
    });
    in_libm("__signbitf", |context| {
        context.ret_i32(i32::from(context.arg_f32(0).is_sign_negative()))
    });
    in_libm("fabsl", |context| {
        context.ret_quad(context.arg_quad(0) & !(1u128 << 127))
    });
    in_libm("copysignl", |context| {
        context.ret_quad(
            (context.arg_quad(0) & !(1u128 << 127)) | (context.arg_quad(1) & (1u128 << 127)),
        )
    });
    for (name, host_function) in [
        ("j0f", host::j0 as unsafe extern "C" fn(f64) -> f64),
        ("j1f", host::j1),
        ("y0f", host::y0),
        ("y1f", host::y1),
    ] {
        in_libm(name, move |context| {
            context.ret_f32(unsafe { host_function(f64::from(context.arg_f32(0))) } as f32)
        });
    }
    in_libm("jnf", |context| {
        context
            .ret_f32(unsafe { host::jn(context.arg_i32(0), f64::from(context.arg_f32(0))) } as f32)
    });
    in_libm("ynf", |context| {
        context
            .ret_f32(unsafe { host::yn(context.arg_i32(0), f64::from(context.arg_f32(0))) } as f32)
    });
}

fn signgam_address() -> usize {
    *SIGNGAM
        .get()
        .expect("signgam is registered before first use")
}

fn register_pointer_results() {
    in_libm("frexp", |context| {
        let mut exponent = 0i32;
        let fraction = unsafe { host::frexp(context.arg_f64(0), &mut exponent) };
        write_value::<i32>(context.arg_ptr(0), exponent);
        context.ret_f64(fraction);
    });
    in_libm("frexpf", |context| {
        let mut exponent = 0i32;
        let fraction = unsafe { host::frexpf(context.arg_f32(0), &mut exponent) };
        write_value::<i32>(context.arg_ptr(0), exponent);
        context.ret_f32(fraction);
    });
    in_libm("modf", |context| {
        let mut whole = 0f64;
        let fraction = unsafe { host::modf(context.arg_f64(0), &mut whole) };
        write_value::<f64>(context.arg_ptr(0), whole);
        context.ret_f64(fraction);
    });
    in_libm("modff", |context| {
        let mut whole = 0f32;
        let fraction = unsafe { host::modff(context.arg_f32(0), &mut whole) };
        write_value::<f32>(context.arg_ptr(0), whole);
        context.ret_f32(fraction);
    });
    in_libm("remquo", |context| {
        let mut quotient = 0i32;
        let remainder =
            unsafe { host::remquo(context.arg_f64(0), context.arg_f64(1), &mut quotient) };
        write_value::<i32>(context.arg_ptr(0), quotient);
        context.ret_f64(remainder);
    });
    in_libm("remquof", |context| {
        let mut quotient = 0i32;
        let remainder =
            unsafe { host::remquof(context.arg_f32(0), context.arg_f32(1), &mut quotient) };
        write_value::<i32>(context.arg_ptr(0), quotient);
        context.ret_f32(remainder);
    });
    in_libm("ldexpf", |context| {
        context.ret_f32(unsafe { host::ldexpf(context.arg_f32(0), context.arg_i32(0)) })
    });
    in_libm("scalbn", |context| {
        context.ret_f64(unsafe { host::scalbn(context.arg_f64(0), context.arg_i32(0)) })
    });
    in_libm("scalbnf", |context| {
        context.ret_f32(unsafe { host::scalbnf(context.arg_f32(0), context.arg_i32(0)) })
    });
    in_libm("scalbln", |context| {
        context.ret_f64(unsafe { host::scalbln(context.arg_f64(0), context.arg_u64(0) as i64) })
    });
    in_libm("scalblnf", |context| {
        context.ret_f32(unsafe { host::scalblnf(context.arg_f32(0), context.arg_u64(0) as i64) })
    });
    in_libm("jn", |context| {
        context.ret_f64(unsafe { host::jn(context.arg_i32(0), context.arg_f64(0)) })
    });
    in_libm("yn", |context| {
        context.ret_f64(unsafe { host::yn(context.arg_i32(0), context.arg_f64(0)) })
    });
}

pub fn register() {
    unary_f64!(
        acos, asin, atan, cos, sin, tan, acosh, asinh, atanh, cosh, sinh, tanh, exp, exp2, expm1,
        log, log10, log1p, log2, logb, cbrt, fabs, sqrt, erf, erfc, tgamma, ceil, floor, nearbyint,
        rint, round, trunc, j0, j1, y0, y1
    );
    unary_f32!(
        acosf, asinf, atanf, cosf, sinf, tanf, acoshf, asinhf, atanhf, coshf, sinhf, tanhf, expf,
        exp2f, expm1f, logf, log10f, log1pf, log2f, logbf, cbrtf, fabsf, sqrtf, erff, erfcf,
        tgammaf, ceilf, floorf, nearbyintf, rintf, roundf, truncf
    );
    binary_f64!(atan2, hypot, pow, fmod, remainder, copysign, nextafter, fdim, fmax, fmin);
    binary_f32!(
        atan2f, hypotf, powf, fmodf, remainderf, copysignf, nextafterf, fdimf, fmaxf, fminf
    );
    in_libm("fma", |context| {
        context.ret_f64(unsafe {
            host::fma(context.arg_f64(0), context.arg_f64(1), context.arg_f64(2))
        })
    });
    in_libm("fmaf", |context| {
        context.ret_f32(unsafe {
            host::fmaf(context.arg_f32(0), context.arg_f32(1), context.arg_f32(2))
        })
    });
    to_integer!(lrint: arg_f64, llrint: arg_f64, lround: arg_f64, llround: arg_f64, lrintf: arg_f32, llrintf: arg_f32, lroundf: arg_f32, llroundf: arg_f32);
    register_adapted();
    register_pointer_results();
    register_classification();
}
