use std::ffi::CString;
use std::sync::Mutex;

use super::libc_core_tests::{address_of, c_text, libc_fixture};

static LOCALE_STATE: Mutex<()> = Mutex::new(());

fn locale_state() -> std::sync::MutexGuard<'static, ()> {
    LOCALE_STATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn upstream_openbsd_ctype_table_has_flag_bits_for_ascii_and_zero_for_high_bytes() {
    let bridged = libc_fixture();
    let flags = |character: i32| {
        bridged
            .call("fx_ctype_table")
            .i32(character)
            .invoke()
            .unwrap()
            .i32()
    };

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
    let guest = |character: i32, which: i32| {
        bridged
            .call("fx_ctype_class")
            .i32(character)
            .i32(which)
            .invoke()
            .unwrap()
            .i32()
    };
    let host: [unsafe extern "C" fn(i32) -> i32; 6] = [
        libc::isalpha,
        libc::isdigit,
        libc::isalnum,
        libc::isspace,
        libc::ispunct,
        libc::isxdigit,
    ];

    for character in -1..=255 {
        for (which, predicate) in host.iter().enumerate() {
            assert_eq!(
                guest(character, which as i32) != 0,
                unsafe { predicate(character) } != 0,
                "predicate {which} for {character}"
            );
        }
        assert_eq!(
            guest(character, 6),
            unsafe { libc::tolower(character) },
            "tolower {character}"
        );
        assert_eq!(
            guest(character, 7),
            unsafe { libc::toupper(character) },
            "toupper {character}"
        );
    }
    assert_eq!(guest(0x141, 0), 0, "values above 255 are never alphabetic");
}

fn set_locale(category: i32, name: &str) -> (i32, String) {
    let bridged = libc_fixture();
    let (name, mut report) = (CString::new(name).unwrap(), [0u8; 32]);
    let code = bridged
        .call("fx_setlocale")
        .i32(category)
        .ptr(name.as_ptr() as usize)
        .ptr(address_of(&mut report))
        .invoke()
        .unwrap()
        .i32();
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
        let value = bridged
            .call("fx_locale_flow")
            .ptr(name.as_ptr() as usize)
            .ptr(&mut code as *mut i32 as usize)
            .invoke()
            .unwrap()
            .u64() as i64;
        (value, code)
    };

    assert_eq!(
        flow("C"),
        (11, 0),
        "one byte inside, previous locale was the global locale"
    );
    assert_eq!(flow("C.UTF-8"), (41, 0));
    assert_eq!(flow("nonexistent"), (-1, 2));
}

#[test]
fn bionic_localeconv_has_a_dot_decimal_point_and_char_max_numeric_fields() {
    let bridged = libc_fixture();
    let (mut point, mut fields) = ([0u8; 8], [0u8; 14]);

    let extra = bridged
        .call("fx_localeconv")
        .ptr(address_of(&mut point))
        .ptr(address_of(&mut fields))
        .invoke()
        .unwrap()
        .i32();

    assert_eq!((c_text(&point), extra), (".".to_string(), 0));
    assert_eq!(fields, [255u8; 14]);
}

const ILLEGAL: u64 = u64::MAX;
const INCOMPLETE: u64 = u64::MAX - 1;

fn decode(bytes: &[u8], count: usize, state: &mut [u8; 8]) -> (u64, u32) {
    let bridged = libc_fixture();
    let mut value = 0u32;
    let mut copy = bytes.to_vec();
    copy.push(0);
    let result = bridged
        .call("fx_mbrtowc")
        .ptr(&mut value as *mut u32 as usize)
        .ptr(copy.as_ptr() as usize)
        .int(count as u64)
        .ptr(state.as_mut_ptr() as usize)
        .invoke()
        .unwrap()
        .u64();
    (result, value)
}

#[test]
fn bionic_mbrtowc_decodes_utf8_boundaries_and_rejects_overlong_surrogate_and_truncated_input() {
    let mut state = [0u8; 8];
    for (bytes, expected) in [
        (&b"\x7f"[..], 0x7f),
        (&b"\xc2\x80"[..], 0x80),
        (&b"\xdf\xbf"[..], 0x7ff),
        (&b"\xe0\xa0\x80"[..], 0x800),
        (&b"\xef\xbf\xbf"[..], 0xffff),
        (&b"\xf0\x90\x80\x80"[..], 0x10000),
        (&b"\xf4\x8f\xbf\xbf"[..], 0x10ffff),
    ] {
        assert_eq!(
            decode(bytes, bytes.len(), &mut state).0,
            bytes.len() as u64,
            "{bytes:x?}"
        );
        assert_eq!(
            decode(bytes, bytes.len(), &mut state).1,
            expected,
            "{bytes:x?}"
        );
    }
    for bytes in [
        &b"\xc0\x80"[..],
        &b"\xed\xa0\x80"[..],
        &b"\xf4\x90\x80\x80"[..],
        &b"\x80"[..],
        &b"\xe0\x80\x80"[..],
    ] {
        assert_eq!(
            decode(bytes, bytes.len(), &mut state).0,
            ILLEGAL,
            "{bytes:x?}"
        );
        assert_eq!(
            state[..4],
            [0, 0, 0, 0],
            "the state is reset after an illegal sequence"
        );
    }
    assert_eq!(
        decode(b"", 0, &mut state).0,
        0,
        "n == 0 returns 0 in Bionic, not -2"
    );
    assert_eq!(decode(b"\x00", 1, &mut state).0, 0);
}

#[test]
fn bionic_mbrtowc_carries_a_partial_sequence_in_the_eight_byte_state_across_calls() {
    let mut state = [0u8; 8];

    assert_eq!(decode(b"\xe2\x82", 2, &mut state).0, INCOMPLETE);
    assert_eq!(state[..3], [0xe2, 0x82, 0]);
    let (result, value) = decode(b"\xac", 1, &mut state);
    assert_eq!(
        (result, value),
        (1, 0x20ac),
        "the euro sign completes with the bytes consumed in this call"
    );
    assert_eq!(state[..4], [0, 0, 0, 0]);
    assert_eq!(
        decode(b"\xe2\x41", 2, &mut state).0,
        ILLEGAL,
        "a non continuation byte in the middle is illegal"
    );
}

#[test]
fn bionic_wcrtomb_encodes_one_to_four_bytes_without_rejecting_surrogates() {
    let bridged = libc_fixture();
    let encode = |value: u32| {
        let (mut buffer, mut state) = ([0u8; 8], [0u8; 8]);
        let length = bridged
            .call("fx_wcrtomb")
            .ptr(buffer.as_mut_ptr() as usize)
            .u32(value)
            .ptr(state.as_mut_ptr() as usize)
            .invoke()
            .unwrap()
            .u64();
        (length, buffer[..length.min(4) as usize].to_vec())
    };

    assert_eq!(encode(0x41), (1, vec![0x41]));
    assert_eq!(encode(0x20ac), (3, vec![0xe2, 0x82, 0xac]));
    assert_eq!(encode(0xd800), (3, vec![0xed, 0xa0, 0x80]));
    assert_eq!(encode(0x1f600), (4, vec![0xf0, 0x9f, 0x98, 0x80]));
    assert_eq!(encode(0x200000).0, ILLEGAL);
}

#[test]
fn bionic_btowc_wctob_mbsinit_mbtowc_wctomb_follow_the_openbsd_wrappers() {
    let bridged = libc_fixture();
    let (mut state, mut value, mut buffer) = ([0u8; 8], 0u32, [0u8; 8]);

    assert_eq!(
        bridged.call("fx_btowc").i32(0x41).invoke().unwrap().u64() as u32,
        0x41
    );
    assert_eq!(
        bridged.call("fx_btowc").i32(0x80).invoke().unwrap().u64() as u32,
        u32::MAX,
        "btowc(0x80) is WEOF"
    );
    assert_eq!(
        bridged.call("fx_btowc").i32(-1).invoke().unwrap().u64() as u32,
        u32::MAX
    );
    assert_eq!(
        bridged.call("fx_wctob").u32(0x41).invoke().unwrap().i32(),
        0x41
    );
    assert_eq!(
        bridged.call("fx_wctob").u32(0x20ac).invoke().unwrap().i32(),
        -1
    );
    assert_eq!(
        bridged
            .call("fx_mbsinit")
            .ptr(state.as_mut_ptr() as usize)
            .invoke()
            .unwrap()
            .i32(),
        1
    );
    state[0] = 0xe2;
    assert_eq!(
        bridged
            .call("fx_mbsinit")
            .ptr(state.as_mut_ptr() as usize)
            .invoke()
            .unwrap()
            .i32(),
        0
    );
    let euro = b"\xe2\x82\xac\0";
    assert_eq!(
        bridged
            .call("fx_mbtowc")
            .ptr(&mut value as *mut u32 as usize)
            .ptr(euro.as_ptr() as usize)
            .int(3)
            .invoke()
            .unwrap()
            .i32(),
        3
    );
    assert_eq!(value, 0x20ac);
    assert_eq!(
        bridged
            .call("fx_wctomb")
            .ptr(buffer.as_mut_ptr() as usize)
            .u32(0x20ac)
            .invoke()
            .unwrap()
            .i32(),
        3
    );
    assert_eq!(&buffer[..3], b"\xe2\x82\xac");
}

#[test]
fn upstream_freebsd_wide_string_functions_compare_unsigned_and_copy_with_padding() {
    let bridged = libc_fixture();

    assert_eq!(bridged.call("fx_wide_strings").invoke().unwrap().i32(), 0);
}
