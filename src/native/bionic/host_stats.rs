use std::ffi::CString;

use super::sysinfo::sysctl_u64;

pub const TICKS_PER_SECOND: u64 = 100;

#[derive(Default, Clone, Copy)]
pub struct TaskStats {
    pub virtual_size: u64,
    pub resident_size: u64,
    pub faults: u64,
    pub pageins: u64,
    pub threads: u64,
    pub parent: i32,
    pub group: u32,
    pub start_seconds: u64,
    pub user_ticks: u64,
    pub system_ticks: u64,
    pub max_resident: u64,
    pub flags: u32,
    pub voluntary_switches: u64,
    pub involuntary_switches: u64,
}

const PROC_FLAG_TRACED: u32 = 0x800;

pub fn task() -> TaskStats {
    let mut info: libc::proc_taskallinfo = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<libc::proc_taskallinfo>() as i32;
    let read = unsafe {
        libc::proc_pidinfo(
            libc::getpid(),
            libc::PROC_PIDTASKALLINFO,
            0,
            &mut info as *mut _ as *mut libc::c_void,
            size,
        )
    };
    let mut stats = TaskStats::default();
    if read == size {
        stats.virtual_size = info.ptinfo.pti_virtual_size;
        stats.resident_size = info.ptinfo.pti_resident_size;
        stats.faults = info.ptinfo.pti_faults as u64;
        stats.pageins = info.ptinfo.pti_pageins as u64;
        stats.threads = info.ptinfo.pti_threadnum as u64;
        stats.parent = info.pbsd.pbi_ppid as i32;
        stats.group = info.pbsd.pbi_pgid;
        stats.start_seconds = info.pbsd.pbi_start_tvsec;
        stats.flags = info.pbsd.pbi_flags;
    }
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) };
    let ticks = |value: libc::timeval| {
        value.tv_sec as u64 * TICKS_PER_SECOND + value.tv_usec as u64 * TICKS_PER_SECOND / 1_000_000
    };
    stats.user_ticks = ticks(usage.ru_utime);
    stats.system_ticks = ticks(usage.ru_stime);
    stats.max_resident = usage.ru_maxrss as u64;
    stats.voluntary_switches = usage.ru_nvcsw as u64;
    stats.involuntary_switches = usage.ru_nivcsw as u64;
    stats
}

#[derive(Default, Clone, Copy)]
pub struct MemoryStats {
    pub vm_stats_available: bool,
    pub total: u64,
    pub free: u64,
    pub available: u64,
    pub cached: u64,
    pub active: u64,
    pub inactive: u64,
    pub anonymous: u64,
    pub swap_total: u64,
    pub swap_free: u64,
}

extern "C" {
    fn mach_host_self() -> libc::mach_port_t;
    fn mach_task_self() -> libc::mach_port_t;
}

pub fn memory() -> MemoryStats {
    let page = sysctl_u64("hw.pagesize").unwrap_or(16384);
    let mut stats = MemoryStats {
        total: sysctl_u64("hw.memsize").unwrap_or(0),
        ..MemoryStats::default()
    };
    let mut vm: libc::vm_statistics64 = unsafe { std::mem::zeroed() };
    let mut count = (std::mem::size_of::<libc::vm_statistics64>()
        / std::mem::size_of::<libc::integer_t>())
        as libc::mach_msg_type_number_t;
    let status = unsafe {
        libc::host_statistics64(
            mach_host_self(),
            libc::HOST_VM_INFO64,
            &mut vm as *mut _ as *mut libc::integer_t,
            &mut count,
        )
    };
    if status == 0 {
        stats.vm_stats_available = true;
        stats.free = u64::from(vm.free_count.saturating_sub(vm.speculative_count)) * page;
        let used = u64::from(vm.internal_page_count.saturating_sub(vm.purgeable_count))
            + u64::from(vm.wire_count)
            + u64::from(vm.compressor_page_count);
        stats.available = stats.total.saturating_sub(used * page);
        stats.cached = u64::from(vm.external_page_count) * page;
        stats.active = u64::from(vm.active_count) * page;
        stats.inactive = u64::from(vm.inactive_count) * page;
        stats.anonymous = u64::from(vm.internal_page_count) * page;
    }
    let mut swap: libc::xsw_usage = unsafe { std::mem::zeroed() };
    let mut length = std::mem::size_of::<libc::xsw_usage>();
    let name = CString::new("vm.swapusage").expect("no nul");
    if unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            &mut swap as *mut _ as *mut libc::c_void,
            &mut length,
            std::ptr::null_mut(),
            0,
        )
    } == 0
    {
        stats.swap_total = swap.xsu_total;
        stats.swap_free = swap.xsu_avail;
    }
    stats
}

pub fn boot_seconds() -> i64 {
    let mut boot = libc::timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    let mut length = std::mem::size_of::<libc::timeval>();
    let name = CString::new("kern.boottime").expect("no nul");
    unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            &mut boot as *mut _ as *mut libc::c_void,
            &mut length,
            std::ptr::null_mut(),
            0,
        )
    };
    boot.tv_sec
}

pub fn uptime_seconds() -> f64 {
    let mut now = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    unsafe { libc::clock_gettime(libc::CLOCK_REALTIME, &mut now) };
    (now.tv_sec - boot_seconds()) as f64 + now.tv_nsec as f64 / 1e9
}

pub fn load_average() -> [f64; 3] {
    let mut loads = [0f64; 3];
    unsafe { libc::getloadavg(loads.as_mut_ptr(), 3) };
    loads
}

pub fn cpu_count() -> u64 {
    sysctl_u64("hw.logicalcpu").unwrap_or(1).max(1)
}

pub fn cpu_count_max() -> u64 {
    sysctl_u64("hw.logicalcpu_max").unwrap_or(1).max(1)
}

pub fn timebase_hz() -> u64 {
    sysctl_u64("hw.tbfrequency").unwrap_or(24_000_000)
}

pub fn cpu_ticks() -> Vec<[u64; 4]> {
    let mut count: libc::natural_t = 0;
    let mut info: libc::processor_info_array_t = std::ptr::null_mut();
    let mut length: libc::mach_msg_type_number_t = 0;
    let status = unsafe {
        libc::host_processor_info(
            mach_host_self(),
            libc::PROCESSOR_CPU_LOAD_INFO,
            &mut count,
            &mut info,
            &mut length,
        )
    };
    if status != 0 || info.is_null() {
        return Vec::new();
    }
    let loads = unsafe {
        std::slice::from_raw_parts(info as *const libc::processor_cpu_load_info, count as usize)
    };
    let ticks = loads
        .iter()
        .map(|load| {
            [
                u64::from(load.cpu_ticks[0]),
                u64::from(load.cpu_ticks[1]),
                u64::from(load.cpu_ticks[2]),
                u64::from(load.cpu_ticks[3]),
            ]
        })
        .collect();
    unsafe {
        libc::vm_deallocate(
            mach_task_self(),
            info as libc::vm_address_t,
            length as libc::vm_size_t * 4,
        )
    };
    ticks
}

pub fn is_traced(stats: &TaskStats) -> bool {
    stats.flags & PROC_FLAG_TRACED != 0
}
