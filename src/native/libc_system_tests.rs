use std::ffi::CString;
use std::sync::Mutex;

use super::bionic::log_sink::{start_capture, take_capture, LogRecord};
use super::bridge_test_support::Bridged;
use super::libc_core_tests::{address_of, c_text, libc_fixture, pointer_text};
use super::loader_test_support::child_outcome;

pub static LOG_STATE: Mutex<()> = Mutex::new(());

fn log_state() -> std::sync::MutexGuard<'static, ()> {
    LOG_STATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn captured(bridged: &Bridged, function: &str) -> (i32, Vec<LogRecord>) {
    start_capture();
    let result = bridged.call(function).invoke().unwrap().i32();
    (result, take_capture())
}

fn record(priority: i32, tag: &str, message: &str) -> LogRecord {
    LogRecord { buffer: 0, priority, tag: tag.to_string(), file: None, line: 0, message: message.to_string() }
}

#[test]
fn liblog_print_write_and_buf_write_reach_the_sink_with_priority_tag_and_formatted_text() {
    let _guard = log_state();
    let bridged = libc_fixture();

    let (result, records) = captured(&bridged, "fx_log_all");

    assert_eq!(result, 3);
    assert_eq!(records, vec![record(4, "fxlog", "7 x  2.50"), record(5, "fxlog", "plain"), record(6, "fxlog", "main buffer")]);
}

#[test]
fn liblog_formats_through_a_1024_byte_buffer_so_long_messages_are_cut_at_1023_characters() {
    let _guard = log_state();
    let bridged = libc_fixture();

    let (result, records) = captured(&bridged, "fx_log_long_message");

    assert_eq!(result, 1);
    assert_eq!(records[0].message.len(), 1023);
}

#[test]
fn liblog_preserves_errno_across_a_log_call() {
    let _guard = log_state();
    let bridged = libc_fixture();

    assert_eq!(bridged.call("fx_log_errno_preserved").invoke().unwrap().i32(), 77);
}

#[test]
fn liblog_null_tag_uses_the_program_name_until_a_default_tag_is_set() {
    let _guard = log_state();
    let bridged = libc_fixture();

    let (_, records) = captured(&bridged, "fx_log_null_tag");
    assert_eq!(records, vec![record(4, "app_process64", "no tag")]);
    let (_, records) = captured(&bridged, "fx_log_set_default_tag");
    assert_eq!(records, vec![record(4, "zz", "default tag")]);
    let (_, records) = captured(&bridged, "fx_log_null_tag");
    assert_eq!(records[0].tag, "app_process64", "the fixture reset the default tag");
}

#[test]
fn liblog_buf_write_accepts_main_radio_system_crash_and_default_buffers_only() {
    let _guard = log_state();
    let bridged = libc_fixture();
    for buffer in [0, 1, 3, 4, 0x7FFF_FFFF] {
        start_capture();
        assert_eq!(bridged.call("fx_log_buffer").i32(buffer).invoke().unwrap().i32(), 1, "buffer {buffer}");
        assert_eq!(take_capture().len(), 1, "buffer {buffer}");
    }
    for buffer in [2, 5, 6, 99] {
        start_capture();
        assert_eq!(bridged.call("fx_log_buffer").i32(buffer).invoke().unwrap().i32(), 1, "buffer {buffer}");
        assert!(take_capture().is_empty(), "buffer {buffer} writes nothing");
    }
}

#[test]
fn liblog_minimum_priority_filters_lower_priorities_and_reports_the_previous_value() {
    let _guard = log_state();
    let bridged = libc_fixture();
    start_capture();

    let blocked = bridged.call("fx_log_minimum").i32(5).i32(4).invoke().unwrap().i32();
    let passed = bridged.call("fx_log_minimum").i32(5).i32(5).invoke().unwrap().i32();

    assert_eq!((blocked, passed), (-1, 1));
    assert_eq!(take_capture(), vec![record(5, "fxmin", "minimum")]);
}

#[test]
fn liblog_routes_messages_to_a_guest_logger_with_the_48_byte_message_struct() {
    let _guard = log_state();
    let bridged = libc_fixture();
    let mut report = [0u8; 64];

    let summary = bridged.call("fx_log_with_guest_logger").ptr(address_of(&mut report)).invoke().unwrap().i32();

    assert_eq!(summary, 48_005);
    assert_eq!(c_text(&report), "guest|routed");
}

fn assert_text(function: &str, expected: &str) {
    let bridged = libc_fixture();
    let (signal, stderr) = child_outcome(|| {
        let _ = bridged.call(function).invoke();
    });
    assert_eq!(signal, Some(libc::SIGABRT), "{function}: {stderr}");
    assert!(stderr.contains(expected), "{function}: {stderr}");
}

#[test]
fn liblog_assert_writes_the_text_to_stderr_and_aborts_for_each_argument_shape() {
    assert_text("fx_log_assert_format", "boom 5\n");
    assert_text("fx_log_assert_condition", "Assertion failed: x == 1\n");
    assert_text("fx_log_assert_nothing", "Unspecified assertion failed\n");
}

#[test]
fn liblog_fatal_priority_stores_the_abort_message_once() {
    let bridged = libc_fixture();

    let (signal, stderr) = child_outcome(|| {
        bridged.call("fx_log_fatal").invoke().unwrap();
        let first = super::bionic::fatal::abort_message().unwrap_or("none").to_string();
        bridged.call("fx_set_abort_message").ptr(c"second".as_ptr() as usize).invoke().unwrap();
        super::host_log::line(&format!("message={first} then={}", super::bionic::fatal::abort_message().unwrap_or("none")));
    });

    assert_eq!(signal, None);
    assert!(stderr.contains("message=fatal 1 then=fatal 1"), "{stderr}");
}

#[test]
fn system_properties_serve_the_device_profile_and_leave_unknown_names_empty() {
    let bridged = libc_fixture();
    let get = |name: &str| {
        let (name, mut value) = (CString::new(name).unwrap(), [0xAAu8; 92]);
        let length = bridged.call("fx_property_get").ptr(name.as_ptr() as usize).ptr(address_of(&mut value)).invoke().unwrap().i32();
        (length, c_text(&value))
    };

    assert_eq!(get("ro.build.version.sdk"), (2, "35".to_string()));
    assert_eq!(get("ro.build.version.release"), (2, "15".to_string()));
    assert_eq!(get("ro.product.cpu.abi"), (9, "arm64-v8a".to_string()));
    assert_eq!(get("ro.kernel.qemu"), (0, String::new()));
    assert_eq!(get("ro.mediatek.platform"), (0, String::new()));
    assert_eq!(get("ro..bad"), (0, String::new()));
    assert_eq!(get("ro.build"), (0, String::new()), "a prefix without its own value is not a property");
}

#[test]
fn system_property_find_returns_a_prop_info_with_bionic_layout() {
    let bridged = libc_fixture();
    let (name, mut read_name, mut read_value, mut serial) = (CString::new("ro.build.version.sdk").unwrap(), [0u8; 32], [0u8; 92], 0u32);

    let length = bridged
        .call("fx_property_info")
        .ptr(name.as_ptr() as usize)
        .ptr(address_of(&mut read_name))
        .ptr(address_of(&mut read_value))
        .ptr(&mut serial as *mut u32 as usize)
        .invoke()
        .unwrap()
        .i32();

    assert_eq!((length, c_text(&read_name), c_text(&read_value)), (2, "ro.build.version.sdk".to_string(), "35".to_string()));
    assert_eq!(serial >> 24, 2, "the top byte of the serial is the value length");
    let info = bridged.call("fx_property_find_address").ptr(name.as_ptr() as usize).invoke().unwrap().u64() as usize;
    assert_eq!(pointer_text(info + 4), "35");
    assert_eq!(pointer_text(info + 96), "ro.build.version.sdk");
}

#[test]
fn system_property_iteration_order_follows_the_trie_with_shorter_tokens_first() {
    let order = super::bionic::props::ordered_names();
    let position = |name: &str| order.iter().position(|candidate| candidate == name).unwrap_or_else(|| panic!("{name}"));

    assert!(position("ro.build.display.id") < position("ro.hardware"), "build (5 characters) sorts before hardware (8)");
    assert!(position("ro.build.version.sdk") < position("ro.build.version.release"), "sdk (3) sorts before release (7)");
}

#[test]
fn bionic_sysconf_returns_the_clang_computed_constants_and_dynamic_values() {
    let bridged = libc_fixture();
    let call = |name: i32| bridged.call("fx_sysconf").i32(name).invoke().unwrap().u64() as i64;

    for (name, value) in super::bionic::sysconf_table::CONSTANTS {
        assert_eq!(call(*name), *value, "_SC {name}");
    }
    assert_eq!(call(0x27), 16384);
    assert_eq!(call(0x28), 16384);
    assert_eq!(call(6), 100);
    assert!(call(0x61) >= 1 && call(0x60) >= call(0x61));
    assert!(call(0x62) > 0);
    assert!(call(11) >= 256);
    assert_eq!(call(9999), -1);
    assert_eq!(bridged.call("fx_getpagesize").invoke().unwrap().i32(), 16384);
}

fn clock(bridged: &Bridged, id: i32) -> (i32, i64, i64) {
    let mut pair = [0i64; 2];
    let code = bridged.call("fx_clock_gettime").i32(id).ptr(pair.as_mut_ptr() as usize).invoke().unwrap().i32();
    (code, pair[0], pair[1])
}

#[test]
fn bionic_clock_gettime_translates_linux_clock_ids_and_rejects_unsupported_ones() {
    let bridged = libc_fixture();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;

    let (code, seconds, nanoseconds) = clock(&bridged, 0);
    assert!(code == 0 && (seconds - now).abs() <= 2 && (0..1_000_000_000).contains(&nanoseconds));
    let (_, first_seconds, first_nanoseconds) = clock(&bridged, 1);
    let (_, second_seconds, second_nanoseconds) = clock(&bridged, 1);
    assert!((second_seconds, second_nanoseconds) >= (first_seconds, first_nanoseconds), "monotonic does not step back");
    assert!(first_seconds < now / 2, "monotonic counts from boot, not from the epoch");
    let (code, boot_seconds, _) = clock(&bridged, 7);
    assert!(code == 0 && boot_seconds > 0, "boottime reads the host clock that includes sleep");
    assert_eq!(clock(&bridged, 10).0, 22, "CLOCK_SGI_CYCLE is not supported");
    assert_eq!(clock(&bridged, 4242).0, 22);
    assert_eq!(clock(&bridged, 2).0, 0, "process cpu time");
    assert_eq!(clock(&bridged, 3).0, 0, "thread cpu time");
}

#[test]
fn bionic_gettimeofday_writes_the_sixteen_byte_timeval_with_microseconds_in_the_second_word() {
    let bridged = libc_fixture();
    let mut pair = [0i64; 2];

    assert_eq!(bridged.call("fx_gettimeofday").ptr(pair.as_mut_ptr() as usize).invoke().unwrap().i32(), 0);

    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    assert!((pair[0] - now).abs() <= 2 && (0..1_000_000).contains(&pair[1]), "{pair:?}");
}

#[test]
fn tzcode_gmtime_matches_the_host_gmtime_r_over_one_hundred_billion_seconds() {
    let bridged = libc_fixture();
    let samples = [0i64, 1, -1, 86_399, 86_400, 951_782_400, 1_709_164_800, 4_102_444_800, -2_208_988_800, 99_999_999_999, -99_999_999_999, 253_402_300_799];

    for seconds in samples {
        let mut fields = [0i32; 9];
        let status = bridged.call("fx_gmtime").int(seconds as u64).ptr(fields.as_mut_ptr() as usize).invoke().unwrap().i32();
        let mut host: libc::tm = unsafe { std::mem::zeroed() };
        unsafe { libc::gmtime_r(&seconds, &mut host) };
        let expected = [host.tm_sec, host.tm_min, host.tm_hour, host.tm_mday, host.tm_mon, host.tm_year, host.tm_wday, host.tm_yday, 0];
        assert_eq!((status, fields), (0, expected), "seconds {seconds}");
    }
}

#[test]
fn tzcode_timegm_normalises_months_and_days_in_place() {
    let bridged = libc_fixture();
    let mut month = 0i32;

    let seconds = bridged.call("fx_timegm").i32(2020).i32(14).i32(1).i32(0).i32(0).i32(0).ptr(&mut month as *mut i32 as usize).invoke().unwrap().u64() as i64;

    let mut host: libc::tm = unsafe { std::mem::zeroed() };
    host.tm_year = 120;
    host.tm_mon = 14;
    host.tm_mday = 1;
    assert_eq!(seconds, unsafe { libc::timegm(&mut host) });
    assert_eq!(month, 2, "month 14 of 2020 is March 2021");
}

#[test]
fn tzcode_asctime_r_formats_the_twenty_six_byte_line() {
    let bridged = libc_fixture();
    let mut buffer = [0u8; 40];

    assert_eq!(bridged.call("fx_asctime").int(0).ptr(address_of(&mut buffer)).invoke().unwrap().i32(), 1);

    assert_eq!(c_text(&buffer), "Thu Jan  1 00:00:00 1970\n");
}

#[test]
fn bionic_usleep_and_sleep_return_zero_after_sleeping() {
    let bridged = libc_fixture();
    let started = std::time::Instant::now();

    assert_eq!(bridged.call("fx_usleep").u32(20_000).invoke().unwrap().i32(), 0);
    assert_eq!(bridged.call("fx_sleep_remaining").invoke().unwrap().i32(), 0);

    assert!(started.elapsed() >= std::time::Duration::from_millis(20));
}
