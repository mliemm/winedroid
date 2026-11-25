use super::super::guest_context::GuestContext;
use super::fd_table;
use super::host_stats;
use super::linux_abi::misc::{DT_DIR, DT_LNK, DT_REG};
use super::proc_maps;
use super::proc_process::{self, pid};
use super::proc_system;
use super::synthetic_node::{Synthetic, PROC_DEVICE, SYS_DEVICE};

fn text(device: (u32, u32), build: impl Fn() -> Vec<u8> + Send + Sync + 'static) -> Option<Synthetic> {
    Some(Synthetic::file(device, build))
}

fn entries(names: &[(&str, u8)]) -> Vec<(String, u8)> {
    names.iter().map(|(name, kind)| (name.to_string(), *kind)).collect()
}

const PROCESS_FILES: &[(&str, u8)] = &[
    ("auxv", DT_REG),
    ("cmdline", DT_REG),
    ("comm", DT_REG),
    ("cwd", DT_LNK),
    ("exe", DT_LNK),
    ("fd", DT_DIR),
    ("maps", DT_REG),
    ("oom_score_adj", DT_REG),
    ("stat", DT_REG),
    ("statm", DT_REG),
    ("status", DT_REG),
    ("task", DT_DIR),
];

fn thread_node(tid: i32, rest: &[&str]) -> Option<Synthetic> {
    proc_process::thread_comm(tid)?;
    match rest {
        [] => Some(Synthetic::dir(PROC_DEVICE, || entries(&[("comm", DT_REG), ("stat", DT_REG), ("status", DT_REG)]))),
        ["comm"] => text(PROC_DEVICE, move || format!("{}\n", proc_process::thread_comm(tid).unwrap_or_default()).into_bytes()),
        ["stat"] => text(PROC_DEVICE, move || proc_process::stat(Some(tid))),
        ["status"] => text(PROC_DEVICE, move || proc_process::status(Some(tid))),
        _ => None,
    }
}

fn process_node(rest: &[&str]) -> Option<Synthetic> {
    match rest {
        [] => Some(Synthetic::dir(PROC_DEVICE, || entries(PROCESS_FILES))),
        ["maps"] => text(PROC_DEVICE, proc_maps::maps),
        ["status"] => text(PROC_DEVICE, || proc_process::status(None)),
        ["stat"] => text(PROC_DEVICE, || proc_process::stat(None)),
        ["statm"] => text(PROC_DEVICE, proc_process::statm),
        ["cmdline"] => text(PROC_DEVICE, proc_process::cmdline),
        ["comm"] => text(PROC_DEVICE, || format!("{}\n", proc_process::process_comm()).into_bytes()),
        ["auxv"] => text(PROC_DEVICE, proc_process::auxv),
        ["oom_score_adj"] => text(PROC_DEVICE, || b"0\n".to_vec()),
        ["exe"] => Some(Synthetic::link(PROC_DEVICE, || format!("/system/bin/{}", super::device_profile::PROGRAM_NAME))),
        ["cwd"] => Some(Synthetic::link(PROC_DEVICE, super::guest_process::cwd)),
        ["fd"] => Some(Synthetic::dir(PROC_DEVICE, proc_process::fd_entries)),
        ["fd", number] => {
            let descriptor: i32 = number.parse().ok()?;
            fd_table::get(descriptor)?;
            Some(Synthetic::link(PROC_DEVICE, move || proc_process::fd_link(descriptor).unwrap_or_default()))
        }
        ["task"] => Some(Synthetic::dir(PROC_DEVICE, proc_process::task_entries)),
        ["task", tid, rest @ ..] => thread_node(tid.parse().ok()?, rest),
        _ => None,
    }
}

fn proc_node(parts: &[&str]) -> Option<Synthetic> {
    match parts {
        [] => Some(Synthetic::dir(PROC_DEVICE, || {
            let mut listing = entries(&[("cpuinfo", DT_REG), ("loadavg", DT_REG), ("meminfo", DT_REG), ("self", DT_LNK), ("stat", DT_REG), ("uptime", DT_REG), ("version", DT_REG)]);
            listing.push((pid().to_string(), DT_DIR));
            listing
        })),
        ["self"] => Some(Synthetic::link(PROC_DEVICE, || pid().to_string())),
        ["cpuinfo"] => text(PROC_DEVICE, proc_system::cpuinfo),
        ["meminfo"] => text(PROC_DEVICE, proc_system::meminfo),
        ["stat"] => text(PROC_DEVICE, proc_system::stat),
        ["uptime"] => text(PROC_DEVICE, proc_system::uptime),
        ["loadavg"] => text(PROC_DEVICE, proc_system::loadavg),
        ["version"] => text(PROC_DEVICE, proc_system::version),
        ["self", rest @ ..] => process_node(rest),
        [number, rest @ ..] => {
            let id: i32 = number.parse().ok()?;
            if id == pid() {
                process_node(rest)
            } else {
                thread_node(id, rest)
            }
        }
    }
}

fn cpu_node(parts: &[&str]) -> Option<Synthetic> {
    let present = || proc_system::cpu_range(host_stats::cpu_count_max());
    match parts {
        [] => Some(Synthetic::dir(SYS_DEVICE, || {
            let mut listing = entries(&[("kernel_max", DT_REG), ("online", DT_REG), ("possible", DT_REG), ("present", DT_REG)]);
            listing.extend((0..host_stats::cpu_count()).map(|index| (format!("cpu{index}"), DT_DIR)));
            listing
        })),
        ["possible"] | ["present"] => text(SYS_DEVICE, present),
        ["online"] => text(SYS_DEVICE, || proc_system::cpu_range(host_stats::cpu_count())),
        ["kernel_max"] => text(SYS_DEVICE, || format!("{}\n", host_stats::cpu_count_max() - 1).into_bytes()),
        [name, rest @ ..] => {
            let index: u64 = name.strip_prefix("cpu")?.parse().ok()?;
            if index >= host_stats::cpu_count() {
                return None;
            }
            match rest {
                [] => Some(Synthetic::dir(SYS_DEVICE, || entries(&[("online", DT_REG), ("topology", DT_DIR)]))),
                ["online"] => text(SYS_DEVICE, || b"1\n".to_vec()),
                ["topology"] => Some(Synthetic::dir(SYS_DEVICE, || entries(&[("core_id", DT_REG), ("core_siblings_list", DT_REG), ("physical_package_id", DT_REG), ("thread_siblings_list", DT_REG)]))),
                ["topology", "physical_package_id"] => text(SYS_DEVICE, || b"0\n".to_vec()),
                ["topology", "core_id"] => text(SYS_DEVICE, move || format!("{index}\n").into_bytes()),
                ["topology", "core_siblings_list"] => text(SYS_DEVICE, || proc_system::cpu_range(host_stats::cpu_count())),
                ["topology", "thread_siblings_list"] => text(SYS_DEVICE, move || format!("{index}\n").into_bytes()),
                _ => None,
            }
        }
    }
}

pub fn lookup(path: &str, _context: &GuestContext) -> Option<Synthetic> {
    let parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    match parts.as_slice() {
        ["proc", rest @ ..] => proc_node(rest),
        ["sys", "devices", "system", "cpu", rest @ ..] => cpu_node(rest),
        ["system", "build.prop"] => text((0xfd, 0), proc_system::build_prop),
        _ => None,
    }
}
