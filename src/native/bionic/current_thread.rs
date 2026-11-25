use super::super::guest_thread::attach_current;

pub fn record() -> usize {
    attach_current().expect("guest thread record") as usize
}

pub fn tid() -> i32 {
    unsafe { (*(record() as *const super::super::pthread_internal::PthreadInternal)).tid }
}
