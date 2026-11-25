use super::bionic::sdk::set_application_target_sdk;
use super::test_deadline::bounded_test;
use super::thread_test_support::{each_variant, shim, word};

bounded_test! {
    fn bionic_static_initializers_have_the_documented_byte_patterns() {
        // cite: bionic pthread.h#PTHREAD_MUTEX_INITIALIZER, PTHREAD_RECURSIVE_MUTEX_INITIALIZER_NP, PTHREAD_ERRORCHECK_MUTEX_INITIALIZER_NP, PTHREAD_COND_INITIALIZER_MONOTONIC_NP
        each_variant(|bridged| {
            let object = |name: &str, bytes: usize| unsafe { std::slice::from_raw_parts(bridged.symbol(name) as *const u8, bytes).to_vec() };

            assert_eq!(object("fx_static_mutex", 40), vec![0u8; 40]);
            assert_eq!(word(bridged.symbol("fx_static_recursive")), 0x4000);
            assert_eq!(word(bridged.symbol("fx_static_errorcheck")), 0x8000);
            assert_eq!(object("fx_static_cond", 48), vec![0u8; 48]);
            assert_eq!(word(bridged.symbol("fx_static_cond_monotonic")), 2);
            assert_eq!(object("fx_static_rwlock", 56), vec![0u8; 56]);
        });
    }
}

bounded_test! {
    fn bionic_zero_initialized_static_mutex_locks_reports_busy_and_unlocks() {
        // cite: bionic pthread_mutex.cpp#pthread_mutex_lock, pthread_mutex_trylock, pthread_mutex_unlock
        each_variant(|bridged| {
            assert_eq!(bridged.call("fx_static_mutex_zero_state").invoke().unwrap().i32(), 1600);
        });
    }
}

bounded_test! {
    fn bionic_mutex_contention_from_four_guest_threads_keeps_the_counter_exact_for_every_mutex_kind() {
        // cite: bionic pthread_mutex.cpp#NormalMutexLock, MutexLockWithTimeout, PIMutexTimedLock
        each_variant(|bridged| {
            for (kind, protocol) in [(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)] {
                let total = bridged.call("fx_mutex_contention").i32(kind).i32(protocol).i32(4).i32(500).invoke().unwrap().u64() as i64;
                assert_eq!(total, 2000, "type {kind} protocol {protocol}");
            }
        });
    }
}

bounded_test! {
    fn bionic_zero_initialized_static_mutex_survives_contention_across_sixteen_threads() {
        // cite: bionic pthread_mutex.cpp#NormalMutexLock
        each_variant(|bridged| {
            assert_eq!(bridged.call("fx_static_mutex_contention").i32(16).i32(200).invoke().unwrap().u64() as i64, 3200);
        });
    }
}

bounded_test! {
    fn bionic_recursive_mutex_allows_2048_locks_then_returns_eagain_and_refuses_extra_unlocks() {
        // cite: bionic pthread_mutex.cpp#RecursiveIncrement, pthread_mutex_unlock
        each_variant(|bridged| {
            assert_eq!(bridged.call("fx_recursive_depth").i32(2048).invoke().unwrap().i32(), 1, "unlock of an unlocked recursive mutex is EPERM");
            assert_eq!(bridged.call("fx_recursive_overflow_code").invoke().unwrap().i32(), 11);
        });
    }
}

bounded_test! {
    fn bionic_errorcheck_mutex_reports_edeadlk_ebusy_and_eperm() {
        // cite: bionic pthread_mutex.cpp#MutexLockWithTimeout, pthread_mutex_trylock, pthread_mutex_unlock
        each_variant(|bridged| {
            assert_eq!(bridged.call("fx_errorcheck_codes").invoke().unwrap().i32(), 35 * 1_000_000 + 16 * 1000 + 1);
        });
    }
}

bounded_test! {
    fn bionic_spinlock_reports_ebusy_for_a_second_trylock() {
        // cite: bionic pthread_spinlock.cpp#pthread_spin_trylock
        each_variant(|bridged| {
            assert_eq!(bridged.call("fx_spin_codes").invoke().unwrap().i32(), 160);
        });
    }
}

bounded_test! {
    fn bionic_sixteen_concurrent_guest_threads_keep_a_contended_counter_exact() {
        // cite: bionic pthread_create.cpp#pthread_create
        each_variant(|bridged| {
            assert_eq!(bridged.call("fx_mutex_contention").i32(0).i32(0).i32(16).i32(100).invoke().unwrap().u64() as i64, 1600);
        });
    }
}

bounded_test! {
    fn bionic_destroyed_mutex_lock_returns_ebusy_below_sdk_28_and_leaves_state_ffff() {
        // cite: bionic pthread_mutex.cpp#HandleUsingDestroyedMutex, pthread_mutex_destroy
        let mutex = Box::leak(Box::new([0xAAu8; 40])).as_mut_ptr() as usize;
        assert_eq!(shim("pthread_mutex_init").ptr(mutex).ptr(0).invoke().unwrap().i32(), 0);
        assert_eq!(shim("pthread_mutex_lock").ptr(mutex).invoke().unwrap().i32(), 0);
        assert_eq!(shim("pthread_mutex_destroy").ptr(mutex).invoke().unwrap().i32(), 16, "destroying a locked mutex is EBUSY");
        assert_eq!(shim("pthread_mutex_unlock").ptr(mutex).invoke().unwrap().i32(), 0);
        assert_eq!(shim("pthread_mutex_destroy").ptr(mutex).invoke().unwrap().i32(), 0);

        assert_eq!(unsafe { *(mutex as *const u16) }, 0xFFFF);
        set_application_target_sdk(27);
        let relock = shim("pthread_mutex_lock").ptr(mutex).invoke().unwrap().i32();
        set_application_target_sdk(0);
        assert_eq!(relock, 16);
    }
}

bounded_test! {
    fn bionic_mutex_init_rejects_a_destroyed_attribute_after_clearing_the_mutex() {
        // cite: bionic pthread_mutex.cpp#pthread_mutex_init, pthread_mutexattr_destroy
        let mutex = Box::leak(Box::new([0xAAu8; 40])).as_mut_ptr() as usize;
        let attribute = Box::leak(Box::new(0i64)) as *mut i64 as usize;
        shim("pthread_mutexattr_init").ptr(attribute).invoke().unwrap();
        shim("pthread_mutexattr_destroy").ptr(attribute).invoke().unwrap();

        assert_eq!(unsafe { *(attribute as *const i64) }, -1);
        assert_eq!(shim("pthread_mutex_init").ptr(mutex).ptr(attribute).invoke().unwrap().i32(), 22);
        assert_eq!(unsafe { std::slice::from_raw_parts(mutex as *const u8, 40).to_vec() }, vec![0u8; 40]);
    }
}

bounded_test! {
    fn bionic_pi_mutex_state_is_c000_and_destroy_frees_it() {
        // cite: bionic pthread_mutex.cpp#pthread_mutex_init, PIMutexDestroy
        let mutex = Box::leak(Box::new([0u8; 40])).as_mut_ptr() as usize;
        let attribute = Box::leak(Box::new(0i64)) as *mut i64 as usize;
        shim("pthread_mutexattr_init").ptr(attribute).invoke().unwrap();
        shim("pthread_mutexattr_setprotocol").ptr(attribute).i32(1).invoke().unwrap();
        shim("pthread_mutex_init").ptr(mutex).ptr(attribute).invoke().unwrap();

        assert_eq!(unsafe { *(mutex as *const u16) }, 0xC000);
        assert_eq!(shim("pthread_mutex_lock").ptr(mutex).invoke().unwrap().i32(), 0);
        assert_eq!(shim("pthread_mutex_destroy").ptr(mutex).invoke().unwrap().i32(), 16);
        assert_eq!(shim("pthread_mutex_unlock").ptr(mutex).invoke().unwrap().i32(), 0);
        assert_eq!(shim("pthread_mutex_destroy").ptr(mutex).invoke().unwrap().i32(), 0);
        assert_eq!(unsafe { *(mutex as *const u16) }, 0xFFFF);
    }
}

bounded_test! {
    fn bionic_mutex_destroy_works_for_every_type_with_and_without_the_shared_bit_and_refuses_a_locked_mutex() {
        // cite: bionic pthread_mutex.cpp#pthread_mutex_destroy
        for kind in 0..=2 {
            for shared in 0..=1 {
                let mutex = Box::leak(Box::new([0xAAu8; 40])).as_mut_ptr() as usize;
                let attribute = Box::leak(Box::new(0i64)) as *mut i64 as usize;
                shim("pthread_mutexattr_init").ptr(attribute).invoke().unwrap();
                shim("pthread_mutexattr_settype").ptr(attribute).i32(kind).invoke().unwrap();
                shim("pthread_mutexattr_setpshared").ptr(attribute).i32(shared).invoke().unwrap();
                assert_eq!(shim("pthread_mutex_init").ptr(mutex).ptr(attribute).invoke().unwrap().i32(), 0, "type {kind} shared {shared}");

                assert_eq!(shim("pthread_mutex_lock").ptr(mutex).invoke().unwrap().i32(), 0);
                assert_eq!(shim("pthread_mutex_destroy").ptr(mutex).invoke().unwrap().i32(), 16, "locked, type {kind} shared {shared}");
                assert_eq!(shim("pthread_mutex_unlock").ptr(mutex).invoke().unwrap().i32(), 0);
                assert_eq!(shim("pthread_mutex_destroy").ptr(mutex).invoke().unwrap().i32(), 0, "type {kind} shared {shared}");
                assert_eq!(unsafe { *(mutex as *const u16) }, 0xFFFF, "type {kind} shared {shared}");
            }
        }
    }
}
