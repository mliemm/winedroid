use std::sync::atomic::Ordering;

use super::super::futex::{self, MATCH_ANY};
use super::abi_types::{EAGAIN, EINVAL};
use super::abs_timeout::{check_timespec, deadline_at, CLOCK_MONOTONIC, CLOCK_REALTIME, ETIMEDOUT};
use super::bionic_lock;
use super::current_thread::tid;
use super::export::in_libc;
use super::guest_atomic::{i32_at, u32_at};
use super::guest_mem::{read_value, write_value};
use super::pthread_mutex::{EBUSY, EDEADLK, EPERM};

pub const RWLOCK_SIZE: usize = 56;
const WRITER_TID: usize = 4;
const PSHARED: usize = 8;
const WRITER_PREFERRED: usize = 9;
const PENDING_LOCK: usize = 12;
const PENDING_READERS: usize = 20;
const PENDING_WRITERS: usize = 24;
const READER_SERIAL: usize = 28;
const WRITER_SERIAL: usize = 32;

const PENDING_READERS_FLAG: i32 = 1;
const PENDING_WRITERS_FLAG: i32 = 2;
const READER_STEP: i32 = 4;
const WRITER_FLAG: i32 = i32::MIN;
const KIND_MASK: i64 = 2;
const RESERVED_MASK: i64 = !3;

fn state(lock: usize) -> &'static std::sync::atomic::AtomicI32 {
    i32_at(lock)
}

fn writer_preferred(lock: usize) -> bool {
    read_value::<u8>(lock + WRITER_PREFERRED) != 0
}

fn can_acquire_read(value: i32, preferred: bool) -> bool {
    // cite: bionic pthread_rwlock.cpp#__can_acquire_read_lock
    !(value < 0 || (preferred && value & PENDING_WRITERS_FLAG != 0))
}

fn can_acquire_write(value: i32) -> bool {
    // cite: bionic pthread_rwlock.cpp#__can_acquire_write_lock
    !(value < 0 || value >= READER_STEP)
}

fn try_read(lock: usize) -> i32 {
    // cite: bionic pthread_rwlock.cpp#__pthread_rwlock_tryrdlock
    let mut old = state(lock).load(Ordering::Relaxed);
    while can_acquire_read(old, writer_preferred(lock)) {
        let new = old.wrapping_add(READER_STEP);
        if new < READER_STEP {
            return EAGAIN;
        }
        match state(lock).compare_exchange_weak(old, new, Ordering::Acquire, Ordering::Relaxed) {
            Ok(_) => return 0,
            Err(current) => old = current,
        }
    }
    EBUSY
}

fn try_write(lock: usize) -> i32 {
    // cite: bionic pthread_rwlock.cpp#__pthread_rwlock_trywrlock
    let mut old = state(lock).load(Ordering::Relaxed);
    while can_acquire_write(old) {
        match state(lock).compare_exchange_weak(
            old,
            old | WRITER_FLAG,
            Ordering::Acquire,
            Ordering::Relaxed,
        ) {
            Ok(_) => {
                i32_at(lock + WRITER_TID).store(tid(), Ordering::Relaxed);
                return 0;
            }
            Err(current) => old = current,
        }
    }
    EBUSY
}

struct Side {
    count: usize,
    serial: usize,
    flag: i32,
}

const READERS: Side = Side {
    count: PENDING_READERS,
    serial: READER_SERIAL,
    flag: PENDING_READERS_FLAG,
};
const WRITERS: Side = Side {
    count: PENDING_WRITERS,
    serial: WRITER_SERIAL,
    flag: PENDING_WRITERS_FLAG,
};

fn block_on(
    lock: usize,
    side: &Side,
    realtime: bool,
    timeout: usize,
    can_acquire: impl Fn(i32) -> bool,
) -> i32 {
    let pending = lock + PENDING_LOCK;
    bionic_lock::lock(pending);
    u32_at(lock + side.count).fetch_add(1, Ordering::Relaxed);
    let old = state(lock).fetch_or(side.flag, Ordering::Relaxed);
    let serial = u32_at(lock + side.serial).load(Ordering::Relaxed);
    bionic_lock::unlock(pending);
    let mut outcome = 0;
    if !can_acquire(old) {
        let deadline = deadline_at(
            timeout,
            if realtime {
                CLOCK_REALTIME
            } else {
                CLOCK_MONOTONIC
            },
        );
        outcome = futex::wait(lock + side.serial, serial, MATCH_ANY, deadline);
    }
    bionic_lock::lock(pending);
    if u32_at(lock + side.count).fetch_sub(1, Ordering::Relaxed) == 1 {
        state(lock).fetch_and(!side.flag, Ordering::Relaxed);
    }
    bionic_lock::unlock(pending);
    if outcome == -ETIMEDOUT {
        ETIMEDOUT
    } else {
        0
    }
}

fn timed_read(lock: usize, realtime: bool, timeout: usize) -> i32 {
    // cite: bionic pthread_rwlock.cpp#__pthread_rwlock_timedrdlock
    if i32_at(lock + WRITER_TID).load(Ordering::Relaxed) == tid() {
        return EDEADLK;
    }
    loop {
        let result = try_read(lock);
        if result == 0 || result == EAGAIN {
            return result;
        }
        let status = check_timespec(timeout, true);
        if status != 0 {
            return status;
        }
        if can_acquire_read(state(lock).load(Ordering::Relaxed), writer_preferred(lock)) {
            continue;
        }
        let preferred = writer_preferred(lock);
        if block_on(lock, &READERS, realtime, timeout, |old| {
            can_acquire_read(old, preferred)
        }) == ETIMEDOUT
        {
            return ETIMEDOUT;
        }
    }
}

fn timed_write(lock: usize, realtime: bool, timeout: usize) -> i32 {
    // cite: bionic pthread_rwlock.cpp#__pthread_rwlock_timedwrlock
    if i32_at(lock + WRITER_TID).load(Ordering::Relaxed) == tid() {
        return EDEADLK;
    }
    loop {
        if try_write(lock) == 0 {
            return 0;
        }
        let status = check_timespec(timeout, true);
        if status != 0 {
            return status;
        }
        if can_acquire_write(state(lock).load(Ordering::Relaxed)) {
            continue;
        }
        if block_on(lock, &WRITERS, realtime, timeout, can_acquire_write) == ETIMEDOUT {
            return ETIMEDOUT;
        }
    }
}

fn unlock(lock: usize) -> i32 {
    // cite: bionic pthread_rwlock.cpp#pthread_rwlock_unlock
    let mut old = state(lock).load(Ordering::Relaxed);
    if old < 0 {
        if i32_at(lock + WRITER_TID).load(Ordering::Relaxed) != tid() {
            return EPERM;
        }
        i32_at(lock + WRITER_TID).store(0, Ordering::Relaxed);
        old = state(lock).fetch_and(i32::MAX, Ordering::Release);
        if old & (PENDING_READERS_FLAG | PENDING_WRITERS_FLAG) == 0 {
            return 0;
        }
    } else if old >= READER_STEP {
        old = state(lock).fetch_sub(READER_STEP, Ordering::Release);
        if old >> 2 != 1 || old & (PENDING_READERS_FLAG | PENDING_WRITERS_FLAG) == 0 {
            return 0;
        }
    } else {
        return EPERM;
    }
    let pending = lock + PENDING_LOCK;
    bionic_lock::lock(pending);
    if u32_at(lock + PENDING_WRITERS).load(Ordering::Relaxed) != 0 {
        u32_at(lock + WRITER_SERIAL).fetch_add(1, Ordering::Relaxed);
        bionic_lock::unlock(pending);
        futex::wake(lock + WRITER_SERIAL, 1, MATCH_ANY);
    } else if u32_at(lock + PENDING_READERS).load(Ordering::Relaxed) != 0 {
        u32_at(lock + READER_SERIAL).fetch_add(1, Ordering::Relaxed);
        bionic_lock::unlock(pending);
        futex::wake(lock + READER_SERIAL, i32::MAX, MATCH_ANY);
    } else {
        bionic_lock::unlock(pending);
    }
    0
}

fn init(lock: usize, attribute: usize) -> i32 {
    // cite: bionic pthread_rwlock.cpp#pthread_rwlock_init
    unsafe { std::ptr::write_bytes(lock as *mut u8, 0, RWLOCK_SIZE) };
    if attribute != 0 {
        let value: i64 = read_value(attribute);
        write_value::<u8>(lock + PSHARED, (value & 1) as u8);
        match (value & KIND_MASK) >> 1 {
            0 => {}
            1 => write_value::<u8>(lock + WRITER_PREFERRED, 1),
            _ => return EINVAL,
        }
        if value & RESERVED_MASK != 0 {
            return EINVAL;
        }
    }
    bionic_lock::init(lock + PENDING_LOCK, read_value::<u8>(lock + PSHARED) != 0);
    0
}

fn clocked(clock: i32, read: bool, lock: usize, timeout: usize) -> i32 {
    let realtime = match clock {
        CLOCK_MONOTONIC => false,
        CLOCK_REALTIME => true,
        _ => return EINVAL,
    };
    if read {
        timed_read(lock, realtime, timeout)
    } else {
        timed_write(lock, realtime, timeout)
    }
}

fn register_attributes() {
    // cite: bionic pthread_rwlock.cpp#pthread_rwlockattr_init, pthread_rwlockattr_destroy, pthread_rwlockattr_setpshared, pthread_rwlockattr_setkind_np
    in_libc("pthread_rwlockattr_init", |context| {
        write_value::<i64>(context.arg_ptr(0), 0);
        context.ret_i32(0);
    });
    in_libc("pthread_rwlockattr_destroy", |context| {
        write_value::<i64>(context.arg_ptr(0), -1);
        context.ret_i32(0);
    });
    in_libc("pthread_rwlockattr_getpshared", |context| {
        write_value::<i32>(
            context.arg_ptr(1),
            (read_value::<i64>(context.arg_ptr(0)) & 1) as i32,
        );
        context.ret_i32(0);
    });
    in_libc("pthread_rwlockattr_setpshared", |context| {
        let shared = i64::from(context.arg_i32(1));
        if shared != 0 && shared != 1 {
            return context.ret_i32(EINVAL);
        }
        let address = context.arg_ptr(0);
        write_value::<i64>(address, (read_value::<i64>(address) & !1) | shared);
        context.ret_i32(0);
    });
    in_libc("pthread_rwlockattr_getkind_np", |context| {
        write_value::<i32>(
            context.arg_ptr(1),
            ((read_value::<i64>(context.arg_ptr(0)) & KIND_MASK) >> 1) as i32,
        );
        context.ret_i32(0);
    });
    in_libc("pthread_rwlockattr_setkind_np", |context| {
        let kind = i64::from(context.arg_i32(1));
        if kind != 0 && kind != 1 {
            return context.ret_i32(EINVAL);
        }
        let address = context.arg_ptr(0);
        write_value::<i64>(
            address,
            (read_value::<i64>(address) & !KIND_MASK) | (kind << 1),
        );
        context.ret_i32(0);
    });
}

pub fn register() {
    register_attributes();
    in_libc("pthread_rwlock_init", |context| {
        context.ret_i32(init(context.arg_ptr(0), context.arg_ptr(1)))
    });
    in_libc("pthread_rwlock_destroy", |context| {
        // cite: bionic pthread_rwlock.cpp#pthread_rwlock_destroy
        let busy = state(context.arg_ptr(0)).load(Ordering::Relaxed) != 0;
        context.ret_i32(if busy { EBUSY } else { 0 });
    });
    in_libc("pthread_rwlock_tryrdlock", |context| {
        context.ret_i32(try_read(context.arg_ptr(0)))
    });
    in_libc("pthread_rwlock_trywrlock", |context| {
        context.ret_i32(try_write(context.arg_ptr(0)))
    });
    in_libc("pthread_rwlock_rdlock", |context| {
        let lock = context.arg_ptr(0);
        context.ret_i32(if try_read(lock) == 0 {
            0
        } else {
            timed_read(lock, false, 0)
        });
    });
    in_libc("pthread_rwlock_wrlock", |context| {
        let lock = context.arg_ptr(0);
        context.ret_i32(if try_write(lock) == 0 {
            0
        } else {
            timed_write(lock, false, 0)
        });
    });
    in_libc("pthread_rwlock_timedrdlock", |context| {
        context.ret_i32(timed_read(context.arg_ptr(0), true, context.arg_ptr(1)))
    });
    in_libc("pthread_rwlock_timedrdlock_monotonic_np", |context| {
        context.ret_i32(timed_read(context.arg_ptr(0), false, context.arg_ptr(1)))
    });
    in_libc("pthread_rwlock_timedwrlock", |context| {
        context.ret_i32(timed_write(context.arg_ptr(0), true, context.arg_ptr(1)))
    });
    in_libc("pthread_rwlock_timedwrlock_monotonic_np", |context| {
        context.ret_i32(timed_write(context.arg_ptr(0), false, context.arg_ptr(1)))
    });
    in_libc("pthread_rwlock_clockrdlock", |context| {
        context.ret_i32(clocked(
            context.arg_i32(1),
            true,
            context.arg_ptr(0),
            context.arg_ptr(2),
        ))
    });
    in_libc("pthread_rwlock_clockwrlock", |context| {
        context.ret_i32(clocked(
            context.arg_i32(1),
            false,
            context.arg_ptr(0),
            context.arg_ptr(2),
        ))
    });
    in_libc("pthread_rwlock_unlock", |context| {
        context.ret_i32(unlock(context.arg_ptr(0)))
    });
}
