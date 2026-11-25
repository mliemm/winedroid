pub const PTHREAD_INTERNAL_SIZE: usize = 784;

pub const JOIN_NOT_JOINED: i32 = 0;
pub const JOIN_EXITED_NOT_JOINED: i32 = 1;
pub const JOIN_JOINED: i32 = 2;
pub const JOIN_DETACHED: i32 = 3;
pub const ATTR_FLAG_DETACHED: u32 = 1;
pub const ATTR_FLAG_INHERIT: u32 = 4;

#[repr(C)]
#[derive(Debug)]
pub struct PthreadAttr {
    pub flags: u32,
    pub reserved_flags: u32,
    pub stack_base: u64,
    pub stack_size: u64,
    pub guard_size: u64,
    pub sched_policy: i32,
    pub sched_priority: i32,
    pub reserved: [u8; 16],
}

#[repr(C, align(16))]
#[derive(Debug)]
pub struct PthreadInternal {
    pub next: u64,
    pub prev: u64,
    pub tid: i32,
    pub cached_pid: u32,
    pub attr: PthreadAttr,
    pub join_state: i32,
    pub join_state_padding: u32,
    pub cleanup_stack: u64,
    pub start_routine: u64,
    pub start_routine_arg: u64,
    pub return_value: u64,
    pub start_mask: u64,
    pub alternate_signal_stack: u64,
    pub shadow_call_stack_guard_region: u64,
    pub stack_top: u64,
    pub terminating: u8,
    pub terminating_padding: [u8; 3],
    pub startup_handshake_lock: [u8; 8],
    pub handshake_padding: u32,
    pub mmap_base: u64,
    pub mmap_size: u64,
    pub mmap_base_unguarded: u64,
    pub mmap_size_unguarded: u64,
    pub vma_name_buffer: [u8; 32],
    pub thread_local_dtors: u64,
    pub current_dlerror: u64,
    pub dlerror_buffer: [u8; 512],
    pub bionic_tls: u64,
    pub errno_value: i32,
    pub errno_padding: u32,
    pub vfork_child_stack_bottom: u64,
}

impl PthreadInternal {
    pub fn zeroed() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::offset_of;

    use super::*;

    #[test]
    fn bionic_pthread_internal_t_field_offsets_match_the_lp64_layout_guests_read() {
        // cite: bionic pthread_internal.h#pthread_internal_t
        assert_eq!(std::mem::size_of::<PthreadInternal>(), PTHREAD_INTERNAL_SIZE);
        assert_eq!(std::mem::align_of::<PthreadInternal>(), 16);
        let offsets = [
            (offset_of!(PthreadInternal, next), 0),
            (offset_of!(PthreadInternal, prev), 8),
            (offset_of!(PthreadInternal, tid), 16),
            (offset_of!(PthreadInternal, cached_pid), 20),
            (offset_of!(PthreadInternal, attr), 24),
            (offset_of!(PthreadInternal, attr) + offset_of!(PthreadAttr, stack_base), 32),
            (offset_of!(PthreadInternal, attr) + offset_of!(PthreadAttr, stack_size), 40),
            (offset_of!(PthreadInternal, attr) + offset_of!(PthreadAttr, guard_size), 48),
            (offset_of!(PthreadInternal, attr) + offset_of!(PthreadAttr, sched_policy), 56),
            (offset_of!(PthreadInternal, attr) + offset_of!(PthreadAttr, reserved), 64),
            (offset_of!(PthreadInternal, join_state), 80),
            (offset_of!(PthreadInternal, cleanup_stack), 88),
            (offset_of!(PthreadInternal, start_routine), 96),
            (offset_of!(PthreadInternal, start_routine_arg), 104),
            (offset_of!(PthreadInternal, return_value), 112),
            (offset_of!(PthreadInternal, start_mask), 120),
            (offset_of!(PthreadInternal, alternate_signal_stack), 128),
            (offset_of!(PthreadInternal, shadow_call_stack_guard_region), 136),
            (offset_of!(PthreadInternal, stack_top), 144),
            (offset_of!(PthreadInternal, terminating), 152),
            (offset_of!(PthreadInternal, startup_handshake_lock), 156),
            (offset_of!(PthreadInternal, mmap_base), 168),
            (offset_of!(PthreadInternal, mmap_size), 176),
            (offset_of!(PthreadInternal, mmap_base_unguarded), 184),
            (offset_of!(PthreadInternal, mmap_size_unguarded), 192),
            (offset_of!(PthreadInternal, vma_name_buffer), 200),
            (offset_of!(PthreadInternal, thread_local_dtors), 232),
            (offset_of!(PthreadInternal, current_dlerror), 240),
            (offset_of!(PthreadInternal, dlerror_buffer), 248),
            (offset_of!(PthreadInternal, bionic_tls), 760),
            (offset_of!(PthreadInternal, errno_value), 768),
            (offset_of!(PthreadInternal, vfork_child_stack_bottom), 776),
        ];
        for (actual, expected) in offsets {
            assert_eq!(actual, expected);
        }
    }
}
