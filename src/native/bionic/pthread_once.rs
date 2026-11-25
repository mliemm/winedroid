use std::sync::atomic::Ordering;

use super::super::call_assign::CallBuilder;
use super::super::futex::{self, MATCH_ANY};
use super::export::in_libc;
use super::guest_atomic::i32_at;

const NOT_YET_STARTED: i32 = 0;
const UNDERWAY: i32 = 1;
const COMPLETE: i32 = 2;

pub fn register() {
    in_libc("pthread_once", |context| {
        // cite: bionic pthread_once.cpp#pthread_once
        let (control, routine) = (context.arg_ptr(0), context.arg_ptr(1));
        let word = i32_at(control);
        let mut old = word.load(Ordering::Acquire);
        loop {
            if old == COMPLETE {
                return context.ret_i32(0);
            }
            if let Err(current) = word.compare_exchange_weak(old, UNDERWAY, Ordering::Acquire, Ordering::Acquire) {
                old = current;
                continue;
            }
            if old == NOT_YET_STARTED {
                if let Err(error) = CallBuilder::new(routine).invoke() {
                    return context.fail(error.to_string());
                }
                word.store(COMPLETE, Ordering::Release);
                futex::wake(control, i32::MAX, MATCH_ANY);
                return context.ret_i32(0);
            }
            futex::wait(control, old as u32, MATCH_ANY, None);
            old = word.load(Ordering::Acquire);
        }
    });
}
