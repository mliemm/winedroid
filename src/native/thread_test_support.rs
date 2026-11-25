use std::time::Instant;

use super::bridge::handler_address;
use super::bridge_test_support::Bridged;
use super::call_assign::CallBuilder;

pub const VARIANTS: [&str; 2] = ["16k", "4k"];

pub fn threads_in(variant: &str) -> Bridged {
    Bridged::load(variant, "libfx_threads.so")
}

pub fn each_variant(check: impl Fn(&Bridged)) {
    for variant in VARIANTS {
        eprintln!("fixture variant {variant}");
        check(&threads_in(variant));
    }
}

pub fn shim(name: &str) -> CallBuilder {
    super::bridge_test_support::register_all();
    CallBuilder::new(handler_address(name).unwrap_or_else(|| panic!("{name} is not registered")))
}

pub fn wait_until_last_thread_is_released(bridged: &Bridged) {
    let record = unsafe { *(bridged.symbol("fx_last_thread") as *const usize) };
    let started = Instant::now();
    while super::guest_thread::is_live_address(record) {
        assert!(started.elapsed().as_secs() < 10, "the detached thread never released its record");
        std::thread::yield_now();
    }
}

pub fn word(address: usize) -> u32 {
    unsafe { std::ptr::read_volatile(address as *const u32) }
}
