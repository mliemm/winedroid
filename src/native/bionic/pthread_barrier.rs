use std::sync::atomic::Ordering;

use super::super::futex::{self, MATCH_ANY};
use super::abi_types::EINVAL;
use super::export::in_libc;
use super::guest_atomic::u32_at;
use super::guest_mem::{read_value, write_value};
use super::pthread_mutex::EBUSY;

const STATE: usize = 4;
const WAIT_COUNT: usize = 8;
const PSHARED: usize = 12;
const WAIT: u32 = 0;
const RELEASE: u32 = 1;
const SERIAL_THREAD: i32 = -1;

fn wait_while(barrier: usize, value: u32) {
    while u32_at(barrier + STATE).load(Ordering::Acquire) == value {
        futex::wait(barrier + STATE, value, MATCH_ANY, None);
    }
}

fn barrier_wait(barrier: usize) -> i32 {
    // cite: bionic pthread_barrier.cpp#pthread_barrier_wait
    let init_count = u32_at(barrier).load(Ordering::Relaxed);
    wait_while(barrier, RELEASE);
    let count = u32_at(barrier + WAIT_COUNT);
    let mut previous = count.load(Ordering::Relaxed);
    loop {
        if previous >= init_count {
            return EINVAL;
        }
        match count.compare_exchange_weak(previous, previous + 1, Ordering::AcqRel, Ordering::Relaxed) {
            Ok(_) => break,
            Err(current) => previous = current,
        }
    }
    let mut result = 0;
    if previous + 1 == init_count {
        result = SERIAL_THREAD;
        if previous != 0 {
            u32_at(barrier + STATE).store(RELEASE, Ordering::Release);
            futex::wake(barrier + STATE, previous as i32, MATCH_ANY);
        }
    } else {
        wait_while(barrier, WAIT);
    }
    if count.fetch_sub(1, Ordering::Release) == 1 {
        u32_at(barrier + STATE).store(WAIT, Ordering::Release);
        futex::wake(barrier + STATE, init_count as i32, MATCH_ANY);
    }
    result
}

pub fn register() {
    in_libc("pthread_barrierattr_init", |context| {
        write_value::<i32>(context.arg_ptr(0), 0);
        context.ret_i32(0);
    });
    in_libc("pthread_barrierattr_destroy", |context| context.ret_i32(0));
    in_libc("pthread_barrierattr_getpshared", |context| {
        write_value::<i32>(context.arg_ptr(1), read_value::<i32>(context.arg_ptr(0)) & 1);
        context.ret_i32(0);
    });
    in_libc("pthread_barrierattr_setpshared", |context| {
        // cite: bionic pthread_barrier.cpp#pthread_barrierattr_setpshared
        let address = context.arg_ptr(0);
        let shared = i32::from(context.arg_i32(1) == 1);
        write_value::<i32>(address, (read_value::<i32>(address) & !1) | shared);
        context.ret_i32(0);
    });
    in_libc("pthread_barrier_init", |context| {
        // cite: bionic pthread_barrier.cpp#pthread_barrier_init
        let (barrier, attribute, count) = (context.arg_ptr(0), context.arg_ptr(1), context.arg_u32(2));
        if count == 0 {
            return context.ret_i32(EINVAL);
        }
        u32_at(barrier).store(count, Ordering::Relaxed);
        u32_at(barrier + STATE).store(WAIT, Ordering::Relaxed);
        u32_at(barrier + WAIT_COUNT).store(0, Ordering::Relaxed);
        write_value::<u8>(barrier + PSHARED, u8::from(attribute != 0 && read_value::<i32>(attribute) & 1 != 0));
        context.ret_i32(0);
    });
    in_libc("pthread_barrier_wait", |context| context.ret_i32(barrier_wait(context.arg_ptr(0))));
    in_libc("pthread_barrier_destroy", |context| {
        // cite: bionic pthread_barrier.cpp#pthread_barrier_destroy
        let barrier = context.arg_ptr(0);
        if u32_at(barrier).load(Ordering::Relaxed) == 0 {
            return context.ret_i32(EINVAL);
        }
        wait_while(barrier, RELEASE);
        if u32_at(barrier + WAIT_COUNT).load(Ordering::Relaxed) != 0 {
            return context.ret_i32(EBUSY);
        }
        u32_at(barrier).store(0, Ordering::Relaxed);
        context.ret_i32(0);
    });
}
