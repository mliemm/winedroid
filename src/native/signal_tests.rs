use std::sync::{Mutex, MutexGuard};

use super::bridge::handler_address;
use super::bridge_test_support::{register_all, Bridged};
use super::call_assign::CallBuilder;
use super::test_deadline::bounded_test;

static SERIAL: Mutex<()> = Mutex::new(());

struct Session {
    bridged: Bridged,
    _serial: MutexGuard<'static, ()>,
    installed: std::cell::RefCell<Vec<i32>>,
}

impl Session {
    fn new() -> Self {
        Self::in_variant("16k")
    }

    fn in_variant(variant: &str) -> Self {
        let serial = SERIAL.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        Self { bridged: Bridged::load(variant, "libfx_signals.so"), _serial: serial, installed: Default::default() }
    }

    fn call(&self, name: &str) -> CallBuilder {
        self.bridged.call(name)
    }

    fn int(&self, name: &str) -> i32 {
        self.call(name).invoke().unwrap().i32()
    }

    fn recorder(&self, signal: i32, flags: u32) {
        self.installed.borrow_mut().push(signal);
        assert_eq!(self.call("fx_install_recorder").i32(signal).u32(flags).invoke().unwrap().i32(), 0);
    }

    fn skipper(&self, signal: i32) {
        self.installed.borrow_mut().push(signal);
        assert_eq!(self.call("fx_install_skipper").i32(signal).invoke().unwrap().i32(), 0);
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        for signal in self.installed.borrow().iter() {
            let _ = self.call("fx_uninstall").i32(*signal).invoke();
        }
    }
}

fn each_session(check: impl Fn(&Session)) {
    for variant in ["16k", "4k"] {
        eprintln!("fixture variant {variant}");
        check(&Session::in_variant(variant));
    }
}

fn shim(name: &str) -> CallBuilder {
    register_all();
    CallBuilder::new(handler_address(name).unwrap_or_else(|| panic!("{name} is not registered")))
}

fn host_cell<T>(value: T) -> usize {
    Box::leak(Box::new(value)) as *mut T as usize
}

fn read<T: Copy>(address: usize) -> T {
    unsafe { std::ptr::read_unaligned(address as *const T) }
}

bounded_test! {
    fn bionic_raise_runs_the_guest_handler_with_tkill_siginfo_and_the_signal_blocked_inside() {
        // cite: bionic raise.cpp#raise
        each_session(|session| {
            session.recorder(10, 0);

            assert_eq!(session.call("fx_raise").i32(10).invoke().unwrap().i32(), 0);

            assert_eq!(session.int("fx_seen_signal"), 10);
            assert_eq!(session.int("fx_seen_code"), -6, "SI_TKILL");
            assert_eq!(session.int("fx_seen_pid"), session.int("fx_getpid"));
            assert_eq!(session.int("fx_seen_blocked_inside"), 1);
            assert_eq!(session.call("fx_blocked_after_handler").i32(10).invoke().unwrap().i32(), 0);
            assert_eq!(session.int("fx_seen_count"), 1);
        });
    }
}

bounded_test! {
    fn bionic_blocked_signal_stays_pending_until_unblocked_then_runs_once() {
        // cite: bionic sigprocmask.cpp#sigprocmask64
        each_session(|session| {
            session.recorder(10, 0);

            assert_eq!(session.call("fx_block_raise_unblock").i32(10).invoke().unwrap().i32(), 1);
        });
    }
}

bounded_test! {
    fn bionic_hardware_segv_reaches_the_guest_handler_with_a_maperr_code_and_the_handler_can_edit_registers() {
        // cite: bionic siginfo.h#SEGV_MAPERR
        each_session(|session| {
            session.skipper(11);

            let resumed = session.call("fx_fault_resume").invoke().unwrap().u64();

            assert_eq!(resumed, 77, "the handler wrote x0 and advanced pc through the ucontext");
            assert_eq!((session.int("fx_seen_signal"), session.int("fx_seen_code")), (11, 1));
            assert_eq!(session.call("fx_seen_address").invoke().unwrap().u64(), 0x10);
            assert_ne!(session.call("fx_seen_pc").invoke().unwrap().u64(), 0);
        });
    }
}

bounded_test! {
    fn bionic_hardware_sigill_and_sigtrap_reach_the_guest_handler_and_resume_after_the_instruction() {
        // cite: bionic siginfo.h#ILL_ILLOPC, TRAP_BRKPT
        each_session(|session| {
            session.skipper(4);
            session.skipper(5);

            assert_eq!(session.call("fx_illegal_resume").invoke().unwrap().u64(), 77);
            assert_eq!((session.int("fx_seen_signal"), session.int("fx_seen_code")), (4, 1));
            assert_eq!(session.call("fx_breakpoint_resume").invoke().unwrap().u64(), 77);
            assert_eq!((session.int("fx_seen_signal"), session.int("fx_seen_code")), (5, 1));
        });
    }
}

bounded_test! {
    fn bionic_pthread_kill_delivers_a_mapped_signal_on_the_target_guest_thread() {
        // cite: bionic pthread_kill.cpp#pthread_kill
        each_session(|session| {
            session.recorder(10, 0);

            assert_eq!(session.call("fx_signal_another_thread").i32(10).invoke().unwrap().i32(), 0);

            assert_eq!(session.int("fx_seen_signal"), 10);
            assert_eq!(session.int("fx_seen_tid"), session.int("fx_worker_tid"));
        });
    }
}

bounded_test! {
    fn bionic_tgkill_with_a_thread_id_reaches_that_guest_thread() {
        // cite: bionic SYSCALLS.TXT#tgkill
        let session = Session::new();
        session.recorder(12, 0);

        assert_eq!(session.call("fx_send_to_thread_by_tid").i32(12).invoke().unwrap().i32(), 0);

        assert_eq!(session.int("fx_seen_signal"), 12);
        assert_eq!(session.int("fx_seen_tid"), session.int("fx_worker_tid"));
    }
}

bounded_test! {
    fn bionic_virtual_signals_without_a_darwin_number_are_delivered_to_the_raising_thread() {
        // cite: bionic signal.h#SIGSTKFLT, __SIGRTMIN
        let session = Session::new();
        session.recorder(16, 0);
        session.recorder(45, 0);

        session.call("fx_raise").i32(45).invoke().unwrap();
        assert_eq!((session.int("fx_seen_signal"), session.int("fx_seen_code")), (45, -6));
        session.call("fx_raise").i32(16).invoke().unwrap();
        assert_eq!(session.int("fx_seen_signal"), 16);
    }
}

bounded_test! {
    fn virtual_signal_sent_to_another_thread_fails_with_enosys_and_a_missing_primitive_log() {
        // cite: bionic pthread_kill.cpp#pthread_kill
        let session = Session::new();
        session.recorder(16, 0);

        assert_eq!(session.call("fx_signal_another_thread").i32(16).invoke().unwrap().i32(), 38);
        assert_eq!(session.int("fx_seen_count"), 0);
    }
}

bounded_test! {
    fn bionic_sigaction_returns_the_stored_flags_handler_and_mask_without_sigkill_and_sigstop() {
        // cite: bionic sigaction.cpp#sigaction
        let session = Session::new();
        session.installed.borrow_mut().push(10);

        assert_eq!(session.int("fx_sigaction_read_back"), 1111);
        assert_eq!(session.int("fx_sigaction_invalid_codes"), 22_222_222);
    }
}

bounded_test! {
    fn bionic_signal_installs_with_restart_and_returns_the_previous_handler() {
        // cite: bionic signal.cpp#_signal
        let session = Session::new();
        session.installed.borrow_mut().push(12);

        assert_eq!(session.call("fx_signal_returns_previous").invoke().unwrap().u64(), 11);
    }
}

bounded_test! {
    fn bionic_handler_with_sa_onstack_runs_on_the_guest_alternate_stack_and_reports_ss_onstack() {
        // cite: bionic SYSCALLS.TXT#sigaltstack
        each_session(|session| {
            session.installed.borrow_mut().push(10);

            assert_eq!(session.int("fx_altstack_handler_runs_on_it"), 11);
        });
    }
}

bounded_test! {
    fn bionic_pthread_kill_signal_zero_probes_a_live_thread_and_an_invalid_thread_reports_esrch() {
        // cite: bionic pthread_kill.cpp#pthread_kill
        let session = Session::new();

        assert_eq!(session.int("fx_pthread_kill_probe_exited"), 0);
        assert_eq!(shim("pthread_kill").ptr(0).i32(0).invoke().unwrap().i32(), 3);
    }
}

bounded_test! {
    fn bionic_signal_set_operations_validate_the_signal_number_and_use_bit_signal_minus_one() {
        // cite: bionic signal.cpp#sigaddset, sigdelset, sigismember
        let set = host_cell(0u64);
        shim("sigaddset").ptr(set).i32(10).invoke().unwrap();
        shim("sigaddset").ptr(set).i32(64).invoke().unwrap();

        assert_eq!(read::<u64>(set), (1 << 9) | (1 << 63));
        assert_eq!(shim("sigismember").ptr(set).i32(10).invoke().unwrap().i32(), 1);
        assert_eq!(shim("sigismember").ptr(set).i32(11).invoke().unwrap().i32(), 0);
        assert_eq!(shim("sigaddset").ptr(set).i32(65).invoke().unwrap().i32(), -1);
        assert_eq!(shim("sigaddset").ptr(set).i32(0).invoke().unwrap().i32(), -1);
        assert_eq!(shim("sigaddset").ptr(0).i32(1).invoke().unwrap().i32(), -1);
        shim("sigfillset").ptr(set).invoke().unwrap();
        assert_eq!(read::<u64>(set), u64::MAX);
        shim("sigdelset").ptr(set).i32(1).invoke().unwrap();
        shim("sigemptyset").ptr(set).invoke().unwrap();
        assert_eq!(read::<u64>(set), 0);
    }
}

bounded_test! {
    fn bionic_sigprocmask_filters_reserved_signals_hides_sigkill_and_returns_the_old_mask() {
        // cite: bionic sigprocmask.cpp#sigprocmask64
        let _serial = SERIAL.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        std::thread::spawn(|| {
            let (all, old, none) = (host_cell(u64::MAX), host_cell(0u64), host_cell(0u64));
            assert_eq!(shim("sigprocmask").i32(2).ptr(all).ptr(old).invoke().unwrap().i32(), 0);
            assert_eq!(shim("sigprocmask").i32(0).ptr(0).ptr(old).invoke().unwrap().i32(), 0);
            let kept = read::<u64>(old);
            let reserved: u64 = (33..=40).map(|signal| 1u64 << (signal - 1)).sum();
            assert_eq!(kept & reserved, 0, "signals 33 to 40 are never blocked");
            assert_ne!(kept & (1 << 31), 0, "signal 32 is always blocked");
            assert_eq!(kept & ((1 << 8) | (1 << 18)), 0, "SIGKILL and SIGSTOP are never in the mask");
            assert_eq!(shim("sigprocmask").i32(9).ptr(none).ptr(0).invoke().unwrap().i32(), -1);
            assert_eq!(shim("pthread_sigmask").i32(9).ptr(none).ptr(0).invoke().unwrap().i32(), 22, "pthread_sigmask returns the errno");
            assert_eq!(shim("sigprocmask").i32(9).ptr(0).ptr(0).invoke().unwrap().i32(), 0, "how is not checked without a set");
            assert_eq!(shim("sigprocmask").i32(2).ptr(none).ptr(0).invoke().unwrap().i32(), 0);
            shim("sigprocmask").i32(0).ptr(0).ptr(old).invoke().unwrap();
            assert_eq!(read::<u64>(old), 1 << 31);
        })
        .join()
        .unwrap();
    }
}

bounded_test! {
    fn bionic_sigaltstack_validates_size_flags_and_disables_with_ss_disable() {
        // cite: bionic SYSCALLS.TXT#sigaltstack
        std::thread::spawn(|| {
            let stack = host_cell([0u64; 3]);
            let storage = host_cell([0u8; 8192]);
            let write = |size: u64, flags: i32| {
                unsafe {
                    std::ptr::write_unaligned(stack as *mut u64, storage as u64);
                    std::ptr::write_unaligned((stack + 8) as *mut i32, flags);
                    std::ptr::write_unaligned((stack + 16) as *mut u64, size);
                }
            };
            write(5119, 0);
            assert_eq!(shim("sigaltstack").ptr(stack).ptr(0).invoke().unwrap().i32(), -1, "below MINSIGSTKSZ");
            write(8192, 4);
            assert_eq!(shim("sigaltstack").ptr(stack).ptr(0).invoke().unwrap().i32(), -1, "unknown flag");
            write(8192, 0);
            assert_eq!(shim("sigaltstack").ptr(stack).ptr(0).invoke().unwrap().i32(), 0);
            let old = host_cell([0u64; 3]);
            shim("sigaltstack").ptr(0).ptr(old).invoke().unwrap();
            assert_eq!((read::<u64>(old), read::<i32>(old + 8), read::<u64>(old + 16)), (storage as u64, 0, 8192));
            write(0, 2);
            assert_eq!(shim("sigaltstack").ptr(stack).ptr(0).invoke().unwrap().i32(), 0);
            shim("sigaltstack").ptr(0).ptr(old).invoke().unwrap();
            assert_eq!((read::<u64>(old), read::<i32>(old + 8), read::<u64>(old + 16)), (0, 2, 0));
        })
        .join()
        .unwrap();
    }
}

bounded_test! {
    fn bionic_kill_refuses_other_processes_and_process_groups_and_accepts_the_signal_zero_probe() {
        // cite: bionic killpg.cpp#killpg
        let pid = unsafe { libc::getpid() };

        assert_eq!(shim("kill").i32(pid).i32(0).invoke().unwrap().i32(), 0);
        assert_eq!(shim("kill").i32(pid).i32(65).invoke().unwrap().i32(), -1);
        assert_eq!(shim("kill").i32(1).i32(15).invoke().unwrap().i32(), -1);
        assert_eq!(shim("kill").i32(0).i32(15).invoke().unwrap().i32(), -1);
        assert_eq!(shim("kill").i32(pid).i32(9).invoke().unwrap().i32(), -1, "SIGKILL would end the host");
        assert_eq!(shim("killpg").i32(-1).i32(15).invoke().unwrap().i32(), -1);
        assert_eq!(shim("tgkill").i32(pid + 1).i32(pid).i32(10).invoke().unwrap().i32(), -1);
        assert_eq!(shim("tgkill").i32(pid).i32(999_999).i32(10).invoke().unwrap().i32(), -1);
    }
}

bounded_test! {
    fn bionic_sigsuspend_keeps_waiting_through_host_wakeups_without_a_signal_and_returns_after_the_guest_handler_ran() {
        // cite: bionic signal.cpp#sigsuspend64
        let session = Session::new();
        session.recorder(10, 0);
        let (blocked, before, previous, empty) = (host_cell(1u64 << 9), host_cell(0u64), host_cell(0u64), host_cell(0u64));
        assert_eq!(shim("pthread_sigmask").i32(0).ptr(blocked).ptr(before).invoke().unwrap().i32(), 0);
        shim("pthread_sigmask").i32(0).ptr(0).ptr(before).invoke().unwrap();
        let target = unsafe { libc::pthread_self() } as usize;
        let sender = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(100));
            for _ in 0..3 {
                std::process::Command::new("/usr/bin/true").status().unwrap();
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
            unsafe { libc::pthread_kill(target as libc::pthread_t, libc::SIGUSR1) };
        });

        let outcome = shim("sigsuspend").ptr(empty).invoke().unwrap().i32();
        let errno = std::io::Error::last_os_error().raw_os_error();
        sender.join().unwrap();

        assert_eq!((outcome, errno), (-1, Some(4)));
        assert_eq!((session.int("fx_seen_signal"), session.int("fx_seen_count")), (10, 1));
        shim("pthread_sigmask").i32(0).ptr(0).ptr(previous).invoke().unwrap();
        assert_ne!(read::<u64>(before) & (1 << 9), 0, "the signal was blocked before sigsuspend");
        assert_eq!(read::<u64>(previous), read::<u64>(before), "the mask from before sigsuspend is back");
    }
}

bounded_test! {
    fn bionic_sigsuspend_with_a_null_set_fails_with_efault_without_waiting() {
        // cite: bionic signal.cpp#sigsuspend64
        assert_eq!(shim("sigsuspend64").ptr(0).invoke().unwrap().i32(), -1);
        assert_eq!(std::io::Error::last_os_error().raw_os_error(), Some(14));
    }
}

bounded_test! {
    fn a_signal_for_a_thread_blocked_in_a_futex_wait_runs_its_guest_handler_and_ends_the_wait_with_eintr() {
        // cite: bionic futex.h#FUTEX_WAIT
        let session = Session::new();
        session.recorder(10, 0);
        let word = host_cell(std::sync::atomic::AtomicU32::new(0));
        let (announce, announced) = std::sync::mpsc::channel();
        let waiter = std::thread::spawn(move || {
            super::guest_thread::attach_current().unwrap();
            announce.send(unsafe { libc::pthread_self() } as usize).unwrap();
            super::bionic::syscall::translate(98, [word as u64, 0, 0, 0, 0, 0])
        });
        let target = announced.recv().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(200));

        unsafe { libc::pthread_kill(target as libc::pthread_t, libc::SIGUSR1) };

        assert_eq!(waiter.join().unwrap(), -4);
        assert_eq!((session.int("fx_seen_signal"), session.int("fx_seen_count")), (10, 1));
    }
}

bounded_test! {
    fn a_signal_already_pending_before_a_futex_wait_ends_that_wait_immediately_with_eintr() {
        // cite: bionic futex.h#FUTEX_WAIT
        let word = host_cell(std::sync::atomic::AtomicU32::new(0));
        let result = std::thread::spawn(move || {
            super::guest_thread::attach_current().unwrap();
            super::signal_state::raise_pending(17);
            super::bionic::syscall::translate(98, [word as u64, 0, 0, 0, 0, 0])
        })
        .join()
        .unwrap();

        assert_eq!(result, -4);
    }
}

bounded_test! {
    fn bionic_realtime_signal_range_starts_after_the_reserved_nine_signals() {
        // cite: bionic __libc_current_sigrtmin.cpp#__libc_current_sigrtmin
        // cite: bionic __libc_current_sigrtmax.cpp#__libc_current_sigrtmax
        assert_eq!(shim("__libc_current_sigrtmin").invoke().unwrap().i32(), 41);
        assert_eq!(shim("__libc_current_sigrtmax").invoke().unwrap().i32(), 64);
    }
}

bounded_test! {
    fn bionic_abort_runs_the_guest_sigabrt_handler_then_terminates_the_process_with_sigabrt() {
        // cite: bionic abort.cpp#abort
        static EMIT: std::sync::Once = std::sync::Once::new();
        EMIT.call_once(|| {
            super::bridge::register_handler("fx_emit", |context| {
                let text = super::bionic::guest_mem::cstr_bytes(context.arg_ptr(0));
                unsafe { libc::write(2, text.as_ptr() as *const libc::c_void, text.len()) };
            });
        });
        let _serial = SERIAL.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let bridged = Bridged::load("16k", "libfx_signals.so");
        let (signal, output) = super::loader_test_support::child_outcome(|| {
            let _ = bridged.call("fx_abort_after_handler").invoke();
        });

        assert_eq!(signal, Some(libc::SIGABRT), "{output}");
        assert!(output.contains("abort handler ran"), "{output}");
    }
}
