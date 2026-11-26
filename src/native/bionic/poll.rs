use super::super::bridge::CallContext;
use super::abi_types::{EBADF, EFAULT, EINVAL};
use super::export::in_libc;
use super::fd_table;
use super::fs_result::{finish_int, from_host};
use super::guest_mem::{read_value, write_bytes, write_value};
use super::linux_abi::poll as events;

const POLL_SET_SIZE: usize = 128;
const FD_LIMIT: usize = POLL_SET_SIZE * 8;

fn error(code: i32) -> i64 {
    -i64::from(code)
}

fn event_to_host(linux: i16) -> i16 {
    // cite: bionic poll.h#POLLIN, POLLPRI, POLLOUT, POLLWRBAND, POLLRDHUP
    let mut host = 0;
    for (from, to) in [
        (events::IN, libc::POLLIN),
        (events::PRI, libc::POLLPRI),
        (events::OUT, libc::POLLOUT),
        (events::ERR, libc::POLLERR),
        (events::HUP, libc::POLLHUP),
        (events::NVAL, libc::POLLNVAL),
        (events::RDNORM, libc::POLLRDNORM),
        (events::RDBAND, libc::POLLRDBAND),
        (events::WRNORM, libc::POLLWRNORM),
    ] {
        if linux & from != 0 {
            host |= to;
        }
    }
    if linux & events::WRBAND != 0 {
        host |= libc::POLLWRBAND;
    }
    host
}

fn event_from_host(host: i16) -> i16 {
    let mut linux = 0;
    for (from, to) in [
        (libc::POLLIN, events::IN),
        (libc::POLLPRI, events::PRI),
        (libc::POLLOUT, events::OUT),
        (libc::POLLERR, events::ERR),
        (libc::POLLHUP, events::HUP),
        (libc::POLLNVAL, events::NVAL),
        (libc::POLLRDNORM, events::RDNORM),
        (libc::POLLRDBAND, events::RDBAND),
        (libc::POLLWRNORM, events::WRNORM),
        (libc::POLLWRBAND, events::WRBAND),
    ] {
        if host & from != 0 {
            linux |= to;
        }
    }
    linux
}

fn poll_from_guest(pointer: usize, count: usize) -> Result<Vec<libc::pollfd>, i32> {
    if count > 0 && pointer == 0 {
        return Err(EFAULT);
    }
    let mut list = Vec::with_capacity(count);
    for index in 0..count {
        let address = pointer + index * 8;
        let fd = read_value::<i32>(address);
        let requested = read_value::<i16>(address + 4);
        let usable = fd >= 0 && fd_table::contains(fd);
        list.push(libc::pollfd {
            fd: if usable { fd } else { -1 },
            events: event_to_host(requested),
            revents: 0,
        });
        if !usable && fd >= 0 {
            write_value::<i16>(address + 6, events::NVAL);
        }
    }
    Ok(list)
}

fn poll_impl(pointer: usize, count: i64, timeout: i32) -> i64 {
    // cite: bionic poll.cpp#poll
    if count < 0 || count as u64 > i64::MAX as u64 / 8 {
        return error(EINVAL);
    }
    let count = count as usize;
    let mut list = match poll_from_guest(pointer, count) {
        Ok(list) => list,
        Err(errno) => return error(errno),
    };
    let status = unsafe { libc::poll(list.as_mut_ptr(), count as libc::nfds_t, timeout) };
    if status < 0 {
        return from_host();
    }
    for (index, entry) in list.iter().enumerate() {
        let address = pointer + index * 8;
        let fd = read_value::<i32>(address);
        if fd < 0 {
            continue;
        }
        if entry.fd < 0 {
            continue;
        }
        write_value::<i16>(address + 6, event_from_host(entry.revents));
    }
    status as i64
}

fn ppoll_impl(pointer: usize, count: i64, timeout_pointer: usize, mask_pointer: usize) -> i64 {
    // cite: bionic poll.cpp#ppoll
    let previous = super::super::signal_state::mask();
    if mask_pointer != 0 {
        super::super::signal_state::set_mask(read_value::<u64>(mask_pointer));
    }
    let timeout = if timeout_pointer == 0 {
        -1
    } else {
        let (seconds, nanos) = (
            read_value::<i64>(timeout_pointer),
            read_value::<i64>(timeout_pointer + 8),
        );
        if seconds < 0 || !(0..1_000_000_000).contains(&nanos) {
            super::super::signal_state::set_mask(previous);
            return error(EINVAL);
        }
        let total = seconds
            .saturating_mul(1000)
            .saturating_add((nanos + 999_999) / 1_000_000);
        i32::try_from(total).unwrap_or(i32::MAX)
    };
    let status = poll_impl(pointer, count, timeout);
    if mask_pointer != 0 {
        super::super::signal_state::set_mask(previous);
    }
    status
}

fn set_contains(set: usize, fd: usize) -> bool {
    read_value::<u32>(set + (fd / 32) * 4) & (1u32 << (fd % 32)) != 0
}

fn set_add(set: usize, fd: usize) {
    let address = set + (fd / 32) * 4;
    write_value::<u32>(address, read_value::<u32>(address) | (1u32 << (fd % 32)));
}

fn set_clear_all(set: usize) {
    write_bytes(set, &[0u8; POLL_SET_SIZE]);
}

fn build_host_set(source: usize, target: usize, count: usize) -> Result<(), i64> {
    set_clear_all(target);
    for fd in 0..count {
        if set_contains(source, fd) {
            if !fd_table::contains(fd as i32) {
                return Err(error(EBADF));
            }
            set_add(target, fd);
        }
    }
    Ok(())
}

fn copy_back(source: usize, target: usize, count: usize) {
    set_clear_all(target);
    for fd in 0..count {
        if set_contains(source, fd) {
            set_add(target, fd);
        }
    }
}

fn select_impl(
    count: i32,
    read: usize,
    write: usize,
    except: usize,
    timeout: usize,
    mask: usize,
    timespec_timeout: bool,
) -> i64 {
    // cite: bionic poll.cpp#select
    if count < 0 || count as usize > FD_LIMIT {
        return error(EINVAL);
    }
    let count = count as usize;
    let mut host_read = [0u8; POLL_SET_SIZE];
    let mut host_write = [0u8; POLL_SET_SIZE];
    let mut host_except = [0u8; POLL_SET_SIZE];
    let (read_pointer, write_pointer, except_pointer) = (
        host_read.as_mut_ptr() as usize,
        host_write.as_mut_ptr() as usize,
        host_except.as_mut_ptr() as usize,
    );
    for (source, target) in [
        (read, read_pointer),
        (write, write_pointer),
        (except, except_pointer),
    ] {
        if source != 0 {
            if let Err(failure) = build_host_set(source, target, count) {
                return failure;
            }
        }
    }
    let mut host_timeval: libc::timeval = unsafe { std::mem::zeroed() };
    let mut host_timespec: libc::timespec = unsafe { std::mem::zeroed() };
    let timeout_pointer = if timeout == 0 {
        std::ptr::null_mut()
    } else if timespec_timeout {
        let (seconds, nanos) = (read_value::<i64>(timeout), read_value::<i64>(timeout + 8));
        if seconds < 0 || !(0..1_000_000_000).contains(&nanos) {
            return error(EINVAL);
        }
        host_timespec.tv_sec = seconds as libc::time_t;
        host_timespec.tv_nsec = nanos as libc::c_long;
        &mut host_timespec as *mut libc::timespec as *mut libc::c_void
    } else {
        let (seconds, micros) = (read_value::<i64>(timeout), read_value::<i64>(timeout + 8));
        if seconds < 0 || !(0..1_000_000).contains(&micros) {
            return error(EINVAL);
        }
        host_timeval.tv_sec = seconds as libc::time_t;
        host_timeval.tv_usec = micros as libc::suseconds_t;
        &mut host_timeval as *mut libc::timeval as *mut libc::c_void
    };
    let previous = super::super::signal_state::mask();
    if mask != 0 {
        super::super::signal_state::set_mask(read_value::<u64>(mask));
    }
    let status = unsafe {
        libc::select(
            count as libc::c_int,
            if read != 0 {
                read_pointer as *mut libc::fd_set
            } else {
                std::ptr::null_mut()
            },
            if write != 0 {
                write_pointer as *mut libc::fd_set
            } else {
                std::ptr::null_mut()
            },
            if except != 0 {
                except_pointer as *mut libc::fd_set
            } else {
                std::ptr::null_mut()
            },
            timeout_pointer as *mut libc::timeval,
        )
    };
    if mask != 0 {
        super::super::signal_state::set_mask(previous);
    }
    if status < 0 {
        return from_host();
    }
    if status == 0 {
        for source in [read, write, except] {
            if source != 0 {
                set_clear_all(source);
            }
        }
        return 0;
    }
    for (source, target) in [
        (read, read_pointer),
        (write, write_pointer),
        (except, except_pointer),
    ] {
        if source != 0 {
            copy_back(target, source, count);
        }
    }
    status as i64
}

pub fn register() {
    for (name, ppoll) in [("poll", false), ("ppoll", true)] {
        in_libc(name, move |context: &mut CallContext| {
            let pointer = context.arg_ptr(0);
            let count = context.arg_u64(1) as i64;
            let result = if ppoll {
                ppoll_impl(pointer, count, context.arg_ptr(2), context.arg_ptr(3))
            } else {
                poll_impl(pointer, count, context.arg_i32(2))
            };
            finish_int(context, result);
        });
    }
    for (name, masked) in [("select", false), ("pselect", true), ("pselect64", true)] {
        in_libc(name, move |context: &mut CallContext| {
            let result = select_impl(
                context.arg_i32(0),
                context.arg_ptr(1),
                context.arg_ptr(2),
                context.arg_ptr(3),
                context.arg_ptr(4),
                if masked { context.arg_ptr(5) } else { 0 },
                masked,
            );
            finish_int(context, result);
        });
    }
}
