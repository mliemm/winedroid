use std::sync::atomic::{AtomicI32, AtomicU16, AtomicU32, AtomicU64};

pub fn u16_at(address: usize) -> &'static AtomicU16 {
    unsafe { AtomicU16::from_ptr(address as *mut u16) }
}

pub fn u32_at(address: usize) -> &'static AtomicU32 {
    unsafe { AtomicU32::from_ptr(address as *mut u32) }
}

pub fn i32_at(address: usize) -> &'static AtomicI32 {
    unsafe { AtomicI32::from_ptr(address as *mut i32) }
}

pub fn u64_at(address: usize) -> &'static AtomicU64 {
    unsafe { AtomicU64::from_ptr(address as *mut u64) }
}
