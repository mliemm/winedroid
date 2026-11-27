use std::ffi::{CStr, CString};
use std::mem;
use std::os::fd::{AsFd, IntoRawFd, RawFd};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use std::sync::atomic::{fence, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

use rusqlite::ffi;
use rusqlite::{Connection, OpenFlags};
use rustix::fs::{FileType, Mode, OFlags};

use crate::capability_fs::Directory;

static VFS_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static PROCESS_SQLITE_LOCK: Mutex<()> = Mutex::new(());

const PENDING_BYTE: libc::off_t = 0x4000_0000;
const RESERVED_BYTE: libc::off_t = PENDING_BYTE + 1;
const SHARED_FIRST: libc::off_t = PENDING_BYTE + 2;
const SHARED_SIZE: libc::off_t = 510;
const SHM_LOCK_BASE: libc::off_t = 120;
const SHM_DMS_BYTE: libc::off_t = SHM_LOCK_BASE + ffi::SQLITE_SHM_NLOCK as libc::off_t;
const FILE_MODE: Mode = Mode::RUSR
    .union(Mode::WUSR)
    .union(Mode::RGRP)
    .union(Mode::WGRP)
    .union(Mode::ROTH)
    .union(Mode::WOTH);

pub struct CapabilityConnection {
    connection: Option<Connection>,
    _vfs: RegisteredVfs,
}

struct RegisteredVfs {
    raw: Box<ffi::sqlite3_vfs>,
    state: Box<VfsState>,
    name: CString,
}

struct VfsState {
    directory: Directory,
    database_leaf: CString,
    virtual_prefix: Vec<u8>,
    underlying: *mut ffi::sqlite3_vfs,
}

#[repr(C)]
struct CapabilityFile {
    base: ffi::sqlite3_file,
    state: *const VfsState,
    fd: RawFd,
    writable: bool,
    name: CString,
    device: u64,
    inode: u64,
    sync_parent: bool,
    lock_level: c_int,
    chunk_size: i64,
    shm_fd: RawFd,
    shm_name: Option<CString>,
    shm_maps: Vec<Option<ShmMap>>,
    _hold: Option<crate::fs_notify::Hold>,
    shm_hold: Option<crate::fs_notify::Hold>,
}

struct ShmMap {
    pointer: *mut c_void,
    length: usize,
}

enum ShmOpenError {
    Busy,
    Io,
}

impl RegisteredVfs {
    fn new(directory: Directory, database_leaf: CString) -> Result<Self, String> {
        let sequence = VFS_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let name = CString::new(format!("winedroid-dirfd-{sequence}"))
            .map_err(|error| error.to_string())?;
        let virtual_prefix = format!("/winedroid-dirfd-{sequence}/").into_bytes();
        let underlying = unsafe { ffi::sqlite3_vfs_find(ptr::null()) };
        if underlying.is_null() {
            return Err("SQLite has no default VFS".to_owned());
        }
        let mut state = Box::new(VfsState {
            directory,
            database_leaf,
            virtual_prefix,
            underlying,
        });
        let mut raw = Box::new(ffi::sqlite3_vfs {
            iVersion: 3,
            szOsFile: mem::size_of::<CapabilityFile>() as c_int,
            mxPathname: 512,
            pNext: ptr::null_mut(),
            zName: name.as_ptr(),
            pAppData: (&mut *state as *mut VfsState).cast(),
            xOpen: Some(vfs_open),
            xDelete: Some(vfs_delete),
            xAccess: Some(vfs_access),
            xFullPathname: Some(vfs_full_pathname),
            xDlOpen: Some(vfs_dl_open),
            xDlError: Some(vfs_dl_error),
            xDlSym: Some(vfs_dl_sym),
            xDlClose: Some(vfs_dl_close),
            xRandomness: Some(vfs_randomness),
            xSleep: Some(vfs_sleep),
            xCurrentTime: Some(vfs_current_time),
            xGetLastError: Some(vfs_get_last_error),
            xCurrentTimeInt64: Some(vfs_current_time_int64),
            xSetSystemCall: Some(vfs_set_system_call),
            xGetSystemCall: Some(vfs_get_system_call),
            xNextSystemCall: Some(vfs_next_system_call),
        });
        let result = unsafe { ffi::sqlite3_vfs_register(&mut *raw, 0) };
        if result != ffi::SQLITE_OK {
            return Err(format!("SQLite VFS registration failed with code {result}"));
        }
        Ok(Self { raw, state, name })
    }

    fn name(&self) -> &str {
        self.name.to_str().expect("generated VFS name is UTF-8")
    }

    fn database_name(&self) -> &str {
        self.state
            .database_leaf
            .to_str()
            .expect("guest database leaf is UTF-8")
    }
}

impl CapabilityConnection {
    pub fn open(directory: Directory, database_leaf: CString) -> Result<Self, String> {
        let vfs = RegisteredVfs::new(directory, database_leaf)?;
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let connection =
            Connection::open_with_flags_and_vfs(vfs.database_name(), flags, vfs.name())
                .map_err(|error| error.to_string())?;
        Ok(Self {
            connection: Some(connection),
            _vfs: vfs,
        })
    }

    pub fn connection(&self) -> &Connection {
        self.connection
            .as_ref()
            .expect("capability SQLite connection remains open")
    }
}

impl Drop for CapabilityConnection {
    fn drop(&mut self) {
        let _guard = process_lock();
        self.connection.take();
    }
}

pub fn process_lock() -> MutexGuard<'static, ()> {
    PROCESS_SQLITE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Drop for RegisteredVfs {
    fn drop(&mut self) {
        unsafe {
            ffi::sqlite3_vfs_unregister(&mut *self.raw);
        }
    }
}

static IO_METHODS: ffi::sqlite3_io_methods = ffi::sqlite3_io_methods {
    iVersion: 3,
    xClose: Some(file_close),
    xRead: Some(file_read),
    xWrite: Some(file_write),
    xTruncate: Some(file_truncate),
    xSync: Some(file_sync),
    xFileSize: Some(file_size),
    xLock: Some(file_lock),
    xUnlock: Some(file_unlock),
    xCheckReservedLock: Some(file_check_reserved_lock),
    xFileControl: Some(file_control),
    xSectorSize: Some(file_sector_size),
    xDeviceCharacteristics: Some(file_device_characteristics),
    xShmMap: Some(file_shm_map),
    xShmLock: Some(file_shm_lock),
    xShmBarrier: Some(file_shm_barrier),
    xShmUnmap: Some(file_shm_unmap),
    xFetch: Some(file_fetch),
    xUnfetch: Some(file_unfetch),
};

unsafe extern "C" fn vfs_open(
    vfs: *mut ffi::sqlite3_vfs,
    name: ffi::sqlite3_filename,
    output: *mut ffi::sqlite3_file,
    flags: c_int,
    output_flags: *mut c_int,
) -> c_int {
    if output.is_null() {
        return ffi::SQLITE_CANTOPEN;
    }
    (*output).pMethods = ptr::null();
    let Some(state) = vfs_state(vfs) else {
        return ffi::SQLITE_CANTOPEN;
    };
    let temporary = name.is_null();
    let Some(leaf) = requested_leaf(state, name, temporary) else {
        return ffi::SQLITE_CANTOPEN;
    };
    let mut open_flags = OFlags::CLOEXEC | OFlags::NOFOLLOW;
    if flags & ffi::SQLITE_OPEN_READWRITE != 0 {
        open_flags |= OFlags::RDWR;
    } else {
        open_flags |= OFlags::RDONLY;
    }
    if flags & ffi::SQLITE_OPEN_CREATE != 0 || temporary {
        open_flags |= OFlags::CREATE;
    }
    if flags & ffi::SQLITE_OPEN_EXCLUSIVE != 0 || temporary {
        open_flags |= OFlags::EXCL;
    }
    let target = crate::fs_notify::InodeKey::at(state.directory.as_fd(), &leaf)
        .map(|key| (key, crate::fs_notify::ANY_WATCH));
    if open_flags.intersects(OFlags::RDWR | OFlags::CREATE)
        && crate::fs_notify::unsynthesized(
            Some(state.directory.as_fd()),
            target,
            "SQLite VFS xOpen",
        )
        .is_some()
    {
        return ffi::SQLITE_CANTOPEN;
    }
    let track_creation = flags & ffi::SQLITE_OPEN_MAIN_JOURNAL != 0
        && flags & ffi::SQLITE_OPEN_CREATE != 0
        && flags & ffi::SQLITE_OPEN_EXCLUSIVE == 0
        && !temporary;
    let opening = crate::fs_notify::Opening::begin(state.directory.as_fd(), &leaf);
    let opened = if track_creation {
        state
            .directory
            .open_leaf_reporting_creation(&leaf, open_flags, FILE_MODE)
    } else {
        state
            .directory
            .open_leaf(&leaf, open_flags, FILE_MODE)
            .map(|fd| (fd, false))
    };
    let (fd, actual_flags, created) = match opened {
        Ok((fd, created)) => (fd, flags, created),
        Err(_)
            if flags & ffi::SQLITE_OPEN_READWRITE != 0 && flags & ffi::SQLITE_OPEN_CREATE == 0 =>
        {
            let fallback = OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW;
            match state.directory.open_leaf(&leaf, fallback, Mode::empty()) {
                Ok(fd) => (
                    fd,
                    (flags & !ffi::SQLITE_OPEN_READWRITE) | ffi::SQLITE_OPEN_READONLY,
                    false,
                ),
                Err(_) => return ffi::SQLITE_CANTOPEN,
            }
        }
        Err(_) => return ffi::SQLITE_CANTOPEN,
    };
    let hold = crate::fs_notify::Hold::open(opening, fd.as_fd(), false);
    let fd = fd.into_raw_fd();
    let delete_on_close = flags & ffi::SQLITE_OPEN_DELETEONCLOSE != 0 || temporary;
    if delete_on_close && state.directory.unlink_leaf(&leaf).is_err() {
        libc::close(fd);
        return ffi::SQLITE_CANTOPEN;
    }
    let mut stat = mem::zeroed::<libc::stat>();
    if libc::fstat(fd, &mut stat) != 0 {
        libc::close(fd);
        return ffi::SQLITE_CANTOPEN;
    }
    if !output_flags.is_null() {
        *output_flags = actual_flags;
    }
    ptr::write(
        output.cast::<CapabilityFile>(),
        CapabilityFile {
            base: ffi::sqlite3_file {
                pMethods: &IO_METHODS,
            },
            state: state as *const VfsState,
            fd,
            writable: actual_flags & ffi::SQLITE_OPEN_READWRITE != 0,
            name: leaf,
            device: stat.st_dev as u64,
            inode: stat.st_ino as u64,
            sync_parent: created && flags & ffi::SQLITE_OPEN_MAIN_JOURNAL != 0,
            lock_level: ffi::SQLITE_LOCK_NONE,
            chunk_size: 0,
            shm_fd: -1,
            shm_name: None,
            shm_maps: Vec::new(),
            _hold: hold,
            shm_hold: None,
        },
    );
    ffi::SQLITE_OK
}

unsafe extern "C" fn vfs_delete(
    vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
    sync_directory: c_int,
) -> c_int {
    let Some(state) = vfs_state(vfs) else {
        return ffi::SQLITE_IOERR_DELETE;
    };
    let Some(leaf) = requested_leaf(state, name, false) else {
        return ffi::SQLITE_IOERR_DELETE;
    };
    match state.directory.unlink_leaf(&leaf) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return ffi::SQLITE_IOERR_DELETE_NOENT;
        }
        Err(_) => return ffi::SQLITE_IOERR_DELETE,
    }
    if sync_directory != 0 && state.directory.sync().is_err() {
        return ffi::SQLITE_IOERR_FSYNC;
    }
    ffi::SQLITE_OK
}

unsafe extern "C" fn vfs_access(
    vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
    flags: c_int,
    output: *mut c_int,
) -> c_int {
    if output.is_null() {
        return ffi::SQLITE_IOERR_ACCESS;
    }
    *output = 0;
    let Some(state) = vfs_state(vfs) else {
        return ffi::SQLITE_IOERR_ACCESS;
    };
    let Some(leaf) = requested_leaf(state, name, false) else {
        return ffi::SQLITE_IOERR_ACCESS;
    };
    let exists = match state.directory.stat_leaf(&leaf) {
        Ok(stat) => FileType::from_raw_mode(stat.st_mode) != FileType::Symlink,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => return ffi::SQLITE_IOERR_ACCESS,
    };
    let allowed = match flags {
        ffi::SQLITE_ACCESS_EXISTS => exists,
        ffi::SQLITE_ACCESS_READ => {
            exists
                && state
                    .directory
                    .open_leaf(
                        &leaf,
                        OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                        Mode::empty(),
                    )
                    .is_ok()
        }
        ffi::SQLITE_ACCESS_READWRITE => {
            exists
                && state
                    .directory
                    .open_leaf(
                        &leaf,
                        OFlags::RDWR | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                        Mode::empty(),
                    )
                    .is_ok()
        }
        _ => false,
    };
    *output = c_int::from(allowed);
    ffi::SQLITE_OK
}

unsafe extern "C" fn vfs_full_pathname(
    vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
    output_length: c_int,
    output: *mut c_char,
) -> c_int {
    if output.is_null() || output_length <= 0 {
        return ffi::SQLITE_CANTOPEN_FULLPATH;
    }
    let Some(state) = vfs_state(vfs) else {
        return ffi::SQLITE_CANTOPEN_FULLPATH;
    };
    let Some(leaf) = requested_leaf(state, name, false) else {
        return ffi::SQLITE_CANTOPEN_FULLPATH;
    };
    let mut full = state.virtual_prefix.clone();
    full.extend_from_slice(leaf.as_bytes());
    if full.len() + 1 > output_length as usize {
        return ffi::SQLITE_CANTOPEN_FULLPATH;
    }
    ptr::copy_nonoverlapping(full.as_ptr().cast::<c_char>(), output, full.len());
    *output.add(full.len()) = 0;
    ffi::SQLITE_OK
}

unsafe extern "C" fn file_close(file: *mut ffi::sqlite3_file) -> c_int {
    let file = ptr::read(file.cast::<CapabilityFile>());
    for map in file.shm_maps.into_iter().flatten() {
        libc::munmap(map.pointer, map.length);
    }
    if file.shm_fd >= 0 {
        libc::close(file.shm_fd);
    }
    let closed = libc::close(file.fd) == 0;
    let target = crate::fs_notify::InodeKey::from_raw(file.device, file.inode);
    let unreported = file.writable
        && crate::fs_notify::unsynthesized(
            Some((*file.state).directory.as_fd()),
            Some((target, crate::fs_notify::IN_CLOSE_WRITE)),
            "SQLite VFS xClose",
        )
        .is_some();
    if closed && !unreported {
        ffi::SQLITE_OK
    } else {
        ffi::SQLITE_IOERR_CLOSE
    }
}

unsafe extern "C" fn file_read(
    file: *mut ffi::sqlite3_file,
    output: *mut c_void,
    amount: c_int,
    offset: ffi::sqlite3_int64,
) -> c_int {
    if output.is_null() || amount < 0 || offset < 0 {
        return ffi::SQLITE_IOERR_READ;
    }
    let file = capability_file(file);
    let output = std::slice::from_raw_parts_mut(output.cast::<u8>(), amount as usize);
    let mut read = 0;
    while read < output.len() {
        let result = libc::pread(
            file.fd,
            output[read..].as_mut_ptr().cast(),
            output.len() - read,
            offset + read as i64,
        );
        if result > 0 {
            read += result as usize;
            continue;
        }
        if result < 0 && last_errno() == libc::EINTR {
            continue;
        }
        if result < 0 {
            return ffi::SQLITE_IOERR_READ;
        }
        output[read..].fill(0);
        return ffi::SQLITE_IOERR_SHORT_READ;
    }
    ffi::SQLITE_OK
}

unsafe extern "C" fn file_write(
    file: *mut ffi::sqlite3_file,
    input: *const c_void,
    amount: c_int,
    offset: ffi::sqlite3_int64,
) -> c_int {
    if input.is_null() || amount < 0 || offset < 0 {
        return ffi::SQLITE_IOERR_WRITE;
    }
    let file = capability_file(file);
    let input = std::slice::from_raw_parts(input.cast::<u8>(), amount as usize);
    let mut written = 0;
    while written < input.len() {
        let result = libc::pwrite(
            file.fd,
            input[written..].as_ptr().cast(),
            input.len() - written,
            offset + written as i64,
        );
        if result > 0 {
            written += result as usize;
            continue;
        }
        if result < 0 && last_errno() == libc::EINTR {
            continue;
        }
        return ffi::SQLITE_IOERR_WRITE;
    }
    ffi::SQLITE_OK
}

unsafe extern "C" fn file_truncate(
    file: *mut ffi::sqlite3_file,
    size: ffi::sqlite3_int64,
) -> c_int {
    let file = capability_file(file);
    if size < 0 || libc::ftruncate(file.fd, size) != 0 {
        return ffi::SQLITE_IOERR_TRUNCATE;
    }
    ffi::SQLITE_OK
}

unsafe extern "C" fn file_sync(file: *mut ffi::sqlite3_file, flags: c_int) -> c_int {
    let file = capability_file(file);
    if sync_file_descriptor(file.fd, flags).is_err() {
        return ffi::SQLITE_IOERR_FSYNC;
    }
    if file.sync_parent {
        let Some(state) = file.state.as_ref() else {
            return ffi::SQLITE_IOERR_FSYNC;
        };
        if state.directory.sync().is_err() {
            return ffi::SQLITE_IOERR_FSYNC;
        }
        file.sync_parent = false;
    }
    ffi::SQLITE_OK
}

unsafe extern "C" fn file_size(
    file: *mut ffi::sqlite3_file,
    output: *mut ffi::sqlite3_int64,
) -> c_int {
    if output.is_null() {
        return ffi::SQLITE_IOERR_FSTAT;
    }
    let file = capability_file(file);
    let mut stat = mem::zeroed::<libc::stat>();
    if libc::fstat(file.fd, &mut stat) != 0 {
        return ffi::SQLITE_IOERR_FSTAT;
    }
    *output = stat.st_size;
    ffi::SQLITE_OK
}

unsafe extern "C" fn file_lock(file: *mut ffi::sqlite3_file, requested: c_int) -> c_int {
    let file = capability_file(file);
    if requested <= file.lock_level {
        return ffi::SQLITE_OK;
    }
    if file.lock_level < ffi::SQLITE_LOCK_SHARED && requested >= ffi::SQLITE_LOCK_SHARED {
        if set_lock(file.fd, libc::F_RDLCK as i16, PENDING_BYTE, 1).is_err() {
            return sqlite_lock_failure(ffi::SQLITE_IOERR_LOCK, last_errno());
        }
        let shared = set_lock(file.fd, libc::F_RDLCK as i16, SHARED_FIRST, SHARED_SIZE);
        let shared_errno = last_errno();
        let _ = set_lock(file.fd, libc::F_UNLCK as i16, PENDING_BYTE, 1);
        if shared.is_err() {
            return sqlite_lock_failure(ffi::SQLITE_IOERR_LOCK, shared_errno);
        }
        file.lock_level = ffi::SQLITE_LOCK_SHARED;
    }
    if file.lock_level < ffi::SQLITE_LOCK_RESERVED && requested >= ffi::SQLITE_LOCK_RESERVED {
        if set_lock(file.fd, libc::F_WRLCK as i16, RESERVED_BYTE, 1).is_err() {
            return sqlite_lock_failure(ffi::SQLITE_IOERR_LOCK, last_errno());
        }
        file.lock_level = ffi::SQLITE_LOCK_RESERVED;
    }
    if file.lock_level < ffi::SQLITE_LOCK_PENDING && requested >= ffi::SQLITE_LOCK_PENDING {
        if set_lock(file.fd, libc::F_WRLCK as i16, PENDING_BYTE, 1).is_err() {
            return sqlite_lock_failure(ffi::SQLITE_IOERR_LOCK, last_errno());
        }
        file.lock_level = ffi::SQLITE_LOCK_PENDING;
    }
    if requested >= ffi::SQLITE_LOCK_EXCLUSIVE {
        if set_lock(file.fd, libc::F_WRLCK as i16, SHARED_FIRST, SHARED_SIZE).is_err() {
            return sqlite_lock_failure(ffi::SQLITE_IOERR_LOCK, last_errno());
        }
        file.lock_level = ffi::SQLITE_LOCK_EXCLUSIVE;
    }
    ffi::SQLITE_OK
}

unsafe extern "C" fn file_unlock(file: *mut ffi::sqlite3_file, target: c_int) -> c_int {
    let file = capability_file(file);
    let result = if target == ffi::SQLITE_LOCK_NONE {
        set_lock(file.fd, libc::F_UNLCK as i16, PENDING_BYTE, SHARED_SIZE + 2)
    } else if target == ffi::SQLITE_LOCK_SHARED {
        set_lock(file.fd, libc::F_RDLCK as i16, SHARED_FIRST, SHARED_SIZE)
            .and_then(|_| set_lock(file.fd, libc::F_UNLCK as i16, PENDING_BYTE, 2))
    } else {
        Ok(())
    };
    if result.is_err() {
        return ffi::SQLITE_IOERR_UNLOCK;
    }
    file.lock_level = target;
    ffi::SQLITE_OK
}

unsafe extern "C" fn file_check_reserved_lock(
    file: *mut ffi::sqlite3_file,
    output: *mut c_int,
) -> c_int {
    if output.is_null() {
        return ffi::SQLITE_IOERR_CHECKRESERVEDLOCK;
    }
    let file = capability_file(file);
    if file.lock_level >= ffi::SQLITE_LOCK_RESERVED {
        *output = 1;
        return ffi::SQLITE_OK;
    }
    match lock_is_held(file.fd, RESERVED_BYTE, 1) {
        Ok(held) => {
            *output = c_int::from(held);
            ffi::SQLITE_OK
        }
        Err(()) => ffi::SQLITE_IOERR_CHECKRESERVEDLOCK,
    }
}

unsafe extern "C" fn file_control(
    file: *mut ffi::sqlite3_file,
    operation: c_int,
    argument: *mut c_void,
) -> c_int {
    let file = capability_file(file);
    match operation {
        ffi::SQLITE_FCNTL_LOCKSTATE if !argument.is_null() => {
            *argument.cast::<c_int>() = file.lock_level;
            ffi::SQLITE_OK
        }
        ffi::SQLITE_FCNTL_CHUNK_SIZE if !argument.is_null() => {
            file.chunk_size = i64::from(*argument.cast::<c_int>());
            ffi::SQLITE_OK
        }
        ffi::SQLITE_FCNTL_SIZE_HINT if !argument.is_null() => {
            let mut size = *argument.cast::<ffi::sqlite3_int64>();
            if file.chunk_size > 0 {
                size = ((size + file.chunk_size - 1) / file.chunk_size) * file.chunk_size;
            }
            let mut stat = mem::zeroed::<libc::stat>();
            if libc::fstat(file.fd, &mut stat) != 0 {
                return ffi::SQLITE_IOERR_FSTAT;
            }
            if stat.st_size < size && libc::ftruncate(file.fd, size) != 0 {
                return ffi::SQLITE_IOERR_TRUNCATE;
            }
            ffi::SQLITE_OK
        }
        ffi::SQLITE_FCNTL_HAS_MOVED if !argument.is_null() => {
            let moved = file.state.as_ref().map_or(true, |state| {
                state.directory.stat_leaf(&file.name).map_or(true, |stat| {
                    FileType::from_raw_mode(stat.st_mode) == FileType::Symlink
                        || stat.st_dev as u64 != file.device
                        || stat.st_ino as u64 != file.inode
                })
            });
            *argument.cast::<c_int>() = c_int::from(moved);
            ffi::SQLITE_OK
        }
        ffi::SQLITE_FCNTL_SYNC | ffi::SQLITE_FCNTL_COMMIT_PHASETWO => ffi::SQLITE_OK,
        _ => ffi::SQLITE_NOTFOUND,
    }
}

unsafe extern "C" fn file_sector_size(_file: *mut ffi::sqlite3_file) -> c_int {
    4096
}

unsafe extern "C" fn file_device_characteristics(_file: *mut ffi::sqlite3_file) -> c_int {
    0
}

unsafe extern "C" fn file_shm_map(
    file: *mut ffi::sqlite3_file,
    page: c_int,
    page_size: c_int,
    extend: c_int,
    output: *mut *mut c_void,
) -> c_int {
    if output.is_null() || page < 0 || page_size <= 0 {
        return ffi::SQLITE_IOERR_SHMMAP;
    }
    *output = ptr::null_mut();
    let file = capability_file(file);
    match ensure_shm_open(file) {
        Ok(()) => {}
        Err(ShmOpenError::Busy) => return ffi::SQLITE_BUSY,
        Err(ShmOpenError::Io) => return ffi::SQLITE_IOERR_SHMOPEN,
    }
    let page = page as usize;
    let page_size = page_size as usize;
    let regions_per_map = shm_regions_per_map(page_size);
    let map_index = page / regions_per_map;
    let region_index = page % regions_per_map;
    let Some(map_length) = regions_per_map.checked_mul(page_size) else {
        return ffi::SQLITE_IOERR_SHMMAP;
    };
    let Some(map_offset) = map_index.checked_mul(map_length) else {
        return ffi::SQLITE_IOERR_SHMMAP;
    };
    let Some(required) = map_offset
        .checked_add(map_length)
        .and_then(|required| i64::try_from(required).ok())
    else {
        return ffi::SQLITE_IOERR_SHMSIZE;
    };
    let mut stat = mem::zeroed::<libc::stat>();
    if libc::fstat(file.shm_fd, &mut stat) != 0 {
        return ffi::SQLITE_IOERR_SHMSIZE;
    }
    if stat.st_size < required {
        if extend == 0 {
            return ffi::SQLITE_OK;
        }
        if libc::ftruncate(file.shm_fd, required) != 0 {
            return ffi::SQLITE_IOERR_SHMSIZE;
        }
    }
    if file.shm_maps.len() <= map_index {
        file.shm_maps.resize_with(map_index + 1, || None);
    }
    if file.shm_maps[map_index].is_none() {
        let pointer = libc::mmap(
            ptr::null_mut(),
            map_length,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            file.shm_fd,
            map_offset as libc::off_t,
        );
        if pointer == libc::MAP_FAILED {
            return ffi::SQLITE_IOERR_SHMMAP;
        }
        file.shm_maps[map_index] = Some(ShmMap {
            pointer,
            length: map_length,
        });
    }
    *output = file.shm_maps[map_index]
        .as_ref()
        .map_or(ptr::null_mut(), |map| {
            map.pointer
                .cast::<u8>()
                .add(region_index * page_size)
                .cast()
        });
    ffi::SQLITE_OK
}

unsafe extern "C" fn file_shm_lock(
    file: *mut ffi::sqlite3_file,
    offset: c_int,
    count: c_int,
    flags: c_int,
) -> c_int {
    let file = capability_file(file);
    if file.shm_fd < 0 || offset < 0 || count <= 0 || offset + count > ffi::SQLITE_SHM_NLOCK {
        return ffi::SQLITE_IOERR_SHMLOCK;
    }
    let lock_type = if flags & ffi::SQLITE_SHM_UNLOCK != 0 {
        libc::F_UNLCK
    } else if flags & ffi::SQLITE_SHM_EXCLUSIVE != 0 {
        libc::F_WRLCK
    } else {
        libc::F_RDLCK
    } as i16;
    match set_lock(
        file.shm_fd,
        lock_type,
        SHM_LOCK_BASE + libc::off_t::from(offset),
        libc::off_t::from(count),
    ) {
        Ok(()) => ffi::SQLITE_OK,
        Err(()) if last_errno() == libc::EACCES || last_errno() == libc::EAGAIN => ffi::SQLITE_BUSY,
        Err(()) => ffi::SQLITE_IOERR_SHMLOCK,
    }
}

unsafe extern "C" fn file_shm_barrier(_file: *mut ffi::sqlite3_file) {
    fence(Ordering::SeqCst);
}

unsafe extern "C" fn file_shm_unmap(file: *mut ffi::sqlite3_file, delete: c_int) -> c_int {
    let file = capability_file(file);
    for map in file.shm_maps.drain(..).flatten() {
        libc::munmap(map.pointer, map.length);
    }
    let mut result = ffi::SQLITE_OK;
    if file.shm_fd >= 0 {
        if delete != 0 {
            if set_lock(file.shm_fd, libc::F_UNLCK as i16, SHM_DMS_BYTE, 1).is_err() {
                result = ffi::SQLITE_IOERR_SHMOPEN;
            } else {
                match set_lock(file.shm_fd, libc::F_WRLCK as i16, SHM_DMS_BYTE, 1) {
                    Ok(()) => {
                        if let (Some(state), Some(name)) =
                            (file.state.as_ref(), file.shm_name.as_ref())
                        {
                            if let Err(error) = state.directory.unlink_leaf(name) {
                                if error.kind() != std::io::ErrorKind::NotFound {
                                    result = ffi::SQLITE_IOERR_SHMOPEN;
                                }
                            }
                        }
                    }
                    Err(()) if matches!(last_errno(), libc::EACCES | libc::EAGAIN) => {}
                    Err(()) => result = ffi::SQLITE_IOERR_SHMOPEN,
                }
            }
        }
        if libc::close(file.shm_fd) != 0 && result == ffi::SQLITE_OK {
            result = ffi::SQLITE_IOERR_CLOSE;
        }
        file.shm_fd = -1;
        file.shm_hold = None;
    }
    file.shm_name = None;
    result
}

unsafe extern "C" fn file_fetch(
    _file: *mut ffi::sqlite3_file,
    _offset: ffi::sqlite3_int64,
    _amount: c_int,
    output: *mut *mut c_void,
) -> c_int {
    if !output.is_null() {
        *output = ptr::null_mut();
    }
    ffi::SQLITE_OK
}

unsafe extern "C" fn file_unfetch(
    _file: *mut ffi::sqlite3_file,
    _offset: ffi::sqlite3_int64,
    _pointer: *mut c_void,
) -> c_int {
    ffi::SQLITE_OK
}

fn shm_regions_per_map(region_size: usize) -> usize {
    let host_page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if host_page_size <= 0 {
        return 1;
    }
    shm_regions_per_map_for_page_size(region_size, host_page_size as usize)
}

fn shm_regions_per_map_for_page_size(region_size: usize, host_page_size: usize) -> usize {
    let mut left = region_size;
    let mut right = host_page_size;
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    host_page_size / left.max(1)
}

fn ensure_shm_open(file: &mut CapabilityFile) -> Result<(), ShmOpenError> {
    if file.shm_fd >= 0 {
        return Ok(());
    }
    let Some(state) = (unsafe { file.state.as_ref() }) else {
        return Err(ShmOpenError::Io);
    };
    let name = CString::new(format!("{}-shm", file.name.to_string_lossy()))
        .map_err(|_| ShmOpenError::Io)?;
    let flags = OFlags::RDWR | OFlags::CREATE | OFlags::CLOEXEC | OFlags::NOFOLLOW;
    let opening = crate::fs_notify::Opening::begin(state.directory.as_fd(), &name);
    let fd = state
        .directory
        .open_leaf(&name, flags, FILE_MODE)
        .map_err(|_| ShmOpenError::Io)?;
    file.shm_hold = crate::fs_notify::Hold::open(opening, fd.as_fd(), false);
    file.shm_fd = fd.into_raw_fd();
    if let Err(error) = initialize_shm_lock(file.shm_fd) {
        unsafe { libc::close(file.shm_fd) };
        file.shm_fd = -1;
        file.shm_hold = None;
        return Err(error);
    }
    file.shm_name = Some(name);
    Ok(())
}

fn initialize_shm_lock(fd: RawFd) -> Result<(), ShmOpenError> {
    match conflicting_lock_type(fd, SHM_DMS_BYTE, 1).map_err(|_| ShmOpenError::Io)? {
        lock_type if lock_type == libc::F_UNLCK as i16 => {
            set_lock(fd, libc::F_WRLCK as i16, SHM_DMS_BYTE, 1)
                .map_err(|_| current_shm_lock_error())?;
            if unsafe { libc::ftruncate(fd, 3) } != 0 {
                return Err(ShmOpenError::Io);
            }
            set_lock(fd, libc::F_RDLCK as i16, SHM_DMS_BYTE, 1)
                .map_err(|_| current_shm_lock_error())
        }
        lock_type if lock_type == libc::F_RDLCK as i16 => {
            set_lock(fd, libc::F_RDLCK as i16, SHM_DMS_BYTE, 1)
                .map_err(|_| current_shm_lock_error())
        }
        lock_type if lock_type == libc::F_WRLCK as i16 => Err(ShmOpenError::Busy),
        _ => Err(ShmOpenError::Io),
    }
}

fn current_shm_lock_error() -> ShmOpenError {
    if matches!(last_errno(), libc::EACCES | libc::EAGAIN) {
        ShmOpenError::Busy
    } else {
        ShmOpenError::Io
    }
}

fn set_lock(fd: RawFd, lock_type: i16, start: libc::off_t, length: libc::off_t) -> Result<(), ()> {
    let mut lock = libc::flock {
        l_start: start,
        l_len: length,
        l_pid: 0,
        l_type: lock_type,
        l_whence: libc::SEEK_SET as i16,
    };
    loop {
        let result = unsafe { libc::fcntl(fd, libc::F_OFD_SETLK, &mut lock) };
        if result == 0 {
            return Ok(());
        }
        if last_errno() != libc::EINTR {
            return Err(());
        }
    }
}

fn lock_is_held(fd: RawFd, start: libc::off_t, length: libc::off_t) -> Result<bool, ()> {
    Ok(conflicting_lock_type(fd, start, length)? != libc::F_UNLCK as i16)
}

fn conflicting_lock_type(fd: RawFd, start: libc::off_t, length: libc::off_t) -> Result<i16, ()> {
    let mut lock = libc::flock {
        l_start: start,
        l_len: length,
        l_pid: 0,
        l_type: libc::F_WRLCK as i16,
        l_whence: libc::SEEK_SET as i16,
    };
    if unsafe { libc::fcntl(fd, libc::F_OFD_GETLK, &mut lock) } != 0 {
        return Err(());
    }
    Ok(lock.l_type)
}

fn sqlite_lock_failure(io_error: c_int, error: c_int) -> c_int {
    if matches!(error, libc::EACCES | libc::EAGAIN) {
        ffi::SQLITE_BUSY
    } else {
        io_error
    }
}

unsafe fn capability_file<'a>(file: *mut ffi::sqlite3_file) -> &'a mut CapabilityFile {
    &mut *file.cast::<CapabilityFile>()
}

unsafe fn vfs_state<'a>(vfs: *mut ffi::sqlite3_vfs) -> Option<&'a VfsState> {
    vfs.as_ref()?.pAppData.cast::<VfsState>().as_ref()
}

unsafe fn requested_leaf(
    state: &VfsState,
    name: *const c_char,
    temporary: bool,
) -> Option<CString> {
    if temporary {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        return CString::new(format!(
            ".{}.sqlite-tmp-{}-{sequence}",
            state.database_leaf.to_string_lossy(),
            std::process::id()
        ))
        .ok();
    }
    if name.is_null() {
        return None;
    }
    let bytes = CStr::from_ptr(name).to_bytes();
    let leaf = bytes
        .strip_prefix(state.virtual_prefix.as_slice())
        .unwrap_or(bytes);
    if leaf.contains(&b'/') || !leaf.starts_with(state.database_leaf.as_bytes()) {
        return None;
    }
    let suffix = &leaf[state.database_leaf.as_bytes().len()..];
    if !suffix.is_empty()
        && (suffix.first() != Some(&b'-')
            || !suffix
                .iter()
                .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'-'))
    {
        return None;
    }
    CString::new(leaf).ok()
}

fn last_errno() -> c_int {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

fn sync_file_descriptor(fd: RawFd, flags: c_int) -> Result<(), ()> {
    #[cfg(target_os = "macos")]
    if flags & ffi::SQLITE_SYNC_FULL != 0 {
        if unsafe { libc::fcntl(fd, libc::F_FULLFSYNC) } == 0 {
            return Ok(());
        }
    }
    if unsafe { libc::fsync(fd) } == 0 {
        Ok(())
    } else {
        Err(())
    }
}

unsafe extern "C" fn vfs_dl_open(vfs: *mut ffi::sqlite3_vfs, name: *const c_char) -> *mut c_void {
    let Some(state) = vfs_state(vfs) else {
        return ptr::null_mut();
    };
    (*state.underlying)
        .xDlOpen
        .map_or(ptr::null_mut(), |callback| callback(state.underlying, name))
}

unsafe extern "C" fn vfs_dl_error(vfs: *mut ffi::sqlite3_vfs, length: c_int, output: *mut c_char) {
    if let Some(state) = vfs_state(vfs) {
        if let Some(callback) = (*state.underlying).xDlError {
            callback(state.underlying, length, output);
        }
    }
}

unsafe extern "C" fn vfs_dl_sym(
    vfs: *mut ffi::sqlite3_vfs,
    handle: *mut c_void,
    symbol: *const c_char,
) -> Option<unsafe extern "C" fn(*mut ffi::sqlite3_vfs, *mut c_void, *const c_char)> {
    let state = vfs_state(vfs)?;
    (*state.underlying)
        .xDlSym
        .and_then(|callback| callback(state.underlying, handle, symbol))
}

unsafe extern "C" fn vfs_dl_close(vfs: *mut ffi::sqlite3_vfs, handle: *mut c_void) {
    if let Some(state) = vfs_state(vfs) {
        if let Some(callback) = (*state.underlying).xDlClose {
            callback(state.underlying, handle);
        }
    }
}

unsafe extern "C" fn vfs_randomness(
    vfs: *mut ffi::sqlite3_vfs,
    length: c_int,
    output: *mut c_char,
) -> c_int {
    let Some(state) = vfs_state(vfs) else {
        return 0;
    };
    (*state.underlying)
        .xRandomness
        .map_or(0, |callback| callback(state.underlying, length, output))
}

unsafe extern "C" fn vfs_sleep(vfs: *mut ffi::sqlite3_vfs, micros: c_int) -> c_int {
    let Some(state) = vfs_state(vfs) else {
        return 0;
    };
    (*state.underlying)
        .xSleep
        .map_or(0, |callback| callback(state.underlying, micros))
}

unsafe extern "C" fn vfs_current_time(vfs: *mut ffi::sqlite3_vfs, output: *mut f64) -> c_int {
    let Some(state) = vfs_state(vfs) else {
        return ffi::SQLITE_IOERR;
    };
    (*state.underlying)
        .xCurrentTime
        .map_or(ffi::SQLITE_IOERR, |callback| {
            callback(state.underlying, output)
        })
}

unsafe extern "C" fn vfs_get_last_error(
    vfs: *mut ffi::sqlite3_vfs,
    length: c_int,
    output: *mut c_char,
) -> c_int {
    let Some(state) = vfs_state(vfs) else {
        return 0;
    };
    (*state.underlying)
        .xGetLastError
        .map_or(0, |callback| callback(state.underlying, length, output))
}

unsafe extern "C" fn vfs_current_time_int64(
    vfs: *mut ffi::sqlite3_vfs,
    output: *mut ffi::sqlite3_int64,
) -> c_int {
    let Some(state) = vfs_state(vfs) else {
        return ffi::SQLITE_IOERR;
    };
    (*state.underlying)
        .xCurrentTimeInt64
        .map_or(ffi::SQLITE_IOERR, |callback| {
            callback(state.underlying, output)
        })
}

unsafe extern "C" fn vfs_set_system_call(
    vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
    call: ffi::sqlite3_syscall_ptr,
) -> c_int {
    let Some(state) = vfs_state(vfs) else {
        return ffi::SQLITE_NOTFOUND;
    };
    (*state.underlying)
        .xSetSystemCall
        .map_or(ffi::SQLITE_NOTFOUND, |callback| {
            callback(state.underlying, name, call)
        })
}

unsafe extern "C" fn vfs_get_system_call(
    vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
) -> ffi::sqlite3_syscall_ptr {
    let state = vfs_state(vfs)?;
    (*state.underlying)
        .xGetSystemCall
        .and_then(|callback| callback(state.underlying, name))
}

unsafe extern "C" fn vfs_next_system_call(
    vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
) -> *const c_char {
    let Some(state) = vfs_state(vfs) else {
        return ptr::null();
    };
    (*state.underlying)
        .xNextSystemCall
        .map_or(ptr::null(), |callback| callback(state.underlying, name))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::fs::OpenOptions;
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::symlink;
    use std::path::PathBuf;
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    use super::*;

    #[test]
    fn capability_vfs_open_in_a_watched_directory_is_a_missing_primitive() {
        let fixture = Fixture::new("watched");
        let directory = Directory::open_ambient(&fixture.database_directory).unwrap();
        let instance = crate::fs_notify::Instance::new();
        instance
            .add_watch(
                crate::fs_notify::InodeKey::of(directory.as_fd()).unwrap(),
                crate::fs_notify::IN_CREATE,
            )
            .unwrap();

        assert!(
            CapabilityConnection::open(directory, CString::new("main.db").unwrap()).is_err(),
            "the winedroid VFS opens host artifacts real sqlite never opens, so a watched directory is unsupported"
        );
        assert!(instance.take_events().is_empty());
        assert!(!fixture.database_directory.join("main.db").exists());
    }

    #[test]
    fn capability_vfs_close_of_a_file_opened_before_the_watch_is_a_missing_primitive() {
        let fixture = Fixture::new("watched_close");
        let directory = Directory::open_ambient(&fixture.database_directory).unwrap();
        let key = crate::fs_notify::InodeKey::of(directory.as_fd()).unwrap();
        let connection =
            CapabilityConnection::open(directory, CString::new("main.db").unwrap()).unwrap();
        connection
            .connection()
            .execute_batch("CREATE TABLE values_table(value INTEGER);")
            .unwrap();
        let instance = crate::fs_notify::Instance::new();
        instance
            .add_watch(key, crate::fs_notify::IN_CLOSE_WRITE)
            .unwrap();
        drop(connection);

        assert!(
            crate::missing::recorded_entries()
                .iter()
                .any(|entry| entry.primitive == "inotify events for SQLite VFS xClose"),
            "inotify(7) IN_CLOSE_WRITE follows the close of the writable database descriptor, so a silent close is a missed event"
        );
        assert!(instance.take_events().is_empty());
    }

    #[test]
    fn capability_vfs_open_of_a_watched_database_file_is_a_missing_primitive() {
        let fixture = Fixture::new("watched_file");
        fs::write(fixture.database_directory.join("main.db"), b"").unwrap();
        let directory = Directory::open_ambient(&fixture.database_directory).unwrap();
        let instance = crate::fs_notify::Instance::new();
        instance
            .add_watch(
                crate::fs_notify::InodeKey::at(directory.as_fd(), c"main.db").unwrap(),
                crate::fs_notify::IN_DELETE_SELF,
            )
            .unwrap();

        assert!(
            CapabilityConnection::open(directory, CString::new("main.db").unwrap()).is_err(),
            "the winedroid VFS deletes and replaces files real sqlite would not, so a watched database file is unsupported"
        );
        assert!(instance.take_events().is_empty());
    }

    #[test]
    fn capability_vfs_descriptor_keeps_a_deleted_watched_database_until_the_connection_closes() {
        let fixture = Fixture::new("watched_delete");
        let directory = Directory::open_ambient(&fixture.database_directory).unwrap();
        let connection =
            CapabilityConnection::open(directory.clone(), CString::new("main.db").unwrap())
                .unwrap();
        connection
            .connection()
            .execute_batch("CREATE TABLE values_table(value INTEGER);")
            .unwrap();
        let instance = crate::fs_notify::Instance::new();
        let wd = instance
            .add_watch(
                crate::fs_notify::InodeKey::at(directory.as_fd(), c"main.db").unwrap(),
                crate::fs_notify::IN_DELETE_SELF,
            )
            .unwrap();
        directory
            .remove(&[CString::new("main.db").unwrap()])
            .unwrap();
        assert!(
            instance.take_events().is_empty(),
            "unlink(2): the file lives while SQLite holds its descriptor"
        );
        drop(connection);
        assert_eq!(
            instance.take_events(),
            vec![
                crate::fs_notify::Event {
                    wd,
                    mask: crate::fs_notify::IN_DELETE_SELF,
                    name: None
                },
                crate::fs_notify::Event {
                    wd,
                    mask: crate::fs_notify::IN_IGNORED,
                    name: None
                },
            ],
            "inotify.h IN_DELETE_SELF then IN_IGNORED at the last close"
        );
    }

    #[test]
    fn capability_vfs_persists_rollback_journal_transactions() {
        let fixture = Fixture::new("rollback");
        let directory = Directory::open_ambient(&fixture.database_directory).unwrap();
        let connection =
            CapabilityConnection::open(directory, CString::new("main.db").unwrap()).unwrap();

        connection
            .connection()
            .execute_batch(
                "CREATE TABLE values_table(value INTEGER); BEGIN; INSERT INTO values_table VALUES (7); COMMIT;",
            )
            .unwrap();
        assert_eq!(
            connection
                .connection()
                .query_row("SELECT value FROM values_table", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            7
        );
        drop(connection);
        let directory = Directory::open_ambient(&fixture.database_directory).unwrap();
        let reopened =
            CapabilityConnection::open(directory, CString::new("main.db").unwrap()).unwrap();
        assert_eq!(
            reopened
                .connection()
                .query_row("SELECT value FROM values_table", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            7
        );
    }

    #[test]
    fn capability_vfs_supports_wal_and_shared_memory() {
        let fixture = Fixture::new("wal");
        let directory = Directory::open_ambient(&fixture.database_directory).unwrap();
        let connection =
            CapabilityConnection::open(directory, CString::new("main.db").unwrap()).unwrap();

        assert_eq!(
            connection
                .connection()
                .query_row("PRAGMA journal_mode=WAL", [], |row| row.get::<_, String>(0))
                .unwrap()
                .to_ascii_lowercase(),
            "wal"
        );
        connection
            .connection()
            .execute_batch("CREATE TABLE values_table(value); INSERT INTO values_table VALUES (9)")
            .unwrap();
        connection
            .connection()
            .execute_batch("PRAGMA wal_checkpoint(FULL)")
            .unwrap();
        assert_eq!(
            connection
                .connection()
                .query_row("SELECT value FROM values_table", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            9
        );
    }

    #[test]
    fn capability_vfs_local_wal_connections_share_snapshot_and_locks() {
        let fixture = Fixture::new("wal_local_connections");
        let first = CapabilityConnection::open(
            Directory::open_ambient(&fixture.database_directory).unwrap(),
            CString::new("main.db").unwrap(),
        )
        .unwrap();
        first
            .connection()
            .execute_batch(
                "PRAGMA journal_mode=WAL; CREATE TABLE values_table(value); INSERT INTO values_table VALUES (0);",
            )
            .unwrap();
        let second = CapabilityConnection::open(
            Directory::open_ambient(&fixture.database_directory).unwrap(),
            CString::new("main.db").unwrap(),
        )
        .unwrap();
        second.connection().busy_timeout(Duration::ZERO).unwrap();
        assert_eq!(
            second
                .connection()
                .query_row("PRAGMA journal_mode", [], |row| row.get::<_, String>(0))
                .unwrap()
                .to_ascii_lowercase(),
            "wal"
        );

        first
            .connection()
            .execute_batch("BEGIN IMMEDIATE; UPDATE values_table SET value = 1;")
            .unwrap();
        assert_eq!(
            second
                .connection()
                .query_row("SELECT value FROM values_table", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        let error = second
            .connection()
            .execute_batch("BEGIN IMMEDIATE;")
            .unwrap_err();
        assert!(matches!(
            error.sqlite_error_code(),
            Some(rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked)
        ));
        first.connection().execute_batch("COMMIT;").unwrap();
        assert_eq!(
            second
                .connection()
                .query_row("SELECT value FROM values_table", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }

    #[test]
    fn capability_vfs_stays_in_held_directory_after_path_replacement() {
        let fixture = Fixture::new("parent_swap");
        let outside = fixture.root.join("outside");
        fs::create_dir(&outside).unwrap();
        let directory = Directory::open_ambient(&fixture.database_directory).unwrap();
        let moved = fixture.root.join("moved");

        fs::rename(&fixture.database_directory, &moved).unwrap();
        symlink(&outside, &fixture.database_directory).unwrap();
        let connection =
            CapabilityConnection::open(directory, CString::new("main.db").unwrap()).unwrap();
        connection
            .connection()
            .execute_batch("CREATE TABLE values_table(value); INSERT INTO values_table VALUES (1)")
            .unwrap();

        assert!(moved.join("main.db").is_file());
        assert!(!outside.join("main.db").exists());
        assert!(!outside.join("main.db-journal").exists());
        assert!(!outside.join("main.db-wal").exists());
        assert!(!outside.join("main.db-shm").exists());
    }

    #[test]
    fn capability_vfs_rejects_a_final_database_symlink() {
        let fixture = Fixture::new("leaf_symlink");
        let outside = fixture.root.join("outside.db");
        fs::write(&outside, b"outside").unwrap();
        symlink(&outside, fixture.database_directory.join("main.db")).unwrap();
        let directory = Directory::open_ambient(&fixture.database_directory).unwrap();

        assert!(CapabilityConnection::open(directory, CString::new("main.db").unwrap()).is_err());
        assert_eq!(fs::read(outside).unwrap(), b"outside");
    }

    #[test]
    fn capability_vfs_rejects_writes_after_database_leaf_replacement() {
        let fixture = Fixture::new("moved_database");
        let directory = Directory::open_ambient(&fixture.database_directory).unwrap();
        let connection =
            CapabilityConnection::open(directory, CString::new("main.db").unwrap()).unwrap();
        connection
            .connection()
            .execute_batch("CREATE TABLE values_table(value)")
            .unwrap();
        fs::rename(
            fixture.database_directory.join("main.db"),
            fixture.database_directory.join("moved.db"),
        )
        .unwrap();

        assert!(connection
            .connection()
            .execute("INSERT INTO values_table VALUES (1)", [])
            .is_err());
    }

    #[test]
    fn capability_vfs_local_connections_enforce_independent_locks() {
        let fixture = Fixture::new("second_connection");
        let first = CapabilityConnection::open(
            Directory::open_ambient(&fixture.database_directory).unwrap(),
            CString::new("main.db").unwrap(),
        )
        .unwrap();
        first
            .connection()
            .execute_batch("CREATE TABLE values_table(value); BEGIN IMMEDIATE;")
            .unwrap();
        let second = CapabilityConnection::open(
            Directory::open_ambient(&fixture.database_directory).unwrap(),
            CString::new("main.db").unwrap(),
        )
        .unwrap();
        second.connection().busy_timeout(Duration::ZERO).unwrap();

        let error = second
            .connection()
            .execute_batch("BEGIN IMMEDIATE;")
            .unwrap_err();
        assert!(matches!(
            error.sqlite_error_code(),
            Some(rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked)
        ));
        drop(second);
        first.connection().execute_batch("COMMIT;").unwrap();
    }

    #[test]
    fn capability_vfs_ofd_locks_conflict_with_external_posix_locks() {
        if let Some(database_path) = std::env::var_os("WINEDROID_SQLITE_POSIX_LOCK_CHILD") {
            let database = OpenOptions::new()
                .read(true)
                .write(true)
                .open(database_path)
                .unwrap();
            let mut lock = libc::flock {
                l_start: RESERVED_BYTE,
                l_len: 1,
                l_pid: 0,
                l_type: libc::F_WRLCK as i16,
                l_whence: libc::SEEK_SET as i16,
            };
            assert_eq!(
                unsafe { libc::fcntl(database.as_raw_fd(), libc::F_SETLK, &mut lock) },
                -1
            );
            assert!(matches!(last_errno(), libc::EACCES | libc::EAGAIN));
            return;
        }

        let fixture = Fixture::new("ofd_posix_conflict");
        let first = CapabilityConnection::open(
            Directory::open_ambient(&fixture.database_directory).unwrap(),
            CString::new("main.db").unwrap(),
        )
        .unwrap();
        first
            .connection()
            .execute_batch("CREATE TABLE values_table(value); BEGIN IMMEDIATE;")
            .unwrap();
        let unrelated = CapabilityConnection::open(
            Directory::open_ambient(&fixture.database_directory).unwrap(),
            CString::new("main.db").unwrap(),
        )
        .unwrap();
        drop(unrelated);

        let status = Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg(
                "capability_sqlite::tests::capability_vfs_ofd_locks_conflict_with_external_posix_locks",
            )
            .arg("--nocapture")
            .env(
                "WINEDROID_SQLITE_POSIX_LOCK_CHILD",
                fixture.database_directory.join("main.db"),
            )
            .status()
            .unwrap();

        assert!(status.success());
        first.connection().execute_batch("COMMIT;").unwrap();
    }

    #[test]
    fn capability_vfs_opens_a_renamed_inode_through_a_second_connection() {
        let fixture = Fixture::new("second_inode_name");
        let first = CapabilityConnection::open(
            Directory::open_ambient(&fixture.database_directory).unwrap(),
            CString::new("main.db").unwrap(),
        )
        .unwrap();
        first
            .connection()
            .execute_batch("CREATE TABLE values_table(value)")
            .unwrap();
        fs::rename(
            fixture.database_directory.join("main.db"),
            fixture.database_directory.join("moved.db"),
        )
        .unwrap();

        let second = CapabilityConnection::open(
            Directory::open_ambient(&fixture.database_directory).unwrap(),
            CString::new("moved.db").unwrap(),
        )
        .unwrap();
        assert_eq!(
            second
                .connection()
                .query_row("SELECT COUNT(*) FROM values_table", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert!(first
            .connection()
            .execute("INSERT INTO values_table VALUES (1)", [])
            .is_err());
    }

    #[test]
    fn capability_vfs_reports_busy_during_cross_process_shm_initialization() {
        if let Some(shm_path) = std::env::var_os("WINEDROID_SQLITE_DMS_CHILD_SHM") {
            let ready_path =
                PathBuf::from(std::env::var_os("WINEDROID_SQLITE_DMS_CHILD_READY").unwrap());
            let release_path =
                PathBuf::from(std::env::var_os("WINEDROID_SQLITE_DMS_CHILD_RELEASE").unwrap());
            let shm = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(shm_path)
                .unwrap();
            set_lock(shm.as_raw_fd(), libc::F_WRLCK as i16, SHM_DMS_BYTE, 1).unwrap();
            fs::write(&ready_path, b"ready").unwrap();
            for _ in 0..1_000 {
                if release_path.exists() {
                    return;
                }
                thread::sleep(Duration::from_millis(10));
            }
            panic!("parent did not release cross-process DMS test child");
        }

        let fixture = Fixture::new("cross_process_dms");
        let shm_path = fixture.database_directory.join("main.db-shm");
        let ready_path = fixture.root.join("child-ready");
        let release_path = fixture.root.join("child-release");
        let mut child = Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg(
                "capability_sqlite::tests::capability_vfs_reports_busy_during_cross_process_shm_initialization",
            )
            .arg("--nocapture")
            .env("WINEDROID_SQLITE_DMS_CHILD_SHM", &shm_path)
            .env("WINEDROID_SQLITE_DMS_CHILD_READY", &ready_path)
            .env("WINEDROID_SQLITE_DMS_CHILD_RELEASE", &release_path)
            .spawn()
            .unwrap();
        for _ in 0..1_000 {
            if ready_path.exists() {
                break;
            }
            if let Some(status) = child.try_wait().unwrap() {
                panic!("cross-process DMS test child exited early with {status}");
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert!(ready_path.exists());

        let state = Box::new(VfsState {
            directory: Directory::open_ambient(&fixture.database_directory).unwrap(),
            database_leaf: CString::new("main.db").unwrap(),
            virtual_prefix: Vec::new(),
            underlying: ptr::null_mut(),
        });
        let mut file = test_capability_file(&state, "main.db");
        let reports_busy = matches!(ensure_shm_open(&mut file), Err(ShmOpenError::Busy));
        fs::write(&release_path, b"release").unwrap();
        let status = child.wait().unwrap();

        assert!(status.success());
        assert!(reports_busy);
    }

    #[test]
    fn sqlite_unix_vfs_first_dms_holder_truncates_stale_shm() {
        let fixture = Fixture::new("stale_shm");
        let shm_path = fixture.database_directory.join("main.db-shm");
        fs::write(&shm_path, vec![0xa5; 4096]).unwrap();
        let state = Box::new(VfsState {
            directory: Directory::open_ambient(&fixture.database_directory).unwrap(),
            database_leaf: CString::new("main.db").unwrap(),
            virtual_prefix: Vec::new(),
            underlying: ptr::null_mut(),
        });
        let mut file = test_capability_file(&state, "main.db");

        assert!(ensure_shm_open(&mut file).is_ok());

        assert_eq!(fs::metadata(shm_path).unwrap().len(), 3);
        assert_eq!(
            unsafe {
                file_shm_unmap(
                    (&mut file as *mut CapabilityFile).cast::<ffi::sqlite3_file>(),
                    0,
                )
            },
            ffi::SQLITE_OK
        );
    }

    #[test]
    fn capability_vfs_groups_wal_index_regions_for_large_host_pages() {
        assert_eq!(shm_regions_per_map_for_page_size(32_768, 4_096), 1);
        assert_eq!(shm_regions_per_map_for_page_size(32_768, 16_384), 1);
        assert_eq!(shm_regions_per_map_for_page_size(32_768, 65_536), 2);
    }

    fn test_capability_file(state: &VfsState, name: &str) -> CapabilityFile {
        CapabilityFile {
            base: ffi::sqlite3_file {
                pMethods: ptr::null(),
            },
            state,
            fd: -1,
            writable: false,
            name: CString::new(name).unwrap(),
            device: 0,
            inode: 0,
            sync_parent: false,
            lock_level: ffi::SQLITE_LOCK_NONE,
            chunk_size: 0,
            shm_fd: -1,
            shm_name: None,
            shm_maps: Vec::new(),
            _hold: None,
            shm_hold: None,
        }
    }

    struct Fixture {
        root: PathBuf,
        database_directory: PathBuf,
    }

    impl Fixture {
        fn new(name: &str) -> Self {
            let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "winedroid-sqlite-vfs-{name}-{}-{sequence}",
                std::process::id()
            ));
            let database_directory = root.join("databases");
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&database_directory).unwrap();
            Self {
                root,
                database_directory,
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
