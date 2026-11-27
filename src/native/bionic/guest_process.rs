use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, RwLock};

use crate::fs_notify::Hold;

use super::device_profile::{APP_GID, APP_UID};

// cite: frameworks-base ZygoteInit.java#handleSystemServerProcess
const INITIAL_UMASK: u32 = 0o077;
const UMASK_BITS: u32 = 0o777;

static CWD: RwLock<(String, Option<Arc<Hold>>)> = RwLock::new((String::new(), None));
static UMASK: AtomicU32 = AtomicU32::new(INITIAL_UMASK);

pub fn cwd() -> String {
    let guard = CWD.read().expect("guest cwd");
    if guard.0.is_empty() {
        "/".to_string()
    } else {
        guard.0.clone()
    }
}

pub fn set_cwd(path: &str, hold: Option<Arc<Hold>>) {
    let previous = std::mem::replace(
        &mut *CWD.write().expect("guest cwd"),
        (path.to_string(), hold),
    );
    drop(previous);
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
