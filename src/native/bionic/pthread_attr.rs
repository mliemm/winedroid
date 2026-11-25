use super::abi_types::EINVAL;
use super::export::in_libc;
use super::guest_mem::{read_value, write_value};

pub const ATTR_SIZE: usize = 56;
pub const FLAG_DETACHED: u32 = 1;
pub const FLAG_INHERIT: u32 = 4;
pub const FLAG_EXPLICIT: u32 = 8;
pub const STACK_SIZE_DEFAULT: u64 = 1024 * 1024 - 32 * 1024;
pub const GUARD_SIZE_DEFAULT: u64 = 4096;
pub const STACK_MIN: u64 = 16384;
const GUEST_PAGE: u64 = 4096;
const ENOTSUP: i32 = 95;

const FLAGS: usize = 0;
const STACK_BASE: usize = 8;
const STACK_SIZE: usize = 16;
const GUARD_SIZE: usize = 24;
const POLICY: usize = 32;
const PRIORITY: usize = 36;

pub fn init(attr: usize) {
    // cite: bionic pthread_attr.cpp#pthread_attr_init
    write_value::<u32>(attr + FLAGS, 0);
    write_value::<u64>(attr + STACK_BASE, 0);
    write_value::<u64>(attr + STACK_SIZE, STACK_SIZE_DEFAULT);
    write_value::<u64>(attr + GUARD_SIZE, GUARD_SIZE_DEFAULT);
    write_value::<i32>(attr + POLICY, 0);
    write_value::<i32>(attr + PRIORITY, 0);
}

fn flags(attr: usize) -> u32 {
    read_value(attr + FLAGS)
}

fn set_flags(attr: usize, value: u32) {
    write_value(attr + FLAGS, value);
}

pub fn register() {
    in_libc("pthread_attr_init", |context| {
        init(context.arg_ptr(0));
        context.ret_i32(0);
    });
    in_libc("pthread_attr_destroy", |context| {
        // cite: bionic pthread_attr.cpp#pthread_attr_destroy
        unsafe { std::ptr::write_bytes(context.arg_ptr(0) as *mut u8, 0x42, ATTR_SIZE) };
        context.ret_i32(0);
    });
    in_libc("pthread_attr_setinheritsched", |context| {
        // cite: bionic pthread_attr.cpp#pthread_attr_setinheritsched
        let attr = context.arg_ptr(0);
        match context.arg_i32(1) {
            0 => set_flags(attr, (flags(attr) & !FLAG_INHERIT) | FLAG_EXPLICIT),
            1 => set_flags(attr, (flags(attr) | FLAG_INHERIT) & !FLAG_EXPLICIT),
            _ => return context.ret_i32(EINVAL),
        }
        context.ret_i32(0);
    });
    in_libc("pthread_attr_getinheritsched", |context| {
        // cite: bionic pthread_attr.cpp#pthread_attr_getinheritsched
        let attr = context.arg_ptr(0);
        let value = if flags(attr) & FLAG_INHERIT != 0 {
            1
        } else if flags(attr) & FLAG_EXPLICIT != 0 {
            0
        } else {
            i32::from(read_value::<i32>(attr + POLICY) == 0)
        };
        write_value::<i32>(context.arg_ptr(1), value);
        context.ret_i32(0);
    });
    in_libc("pthread_attr_setdetachstate", |context| {
        // cite: bionic pthread_attr.cpp#pthread_attr_setdetachstate
        let attr = context.arg_ptr(0);
        match context.arg_i32(1) {
            1 => set_flags(attr, flags(attr) | FLAG_DETACHED),
            0 => set_flags(attr, flags(attr) & !FLAG_DETACHED),
            _ => return context.ret_i32(EINVAL),
        }
        context.ret_i32(0);
    });
    in_libc("pthread_attr_getdetachstate", |context| {
        write_value::<i32>(context.arg_ptr(1), i32::from(flags(context.arg_ptr(0)) & FLAG_DETACHED != 0));
        context.ret_i32(0);
    });
    in_libc("pthread_attr_setschedpolicy", |context| {
        write_value::<i32>(context.arg_ptr(0) + POLICY, context.arg_i32(1));
        context.ret_i32(0);
    });
    in_libc("pthread_attr_getschedpolicy", |context| {
        write_value::<i32>(context.arg_ptr(1), read_value::<i32>(context.arg_ptr(0) + POLICY));
        context.ret_i32(0);
    });
    in_libc("pthread_attr_setschedparam", |context| {
        write_value::<i32>(context.arg_ptr(0) + PRIORITY, read_value::<i32>(context.arg_ptr(1)));
        context.ret_i32(0);
    });
    in_libc("pthread_attr_getschedparam", |context| {
        write_value::<i32>(context.arg_ptr(1), read_value::<i32>(context.arg_ptr(0) + PRIORITY));
        context.ret_i32(0);
    });
    in_libc("pthread_attr_setstacksize", |context| {
        // cite: bionic pthread_attr.cpp#pthread_attr_setstacksize
        let size = context.arg_u64(1);
        if size < STACK_MIN {
            return context.ret_i32(EINVAL);
        }
        write_value::<u64>(context.arg_ptr(0) + STACK_SIZE, size);
        context.ret_i32(0);
    });
    in_libc("pthread_attr_getstacksize", |context| {
        write_value::<u64>(context.arg_ptr(1), read_value::<u64>(context.arg_ptr(0) + STACK_SIZE));
        context.ret_i32(0);
    });
    in_libc("pthread_attr_setstack", |context| {
        // cite: bionic pthread_attr.cpp#pthread_attr_setstack
        let (attr, base, size) = (context.arg_ptr(0), context.arg_u64(1), context.arg_u64(2));
        if size & (GUEST_PAGE - 1) != 0 || size < STACK_MIN || base & (GUEST_PAGE - 1) != 0 {
            return context.ret_i32(EINVAL);
        }
        write_value::<u64>(attr + STACK_BASE, base);
        write_value::<u64>(attr + STACK_SIZE, size);
        context.ret_i32(0);
    });
    in_libc("pthread_attr_getstack", |context| {
        let attr = context.arg_ptr(0);
        write_value::<u64>(context.arg_ptr(1), read_value::<u64>(attr + STACK_BASE));
        write_value::<u64>(context.arg_ptr(2), read_value::<u64>(attr + STACK_SIZE));
        context.ret_i32(0);
    });
    in_libc("pthread_attr_setguardsize", |context| {
        write_value::<u64>(context.arg_ptr(0) + GUARD_SIZE, context.arg_u64(1));
        context.ret_i32(0);
    });
    in_libc("pthread_attr_getguardsize", |context| {
        write_value::<u64>(context.arg_ptr(1), read_value::<u64>(context.arg_ptr(0) + GUARD_SIZE));
        context.ret_i32(0);
    });
    in_libc("pthread_attr_setscope", |context| {
        // cite: bionic pthread_attr.cpp#pthread_attr_setscope
        context.ret_i32(match context.arg_i32(1) {
            0 => 0,
            1 => ENOTSUP,
            _ => EINVAL,
        });
    });
    in_libc("pthread_attr_getscope", |context| {
        write_value::<i32>(context.arg_ptr(1), 0);
        context.ret_i32(0);
    });
}
