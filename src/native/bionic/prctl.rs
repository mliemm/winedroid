use super::super::host_log;
use super::super::thread_list;
use super::abi_types::EINVAL;
use super::current_thread;
use super::export::in_libc;
use super::guest_mem::{cstr_bytes_bounded, write_bytes};

pub const PR_SET_NAME: i32 = 15;
pub const PR_GET_NAME: i32 = 16;
const NAME_BYTES: usize = 16;

fn read_name(address: usize) -> Vec<u8> {
    // cite: bionic prctl.h#PR_SET_NAME
    cstr_bytes_bounded(address, NAME_BYTES - 1).to_vec()
}

pub fn control(option: i32, argument: usize) -> i64 {
    // cite: bionic SYSCALLS.TXT#prctl
    let record = current_thread::record();
    match option {
        PR_SET_NAME => {
            let name = read_name(argument);
            thread_list::set_name(record, &name);
            let mut text = name;
            text.push(0);
            unsafe { libc::pthread_setname_np(text.as_ptr() as *const _) };
            0
        }
        PR_GET_NAME => {
            write_bytes(argument, &thread_list::name_of(record));
            0
        }
        other => {
            host_log::line(&format!("FIXME: prctl option {other} is not translated; next missing primitive: prctl option {other}"));
            -i64::from(EINVAL)
        }
    }
}

pub fn register() {
    in_libc("prctl", |context| {
        let result = control(context.arg_i32(0), context.arg_ptr(1));
        if result < 0 {
            context.set_errno((-result) as i32);
            context.ret_i32(-1);
        } else {
            context.ret_i32(result as i32);
        }
    });
}
