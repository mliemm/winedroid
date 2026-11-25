use std::sync::OnceLock;

use super::guest_regs::Regs;
use super::trampoline::{template_bounds, STUB_COUNT, STUB_SLOT_BYTES};

const HOST_PAGE: usize = 16384;
const LITERAL_BYTES: usize = 8;

extern "C" {
    fn sys_icache_invalidate(start: *mut libc::c_void, length: usize);
}

pub type DispatchFunction = extern "C" fn(u64, *mut Regs);

static BASE: OnceLock<usize> = OnceLock::new();

fn install(dispatch: DispatchFunction) -> usize {
    let (start, end) = template_bounds();
    let length = end - start;
    assert!(length > STUB_COUNT * STUB_SLOT_BYTES + LITERAL_BYTES, "stub template is truncated");
    let mapped = length.div_ceil(HOST_PAGE) * HOST_PAGE;
    let region = unsafe { libc::mmap(std::ptr::null_mut(), mapped, libc::PROT_READ | libc::PROT_WRITE, libc::MAP_PRIVATE | libc::MAP_ANON, -1, 0) };
    assert_ne!(region, libc::MAP_FAILED, "stub table mapping failed");
    unsafe {
        std::ptr::copy_nonoverlapping(start as *const u8, region as *mut u8, length);
        std::ptr::write_unaligned((region as *mut u8).add(length - LITERAL_BYTES) as *mut u64, dispatch as usize as u64);
        assert_eq!(libc::mprotect(region, mapped, libc::PROT_READ | libc::PROT_EXEC), 0, "stub table protection failed");
        sys_icache_invalidate(region, mapped);
    }
    region as usize
}

pub fn initialize(dispatch: DispatchFunction) -> usize {
    *BASE.get_or_init(|| install(dispatch))
}

pub fn stub_address(id: usize) -> usize {
    assert!(id < STUB_COUNT, "stub id {id} is outside the table");
    BASE.get().copied().expect("stub table is initialized before its first stub is handed out") + id * STUB_SLOT_BYTES
}

pub fn table_base() -> Option<usize> {
    BASE.get().copied()
}
