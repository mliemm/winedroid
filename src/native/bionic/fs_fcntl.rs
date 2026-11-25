use std::sync::atomic::Ordering;

use super::super::bridge::CallContext;
use super::abi_types::{EBADF, EFAULT, EINVAL};
use super::export::in_libc;
use super::fatal::fortify_fatal;
use super::fd_table::{self, FdEntry, FdKind, SEALING_ALLOWED};
use super::fs_result::{finish_int, from_host, missing};
use super::guest_mem::{read_value, write_bytes, write_value};
use super::linux_abi::{fcntl, ioctl, open};
use super::translate::{lock_type_from_host, lock_type_to_host, status_flags_from_host, status_flags_to_host, DARWIN_F_NOCACHE};

const EPERM: i32 = 1;
const ENOTTY: i32 = 25;
const EWOULDBLOCK: i32 = 11;
const FLOCK_SIZE: usize = 32;
const SEAL_MASK: u32 = 0x1f;
const SETTABLE_STATUS: i32 = open::APPEND | open::NONBLOCK | open::FASYNC | open::DIRECT | open::NOATIME;
const HOST_PIPE_CAPACITY: i64 = 16384;

fn error(code: i32) -> i64 {
    -i64::from(code)
}

fn host_status(result: i32) -> i64 {
    if result < 0 {
        from_host()
    } else {
        i64::from(result)
    }
}

fn is_doorbell(entry: &FdEntry) -> bool {
    matches!(entry.kind, FdKind::EventFd | FdKind::TimerFd)
}

fn duplicate_with(fd: i32, entry: &FdEntry, host_command: i32, minimum: i32) -> i64 {
    let new = unsafe { libc::fcntl(fd, host_command, minimum) };
    if new < 0 {
        return from_host();
    }
    fd_table::adopt_duplicate(entry, new, host_command == libc::F_DUPFD_CLOEXEC);
    i64::from(new)
}

fn current_flags(fd: i32, entry: &FdEntry) -> i64 {
    if is_doorbell(entry) {
        return i64::from(entry.flags());
    }
    let host = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if host < 0 {
        return from_host();
    }
    let live = open::APPEND | open::NONBLOCK | open::FASYNC;
    i64::from((entry.flags() & !live) | status_flags_from_host(host))
}

fn set_flags(fd: i32, entry: &FdEntry, linux: i32) -> i64 {
    let wanted = linux & SETTABLE_STATUS;
    if is_doorbell(entry) {
        entry.linux_flags.store((entry.flags() & !SETTABLE_STATUS) | wanted, Ordering::SeqCst);
        return 0;
    }
    let host = status_flags_to_host(wanted);
    if unsafe { libc::fcntl(fd, libc::F_SETFL, host) } < 0 {
        return from_host();
    }
    if (wanted ^ entry.flags()) & open::DIRECT != 0 {
        unsafe { libc::fcntl(fd, DARWIN_F_NOCACHE, i32::from(wanted & open::DIRECT != 0)) };
    }
    let kept = entry.flags() & !SETTABLE_STATUS;
    entry.linux_flags.store(kept | wanted, Ordering::SeqCst);
    0
}

fn lock(fd: i32, linux_command: i32, pointer: usize) -> i64 {
    if pointer == 0 {
        return error(EFAULT);
    }
    let host_command = match linux_command {
        fcntl::GETLK => libc::F_GETLK,
        fcntl::SETLK => libc::F_SETLK,
        fcntl::SETLKW => libc::F_SETLKW,
        fcntl::OFD_GETLK => libc::F_OFD_GETLK,
        fcntl::OFD_SETLK => libc::F_OFD_SETLK,
        _ => libc::F_OFD_SETLKW,
    };
    let Some(kind) = lock_type_to_host(read_value::<i16>(pointer)) else { return error(EINVAL) };
    let mut request: libc::flock = unsafe { std::mem::zeroed() };
    request.l_type = kind;
    request.l_whence = read_value::<i16>(pointer + 2);
    request.l_start = read_value::<i64>(pointer + 8);
    request.l_len = read_value::<i64>(pointer + 16);
    request.l_pid = if matches!(linux_command, fcntl::OFD_GETLK | fcntl::OFD_SETLK | fcntl::OFD_SETLKW) { 0 } else { read_value::<i32>(pointer + 24) };
    if unsafe { libc::fcntl(fd, host_command, &mut request) } < 0 {
        return from_host();
    }
    if matches!(linux_command, fcntl::GETLK | fcntl::OFD_GETLK) {
        let mut out = [0u8; FLOCK_SIZE];
        out[0..2].copy_from_slice(&lock_type_from_host(request.l_type).to_le_bytes());
        out[2..4].copy_from_slice(&request.l_whence.to_le_bytes());
        out[8..16].copy_from_slice(&request.l_start.to_le_bytes());
        out[16..24].copy_from_slice(&request.l_len.to_le_bytes());
        out[24..28].copy_from_slice(&request.l_pid.to_le_bytes());
        write_bytes(pointer, &out);
    }
    0
}

fn seals(entry: &FdEntry, command: i32, argument: u64) -> i64 {
    let current = entry.seals.load(Ordering::SeqCst);
    if current & SEALING_ALLOWED == 0 {
        return error(EINVAL);
    }
    if command == fcntl::GET_SEALS {
        return i64::from(current & SEAL_MASK);
    }
    if current & fcntl::SEAL_SEAL != 0 {
        return error(EPERM);
    }
    entry.seals.fetch_or(argument as u32 & SEAL_MASK, Ordering::SeqCst);
    0
}

pub fn control(fd: i32, command: i32, argument: u64) -> i64 {
    // cite: bionic fcntl.cpp#fcntl
    let Some(entry) = fd_table::get(fd) else { return error(EBADF) };
    match command {
        fcntl::DUPFD => duplicate_with(fd, &entry, libc::F_DUPFD, argument as i32),
        fcntl::DUPFD_CLOEXEC => duplicate_with(fd, &entry, libc::F_DUPFD_CLOEXEC, argument as i32),
        fcntl::GETFD => host_status(unsafe { libc::fcntl(fd, libc::F_GETFD) }),
        fcntl::SETFD => {
            if argument & !(fcntl::FD_CLOEXEC as u64) != 0 {
                fortify_fatal(&format!("fcntl(F_SETFD) passed non-FD_CLOEXEC flag: {argument:#x}"));
            }
            host_status(unsafe { libc::fcntl(fd, libc::F_SETFD, argument as i32) })
        }
        fcntl::GETFL => current_flags(fd, &entry),
        fcntl::SETFL => set_flags(fd, &entry, argument as i32),
        fcntl::GETLK | fcntl::SETLK | fcntl::SETLKW | fcntl::OFD_GETLK | fcntl::OFD_SETLK | fcntl::OFD_SETLKW => lock(fd, command, argument as usize),
        fcntl::SETOWN => host_status(unsafe { libc::fcntl(fd, libc::F_SETOWN, argument as i32) }),
        fcntl::GETOWN => host_status(unsafe { libc::fcntl(fd, libc::F_GETOWN) }),
        fcntl::ADD_SEALS | fcntl::GET_SEALS => seals(&entry, command, argument),
        fcntl::GETPIPE_SZ if entry.kind == FdKind::Pipe => HOST_PIPE_CAPACITY,
        other => {
            missing("fcntl", &format!("command {other} has no host equivalent"));
            error(EINVAL)
        }
    }
}

fn control_result(context: &mut CallContext, result: i64) {
    finish_int(context, result);
}

fn flock_fd(fd: i32, operation: i32) -> i64 {
    if !fd_table::contains(fd) {
        return error(EBADF);
    }
    if operation & !(fcntl::LOCK_SH | fcntl::LOCK_EX | fcntl::LOCK_UN | fcntl::LOCK_NB) != 0 {
        return error(EINVAL);
    }
    if unsafe { libc::flock(fd, operation) } < 0 {
        let failure = from_host();
        if failure == error(EWOULDBLOCK) {
            return failure;
        }
        return failure;
    }
    0
}

fn lockf_fd(fd: i32, command: i32, length: i64) -> i64 {
    if !fd_table::contains(fd) {
        return error(EBADF);
    }
    if unsafe { libc::lockf(fd, command, length) } < 0 {
        from_host()
    } else {
        0
    }
}

pub fn ioctl_fd(fd: i32, request: u32, argument: usize) -> i64 {
    // cite: bionic ioctls.h#FIONREAD
    let Some(entry) = fd_table::get(fd) else { return error(EBADF) };
    match request {
        ioctl::FIONREAD => {
            if argument == 0 {
                return error(EFAULT);
            }
            let mut pending: libc::c_int = 0;
            if unsafe { libc::ioctl(fd, libc::FIONREAD, &mut pending) } < 0 {
                return from_host();
            }
            write_value::<i32>(argument, pending);
            0
        }
        ioctl::FIONBIO => {
            if argument == 0 {
                return error(EFAULT);
            }
            let enable = read_value::<i32>(argument) != 0;
            let current = entry.flags();
            let wanted = if enable { current | open::NONBLOCK } else { current & !open::NONBLOCK };
            set_flags(fd, &entry, wanted)
        }
        ioctl::FIOCLEX => host_status(unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) }),
        ioctl::FIONCLEX => host_status(unsafe { libc::fcntl(fd, libc::F_SETFD, 0) }),
        ioctl::TIOCGWINSZ => {
            if argument == 0 {
                return error(EFAULT);
            }
            let mut size: libc::winsize = unsafe { std::mem::zeroed() };
            if unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, &mut size) } < 0 {
                return from_host();
            }
            for (index, value) in [size.ws_row, size.ws_col, size.ws_xpixel, size.ws_ypixel].into_iter().enumerate() {
                write_value::<u16>(argument + index * 2, value);
            }
            0
        }
        other => match super::net_ioctl::request(&entry, fd, other, argument) {
            Some(result) => result,
            None => {
                missing("ioctl", &format!("request {other:#x} has no translation"));
                error(if entry.kind == FdKind::Socket { EINVAL } else { ENOTTY })
            }
        },
    }
}

pub fn register() {
    in_libc("fcntl", |context| {
        let result = control(context.arg_i32(0), context.arg_i32(1), context.arg_u64(2));
        control_result(context, result);
    });
    in_libc("ioctl", |context| {
        let result = ioctl_fd(context.arg_i32(0), context.arg_u32(1), context.arg_ptr(2));
        control_result(context, result);
    });
    in_libc("flock", |context| {
        let result = flock_fd(context.arg_i32(0), context.arg_i32(1));
        finish_int(context, result);
    });
    for name in ["lockf", "lockf64"] {
        in_libc(name, |context| {
            let result = lockf_fd(context.arg_i32(0), context.arg_i32(1), context.arg_u64(2) as i64);
            finish_int(context, result);
        });
    }
}
