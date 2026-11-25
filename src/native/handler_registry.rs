use std::sync::Once;

use super::thread_handlers::register_thread_handlers;

const GROUPS: &[fn()] = &[register_thread_handlers];

pub fn register_all_groups() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        for group in GROUPS {
            group();
        }
    });
}
