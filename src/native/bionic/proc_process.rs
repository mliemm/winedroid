use super::super::signal_numbers::{bit, SIGNAL_COUNT};
use super::super::signal_state;
use super::super::thread_list;
use super::super::guest_thread;
use super::fd_table::{self, FdKind};
use super::guest_process;
use super::host_stats::{self, TICKS_PER_SECOND};
use super::sysinfo::{auxiliary, AT_CLKTCK, AT_EGID, AT_EUID, AT_FLAGS, AT_GID, AT_HWCAP, AT_HWCAP2, AT_PAGESZ, AT_PLATFORM, AT_RANDOM, AT_SECURE, AT_UID};
use crate::native::guest_context;
use super::abi_types::PAGE_SIZE;

pub fn pid() -> i32 {
    unsafe { libc::getpid() }
}

fn comm_of(record: usize) -> String {
    let bytes = thread_list::name_of(record);
    let end = bytes.iter().position(|byte| *byte == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

pub fn primary_record() -> Option<usize> {
    guest_thread::live_records().into_iter().map(|(record, _)| record).find(|record| guest_thread::is_primary(*record as *mut _))
}

pub fn thread_comm(tid: i32) -> Option<String> {
    let record = guest_thread::live_records().into_iter().find(|(_, live)| *live == tid)?.0;
    Some(comm_of(record))
}

pub fn process_comm() -> String {
    primary_record().map(comm_of).unwrap_or_else(|| "app_process64".to_string())
}

pub fn cmdline() -> Vec<u8> {
    let package = guest_context::current().map(|context| context.package.clone()).unwrap_or_default();
    let mut bytes = package.into_bytes();
    bytes.push(0);
    bytes
}

fn signal_masks() -> (u64, u64) {
    let (mut ignored, mut caught) = (0u64, 0u64);
    for signal in 1..=SIGNAL_COUNT {
        let action = signal_state::action_of(signal);
        if action.handler == signal_state::SIG_IGN {
            ignored |= bit(signal);
        } else if action.has_handler() {
            caught |= bit(signal);
        }
    }
    (ignored, caught)
}

fn traced_by(task: &host_stats::TaskStats) -> i32 {
    if host_stats::is_traced(task) {
        task.parent
    } else {
        0
    }
}

fn next_power_of_two_above(highest: i32) -> u32 {
    (highest.max(0) as u32 + 1).next_power_of_two().max(64)
}

pub fn status(thread: Option<i32>) -> Vec<u8> {
    let task = host_stats::task();
    let (ignored, caught) = signal_masks();
    let highest = fd_table::list().into_iter().max().unwrap_or(0);
    let name = thread.and_then(thread_comm).unwrap_or_else(process_comm);
    let uid = guest_process::uid();
    let gid = guest_process::gid();
    let mut text = String::new();
    let mut line = |label: &str, value: String| text.push_str(&format!("{label}:\t{value}\n"));
    line("Name", name);
    line("Umask", format!("{:04o}", guest_process::umask()));
    line("State", "S (sleeping)".to_string());
    line("Tgid", pid().to_string());
    line("Ngid", "0".to_string());
    line("Pid", pid().to_string());
    line("PPid", task.parent.to_string());
    line("TracerPid", traced_by(&task).to_string());
    line("Uid", format!("{uid}\t{uid}\t{uid}\t{uid}"));
    line("Gid", format!("{gid}\t{gid}\t{gid}\t{gid}"));
    line("FDSize", next_power_of_two_above(highest).to_string());
    line("Groups", String::new());
    line("VmPeak", format!("{:>8} kB", task.virtual_size / 1024));
    line("VmSize", format!("{:>8} kB", task.virtual_size / 1024));
    line("VmLck", format!("{:>8} kB", 0));
    line("VmPin", format!("{:>8} kB", 0));
    line("VmHWM", format!("{:>8} kB", task.max_resident / 1024));
    line("VmRSS", format!("{:>8} kB", task.resident_size / 1024));
    line("Threads", task.threads.to_string());
    line("SigQ", "0/0".to_string());
    line("SigPnd", format!("{:016x}", signal_state::pending()));
    line("ShdPnd", "0000000000000000".to_string());
    line("SigBlk", format!("{:016x}", signal_state::mask()));
    line("SigIgn", format!("{ignored:016x}"));
    line("SigCgt", format!("{caught:016x}"));
    for capability in ["CapInh", "CapPrm", "CapEff", "CapBnd", "CapAmb"] {
        line(capability, "0000000000000000".to_string());
    }
    line("NoNewPrivs", "0".to_string());
    line("Seccomp", "0".to_string());
    line("voluntary_ctxt_switches", task.voluntary_switches.to_string());
    line("nonvoluntary_ctxt_switches", task.involuntary_switches.to_string());
    text.into_bytes()
}

pub fn stat(thread: Option<i32>) -> Vec<u8> {
    // cite: bionic pthread_attr.cpp#__get_main_stack_startstack
    let task = host_stats::task();
    let (ignored, caught) = signal_masks();
    let name = thread.and_then(thread_comm).unwrap_or_else(process_comm);
    let boot = host_stats::boot_seconds() as u64;
    let start = task.start_seconds.saturating_sub(boot) * TICKS_PER_SECOND;
    let mut fields = vec!["0".to_string(); 52];
    let mut set = |number: usize, value: String| fields[number - 1] = value;
    set(1, thread.unwrap_or_else(pid).to_string());
    set(2, format!("({name})"));
    set(3, "S".to_string());
    set(4, task.parent.to_string());
    set(5, task.group.to_string());
    set(6, task.group.to_string());
    set(8, "-1".to_string());
    set(9, "4194560".to_string());
    set(10, task.faults.to_string());
    set(12, task.pageins.to_string());
    set(14, task.user_ticks.to_string());
    set(15, task.system_ticks.to_string());
    set(18, "20".to_string());
    set(20, task.threads.to_string());
    set(22, start.to_string());
    set(23, task.virtual_size.to_string());
    set(24, (task.resident_size / PAGE_SIZE as u64).to_string());
    set(25, u64::MAX.to_string());
    set(32, signal_state::mask().to_string());
    set(33, ignored.to_string());
    set(34, caught.to_string());
    set(38, "17".to_string());
    format!("{}\n", fields.join(" ")).into_bytes()
}

pub fn statm() -> Vec<u8> {
    let task = host_stats::task();
    let page = PAGE_SIZE as u64;
    format!("{} {} 0 0 0 0 0\n", task.virtual_size / page, task.resident_size / page).into_bytes()
}

pub fn auxv() -> Vec<u8> {
    let mut bytes = Vec::new();
    for kind in [AT_PAGESZ, AT_CLKTCK, AT_HWCAP, AT_HWCAP2, AT_FLAGS, AT_UID, AT_EUID, AT_GID, AT_EGID, AT_SECURE, AT_RANDOM, AT_PLATFORM] {
        if let Some(value) = auxiliary(kind) {
            bytes.extend_from_slice(&kind.to_le_bytes());
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    bytes.extend_from_slice(&[0u8; 16]);
    bytes
}

pub fn fd_entries() -> Vec<(String, u8)> {
    let mut entries = vec![(".".to_string(), 4), ("..".to_string(), 4)];
    entries.extend(fd_table::list().into_iter().map(|fd| (fd.to_string(), super::linux_abi::misc::DT_LNK)));
    entries
}

pub fn fd_link(fd: i32) -> Option<String> {
    let entry = fd_table::get(fd)?;
    let host_inode = || {
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        unsafe { libc::fstat(fd, &mut stat) };
        stat.st_ino
    };
    Some(match entry.kind {
        FdKind::Pipe => format!("pipe:[{}]", host_inode()),
        FdKind::Socket => format!("socket:[{}]", host_inode()),
        FdKind::EventFd => "anon_inode:[eventfd]".to_string(),
        FdKind::TimerFd => "anon_inode:[timerfd]".to_string(),
        FdKind::Epoll => "anon_inode:[eventpoll]".to_string(),
        _ => entry.guest_path.clone(),
    })
}

pub fn task_entries() -> Vec<(String, u8)> {
    let mut entries = vec![(".".to_string(), 4), ("..".to_string(), 4)];
    let mut tids: Vec<i32> = guest_thread::live_records().into_iter().map(|(_, tid)| tid).collect();
    tids.sort_unstable();
    entries.extend(tids.into_iter().map(|tid| (tid.to_string(), super::linux_abi::misc::DT_DIR)));
    entries
}
