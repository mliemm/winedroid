use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::super::bridge::CallContext;
use super::abi_types::{EBADF, EFAULT, EINVAL};
use super::export::in_libc;
use super::fd_table::{self, FdEntry, FdKind};
use super::fs_result::{finish_int, from_host, missing};
use super::guest_mem::{read_value, write_value};
use super::linux_abi::{epoll as flags, open};

const EEXIST: i32 = 17;
const ENOENT: i32 = 2;

const EVENT_SIZE: usize = 16;

const INTEREST: u32 = flags::IN | flags::PRI | flags::OUT | flags::ERR | flags::HUP | flags::RDNORM | flags::RDBAND | flags::WRNORM | flags::WRBAND | flags::RDHUP | flags::EXCLUSIVE | flags::WAKEUP | flags::ONESHOT | flags::ET;

const READ_INTEREST: u32 = flags::IN | flags::RDNORM | flags::RDHUP;

const WRITE_INTEREST: u32 = flags::OUT | flags::WRNORM;

pub struct Registration {
    pub events: u32,
    pub data: u64,
    pub armed: bool,
}

pub struct EpollState {
    pub registrations: Mutex<HashMap<i32, Registration>>,
}

fn error(code: i32) -> i64 {
    -i64::from(code)
}

fn state_of(fd: i32) -> Result<Arc<EpollState>, i64> {
    let entry = fd_table::get(fd).ok_or(error(EBADF))?;
    if entry.kind != FdKind::Epoll {
        return Err(error(EINVAL));
    }
    entry.payload_as::<EpollState>().ok_or(error(EINVAL))
}

pub fn create(flags_value: i32) -> i64 {
    // cite: bionic sys_epoll.cpp#epoll_create1
    if flags_value & !flags::CLOEXEC != 0 {
        return error(EINVAL);
    }
    let queue = unsafe { libc::kqueue() };
    if queue < 0 {
        return from_host();
    }
    if flags_value & flags::CLOEXEC != 0 {
        unsafe { libc::fcntl(queue, libc::F_SETFD, libc::FD_CLOEXEC) };
    }
    let state = Arc::new(EpollState { registrations: Mutex::new(HashMap::new()) });
    let entry = FdEntry::new(FdKind::Epoll, "anon_inode:[eventpoll]", open::RDWR).with_payload(state);
    fd_table::adopt(queue, entry);
    i64::from(queue)
}

struct Change {
    filter: i16,
    flags: u16,
}

fn change(queue: i32, fd: i32, changes: &[Change]) -> Result<(), i32> {
    if changes.is_empty() {
        return Ok(());
    }
    let events: Vec<libc::kevent> = changes
        .iter()
        .map(|change| libc::kevent {
            ident: fd as libc::uintptr_t,
            filter: change.filter,
            flags: change.flags,
            fflags: 0,
            data: 0,
            udata: fd as *mut libc::c_void,
        })
        .collect();
    let status = unsafe { libc::kevent(queue, events.as_ptr(), events.len() as libc::c_int, std::ptr::null_mut(), 0, std::ptr::null()) };
    if status < 0 {
        Err((-from_host()) as i32)
    } else {
        Ok(())
    }
}

fn apply(queue: i32, fd: i32, events: u32, edge: bool, previous: Option<u32>) -> Result<(), i32> {
    let mut changes = Vec::new();
    let mut read_flags = libc::EV_ADD | libc::EV_ENABLE;
    if edge {
        read_flags |= libc::EV_CLEAR;
    }
    if events & READ_INTEREST != 0 {
        changes.push(Change { filter: libc::EVFILT_READ, flags: read_flags });
    } else if previous.is_some_and(|previous| previous & READ_INTEREST != 0) {
        changes.push(Change { filter: libc::EVFILT_READ, flags: libc::EV_DELETE });
    }
    let mut write_flags = libc::EV_ADD | libc::EV_ENABLE;
    if edge {
        write_flags |= libc::EV_CLEAR;
    }
    if events & WRITE_INTEREST != 0 {
        changes.push(Change { filter: libc::EVFILT_WRITE, flags: write_flags });
    } else if previous.is_some_and(|previous| previous & WRITE_INTEREST != 0) {
        changes.push(Change { filter: libc::EVFILT_WRITE, flags: libc::EV_DELETE });
    }
    change(queue, fd, &changes)
}

fn control(epfd: i32, operation: i32, fd: i32, event_pointer: usize) -> i64 {
    // cite: bionic epoll.h#epoll_ctl
    let state = match state_of(epfd) {
        Ok(state) => state,
        Err(failure) => return failure,
    };
    if fd == epfd || !fd_table::contains(fd) {
        return error(EBADF);
    }
    if !matches!(operation, flags::CTL_ADD | flags::CTL_DEL | flags::CTL_MOD) {
        return error(EINVAL);
    }
    if operation != flags::CTL_DEL && event_pointer == 0 {
        return error(EFAULT);
    }
    if operation == flags::CTL_DEL {
        let removed = state.registrations.lock().expect("epoll registrations").remove(&fd);
        let _ = change(epfd, fd, &[Change { filter: libc::EVFILT_READ, flags: libc::EV_DELETE }, Change { filter: libc::EVFILT_WRITE, flags: libc::EV_DELETE }]);
        return if removed.is_some() { 0 } else { error(ENOENT) };
    }
    let events = read_value::<u32>(event_pointer);
    let data = read_value::<u64>(event_pointer + 8);
    if events & !INTEREST != 0 {
        return error(EINVAL);
    }
    if events & flags::PRI != 0 {
        missing("epoll_ctl", "EPOLLPRI needs out of band data reporting kqueue does not provide");
        return error(EINVAL);
    }
    if events & flags::ET != 0 && events & flags::ONESHOT != 0 {
        return error(EINVAL);
    }
    let mut registrations = state.registrations.lock().expect("epoll registrations");
    let previous = registrations.get(&fd).map(|registration| registration.events);
    if operation == flags::CTL_ADD && previous.is_some() {
        return error(EEXIST);
    }
    if operation == flags::CTL_MOD && previous.is_none() {
        return error(ENOENT);
    }
    if let Err(errno) = apply(epfd, fd, events, events & flags::ET != 0, previous) {
        return error(errno);
    }
    registrations.insert(fd, Registration { events, data, armed: true });
    0
}

fn timeout_of(milliseconds: i64) -> libc::timespec {
    if milliseconds < 0 {
        return libc::timespec { tv_sec: 0, tv_nsec: 0 };
    }
    libc::timespec { tv_sec: milliseconds / 1000, tv_nsec: (milliseconds % 1000) * 1_000_000 }
}

fn event_flags(interest: u32, filter: i16, returned: u16) -> u32 {
    let mut events = 0;
    if returned & libc::EV_ERROR != 0 {
        return flags::ERR;
    }
    if filter == libc::EVFILT_READ {
        if interest & flags::IN != 0 {
            events |= flags::IN;
        }
        if interest & flags::RDNORM != 0 {
            events |= flags::RDNORM;
        }
    }
    if filter == libc::EVFILT_WRITE {
        if interest & flags::OUT != 0 {
            events |= flags::OUT;
        }
        if interest & flags::WRNORM != 0 {
            events |= flags::WRNORM;
        }
    }
    if returned & libc::EV_EOF != 0 {
        if filter == libc::EVFILT_READ && interest & flags::RDHUP != 0 {
            events |= flags::RDHUP;
        }
        events |= flags::HUP;
    }
    events
}

fn wait_impl(epfd: i32, events_pointer: usize, max_events: i32, timeout: Option<libc::timespec>, mask: usize) -> i64 {
    // cite: bionic sys_epoll.cpp#epoll_wait
    let state = match state_of(epfd) {
        Ok(state) => state,
        Err(failure) => return failure,
    };
    if max_events <= 0 {
        return error(EINVAL);
    }
    if events_pointer == 0 {
        return error(EFAULT);
    }
    if max_events as usize > 0x7fff_ffff / EVENT_SIZE {
        return error(EINVAL);
    }
    let previous = super::super::signal_state::mask();
    if mask != 0 {
        super::super::signal_state::set_mask(read_value::<u64>(mask));
    }
    let mut returned = vec![libc::kevent { ident: 0, filter: 0, flags: 0, fflags: 0, data: 0, udata: std::ptr::null_mut() }; max_events as usize];
    let timeout_pointer = timeout.as_ref().map_or(std::ptr::null(), |value| value as *const libc::timespec);
    let count = unsafe { libc::kevent(epfd, std::ptr::null(), 0, returned.as_mut_ptr(), max_events, timeout_pointer) };
    let failure = if count < 0 { Some(from_host()) } else { None };
    if mask != 0 {
        super::super::signal_state::set_mask(previous);
    }
    if let Some(failure) = failure {
        return failure;
    }
    let mut merged: Vec<(i32, u32)> = Vec::new();
    let mut oneshot = Vec::new();
    let mut registrations = state.registrations.lock().expect("epoll registrations");
    for entry in returned.iter().take(count as usize) {
        let fd = entry.udata as usize as i32;
        let Some(registration) = registrations.get(&fd) else { continue };
        if !registration.armed || !fd_table::contains(fd) {
            continue;
        }
        let events = event_flags(registration.events, entry.filter, entry.flags);
        if events == 0 {
            continue;
        }
        if let Some(existing) = merged.iter_mut().find(|(candidate, _)| *candidate == fd) {
            existing.1 |= events;
        } else {
            merged.push((fd, events));
        }
        if registration.events & flags::ONESHOT != 0 {
            oneshot.push(fd);
        }
    }
    let mut written = 0usize;
    for (fd, events) in merged.iter().take(max_events as usize) {
        let data = registrations.get(fd).map(|registration| registration.data).unwrap_or(0);
        write_value::<u32>(events_pointer + written * EVENT_SIZE, *events);
        write_value::<u64>(events_pointer + written * EVENT_SIZE + 8, data);
        written += 1;
    }
    for fd in oneshot {
        if let Some(registration) = registrations.get_mut(&fd) {
            registration.armed = false;
        }
        let _ = change(epfd, fd, &[Change { filter: libc::EVFILT_READ, flags: libc::EV_DELETE }, Change { filter: libc::EVFILT_WRITE, flags: libc::EV_DELETE }]);
    }
    written as i64
}

pub fn forget_closed(fd: i32) {
    for epfd in fd_table::list() {
        if let Some(entry) = fd_table::get(epfd) {
            if let Some(state) = entry.payload_as::<EpollState>() {
                state.registrations.lock().expect("epoll registrations").remove(&fd);
            }
        }
    }
}

fn handle(context: &mut CallContext, result: i64) {
    finish_int(context, result);
}

pub fn register() {
    in_libc("epoll_create", |context| handle(context, create(0)));
    in_libc("epoll_create1", |context| handle(context, create(context.arg_i32(0))));
    in_libc("epoll_ctl", |context| handle(context, control(context.arg_i32(0), context.arg_i32(1), context.arg_i32(2), context.arg_ptr(3))));
    in_libc("epoll_wait", |context| {
        let milliseconds = i64::from(context.arg_i32(3));
        let timeout = if milliseconds < 0 { None } else { Some(timeout_of(milliseconds)) };
        handle(context, wait_impl(context.arg_i32(0), context.arg_ptr(1), context.arg_i32(2), timeout, 0));
    });
    in_libc("epoll_pwait", |context| {
        let milliseconds = i64::from(context.arg_i32(3));
        let timeout = if milliseconds < 0 { None } else { Some(timeout_of(milliseconds)) };
        handle(context, wait_impl(context.arg_i32(0), context.arg_ptr(1), context.arg_i32(2), timeout, context.arg_ptr(4)));
    });
    in_libc("epoll_pwait2", |context| {
        let pointer = context.arg_ptr(3);
        let timeout = if pointer == 0 {
            None
        } else {
            let (seconds, nanos) = (read_value::<i64>(pointer), read_value::<i64>(pointer + 8));
            if seconds < 0 || !(0..1_000_000_000).contains(&nanos) {
                return handle(context, error(EINVAL));
            }
            Some(libc::timespec { tv_sec: seconds as libc::time_t, tv_nsec: nanos as libc::c_long })
        };
        handle(context, wait_impl(context.arg_i32(0), context.arg_ptr(1), context.arg_i32(2), timeout, context.arg_ptr(4)));
    });
}
