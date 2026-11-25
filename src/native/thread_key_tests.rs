use super::loader_test_support::child_result;
use super::test_deadline::bounded_test;
use super::thread_test_support::{each_variant, shim};

bounded_test! {
    fn bionic_key_destructor_runs_once_with_the_stored_value_at_thread_exit() {
        // cite: bionic pthread_key.cpp#pthread_key_clean_all
        each_variant(|bridged| {
            assert_eq!(bridged.call("fx_key_destructor_runs").invoke().unwrap().u64(), 100_000 + 0x1234);
        });
    }
}

bounded_test! {
    fn bionic_key_destructors_that_rearm_their_key_run_for_four_rounds() {
        // cite: bionic pthread_key.cpp#pthread_key_clean_all
        // cite: bionic limits.h#PTHREAD_DESTRUCTOR_ITERATIONS
        each_variant(|bridged| {
            assert_eq!(bridged.call("fx_key_destructor_rounds").invoke().unwrap().i32(), 4);
        });
    }
}

bounded_test! {
    fn bionic_first_guest_key_is_the_valid_flag_or_index_two_and_a_deleted_slot_is_reused_without_stale_data() {
        // cite: bionic pthread_key.cpp#pthread_key_create, pthread_getspecific
        each_variant(|bridged| {
            let key = bridged.call("fx_key_create_first").invoke().unwrap().i32() as u32;

            assert!(key & 0x8000_0000 != 0 && (key & 0x7FFF_FFFF) >= 2, "{key:#x}");
            assert_eq!(bridged.call("fx_key_reuse_clears_old_values").invoke().unwrap().i32(), 1);
        });
    }
}

bounded_test! {
    fn bionic_invalid_keys_return_einval_and_null() {
        // cite: bionic pthread_key.cpp#KeyInValidRange
        each_variant(|bridged| {
            assert_eq!(bridged.call("fx_key_invalid_codes").invoke().unwrap().i32(), 22 * 10000 + 22 * 100);
        });
    }
}

bounded_test! {
    fn bionic_pthread_specific_values_are_private_to_each_thread() {
        // cite: bionic pthread_key.cpp#pthread_setspecific, pthread_getspecific
        each_variant(|bridged| {
            assert_eq!(bridged.call("fx_specific_per_thread").i32(8).invoke().unwrap().i32(), 8 + 100);
        });
    }
}

bounded_test! {
    fn bionic_key_create_reports_eagain_when_every_key_is_taken_and_reuses_a_deleted_key() {
        // cite: bionic pthread_key.cpp#pthread_key_create, pthread_key_delete
        super::bridge_test_support::register_all();
        let (signal, code, output) = child_result(|| {
            let slot = Box::leak(Box::new(0u32)) as *mut u32 as usize;
            let mut created = Vec::new();
            let refusal = loop {
                let status = shim("pthread_key_create").ptr(slot).ptr(0).invoke().unwrap().i32();
                if status != 0 {
                    break status;
                }
                created.push(unsafe { *(slot as *const u32) });
                assert!(created.len() <= 130, "more keys than the table holds");
            };
            assert_eq!(refusal, 11);
            assert!(!created.is_empty(), "no key was free in the forked copy");
            assert_eq!(shim("pthread_key_delete").u32(created[0]).invoke().unwrap().i32(), 0);
            assert_eq!(shim("pthread_key_create").ptr(slot).ptr(0).invoke().unwrap().i32(), 0);
            assert_eq!(unsafe { *(slot as *const u32) }, created[0]);
            assert_eq!(shim("pthread_key_create").ptr(slot).ptr(0).invoke().unwrap().i32(), 11);
        });

        assert_eq!((signal, code), (None, Some(0)), "{output}");
    }
}

static EXIT_DESTRUCTOR_VALUES: std::sync::Mutex<Vec<usize>> = std::sync::Mutex::new(Vec::new());

extern "C" fn record_destructor_value(value: usize) {
    EXIT_DESTRUCTOR_VALUES.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).push(value);
}

bounded_test! {
    fn a_host_created_thread_runs_the_guest_key_destructors_when_it_exits() {
        // cite: bionic pthread_key.cpp#pthread_key_clean_all
        let key = Box::leak(Box::new(0u32)) as *mut u32 as usize;
        assert_eq!(shim("pthread_key_create").ptr(key).ptr(record_destructor_value as *const () as usize).invoke().unwrap().i32(), 0);
        let key_value = unsafe { *(key as *const u32) };

        std::thread::spawn(move || {
            assert_eq!(shim("pthread_setspecific").u32(key_value).ptr(0xBEEF).invoke().unwrap().i32(), 0);
        })
        .join()
        .unwrap();

        assert_eq!(shim("pthread_key_delete").u32(key_value).invoke().unwrap().i32(), 0);
        assert!(EXIT_DESTRUCTOR_VALUES.lock().unwrap().contains(&0xBEEF));
    }
}
