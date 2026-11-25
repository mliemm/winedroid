use std::sync::atomic::Ordering;

use super::super::futex::{self, MATCH_ANY};
use super::super::pthread_internal::{PthreadInternal, JOIN_DETACHED, JOIN_EXITED_NOT_JOINED, JOIN_JOINED, JOIN_NOT_JOINED};
use super::super::thread_exit;
use super::super::thread_list::{self, find};
use super::super::thread_start;
use super::abi_types::{EINVAL, ERANGE};
use super::current_thread;
use super::export::in_libc;
use super::guest_mem::{cstr_bytes, read_value, write_value};
use super::pthread_attr::{ATTR_SIZE, FLAG_DETACHED};
use super::pthread_mutex::EDEADLK;
use super::pthread_sched;
use super::super::call_assign::CallBuilder;

const ESRCH: i32 = 3;
const NAME_LIMIT: usize = 16;

fn record_of(thread: usize) -> *mut PthreadInternal {
    thread as *mut PthreadInternal
}

fn join(thread: usize, result: usize) -> i32 {
    // cite: bionic pthread_join.cpp#pthread_join
    if thread == current_thread::record() {
        return EDEADLK;
    }
    let Some(record) = find(thread, "pthread_join") else { return ESRCH };
    let state = thread_exit::join_state(record);
    let mut old = JOIN_NOT_JOINED;
    while old == JOIN_NOT_JOINED || old == JOIN_EXITED_NOT_JOINED {
        match state.compare_exchange_weak(old, JOIN_JOINED, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => break,
            Err(current) => old = current,
        }
    }
    if old == JOIN_DETACHED || old == JOIN_JOINED {
        return EINVAL;
    }
    let tid = unsafe { (*record_of(record)).tid };
    let tid_word = thread_exit::tid_word(record);
    while super::guest_atomic::i32_at(tid_word).load(Ordering::Acquire) != 0 {
        futex::wait(tid_word, tid as u32, MATCH_ANY, None);
    }
    if let Some(host) = thread_list::host_of(record) {
        unsafe { libc::pthread_join(host, std::ptr::null_mut()) };
    }
    if result != 0 {
        write_value::<u64>(result, unsafe { (*record_of(record)).return_value });
    }
    thread_exit::release(record);
    0
}

fn detach(thread: usize) -> i32 {
    // cite: bionic pthread_detach.cpp#pthread_detach
    let Some(record) = find(thread, "pthread_detach") else { return ESRCH };
    let state = thread_exit::join_state(record);
    let mut old = JOIN_NOT_JOINED;
    while old == JOIN_NOT_JOINED {
        match state.compare_exchange_weak(old, JOIN_DETACHED, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => break,
            Err(current) => old = current,
        }
    }
    match old {
        JOIN_NOT_JOINED => {
            if let Some(host) = thread_list::host_of(record).filter(|_| thread_list::was_created_by_guest(record)) {
                unsafe { libc::pthread_detach(host) };
            }
            0
        }
        JOIN_EXITED_NOT_JOINED => join(thread, 0),
        _ => EINVAL,
    }
}

fn exit_current(context: &mut super::super::bridge::CallContext) {
    // cite: bionic pthread_exit.cpp#pthread_exit
    let record = current_thread::record();
    let outcome = thread_exit::run_guest_storage(record, context.arg_u64(0));
    if let Err(message) = outcome {
        return context.fail(message);
    }
    if !thread_list::was_created_by_guest(record) {
        return context.fail("guest thread called pthread_exit; next missing primitive: ending a host created thread from guest code");
    }
    thread_exit::request_exit();
    context.fail("pthread_exit");
}

fn host_of(thread: usize, caller: &str) -> Result<(usize, libc::pthread_t), i32> {
    let record = find(thread, caller).ok_or(ESRCH)?;
    let host = thread_list::host_of(record).ok_or(ESRCH)?;
    Ok((record, host))
}

pub fn register() {
    in_libc("pthread_create", |context| context.ret_i32(thread_start::create(context.arg_ptr(0), context.arg_ptr(1), context.arg_ptr(2), context.arg_ptr(3))));
    in_libc("pthread_join", |context| context.ret_i32(join(context.arg_ptr(0), context.arg_ptr(1))));
    in_libc("pthread_detach", |context| context.ret_i32(detach(context.arg_ptr(0))));
    in_libc("pthread_exit", exit_current);
    in_libc("pthread_getattr_np", |context| {
        // cite: bionic pthread_attr.cpp#pthread_getattr_np
        let Some(record) = find(context.arg_ptr(0), "pthread_getattr_np") else { return context.ret_i32(ESRCH) };
        let attr = context.arg_ptr(1);
        unsafe { std::ptr::copy_nonoverlapping(&raw const (*record_of(record)).attr as *const u8, attr as *mut u8, ATTR_SIZE) };
        if thread_exit::join_state(record).load(Ordering::SeqCst) == JOIN_DETACHED {
            write_value::<u32>(attr, read_value::<u32>(attr) | FLAG_DETACHED);
        }
        context.ret_i32(0);
    });
    in_libc("pthread_gettid_np", |context| {
        // cite: bionic pthread_gettid_np.cpp#pthread_gettid_np
        let tid = find(context.arg_ptr(0), "pthread_gettid_np").map_or(-1, |record| unsafe { (*record_of(record)).tid });
        context.ret_i32(tid);
    });
    in_libc("__pthread_cleanup_push", |context| {
        // cite: bionic pthread_exit.cpp#__pthread_cleanup_push
        let (frame, routine, argument) = (context.arg_ptr(0), context.arg_u64(1), context.arg_u64(2));
        let record = record_of(current_thread::record());
        write_value::<u64>(frame, unsafe { (*record).cleanup_stack });
        write_value::<u64>(frame + 8, routine);
        write_value::<u64>(frame + 16, argument);
        unsafe { (*record).cleanup_stack = frame as u64 };
    });
    in_libc("__pthread_cleanup_pop", |context| {
        // cite: bionic pthread_exit.cpp#__pthread_cleanup_pop
        let (frame, execute) = (context.arg_ptr(0), context.arg_i32(1) != 0);
        let record = record_of(current_thread::record());
        unsafe { (*record).cleanup_stack = read_value::<u64>(frame) };
        if execute {
            let (routine, argument) = (read_value::<u64>(frame + 8), read_value::<u64>(frame + 16));
            if let Err(error) = CallBuilder::new(routine as usize).int(argument).invoke() {
                context.fail(error.to_string());
            }
        }
    });
    in_libc("pthread_setname_np", |context| {
        // cite: bionic pthread_setname_np.cpp#pthread_setname_np
        let name = cstr_bytes(context.arg_ptr(1));
        if name.len() >= NAME_LIMIT {
            return context.ret_i32(ERANGE);
        }
        let Some(record) = find(context.arg_ptr(0), "pthread_setname_np") else { return context.ret_i32(ESRCH) };
        thread_list::set_name(record, name);
        if record == current_thread::record() {
            let mut text = name.to_vec();
            text.push(0);
            unsafe { libc::pthread_setname_np(text.as_ptr() as *const _) };
        }
        context.ret_i32(0);
    });
    in_libc("pthread_getname_np", |context| {
        // cite: bionic pthread_setname_np.cpp#pthread_getname_np
        if context.arg_u64(2) < NAME_LIMIT as u64 {
            return context.ret_i32(ERANGE);
        }
        let Some(record) = find(context.arg_ptr(0), "pthread_getname_np") else { return context.ret_i32(ESRCH) };
        super::guest_mem::write_bytes(context.arg_ptr(1), &thread_list::name_of(record));
        context.ret_i32(0);
    });
    in_libc("pthread_getcpuclockid", |context| {
        // cite: bionic pthread_getcpuclockid.cpp#pthread_getcpuclockid
        let Some(record) = find(context.arg_ptr(0), "pthread_getcpuclockid") else { return context.ret_i32(ESRCH) };
        let tid = unsafe { (*record_of(record)).tid };
        write_value::<i32>(context.arg_ptr(1), ((!tid) << 3) | 2 | 4);
        context.ret_i32(0);
    });
    in_libc("pthread_getschedparam", |context| {
        // cite: bionic pthread_getschedparam.cpp#pthread_getschedparam
        let outcome = host_of(context.arg_ptr(0), "pthread_getschedparam").and_then(|(_, host)| pthread_sched::get_host(host));
        match outcome {
            Ok((policy, priority)) => {
                write_value::<i32>(context.arg_ptr(1), policy);
                write_value::<i32>(context.arg_ptr(2), priority);
                context.ret_i32(0);
            }
            Err(code) => context.ret_i32(code),
        }
    });
    in_libc("pthread_setschedparam", |context| {
        // cite: bionic pthread_setschedparam.cpp#pthread_setschedparam
        let priority = read_value::<i32>(context.arg_ptr(2));
        let outcome = host_of(context.arg_ptr(0), "pthread_setschedparam").and_then(|(_, host)| pthread_sched::set_host(host, context.arg_i32(1), priority));
        context.ret_i32(outcome.err().unwrap_or(0));
    });
    in_libc("pthread_setschedprio", |context| {
        // cite: bionic pthread_setschedparam.cpp#pthread_setschedprio
        let priority = context.arg_i32(1);
        let outcome = host_of(context.arg_ptr(0), "pthread_setschedprio").and_then(|(_, host)| {
            let (policy, _) = pthread_sched::get_host(host)?;
            pthread_sched::set_host(host, policy, priority)
        });
        context.ret_i32(outcome.err().unwrap_or(0));
    });
    in_libc("sched_get_priority_max", |context| match pthread_sched::priority_limit(context.arg_i32(0), true) {
        Some(value) => context.ret_i32(value),
        None => {
            context.set_errno(EINVAL);
            context.ret_i32(-1);
        }
    });
    in_libc("sched_get_priority_min", |context| match pthread_sched::priority_limit(context.arg_i32(0), false) {
        Some(value) => context.ret_i32(value),
        None => {
            context.set_errno(EINVAL);
            context.ret_i32(-1);
        }
    });
    in_libc("sched_yield", |context| {
        unsafe { libc::sched_yield() };
        context.ret_i32(0);
    });
}
