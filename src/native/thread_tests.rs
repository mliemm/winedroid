use std::time::Instant;

use super::bionic::sdk::set_application_target_sdk;
use super::bridge::handler_address;
use super::bridge_test_support::Bridged;
use super::call_assign::CallBuilder;

fn threads() -> Bridged {
    Bridged::load("16k", "libfx_threads.so")
}

fn shim(name: &str) -> CallBuilder {
    super::bridge_test_support::register_all();
    CallBuilder::new(handler_address(name).unwrap_or_else(|| panic!("{name} is not registered")))
}

fn wait_until_last_thread_is_released(bridged: &Bridged) {
    let record = unsafe { *(bridged.symbol("fx_last_thread") as *const usize) };
    let started = Instant::now();
    while super::guest_thread::is_live_address(record) {
        assert!(started.elapsed().as_secs() < 10, "the detached thread never released its record");
        std::thread::yield_now();
    }
}

fn word(address: usize) -> u32 {
    unsafe { std::ptr::read_volatile(address as *const u32) }
}

#[test]
fn bionic_static_initializers_have_the_documented_byte_patterns() {
    // cite: bionic pthread.h#PTHREAD_MUTEX_INITIALIZER, PTHREAD_RECURSIVE_MUTEX_INITIALIZER_NP, PTHREAD_ERRORCHECK_MUTEX_INITIALIZER_NP, PTHREAD_COND_INITIALIZER_MONOTONIC_NP
    let bridged = threads();
    let object = |name: &str, bytes: usize| unsafe { std::slice::from_raw_parts(bridged.symbol(name) as *const u8, bytes).to_vec() };

    assert_eq!(object("fx_static_mutex", 40), vec![0u8; 40]);
    assert_eq!(word(bridged.symbol("fx_static_recursive")), 0x4000);
    assert_eq!(word(bridged.symbol("fx_static_errorcheck")), 0x8000);
    assert_eq!(object("fx_static_cond", 48), vec![0u8; 48]);
    assert_eq!(word(bridged.symbol("fx_static_cond_monotonic")), 2);
    assert_eq!(object("fx_static_rwlock", 56), vec![0u8; 56]);
}

#[test]
fn bionic_zero_initialized_static_mutex_locks_reports_busy_and_unlocks() {
    // cite: bionic pthread_mutex.cpp#pthread_mutex_lock, pthread_mutex_trylock, pthread_mutex_unlock
    let bridged = threads();

    assert_eq!(bridged.call("fx_static_mutex_zero_state").invoke().unwrap().i32(), 1600);
}

#[test]
fn bionic_mutex_contention_from_four_guest_threads_keeps_the_counter_exact_for_every_mutex_kind() {
    // cite: bionic pthread_mutex.cpp#NormalMutexLock, MutexLockWithTimeout, PIMutexTimedLock
    let bridged = threads();
    for (kind, protocol) in [(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)] {
        let total = bridged.call("fx_mutex_contention").i32(kind).i32(protocol).i32(4).i32(500).invoke().unwrap().u64() as i64;
        assert_eq!(total, 2000, "type {kind} protocol {protocol}");
    }
}

#[test]
fn bionic_zero_initialized_static_mutex_survives_contention_across_sixteen_threads() {
    // cite: bionic pthread_mutex.cpp#NormalMutexLock
    let bridged = threads();

    assert_eq!(bridged.call("fx_static_mutex_contention").i32(16).i32(200).invoke().unwrap().u64() as i64, 3200);
}

#[test]
fn bionic_recursive_mutex_allows_2048_locks_then_returns_eagain_and_refuses_extra_unlocks() {
    // cite: bionic pthread_mutex.cpp#RecursiveIncrement, pthread_mutex_unlock
    let bridged = threads();

    assert_eq!(bridged.call("fx_recursive_depth").i32(2048).invoke().unwrap().i32(), 1, "unlock of an unlocked recursive mutex is EPERM");
    assert_eq!(bridged.call("fx_recursive_overflow_code").invoke().unwrap().i32(), 11);
}

#[test]
fn bionic_errorcheck_mutex_reports_edeadlk_ebusy_and_eperm() {
    // cite: bionic pthread_mutex.cpp#MutexLockWithTimeout, pthread_mutex_trylock, pthread_mutex_unlock
    let bridged = threads();

    assert_eq!(bridged.call("fx_errorcheck_codes").invoke().unwrap().i32(), 35 * 1_000_000 + 16 * 1000 + 1);
}

#[test]
fn bionic_condvar_producer_consumer_delivers_every_item() {
    // cite: bionic pthread_cond.cpp#__pthread_cond_timedwait, __pthread_cond_pulse
    let bridged = threads();

    assert_eq!(bridged.call("fx_condvar_queue").i32(300).invoke().unwrap().u64(), 300 * 301 / 2);
}

#[test]
fn bionic_cond_broadcast_wakes_every_waiter() {
    // cite: bionic pthread_cond.cpp#pthread_cond_broadcast
    let bridged = threads();

    assert_eq!(bridged.call("fx_cond_broadcast_wakes_all").i32(8).invoke().unwrap().i32(), 8);
}

#[test]
fn bionic_cond_timedwait_times_out_on_both_clocks_and_returns_with_the_mutex_locked() {
    // cite: bionic pthread_cond.cpp#__pthread_cond_timedwait
    let bridged = threads();
    for monotonic in [0, 1] {
        let started = Instant::now();
        let code = bridged.call("fx_cond_timedwait").i32(60).i32(monotonic).invoke().unwrap().i32();

        assert_eq!(code, 110 * 1000 + 1, "monotonic {monotonic}");
        assert!(started.elapsed().as_millis() >= 50, "{:?}", started.elapsed());
    }
}

#[test]
fn bionic_key_destructor_runs_once_with_the_stored_value_at_thread_exit() {
    // cite: bionic pthread_key.cpp#pthread_key_clean_all
    let bridged = threads();

    assert_eq!(bridged.call("fx_key_destructor_runs").invoke().unwrap().u64(), 100_000 + 0x1234);
}

#[test]
fn bionic_key_destructors_that_rearm_their_key_run_for_four_rounds() {
    // cite: bionic pthread_key.cpp#pthread_key_clean_all
    // cite: bionic limits.h#PTHREAD_DESTRUCTOR_ITERATIONS
    let bridged = threads();

    assert_eq!(bridged.call("fx_key_destructor_rounds").invoke().unwrap().i32(), 4);
}

#[test]
fn bionic_first_guest_key_is_the_valid_flag_or_index_two_and_a_deleted_slot_is_reused_without_stale_data() {
    // cite: bionic pthread_key.cpp#pthread_key_create, pthread_getspecific
    let bridged = threads();
    let key = bridged.call("fx_key_create_first").invoke().unwrap().i32() as u32;

    assert!(key & 0x8000_0000 != 0 && (key & 0x7FFF_FFFF) >= 2, "{key:#x}");
    assert_eq!(bridged.call("fx_key_reuse_clears_old_values").invoke().unwrap().i32(), 1);
}

#[test]
fn bionic_invalid_keys_return_einval_and_null() {
    // cite: bionic pthread_key.cpp#KeyInValidRange
    let bridged = threads();

    assert_eq!(bridged.call("fx_key_invalid_codes").invoke().unwrap().i32(), 22 * 10000 + 22 * 100);
}

#[test]
fn bionic_pthread_specific_values_are_private_to_each_thread() {
    // cite: bionic pthread_key.cpp#pthread_setspecific, pthread_getspecific
    let bridged = threads();

    assert_eq!(bridged.call("fx_specific_per_thread").i32(8).invoke().unwrap().i32(), 8 + 100);
}

#[test]
fn bionic_thread_local_variable_has_a_private_copy_per_guest_thread() {
    // cite: bionic bionic_elf_tls.cpp#__tls_get_addr
    let bridged = threads();

    assert_eq!(bridged.call("fx_tls_variable_per_thread").i32(6).invoke().unwrap().i32(), 6 + 100);
}

#[test]
fn bionic_pthread_once_runs_the_routine_exactly_once_and_ends_in_state_two() {
    // cite: bionic pthread_once.cpp#pthread_once
    let bridged = threads();

    assert_eq!(bridged.call("fx_once_race").i32(8).invoke().unwrap().i32(), 11);
    assert_eq!(bridged.call("fx_once_state").invoke().unwrap().i32(), 2);
}

#[test]
fn bionic_thread_return_values_arrive_through_join_for_return_and_pthread_exit() {
    // cite: bionic pthread_join.cpp#pthread_join
    // cite: bionic pthread_exit.cpp#pthread_exit
    let bridged = threads();

    assert_eq!(bridged.call("fx_thread_exit_values").invoke().unwrap().u64(), 17025);
    assert_eq!(bridged.call("fx_thread_exit_from_nested_frames").invoke().unwrap().u64(), 91);
}

#[test]
fn bionic_join_reports_einval_for_a_detached_thread_and_edeadlk_for_self() {
    // cite: bionic pthread_join.cpp#pthread_join
    let bridged = threads();

    assert_eq!(bridged.call("fx_join_detached_code").invoke().unwrap().i32(), 22);
    wait_until_last_thread_is_released(&bridged);
    assert_eq!(bridged.call("fx_join_self_code").invoke().unwrap().i32(), 35);
}

#[test]
fn bionic_detached_threads_run_and_free_themselves_and_late_detach_of_an_exited_thread_succeeds() {
    // cite: bionic pthread_detach.cpp#pthread_detach
    // cite: bionic pthread_exit.cpp#pthread_exit
    let bridged = threads();

    assert_eq!(bridged.call("fx_detached_thread_runs").invoke().unwrap().i32(), 1);
    wait_until_last_thread_is_released(&bridged);
    assert_eq!(bridged.call("fx_detach_after_exit_code").invoke().unwrap().i32(), 0);
    wait_until_last_thread_is_released(&bridged);
    assert_eq!(bridged.call("fx_detach_running_thread_code").invoke().unwrap().i32(), 0);
    wait_until_last_thread_is_released(&bridged);
}

#[test]
fn bionic_pthread_create_writes_the_thread_before_the_start_routine_runs() {
    // cite: bionic pthread_create.cpp#pthread_create, __pthread_start
    let bridged = threads();
    for _ in 0..20 {
        assert_eq!(bridged.call("fx_thread_out_is_written_before_the_routine_runs").invoke().unwrap().i32(), 1);
    }
}

#[test]
fn bionic_thread_attribute_stack_size_is_what_the_thread_sees_through_getattr_np() {
    // cite: bionic pthread_attr.cpp#pthread_getattr_np, pthread_attr_setstacksize
    let bridged = threads();

    assert_eq!(bridged.call("fx_attr_stack_size_seen_by_thread").int(262_144).invoke().unwrap().u64(), 262_144);
}

#[test]
fn bionic_pthread_exit_runs_thread_local_destructors_then_cleanup_handlers_then_key_destructors() {
    // cite: bionic pthread_exit.cpp#pthread_exit
    let bridged = threads();

    assert_eq!(bridged.call("fx_exit_cleanup_order").invoke().unwrap().i32(), 123);
    assert_eq!(bridged.call("fx_cleanup_pop_execute_flags").invoke().unwrap().i32(), 2);
}

#[test]
fn bionic_semaphores_pass_tokens_between_threads_and_report_the_documented_errors() {
    // cite: bionic semaphore.cpp#sem_wait, __sem_timedwait, sem_trywait, sem_init, sem_post, sem_clockwait
    let bridged = threads();

    assert_eq!(bridged.call("fx_sem_ping_pong").i32(500).invoke().unwrap().i32(), 500);
    assert_eq!(bridged.call("fx_sem_timedwait_errno").invoke().unwrap().i32(), 110);
    assert_eq!(bridged.call("fx_sem_trywait_errno").invoke().unwrap().i32(), 11);
    assert_eq!(bridged.call("fx_sem_init_over_max_errno").invoke().unwrap().i32(), 22);
    assert_eq!(bridged.call("fx_sem_post_at_max_errno").invoke().unwrap().i32(), 75);
    assert_eq!(bridged.call("fx_sem_clockwait_bad_clock").invoke().unwrap().i32(), 22 * 1000, "the quirk returns EINVAL as the value with errno untouched");
}

#[test]
fn bionic_barrier_has_one_serial_thread_per_cycle_and_rejects_a_zero_count() {
    // cite: bionic pthread_barrier.cpp#pthread_barrier_wait, pthread_barrier_init
    let bridged = threads();

    assert_eq!(bridged.call("fx_barrier_serial_count").i32(4).i32(3).invoke().unwrap().i32(), 30);
    assert_eq!(bridged.call("fx_barrier_init_zero_code").invoke().unwrap().i32(), 22);
}

#[test]
fn bionic_spinlock_reports_ebusy_for_a_second_trylock() {
    // cite: bionic pthread_spinlock.cpp#pthread_spin_trylock
    let bridged = threads();

    assert_eq!(bridged.call("fx_spin_codes").invoke().unwrap().i32(), 160);
}

#[test]
fn bionic_rwlock_readers_share_writers_exclude_and_unlock_checks_ownership() {
    // cite: bionic pthread_rwlock.cpp#__pthread_rwlock_tryrdlock, __pthread_rwlock_trywrlock, pthread_rwlock_unlock
    let bridged = threads();

    assert_eq!(bridged.call("fx_rwlock_exclusion_codes").invoke().unwrap().i32(), 10);
    assert_eq!(bridged.call("fx_rwlock_timedwrlock_code").invoke().unwrap().i32(), 110);
    assert_eq!(bridged.call("fx_rwlock_reader_overflow_code").invoke().unwrap().i32(), 11);
}

#[test]
fn bionic_rwlock_prefer_writer_kind_refuses_new_readers_while_a_writer_waits() {
    // cite: bionic pthread_rwlock.cpp#__can_acquire_read_lock
    let bridged = threads();

    assert_eq!(bridged.call("fx_rwlock_pending_writer_blocks_new_readers").invoke().unwrap().i32(), 16 * 100);
}

#[test]
fn syscall_futex_wait_wake_mismatch_and_timeout_follow_the_kernel_contract() {
    // cite: bionic futex.h#FUTEX_WAIT_BITSET, FUTEX_WAKE
    let bridged = threads();

    assert_eq!(bridged.call("fx_futex_wait_then_wake").invoke().unwrap().i32() % 10, 1);
    assert_eq!(bridged.call("fx_futex_wait_mismatch_errno").invoke().unwrap().i32(), 11);
    let started = Instant::now();
    assert_eq!(bridged.call("fx_futex_wait_timeout_errno").i32(40).invoke().unwrap().i32(), 110);
    assert!(started.elapsed().as_millis() >= 35);
}

#[test]
fn syscall_gettid_matches_gettid() {
    // cite: bionic unistd.h#__NR_gettid
    let bridged = threads();
    let packed = bridged.call("fx_gettid_via_syscall").invoke().unwrap().u64();

    assert_eq!(packed / 1_000_000, packed % 1_000_000);
    assert!(packed % 1_000_000 > 0);
}

#[test]
fn bionic_thread_name_set_and_get_round_trip() {
    // cite: bionic pthread_setname_np.cpp#pthread_setname_np, pthread_getname_np
    let bridged = threads();

    assert_eq!(bridged.call("fx_thread_name_round_trip").invoke().unwrap().i32(), 1);
}

#[test]
fn bionic_sixteen_concurrent_guest_threads_keep_a_contended_counter_exact() {
    // cite: bionic pthread_create.cpp#pthread_create
    let bridged = threads();

    assert_eq!(bridged.call("fx_mutex_contention").i32(0).i32(0).i32(16).i32(100).invoke().unwrap().u64() as i64, 1600);
}

#[test]
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

#[test]
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

#[test]
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

#[test]
fn bionic_condattr_set_clock_keeps_the_monotonic_bit_when_realtime_is_set_afterwards() {
    // cite: bionic pthread_cond.cpp#pthread_condattr_setclock
    let attribute = Box::leak(Box::new(0i64)) as *mut i64 as usize;
    let cond = Box::leak(Box::new([0xFFu8; 48])).as_mut_ptr() as usize;
    shim("pthread_condattr_init").ptr(attribute).invoke().unwrap();
    shim("pthread_condattr_setclock").ptr(attribute).i32(1).invoke().unwrap();
    shim("pthread_condattr_setclock").ptr(attribute).i32(0).invoke().unwrap();

    assert_eq!(unsafe { *(attribute as *const i64) }, 2);
    assert_eq!(shim("pthread_condattr_setclock").ptr(attribute).i32(5).invoke().unwrap().i32(), 22);
    shim("pthread_cond_init").ptr(cond).ptr(attribute).invoke().unwrap();
    assert_eq!(word(cond), 2);
    assert_eq!(word(cond + 4), 0);
}

#[test]
fn bionic_pthread_attr_defaults_and_validation_follow_pthread_attr_cpp() {
    // cite: bionic pthread_attr.cpp#pthread_attr_init, pthread_attr_setstacksize, pthread_attr_setdetachstate, pthread_attr_setscope
    let attr = Box::leak(Box::new([0xEEu8; 56])).as_mut_ptr() as usize;
    shim("pthread_attr_init").ptr(attr).invoke().unwrap();
    let read = |offset: usize| unsafe { *((attr + offset) as *const u64) };

    assert_eq!((word(attr), read(8), read(16), read(24)), (0, 0, 1_015_808, 4096));
    assert_eq!(shim("pthread_attr_setstacksize").ptr(attr).int(16383).invoke().unwrap().i32(), 22);
    assert_eq!(shim("pthread_attr_setdetachstate").ptr(attr).i32(7).invoke().unwrap().i32(), 22);
    assert_eq!(shim("pthread_attr_setdetachstate").ptr(attr).i32(1).invoke().unwrap().i32(), 0);
    assert_eq!(word(attr), 1);
    assert_eq!(shim("pthread_attr_setscope").ptr(attr).i32(1).invoke().unwrap().i32(), 95);
}

#[test]
fn bionic_sem_value_encoding_keeps_the_shared_bit_and_the_value_shifted_left_by_one() {
    // cite: bionic semaphore.cpp#SEMCOUNT_FROM_VALUE, sem_init, sem_getvalue
    let semaphore = Box::leak(Box::new([0u8; 16])).as_mut_ptr() as usize;
    let value = Box::leak(Box::new(0i32)) as *mut i32 as usize;
    shim("sem_init").ptr(semaphore).i32(1).i32(5).invoke().unwrap();

    assert_eq!(word(semaphore), (5 << 1) | 1);
    shim("sem_getvalue").ptr(semaphore).ptr(value).invoke().unwrap();
    assert_eq!(unsafe { *(value as *const i32) }, 5);
    assert_eq!(shim("sem_trywait").ptr(semaphore).invoke().unwrap().i32(), 0);
    assert_eq!(word(semaphore), (4 << 1) | 1);
}

#[test]
fn bionic_thread_list_rejects_addresses_that_are_not_live_records() {
    // cite: bionic pthread_internal.cpp#__pthread_internal_find
    super::bridge_test_support::register_all();
    let mine = shim("pthread_self").invoke().unwrap().u64() as usize;

    assert_eq!(super::thread_list::lookup(mine), Some(mine));
    assert_eq!(super::thread_list::lookup(mine + 16), None);
    assert_eq!(super::thread_list::lookup(0), None);
}
