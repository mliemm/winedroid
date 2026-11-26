use std::any::Any;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

use crate::error::{Result, WinedroidError};

use super::bionic::pthread_key;
use super::data_fault::ensure_alternate_stack;
use super::fork_gate;
use super::host_log;
use super::pthread_internal::{
    PthreadInternal, ATTR_FLAG_DETACHED, ATTR_FLAG_INHERIT, JOIN_DETACHED,
};
use super::thread_block::{
    block, host_thread_pointer, set_slot, slot_word, SLOT_ART_THREAD_SELF, SLOT_BIONIC_TLS,
    SLOT_STACK_GUARD, SLOT_THREAD_ID,
};
use super::thread_exit;
use super::thread_list;

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
    if thread_exit::has_pending_guest_storage(address) {
        if let Err(message) = thread_exit::run_guest_storage(address, 0) {
            host_log::line(&format!("FIXME: guest thread storage destructor failed on a host created thread: {message}; next missing primitive: guest destructor failure"));
        }
    }
    thread_exit::clear_child_tid(address);
    set_slot(SLOT_THREAD_ID, 0);
    let mut registry = registry();
    release(&mut registry, address);
    drop(registry);
    thread_list::forget(address);
    free_memory(address);
}

fn registry() -> MutexGuard<'static, Registry> {
    REGISTRY.lock().unwrap_or_else(PoisonError::into_inner)
}

pub(super) fn hold_for_fork(held: &mut Vec<Box<dyn Any>>) {
    held.push(Box::new(registry()));
}

pub(super) fn free_memory(address: usize) {
    let record = address as *mut PthreadInternal;
    unsafe {
        libc::free((*record).bionic_tls as *mut libc::c_void);
        std::alloc::dealloc(record as *mut u8, record_layout());
    }
}

pub(super) fn unregister(address: usize) {
    release(&mut registry(), address);
}

pub(super) fn register_created(address: usize, tid: i32) {
    registry().records.insert(address, tid);
}

pub fn is_live_address(address: usize) -> bool {
    registry().records.contains_key(&address)
}

pub fn tid_of(address: usize) -> Option<i32> {
    registry().records.get(&address).copied()
}

pub fn live_records() -> Vec<(usize, i32)> {
    registry()
        .records
        .iter()
        .map(|(address, tid)| (*address, *tid))
        .collect()
}

pub fn detach_current() -> Result<()> {
    let record = attach_current()? as usize;
    thread_exit::run_guest_storage(record, 0).map_err(WinedroidError::new)
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
    Registry {
        records: BTreeMap::new(),
        primary: None,
        primary_assigned: false,
    }
}

fn record_layout() -> std::alloc::Layout {
    std::alloc::Layout::new::<PthreadInternal>()
}

fn record_key() -> libc::pthread_key_t {
    *RECORD_KEY.get_or_init(|| {
        let mut key: libc::pthread_key_t = 0;
        assert_eq!(
            unsafe { libc::pthread_key_create(&mut key, Some(free_record)) },
            0,
            "thread record key"
        );
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
    registry().records.get(&address) == Some(&tid)
}

pub(super) fn next_tid() -> i32 {
    let pid = unsafe { libc::getpid() };
    loop {
        let seen = NEXT_TID.load(Ordering::SeqCst);
        let candidate = if seen == 0 { pid + 1 } else { seen + 1 };
        let candidate = if candidate == pid {
            candidate + 1
        } else {
            candidate
        };
        if NEXT_TID
            .compare_exchange(seen, candidate, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            return candidate;
        }
    }
}

pub(super) fn stack_bounds() -> (u64, u64) {
    let host = unsafe { libc::pthread_self() };
    let top = unsafe { libc::pthread_get_stackaddr_np(host) } as u64;
    let size = unsafe { libc::pthread_get_stacksize_np(host) } as u64;
    (top - size, size)
}

pub(super) fn install_slots(record: *mut PthreadInternal, requested_stack: Option<u64>) {
    ensure_alternate_stack();
    let (host_base, host_size) = stack_bounds();
    let top = host_base + host_size;
    unsafe {
        match requested_stack {
            Some(size) if (*record).attr.stack_base == 0 => {
                (*record).attr.stack_base = top - size;
                (*record).attr.stack_size = size;
            }
            Some(_) => {}
            None => {
                (*record).attr.stack_base = host_base;
                (*record).attr.stack_size = host_size;
            }
        }
        (*record).stack_top = (*record).attr.stack_base + (*record).attr.stack_size;
        (*record).bionic_tls = pthread_key::new_block() as u64;
    }
    set_slot(SLOT_BIONIC_TLS, unsafe { (*record).bionic_tls });
    set_slot(SLOT_THREAD_ID, record as u64);
    set_slot(SLOT_STACK_GUARD, block().canary());
}

pub fn attach_current() -> Result<*mut PthreadInternal> {
    if let Some(record) = current_record() {
        return Ok(record);
    }
    fork_gate::install();
    let record = unsafe { std::alloc::alloc_zeroed(record_layout()) } as *mut PthreadInternal;
    assert!(!record.is_null(), "thread record allocation failed");
    let pid = unsafe { libc::getpid() };
    let address = record as usize;
    let primary = claim_primary(&mut registry(), address);
    let tid = if primary { pid } else { next_tid() };
    unsafe {
        (*record).tid = tid;
        (*record).cached_pid = pid as u32;
        (*record).attr.flags = ATTR_FLAG_INHERIT | ATTR_FLAG_DETACHED;
        (*record).join_state = JOIN_DETACHED;
        assert_eq!(
            libc::pthread_setspecific(record_key(), record as *const libc::c_void),
            0,
            "thread record store"
        );
    }
    install_slots(record, None);
    thread_list::remember(address, unsafe { libc::pthread_self() }, false);
    registry().records.insert(address, tid);
    Ok(record)
}

pub(super) fn install_created(record: *mut PthreadInternal) {
    install_slots(record, Some(unsafe { (*record).attr.stack_size }));
}

pub fn set_primary() -> Result<()> {
    let record = attach_current()? as usize;
    let registry = registry();
    match registry.primary {
        Some(primary) if primary == record => Ok(()),
        _ => Err(WinedroidError::new("another thread is the primary guest thread; next missing primitive: reassigning the primary guest thread")),
    }
}

pub fn is_primary(record: *mut PthreadInternal) -> bool {
    registry().primary == Some(record as usize)
}

pub fn art_thread_slot_is_null() -> bool {
    slot_word(SLOT_ART_THREAD_SELF) == 0
}
