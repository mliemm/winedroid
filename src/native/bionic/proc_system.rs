use super::host_stats::{self, TICKS_PER_SECOND};
use super::props;
use super::sysinfo::{auxiliary, sysctl_u64, AT_HWCAP, AT_HWCAP2};

// cite: bionic hwcap.h#HWCAP_FP, HWCAP_ASIMD, HWCAP_ATOMICS, HWCAP_DCPOP, HWCAP_PACG
pub const HWCAP_NAMES: [&str; 32] = [
    "fp", "asimd", "evtstrm", "aes", "pmull", "sha1", "sha2", "crc32", "atomics", "fphp", "asimdhp", "cpuid", "asimdrdm", "jscvt", "fcma", "lrcpc", "dcpop", "sha3", "sm3", "sm4", "asimddp", "sha512", "sve",
    "asimdfhm", "dit", "uscat", "ilrcpc", "flagm", "ssbs", "sb", "paca", "pacg",
];

// cite: bionic hwcap.h#HWCAP2_DCPODP, HWCAP2_BTI, HWCAP2_RPRES
pub const HWCAP2_NAMES: [&str; 22] = [
    "dcpodp", "sve2", "sveaes", "svepmull", "svebitperm", "svesha3", "svesm4", "flagm2", "frint", "svei8mm", "svef32mm", "svef64mm", "svebf16", "i8mm", "bf16", "dgh", "rng", "bti", "mte", "ecv", "afp", "rpres",
];

pub fn feature_names() -> Vec<&'static str> {
    let mut names = Vec::new();
    let first = auxiliary(AT_HWCAP).unwrap_or(0);
    let second = auxiliary(AT_HWCAP2).unwrap_or(0);
    for (index, name) in HWCAP_NAMES.iter().enumerate() {
        if first >> index & 1 == 1 {
            names.push(*name);
        }
    }
    for (index, name) in HWCAP2_NAMES.iter().enumerate() {
        if second >> index & 1 == 1 {
            names.push(*name);
        }
    }
    names
}

const CPU_FAMILY_PARTS: &[(u64, u32)] = &[(0x1b588bb3, 0x022), (0xda33d83d, 0x032), (0x8765edea, 0x048), (0x2876f5b5, 0x049), (0x75d4acb9, 0x050)];

pub fn cpu_part() -> u32 {
    let family = sysctl_u64("hw.cpufamily").unwrap_or(0);
    match CPU_FAMILY_PARTS.iter().find(|(known, _)| *known == family) {
        Some((_, part)) => *part,
        None => {
            super::fs_result::missing("cpu part", &format!("no MIDR part is known for hw.cpufamily {family:#x}"));
            0
        }
    }
}

pub fn cpuinfo() -> Vec<u8> {
    let mut text = String::new();
    let features = feature_names().join(" ");
    let bogo = host_stats::timebase_hz() as f64 / 500_000.0;
    for index in 0..host_stats::cpu_count() {
        text.push_str(&format!(
            "processor\t: {index}\nBogoMIPS\t: {bogo:.2}\nFeatures\t: {features}\nCPU implementer\t: 0x61\nCPU architecture: 8\nCPU variant\t: 0x0\nCPU part\t: {:#05x}\nCPU revision\t: 0\n\n",
            cpu_part()
        ));
    }
    text.into_bytes()
}

fn kilobytes(label: &str, bytes: u64) -> String {
    format!("{:<16}{:>8} kB\n", format!("{label}:"), bytes / 1024)
}

pub fn meminfo() -> Vec<u8> {
    let memory = host_stats::memory();
    let mut text = String::new();
    for (label, value) in [
        ("MemTotal", memory.total),
        ("MemFree", memory.free),
        ("MemAvailable", memory.available),
        ("Cached", memory.cached),
        ("Active", memory.active),
        ("Inactive", memory.inactive),
        ("SwapTotal", memory.swap_total),
        ("SwapFree", memory.swap_free),
        ("AnonPages", memory.anonymous),
    ] {
        text.push_str(&kilobytes(label, value));
    }
    text.into_bytes()
}

pub fn uptime() -> Vec<u8> {
    let idle: u64 = host_stats::cpu_ticks().iter().map(|ticks| ticks[2]).sum();
    format!("{:.2} {:.2}\n", host_stats::uptime_seconds(), idle as f64 / TICKS_PER_SECOND as f64).into_bytes()
}

pub fn loadavg() -> Vec<u8> {
    let loads = host_stats::load_average();
    format!("{:.2} {:.2} {:.2} 1/1 {}\n", loads[0], loads[1], loads[2], unsafe { libc::getpid() }).into_bytes()
}

pub fn version() -> Vec<u8> {
    format!("Linux version {} (winedroid@localhost) {}\n", super::device_profile::KERNEL_RELEASE, super::device_profile::KERNEL_VERSION).into_bytes()
}

pub fn stat() -> Vec<u8> {
    let cpus = host_stats::cpu_ticks();
    let total = cpus.iter().fold([0u64; 4], |sum, ticks| [sum[0] + ticks[0], sum[1] + ticks[1], sum[2] + ticks[2], sum[3] + ticks[3]]);
    let mut text = format!("cpu  {} {} {} {} 0 0 0 0 0 0\n", total[0], total[3], total[1], total[2]);
    for (index, ticks) in cpus.iter().enumerate() {
        text.push_str(&format!("cpu{index} {} {} {} {} 0 0 0 0 0 0\n", ticks[0], ticks[3], ticks[1], ticks[2]));
    }
    text.push_str(&format!("btime {}\n", host_stats::boot_seconds()));
    text.into_bytes()
}

pub fn cpu_range(count: u64) -> Vec<u8> {
    if count <= 1 {
        b"0\n".to_vec()
    } else {
        format!("0-{}\n", count - 1).into_bytes()
    }
}

pub fn build_prop() -> Vec<u8> {
    let mut text = String::new();
    for name in props::ordered_names() {
        if name.starts_with("ro.") {
            if let Some(value) = props::get(&name) {
                text.push_str(&format!("{name}={value}\n"));
            }
        }
    }
    text.into_bytes()
}
