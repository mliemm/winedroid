use super::abi_types::EINVAL;

const CLOCK_REALTIME: i32 = 0;
const CLOCK_MONOTONIC: i32 = 1;
const CLOCK_PROCESS_CPUTIME_ID: i32 = 2;
const CLOCK_THREAD_CPUTIME_ID: i32 = 3;
const CLOCK_MONOTONIC_RAW: i32 = 4;
const CLOCK_REALTIME_COARSE: i32 = 5;
const CLOCK_MONOTONIC_COARSE: i32 = 6;
const CLOCK_BOOTTIME: i32 = 7;
const CLOCK_REALTIME_ALARM: i32 = 8;
const CLOCK_BOOTTIME_ALARM: i32 = 9;
const CLOCK_TAI: i32 = 11;

pub fn host_clock(guest: i32) -> Option<libc::clockid_t> {
    // cite: bionic time.h#CLOCK_REALTIME, CLOCK_MONOTONIC, CLOCK_BOOTTIME
    Some(match guest {
        CLOCK_REALTIME | CLOCK_REALTIME_COARSE | CLOCK_REALTIME_ALARM | CLOCK_TAI => libc::CLOCK_REALTIME,
        CLOCK_MONOTONIC | CLOCK_MONOTONIC_COARSE => libc::CLOCK_UPTIME_RAW,
        CLOCK_MONOTONIC_RAW => libc::CLOCK_MONOTONIC_RAW,
        CLOCK_BOOTTIME | CLOCK_BOOTTIME_ALARM => libc::CLOCK_MONOTONIC,
        CLOCK_PROCESS_CPUTIME_ID => libc::CLOCK_PROCESS_CPUTIME_ID,
        CLOCK_THREAD_CPUTIME_ID => libc::CLOCK_THREAD_CPUTIME_ID,
        _ => return None,
    })
}

pub fn is_thread_cpu_clock(guest: i32) -> bool {
    guest == CLOCK_THREAD_CPUTIME_ID
}

pub fn now(guest: i32) -> Result<libc::timespec, i32> {
    let host = host_clock(guest).ok_or(EINVAL)?;
    let mut value = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    if unsafe { libc::clock_gettime(host, &mut value) } != 0 {
        return Err(EINVAL);
    }
    Ok(value)
}

pub fn resolution(guest: i32) -> Result<libc::timespec, i32> {
    let host = host_clock(guest).ok_or(EINVAL)?;
    let mut value = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    if unsafe { libc::clock_getres(host, &mut value) } != 0 {
        return Err(EINVAL);
    }
    Ok(value)
}
