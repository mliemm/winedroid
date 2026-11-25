use super::bionic::abs_timeout::{Deadline, CLOCK_MONOTONIC, CLOCK_REALTIME};
use super::bionic::current_thread;
use super::bionic::guest_mem::read_value;
use super::futex::{self, MATCH_ANY};
use super::host_log;

const ENOSYS: i64 = 38;
const EINVAL: i64 = 22;
const FUTEX_WAIT: i32 = 0;
const FUTEX_WAKE: i32 = 1;
const FUTEX_REQUEUE: i32 = 3;
const FUTEX_CMP_REQUEUE: i32 = 4;
const FUTEX_WAKE_OP: i32 = 5;
const FUTEX_LOCK_PI: i32 = 6;
const FUTEX_UNLOCK_PI: i32 = 7;
const FUTEX_TRYLOCK_PI: i32 = 8;
const FUTEX_WAIT_BITSET: i32 = 9;
const FUTEX_WAKE_BITSET: i32 = 10;
const FUTEX_WAIT_REQUEUE_PI: i32 = 11;
const FUTEX_LOCK_PI2: i32 = 13;
const FUTEX_PRIVATE_FLAG: i32 = 128;
const FUTEX_CLOCK_REALTIME: i32 = 256;

fn absolute(timeout: usize, clock: i32) -> Option<Deadline> {
    (timeout != 0).then(|| {
        let (seconds, nanos) = read_value::<(i64, i64)>(timeout);
        Deadline { clock, seconds, nanos }
    })
}

fn relative(timeout: usize) -> Result<Option<Deadline>, i64> {
    if timeout == 0 {
        return Ok(None);
    }
    let (seconds, nanos) = read_value::<(i64, i64)>(timeout);
    if seconds < 0 || !(0..1_000_000_000).contains(&nanos) {
        return Err(EINVAL);
    }
    Ok(Some(Deadline::after_ns(CLOCK_MONOTONIC, seconds.saturating_mul(1_000_000_000).saturating_add(nanos))))
}

pub fn futex_syscall(arguments: [u64; 6]) -> i64 {
    // cite: bionic futex.h#FUTEX_WAIT_BITSET, FUTEX_LOCK_PI, FUTEX_TID_MASK, FUTEX_OP
    let (address, operation, value, timeout, address2, value3) =
        (arguments[0] as usize, arguments[1] as i32, arguments[2] as u32, arguments[3] as usize, arguments[4] as usize, arguments[5] as u32);
    let command = operation & !(FUTEX_PRIVATE_FLAG | FUTEX_CLOCK_REALTIME);
    if operation & FUTEX_CLOCK_REALTIME != 0 && !matches!(command, FUTEX_WAIT_BITSET | FUTEX_WAIT_REQUEUE_PI | FUTEX_LOCK_PI2) {
        return -ENOSYS;
    }
    let clock = if operation & FUTEX_CLOCK_REALTIME != 0 { CLOCK_REALTIME } else { CLOCK_MONOTONIC };
    let result = match command {
        FUTEX_WAIT => match relative(timeout) {
            Ok(deadline) => futex::wait(address, value, MATCH_ANY, deadline),
            Err(code) => return -code,
        },
        FUTEX_WAIT_BITSET => futex::wait(address, value, value3, absolute(timeout, clock)),
        FUTEX_WAKE => futex::wake(address, value as i32, MATCH_ANY),
        FUTEX_WAKE_BITSET => futex::wake(address, value as i32, value3),
        FUTEX_REQUEUE => futex::requeue(address, value as i32, address2, timeout as i32, None),
        FUTEX_CMP_REQUEUE => futex::requeue(address, value as i32, address2, timeout as i32, Some(value3)),
        FUTEX_WAKE_OP => futex::wake_op(address, value as i32, address2, timeout as i32, value3),
        FUTEX_LOCK_PI => futex::lock_pi(address, current_thread::tid(), absolute(timeout, CLOCK_REALTIME)),
        FUTEX_LOCK_PI2 => futex::lock_pi(address, current_thread::tid(), absolute(timeout, clock)),
        FUTEX_UNLOCK_PI => futex::unlock_pi(address, current_thread::tid()),
        FUTEX_TRYLOCK_PI => futex::trylock_pi(address, current_thread::tid()),
        other => {
            host_log::line(&format!("FIXME: futex command {other} is not translated; next missing primitive: futex command {other}"));
            return -ENOSYS;
        }
    };
    i64::from(result)
}
