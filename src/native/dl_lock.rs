use std::ops::Deref;
use std::sync::{Condvar, Mutex};
use std::thread::{self, ThreadId};

// cite: bionic dlfcn.cpp#g_dl_mutex
pub struct RecursiveLock<T> {
    owner: Mutex<(Option<ThreadId>, usize)>,
    released: Condvar,
    value: T,
}

pub struct RecursiveGuard<'a, T> {
    lock: &'a RecursiveLock<T>,
}

impl<T> RecursiveLock<T> {
    pub fn new(value: T) -> Self {
        Self { owner: Mutex::new((None, 0)), released: Condvar::new(), value }
    }

    pub fn lock(&self) -> RecursiveGuard<'_, T> {
        let me = thread::current().id();
        let mut owner = self.owner.lock().expect("dl lock state");
        while owner.0.is_some_and(|holder| holder != me) {
            owner = self.released.wait(owner).expect("dl lock state");
        }
        owner.0 = Some(me);
        owner.1 += 1;
        RecursiveGuard { lock: self }
    }
}

impl<T> Deref for RecursiveGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.lock.value
    }
}

impl<T> Drop for RecursiveGuard<'_, T> {
    fn drop(&mut self) {
        let mut owner = self.lock.owner.lock().expect("dl lock state");
        owner.1 -= 1;
        if owner.1 == 0 {
            owner.0 = None;
            self.lock.released.notify_one();
        }
    }
}

unsafe impl<T: Send> Sync for RecursiveLock<T> {}
unsafe impl<T: Send> Send for RecursiveLock<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{mpsc, Arc};
    use std::time::Duration;

    #[test]
    fn bionic_g_dl_mutex_is_recursive_on_one_thread() {
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let lock = RecursiveLock::new(());
            let outer = lock.lock();
            let inner = lock.lock();
            drop(inner);
            drop(outer);
            let _ = sender.send(());
        });

        assert!(receiver.recv_timeout(Duration::from_secs(10)).is_ok(), "nested lock on the same thread deadlocked");
    }

    #[test]
    fn bionic_g_dl_mutex_excludes_other_threads_until_the_last_nested_guard_drops() {
        let lock = Arc::new(RecursiveLock::new(()));
        let entered = Arc::new(AtomicBool::new(false));
        let (held, held_receiver) = mpsc::channel();
        let (release_inner, inner_gate) = mpsc::channel::<()>();
        let (release_outer, outer_gate) = mpsc::channel::<()>();
        let holder = {
            let lock = lock.clone();
            thread::spawn(move || {
                let outer = lock.lock();
                let inner = lock.lock();
                let _ = held.send(());
                let _ = inner_gate.recv();
                drop(inner);
                let _ = outer_gate.recv();
                drop(outer);
            })
        };
        assert!(held_receiver.recv_timeout(Duration::from_secs(10)).is_ok(), "nested lock on the holder thread deadlocked");
        let contender = {
            let (lock, entered) = (lock.clone(), entered.clone());
            thread::spawn(move || {
                let _guard = lock.lock();
                entered.store(true, Ordering::SeqCst);
            })
        };

        thread::sleep(Duration::from_millis(100));
        assert!(!entered.load(Ordering::SeqCst), "contender entered while both guards were held");
        release_inner.send(()).unwrap();
        thread::sleep(Duration::from_millis(100));
        assert!(!entered.load(Ordering::SeqCst), "contender entered while the outer guard was held");
        release_outer.send(()).unwrap();
        holder.join().unwrap();
        contender.join().unwrap();
        assert!(entered.load(Ordering::SeqCst));
    }
}
