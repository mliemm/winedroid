use std::cell::Cell;
use std::sync::atomic::{AtomicPtr, Ordering};

use super::signal_numbers::{bit, to_linux_mask, SIGKILL, SIGNAL_COUNT, SIGSTOP};

pub const SIG_DFL: u64 = 0;
pub const SIG_IGN: u64 = 1;

pub const SA_NOCLDSTOP: u32 = 1;
pub const SA_NOCLDWAIT: u32 = 2;
pub const SA_SIGINFO: u32 = 4;
pub const SA_ONSTACK: u32 = 0x0800_0000;
pub const SA_RESTART: u32 = 0x1000_0000;
pub const SA_NODEFER: u32 = 0x4000_0000;
pub const SA_RESETHAND: u32 = 0x8000_0000;

pub const SS_ONSTACK: i32 = 1;
pub const SS_DISABLE: i32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Action {
    pub flags: u32,
    pub handler: u64,
    pub mask: u64,
    pub restorer: u64,
}

pub const DEFAULT_ACTION: Action = Action { flags: 0, handler: SIG_DFL, mask: 0, restorer: 0 };

impl Action {
    pub fn has_handler(&self) -> bool {
        self.handler > SIG_IGN
    }
}

static ACTIONS: [AtomicPtr<Action>; SIGNAL_COUNT as usize + 1] = [const { AtomicPtr::new(std::ptr::null_mut()) }; SIGNAL_COUNT as usize + 1];

pub fn action_of(signal: i32) -> Action {
    let published = ACTIONS[signal as usize].load(Ordering::Acquire);
    unsafe { published.as_ref() }.copied().unwrap_or(DEFAULT_ACTION)
}

pub fn publish_action(signal: i32, action: Action) {
    ACTIONS[signal as usize].store(Box::leak(Box::new(action)), Ordering::Release);
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AlternateStack {
    pub base: u64,
    pub size: u64,
    pub disabled: bool,
}

thread_local! {
    static IN_GUEST: Cell<bool> = const { Cell::new(false) };
    static PENDING: Cell<u64> = const { Cell::new(0) };
    static MASK: Cell<Option<u64>> = const { Cell::new(None) };
    static ALTERNATE: Cell<Option<AlternateStack>> = const { Cell::new(None) };
    static THREAD_DIRECTED: Cell<u64> = const { Cell::new(0) };
}

pub fn mark_thread_directed(signal: i32) {
    let _ = THREAD_DIRECTED.try_with(|marks| marks.set(marks.get() | bit(signal)));
}

pub fn take_thread_directed(signal: i32) -> bool {
    THREAD_DIRECTED
        .try_with(|marks| {
            let directed = marks.get() & bit(signal) != 0;
            marks.set(marks.get() & !bit(signal));
            directed
        })
        .unwrap_or(false)
}

pub fn in_guest() -> bool {
    IN_GUEST.try_with(Cell::get).unwrap_or(false)
}

pub fn set_in_guest(value: bool) -> bool {
    IN_GUEST.try_with(|flag| flag.replace(value)).unwrap_or(false)
}

pub fn pending() -> u64 {
    PENDING.try_with(Cell::get).unwrap_or(0)
}

pub fn raise_pending(signal: i32) {
    let _ = PENDING.try_with(|pending| pending.set(pending.get() | bit(signal)));
}

pub fn take_deliverable() -> Option<i32> {
    let blocked = mask();
    PENDING
        .try_with(|pending| {
            let ready = pending.get() & !blocked;
            (ready != 0).then(|| {
                let signal = ready.trailing_zeros() as i32 + 1;
                pending.set(pending.get() & !bit(signal));
                signal
            })
        })
        .ok()
        .flatten()
}

fn host_mask() -> u64 {
    let mut current: libc::sigset_t = 0;
    unsafe { libc::pthread_sigmask(libc::SIG_BLOCK, std::ptr::null(), &mut current) };
    to_linux_mask(current)
}

pub fn mask() -> u64 {
    MASK.try_with(|cell| match cell.get() {
        Some(value) => value,
        None => {
            let value = host_mask();
            cell.set(Some(value));
            value
        }
    })
    .unwrap_or(0)
}

pub fn set_mask(value: u64) {
    let value = value & !(bit(SIGKILL) | bit(SIGSTOP));
    let _ = MASK.try_with(|cell| cell.set(Some(value)));
}

pub fn alternate() -> Option<AlternateStack> {
    ALTERNATE.try_with(Cell::get).ok().flatten()
}

pub fn set_alternate(stack: AlternateStack) {
    let _ = ALTERNATE.try_with(|cell| cell.set(Some(stack)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actions_start_as_default_and_publish_replaces_them() {
        // cite: bionic sigaction.cpp#sigaction
        let action = Action { flags: SA_SIGINFO | SA_RESTART, handler: 0x1234, mask: bit(10), restorer: 0 };

        assert_eq!(action_of(50), DEFAULT_ACTION);
        publish_action(50, action);
        assert_eq!(action_of(50), action);
        publish_action(50, DEFAULT_ACTION);
        assert!(!action_of(50).has_handler());
    }

    #[test]
    fn pending_signals_are_delivered_lowest_first_and_skip_blocked_ones() {
        // cite: bionic sigprocmask.cpp#sigprocmask
        std::thread::spawn(|| {
            set_mask(bit(10));
            raise_pending(10);
            raise_pending(12);
            raise_pending(3);

            assert_eq!(take_deliverable(), Some(3));
            assert_eq!(take_deliverable(), Some(12));
            assert_eq!(take_deliverable(), None);
            assert_eq!(pending(), bit(10));
            set_mask(bit(9) | bit(19));
            assert_eq!(mask(), 0, "SIGKILL and SIGSTOP are never kept in the mask");
        })
        .join()
        .unwrap();
    }
}
