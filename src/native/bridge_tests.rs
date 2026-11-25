use std::ffi::CString;

use super::bridge::{handler_address, native_provider, register_handler, register_missing, BridgeProvider};
use super::bridge_test_support::take_stack_seen;
use super::bridge_test_support::{register_all, take_atexit, take_logged, Bridged};
use super::call_assign::CallBuilder;
use super::link_resolution::{ImportKind, SystemLibraryProvider};
use super::loader_test_support::{child_outcome, host_protection_of, terminating_signal_of_child};
use super::quad::f64_to_quad;
use super::stub_table::{stub_address, table_base};

const VARIANTS: [&str; 2] = ["16k", "4k"];
const RELOCATING: [&str; 3] = ["libfx_reloc.so", "libfx_aps2.so", "libfx_relr.so"];

#[test]
fn fixture_source_fx_reloc_runs_strlen_memcpy_malloc_and_snprintf_through_bridge_handlers() {
    for variant in VARIANTS {
        for library in RELOCATING {
            let bridged = Bridged::load(variant, library);
            let label = format!("{variant}/{library}");
            let text = CString::new("hello bridge").unwrap();

            assert_eq!(bridged.call("fx_strlen").ptr(text.as_ptr() as usize).invoke().unwrap().u64(), 12, "{label} strlen");
            assert_eq!(bridged.call("fx_ctor_marker").invoke().unwrap().i32(), 0xC0DE, "{label} constructor ran through call_guest");
            let bytes = [1u8, 2, 3, 4, 250];
            assert_eq!(bridged.call("fx_copy_sum").ptr(bytes.as_ptr() as usize).int(5).invoke().unwrap().i32(), 260, "{label} malloc memcpy free");

            let mut buffer = [0u8; 64];
            let label_text = CString::new("abc").unwrap();
            let written = bridged
                .call("fx_format")
                .ptr(buffer.as_mut_ptr() as usize)
                .int(64)
                .i32(42)
                .ptr(label_text.as_ptr() as usize)
                .f64(1.5)
                .invoke()
                .unwrap()
                .i32();
            assert_eq!(&buffer[..written as usize], b"42 abc 1.500000", "{label} snprintf %d %s %f read from AAPCS64 registers");
            assert_eq!(written, 15, "{label}");

            assert_eq!(bridged.call("fx_lock_roundtrip").invoke().unwrap().i32(), 0, "{label}");
            let message = CString::new("note").unwrap();
            bridged.call("fx_log").ptr(message.as_ptr() as usize).invoke().unwrap();
            assert_eq!(take_logged(), vec![(4, "fx_reloc".to_string(), "note 7".to_string())], "{label} liblog variadic");
            assert_eq!(take_atexit().len(), 1, "{label} __cxa_atexit from the constructor");
        }
    }
}

#[test]
fn fixture_source_fx_reloc_function_pointers_to_imports_equal_their_stub_address() {
    let bridged = Bridged::load("16k", "libfx_reloc.so");
    let address = bridged.call("fx_strlen_address").invoke().unwrap().u64() as usize;

    assert_eq!(address, handler_address("strlen").unwrap());
    let table = bridged.symbol("fx_import_table");
    let entries = unsafe { std::slice::from_raw_parts(table as *const usize, 2) };
    assert_eq!(entries, [handler_address("strlen").unwrap(), handler_address("memcpy").unwrap()]);
}

#[test]
fn fixture_source_fx_guarded_sum_reads_the_canary_from_the_thread_pointer_slot_and_returns() {
    let bridged = Bridged::load("16k", "libfx_reloc.so");
    let text = CString::new("abc").unwrap();

    assert_eq!(bridged.call("fx_guarded_sum").ptr(text.as_ptr() as usize).invoke().unwrap().i32(), 294);
}

#[test]
fn bionic_stack_chk_fail_aborts_when_a_guest_overflow_overwrites_the_canary() {
    let bridged = Bridged::load("16k", "libfx_bridge.so");
    let fine = bridged.call("fx_canary_overflow").int(8).invoke().unwrap().i32();
    assert_eq!(fine, 0x41);

    let (signal, stderr) = child_outcome(|| {
        let _ = bridged.call("fx_canary_overflow").int(64).invoke();
    });

    assert_eq!(signal, Some(libc::SIGABRT));
    assert!(stderr.contains("stack corruption detected"), "{stderr}");
}

#[test]
fn stub_table_lives_in_an_anonymous_read_execute_mapping_with_stable_addresses_per_name() {
    register_all();
    let base = table_base().unwrap();
    let first = handler_address("strlen").unwrap();

    assert_eq!(base % 16384, 0);
    assert_eq!(host_protection_of(base), libc::PROT_READ | libc::PROT_EXEC);
    assert_eq!(host_protection_of(stub_address(8191)), libc::PROT_READ | libc::PROT_EXEC);
    assert!(first >= base && first < base + 8192 * 8);
    assert!(std::panic::catch_unwind(|| register_handler("strlen", |_| {})).is_err(), "a second handler for strlen is refused");
    assert_eq!(handler_address("strlen"), Some(first));
    assert_eq!(register_missing("/guest/libother.so", "strlen"), first);
    let exe_text = bridge_text_address();
    assert!(exe_text < base || exe_text >= base + 16384 * 5, "stub table is not inside the host binary");
}

fn bridge_text_address() -> usize {
    bridge_text_address as *const () as usize
}

#[test]
fn bridge_provider_resolves_registered_names_data_objects_and_missing_imports_without_fake_success() {
    register_all();
    let provider = BridgeProvider;

    assert_eq!(provider.export("libc.so", "strlen"), handler_address("strlen"));
    assert_eq!(provider.export("libfoo.so", "strlen"), None);
    let guard = provider.export("libc.so", "__stack_chk_guard").unwrap();
    assert_eq!(unsafe { *(guard as *const u64) }, super::thread_block::block().canary());

    let missing = provider.unresolved("/guest/libx.so", "never_registered_function", ImportKind::Function);
    assert_eq!(provider.unresolved("/guest/liby.so", "never_registered_function", ImportKind::Function), missing);
    assert_eq!(provider.export("libc.so", "never_registered_function"), None);
    let data = provider.unresolved("/guest/libx.so", "never_registered_object", ImportKind::Data);
    assert_eq!(host_protection_of(data), 0, "unresolved data faults on access");
}

#[test]
fn calling_an_unresolved_import_reports_the_explicit_error_and_the_guest_call_stays_usable() {
    let bridged = Bridged::load("16k", "libfx_bridge.so");

    for _ in 0..50 {
        let error = bridged.call("fx_call_missing").invoke().unwrap_err().to_string();
        assert!(error.contains("FIXME: native import not implemented"), "{error}");
        assert!(error.contains("fx_not_registered"), "{error}");
        assert!(error.contains("libfx_bridge.so"), "{error}");
        assert_eq!(bridged.call("fx_va_sum_int").i32(2).i32(40).i32(2).invoke().unwrap().u64(), 42);
    }
}

#[test]
fn registering_a_handler_for_a_missing_import_upgrades_the_same_stub_address() {
    register_all();
    let missing = register_missing("/guest/libz.so", "fx_late_binding");
    let handled = register_handler("fx_late_binding", |context| context.ret_i32(77));

    assert_eq!(missing, handled);
    assert_eq!(CallBuilder::new(handled).invoke().unwrap().i32(), 77);
}

#[test]
fn a_handler_panic_aborts_the_process_instead_of_unwinding_into_the_stub() {
    register_all();
    let address = register_handler("fx_panics", |_| panic!("handler failure"));

    let signal = terminating_signal_of_child(|| {
        let _ = CallBuilder::new(address).invoke();
    });

    assert_eq!(signal, Some(libc::SIGABRT));
}

#[test]
fn guest_to_host_to_guest_calls_nest_three_deep() {
    let bridged = Bridged::load("16k", "libfx_bridge.so");

    for depth in [1u64, 2, 3, 6] {
        assert_eq!(bridged.call("fx_nest").int(depth).invoke().unwrap().u64(), depth);
    }
}

#[test]
fn a_missing_import_inside_a_nested_call_unwinds_to_the_innermost_call_only() {
    register_all();
    let bridged = Bridged::load("16k", "libfx_bridge.so");
    let outer = register_handler("fx_nest_with_missing", {
        let missing = bridged.symbol("fx_call_missing");
        move |context| {
            let inner = CallBuilder::new(missing).invoke();
            context.ret_i32(i32::from(inner.is_err()));
        }
    });

    assert_eq!(CallBuilder::new(outer).invoke().unwrap().i32(), 1);
}

#[test]
fn guest_callee_saved_registers_survive_import_calls_and_unwinding() {
    let bridged = Bridged::load("16k", "libfx_bridge.so");
    let seed = 3i64;
    let integers: i64 = (1..=10).map(|index| (seed + index) * index).sum();
    let reals: f64 = (0..8).map(|index| seed as f64 * (0.5 + index as f64)).sum();
    let expected = integers * 1000 + reals as i64;

    assert_eq!(bridged.call("fx_callee_saved").int(seed as u64).invoke().unwrap().u64() as i64, expected);
    let _ = bridged.call("fx_call_missing").invoke();
    assert_eq!(bridged.call("fx_callee_saved").int(seed as u64).invoke().unwrap().u64() as i64, expected);
}

#[test]
fn stack_pointer_is_sixteen_byte_aligned_for_every_stack_argument_count() {
    let bridged = Bridged::load("16k", "libfx_bridge.so");
    let function = bridged.symbol("fx_sp_probe");
    let mut previous = None;
    for stack_words in 0..10u64 {
        let mut call = CallBuilder::new(function);
        for _ in 0..8 + stack_words {
            call = call.int(0);
        }
        let sp = call.invoke().unwrap().u64();
        assert_eq!(sp % 16, 0, "{stack_words} stack words");
        if let Some(last) = previous {
            assert!(sp <= last, "the stack grows with its arguments");
        }
        previous = Some(sp);
    }
    assert_eq!(bridged.call("fx_call_sp_import").invoke().unwrap().i32(), 1, "stub entered with an aligned sp");
}

#[test]
fn a_handler_sees_a_sixteen_byte_aligned_sp_that_points_at_the_first_stack_argument_for_every_stack_word_count() {
    register_all();
    let stub = handler_address("fx_sp_check").unwrap();
    for stack_words in 0..10u64 {
        let mut call = CallBuilder::new(stub);
        for value in 0..8 + stack_words {
            call = call.int(if value < 8 { value } else { 1000 + value });
        }

        let aligned = call.invoke().unwrap().i32();

        let (sp, words) = take_stack_seen();
        assert_eq!(aligned, 1, "{stack_words} stack words: handler sp {sp:#x} is not 16 byte aligned");
        assert_eq!(sp % 16, 0, "{stack_words} stack words: handler sp {sp:#x} is not 16 byte aligned");
        let expected: Vec<u64> = (0..stack_words).map(|index| 1008 + index).collect();
        assert_eq!(words[..stack_words as usize], expected[..], "{stack_words} stack words: sp {sp:#x} does not point at the first stack argument");
    }
}

#[test]
fn narrow_integer_arguments_are_read_from_the_low_bits_only() {
    register_all();
    let stub = handler_address("fx_take_narrow").unwrap();
    let dirty = 0xDEAD_BEEF_0000_0000u64;

    let result = CallBuilder::new(stub)
        .int(dirty | 0xFFFF_FFFF)
        .int(dirty | 0xFFFE)
        .int(dirty | 0xFFFF_FFFD)
        .int(dirty | 1)
        .int(dirty | 200)
        .invoke()
        .unwrap()
        .i32();

    assert_eq!(result, -1 - 2 - 3 + 200 + 1000);
}

#[test]
fn indirect_hfa_pair_and_long_double_results_come_back_in_the_aapcs64_registers() {
    let bridged = Bridged::load("16k", "libfx_bridge.so");

    assert_eq!(bridged.call("fx_call_get_s24").invoke().unwrap().u64(), 321, "sret through x8");
    assert_eq!(bridged.call("fx_call_get_hfa4").invoke().unwrap().f32(), 4321.0, "HFA in s0 to s3");
    assert_eq!(bridged.call("fx_call_get_pair").invoke().unwrap().u64(), 7009, "x0 and x1");
    assert_eq!(bridged.call("fx_call_get_ld").invoke().unwrap().quad(), f64_to_quad(2.5), "q0");
}

#[test]
fn errno_is_restored_unless_the_handler_sets_it_and_the_guest_reads_the_host_slot() {
    let bridged = Bridged::load("16k", "libfx_bridge.so");

    assert_eq!(bridged.call("fx_errno_flow").invoke().unwrap().i32(), 5034);
}

fn local_table(slots: usize) -> Vec<std::sync::atomic::AtomicPtr<super::bridge::Entry>> {
    (0..slots).map(|_| std::sync::atomic::AtomicPtr::new(std::ptr::null_mut())).collect()
}

#[test]
fn a_full_stub_table_binds_further_names_to_an_explicit_overflow_entry_instead_of_panicking() {
    let entries = local_table(4);
    let mut ids = std::collections::HashMap::new();

    let first: Vec<usize> = ["a", "b", "c"].iter().map(|name| super::bridge::bind_for_test(&entries, &mut ids, name, true)).collect();
    let over = super::bridge::bind_for_test(&entries, &mut ids, "d", true);
    let over_again = super::bridge::bind_for_test(&entries, &mut ids, "e", false);

    assert_eq!(first, [0, 1, 2]);
    assert_eq!((over, over_again), (3, 3));
    assert!(super::bridge::entry_is_overflow(&entries, 3));
    assert!(!super::bridge::entry_is_overflow(&entries, 2));
}

#[test]
#[should_panic(expected = "native handler already registered: dup")]
fn registering_a_second_handler_for_the_same_symbol_is_refused() {
    let entries = local_table(4);
    let mut ids = std::collections::HashMap::new();
    super::bridge::bind_for_test(&entries, &mut ids, "dup", true);
    super::bridge::bind_for_test(&entries, &mut ids, "dup", true);
}

#[test]
fn a_handler_replaces_a_missing_binding_for_the_same_name() {
    let entries = local_table(4);
    let mut ids = std::collections::HashMap::new();
    let missing = super::bridge::bind_for_test(&entries, &mut ids, "late", false);
    let handler = super::bridge::bind_for_test(&entries, &mut ids, "late", true);

    assert_eq!(missing, handler);
    assert_eq!(super::bridge::bind_for_test(&entries, &mut ids, "late", false), handler);
}

#[test]
fn native_provider_registers_the_thread_handlers_once_and_serves_them_on_every_call() {
    let first = native_provider();
    let second = native_provider();

    let pthread_self = first.export("libc.so", "pthread_self").expect("pthread_self is registered");
    assert_eq!(second.export("libc.so", "pthread_self"), Some(pthread_self));
    assert!(second.export("libc.so", "__stack_chk_guard").is_some());
}

#[test]
fn an_unresolved_data_import_message_names_the_importer_the_object_and_the_missing_primitive() {
    let message = super::bridge::missing_data_message("/guest/libx.so", "environ_like_object");

    assert!(message.starts_with("FIXME: native data import not implemented"), "{message}");
    assert!(message.contains("/guest/libx.so") && message.contains("environ_like_object"), "{message}");
    assert!(message.contains("next missing primitive"), "{message}");
}
