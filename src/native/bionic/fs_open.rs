use std::ffi::CString;
use std::sync::atomic::Ordering;

use super::super::bridge::CallContext;
use super::abi_types::{EFAULT, EINVAL};
use super::export::in_libc;
use super::fatal::fortify_fatal;
use super::fd_table::{self, FdEntry, FdKind, SyntheticStat, SEALING_ALLOWED};
use super::fs_result::{finish_int, from_host};
use super::guest_mem::{cstr_bytes, write_bytes};
use super::guest_paths::{self, Access, Device, HostRef, Resolved, VirtualDir};
use super::guest_process;
use super::linux_abi::{at, misc, open};
use super::stat_convert::{S_IFCHR, S_IFDIR, S_IFREG};
use super::synthetic_node::Synthetic;
use super::temp_file::{anonymous_file, placeholder_fd, random_suffix};
use super::translate::{open_flags_to_host, DARWIN_F_NOCACHE};

const EACCES: i32 = 13;
const EEXIST: i32 = 17;
const EISDIR: i32 = 21;
const EROFS: i32 = 30;
const NAME_LIMIT: usize = 249;
const MAX_TEMP_ATTEMPTS: usize = 100;
const RECORDED_OFF: i32 = open::CREAT | open::EXCL | open::TRUNC | open::NOCTTY;
const HOST_ONLY_BITS: i32 =
    open::DIRECT | open::LARGEFILE | open::NOATIME | open::PATH | open::TMPFILE_BIT;

pub fn path_argument(pointer: usize) -> Result<&'static [u8], i32> {
    if pointer == 0 {
        Err(EFAULT)
    } else {
        Ok(cstr_bytes(pointer))
    }
}

fn needs_mode(flags: i32) -> bool {
    // cite: bionic open.cpp#needs_mode
    flags & open::CREAT == open::CREAT || flags & open::TMPFILE == open::TMPFILE
}

fn wants_write(flags: i32) -> bool {
    flags & open::ACCMODE != open::RDONLY
        || flags & (open::CREAT | open::TRUNC | open::TMPFILE_BIT) != 0
}

fn recorded_flags(flags: i32) -> i32 {
    (flags & !RECORDED_OFF) | open::LARGEFILE
}

fn adopt_host(
    fd: i64,
    kind: FdKind,
    guest_path: &str,
    flags: i32,
    synthetic: Option<SyntheticStat>,
) -> i64 {
    let mut entry = FdEntry::new(kind, guest_path, recorded_flags(flags));
    entry.synthetic = synthetic;
    fd_table::adopt(fd as i32, entry);
    fd
}

fn open_host_file(reference: &HostRef, flags: i32, mode: u32) -> i64 {
    if wants_write(flags) && reference.read_only {
        return -i64::from(EROFS);
    }
    if flags & open::TMPFILE == open::TMPFILE {
        return open_unnamed(reference, flags, mode);
    }
    let mut host_flags = open_flags_to_host(flags & !HOST_ONLY_BITS) | libc::O_NOFOLLOW;
    let path_only = flags & open::PATH != 0;
    if path_only {
        host_flags =
            libc::O_RDONLY | libc::O_NOFOLLOW | (open_flags_to_host(flags) & libc::O_CLOEXEC);
    }
    let created = guest_process::apply_umask(mode);
    let fd = reference.open(host_flags, created);
    if fd < 0 {
        return fd;
    }
    let mut stat: libc::stat = unsafe { std::mem::zeroed() };
    unsafe { libc::fstat(fd as i32, &mut stat) };
    let directory = u32::from(stat.st_mode) & super::stat_convert::S_IFMT == S_IFDIR;
    if flags & open::DIRECT != 0 {
        unsafe { libc::fcntl(fd as i32, DARWIN_F_NOCACHE, 1) };
    }
    let kind = if path_only {
        FdKind::PathOnly
    } else if directory {
        FdKind::Dir
    } else {
        FdKind::Host
    };
    adopt_host(fd, kind, &reference.guest_path, flags, None)
}

fn open_unnamed(reference: &HostRef, flags: i32, mode: u32) -> i64 {
    let access = flags & open::ACCMODE;
    if access != open::WRONLY && access != open::RDWR {
        return -i64::from(EINVAL);
    }
    let directory = reference.open(libc::O_RDONLY | libc::O_DIRECTORY, 0);
    if directory < 0 {
        return directory;
    }
    let directory = directory as i32;
    let created = guest_process::apply_umask(mode);
    for _ in 0..MAX_TEMP_ATTEMPTS {
        let suffix = random_suffix();
        let name = CString::new(format!(".wd-tmpfile-{}", String::from_utf8_lossy(&suffix)))
            .expect("no nul");
        let fd = unsafe {
            libc::openat(
                directory,
                name.as_ptr(),
                libc::O_CREAT | libc::O_EXCL | libc::O_RDWR | libc::O_CLOEXEC | libc::O_NOFOLLOW,
                created as libc::c_uint,
            )
        };
        if fd >= 0 {
            unsafe { libc::unlinkat(directory, name.as_ptr(), 0) };
            unsafe { libc::close(directory) };
            return adopt_host(
                i64::from(fd),
                FdKind::Host,
                &reference.guest_path,
                flags,
                None,
            );
        }
        let error = from_host();
        if error != -i64::from(EEXIST) {
            unsafe { libc::close(directory) };
            return error;
        }
    }
    unsafe { libc::close(directory) };
    -i64::from(EEXIST)
}

fn open_device(device: &Device, flags: i32) -> i64 {
    let host = CString::new(guest_paths::device_host_path(device.name)).expect("no nul");
    let host_flags =
        open_flags_to_host(flags & !HOST_ONLY_BITS & !(open::CREAT | open::EXCL | open::TRUNC));
    let fd = unsafe { libc::open(host.as_ptr(), host_flags) };
    if fd < 0 {
        return from_host();
    }
    let synthetic = SyntheticStat {
        mode: S_IFCHR | 0o666,
        dev: (0, 5),
        rdev: device.rdev,
        zero_size: false,
    };
    adopt_host(
        i64::from(fd),
        FdKind::Host,
        &format!("/dev/{}", device.name),
        flags,
        Some(synthetic),
    )
}

fn open_synthetic(node: Synthetic, path: &str, flags: i32) -> i64 {
    if wants_write(flags) {
        return -i64::from(EACCES);
    }
    match node {
        Synthetic::File { content, device } => {
            let fd = anonymous_file();
            if fd < 0 {
                return fd;
            }
            let bytes = content();
            let written = unsafe {
                libc::write(
                    fd as i32,
                    bytes.as_ptr() as *const libc::c_void,
                    bytes.len(),
                )
            };
            if written < 0 || written as usize != bytes.len() {
                unsafe { libc::close(fd as i32) };
                return from_host();
            }
            unsafe { libc::lseek(fd as i32, 0, libc::SEEK_SET) };
            if flags & open::CLOEXEC == 0 {
                unsafe { libc::fcntl(fd as i32, libc::F_SETFD, 0) };
            }
            let zero_size = device == super::synthetic_node::PROC_DEVICE
                || device == super::synthetic_node::SYS_DEVICE;
            adopt_host(
                fd,
                FdKind::Host,
                path,
                flags,
                Some(SyntheticStat {
                    mode: S_IFREG | 0o444,
                    dev: device,
                    rdev: (0, 0),
                    zero_size,
                }),
            )
        }
        Synthetic::Dir { entries, device } => open_listing(entries(), path, flags, 0o555, device),
        Synthetic::Link { .. } => -i64::from(super::abi_types::ENOENT),
    }
}

fn open_listing(
    entries: Vec<(String, u8)>,
    path: &str,
    flags: i32,
    mode: u32,
    device: (u32, u32),
) -> i64 {
    if flags & open::ACCMODE != open::RDONLY {
        return -i64::from(EISDIR);
    }
    let fd = placeholder_fd();
    if fd < 0 {
        return fd;
    }
    let items = entries
        .into_iter()
        .map(|(name, kind)| fd_table::DirItem {
            inode: super::stat_convert::path_hash(&format!("{path}/{name}")),
            name: name.into_bytes(),
            kind,
        })
        .collect();
    let entry =
        FdEntry::new(FdKind::Dir, path, recorded_flags(flags)).with_synthetic(SyntheticStat {
            mode: S_IFDIR | mode,
            dev: device,
            rdev: (0, 0),
            zero_size: false,
        });
    *entry.dir.lock().expect("dir state") = fd_table::DirState {
        items: Some(items),
        position: 0,
    };
    fd_table::adopt(fd as i32, entry);
    fd
}

fn open_virtual(directory: VirtualDir, flags: i32) -> i64 {
    let mut entries = vec![
        (".".to_string(), misc::DT_DIR),
        ("..".to_string(), misc::DT_DIR),
    ];
    entries.extend(directory.entries);
    if wants_write(flags) {
        return -i64::from(if flags & open::ACCMODE != open::RDONLY {
            EISDIR
        } else {
            EACCES
        });
    }
    open_listing(
        entries,
        &directory.guest_path,
        flags,
        directory.mode,
        (0, 0x1),
    )
}

pub fn open_resolved(resolved: Resolved, flags: i32, mode: u32) -> i64 {
    match resolved {
        Resolved::Host(reference) => open_host_file(&reference, flags, mode),
        Resolved::Device(device) => open_device(&device, flags),
        Resolved::Synthetic(node, path) => open_synthetic(node, &path, flags),
        Resolved::Virtual(directory) => open_virtual(directory, flags),
    }
}

pub fn open_at(dirfd: i32, path: &[u8], flags: i32, mode: u32) -> i64 {
    if flags & open::TMPFILE == open::TMPFILE && flags & open::ACCMODE == open::RDONLY {
        return -i64::from(EINVAL);
    }
    let access = if wants_write(flags) {
        Access::Write
    } else {
        Access::Read
    };
    let resolved = if flags & open::NOFOLLOW != 0 {
        guest_paths::resolve(dirfd, path, access)
    } else {
        guest_paths::resolve_follow(dirfd, path, access)
    };
    match resolved {
        Ok(resolved) => open_resolved(resolved, flags, mode),
        Err(errno) => -i64::from(errno),
    }
}

fn template_parts(pointer: usize, suffix: usize) -> Result<(Vec<u8>, usize), i32> {
    let template = path_argument(pointer)?.to_vec();
    // cite: bionic mktemp.c#mkostemps
    let marker = b"XXXXXX";
    if template.len() < marker.len() + suffix
        || &template[template.len() - suffix - marker.len()..template.len() - suffix] != marker
    {
        return Err(EINVAL);
    }
    Ok((template.clone(), template.len() - suffix - marker.len()))
}

fn mkstemp_at(pointer: usize, suffix: usize, extra_flags: i32) -> i64 {
    let (mut template, start) = match template_parts(pointer, suffix) {
        Ok(parts) => parts,
        Err(errno) => return -i64::from(errno),
    };
    let allowed = open::APPEND | open::CLOEXEC | open::SYNC;
    if extra_flags & !allowed != 0 {
        return -i64::from(EINVAL);
    }
    for _ in 0..MAX_TEMP_ATTEMPTS {
        template[start..start + 6].copy_from_slice(&random_suffix());
        let flags = open::RDWR | open::CREAT | open::EXCL | extra_flags;
        let result = open_at(at::FDCWD, &template, flags, 0o600);
        if result != -i64::from(EEXIST) {
            if result >= 0 {
                write_bytes(pointer + start, &template[start..start + 6]);
            }
            return result;
        }
    }
    -i64::from(EEXIST)
}

fn mkdtemp_at(pointer: usize) -> i64 {
    let (mut template, start) = match template_parts(pointer, 0) {
        Ok(parts) => parts,
        Err(errno) => return -i64::from(errno),
    };
    for _ in 0..MAX_TEMP_ATTEMPTS {
        template[start..start + 6].copy_from_slice(&random_suffix());
        let result = super::fs_ops::make_directory(at::FDCWD, &template, 0o700);
        if result != -i64::from(EEXIST) {
            if result == 0 {
                write_bytes(pointer + start, &template[start..start + 6]);
            }
            return result;
        }
    }
    -i64::from(EEXIST)
}

fn memfd_create(name: usize, flags: u32) -> i64 {
    if name == 0 {
        return -i64::from(EFAULT);
    }
    let name = cstr_bytes(name);
    if name.len() > NAME_LIMIT {
        return -i64::from(EINVAL);
    }
    // cite: bionic memfd.h#MFD_ALLOW_SEALING
    if flags & !(misc::MFD_CLOEXEC | misc::MFD_ALLOW_SEALING) != 0 {
        return -i64::from(EINVAL);
    }
    let fd = anonymous_file();
    if fd < 0 {
        return fd;
    }
    if flags & misc::MFD_CLOEXEC == 0 {
        unsafe { libc::fcntl(fd as i32, libc::F_SETFD, 0) };
    }
    let entry = FdEntry::new(
        FdKind::Host,
        format!("/memfd:{} (deleted)", String::from_utf8_lossy(name)),
        open::RDWR | open::LARGEFILE,
    );
    if flags & misc::MFD_ALLOW_SEALING != 0 {
        entry.seals.store(SEALING_ALLOWED, Ordering::SeqCst);
    }
    fd_table::adopt(fd as i32, entry);
    fd
}

fn handle_open(context: &mut CallContext, dirfd: i32, path_index: usize, checked: bool) {
    let flags = context.arg_i32(path_index + 1);
    if checked && needs_mode(flags) {
        fortify_fatal("open: called with O_CREAT/O_TMPFILE but no mode");
    }
    let mode = if needs_mode(flags) {
        context.arg_u32(path_index + 2)
    } else {
        0
    };
    let result = match path_argument(context.arg_ptr(path_index)) {
        Ok(path) => open_at(dirfd, path, flags, mode),
        Err(errno) => -i64::from(errno),
    };
    finish_int(context, result);
}

pub fn register() {
    for name in ["open", "open64"] {
        in_libc(name, |context| handle_open(context, at::FDCWD, 0, false));
    }
    in_libc("__open_2", |context| {
        handle_open(context, at::FDCWD, 0, true)
    });
    for name in ["openat", "openat64"] {
        in_libc(name, |context| {
            let dirfd = context.arg_i32(0);
            handle_open(context, dirfd, 1, false);
        });
    }
    in_libc("__openat_2", |context| {
        let dirfd = context.arg_i32(0);
        handle_open(context, dirfd, 1, true);
    });
    for name in ["creat", "creat64"] {
        in_libc(name, |context| {
            // cite: bionic open.cpp#creat
            let result = match path_argument(context.arg_ptr(0)) {
                Ok(path) => open_at(
                    at::FDCWD,
                    path,
                    open::WRONLY | open::CREAT | open::TRUNC,
                    context.arg_u32(1),
                ),
                Err(errno) => -i64::from(errno),
            };
            finish_int(context, result);
        });
    }
    for name in ["mkstemp", "mkstemp64"] {
        in_libc(name, |context| {
            let result = mkstemp_at(context.arg_ptr(0), 0, 0);
            finish_int(context, result);
        });
    }
    for name in ["mkostemp", "mkostemp64"] {
        in_libc(name, |context| {
            let result = mkstemp_at(context.arg_ptr(0), 0, context.arg_i32(1));
            finish_int(context, result);
        });
    }
    for name in ["mkstemps", "mkstemps64"] {
        in_libc(name, |context| {
            let result = mkstemp_at(context.arg_ptr(0), context.arg_i32(1).max(0) as usize, 0);
            finish_int(context, result);
        });
    }
    for name in ["mkostemps", "mkostemps64"] {
        in_libc(name, |context| {
            let result = mkstemp_at(
                context.arg_ptr(0),
                context.arg_i32(1).max(0) as usize,
                context.arg_i32(2),
            );
            finish_int(context, result);
        });
    }
    in_libc("mkdtemp", |context| {
        let result = mkdtemp_at(context.arg_ptr(0));
        if result < 0 {
            context.set_errno((-result) as i32);
            context.ret_ptr(0);
        } else {
            context.ret_ptr(context.arg_ptr(0));
        }
    });
    in_libc("memfd_create", |context| {
        let result = memfd_create(context.arg_ptr(0), context.arg_u32(1));
        finish_int(context, result);
    });
}
