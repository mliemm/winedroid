use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Once;

use super::super::call_assign::CallBuilder;
use super::super::pthread_internal::PthreadInternal;
use super::abi_types::{EAGAIN, EINVAL};
use super::current_thread;
use super::export::in_libc;
use super::guest_mem::write_value;

pub const KEY_COUNT: usize = 130;
pub const RESERVED_COUNT: usize = 2;
pub const KEY_BLOCK_BYTES: usize = KEY_COUNT * 16;
const DESTRUCTOR_ITERATIONS: usize = 4;
const KEY_VALID_FLAG: u32 = 1 << 31;
const IN_USE_BIT: usize = 1;

struct KeyEntry {
    seq: AtomicUsize,
    destructor: AtomicUsize,
}

static KEY_MAP: [KeyEntry; KEY_COUNT] = [const {
    KeyEntry {
        seq: AtomicUsize::new(0),
        destructor: AtomicUsize::new(0),
    }
}; KEY_COUNT];

fn reserve_bionic_keys() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        // cite: bionic bionic_tls.h#BIONIC_PTHREAD_KEY_RESERVED_COUNT
        for entry in &KEY_MAP[..RESERVED_COUNT] {
            entry.seq.store(IN_USE_BIT, Ordering::Relaxed);
        }
    });
}

fn index_of(key: u32) -> Option<usize> {
    // cite: bionic pthread_key.cpp#KeyInValidRange
    ((key as i32) < (KEY_VALID_FLAG | KEY_COUNT as u32) as i32)
        .then_some((key & !KEY_VALID_FLAG) as usize)
}

fn key_data(record: usize, index: usize) -> (&'static AtomicUsize, &'static AtomicUsize) {
    let block = unsafe { (*(record as *const PthreadInternal)).bionic_tls } as usize;
    let entry = block + index * 16;
    unsafe {
        (
            &*(entry as *const AtomicUsize),
            &*((entry + 8) as *const AtomicUsize),
        )
    }
}

pub fn new_block() -> usize {
    let block = unsafe { libc::calloc(1, KEY_BLOCK_BYTES) } as usize;
    assert_ne!(block, 0, "thread key block allocation failed");
    block
}

pub fn has_pending_destructors(record: usize) -> bool {
    reserve_bionic_keys();
    (0..KEY_COUNT).any(|index| {
        let (seq, data) = key_data(record, index);
        let live = KEY_MAP[index].seq.load(Ordering::Relaxed);
        live & IN_USE_BIT != 0
            && live == seq.load(Ordering::Relaxed)
            && data.load(Ordering::Relaxed) != 0
            && KEY_MAP[index].destructor.load(Ordering::Relaxed) != 0
    })
}

pub fn clean_all(record: usize) -> Result<(), String> {
    // cite: bionic pthread_key.cpp#pthread_key_clean_all
    reserve_bionic_keys();
    for _ in 0..DESTRUCTOR_ITERATIONS {
        let mut called = 0;
        for index in 0..KEY_COUNT {
            let entry = &KEY_MAP[index];
            let live = entry.seq.load(Ordering::Relaxed);
            let (seq, data) = key_data(record, index);
            if live & IN_USE_BIT == 0
                || live != seq.load(Ordering::Relaxed)
                || data.load(Ordering::Relaxed) == 0
            {
                continue;
            }
            let destructor = entry.destructor.load(Ordering::Relaxed);
            if destructor == 0 || entry.seq.load(Ordering::Acquire) != live {
                continue;
            }
            let value = data.swap(0, Ordering::Relaxed);
            CallBuilder::new(destructor)
                .ptr(value)
                .invoke()
                .map_err(|error| error.to_string())?;
            called += 1;
        }
        if called == 0 {
            break;
        }
    }
    Ok(())
}

pub fn register() {
    reserve_bionic_keys();
    in_libc("pthread_key_create", |context| {
        // cite: bionic pthread_key.cpp#pthread_key_create
        let destructor = context.arg_u64(1) as usize;
        for (index, entry) in KEY_MAP.iter().enumerate() {
            let mut seq = entry.seq.load(Ordering::Relaxed);
            while seq & IN_USE_BIT == 0 {
                match entry.seq.compare_exchange_weak(
                    seq,
                    seq + IN_USE_BIT,
                    Ordering::SeqCst,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        entry.destructor.store(destructor, Ordering::SeqCst);
                        write_value::<u32>(context.arg_ptr(0), index as u32 | KEY_VALID_FLAG);
                        return context.ret_i32(0);
                    }
                    Err(current) => seq = current,
                }
            }
        }
        context.ret_i32(EAGAIN);
    });
    in_libc("pthread_key_delete", |context| {
        // cite: bionic pthread_key.cpp#pthread_key_delete
        let Some(index) = index_of(context.arg_u32(0)) else {
            return context.ret_i32(EINVAL);
        };
        let seq = KEY_MAP[index].seq.load(Ordering::Relaxed);
        let deleted = seq & IN_USE_BIT != 0
            && KEY_MAP[index]
                .seq
                .compare_exchange(seq, seq + IN_USE_BIT, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok();
        context.ret_i32(if deleted { 0 } else { EINVAL });
    });
    in_libc("pthread_getspecific", |context| {
        // cite: bionic pthread_key.cpp#pthread_getspecific
        let Some(index) = index_of(context.arg_u32(0)) else {
            return context.ret_ptr(0);
        };
        let seq = KEY_MAP[index].seq.load(Ordering::Relaxed);
        let (stored, data) = key_data(current_thread::record(), index);
        if seq & IN_USE_BIT != 0 && stored.load(Ordering::Relaxed) == seq {
            return context.ret_ptr(data.load(Ordering::Relaxed));
        }
        data.store(0, Ordering::Relaxed);
        context.ret_ptr(0);
    });
    in_libc("pthread_setspecific", |context| {
        // cite: bionic pthread_key.cpp#pthread_setspecific
        let Some(index) = index_of(context.arg_u32(0)) else {
            return context.ret_i32(EINVAL);
        };
        let seq = KEY_MAP[index].seq.load(Ordering::Relaxed);
        if seq & IN_USE_BIT == 0 {
            return context.ret_i32(EINVAL);
        }
        let (stored, data) = key_data(current_thread::record(), index);
        stored.store(seq, Ordering::Relaxed);
        data.store(context.arg_u64(1) as usize, Ordering::Relaxed);
        context.ret_i32(0);
    });
}
