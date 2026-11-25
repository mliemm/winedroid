use std::arch::asm;

use super::export::{data_in_libm, in_libm};
use super::guest_mem::{read_value, write_value};

const ALL_EXCEPTIONS: u32 = 0x9f;
const ROUNDING_SHIFT: u32 = 22;
const ROUNDING_MASK: u32 = 3 << ROUNDING_SHIFT;

fn read_fpcr() -> u32 {
    let value: u64;
    unsafe { asm!("mrs {}, fpcr", out(reg) value, options(nomem, nostack, preserves_flags)) };
    value as u32
}

fn write_fpcr(value: u32) {
    unsafe { asm!("msr fpcr, {}", in(reg) u64::from(value), options(nomem, nostack)) };
}

fn read_fpsr() -> u32 {
    let value: u64;
    unsafe { asm!("mrs {}, fpsr", out(reg) value, options(nomem, nostack, preserves_flags)) };
    value as u32
}

fn write_fpsr(value: u32) {
    unsafe { asm!("msr fpsr, {}", in(reg) u64::from(value), options(nomem, nostack)) };
}

pub fn clear_exceptions(mask: u32) {
    // cite: bionic fenv_arm.h#FE_ALL_EXCEPT
    write_fpsr(read_fpsr() & !(mask & ALL_EXCEPTIONS));
}

pub fn raise_exceptions(mask: u32) {
    write_fpsr(read_fpsr() | (mask & ALL_EXCEPTIONS));
}

pub fn test_exceptions(mask: u32) -> u32 {
    read_fpsr() & mask & ALL_EXCEPTIONS
}

pub fn rounding_mode() -> u32 {
    // cite: bionic fenv.h#fegetround
    (read_fpcr() >> ROUNDING_SHIFT) & 3
}

pub fn set_rounding_mode(mode: u32) -> bool {
    if mode > 3 {
        return false;
    }
    write_fpcr((read_fpcr() & !ROUNDING_MASK) | (mode << ROUNDING_SHIFT));
    true
}

pub fn register() {
    data_in_libm("__fe_dfl_env", 8);
    in_libm("feclearexcept", |context| {
        clear_exceptions(context.arg_u32(0));
        context.ret_i32(0);
    });
    in_libm("fegetexceptflag", |context| {
        write_value::<u32>(context.arg_ptr(0), test_exceptions(context.arg_u32(1)));
        context.ret_i32(0);
    });
    in_libm("fesetexceptflag", |context| {
        let mask = context.arg_u32(1) & ALL_EXCEPTIONS;
        let flags = read_value::<u32>(context.arg_ptr(0)) & mask;
        write_fpsr((read_fpsr() & !mask) | flags);
        context.ret_i32(0);
    });
    in_libm("feraiseexcept", |context| {
        raise_exceptions(context.arg_u32(0));
        context.ret_i32(0);
    });
    in_libm("fetestexcept", |context| context.ret_i32(test_exceptions(context.arg_u32(0)) as i32));
    in_libm("fegetround", |context| context.ret_i32(rounding_mode() as i32));
    in_libm("fesetround", |context| {
        let accepted = set_rounding_mode(context.arg_u32(0));
        context.ret_i32(if accepted { 0 } else { -1 });
    });
    in_libm("fegetenv", |context| {
        write_value::<u32>(context.arg_ptr(0), read_fpcr());
        write_value::<u32>(context.arg_ptr(0) + 4, read_fpsr());
        context.ret_i32(0);
    });
    in_libm("fesetenv", |context| {
        let (control, status) = (read_value::<u32>(context.arg_ptr(0)), read_value::<u32>(context.arg_ptr(0) + 4));
        if control != read_fpcr() {
            write_fpcr(control);
        }
        write_fpsr(status);
        context.ret_i32(0);
    });
    in_libm("feholdexcept", |context| {
        write_value::<u32>(context.arg_ptr(0), read_fpcr());
        write_value::<u32>(context.arg_ptr(0) + 4, read_fpsr());
        write_fpsr(read_fpsr() & 0xFFFF_FFE0 & !0x80);
        context.ret_i32(0);
    });
    in_libm("feupdateenv", |context| {
        let pending = read_fpsr() & ALL_EXCEPTIONS;
        let (control, status) = (read_value::<u32>(context.arg_ptr(0)), read_value::<u32>(context.arg_ptr(0) + 4));
        if control != read_fpcr() {
            write_fpcr(control);
        }
        write_fpsr(status | pending);
        context.ret_i32(0);
    });
    in_libm("feenableexcept", |context| context.ret_i32(-1));
    in_libm("fedisableexcept", |context| context.ret_i32(0));
    in_libm("fegetexcept", |context| context.ret_i32(0));
}
