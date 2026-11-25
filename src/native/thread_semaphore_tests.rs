use super::test_deadline::bounded_test;
use super::thread_test_support::{each_variant, shim, word};

bounded_test! {
    fn bionic_semaphores_pass_tokens_between_threads_and_report_the_documented_errors() {
        // cite: bionic semaphore.cpp#sem_wait, __sem_timedwait, sem_trywait, sem_init, sem_post, sem_clockwait
        each_variant(|bridged| {
            assert_eq!(bridged.call("fx_sem_ping_pong").i32(500).invoke().unwrap().i32(), 500);
            assert_eq!(bridged.call("fx_sem_timedwait_errno").invoke().unwrap().i32(), 110);
            assert_eq!(bridged.call("fx_sem_trywait_errno").invoke().unwrap().i32(), 11);
            assert_eq!(bridged.call("fx_sem_init_over_max_errno").invoke().unwrap().i32(), 22);
            assert_eq!(bridged.call("fx_sem_post_at_max_errno").invoke().unwrap().i32(), 75);
            assert_eq!(bridged.call("fx_sem_clockwait_bad_clock").invoke().unwrap().i32(), 22 * 1000, "the quirk returns EINVAL as the value with errno untouched");
        });
    }
}

bounded_test! {
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
}
