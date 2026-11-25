use super::super::bridge::CallContext;
use super::super::host_log;
use super::abi_types::{EINVAL, ENOMEM, MALLINFO_SIZE};
use super::export::in_libc;

const M_DECAY_TIME: i32 = -100;
const M_PURGE: i32 = -101;
const M_MEMTAG_TUNING: i32 = -102;
const M_THREAD_DISABLE_MEM_INIT: i32 = -103;
const M_PURGE_ALL: i32 = -104;
const M_CACHE_COUNT_MAX: i32 = -200;
const M_CACHE_SIZE_MAX: i32 = -201;
const M_TSDS_COUNT_MAX: i32 = -202;
const M_BIONIC_ZERO_INIT: i32 = -203;
const M_BIONIC_SET_HEAP_TAGGING_LEVEL: i32 = -204;
const HEAP_TAGGING_NONE: i32 = 0;

#[repr(C)]
struct ZoneStatistics {
    blocks_in_use: u32,
    size_in_use: usize,
    max_size_in_use: usize,
    size_allocated: usize,
}

extern "C" {
    fn malloc_zone_statistics(zone: *mut libc::c_void, stats: *mut ZoneStatistics);
    fn malloc_size(pointer: *const libc::c_void) -> usize;
}

fn return_allocation(context: &mut CallContext, pointer: *mut libc::c_void) {
    if pointer.is_null() {
        context.set_errno(ENOMEM);
    }
    context.ret_ptr(pointer as usize);
}

fn realloc(context: &mut CallContext) {
    // cite: bionic malloc_common.cpp#realloc
    let (pointer, size) = (context.arg_ptr(0), context.arg_u64(1) as usize);
    if pointer != 0 && size == 0 {
        unsafe { libc::free(pointer as *mut libc::c_void) };
        context.ret_ptr(0);
        return;
    }
    let grown = unsafe { libc::realloc(pointer as *mut libc::c_void, size) };
    return_allocation(context, grown);
}

fn reallocarray(context: &mut CallContext) {
    // cite: bionic malloc_common.cpp#reallocarray
    let (pointer, count, size) = (context.arg_ptr(0), context.arg_u64(1), context.arg_u64(2));
    let Some(total) = count.checked_mul(size) else {
        context.set_errno(ENOMEM);
        context.ret_ptr(0);
        return;
    };
    if pointer != 0 && total == 0 {
        unsafe { libc::free(pointer as *mut libc::c_void) };
        context.ret_ptr(0);
        return;
    }
    let grown = unsafe { libc::realloc(pointer as *mut libc::c_void, total as usize) };
    return_allocation(context, grown);
}

fn aligned(context: &mut CallContext, round_to_power_of_two: bool) {
    // cite: bionic malloc_common.cpp#memalign
    let mut alignment = (context.arg_u64(0) as usize).max(std::mem::size_of::<usize>());
    if !alignment.is_power_of_two() {
        if !round_to_power_of_two {
            context.set_errno(EINVAL);
            context.ret_ptr(0);
            return;
        }
        alignment = alignment.checked_next_power_of_two().unwrap_or(0);
    }
    let mut result: *mut libc::c_void = std::ptr::null_mut();
    let status = unsafe { libc::posix_memalign(&mut result, alignment, context.arg_u64(1) as usize) };
    if status != 0 {
        context.set_errno(status);
        context.ret_ptr(0);
        return;
    }
    context.ret_ptr(result as usize);
}

fn mallinfo(context: &mut CallContext) {
    // cite: bionic malloc.h#mallinfo
    let mut statistics = ZoneStatistics { blocks_in_use: 0, size_in_use: 0, max_size_in_use: 0, size_allocated: 0 };
    unsafe { malloc_zone_statistics(std::ptr::null_mut(), &mut statistics) };
    let mut fields = [0u64; MALLINFO_SIZE / 8];
    fields[0] = statistics.size_allocated as u64;
    fields[5] = statistics.max_size_in_use as u64;
    fields[7] = statistics.size_in_use as u64;
    fields[8] = statistics.size_allocated.saturating_sub(statistics.size_in_use) as u64;
    let bytes: Vec<u8> = fields.iter().flat_map(|field| field.to_le_bytes()).collect();
    context.ret_sret(&bytes);
}

fn mallopt(context: &mut CallContext) {
    // cite: bionic malloc.h#M_DECAY_TIME, M_PURGE, M_PURGE_ALL, M_MEMTAG_TUNING, M_THREAD_DISABLE_MEM_INIT, M_CACHE_COUNT_MAX, M_CACHE_SIZE_MAX, M_TSDS_COUNT_MAX, M_BIONIC_ZERO_INIT, M_BIONIC_SET_HEAP_TAGGING_LEVEL
    // cite: bionic heap_tagging.cpp#SetHeapTaggingLevel
    let (option, value) = (context.arg_i32(0), context.arg_i32(1));
    let accepted = match option {
        M_DECAY_TIME | M_THREAD_DISABLE_MEM_INIT | M_MEMTAG_TUNING => value == 0 || value == 1,
        M_PURGE | M_PURGE_ALL | M_CACHE_COUNT_MAX | M_CACHE_SIZE_MAX | M_TSDS_COUNT_MAX => true,
        M_BIONIC_SET_HEAP_TAGGING_LEVEL => value == HEAP_TAGGING_NONE,
        M_BIONIC_ZERO_INIT => {
            host_log::line("missing-primitive mallopt(M_BIONIC_ZERO_INIT): malloc zero initialisation on demand");
            false
        }
        _ => false,
    };
    context.ret_i32(i32::from(accepted));
}

pub fn register() {
    in_libc("malloc", |context| {
        // cite: bionic malloc_common.cpp#malloc
        let pointer = unsafe { libc::malloc(context.arg_u64(0) as usize) };
        return_allocation(context, pointer);
    });
    in_libc("free", |context| unsafe { libc::free(context.arg_ptr(0) as *mut libc::c_void) });
    in_libc("calloc", |context| {
        // cite: bionic malloc_common.cpp#calloc
        let pointer = unsafe { libc::calloc(context.arg_u64(0) as usize, context.arg_u64(1) as usize) };
        return_allocation(context, pointer);
    });
    in_libc("realloc", realloc);
    in_libc("reallocarray", reallocarray);
    in_libc("posix_memalign", |context| {
        let status = unsafe { libc::posix_memalign(context.arg_ptr(0) as *mut *mut libc::c_void, context.arg_u64(1) as usize, context.arg_u64(2) as usize) };
        context.ret_i32(status);
    });
    in_libc("memalign", |context| aligned(context, true));
    in_libc("aligned_alloc", |context| aligned(context, false));
    in_libc("malloc_usable_size", |context| {
        let pointer = context.arg_ptr(0);
        context.ret_u64(if pointer == 0 { 0 } else { unsafe { malloc_size(pointer as *const libc::c_void) as u64 } });
    });
    in_libc("mallinfo", mallinfo);
    in_libc("mallopt", mallopt);
}
