use super::super::bridge::CallContext;
use super::super::{guest_thread, thread_list};
use super::abi_types::{EFAULT, EINVAL};
use super::current_thread;
use super::export::in_libc;
use super::fs_result::{linux_errno_of, missing};
use super::guest_mem::{read_value, write_value};
use super::sysinfo::sysctl_u64;

const ESRCH: i32 = 3;
const ECHILD: i32 = 10;

// cite: bionic resource.h#RLIMIT_CPU, RLIMIT_NOFILE, RLIMIT_AS, RLIM_NLIMITS, RLIM_INFINITY
const RESOURCE_COUNT: i32 = 16;
const GUEST_INFINITY: u64 = u64::MAX;

// cite: bionic resource.h#PRIO_MIN, PRIO_MAX, PRIO_PROCESS, PRIO_PGRP, PRIO_USER
const NICE_MIN: i32 = -20;
const NICE_MAX: i32 = 19;
const PRIO_PROCESS: i32 = 0;
const PRIO_PGRP: i32 = 1;
const PRIO_USER: i32 = 2;

const DARWIN_ITIMER_MAX_SECONDS: u32 = 100_000_000;

fn host_resource(resource: i32) -> Result<libc::c_int, i32> {
    if !(0..RESOURCE_COUNT).contains(&resource) {
        return Err(EINVAL);
    }
    Ok(match resource {
        0 => libc::RLIMIT_CPU,
        1 => libc::RLIMIT_FSIZE,
        2 => libc::RLIMIT_DATA,
        3 => libc::RLIMIT_STACK,
        4 => libc::RLIMIT_CORE,
        6 => libc::RLIMIT_NPROC,
        7 => libc::RLIMIT_NOFILE,
        8 => libc::RLIMIT_MEMLOCK,
        9 => libc::RLIMIT_AS,
        other => {
            missing(
                &format!("rlimit resource {other}"),
                "Darwin has no equivalent resource limit",
            );
            return Err(EINVAL);
        }
    })
}

fn guest_limit(resource: libc::c_int, host: u64) -> u64 {
    let host = match sysctl_u64("kern.maxfilesperproc") {
        Some(ceiling) if resource == libc::RLIMIT_NOFILE => host.min(ceiling),
        _ => host,
    };
    if host == libc::RLIM_INFINITY {
        GUEST_INFINITY
    } else {
        host
    }
}

fn host_status(status: libc::c_int) -> Result<(), i32> {
    if status == 0 {
        Ok(())
    } else {
        Err(linux_errno_of(unsafe { *libc::__error() }))
    }
}

fn get_limit(resource: i32, address: usize) -> Result<(), i32> {
    let host = host_resource(resource)?;
    if address == 0 {
        return Err(EFAULT);
    }
    let mut limit: libc::rlimit = unsafe { std::mem::zeroed() };
    host_status(unsafe { libc::getrlimit(host, &mut limit) })?;
    write_value::<u64>(address, guest_limit(host, limit.rlim_cur));
    write_value::<u64>(address + 8, guest_limit(host, limit.rlim_max));
    Ok(())
}

fn set_limit(resource: i32, address: usize) -> Result<(), i32> {
    let host = host_resource(resource)?;
    if address == 0 {
        return Err(EFAULT);
    }
    let limit = libc::rlimit {
        rlim_cur: read_value::<u64>(address).min(libc::RLIM_INFINITY),
        rlim_max: read_value::<u64>(address + 8).min(libc::RLIM_INFINITY),
    };
    host_status(unsafe { libc::setrlimit(host, &limit) })
}

fn priority_target(which: i32, who: i32) -> Result<usize, i32> {
    match which {
        PRIO_PROCESS => {}
        PRIO_PGRP | PRIO_USER => {
            missing(
                "process group and user priority",
                "only single thread priorities are translated",
            );
            return Err(EINVAL);
        }
        _ => return Err(EINVAL),
    }
    if who == 0 {
        return Ok(current_thread::record());
    }
    guest_thread::live_records()
        .into_iter()
        .find(|(_, tid)| *tid == who)
        .map(|(record, _)| record)
        .ok_or(ESRCH)
}

fn darwin_priority(nice: i32) -> i32 {
    let (low, high) = unsafe {
        (
            libc::sched_get_priority_min(libc::SCHED_OTHER),
            libc::sched_get_priority_max(libc::SCHED_OTHER),
        )
    };
    let normal = (low + high) / 2;
    if nice <= 0 {
        normal + nice * (high - normal) / NICE_MIN
    } else {
        normal - nice * (normal - low) / NICE_MAX
    }
}

fn apply_nice(host: libc::pthread_t, nice: i32) -> Result<(), i32> {
    let mut policy = 0;
    let mut parameter: libc::sched_param = unsafe { std::mem::zeroed() };
    let status = unsafe { libc::pthread_getschedparam(host, &mut policy, &mut parameter) };
    if status != 0 {
        return Err(linux_errno_of(status));
    }
    if policy != libc::SCHED_OTHER {
        return Ok(());
    }
    parameter.sched_priority = darwin_priority(nice);
    match unsafe { libc::pthread_setschedparam(host, policy, &parameter) } {
        0 => Ok(()),
        code => Err(linux_errno_of(code)),
    }
}

fn set_priority(which: i32, who: i32, priority: i32) -> Result<(), i32> {
    let record = priority_target(which, who)?;
    let nice = priority.clamp(NICE_MIN, NICE_MAX);
    apply_nice(thread_list::host_of(record).ok_or(ESRCH)?, nice)?;
    thread_list::set_nice(record, nice);
    Ok(())
}

pub fn inherit_priority(parent: usize, child: usize, host: libc::pthread_t) -> Result<(), i32> {
    let nice = thread_list::nice_of(parent);
    if nice == 0 {
        return Ok(());
    }
    thread_list::set_nice(child, nice);
    apply_nice(host, nice)
}

fn finish(context: &mut CallContext, outcome: Result<i32, i32>) {
    match outcome {
        Ok(value) => context.ret_i32(value),
        Err(code) => {
            context.set_errno(code);
            context.ret_i32(-1);
        }
    }
}

fn alarm(context: &mut CallContext) {
    // cite: bionic alarm.c#alarm
    let seconds = context.arg_u32(0);
    if seconds > DARWIN_ITIMER_MAX_SECONDS {
        context.fail("alarm beyond the Darwin interval timer limit; next missing primitive: alarm over 100000000 seconds");
        return;
    }
    let mut next: libc::itimerval = unsafe { std::mem::zeroed() };
    next.it_value.tv_sec = seconds as libc::time_t;
    let mut previous: libc::itimerval = unsafe { std::mem::zeroed() };
    if let Err(code) =
        host_status(unsafe { libc::setitimer(libc::ITIMER_REAL, &next, &mut previous) })
    {
        context.set_errno(code);
        context.ret_u32(u32::MAX);
        return;
    }
    let partial = u32::from(previous.it_value.tv_usec != 0);
    context.ret_u32(previous.it_value.tv_sec as u32 + partial);
}

pub fn register() {
    for name in ["getrlimit", "getrlimit64"] {
        in_libc(name, |context| {
            let outcome = get_limit(context.arg_i32(0), context.arg_ptr(1));
            finish(context, outcome.map(|()| 0));
        });
    }
    for name in ["setrlimit", "setrlimit64"] {
        in_libc(name, |context| {
            let outcome = set_limit(context.arg_i32(0), context.arg_ptr(1));
            finish(context, outcome.map(|()| 0));
        });
    }
    in_libc("setpriority", |context| {
        // cite: bionic SYSCALLS.TXT#setpriority
        let outcome = set_priority(context.arg_i32(0), context.arg_i32(1), context.arg_i32(2));
        finish(context, outcome.map(|()| 0));
    });
    in_libc("getpriority", |context| {
        // cite: bionic getpriority.cpp#getpriority
        let outcome = priority_target(context.arg_i32(0), context.arg_i32(1));
        finish(context, outcome.map(thread_list::nice_of));
    });
    in_libc("alarm", alarm);
    in_libc("waitpid", |context| {
        // cite: bionic wait.cpp#waitpid
        finish(context, Err(ECHILD));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::bridge::handler_address;
    use crate::native::bridge_test_support::register_all;
    use crate::native::call_assign::CallBuilder;

    fn shim(name: &str) -> CallBuilder {
        register_all();
        CallBuilder::new(
            handler_address(name).unwrap_or_else(|| panic!("{name} is not registered")),
        )
    }

    fn outcome(call: CallBuilder) -> (i32, i32) {
        let value = call.invoke().unwrap().i32();
        let errno = unsafe { *libc::__error() };
        (value, if value == -1 { errno } else { 0 })
    }

    fn limit(resource: i32) -> [u64; 2] {
        let mut limit = [0u64; 2];
        let address = limit.as_mut_ptr() as usize;
        assert_eq!(
            outcome(shim("getrlimit").i32(resource).ptr(address)),
            (0, 0)
        );
        limit
    }

    fn set(resource: i32, limit: [u64; 2]) -> (i32, i32) {
        outcome(shim("setrlimit").i32(resource).ptr(limit.as_ptr() as usize))
    }

    #[test]
    fn linux_resource_numbers_and_infinity_are_translated_to_the_darwin_limits() {
        // cite: bionic resource.h#RLIMIT_NOFILE, RLIMIT_CORE, RLIMIT_AS, RLIM_INFINITY, RLIM_NLIMITS
        let [soft, hard] = limit(7);
        let ceiling = sysctl_u64("kern.maxfilesperproc").unwrap();
        assert!(soft <= hard && hard <= ceiling, "{soft} {hard} {ceiling}");
        assert_eq!(limit(9), [u64::MAX; 2]);
        assert_eq!(outcome(shim("getrlimit64").i32(7).ptr(0)), (-1, 14));
        for resource in [5, 10, 16, -1] {
            assert_eq!(
                outcome(shim("getrlimit").i32(resource).ptr(0)),
                (-1, 22),
                "resource {resource}"
            );
        }

        let core = limit(4);
        assert_eq!(set(4, [0, core[1]]), (0, 0));
        assert_eq!(limit(4), [0, core[1]]);
        assert_eq!(set(4, core), (0, 0));
        assert_eq!(set(7, [hard, hard - 1]), (-1, 22));
    }

    #[test]
    fn nice_values_are_per_thread_clamped_and_move_the_darwin_priority() {
        // cite: bionic resource.h#PRIO_MIN, PRIO_MAX, PRIO_PROCESS, PRIO_USER
        std::thread::spawn(|| {
            let host_priority = || {
                let mut policy = 0;
                let mut parameter: libc::sched_param = unsafe { std::mem::zeroed() };
                unsafe {
                    libc::pthread_getschedparam(libc::pthread_self(), &mut policy, &mut parameter)
                };
                parameter.sched_priority
            };
            let (low, high) = unsafe {
                (
                    libc::sched_get_priority_min(libc::SCHED_OTHER),
                    libc::sched_get_priority_max(libc::SCHED_OTHER),
                )
            };
            let nice = |value: i32| outcome(shim("setpriority").i32(0).i32(0).i32(value));
            let current = || outcome(shim("getpriority").i32(0).i32(0));

            assert_eq!(current(), (0, 0));
            assert_eq!(nice(40), (0, 0));
            assert_eq!((current().0, host_priority()), (19, low));
            assert_eq!(nice(-100), (0, 0));
            assert_eq!((current().0, host_priority()), (-20, high));
            assert_eq!(nice(0), (0, 0));
            assert_eq!(host_priority(), (low + high) / 2);
            assert_eq!(
                outcome(shim("setpriority").i32(0).i32(999_999).i32(0)),
                (-1, 3)
            );
            assert_eq!(outcome(shim("setpriority").i32(3).i32(0).i32(0)), (-1, 22));
            assert_eq!(outcome(shim("getpriority").i32(2).i32(0)), (-1, 22));
        })
        .join()
        .unwrap();
    }

    #[test]
    fn alarm_returns_the_remaining_seconds_rounded_up() {
        // cite: bionic alarm.c#alarm
        let alarm = |seconds: u32| shim("alarm").u32(seconds).invoke().unwrap().i32();

        assert_eq!(alarm(100), 0);
        assert_eq!(alarm(0), 100);
        assert_eq!(alarm(0), 0);
    }

    #[test]
    fn a_process_that_cannot_create_children_has_none_to_wait_for() {
        // cite: bionic wait.cpp#waitpid
        assert_eq!(outcome(shim("waitpid").i32(-1).ptr(0).i32(0)), (-1, 10));
    }

    #[test]
    fn sysinfo_fills_the_aarch64_layout_and_nothing_past_it() {
        // cite: bionic sysinfo.h#sysinfo
        let mut block = [0xAAu8; 128];
        let address = block.as_mut_ptr() as usize;
        assert_eq!(outcome(shim("sysinfo").ptr(address)), (0, 0));
        let word =
            |offset: usize| u64::from_le_bytes(block[offset..offset + 8].try_into().unwrap());

        assert!(word(0) > 0);
        assert_eq!(word(32), sysctl_u64("hw.memsize").unwrap());
        assert!(word(40) <= word(32));
        assert_eq!((word(48), word(56), word(88), word(96)), (0, 0, 0, 0));
        assert!(u16::from_le_bytes([block[80], block[81]]) >= 1);
        assert_eq!(block[104..108], 1u32.to_le_bytes());
        assert_eq!(block[112..], [0xAA; 16]);
        assert_eq!(outcome(shim("sysinfo").ptr(0)), (-1, 14));
    }
}
