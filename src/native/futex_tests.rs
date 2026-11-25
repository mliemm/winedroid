use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::bionic::abs_timeout::Deadline;
use super::futex::{lock_pi, queued_at, requeue, trylock_pi, unlock_pi, wait, wake, wake_op, EAGAIN, EDEADLK, EINVAL, ENOSYS, EPERM, ETIMEDOUT, MATCH_ANY};
use super::test_deadline::bounded_test;

fn address_of(cell: &AtomicU32) -> usize {
    cell as *const AtomicU32 as usize
}

fn after(milliseconds: i64) -> Deadline {
    Deadline::after_ns(1, milliseconds * 1_000_000)
}

bounded_test! {
    fn futex_wait_returns_eagain_when_the_word_differs_from_the_expected_value() {
        // cite: bionic bionic_futex.cpp#FutexWithTimeout
        let cell = AtomicU32::new(5);

        assert_eq!(wait(address_of(&cell), 4, MATCH_ANY, None), -EAGAIN);
    }
}

bounded_test! {
    fn futex_wait_and_wake_reject_unaligned_addresses_and_empty_bitsets() {
        // cite: bionic futex.h#FUTEX_WAIT_BITSET, FUTEX_WAKE_BITSET
        let cell = AtomicU32::new(0);

        assert_eq!(wait(address_of(&cell) + 1, 0, MATCH_ANY, None), -EINVAL);
        assert_eq!(wait(address_of(&cell), 0, 0, None), -EINVAL);
        assert_eq!(wake(address_of(&cell) + 2, 1, MATCH_ANY), -EINVAL);
        assert_eq!(wake(address_of(&cell), 1, 0), -EINVAL);
    }
}

bounded_test! {
    fn futex_wait_times_out_after_the_requested_time_and_leaves_no_waiter_behind() {
        // cite: bionic bionic_futex.cpp#FutexWithTimeout
        let cell = AtomicU32::new(0);
        let started = Instant::now();

        let result = wait(address_of(&cell), 0, MATCH_ANY, Some(after(60)));

        assert_eq!(result, -ETIMEDOUT);
        assert!(started.elapsed() >= Duration::from_millis(55), "{:?}", started.elapsed());
        assert_eq!(wake(address_of(&cell), i32::MAX, MATCH_ANY), 0);
    }
}

bounded_test! {
    fn futex_wait_checks_the_value_before_an_expired_deadline() {
        // cite: bionic bionic_futex.cpp#FutexWithTimeout
        let cell = AtomicU32::new(7);

        assert_eq!(wait(address_of(&cell), 0, MATCH_ANY, Some(after(-5))), -EAGAIN);
        assert_eq!(wait(address_of(&cell), 7, MATCH_ANY, Some(after(-5))), -ETIMEDOUT);
    }
}

bounded_test! {
    fn futex_wake_releases_a_blocked_waiter_and_counts_only_woken_waiters() {
        // cite: bionic futex.h#FUTEX_WAKE
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
}

bounded_test! {
    fn futex_wake_count_bounds_the_number_of_released_waiters() {
        // cite: bionic futex.h#FUTEX_WAKE
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
}

bounded_test! {
    fn futex_bitset_wake_skips_waiters_whose_mask_does_not_intersect() {
        // cite: bionic futex.h#FUTEX_WAKE_BITSET
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
}

bounded_test! {
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
}

bounded_test! {
    fn futex_store_then_wake_never_loses_a_wakeup_over_many_rounds() {
        // cite: bionic bionic_futex.cpp#FutexWithTimeout
        // cite: bionic bionic_futex.h#__futex_wake_ex
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
}

bounded_test! {
    fn futex_wake_with_a_count_of_zero_or_less_wakes_one_waiter_like_the_kernel() {
        // cite: bionic futex.h#FUTEX_WAKE
        let cell = Arc::new(AtomicU32::new(0));
        let waiters = spawn_waiters(&cell, 3);

        assert_eq!(wake(address_of(&cell), 0, MATCH_ANY), 1);
        assert_eq!(wake(address_of(&cell), -7, MATCH_ANY), 1);
        assert_eq!(wake(address_of(&cell), i32::MAX, MATCH_ANY), 1);
        assert!(waiters.into_iter().all(|waiter| waiter.join().unwrap() == 0));
    }
}

fn spawn_waiters(cell: &Arc<AtomicU32>, count: usize) -> Vec<std::thread::JoinHandle<i32>> {
    let waiters = (0..count)
        .map(|_| {
            let cell = cell.clone();
            std::thread::spawn(move || wait(address_of(&cell), cell.load(Ordering::SeqCst), MATCH_ANY, Some(after(5000))))
        })
        .collect();
    while queued_at(address_of(cell)) < count {
        std::thread::yield_now();
    }
    waiters
}

const SET: u32 = 0;
const ADD: u32 = 1;
const OR: u32 = 2;
const ANDN: u32 = 3;
const XOR: u32 = 4;
const EQ: u32 = 0;
const NE: u32 = 1;
const LT: u32 = 2;
const LE: u32 = 3;
const GT: u32 = 4;
const GE: u32 = 5;
const OPARG_SHIFT: u32 = 8;

fn encode(operation: u32, compare: u32, operand: i32, comparand: i32) -> u32 {
    (operation << 28) | (compare << 24) | ((operand as u32 & 0xFFF) << 12) | (comparand as u32 & 0xFFF)
}

fn word_after(operation: u32, start: u32, operand: i32) -> u32 {
    let (first, second) = (AtomicU32::new(0), AtomicU32::new(start));
    wake_op(address_of(&first), 1, address_of(&second), 1, encode(operation, EQ, operand, 0));
    second.load(Ordering::SeqCst)
}

fn second_word_waiters_woken_when(start: u32, compare: u32, comparand: i32) -> i32 {
    let (first, second) = (AtomicU32::new(0), Arc::new(AtomicU32::new(start)));
    let waiters = spawn_waiters(&second, 1);
    let woken = wake_op(address_of(&first), 1, address_of(&second), 1, encode(ADD, compare, 0, comparand));
    wake(address_of(&second), i32::MAX, MATCH_ANY);
    assert!(waiters.into_iter().all(|waiter| waiter.join().unwrap() == 0));
    woken
}

bounded_test! {
    fn futex_wake_op_wakes_the_first_word_always_and_the_second_word_only_when_the_comparison_holds() {
        // cite: bionic futex.h#FUTEX_OP, FUTEX_OP_ADD, FUTEX_OP_CMP_EQ
        let (first, second) = (Arc::new(AtomicU32::new(0)), Arc::new(AtomicU32::new(5)));
        let (on_first, on_second) = (spawn_waiters(&first, 1), spawn_waiters(&second, 1));

        let matched = wake_op(address_of(&first), 1, address_of(&second), 1, encode(ADD, EQ, 3, 5));

        assert_eq!((matched, second.load(Ordering::SeqCst)), (2, 8));
        assert!(on_first.into_iter().chain(on_second).all(|waiter| waiter.join().unwrap() == 0));
        let on_first = spawn_waiters(&first, 1);
        let on_second = spawn_waiters(&second, 1);

        let unmatched = wake_op(address_of(&first), 1, address_of(&second), 1, encode(ADD, EQ, 0, 5));

        assert_eq!(unmatched, 1, "the second word holds 8, not 5");
        assert_eq!(queued_at(address_of(&second)), 1);
        wake(address_of(&second), 1, MATCH_ANY);
        assert!(on_first.into_iter().chain(on_second).all(|waiter| waiter.join().unwrap() == 0));
    }
}

bounded_test! {
    fn futex_wake_op_applies_each_operation_to_the_second_word() {
        // cite: bionic futex.h#FUTEX_OP_SET, FUTEX_OP_ADD, FUTEX_OP_OR, FUTEX_OP_ANDN, FUTEX_OP_XOR
        assert_eq!(word_after(SET, 12, 3), 3);
        assert_eq!(word_after(ADD, 12, 3), 15);
        assert_eq!(word_after(OR, 12, 3), 15);
        assert_eq!(word_after(ANDN, 12, 4), 8);
        assert_eq!(word_after(XOR, 12, 5), 9);
    }
}

bounded_test! {
    fn futex_wake_op_decodes_a_signed_12_bit_operand_and_the_oparg_shift_flag() {
        // cite: bionic futex.h#FUTEX_OP_ADD, FUTEX_OP_OPARG_SHIFT
        assert_eq!(word_after(ADD, 5, -1), 4, "operand 0xFFF is minus one");
        assert_eq!(word_after(ADD, 5, -2048), 5u32.wrapping_sub(2048));
        assert_eq!(word_after(ADD, 5, 2047), 5 + 2047);
        assert_eq!(word_after(SET | OPARG_SHIFT, 0, 3), 8, "the shift flag turns operand 3 into 1 << 3");
        assert_eq!(word_after(SET | OPARG_SHIFT, 0, 33), 2, "the shift count is masked to five bits");
    }
}

bounded_test! {
    fn futex_wake_op_compares_signed_values_with_every_comparison_code() {
        // cite: bionic futex.h#FUTEX_OP_CMP_EQ, FUTEX_OP_CMP_NE, FUTEX_OP_CMP_LT, FUTEX_OP_CMP_LE, FUTEX_OP_CMP_GT, FUTEX_OP_CMP_GE
        let cases = [
            (EQ, 5, 5, true), (EQ, 5, 4, false), (NE, 5, 4, true), (NE, 5, 5, false), (LT, 5, 6, true), (LT, 5, 5, false), (LE, 5, 5, true), (LE, 5, 4, false),
            (GT, 5, 4, true), (GT, 5, 5, false), (GE, 5, 5, true), (GE, 5, 6, false), (GT, u32::MAX, -2, true), (LT, u32::MAX, 0, true), (LT, 5, -2, false),
        ];
        for (compare, start, comparand, expected) in cases {
            assert_eq!(second_word_waiters_woken_when(start, compare, comparand), i32::from(expected), "compare {compare} of {start} with {comparand}");
        }
    }
}

bounded_test! {
    fn futex_wake_op_with_an_unknown_operation_or_comparison_fails_with_enosys_and_wakes_nobody() {
        // cite: bionic futex.h#FUTEX_OP
        let (first, second) = (Arc::new(AtomicU32::new(0)), AtomicU32::new(5));
        let waiters = spawn_waiters(&first, 1);

        assert_eq!(wake_op(address_of(&first), 1, address_of(&second), 1, encode(5, EQ, 1, 0)), -ENOSYS);
        assert_eq!(second.load(Ordering::SeqCst), 5, "an unknown operation leaves the word alone");
        assert_eq!(wake_op(address_of(&first), 1, address_of(&second), 1, encode(ADD, 6, 0, 0)), -ENOSYS);
        assert_eq!(queued_at(address_of(&first)), 1, "the first word is not woken when the second word's operation fails");
        assert_eq!(wake_op(address_of(&first) + 1, 1, address_of(&second), 1, 0), -EINVAL);
        wake(address_of(&first), 1, MATCH_ANY);
        assert!(waiters.into_iter().all(|waiter| waiter.join().unwrap() == 0));
    }
}

bounded_test! {
    fn futex_cmp_requeue_wakes_the_requested_count_and_a_later_wake_on_the_second_word_releases_the_moved_waiters() {
        // cite: bionic futex.h#FUTEX_CMP_REQUEUE
        let (first, second) = (Arc::new(AtomicU32::new(0)), Arc::new(AtomicU32::new(0)));
        let waiters = spawn_waiters(&first, 3);

        assert_eq!(requeue(address_of(&first), 1, address_of(&second), i32::MAX, Some(0)), 3);
        assert_eq!(queued_at(address_of(&first)), 0);
        assert_eq!(queued_at(address_of(&second)), 2);
        assert_eq!(wake(address_of(&second), i32::MAX, MATCH_ANY), 2);
        assert!(waiters.into_iter().all(|waiter| waiter.join().unwrap() == 0));
    }
}

bounded_test! {
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
