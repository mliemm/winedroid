use super::abi_types::EINVAL;
use super::clocks::{is_thread_cpu_clock, now};

const TIMER_ABSTIME: i32 = 1;

pub fn nanosleep(request: &libc::timespec, remaining: &mut libc::timespec) -> i32 {
    let status = unsafe { libc::nanosleep(request, remaining) };
    if status == 0 {
        0
    } else {
        unsafe { *libc::__error() }
    }
}

pub fn sleep_seconds(seconds: u32) -> u32 {
    // cite: bionic sleep.cpp#sleep
    let request = libc::timespec { tv_sec: i64::from(seconds), tv_nsec: 0 };
    let mut remaining = request;
    if nanosleep(&request, &mut remaining) != 0 {
        return remaining.tv_sec as u32;
    }
    0
}

pub fn sleep_micros(microseconds: u32) -> i32 {
    // cite: bionic usleep.cpp#usleep
    let request = libc::timespec { tv_sec: i64::from(microseconds / 1_000_000), tv_nsec: i64::from(microseconds % 1_000_000) * 1000 };
    let mut remaining = request;
    if nanosleep(&request, &mut remaining) == 0 {
        0
    } else {
        -1
    }
}

pub fn clock_sleep(clock: i32, flags: i32, request: libc::timespec, remaining: &mut libc::timespec) -> i32 {
    // cite: bionic clock_nanosleep.cpp#clock_nanosleep
    if is_thread_cpu_clock(clock) {
        return EINVAL;
    }
    let mut relative = request;
    if flags & TIMER_ABSTIME != 0 {
        let current = match now(clock) {
            Ok(value) => value,
            Err(code) => return code,
        };
        let mut seconds = request.tv_sec - current.tv_sec;
        let mut nanoseconds = request.tv_nsec - current.tv_nsec;
        if nanoseconds < 0 {
            seconds -= 1;
            nanoseconds += 1_000_000_000;
        }
        if seconds < 0 {
            return 0;
        }
        relative = libc::timespec { tv_sec: seconds, tv_nsec: nanoseconds };
    }
    nanosleep(&relative, remaining)
}
