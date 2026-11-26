use std::ffi::CString;
use std::sync::OnceLock;

use super::abi_types::{
    EFAULT, EINVAL, ENAMETOOLONG, ENOENT, PAGE_SIZE, UTSNAME_FIELD, UTSNAME_SIZE,
};
use super::device_profile::{
    APP_GID, APP_UID, DOMAIN_NAME, HOST_NAME, KERNEL_RELEASE, KERNEL_VERSION,
};
use super::export::in_libc;
use super::guest_mem::write_bytes;
use super::host_stats;
use super::sysconf_table::CONSTANTS;

pub const AT_FLAGS: u64 = 8;
pub const AT_UID: u64 = 11;
pub const AT_EUID: u64 = 12;
pub const AT_GID: u64 = 13;
pub const AT_EGID: u64 = 14;
pub const AT_PLATFORM: u64 = 15;
pub const AT_HWCAP: u64 = 16;
pub const AT_PAGESZ: u64 = 6;
pub const AT_CLKTCK: u64 = 17;
pub const AT_SECURE: u64 = 23;
pub const AT_RANDOM: u64 = 25;
pub const AT_HWCAP2: u64 = 26;
const CLOCK_TICKS: i64 = 100;
const ARG_MAX_FLOOR: u64 = 131072;
const STACK_DEFAULT: u64 = 8_388_608;
const OPEN_FILES_TARGET: u64 = 32768;

// cite: bionic sysinfo.h#sysinfo, SI_LOAD_SHIFT
const SYSINFO_SIZE: usize = 112;
const LOAD_SCALE: f64 = 65536.0;

const HWCAP: &[(u32, &str)] = &[
    (0, "hw.optional.AdvSIMD"),
    (1, "hw.optional.AdvSIMD"),
    (3, "hw.optional.arm.FEAT_AES"),
    (4, "hw.optional.arm.FEAT_PMULL"),
    (5, "hw.optional.arm.FEAT_SHA1"),
    (6, "hw.optional.arm.FEAT_SHA256"),
    (7, "hw.optional.arm.FEAT_CRC32"),
    (8, "hw.optional.arm.FEAT_LSE"),
    (9, "hw.optional.arm.FEAT_FP16"),
    (10, "hw.optional.arm.FEAT_FP16"),
    (12, "hw.optional.arm.FEAT_RDM"),
    (13, "hw.optional.arm.FEAT_JSCVT"),
    (14, "hw.optional.arm.FEAT_FCMA"),
    (15, "hw.optional.arm.FEAT_LRCPC"),
    (16, "hw.optional.arm.FEAT_DPB"),
    (17, "hw.optional.arm.FEAT_SHA3"),
    (20, "hw.optional.arm.FEAT_DotProd"),
    (21, "hw.optional.arm.FEAT_SHA512"),
    (23, "hw.optional.arm.FEAT_FHM"),
    (24, "hw.optional.arm.FEAT_DIT"),
    (25, "hw.optional.arm.FEAT_LSE2"),
    (26, "hw.optional.arm.FEAT_LRCPC2"),
    (27, "hw.optional.arm.FEAT_FlagM"),
    (28, "hw.optional.arm.FEAT_SSBS"),
    (29, "hw.optional.arm.FEAT_SB"),
];

const HWCAP2: &[(u32, &str)] = &[
    (0, "hw.optional.arm.FEAT_DPB2"),
    (7, "hw.optional.arm.FEAT_FlagM2"),
    (8, "hw.optional.arm.FEAT_FRINTTS"),
    (13, "hw.optional.arm.FEAT_I8MM"),
    (14, "hw.optional.arm.FEAT_BF16"),
    (17, "hw.optional.arm.FEAT_BTI"),
    (19, "hw.optional.arm.FEAT_ECV"),
    (20, "hw.optional.arm.FEAT_AFP"),
    (21, "hw.optional.arm.FEAT_RPRES"),
];

static RANDOM_BYTES: OnceLock<usize> = OnceLock::new();
static PLATFORM: OnceLock<usize> = OnceLock::new();

pub fn sysctl_u64(name: &str) -> Option<u64> {
    let name = CString::new(name).ok()?;
    let mut buffer = [0u8; 8];
    let mut length = buffer.len();
    let status = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            buffer.as_mut_ptr() as *mut libc::c_void,
            &mut length,
            std::ptr::null_mut(),
            0,
        )
    };
    (status == 0).then(|| match length {
        4 => u64::from(u32::from_le_bytes(buffer[..4].try_into().expect("4 bytes"))),
        8 => u64::from_le_bytes(buffer),
        _ => 0,
    })
}

fn capability_mask(table: &[(u32, &str)]) -> u64 {
    table
        .iter()
        .filter(|(_, name)| sysctl_u64(name) == Some(1))
        .fold(0, |mask, (bit, _)| mask | (1u64 << bit))
}

fn host_limit(resource: i32) -> u64 {
    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    unsafe { libc::getrlimit(resource as _, &mut limit) };
    if limit.rlim_cur == libc::RLIM_INFINITY {
        u64::MAX
    } else {
        limit.rlim_cur
    }
}

fn raise_open_file_limit() {
    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit) };
    let per_process = sysctl_u64("kern.maxfilesperproc").unwrap_or(OPEN_FILES_TARGET);
    let wanted = OPEN_FILES_TARGET.min(per_process).min(limit.rlim_max);
    if wanted > limit.rlim_cur {
        limit.rlim_cur = wanted;
        unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &limit) };
    }
}

fn dynamic_sysconf(name: i32) -> Option<i64> {
    // cite: bionic sysconf.cpp#sysconf
    Some(match name {
        0 => {
            let stack = match host_limit(libc::RLIMIT_STACK) {
                u64::MAX => STACK_DEFAULT,
                limit => limit,
            };
            (stack / 4).min(3 * STACK_DEFAULT / 4).max(ARG_MAX_FLOOR) as i64
        }
        5 => host_limit(libc::RLIMIT_NPROC) as i64,
        6 => CLOCK_TICKS,
        11 => host_limit(libc::RLIMIT_NOFILE) as i64,
        0x27 | 0x28 => PAGE_SIZE as i64,
        0x60 => sysctl_u64("hw.logicalcpu_max")? as i64,
        0x61 => sysctl_u64("hw.logicalcpu")? as i64,
        0x62 => (sysctl_u64("hw.memsize")? / PAGE_SIZE as u64) as i64,
        0x63 => {
            (sysctl_u64("vm.page_free_count")? * sysctl_u64("hw.pagesize")? / PAGE_SIZE as u64)
                as i64
        }
        _ => return None,
    })
}

pub fn sysconf(name: i32) -> Option<i64> {
    dynamic_sysconf(name).or_else(|| {
        CONSTANTS
            .iter()
            .find(|(number, _)| *number == name)
            .map(|(_, value)| *value)
    })
}

pub fn auxiliary(kind: u64) -> Option<u64> {
    // cite: bionic getauxval.cpp#getauxval
    match kind {
        AT_PAGESZ => Some(PAGE_SIZE as u64),
        AT_CLKTCK => Some(CLOCK_TICKS as u64),
        AT_HWCAP => Some(capability_mask(HWCAP)),
        AT_HWCAP2 => Some(capability_mask(HWCAP2)),
        AT_SECURE | AT_FLAGS => Some(0),
        AT_UID | AT_EUID => Some(u64::from(APP_UID)),
        AT_GID | AT_EGID => Some(u64::from(APP_GID)),
        AT_PLATFORM => Some(*PLATFORM.get_or_init(|| {
            Box::leak(Box::new(CString::new("aarch64").expect("no nul"))).as_ptr() as usize
        }) as u64),
        AT_RANDOM => Some(*RANDOM_BYTES.get_or_init(|| {
            let bytes: &'static mut [u8; 16] = Box::leak(Box::new([0u8; 16]));
            unsafe { libc::getentropy(bytes.as_mut_ptr() as *mut libc::c_void, 16) };
            bytes.as_ptr() as usize
        }) as u64),
        _ => None,
    }
}

fn write_utsname(address: usize) {
    // cite: bionic utsname.h#utsname
    let mut block = vec![0u8; UTSNAME_SIZE];
    for (index, field) in [
        "Linux",
        HOST_NAME,
        KERNEL_RELEASE,
        KERNEL_VERSION,
        "aarch64",
        DOMAIN_NAME,
    ]
    .iter()
    .enumerate()
    {
        block[index * UTSNAME_FIELD..index * UTSNAME_FIELD + field.len()]
            .copy_from_slice(field.as_bytes());
    }
    write_bytes(address, &block);
}

fn write_sysinfo(address: usize) {
    // cite: bionic sysinfo.h#sysinfo
    let memory = host_stats::memory();
    let loads = host_stats::load_average().map(|load| (load * LOAD_SCALE) as u64);
    let mut block = [0u8; SYSINFO_SIZE];
    for (offset, value) in [
        (0, host_stats::uptime_seconds() as u64),
        (8, loads[0]),
        (16, loads[1]),
        (24, loads[2]),
        (32, memory.total),
        (40, memory.free),
        (64, memory.swap_total),
        (72, memory.swap_free),
    ] {
        block[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
    block[80..82].copy_from_slice(&(host_stats::task().threads as u16).to_le_bytes());
    block[104..108].copy_from_slice(&1u32.to_le_bytes());
    write_bytes(address, &block);
}

fn copy_name(context: &mut super::super::bridge::CallContext, text: &str) {
    let (buffer, length) = (context.arg_ptr(0), context.arg_u64(1) as usize);
    if text.len() + 1 > length {
        context.set_errno(ENAMETOOLONG);
        context.ret_i32(-1);
        return;
    }
    let mut bytes = text.as_bytes().to_vec();
    bytes.push(0);
    write_bytes(buffer, &bytes);
    context.ret_i32(0);
}

pub fn register() {
    raise_open_file_limit();
    in_libc("sysconf", |context| match sysconf(context.arg_i32(0)) {
        Some(value) => context.ret_i64(value),
        None => {
            context.set_errno(EINVAL);
            context.ret_i64(-1);
        }
    });
    in_libc("getpagesize", |context| context.ret_i32(PAGE_SIZE as i32));
    in_libc("getauxval", |context| match auxiliary(context.arg_u64(0)) {
        Some(value) => context.ret_u64(value),
        None => {
            context.set_errno(ENOENT);
            context.ret_u64(0);
        }
    });
    in_libc("uname", |context| {
        write_utsname(context.arg_ptr(0));
        context.ret_i32(0);
    });
    in_libc("sysinfo", |context| {
        if context.arg_ptr(0) == 0 {
            context.set_errno(EFAULT);
            context.ret_i32(-1);
            return;
        }
        write_sysinfo(context.arg_ptr(0));
        context.ret_i32(0);
    });
    in_libc("gethostname", |context| copy_name(context, HOST_NAME));
    in_libc("getdomainname", |context| copy_name(context, DOMAIN_NAME));
    in_libc("getuid", |context| context.ret_u32(APP_UID));
    in_libc("geteuid", |context| context.ret_u32(APP_UID));
    in_libc("getgid", |context| context.ret_u32(APP_GID));
    in_libc("getegid", |context| context.ret_u32(APP_GID));
}
