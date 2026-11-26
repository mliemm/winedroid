use std::sync::Mutex;

use super::super::bridge::CallContext;
use super::super::guest_thread;
use super::super::host_log;
use super::super::signal_numbers::{
    self, bit, darwin_to_linux, filter_reserved, is_valid, is_virtual, linux_to_darwin,
    to_darwin_mask, to_linux_mask, SIGKILL, SIGSTOP,
};
use super::super::signal_router;
use super::super::signal_state::{
    self, action_of, alternate, mark_thread_directed, publish_action, raise_pending, set_alternate,
    set_mask, Action, AlternateStack, SA_RESTART, SS_DISABLE, SS_ONSTACK,
};
use super::super::thread_list;
use super::abi_types::EINVAL;
use super::current_thread;
use super::export::in_libc;
use super::guest_mem::{read_value, write_value};

const ESRCH: i32 = 3;
const EPERM: i32 = 1;
const ENOMEM: i32 = 12;
const ENOSYS: i32 = 38;
const EFAULT: i32 = 14;
const EINTR: i32 = 4;
const MIN_ALTERNATE_STACK: u64 = 5120;
const SIG_BLOCK: i32 = 0;
const SIG_UNBLOCK: i32 = 1;
const SIG_SETMASK: i32 = 2;
const SIG_ERR: u64 = u64::MAX;

static ACTION_LOCK: Mutex<()> = Mutex::new(());

fn fail(context: &mut CallContext, code: i32) {
    context.set_errno(code);
    context.ret_i32(-1);
}

fn set_operation(context: &mut CallContext, change: impl Fn(u64, i32) -> u64) {
    let (set, signal) = (context.arg_ptr(0), context.arg_i32(1));
    if set == 0 || !is_valid(signal) {
        return fail(context, EINVAL);
    }
    write_value::<u64>(set, change(read_value(set), signal));
    context.ret_i32(0);
}

fn write_action(address: usize, action: &Action) {
    write_value::<u32>(address, action.flags);
    write_value::<u64>(address + 8, action.handler);
    write_value::<u64>(address + 16, action.mask);
    write_value::<u64>(address + 24, action.restorer);
}

fn read_action(address: usize) -> Action {
    Action {
        flags: read_value(address),
        handler: read_value(address + 8),
        mask: read_value(address + 16),
        restorer: read_value(address + 24),
    }
}

pub fn change_action(signal: i32, new: Option<Action>, old: Option<usize>) -> Result<(), i32> {
    // cite: bionic sigaction.cpp#sigaction
    if !is_valid(signal) || (new.is_some() && matches!(signal, SIGKILL | SIGSTOP)) {
        return Err(EINVAL);
    }
    let _guard = ACTION_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(old) = old {
        write_action(old, &action_of(signal));
    }
    if let Some(mut action) = new {
        action.mask &= !(bit(SIGKILL) | bit(SIGSTOP));
        publish_action(signal, action);
        signal_router::install(signal, &action)?;
    }
    Ok(())
}

fn process_mask(how: i32, new: Option<u64>, old: Option<usize>) -> Result<(), i32> {
    // cite: bionic sigprocmask.cpp#sigprocmask64
    if new.is_some() && !matches!(how, SIG_BLOCK | SIG_UNBLOCK | SIG_SETMASK) {
        return Err(EINVAL);
    }
    let current = signal_state::mask();
    if let Some(old) = old {
        write_value::<u64>(old, current);
    }
    let Some(requested) = new else { return Ok(()) };
    let requested = filter_reserved(requested, how);
    let result = match how {
        SIG_BLOCK => current | requested,
        SIG_UNBLOCK => current & !requested,
        _ => requested,
    };
    set_mask(result);
    signal_router::apply_darwin_mask(signal_state::mask());
    Ok(())
}

fn suspend(temporary: u64) {
    // cite: bionic signal.cpp#sigsuspend64
    let previous = signal_state::mask();
    set_mask(temporary);
    let host: libc::sigset_t = to_darwin_mask(signal_state::mask());
    while signal_state::pending() & !signal_state::mask() == 0 {
        unsafe { libc::sigsuspend(&host) };
    }
    signal_router::deliver_pending();
    set_mask(previous);
    signal_router::apply_darwin_mask(signal_state::mask());
}

fn tid_to_record(tid: i32) -> Option<usize> {
    guest_thread::live_records()
        .into_iter()
        .find(|(_, candidate)| *candidate == tid)
        .map(|(record, _)| record)
}

fn send_to_thread(record: usize, signal: i32) -> i32 {
    if signal == 0 {
        return 0;
    }
    if !is_valid(signal) {
        return EINVAL;
    }
    if matches!(signal, SIGKILL | SIGSTOP) {
        host_log::line(&format!("FIXME: signal {signal} would stop the host process; next missing primitive: guest directed SIGKILL and SIGSTOP"));
        return EPERM;
    }
    let myself = record == current_thread::record();
    if is_virtual(signal) {
        if !myself {
            host_log::line(&format!("FIXME: signal {signal} has no host number and cannot cross threads; next missing primitive: cross thread delivery of virtual signal {signal}"));
            return ENOSYS;
        }
        mark_thread_directed(signal);
        raise_pending(signal);
        return 0;
    }
    let (Some(host), Some(darwin)) = (thread_list::host_of(record), linux_to_darwin(signal)) else {
        return ESRCH;
    };
    if myself {
        mark_thread_directed(signal);
    }
    unsafe { libc::pthread_kill(host, darwin) }
}

pub fn tgkill(group: i32, tid: i32, signal: i32) -> i64 {
    // cite: bionic SYSCALLS.TXT#tgkill
    if group <= 0 || tid <= 0 || !(0..=signal_numbers::SIGNAL_COUNT).contains(&signal) {
        return -i64::from(EINVAL);
    }
    if group != unsafe { libc::getpid() } {
        return -i64::from(ESRCH);
    }
    match tid_to_record(tid) {
        Some(record) => -i64::from(send_to_thread(record, signal)),
        None => -i64::from(ESRCH),
    }
}

fn kill_process(pid: i32, signal: i32) -> i32 {
    // cite: bionic killpg.cpp#killpg
    if !(0..=signal_numbers::SIGNAL_COUNT).contains(&signal) {
        return EINVAL;
    }
    if pid != unsafe { libc::getpid() } {
        return if signal == 0 && pid > 0 && unsafe { libc::kill(pid, 0) } == 0 {
            0
        } else {
            EPERM
        };
    }
    if signal == 0 {
        return 0;
    }
    if matches!(signal, SIGKILL | SIGSTOP) {
        return EPERM;
    }
    match linux_to_darwin(signal) {
        Some(darwin) => unsafe { libc::kill(pid, darwin) },
        None => send_to_thread(current_thread::record(), signal),
    }
}

fn alternate_stack(context: &mut CallContext) {
    // cite: bionic SYSCALLS.TXT#sigaltstack
    let (new, old) = (context.arg_ptr(0), context.arg_ptr(1));
    let current = alternate().unwrap_or(AlternateStack {
        base: 0,
        size: 0,
        disabled: true,
    });
    let sp = signal_router::current_stack_pointer();
    let on_stack = !current.disabled && (current.base..current.base + current.size).contains(&sp);
    if old != 0 {
        write_value::<u64>(old, current.base);
        write_value::<i32>(
            old + 8,
            if current.disabled {
                SS_DISABLE
            } else if on_stack {
                SS_ONSTACK
            } else {
                0
            },
        );
        write_value::<u64>(old + 16, current.size);
    }
    if new == 0 {
        return context.ret_i32(0);
    }
    if on_stack {
        return fail(context, EPERM);
    }
    let (base, flags, size): (u64, i32, u64) =
        (read_value(new), read_value(new + 8), read_value(new + 16));
    if flags & !SS_DISABLE != 0 {
        return fail(context, EINVAL);
    }
    if flags & SS_DISABLE != 0 {
        set_alternate(AlternateStack {
            base: 0,
            size: 0,
            disabled: true,
        });
        return context.ret_i32(0);
    }
    if size < MIN_ALTERNATE_STACK {
        return fail(context, ENOMEM);
    }
    set_alternate(AlternateStack {
        base,
        size,
        disabled: false,
    });
    context.ret_i32(0);
}

fn register_set_operations() {
    for suffix in ["", "64"] {
        in_libc(&format!("sigemptyset{suffix}"), |context| {
            // cite: bionic signal.cpp#sigemptyset
            let set = context.arg_ptr(0);
            if set == 0 {
                return fail(context, EINVAL);
            }
            write_value::<u64>(set, 0);
            context.ret_i32(0);
        });
        in_libc(&format!("sigfillset{suffix}"), |context| {
            // cite: bionic signal.cpp#sigfillset
            let set = context.arg_ptr(0);
            if set == 0 {
                return fail(context, EINVAL);
            }
            write_value::<u64>(set, u64::MAX);
            context.ret_i32(0);
        });
        in_libc(&format!("sigaddset{suffix}"), |context| {
            set_operation(context, |set, signal| set | bit(signal))
        });
        in_libc(&format!("sigdelset{suffix}"), |context| {
            set_operation(context, |set, signal| set & !bit(signal))
        });
        in_libc(&format!("sigismember{suffix}"), |context| {
            // cite: bionic signal.cpp#sigismember
            let (set, signal) = (context.arg_ptr(0), context.arg_i32(1));
            if set == 0 || !is_valid(signal) {
                return fail(context, EINVAL);
            }
            context.ret_i32(i32::from(read_value::<u64>(set) & bit(signal) != 0));
        });
    }
}

fn register_actions() {
    for name in ["sigaction", "sigaction64"] {
        in_libc(name, |context| {
            let (act, old) = (context.arg_ptr(1), context.arg_ptr(2));
            let new = (act != 0).then(|| read_action(act));
            match change_action(context.arg_i32(0), new, (old != 0).then_some(old)) {
                Ok(()) => context.ret_i32(0),
                Err(code) => fail(context, code),
            }
        });
    }
    in_libc("signal", |context| {
        // cite: bionic signal.cpp#_signal
        let signal = context.arg_i32(0);
        let action = Action {
            flags: SA_RESTART,
            handler: context.arg_u64(1),
            mask: 0,
            restorer: 0,
        };
        let previous = action_of(signal);
        match change_action(signal, Some(action), None) {
            Ok(()) => context.ret_u64(previous.handler),
            Err(_) => {
                context.set_errno(EINVAL);
                context.ret_u64(SIG_ERR);
            }
        }
    });
}

fn register_masks() {
    for name in ["sigprocmask", "sigprocmask64"] {
        in_libc(name, |context| {
            let (new, old) = (context.arg_ptr(1), context.arg_ptr(2));
            match process_mask(
                context.arg_i32(0),
                (new != 0).then(|| read_value(new)),
                (old != 0).then_some(old),
            ) {
                Ok(()) => context.ret_i32(0),
                Err(code) => fail(context, code),
            }
        });
    }
    for name in ["pthread_sigmask", "pthread_sigmask64"] {
        in_libc(name, |context| {
            // cite: bionic signal.cpp#pthread_sigmask64
            let (new, old) = (context.arg_ptr(1), context.arg_ptr(2));
            let outcome = process_mask(
                context.arg_i32(0),
                (new != 0).then(|| read_value(new)),
                (old != 0).then_some(old),
            );
            context.ret_i32(outcome.err().unwrap_or(0));
        });
    }
    for name in ["sigpending", "sigpending64"] {
        in_libc(name, |context| {
            // cite: bionic signal.cpp#sigpending
            let mut host: libc::sigset_t = 0;
            unsafe { libc::sigpending(&mut host) };
            write_value::<u64>(
                context.arg_ptr(0),
                to_linux_mask(host) | (signal_state::pending() & signal_state::mask()),
            );
            context.ret_i32(0);
        });
    }
    for name in ["sigsuspend", "sigsuspend64"] {
        in_libc(name, |context| {
            // cite: bionic signal.cpp#sigsuspend64
            let set = context.arg_ptr(0);
            if set == 0 {
                return fail(context, EFAULT);
            }
            suspend(filter_reserved(read_value(set), SIG_SETMASK));
            fail(context, EINTR);
        });
    }
    in_libc("sigwait", |context| {
        // cite: bionic signal.cpp#sigwait64
        let set: u64 = read_value(context.arg_ptr(0));
        let waitable = to_darwin_mask(set);
        if waitable == 0 {
            context.fail("sigwait on a set with no host signal; next missing primitive: sigwait on virtual signals");
            return;
        }
        let mut received = 0;
        let status = unsafe { libc::sigwait(&waitable, &mut received) };
        if status == 0 {
            write_value::<i32>(
                context.arg_ptr(1),
                darwin_to_linux(received).unwrap_or(received),
            );
        }
        context.ret_i32(status);
    });
}

fn register_delivery() {
    in_libc("sigaltstack", alternate_stack);
    in_libc("__libc_current_sigrtmin", |context| {
        context.ret_i32(signal_numbers::rt_min())
    });
    in_libc("__libc_current_sigrtmax", |context| {
        context.ret_i32(signal_numbers::rt_max())
    });
    in_libc("raise", |context| {
        // cite: bionic raise.cpp#raise
        let outcome = tgkill(
            unsafe { libc::getpid() },
            current_thread::tid(),
            context.arg_i32(0),
        );
        if outcome < 0 {
            return fail(context, (-outcome) as i32);
        }
        signal_router::deliver_pending();
        context.ret_i32(0);
    });
    in_libc("tgkill", |context| {
        let outcome = tgkill(context.arg_i32(0), context.arg_i32(1), context.arg_i32(2));
        if outcome < 0 {
            return fail(context, (-outcome) as i32);
        }
        signal_router::deliver_pending();
        context.ret_i32(0);
    });
    in_libc("kill", |context| {
        match kill_process(context.arg_i32(0), context.arg_i32(1)) {
            0 => {
                signal_router::deliver_pending();
                context.ret_i32(0);
            }
            code => fail(context, code),
        }
    });
    in_libc("killpg", |context| {
        // cite: bionic killpg.cpp#killpg
        let group = context.arg_i32(0);
        match if group < 0 {
            EINVAL
        } else {
            kill_process(-group, context.arg_i32(1))
        } {
            0 => context.ret_i32(0),
            code => fail(context, code),
        }
    });
    in_libc("pthread_kill", |context| {
        // cite: bionic pthread_kill.cpp#pthread_kill
        let Some(record) = thread_list::find(context.arg_ptr(0), "pthread_kill") else {
            return context.ret_i32(ESRCH);
        };
        let tid = guest_thread::tid_of(record).unwrap_or(0);
        if tid == 0 {
            return context.ret_i32(ESRCH);
        }
        let code = send_to_thread(record, context.arg_i32(1));
        if code == 0 {
            signal_router::deliver_pending();
        }
        context.ret_i32(code);
    });
}

pub fn register() {
    register_set_operations();
    register_actions();
    register_masks();
    register_delivery();
}
