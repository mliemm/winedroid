use std::any::Any;
use std::sync::atomic::AtomicU32;
use std::sync::mpsc;
use std::time::Duration;

use super::fork_gate;
use super::futex::{self, MATCH_ANY};
use super::guest_thread::{self, attach_current};
use super::loader_test_support::child_result;
use super::test_deadline::bounded_test;
use super::thread_list;

const HOLD: Duration = Duration::from_millis(300);
const CHILD_LIMIT_SECONDS: u32 = 10;

type Hold = fn(&mut Vec<Box<dyn Any>>);

fn hold_in_background(hold: Hold) -> std::thread::JoinHandle<()> {
    let (ready, started) = mpsc::channel();
    let holder = std::thread::spawn(move || {
        let mut held = Vec::new();
        hold(&mut held);
        ready.send(()).unwrap();
        std::thread::sleep(HOLD);
        drop(held);
    });
    started.recv().unwrap();
    holder
}

fn fork_a_fresh_thread_that_attaches() -> (Option<i32>, Option<i32>) {
    std::thread::spawn(|| {
        let (signal, code, _) = child_result(|| {
            unsafe { libc::alarm(CHILD_LIMIT_SECONDS) };
            attach_current().expect("the forked child attaches its thread");
            thread_list::inherit_name(1, 2);
            let word = AtomicU32::new(0);
            futex::wake(&word as *const AtomicU32 as usize, 1, MATCH_ANY);
        });
        (signal, code)
    })
    .join()
    .unwrap()
}

bounded_test! {
    fn a_fork_waits_for_a_thread_holding_the_guest_thread_registry_and_the_child_can_attach() {
        // cite: bionic fork.cpp#fork
        // cite: bionic pthread_atfork.cpp#__bionic_atfork_run_prepare, __bionic_atfork_run_child, __bionic_atfork_run_parent
        fork_gate::install();
        let holder = hold_in_background(guest_thread::hold_for_fork);

        assert_eq!(fork_a_fresh_thread_that_attaches(), (None, Some(0)));
        holder.join().unwrap();
    }
}

bounded_test! {
    fn a_fork_waits_for_a_thread_holding_the_host_thread_and_name_tables_and_the_child_can_attach() {
        // cite: bionic fork.cpp#fork
        // cite: bionic pthread_atfork.cpp#__bionic_atfork_run_prepare, __bionic_atfork_run_child, __bionic_atfork_run_parent
        fork_gate::install();
        let holder = hold_in_background(thread_list::hold_for_fork);

        assert_eq!(fork_a_fresh_thread_that_attaches(), (None, Some(0)));
        holder.join().unwrap();
    }
}

bounded_test! {
    fn a_fork_waits_for_a_thread_holding_every_futex_bucket_and_the_child_can_wake() {
        // cite: bionic fork.cpp#fork
        // cite: bionic pthread_atfork.cpp#__bionic_atfork_run_prepare, __bionic_atfork_run_child, __bionic_atfork_run_parent
        fork_gate::install();
        let holder = hold_in_background(futex::hold_for_fork);

        assert_eq!(fork_a_fresh_thread_that_attaches(), (None, Some(0)));
        holder.join().unwrap();
    }
}

bounded_test! {
    fn the_gate_releases_every_lock_in_the_parent_so_registry_calls_still_work_after_a_fork() {
        // cite: bionic fork.cpp#fork
        // cite: bionic pthread_atfork.cpp#__bionic_atfork_run_prepare, __bionic_atfork_run_child, __bionic_atfork_run_parent
        fork_gate::install();
        let (signal, code, _) = std::thread::spawn(|| child_result(|| {})).join().unwrap();

        assert_eq!((signal, code), (None, Some(0)));
        assert!(!guest_thread::is_live_address(1));
        assert!(thread_list::host_of(1).is_none());
        assert_eq!(futex::wake(&AtomicU32::new(0) as *const AtomicU32 as usize, 1, MATCH_ANY), 0);
    }
}
