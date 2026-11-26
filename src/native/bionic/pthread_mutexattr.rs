use super::abi_types::EINVAL;
use super::export::in_libc;
use super::guest_mem::{read_value, write_value};

pub(super) const ATTR_TYPE_MASK: i64 = 0xF;
pub(super) const ATTR_SHARED_MASK: i64 = 0x10;
pub(super) const ATTR_PROTOCOL_MASK: i64 = 0x20;
pub(super) const ATTR_PROTOCOL_SHIFT: i64 = 5;
pub(super) const TYPE_NORMAL_VALUE: i64 = 0;
pub(super) const TYPE_RECURSIVE_VALUE: i64 = 1;
pub(super) const TYPE_ERRORCHECK_VALUE: i64 = 2;
pub(super) const PRIO_INHERIT: i64 = 1;

fn attribute_get(attribute: usize) -> i64 {
    read_value(attribute)
}

pub fn register() {
    // cite: bionic pthread_mutex.cpp#pthread_mutexattr_init, pthread_mutexattr_destroy, pthread_mutexattr_gettype, pthread_mutexattr_settype
    in_libc("pthread_mutexattr_init", |context| {
        write_value::<i64>(context.arg_ptr(0), 0);
        context.ret_i32(0);
    });
    in_libc("pthread_mutexattr_destroy", |context| {
        write_value::<i64>(context.arg_ptr(0), -1);
        context.ret_i32(0);
    });
    in_libc("pthread_mutexattr_gettype", |context| {
        let kind = attribute_get(context.arg_ptr(0)) & ATTR_TYPE_MASK;
        if !(0..=2).contains(&kind) {
            return context.ret_i32(EINVAL);
        }
        write_value::<i32>(context.arg_ptr(1), kind as i32);
        context.ret_i32(0);
    });
    in_libc("pthread_mutexattr_settype", |context| {
        let kind = i64::from(context.arg_i32(1));
        if !(0..=2).contains(&kind) {
            return context.ret_i32(EINVAL);
        }
        let attribute = context.arg_ptr(0);
        write_value::<i64>(
            attribute,
            (attribute_get(attribute) & !ATTR_TYPE_MASK) | kind,
        );
        context.ret_i32(0);
    });
    in_libc("pthread_mutexattr_setpshared", |context| {
        let attribute = context.arg_ptr(0);
        match context.arg_i32(1) {
            0 => write_value::<i64>(attribute, attribute_get(attribute) & !ATTR_SHARED_MASK),
            1 => write_value::<i64>(attribute, attribute_get(attribute) | ATTR_SHARED_MASK),
            _ => return context.ret_i32(EINVAL),
        }
        context.ret_i32(0);
    });
    in_libc("pthread_mutexattr_getpshared", |context| {
        write_value::<i32>(
            context.arg_ptr(1),
            i32::from(attribute_get(context.arg_ptr(0)) & ATTR_SHARED_MASK != 0),
        );
        context.ret_i32(0);
    });
    in_libc("pthread_mutexattr_setprotocol", |context| {
        let protocol = i64::from(context.arg_i32(1));
        if !(0..=1).contains(&protocol) {
            return context.ret_i32(EINVAL);
        }
        let attribute = context.arg_ptr(0);
        write_value::<i64>(
            attribute,
            (attribute_get(attribute) & !ATTR_PROTOCOL_MASK) | (protocol << ATTR_PROTOCOL_SHIFT),
        );
        context.ret_i32(0);
    });
    in_libc("pthread_mutexattr_getprotocol", |context| {
        write_value::<i32>(
            context.arg_ptr(1),
            ((attribute_get(context.arg_ptr(0)) & ATTR_PROTOCOL_MASK) >> ATTR_PROTOCOL_SHIFT)
                as i32,
        );
        context.ret_i32(0);
    });
}
