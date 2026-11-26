use super::super::host_log;
use super::abi_types::EINVAL;
use super::errno_map::linux_from_darwin;

const SCHED_OTHER: i32 = 0;
const SCHED_FIFO: i32 = 1;
const SCHED_RR: i32 = 2;
const SCHED_BATCH: i32 = 3;
const SCHED_IDLE: i32 = 5;
const LINUX_PRIORITY_MAX: i32 = 99;

fn host_policy(linux: i32) -> Option<i32> {
    match linux {
        SCHED_FIFO => Some(libc::SCHED_FIFO),
        SCHED_RR => Some(libc::SCHED_RR),
        _ => None,
    }
}

pub fn set_host(host: libc::pthread_t, policy: i32, priority: i32) -> Result<(), i32> {
    // cite: bionic pthread_setschedparam.cpp#pthread_setschedparam
    match policy {
        SCHED_OTHER => {
            if priority == 0 {
                Ok(())
            } else {
                Err(EINVAL)
            }
        }
        SCHED_BATCH | SCHED_IDLE => {
            host_log::line(&format!("FIXME: scheduling policy {policy} has no host equivalent; next missing primitive: SCHED_BATCH and SCHED_IDLE thread policies"));
            Err(EINVAL)
        }
        SCHED_FIFO | SCHED_RR => {
            if !(1..=LINUX_PRIORITY_MAX).contains(&priority) {
                return Err(EINVAL);
            }
            let darwin = host_policy(policy).ok_or(EINVAL)?;
            let (low, high) = unsafe {
                (
                    libc::sched_get_priority_min(darwin),
                    libc::sched_get_priority_max(darwin),
                )
            };
            let scaled = low + (priority - 1) * (high - low) / (LINUX_PRIORITY_MAX - 1);
            let mut parameter: libc::sched_param = unsafe { std::mem::zeroed() };
            parameter.sched_priority = scaled;
            match unsafe { libc::pthread_setschedparam(host, darwin, &parameter) } {
                0 => Ok(()),
                code => Err(linux_from_darwin(code).unwrap_or(EINVAL)),
            }
        }
        _ => Err(EINVAL),
    }
}

pub fn get_host(host: libc::pthread_t) -> Result<(i32, i32), i32> {
    // cite: bionic pthread_getschedparam.cpp#pthread_getschedparam
    let mut darwin = 0;
    let mut parameter: libc::sched_param = unsafe { std::mem::zeroed() };
    match unsafe { libc::pthread_getschedparam(host, &mut darwin, &mut parameter) } {
        0 => {}
        code => return Err(linux_from_darwin(code).unwrap_or(EINVAL)),
    }
    let (low, high) = unsafe {
        (
            libc::sched_get_priority_min(darwin),
            libc::sched_get_priority_max(darwin),
        )
    };
    Ok(match darwin {
        libc::SCHED_FIFO | libc::SCHED_RR if high > low => {
            let policy = if darwin == libc::SCHED_FIFO {
                SCHED_FIFO
            } else {
                SCHED_RR
            };
            (
                policy,
                1 + (parameter.sched_priority - low) * (LINUX_PRIORITY_MAX - 1) / (high - low),
            )
        }
        _ => (SCHED_OTHER, 0),
    })
}

pub fn priority_limit(policy: i32, maximum: bool) -> Option<i32> {
    // cite: bionic SYSCALLS.TXT#sched_get_priority_max, sched_get_priority_min
    match policy {
        SCHED_OTHER | SCHED_BATCH | SCHED_IDLE => Some(0),
        SCHED_FIFO | SCHED_RR => Some(if maximum { LINUX_PRIORITY_MAX } else { 1 }),
        _ => None,
    }
}
