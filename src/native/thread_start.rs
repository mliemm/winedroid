use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};

use super::bionic::errno_map::linux_from_darwin;
use super::bionic::guest_mem::write_value;
use super::bionic::pthread_attr::{self, ATTR_SIZE, FLAG_DETACHED, FLAG_EXPLICIT, FLAG_INHERIT};
use super::bionic::pthread_sched;
use super::bionic::{bionic_lock, current_thread};
use super::call_assign::CallBuilder;
use super::futex::{self, MATCH_ANY};
use super::guest_thread::{self, install_created, next_tid, register_created};
use super::host_log;
use super::pthread_internal::{PthreadInternal, JOIN_DETACHED, JOIN_NOT_JOINED};
use super::thread_exit;
use super::thread_list;

const EAGAIN: i32 = 11;
const HOST_PAGE: u64 = 16384;
const GUEST_PAGE: u64 = 4096;
const HOST_HEADROOM: u64 = 256 * 1024;
const HANDSHAKE_OFFSET: usize = 156;

extern "C" {
    fn pthread_attr_setguardsize(attr: *mut libc::pthread_attr_t, size: usize) -> i32;
    fn pthread_attr_setstack(attr: *mut libc::pthread_attr_t, base: *mut c_void, size: usize) -> i32;
}

struct Start {
    record: usize,
    ready: AtomicU32,
}

fn round_up(value: u64, unit: u64) -> Option<u64> {
    value.checked_add(unit - 1).map(|sum| sum / unit * unit)
}

fn failure_to_run(record: usize, message: &str) {
    host_log::line(&format!("FIXME: guest thread {} start routine failed: {message}", unsafe { (*(record as *const PthreadInternal)).tid }));
    if crate::missing::strict_enabled() {
        std::process::abort();
    }
}

extern "C" fn host_thread_main(raw: *mut c_void) -> *mut c_void {
    let start = unsafe { &*(raw as *const Start) };
    let record = start.record;
    install_created(record as *mut PthreadInternal);
    let ready = &start.ready as *const AtomicU32 as usize;
    start.ready.store(1, Ordering::SeqCst);
    futex::wake(ready, 1, MATCH_ANY);
    bionic_lock::lock(record + HANDSHAKE_OFFSET);
    bionic_lock::unlock(record + HANDSHAKE_OFFSET);
    let (routine, argument) = unsafe { ((*(record as *const PthreadInternal)).start_routine, (*(record as *const PthreadInternal)).start_routine_arg) };
    let outcome = if routine == 0 { Ok(0) } else { CallBuilder::new(routine as usize).ptr(argument as usize).invoke().map(|result| result.u64()) };
    let finished = match outcome {
        Ok(result) => thread_exit::run_guest_storage(record, result),
        Err(error) if thread_exit::take_exit_request() => {
            let _ = error;
            Ok(())
        }
        Err(error) => {
            failure_to_run(record, &error.to_string());
            thread_exit::run_guest_storage(record, 0)
        }
    };
    if let Err(message) = finished {
        failure_to_run(record, &message);
    }
    thread_exit::publish_exit(record);
    std::ptr::null_mut()
}

fn host_attributes(attr: usize) -> Result<libc::pthread_attr_t, i32> {
    let mut host: libc::pthread_attr_t = unsafe { std::mem::zeroed() };
    unsafe { libc::pthread_attr_init(&mut host) };
    let flags: u32 = super::bionic::guest_mem::read_value(attr);
    let (base, size, guard): (u64, u64, u64) = (
        super::bionic::guest_mem::read_value(attr + 8),
        super::bionic::guest_mem::read_value(attr + 16),
        super::bionic::guest_mem::read_value(attr + 24),
    );
    if base == 0 {
        let guard = round_up(guard, GUEST_PAGE).ok_or(EAGAIN)?;
        let stack = round_up(size.checked_add(guard).ok_or(EAGAIN)?, HOST_PAGE).and_then(|bytes| bytes.checked_add(HOST_HEADROOM)).ok_or(EAGAIN)?;
        unsafe {
            libc::pthread_attr_setstacksize(&mut host, stack as usize);
            pthread_attr_setguardsize(&mut host, round_up(guard, HOST_PAGE).ok_or(EAGAIN)?.max(HOST_PAGE) as usize);
        }
    } else {
        if base % HOST_PAGE != 0 || size % HOST_PAGE != 0 {
            host_log::line("FIXME: guest thread stack is not 16 KiB aligned; next missing primitive: custom thread stack not 16 KiB aligned");
            return Err(EAGAIN);
        }
        unsafe { pthread_attr_setstack(&mut host, base as *mut c_void, size as usize) };
    }
    if flags & FLAG_DETACHED != 0 {
        unsafe { libc::pthread_attr_setdetachstate(&mut host, libc::PTHREAD_CREATE_DETACHED) };
    }
    Ok(host)
}

fn schedule(record: usize, host: libc::pthread_t, attr: usize) -> Result<(), i32> {
    // cite: bionic pthread_create.cpp#__init_thread
    let flags: u32 = super::bionic::guest_mem::read_value(attr);
    let (policy, priority) = (super::bionic::guest_mem::read_value::<i32>(attr + 32), super::bionic::guest_mem::read_value::<i32>(attr + 36));
    let need = if flags & FLAG_INHERIT != 0 { false } else if flags & FLAG_EXPLICIT != 0 { true } else { policy != 0 };
    let _ = record;
    if need { pthread_sched::set_host(host, policy, priority) } else { Ok(()) }
}

pub fn create(thread_out: usize, attr: usize, routine: usize, argument: usize) -> i32 {
    // cite: bionic pthread_create.cpp#pthread_create
    let mut attributes = [0u8; ATTR_SIZE];
    let attr_address = attributes.as_mut_ptr() as usize;
    if attr == 0 {
        pthread_attr::init(attr_address);
    } else {
        unsafe { std::ptr::copy_nonoverlapping(attr as *const u8, attributes.as_mut_ptr(), ATTR_SIZE) };
    }
    let host_attr = match host_attributes(attr_address) {
        Ok(host) => host,
        Err(code) => return code,
    };
    let creator = current_thread::record();
    let record = unsafe { std::alloc::alloc_zeroed(std::alloc::Layout::new::<PthreadInternal>()) } as *mut PthreadInternal;
    assert!(!record.is_null(), "thread record allocation failed");
    let address = record as usize;
    let flags: u32 = super::bionic::guest_mem::read_value(attr_address);
    let tid = next_tid();
    unsafe {
        std::ptr::copy_nonoverlapping(attributes.as_ptr(), &raw mut (*record).attr as *mut u8, ATTR_SIZE);
        (*record).tid = tid;
        (*record).cached_pid = libc::getpid() as u32;
        (*record).join_state = if flags & FLAG_DETACHED != 0 { JOIN_DETACHED } else { JOIN_NOT_JOINED };
        (*record).start_routine = routine as u64;
        (*record).start_routine_arg = argument as u64;
        (*record).start_mask = (*(creator as *const PthreadInternal)).start_mask;
    }
    register_created(address, tid);
    bionic_lock::init(address + HANDSHAKE_OFFSET, false);
    bionic_lock::lock(address + HANDSHAKE_OFFSET);
    let start = Box::into_raw(Box::new(Start { record: address, ready: AtomicU32::new(0) }));
    let mut host: libc::pthread_t = unsafe { std::mem::zeroed() };
    let status = unsafe { libc::pthread_create(&mut host, &host_attr, host_thread_main, start as *mut c_void) };
    if status != 0 {
        drop(unsafe { Box::from_raw(start) });
        guest_thread::unregister(address);
        guest_thread::free_memory(address);
        return linux_from_darwin(status).unwrap_or(EAGAIN);
    }
    thread_list::remember(address, host, true);
    thread_list::inherit_name(creator, address);
    let ready = unsafe { &(*start).ready };
    while ready.load(Ordering::SeqCst) == 0 {
        futex::wait(ready as *const AtomicU32 as usize, 0, MATCH_ANY, None);
    }
    drop(unsafe { Box::from_raw(start) });
    if let Err(code) = schedule(address, host, attr_address) {
        host_log::line(&format!("pthread_create sched_setscheduler failed: errno {code}"));
        unsafe {
            (*record).start_routine = 0;
            (*record).join_state = JOIN_DETACHED;
        }
        if flags & FLAG_DETACHED == 0 {
            unsafe { libc::pthread_detach(host) };
        }
        bionic_lock::unlock(address + HANDSHAKE_OFFSET);
        return code;
    }
    write_value::<u64>(thread_out, address as u64);
    bionic_lock::unlock(address + HANDSHAKE_OFFSET);
    0
}
