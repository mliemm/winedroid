use super::super::bridge::CallContext;
use super::abi_types::{EBADF, EFAULT, EINVAL};
use super::export::in_libc;
use super::fd_table::{self, FdEntry, FdKind};
use super::fdsan;
use super::fs_result::{finish_int, from_host, missing};
use super::guest_mem::write_value;
use super::linux_abi::{misc, open};

fn error(code: i32) -> i64 {
    -i64::from(code)
}

pub fn close_fd(fd: i32) -> i64 {
    // cite: bionic fdsan.cpp#close
    super::epoll::forget_closed(fd);
    fdsan::close_with_tag(fd, 0)
}

fn close_untagged(fd: i32) {
    if fd_table::take(fd).is_some() {
        unsafe { libc::close(fd) };
    }
}

fn close_range(first: u32, last: u32, flags: u32) -> i64 {
    // cite: bionic close_range.h#CLOSE_RANGE_CLOEXEC
    if first > last || flags & !(misc::CLOSE_RANGE_UNSHARE | misc::CLOSE_RANGE_CLOEXEC) != 0 {
        return error(EINVAL);
    }
    for fd in fd_table::list() {
        let number = fd as u32;
        if number < first || number > last {
            continue;
        }
        if flags & misc::CLOSE_RANGE_CLOEXEC != 0 {
            unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) };
        } else {
            close_untagged(fd);
        }
    }
    0
}

pub fn dup_fd(old: i32) -> i64 {
    let Some(entry) = fd_table::get(old) else { return error(EBADF) };
    let fd = unsafe { libc::dup(old) };
    if fd < 0 {
        return from_host();
    }
    fd_table::adopt_duplicate(&entry, fd, false);
    i64::from(fd)
}

fn replace_target(new: i32) -> Result<(), i64> {
    if fd_table::contains(new) {
        return Ok(());
    }
    if unsafe { libc::fcntl(new, libc::F_GETFD) } >= 0 {
        missing("dup2", &format!("descriptor {new} belongs to the host runtime"));
        return Err(error(EBADF));
    }
    Ok(())
}

fn dup_to(old: i32, new: i32, cloexec: bool) -> i64 {
    let Some(entry) = fd_table::get(old) else { return error(EBADF) };
    if let Err(failure) = replace_target(new) {
        return failure;
    }
    if unsafe { libc::dup2(old, new) } < 0 {
        return from_host();
    }
    if cloexec {
        unsafe { libc::fcntl(new, libc::F_SETFD, libc::FD_CLOEXEC) };
    }
    fd_table::adopt_duplicate(&entry, new, cloexec);
    i64::from(new)
}

pub fn pipe_with_flags(pointer: usize, flags: i32) -> i64 {
    // cite: bionic pipe.cpp#pipe2
    if pointer == 0 {
        return error(EFAULT);
    }
    if flags & !(open::CLOEXEC | open::NONBLOCK | open::DIRECT) != 0 {
        return error(EINVAL);
    }
    if flags & open::DIRECT != 0 {
        missing("pipe2", "O_DIRECT packet mode has no host pipe equivalent");
        return error(EINVAL);
    }
    let mut ends = [0i32; 2];
    if unsafe { libc::pipe(ends.as_mut_ptr()) } < 0 {
        return from_host();
    }
    for end in ends {
        if flags & open::CLOEXEC != 0 {
            unsafe { libc::fcntl(end, libc::F_SETFD, libc::FD_CLOEXEC) };
        }
        if flags & open::NONBLOCK != 0 {
            unsafe { libc::fcntl(end, libc::F_SETFL, libc::O_NONBLOCK) };
        }
    }
    let inherited = flags & (open::NONBLOCK | open::CLOEXEC);
    fd_table::adopt(ends[0], FdEntry::new(FdKind::Pipe, "pipe", open::RDONLY | (inherited & open::NONBLOCK)));
    fd_table::adopt(ends[1], FdEntry::new(FdKind::Pipe, "pipe", open::WRONLY | (inherited & open::NONBLOCK)));
    write_value::<i32>(pointer, ends[0]);
    write_value::<i32>(pointer + 4, ends[1]);
    0
}

fn handle(context: &mut CallContext, work: impl FnOnce(&CallContext) -> i64) {
    let result = work(context);
    finish_int(context, result);
}

pub fn register() {
    in_libc("close", |context| handle(context, |context| close_fd(context.arg_i32(0))));
    in_libc("close_range", |context| handle(context, |context| close_range(context.arg_u32(0), context.arg_u32(1), context.arg_u32(2))));
    in_libc("dup", |context| handle(context, |context| dup_fd(context.arg_i32(0))));
    in_libc("dup2", |context| {
        handle(context, |context| {
            // cite: bionic dup.cpp#dup2
            let (old, new) = (context.arg_i32(0), context.arg_i32(1));
            if old == new {
                return if unsafe { libc::fcntl(old, libc::F_GETFD) } < 0 || !fd_table::contains(old) { error(EBADF) } else { i64::from(new) };
            }
            dup_to(old, new, false)
        })
    });
    in_libc("dup3", |context| {
        handle(context, |context| {
            // cite: bionic dup.cpp#dup3
            let (old, new, flags) = (context.arg_i32(0), context.arg_i32(1), context.arg_i32(2));
            if old == new || flags & !open::CLOEXEC != 0 {
                return error(EINVAL);
            }
            dup_to(old, new, flags & open::CLOEXEC != 0)
        })
    });
    in_libc("pipe", |context| handle(context, |context| pipe_with_flags(context.arg_ptr(0), 0)));
    in_libc("pipe2", |context| handle(context, |context| pipe_with_flags(context.arg_ptr(0), context.arg_i32(1))));
}
