use std::ffi::CString;
use std::sync::Mutex;

use super::bionic::abi_types::EILSEQ;
use super::bionic::log::is_loggable;
use super::bionic::printf::format;
use super::bridge::{handler_address, BridgeProvider};
use super::bridge_test_support::register_all;
use super::call_assign::CallBuilder;
use super::libc_core_tests::pointer_text;
use super::libc_coverage_tests::{exported_names, registered};
use super::libc_system_tests::LOG_STATE;
use super::link_resolution::SystemLibraryProvider;
use super::loader_test_support::{child_outcome, fixture_dir};
use super::runtime::NativeRuntime;
use super::va_list::{GuestVaList, GuestVaListRaw};

fn handler(name: &str) -> CallBuilder {
    register_all();
    CallBuilder::new(handler_address(name).unwrap_or_else(|| panic!("{name} is not registered")))
}

fn render(format_text: &str, args: &[u64]) -> String {
    let raw = GuestVaListRaw {
        stack: args.as_ptr() as u64,
        gr_top: 0,
        vr_top: 0,
        gr_offs: 0,
        vr_offs: 0,
    };
    let mut list = GuestVaList::from_guest(&raw as *const GuestVaListRaw as u64).unwrap();
    String::from_utf8(format(format_text.as_bytes(), &mut list).unwrap()).unwrap()
}

fn text_argument(text: &str) -> (CString, u64) {
    let owned = CString::new(text).unwrap();
    let address = owned.as_ptr() as u64;
    (owned, address)
}

fn wide(text: &str) -> Vec<u32> {
    text.chars()
        .map(u32::from)
        .chain(std::iter::once(0))
        .collect()
}

fn address<T>(values: &[T]) -> u64 {
    values.as_ptr() as u64
}

fn address_mut<T>(values: &mut [T]) -> u64 {
    values.as_mut_ptr() as u64
}

fn host_errno() -> i32 {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

#[test]
fn bionic_vfprintf_keeps_the_zero_flag_for_strings_and_characters() {
    let (_ab, ab) = text_argument("ab");

    assert_eq!(render("%05s|", &[ab]), "000ab|");
    assert_eq!(render("%05c|", &[u64::from(b'x')]), "0000x|");
    assert_eq!(render("%-05s|", &[ab]), "ab   |");
    assert_eq!(render("%5s|", &[ab]), "   ab|");
    assert_eq!(render("%05.1s|", &[ab]), "0000a|");
}

#[test]
fn bionic_vfprintf_formats_binary_conversions_with_the_alternate_prefix() {
    assert_eq!(render("%b|%B", &[5, 5]), "101|101");
    assert_eq!(render("%#b|%#B|%#b", &[5, 5, 0]), "0b101|0B101|0");
    assert_eq!(
        render("%08b|%#8b|%.5b", &[5, 5, 5]),
        "00000101|   0b101|00101"
    );
    assert_eq!(render("%lb", &[u64::MAX]), "1".repeat(64));
}

#[test]
fn bionic_vfprintf_long_aliases_read_a_64_bit_argument() {
    let wide_value = 0x1_0000_0001u64;

    assert_eq!(render("%D", &[wide_value]), "4294967297");
    assert_eq!(render("%d", &[wide_value]), "1");
    assert_eq!(render("%D", &[(-5i64) as u64]), "-5");
    assert_eq!(render("%U", &[u64::MAX]), "18446744073709551615");
    assert_eq!(render("%O", &[1u64 << 32]), "40000000000");
    assert_eq!(render("%o", &[1u64 << 32]), "0");
}

#[test]
fn bionic_vfprintf_w_length_modifiers_select_the_integer_width() {
    assert_eq!(render("%w8d", &[0x1ff]), "-1");
    assert_eq!(render("%w16d", &[0x1_ffff]), "-1");
    assert_eq!(render("%w32d", &[0x1_0000_0001]), "1");
    assert_eq!(render("%w64d", &[0x1_0000_0001]), "4294967297");
    assert_eq!(render("%w8x", &[0x1ff]), "ff");
    assert_eq!(render("%w16u", &[0x1_0003]), "3");
}

#[test]
fn bionic_vfprintf_w_with_an_unsupported_width_aborts_with_the_fortify_text() {
    let (signal, stderr) = child_outcome(|| {
        render("%w7d", &[1]);
    });

    assert_eq!(signal, Some(libc::SIGABRT));
    assert!(stderr.contains("FORTIFY: %w7 is unsupported\n"), "{stderr}");
}

#[test]
fn bionic_vfprintf_prints_an_unknown_conversion_letter_through_the_width_logic() {
    assert_eq!(render("%y|%5y|%-3y|%5%", &[]), "y|    y|y  |    %");
}

#[test]
fn bionic_vfprintf_wide_aliases_print_utf8_for_wide_characters_and_strings() {
    let wide_text = wide("h\u{e9}");
    let pointer = address(&wide_text);

    assert_eq!(
        render("%C|%lc", &[u64::from('A' as u32), u64::from('B' as u32)]),
        "A|B"
    );
    assert_eq!(render("%S|%ls", &[pointer, pointer]), "h\u{e9}|h\u{e9}");
}

#[test]
fn bionic_vfprintf_precision_on_a_string_reads_at_most_that_many_bytes() {
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
    let base = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            page * 2,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_ANON | libc::MAP_PRIVATE,
            -1,
            0,
        )
    } as usize;
    assert_ne!(base, libc::MAP_FAILED as usize);
    assert_eq!(
        unsafe { libc::mprotect((base + page) as *mut libc::c_void, page, libc::PROT_NONE) },
        0
    );
    let text = base + page - 3;
    unsafe { std::ptr::copy_nonoverlapping(b"abc".as_ptr(), text as *mut u8, 3) };

    let (signal, stderr) = child_outcome(|| {
        let rendered = render("%.3s|%.2s", &[text as u64, text as u64]);
        super::host_log::line(&format!("rendered={rendered}"));
    });

    assert_eq!(signal, None, "{stderr}");
    assert!(stderr.contains("rendered=abc|ab"), "{stderr}");
    unsafe { libc::munmap(base as *mut libc::c_void, page * 2) };
}

#[test]
fn bionic_div_and_ldiv_of_the_minimum_value_by_minus_one_wrap_like_sdiv() {
    let quotient_and_remainder = handler("div").i32(i32::MIN).i32(-1).invoke().unwrap().u64();
    assert_eq!(
        (
            quotient_and_remainder as u32,
            (quotient_and_remainder >> 32) as u32
        ),
        (i32::MIN as u32, 0)
    );

    let (quotient, remainder) = handler("ldiv")
        .int(i64::MIN as u64)
        .int((-1i64) as u64)
        .invoke()
        .unwrap()
        .pair();
    assert_eq!((quotient, remainder), (i64::MIN as u64, 0));

    let ordinary = handler("div").i32(-7).i32(2).invoke().unwrap().u64();
    assert_eq!(
        (ordinary as u32 as i32, (ordinary >> 32) as u32 as i32),
        (-3, -1)
    );
}

#[test]
fn libstdcxx_allocation_functions_allocate_free_and_return_null_only_for_nothrow() {
    let pointer = handler("_Znwm").int(64).invoke().unwrap().u64();
    assert_ne!(pointer, 0);
    unsafe { std::ptr::write_bytes(pointer as *mut u8, 0x5a, 64) };
    handler("_ZdlPv").int(pointer).invoke().unwrap();
    let array = handler("_Znam").int(32).invoke().unwrap().u64();
    assert_ne!(array, 0);
    handler("_ZdaPv").int(array).invoke().unwrap();

    assert_eq!(
        handler("_ZnwmRKSt9nothrow_t")
            .int(u64::MAX)
            .int(0)
            .invoke()
            .unwrap()
            .u64(),
        0
    );
    assert_eq!(
        handler("_ZnamRKSt9nothrow_t")
            .int(u64::MAX)
            .int(0)
            .invoke()
            .unwrap()
            .u64(),
        0
    );
    let small = handler("_ZnwmRKSt9nothrow_t")
        .int(8)
        .int(0)
        .invoke()
        .unwrap()
        .u64();
    assert_ne!(small, 0);
    handler("_ZdlPvRKSt9nothrow_t")
        .int(small)
        .int(0)
        .invoke()
        .unwrap();
}

#[test]
fn libstdcxx_throwing_new_aborts_with_the_bionic_text_when_malloc_fails() {
    for (name, expected) in [
        (
            "_Znwm",
            "new failed to allocate 18446744073709551615 bytes\n",
        ),
        (
            "_Znam",
            "new[] failed to allocate 18446744073709551615 bytes\n",
        ),
    ] {
        let (signal, stderr) = child_outcome(|| {
            let _ = handler(name).int(u64::MAX).invoke();
        });

        assert_eq!(signal, Some(libc::SIGABRT), "{name}: {stderr}");
        assert!(stderr.contains(expected), "{name}: {stderr}");
    }
}

#[test]
fn libstdcxx_pure_virtual_aborts_with_the_bionic_text() {
    let (signal, stderr) = child_outcome(|| {
        let _ = handler("__cxa_pure_virtual").invoke();
    });

    assert_eq!(signal, Some(libc::SIGABRT));
    assert!(
        stderr.contains(
            "Pure virtual function called. Are you calling virtual methods from a destructor?\n"
        ),
        "{stderr}"
    );
}

#[test]
fn libstdcxx_guard_acquire_release_and_abort_follow_the_one_time_construction_states() {
    let mut first = [0u64; 1];
    let mut second = [0u64; 1];
    let acquire = |guard: &mut [u64; 1]| {
        handler("__cxa_guard_acquire")
            .int(guard.as_mut_ptr() as u64)
            .invoke()
            .unwrap()
            .i32()
    };

    assert_eq!(acquire(&mut first), 1);
    handler("__cxa_guard_release")
        .int(first.as_mut_ptr() as u64)
        .invoke()
        .unwrap();
    assert_eq!(first[0] & 1, 1, "the low bit is what compiled code tests");
    assert_eq!(acquire(&mut first), 0);

    assert_eq!(acquire(&mut second), 1);
    handler("__cxa_guard_abort")
        .int(second.as_mut_ptr() as u64)
        .invoke()
        .unwrap();
    assert_eq!(second[0] & 1, 0);
    assert_eq!(acquire(&mut second), 1);
}

#[test]
fn libstdcxx_guard_acquire_waits_while_another_thread_is_constructing() {
    let guard = Box::leak(Box::new([0u64; 1])).as_mut_ptr() as u64;
    assert_eq!(
        handler("__cxa_guard_acquire")
            .int(guard)
            .invoke()
            .unwrap()
            .i32(),
        1
    );

    let waiter = std::thread::spawn(move || {
        handler("__cxa_guard_acquire")
            .int(guard)
            .invoke()
            .unwrap()
            .i32()
    });
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert!(
        !waiter.is_finished(),
        "the second acquire must wait for the release"
    );
    handler("__cxa_guard_release").int(guard).invoke().unwrap();

    assert_eq!(waiter.join().unwrap(), 0);
}

#[test]
fn libstdcxx_map_exports_are_registered_under_libstdcxx_only() {
    register_all();
    let Some(root) = super::bionic_reference::reference_root() else {
        eprintln!("skipped: no android-ref checkout");
        return;
    };
    let exported = exported_names(&root.join("bionic/libc/libstdc++.map.txt"));

    assert!(
        exported.contains("_Znwm")
            && exported.contains("__cxa_guard_acquire")
            && exported.contains("_ZSt7nothrow"),
        "{exported:?}"
    );
    let missing: Vec<&String> = exported
        .iter()
        .filter(|name| !registered("libstdc++.so", name))
        .collect();
    assert!(
        missing.is_empty(),
        "libstdc++.map.txt exports without a registration: {missing:?}"
    );
    let leaked: Vec<&String> = exported
        .iter()
        .filter(|name| registered("libc.so", name))
        .collect();
    assert!(
        leaked.is_empty(),
        "libstdc++ exports visible through libc.so: {leaked:?}"
    );
}

static EXIT_ORDER: Mutex<Vec<u64>> = Mutex::new(Vec::new());

extern "C" fn record_exit(argument: u64) {
    EXIT_ORDER.lock().unwrap().push(argument);
}

#[test]
fn bionic_cxa_finalize_with_a_dso_runs_only_that_dsos_handlers_in_reverse_order() {
    let (first_dso, second_dso) = (0x7e57_0000_1000u64, 0x7e57_0000_2000u64);
    let register = |argument: u64, dso: u64| {
        assert_eq!(
            handler("__cxa_atexit")
                .ptr(record_exit as *const () as usize)
                .int(argument)
                .int(dso)
                .invoke()
                .unwrap()
                .i32(),
            0
        )
    };
    register(1, first_dso);
    register(2, second_dso);
    register(3, first_dso);

    handler("__cxa_finalize").int(first_dso).invoke().unwrap();
    assert_eq!(*EXIT_ORDER.lock().unwrap(), vec![3, 1]);
    handler("__cxa_finalize").int(first_dso).invoke().unwrap();
    assert_eq!(
        *EXIT_ORDER.lock().unwrap(),
        vec![3, 1],
        "finalized handlers never run twice"
    );
    handler("__cxa_finalize").int(second_dso).invoke().unwrap();
    assert_eq!(*EXIT_ORDER.lock().unwrap(), vec![3, 1, 2]);
}

#[test]
fn system_property_long_ro_values_carry_the_long_flag_and_the_legacy_error_text() {
    let value = "x".repeat(100);
    super::bionic::props::set_for_test("ro.repair.long", Some(&value));
    super::bionic::props::set_for_test("ro.repair.short", Some("abc"));
    let (long_name, short_name) = (
        CString::new("ro.repair.long").unwrap(),
        CString::new("ro.repair.short").unwrap(),
    );
    let serial_of = |name: &CString| {
        let info = handler("__system_property_find")
            .ptr(name.as_ptr() as usize)
            .invoke()
            .unwrap()
            .u64();
        assert_ne!(info, 0);
        (
            info,
            handler("__system_property_serial")
                .int(info)
                .invoke()
                .unwrap()
                .u64() as u32,
        )
    };

    let (long_info, long_serial) = serial_of(&long_name);
    let (_, short_serial) = serial_of(&short_name);

    assert_eq!(
        long_serial & (1 << 16),
        1 << 16,
        "kLongFlag is bit 16 of the serial"
    );
    assert_eq!(short_serial & (1 << 16), 0);
    assert_eq!(
        long_serial >> 24,
        50,
        "the legacy error message length is in the top byte"
    );
    assert_eq!(
        pointer_text(long_info as usize + 4),
        "Must use __system_property_read_callback() to read"
    );
    let mut buffer = [0u8; 92];
    let length = handler("__system_property_get")
        .ptr(long_name.as_ptr() as usize)
        .ptr(buffer.as_mut_ptr() as usize)
        .invoke()
        .unwrap()
        .i32();
    assert_eq!(length, 50);
}

struct TagProperties(Vec<String>);

impl TagProperties {
    fn set(&mut self, name: &str, value: &str) {
        super::bionic::props::set_for_test(name, Some(value));
        self.0.push(name.to_string());
    }
}

impl Drop for TagProperties {
    fn drop(&mut self) {
        for name in &self.0 {
            super::bionic::props::set_for_test(name, None);
        }
    }
}

fn loggable(priority: i32, tag: &str) -> bool {
    is_loggable(priority, Some(tag.as_bytes()), 2)
}

#[test]
fn liblog_tag_properties_are_read_in_the_order_log_tag_then_persist_log_tag() {
    let _guard = LOG_STATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut properties = TagProperties(Vec::new());
    properties.set("log.tag.ordera", "E");
    properties.set("persist.log.tag.ordera", "D");
    properties.set("persist.log.tag.orderb", "W");

    assert!(
        !loggable(5, "ordera") && loggable(6, "ordera"),
        "log.tag.<tag> wins over persist.log.tag.<tag>"
    );
    assert!(
        !loggable(4, "orderb") && loggable(5, "orderb"),
        "persist.log.tag.<tag> is used when log.tag.<tag> is missing"
    );
    assert!(
        loggable(2, "untagged") && !loggable(1, "untagged"),
        "no property falls back to the default priority"
    );
}

#[test]
fn liblog_an_invalid_tag_value_falls_back_to_the_global_properties_and_letters_are_case_insensitive(
) {
    let _guard = LOG_STATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut properties = TagProperties(Vec::new());
    properties.set("log.tag.invalid", "Q");
    properties.set("persist.log.tag.invalid", "D");
    properties.set("persist.log.tag", "W");
    properties.set("log.tag.lower", "e");

    assert!(
        !loggable(4, "invalid") && loggable(5, "invalid"),
        "the invalid specific value is skipped for the global one, not for the next specific key"
    );
    assert!(
        !loggable(5, "lower") && loggable(6, "lower"),
        "a lower case letter selects the same priority"
    );
    properties.set("log.tag", "E");
    assert!(
        !loggable(5, "invalid") && loggable(6, "invalid"),
        "log.tag is read before persist.log.tag"
    );
}

#[test]
fn liblog_assert_and_false_values_map_to_fatal_and_silent_priorities() {
    let _guard = LOG_STATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut properties = TagProperties(Vec::new());
    properties.set("log.tag.asserted", "A");
    properties.set("log.tag.fatal", "F");
    properties.set("log.tag.silent", "S");
    properties.set("log.tag.disabled", "false");
    properties.set("log.tag.enabled", "true");

    assert!(!loggable(6, "asserted") && loggable(7, "asserted"));
    assert!(!loggable(6, "fatal") && loggable(7, "fatal"));
    assert!(!loggable(7, "silent") && loggable(8, "silent"));
    assert!(
        !loggable(7, "disabled"),
        "the boolean false is the silent priority"
    );
    assert!(
        loggable(2, "enabled") && !loggable(1, "enabled"),
        "the boolean true is not a priority letter"
    );
}

#[test]
fn liblog_a_null_or_empty_tag_looks_up_the_default_tag_properties() {
    let _guard = LOG_STATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut properties = TagProperties(Vec::new());
    properties.set("log.tag.app_process64", "E");

    for tag in [None, Some(&b""[..])] {
        assert!(!is_loggable(5, tag, 2) && is_loggable(6, tag, 2), "{tag:?}");
    }
}

fn runtime() -> &'static NativeRuntime {
    register_all();
    NativeRuntime::initialize(vec![fixture_dir("16k")]).unwrap()
}

#[test]
fn libdl_android_get_ld_library_path_joins_the_search_directories_and_aborts_when_the_buffer_is_short(
) {
    let directories: Vec<String> = runtime()
        .library_dirs()
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    let joined = directories.join(":");
    let required = joined.len() + 1;
    let mut buffer = vec![0xAAu8; required + 8];
    let address = buffer.as_mut_ptr() as usize;

    let (signal, stderr) = child_outcome(|| {
        handler("android_get_LD_LIBRARY_PATH")
            .ptr(address)
            .int(required as u64)
            .invoke()
            .unwrap();
        let written = unsafe { std::slice::from_raw_parts(address as *const u8, required + 1) };
        super::host_log::line(&format!(
            "written={} next={}",
            String::from_utf8_lossy(&written[..required - 1]),
            written[required]
        ));
    });
    assert_eq!(
        signal, None,
        "a buffer of exactly the required length is accepted: {stderr}"
    );
    assert!(
        stderr.contains(&format!("written={joined} next=170")),
        "{stderr}"
    );
    let (signal, stderr) = child_outcome(|| {
        let _ = handler("android_get_LD_LIBRARY_PATH")
            .ptr(address)
            .int(required as u64 - 1)
            .invoke();
    });
    assert_eq!(signal, Some(libc::SIGABRT), "{stderr}");
    assert!(stderr.contains(&format!("android_get_LD_LIBRARY_PATH failed, buffer too small: buffer len {}, required len {required}\n", required - 1)), "{stderr}");
}

#[test]
fn bionic_wide_class_functions_follow_the_narrow_fallback_when_no_icu_is_loaded() {
    let call = |name: &str, character: u64| {
        i32::from(handler(name).int(character).invoke().unwrap().i32() != 0)
    };

    assert_eq!(call("iswalpha", u64::from(b'q')), 1);
    assert_eq!(
        call("iswalpha", 0x3b1),
        0,
        "wc above 0xff is in no narrow class"
    );
    assert_eq!(
        call("iswalpha", u64::from(u32::MAX)),
        0,
        "WEOF is in no class"
    );
    assert_eq!(call("iswdigit", u64::from(b'7')), 1);
    assert_eq!(call("iswpunct", u64::from(b'!')), 1);
    assert_eq!(call("iswpunct", u64::from(b'a')), 0);
    assert_eq!(call("iswupper", u64::from(b'Q')), 1);
    assert_eq!(call("iswupper", u64::from(b'q')), 0);
    assert_eq!(call("iswlower", u64::from(b'q')), 1);
    assert_eq!(call("iswcntrl", 7), 1);
    assert_eq!(call("iswcntrl", u64::from(b'a')), 0);
    assert_eq!(call("iswspace", u64::from(b'\t')), 1);
    assert_eq!(call("iswprint", u64::from(b' ')), 1);
    assert_eq!(call("iswalnum", u64::from(b'_')), 0);
    assert_eq!(call("iswxdigit", u64::from(b'f')), 1);
    assert_eq!(call("iswblank", u64::from(b'\t')), 1);
    assert_eq!(call("iswgraph", u64::from(b' ')), 0);
    assert_eq!(
        handler("iswalpha_l")
            .int(u64::from(b'a'))
            .int(0)
            .invoke()
            .unwrap()
            .i32(),
        1
    );
    let convert = |name: &str, character: u64| handler(name).int(character).invoke().unwrap().i32();
    assert_eq!(convert("towlower", u64::from(b'Q')), i32::from(b'q'));
    assert_eq!(convert("towupper", u64::from(b'q')), i32::from(b'Q'));
    assert_eq!(convert("towlower", 0xc0), 0xc0);
}

#[test]
fn bionic_wctype_iswctype_wctrans_and_towctrans_use_the_class_numbers_of_wctype_cpp() {
    let class = |name: &str| {
        let (_owned, pointer) = text_argument(name);
        handler("wctype").int(pointer).invoke().unwrap().u64()
    };
    assert_eq!(
        (
            class("alnum"),
            class("alpha"),
            class("xdigit"),
            class("bogus")
        ),
        (1, 2, 12, 0)
    );
    let member = |character: u64, class: u64| {
        handler("iswctype")
            .int(character)
            .int(class)
            .invoke()
            .unwrap()
            .i32()
    };
    assert_eq!(
        (
            member(u64::from(b'a'), 2),
            member(u64::from(b'1'), 2),
            member(u64::from(b'1'), 5),
            member(u64::from(b'a'), 99)
        ),
        (1, 0, 1, 0)
    );

    let (_lower, lower) = text_argument("tolower");
    let (_upper, upper) = text_argument("toupper");
    let (_other, other) = text_argument("tosomething");
    let translation = |name: u64| handler("wctrans").int(name).invoke().unwrap().u64();
    assert_eq!(
        (translation(lower), translation(upper), translation(other)),
        (1, 2, 0)
    );
    let translate = |character: u64, kind: u64| {
        handler("towctrans")
            .int(character)
            .int(kind)
            .invoke()
            .unwrap()
            .i32()
    };
    assert_eq!(
        (translate(u64::from(b'A'), 1), translate(u64::from(b'a'), 2)),
        (i32::from(b'a'), i32::from(b'A'))
    );
    assert_eq!(translate(u64::from(b'A'), 7), 0);
    assert_eq!(host_errno(), libc::EINVAL);
}

#[test]
fn bionic_wcsstr_wcscasecmp_and_wcsxfrm_follow_the_freebsd_and_openbsd_sources() {
    let haystack = wide("hello world");
    let find = |needle: &str| {
        handler("wcsstr")
            .int(address(&haystack))
            .int(address(&wide(needle)))
            .invoke()
            .unwrap()
            .u64()
    };
    assert_eq!(find("o w"), address(&haystack) + 16);
    assert_eq!(find("d"), address(&haystack) + 40);
    assert_eq!(find(""), address(&haystack));
    assert_eq!(find("worlds"), 0);
    assert_eq!(find("x"), 0);

    let folded = |left: &str, right: &str| {
        handler("wcscasecmp")
            .int(address(&wide(left)))
            .int(address(&wide(right)))
            .invoke()
            .unwrap()
            .i32()
    };
    assert_eq!(folded("HeLLo", "hello"), 0);
    assert_eq!(folded("a", "b"), -1);
    assert_eq!(folded("abc", "ab"), i32::from(b'c'));
    assert_eq!(folded("ab", "abc"), -i32::from(b'c'));
    let limited = |left: &str, right: &str, count: u64| {
        handler("wcsncasecmp")
            .int(address(&wide(left)))
            .int(address(&wide(right)))
            .int(count)
            .invoke()
            .unwrap()
            .i32()
    };
    assert_eq!(
        (
            limited("abcX", "ABCY", 3),
            limited("abcX", "ABCY", 4),
            limited("abc", "xyz", 0)
        ),
        (0, -1, 0)
    );

    let source = wide("abcdef");
    let mut destination = [0xffff_ffffu32; 8];
    let total = handler("wcsxfrm")
        .int(address_mut(&mut destination))
        .int(address(&source))
        .int(4)
        .invoke()
        .unwrap()
        .u64();
    assert_eq!(
        (total, &destination[..5]),
        (
            6,
            &[
                u32::from(b'a'),
                u32::from(b'b'),
                u32::from(b'c'),
                0,
                0xffff_ffff
            ][..]
        )
    );
    assert_eq!(
        handler("wcsxfrm")
            .int(0)
            .int(address(&source))
            .int(0)
            .invoke()
            .unwrap()
            .u64(),
        6
    );
}

#[test]
fn bionic_mbsrtowcs_and_mbstowcs_convert_utf8_and_report_progress_through_src() {
    let utf8 = CString::new("a\u{e9}\u{20ac}\u{1f600}").unwrap();
    let mut source = utf8.as_ptr() as u64;
    let measured = handler("mbsrtowcs")
        .int(0)
        .int(&mut source as *mut u64 as u64)
        .int(0)
        .int(0)
        .invoke()
        .unwrap()
        .u64();
    assert_eq!(
        (measured, source),
        (4, utf8.as_ptr() as u64),
        "measuring leaves src alone"
    );

    let mut destination = [0xffff_ffffu32; 8];
    let converted = handler("mbsrtowcs")
        .int(address_mut(&mut destination))
        .int(&mut source as *mut u64 as u64)
        .int(8)
        .int(0)
        .invoke()
        .unwrap()
        .u64();
    assert_eq!((converted, source), (4, 0));
    assert_eq!(
        &destination[..6],
        &[0x61, 0xe9, 0x20ac, 0x1f600, 0, 0xffff_ffff]
    );

    let mut source = utf8.as_ptr() as u64;
    let mut partial = [0u32; 8];
    let converted = handler("mbsrtowcs")
        .int(address_mut(&mut partial))
        .int(&mut source as *mut u64 as u64)
        .int(2)
        .int(0)
        .invoke()
        .unwrap()
        .u64();
    assert_eq!(
        (converted, source - utf8.as_ptr() as u64),
        (2, 3),
        "two characters take three bytes"
    );

    let whole = handler("mbstowcs")
        .int(address_mut(&mut partial))
        .int(utf8.as_ptr() as u64)
        .int(8)
        .invoke()
        .unwrap()
        .u64();
    assert_eq!(
        (whole, &partial[..4]),
        (4, &[0x61, 0xe9, 0x20ac, 0x1f600][..])
    );
    assert_eq!(
        handler("mbstowcs")
            .int(0)
            .int(utf8.as_ptr() as u64)
            .int(0)
            .invoke()
            .unwrap()
            .u64(),
        4
    );
}

#[test]
fn bionic_mbsrtowcs_rejects_an_illegal_sequence_with_eilseq_and_points_src_at_it() {
    let bytes = CString::new(vec![b'a', b'b', 0xff, b'c']).unwrap();
    let mut source = bytes.as_ptr() as u64;
    let mut destination = [0u32; 8];

    let result = handler("mbsrtowcs")
        .int(address_mut(&mut destination))
        .int(&mut source as *mut u64 as u64)
        .int(8)
        .int(0)
        .invoke()
        .unwrap()
        .u64();

    assert_eq!(result, u64::MAX);
    assert_eq!(host_errno(), EILSEQ);
    assert_eq!(source - bytes.as_ptr() as u64, 2);
    assert_eq!(&destination[..2], &[0x61, 0x62]);
}

#[test]
fn bionic_wcsrtombs_and_wcstombs_encode_utf8_and_stop_before_a_character_that_does_not_fit() {
    let text = wide("a\u{e9}\u{20ac}");
    let mut source = address(&text);
    let measured = handler("wcsrtombs")
        .int(0)
        .int(&mut source as *mut u64 as u64)
        .int(0)
        .int(0)
        .invoke()
        .unwrap()
        .u64();
    assert_eq!(measured, 6);
    assert_eq!(
        handler("wcsnrtombs")
            .int(0)
            .int(&mut source as *mut u64 as u64)
            .int(2)
            .int(0)
            .int(0)
            .invoke()
            .unwrap()
            .u64(),
        3
    );

    let mut output = [0xAAu8; 16];
    let written = handler("wcstombs")
        .int(address_mut(&mut output))
        .int(address(&text))
        .int(4)
        .invoke()
        .unwrap()
        .u64();
    assert_eq!(
        (written, &output[..4]),
        (3, &b"a\xc3\xa9\xAA"[..]),
        "the euro sign needs three bytes and one is left"
    );

    let mut source = address(&text);
    let full = handler("wcsrtombs")
        .int(address_mut(&mut output))
        .int(&mut source as *mut u64 as u64)
        .int(16)
        .int(0)
        .invoke()
        .unwrap()
        .u64();
    assert_eq!(
        (full, source, &output[..7]),
        (6, 0, &b"a\xc3\xa9\xe2\x82\xac\0"[..])
    );
}

#[test]
fn bionic_wcsrtombs_rejects_a_code_point_above_the_utf8_range() {
    let text = [u32::from(b'a'), 0x20_0000, 0];
    let mut source = address(&text);
    let mut output = [0u8; 8];

    let result = handler("wcsrtombs")
        .int(address_mut(&mut output))
        .int(&mut source as *mut u64 as u64)
        .int(8)
        .int(0)
        .invoke()
        .unwrap()
        .u64();

    assert_eq!(result, u64::MAX);
    assert_eq!(host_errno(), EILSEQ);
    assert_eq!(source - address(&text), 4);
    assert_eq!(output[0], b'a');
}

#[test]
fn bionic_wcstod_and_wcstof_skip_wide_spaces_parse_ascii_and_set_the_end_pointer() {
    let text = wide("  12.5abc");
    let mut end = 0u64;

    let value = handler("wcstod")
        .int(address(&text))
        .int(&mut end as *mut u64 as u64)
        .invoke()
        .unwrap()
        .f64();
    assert_eq!((value, end - address(&text)), (12.5, 6 * 4));

    let exponent = wide("1e2");
    let single = handler("wcstof")
        .int(address(&exponent))
        .int(0)
        .invoke()
        .unwrap()
        .f32();
    assert_eq!(single, 100.0);

    let none = wide("  abc");
    let zero = handler("wcstod")
        .int(address(&none))
        .int(&mut end as *mut u64 as u64)
        .invoke()
        .unwrap()
        .f64();
    assert_eq!(
        (zero, end),
        (0.0, address(&none)),
        "no conversion points the end back at the original string"
    );

    let huge = wide("1e999");
    let infinite = handler("wcstod")
        .int(address(&huge))
        .int(0)
        .invoke()
        .unwrap()
        .f64();
    assert_eq!((infinite, host_errno()), (f64::INFINITY, libc::ERANGE));
}

#[test]
fn bionic_nl_langinfo_answers_the_c_locale_tables_and_the_codeset_follows_mb_cur_max() {
    let item = |number: i32| {
        pointer_text(handler("nl_langinfo").i32(number).invoke().unwrap().u64() as usize)
    };

    assert_eq!(
        (
            item(2).as_str(),
            item(3).as_str(),
            item(6).as_str(),
            item(8).as_str(),
            item(21).as_str()
        ),
        ("%F %T %z", "%F", "AM", "Sunday", "Sat")
    );
    assert_eq!(
        (
            item(22).as_str(),
            item(45).as_str(),
            item(51).as_str(),
            item(53).as_str(),
            item(54).as_str()
        ),
        ("January", "Dec", ".", "^[yY]", "^[nN]")
    );
    assert_eq!(
        (item(46), item(55), item(999), item(-1)),
        (String::new(), String::new(), String::new(), String::new())
    );

    let (_c, c_name) = text_argument("C");
    let (_utf8, utf8_name) = text_argument("C.UTF-8");
    let locale = |name: u64| {
        handler("newlocale")
            .i32(0x1fff)
            .int(name)
            .int(0)
            .invoke()
            .unwrap()
            .u64()
    };
    let (byte_locale, utf8_locale) = (locale(c_name), locale(utf8_name));
    assert_ne!((byte_locale, utf8_locale), (0, 0));
    let previous = handler("uselocale")
        .int(byte_locale)
        .invoke()
        .unwrap()
        .u64();
    assert_eq!(item(1), "ASCII");
    handler("uselocale").int(utf8_locale).invoke().unwrap();
    assert_eq!(item(1), "UTF-8");
    handler("uselocale").int(previous).invoke().unwrap();
}

#[test]
fn bionic_sf_array_and_the_std_stream_pointers_are_laid_out_like_stdio_cpp() {
    register_all();
    let provider = BridgeProvider;
    let files = provider
        .export("libc.so", "__sF")
        .expect("__sF is exported by libc.so");
    let field = |index: usize, offset: usize| unsafe {
        std::ptr::read((files + index * 152 + offset) as *const i32)
    };

    assert_eq!(
        [
            (field(0, 16), field(0, 20)),
            (field(1, 16), field(1, 20)),
            (field(2, 16), field(2, 20))
        ],
        [(4, 0), (8, 1), (10, 2)]
    );
    for (index, name) in ["stdin", "stdout", "stderr"].into_iter().enumerate() {
        let cell = provider
            .export("libc.so", name)
            .unwrap_or_else(|| panic!("{name}"));
        assert_eq!(
            unsafe { std::ptr::read(cell as *const usize) },
            files + index * 152,
            "{name}"
        );
    }
}

#[test]
fn bionic_remquo_and_remquof_return_the_remainder_and_write_the_signed_quotient_bits() {
    let mut quotient = 0i32;
    let remainder = handler("remquo")
        .f64(10.0)
        .f64(3.0)
        .ptr(&mut quotient as *mut i32 as usize)
        .invoke()
        .unwrap()
        .f64();
    assert_eq!((remainder, quotient), (1.0, 3));

    let mut quotient = 0i32;
    let remainder = handler("remquof")
        .f32(-7.5)
        .f32(2.0)
        .ptr(&mut quotient as *mut i32 as usize)
        .invoke()
        .unwrap()
        .f32();
    assert_eq!((remainder, quotient), (0.5, -4));
}

#[test]
fn liblog_default_buffer_is_stored_as_main_by_the_logd_logger_and_not_by_the_writer() {
    let _guard = LOG_STATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    super::bionic::log_sink::start_capture();
    let (tag, message) = (text_argument("repairbuf"), text_argument("message"));

    let result = handler("__android_log_buf_write")
        .i32(0x7FFF_FFFF)
        .i32(4)
        .int(tag.1)
        .int(message.1)
        .invoke()
        .unwrap()
        .i32();

    let records = super::bionic::log_sink::take_capture();
    assert_eq!(
        (
            result,
            records.len(),
            records[0].buffer,
            records[0].message.as_str()
        ),
        (1, 1, 0, "message")
    );
}

#[test]
fn bionic_basename_and_dirname_follow_libgen_cpp_for_slashes_and_empty_paths() {
    let call = |name: &str, path: Option<&str>| {
        let owned = path.map(|text| CString::new(text).unwrap());
        let pointer = owned.as_ref().map_or(0, |text| text.as_ptr() as u64);
        let result = handler(name).int(pointer).invoke().unwrap().u64();
        assert_ne!(result, 0, "{name}({path:?})");
        pointer_text(result as usize)
    };
    let table = [
        (Some("/usr/lib"), "lib", "/usr"),
        (Some("/usr/"), "usr", "/"),
        (Some("usr"), "usr", "."),
        (Some("/"), "/", "/"),
        (Some("//"), "/", "/"),
        (Some("."), ".", "."),
        (Some(""), ".", "."),
        (None, ".", "."),
        (Some("a//b"), "b", "a"),
        (Some("a/b/"), "b", "a"),
        (Some("///a///"), "a", "/"),
    ];
    for (path, base, directory) in table {
        assert_eq!(call("basename", path), base, "basename({path:?})");
        assert_eq!(call("dirname", path), directory, "dirname({path:?})");
    }
}

#[test]
fn bionic_basename_fails_with_erange_and_dirname_with_enametoolong_for_oversized_paths() {
    let long = CString::new("a".repeat(5000)).unwrap();

    let basename = handler("basename")
        .int(long.as_ptr() as u64)
        .invoke()
        .unwrap()
        .u64();
    assert_eq!((basename, host_errno()), (0, 34));
    let dirname = handler("dirname")
        .int(long.as_ptr() as u64)
        .invoke()
        .unwrap()
        .u64();
    assert_eq!(
        pointer_text(dirname as usize),
        ".",
        "a path without a slash has the directory ."
    );
    let with_directory = CString::new(format!("{}/b", "a".repeat(5000))).unwrap();
    let dirname = handler("dirname")
        .int(with_directory.as_ptr() as u64)
        .invoke()
        .unwrap()
        .u64();
    assert_eq!((dirname, host_errno()), (0, 36));
}
