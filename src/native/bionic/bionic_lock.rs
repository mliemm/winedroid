use std::sync::atomic::Ordering;

use super::super::futex::{self, MATCH_ANY};
use super::guest_atomic::u32_at;

const UNLOCKED: u32 = 0;
const LOCKED_WITHOUT_WAITER: u32 = 1;
const LOCKED_WITH_WAITER: u32 = 2;
pub const LOCK_SIZE: usize = 8;

pub fn init(lock: usize, process_shared: bool) {
    // cite: bionic bionic_lock.h#Lock
    u32_at(lock).store(UNLOCKED, Ordering::Relaxed);
    unsafe { *((lock + 4) as *mut u8) = u8::from(process_shared) };
}

pub fn try_lock(lock: usize) -> bool {
    // cite: bionic bionic_lock.h#trylock
    u32_at(lock)
        .compare_exchange(
            UNLOCKED,
            LOCKED_WITHOUT_WAITER,
            Ordering::Acquire,
            Ordering::Relaxed,
        )
        .is_ok()
}

pub fn lock(lock: usize) {
    // cite: bionic bionic_lock.h#lock
    let state = u32_at(lock);
    if state
        .compare_exchange(
            UNLOCKED,
            LOCKED_WITHOUT_WAITER,
            Ordering::Acquire,
            Ordering::Relaxed,
        )
        .is_ok()
    {
        return;
    }
    while state.swap(LOCKED_WITH_WAITER, Ordering::Acquire) != UNLOCKED {
        futex::wait(lock, LOCKED_WITH_WAITER, MATCH_ANY, None);
    }
}

pub fn unlock(lock: usize) {
    // cite: bionic bionic_lock.h#unlock
    if u32_at(lock).swap(UNLOCKED, Ordering::Release) == LOCKED_WITH_WAITER {
        futex::wake(lock, 1, MATCH_ANY);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::native::test_deadline::bounded_test;

    bounded_test! {
        fn the_lock_serialises_increments_from_several_threads() {
            // cite: bionic bionic_lock.h#Lock
            let words = Arc::new(([0u64; 2], std::cell::UnsafeCell::new(0u64)));
            struct Shared(Arc<([u64; 2], std::cell::UnsafeCell<u64>)>);
            unsafe impl Send for Shared {}
            let handles: Vec<_> = (0..4)
                .map(|_| {
                    let shared = Shared(words.clone());
                    std::thread::spawn(move || {
                        let shared = shared;
                        let address = shared.0 .0.as_ptr() as usize;
                        for _ in 0..2000 {
                            lock(address);
                            unsafe { *shared.0 .1.get() += 1 };
                            unlock(address);
                        }
                    })
                })
                .collect();
            for handle in handles {
                handle.join().unwrap();
            }
            assert_eq!(unsafe { *words.1.get() }, 8000);
            assert!(try_lock(words.0.as_ptr() as usize));
            assert!(!try_lock(words.0.as_ptr() as usize));
        }
    }
}
