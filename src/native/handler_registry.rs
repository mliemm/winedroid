use std::sync::Once;

use super::bionic::register_libc_core;
use super::thread_handlers::register_thread_handlers;
use super::zlib;

const GROUPS: &[fn()] = &[register_thread_handlers, register_libc_core, zlib::register];

pub fn register_all_groups() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        for group in GROUPS {
            group();
        }
    });
}
