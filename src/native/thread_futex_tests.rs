use std::sync::atomic::AtomicU32;
use std::time::{Duration, Instant};

use super::bionic::abs_timeout::{Deadline, CLOCK_MONOTONIC, CLOCK_REALTIME};
use super::bionic::syscall::translate;
use super::futex::{self, MATCH_ANY};
use super::futex_syscall::futex_syscall;
use super::test_deadline::bounded_test;
use super::thread_test_support::{each_variant, shim, word};

bounded_test! {
    fn syscall_futex_wait_wake_mismatch_and_timeout_follow_the_kernel_contract() {
        // cite: bionic futex.h#FUTEX_WAIT_BITSET, FUTEX_WAKE
        each_variant(|bridged| {
            assert_eq!(bridged.call("fx_futex_wait_then_wake").invoke().unwrap().i32() % 10, 1);
            assert_eq!(bridged.call("fx_futex_wait_mismatch_errno").invoke().unwrap().i32(), 11);
            let started = Instant::now();
            assert_eq!(bridged.call("fx_futex_wait_timeout_errno").i32(40).invoke().unwrap().i32(), 110);
            assert!(started.elapsed().as_millis() >= 35);
        });
    }
}

bounded_test! {
    fn syscall_gettid_matches_gettid() {
        // cite: bionic unistd.h#__NR_gettid
        each_variant(|bridged| {
            let packed = bridged.call("fx_gettid_via_syscall").invoke().unwrap().u64();

            assert_eq!(packed / 1_000_000, packed % 1_000_000);
            assert!(packed % 1_000_000 > 0);
        });
    }
}

bounded_test! {
    fn set_tid_address_word_is_cleared_and_woken_when_a_host_created_thread_exits() {
        // cite: bionic SYSCALLS.TXT#__set_tid_address
        let word = Box::leak(Box::new(AtomicU32::new(1))) as *const AtomicU32 as usize;
        let exiting = std::thread::spawn(move || {
            let tid = shim("syscall").int(96).ptr(word).invoke().unwrap().u64() as i32;
            std::thread::sleep(Duration::from_millis(200));
            tid
        });

        let outcome = futex::wait(word, 1, MATCH_ANY, Some(Deadline::after_ns(CLOCK_MONOTONIC, 5_000_000_000)));
        let tid = exiting.join().unwrap();

        assert_ne!(outcome, -110, "the exiting thread never woke the word");
        assert_eq!(self::word(word), 0);
        assert!(tid > 0);
    }
}

extern "C" fn register_tid_word_and_return(word: usize) -> usize {
    translate(96, [word as u64, 0, 0, 0, 0, 0]);
    word
}

bounded_test! {
    fn set_tid_address_word_is_cleared_before_a_guest_created_thread_can_be_joined() {
        // cite: bionic SYSCALLS.TXT#__set_tid_address
        let word = Box::leak(Box::new(AtomicU32::new(1))) as *const AtomicU32 as usize;
        let thread = Box::leak(Box::new(0u64)) as *mut u64 as usize;

        assert_eq!(shim("pthread_create").ptr(thread).ptr(0).ptr(register_tid_word_and_return as *const () as usize).ptr(word).invoke().unwrap().i32(), 0);
        let joined = shim("pthread_join").ptr(unsafe { *(thread as *const u64) } as usize).ptr(0).invoke().unwrap().i32();

        assert_eq!(joined, 0);
        assert_eq!(self::word(word), 0);
    }
}

bounded_test! {
    fn set_tid_address_with_a_null_address_unregisters_the_word() {
        // cite: bionic SYSCALLS.TXT#__set_tid_address
        let word = Box::leak(Box::new(AtomicU32::new(1))) as *const AtomicU32 as usize;
        std::thread::spawn(move || {
            shim("syscall").int(96).ptr(word).invoke().unwrap();
            shim("syscall").int(96).ptr(0).invoke().unwrap();
        })
        .join()
        .unwrap();

        assert_eq!(self::word(word), 1);
    }
}

const FUTEX_LOCK_PI: i32 = 6;
const FUTEX_LOCK_PI2: i32 = 13;
const FUTEX_PRIVATE_FLAG: i32 = 128;
const FUTEX_CLOCK_REALTIME: i32 = 256;
const OWNER_TID: u32 = 4242;

fn timespec_after(clock: i32, milliseconds: i64) -> usize {
    let deadline = Deadline::after_ns(clock, milliseconds * 1_000_000);
    Box::leak(Box::new((deadline.seconds, deadline.nanos))) as *const (i64, i64) as usize
}

fn lock_pi_syscall(cell: &AtomicU32, operation: i32, timeout: usize) -> i64 {
    futex_syscall([cell as *const AtomicU32 as u64, (operation | FUTEX_PRIVATE_FLAG) as u64, 0, timeout as u64, 0, 0])
}

fn times_out_after_the_deadline(operation: i32, clock: i32) -> bool {
    let cell = AtomicU32::new(OWNER_TID);
    let started = Instant::now();

    let result = lock_pi_syscall(&cell, operation, timespec_after(clock, 60));

    result == -110 && started.elapsed() >= Duration::from_millis(50)
}

bounded_test! {
    fn futex_lock_pi2_reads_its_timeout_on_the_monotonic_clock_and_on_the_realtime_clock_only_with_the_flag() {
        // cite: bionic futex.h#FUTEX_LOCK_PI2, FUTEX_CLOCK_REALTIME
        assert!(times_out_after_the_deadline(FUTEX_LOCK_PI2, CLOCK_MONOTONIC));
        assert!(times_out_after_the_deadline(FUTEX_LOCK_PI2 | FUTEX_CLOCK_REALTIME, CLOCK_REALTIME));
    }
}

bounded_test! {
    fn futex_lock_pi_reads_its_timeout_on_the_realtime_clock_and_refuses_the_clock_realtime_flag() {
        // cite: bionic futex.h#FUTEX_LOCK_PI, FUTEX_CLOCK_REALTIME
        assert!(times_out_after_the_deadline(FUTEX_LOCK_PI, CLOCK_REALTIME));
        let cell = AtomicU32::new(0);

        assert_eq!(lock_pi_syscall(&cell, FUTEX_LOCK_PI | FUTEX_CLOCK_REALTIME, 0), -38);
        assert_eq!(cell.load(std::sync::atomic::Ordering::SeqCst), 0, "a refused call must not take the lock");
    }
}
