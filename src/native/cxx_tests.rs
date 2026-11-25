use std::ffi::CString;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::bionic::abi_types::ERANGE;
use super::bionic::atfork;
use super::bionic::guest_mem::read_value;
use super::bionic::log_sink::{start_capture, take_capture};
use super::bionic::stdio::flush_all_streams;
use super::bridge_test_support::register_all;
use super::thread_test_support::shim;

const ANDROID_LOG_ERROR: i32 = 6;
const ANDROID_LOG_INFO: i32 = 4;

static SYSLOG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn text(value: &str) -> CString {
    CString::new(value).unwrap()
}

fn wide(value: &str) -> Vec<u32> {
    value.chars().map(|character| character as u32).chain([0]).collect()
}

fn wide_text(value: &[u32]) -> String {
    value.iter().take_while(|character| **character != 0).map(|character| char::from_u32(*character).unwrap_or('?')).collect()
}

fn clear_errno() {
    unsafe { *libc::__error() = 0 };
}

fn errno() -> i32 {
    unsafe { *libc::__error() }
}

fn strtold(value: &str) -> (u128, usize, i32) {
    register_all();
    let input = text(value);
    let mut end: usize = 0;
    clear_errno();
    let result = shim("strtold").ptr(input.as_ptr() as usize).ptr((&mut end as *mut usize) as usize).invoke().unwrap();
    let bits = result.quad();
    let code = if errno() == 0 { 0 } else { errno() };
    (bits, end - input.as_ptr() as usize, code)
}

#[test]
fn bionic_strtold_returns_correctly_rounded_binary128_bit_patterns_for_the_expectation_table() {
    let vectors: [(&str, u128); 6] = [
        ("0.1", 0x3ffb_9999_9999_9999_9999_9999_9999_999a),
        ("1.5", 0x3fff_8000_0000_0000_0000_0000_0000_0000),
        ("3.14159265358979323846264338327950288", 0x4000_921f_b544_42d1_8469_898c_c517_01b8),
        ("1e400", 0x452f_b4ec_7f91_973f_f3cb_1ccf_26fb_c178),
        ("1e-4950", 0x0000_0000_0000_0000_0005_7c96_47e1_a018),
        ("0x1.8p3", 0x4002_8000_0000_0000_0000_0000_0000_0000),
    ];
    for (input, expected) in vectors {
        let (bits, consumed, code) = strtold(input);
        assert_eq!(bits, expected, "{input}");
        assert_eq!(consumed, input.len(), "{input} consumed");
        assert_eq!(code, 0, "{input} must not set errno");
    }
}

#[test]
fn bionic_strtold_reports_overflow_and_underflow_through_erange_and_leaves_the_end_pointer_on_failure() {
    let (overflow, consumed, code) = strtold("1e4933");
    assert_eq!(overflow, 0x7fff_0000_0000_0000_0000_0000_0000_0000);
    assert_eq!(consumed, 6);
    assert_eq!(code, ERANGE);
    let (subnormal, _, code) = strtold("1e-4951");
    assert_eq!(subnormal, 0x8c75_6d96_9002, "1e-4951 is still a representable subnormal");
    assert_eq!(code, 0);
    let (underflow, _, code) = strtold("1e-5000");
    assert_eq!(underflow, 0);
    assert_eq!(code, ERANGE);
    let (_, consumed, code) = strtold("abc");
    assert_eq!(consumed, 0);
    assert_eq!(code, 0);
    let (_, consumed, _) = strtold("0.1f");
    assert_eq!(consumed, 3, "the f suffix is not consumed");
}

#[test]
fn bionic_strtold_l_and_wcstold_agree_with_strtold_and_write_the_wide_end_pointer() {
    register_all();
    let input = text("2.5");
    let mut end: usize = 0;
    let bits = shim("strtold_l").ptr(input.as_ptr() as usize).ptr((&mut end as *mut usize) as usize).ptr(1).invoke().unwrap().quad();
    assert_eq!(bits, 0x4000_4000_0000_0000_0000_0000_0000_0000);

    let source = wide("  0x1.8p3rest");
    let mut wide_end: usize = 0;
    let bits = shim("wcstold")
        .ptr(source.as_ptr() as usize)
        .ptr((&mut wide_end as *mut usize) as usize)
        .invoke()
        .unwrap()
        .quad();
    assert_eq!(bits, 0x4002_8000_0000_0000_0000_0000_0000_0000);
    assert_eq!(wide_end, source.as_ptr() as usize + 9 * 4, "the end pointer is two spaces plus the eight numeric characters");
    let bits = shim("wcstold_l")
        .ptr(source.as_ptr() as usize)
        .ptr(0)
        .ptr(1)
        .invoke()
        .unwrap()
        .quad();
    assert_eq!(bits, 0x4002_8000_0000_0000_0000_0000_0000_0000);
}

#[test]
fn bionic_syslog_maps_priorities_filters_on_the_priority_mask_and_defaults_the_tag_to_the_program_name() {
    let _guard = SYSLOG_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    register_all();
    start_capture();
    let tag = text("winedroid-test");
    let format = text("value %d");
    shim("openlog").ptr(tag.as_ptr() as usize).i32(0).i32(0).invoke().unwrap();
    shim("setlogmask").i32(0).invoke().unwrap();
    for (priority, expected) in [(0, ANDROID_LOG_ERROR), (3, ANDROID_LOG_ERROR), (4, 5), (5, ANDROID_LOG_INFO), (6, ANDROID_LOG_INFO), (7, 3)] {
        shim("syslog").i32(priority).ptr(format.as_ptr() as usize).int(42).invoke().unwrap();
        let records = take_capture();
        assert_eq!(records.len(), 1, "priority {priority}");
        assert_eq!(records[0].priority, expected, "priority {priority}");
        assert_eq!(records[0].tag, "winedroid-test");
        assert_eq!(records[0].message, "value 42");
        start_capture();
    }

    let previous = shim("setlogmask").i32(1 << 7).invoke().unwrap().i32();
    assert_eq!(previous, 0xff, "the default mask is every priority");
    shim("syslog").i32(3).ptr(format.as_ptr() as usize).int(1).invoke().unwrap();
    assert!(take_capture().is_empty(), "a masked priority is dropped");
    start_capture();
    let unchanged = shim("setlogmask").i32(0).invoke().unwrap().i32();
    assert_eq!(unchanged, 1 << 7, "zero queries the mask without writing it");

    shim("closelog").invoke().unwrap();
    shim("syslog").i32(7).ptr(format.as_ptr() as usize).int(2).invoke().unwrap();
    let records = take_capture();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].tag, super::bionic::env::program_name(), "getprogname supplies the default tag");
}

#[test]
fn bionic_syslog_truncates_a_formatted_line_at_1023_bytes() {
    let _guard = SYSLOG_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    register_all();
    start_capture();
    shim("setlogmask").i32(0xff).invoke().unwrap();
    let long = text(&"x".repeat(2000));
    let format = text("%s");
    shim("syslog").i32(6).ptr(format.as_ptr() as usize).ptr(long.as_ptr() as usize).invoke().unwrap();
    let records = take_capture();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].message.len(), 1023);
}

#[test]
fn bionic_wcstok_walks_tokens_across_consecutive_delimiters_and_reports_the_end() {
    register_all();
    let mut buffer = wide("  alpha,,beta gamma ");
    let delimiter = wide(" ,");
    let mut state: usize = 0;
    let mut tokens = Vec::new();
    let mut source = buffer.as_mut_ptr() as usize;
    loop {
        let token = shim("wcstok")
            .ptr(source)
            .ptr(delimiter.as_ptr() as usize)
            .ptr((&mut state as *mut usize) as usize)
            .invoke()
            .unwrap()
            .u64() as usize;
        if token == 0 {
            break;
        }
        tokens.push(token);
        source = 0;
    }
    let texts: Vec<String> = tokens
        .iter()
        .map(|token| {
            let mut value = Vec::new();
            let mut index = 0;
            loop {
                let character = read_value::<u32>(token + index * 4);
                if character == 0 {
                    break;
                }
                value.push(character);
                index += 1;
            }
            wide_text(&value)
        })
        .collect();
    assert_eq!(texts, ["alpha", "beta", "gamma"]);
    assert_eq!(state, 0, "the continuation is null after the last token");
}

#[test]
fn bionic_wcsncat_appends_at_most_n_characters_and_always_terminates() {
    register_all();
    let mut destination = wide("abc");
    destination.resize(8, 0);
    let source = wide("defgh");
    let result = shim("wcsncat")
        .ptr(destination.as_mut_ptr() as usize)
        .ptr(source.as_ptr() as usize)
        .int(3)
        .invoke()
        .unwrap()
        .u64();
    assert_eq!(result, destination.as_ptr() as u64);
    assert_eq!(wide_text(&destination), "abcdef");
    assert_eq!(destination[6], 0);
}

#[test]
fn bionic_wcsftime_produces_the_same_text_as_strftime_for_the_same_broken_down_time() {
    register_all();
    let mut parts = vec![0u8; 56];
    let fields = [42i32, 5, 13, 9, 10, 124, 2, 343, 0];
    for (index, value) in fields.iter().enumerate() {
        parts[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
    }
    let zone = text("UTC");
    parts[48..56].copy_from_slice(&(zone.as_ptr() as u64).to_le_bytes());
    let format = text("%Y-%m-%d %H:%M:%S %a");
    let mut byte_buffer = vec![0u8; 64];
    let written = shim("strftime")
        .ptr(byte_buffer.as_mut_ptr() as usize)
        .int(64)
        .ptr(format.as_ptr() as usize)
        .ptr(parts.as_mut_ptr() as usize)
        .invoke()
        .unwrap()
        .u64() as usize;
    let expected = String::from_utf8(byte_buffer[..written].to_vec()).unwrap();

    let wide_format: Vec<u32> = "%Y-%m-%d %H:%M:%S %a".chars().map(|character| character as u32).chain([0]).collect();
    let mut wide_buffer = vec![0u32; 64];
    let written = shim("wcsftime")
        .ptr(wide_buffer.as_mut_ptr() as usize)
        .int(64)
        .ptr(wide_format.as_ptr() as usize)
        .ptr(parts.as_mut_ptr() as usize)
        .invoke()
        .unwrap()
        .u64() as usize;
    assert_eq!(wide_text(&wide_buffer[..written]), expected);

    let mut small = vec![0x5au32; 8];
    let written = shim("wcsftime_l")
        .ptr(small.as_mut_ptr() as usize)
        .int(4)
        .ptr(wide_format.as_ptr() as usize)
        .ptr(parts.as_mut_ptr() as usize)
        .ptr(1)
        .invoke()
        .unwrap()
        .u64();
    assert_eq!(written, 0, "a buffer that cannot hold the text returns zero");
    assert_eq!(small[3], 0, "the last writable character is a terminator");
}

#[test]
fn bionic_cxa_finalize_unregisters_the_atfork_handlers_that_belong_to_the_finalized_dso() {
    register_all();
    let dso = 0xdead_0000_1234u64;
    shim("__register_atfork").int(1).int(2).int(3).int(dso).invoke().unwrap();
    assert_eq!(atfork::entries().iter().filter(|entry| entry.dso == dso).count(), 1);

    shim("__cxa_finalize").ptr(dso as usize).invoke().unwrap();

    assert_eq!(atfork::entries().iter().filter(|entry| entry.dso == dso).count(), 0);
}

static CLEANUP_HOOK_CALLS: AtomicUsize = AtomicUsize::new(0);

extern "C" fn counting_cleanup() {
    CLEANUP_HOOK_CALLS.fetch_add(1, Ordering::SeqCst);
}

#[test]
fn bionic_cxa_finalize_without_a_dso_runs_the_stdio_cleanup_hook() {
    register_all();
    let before = CLEANUP_HOOK_CALLS.load(Ordering::SeqCst);
    super::bionic::atexit::set_stdio_cleanup(counting_cleanup);

    super::bionic::atexit::run_finish_dso(0);

    assert!(CLEANUP_HOOK_CALLS.load(Ordering::SeqCst) > before);
    super::bionic::atexit::set_stdio_cleanup(flush_all_streams);
}
