use std::ffi::CString;

use super::bridge_test_support::Bridged;
use super::loader_test_support::child_outcome;

pub fn libc_fixture() -> Bridged {
    Bridged::load("16k", "libfx_libc.so")
}

pub fn address_of(bytes: &mut [u8]) -> usize {
    bytes.as_mut_ptr() as usize
}

pub fn c_text(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|byte| *byte == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

pub fn pointer_text(pointer: usize) -> String {
    unsafe { std::ffi::CStr::from_ptr(pointer as *const _) }.to_string_lossy().into_owned()
}

fn call_text(function: &str, size: usize) -> (String, i32) {
    let bridged = libc_fixture();
    let mut buffer = vec![0u8; size];
    let result = bridged.call(function).ptr(address_of(&mut buffer)).int(size as u64).invoke().unwrap().i32();
    (c_text(&buffer), result)
}

#[test]
fn bionic_vfprintf_formats_integers_strings_flags_and_precision_from_guest_varargs() {
    let (text, written) = call_text("fx_snprintf_mix", 256);

    assert_eq!(text, "-42  3.14 str z ff 18446744073709551615 %|7   |0042|+5|010|0XFF|abc|      abcd|");
    assert_eq!(written as usize, text.len());
}

#[test]
fn bionic_vfprintf_applies_integer_precision_as_minimum_digits_and_ignores_the_zero_flag_then() {
    let (text, _) = call_text("fx_snprintf_int_precision", 128);

    assert_eq!(text, "00042|     007||0ff     |     005|3");
}

#[test]
fn bionic_vfprintf_formats_floating_point_conversions() {
    let (text, _) = call_text("fx_snprintf_floats", 128);

    assert_eq!(text, "1.234568e+04|0.0001|1E-10|2|-1.230e-04|0x1p+0");
}

#[test]
fn bionic_vsnprintf_returns_the_untruncated_length_and_terminates_the_truncated_text() {
    let bridged = libc_fixture();
    let mut buffer = [0xAAu8; 8];

    assert_eq!(bridged.call("fx_snprintf_truncated").ptr(address_of(&mut buffer)).invoke().unwrap().i32(), 8);
    assert_eq!(&buffer[..4], b"abc\0");
    assert_eq!(buffer[4], 0xAA, "bytes past the size are untouched");
    assert_eq!(bridged.call("fx_snprintf_measure").invoke().unwrap().i32(), 13);
}

#[test]
fn bionic_vfprintf_prints_null_strings_and_pointers_like_bionic() {
    let bridged = libc_fixture();
    let mut buffer = [0u8; 64];

    bridged.call("fx_sprintf_pointers").ptr(address_of(&mut buffer)).invoke().unwrap();

    assert_eq!(c_text(&buffer), "(null)|0x0|0x1234");
}

#[test]
fn bionic_vsnprintf_and_vsprintf_read_a_guest_va_list() {
    let bridged = libc_fixture();
    let (mut buffer, format) = ([0u8; 64], CString::new("%s=%d/%c").unwrap());
    let key = CString::new("key").unwrap();

    let written = bridged
        .call("fx_vsnprintf_wrap")
        .ptr(address_of(&mut buffer))
        .int(64)
        .ptr(format.as_ptr() as usize)
        .ptr(key.as_ptr() as usize)
        .i32(9)
        .i32(i32::from(b'q'))
        .invoke()
        .unwrap()
        .i32();
    assert_eq!((c_text(&buffer), written), ("key=9/q".to_string(), 7));

    let mut second = [0u8; 64];
    bridged.call("fx_vsprintf_wrap").ptr(address_of(&mut second)).ptr(format.as_ptr() as usize).ptr(key.as_ptr() as usize).i32(-3).i32(i32::from(b'x')).invoke().unwrap();
    assert_eq!(c_text(&second), "key=-3/x");
}

#[test]
fn bionic_fortified_snprintf_formats_when_the_buffer_is_large_enough() {
    let bridged = libc_fixture();
    let mut buffer = [0u8; 16];

    assert_eq!(bridged.call("fx_snprintf_chk_ok").ptr(address_of(&mut buffer)).invoke().unwrap().i32(), 4);
    assert_eq!(c_text(&buffer), "5:ok");
}

#[test]
fn bionic_fortified_snprintf_aborts_with_the_fortify_line_when_the_claim_exceeds_the_buffer() {
    let bridged = libc_fixture();
    let mut buffer = [0u8; 64];
    let address = address_of(&mut buffer);

    let (signal, stderr) = child_outcome(|| {
        let _ = bridged.call("fx_snprintf_chk_overflow").ptr(address).invoke();
    });

    assert_eq!(signal, Some(libc::SIGABRT));
    assert!(stderr.contains("FORTIFY: vsnprintf: prevented 32-byte write into 16-byte buffer\n"), "{stderr}");
}

#[test]
fn bionic_fortified_vsprintf_checks_the_written_length_after_the_write() {
    let bridged = libc_fixture();
    let mut buffer = [0u8; 64];
    let (address, format, text) = (address_of(&mut buffer), CString::new("%s").unwrap(), CString::new("12345678").unwrap());

    let (signal, stderr) = child_outcome(|| {
        let _ = bridged.call("fx_vsprintf_chk_overflow").ptr(address).ptr(format.as_ptr() as usize).ptr(text.as_ptr() as usize).invoke();
    });

    assert_eq!(signal, Some(libc::SIGABRT));
    assert!(stderr.contains("FORTIFY: vsprintf: prevented 9-byte write into 4-byte buffer\n"), "{stderr}");
}

struct Parse {
    value: i64,
    consumed: i64,
    error: i32,
}

fn parse_signed(text: &str, base: i32) -> Parse {
    let bridged = libc_fixture();
    let (text, mut consumed, mut error) = (CString::new(text).unwrap(), 0i64, 0i32);
    let value = bridged.call("fx_strtol").ptr(text.as_ptr() as usize).i32(base).ptr(&mut consumed as *mut i64 as usize).ptr(&mut error as *mut i32 as usize).invoke().unwrap().u64() as i64;
    Parse { value, consumed, error }
}

fn parse_unsigned(text: &str, base: i32) -> (u64, i64, i32) {
    let bridged = libc_fixture();
    let (text, mut consumed, mut error) = (CString::new(text).unwrap(), 0i64, 0i32);
    let value = bridged.call("fx_strtoul").ptr(text.as_ptr() as usize).i32(base).ptr(&mut consumed as *mut i64 as usize).ptr(&mut error as *mut i32 as usize).invoke().unwrap().u64();
    (value, consumed, error)
}

#[test]
fn bionic_strtol_accepts_the_binary_prefix_that_the_host_strtol_rejects() {
    let parsed = parse_signed("0b101", 0);

    assert_eq!((parsed.value, parsed.consumed, parsed.error), (5, 5, 0));
    let host = unsafe { libc::strtol(c"0b101".as_ptr(), std::ptr::null_mut(), 0) };
    assert_eq!(host, 0, "the oracle for rejecting the host binding is macOS strtol");
}

#[test]
fn bionic_strtol_prefix_base_and_range_rules() {
    let hex = parse_signed("0x1f", 0);
    assert_eq!((hex.value, hex.consumed, hex.error), (31, 4, 0));
    let bare = parse_signed("0x", 0);
    assert_eq!((bare.value, bare.consumed, bare.error), (0, 1, 0));
    let spaced = parse_signed("  -12abc", 10);
    assert_eq!((spaced.value, spaced.consumed), (-12, 5));
    let big = parse_signed("zz", 36);
    assert_eq!(big.value, 1295);
    let overflow = parse_signed("99999999999999999999", 10);
    assert_eq!((overflow.value, overflow.error), (i64::MAX, 34));
    let underflow = parse_signed("-99999999999999999999", 10);
    assert_eq!((underflow.value, underflow.error), (i64::MIN, 34));
    let minimum = parse_signed("-9223372036854775808", 10);
    assert_eq!((minimum.value, minimum.error), (i64::MIN, 0));
    let octal = parse_signed("0777", 0);
    assert_eq!(octal.value, 0o777);
    for base in [1, 37, -1] {
        let invalid = parse_signed("12", base);
        assert_eq!((invalid.value, invalid.consumed, invalid.error), (0, 0, 22), "base {base}");
    }
    let none = parse_signed("abc", 10);
    assert_eq!((none.value, none.consumed), (0, 0));
}

#[test]
fn bionic_strtoul_negates_unsigned_values_and_reports_range_errors() {
    assert_eq!(parse_unsigned("-1", 10), (u64::MAX, 2, 0));
    assert_eq!(parse_unsigned("18446744073709551615", 10), (u64::MAX, 20, 0));
    assert_eq!(parse_unsigned("18446744073709551616", 10), (u64::MAX, 20, 34));
    assert_eq!(parse_unsigned("0b11", 2), (3, 4, 0));
}

#[test]
fn bionic_atoi_reads_a_leading_decimal_integer() {
    let bridged = libc_fixture();
    let text = CString::new(" 42x").unwrap();

    assert_eq!(bridged.call("fx_atoi").ptr(text.as_ptr() as usize).invoke().unwrap().i32(), 42);
}

fn parse_double(text: &str) -> (f64, i64, i32) {
    let bridged = libc_fixture();
    let (text, mut consumed, mut error) = (CString::new(text).unwrap(), 0i64, 0i32);
    let value = bridged.call("fx_strtod").ptr(text.as_ptr() as usize).ptr(&mut consumed as *mut i64 as usize).ptr(&mut error as *mut i32 as usize).invoke().unwrap().f64();
    (value, consumed, error)
}

#[test]
fn gdtoa_strtod_vectors_for_hex_inf_nan_and_range_errors() {
    assert_eq!(parse_double("0x1p-2"), (0.25, 6, 0));
    assert_eq!(parse_double("inf"), (f64::INFINITY, 3, 0));
    assert_eq!(parse_double("infinity"), (f64::INFINITY, 8, 0));
    let (nan, consumed, _) = parse_double("nan(0x1)");
    assert!(nan.is_nan() && consumed == 8);
    assert_eq!(parse_double("1e999999999999"), (f64::INFINITY, 14, 34));
    assert_eq!(parse_double("1e-999999").2, 34);
    let (zero, consumed, _) = parse_double("  -0");
    assert!(zero == 0.0 && zero.is_sign_negative() && consumed == 4);
    assert_eq!(parse_double(".").1, 0);
    assert_eq!(parse_double("1e"), (1.0, 1, 0));
    assert_eq!(parse_double("0x"), (0.0, 1, 0));
}

fn fnv(values: &[i32]) -> u64 {
    values.iter().flat_map(|value| value.to_le_bytes()).fold(0xcbf2_9ce4_8422_2325u64, |hash, byte| (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3))
}

fn sort_with_sequence(values: &[i32]) -> (Vec<i32>, usize, u64, Vec<i32>) {
    let bridged = libc_fixture();
    let mut data = values.to_vec();
    let calls = bridged.call("fx_qsort_ints").ptr(data.as_mut_ptr() as usize).int(data.len() as u64).invoke().unwrap().i32() as usize;
    let sequence: Vec<i32> = (0..calls).flat_map(|index| (0..2).map(move |side| (index, side))).map(|(index, side)| bridged.call("fx_comparison_value").i32(index as i32).i32(side).invoke().unwrap().i32()).collect();
    (data, calls, fnv(&sequence), sequence)
}

#[test]
fn upstream_freebsd_qsort_comparator_call_sequence_for_five_eight_and_fifty_elements() {
    let (sorted, calls, hash, sequence) = sort_with_sequence(&[5, 3, 9, 1, 7]);
    assert_eq!((sorted, calls, hash), (vec![1, 3, 5, 7, 9], 7, 0xecd203e377ad8b3d));
    assert_eq!(&sequence[..6], &[5, 3, 5, 9, 9, 1]);

    let (sorted, calls, hash, _) = sort_with_sequence(&[8, 2, 7, 3, 6, 4, 5, 1]);
    assert_eq!((sorted, calls, hash), ((1..=8).collect::<Vec<_>>(), 16, 0xdeef_d5f5_79fb_9efc));

    let fifty: Vec<i32> = (0..50).map(|index| (index * 37 + 11) % 50).collect();
    let (sorted, calls, hash, _) = sort_with_sequence(&fifty);
    assert_eq!((sorted, calls, hash), ((0..50).collect::<Vec<_>>(), 260, 0x274b_3e39_4ca3_a73a));
}

#[test]
fn upstream_freebsd_qsort_comparator_call_sequence_with_equal_keys() {
    let (sorted, calls, hash, _) = sort_with_sequence(&[4; 10]);
    assert_eq!((sorted, calls, hash), (vec![4; 10], 12, 0x6801_e37d_1fb5_3fa5));

    let (sorted, calls, hash, _) = sort_with_sequence(&[3, 1, 3, 1, 3, 1, 2, 2, 2, 0, 0, 0]);
    assert_eq!((sorted, calls, hash), (vec![0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3], 23, 0xf824_5cc2_46b2_d9f6));
}

#[test]
fn upstream_freebsd_qsort_orders_equal_sized_partitions_by_recursing_on_the_left_first() {
    let input = [11, 23, 28, 4, 28, 20, 24, 26, 0, 13, 22, 16, 9, 14, 11, 13, 27, 18, 18, 13, 28];

    let (sorted, calls, hash, _) = sort_with_sequence(&input);

    let mut expected = input.to_vec();
    expected.sort_unstable();
    assert_eq!((sorted, calls, hash), (expected, 63, 0x2f0e_90dd_b588_07ad));
}

#[test]
fn qsort_of_zero_elements_never_touches_the_base_pointer() {
    let bridged = libc_fixture();

    assert_eq!(bridged.call("fx_qsort_ints").ptr(0).int(0).invoke().unwrap().i32(), 0);
}

#[test]
fn upstream_netbsd_bsearch_finds_present_keys_and_returns_null_for_absent_ones() {
    let bridged = libc_fixture();
    let mut data = [1, 3, 5, 7, 9, 11];
    let base = data.as_mut_ptr() as usize;

    for (index, key) in [1, 3, 5, 7, 9, 11].into_iter().enumerate() {
        assert_eq!(bridged.call("fx_bsearch_ints").i32(key).ptr(base).int(6).invoke().unwrap().u64() as usize, base + index * 4, "key {key}");
    }
    assert_eq!(bridged.call("fx_bsearch_ints").i32(4).ptr(base).int(6).invoke().unwrap().u64(), 0);
}

#[test]
fn bionic_string_functions_cover_case_folding_search_and_bounded_copies() {
    let bridged = libc_fixture();
    let mut buffer = [0u8; 32];

    assert_eq!(bridged.call("fx_string_ops").ptr(address_of(&mut buffer)).invoke().unwrap().i32(), 0);
}

#[test]
fn bionic_strtok_r_splits_on_any_delimiter() {
    let bridged = libc_fixture();
    let mut text = *b"alpha,beta gamma\0";
    let mut out = [0u8; 64];

    let count = bridged.call("fx_strtok_words").ptr(address_of(&mut text)).ptr(address_of(&mut out)).invoke().unwrap().i32();

    assert_eq!((count, c_text(&out)), (3, "alpha|beta|gamma|".to_string()));
}

#[test]
fn bionic_strlen_counts_bytes_to_the_terminator() {
    let bridged = libc_fixture();
    let text = CString::new("hello bridge").unwrap();

    assert_eq!(bridged.call("fx_strlen").ptr(text.as_ptr() as usize).invoke().unwrap().u64(), 12);
}
