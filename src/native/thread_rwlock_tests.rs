use super::test_deadline::bounded_test;
use super::thread_test_support::each_variant;

bounded_test! {
    fn bionic_rwlock_readers_share_writers_exclude_and_unlock_checks_ownership() {
        // cite: bionic pthread_rwlock.cpp#__pthread_rwlock_tryrdlock, __pthread_rwlock_trywrlock, pthread_rwlock_unlock
        each_variant(|bridged| {
            assert_eq!(bridged.call("fx_rwlock_exclusion_codes").invoke().unwrap().i32(), 10);
            assert_eq!(bridged.call("fx_rwlock_timedwrlock_code").invoke().unwrap().i32(), 110);
            assert_eq!(bridged.call("fx_rwlock_reader_overflow_code").invoke().unwrap().i32(), 11);
        });
    }
}

bounded_test! {
    fn bionic_rwlock_prefer_writer_kind_refuses_new_readers_while_a_writer_waits() {
        // cite: bionic pthread_rwlock.cpp#__can_acquire_read_lock
        each_variant(|bridged| {
            assert_eq!(bridged.call("fx_rwlock_pending_writer_blocks_new_readers").invoke().unwrap().i32(), 16 * 100);
        });
    }
}

bounded_test! {
    fn bionic_rwlock_keeps_a_pair_of_counters_consistent_under_mixed_readers_and_writers_for_both_kinds() {
        // cite: bionic pthread_rwlock.cpp#pthread_rwlock_unlock, __pthread_rwlock_timedrdlock, __pthread_rwlock_timedwrlock
        each_variant(|bridged| {
            for kind in [0, 1] {
                let packed = bridged.call("fx_rwlock_mixed").i32(kind).i32(4).i32(3).i32(150).invoke().unwrap().u64() as i64;

                assert_eq!(packed, 3 * 150, "kind {kind}: torn reads times 100000000 plus the writer total");
            }
        });
    }
}

bounded_test! {
    fn bionic_rwlock_readers_blocked_behind_a_writer_all_resume_when_it_unlocks() {
        // cite: bionic pthread_rwlock.cpp#pthread_rwlock_unlock, __pthread_rwlock_timedrdlock
        each_variant(|bridged| {
            assert_eq!(bridged.call("fx_rwlock_readers_wait_for_the_writer").i32(6).invoke().unwrap().i32(), 61);
        });
    }
}
