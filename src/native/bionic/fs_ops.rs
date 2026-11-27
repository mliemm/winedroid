use std::ffi::CString;

use super::super::bridge::CallContext;
use super::abi_types::{EBADF, EINVAL, ERANGE};
use super::device_profile::{APP_GID, APP_UID};
use super::export::in_libc;
use super::fd_table::{self, FdKind};
use super::fs_open::path_argument;
use super::fs_result::{finish_int, finish_long, from_host, missing};
use super::guest_mem::{cstr_bytes, read_value, write_bytes};
use super::guest_paths::{self, Access, HostRef, Resolved};
use super::guest_process;
use super::linux_abi::{at, misc};
use super::stat_convert::{S_IFDIR, S_IFLNK, S_IFMT};

const EACCES: i32 = 13;
const EPERM: i32 = 1;
const EBUSY: i32 = 16;
const EEXIST: i32 = 17;
const ENOTDIR: i32 = 20;
const EISDIR: i32 = 21;
const EXDEV: i32 = 18;
const EROFS: i32 = 30;
const EOPNOTSUPP: i32 = 95;

fn error(code: i32) -> i64 {
    -i64::from(code)
}

fn mutable_leaf(resolved: Resolved) -> Result<HostRef, i32> {
    match guest_paths::host_ref(resolved, Access::Write)? {
        reference if reference.leaf.is_none() => Err(EBUSY),
        reference if reference.protected => Err(EACCES),
        reference => Ok(reference),
    }
}

pub fn make_directory(dirfd: i32, path: &[u8], mode: u32) -> i64 {
    let resolved = match guest_paths::resolve(dirfd, path, Access::Write) {
        Ok(resolved) => resolved,
        Err(errno) => return error(errno),
    };
    match resolved {
        Resolved::Host(reference) => {
            if reference.read_only {
                return error(EROFS);
            }
            if reference.leaf.is_none() {
                return error(EEXIST);
            }
            let created = guest_process::apply_umask(mode) & 0o777;
            if unsafe {
                libc::mkdirat(
                    reference.dir_fd(),
                    reference.name().as_ptr(),
                    created as libc::mode_t,
                )
            } < 0
            {
                from_host()
            } else {
                report_created(&reference, true)
            }
        }
        Resolved::Virtual(_) | Resolved::Synthetic(..) | Resolved::Device(_) => error(EEXIST),
    }
}

fn report_created(reference: &HostRef, is_directory: bool) -> i64 {
    crate::fs_notify::created(reference.dir.as_fd(), reference.name(), is_directory);
    0
}

fn watched_directory(reference: &HostRef) -> Option<crate::fs_notify::InodeKey> {
    crate::fs_notify::directory_key(reference.dir.as_fd(), reference.name())
}

fn report_removed(watched: Option<crate::fs_notify::InodeKey>) -> i64 {
    if let Some(key) = watched {
        crate::fs_notify::removed_directory(key);
    }
    0
}

fn is_directory(reference: &HostRef) -> bool {
    reference
        .stat()
        .map(|stat| u32::from(stat.st_mode) & S_IFMT == S_IFDIR)
        .unwrap_or(false)
}

pub fn unlink_at(dirfd: i32, path: &[u8], flags: i32) -> i64 {
    if flags & !at::REMOVEDIR != 0 {
        return error(EINVAL);
    }
    let resolved = match guest_paths::resolve(dirfd, path, Access::Write) {
        Ok(resolved) => resolved,
        Err(errno) => return error(errno),
    };
    let reference = match mutable_leaf(resolved) {
        Ok(reference) => reference,
        Err(errno) => return error(errno),
    };
    let remove_directory = flags & at::REMOVEDIR != 0;
    if !remove_directory && is_directory(&reference) {
        return error(EISDIR);
    }
    let host_flags = if remove_directory {
        libc::AT_REMOVEDIR
    } else {
        0
    };
    let watched = watched_directory(&reference);
    if unsafe { libc::unlinkat(reference.dir_fd(), reference.name().as_ptr(), host_flags) } < 0 {
        from_host()
    } else {
        report_removed(watched)
    }
}

pub fn rename_at(old_dirfd: i32, old: &[u8], new_dirfd: i32, new: &[u8], flags: u32) -> i64 {
    // cite: bionic fs.h#RENAME_NOREPLACE
    let source = match guest_paths::resolve(old_dirfd, old, Access::Write) {
        Ok(resolved) => resolved,
        Err(errno) => return error(errno),
    };
    let destination = match guest_paths::resolve(new_dirfd, new, Access::Write) {
        Ok(resolved) => resolved,
        Err(errno) => return error(errno),
    };
    let (source, destination) = match (mutable_leaf(source), mutable_leaf(destination)) {
        (Ok(source), Ok(destination)) => (source, destination),
        (Err(errno), _) | (_, Err(errno)) => return error(errno),
    };
    if source.mount != destination.mount {
        return error(EXDEV);
    }
    let host_flags = match flags {
        0 => 0,
        at::RENAME_NOREPLACE => libc::RENAME_EXCL,
        at::RENAME_EXCHANGE => libc::RENAME_SWAP,
        _ => return error(EINVAL),
    };
    let rename = crate::fs_notify::Rename::prepare(
        source.dir.as_fd(),
        source.name(),
        destination.dir.as_fd(),
        destination.name(),
        flags == at::RENAME_EXCHANGE,
    );
    let status = unsafe {
        libc::renameatx_np(
            source.dir_fd(),
            source.name().as_ptr(),
            destination.dir_fd(),
            destination.name().as_ptr(),
            host_flags,
        )
    };
    if status < 0 {
        from_host()
    } else {
        rename.finish();
        0
    }
}

fn symlink_at(target: &[u8], dirfd: i32, path: &[u8]) -> i64 {
    let resolved = match guest_paths::resolve(dirfd, path, Access::Write) {
        Ok(resolved) => resolved,
        Err(errno) => return error(errno),
    };
    let Ok(target) = CString::new(target) else {
        return error(EINVAL);
    };
    match guest_paths::host_ref(resolved, Access::Write) {
        Ok(reference) if reference.leaf.is_some() => {
            if unsafe {
                libc::symlinkat(
                    target.as_ptr(),
                    reference.dir_fd(),
                    reference.name().as_ptr(),
                )
            } < 0
            {
                from_host()
            } else {
                report_created(&reference, false)
            }
        }
        Ok(_) => error(EEXIST),
        Err(errno) => error(errno),
    }
}

fn link_at(old_dirfd: i32, old: &[u8], new_dirfd: i32, new: &[u8], flags: i32) -> i64 {
    if flags & !at::SYMLINK_FOLLOW != 0 {
        return error(EINVAL);
    }
    let source = if flags & at::SYMLINK_FOLLOW != 0 {
        guest_paths::resolve_follow(old_dirfd, old, Access::Write)
    } else {
        guest_paths::resolve(old_dirfd, old, Access::Write)
    };
    let destination = guest_paths::resolve(new_dirfd, new, Access::Write);
    let (source, destination) = match (
        source.and_then(mutable_leaf),
        destination.and_then(mutable_leaf),
    ) {
        (Ok(source), Ok(destination)) => (source, destination),
        (Err(errno), _) | (_, Err(errno)) => return error(errno),
    };
    if source.mount != destination.mount {
        return error(EXDEV);
    }
    if unsafe {
        libc::linkat(
            source.dir_fd(),
            source.name().as_ptr(),
            destination.dir_fd(),
            destination.name().as_ptr(),
            0,
        )
    } < 0
    {
        from_host()
    } else {
        report_created(&destination, false)
    }
}

fn read_link_at(dirfd: i32, path: &[u8], buffer: usize, size: usize) -> i64 {
    // cite: bionic readlink.cpp#readlink
    if size == 0 {
        return error(EINVAL);
    }
    let resolved = match guest_paths::resolve(dirfd, path, Access::Read) {
        Ok(resolved) => resolved,
        Err(errno) => return error(errno),
    };
    let target = match &resolved {
        Resolved::Host(reference) => match reference.stat() {
            Ok(stat) if u32::from(stat.st_mode) & S_IFMT == S_IFLNK => {
                reference.link_target().map(String::into_bytes)
            }
            Ok(_) => return error(EINVAL),
            Err(errno) => return error(errno),
        },
        Resolved::Synthetic(super::synthetic_node::Synthetic::Link { target, .. }, _) => {
            Some(target().into_bytes())
        }
        _ => return error(EINVAL),
    };
    let Some(target) = target else {
        return error(EINVAL);
    };
    let copied = target.len().min(size);
    write_bytes(buffer, &target[..copied]);
    copied as i64
}

pub fn canonical_path(dirfd: i32, path: &[u8]) -> Result<String, i32> {
    let absolute = guest_paths::absolute(dirfd, path)?;
    let (resolved, canonical) = guest_paths::resolve_follow_with_path(&absolute, Access::Read)?;
    match resolved {
        Resolved::Host(reference) => reference.stat().map(|_| canonical),
        _ => Ok(canonical),
    }
}

fn change_mode(dirfd: i32, path: &[u8], mode: u32, flags: i32) -> i64 {
    if flags != 0 {
        return error(EOPNOTSUPP);
    }
    match guest_paths::resolve_follow(dirfd, path, Access::Write) {
        Ok(Resolved::Host(reference)) => {
            if reference.read_only {
                return error(EROFS);
            }
            if unsafe {
                libc::fchmodat(
                    reference.dir_fd(),
                    reference.name().as_ptr(),
                    (mode & 0o7777) as libc::mode_t,
                    0,
                )
            } < 0
            {
                from_host()
            } else {
                0
            }
        }
        Ok(_) => error(EPERM),
        Err(errno) => error(errno),
    }
}

fn change_owner(dirfd: i32, path: &[u8], uid: u32, gid: u32) -> i64 {
    let unchanged_or_app =
        (uid == u32::MAX || uid == APP_UID) && (gid == u32::MAX || gid == APP_GID);
    match guest_paths::resolve_follow(dirfd, path, Access::Read) {
        Ok(_) if unchanged_or_app => 0,
        Ok(_) => error(EPERM),
        Err(errno) => error(errno),
    }
}

fn host_time(seconds: i64, nanoseconds: i64) -> Result<libc::timespec, i32> {
    // cite: bionic stat.h#UTIME_NOW, UTIME_OMIT
    let nanos = match nanoseconds {
        misc::UTIME_NOW => libc::UTIME_NOW,
        misc::UTIME_OMIT => libc::UTIME_OMIT,
        value if (0..1_000_000_000).contains(&value) => value as libc::c_long,
        _ => return Err(EINVAL),
    };
    Ok(libc::timespec {
        tv_sec: seconds,
        tv_nsec: nanos,
    })
}

fn read_times(pointer: usize) -> Result<[libc::timespec; 2], i32> {
    if pointer == 0 {
        let now = libc::timespec {
            tv_sec: 0,
            tv_nsec: libc::UTIME_NOW,
        };
        return Ok([now, now]);
    }
    Ok([
        host_time(read_value::<i64>(pointer), read_value::<i64>(pointer + 8))?,
        host_time(
            read_value::<i64>(pointer + 16),
            read_value::<i64>(pointer + 24),
        )?,
    ])
}

fn set_times(dirfd: i32, path: &[u8], times: &[libc::timespec; 2], flags: i32) -> i64 {
    if flags & !at::SYMLINK_NOFOLLOW != 0 {
        return error(EINVAL);
    }
    let resolved = if flags & at::SYMLINK_NOFOLLOW != 0 {
        guest_paths::resolve(dirfd, path, Access::Write)
    } else {
        guest_paths::resolve_follow(dirfd, path, Access::Write)
    };
    match resolved {
        Ok(Resolved::Host(reference)) => {
            if reference.read_only {
                return error(EROFS);
            }
            let host_flags = if flags & at::SYMLINK_NOFOLLOW != 0 {
                libc::AT_SYMLINK_NOFOLLOW
            } else {
                0
            };
            if unsafe {
                libc::utimensat(
                    reference.dir_fd(),
                    reference.name().as_ptr(),
                    times.as_ptr(),
                    host_flags,
                )
            } < 0
            {
                from_host()
            } else {
                0
            }
        }
        Ok(_) => error(EPERM),
        Err(errno) => error(errno),
    }
}

fn set_times_fd(fd: i32, times: &[libc::timespec; 2]) -> i64 {
    if !fd_table::contains(fd) {
        return error(EBADF);
    }
    if unsafe { libc::futimens(fd, times.as_ptr()) } < 0 {
        from_host()
    } else {
        0
    }
}

fn timeval_pair(pointer: usize) -> Result<[libc::timespec; 2], i32> {
    if pointer == 0 {
        return read_times(0);
    }
    let mut times = [libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    }; 2];
    for (index, slot) in times.iter_mut().enumerate() {
        let (seconds, micros) = (
            read_value::<i64>(pointer + index * 16),
            read_value::<i64>(pointer + index * 16 + 8),
        );
        if !(0..1_000_000).contains(&micros) {
            return Err(EINVAL);
        }
        *slot = libc::timespec {
            tv_sec: seconds,
            tv_nsec: (micros * 1000) as libc::c_long,
        };
    }
    Ok(times)
}

fn make_fifo(dirfd: i32, path: &[u8], mode: u32) -> i64 {
    match guest_paths::resolve(dirfd, path, Access::Write).and_then(mutable_leaf) {
        Ok(reference) => {
            let created = guest_process::apply_umask(mode) & 0o777;
            if unsafe {
                libc::mkfifoat(
                    reference.dir_fd(),
                    reference.name().as_ptr(),
                    created as libc::mode_t,
                )
            } < 0
            {
                from_host()
            } else {
                report_created(&reference, false)
            }
        }
        Err(errno) => error(errno),
    }
}

fn make_node(dirfd: i32, path: &[u8], mode: u32) -> i64 {
    match mode & S_IFMT {
        0o010000 => make_fifo(dirfd, path, mode),
        0 | 0o100000 => {
            let result = super::fs_open::open_at(
                dirfd,
                path,
                super::linux_abi::open::RDONLY
                    | super::linux_abi::open::CREAT
                    | super::linux_abi::open::EXCL,
                mode & 0o7777,
            );
            if result >= 0 {
                super::fs_close_dup::close_fd(result as i32)
            } else {
                result
            }
        }
        other => {
            missing(
                "mknod",
                &format!("file type {other:#o} cannot be created without privileges"),
            );
            error(EPERM)
        }
    }
}

fn change_directory(path: &[u8]) -> i64 {
    match guest_paths::resolve_follow(at::FDCWD, path, Access::Read) {
        Ok(Resolved::Host(reference)) => match reference.stat() {
            Ok(stat) if u32::from(stat.st_mode) & S_IFMT == S_IFDIR => {
                guest_process::set_cwd(&reference.guest_path);
                0
            }
            Ok(_) => error(ENOTDIR),
            Err(errno) => error(errno),
        },
        Ok(Resolved::Virtual(directory)) => {
            guest_process::set_cwd(&directory.guest_path);
            0
        }
        Ok(Resolved::Synthetic(super::synthetic_node::Synthetic::Dir { .. }, path)) => {
            guest_process::set_cwd(&path);
            0
        }
        Ok(_) => error(ENOTDIR),
        Err(errno) => error(errno),
    }
}

fn truncate_path(path: &[u8], length: i64) -> i64 {
    if length < 0 {
        return error(EINVAL);
    }
    match guest_paths::resolve_follow(at::FDCWD, path, Access::Write) {
        Ok(Resolved::Host(reference)) => {
            if reference.read_only {
                return error(EROFS);
            }
            let fd = reference.open(libc::O_WRONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC, 0);
            if fd < 0 {
                return fd;
            }
            let status = unsafe { libc::ftruncate(fd as i32, length) };
            let failure = if status < 0 { Some(from_host()) } else { None };
            unsafe { libc::close(fd as i32) };
            failure.unwrap_or(0)
        }
        Ok(_) => error(EACCES),
        Err(errno) => error(errno),
    }
}

fn get_cwd(context: &mut CallContext) {
    // cite: bionic getcwd.cpp#getcwd
    let (buffer, size) = (context.arg_ptr(0), context.arg_u64(1) as usize);
    let cwd = guest_process::cwd();
    let needed = cwd.len() + 1;
    if buffer != 0 && size == 0 {
        context.set_errno(EINVAL);
        context.ret_ptr(0);
        return;
    }
    if buffer == 0 {
        let capacity = if size == 0 { needed } else { size };
        if capacity < needed {
            context.set_errno(ERANGE);
            context.ret_ptr(0);
            return;
        }
        let allocation = unsafe { libc::malloc(needed) } as usize;
        write_bytes(allocation, cwd.as_bytes());
        super::guest_mem::write_value::<u8>(allocation + cwd.len(), 0);
        context.ret_ptr(allocation);
        return;
    }
    if size < needed {
        context.set_errno(ERANGE);
        context.ret_ptr(0);
        return;
    }
    write_bytes(buffer, cwd.as_bytes());
    super::guest_mem::write_value::<u8>(buffer + cwd.len(), 0);
    context.ret_ptr(buffer);
}

fn with_path(
    context: &mut CallContext,
    index: usize,
    work: impl FnOnce(&mut CallContext, &'static [u8]) -> i64,
) {
    let result = match path_argument(context.arg_ptr(index)) {
        Ok(path) => work(context, path),
        Err(errno) => error(errno),
    };
    finish_int(context, result);
}

pub fn register() {
    in_libc("unlink", |context| {
        with_path(context, 0, |_, path| unlink_at(at::FDCWD, path, 0))
    });
    in_libc("rmdir", |context| {
        with_path(context, 0, |_, path| {
            unlink_at(at::FDCWD, path, at::REMOVEDIR)
        })
    });
    in_libc("unlinkat", |context| {
        with_path(context, 1, |context, path| {
            unlink_at(context.arg_i32(0), path, context.arg_i32(2))
        })
    });
    in_libc("mkdir", |context| {
        with_path(context, 0, |context, path| {
            make_directory(at::FDCWD, path, context.arg_u32(1))
        })
    });
    in_libc("mkdirat", |context| {
        with_path(context, 1, |context, path| {
            make_directory(context.arg_i32(0), path, context.arg_u32(2))
        })
    });
    in_libc("rename", |context| {
        let result = match (
            path_argument(context.arg_ptr(0)),
            path_argument(context.arg_ptr(1)),
        ) {
            (Ok(old), Ok(new)) => rename_at(at::FDCWD, old, at::FDCWD, new, 0),
            (Err(errno), _) | (_, Err(errno)) => error(errno),
        };
        finish_int(context, result);
    });
    for (name, flagged) in [("renameat", false), ("renameat2", true)] {
        in_libc(name, move |context| {
            let result = match (
                path_argument(context.arg_ptr(1)),
                path_argument(context.arg_ptr(3)),
            ) {
                (Ok(old), Ok(new)) => rename_at(
                    context.arg_i32(0),
                    old,
                    context.arg_i32(2),
                    new,
                    if flagged { context.arg_u32(4) } else { 0 },
                ),
                (Err(errno), _) | (_, Err(errno)) => error(errno),
            };
            finish_int(context, result);
        });
    }
    in_libc("symlink", |context| {
        let result = match (
            path_argument(context.arg_ptr(0)),
            path_argument(context.arg_ptr(1)),
        ) {
            (Ok(target), Ok(link)) => symlink_at(target, at::FDCWD, link),
            (Err(errno), _) | (_, Err(errno)) => error(errno),
        };
        finish_int(context, result);
    });
    in_libc("symlinkat", |context| {
        let result = match (
            path_argument(context.arg_ptr(0)),
            path_argument(context.arg_ptr(2)),
        ) {
            (Ok(target), Ok(link)) => symlink_at(target, context.arg_i32(1), link),
            (Err(errno), _) | (_, Err(errno)) => error(errno),
        };
        finish_int(context, result);
    });
    in_libc("link", |context| {
        let result = match (
            path_argument(context.arg_ptr(0)),
            path_argument(context.arg_ptr(1)),
        ) {
            (Ok(old), Ok(new)) => link_at(at::FDCWD, old, at::FDCWD, new, 0),
            (Err(errno), _) | (_, Err(errno)) => error(errno),
        };
        finish_int(context, result);
    });
    in_libc("linkat", |context| {
        let result = match (
            path_argument(context.arg_ptr(1)),
            path_argument(context.arg_ptr(3)),
        ) {
            (Ok(old), Ok(new)) => link_at(
                context.arg_i32(0),
                old,
                context.arg_i32(2),
                new,
                context.arg_i32(4),
            ),
            (Err(errno), _) | (_, Err(errno)) => error(errno),
        };
        finish_int(context, result);
    });
    in_libc("readlink", |context| {
        let result = match path_argument(context.arg_ptr(0)) {
            Ok(path) => read_link_at(
                at::FDCWD,
                path,
                context.arg_ptr(1),
                context.arg_u64(2) as usize,
            ),
            Err(errno) => error(errno),
        };
        finish_long(context, result);
    });
    in_libc("readlinkat", |context| {
        let result = match path_argument(context.arg_ptr(1)) {
            Ok(path) => read_link_at(
                context.arg_i32(0),
                path,
                context.arg_ptr(2),
                context.arg_u64(3) as usize,
            ),
            Err(errno) => error(errno),
        };
        finish_long(context, result);
    });
    in_libc("realpath", |context| {
        // cite: bionic realpath.cpp#realpath
        let path = context.arg_ptr(0);
        if path == 0 {
            context.set_errno(EINVAL);
            context.ret_ptr(0);
            return;
        }
        match canonical_path(at::FDCWD, cstr_bytes(path)) {
            Ok(canonical) => {
                let target = if context.arg_ptr(1) == 0 {
                    unsafe { libc::malloc(canonical.len() + 1) as usize }
                } else {
                    context.arg_ptr(1)
                };
                write_bytes(target, canonical.as_bytes());
                super::guest_mem::write_value::<u8>(target + canonical.len(), 0);
                context.ret_ptr(target);
            }
            Err(errno) => {
                context.set_errno(errno);
                context.ret_ptr(0);
            }
        }
    });
    in_libc("chmod", |context| {
        with_path(context, 0, |context, path| {
            change_mode(at::FDCWD, path, context.arg_u32(1), 0)
        })
    });
    in_libc("fchmodat", |context| {
        with_path(context, 1, |context, path| {
            change_mode(
                context.arg_i32(0),
                path,
                context.arg_u32(2),
                context.arg_i32(3),
            )
        })
    });
    in_libc("fchmod", |context| {
        let fd = context.arg_i32(0);
        let result = if !fd_table::contains(fd) {
            error(EBADF)
        } else if unsafe { libc::fchmod(fd, (context.arg_u32(1) & 0o7777) as libc::mode_t) } < 0 {
            from_host()
        } else {
            0
        };
        finish_int(context, result);
    });
    for name in ["chown", "lchown"] {
        in_libc(name, |context| {
            with_path(context, 0, |context, path| {
                change_owner(at::FDCWD, path, context.arg_u32(1), context.arg_u32(2))
            })
        });
    }
    in_libc("fchownat", |context| {
        with_path(context, 1, |context, path| {
            change_owner(
                context.arg_i32(0),
                path,
                context.arg_u32(2),
                context.arg_u32(3),
            )
        })
    });
    in_libc("fchown", |context| {
        let fd = context.arg_i32(0);
        let (uid, gid) = (context.arg_u32(1), context.arg_u32(2));
        let result = if !fd_table::contains(fd) {
            error(EBADF)
        } else if (uid == u32::MAX || uid == APP_UID) && (gid == u32::MAX || gid == APP_GID) {
            0
        } else {
            error(EPERM)
        };
        finish_int(context, result);
    });
    in_libc("utimensat", |context| {
        let (dirfd, flags) = (context.arg_i32(0), context.arg_i32(3));
        let times = match read_times(context.arg_ptr(2)) {
            Ok(times) => times,
            Err(errno) => return finish_int(context, error(errno)),
        };
        let result = if context.arg_ptr(1) == 0 {
            set_times_fd(dirfd, &times)
        } else {
            with_path_value(context.arg_ptr(1), |path| {
                set_times(dirfd, path, &times, flags)
            })
        };
        finish_int(context, result);
    });
    in_libc("futimens", |context| {
        let result = match read_times(context.arg_ptr(1)) {
            Ok(times) => set_times_fd(context.arg_i32(0), &times),
            Err(errno) => error(errno),
        };
        finish_int(context, result);
    });
    in_libc("utime", |context| {
        let pointer = context.arg_ptr(1);
        let times = if pointer == 0 {
            read_times(0)
        } else {
            Ok([
                libc::timespec {
                    tv_sec: read_value::<i64>(pointer),
                    tv_nsec: 0,
                },
                libc::timespec {
                    tv_sec: read_value::<i64>(pointer + 8),
                    tv_nsec: 0,
                },
            ])
        };
        let result = match times {
            Ok(times) => with_path_value(context.arg_ptr(0), |path| {
                set_times(at::FDCWD, path, &times, 0)
            }),
            Err(errno) => error(errno),
        };
        finish_int(context, result);
    });
    for (name, nofollow) in [("utimes", 0), ("lutimes", at::SYMLINK_NOFOLLOW)] {
        in_libc(name, move |context| {
            let result = match timeval_pair(context.arg_ptr(1)) {
                Ok(times) => with_path_value(context.arg_ptr(0), |path| {
                    set_times(at::FDCWD, path, &times, nofollow)
                }),
                Err(errno) => error(errno),
            };
            finish_int(context, result);
        });
    }
    in_libc("futimes", |context| {
        let result = match timeval_pair(context.arg_ptr(1)) {
            Ok(times) => set_times_fd(context.arg_i32(0), &times),
            Err(errno) => error(errno),
        };
        finish_int(context, result);
    });
    in_libc("futimesat", |context| {
        let result = match timeval_pair(context.arg_ptr(2)) {
            Ok(times) => with_path_value(context.arg_ptr(1), |path| {
                set_times(context.arg_i32(0), path, &times, 0)
            }),
            Err(errno) => error(errno),
        };
        finish_int(context, result);
    });
    in_libc("mkfifo", |context| {
        with_path(context, 0, |context, path| {
            make_fifo(at::FDCWD, path, context.arg_u32(1))
        })
    });
    in_libc("mkfifoat", |context| {
        with_path(context, 1, |context, path| {
            make_fifo(context.arg_i32(0), path, context.arg_u32(2))
        })
    });
    in_libc("mknod", |context| {
        with_path(context, 0, |context, path| {
            make_node(at::FDCWD, path, context.arg_u32(1))
        })
    });
    in_libc("mknodat", |context| {
        with_path(context, 1, |context, path| {
            make_node(context.arg_i32(0), path, context.arg_u32(2))
        })
    });
    in_libc("umask", |context| {
        context.ret_u32(guest_process::swap_umask(context.arg_u32(0)))
    });
    in_libc("chdir", |context| {
        with_path(context, 0, |_, path| change_directory(path))
    });
    in_libc("fchdir", |context| {
        let result = match fd_table::get(context.arg_i32(0)) {
            Some(entry) if entry.kind == FdKind::Dir || entry.kind == FdKind::PathOnly => {
                guest_process::set_cwd(&entry.guest_path);
                0
            }
            Some(_) => error(ENOTDIR),
            None => error(EBADF),
        };
        finish_int(context, result);
    });
    in_libc("chroot", |context| finish_int(context, error(EPERM)));
    in_libc("getcwd", get_cwd);
    for name in ["truncate", "truncate64"] {
        in_libc(name, |context| {
            with_path(context, 0, |context, path| {
                truncate_path(path, context.arg_u64(1) as i64)
            })
        });
    }
}

fn with_path_value(pointer: usize, work: impl FnOnce(&'static [u8]) -> i64) -> i64 {
    match path_argument(pointer) {
        Ok(path) => work(path),
        Err(errno) => error(errno),
    }
}
