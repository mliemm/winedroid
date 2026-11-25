use std::ffi::CString;
use std::sync::Mutex;

use super::bridge_test_support::Bridged;
use super::libc_core_tests::{address_of, c_text, libc_fixture};

static TIME_ZONE: Mutex<()> = Mutex::new(());

fn host_tzset() {
    extern "C" {
        fn tzset();
    }
    unsafe { tzset() };
}

struct Zone<'a> {
    bridged: &'a Bridged,
    _guard: std::sync::MutexGuard<'static, ()>,
}

fn use_zone<'a>(bridged: &'a Bridged, name: &str) -> Zone<'a> {
    let guard = TIME_ZONE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let text = CString::new(name).unwrap();
    bridged.call("fx_set_time_zone").ptr(text.as_ptr() as usize).invoke().unwrap();
    std::env::set_var("TZ", name);
    host_tzset();
    Zone { bridged, _guard: guard }
}

impl Drop for Zone<'_> {
    fn drop(&mut self) {
        self.bridged.call("fx_set_time_zone").ptr(0).invoke().unwrap();
        std::env::remove_var("TZ");
        host_tzset();
    }
}

fn guest_local(bridged: &Bridged, seconds: i64) -> ([i32; 10], String) {
    let (mut fields, mut zone) = ([0i32; 10], [0u8; 16]);
    let code = bridged.call("fx_localtime").int(seconds as u64).ptr(fields.as_mut_ptr() as usize).ptr(address_of(&mut zone)).invoke().unwrap().i32();
    assert_eq!(code, 0);
    (fields, c_text(&zone))
}

fn host_local(seconds: i64) -> ([i32; 10], String) {
    let mut parts: libc::tm = unsafe { std::mem::zeroed() };
    unsafe { libc::localtime_r(&seconds, &mut parts) };
    let zone = unsafe { std::ffi::CStr::from_ptr(parts.tm_zone) }.to_string_lossy().into_owned();
    ([parts.tm_sec, parts.tm_min, parts.tm_hour, parts.tm_mday, parts.tm_mon, parts.tm_year, parts.tm_wday, parts.tm_yday, parts.tm_isdst, parts.tm_gmtoff as i32], zone)
}

const SAMPLES: [i64; 22] = [
    0, 86_400 * 365, 320_000_000, 951_782_400, 1_173_000_000, 1_173_600_000, 1_678_604_399, 1_678_604_400, 1_699_164_000, 1_699_164_001, 1_699_167_599, 1_699_167_600,
    1_709_164_800, 1_720_000_000, 1_735_689_600, 2_140_000_000, 2_500_000_000, 4_102_444_800, 4_117_000_000, -1, -1_000_000_000, 7_000_000_000,
];

#[test]
fn tzcode_localtime_matches_the_host_for_zone_ids_across_dst_transitions_and_the_posix_footer() {
    let bridged = libc_fixture();
    for zone in ["Asia/Jakarta", "America/New_York", "Europe/London", "Australia/Sydney", "America/Sao_Paulo", "Asia/Kolkata", "UTC"] {
        let _zone = use_zone(&bridged, zone);
        for seconds in SAMPLES {
            assert_eq!(guest_local(&bridged, seconds), host_local(seconds), "{zone} at {seconds}");
        }
    }
}

#[test]
fn tzcode_localtime_parses_posix_tz_strings_with_month_week_day_rules() {
    let bridged = libc_fixture();
    for zone in ["EST5EDT,M3.2.0,M11.1.0", "CET-1CEST,M3.5.0,M10.5.0/3", "<+03>-3", "PST8PDT"] {
        let _zone = use_zone(&bridged, zone);
        for seconds in SAMPLES {
            assert_eq!(guest_local(&bridged, seconds), host_local(seconds), "{zone} at {seconds}");
        }
    }
}

#[test]
fn tzcode_settzname_publishes_tzname_timezone_and_daylight() {
    let bridged = libc_fixture();
    let _zone = use_zone(&bridged, "America/New_York");
    let (mut names, mut west) = ([0u8; 32], 0i64);

    let daylight = bridged.call("fx_time_zone_globals").ptr(address_of(&mut names)).ptr(&mut west as *mut i64 as usize).invoke().unwrap().i32();

    assert_eq!((c_text(&names), west, daylight), ("EST|EDT".to_string(), 18_000, 1));
}

fn mktime(bridged: &Bridged, fields: [i32; 7]) -> (i64, [i32; 9]) {
    let mut out = [0i32; 9];
    let seconds = bridged
        .call("fx_mktime")
        .i32(fields[0])
        .i32(fields[1])
        .i32(fields[2])
        .i32(fields[3])
        .i32(fields[4])
        .i32(fields[5])
        .i32(fields[6])
        .ptr(out.as_mut_ptr() as usize)
        .invoke()
        .unwrap()
        .u64() as i64;
    (seconds, out)
}

fn host_mktime(fields: [i32; 7]) -> (i64, [i32; 9]) {
    let mut parts: libc::tm = unsafe { std::mem::zeroed() };
    (parts.tm_year, parts.tm_mon, parts.tm_mday, parts.tm_hour, parts.tm_min, parts.tm_sec, parts.tm_isdst) = (fields[0] - 1900, fields[1], fields[2], fields[3], fields[4], fields[5], fields[6]);
    let seconds = unsafe { libc::mktime(&mut parts) };
    (seconds, [parts.tm_sec, parts.tm_min, parts.tm_hour, parts.tm_mday, parts.tm_mon, parts.tm_year, parts.tm_wday, parts.tm_yday, parts.tm_isdst])
}

#[test]
fn tzcode_mktime_normalises_fields_and_matches_the_host_outside_dst_gaps() {
    let bridged = libc_fixture();
    let _zone = use_zone(&bridged, "America/New_York");
    for fields in [
        [2023, 5, 15, 12, 0, 0, -1],
        [2020, 14, 1, 0, 0, 0, -1],
        [2021, 0, 0, 0, 0, 0, -1],
        [2021, 0, 1, 0, 0, -1, -1],
        [2023, 10, 5, 1, 30, 0, 1],
        [2023, 10, 5, 1, 30, 0, 0],
        [2023, 10, 5, 3, 0, 0, -1],
        [1970, 0, 1, 0, 0, 0, -1],
        [2100, 6, 4, 12, 0, 0, -1],
    ] {
        assert_eq!(mktime(&bridged, fields), host_mktime(fields), "{fields:?}");
    }
}

#[test]
fn tzcode_mktime_moves_a_nonexistent_spring_forward_time_past_the_gap() {
    let bridged = libc_fixture();
    let _zone = use_zone(&bridged, "America/New_York");

    let (seconds, fields) = mktime(&bridged, [2023, 2, 12, 2, 30, 0, -1]);

    assert_eq!(seconds, 1_678_606_200, "2023-03-12 07:30:00 UTC");
    assert_eq!((fields[2], fields[8]), (3, 1), "normalised to 03:30 in daylight time");
}

#[test]
fn tzcode_ctime_r_formats_local_time() {
    let bridged = libc_fixture();
    let _zone = use_zone(&bridged, "Asia/Jakarta");
    let mut buffer = [0u8; 40];

    assert_eq!(bridged.call("fx_ctime").int(0).ptr(address_of(&mut buffer)).invoke().unwrap().i32(), 1);

    assert_eq!(c_text(&buffer), "Thu Jan  1 07:00:00 1970\n");
}

fn strftime(bridged: &Bridged, format: &str, seconds: i64, local: bool, size: usize) -> (usize, String) {
    let (format, mut buffer) = (CString::new(format).unwrap(), vec![0u8; size]);
    let length = bridged.call("fx_strftime").ptr(address_of(&mut buffer)).int(size as u64).ptr(format.as_ptr() as usize).int(seconds as u64).i32(i32::from(local)).invoke().unwrap().u64() as usize;
    (length, c_text(&buffer))
}

#[test]
fn tzcode_strftime_c_locale_conversions_for_a_known_instant() {
    let bridged = libc_fixture();
    let _zone = use_zone(&bridged, "Asia/Jakarta");
    let instant = 1_709_078_400 + 3 * 3600 + 5 * 60 + 9;

    for (format, expected) in [
        ("%a %A %b %B %h", "Wed Wednesday Feb February Feb"),
        ("%c", "Wed Feb 28 10:05:09 2024"),
        ("%D %F %T %R %r", "02/28/24 2024-02-28 10:05:09 10:05 10:05:09 AM"),
        ("%e|%d|%j|%k|%l|%I|%H|%M|%S|%m|%y|%Y|%C", "28|28|059|10|10|10|10|05|09|02|24|2024|20"),
        ("%p %P %u %w %U %W %V %G %g", "AM am 3 3 08 09 09 2024 24"),
        ("%x %X %n%t%%", "02/28/24 10:05:09 \n\t%"),
        ("%Z %z %s", "WIB +0700 1709089509"),
        ("%-d %_d %0e %^a %#b %-H|%_m", "28 28 28 WED fEB 10| 2"),
    ] {
        let (length, text) = strftime(&bridged, format, instant, true, 128);
        assert_eq!((length, text.as_str()), (expected.len(), expected), "{format}");
    }
}

#[test]
fn tzcode_strftime_returns_zero_when_the_result_does_not_fit() {
    let bridged = libc_fixture();

    assert_eq!(strftime(&bridged, "%Y-%m-%d", 0, false, 11), (10, "1970-01-01".to_string()));
    assert_eq!(strftime(&bridged, "%Y-%m-%d", 0, false, 10).0, 0);
}
