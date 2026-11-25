use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::RwLock;

use super::device_profile::{APP_GID, APP_UID};

// cite: frameworks-base ZygoteInit.java#handleSystemServerProcess
const INITIAL_UMASK: u32 = 0o077;
const UMASK_BITS: u32 = 0o777;

static CWD: RwLock<String> = RwLock::new(String::new());
static UMASK: AtomicU32 = AtomicU32::new(INITIAL_UMASK);

pub fn cwd() -> String {
    let guard = CWD.read().expect("guest cwd");
    if guard.is_empty() {
        "/".to_string()
    } else {
        guard.clone()
    }
}

pub fn set_cwd(path: &str) {
    *CWD.write().expect("guest cwd") = path.to_string();
}

pub fn umask() -> u32 {
    UMASK.load(Ordering::SeqCst)
}

pub fn swap_umask(mask: u32) -> u32 {
    // cite: bionic stat.h#umask
    UMASK.swap(mask & UMASK_BITS, Ordering::SeqCst)
}

pub fn apply_umask(mode: u32) -> u32 {
    mode & !umask() & 0o7777
}

pub fn uid() -> u32 {
    APP_UID
}

pub fn gid() -> u32 {
    APP_GID
}
