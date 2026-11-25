use super::bridge::BridgeProvider;
use super::bridge_test_support::Bridged;
use super::data_fault::{install, SLOT_BYTES};
use super::guest_thread::attach_current;
use super::link_resolution::{ImportKind, SystemLibraryProvider};
use super::loader_test_support::{child_exit_code, child_outcome};

extern "C" fn exit_with_seventy_seven(_: libc::c_int, _: *mut libc::siginfo_t, _: *mut libc::c_void) {
    unsafe { libc::_exit(77) };
}

fn set_action(signal: libc::c_int, handler: usize, flags: i32) {
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = handler;
        action.sa_flags = flags;
        libc::sigemptyset(&mut action.sa_mask);
        assert_eq!(libc::sigaction(signal, &action, std::ptr::null_mut()), 0);
    }
}

fn touch(address: usize) {
    unsafe { std::ptr::read_volatile(address as *const u8) };
}

#[test]
fn reading_an_unresolved_data_import_logs_the_library_and_symbol_then_aborts() {
    let bridged = Bridged::load("16k", "libfx_bridge.so");

    let (signal, stderr) = child_outcome(|| {
        let _ = bridged.call("fx_read_missing_object").invoke();
    });

    assert_eq!(signal, Some(libc::SIGABRT));
    assert!(stderr.contains("FIXME: native data import not implemented: "), "{stderr}");
    assert!(stderr.contains("libfx_bridge.so fx_missing_object"), "{stderr}");
}

#[test]
fn a_fault_inside_a_later_data_slot_names_that_symbol_by_its_offset() {
    let provider = BridgeProvider;
    let first = provider.unresolved("/guest/libz.so", "fx_data_slot_first", ImportKind::Data);
    let second = provider.unresolved("/guest/libz.so", "fx_data_slot_second", ImportKind::Data);
    assert_eq!(second - first, SLOT_BYTES);

    let (signal, stderr) = child_outcome(|| touch(second + SLOT_BYTES - 8));
    let (first_signal, first_stderr) = child_outcome(|| touch(first));

    assert_eq!(signal, Some(libc::SIGABRT));
    assert!(stderr.contains("/guest/libz.so fx_data_slot_second") && !stderr.contains("fx_data_slot_first"), "{stderr}");
    assert_eq!(first_signal, Some(libc::SIGABRT));
    assert!(first_stderr.contains("fx_data_slot_first") && !first_stderr.contains("fx_data_slot_second"), "{first_stderr}");
}

#[test]
fn a_fault_outside_every_unresolved_data_slot_goes_to_the_previous_handler() {
    BridgeProvider.unresolved("/guest/libz.so", "fx_data_slot_chain", ImportKind::Data);

    for signal in [libc::SIGSEGV, libc::SIGBUS] {
        let code = child_exit_code(|| {
            set_action(libc::SIGSEGV, exit_with_seventy_seven as *const () as usize, libc::SA_SIGINFO);
            set_action(libc::SIGBUS, exit_with_seventy_seven as *const () as usize, libc::SA_SIGINFO);
            install();
            unsafe { libc::raise(signal) };
            touch(8);
        });
        assert_eq!(code, Some(77), "signal {signal}");
    }
}

#[test]
fn a_fault_outside_every_slot_with_the_default_action_underneath_still_kills_the_process_with_that_signal() {
    BridgeProvider.unresolved("/guest/libz.so", "fx_data_slot_default", ImportKind::Data);

    let (signal, stderr) = child_outcome(|| {
        set_action(libc::SIGSEGV, libc::SIG_DFL, 0);
        set_action(libc::SIGBUS, libc::SIG_DFL, 0);
        install();
        touch(8);
    });

    assert!(matches!(signal, Some(libc::SIGSEGV) | Some(libc::SIGBUS)), "{signal:?}");
    assert!(!stderr.contains("FIXME"), "{stderr}");
}

extern "C" fn attach_on_bare_thread(_: *mut libc::c_void) -> *mut libc::c_void {
    let mut before: libc::stack_t = unsafe { std::mem::zeroed() };
    unsafe { libc::sigaltstack(std::ptr::null(), &mut before) };
    attach_current().unwrap();
    let mut after: libc::stack_t = unsafe { std::mem::zeroed() };
    unsafe { libc::sigaltstack(std::ptr::null(), &mut after) };
    let had_none = before.ss_flags & libc::SS_DISABLE != 0;
    let has_one = after.ss_flags & libc::SS_DISABLE == 0 && after.ss_size >= 32 * 1024;
    usize::from(had_none && has_one) as *mut libc::c_void
}

#[test]
fn a_thread_without_an_alternate_signal_stack_gets_one_when_it_enters_guest_code() {
    let mut thread: libc::pthread_t = unsafe { std::mem::zeroed() };
    let mut result: *mut libc::c_void = std::ptr::null_mut();

    unsafe {
        assert_eq!(libc::pthread_create(&mut thread, std::ptr::null(), attach_on_bare_thread, std::ptr::null_mut()), 0);
        assert_eq!(libc::pthread_join(thread, &mut result), 0);
    }

    assert_eq!(result as usize, 1);
}
