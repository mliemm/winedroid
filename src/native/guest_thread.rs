use std::collections::BTreeMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::error::{Result, WinedroidError};

use super::data_fault::ensure_alternate_stack;
use super::pthread_internal::{PthreadInternal, ATTR_FLAG_INHERIT, JOIN_NOT_JOINED};
use super::thread_block::{block, host_thread_pointer, set_slot, slot_word, SLOT_ART_THREAD_SELF, SLOT_STACK_GUARD, SLOT_THREAD_ID};


pub(super) struct Registry {
    records: BTreeMap<usize, i32>,
    primary: Option<usize>,
    primary_assigned: bool,
}

static RECORD_KEY: OnceLock<libc::pthread_key_t> = OnceLock::new();
static REGISTRY: Mutex<Registry> = Mutex::new(empty_registry());
static NEXT_TID: AtomicI32 = AtomicI32::new(0);

extern "C" fn free_record(record: *mut libc::c_void) {
    let address = record as usize;
    set_slot(SLOT_THREAD_ID, 0);
    let mut registry = REGISTRY.lock().expect("guest thread registry");
    release(&mut registry, address);
    drop(registry);
    unsafe { std::alloc::dealloc(record as *mut u8, record_layout()) };
}

pub(super) fn release(registry: &mut Registry, address: usize) {
    registry.records.remove(&address);
    if registry.primary == Some(address) {
        registry.primary = None;
    }
}

pub(super) fn claim_primary(registry: &mut Registry, address: usize) -> bool {
    let first = !registry.primary_assigned;
    if first {
        registry.primary = Some(address);
        registry.primary_assigned = true;
    }
    first
}

pub(super) const fn empty_registry() -> Registry {
    Registry { records: BTreeMap::new(), primary: None, primary_assigned: false }
}

fn record_layout() -> std::alloc::Layout {
    std::alloc::Layout::new::<PthreadInternal>()
}

fn record_key() -> libc::pthread_key_t {
    *RECORD_KEY.get_or_init(|| {
        let mut key: libc::pthread_key_t = 0;
        assert_eq!(unsafe { libc::pthread_key_create(&mut key, Some(free_record)) }, 0, "thread record key");
        key
    })
}

pub fn thread_pointer() -> usize {
    host_thread_pointer()
}

pub fn current_record() -> Option<*mut PthreadInternal> {
    let record = slot_word(SLOT_THREAD_ID) as *mut PthreadInternal;
    (!record.is_null()).then_some(record)
}

pub fn is_record_live(address: usize, tid: i32) -> bool {
    REGISTRY.lock().expect("guest thread registry").records.get(&address) == Some(&tid)
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
    ensure_alternate_stack();
    let record = unsafe { std::alloc::alloc_zeroed(record_layout()) } as *mut PthreadInternal;
    assert!(!record.is_null(), "thread record allocation failed");
    let (stack_base, stack_size) = stack_bounds();
    let pid = unsafe { libc::getpid() };
    let address = record as usize;
    let mut registry = REGISTRY.lock().expect("guest thread registry");
    let primary = claim_primary(&mut registry, address);
    let tid = if primary { pid } else { next_tid() };
    unsafe {
        (*record).tid = tid;
        (*record).cached_pid = pid as u32;
        (*record).attr.flags = ATTR_FLAG_INHERIT;
        (*record).attr.stack_base = stack_base;
        (*record).attr.stack_size = stack_size;
        (*record).stack_top = stack_base + stack_size;
        (*record).join_state = JOIN_NOT_JOINED;
        assert_eq!(libc::pthread_setspecific(record_key(), record as *const libc::c_void), 0, "thread record store");
    }
    set_slot(SLOT_THREAD_ID, address as u64);
    set_slot(SLOT_STACK_GUARD, block().canary());
    registry.records.insert(address, tid);
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
    slot_word(SLOT_ART_THREAD_SELF) == 0
}
