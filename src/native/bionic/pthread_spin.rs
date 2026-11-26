use super::bionic_lock;
use super::export::in_libc;
use super::pthread_mutex::EBUSY;

const TRIES_BEFORE_SLEEPING: usize = 10000;

pub fn register() {
    in_libc("pthread_spin_init", |context| {
        // cite: bionic pthread_spinlock.cpp#pthread_spin_init
        bionic_lock::init(context.arg_ptr(0), context.arg_i32(1) != 0);
        context.ret_i32(0);
    });
    in_libc("pthread_spin_destroy", |context| {
        // cite: bionic pthread_spinlock.cpp#pthread_spin_destroy
        context.ret_i32(if bionic_lock::try_lock(context.arg_ptr(0)) {
            0
        } else {
            EBUSY
        });
    });
    in_libc("pthread_spin_trylock", |context| {
        // cite: bionic pthread_spinlock.cpp#pthread_spin_trylock
        context.ret_i32(if bionic_lock::try_lock(context.arg_ptr(0)) {
            0
        } else {
            EBUSY
        });
    });
    in_libc("pthread_spin_lock", |context| {
        // cite: bionic pthread_spinlock.cpp#pthread_spin_lock
        let lock = context.arg_ptr(0);
        if !(0..TRIES_BEFORE_SLEEPING).any(|_| bionic_lock::try_lock(lock)) {
            bionic_lock::lock(lock);
        }
        context.ret_i32(0);
    });
    in_libc("pthread_spin_unlock", |context| {
        // cite: bionic pthread_spinlock.cpp#pthread_spin_unlock
        bionic_lock::unlock(context.arg_ptr(0));
        context.ret_i32(0);
    });
}
