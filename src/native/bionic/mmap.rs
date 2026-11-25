use super::abi_types::{EBADF, EINVAL, ENOMEM, PAGE_SIZE};
use super::export::in_libc;
use super::fd_table::{self, FdKind};
use super::fs_result::{finish_int, finish_long, from_host, missing};
use super::linux_abi::{mman, open};
use super::map_registry::{self, FileBacking, MapRecord};
use super::stat_convert::linux_device;
use super::translate::prot_to_host;

const ENODEV: i32 = 19;
const EACCES: i32 = 13;
const EEXIST: i32 = 17;

const KNOWN_PROT: i32 = mman::PROT_READ | mman::PROT_WRITE | mman::PROT_EXEC | mman::PROT_SEM | mman::PROT_BTI | mman::PROT_MTE;
const KNOWN_MAP: i32 = mman::MAP_SHARED | mman::MAP_PRIVATE | mman::MAP_FIXED | mman::MAP_ANONYMOUS | mman::MAP_GROWSDOWN | mman::MAP_DENYWRITE | mman::MAP_EXECUTABLE | mman::MAP_LOCKED | mman::MAP_NORESERVE | mman::MAP_POPULATE | mman::MAP_NONBLOCK | mman::MAP_STACK | mman::MAP_HUGETLB | mman::MAP_SYNC | mman::MAP_FIXED_NOREPLACE;

fn error(code: i32) -> i64 {
    -i64::from(code)
}

fn page_aligned(value: i64) -> bool {
    value >= 0 && (value as usize) % PAGE_SIZE == 0
}

fn host_map_flags(linux: i32) -> Option<i32> {
    // cite: bionic mman.h#MAP_PRIVATE, MAP_SHARED
    // cite: bionic mman-common.h#MAP_FIXED
    if linux & !KNOWN_MAP & !mman::MAP_TYPE != 0 {
        return None;
    }
    if linux & mman::MAP_HUGETLB != 0 || linux & mman::MAP_SYNC != 0 {
        return None;
    }
    let mut flags = match linux & mman::MAP_TYPE {
        mman::MAP_SHARED | mman::MAP_SHARED_VALIDATE => libc::MAP_SHARED,
        mman::MAP_PRIVATE => libc::MAP_PRIVATE,
        _ => return None,
    };
    if linux & mman::MAP_FIXED != 0 {
        flags |= libc::MAP_FIXED;
    }
    if linux & mman::MAP_ANONYMOUS != 0 {
        flags |= libc::MAP_ANON;
    }
    if linux & mman::MAP_NORESERVE != 0 {
        // cite: bionic mman.h#MAP_NORESERVE
        flags |= 0x40;
    }
    Some(flags)
}

fn backing_of(fd: i32) -> Result<(i32, FileBacking, bool), i32> {
    let entry = fd_table::get(fd).ok_or(EBADF)?;
    let writable = entry.flags() & open::ACCMODE != open::RDONLY;
    if matches!(entry.kind, FdKind::Dir | FdKind::PathOnly) {
        return Err(ENODEV);
    }
    let mut stat: libc::stat = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstat(fd, &mut stat) } < 0 {
        return Err((-from_host()) as i32);
    }
    let device = linux_device(stat.st_dev);
    Ok((fd, FileBacking { guest_path: entry.guest_path.clone(), offset: 0, device, inode: stat.st_ino }, writable))
}

pub fn map(address: usize, length: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> i64 {
    // cite: bionic mmap.cpp#mmap
    if length == 0 || length > isize::MAX as usize || !page_aligned(offset) {
        return error(EINVAL);
    }
    if prot & !KNOWN_PROT != 0 {
        return error(EINVAL);
    }
    let Some(host_flags) = host_map_flags(flags) else {
        missing("mmap", &format!("flag set {flags:#x} has no host equivalent"));
        return error(EINVAL);
    };
    let anonymous = flags & mman::MAP_ANONYMOUS != 0;
    let mut backing = None;
    let (host_fd, host_offset, writable) = if anonymous {
        (-1, 0, true)
    } else {
        match backing_of(fd) {
            Ok((host_fd, file, writable)) => {
                backing = Some(file);
                (host_fd, offset as libc::off_t, writable)
            }
            Err(errno) => return error(errno),
        }
    };
    if !anonymous && prot & mman::PROT_WRITE != 0 && flags & mman::MAP_SHARED != 0 && !writable {
        return error(EACCES);
    }
    if flags & mman::MAP_FIXED_NOREPLACE != 0 && !map_registry::covering(address, length).is_empty() {
        return error(EEXIST);
    }
    let executable = prot & mman::PROT_EXEC != 0;
    let host_prot = if executable { (prot_to_host(prot) & !libc::PROT_EXEC) | libc::PROT_READ | libc::PROT_WRITE } else { prot_to_host(prot) };
    let mapped = unsafe { libc::mmap(address as *mut libc::c_void, length, host_prot, host_flags, host_fd, host_offset) };
    if mapped == libc::MAP_FAILED {
        return from_host();
    }
    if executable && unsafe { libc::mprotect(mapped, length, prot_to_host(prot)) } < 0 {
        let failure = from_host();
        unsafe { libc::munmap(mapped, length) };
        return failure;
    }
    if let Some(file) = backing.as_mut() {
        file.offset = offset as u64;
    }
    map_registry::insert(mapped as usize, MapRecord { len: length, prot: prot_to_host(prot), shared: flags & mman::MAP_TYPE != mman::MAP_PRIVATE, no_reserve: flags & mman::MAP_NORESERVE != 0, file: backing, name: None });
    mapped as i64
}

pub fn unmap(address: usize, length: usize) -> i64 {
    // cite: bionic mman.h#munmap
    if length == 0 || address % PAGE_SIZE != 0 || length % PAGE_SIZE != 0 {
        return error(EINVAL);
    }
    if unsafe { libc::munmap(address as *mut libc::c_void, length) } < 0 {
        return from_host();
    }
    map_registry::remove_range(address, length);
    0
}

fn host_prot(prot: i32) -> Option<i32> {
    if prot & !KNOWN_PROT != 0 {
        return None;
    }
    if prot & (mman::PROT_SEM | mman::PROT_BTI | mman::PROT_MTE) != 0 {
        missing("mprotect", "PROT_SEM, PROT_BTI and PROT_MTE have no host equivalent");
    }
    Some(prot_to_host(prot))
}

pub fn protect(address: usize, length: usize, prot: i32) -> i64 {
    // cite: bionic mman.h#mprotect
    if address % PAGE_SIZE != 0 || length == 0 {
        return error(EINVAL);
    }
    let Some(host) = host_prot(prot) else { return error(EINVAL) };
    if unsafe { libc::mprotect(address as *mut libc::c_void, length, host) } < 0 {
        return from_host();
    }
    map_registry::update_range(address, length, |record| record.prot = host);
    0
}

fn advice_to_host(advice: i32) -> Option<i32> {
    // cite: bionic mman-common.h#MADV_NORMAL, MADV_RANDOM, MADV_SEQUENTIAL, MADV_WILLNEED, MADV_DONTNEED, MADV_FREE
    match advice {
        mman::MADV_NORMAL => Some(libc::MADV_NORMAL),
        mman::MADV_RANDOM => Some(libc::MADV_RANDOM),
        mman::MADV_SEQUENTIAL => Some(libc::MADV_SEQUENTIAL),
        mman::MADV_WILLNEED => Some(libc::MADV_WILLNEED),
        mman::MADV_DONTNEED => Some(libc::MADV_DONTNEED),
        mman::MADV_FREE => Some(libc::MADV_FREE),
        _ => None,
    }
}

pub fn advise(address: usize, length: usize, advice: i32) -> i64 {
    // cite: bionic mman.h#madvise
    if address % PAGE_SIZE != 0 {
        return error(EINVAL);
    }
    let Some(host) = advice_to_host(advice) else {
        missing("madvise", &format!("advice {advice} has no host equivalent"));
        return error(EINVAL);
    };
    if unsafe { libc::madvise(address as *mut libc::c_void, length, host) } < 0 {
        return from_host();
    }
    0
}

fn page_round(length: usize) -> usize {
    length.div_ceil(PAGE_SIZE) * PAGE_SIZE
}

fn copy_region(source: usize, old_size: usize, new_size: usize) -> i64 {
    let replacement = unsafe { libc::mmap(std::ptr::null_mut(), new_size, libc::PROT_READ | libc::PROT_WRITE, libc::MAP_PRIVATE | libc::MAP_ANON, -1, 0) };
    if replacement == libc::MAP_FAILED {
        return from_host();
    }
    unsafe { std::ptr::copy_nonoverlapping(source as *const u8, replacement as *mut u8, old_size) };
    let record = map_registry::find(source).map(|(base, record)| {
        let mut shifted = record;
        shifted.file.as_mut().into_iter().for_each(|file| file.offset += (source - base) as u64);
        shifted
    });
    if record.as_ref().is_some_and(|record| record.file.is_some()) {
        missing("mremap", "growing a file backed mapping has no host equivalent");
        return error(ENOMEM);
    }
    unsafe { libc::munmap(source as *mut libc::c_void, old_size) };
    map_registry::remove_range(source, old_size);
    if let Some(mut record) = record {
        record.len = new_size;
        map_registry::insert(replacement as usize, record);
    }
    replacement as i64
}

pub fn remap(old_address: usize, old_size: usize, new_size: usize, flags: i32) -> i64 {
    // cite: bionic mremap.cpp#mremap
    if old_address % PAGE_SIZE != 0 {
        return error(EINVAL);
    }
    if flags & mman::MREMAP_FIXED != 0 {
        missing("mremap", "MREMAP_FIXED needs a caller chosen address");
        return error(EINVAL);
    }
    if flags & mman::MREMAP_DONTUNMAP != 0 {
        missing("mremap", "MREMAP_DONTUNMAP has no host equivalent");
        return error(EINVAL);
    }
    if new_size == 0 {
        return error(EINVAL);
    }
    let old_size = page_round(old_size);
    let new_size = page_round(new_size);
    if new_size == old_size {
        return old_address as i64;
    }
    if new_size < old_size {
        let target = old_address + new_size;
        if unsafe { libc::munmap(target as *mut libc::c_void, old_size - new_size) } < 0 {
            return from_host();
        }
        map_registry::remove_range(target, old_size - new_size);
        map_registry::update_range(old_address, new_size, |record| record.len = record.len.min(new_size));
        return old_address as i64;
    }
    copy_region(old_address, old_size, new_size)
}

pub fn sync(address: usize, length: usize, flags: i32) -> i64 {
    // cite: bionic mman-common.h#MS_ASYNC, MS_INVALIDATE, MS_SYNC
    if address % PAGE_SIZE != 0 {
        return error(EINVAL);
    }
    let mut host = 0;
    for (from, to) in [(mman::MS_ASYNC, libc::MS_ASYNC), (mman::MS_INVALIDATE, libc::MS_INVALIDATE), (mman::MS_SYNC, libc::MS_SYNC)] {
        if flags & from != 0 {
            host |= to;
        }
    }
    if host == 0 || flags & !(mman::MS_ASYNC | mman::MS_INVALIDATE | mman::MS_SYNC) != 0 {
        return error(EINVAL);
    }
    if unsafe { libc::msync(address as *mut libc::c_void, length, host) } < 0 {
        from_host()
    } else {
        0
    }
}

fn lock_range(address: usize, length: usize, unlock: bool) -> i64 {
    let result = unsafe { if unlock { libc::munlock(address as *mut libc::c_void, length) } else { libc::mlock(address as *const libc::c_void, length) } };
    if result < 0 {
        from_host()
    } else {
        0
    }
}

fn lock_all(flags: i32, unlock: bool) -> i64 {
    if flags & !(mman::MCL_CURRENT | mman::MCL_FUTURE | mman::MCL_ONFAULT) != 0 {
        return error(EINVAL);
    }
    if flags & mman::MCL_ONFAULT != 0 {
        missing("mlockall", "MCL_ONFAULT has no host equivalent");
        return error(EINVAL);
    }
    let result = unsafe { if unlock { libc::munlockall() } else { libc::mlockall(flags) } };
    if result < 0 {
        from_host()
    } else {
        0
    }
}

fn mincore(address: usize, length: usize, vector: usize) -> i64 {
    // cite: bionic mman.h#mincore
    if address % PAGE_SIZE != 0 || vector == 0 {
        return error(EINVAL);
    }
    let pages = length.div_ceil(PAGE_SIZE);
    let mut bytes = vec![0u8; pages];
    if unsafe { libc::mincore(address as *mut libc::c_void, length, bytes.as_mut_ptr() as *mut libc::c_char) } < 0 {
        return from_host();
    }
    super::guest_mem::write_bytes(vector, &bytes);
    0
}

pub fn register() {
    for name in ["mmap", "mmap64"] {
        in_libc(name, |context| {
            let result = map(context.arg_ptr(0), context.arg_u64(1) as usize, context.arg_i32(2), context.arg_i32(3), context.arg_i32(4), context.arg_u64(5) as i64);
            if result < 0 {
                finish_long(context, result);
            } else {
                context.ret_ptr(result as usize);
            }
        });
    }
    in_libc("munmap", |context| {
        let result = unmap(context.arg_ptr(0), context.arg_u64(1) as usize);
        finish_int(context, result);
    });
    in_libc("mprotect", |context| {
        let result = protect(context.arg_ptr(0), context.arg_u64(1) as usize, context.arg_i32(2));
        finish_int(context, result);
    });
    in_libc("madvise", |context| {
        let result = advise(context.arg_ptr(0), context.arg_u64(1) as usize, context.arg_i32(2));
        finish_int(context, result);
    });
    in_libc("mremap", |context| {
        let result = remap(context.arg_ptr(0), context.arg_u64(1) as usize, context.arg_u64(2) as usize, context.arg_i32(3));
        if result < 0 {
            finish_long(context, result);
        } else {
            context.ret_ptr(result as usize);
        }
    });
    in_libc("msync", |context| {
        let result = sync(context.arg_ptr(0), context.arg_u64(1) as usize, context.arg_i32(2));
        finish_int(context, result);
    });
    in_libc("mlock", |context| {
        let result = lock_range(context.arg_ptr(0), context.arg_u64(1) as usize, false);
        finish_int(context, result);
    });
    in_libc("munlock", |context| {
        let result = lock_range(context.arg_ptr(0), context.arg_u64(1) as usize, true);
        finish_int(context, result);
    });
    in_libc("mlockall", |context| {
        let result = lock_all(context.arg_i32(0), false);
        finish_int(context, result);
    });
    in_libc("munlockall", |context| {
        let result = lock_all(0, true);
        finish_int(context, result);
    });
    in_libc("mincore", |context| {
        let result = mincore(context.arg_ptr(0), context.arg_u64(1) as usize, context.arg_ptr(2));
        finish_int(context, result);
    });
}
