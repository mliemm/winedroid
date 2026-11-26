use std::sync::{Arc, Condvar, Mutex};

use super::abi_types::{EAGAIN, EFAULT, EINVAL};
use super::doorbell::Doorbell;
use super::export::in_libc;
use super::fd_table::{self, FdEntry, FdKind, SyntheticStat};
use super::fs_result::finish_int;
use super::guest_mem::{read_value, write_value};
use super::linux_abi::{epoll as flags_of, open};
use super::stat_convert::S_IFREG;

const MAX_COUNTER: u64 = u64::MAX - 1;

pub struct EventFdState {
    bell: Doorbell,
    counter: Mutex<u64>,
    changed: Condvar,
    semaphore: bool,
}

fn error(code: i32) -> i64 {
    -i64::from(code)
}

pub fn create(initial: u32, flags: i32) -> i64 {
    // cite: bionic eventfd.h#EFD_SEMAPHORE
    if flags & !(flags_of::EFD_SEMAPHORE | flags_of::EFD_NONBLOCK | flags_of::EFD_CLOEXEC) != 0 {
        return error(EINVAL);
    }
    let bell = match Doorbell::new() {
        Ok(bell) => bell,
        Err(failure) => return failure,
    };
    let fd = bell.guest;
    if flags & flags_of::EFD_CLOEXEC != 0 {
        unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) };
    }
    if initial > 0 {
        bell.ring();
    }
    let state = Arc::new(EventFdState {
        bell,
        counter: Mutex::new(u64::from(initial)),
        changed: Condvar::new(),
        semaphore: flags & flags_of::EFD_SEMAPHORE != 0,
    });
    let guest_flags = open::RDWR
        | if flags & flags_of::EFD_NONBLOCK != 0 {
            open::NONBLOCK
        } else {
            0
        };
    let entry = FdEntry::new(FdKind::EventFd, "anon_inode:[eventfd]", guest_flags)
        .with_synthetic(SyntheticStat {
            mode: S_IFREG | 0o600,
            dev: (0, 0xe),
            rdev: (0, 0),
            zero_size: false,
        })
        .with_payload(state);
    fd_table::adopt(fd, entry);
    i64::from(fd)
}

pub fn read(entry: &FdEntry, buffer: usize, length: usize) -> i64 {
    // cite: bionic eventfd.cpp#eventfd_read
    if length < 8 {
        return error(EINVAL);
    }
    if buffer == 0 {
        return error(EFAULT);
    }
    let Some(state) = entry.payload_as::<EventFdState>() else {
        return error(EINVAL);
    };
    loop {
        let mut counter = state.counter.lock().expect("eventfd counter");
        if *counter > 0 {
            let taken = if state.semaphore { 1 } else { *counter };
            *counter -= taken;
            if *counter == 0 {
                state.bell.drain();
            }
            state.changed.notify_all();
            write_value::<u64>(buffer, taken);
            return 8;
        }
        drop(counter);
        if entry.flags() & open::NONBLOCK != 0 {
            return error(EAGAIN);
        }
        let waited = state.bell.wait_readable();
        if waited < 0 {
            return waited;
        }
    }
}

pub fn write(entry: &FdEntry, buffer: usize, length: usize) -> i64 {
    if length != 8 {
        return error(EINVAL);
    }
    if buffer == 0 {
        return error(EFAULT);
    }
    let Some(state) = entry.payload_as::<EventFdState>() else {
        return error(EINVAL);
    };
    let value = read_value::<u64>(buffer);
    if value == u64::MAX {
        return error(EINVAL);
    }
    let mut counter = state.counter.lock().expect("eventfd counter");
    while *counter > MAX_COUNTER - value {
        if entry.flags() & open::NONBLOCK != 0 {
            return error(EAGAIN);
        }
        counter = state.changed.wait(counter).expect("eventfd counter");
    }
    *counter += value;
    if value > 0 {
        state.bell.ring();
    }
    8
}

pub fn register() {
    in_libc("eventfd", |context| {
        let result = create(context.arg_u32(0), context.arg_i32(1));
        finish_int(context, result);
    });
    in_libc("eventfd_read", |context| {
        // cite: bionic eventfd.cpp#eventfd_read
        let (fd, destination) = (context.arg_i32(0), context.arg_ptr(1));
        let result = super::fs_io::read_fd(fd, destination, 8);
        if result == 8 {
            context.ret_i32(0);
        } else {
            if result < 0 {
                context.set_errno((-result) as i32);
            }
            context.ret_i32(-1);
        }
    });
    in_libc("eventfd_write", |context| {
        // cite: bionic eventfd.cpp#eventfd_write
        let (fd, value) = (context.arg_i32(0), context.arg_u64(1));
        let cell = Box::new(value);
        let result = super::fs_io::write_fd(fd, &*cell as *const u64 as usize, 8);
        if result == 8 {
            context.ret_i32(0);
        } else {
            if result < 0 {
                context.set_errno((-result) as i32);
            }
            context.ret_i32(-1);
        }
    });
}
