extern "C" {
    static mach_task_self_: u32;
}

pub fn current() -> u32 {
    unsafe { mach_task_self_ }
}
