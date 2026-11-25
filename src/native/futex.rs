use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use super::bionic::abs_timeout::Deadline;

pub const MATCH_ANY: u32 = 0xFFFF_FFFF;
pub const EAGAIN: i32 = 11;
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
    fn os_sync_wait_on_address_with_timeout(address: *mut c_void, value: u64, size: usize, flags: u32, clock: u32, timeout_ns: u64) -> i32;
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
    ((address >> 2) as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_shr(56) as usize % BUCKETS
}

fn lock_bucket(index: usize) -> MutexGuard<'static, Vec<Arc<Waiter>>> {
    TABLE[index].lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn word(address: usize) -> &'static AtomicU32 {
    unsafe { AtomicU32::from_ptr(address as *mut u32) }
}

fn park(waiter: &Waiter, deadline: Option<Deadline>) -> bool {
    let flag = &waiter.woken as *const AtomicU32 as *mut c_void;
    loop {
        if waiter.woken.load(Ordering::Acquire) == 1 {
            return false;
        }
        let status = match deadline {
            None => unsafe { os_sync_wait_on_address(flag, 0, 4, 0) },
            Some(deadline) => {
                let remaining = deadline.remaining_ns();
                if remaining <= 0 {
                    return true;
                }
                unsafe { os_sync_wait_on_address_with_timeout(flag, 0, 4, 0, MACH_ABSOLUTE_TIME_CLOCK, remaining.min(LONGEST_PARK_NS) as u64) }
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
    let waiter = Arc::new(Waiter { address: AtomicUsize::new(address), bitset, woken: AtomicU32::new(0) });
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
    if !park(&waiter, deadline) {
        return 0;
    }
    if leave_queue(&waiter) {
        -ETIMEDOUT
    } else {
        0
    }
}

pub fn wake(address: usize, count: i32, bitset: u32) -> i32 {
    // cite: bionic bionic_futex.h#__futex_wake_ex
    if count < 0 || bitset == 0 {
        return -EINVAL;
    }
    let mut released: Vec<Arc<Waiter>> = Vec::new();
    {
        let mut bucket = lock_bucket(bucket_index(address));
        let mut index = 0;
        while index < bucket.len() && (released.len() as i64) < i64::from(count) {
            if bucket[index].address.load(Ordering::SeqCst) == address && bucket[index].bitset & bitset != 0 {
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
        unsafe { os_sync_wake_by_address_any(&waiter.woken as *const AtomicU32 as *mut c_void, 4, 0) };
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
        Self { low_index, low, high }
    }

    fn at(&mut self, index: usize) -> &mut Vec<Arc<Waiter>> {
        match (&mut self.high, index == self.low_index) {
            (Some(high), false) => high,
            _ => &mut self.low,
        }
    }
}

pub fn requeue(from: usize, wake_count: i32, to: usize, move_count: i32, compare: Option<u32>) -> i32 {
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

pub fn wake_op(first: usize, first_count: i32, second: usize, second_count: i32, encoded: u32) -> i32 {
    // cite: bionic futex.h#FUTEX_OP, FUTEX_OP_SET, FUTEX_OP_ADD, FUTEX_OP_OR, FUTEX_OP_ANDN, FUTEX_OP_XOR, FUTEX_OP_CMP_EQ
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
            _ => return -EINVAL,
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
        _ => return -EINVAL,
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
            if target.compare_exchange(value, tid | others, Ordering::Acquire, Ordering::SeqCst).is_ok() {
                return 0;
            }
            continue;
        }
        if value & TID_MASK == tid {
            return -EDEADLK;
        }
        let marked = value | WAITERS;
        if value != marked && target.compare_exchange(value, marked, Ordering::SeqCst, Ordering::SeqCst).is_err() {
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
    if value & TID_MASK == 0 && target.compare_exchange(value, tid as u32 | (value & WAITERS), Ordering::Acquire, Ordering::SeqCst).is_ok() {
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
            if target.compare_exchange(value, 0, Ordering::Release, Ordering::SeqCst).is_ok() {
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
mod tests {
    use std::sync::atomic::AtomicU32;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use super::*;

    fn address_of(cell: &AtomicU32) -> usize {
        cell as *const AtomicU32 as usize
    }

    fn queued_at(address: usize) -> usize {
        TABLE.iter().map(|bucket| bucket.lock().unwrap().iter().filter(|waiter| waiter.address.load(Ordering::SeqCst) == address).count()).sum()
    }

    fn after(milliseconds: i64) -> Deadline {
        Deadline::after_ns(1, milliseconds * 1_000_000)
    }

    #[test]
    fn futex_wait_returns_eagain_when_the_word_differs_from_the_expected_value() {
        // cite: bionic bionic_futex.cpp#FutexWithTimeout
        let cell = AtomicU32::new(5);

        assert_eq!(wait(address_of(&cell), 4, MATCH_ANY, None), -EAGAIN);
    }

    #[test]
    fn futex_wait_rejects_unaligned_addresses_and_empty_bitsets() {
        let cell = AtomicU32::new(0);

        assert_eq!(wait(address_of(&cell) + 1, 0, MATCH_ANY, None), -EINVAL);
        assert_eq!(wait(address_of(&cell), 0, 0, None), -EINVAL);
        assert_eq!(wake(address_of(&cell), 1, 0), -EINVAL);
        assert_eq!(wake(address_of(&cell), -1, MATCH_ANY), -EINVAL);
    }

    #[test]
    fn futex_wait_times_out_after_the_requested_time_and_leaves_no_waiter_behind() {
        let cell = AtomicU32::new(0);
        let started = Instant::now();

        let result = wait(address_of(&cell), 0, MATCH_ANY, Some(after(60)));

        assert_eq!(result, -ETIMEDOUT);
        assert!(started.elapsed() >= Duration::from_millis(55), "{:?}", started.elapsed());
        assert_eq!(wake(address_of(&cell), i32::MAX, MATCH_ANY), 0);
    }

    #[test]
    fn futex_wait_checks_the_value_before_an_expired_deadline() {
        let cell = AtomicU32::new(7);

        assert_eq!(wait(address_of(&cell), 0, MATCH_ANY, Some(after(-5))), -EAGAIN);
        assert_eq!(wait(address_of(&cell), 7, MATCH_ANY, Some(after(-5))), -ETIMEDOUT);
    }

    #[test]
    fn futex_wake_releases_a_blocked_waiter_and_counts_only_woken_waiters() {
        let cell = Arc::new(AtomicU32::new(0));
        let waiter = {
            let cell = cell.clone();
            std::thread::spawn(move || wait(address_of(&cell), 0, MATCH_ANY, None))
        };
        let mut woken = 0;
        while woken == 0 {
            std::thread::yield_now();
            woken = wake(address_of(&cell), 1, MATCH_ANY);
        }

        assert_eq!(waiter.join().unwrap(), 0);
        assert_eq!(woken, 1);
    }

    #[test]
    fn futex_wake_count_bounds_the_number_of_released_waiters() {
        let cell = Arc::new(AtomicU32::new(0));
        let waiters: Vec<_> = (0..3)
            .map(|_| {
                let cell = cell.clone();
                std::thread::spawn(move || wait(address_of(&cell), 0, MATCH_ANY, Some(after(5000))))
            })
            .collect();
        let queued = || queued_at(address_of(&cell));
        while queued() < 3 {
            std::thread::yield_now();
        }

        assert_eq!(wake(address_of(&cell), 1, MATCH_ANY), 1);
        assert_eq!(queued(), 2);
        assert_eq!(wake(address_of(&cell), i32::MAX, MATCH_ANY), 2);
        assert!(waiters.into_iter().all(|waiter| waiter.join().unwrap() == 0));
    }

    #[test]
    fn futex_bitset_wake_skips_waiters_whose_mask_does_not_intersect() {
        let cell = Arc::new(AtomicU32::new(0));
        let waiter = {
            let cell = cell.clone();
            std::thread::spawn(move || wait(address_of(&cell), 0, 2, Some(after(300))))
        };
        while queued_at(address_of(&cell)) == 0 {
            std::thread::yield_now();
        }

        assert_eq!(wake(address_of(&cell), 1, 1), 0);
        assert_eq!(waiter.join().unwrap(), -ETIMEDOUT);
    }

    #[test]
    fn futex_requeue_moves_a_waiter_to_the_second_word_where_a_wake_releases_it() {
        // cite: bionic futex.h#FUTEX_CMP_REQUEUE
        let first = Arc::new(AtomicU32::new(0));
        let second = Arc::new(AtomicU32::new(0));
        let waiter = {
            let first = first.clone();
            std::thread::spawn(move || wait(address_of(&first), 0, MATCH_ANY, Some(after(5000))))
        };
        while queued_at(address_of(&first)) == 0 {
            std::thread::yield_now();
        }

        assert_eq!(requeue(address_of(&first), 0, address_of(&second), 1, Some(1)), -EAGAIN);
        assert_eq!(requeue(address_of(&first), 0, address_of(&second), 1, Some(0)), 1);
        assert_eq!(wake(address_of(&first), 1, MATCH_ANY), 0);
        assert_eq!(wake(address_of(&second), 1, MATCH_ANY), 1);
        assert_eq!(waiter.join().unwrap(), 0);
    }

    #[test]
    fn futex_store_then_wake_never_loses_a_wakeup_over_many_rounds() {
        let cell = Arc::new(AtomicU32::new(0));
        for round in 1..=300u32 {
            let waiter = {
                let cell = cell.clone();
                std::thread::spawn(move || {
                    while cell.load(Ordering::SeqCst) < round {
                        wait(address_of(&cell), round - 1, MATCH_ANY, Some(after(3000)));
                    }
                })
            };
            cell.store(round, Ordering::SeqCst);
            wake(address_of(&cell), 1, MATCH_ANY);
            waiter.join().unwrap();
        }
    }

    #[test]
    fn futex_wake_op_applies_the_operation_and_wakes_the_second_word_only_when_the_comparison_holds() {
        // cite: bionic futex.h#FUTEX_OP
        let first = AtomicU32::new(0);
        let second = AtomicU32::new(5);
        let encoded = (1u32 << 28) | (1 << 24) | (3 << 12) | 5;

        assert_eq!(wake_op(address_of(&first), 1, address_of(&second), 1, encoded), 0);
        assert_eq!(second.load(Ordering::SeqCst), 8);
    }

    #[test]
    fn futex_pi_lock_reports_deadlock_for_the_owner_and_hands_over_on_unlock() {
        // cite: bionic futex.h#FUTEX_LOCK_PI, FUTEX_UNLOCK_PI
        let cell = AtomicU32::new(0);

        assert_eq!(lock_pi(address_of(&cell), 77, None), 0);
        assert_eq!(cell.load(Ordering::SeqCst), 77);
        assert_eq!(lock_pi(address_of(&cell), 77, None), -EDEADLK);
        assert_eq!(trylock_pi(address_of(&cell), 78), -EAGAIN);
        assert_eq!(unlock_pi(address_of(&cell), 78), -EPERM);
        assert_eq!(lock_pi(address_of(&cell), 78, Some(after(20))), -ETIMEDOUT);
        assert_eq!(unlock_pi(address_of(&cell), 77), 0);
        assert_eq!(cell.load(Ordering::SeqCst), 0);
    }
}
