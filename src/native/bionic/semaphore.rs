use std::sync::atomic::{AtomicU32, Ordering};

use super::super::bridge::CallContext;
use super::super::futex::{self, MATCH_ANY};
use super::abi_types::{EAGAIN, EINVAL, EOVERFLOW};
use super::abs_timeout::{check_timespec, deadline_at, CLOCK_MONOTONIC, CLOCK_REALTIME, ETIMEDOUT};
use super::export::in_libc;
use super::guest_atomic::u32_at;
use super::sdk::application_target_sdk;

const SHARED_MASK: u32 = 1;
const VALUE_MASK: u32 = 0xFFFF_FFFE;
const SEM_VALUE_MAX: i32 = 0x3FFF_FFFF;
const ONE: u32 = 2;
const MINUS_ONE: u32 = 0xFFFF_FFFE;
const ENOSYS: i32 = 38;
const EINTR: i32 = 4;

fn from_value(value: u32) -> u32 {
    value.wrapping_shl(1) & VALUE_MASK
}

fn to_value(count: u32) -> i32 {
    (count as i32) >> 1
}

fn decrement(count: u32) -> u32 {
    count.wrapping_sub(ONE) & VALUE_MASK
}

fn increment(count: u32) -> u32 {
    count.wrapping_add(ONE) & VALUE_MASK
}

fn update(count: &AtomicU32, stop: impl Fn(i32) -> bool, next: impl Fn(u32) -> u32) -> i32 {
    let mut old = count.load(Ordering::Relaxed);
    let shared = old & SHARED_MASK;
    loop {
        if stop(to_value(old)) {
            break;
        }
        match count.compare_exchange_weak(old, next(old) | shared, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => break,
            Err(current) => old = current,
        }
    }
    to_value(old)
}

fn dec(count: &AtomicU32) -> i32 {
    // cite: bionic semaphore.cpp#__sem_dec
    update(count, |value| value < 0, decrement)
}

fn try_dec(count: &AtomicU32) -> i32 {
    // cite: bionic semaphore.cpp#__sem_trydec
    update(count, |value| value <= 0, decrement)
}

fn inc(count: &AtomicU32) -> i32 {
    // cite: bionic semaphore.cpp#__sem_inc
    let mut old = count.load(Ordering::Relaxed);
    let shared = old & SHARED_MASK;
    loop {
        if to_value(old) == SEM_VALUE_MAX {
            break;
        }
        let next = if to_value(old) < 0 { ONE | shared } else { increment(old) | shared };
        match count.compare_exchange_weak(old, next, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => break,
            Err(current) => old = current,
        }
    }
    to_value(old)
}

fn fail(context: &mut CallContext, code: i32) {
    context.set_errno(code);
    context.ret_i32(-1);
}

fn timed_wait(context: &mut CallContext, sem: usize, timeout: usize, realtime: bool) {
    // cite: bionic semaphore.cpp#__sem_timedwait
    let count = u32_at(sem);
    if try_dec(count) > 0 {
        return context.ret_i32(0);
    }
    let status = check_timespec(timeout, false);
    if status != 0 {
        return fail(context, status);
    }
    let shared = count.load(Ordering::Relaxed) & SHARED_MASK;
    let deadline = deadline_at(timeout, if realtime { CLOCK_REALTIME } else { CLOCK_MONOTONIC });
    loop {
        if dec(count) > 0 {
            return context.ret_i32(0);
        }
        let result = futex::wait(sem, shared | MINUS_ONE, MATCH_ANY, deadline);
        if result == -ETIMEDOUT || result == -EINTR {
            return fail(context, -result);
        }
    }
}

pub fn register() {
    in_libc("sem_init", |context| {
        // cite: bionic semaphore.cpp#sem_init
        let value = context.arg_u32(2);
        if value > SEM_VALUE_MAX as u32 {
            return fail(context, EINVAL);
        }
        let shared = u32::from(context.arg_i32(1) != 0);
        u32_at(context.arg_ptr(0)).store(from_value(value) | shared, Ordering::Relaxed);
        context.ret_i32(0);
    });
    in_libc("sem_destroy", |context| context.ret_i32(0));
    in_libc("sem_open", |context| {
        // cite: bionic semaphore.cpp#sem_open
        context.set_errno(ENOSYS);
        context.ret_ptr(0);
    });
    in_libc("sem_close", |context| fail(context, ENOSYS));
    in_libc("sem_unlink", |context| fail(context, ENOSYS));
    in_libc("sem_wait", |context| {
        // cite: bionic semaphore.cpp#sem_wait
        let sem = context.arg_ptr(0);
        let count = u32_at(sem);
        let shared = count.load(Ordering::Relaxed) & SHARED_MASK;
        loop {
            if dec(count) > 0 {
                return context.ret_i32(0);
            }
            let result = futex::wait(sem, shared | MINUS_ONE, MATCH_ANY, None);
            if application_target_sdk() >= 24 && result == -EINTR {
                return fail(context, EINTR);
            }
        }
    });
    in_libc("sem_timedwait", |context| {
        let (sem, timeout) = (context.arg_ptr(0), context.arg_ptr(1));
        timed_wait(context, sem, timeout, true);
    });
    in_libc("sem_timedwait_monotonic_np", |context| {
        let (sem, timeout) = (context.arg_ptr(0), context.arg_ptr(1));
        timed_wait(context, sem, timeout, false);
    });
    in_libc("sem_clockwait", |context| {
        // cite: bionic semaphore.cpp#sem_clockwait
        let (sem, timeout) = (context.arg_ptr(0), context.arg_ptr(2));
        match context.arg_i32(1) {
            CLOCK_MONOTONIC => timed_wait(context, sem, timeout, false),
            CLOCK_REALTIME => timed_wait(context, sem, timeout, true),
            _ => context.ret_i32(EINVAL),
        }
    });
    in_libc("sem_post", |context| {
        // cite: bionic semaphore.cpp#sem_post
        let sem = context.arg_ptr(0);
        let old = inc(u32_at(sem));
        if old < 0 {
            futex::wake(sem, i32::MAX, MATCH_ANY);
        } else if old == SEM_VALUE_MAX {
            return fail(context, EOVERFLOW);
        }
        context.ret_i32(0);
    });
    in_libc("sem_trywait", |context| {
        // cite: bionic semaphore.cpp#sem_trywait
        if try_dec(u32_at(context.arg_ptr(0))) > 0 {
            context.ret_i32(0);
        } else {
            fail(context, EAGAIN);
        }
    });
    in_libc("sem_getvalue", |context| {
        // cite: bionic semaphore.cpp#sem_getvalue
        let value = to_value(u32_at(context.arg_ptr(0)).load(Ordering::SeqCst)).max(0);
        super::guest_mem::write_value::<i32>(context.arg_ptr(1), value);
        context.ret_i32(0);
    });
}
