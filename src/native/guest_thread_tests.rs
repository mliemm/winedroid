use super::bridge_test_support::Bridged;
use super::guest_thread::{attach_current, current_record, is_primary, is_record_live, set_primary};
use super::pthread_internal::PthreadInternal;
use super::test_deadline::bounded_test;
use super::thread_block::block;

fn guest_values(bridged: &Bridged) -> (usize, i32, i32, u64) {
    let record = bridged.call("fx_self_record").invoke().unwrap().u64() as usize;
    let tid = bridged.call("fx_tid").invoke().unwrap().i32();
    let pid = bridged.call("fx_pid").invoke().unwrap().i32();
    let canary = bridged.call("fx_canary_slot").invoke().unwrap().u64();
    (record, tid, pid, canary)
}

bounded_test! {
    fn each_thread_that_enters_guest_code_gets_its_own_pthread_internal_record() {
        let bridged = std::sync::Arc::new(Bridged::load("16k", "libfx_bridge.so"));
        let (own_record, own_tid, own_pid, _) = guest_values(&bridged);

        let meeting = std::sync::Arc::new(std::sync::Barrier::new(2));
        let workers: Vec<_> = (0..2)
            .map(|_| {
                let bridged = bridged.clone();
                let meeting = meeting.clone();
                std::thread::spawn(move || {
                    let first = guest_values(&bridged);
                    let second = guest_values(&bridged);
                    assert_eq!(first, second, "attach is idempotent per thread");
                    let record = first.0 as *const PthreadInternal;
                    assert_eq!(unsafe { (*record).tid }, first.1, "guest gettid reads the record tid field at +16");
                    assert_eq!(unsafe { (*record).cached_pid }, first.2 as u32);
                    meeting.wait();
                    first
                })
            })
            .collect();
        let results: Vec<_> = workers.into_iter().map(|worker| worker.join().unwrap()).collect();

        let records = [own_record, results[0].0, results[1].0];
        let tids = [own_tid, results[0].1, results[1].1];
        assert!(records[0] != records[1] && records[1] != records[2] && records[0] != records[2], "{records:x?}");
        assert!(tids[0] != tids[1] && tids[1] != tids[2] && tids[0] != tids[2], "{tids:?}");
        assert!(results.iter().all(|(_, _, pid, _)| *pid == own_pid));
        assert!(own_pid > 0);
        assert!(results.iter().all(|(record, tid, ..)| !is_record_live(*record, *tid)), "records are freed when the host thread exits");
        assert!(is_record_live(own_record, own_tid));
    }
}

bounded_test! {
    fn the_stack_guard_slot_of_the_thread_pointer_holds_the_same_nonzero_canary_on_every_guest_thread() {
        let bridged = std::sync::Arc::new(Bridged::load("16k", "libfx_bridge.so"));
        let other = {
            let bridged = bridged.clone();
            std::thread::spawn(move || bridged.call("fx_canary_slot").invoke().unwrap().u64()).join().unwrap()
        };

        assert_eq!(guest_values(&bridged).3, block().canary());
        assert_eq!(other, block().canary());
        assert_ne!(other, 0);
    }
}

bounded_test! {
    fn the_first_attached_thread_is_primary_with_tid_equal_to_the_pid_and_others_are_not() {
        let bridged = std::sync::Arc::new(Bridged::load("16k", "libfx_bridge.so"));
        let (_, own_tid, pid, _) = guest_values(&bridged);
        let record = current_record().unwrap();
        let primary_here = is_primary(record);

        assert_eq!(own_tid == pid, primary_here);
        let secondary = {
            let bridged = bridged.clone();
            std::thread::spawn(move || {
                let (_, tid, pid, _) = guest_values(&bridged);
                (tid, pid, is_primary(current_record().unwrap()), set_primary().is_err())
            })
            .join()
            .unwrap()
        };
        assert_ne!(secondary.0, secondary.1);
        assert!(!secondary.2);
        assert!(secondary.3, "a second thread cannot take over the primary role");
    }
}

bounded_test! {
    fn attach_is_a_no_op_for_an_attached_thread_and_returns_the_same_record() {
        let first = attach_current().unwrap();
        assert_eq!(attach_current().unwrap(), first);
        assert_eq!(current_record(), Some(first));
    }
}

bounded_test! {
    fn the_primary_role_is_never_handed_to_a_later_thread_after_the_primary_record_is_freed() {
        use super::guest_thread::{claim_primary, empty_registry, release};
        let mut registry = empty_registry();

        assert!(claim_primary(&mut registry, 0x1000));
        release(&mut registry, 0x1000);
        assert!(!claim_primary(&mut registry, 0x2000), "tid equal to the pid belongs to the main thread for the whole process");
        assert!(!claim_primary(&mut registry, 0x1000), "a reused record address is not primary either");
    }
}
