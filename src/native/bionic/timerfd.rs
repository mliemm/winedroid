use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::{Arc, Condvar, Mutex, Once, Weak};
use std::time::Duration;

use super::abi_types::{EAGAIN, EFAULT, EINVAL};
use super::clocks;
use super::doorbell::Doorbell;
use super::export::in_libc;
use super::fd_table::{self, FdEntry, FdKind, SyntheticStat};
use super::fs_result::{finish_int, missing};
use super::guest_mem::{read_value, write_value};
use super::linux_abi::{epoll as flags_of, open};
use super::stat_convert::S_IFREG;

const EPERM: i32 = 1;
const THREAD_STACK: usize = 256 * 1024;
const CLOCK_REALTIME: i32 = 0;
const CLOCK_MONOTONIC: i32 = 1;
const CLOCK_BOOTTIME: i32 = 7;

#[derive(Default)]
struct Inner {
    interval: i64,
    next: Option<i64>,
    expirations: u64,
    generation: u64,
}

pub struct TimerFdState {
    bell: Doorbell,
    clock: i32,
    inner: Mutex<Inner>,
}

struct Scheduler {
    queue: Mutex<BinaryHeap<Reverse<(i64, u64, usize)>>>,
    wake: Condvar,
    timers: Mutex<Vec<Weak<TimerFdState>>>,
}

fn error(code: i32) -> i64 {
    -i64::from(code)
}

extern "C" {
    fn clock_gettime_nsec_np(clock: u32) -> u64;
}

const CLOCK_UPTIME_RAW: u32 = 8;

fn monotonic_ns() -> i64 {
    unsafe { clock_gettime_nsec_np(CLOCK_UPTIME_RAW) as i64 }
}

fn clock_ns(clock: i32) -> i64 {
    clocks::now(clock)
        .map(|stamp| stamp.tv_sec * 1_000_000_000 + stamp.tv_nsec)
        .unwrap_or(0)
}

static SCHEDULER: Scheduler = Scheduler {
    queue: Mutex::new(BinaryHeap::new()),
    wake: Condvar::new(),
    timers: Mutex::new(Vec::new()),
};
static START: Once = Once::new();

fn scheduler() -> &'static Scheduler {
    START.call_once(|| {
        std::thread::Builder::new()
            .name("winedroid-timerfd".to_string())
            .stack_size(THREAD_STACK)
            .spawn(run)
            .expect("timerfd thread");
    });
    &SCHEDULER
}

fn state_at(index: usize) -> Option<Arc<TimerFdState>> {
    scheduler()
        .timers
        .lock()
        .expect("timer list")
        .get(index)
        .and_then(Weak::upgrade)
}

fn schedule(index: usize, deadline: i64, generation: u64) {
    let scheduler = scheduler();
    scheduler
        .queue
        .lock()
        .expect("timer queue")
        .push(Reverse((deadline, generation, index)));
    scheduler.wake.notify_all();
}

fn fire(index: usize, deadline: i64, generation: u64) {
    let Some(state) = state_at(index) else { return };
    let mut inner = state.inner.lock().expect("timer state");
    if inner.generation != generation || inner.next != Some(deadline) {
        return;
    }
    let now = monotonic_ns();
    if inner.interval > 0 {
        let count = 1 + ((now - deadline).max(0) / inner.interval) as u64;
        inner.expirations += count;
        let next = deadline + count as i64 * inner.interval;
        inner.next = Some(next);
        schedule(index, next, generation);
    } else {
        inner.expirations += 1;
        inner.next = None;
    }
    state.bell.ring();
}

fn run() {
    let mut queue = SCHEDULER.queue.lock().expect("timer queue");
    loop {
        let now = monotonic_ns();
        match queue.peek().copied() {
            Some(Reverse((deadline, generation, index))) if deadline <= now => {
                queue.pop();
                drop(queue);
                fire(index, deadline, generation);
                queue = SCHEDULER.queue.lock().expect("timer queue");
            }
            Some(Reverse((deadline, _, _))) => {
                queue = SCHEDULER
                    .wake
                    .wait_timeout(queue, Duration::from_nanos((deadline - now) as u64))
                    .expect("timer queue")
                    .0;
            }
            None => queue = SCHEDULER.wake.wait(queue).expect("timer queue"),
        }
    }
}

pub fn create(clock: i32, flags: i32) -> i64 {
    // cite: bionic timerfd.h#TFD_TIMER_ABSTIME
    if !matches!(clock, CLOCK_REALTIME | CLOCK_MONOTONIC | CLOCK_BOOTTIME) {
        return error(if matches!(clock, 8 | 9) {
            EPERM
        } else {
            EINVAL
        });
    }
    if flags & !(flags_of::EFD_NONBLOCK | flags_of::EFD_CLOEXEC) != 0 {
        return error(EINVAL);
    }
    let bell = match Doorbell::new() {
        Ok(bell) => bell,
        Err(failure) => return failure,
    };
    let fd = bell.guest;
    if flags & flags_of::EFD_CLOEXEC != 0 {
        unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) };
    }
    let state = Arc::new(TimerFdState {
        bell,
        clock,
        inner: Mutex::new(Inner::default()),
    });
    scheduler()
        .timers
        .lock()
        .expect("timer list")
        .push(Arc::downgrade(&state));
    let guest_flags = open::RDWR
        | if flags & flags_of::EFD_NONBLOCK != 0 {
            open::NONBLOCK
        } else {
            0
        };
    let entry = FdEntry::new(FdKind::TimerFd, "anon_inode:[timerfd]", guest_flags)
        .with_synthetic(SyntheticStat {
            mode: S_IFREG | 0o600,
            dev: (0, 0xe),
            rdev: (0, 0),
            zero_size: false,
        })
        .with_payload(state);
    fd_table::adopt(fd, entry);
    i64::from(fd)
}

pub fn read(entry: &FdEntry, buffer: usize, length: usize) -> i64 {
    if length < 8 {
        return error(EINVAL);
    }
    if buffer == 0 {
        return error(EFAULT);
    }
    let Some(state) = entry.payload_as::<TimerFdState>() else {
        return error(EINVAL);
    };
    loop {
        {
            let mut inner = state.inner.lock().expect("timer state");
            if inner.expirations > 0 {
                let count = std::mem::take(&mut inner.expirations);
                state.bell.drain();
                write_value::<u64>(buffer, count);
                return 8;
            }
        }
        if entry.flags() & open::NONBLOCK != 0 {
            return error(EAGAIN);
        }
        let waited = state.bell.wait_readable();
        if waited < 0 {
            return waited;
        }
    }
}

fn read_spec(pointer: usize) -> Result<(i64, i64), i32> {
    let (seconds, nanos) = (read_value::<i64>(pointer), read_value::<i64>(pointer + 8));
    if seconds < 0 || !(0..1_000_000_000).contains(&nanos) {
        return Err(EINVAL);
    }
    Ok((seconds, nanos))
}

fn write_spec(pointer: usize, interval: i64, remaining: i64) {
    for (offset, value) in [(0, interval), (16, remaining)] {
        write_value::<i64>(pointer + offset, value / 1_000_000_000);
        write_value::<i64>(pointer + offset + 8, value % 1_000_000_000);
    }
}

fn remaining_of(inner: &Inner) -> i64 {
    inner.next.map_or(0, |next| (next - monotonic_ns()).max(1))
}

fn state_of(fd: i32) -> Result<Arc<TimerFdState>, i32> {
    let entry = fd_table::get(fd).ok_or(9)?;
    if entry.kind != FdKind::TimerFd {
        return Err(EINVAL);
    }
    entry.payload_as::<TimerFdState>().ok_or(EINVAL)
}

fn index_of(state: &Arc<TimerFdState>) -> usize {
    scheduler()
        .timers
        .lock()
        .expect("timer list")
        .iter()
        .position(|weak| {
            weak.upgrade()
                .is_some_and(|candidate| Arc::ptr_eq(&candidate, state))
        })
        .expect("timer is registered")
}

pub fn set_time(fd: i32, flags: i32, new_pointer: usize, old_pointer: usize) -> i64 {
    let state = match state_of(fd) {
        Ok(state) => state,
        Err(errno) => return error(errno),
    };
    if flags & !(flags_of::TFD_TIMER_ABSTIME | flags_of::TFD_TIMER_CANCEL_ON_SET) != 0 {
        return error(EINVAL);
    }
    if new_pointer == 0 {
        return error(EFAULT);
    }
    let (interval, value) = match (read_spec(new_pointer), read_spec(new_pointer + 16)) {
        (Ok(interval), Ok(value)) => (
            interval.0 * 1_000_000_000 + interval.1,
            value.0 * 1_000_000_000 + value.1,
        ),
        (Err(errno), _) | (_, Err(errno)) => return error(errno),
    };
    if flags & flags_of::TFD_TIMER_CANCEL_ON_SET != 0 {
        if flags & flags_of::TFD_TIMER_ABSTIME == 0 || state.clock != CLOCK_REALTIME {
            return error(EINVAL);
        }
        missing(
            "timerfd cancel-on-set",
            "the host cannot report wall clock steps",
        );
    }
    let mut inner = state.inner.lock().expect("timer state");
    if old_pointer != 0 {
        write_spec(old_pointer, inner.interval, remaining_of(&inner));
    }
    inner.generation += 1;
    inner.expirations = 0;
    state.bell.drain();
    inner.interval = interval;
    inner.next = None;
    if value > 0 {
        let relative = if flags & flags_of::TFD_TIMER_ABSTIME != 0 {
            (value - clock_ns(state.clock)).max(0)
        } else {
            value
        };
        let deadline = monotonic_ns() + relative;
        inner.next = Some(deadline);
        let generation = inner.generation;
        drop(inner);
        schedule(index_of(&state), deadline, generation);
    }
    0
}

pub fn get_time(fd: i32, pointer: usize) -> i64 {
    let state = match state_of(fd) {
        Ok(state) => state,
        Err(errno) => return error(errno),
    };
    if pointer == 0 {
        return error(EFAULT);
    }
    let inner = state.inner.lock().expect("timer state");
    write_spec(pointer, inner.interval, remaining_of(&inner));
    0
}

pub fn register() {
    in_libc("timerfd_create", |context| {
        let result = create(context.arg_i32(0), context.arg_i32(1));
        finish_int(context, result);
    });
    in_libc("timerfd_settime", |context| {
        let result = set_time(
            context.arg_i32(0),
            context.arg_i32(1),
            context.arg_ptr(2),
            context.arg_ptr(3),
        );
        finish_int(context, result);
    });
    in_libc("timerfd_gettime", |context| {
        let result = get_time(context.arg_i32(0), context.arg_ptr(1));
        finish_int(context, result);
    });
}
