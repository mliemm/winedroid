use super::test_deadline::bounded_test;
use super::thread_test_support::{each_variant, shim};

bounded_test! {
    fn bionic_barrier_has_one_serial_thread_per_cycle_and_rejects_a_zero_count() {
        // cite: bionic pthread_barrier.cpp#pthread_barrier_wait, pthread_barrier_init
        each_variant(|bridged| {
            assert_eq!(bridged.call("fx_barrier_serial_count").i32(4).i32(3).invoke().unwrap().i32(), 30);
            assert_eq!(bridged.call("fx_barrier_init_zero_code").invoke().unwrap().i32(), 22);
        });
    }
}

bounded_test! {
    fn bionic_barrierattr_destroy_clears_the_attribute_word() {
        // cite: bionic pthread_barrier.cpp#pthread_barrierattr_destroy, pthread_barrierattr_setpshared
        let attribute = Box::leak(Box::new(0i32)) as *mut i32 as usize;
        shim("pthread_barrierattr_init").ptr(attribute).invoke().unwrap();
        shim("pthread_barrierattr_setpshared").ptr(attribute).i32(1).invoke().unwrap();
        assert_eq!(unsafe { *(attribute as *const i32) }, 1);

        assert_eq!(shim("pthread_barrierattr_destroy").ptr(attribute).invoke().unwrap().i32(), 0);

        assert_eq!(unsafe { *(attribute as *const i32) }, 0);
    }
}
