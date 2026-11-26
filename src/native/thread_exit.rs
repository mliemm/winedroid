use std::cell::Cell;
use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};

use super::bionic::pthread_key;
use super::call_assign::CallBuilder;
use super::futex::{self, MATCH_ANY};
use super::guest_thread;
use super::pthread_internal::{
    PthreadInternal, JOIN_DETACHED, JOIN_EXITED_NOT_JOINED, JOIN_NOT_JOINED,
};
use super::thread_block::{set_slot, SLOT_BIONIC_TLS, SLOT_THREAD_ID};
use super::thread_list;

#[repr(C)]
struct LocalDestructor {
    function: u64,
    argument: u64,
    dso: u64,
    next: u64,
}

#[repr(C)]
struct CleanupFrame {
    previous: u64,
    routine: u64,
    argument: u64,
}

thread_local! {
    static EXIT_REQUESTED: Cell<bool> = const { Cell::new(false) };
}

pub fn request_exit() {
    EXIT_REQUESTED.with(|flag| flag.set(true));
}

pub fn exit_requested() -> bool {
    EXIT_REQUESTED.with(Cell::get)
}

pub fn take_exit_request() -> bool {
    EXIT_REQUESTED.with(|flag| flag.replace(false))
}

fn thread(record: usize) -> *mut PthreadInternal {
    record as *mut PthreadInternal
}

pub fn join_state(record: usize) -> &'static AtomicI32 {
    unsafe { AtomicI32::from_ptr(&raw mut (*thread(record)).join_state) }
}

pub fn tid_word(record: usize) -> usize {
    unsafe { &raw mut (*thread(record)).tid as usize }
}

pub fn add_thread_local_destructor(record: usize, function: u64, argument: u64, dso: u64) {
    // cite: bionic __cxa_thread_atexit_impl.cpp#__cxa_thread_atexit_impl
    let node = Box::into_raw(Box::new(LocalDestructor {
        function,
        argument,
        dso,
        next: unsafe { (*thread(record)).thread_local_dtors },
    }));
    unsafe { (*thread(record)).thread_local_dtors = node as u64 };
}

pub fn finalize_thread_locals(record: usize) -> Result<(), String> {
    // cite: bionic __cxa_thread_atexit_impl.cpp#__cxa_thread_finalize
    loop {
        let head = unsafe { (*thread(record)).thread_local_dtors } as *mut LocalDestructor;
        if head.is_null() {
            return Ok(());
        }
        let node = unsafe { Box::from_raw(head) };
        unsafe { (*thread(record)).thread_local_dtors = node.next };
        CallBuilder::new(node.function as usize)
            .int(node.argument)
            .invoke()
            .map_err(|error| error.to_string())?;
    }
}

fn run_cleanup_stack(record: usize) -> Result<(), String> {
    // cite: bionic pthread_exit.cpp#pthread_exit
    loop {
        let top = unsafe { (*thread(record)).cleanup_stack } as *const CleanupFrame;
        if top.is_null() {
            return Ok(());
        }
        let (previous, routine, argument) =
            unsafe { ((*top).previous, (*top).routine, (*top).argument) };
        unsafe { (*thread(record)).cleanup_stack = previous };
        CallBuilder::new(routine as usize)
            .int(argument)
            .invoke()
            .map_err(|error| error.to_string())?;
    }
}

pub fn run_guest_storage(record: usize, return_value: u64) -> Result<(), String> {
    // cite: bionic pthread_exit.cpp#pthread_exit
    finalize_thread_locals(record)?;
    unsafe { (*thread(record)).return_value = return_value };
    run_cleanup_stack(record)?;
    pthread_key::clean_all(record)
}

pub fn has_pending_guest_storage(record: usize) -> bool {
    unsafe {
        (*thread(record)).thread_local_dtors != 0
            || (*thread(record)).cleanup_stack != 0
            || pthread_key::has_pending_destructors(record)
    }
}

fn block_every_signal() {
    let mut everything: libc::sigset_t = 0;
    unsafe {
        libc::sigfillset(&mut everything);
        libc::pthread_sigmask(libc::SIG_BLOCK, &everything, std::ptr::null_mut());
    }
}

pub fn clear_child_tid(record: usize) {
    // cite: bionic SYSCALLS.TXT#__set_tid_address
    let Some(address) = thread_list::take_clear_tid(record) else {
        return;
    };
    unsafe { AtomicU32::from_ptr(address as *mut u32) }.store(0, Ordering::Release);
    futex::wake(address, i32::MAX, MATCH_ANY);
}

pub fn publish_exit(record: usize) {
    // cite: bionic pthread_exit.cpp#pthread_exit
    let state = join_state(record);
    let mut old = JOIN_NOT_JOINED;
    while old == JOIN_NOT_JOINED {
        match state.compare_exchange_weak(
            old,
            JOIN_EXITED_NOT_JOINED,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => break,
            Err(current) => old = current,
        }
    }
    unsafe { (*thread(record)).terminating = 1 };
    clear_child_tid(record);
    block_every_signal();
    set_slot(SLOT_THREAD_ID, 0);
    set_slot(SLOT_BIONIC_TLS, 0);
    if old == JOIN_DETACHED {
        release(record);
        return;
    }
    let tid = unsafe { AtomicI32::from_ptr(&raw mut (*thread(record)).tid) };
    tid.store(0, Ordering::Release);
    futex::wake(tid_word(record), i32::MAX, MATCH_ANY);
}

pub fn release(record: usize) {
    thread_list::forget(record);
    guest_thread::unregister(record);
    guest_thread::free_memory(record);
}
