use std::ffi::CString;
use std::sync::Mutex;

use super::libc_core_tests::{address_of, c_text, libc_fixture};

static LOCALE_STATE: Mutex<()> = Mutex::new(());

fn locale_state() -> std::sync::MutexGuard<'static, ()> {
    LOCALE_STATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn upstream_openbsd_ctype_table_has_flag_bits_for_ascii_and_zero_for_high_bytes() {
    let bridged = libc_fixture();
    let flags = |character: i32| bridged.call("fx_ctype_table").i32(character).invoke().unwrap().i32();

    assert_eq!(flags(-1), 0, "the EOF slot");
    assert_eq!(flags(0), 0x20);
    assert_eq!(flags(9), 0x28);
    assert_eq!(flags(32), 0x88);
    assert_eq!(flags(i32::from(b'0')), 0x04);
    assert_eq!(flags(i32::from(b'A')), 0x41);
    assert_eq!(flags(i32::from(b'G')), 0x01);
    assert_eq!(flags(i32::from(b'a')), 0x42);
    assert_eq!(flags(i32::from(b'z')), 0x02);
    assert_eq!(flags(i32::from(b'~')), 0x10);
    assert_eq!(flags(127), 0x20);
    assert_eq!(flags(128), 0);
    assert_eq!(flags(255), 0);
}

#[test]
fn bionic_ctype_predicates_agree_with_the_macos_c_locale_for_every_byte_and_eof() {
    let bridged = libc_fixture();
    let guest = |character: i32, which: i32| bridged.call("fx_ctype_class").i32(character).i32(which).invoke().unwrap().i32();
    let host: [unsafe extern "C" fn(i32) -> i32; 6] = [libc::isalpha, libc::isdigit, libc::isalnum, libc::isspace, libc::ispunct, libc::isxdigit];

    for character in -1..=255 {
        for (which, predicate) in host.iter().enumerate() {
            assert_eq!(guest(character, which as i32) != 0, unsafe { predicate(character) } != 0, "predicate {which} for {character}");
        }
        assert_eq!(guest(character, 6), unsafe { libc::tolower(character) }, "tolower {character}");
        assert_eq!(guest(character, 7), unsafe { libc::toupper(character) }, "toupper {character}");
    }
    assert_eq!(guest(0x141, 0), 0, "values above 255 are never alphabetic");
}

fn set_locale(category: i32, name: &str) -> (i32, String) {
    let bridged = libc_fixture();
    let (name, mut report) = (CString::new(name).unwrap(), [0u8; 32]);
    let code = bridged.call("fx_setlocale").i32(category).ptr(name.as_ptr() as usize).ptr(address_of(&mut report)).invoke().unwrap().i32();
    (code, c_text(&report))
}

#[test]
fn bionic_setlocale_accepts_the_supported_names_and_reports_the_utf8_state() {
    let _guard = locale_state();
    let bridged = libc_fixture();

    assert_eq!(set_locale(6, "C"), (0, "C".to_string()));
    assert_eq!(bridged.call("fx_mb_cur_max").invoke().unwrap().u64(), 1);
    assert_eq!(set_locale(0, "en_US.UTF-8"), (0, "C.UTF-8".to_string()));
    assert_eq!(bridged.call("fx_mb_cur_max").invoke().unwrap().u64(), 4);
    assert_eq!(set_locale(6, "xx_YY"), (2, String::new()));
    assert_eq!(set_locale(99, "C"), (22, String::new()));
    assert_eq!(set_locale(6, ""), (0, "C.UTF-8".to_string()));
}

#[test]
fn bionic_newlocale_and_uselocale_scope_the_multibyte_width_to_the_calling_thread() {
    let _guard = locale_state();
    let bridged = libc_fixture();
    let flow = |name: &str| {
        let (name, mut code) = (CString::new(name).unwrap(), 0i32);
        let value = bridged.call("fx_locale_flow").ptr(name.as_ptr() as usize).ptr(&mut code as *mut i32 as usize).invoke().unwrap().u64() as i64;
        (value, code)
    };

    assert_eq!(flow("C"), (11, 0), "one byte inside, previous locale was the global locale");
    assert_eq!(flow("C.UTF-8"), (41, 0));
    assert_eq!(flow("nonexistent"), (-1, 2));
}

#[test]
fn bionic_localeconv_has_a_dot_decimal_point_and_char_max_numeric_fields() {
    let bridged = libc_fixture();
    let (mut point, mut fields) = ([0u8; 8], [0u8; 14]);

    let extra = bridged.call("fx_localeconv").ptr(address_of(&mut point)).ptr(address_of(&mut fields)).invoke().unwrap().i32();

    assert_eq!((c_text(&point), extra), (".".to_string(), 0));
    assert_eq!(fields, [255u8; 14]);
}
