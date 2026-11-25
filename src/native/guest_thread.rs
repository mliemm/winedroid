use std::collections::BTreeSet;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::error::{Result, WinedroidError};

use super::pthread_internal::{PthreadInternal, ATTR_FLAG_INHERIT, JOIN_NOT_JOINED};

const MAX_TSD_KEY: usize = 4095;
const STACK_GUARD_SLOT: usize = 5;

struct ThreadKeys {
    record: libc::pthread_key_t,
    art_thread: libc::pthread_key_t,
}

struct Registry {
    records: BTreeSet<usize>,
    primary: Option<usize>,
}

static KEYS: OnceLock<ThreadKeys> = OnceLock::new();
static REGISTRY: Mutex<Registry> = Mutex::new(Registry { records: BTreeSet::new(), primary: None });
static NEXT_TID: AtomicI32 = AtomicI32::new(0);

extern "C" fn free_record(record: *mut libc::c_void) {
    let address = record as usize;
    let mut registry = REGISTRY.lock().expect("guest thread registry");
    registry.records.remove(&address);
    if registry.primary == Some(address) {
        registry.primary = None;
    }
    drop(registry);
    unsafe { std::alloc::dealloc(record as *mut u8, record_layout()) };
}

fn record_layout() -> std::alloc::Layout {
    std::alloc::Layout::new::<PthreadInternal>()
}

fn keys() -> &'static ThreadKeys {
    KEYS.get_or_init(|| {
        let mut record: libc::pthread_key_t = 0;
        let mut art_thread: libc::pthread_key_t = 0;
        unsafe {
            assert_eq!(libc::pthread_key_create(&mut record, Some(free_record)), 0, "thread record key");
            assert_eq!(libc::pthread_key_create(&mut art_thread, None), 0, "art thread key");
        }
        assert!(record as usize <= MAX_TSD_KEY && art_thread as usize <= MAX_TSD_KEY, "thread keys do not fit an LDR immediate");
        ThreadKeys { record, art_thread }
    })
}

pub fn record_key() -> usize {
    keys().record as usize
}

pub fn art_thread_key() -> usize {
    keys().art_thread as usize
}

pub fn thread_pointer() -> usize {
    let value: usize;
    unsafe { std::arch::asm!("mrs {}, tpidrro_el0", out(reg) value, options(nomem, nostack, preserves_flags)) };
    value
}

pub fn slot_word(slot: usize) -> u64 {
    unsafe { *(thread_pointer() as *const u64).add(slot) }
}

pub fn current_record() -> Option<*mut PthreadInternal> {
    let record = unsafe { *(thread_pointer() as *const u64).add(record_key()) } as *mut PthreadInternal;
    (!record.is_null()).then_some(record)
}

pub fn is_record_live(address: usize) -> bool {
    REGISTRY.lock().expect("guest thread registry").records.contains(&address)
}

fn next_tid() -> i32 {
    let pid = unsafe { libc::getpid() };
    loop {
        let seen = NEXT_TID.load(Ordering::SeqCst);
        let candidate = if seen == 0 { pid + 1 } else { seen + 1 };
        let candidate = if candidate == pid { candidate + 1 } else { candidate };
        if NEXT_TID.compare_exchange(seen, candidate, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
            return candidate;
        }
    }
}

fn stack_bounds() -> (u64, u64) {
    let host = unsafe { libc::pthread_self() };
    let top = unsafe { libc::pthread_get_stackaddr_np(host) } as u64;
    let size = unsafe { libc::pthread_get_stacksize_np(host) } as u64;
    (top - size, size)
}

pub fn attach_current() -> Result<*mut PthreadInternal> {
    if let Some(record) = current_record() {
        return Ok(record);
    }
    // cite: bionic tls_defines.h#TLS_SLOT_STACK_GUARD
    if slot_word(STACK_GUARD_SLOT) != 0 {
        return Err(WinedroidError::new(
            "guest code on a thread whose stack guard slot is not zero; next missing primitive: guest code on a workqueue thread",
        ));
    }
    let key = keys().record;
    let record = unsafe { std::alloc::alloc_zeroed(record_layout()) } as *mut PthreadInternal;
    assert!(!record.is_null(), "thread record allocation failed");
    let (stack_base, stack_size) = stack_bounds();
    let pid = unsafe { libc::getpid() };
    let address = record as usize;
    let mut registry = REGISTRY.lock().expect("guest thread registry");
    let primary = registry.primary.is_none();
    unsafe {
        (*record).tid = if primary { pid } else { next_tid() };
        (*record).cached_pid = pid as u32;
        (*record).attr.flags = ATTR_FLAG_INHERIT;
        (*record).attr.stack_base = stack_base;
        (*record).attr.stack_size = stack_size;
        (*record).stack_top = stack_base + stack_size;
        (*record).join_state = JOIN_NOT_JOINED;
        assert_eq!(libc::pthread_setspecific(key, record as *const libc::c_void), 0, "thread record store");
    }
    registry.records.insert(address);
    if primary {
        registry.primary = Some(address);
    }
    Ok(record)
}

pub fn set_primary() -> Result<()> {
    let record = attach_current()? as usize;
    let registry = REGISTRY.lock().expect("guest thread registry");
    match registry.primary {
        Some(primary) if primary == record => Ok(()),
        _ => Err(WinedroidError::new("another thread is the primary guest thread; next missing primitive: reassigning the primary guest thread")),
    }
}

pub fn is_primary(record: *mut PthreadInternal) -> bool {
    REGISTRY.lock().expect("guest thread registry").primary == Some(record as usize)
}

pub fn art_thread_slot_is_null() -> bool {
    unsafe { *(thread_pointer() as *const u64).add(art_thread_key()) == 0 }
}
