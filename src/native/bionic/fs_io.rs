use std::sync::atomic::Ordering;
use std::sync::Arc;

use super::abi_types::{EBADF, EFAULT, EINVAL, PAGE_SIZE};
use super::export::in_libc;
use super::fd_table::{self, FdEntry, FdKind, SEALING_ALLOWED};
use super::fs_result::{finish_int, finish_long, from_host, missing};
use super::guest_mem::{read_value, write_value};
use super::linux_abi::{fcntl, misc, open, seek};
use super::translate::{seek_to_host, DARWIN_F_FULLFSYNC};

const EISDIR: i32 = 21;
const ESPIPE: i32 = 29;
const EPERM: i32 = 1;
const ENOTTY: i32 = 25;
const EOPNOTSUPP: i32 = 95;
const MAX_RW_COUNT: usize = isize::MAX as usize & !(PAGE_SIZE - 1);
const COPY_CHUNK: usize = 64 * 1024;

fn error(code: i32) -> i64 {
    -i64::from(code)
}

pub fn entry_of(fd: i32) -> Result<Arc<FdEntry>, i64> {
    fd_table::get(fd).ok_or(error(EBADF))
}

fn readable(entry: &FdEntry) -> bool {
    entry.flags() & open::ACCMODE != open::WRONLY
}

fn writable(entry: &FdEntry) -> bool {
    entry.flags() & open::ACCMODE != open::RDONLY
}

fn without_sigpipe(entry: &FdEntry, work: impl FnOnce() -> i64) -> i64 {
    if entry.kind != FdKind::Pipe {
        return work();
    }
    let mut blocked: libc::sigset_t = unsafe { std::mem::zeroed() };
    let mut previous: libc::sigset_t = unsafe { std::mem::zeroed() };
    unsafe {
        libc::sigemptyset(&mut blocked);
        libc::sigaddset(&mut blocked, libc::SIGPIPE);
        libc::pthread_sigmask(libc::SIG_BLOCK, &blocked, &mut previous);
    }
    let result = work();
    if result == error(32) {
        let mut pending: libc::sigset_t = unsafe { std::mem::zeroed() };
        let mut received = 0;
        unsafe {
            libc::sigpending(&mut pending);
            if libc::sigismember(&pending, libc::SIGPIPE) == 1 {
                libc::sigwait(&blocked, &mut received);
            }
        }
    }
    unsafe { libc::pthread_sigmask(libc::SIG_SETMASK, &previous, std::ptr::null_mut()) };
    result
}

pub fn read_fd(fd: i32, buffer: usize, count: usize) -> i64 {
    let entry = match entry_of(fd) {
        Ok(entry) => entry,
        Err(failure) => return failure,
    };
    if !readable(&entry) {
        return error(EBADF);
    }
    match entry.kind {
        FdKind::Dir => return error(EISDIR),
        FdKind::PathOnly => return error(EBADF),
        FdKind::Epoll => return error(EINVAL),
        FdKind::EventFd => return super::eventfd::read(&entry, buffer, count),
        FdKind::TimerFd => return super::timerfd::read(&entry, buffer, count),
        _ => {}
    }
    if buffer == 0 && count > 0 {
        return error(EFAULT);
    }
    let count = count.min(MAX_RW_COUNT);
    let read = unsafe { libc::read(fd, buffer as *mut libc::c_void, count) };
    if read < 0 {
        from_host()
    } else {
        read as i64
    }
}

fn sealed_against_write(entry: &FdEntry) -> bool {
    let seals = entry.seals.load(Ordering::SeqCst);
    seals & SEALING_ALLOWED != 0 && seals & fcntl::SEAL_WRITE != 0
}

pub fn write_fd(fd: i32, buffer: usize, count: usize) -> i64 {
    let entry = match entry_of(fd) {
        Ok(entry) => entry,
        Err(failure) => return failure,
    };
    if !writable(&entry) {
        return error(EBADF);
    }
    match entry.kind {
        FdKind::Dir => return error(EISDIR),
        FdKind::PathOnly => return error(EBADF),
        FdKind::Epoll => return error(EINVAL),
        FdKind::EventFd => return super::eventfd::write(&entry, buffer, count),
        FdKind::TimerFd => return error(EINVAL),
        _ => {}
    }
    if sealed_against_write(&entry) {
        return error(EPERM);
    }
    if buffer == 0 && count > 0 {
        return error(EFAULT);
    }
    let count = count.min(MAX_RW_COUNT);
    without_sigpipe(&entry, || {
        let written = unsafe { libc::write(fd, buffer as *const libc::c_void, count) };
        if written < 0 {
            from_host()
        } else {
            written as i64
        }
    })
}

fn positioned_kind(entry: &FdEntry) -> Result<(), i64> {
    match entry.kind {
        FdKind::Pipe | FdKind::Socket | FdKind::EventFd | FdKind::TimerFd | FdKind::Epoll => Err(error(ESPIPE)),
        FdKind::Dir => Err(error(EISDIR)),
        FdKind::PathOnly => Err(error(EBADF)),
        FdKind::Host => Ok(()),
    }
}

pub fn pread_fd(fd: i32, buffer: usize, count: usize, offset: i64) -> i64 {
    let entry = match entry_of(fd) {
        Ok(entry) => entry,
        Err(failure) => return failure,
    };
    if !readable(&entry) {
        return error(EBADF);
    }
    if let Err(failure) = positioned_kind(&entry) {
        return failure;
    }
    if offset < 0 {
        return error(EINVAL);
    }
    if buffer == 0 && count > 0 {
        return error(EFAULT);
    }
    let read = unsafe { libc::pread(fd, buffer as *mut libc::c_void, count.min(MAX_RW_COUNT), offset) };
    if read < 0 {
        from_host()
    } else {
        read as i64
    }
}

pub fn pwrite_fd(fd: i32, buffer: usize, count: usize, offset: i64) -> i64 {
    let entry = match entry_of(fd) {
        Ok(entry) => entry,
        Err(failure) => return failure,
    };
    if !writable(&entry) {
        return error(EBADF);
    }
    if let Err(failure) = positioned_kind(&entry) {
        return failure;
    }
    if sealed_against_write(&entry) {
        return error(EPERM);
    }
    if offset < 0 {
        return error(EINVAL);
    }
    if buffer == 0 && count > 0 {
        return error(EFAULT);
    }
    let count = count.min(MAX_RW_COUNT);
    let written = if entry.flags() & open::APPEND != 0 {
        // cite: bionic preadv_pwritev.cpp#pwritev
        let end = unsafe { libc::lseek(fd, 0, libc::SEEK_END) };
        if end < 0 {
            return from_host();
        }
        unsafe { libc::pwrite(fd, buffer as *const libc::c_void, count, end) }
    } else {
        unsafe { libc::pwrite(fd, buffer as *const libc::c_void, count, offset) }
    };
    if written < 0 {
        from_host()
    } else {
        written as i64
    }
}

fn vector(pointer: usize, count: i32) -> Result<Vec<(usize, usize)>, i64> {
    // cite: bionic limits.h#IOV_MAX
    if count < 0 || count > misc::IOV_MAX {
        return Err(error(EINVAL));
    }
    if count > 0 && pointer == 0 {
        return Err(error(EFAULT));
    }
    let mut total = 0usize;
    let mut parts = Vec::new();
    for index in 0..count as usize {
        let base = read_value::<usize>(pointer + index * 16);
        let length = read_value::<usize>(pointer + index * 16 + 8);
        total = total.checked_add(length).filter(|sum| *sum <= isize::MAX as usize).ok_or(error(EINVAL))?;
        parts.push((base, length));
    }
    Ok(parts)
}

pub fn readv_fd(fd: i32, pointer: usize, count: i32, offset: Option<i64>) -> i64 {
    let parts = match vector(pointer, count) {
        Ok(parts) => parts,
        Err(failure) => return failure,
    };
    let mut done = 0i64;
    let mut position = offset;
    for (base, length) in parts {
        if length == 0 {
            continue;
        }
        let result = match position {
            Some(at) => pread_fd(fd, base, length, at),
            None => read_fd(fd, base, length),
        };
        if result < 0 {
            return if done > 0 { done } else { result };
        }
        done += result;
        position = position.map(|at| at + result);
        if (result as usize) < length {
            break;
        }
    }
    done
}

pub fn writev_fd(fd: i32, pointer: usize, count: i32, offset: Option<i64>) -> i64 {
    let parts = match vector(pointer, count) {
        Ok(parts) => parts,
        Err(failure) => return failure,
    };
    let mut done = 0i64;
    let mut position = offset;
    for (base, length) in parts {
        if length == 0 {
            continue;
        }
        let result = match position {
            Some(at) => pwrite_fd(fd, base, length, at),
            None => write_fd(fd, base, length),
        };
        if result < 0 {
            return if done > 0 { done } else { result };
        }
        done += result;
        position = position.map(|at| at + result);
        if (result as usize) < length {
            break;
        }
    }
    done
}

pub fn lseek_fd(fd: i32, offset: i64, whence: i32) -> i64 {
    let entry = match entry_of(fd) {
        Ok(entry) => entry,
        Err(failure) => return failure,
    };
    match entry.kind {
        FdKind::Pipe | FdKind::Socket | FdKind::EventFd | FdKind::TimerFd | FdKind::Epoll => return error(ESPIPE),
        FdKind::PathOnly => return error(EBADF),
        FdKind::Dir => {
            if offset == 0 && whence == seek::SET {
                let mut state = entry.dir.lock().expect("dir state");
                state.position = 0;
                if entry.synthetic.is_none() {
                    state.items = None;
                }
                return 0;
            }
            return error(EINVAL);
        }
        FdKind::Host => {}
    }
    let Some(host_whence) = seek_to_host(whence) else { return error(EINVAL) };
    let position = unsafe { libc::lseek(fd, offset, host_whence) };
    if position < 0 {
        from_host()
    } else {
        position
    }
}

pub fn copy_between(source: i32, source_offset: Option<&mut i64>, destination: i32, count: usize) -> i64 {
    let mut buffer = vec![0u8; COPY_CHUNK];
    let mut total = 0i64;
    let mut position = source_offset;
    let mut remaining = count.min(0x7fff_f000);
    while remaining > 0 {
        let want = remaining.min(COPY_CHUNK);
        let read = match position.as_deref() {
            Some(at) => pread_fd(source, buffer.as_mut_ptr() as usize, want, *at),
            None => read_fd(source, buffer.as_mut_ptr() as usize, want),
        };
        if read < 0 {
            return if total > 0 { total } else { read };
        }
        if read == 0 {
            break;
        }
        let mut written = 0usize;
        while written < read as usize {
            let step = write_fd(destination, buffer.as_ptr() as usize + written, read as usize - written);
            if step < 0 {
                return if total + written as i64 > 0 { total + written as i64 } else { step };
            }
            written += step as usize;
        }
        if let Some(at) = position.as_deref_mut() {
            *at += read;
        }
        total += read;
        remaining -= read as usize;
    }
    total
}

fn truncate_fd(fd: i32, length: i64) -> i64 {
    let entry = match entry_of(fd) {
        Ok(entry) => entry,
        Err(failure) => return failure,
    };
    if length < 0 {
        return error(EINVAL);
    }
    if !writable(&entry) || entry.kind != FdKind::Host {
        return error(if entry.kind == FdKind::Dir { EISDIR } else { EINVAL });
    }
    let seals = entry.seals.load(Ordering::SeqCst);
    if seals & SEALING_ALLOWED != 0 {
        let mut current: libc::stat = unsafe { std::mem::zeroed() };
        unsafe { libc::fstat(fd, &mut current) };
        if (length < current.st_size && seals & fcntl::SEAL_SHRINK != 0) || (length > current.st_size && seals & fcntl::SEAL_GROW != 0) || seals & fcntl::SEAL_WRITE != 0 {
            return error(EPERM);
        }
    }
    if unsafe { libc::ftruncate(fd, length) } < 0 {
        from_host()
    } else {
        0
    }
}

fn sync_fd(fd: i32) -> i64 {
    let entry = match entry_of(fd) {
        Ok(entry) => entry,
        Err(failure) => return failure,
    };
    if matches!(entry.kind, FdKind::Pipe | FdKind::Socket | FdKind::EventFd | FdKind::TimerFd | FdKind::Epoll) {
        return error(EINVAL);
    }
    if unsafe { libc::fcntl(fd, DARWIN_F_FULLFSYNC) } >= 0 || unsafe { libc::fsync(fd) } >= 0 {
        0
    } else {
        from_host()
    }
}

fn advise(fd: i32, offset: i64, length: i64, advice: i32) -> i64 {
    let entry = match entry_of(fd) {
        Ok(entry) => entry,
        Err(failure) => return failure,
    };
    if matches!(entry.kind, FdKind::Pipe | FdKind::Socket) {
        return error(ESPIPE);
    }
    // cite: bionic fcntl.h#POSIX_FADV_NORMAL, POSIX_FADV_NOREUSE
    match advice {
        0 | 4 | 5 => 0,
        1 | 2 => {
            unsafe { libc::fcntl(fd, libc::F_RDAHEAD, i32::from(advice == 2)) };
            0
        }
        3 => {
            let advisory = libc::radvisory { ra_offset: offset, ra_count: length.clamp(0, i64::from(i32::MAX)) as i32 };
            unsafe { libc::fcntl(fd, libc::F_RDADVISE, &advisory) };
            0
        }
        _ => error(EINVAL),
    }
}

pub fn fallocate_fd(fd: i32, mode: i32, offset: i64, length: i64) -> i64 {
    let entry = match entry_of(fd) {
        Ok(entry) => entry,
        Err(failure) => return failure,
    };
    if length <= 0 || offset < 0 {
        return error(EINVAL);
    }
    if !writable(&entry) || entry.kind != FdKind::Host {
        return error(EBADF);
    }
    let end = offset + length;
    match mode {
        0 | misc::FALLOC_FL_KEEP_SIZE => {
            let mut store = libc::fstore_t { fst_flags: libc::F_ALLOCATEALL, fst_posmode: libc::F_PEOFPOSMODE, fst_offset: 0, fst_length: end, fst_bytesalloc: 0 };
            if unsafe { libc::fcntl(fd, libc::F_PREALLOCATE, &mut store) } < 0 {
                return from_host();
            }
            let mut current: libc::stat = unsafe { std::mem::zeroed() };
            unsafe { libc::fstat(fd, &mut current) };
            if mode == 0 && end > current.st_size && unsafe { libc::ftruncate(fd, end) } < 0 {
                return from_host();
            }
            0
        }
        value if value == misc::FALLOC_FL_PUNCH_HOLE | misc::FALLOC_FL_KEEP_SIZE => {
            let hole = libc::fpunchhole_t { fp_flags: 0, reserved: 0, fp_offset: offset, fp_length: length };
            if unsafe { libc::fcntl(fd, 99, &hole) } < 0 {
                from_host()
            } else {
                0
            }
        }
        other => {
            missing("fallocate", &format!("mode {other:#x} has no host equivalent"));
            error(EOPNOTSUPP)
        }
    }
}

fn is_terminal(fd: i32) -> bool {
    fd_table::contains(fd) && unsafe { libc::isatty(fd) } == 1
}

pub fn register() {
    in_libc("read", |context| {
        let result = read_fd(context.arg_i32(0), context.arg_ptr(1), context.arg_u64(2) as usize);
        finish_long(context, result);
    });
    in_libc("write", |context| {
        let result = write_fd(context.arg_i32(0), context.arg_ptr(1), context.arg_u64(2) as usize);
        finish_long(context, result);
    });
    for name in ["pread", "pread64"] {
        in_libc(name, |context| {
            let result = pread_fd(context.arg_i32(0), context.arg_ptr(1), context.arg_u64(2) as usize, context.arg_u64(3) as i64);
            finish_long(context, result);
        });
    }
    for name in ["pwrite", "pwrite64"] {
        in_libc(name, |context| {
            let result = pwrite_fd(context.arg_i32(0), context.arg_ptr(1), context.arg_u64(2) as usize, context.arg_u64(3) as i64);
            finish_long(context, result);
        });
    }
    in_libc("readv", |context| {
        let result = readv_fd(context.arg_i32(0), context.arg_ptr(1), context.arg_i32(2), None);
        finish_long(context, result);
    });
    in_libc("writev", |context| {
        let result = writev_fd(context.arg_i32(0), context.arg_ptr(1), context.arg_i32(2), None);
        finish_long(context, result);
    });
    for name in ["preadv", "preadv64"] {
        in_libc(name, |context| {
            let result = readv_fd(context.arg_i32(0), context.arg_ptr(1), context.arg_i32(2), Some(context.arg_u64(3) as i64));
            finish_long(context, result);
        });
    }
    for name in ["pwritev", "pwritev64"] {
        in_libc(name, |context| {
            let result = writev_fd(context.arg_i32(0), context.arg_ptr(1), context.arg_i32(2), Some(context.arg_u64(3) as i64));
            finish_long(context, result);
        });
    }
    for name in ["preadv2", "preadv64v2"] {
        in_libc(name, |context| {
            // cite: bionic preadv_pwritev.cpp#preadv64v2
            let result = if context.arg_i32(4) != 0 { error(EOPNOTSUPP) } else { readv_fd(context.arg_i32(0), context.arg_ptr(1), context.arg_i32(2), (context.arg_u64(3) as i64 >= 0).then(|| context.arg_u64(3) as i64)) };
            finish_long(context, result);
        });
    }
    for name in ["pwritev2", "pwritev64v2"] {
        in_libc(name, |context| {
            let result = if context.arg_i32(4) != 0 { error(EOPNOTSUPP) } else { writev_fd(context.arg_i32(0), context.arg_ptr(1), context.arg_i32(2), (context.arg_u64(3) as i64 >= 0).then(|| context.arg_u64(3) as i64)) };
            finish_long(context, result);
        });
    }
    for name in ["lseek", "lseek64"] {
        in_libc(name, |context| {
            let result = lseek_fd(context.arg_i32(0), context.arg_u64(1) as i64, context.arg_i32(2));
            finish_long(context, result);
        });
    }
    for name in ["sendfile", "sendfile64"] {
        in_libc(name, |context| {
            let offset_pointer = context.arg_ptr(2);
            let mut offset = if offset_pointer != 0 { Some(read_value::<i64>(offset_pointer)) } else { None };
            let result = copy_between(context.arg_i32(1), offset.as_mut(), context.arg_i32(0), context.arg_u64(3) as usize);
            if let (Some(updated), true) = (offset, result >= 0) {
                write_value::<i64>(offset_pointer, updated);
            }
            finish_long(context, result);
        });
    }
    in_libc("copy_file_range", |context| {
        // cite: bionic SYSCALLS.TXT#copy_file_range
        let (input_pointer, output_pointer) = (context.arg_ptr(1), context.arg_ptr(3));
        let mut input = (input_pointer != 0).then(|| read_value::<i64>(input_pointer));
        if context.arg_u32(5) != 0 {
            finish_long(context, error(EINVAL));
            return;
        }
        let (source, destination) = (context.arg_i32(0), context.arg_i32(2));
        let result = if output_pointer != 0 {
            missing("copy_file_range", "an explicit output offset needs pwrite based copying");
            error(EOPNOTSUPP)
        } else {
            copy_between(source, input.as_mut(), destination, context.arg_u64(4) as usize)
        };
        if let (Some(updated), true) = (input, result >= 0) {
            write_value::<i64>(input_pointer, updated);
        }
        finish_long(context, result);
    });
    for name in ["ftruncate", "ftruncate64"] {
        in_libc(name, |context| {
            let result = truncate_fd(context.arg_i32(0), context.arg_u64(1) as i64);
            finish_int(context, result);
        });
    }
    for name in ["fsync", "fdatasync"] {
        in_libc(name, |context| {
            let result = sync_fd(context.arg_i32(0));
            finish_int(context, result);
        });
    }
    in_libc("syncfs", |context| {
        let result = sync_fd(context.arg_i32(0));
        finish_int(context, result);
    });
    in_libc("sync", |_| unsafe { libc::sync() });
    for name in ["posix_fadvise", "posix_fadvise64"] {
        in_libc(name, |context| {
            let result = advise(context.arg_i32(0), context.arg_u64(1) as i64, context.arg_u64(2) as i64, context.arg_i32(3));
            context.ret_i32(if result < 0 { (-result) as i32 } else { 0 });
        });
    }
    for name in ["fallocate", "fallocate64"] {
        in_libc(name, |context| {
            let result = fallocate_fd(context.arg_i32(0), context.arg_i32(1), context.arg_u64(2) as i64, context.arg_u64(3) as i64);
            finish_int(context, result);
        });
    }
    for name in ["posix_fallocate", "posix_fallocate64"] {
        in_libc(name, |context| {
            let result = fallocate_fd(context.arg_i32(0), 0, context.arg_u64(1) as i64, context.arg_u64(2) as i64);
            context.ret_i32(if result < 0 { (-result) as i32 } else { 0 });
        });
    }
    in_libc("isatty", |context| {
        if is_terminal(context.arg_i32(0)) {
            context.ret_i32(1);
        } else {
            context.set_errno(ENOTTY);
            context.ret_i32(0);
        }
    });
}
