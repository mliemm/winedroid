use std::any::Any;
use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use super::bionic::abs_timeout::Deadline;
use super::{signal_router, signal_state};

pub const MATCH_ANY: u32 = 0xFFFF_FFFF;
pub const EAGAIN: i32 = 11;
pub const EINTR: i32 = 4;
pub const ENOSYS: i32 = 38;
pub const EINVAL: i32 = 22;
pub const EDEADLK: i32 = 35;
pub const EPERM: i32 = 1;
pub const ETIMEDOUT: i32 = 110;
pub const WAITERS: u32 = 0x8000_0000;
pub const TID_MASK: u32 = 0x3FFF_FFFF;

const BUCKETS: usize = 256;
const MACH_ABSOLUTE_TIME_CLOCK: u32 = 32;
const LONGEST_PARK_NS: i64 = 1 << 40;

extern "C" {
    fn os_sync_wait_on_address(address: *mut c_void, value: u64, size: usize, flags: u32) -> i32;
    fn os_sync_wait_on_address_with_timeout(
        address: *mut c_void,
        value: u64,
        size: usize,
        flags: u32,
        clock: u32,
        timeout_ns: u64,
    ) -> i32;
    fn os_sync_wake_by_address_any(address: *mut c_void, size: usize, flags: u32) -> i32;
}

struct Waiter {
    address: AtomicUsize,
    bitset: u32,
    woken: AtomicU32,
}

type Bucket = Mutex<Vec<Arc<Waiter>>>;

static TABLE: [Bucket; BUCKETS] = [const { Mutex::new(Vec::new()) }; BUCKETS];

fn bucket_index(address: usize) -> usize {
    ((address >> 2) as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_shr(56) as usize
        % BUCKETS
}

fn lock_bucket(index: usize) -> MutexGuard<'static, Vec<Arc<Waiter>>> {
    TABLE[index]
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(super) fn hold_for_fork(held: &mut Vec<Box<dyn Any>>) {
    held.extend((0..BUCKETS).map(|index| Box::new(lock_bucket(index)) as Box<dyn Any>));
}

fn word(address: usize) -> &'static AtomicU32 {
    unsafe { AtomicU32::from_ptr(address as *mut u32) }
}

enum Parked {
    Woken,
    Expired,
    Interrupted,
}

fn interrupted_by_signal() -> bool {
    signal_state::pending() != 0 && signal_state::pending() & !signal_state::mask() != 0
}

fn park(waiter: &Waiter, deadline: Option<Deadline>) -> Parked {
    let flag = &waiter.woken as *const AtomicU32 as *mut c_void;
    loop {
        if waiter.woken.load(Ordering::Acquire) == 1 {
            return Parked::Woken;
        }
        if interrupted_by_signal() {
            return Parked::Interrupted;
        }
        let status = match deadline {
            None => unsafe { os_sync_wait_on_address(flag, 0, 4, 0) },
            Some(deadline) => {
                let remaining = deadline.remaining_ns();
                if remaining <= 0 {
                    return Parked::Expired;
                }
                unsafe {
                    os_sync_wait_on_address_with_timeout(
                        flag,
                        0,
                        4,
                        0,
                        MACH_ABSOLUTE_TIME_CLOCK,
                        remaining.min(LONGEST_PARK_NS) as u64,
                    )
                }
            }
        };
        if status < 0 {
            let error = unsafe { *libc::__error() };
            if error != libc::ETIMEDOUT && error != libc::EINTR {
                std::thread::yield_now();
            }
        }
    }
}

fn leave_queue(waiter: &Arc<Waiter>) -> bool {
    loop {
        let address = waiter.address.load(Ordering::SeqCst);
        let mut bucket = lock_bucket(bucket_index(address));
        if let Some(position) = bucket.iter().position(|queued| Arc::ptr_eq(queued, waiter)) {
            bucket.remove(position);
            return true;
        }
        if waiter.address.load(Ordering::SeqCst) == address {
            return false;
        }
    }
}

pub fn wait(address: usize, expected: u32, bitset: u32, deadline: Option<Deadline>) -> i32 {
    // cite: bionic bionic_futex.cpp#FutexWithTimeout
    if address % 4 != 0 || bitset == 0 {
        return -EINVAL;
    }
    let waiter = Arc::new(Waiter {
        address: AtomicUsize::new(address),
        bitset,
        woken: AtomicU32::new(0),
    });
    {
        let mut bucket = lock_bucket(bucket_index(address));
        if word(address).load(Ordering::SeqCst) != expected {
            return -EAGAIN;
        }
        if deadline.is_some_and(|deadline| deadline.remaining_ns() <= 0) {
            return -ETIMEDOUT;
        }
        bucket.push(waiter.clone());
    }
    match park(&waiter, deadline) {
        Parked::Woken => 0,
        Parked::Expired if leave_queue(&waiter) => -ETIMEDOUT,
        Parked::Interrupted if leave_queue(&waiter) => {
            signal_router::deliver_pending();
            -EINTR
        }
        Parked::Expired | Parked::Interrupted => 0,
    }
}

pub fn wake(address: usize, count: i32, bitset: u32) -> i32 {
    // cite: bionic bionic_futex.h#__futex_wake_ex
    if address % 4 != 0 || bitset == 0 {
        return -EINVAL;
    }
    let limit = i64::from(count.max(1));
    let mut released: Vec<Arc<Waiter>> = Vec::new();
    {
        let mut bucket = lock_bucket(bucket_index(address));
        let mut index = 0;
        while index < bucket.len() && (released.len() as i64) < limit {
            if bucket[index].address.load(Ordering::SeqCst) == address
                && bucket[index].bitset & bitset != 0
            {
                let waiter = bucket.remove(index);
                waiter.woken.store(1, Ordering::Release);
                released.push(waiter);
            } else {
                index += 1;
            }
        }
    }
    notify(&released);
    released.len() as i32
}

fn notify(released: &[Arc<Waiter>]) {
    for waiter in released {
        unsafe {
            os_sync_wake_by_address_any(&waiter.woken as *const AtomicU32 as *mut c_void, 4, 0)
        };
    }
}

struct Pair {
    low_index: usize,
    low: MutexGuard<'static, Vec<Arc<Waiter>>>,
    high: Option<MutexGuard<'static, Vec<Arc<Waiter>>>>,
}

impl Pair {
    fn lock(first: usize, second: usize) -> Self {
        let (low_index, high_index) = (first.min(second), first.max(second));
        let low = lock_bucket(low_index);
        let high = (high_index != low_index).then(|| lock_bucket(high_index));
        Self {
            low_index,
            low,
            high,
        }
    }

    fn at(&mut self, index: usize) -> &mut Vec<Arc<Waiter>> {
        match (&mut self.high, index == self.low_index) {
            (Some(high), false) => high,
            _ => &mut self.low,
        }
    }
}

pub fn requeue(
    from: usize,
    wake_count: i32,
    to: usize,
    move_count: i32,
    compare: Option<u32>,
) -> i32 {
    // cite: bionic futex.h#FUTEX_REQUEUE, FUTEX_CMP_REQUEUE
    if wake_count < 0 || move_count < 0 || from % 4 != 0 || to % 4 != 0 {
        return -EINVAL;
    }
    let (from_index, to_index) = (bucket_index(from), bucket_index(to));
    let mut pair = Pair::lock(from_index, to_index);
    if compare.is_some_and(|expected| word(from).load(Ordering::SeqCst) != expected) {
        return -EAGAIN;
    }
    let mut released: Vec<Arc<Waiter>> = Vec::new();
    let mut moved = Vec::new();
    {
        let source = pair.at(from_index);
        let mut index = 0;
        while index < source.len() {
            if source[index].address.load(Ordering::SeqCst) != from {
                index += 1;
            } else if (released.len() as i64) < i64::from(wake_count) {
                let waiter = source.remove(index);
                waiter.woken.store(1, Ordering::Release);
                released.push(waiter);
            } else if (moved.len() as i64) < i64::from(move_count) {
                let waiter = source.remove(index);
                waiter.address.store(to, Ordering::SeqCst);
                moved.push(waiter);
            } else {
                break;
            }
        }
    }
    let total = released.len() + moved.len();
    pair.at(to_index).extend(moved);
    drop(pair);
    notify(&released);
    total as i32
}

pub fn wake_op(
    first: usize,
    first_count: i32,
    second: usize,
    second_count: i32,
    encoded: u32,
) -> i32 {
    // cite: bionic futex.h#FUTEX_OP, FUTEX_OP_SET, FUTEX_OP_ADD, FUTEX_OP_OR, FUTEX_OP_ANDN, FUTEX_OP_XOR, FUTEX_OP_CMP_EQ
    if first % 4 != 0 || second % 4 != 0 {
        return -EINVAL;
    }
    let operation = (encoded >> 28) & 7;
    let compare = (encoded >> 24) & 15;
    let mut operand = ((encoded & 0x00FF_F000) << 8) as i32 >> 20;
    let comparand = ((encoded & 0xFFF) << 20) as i32 >> 20;
    if encoded & (8 << 28) != 0 {
        operand = 1i32.wrapping_shl(operand as u32 & 31);
    }
    let target = word(second);
    let mut old = target.load(Ordering::SeqCst);
    loop {
        let updated = match operation {
            0 => operand as u32,
            1 => old.wrapping_add(operand as u32),
            2 => old | operand as u32,
            3 => old & !(operand as u32),
            4 => old ^ operand as u32,
            _ => return -ENOSYS,
        };
        match target.compare_exchange(old, updated, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => break,
            Err(current) => old = current,
        }
    }
    let old = old as i32;
    let matched = match compare {
        0 => old == comparand,
        1 => old != comparand,
        2 => old < comparand,
        3 => old <= comparand,
        4 => old > comparand,
        5 => old >= comparand,
        _ => return -ENOSYS,
    };
    let mut total = wake(first, first_count, MATCH_ANY);
    if total >= 0 && matched {
        total += wake(second, second_count, MATCH_ANY).max(0);
    }
    total
}

pub fn lock_pi(address: usize, tid: i32, deadline: Option<Deadline>) -> i32 {
    // cite: bionic futex.h#FUTEX_LOCK_PI, FUTEX_WAITERS, FUTEX_TID_MASK
    if address % 4 != 0 {
        return -EINVAL;
    }
    let tid = tid as u32;
    let target = word(address);
    let mut slept = false;
    loop {
        let value = target.load(Ordering::SeqCst);
        if value & TID_MASK == 0 {
            let others = if slept { WAITERS } else { value & WAITERS };
            if target
                .compare_exchange(value, tid | others, Ordering::Acquire, Ordering::SeqCst)
                .is_ok()
            {
                return 0;
            }
            continue;
        }
        if value & TID_MASK == tid {
            return -EDEADLK;
        }
        let marked = value | WAITERS;
        if value != marked
            && target
                .compare_exchange(value, marked, Ordering::SeqCst, Ordering::SeqCst)
                .is_err()
        {
            continue;
        }
        slept = true;
        if wait(address, marked, MATCH_ANY, deadline) == -ETIMEDOUT {
            return -ETIMEDOUT;
        }
    }
}

pub fn trylock_pi(address: usize, tid: i32) -> i32 {
    // cite: bionic futex.h#FUTEX_TRYLOCK_PI
    let target = word(address);
    let value = target.load(Ordering::SeqCst);
    if value & TID_MASK == tid as u32 {
        return -EDEADLK;
    }
    if value & TID_MASK == 0
        && target
            .compare_exchange(
                value,
                tid as u32 | (value & WAITERS),
                Ordering::Acquire,
                Ordering::SeqCst,
            )
            .is_ok()
    {
        return 0;
    }
    -EAGAIN
}

pub fn unlock_pi(address: usize, tid: i32) -> i32 {
    // cite: bionic futex.h#FUTEX_UNLOCK_PI
    let target = word(address);
    loop {
        let value = target.load(Ordering::SeqCst);
        if value & TID_MASK != tid as u32 {
            return -EPERM;
        }
        if value & WAITERS == 0 {
            if target
                .compare_exchange(value, 0, Ordering::Release, Ordering::SeqCst)
                .is_ok()
            {
                return 0;
            }
            continue;
        }
        target.store(0, Ordering::Release);
        wake(address, 1, MATCH_ANY);
        return 0;
    }
}

#[cfg(test)]
pub(super) fn queued_at(address: usize) -> usize {
    TABLE
        .iter()
        .map(|bucket| {
            bucket
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .iter()
                .filter(|waiter| waiter.address.load(Ordering::SeqCst) == address)
                .count()
        })
        .sum()
}
