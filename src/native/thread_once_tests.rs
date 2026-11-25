use super::test_deadline::bounded_test;
use super::thread_test_support::{each_variant};

bounded_test! {
    fn bionic_pthread_once_runs_the_routine_exactly_once_and_ends_in_state_two() {
        // cite: bionic pthread_once.cpp#pthread_once
        each_variant(|bridged| {
            assert_eq!(bridged.call("fx_once_race").i32(8).invoke().unwrap().i32(), 11);
            assert_eq!(bridged.call("fx_once_state").invoke().unwrap().i32(), 2);
        });
    }
}

bounded_test! {
    fn bionic_pthread_once_makes_every_caller_wait_until_the_initializer_has_finished() {
        // cite: bionic pthread_once.cpp#pthread_once
        each_variant(|bridged| {
            assert_eq!(bridged.call("fx_once_waiters_see_the_finished_initializer").i32(6).invoke().unwrap().i32(), 61);
        });
    }
}
