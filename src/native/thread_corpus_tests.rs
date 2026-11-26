use super::bridge_test_support::Bridged;
use super::test_deadline::bounded_test;

fn libcxx() -> Option<Bridged> {
    use std::sync::Arc;
    let dir = super::loader_test_support::corpus_dir();
    if !dir.join("libc++_shared.so").exists() {
        eprintln!(
            "skipped: TikTok library extraction not found at {}",
            dir.display()
        );
        return None;
    }
    super::bridge_test_support::register_all();
    let loader = Arc::new(
        super::loader::Loader::new(vec![dir], Box::new(super::bridge::BridgeProvider))
            .with_strict(false),
    );
    let handle = loader
        .dlopen(Some("libc++_shared.so"), 2, None)
        .unwrap_or_else(|error| panic!("libc++_shared.so: {error}"));
    Some(Bridged { loader, handle })
}

bounded_test! {
    fn measured_libcxx_shared_mutex_lock_try_lock_and_unlock_work_on_a_zeroed_bionic_mutex() {
        // cite: bionic pthread_mutex.cpp#NormalMutexTryLock, pthread_mutex_unlock
        let Some(libcxx) = libcxx() else { return };
        let storage = Box::leak(Box::new([0u64; 8])).as_mut_ptr() as usize;
        let call = |name: &str| libcxx.call(name).ptr(storage).invoke().unwrap().i32();

        call("_ZNSt6__ndk15mutex4lockEv");
        assert_eq!(call("_ZNSt6__ndk15mutex8try_lockEv") & 0xFF, 0, "try_lock of a held mutex is false");
        call("_ZNSt6__ndk15mutex6unlockEv");
        assert_eq!(call("_ZNSt6__ndk15mutex8try_lockEv") & 0xFF, 1);
        assert_eq!(unsafe { *(storage as *const u16) }, 1, "locked without waiters in the Bionic state word");
    }
}

bounded_test! {
    fn measured_libcxx_shared_recursive_mutex_locks_twice_through_a_bionic_recursive_mutex() {
        // cite: bionic pthread_mutex.cpp#RecursiveIncrement, MutexLockWithTimeout
        let Some(libcxx) = libcxx() else { return };
        let storage = Box::leak(Box::new([0u64; 8])).as_mut_ptr() as usize;
        libcxx.call("_ZNSt6__ndk115recursive_mutexC1Ev").ptr(storage).invoke().unwrap();

        assert_eq!(unsafe { *(storage as *const u16) }, 0x4000);
        libcxx.call("_ZNSt6__ndk115recursive_mutex4lockEv").ptr(storage).invoke().unwrap();
        libcxx.call("_ZNSt6__ndk115recursive_mutex4lockEv").ptr(storage).invoke().unwrap();
        assert_eq!(unsafe { *(storage as *const u16) } & 0x1FFC, 4, "second lock raises the recursion counter");
        libcxx.call("_ZNSt6__ndk115recursive_mutex6unlockEv").ptr(storage).invoke().unwrap();
        libcxx.call("_ZNSt6__ndk115recursive_mutex6unlockEv").ptr(storage).invoke().unwrap();
        assert_eq!(unsafe { *(storage as *const u16) }, 0x4000);
    }
}

bounded_test! {
    fn measured_libkeva_loads_and_runs_its_constructors_with_the_real_pthread_shim() {
        // cite: bionic linker_soinfo.cpp#call_constructors
        let dir = super::loader_test_support::corpus_dir();
        if !dir.join("libkeva.so").exists() {
            eprintln!("skipped: TikTok library extraction not found at {}", dir.display());
            return;
        }
        super::bridge_test_support::register_all();
        let loader = super::loader::Loader::new(vec![dir], Box::new(super::bridge::BridgeProvider)).with_strict(true);

        let handle = loader.dlopen(Some("libkeva.so"), 2, None);

        assert!(handle.is_ok(), "{handle:?}");
    }
}

bounded_test! {
    fn every_pthread_semaphore_and_signal_export_of_the_milestone_resolves_to_a_handler_in_libc() {
        // cite: bionic libc.map.txt#pthread_create, __register_atfork, __tls_get_addr, sem_clockwait
        use super::link_resolution::SystemLibraryProvider;
        super::bridge_test_support::register_all();
        let names = "pthread_attr_destroy pthread_attr_getdetachstate pthread_attr_getguardsize pthread_attr_getinheritsched pthread_attr_getschedparam \
            pthread_attr_getschedpolicy pthread_attr_getscope pthread_attr_getstack pthread_attr_getstacksize pthread_attr_init pthread_attr_setdetachstate \
            pthread_attr_setguardsize pthread_attr_setinheritsched pthread_attr_setschedparam pthread_attr_setschedpolicy pthread_attr_setscope \
            pthread_attr_setstack pthread_attr_setstacksize pthread_getattr_np pthread_mutexattr_destroy pthread_mutexattr_getpshared \
            pthread_mutexattr_gettype pthread_mutexattr_getprotocol pthread_mutexattr_init pthread_mutexattr_setpshared pthread_mutexattr_settype \
            pthread_mutexattr_setprotocol pthread_mutex_clocklock pthread_mutex_destroy pthread_mutex_init pthread_mutex_lock pthread_mutex_timedlock \
            pthread_mutex_timedlock_monotonic_np pthread_mutex_trylock pthread_mutex_unlock pthread_condattr_destroy pthread_condattr_getclock \
            pthread_condattr_getpshared pthread_condattr_init pthread_condattr_setclock pthread_condattr_setpshared pthread_cond_broadcast \
            pthread_cond_clockwait pthread_cond_destroy pthread_cond_init pthread_cond_signal pthread_cond_timedwait pthread_cond_timedwait_monotonic_np \
            pthread_cond_wait pthread_rwlockattr_init pthread_rwlockattr_destroy pthread_rwlockattr_getpshared pthread_rwlockattr_setpshared \
            pthread_rwlockattr_getkind_np pthread_rwlockattr_setkind_np pthread_rwlock_clockrdlock pthread_rwlock_clockwrlock pthread_rwlock_destroy \
            pthread_rwlock_init pthread_rwlock_rdlock pthread_rwlock_timedrdlock pthread_rwlock_timedrdlock_monotonic_np pthread_rwlock_timedwrlock \
            pthread_rwlock_timedwrlock_monotonic_np pthread_rwlock_tryrdlock pthread_rwlock_trywrlock pthread_rwlock_unlock pthread_rwlock_wrlock \
            pthread_barrierattr_destroy pthread_barrierattr_getpshared pthread_barrierattr_init pthread_barrierattr_setpshared pthread_barrier_destroy \
            pthread_barrier_init pthread_barrier_wait pthread_spin_destroy pthread_spin_init pthread_spin_lock pthread_spin_trylock pthread_spin_unlock \
            pthread_key_create pthread_key_delete pthread_getspecific pthread_setspecific pthread_once pthread_create pthread_detach pthread_equal \
            pthread_exit pthread_getcpuclockid pthread_getname_np pthread_getschedparam pthread_gettid_np pthread_join pthread_kill pthread_self \
            pthread_setname_np pthread_setschedparam pthread_setschedprio pthread_sigmask pthread_sigmask64 pthread_atfork __register_atfork \
            __pthread_cleanup_push __pthread_cleanup_pop __cxa_thread_atexit_impl sem_clockwait sem_destroy sem_getvalue sem_init sem_post sem_timedwait \
            sem_timedwait_monotonic_np sem_trywait sem_wait sem_open sem_close sem_unlink sched_yield sched_get_priority_max sched_get_priority_min \
            __tls_get_addr gettid __errno syscall sigaction sigaction64 signal sigprocmask sigprocmask64 sigpending sigaltstack sigemptyset sigfillset \
            sigaddset sigdelset sigismember sigwait sigsuspend sigsuspend64 prctl raise kill killpg tgkill __libc_current_sigrtmin __libc_current_sigrtmax";

        let missing: Vec<_> = names.split_whitespace().filter(|name| super::bridge::BridgeProvider.export("libc.so", name).is_none()).collect();

        assert!(missing.is_empty(), "{missing:?}");
    }
}
