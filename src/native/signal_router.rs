use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Mutex;

use super::bionic::guest_mem::write_value;
use super::call_assign::CallBuilder;
use super::data_fault;
use super::guest_thread::current_record;
use super::host_log;
use super::signal_frame::{
    self, Delivery, StackRecord, FRAME_BYTES, SIGINFO_OFFSET, SI_TKILL, UCONTEXT_OFFSET,
};
use super::signal_numbers::{
    bit, is_synchronous, linux_to_darwin, to_darwin_mask, SIGKILL, SIGSTOP,
};
use super::signal_state::{
    action_of, alternate, mask, publish_action, raise_pending, set_mask, take_deliverable,
    take_thread_directed, Action, DEFAULT_ACTION, SA_NOCLDSTOP, SA_NOCLDWAIT, SA_NODEFER,
    SA_ONSTACK, SA_RESETHAND, SA_RESTART, SS_DISABLE, SS_ONSTACK,
};
use super::thread_list;

const DARWIN_SIGNALS: usize = 32;

static PREVIOUS: [AtomicPtr<libc::sigaction>; DARWIN_SIGNALS] =
    [const { AtomicPtr::new(std::ptr::null_mut()) }; DARWIN_SIGNALS];
static INSTALL: Mutex<()> = Mutex::new(());

fn router_address() -> usize {
    router as extern "C" fn(libc::c_int, *mut libc::siginfo_t, *mut libc::c_void) as usize
}

unsafe fn chain(signal: libc::c_int, info: *mut libc::siginfo_t, context: *mut libc::c_void) {
    let previous = PREVIOUS[signal as usize].load(Ordering::Acquire);
    let Some(previous) = previous.as_ref() else {
        return restore_default(signal);
    };
    if previous.sa_sigaction == libc::SIG_DFL || previous.sa_sigaction == libc::SIG_IGN {
        return restore_default(signal);
    }
    if previous.sa_flags & libc::SA_SIGINFO != 0 {
        let handler: extern "C" fn(libc::c_int, *mut libc::siginfo_t, *mut libc::c_void) =
            std::mem::transmute(previous.sa_sigaction);
        handler(signal, info, context);
    } else {
        let handler: extern "C" fn(libc::c_int) = std::mem::transmute(previous.sa_sigaction);
        handler(signal);
    }
}

fn restore_default(signal: libc::c_int) {
    let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
    action.sa_sigaction = libc::SIG_DFL;
    unsafe { libc::sigaction(signal, &action, std::ptr::null_mut()) };
}

fn forward_to_attached_thread(darwin_signal: libc::c_int) {
    if let Some(host) = thread_list::try_other_host() {
        unsafe { libc::pthread_kill(host, darwin_signal) };
    }
}

extern "C" fn router(
    darwin_signal: libc::c_int,
    info: *mut libc::siginfo_t,
    context: *mut libc::c_void,
) {
    let Some(delivery) = signal_frame::classify(darwin_signal, info as usize, context as usize)
    else {
        return unsafe { chain(darwin_signal, info, context) };
    };
    let action = action_of(delivery.signal);
    if !action.has_handler() {
        return unsafe { chain(darwin_signal, info, context) };
    }
    if delivery.fault {
        if !super::signal_state::in_guest() || data_fault::is_reserved(delivery.address as usize) {
            return unsafe { chain(darwin_signal, info, context) };
        }
        return deliver(&delivery, Some(context as usize));
    }
    if current_record().is_none() {
        return forward_to_attached_thread(darwin_signal);
    }
    if super::signal_state::in_guest() {
        deliver(&delivery, Some(context as usize));
    } else {
        raise_pending(delivery.signal);
    }
}

fn apply_host(darwin: i32, action: &Action) -> Result<(), i32> {
    let _guard = INSTALL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let current = PREVIOUS[darwin as usize].load(Ordering::Acquire);
    if !action.has_handler() {
        let mut restore: libc::sigaction = unsafe { std::mem::zeroed() };
        match unsafe { current.as_ref() } {
            Some(previous) => restore = *previous,
            None => {
                restore.sa_sigaction = if action.handler == 1 {
                    libc::SIG_IGN
                } else {
                    libc::SIG_DFL
                }
            }
        }
        if action.handler == 1
            && !is_synchronous(super::signal_numbers::darwin_to_linux(darwin).unwrap_or(0))
        {
            restore.sa_sigaction = libc::SIG_IGN;
        }
        return match unsafe { libc::sigaction(darwin, &restore, std::ptr::null_mut()) } {
            0 => Ok(()),
            _ => Err(unsafe { *libc::__error() }),
        };
    }
    let mut host: libc::sigaction = unsafe { std::mem::zeroed() };
    host.sa_sigaction = router_address();
    host.sa_flags = libc::SA_SIGINFO | libc::SA_ONSTACK;
    for (guest, flag) in [
        (SA_RESTART, libc::SA_RESTART),
        (SA_NODEFER, libc::SA_NODEFER),
        (SA_NOCLDSTOP, libc::SA_NOCLDSTOP),
        (SA_NOCLDWAIT, libc::SA_NOCLDWAIT),
    ] {
        if action.flags & guest != 0 {
            host.sa_flags |= flag;
        }
    }
    host.sa_mask = to_darwin_mask(action.mask);
    let mut previous: Box<libc::sigaction> = Box::new(unsafe { std::mem::zeroed() });
    if unsafe { libc::sigaction(darwin, &host, &mut *previous) } != 0 {
        return Err(unsafe { *libc::__error() });
    }
    if current.is_null() && previous.sa_sigaction != router_address() {
        PREVIOUS[darwin as usize].store(Box::into_raw(previous), Ordering::Release);
    }
    Ok(())
}

pub fn install(signal: i32, action: &Action) -> Result<(), i32> {
    // cite: bionic sigaction.cpp#sigaction
    let Some(darwin) = linux_to_darwin(signal) else {
        return Ok(());
    };
    apply_host(darwin, action)?;
    if matches!(signal, 7 | 11) {
        apply_host(
            if darwin == libc::SIGBUS {
                libc::SIGSEGV
            } else {
                libc::SIGBUS
            },
            action,
        )?;
    }
    Ok(())
}

fn current_frame_base(context: Option<usize>, action: &Action) -> Option<usize> {
    let context = context?;
    let mut base = signal_frame::interrupted_sp(context);
    if action.flags & SA_ONSTACK != 0 {
        if let Some(stack) = alternate().filter(|stack| !stack.disabled) {
            if !(stack.base..stack.base + stack.size).contains(&base) {
                base = stack.base + stack.size;
            }
        }
    }
    Some((base as usize - FRAME_BYTES) & !15)
}

fn stack_record(on_alternate: bool) -> StackRecord {
    match alternate().filter(|stack| !stack.disabled) {
        Some(stack) => StackRecord {
            base: stack.base,
            size: stack.size,
            flags: if on_alternate { SS_ONSTACK } else { 0 },
        },
        None => StackRecord {
            base: 0,
            size: 0,
            flags: SS_DISABLE,
        },
    }
}

fn on_alternate_stack(sp: u64) -> bool {
    alternate()
        .is_some_and(|stack| !stack.disabled && (stack.base..stack.base + stack.size).contains(&sp))
}

fn alternate_top(action: &Action) -> Option<usize> {
    let stack = alternate().filter(|stack| !stack.disabled && action.flags & SA_ONSTACK != 0)?;
    Some(((stack.base + stack.size) as usize - FRAME_BYTES) & !15)
}

pub fn current_stack_pointer() -> u64 {
    let value: u64;
    unsafe {
        std::arch::asm!("mov {}, sp", out(reg) value, options(nomem, nostack, preserves_flags))
    };
    value
}

fn deliver(delivery: &Delivery, context: Option<usize>) {
    let signal = delivery.signal;
    let action = action_of(signal);
    if action.flags & SA_RESETHAND != 0 {
        publish_action(signal, DEFAULT_ACTION);
        let _ = install(signal, &DEFAULT_ACTION);
    }
    let heap: Vec<u128>;
    let (frame, handler_stack) = match current_frame_base(context, &action) {
        Some(base) => (base, Some(base)),
        None => {
            heap = vec![0u128; FRAME_BYTES.div_ceil(16)];
            let on_alternate = on_alternate_stack(current_stack_pointer());
            (
                heap.as_ptr() as usize,
                if on_alternate {
                    None
                } else {
                    alternate_top(&action)
                },
            )
        }
    };
    let handler_sp = handler_stack.map_or(current_stack_pointer(), |top| top as u64);
    let saved_mask = mask();
    signal_frame::write_siginfo(frame + SIGINFO_OFFSET, delivery);
    signal_frame::write_ucontext(
        frame + UCONTEXT_OFFSET,
        saved_mask,
        &stack_record(on_alternate_stack(handler_sp)),
        context,
        if delivery.fault { delivery.address } else { 0 },
        delivery.fault,
    );
    if let Some(context) = context {
        let source = signal_frame::mcontext(context);
        write_value::<u64>(
            frame,
            super::bionic::guest_mem::read_value::<u64>(source + 248),
        );
        write_value::<u64>(
            frame + 8,
            super::bionic::guest_mem::read_value::<u64>(source + 256),
        );
    }
    let handler_mask = saved_mask
        | action.mask
        | if action.flags & SA_NODEFER != 0 {
            0
        } else {
            bit(signal)
        };
    set_mask(handler_mask);
    if context.is_none() {
        apply_darwin_mask(mask());
    }
    let call = CallBuilder::new(action.handler as usize)
        .i32(signal)
        .ptr(frame + SIGINFO_OFFSET)
        .ptr(frame + UCONTEXT_OFFSET);
    let call = match handler_stack {
        Some(top) => call.on_stack(top),
        None => call,
    };
    if let Err(error) = call.invoke() {
        host_log::line(&format!("FIXME: guest signal handler for signal {signal} failed: {error}; next missing primitive: guest signal handler failure"));
        std::process::abort();
    }
    let restored =
        signal_frame::handler_mask(frame + UCONTEXT_OFFSET) & !(bit(SIGKILL) | bit(SIGSTOP));
    set_mask(restored);
    match context {
        Some(context) => {
            signal_frame::read_back(frame + UCONTEXT_OFFSET, context);
            signal_frame::restore_darwin_mask(context, restored);
        }
        None => apply_darwin_mask(restored),
    }
}

pub fn apply_darwin_mask(linux_mask: u64) {
    let host: libc::sigset_t = to_darwin_mask(linux_mask);
    unsafe { libc::pthread_sigmask(libc::SIG_SETMASK, &host, std::ptr::null_mut()) };
}

pub fn deliver_pending() {
    while let Some(signal) = take_deliverable() {
        let code = if take_thread_directed(signal) {
            SI_TKILL
        } else {
            0
        };
        let delivery = Delivery {
            signal,
            code,
            address: 0,
            pid: unsafe { libc::getpid() },
            uid: unsafe { libc::getuid() },
            fault: false,
        };
        if action_of(signal).has_handler() {
            deliver(&delivery, None);
        }
    }
}
