use std::ffi::CString;

use super::bionic::errno::message;
use super::bionic::strerror_table::{COUNT, TABLE};
use super::libc_core_tests::{address_of, c_text, libc_fixture, pointer_text};
use super::loader_test_support::{child_outcome, child_result};

#[test]
fn bionic_realloc_with_size_zero_frees_and_returns_null_where_macos_returns_a_block() {
    let bridged = libc_fixture();

    assert_eq!(bridged.call("fx_realloc_zero_returns_null").invoke().unwrap().i32(), 1);
    let host = unsafe { libc::realloc(libc::malloc(16), 0) };
    assert!(!host.is_null(), "the oracle for rejecting the host binding is macOS realloc(p, 0)");
    unsafe { libc::free(host) };
}

#[test]
fn bionic_calloc_and_reallocarray_overflow_fail_with_enomem() {
    let bridged = libc_fixture();

    assert_eq!(bridged.call("fx_calloc_overflow").invoke().unwrap().i32(), 12);
    assert_eq!(bridged.call("fx_reallocarray_overflow").invoke().unwrap().i32(), 12);
}

#[test]
fn bionic_reallocarray_with_a_zero_product_frees_and_returns_null() {
    let bridged = libc_fixture();

    assert_eq!(bridged.call("fx_reallocarray_zero_returns_null").int(0).int(8).invoke().unwrap().i32(), 1);
    assert_eq!(bridged.call("fx_reallocarray_zero_returns_null").int(8).int(0).invoke().unwrap().i32(), 1);
}

#[test]
fn bionic_memalign_returns_aligned_blocks_with_enough_usable_size() {
    let bridged = libc_fixture();

    for alignment in [16u64, 64, 4096] {
        assert_eq!(bridged.call("fx_memalign_usable").int(alignment).int(100).invoke().unwrap().i32(), 1, "alignment {alignment}");
    }
}

#[test]
fn bionic_mallinfo_in_use_bytes_grow_after_a_one_mebibyte_allocation() {
    let bridged = libc_fixture();

    let best = (0..50).map(|_| bridged.call("fx_mallinfo_in_use_growth").invoke().unwrap().u64()).max().unwrap();

    assert!(best >= 1 << 20, "{best}");
}

#[test]
fn bionic_mallopt_follows_the_documented_option_table() {
    let bridged = libc_fixture();
    let option = |option: i32, value: i32| bridged.call("fx_mallopt").i32(option).i32(value).invoke().unwrap().i32();

    assert_eq!(option(-100, 0), 1, "M_DECAY_TIME 0");
    assert_eq!(option(-100, 2), 0, "M_DECAY_TIME 2");
    assert_eq!(option(-101, 0), 1, "M_PURGE");
    assert_eq!(option(-104, 0), 1, "M_PURGE_ALL");
    assert_eq!(option(-204, 0), 1, "heap tagging NONE");
    assert_eq!(option(-204, 1), 0, "heap tagging TBI is refused");
    assert_eq!(option(-203, 0), 0, "zero init on demand is a recorded deviation");
    assert_eq!(option(12345, 0), 0, "unknown option");
}

#[test]
fn bionic_strerror_returns_linux_text_for_every_table_row_and_unknown_text_otherwise() {
    let bridged = libc_fixture();

    for number in 0..COUNT as i32 {
        let pointer = bridged.call("fx_strerror").i32(number).invoke().unwrap().u64() as usize;
        let expected = message(number).map_or_else(|| format!("Unknown error {number}"), str::to_string);
        assert_eq!(pointer_text(pointer), expected, "errno {number}");
    }
    assert_eq!(pointer_text(bridged.call("fx_strerror").i32(11).invoke().unwrap().u64() as usize), "Try again");
    assert_eq!(pointer_text(bridged.call("fx_strerror").i32(999).invoke().unwrap().u64() as usize), "Unknown error 999");
    assert_eq!(pointer_text(bridged.call("fx_strerror").i32(-5).invoke().unwrap().u64() as usize), "Unknown error -5");
}

#[test]
fn bionic_strerror_r_truncates_with_erange_and_formats_unknown_numbers() {
    let bridged = libc_fixture();
    let mut buffer = [0u8; 32];

    let result = bridged.call("fx_strerror_r").i32(2).ptr(address_of(&mut buffer)).int(5).invoke().unwrap().i32();
    assert_eq!((result, c_text(&buffer)), (34, "No s".to_string()));
    let result = bridged.call("fx_strerror_r").i32(2).ptr(address_of(&mut buffer)).int(32).invoke().unwrap().i32();
    assert_eq!((result, c_text(&buffer)), (0, "No such file or directory".to_string()));
    let exact = bridged.call("fx_strerror_r").i32(2).ptr(address_of(&mut buffer)).int(26).invoke().unwrap().i32();
    let one_short = bridged.call("fx_strerror_r").i32(2).ptr(address_of(&mut buffer)).int(25).invoke().unwrap().i32();
    assert_eq!((exact, one_short), (0, 34), "the terminator needs the last byte");
    bridged.call("fx_strerror_r").i32(4242).ptr(address_of(&mut buffer)).int(32).invoke().unwrap();
    assert_eq!(c_text(&buffer), "Unknown error 4242");
}

#[test]
fn bionic_strerror_table_matches_the_text_of_strerror_cpp() {
    let Some(root) = super::bionic_reference::reference_root() else { return };
    let source = std::fs::read_to_string(root.join("bionic/libc/bionic/strerror.cpp")).unwrap();
    let mut rows = 0;
    for line in source.lines().filter(|line| line.trim_start().starts_with('[')) {
        let (name, text) = line.trim().trim_start_matches('[').split_once("] = \"").unwrap();
        let text = text.trim_end_matches("\",");
        let number = if name == "0" { 0 } else { super::bionic_reference::linux_errno_value(&root, name).unwrap_or_else(|| panic!("no number for {name}")) };
        assert_eq!(TABLE[number as usize], Some(text), "{name}");
        rows += 1;
    }
    assert_eq!(rows, TABLE.iter().flatten().count());
}

#[test]
fn errno_translation_maps_host_eagain_to_linux_eagain_and_keeps_shared_numbers() {
    let bridged = libc_fixture();
    let mut ends = [0i32; 2];
    unsafe {
        assert_eq!(libc::pipe(ends.as_mut_ptr()), 0);
        libc::fcntl(ends[0], libc::F_SETFL, libc::O_NONBLOCK);
    }

    let code = bridged.call("fx_errno_after_read").i32(ends[0]).invoke().unwrap().i32();
    let bad = bridged.call("fx_errno_after_read").i32(-1).invoke().unwrap().i32();

    unsafe {
        libc::close(ends[0]);
        libc::close(ends[1]);
    }
    assert_eq!(code, 11, "host EAGAIN is 35, Linux EAGAIN is 11");
    assert_eq!(bad, 9, "EBADF is the same number on both");
    assert_eq!(bridged.call("fx_errno_after_strtol_overflow").invoke().unwrap().i32(), 34);
}

#[test]
fn bionic_environ_starts_with_android_variables_and_no_host_variables() {
    let bridged = libc_fixture();
    let length = |name: &str| {
        let name = CString::new(name).unwrap();
        bridged.call("fx_getenv_length").ptr(name.as_ptr() as usize).invoke().unwrap().i32()
    };

    assert_eq!(length("ANDROID_ROOT"), 7);
    assert_eq!(length("EXTERNAL_STORAGE"), 7);
    assert_eq!(length("PATH") > 20, true);
    assert_eq!(length("HOME"), -1);
    assert_eq!(length("TMPDIR"), -1);
    assert_eq!(length("ANDROID_RO"), -1, "a prefix of a name is not the name");
}

#[test]
fn upstream_openbsd_setenv_rules_for_overwrite_invalid_names_and_unsetenv() {
    let bridged = libc_fixture();
    let set = |name: &str, value: &str, overwrite: i32| {
        let (name, value) = (CString::new(name).unwrap(), CString::new(value).unwrap());
        bridged.call("fx_setenv").ptr(name.as_ptr() as usize).ptr(value.as_ptr() as usize).i32(overwrite).invoke().unwrap().i32()
    };
    let length = |name: &str| {
        let name = CString::new(name).unwrap();
        bridged.call("fx_getenv_length").ptr(name.as_ptr() as usize).invoke().unwrap().i32()
    };

    assert_eq!(set("FX_NEW_VARIABLE", "abc", 1), 0);
    assert_eq!(length("FX_NEW_VARIABLE"), 3);
    assert_eq!(set("FX_NEW_VARIABLE", "longer-value", 0), 0);
    assert_eq!(length("FX_NEW_VARIABLE"), 3, "no overwrite keeps the old value");
    assert_eq!(set("FX_NEW_VARIABLE", "longer-value", 1), 0);
    assert_eq!(length("FX_NEW_VARIABLE"), 12);
    assert_eq!(set("", "x", 1), -1);
    assert_eq!(set("A=B", "x", 1), -1);
    let name = CString::new("FX_NEW_VARIABLE").unwrap();
    assert_eq!(bridged.call("fx_unsetenv").ptr(name.as_ptr() as usize).invoke().unwrap().i32(), 0);
    assert_eq!(length("FX_NEW_VARIABLE"), -1);
}

#[test]
fn upstream_openbsd_putenv_stores_the_callers_pointer_itself() {
    let bridged = libc_fixture();
    let entry: &'static mut [u8; 22] = Box::leak(Box::new(*b"FX_PUT_VARIABLE=value\0"));
    let name = CString::new("FX_PUT_VARIABLE").unwrap();

    let identical = bridged.call("fx_putenv_identity").ptr(address_of(entry)).ptr(name.as_ptr() as usize).invoke().unwrap().i32();
    assert_eq!(bridged.call("fx_unsetenv").ptr(name.as_ptr() as usize).invoke().unwrap().i32(), 0);
    assert_eq!(identical, 1);
}

fn rand_values(seed: u32) -> Vec<i32> {
    let bridged = libc_fixture();
    let mut values = [0i32; 6];
    bridged.call("fx_rand_sequence").u32(seed).ptr(values.as_mut_ptr() as usize).int(6).invoke().unwrap();
    values.to_vec()
}

#[test]
fn upstream_netbsd_random_matches_the_natively_compiled_source_for_special_seeds() {
    assert_eq!(rand_values(0), [2059215011, 2052119187, 2047820401, 968558420, 2033801269, 2028408493]);
    assert_eq!(rand_values(1), [1804289383, 846930886, 1681692777, 1714636915, 1957747793, 424238335]);
    assert_eq!(rand_values(42), [71876166, 708592740, 1483128881, 907283241, 442951012, 537146758]);
    assert_eq!(rand_values(0x7fff_ffff), [977399425, 2046899839, 966903128, 2032571191, 2027351843, 946434570]);
    assert_eq!(rand_values(0xffff_ffff), [254925627, 1205188300, 366127624, 1401405153, 76053476, 1604170158]);
}

#[test]
fn macos_random_differs_from_the_netbsd_source_for_seed_zero_which_is_why_it_is_not_bound() {
    extern "C" {
        fn srandom(seed: u32);
        fn random() -> i64;
    }
    unsafe { srandom(0) };
    let host = unsafe { random() };

    assert_ne!(host as i32, 2059215011);
}

#[test]
fn upstream_netbsd_lrand48_matches_the_macos_lrand48_of_the_same_lineage() {
    let bridged = libc_fixture();
    let mut values = [0i64; 4];

    bridged.call("fx_lrand48_sequence").int(0).ptr(values.as_mut_ptr() as usize).int(4).invoke().unwrap();
    assert_eq!(values, [366850414, 1610402240, 206956554, 1869309841]);
    bridged.call("fx_lrand48_sequence").int(12345).ptr(values.as_mut_ptr() as usize).int(4).invoke().unwrap();
    assert_eq!(values, [483889296, 1973930609, 444188209, 1556452597]);
}

#[test]
fn bionic_assert2_aborts_with_the_file_line_function_and_expression_text() {
    let bridged = libc_fixture();

    let (signal, stderr) = child_outcome(|| {
        let _ = bridged.call("fx_assert_fail").invoke();
    });

    assert_eq!(signal, Some(libc::SIGABRT));
    assert!(stderr.contains("fx_libc.c:42: fx_assert_fail: assertion \"value > 0\" failed\n"), "{stderr}");
}

#[test]
fn bionic_abort_raises_sigabrt() {
    let bridged = libc_fixture();

    let (signal, _) = child_outcome(|| {
        let _ = bridged.call("fx_abort").invoke();
    });

    assert_eq!(signal, Some(libc::SIGABRT));
}

static MARKER_ADDRESS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

extern "C" fn report_exit(status: i32) {
    let marker = unsafe { *(MARKER_ADDRESS.load(std::sync::atomic::Ordering::SeqCst) as *const i32) };
    let line = format!("marker={marker} status={status}\n");
    unsafe {
        libc::write(2, line.as_ptr() as *const libc::c_void, line.len());
        libc::_exit(status);
    }
}

#[test]
fn bionic_exit_runs_exit_handlers_in_reverse_registration_order_then_leaves_with_the_status() {
    let since = super::bionic::atexit::append_count();
    let bridged = libc_fixture();
    MARKER_ADDRESS.store(bridged.symbol("fx_exit_marker"), std::sync::atomic::Ordering::SeqCst);

    let module = bridged.loader.module_info(bridged.handle).unwrap();
    let (signal, code, stderr) = child_result(|| {
        super::bionic::atexit::retain_within(since, module.region, module.region_len);
        super::bionic::atexit::set_exit_function(report_exit);
        let _ = bridged.call("fx_exit_with_atexit").int(3).invoke();
    });

    assert_eq!((signal, code), (None, Some(3)), "{stderr}");
    assert!(stderr.contains("marker=21 status=3\n"), "{stderr}");
}

#[test]
fn fortify_checks_pass_for_in_bounds_calls_and_abort_with_the_bionic_text_otherwise() {
    let bridged = libc_fixture();
    let text = CString::new("hello").unwrap();
    let mut buffer = [0u8; 32];
    let destination = address_of(&mut buffer);
    let source = text.as_ptr() as usize;

    assert_eq!(bridged.call("fx_chk_strlen").ptr(source).int(6).invoke().unwrap().i32(), 5);
    assert_eq!(bridged.call("fx_chk_memcpy").ptr(destination).ptr(source).int(5).int(32).invoke().unwrap().i32(), i32::from(b'h'));
    assert_eq!(bridged.call("fx_chk_strchr").ptr(source).i32(i32::from(b'l')).int(6).invoke().unwrap().i32(), 2);
    assert_eq!(bridged.call("fx_chk_strchr").ptr(source).i32(i32::from(b'z')).int(6).invoke().unwrap().i32(), -1);
    assert_eq!(bridged.call("fx_chk_fd_set").i32(70).int(128).invoke().unwrap().i32(), 1);

    let aborts = |work: &dyn Fn() -> Option<String>| {
        let (signal, stderr) = child_outcome(|| {
            work();
        });
        assert_eq!(signal, Some(libc::SIGABRT), "{stderr}");
        stderr
    };
    let strlen = aborts(&|| bridged.call("fx_chk_strlen").ptr(source).int(5).invoke().ok().map(|_| String::new()));
    assert!(strlen.contains("FORTIFY: strlen: detected read past end of buffer\n"), "{strlen}");
    let memcpy = aborts(&|| bridged.call("fx_chk_memcpy").ptr(destination).ptr(source).int(5).int(4).invoke().ok().map(|_| String::new()));
    assert!(memcpy.contains("FORTIFY: memcpy: prevented 5-byte write into 4-byte buffer\n"), "{memcpy}");
    let huge = aborts(&|| bridged.call("fx_chk_memcpy").ptr(destination).ptr(source).int(u64::MAX).int(4).invoke().ok().map(|_| String::new()));
    assert!(huge.contains(&format!("FORTIFY: memcpy: count {} > SSIZE_MAX\n", u64::MAX)), "{huge}");
    let memset = aborts(&|| bridged.call("fx_chk_memset").ptr(destination).int(9).int(8).invoke().ok().map(|_| String::new()));
    assert!(memset.contains("FORTIFY: memset: prevented 9-byte write into 8-byte buffer\n"), "{memset}");
    let strcpy = aborts(&|| bridged.call("fx_chk_strcpy").ptr(destination).ptr(source).int(5).invoke().ok().map(|_| String::new()));
    assert!(strcpy.contains("FORTIFY: strcpy: prevented 6-byte write into 5-byte buffer\n"), "{strcpy}");
    let strchr = aborts(&|| bridged.call("fx_chk_strchr").ptr(source).i32(i32::from(b'z')).int(3).invoke().ok().map(|_| String::new()));
    assert!(strchr.contains("FORTIFY: strchr: prevented read past end of buffer\n"), "{strchr}");
    let strncpy = aborts(&|| bridged.call("fx_chk_strncpy2").ptr(destination).ptr(source).int(10).int(32).int(3).invoke().ok().map(|_| String::new()));
    assert!(strncpy.contains("FORTIFY: strncpy: detected read past end of 3-byte buffer\n"), "{strncpy}");
    let descriptor = aborts(&|| bridged.call("fx_chk_fd_set").i32(2000).int(128).invoke().ok().map(|_| String::new()));
    assert!(descriptor.contains("FORTIFY: FD_SET: file descriptor 2000 >= FD_SETSIZE 1024\n"), "{descriptor}");
    let small = aborts(&|| bridged.call("fx_chk_fd_set").i32(3).int(64).invoke().ok().map(|_| String::new()));
    assert!(small.contains("FORTIFY: FD_SET: set size 64 is too small to be an fd_set\n"), "{small}");
}

#[test]
fn fortify_strcat_chk_detects_the_overflow_and_appends_in_bounds() {
    let bridged = libc_fixture();
    let (mut buffer, text) = ([0u8; 16], CString::new("abc").unwrap());
    buffer[..2].copy_from_slice(b"xy");
    let destination = address_of(&mut buffer);

    assert_eq!(bridged.call("fx_chk_strcat").ptr(destination).ptr(text.as_ptr() as usize).int(16).invoke().unwrap().i32(), 5);
    let (signal, stderr) = child_outcome(|| {
        let _ = bridged.call("fx_chk_strcat").ptr(destination).ptr(text.as_ptr() as usize).int(6).invoke();
    });
    assert_eq!(signal, Some(libc::SIGABRT));
    assert!(stderr.contains("FORTIFY: strcat: prevented write past end of 0-byte buffer\n"), "{stderr}");
}
