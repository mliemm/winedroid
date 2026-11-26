use std::any::Any;
use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};

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
static CLEAR_TIDS: Mutex<BTreeMap<usize, usize>> = Mutex::new(BTreeMap::new());

pub(super) fn hold_for_fork(held: &mut Vec<Box<dyn Any>>) {
    held.push(Box::new(
        HOSTS.lock().unwrap_or_else(PoisonError::into_inner),
    ));
    held.push(Box::new(
        NAMES.lock().unwrap_or_else(PoisonError::into_inner),
    ));
    held.push(Box::new(
        CLEAR_TIDS.lock().unwrap_or_else(PoisonError::into_inner),
    ));
}

pub fn remember(record: usize, handle: libc::pthread_t, created: bool) {
    HOSTS.lock().expect("thread hosts").insert(
        record,
        HostThread {
            handle: handle as usize,
            created,
        },
    );
}

pub fn forget(record: usize) {
    HOSTS.lock().expect("thread hosts").remove(&record);
    NAMES.lock().expect("thread names").remove(&record);
    CLEAR_TIDS.lock().expect("clear tids").remove(&record);
}

pub fn set_clear_tid(record: usize, address: usize) {
    // cite: bionic SYSCALLS.TXT#__set_tid_address
    let mut table = CLEAR_TIDS.lock().expect("clear tids");
    if address == 0 {
        table.remove(&record);
    } else {
        table.insert(record, address);
    }
}

pub fn take_clear_tid(record: usize) -> Option<usize> {
    CLEAR_TIDS.lock().expect("clear tids").remove(&record)
}

pub fn host_of(record: usize) -> Option<libc::pthread_t> {
    HOSTS
        .lock()
        .expect("thread hosts")
        .get(&record)
        .map(|host| host.handle as libc::pthread_t)
}

pub fn was_created_by_guest(record: usize) -> bool {
    HOSTS
        .lock()
        .expect("thread hosts")
        .get(&record)
        .is_some_and(|host| host.created)
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
            fatal(
                None,
                &format!("invalid pthread_t {thread:#x} passed to {caller}"),
            );
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
    NAMES
        .lock()
        .expect("thread names")
        .get(&record)
        .copied()
        .unwrap_or_else(|| {
            let mut comm = [0u8; COMM_BYTES];
            let program = PROGRAM_NAME.as_bytes();
            comm[..program.len().min(COMM_BYTES - 1)]
                .copy_from_slice(&program[..program.len().min(COMM_BYTES - 1)]);
            comm
        })
}

pub fn inherit_name(parent: usize, child: usize) {
    let name = name_of(parent);
    NAMES.lock().expect("thread names").insert(child, name);
}

fn first_other(hosts: &BTreeMap<usize, HostThread>, me: usize) -> Option<libc::pthread_t> {
    hosts
        .values()
        .map(|host| host.handle)
        .find(|handle| *handle != me)
        .map(|handle| handle as libc::pthread_t)
}

pub fn try_other_host() -> Option<libc::pthread_t> {
    let me = unsafe { libc::pthread_self() } as usize;
    let hosts = HOSTS.try_lock().ok()?;
    first_other(&hosts, me)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::test_deadline::bounded_test;

    fn hosts(handles: &[usize]) -> BTreeMap<usize, HostThread> {
        handles
            .iter()
            .enumerate()
            .map(|(index, handle)| {
                (
                    index + 1,
                    HostThread {
                        handle: *handle,
                        created: false,
                    },
                )
            })
            .collect()
    }

    bounded_test! {
        fn the_forwarding_target_is_the_first_attached_thread_other_than_the_caller() {
            // cite: bionic SYSCALLS.TXT#kill
            let table = hosts(&[100, 200, 300]);

            assert_eq!(first_other(&table, 100), Some(200 as libc::pthread_t));
            assert_eq!(first_other(&table, 200), Some(100 as libc::pthread_t));
            assert_eq!(first_other(&table, 999), Some(100 as libc::pthread_t));
        }
    }

    bounded_test! {
        fn there_is_no_forwarding_target_when_the_caller_is_the_only_attached_thread_or_none_is_attached() {
            // cite: bionic SYSCALLS.TXT#kill
            assert_eq!(first_other(&hosts(&[100]), 100), None);
            assert_eq!(first_other(&hosts(&[]), 100), None);
        }
    }

    bounded_test! {
        fn a_clear_tid_address_is_taken_once_and_forgetting_the_record_drops_it() {
            // cite: bionic SYSCALLS.TXT#__set_tid_address
            set_clear_tid(0x7001, 0x40);
            set_clear_tid(0x7002, 0x44);

            assert_eq!(take_clear_tid(0x7001), Some(0x40));
            assert_eq!(take_clear_tid(0x7001), None);
            forget(0x7002);
            assert_eq!(take_clear_tid(0x7002), None);
            set_clear_tid(0x7003, 0x48);
            set_clear_tid(0x7003, 0);
            assert_eq!(take_clear_tid(0x7003), None, "a null address clears the registration");
        }
    }
}
