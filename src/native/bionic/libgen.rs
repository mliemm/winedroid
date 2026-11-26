use std::cell::RefCell;

use super::super::bridge::CallContext;
use super::abi_types::{ENAMETOOLONG, ERANGE};
use super::export::in_libc;
use super::guest_mem::{optional_cstr, write_bytes, write_value};

const MAX_PATH_LENGTH: usize = 4096;

thread_local! {
    static BASENAME: RefCell<Box<[u8; MAX_PATH_LENGTH]>> = RefCell::new(Box::new([0; MAX_PATH_LENGTH]));
    static DIRNAME: RefCell<Box<[u8; MAX_PATH_LENGTH]>> = RefCell::new(Box::new([0; MAX_PATH_LENGTH]));
}

fn base_of(path: Option<&[u8]>) -> Vec<u8> {
    // cite: bionic libgen.cpp#__basename_r
    let Some(path) = path.filter(|path| !path.is_empty()) else {
        return b".".to_vec();
    };
    let mut end = path.len() - 1;
    while end > 0 && path[end] == b'/' {
        end -= 1;
    }
    if end == 0 && path[0] == b'/' {
        return b"/".to_vec();
    }
    let mut start = end;
    while start > 0 && path[start - 1] != b'/' {
        start -= 1;
    }
    path[start..=end].to_vec()
}

fn directory_of(path: Option<&[u8]>) -> Vec<u8> {
    // cite: bionic libgen.cpp#__dirname_r
    let Some(path) = path.filter(|path| !path.is_empty()) else {
        return b".".to_vec();
    };
    let mut end = path.len() - 1;
    while end > 0 && path[end] == b'/' {
        end -= 1;
    }
    while end > 0 && path[end] != b'/' {
        end -= 1;
    }
    if end == 0 {
        return if path[0] == b'/' {
            b"/".to_vec()
        } else {
            b".".to_vec()
        };
    }
    loop {
        end -= 1;
        if !(end > 0 && path[end] == b'/') {
            break;
        }
    }
    path[..=end].to_vec()
}

fn into_buffer(
    context: &mut CallContext,
    result: Vec<u8>,
    long_name_errno: Option<i32>,
    buffer: &'static std::thread::LocalKey<RefCell<Box<[u8; MAX_PATH_LENGTH]>>>,
) {
    if let Some(code) = long_name_errno {
        context.set_errno(code);
        context.ret_ptr(0);
        return;
    }
    if result.len() > MAX_PATH_LENGTH - 1 {
        context.set_errno(ERANGE);
        context.ret_ptr(0);
        return;
    }
    let address = buffer.with(|storage| storage.borrow().as_ptr() as usize);
    write_bytes(address, &result);
    write_value::<u8>(address + result.len(), 0);
    context.ret_ptr(address);
}

pub fn register() {
    in_libc("basename", |context| {
        let result = base_of(optional_cstr(context.arg_ptr(0)));
        into_buffer(context, result, None, &BASENAME);
    });
    in_libc("dirname", |context| {
        let result = directory_of(optional_cstr(context.arg_ptr(0)));
        let too_long = (result.len() + 1 > MAX_PATH_LENGTH).then_some(ENAMETOOLONG);
        into_buffer(context, result, too_long, &DIRNAME);
    });
}
