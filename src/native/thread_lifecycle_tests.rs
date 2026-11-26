use super::call_assign::CallBuilder;
use super::loader_test_support::child_result;
use super::test_deadline::bounded_test;
use super::thread_test_support::{
    each_variant, shim, threads_in, wait_until_last_thread_is_released, word,
};

bounded_test! {
    fn bionic_thread_return_values_arrive_through_join_for_return_and_pthread_exit() {
        // cite: bionic pthread_join.cpp#pthread_join
        // cite: bionic pthread_exit.cpp#pthread_exit
        each_variant(|bridged| {
            assert_eq!(bridged.call("fx_thread_exit_values").invoke().unwrap().u64(), 17025);
            assert_eq!(bridged.call("fx_thread_exit_from_nested_frames").invoke().unwrap().u64(), 91);
        });
    }
}

bounded_test! {
    fn bionic_join_reports_einval_for_a_detached_thread_and_edeadlk_for_self() {
        // cite: bionic pthread_join.cpp#pthread_join
        each_variant(|bridged| {
            assert_eq!(bridged.call("fx_join_detached_code").invoke().unwrap().i32(), 22);
            wait_until_last_thread_is_released(bridged);
            assert_eq!(bridged.call("fx_join_self_code").invoke().unwrap().i32(), 35);
        });
    }
}

bounded_test! {
    fn bionic_detached_threads_run_and_free_themselves_and_late_detach_of_an_exited_thread_succeeds() {
        // cite: bionic pthread_detach.cpp#pthread_detach
        // cite: bionic pthread_exit.cpp#pthread_exit
        each_variant(|bridged| {
            assert_eq!(bridged.call("fx_detached_thread_runs").invoke().unwrap().i32(), 1);
            wait_until_last_thread_is_released(bridged);
            assert_eq!(bridged.call("fx_detach_after_exit_code").invoke().unwrap().i32(), 0);
            wait_until_last_thread_is_released(bridged);
            assert_eq!(bridged.call("fx_detach_running_thread_code").invoke().unwrap().i32(), 0);
            wait_until_last_thread_is_released(bridged);
        });
    }
}

bounded_test! {
    fn bionic_pthread_create_writes_the_thread_before_the_start_routine_runs() {
        // cite: bionic pthread_create.cpp#pthread_create, __pthread_start
        each_variant(|bridged| {
            for _ in 0..20 {
                assert_eq!(bridged.call("fx_thread_out_is_written_before_the_routine_runs").invoke().unwrap().i32(), 1);
            }
        });
    }
}

bounded_test! {
    fn bionic_thread_attribute_stack_size_is_what_the_thread_sees_through_getattr_np() {
        // cite: bionic pthread_attr.cpp#pthread_getattr_np, pthread_attr_setstacksize
        each_variant(|bridged| {
            assert_eq!(bridged.call("fx_attr_stack_size_seen_by_thread").int(262_144).invoke().unwrap().u64(), 262_144);
        });
    }
}

bounded_test! {
    fn bionic_pthread_exit_runs_thread_local_destructors_then_cleanup_handlers_then_key_destructors() {
        // cite: bionic pthread_exit.cpp#pthread_exit
        each_variant(|bridged| {
            assert_eq!(bridged.call("fx_exit_cleanup_order").invoke().unwrap().i32(), 123);
            assert_eq!(bridged.call("fx_cleanup_pop_execute_flags").invoke().unwrap().i32(), 2);
        });
    }
}

bounded_test! {
    fn bionic_thread_name_set_and_get_round_trip() {
        // cite: bionic pthread_setname_np.cpp#pthread_setname_np, pthread_getname_np
        each_variant(|bridged| {
            assert_eq!(bridged.call("fx_thread_name_round_trip").invoke().unwrap().i32(), 1);
        });
    }
}

bounded_test! {
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
}

bounded_test! {
    fn bionic_thread_list_rejects_addresses_that_are_not_live_records() {
        // cite: bionic pthread_internal.cpp#__pthread_internal_find
        super::bridge_test_support::register_all();
        let mine = shim("pthread_self").invoke().unwrap().u64() as usize;

        assert_eq!(super::thread_list::lookup(mine), Some(mine));
        assert_eq!(super::thread_list::lookup(mine + 16), None);
        assert_eq!(super::thread_list::lookup(0), None);
    }
}

extern "C" fn report_host_stack_size(_argument: usize) -> usize {
    unsafe { libc::pthread_get_stacksize_np(libc::pthread_self()) }
}

fn host_stack_size_of_a_guest_thread(attribute: usize) -> usize {
    let (thread, result) = (
        Box::leak(Box::new(0u64)) as *mut u64 as usize,
        Box::leak(Box::new(0usize)) as *mut usize as usize,
    );
    assert_eq!(
        shim("pthread_create")
            .ptr(thread)
            .ptr(attribute)
            .ptr(report_host_stack_size as *const () as usize)
            .ptr(0)
            .invoke()
            .unwrap()
            .i32(),
        0
    );
    assert_eq!(
        shim("pthread_join")
            .ptr(unsafe { *(thread as *const u64) } as usize)
            .ptr(result)
            .invoke()
            .unwrap()
            .i32(),
        0
    );
    unsafe { *(result as *const usize) }
}

bounded_test! {
    fn the_host_thread_behind_a_guest_thread_gets_the_default_or_requested_stack_and_never_the_512_kib_host_default() {
        // cite: bionic pthread_internal.h#PTHREAD_STACK_SIZE_DEFAULT
        // cite: bionic pthread_attr.cpp#pthread_attr_setstacksize
        let attribute = Box::leak(Box::new([0u8; 56])).as_mut_ptr() as usize;
        shim("pthread_attr_init").ptr(attribute).invoke().unwrap();
        shim("pthread_attr_setstacksize").ptr(attribute).int(2 << 20).invoke().unwrap();

        assert!(host_stack_size_of_a_guest_thread(0) >= 1 << 20, "default attribute");
        assert!(host_stack_size_of_a_guest_thread(attribute) >= 2 << 20, "explicit 2 MiB attribute");
    }
}

bounded_test! {
    fn a_guest_thread_can_touch_more_stack_than_the_512_kib_host_default_with_default_and_explicit_sizes() {
        // cite: bionic pthread_internal.h#PTHREAD_STACK_SIZE_DEFAULT
        // cite: bionic pthread_attr.cpp#pthread_attr_setstacksize
        super::fork_gate::install();
        let bridged = threads_in("16k");
        let touch = bridged.symbol("fx_thread_touches_stack");
        for (bytes, stack) in [(768u64 * 1024, 0u64), (2560 * 1024, 3 << 20)] {
            let (signal, code, output) = child_result(|| {
                assert_eq!(CallBuilder::new(touch).int(bytes).int(stack).invoke().unwrap().i32(), 1);
            });

            assert_eq!((signal, code), (None, Some(0)), "touching {bytes} bytes with stack size {stack}: {output}");
        }
    }
}
