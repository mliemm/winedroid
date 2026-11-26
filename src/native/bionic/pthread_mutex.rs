use std::sync::atomic::{AtomicI32, AtomicU16, Ordering};

use super::super::futex::{self, MATCH_ANY, TID_MASK};
use super::abi_types::EAGAIN;
use super::abi_types::EINVAL;
use super::abs_timeout::{
    check_timespec, deadline_at, Deadline, CLOCK_MONOTONIC, CLOCK_REALTIME, ETIMEDOUT,
};
use super::current_thread::tid;
use super::export::in_libc;
use super::fatal::fortify_fatal;
use super::guest_atomic::{i32_at, u16_at};
use super::guest_mem::{read_value, write_value};
use super::pthread_mutexattr::{
    ATTR_PROTOCOL_MASK, ATTR_PROTOCOL_SHIFT, ATTR_SHARED_MASK, ATTR_TYPE_MASK, PRIO_INHERIT,
    TYPE_ERRORCHECK_VALUE, TYPE_NORMAL_VALUE, TYPE_RECURSIVE_VALUE,
};
use super::sdk::application_target_sdk;

pub const EBUSY: i32 = 16;
pub const EDEADLK: i32 = 35;
pub const EPERM: i32 = 1;

pub const MUTEX_SIZE: usize = 40;
const TYPE_MASK: u16 = 0xC000;
const SHARED_MASK: u16 = 0x2000;
const COUNTER_MASK: u16 = 0x1FFC;
const COUNTER_ONE: u16 = 4;
const TYPE_RECURSIVE: u16 = 0x4000;
const TYPE_ERRORCHECK: u16 = 0x8000;
const PI_STATE: u16 = 0xC000;
const DESTROYED: u16 = 0xFFFF;

fn state(mutex: usize) -> &'static AtomicU16 {
    u16_at(mutex)
}

fn owner(mutex: usize) -> &'static AtomicI32 {
    i32_at(mutex + 4)
}

fn pi_owner(mutex: usize) -> &'static AtomicI32 {
    i32_at(mutex + 8)
}

fn pi_type(mutex: usize) -> u8 {
    read_value::<u8>(mutex + 4)
}

fn pi_counter(mutex: usize) -> &'static AtomicU16 {
    u16_at(mutex + 6)
}

fn locked_uncontended(old: u16) -> bool {
    old & 3 == 1
}

fn normal_try_lock(mutex: usize, shared: u16) -> i32 {
    // cite: bionic pthread_mutex.cpp#NormalMutexTryLock
    match state(mutex).compare_exchange(shared, shared | 1, Ordering::Acquire, Ordering::Relaxed) {
        Ok(_) => 0,
        Err(_) => EBUSY,
    }
}

fn normal_lock(mutex: usize, shared: u16, deadline: Option<Deadline>, timeout: usize) -> i32 {
    // cite: bionic pthread_mutex.cpp#NormalMutexLock
    if normal_try_lock(mutex, shared) == 0 {
        return 0;
    }
    let status = check_timespec(timeout, true);
    if status != 0 {
        return status;
    }
    while state(mutex).swap(shared | 2, Ordering::Acquire) != shared {
        if futex::wait(mutex, u32::from(shared | 2), MATCH_ANY, deadline) == -ETIMEDOUT {
            return ETIMEDOUT;
        }
    }
    0
}

fn recursive_increment(mutex: usize, old: u16) -> i32 {
    // cite: bionic pthread_mutex.cpp#RecursiveIncrement
    if old & COUNTER_MASK == COUNTER_MASK {
        return EAGAIN;
    }
    state(mutex).fetch_add(COUNTER_ONE, Ordering::Relaxed);
    0
}

fn lock_with_timeout(mutex: usize, realtime: bool, timeout: usize) -> i32 {
    // cite: bionic pthread_mutex.cpp#MutexLockWithTimeout
    let deadline = deadline_at(
        timeout,
        if realtime {
            CLOCK_REALTIME
        } else {
            CLOCK_MONOTONIC
        },
    );
    let mut old = state(mutex).load(Ordering::Relaxed);
    let mutex_type = old & TYPE_MASK;
    let shared = old & SHARED_MASK;
    if mutex_type == 0 {
        return normal_lock(mutex, shared, deadline, timeout);
    }
    let me = tid();
    if owner(mutex).load(Ordering::Relaxed) == me {
        if mutex_type == TYPE_ERRORCHECK {
            return EDEADLK;
        }
        return recursive_increment(mutex, old);
    }
    let unlocked = mutex_type | shared;
    let uncontended = unlocked | 1;
    let contended = unlocked | 2;
    if old == unlocked
        && state(mutex)
            .compare_exchange(unlocked, uncontended, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
    {
        owner(mutex).store(me, Ordering::Relaxed);
        return 0;
    }
    loop {
        if old == unlocked {
            match state(mutex).compare_exchange_weak(
                old,
                contended,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    owner(mutex).store(me, Ordering::Relaxed);
                    return 0;
                }
                Err(current) => {
                    old = current;
                    continue;
                }
            }
        } else if locked_uncontended(old) {
            let flipped = old ^ 3;
            match state(mutex).compare_exchange_weak(
                old,
                flipped,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => old = flipped,
                Err(current) => {
                    old = current;
                    continue;
                }
            }
        }
        let status = check_timespec(timeout, true);
        if status != 0 {
            return status;
        }
        if futex::wait(mutex, u32::from(old), MATCH_ANY, deadline) == -ETIMEDOUT {
            return ETIMEDOUT;
        }
        old = state(mutex).load(Ordering::Relaxed);
    }
}

fn pi_try_lock(mutex: usize) -> i32 {
    // cite: bionic pthread_mutex.cpp#PIMutexTryLock
    let me = tid();
    if pi_owner(mutex)
        .compare_exchange(0, me, Ordering::Acquire, Ordering::Relaxed)
        .is_ok()
    {
        return 0;
    }
    let current = pi_owner(mutex).load(Ordering::Relaxed);
    if current as u32 & TID_MASK == me as u32 {
        return match i64::from(pi_type(mutex)) {
            TYPE_ERRORCHECK_VALUE => EDEADLK,
            TYPE_RECURSIVE_VALUE => {
                if pi_counter(mutex).load(Ordering::Relaxed) == 0xFFFF {
                    EAGAIN
                } else {
                    pi_counter(mutex).fetch_add(1, Ordering::Relaxed);
                    0
                }
            }
            _ => EBUSY,
        };
    }
    EBUSY
}

fn pi_timed_lock(mutex: usize, realtime: bool, timeout: usize) -> i32 {
    // cite: bionic pthread_mutex.cpp#PIMutexTimedLock
    let result = pi_try_lock(mutex);
    if result != EBUSY {
        return result;
    }
    let deadline = deadline_at(
        timeout,
        if realtime {
            CLOCK_REALTIME
        } else {
            CLOCK_MONOTONIC
        },
    );
    -futex::lock_pi(mutex + 8, tid(), deadline)
}

fn pi_unlock(mutex: usize) -> i32 {
    // cite: bionic pthread_mutex.cpp#PIMutexUnlock
    let me = tid();
    let kind = i64::from(pi_type(mutex));
    let old_owner;
    if kind == TYPE_NORMAL_VALUE {
        if pi_owner(mutex)
            .compare_exchange(me, 0, Ordering::Release, Ordering::Relaxed)
            .is_ok()
        {
            return 0;
        }
    }
    old_owner = pi_owner(mutex).load(Ordering::Relaxed);
    if me as u32 != old_owner as u32 & TID_MASK {
        return EPERM;
    }
    if kind == TYPE_RECURSIVE_VALUE && pi_counter(mutex).load(Ordering::Relaxed) != 0 {
        pi_counter(mutex).fetch_sub(1, Ordering::Relaxed);
        return 0;
    }
    if old_owner == me
        && pi_owner(mutex)
            .compare_exchange(me, 0, Ordering::Release, Ordering::Relaxed)
            .is_ok()
    {
        return 0;
    }
    -futex::unlock_pi(mutex + 8, me)
}

fn destroyed(mutex: usize, function: &str) -> i32 {
    // cite: bionic pthread_mutex.cpp#HandleUsingDestroyedMutex
    if application_target_sdk() >= 28 {
        fortify_fatal(&format!(
            "{function} called on a destroyed mutex ({mutex:#x})"
        ));
    }
    EBUSY
}

pub fn lock(mutex: usize) -> i32 {
    // cite: bionic pthread_mutex.cpp#pthread_mutex_lock
    let old = state(mutex).load(Ordering::Relaxed);
    if old & TYPE_MASK == 0 && normal_try_lock(mutex, old & SHARED_MASK) == 0 {
        return 0;
    }
    if old == PI_STATE {
        if pi_try_lock(mutex) == 0 {
            return 0;
        }
        return pi_timed_lock(mutex, false, 0);
    }
    if old == DESTROYED {
        return destroyed(mutex, "pthread_mutex_lock");
    }
    lock_with_timeout(mutex, false, 0)
}

pub fn timed_lock(mutex: usize, realtime: bool, timeout: usize, function: &str) -> i32 {
    // cite: bionic pthread_mutex.cpp#__pthread_mutex_timedlock
    let old = state(mutex).load(Ordering::Relaxed);
    if old & TYPE_MASK == 0 && normal_try_lock(mutex, old & SHARED_MASK) == 0 {
        return 0;
    }
    if old == PI_STATE {
        return pi_timed_lock(mutex, realtime, timeout);
    }
    if old == DESTROYED {
        return destroyed(mutex, function);
    }
    lock_with_timeout(mutex, realtime, timeout)
}

pub fn unlock(mutex: usize) -> i32 {
    // cite: bionic pthread_mutex.cpp#pthread_mutex_unlock
    let old = state(mutex).load(Ordering::Relaxed);
    let mutex_type = old & TYPE_MASK;
    let shared = old & SHARED_MASK;
    if mutex_type == 0 {
        if state(mutex).swap(shared, Ordering::Release) == shared | 2 {
            futex::wake(mutex, 1, MATCH_ANY);
        }
        return 0;
    }
    if old == PI_STATE {
        return pi_unlock(mutex);
    }
    if old == DESTROYED {
        return destroyed(mutex, "pthread_mutex_unlock");
    }
    if tid() != owner(mutex).load(Ordering::Relaxed) {
        return EPERM;
    }
    if old & COUNTER_MASK != 0 {
        state(mutex).fetch_sub(COUNTER_ONE, Ordering::Relaxed);
        return 0;
    }
    owner(mutex).store(0, Ordering::Relaxed);
    if state(mutex).swap(mutex_type | shared, Ordering::Release) & 3 == 2 {
        futex::wake(mutex, 1, MATCH_ANY);
    }
    0
}

pub fn try_lock(mutex: usize) -> i32 {
    // cite: bionic pthread_mutex.cpp#pthread_mutex_trylock
    let old = state(mutex).load(Ordering::Relaxed);
    let mutex_type = old & TYPE_MASK;
    if mutex_type == 0 {
        return normal_try_lock(mutex, old & SHARED_MASK);
    }
    if old == PI_STATE {
        return pi_try_lock(mutex);
    }
    if old == DESTROYED {
        return destroyed(mutex, "pthread_mutex_trylock");
    }
    let me = tid();
    if owner(mutex).load(Ordering::Relaxed) == me {
        if mutex_type == TYPE_ERRORCHECK {
            return EBUSY;
        }
        return recursive_increment(mutex, old);
    }
    let unlocked = mutex_type | (old & SHARED_MASK);
    if state(mutex)
        .compare_exchange(unlocked, unlocked | 1, Ordering::Acquire, Ordering::Relaxed)
        .is_ok()
    {
        owner(mutex).store(me, Ordering::Relaxed);
        return 0;
    }
    EBUSY
}

pub fn init(mutex: usize, attribute: usize) -> i32 {
    // cite: bionic pthread_mutex.cpp#pthread_mutex_init
    unsafe { std::ptr::write_bytes(mutex as *mut u8, 0, MUTEX_SIZE) };
    if attribute == 0 {
        return 0;
    }
    let attribute: i64 = read_value(attribute);
    let mut value: u16 = if attribute & ATTR_SHARED_MASK != 0 {
        SHARED_MASK
    } else {
        0
    };
    match attribute & ATTR_TYPE_MASK {
        TYPE_NORMAL_VALUE => {}
        TYPE_RECURSIVE_VALUE => value |= TYPE_RECURSIVE,
        TYPE_ERRORCHECK_VALUE => value |= TYPE_ERRORCHECK,
        _ => return EINVAL,
    }
    if (attribute & ATTR_PROTOCOL_MASK) >> ATTR_PROTOCOL_SHIFT == PRIO_INHERIT {
        state(mutex).store(PI_STATE, Ordering::Relaxed);
        write_value::<u8>(mutex + 4, (attribute & ATTR_TYPE_MASK) as u8);
        write_value::<u8>(mutex + 5, u8::from(attribute & ATTR_SHARED_MASK != 0));
    } else {
        state(mutex).store(value, Ordering::Relaxed);
    }
    0
}

fn destroy(mutex: usize) -> i32 {
    // cite: bionic pthread_mutex.cpp#pthread_mutex_destroy, PIMutexDestroy
    let old = state(mutex).load(Ordering::Relaxed);
    if old == DESTROYED {
        return destroyed(mutex, "pthread_mutex_destroy");
    }
    if old == PI_STATE {
        if pi_owner(mutex)
            .compare_exchange(0, -1, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            state(mutex).store(DESTROYED, Ordering::SeqCst);
            return 0;
        }
        return EBUSY;
    }
    if old & 3 == 0
        && state(mutex)
            .compare_exchange(old, DESTROYED, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
    {
        return 0;
    }
    EBUSY
}

fn clock_lock(mutex: usize, clock: i32, timeout: usize) -> i32 {
    // cite: bionic pthread_mutex.cpp#pthread_mutex_clocklock
    match clock {
        CLOCK_MONOTONIC => timed_lock(mutex, false, timeout, "pthread_mutex_clocklock"),
        CLOCK_REALTIME => timed_lock(mutex, true, timeout, "pthread_mutex_clocklock"),
        _ => {
            if state(mutex).load(Ordering::Relaxed) == DESTROYED {
                return destroyed(mutex, "pthread_mutex_clocklock");
            }
            EINVAL
        }
    }
}

pub fn register() {
    in_libc("pthread_mutex_init", |context| {
        context.ret_i32(init(context.arg_ptr(0), context.arg_ptr(1)))
    });
    in_libc("pthread_mutex_destroy", |context| {
        context.ret_i32(destroy(context.arg_ptr(0)))
    });
    in_libc("pthread_mutex_lock", |context| {
        context.ret_i32(lock(context.arg_ptr(0)))
    });
    in_libc("pthread_mutex_unlock", |context| {
        context.ret_i32(unlock(context.arg_ptr(0)))
    });
    in_libc("pthread_mutex_trylock", |context| {
        context.ret_i32(try_lock(context.arg_ptr(0)))
    });
    in_libc("pthread_mutex_timedlock", |context| {
        context.ret_i32(timed_lock(
            context.arg_ptr(0),
            true,
            context.arg_ptr(1),
            "pthread_mutex_timedlock",
        ))
    });
    in_libc("pthread_mutex_timedlock_monotonic_np", |context| {
        context.ret_i32(timed_lock(
            context.arg_ptr(0),
            false,
            context.arg_ptr(1),
            "pthread_mutex_timedlock_monotonic_np",
        ))
    });
    in_libc("pthread_mutex_clocklock", |context| {
        context.ret_i32(clock_lock(
            context.arg_ptr(0),
            context.arg_i32(1),
            context.arg_ptr(2),
        ))
    });
}
