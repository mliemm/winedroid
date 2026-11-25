use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use super::super::bridge::CallContext;
use super::super::call_assign::CallBuilder;
use super::export::in_libc;

#[derive(Clone, Copy)]
struct Registration {
    function: u64,
    argument: u64,
    dso: u64,
    sequence: u64,
    thread: std::thread::ThreadId,
}

struct Table {
    entries: Vec<Option<Registration>>,
    appends: u64,
}

static TABLE: Mutex<Table> = Mutex::new(Table { entries: Vec::new(), appends: 0 });
static DEPTH: AtomicUsize = AtomicUsize::new(0);
static EXIT_FUNCTION: Mutex<extern "C" fn(i32)> = Mutex::new(host_exit);

extern "C" fn host_exit(status: i32) {
    unsafe { libc::_exit(status) }
}

pub fn set_exit_function(function: extern "C" fn(i32)) {
    *EXIT_FUNCTION.lock().expect("exit function") = function;
}

pub fn registration_count() -> usize {
    TABLE.lock().expect("atexit table").entries.iter().flatten().count()
}

#[cfg(test)]
pub fn retain_within(since: u64, start: usize, length: usize) {
    let mut table = TABLE.lock().expect("atexit table");
    table.entries.retain(|entry| entry.is_some_and(|entry| entry.thread == std::thread::current().id() && entry.sequence >= since && entry.dso as usize >= start && (entry.dso as usize) < start + length));
}

#[cfg(test)]
pub fn append_count() -> u64 {
    TABLE.lock().expect("atexit table").appends
}

#[cfg(test)]
pub fn registrations_within(since: u64, start: usize, length: usize) -> Vec<(u64, u64, u64)> {
    let table = TABLE.lock().expect("atexit table");
    table
        .entries
        .iter()
        .flatten()
        .filter(|entry| entry.thread == std::thread::current().id() && entry.sequence >= since && entry.dso as usize >= start && (entry.dso as usize) < start + length)
        .map(|entry| (entry.function, entry.argument, entry.dso))
        .collect()
}

fn register_exit_handler(function: u64, argument: u64, dso: u64) -> i32 {
    // cite: bionic atexit.cpp#__cxa_atexit
    if function == 0 {
        return -1;
    }
    let mut table = TABLE.lock().expect("atexit table");
    let sequence = table.appends;
    table.entries.push(Some(Registration { function, argument, dso, sequence, thread: std::thread::current().id() }));
    table.appends += 1;
    0
}

fn call_registered(registration: Registration) -> Result<(), String> {
    CallBuilder::new(registration.function as usize).int(registration.argument).invoke().map(|_| ()).map_err(|error| error.to_string())
}

pub fn finalize(dso: u64) -> Result<(), String> {
    // cite: bionic atexit.cpp#__cxa_finalize
    DEPTH.fetch_add(1, Ordering::SeqCst);
    let outcome = finalize_entries(dso);
    let outermost = DEPTH.fetch_sub(1, Ordering::SeqCst) == 1;
    if outermost && dso != 0 {
        TABLE.lock().expect("atexit table").entries.retain(Option::is_some);
    }
    outcome
}

fn finalize_entries(dso: u64) -> Result<(), String> {
    'restart: loop {
        let appends = TABLE.lock().expect("atexit table").appends;
        let mut index = TABLE.lock().expect("atexit table").entries.len();
        while index > 0 {
            index -= 1;
            let taken = {
                let mut table = TABLE.lock().expect("atexit table");
                match table.entries.get(index).copied().flatten() {
                    Some(entry) if dso == 0 || entry.dso == dso => {
                        table.entries[index] = None;
                        Some(entry)
                    }
                    _ => None,
                }
            };
            let Some(entry) = taken else { continue };
            call_registered(entry)?;
            if TABLE.lock().expect("atexit table").appends != appends {
                continue 'restart;
            }
        }
        return Ok(());
    }
}

pub fn run_thread_destructors() -> Result<(), String> {
    // cite: bionic __cxa_thread_atexit_impl.cpp#__cxa_thread_finalize
    super::super::thread_exit::finalize_thread_locals(super::current_thread::record())
}

fn exit_process(context: &mut CallContext) {
    // cite: bionic exit.cpp#exit
    let status = context.arg_i32(0);
    if let Err(message) = run_thread_destructors().and_then(|()| finalize(0)) {
        context.fail(message);
        return;
    }
    let function = *EXIT_FUNCTION.lock().expect("exit function");
    function(status);
}

pub fn register() {
    in_libc("__cxa_atexit", |context| context.ret_i32(register_exit_handler(context.arg_u64(0), context.arg_u64(1), context.arg_u64(2))));
    in_libc("__cxa_finalize", |context| {
        if let Err(message) = finalize(context.arg_u64(0)) {
            context.fail(message);
        }
    });
    in_libc("__cxa_thread_atexit_impl", |context| {
        // cite: bionic __cxa_thread_atexit_impl.cpp#__cxa_thread_atexit_impl
        let (function, argument, dso) = (context.arg_u64(0), context.arg_u64(1), context.arg_u64(2));
        super::super::thread_exit::add_thread_local_destructor(super::current_thread::record(), function, argument, dso);
        context.ret_i32(0);
    });
    in_libc("exit", exit_process);
    in_libc("_exit", |context| {
        // cite: bionic exit.cpp#_exit
        let function = *EXIT_FUNCTION.lock().expect("exit function");
        function(context.arg_i32(0));
    });
}
