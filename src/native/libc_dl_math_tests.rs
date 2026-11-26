use super::bionic::log_sink::start_capture;
use super::bridge::handler_address;
use super::bridge_test_support::{register_all, Bridged};
use super::dl_api::DlHandle;
use super::libc_core_tests::{address_of, c_text};
use super::loader_test_support::fixture_dir;
use super::runtime::NativeRuntime;

fn runtime() -> &'static NativeRuntime {
    register_all();
    NativeRuntime::initialize(vec![fixture_dir("16k")]).unwrap()
}

fn dl_function(name: &str) -> usize {
    let runtime = runtime();
    let handle = runtime
        .loader()
        .dlopen(Some("libfx_dlmain.so"), 2, None)
        .unwrap();
    runtime
        .loader()
        .dlsym(DlHandle::Module(handle), name, None)
        .unwrap()
}

fn call(name: &str, args: &[u64]) -> i32 {
    start_capture();
    runtime().call_guest(dl_function(name), args).unwrap().i32()
}

fn call_text(name: &str) -> (i32, String) {
    let mut message = [0u8; 600];
    let result = call(name, &[address_of(&mut message) as u64]);
    (result, c_text(&message))
}

#[test]
fn bionic_dlopen_dlsym_dladdr_and_dlclose_load_a_second_fixture_from_guest_code() {
    assert_eq!(call("fx_dl_open_call_close", &[2, 3]), 5);
}

#[test]
fn bionic_dlerror_reports_a_missing_library_once_and_then_returns_null() {
    assert_eq!(
        call_text("fx_dl_missing_library"),
        (
            0,
            "dlopen failed: library \"libnope.so\" not found".to_string()
        )
    );
}

#[test]
fn bionic_do_dlopen_rejects_invalid_flags_with_the_hex_flag_text() {
    assert_eq!(
        call_text("fx_dl_invalid_flags"),
        (
            0,
            "dlopen failed: invalid flags to dlopen: 40000000".to_string()
        )
    );
}

#[test]
fn bionic_dlsym_reports_undefined_symbols_and_invalid_handles_without_a_prefix() {
    assert_eq!(
        call_text("fx_dl_missing_symbol"),
        (0, "undefined symbol: no_such_symbol".to_string())
    );
    assert_eq!(
        call_text("fx_dl_bad_handle"),
        (-1, "dlsym failed: invalid handle: 0x1234".to_string())
    );
}

#[test]
fn bionic_dlsym_of_a_system_library_handle_answers_only_that_librarys_exports() {
    let function = dl_function("fx_dl_system_symbol");
    let symbol = |library: &str, name: &str| {
        let (library, name) = (
            std::ffi::CString::new(library).unwrap(),
            std::ffi::CString::new(name).unwrap(),
        );
        runtime()
            .call_guest(function, &[library.as_ptr() as u64, name.as_ptr() as u64])
            .unwrap()
            .u64() as usize
    };

    assert_eq!(symbol("libm.so", "sin"), handler_address("sin").unwrap());
    assert_eq!(
        symbol("libm.so", "malloc"),
        0,
        "malloc is exported by libc.so, not libm.so"
    );
    assert_eq!(
        symbol("libc.so", "malloc"),
        handler_address("malloc").unwrap()
    );
    assert_eq!(symbol("libc.so", "sin"), 0);
    assert_eq!(
        symbol("libdl.so", "dlopen"),
        handler_address("dlopen").unwrap()
    );
    assert_eq!(
        symbol("liblog.so", "__android_log_print"),
        handler_address("__android_log_print").unwrap()
    );
}

#[test]
fn bionic_dlsym_rtld_default_searches_the_global_group_including_system_libraries() {
    let function = dl_function("fx_dl_default_symbol");
    let lookup = |name: &str| {
        let name = std::ffi::CString::new(name).unwrap();
        runtime()
            .call_guest(function, &[name.as_ptr() as u64])
            .unwrap()
            .u64() as usize
    };

    assert_eq!(lookup("strlen"), handler_address("strlen").unwrap());
    assert_eq!(
        lookup("add"),
        0,
        "a library that was never opened is not in the global group"
    );
    assert_eq!(
        lookup("ANativeWindow_lock"),
        0,
        "unimplemented names are absent, not stubbed"
    );
}

#[test]
fn bionic_dl_iterate_phdr_walks_every_module_and_stops_on_a_nonzero_callback_result() {
    let all = call("fx_dl_iterate_count", &[0]);
    let stopped = call("fx_dl_iterate_count", &[3]);

    assert!(all >= 14, "13 system modules plus the fixture: {all}");
    assert_eq!(stopped, 5);
    assert_eq!(call("fx_dl_counted", &[]), 3);
}

fn math() -> Bridged {
    Bridged::load("16k", "libfx_math.so")
}

fn within(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= expected.abs() * 1e-15 + 1e-300,
        "{actual} vs {expected}"
    );
}

#[test]
fn libm_double_functions_forward_to_the_host_with_aapcs64_register_classes() {
    let bridged = math();

    within(
        bridged.call("fx_sin").f64(0.5).invoke().unwrap().f64(),
        0.479425538604203,
    );
    within(
        bridged
            .call("fx_pow")
            .f64(2.0)
            .f64(10.0)
            .invoke()
            .unwrap()
            .f64(),
        1024.0,
    );
    within(
        bridged
            .call("fx_atan2")
            .f64(1.0)
            .f64(1.0)
            .invoke()
            .unwrap()
            .f64(),
        std::f64::consts::FRAC_PI_4,
    );
    within(
        bridged
            .call("fx_hypot")
            .f64(3.0)
            .f64(4.0)
            .invoke()
            .unwrap()
            .f64(),
        5.0,
    );
    assert_eq!(
        bridged
            .call("fx_powf")
            .f32(2.0)
            .f32(3.0)
            .invoke()
            .unwrap()
            .f32(),
        8.0
    );
    assert_eq!(
        bridged.call("fx_sqrtf").f32(2.25).invoke().unwrap().f32(),
        1.5
    );
    assert_eq!(
        bridged
            .call("fx_fma")
            .f64(2.0)
            .f64(3.0)
            .f64(4.0)
            .invoke()
            .unwrap()
            .f64(),
        10.0
    );
    assert_eq!(
        bridged
            .call("fx_ldexp")
            .f64(1.5)
            .i32(4)
            .invoke()
            .unwrap()
            .f64(),
        24.0
    );
    assert_eq!(
        bridged
            .call("fx_scalbln")
            .f64(1.0)
            .int(100)
            .invoke()
            .unwrap()
            .f64(),
        2f64.powi(100)
    );
    assert_eq!(
        bridged
            .call("fx_fmax")
            .f64(f64::NAN)
            .f64(1.0)
            .invoke()
            .unwrap()
            .f64(),
        1.0
    );
    assert_eq!(
        bridged.call("fx_round").f64(-2.5).invoke().unwrap().f64(),
        -3.0
    );
    assert_eq!(
        bridged.call("fx_lrint").f64(2.5).invoke().unwrap().u64() as i64,
        2
    );
    assert_eq!(
        bridged.call("fx_llround").f64(-2.5).invoke().unwrap().u64() as i64,
        -3
    );
}

#[test]
fn bionic_ilogb_uses_the_bionic_constants_where_the_host_returns_int_min() {
    let bridged = math();
    let ilogb = |value: f64| bridged.call("fx_ilogb").f64(value).invoke().unwrap().i32();

    assert_eq!(ilogb(0.0), -0x7FFF_FFFF);
    assert_eq!(ilogb(f64::NAN), 0x7FFF_FFFF);
    assert_eq!(ilogb(f64::INFINITY), 0x7FFF_FFFF);
    assert_eq!(ilogb(8.0), 3);
    assert_eq!(ilogb(5e-324), -1074);
    assert_eq!(
        bridged.call("fx_ilogbf").f32(0.0).invoke().unwrap().i32(),
        -0x7FFF_FFFF
    );
    let system = unsafe { libc::dlopen(c"/usr/lib/libSystem.B.dylib".as_ptr(), libc::RTLD_NOW) };
    let symbol = unsafe { libc::dlsym(system, c"ilogb".as_ptr()) };
    let host_ilogb: extern "C" fn(f64) -> i32 = unsafe { std::mem::transmute(symbol) };
    assert_eq!(
        host_ilogb(std::hint::black_box(0.0)),
        i32::MIN,
        "macOS FP_ILOGB0 is INT_MIN, which is why ilogb is adapted"
    );
}

#[test]
fn bionic_nan_places_hex_digits_as_a_payload_ored_into_the_quiet_nan_bits() {
    let bridged = math();
    let nan = |text: &str| {
        let text = std::ffi::CString::new(text).unwrap();
        bridged
            .call("fx_nan")
            .ptr(text.as_ptr() as usize)
            .invoke()
            .unwrap()
            .f64()
            .to_bits()
    };

    assert_eq!(nan(""), 0x7ff8_0000_0000_0000);
    assert_eq!(nan("0xabc"), 0x7ff8_0000_0000_0abc);
    assert_eq!(nan("abc"), 0x7ff8_0000_0000_0abc);
    assert_eq!(
        nan("fffffffffffffffff"),
        u64::MAX,
        "digits are ORed, so the sign and exponent bits are set too"
    );
    let nanf = std::ffi::CString::new("0x7fffff").unwrap();
    assert_eq!(
        bridged
            .call("fx_nanf")
            .ptr(nanf.as_ptr() as usize)
            .invoke()
            .unwrap()
            .f32()
            .to_bits(),
        0x7fff_ffff
    );
}

#[test]
fn bionic_lgamma_stores_the_sign_in_signgam_and_lgamma_r_in_the_out_parameter() {
    let bridged = math();
    let (mut sign, mut sign_r) = (0i32, 0i32);

    within(
        bridged
            .call("fx_lgamma")
            .f64(-0.5)
            .ptr(&mut sign as *mut i32 as usize)
            .invoke()
            .unwrap()
            .f64(),
        1.2655121234846454,
    );
    within(
        bridged
            .call("fx_lgamma_r")
            .f64(-0.5)
            .ptr(&mut sign_r as *mut i32 as usize)
            .invoke()
            .unwrap()
            .f64(),
        1.2655121234846454,
    );

    assert_eq!((sign, sign_r), (-1, -1));
    bridged
        .call("fx_lgamma")
        .f64(0.5)
        .ptr(&mut sign as *mut i32 as usize)
        .invoke()
        .unwrap();
    assert_eq!(sign, 1);
}

#[test]
fn bionic_frexp_modf_and_sincos_write_their_pointer_results() {
    let bridged = math();
    let (mut exponent, mut whole, mut sine, mut cosine) = (0i32, 0f64, 0f64, 0f64);

    assert_eq!(
        bridged
            .call("fx_frexp")
            .f64(8.0)
            .ptr(&mut exponent as *mut i32 as usize)
            .invoke()
            .unwrap()
            .f64(),
        0.5
    );
    assert_eq!(exponent, 4);
    assert_eq!(
        bridged
            .call("fx_modf")
            .f64(-3.75)
            .ptr(&mut whole as *mut f64 as usize)
            .invoke()
            .unwrap()
            .f64(),
        -0.75
    );
    assert_eq!(whole, -3.0);
    bridged
        .call("fx_sincos")
        .f64(0.0)
        .ptr(&mut sine as *mut f64 as usize)
        .ptr(&mut cosine as *mut f64 as usize)
        .invoke()
        .unwrap();
    assert_eq!((sine, cosine), (0.0, 1.0));
}

#[test]
fn fenv_arm_rounding_modes_use_the_raw_fpcr_field_values() {
    let bridged = math();
    let round_trip = |mode: i32| {
        bridged
            .call("fx_round_trip")
            .i32(mode)
            .invoke()
            .unwrap()
            .i32()
    };

    assert_eq!(round_trip(0), 0);
    assert_eq!(round_trip(1), 1, "FE_UPWARD is 1 where macOS uses 0x400000");
    assert_eq!(round_trip(2), 2);
    assert_eq!(round_trip(3), 3);
    assert_eq!(bridged.call("fx_round_invalid").invoke().unwrap().i32(), -1);
    assert_eq!(
        bridged
            .call("fx_round_effect")
            .i32(2)
            .f64(2.5)
            .invoke()
            .unwrap()
            .f64(),
        2.0
    );
    assert_eq!(
        bridged
            .call("fx_round_effect")
            .i32(1)
            .f64(2.5)
            .invoke()
            .unwrap()
            .f64(),
        3.0
    );
    assert_eq!(
        bridged
            .call("fx_round_effect")
            .i32(3)
            .f64(-2.5)
            .invoke()
            .unwrap()
            .f64(),
        -2.0
    );
    assert_eq!(
        bridged
            .call("fx_round_effect")
            .i32(0)
            .f64(2.5)
            .invoke()
            .unwrap()
            .f64(),
        2.0
    );
    assert_eq!(
        bridged
            .call("fx_round_effect")
            .i32(0)
            .f64(3.5)
            .invoke()
            .unwrap()
            .f64(),
        4.0
    );
    assert_eq!(
        bridged.call("fx_env_round_trip").invoke().unwrap().i32(),
        200
    );
}

#[test]
fn fenv_arm_exception_flags_come_from_fpsr_and_are_masked_to_the_architectural_bits() {
    let bridged = math();

    assert_eq!(
        bridged.call("fx_exception_flags").invoke().unwrap().i32() & 2,
        2,
        "1.0 / 0.0 raises FE_DIVBYZERO"
    );
    assert_eq!(bridged.call("fx_raise_all").invoke().unwrap().i32(), 0x9f);
}

#[test]
fn libm_long_double_fabsl_and_copysignl_operate_on_binary128_bits() {
    let bridged = math();
    let minus_two = super::quad::f64_to_quad(-2.0);
    let plus_three = super::quad::f64_to_quad(3.0);

    assert_eq!(
        bridged
            .call("fx_fabsl")
            .quad(minus_two)
            .invoke()
            .unwrap()
            .quad(),
        super::quad::f64_to_quad(2.0)
    );
    assert_eq!(
        bridged
            .call("fx_copysignl")
            .quad(plus_three)
            .quad(minus_two)
            .invoke()
            .unwrap()
            .quad(),
        super::quad::f64_to_quad(-3.0)
    );
}
