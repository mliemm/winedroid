use std::collections::BTreeMap;
use std::sync::Mutex;

use super::bionic::device_profile::PROGRAM_NAME;
use super::bionic::fatal::fatal;
use super::bionic::sdk::application_target_sdk;
use super::guest_thread::is_live_address;
use super::host_log;

const INVALID_THREAD_FATAL_SDK: i32 = 26;
const COMM_BYTES: usize = 16;

#[derive(Clone, Copy)]
struct HostThread {
    handle: usize,
    created: bool,
}

static HOSTS: Mutex<BTreeMap<usize, HostThread>> = Mutex::new(BTreeMap::new());
static NAMES: Mutex<BTreeMap<usize, [u8; COMM_BYTES]>> = Mutex::new(BTreeMap::new());

pub fn remember(record: usize, handle: libc::pthread_t, created: bool) {
    HOSTS.lock().expect("thread hosts").insert(record, HostThread { handle: handle as usize, created });
}

pub fn forget(record: usize) {
    HOSTS.lock().expect("thread hosts").remove(&record);
    NAMES.lock().expect("thread names").remove(&record);
}

pub fn host_of(record: usize) -> Option<libc::pthread_t> {
    HOSTS.lock().expect("thread hosts").get(&record).map(|host| host.handle as libc::pthread_t)
}

pub fn was_created_by_guest(record: usize) -> bool {
    HOSTS.lock().expect("thread hosts").get(&record).is_some_and(|host| host.created)
}

pub fn lookup(thread: usize) -> Option<usize> {
    is_live_address(thread).then_some(thread)
}

pub fn find(thread: usize, caller: &str) -> Option<usize> {
    // cite: bionic pthread_internal.cpp#__pthread_internal_find
    if let Some(found) = lookup(thread) {
        return Some(found);
    }
    if application_target_sdk() >= INVALID_THREAD_FATAL_SDK {
        if thread == 0 {
            host_log::line(&format!("invalid pthread_t (0) passed to {caller}"));
        } else {
            fatal(None, &format!("invalid pthread_t {thread:#x} passed to {caller}"));
        }
    }
    None
}

pub fn set_name(record: usize, name: &[u8]) {
    let mut comm = [0u8; COMM_BYTES];
    comm[..name.len().min(COMM_BYTES - 1)].copy_from_slice(&name[..name.len().min(COMM_BYTES - 1)]);
    NAMES.lock().expect("thread names").insert(record, comm);
}

pub fn name_of(record: usize) -> [u8; COMM_BYTES] {
    NAMES.lock().expect("thread names").get(&record).copied().unwrap_or_else(|| {
        let mut comm = [0u8; COMM_BYTES];
        let program = PROGRAM_NAME.as_bytes();
        comm[..program.len().min(COMM_BYTES - 1)].copy_from_slice(&program[..program.len().min(COMM_BYTES - 1)]);
        comm
    })
}

pub fn inherit_name(parent: usize, child: usize) {
    let name = name_of(parent);
    NAMES.lock().expect("thread names").insert(child, name);
}

pub fn try_other_host() -> Option<libc::pthread_t> {
    let me = unsafe { libc::pthread_self() } as usize;
    HOSTS.try_lock().ok()?.values().map(|host| host.handle).find(|handle| *handle != me).map(|handle| handle as libc::pthread_t)
}
