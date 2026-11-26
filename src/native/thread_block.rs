use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

pub const SLOT_BIONIC_TLS: isize = -1;
pub const SLOT_DTV: isize = 0;
pub const SLOT_THREAD_ID: isize = 1;
pub const SLOT_APP: isize = 2;
pub const SLOT_OPENGL: isize = 3;
pub const SLOT_OPENGL_API: isize = 4;
pub const SLOT_STACK_GUARD: isize = 5;
pub const SLOT_SANITIZER: isize = 6;
pub const SLOT_ART_THREAD_SELF: isize = 7;

const MIN_SLOT: isize = SLOT_BIONIC_TLS;
const MAX_SLOT: isize = SLOT_ART_THREAD_SELF;
const SLOT_COUNT: usize = (MAX_SLOT - MIN_SLOT + 1) as usize;
const KEY_ATTEMPTS: usize = 1024;

pub static GUEST_TP_BIAS: AtomicUsize = AtomicUsize::new(0);

pub struct ThreadBlock {
    first_key: usize,
    canary: u64,
}

static BLOCK: OnceLock<ThreadBlock> = OnceLock::new();

fn create_key() -> libc::pthread_key_t {
    let mut key: libc::pthread_key_t = 0;
    let status = unsafe { libc::pthread_key_create(&mut key, None) };
    assert_eq!(
        status, 0,
        "thread block key allocation failed with {status}"
    );
    key
}

fn consecutive_run(keys: &[libc::pthread_key_t]) -> Option<usize> {
    let mut sorted: Vec<usize> = keys.iter().map(|key| *key as usize).collect();
    sorted.sort_unstable();
    sorted
        .windows(SLOT_COUNT)
        .find(|window| window[SLOT_COUNT - 1] - window[0] == SLOT_COUNT - 1)
        .map(|window| window[0])
}

fn random_canary() -> u64 {
    let mut bytes = [0u8; 8];
    let status = unsafe { libc::getentropy(bytes.as_mut_ptr() as *mut libc::c_void, bytes.len()) };
    assert_eq!(status, 0, "getentropy failed");
    // cite: bionic __libc_init_main_thread.cpp#android_reset_stack_guards
    u64::from_le_bytes(bytes)
}

fn allocate() -> ThreadBlock {
    // cite: bionic tls_defines.h#TLS_SLOT_BIONIC_TLS, TLS_SLOT_ART_THREAD_SELF, MAX_TLS_SLOT
    let mut created: Vec<libc::pthread_key_t> = Vec::new();
    let first_key = loop {
        assert!(
            created.len() < KEY_ATTEMPTS,
            "no run of {SLOT_COUNT} consecutive thread specific keys"
        );
        created.push(create_key());
        if let Some(first) = consecutive_run(&created) {
            break first;
        }
    };
    for key in created
        .iter()
        .filter(|key| (**key as usize) < first_key || (**key as usize) >= first_key + SLOT_COUNT)
    {
        unsafe { libc::pthread_key_delete(*key) };
    }
    let bias = 8 * (first_key + (0 - MIN_SLOT) as usize);
    assert!(
        bias < 1 << 24,
        "thread block does not fit the two add veneer"
    );
    GUEST_TP_BIAS.store(bias, Ordering::SeqCst);
    ThreadBlock {
        first_key,
        canary: random_canary(),
    }
}

pub fn block() -> &'static ThreadBlock {
    BLOCK.get_or_init(allocate)
}

impl ThreadBlock {
    pub fn key(&self, slot: isize) -> usize {
        assert!(
            (MIN_SLOT..=MAX_SLOT).contains(&slot),
            "Bionic TLS slot {slot} is outside the thread block"
        );
        self.first_key + (slot - MIN_SLOT) as usize
    }

    pub fn canary(&self) -> u64 {
        self.canary
    }

    pub fn bias(&self) -> usize {
        8 * self.key(SLOT_DTV)
    }
}

pub fn host_thread_pointer() -> usize {
    let value: usize;
    unsafe {
        std::arch::asm!("mrs {}, tpidrro_el0", out(reg) value, options(nomem, nostack, preserves_flags))
    };
    value
}

pub fn guest_thread_pointer() -> usize {
    host_thread_pointer() + block().bias()
}

pub fn slot_word(slot: isize) -> u64 {
    unsafe { *(guest_thread_pointer() as *const u64).offset(slot) }
}

pub fn set_slot(slot: isize, value: u64) {
    let status = unsafe {
        libc::pthread_setspecific(
            block().key(slot) as libc::pthread_key_t,
            value as *const libc::c_void,
        )
    };
    assert_eq!(
        status, 0,
        "thread block slot {slot} store failed with {status}"
    );
}
