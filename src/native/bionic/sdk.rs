use std::sync::atomic::{AtomicI32, Ordering};

use super::device_profile::SDK;

static TARGET: AtomicI32 = AtomicI32::new(0);

pub fn application_target_sdk() -> i32 {
    match TARGET.load(Ordering::SeqCst) {
        0 => SDK.parse().expect("device profile SDK is numeric"),
        value => value,
    }
}

pub fn set_application_target_sdk(value: i32) {
    TARGET.store(value, Ordering::SeqCst);
}
