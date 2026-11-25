use std::ffi::CString;

use super::bridge::{handler_address, BridgeProvider};
use super::bridge_test_support::register_all;
use super::dl_api::DlHandle;
use super::link_resolution::SystemLibraryProvider;
use super::loader_test_support::fixture_dir;
use super::runtime::NativeRuntime;

fn runtime() -> &'static NativeRuntime {
    register_all();
    NativeRuntime::initialize(vec![fixture_dir("16k")]).unwrap()
}

#[test]
fn initialize_is_idempotent_and_every_thread_gets_the_same_process_wide_runtime() {
    let first = runtime() as *const NativeRuntime as usize;

    let addresses: Vec<usize> = (0..8).map(|_| std::thread::spawn(|| runtime() as *const NativeRuntime as usize)).collect::<Vec<_>>().into_iter().map(|thread| thread.join().unwrap()).collect();

    assert!(addresses.iter().all(|address| *address == first), "{addresses:x?}");
    assert_eq!(NativeRuntime::current().map(|runtime| runtime as *const NativeRuntime as usize), Some(first));
}

#[test]
fn initializing_again_with_other_library_directories_is_an_explicit_error_not_a_second_runtime() {
    let first = runtime() as *const NativeRuntime as usize;

    let error = NativeRuntime::initialize(vec![fixture_dir("4k")]).err().expect("different directories are refused").to_string();

    assert!(error.contains("already initialized"), "{error}");
    assert_eq!(runtime() as *const NativeRuntime as usize, first);
}

#[test]
fn the_runtime_installs_every_registered_handler_group_into_its_bridge_provider() {
    runtime();

    assert!(handler_address("pthread_self").is_some(), "thread handler group");
    assert!(BridgeProvider.export("libc.so", "__stack_chk_guard").is_some(), "thread data object");
}

#[test]
fn a_library_loaded_through_the_runtime_runs_constructors_and_answers_call_guest() {
    let runtime = runtime();

    let handle = runtime.loader().dlopen(Some("libfx_reloc.so"), 2, None).unwrap();
    let address = |name: &str| runtime.loader().dlsym(DlHandle::Module(handle), name, None).unwrap();
    let text = CString::new("abcd").unwrap();

    assert_eq!(runtime.call_guest(address("fx_ctor_marker"), &[]).unwrap().i32(), 0xC0DE, "init_array ran through the runtime");
    assert_eq!(runtime.call_guest(address("fx_strlen"), &[text.as_ptr() as u64]).unwrap().u64(), 4);
    assert_eq!(runtime.call_guest(address("fx_copy_sum"), &[[1u8, 2, 3, 4].as_ptr() as u64, 4]).unwrap().i32(), 10);
}

const FRESH_PROCESS: &str = "WINEDROID_FRESH_RUNTIME_PROCESS";
const FRESH_TEST: &str = "native::runtime_tests::initialize_alone_installs_every_handler_group_in_a_fresh_process";

#[test]
fn initialize_alone_installs_every_handler_group_in_a_fresh_process() {
    if std::env::var_os(FRESH_PROCESS).is_some() {
        NativeRuntime::initialize(vec![fixture_dir("16k")]).unwrap();
        for name in ["pthread_self", "gettid", "getpid", "__errno", "__stack_chk_fail"] {
            assert!(handler_address(name).is_some(), "{name} handler");
        }
        assert!(BridgeProvider.export("libc.so", "__stack_chk_guard").is_some(), "thread data object");
        return;
    }

    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", FRESH_TEST, "--test-threads=1", "--nocapture"])
        .env(FRESH_PROCESS, "1")
        .output()
        .unwrap();
    let report = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));

    assert!(output.status.success() && report.contains("1 passed"), "{report}");
}
