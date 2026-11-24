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
