use std::sync::atomic::Ordering;

use super::super::futex::{self, MATCH_ANY};
use super::abi_types::EINVAL;
use super::abs_timeout::{check_timespec, deadline_at, CLOCK_MONOTONIC, CLOCK_REALTIME, ETIMEDOUT};
use super::export::in_libc;
use super::guest_atomic::u32_at;
use super::guest_mem::{read_value, write_value};
use super::pthread_mutex;

const COND_COUNTER_STEP: u32 = 4;
const COND_CLOCK_MASK: u32 = 2;
const ATTR_DESTROYED: i64 = 0xdeada11d;
const COND_DESTROYED: u32 = 0xdeadc04d;

fn wait_on(cond: usize, mutex: usize, realtime: bool, timeout: usize) -> i32 {
    // cite: bionic pthread_cond.cpp#__pthread_cond_timedwait
    let status = check_timespec(timeout, true);
    if status != 0 {
        return status;
    }
    let state = u32_at(cond);
    let waiters = u32_at(cond + 4);
    let old = state.load(Ordering::Relaxed);
    waiters.fetch_add(1, Ordering::Relaxed);
    pthread_mutex::unlock(mutex);
    let deadline = deadline_at(timeout, if realtime { CLOCK_REALTIME } else { CLOCK_MONOTONIC });
    let outcome = futex::wait(cond, old, MATCH_ANY, deadline);
    waiters.fetch_sub(1, Ordering::Relaxed);
    pthread_mutex::lock(mutex);
    if outcome == -ETIMEDOUT {
        ETIMEDOUT
    } else {
        0
    }
}

fn pulse(cond: usize, count: i32) -> i32 {
    // cite: bionic pthread_cond.cpp#__pthread_cond_pulse
    if u32_at(cond + 4).load(Ordering::Relaxed) == 0 {
        return 0;
    }
    u32_at(cond).fetch_add(COND_COUNTER_STEP, Ordering::Relaxed);
    futex::wake(cond, count, MATCH_ANY);
    0
}

fn attribute(address: usize) -> i64 {
    read_value(address)
}

fn register_attributes() {
    // cite: bionic pthread_cond.cpp#pthread_condattr_init, pthread_condattr_setpshared, pthread_condattr_setclock, pthread_condattr_destroy
    in_libc("pthread_condattr_init", |context| {
        write_value::<i64>(context.arg_ptr(0), 0);
        context.ret_i32(0);
    });
    in_libc("pthread_condattr_destroy", |context| {
        write_value::<i64>(context.arg_ptr(0), ATTR_DESTROYED);
        context.ret_i32(0);
    });
    in_libc("pthread_condattr_getpshared", |context| {
        write_value::<i32>(context.arg_ptr(1), (attribute(context.arg_ptr(0)) & 1) as i32);
        context.ret_i32(0);
    });
    in_libc("pthread_condattr_setpshared", |context| {
        let shared = i64::from(context.arg_i32(1));
        if shared != 0 && shared != 1 {
            return context.ret_i32(EINVAL);
        }
        let address = context.arg_ptr(0);
        write_value::<i64>(address, attribute(address) | shared);
        context.ret_i32(0);
    });
    in_libc("pthread_condattr_getclock", |context| {
        write_value::<i32>(context.arg_ptr(1), ((attribute(context.arg_ptr(0)) & i64::from(COND_CLOCK_MASK)) >> 1) as i32);
        context.ret_i32(0);
    });
    in_libc("pthread_condattr_setclock", |context| {
        let clock = i64::from(context.arg_i32(1));
        if clock != i64::from(CLOCK_REALTIME) && clock != i64::from(CLOCK_MONOTONIC) {
            return context.ret_i32(EINVAL);
        }
        let address = context.arg_ptr(0);
        write_value::<i64>(address, attribute(address) | (clock << 1));
        context.ret_i32(0);
    });
}

pub fn register() {
    register_attributes();
    in_libc("pthread_cond_init", |context| {
        // cite: bionic pthread_cond.cpp#pthread_cond_init
        let (cond, attributes) = (context.arg_ptr(0), context.arg_ptr(1));
        u32_at(cond).store(if attributes == 0 { 0 } else { (attribute(attributes) & 3) as u32 }, Ordering::Relaxed);
        u32_at(cond + 4).store(0, Ordering::Relaxed);
        context.ret_i32(0);
    });
    in_libc("pthread_cond_destroy", |context| {
        // cite: bionic pthread_cond.cpp#pthread_cond_destroy
        u32_at(context.arg_ptr(0)).store(COND_DESTROYED, Ordering::Relaxed);
        context.ret_i32(0);
    });
    in_libc("pthread_cond_broadcast", |context| context.ret_i32(pulse(context.arg_ptr(0), i32::MAX)));
    in_libc("pthread_cond_signal", |context| context.ret_i32(pulse(context.arg_ptr(0), 1)));
    in_libc("pthread_cond_wait", |context| context.ret_i32(wait_on(context.arg_ptr(0), context.arg_ptr(1), false, 0)));
    in_libc("pthread_cond_timedwait", |context| {
        // cite: bionic pthread_cond.cpp#pthread_cond_timedwait
        let cond = context.arg_ptr(0);
        let realtime = u32_at(cond).load(Ordering::Relaxed) & COND_CLOCK_MASK == 0;
        context.ret_i32(wait_on(cond, context.arg_ptr(1), realtime, context.arg_ptr(2)));
    });
    in_libc("pthread_cond_timedwait_monotonic_np", |context| context.ret_i32(wait_on(context.arg_ptr(0), context.arg_ptr(1), false, context.arg_ptr(2))));
    in_libc("pthread_cond_clockwait", |context| {
        // cite: bionic pthread_cond.cpp#pthread_cond_clockwait
        let result = match context.arg_i32(2) {
            CLOCK_MONOTONIC => wait_on(context.arg_ptr(0), context.arg_ptr(1), false, context.arg_ptr(3)),
            CLOCK_REALTIME => wait_on(context.arg_ptr(0), context.arg_ptr(1), true, context.arg_ptr(3)),
            _ => EINVAL,
        };
        context.ret_i32(result);
    });
}
