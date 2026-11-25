use std::any::Any;
use std::cell::RefCell;
use std::sync::Once;

use super::{futex, guest_thread, thread_list};

thread_local! {
    static HELD: RefCell<Vec<Box<dyn Any>>> = const { RefCell::new(Vec::new()) };
}

extern "C" fn prepare() {
    HELD.with(|held| {
        let mut held = held.borrow_mut();
        guest_thread::hold_for_fork(&mut held);
        thread_list::hold_for_fork(&mut held);
        futex::hold_for_fork(&mut held);
    });
}

extern "C" fn release() {
    let held = HELD.with(|held| std::mem::take(&mut *held.borrow_mut()));
    drop(held);
}

pub fn install() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let status = unsafe { libc::pthread_atfork(Some(prepare), Some(release), Some(release)) };
        assert_eq!(status, 0, "pthread_atfork for the thread registries");
    });
}
