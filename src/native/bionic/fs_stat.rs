use super::super::bridge::CallContext;
use super::abi_types::{EBADF, EINVAL};
use super::export::in_libc;
use super::fd_table;
use super::fs_open::path_argument;
use super::fs_result::{finish_int, from_host};
use super::guest_mem::write_bytes;
use super::guest_paths::{self, Access, Resolved};
use super::linux_abi::{at, stat as layout};
use super::stat_convert::{self, from_host as stat_from_host, path_hash, virtual_node, S_IFCHR, S_IFDIR, S_IFLNK, S_IFREG};
use super::synthetic_node::Synthetic;

const EACCES: i32 = 13;
const EROFS: i32 = 30;
const STATX_BASIC_STATS: u32 = 0x7ff;
const STATX_BTIME: u32 = 0x800;
const STATX_SIZE: usize = 256;
const ACCESS_MODES: i32 = 7;

fn error(code: i32) -> i64 {
    -i64::from(code)
}

pub struct StatResult {
    pub bytes: [u8; layout::SIZE],
    pub birth: (i64, i64),
}

fn from_host_stat(host: &libc::stat, synthetic: Option<fd_table::SyntheticStat>) -> StatResult {
    StatResult { bytes: stat_from_host(host, synthetic), birth: (host.st_birthtime, host.st_birthtime_nsec) }
}

fn synthetic_result(bytes: [u8; layout::SIZE]) -> StatResult {
    let birth = (i64::from_le_bytes(bytes[88..96].try_into().expect("8 bytes")), 0);
    StatResult { bytes, birth }
}

pub fn stat_resolved(resolved: &Resolved) -> Result<StatResult, i32> {
    match resolved {
        Resolved::Host(reference) => Ok(from_host_stat(&reference.stat()?, None)),
        Resolved::Device(device) => Ok(synthetic_result(virtual_node(S_IFCHR | 0o666, 0, path_hash(device.name), (0, 5), device.rdev))),
        Resolved::Synthetic(node, path) => {
            let (mode, device) = match node {
                Synthetic::File { device, .. } => (S_IFREG | 0o444, *device),
                Synthetic::Dir { device, .. } => (S_IFDIR | 0o555, *device),
                Synthetic::Link { device, .. } => (S_IFLNK | 0o777, *device),
            };
            Ok(synthetic_result(virtual_node(mode, 0, path_hash(path), device, (0, 0))))
        }
        Resolved::Virtual(directory) => Ok(synthetic_result(virtual_node(S_IFDIR | directory.mode, directory.owner, path_hash(&directory.guest_path), (0, 1), (0, 0)))),
    }
}

pub fn stat_fd(fd: i32) -> Result<StatResult, i32> {
    let entry = fd_table::get(fd).ok_or(EBADF)?;
    let mut host: libc::stat = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstat(fd, &mut host) } < 0 {
        return Err((-from_host()) as i32);
    }
    Ok(from_host_stat(&host, entry.synthetic))
}

pub fn stat_at(dirfd: i32, path: &[u8], flags: i32) -> Result<StatResult, i32> {
    let known = at::SYMLINK_NOFOLLOW | at::EMPTY_PATH;
    if flags & !known != 0 {
        return Err(EINVAL);
    }
    if path.is_empty() && flags & at::EMPTY_PATH != 0 {
        return stat_fd(dirfd);
    }
    let resolved = if flags & at::SYMLINK_NOFOLLOW != 0 { guest_paths::resolve(dirfd, path, Access::Read)? } else { guest_paths::resolve_follow(dirfd, path, Access::Read)? };
    stat_resolved(&resolved)
}

fn deliver(context: &mut CallContext, outcome: Result<StatResult, i32>, buffer: usize) {
    match outcome {
        Ok(result) => {
            write_bytes(buffer, &result.bytes);
            context.ret_i32(0);
        }
        Err(errno) => finish_int(context, error(errno)),
    }
}

fn decode_device(device: u64) -> (u32, u32) {
    let major = ((device >> 8) & 0xfff) | ((device >> 32) & !0xfff);
    let minor = (device & 0xff) | ((device >> 12) & !0xff);
    (major as u32, minor as u32)
}

fn statx_bytes(result: &StatResult) -> [u8; STATX_SIZE] {
    // cite: bionic stat.h#statx
    let stat = &result.bytes;
    let word = |offset: usize| u64::from_le_bytes(stat[offset..offset + 8].try_into().expect("8 bytes"));
    let half = |offset: usize| u32::from_le_bytes(stat[offset..offset + 4].try_into().expect("4 bytes"));
    let mut out = [0u8; STATX_SIZE];
    let mut put = |offset: usize, bytes: &[u8]| out[offset..offset + bytes.len()].copy_from_slice(bytes);
    put(0, &(STATX_BASIC_STATS | STATX_BTIME).to_le_bytes());
    put(4, &half(layout::ST_BLKSIZE).to_le_bytes());
    put(16, &half(layout::ST_NLINK).to_le_bytes());
    put(20, &half(layout::ST_UID).to_le_bytes());
    put(24, &half(layout::ST_GID).to_le_bytes());
    put(28, &(half(layout::ST_MODE) as u16).to_le_bytes());
    put(32, &word(layout::ST_INO).to_le_bytes());
    put(40, &word(layout::ST_SIZE).to_le_bytes());
    put(48, &word(layout::ST_BLOCKS).to_le_bytes());
    for (target, source) in [(64, layout::ST_ATIME), (96, layout::ST_CTIME), (112, layout::ST_MTIME)] {
        put(target, &word(source).to_le_bytes());
        put(target + 8, &(word(source + 8) as u32).to_le_bytes());
    }
    put(80, &result.birth.0.to_le_bytes());
    put(88, &(result.birth.1 as u32).to_le_bytes());
    let (rdev_major, rdev_minor) = decode_device(word(layout::ST_RDEV));
    let (dev_major, dev_minor) = decode_device(word(layout::ST_DEV));
    put(128, &rdev_major.to_le_bytes());
    put(132, &rdev_minor.to_le_bytes());
    put(136, &dev_major.to_le_bytes());
    put(140, &dev_minor.to_le_bytes());
    out
}

fn host_filesystem_stats(resolved: &Resolved) -> Result<libc::statfs, i32> {
    let mut stats: libc::statfs = unsafe { std::mem::zeroed() };
    let status = match resolved {
        Resolved::Host(reference) => {
            let fd = reference.open(libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC, 0);
            if fd < 0 {
                return Err((-fd) as i32);
            }
            let status = unsafe { libc::fstatfs(fd as i32, &mut stats) };
            unsafe { libc::close(fd as i32) };
            status
        }
        _ => unsafe { libc::statfs(c"/".as_ptr(), &mut stats) },
    };
    if status < 0 {
        return Err((-from_host()) as i32);
    }
    Ok(stats)
}

fn filesystem_kind(resolved: &Resolved) -> Option<i64> {
    match resolved {
        Resolved::Synthetic(node, _) if node.device() == super::synthetic_node::PROC_DEVICE => Some(0x9fa0),
        Resolved::Synthetic(node, _) if node.device() == super::synthetic_node::SYS_DEVICE => Some(0x62656572),
        Resolved::Synthetic(..) | Resolved::Device(_) | Resolved::Virtual(_) => Some(0x01021994),
        Resolved::Host(_) => None,
    }
}

fn statfs_of(resolved: &Resolved) -> Result<[u8; 120], i32> {
    let mut bytes = stat_convert::statfs_bytes(&host_filesystem_stats(resolved)?);
    if let Some(kind) = filesystem_kind(resolved) {
        bytes[0..8].copy_from_slice(&kind.to_le_bytes());
    }
    // cite: bionic sys_statfs.cpp#ST_VALID
    let flags = i64::from_le_bytes(bytes[80..88].try_into().expect("8 bytes")) & !0x20;
    bytes[80..88].copy_from_slice(&flags.to_le_bytes());
    Ok(bytes)
}

fn statvfs_of(statfs: &[u8; 120]) -> [u8; 112] {
    // cite: bionic sys_statvfs.cpp#__bionic_statfs_to_statvfs
    let word = |offset: usize| u64::from_le_bytes(statfs[offset..offset + 8].try_into().expect("8 bytes"));
    let mut out = [0u8; 112];
    let mut put = |offset: usize, value: u64| out[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    put(0, word(8));
    put(8, word(72));
    put(16, word(16));
    put(24, word(24));
    put(32, word(32));
    put(40, word(40));
    put(48, word(48));
    put(56, word(48));
    let fsid = u64::from(u32::from_le_bytes(statfs[56..60].try_into().expect("4 bytes"))) | u64::from(u32::from_le_bytes(statfs[60..64].try_into().expect("4 bytes"))) << 32;
    put(64, fsid);
    put(72, word(80));
    put(80, word(64));
    out
}

fn statfs_for_fd(fd: i32) -> Result<[u8; 120], i32> {
    let entry = fd_table::get(fd).ok_or(EBADF)?;
    let mut stats: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstatfs(fd, &mut stats) } < 0 {
        return Err((-from_host()) as i32);
    }
    let mut bytes = stat_convert::statfs_bytes(&stats);
    if entry.synthetic.is_some() {
        bytes[0..8].copy_from_slice(&0x9fa0i64.to_le_bytes());
    }
    let flags = i64::from_le_bytes(bytes[80..88].try_into().expect("8 bytes")) & !0x20;
    bytes[80..88].copy_from_slice(&flags.to_le_bytes());
    Ok(bytes)
}

pub fn access_at(dirfd: i32, path: &[u8], mode: i32, flags: i32) -> i64 {
    // cite: bionic faccessat.cpp#faccessat
    if mode & !ACCESS_MODES != 0 || flags != 0 {
        return error(EINVAL);
    }
    let resolved = match guest_paths::resolve_follow(dirfd, path, Access::Read) {
        Ok(resolved) => resolved,
        Err(errno) => return error(errno),
    };
    let wants_write = mode & 2 != 0;
    match &resolved {
        Resolved::Host(reference) => {
            if wants_write && reference.read_only {
                return error(EROFS);
            }
            if unsafe { libc::faccessat(reference.dir_fd(), reference.name().as_ptr(), mode, libc::AT_EACCESS) } < 0 {
                from_host()
            } else {
                0
            }
        }
        Resolved::Device(_) => 0,
        Resolved::Synthetic(node, _) => {
            let executable = mode & 1 != 0 && !matches!(node, Synthetic::Dir { .. });
            if wants_write || executable {
                error(EACCES)
            } else {
                0
            }
        }
        Resolved::Virtual(_) => {
            if wants_write {
                error(EACCES)
            } else {
                0
            }
        }
    }
}

pub fn register() {
    for name in ["stat", "stat64"] {
        in_libc(name, |context| match path_argument(context.arg_ptr(0)) {
            Ok(path) => {
                let outcome = stat_at(at::FDCWD, path, 0);
                deliver(context, outcome, context.arg_ptr(1));
            }
            Err(errno) => finish_int(context, error(errno)),
        });
    }
    for name in ["lstat", "lstat64"] {
        in_libc(name, |context| match path_argument(context.arg_ptr(0)) {
            Ok(path) => {
                let outcome = stat_at(at::FDCWD, path, at::SYMLINK_NOFOLLOW);
                deliver(context, outcome, context.arg_ptr(1));
            }
            Err(errno) => finish_int(context, error(errno)),
        });
    }
    for name in ["fstat", "fstat64"] {
        in_libc(name, |context| {
            let outcome = stat_fd(context.arg_i32(0));
            deliver(context, outcome, context.arg_ptr(1));
        });
    }
    for name in ["fstatat", "fstatat64"] {
        in_libc(name, |context| match path_argument(context.arg_ptr(1)) {
            Ok(path) => {
                let outcome = stat_at(context.arg_i32(0), path, context.arg_i32(3));
                deliver(context, outcome, context.arg_ptr(2));
            }
            Err(errno) => finish_int(context, error(errno)),
        });
    }
    in_libc("statx", |context| {
        let (dirfd, flags, buffer) = (context.arg_i32(0), context.arg_i32(2), context.arg_ptr(4));
        let result = match path_argument(context.arg_ptr(1)) {
            Ok(path) => stat_at(dirfd, path, flags & (at::SYMLINK_NOFOLLOW | at::EMPTY_PATH)),
            Err(errno) => Err(errno),
        };
        let unknown = flags & !(at::SYMLINK_NOFOLLOW | at::EMPTY_PATH | 0x800 | 0x6000);
        match result {
            Ok(result) if unknown == 0 => {
                write_bytes(buffer, &statx_bytes(&result));
                context.ret_i32(0);
            }
            Ok(_) => finish_int(context, error(EINVAL)),
            Err(errno) => finish_int(context, error(errno)),
        }
    });
    for name in ["statfs", "statfs64"] {
        in_libc(name, |context| match path_argument(context.arg_ptr(0)) {
            Ok(path) => match guest_paths::resolve_follow(at::FDCWD, path, Access::Read).and_then(|resolved| statfs_of(&resolved)) {
                Ok(bytes) => {
                    write_bytes(context.arg_ptr(1), &bytes);
                    context.ret_i32(0);
                }
                Err(errno) => finish_int(context, error(errno)),
            },
            Err(errno) => finish_int(context, error(errno)),
        });
    }
    for name in ["fstatfs", "fstatfs64"] {
        in_libc(name, |context| match statfs_for_fd(context.arg_i32(0)) {
            Ok(bytes) => {
                write_bytes(context.arg_ptr(1), &bytes);
                context.ret_i32(0);
            }
            Err(errno) => finish_int(context, error(errno)),
        });
    }
    for name in ["statvfs", "statvfs64"] {
        in_libc(name, |context| match path_argument(context.arg_ptr(0)) {
            Ok(path) => match guest_paths::resolve_follow(at::FDCWD, path, Access::Read).and_then(|resolved| statfs_of(&resolved)) {
                Ok(bytes) => {
                    write_bytes(context.arg_ptr(1), &statvfs_of(&bytes));
                    context.ret_i32(0);
                }
                Err(errno) => finish_int(context, error(errno)),
            },
            Err(errno) => finish_int(context, error(errno)),
        });
    }
    for name in ["fstatvfs", "fstatvfs64"] {
        in_libc(name, |context| match statfs_for_fd(context.arg_i32(0)) {
            Ok(bytes) => {
                write_bytes(context.arg_ptr(1), &statvfs_of(&bytes));
                context.ret_i32(0);
            }
            Err(errno) => finish_int(context, error(errno)),
        });
    }
    in_libc("access", |context| match path_argument(context.arg_ptr(0)) {
        Ok(path) => {
            let result = access_at(at::FDCWD, path, context.arg_i32(1), 0);
            finish_int(context, result);
        }
        Err(errno) => finish_int(context, error(errno)),
    });
    in_libc("faccessat", |context| match path_argument(context.arg_ptr(1)) {
        Ok(path) => {
            let result = access_at(context.arg_i32(0), path, context.arg_i32(2), context.arg_i32(3));
            finish_int(context, result);
        }
        Err(errno) => finish_int(context, error(errno)),
    });
}
