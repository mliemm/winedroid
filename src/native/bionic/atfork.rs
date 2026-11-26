use std::sync::Mutex;

use super::export::in_libc;

#[derive(Clone, Copy)]
pub struct Entry {
    pub prepare: u64,
    pub parent: u64,
    pub child: u64,
    pub dso: u64,
}

static ENTRIES: Mutex<Vec<Entry>> = Mutex::new(Vec::new());

pub fn entries() -> Vec<Entry> {
    ENTRIES.lock().expect("atfork list").clone()
}

pub fn unregister(dso: u64) {
    // cite: bionic pthread_atfork.cpp#__unregister_atfork
    ENTRIES
        .lock()
        .expect("atfork list")
        .retain(|entry| entry.dso != dso);
}

fn register_entry(prepare: u64, parent: u64, child: u64, dso: u64) {
    // cite: bionic pthread_atfork.cpp#__register_atfork
    ENTRIES.lock().expect("atfork list").push(Entry {
        prepare,
        parent,
        child,
        dso,
    });
}

pub fn register() {
    in_libc("pthread_atfork", |context| {
        register_entry(
            context.arg_u64(0),
            context.arg_u64(1),
            context.arg_u64(2),
            0,
        );
        context.ret_i32(0);
    });
    in_libc("__register_atfork", |context| {
        register_entry(
            context.arg_u64(0),
            context.arg_u64(1),
            context.arg_u64(2),
            context.arg_u64(3),
        );
        context.ret_i32(0);
    });
}
