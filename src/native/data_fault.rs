use std::cell::RefCell;

use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use super::host_log;

pub const SLOT_BYTES: usize = 16384;
const SLOT_COUNT: usize = 65536;

struct Slots {
    base: usize,
    messages: Box<[AtomicPtr<String>]>,
}

static SLOTS: OnceLock<Slots> = OnceLock::new();
static NEXT: AtomicUsize = AtomicUsize::new(0);
static ALLOCATION: Mutex<()> = Mutex::new(());
static PREVIOUS_SEGV: AtomicPtr<libc::sigaction> = AtomicPtr::new(std::ptr::null_mut());
static PREVIOUS_BUS: AtomicPtr<libc::sigaction> = AtomicPtr::new(std::ptr::null_mut());

fn slots() -> &'static Slots {
    SLOTS.get_or_init(|| {
        let region = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                SLOT_BYTES * SLOT_COUNT,
                libc::PROT_NONE,
                libc::MAP_PRIVATE | libc::MAP_ANON | libc::MAP_NORESERVE,
                -1,
                0,
            )
        };
        assert_ne!(
            region,
            libc::MAP_FAILED,
            "unresolved data reservation failed"
        );
        Slots {
            base: region as usize,
            messages: (0..SLOT_COUNT)
                .map(|_| AtomicPtr::new(std::ptr::null_mut()))
                .collect(),
        }
    })
}

pub fn reserve(message: String) -> Option<usize> {
    install();
    let slots = slots();
    let _allocation = ALLOCATION.lock().expect("unresolved data allocation");
    let index = NEXT.load(Ordering::SeqCst);
    if index >= SLOT_COUNT {
        return None;
    }
    slots.messages[index].store(Box::into_raw(Box::new(message)), Ordering::Release);
    NEXT.store(index + 1, Ordering::SeqCst);
    Some(slots.base + index * SLOT_BYTES)
}

fn message_at(address: usize) -> Option<&'static str> {
    let slots = SLOTS.get()?;
    let offset = address.checked_sub(slots.base)?;
    let message = slots
        .messages
        .get(offset / SLOT_BYTES)?
        .load(Ordering::Acquire);
    unsafe { message.as_ref() }.map(String::as_str)
}

pub fn is_reserved(address: usize) -> bool {
    message_at(address).is_some()
}

fn previous_of(signal: libc::c_int) -> &'static AtomicPtr<libc::sigaction> {
    if signal == libc::SIGBUS {
        &PREVIOUS_BUS
    } else {
        &PREVIOUS_SEGV
    }
}

fn restore_default(signal: libc::c_int) {
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = libc::SIG_DFL;
        libc::sigemptyset(&mut action.sa_mask);
        libc::sigaction(signal, &action, std::ptr::null_mut());
    }
}

unsafe fn chain(signal: libc::c_int, info: *mut libc::siginfo_t, context: *mut libc::c_void) {
    let previous = previous_of(signal).load(Ordering::Acquire);
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

extern "C" fn on_fault(
    signal: libc::c_int,
    info: *mut libc::siginfo_t,
    context: *mut libc::c_void,
) {
    let address = unsafe { (*info).si_addr() } as usize;
    if let Some(message) = message_at(address) {
        host_log::line(message);
        unsafe { libc::abort() };
    }
    unsafe { chain(signal, info, context) };
}

fn handler_installed(signal: libc::c_int) -> bool {
    let mut current: libc::sigaction = unsafe { std::mem::zeroed() };
    unsafe { libc::sigaction(signal, std::ptr::null(), &mut current) };
    current.sa_sigaction == on_fault as *const () as usize
        && current.sa_flags & libc::SA_SIGINFO != 0
}

pub fn install() {
    for signal in [libc::SIGSEGV, libc::SIGBUS] {
        if handler_installed(signal) {
            continue;
        }
        let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
        action.sa_sigaction = on_fault as *const () as usize;
        action.sa_flags = libc::SA_SIGINFO | libc::SA_ONSTACK;
        unsafe { libc::sigemptyset(&mut action.sa_mask) };
        let mut previous: Box<libc::sigaction> = Box::new(unsafe { std::mem::zeroed() });
        let status = unsafe { libc::sigaction(signal, &action, &mut *previous) };
        assert_eq!(
            status, 0,
            "installing the unresolved data fault handler failed"
        );
        previous_of(signal).store(Box::into_raw(previous), Ordering::Release);
    }
}

struct AlternateStack(Option<Box<[u8]>>);

impl Drop for AlternateStack {
    fn drop(&mut self) {
        if self.0.is_some() {
            let disable = libc::stack_t {
                ss_sp: std::ptr::null_mut(),
                ss_size: 0,
                ss_flags: libc::SS_DISABLE,
            };
            unsafe { libc::sigaltstack(&disable, std::ptr::null_mut()) };
        }
    }
}

thread_local! {
    static ALTERNATE: RefCell<AlternateStack> = const { RefCell::new(AlternateStack(None)) };
}

pub fn ensure_alternate_stack() {
    let mut current: libc::stack_t = unsafe { std::mem::zeroed() };
    unsafe { libc::sigaltstack(std::ptr::null(), &mut current) };
    if current.ss_flags & libc::SS_DISABLE == 0 {
        return;
    }
    ALTERNATE.with(|alternate| {
        let mut stack = vec![0u8; 64 * 1024].into_boxed_slice();
        let installed = libc::stack_t {
            ss_sp: stack.as_mut_ptr() as *mut libc::c_void,
            ss_size: stack.len(),
            ss_flags: 0,
        };
        if unsafe { libc::sigaltstack(&installed, std::ptr::null_mut()) } == 0 {
            alternate.borrow_mut().0 = Some(stack);
        }
    });
}
