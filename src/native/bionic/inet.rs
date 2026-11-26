use super::abi_types::EINVAL;
use super::export::in_libc;
use super::fs_result::finish_int;
use super::guest_mem::{write_bytes, write_value};

const EAFNOSUPPORT: i32 = 97;

extern "C" {
    fn inet_pton(
        af: libc::c_int,
        source: *const libc::c_char,
        destination: *mut libc::c_void,
    ) -> libc::c_int;
    fn inet_ntop(
        af: libc::c_int,
        source: *const libc::c_void,
        destination: *mut libc::c_char,
        size: libc::socklen_t,
    ) -> *const libc::c_char;
}

thread_local! {
    static NT0A_BUFFER: std::cell::RefCell<[u8; 16]> = const { std::cell::RefCell::new([0; 16]) };
}

fn error(code: i32) -> i64 {
    -i64::from(code)
}

pub fn register() {
    in_libc("htons", |context| {
        context.ret_u32(u32::from((context.arg_u32(0) as u16).to_be()))
    });
    in_libc("htonl", |context| {
        context.ret_u32(context.arg_u32(0).to_be())
    });
    in_libc("ntohs", |context| {
        context.ret_u32(u32::from((context.arg_u32(0) as u16).to_be()))
    });
    in_libc("ntohl", |context| {
        context.ret_u32(context.arg_u32(0).to_be())
    });
    in_libc("inet_pton", |context| {
        // cite: bionic inet_pton.c#inet_pton
        let family = context.arg_i32(0);
        let source = context.arg_ptr(1);
        let destination = context.arg_ptr(2);
        if source == 0 || destination == 0 {
            return finish_int(context, error(super::abi_types::EFAULT));
        }
        let host_family = match family {
            super::linux_abi::socket::AF_INET => libc::AF_INET,
            super::linux_abi::socket::AF_INET6 => libc::AF_INET6,
            _ => return finish_int(context, error(EAFNOSUPPORT)),
        };
        let status = unsafe {
            inet_pton(
                host_family,
                source as *const libc::c_char,
                destination as *mut libc::c_void,
            )
        };
        finish_int(context, i64::from(status))
    });
    in_libc("inet_ntop", |context| {
        let family = context.arg_i32(0);
        let source = context.arg_ptr(1);
        let destination = context.arg_ptr(2);
        let capacity = context.arg_u32(3) as usize;
        if source == 0 || destination == 0 || capacity == 0 {
            context.set_errno(EINVAL);
            context.ret_ptr(0);
            return;
        }
        let host_family = match family {
            super::linux_abi::socket::AF_INET => libc::AF_INET,
            super::linux_abi::socket::AF_INET6 => libc::AF_INET6,
            _ => {
                context.set_errno(EAFNOSUPPORT);
                context.ret_ptr(0);
                return;
            }
        };
        let status = unsafe {
            inet_ntop(
                host_family,
                source as *const libc::c_void,
                destination as *mut libc::c_char,
                capacity as libc::socklen_t,
            )
        };
        if status.is_null() {
            context.set_errno(EINVAL);
            context.ret_ptr(0);
        } else {
            context.ret_ptr(destination);
        }
    });
    in_libc("inet_addr", |context| {
        // cite: bionic arpa_inet.cpp#inet_addr
        let source = context.arg_ptr(0);
        if source == 0 {
            context.ret_u32(u32::MAX);
            return;
        }
        let mut address = [0u8; 4];
        let status = unsafe {
            inet_pton(
                libc::AF_INET,
                source as *const libc::c_char,
                address.as_mut_ptr() as *mut libc::c_void,
            )
        };
        if status != 1 {
            context.ret_u32(u32::MAX);
            return;
        }
        context.ret_u32(u32::from_ne_bytes(address))
    });
    in_libc("inet_ntoa", |context| {
        // cite: bionic inet_ntoa.c#inet_ntoa
        let packed = context.arg_u32(0);
        let pointer = NT0A_BUFFER.with(|buffer| {
            let mut buffer = buffer.borrow_mut();
            let mut text = [0i8; 16];
            let status = unsafe {
                inet_ntop(
                    libc::AF_INET,
                    &packed as *const u32 as *const libc::c_void,
                    text.as_mut_ptr() as *mut libc::c_char,
                    text.len() as libc::socklen_t,
                )
            };
            if status.is_null() {
                return 0usize;
            }
            let length = unsafe { libc::strlen(text.as_ptr()) };
            write_bytes(buffer.as_mut_ptr() as usize, unsafe {
                std::slice::from_raw_parts(text.as_ptr() as *const u8, length)
            });
            write_value::<u8>(buffer.as_mut_ptr() as usize + length, 0);
            buffer.as_mut_ptr() as usize
        });
        context.ret_ptr(pointer);
    });
}
