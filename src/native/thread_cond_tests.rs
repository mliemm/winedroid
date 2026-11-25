use std::time::Instant;

use super::test_deadline::bounded_test;
use super::thread_test_support::{each_variant, shim, word};

bounded_test! {
    fn bionic_condvar_producer_consumer_delivers_every_item() {
        // cite: bionic pthread_cond.cpp#__pthread_cond_timedwait, __pthread_cond_pulse
        each_variant(|bridged| {
            assert_eq!(bridged.call("fx_condvar_queue").i32(300).invoke().unwrap().u64(), 300 * 301 / 2);
        });
    }
}

bounded_test! {
    fn bionic_cond_broadcast_wakes_every_waiter() {
        // cite: bionic pthread_cond.cpp#pthread_cond_broadcast
        each_variant(|bridged| {
            assert_eq!(bridged.call("fx_cond_broadcast_wakes_all").i32(8).invoke().unwrap().i32(), 8);
        });
    }
}

bounded_test! {
    fn bionic_cond_timedwait_times_out_on_both_clocks_and_returns_with_the_mutex_locked() {
        // cite: bionic pthread_cond.cpp#__pthread_cond_timedwait
        each_variant(|bridged| {
            for monotonic in [0, 1] {
                let started = Instant::now();
                let code = bridged.call("fx_cond_timedwait").i32(60).i32(monotonic).invoke().unwrap().i32();

                assert_eq!(code, 110 * 1000 + 1, "monotonic {monotonic}");
                assert!(started.elapsed().as_millis() >= 50, "{:?}", started.elapsed());
            }
        });
    }
}

bounded_test! {
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
}
