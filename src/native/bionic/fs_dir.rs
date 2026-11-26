use std::sync::{Arc, Mutex};

use super::super::call_assign::CallBuilder;
use super::abi_types::{EBADF, EINVAL};
use super::export::in_libc;
use super::fatal::fortify_fatal;
use super::fd_table::{self, DirItem, FdEntry, FdKind};
use super::fdsan::{self, OWNER_DIR};
use super::fs_open::{open_at, path_argument};
use super::fs_result::{finish_int, from_host};
use super::guest_mem::{write_bytes, write_value};
use super::linux_abi::{at, open, stat as layout};

const ENOTDIR: i32 = 20;
const ENOMEM: i32 = 12;
const SCAN_INITIAL: usize = 32;

#[repr(align(8))]
struct DirentBuffer([u8; layout::DIRENT_SIZE]);

struct DirStream {
    fd: i32,
    entry: Arc<FdEntry>,
    buffer: Mutex<DirentBuffer>,
}

fn stream<'a>(pointer: usize, function: &str) -> &'a DirStream {
    if pointer == 0 {
        fortify_fatal(&format!("{function}: null DIR*"));
    }
    unsafe { &*(pointer as *const DirStream) }
}

fn tag_of(pointer: usize) -> u64 {
    // cite: bionic dirent.cpp#__get_dir_tag
    fdsan::create_owner_tag(OWNER_DIR, pointer as u64)
}

fn allocate(fd: i32) -> Result<usize, i32> {
    let entry = fd_table::get(fd).ok_or(EBADF)?;
    let pointer = Box::into_raw(Box::new(DirStream {
        fd,
        entry,
        buffer: Mutex::new(DirentBuffer([0u8; layout::DIRENT_SIZE])),
    })) as usize;
    fdsan::exchange_owner_tag(fd, 0, tag_of(pointer));
    Ok(pointer)
}

fn host_items(fd: i32) -> Result<Vec<DirItem>, i32> {
    let duplicate = unsafe { libc::dup(fd) };
    if duplicate < 0 {
        return Err((-from_host()) as i32);
    }
    let directory = unsafe { libc::fdopendir(duplicate) };
    if directory.is_null() {
        unsafe { libc::close(duplicate) };
        return Err((-from_host()) as i32);
    }
    unsafe { libc::rewinddir(directory) };
    let mut items = Vec::new();
    loop {
        let record = unsafe { libc::readdir(directory) };
        if record.is_null() {
            break;
        }
        let record = unsafe { &*record };
        let length = record.d_namlen as usize;
        let name =
            unsafe { std::slice::from_raw_parts(record.d_name.as_ptr() as *const u8, length) }
                .to_vec();
        items.push(DirItem {
            inode: record.d_ino,
            name,
            kind: record.d_type,
        });
    }
    unsafe { libc::closedir(directory) };
    Ok(items)
}

fn next_record(directory: &DirStream) -> Result<Option<usize>, i32> {
    let mut state = directory.entry.dir.lock().expect("dir state");
    if state.items.is_none() {
        state.items = Some(host_items(directory.fd)?);
    }
    let position = state.position;
    let Some(item) = state.items.as_ref().and_then(|items| items.get(position)) else {
        return Ok(None);
    };
    let mut buffer = directory.buffer.lock().expect("dirent buffer");
    buffer.0.fill(0);
    let name = &item.name[..item.name.len().min(super::linux_abi::misc::NAME_MAX)];
    // cite: bionic dirent.h#__DIRENT64_BODY
    buffer.0[0..8].copy_from_slice(&item.inode.to_le_bytes());
    buffer.0[8..16].copy_from_slice(&((position + 1) as i64).to_le_bytes());
    let reclen = (layout::DIRENT_NAME + name.len() + 1).div_ceil(8) * 8;
    buffer.0[layout::DIRENT_RECLEN..layout::DIRENT_RECLEN + 2]
        .copy_from_slice(&(reclen as u16).to_le_bytes());
    buffer.0[layout::DIRENT_TYPE] = item.kind;
    buffer.0[layout::DIRENT_NAME..layout::DIRENT_NAME + name.len()].copy_from_slice(name);
    state.position += 1;
    Ok(Some(buffer.0.as_ptr() as usize))
}

fn open_directory(path: &[u8]) -> Result<usize, i32> {
    let fd = open_at(
        at::FDCWD,
        path,
        open::CLOEXEC | open::DIRECTORY | open::RDONLY,
        0,
    );
    if fd < 0 {
        return Err((-fd) as i32);
    }
    allocate(fd as i32).inspect_err(|_| {
        super::fs_close_dup::close_fd(fd as i32);
    })
}

fn directory_of_fd(fd: i32) -> Result<usize, i32> {
    let entry = fd_table::get(fd).ok_or(EBADF)?;
    if entry.kind != FdKind::Dir {
        return Err(ENOTDIR);
    }
    allocate(fd)
}

fn read_into(pointer: usize, target: usize, record: usize) {
    let _ = pointer;
    let reclen = u16::from_le_bytes(
        unsafe { std::slice::from_raw_parts((record + layout::DIRENT_RECLEN) as *const u8, 2) }
            .try_into()
            .expect("2 bytes"),
    );
    write_bytes(target, unsafe {
        std::slice::from_raw_parts(record as *const u8, reclen as usize)
    });
}

fn entry_name(pointer: usize) -> Vec<u8> {
    super::guest_mem::cstr_bytes(pointer + layout::DIRENT_NAME).to_vec()
}

fn scan(path: &[u8], filter: usize, compare: usize) -> Result<(usize, i32), i32> {
    // cite: bionic scandir.cpp#scandir
    let directory = open_directory(path)?;
    let dir = stream(directory, "scandir");
    let mut kept: Vec<usize> = Vec::with_capacity(SCAN_INITIAL);
    loop {
        let record = match next_record(dir) {
            Ok(Some(record)) => record,
            Ok(None) => break,
            Err(errno) => return Err(errno),
        };
        if filter != 0 {
            let verdict = CallBuilder::new(filter)
                .ptr(record)
                .invoke()
                .map(|result| result.i32())
                .unwrap_or(0);
            if verdict == 0 {
                continue;
            }
        }
        let reclen = u16::from_le_bytes(
            unsafe { std::slice::from_raw_parts((record + layout::DIRENT_RECLEN) as *const u8, 2) }
                .try_into()
                .expect("2 bytes"),
        ) as usize;
        let copy = unsafe { libc::malloc(reclen) } as usize;
        if copy == 0 {
            return Err(ENOMEM);
        }
        write_bytes(copy, unsafe {
            std::slice::from_raw_parts(record as *const u8, reclen)
        });
        kept.push(copy);
    }
    close_stream(directory);
    if compare != 0 {
        kept.sort_by(|left, right| {
            let (left_cell, right_cell) = (Box::new(*left), Box::new(*right));
            let outcome = CallBuilder::new(compare)
                .ptr(&*left_cell as *const usize as usize)
                .ptr(&*right_cell as *const usize as usize)
                .invoke()
                .map(|result| result.i32())
                .unwrap_or(0);
            outcome.cmp(&0)
        });
    }
    let list = unsafe { libc::malloc(kept.len().max(1) * 8) } as usize;
    for (index, record) in kept.iter().enumerate() {
        write_value::<usize>(list + index * 8, *record);
    }
    Ok((list, kept.len() as i32))
}

fn close_stream(pointer: usize) -> i64 {
    let directory = unsafe { Box::from_raw(pointer as *mut DirStream) };
    fdsan::close_with_tag(directory.fd, tag_of(pointer))
}

pub fn register() {
    in_libc("opendir", |context| {
        match path_argument(context.arg_ptr(0)).and_then(open_directory) {
            Ok(pointer) => context.ret_ptr(pointer),
            Err(errno) => {
                context.set_errno(errno);
                context.ret_ptr(0);
            }
        }
    });
    in_libc("fdopendir", |context| {
        match directory_of_fd(context.arg_i32(0)) {
            Ok(pointer) => context.ret_ptr(pointer),
            Err(errno) => {
                context.set_errno(errno);
                context.ret_ptr(0);
            }
        }
    });
    for name in ["readdir", "readdir64"] {
        in_libc(name, |context| {
            match next_record(stream(context.arg_ptr(0), "readdir")) {
                Ok(record) => context.ret_ptr(record.unwrap_or(0)),
                Err(errno) => {
                    context.set_errno(errno);
                    context.ret_ptr(0);
                }
            }
        });
    }
    for name in ["readdir_r", "readdir64_r"] {
        in_libc(name, |context| {
            let (result_slot, target) = (context.arg_ptr(2), context.arg_ptr(1));
            write_value::<usize>(result_slot, 0);
            match next_record(stream(context.arg_ptr(0), "readdir_r")) {
                Ok(Some(record)) => {
                    read_into(context.arg_ptr(0), target, record);
                    write_value::<usize>(result_slot, target);
                    context.ret_i32(0);
                }
                Ok(None) => context.ret_i32(0),
                Err(errno) => context.ret_i32(errno),
            }
        });
    }
    in_libc("closedir", |context| {
        let pointer = context.arg_ptr(0);
        if pointer == 0 {
            context.set_errno(EINVAL);
            context.ret_i32(-1);
            return;
        }
        let result = close_stream(pointer);
        finish_int(context, result);
    });
    in_libc("rewinddir", |context| {
        let directory = stream(context.arg_ptr(0), "rewinddir");
        let mut state = directory.entry.dir.lock().expect("dir state");
        state.position = 0;
        if directory.entry.synthetic.is_none() {
            state.items = None;
        }
    });
    in_libc("seekdir", |context| {
        let directory = stream(context.arg_ptr(0), "seekdir");
        directory.entry.dir.lock().expect("dir state").position = context.arg_u64(1) as usize;
    });
    in_libc("telldir", |context| {
        let directory = stream(context.arg_ptr(0), "telldir");
        let position = directory.entry.dir.lock().expect("dir state").position;
        context.ret_i64(position as i64);
    });
    in_libc("dirfd", |context| {
        context.ret_i32(stream(context.arg_ptr(0), "dirfd").fd)
    });
    for name in ["alphasort", "alphasort64"] {
        in_libc(name, |context| {
            // cite: bionic dirent.cpp#alphasort
            let left = entry_name(super::guest_mem::read_value::<usize>(context.arg_ptr(0)));
            let right = entry_name(super::guest_mem::read_value::<usize>(context.arg_ptr(1)));
            context.ret_i32(left.cmp(&right) as i32);
        });
    }
    for name in ["scandir", "scandir64"] {
        in_libc(name, |context| {
            let outcome = path_argument(context.arg_ptr(0))
                .and_then(|path| scan(path, context.arg_ptr(2), context.arg_ptr(3)));
            match outcome {
                Ok((list, count)) => {
                    write_value::<usize>(context.arg_ptr(1), list);
                    context.ret_i32(count);
                }
                Err(errno) => {
                    context.set_errno(errno);
                    context.ret_i32(-1);
                }
            }
        });
    }
}
