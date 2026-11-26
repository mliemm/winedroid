use super::super::bridge::CallContext;
use super::abi_types::{EBADF, EFAULT, EINVAL, EIO, ENOENT};
use super::export::in_libc;
use super::fd_table;
use super::fortify_check::check_access;
use super::fs_open::path_argument;
use super::fs_result::{finish_int, finish_long, missing};
use super::guest_mem::{cstr_bytes, read_value, write_value};
use super::linux_abi::at;
use super::stdio_files;
use super::stream::{self, Stream};

const ENOMEM: i32 = 12;

fn error(code: i32) -> i64 {
    -i64::from(code)
}

fn stream_at(pointer: usize) -> Result<std::sync::Arc<Stream>, i64> {
    stream::of(pointer).map_err(error)
}

fn read_line(stream: &Stream, pointer: usize, limit: usize, delimiter: Option<u8>) -> i64 {
    if limit == 0 {
        return 0;
    }
    let mut written = 0usize;
    while written + 1 < limit {
        match stream::read_byte(stream) {
            Ok(Some(byte)) => {
                write_value::<u8>(pointer + written, byte);
                written += 1;
                if delimiter == Some(byte) || (delimiter.is_none() && byte == b'\n') {
                    break;
                }
            }
            Ok(None) => break,
            Err(errno) => return error(errno),
        }
    }
    write_value::<u8>(pointer + written, 0);
    written as i64
}

fn get_delimiter(line_pointer: usize, size_pointer: usize, delimiter: i32, file: usize) -> i64 {
    // cite: bionic getdelim.c#getdelim
    if line_pointer == 0 || size_pointer == 0 {
        return error(EINVAL);
    }
    let stream = match stream_at(file) {
        Ok(stream) => stream,
        Err(failure) => return failure,
    };
    let delimiter = delimiter as u8;
    let mut allocation = read_value::<usize>(line_pointer);
    let mut capacity = read_value::<usize>(size_pointer);
    if allocation == 0 || capacity == 0 {
        capacity = 120;
        allocation = unsafe { libc::malloc(capacity) } as usize;
        if allocation == 0 {
            return error(ENOMEM);
        }
        write_value::<usize>(line_pointer, allocation);
        write_value::<usize>(size_pointer, capacity);
    }
    let mut length = 0usize;
    loop {
        match stream::read_byte(&stream) {
            Ok(Some(byte)) => {
                if length + 1 >= capacity {
                    let grown = capacity.saturating_mul(2).max(length + 2);
                    let resized =
                        unsafe { libc::realloc(allocation as *mut libc::c_void, grown) } as usize;
                    if resized == 0 {
                        return error(ENOMEM);
                    }
                    allocation = resized;
                    capacity = grown;
                    write_value::<usize>(line_pointer, allocation);
                    write_value::<usize>(size_pointer, capacity);
                }
                write_value::<u8>(allocation + length, byte);
                length += 1;
                if byte == delimiter {
                    break;
                }
            }
            Ok(None) => {
                if length == 0 {
                    return -1;
                }
                break;
            }
            Err(errno) => return error(errno),
        }
    }
    write_value::<u8>(allocation + length, 0);
    length as i64
}

fn open_stream(path: &[u8], mode: &[u8]) -> Result<usize, i32> {
    let (access, readable, writable) = stream::mode_of(mode)?;
    let fd = super::fs_open::open_at(at::FDCWD, path, access, 0o666);
    if fd < 0 {
        return Err((-fd) as i32);
    }
    let fd = fd as i32;
    if access & super::linux_abi::open::APPEND != 0 {
        super::fs_io::lseek_fd(fd, 0, super::linux_abi::seek::END);
    }
    let buffering = stream::buffering_for(fd);
    let file_flags = (if readable {
        super::linux_abi::stdio::SRD
    } else {
        0
    }) | (if writable {
        super::linux_abi::stdio::SWR
    } else {
        0
    });
    let stream = stream::create(fd, readable, writable, buffering, false, file_flags);
    let file = stream::allocate(&stream);
    if file == 0 {
        super::fs_close_dup::close_fd(fd);
        return Err(ENOMEM);
    }
    Ok(file)
}

fn file_of(context: &CallContext, index: usize) -> Result<std::sync::Arc<Stream>, i64> {
    stream_at(context.arg_ptr(index))
}

pub fn standard(index: usize) -> usize {
    stdio_files::standard(index)
}

pub extern "C" fn flush_all_streams() {
    // cite: bionic stdio.cpp#__libc_stdio_cleanup
    stream::flush_all();
}

pub fn register() {
    super::atexit::set_stdio_cleanup(flush_all_streams);
    for name in ["fopen", "fopen64"] {
        in_libc(name, move |context: &mut CallContext| {
            match (
                path_argument(context.arg_ptr(0)),
                path_argument(context.arg_ptr(1)),
            ) {
                (Ok(path), Ok(mode)) => match open_stream(path, mode) {
                    Ok(file) => context.ret_ptr(file),
                    Err(errno) => {
                        context.set_errno(errno);
                        context.ret_ptr(0);
                    }
                },
                (Err(errno), _) | (_, Err(errno)) => {
                    context.set_errno(errno);
                    context.ret_ptr(0);
                }
            }
        });
    }
    in_libc("fdopen", |context| {
        // cite: bionic stdio.cpp#fdopen
        let fd = context.arg_i32(0);
        let mode = match path_argument(context.arg_ptr(1)) {
            Ok(mode) => mode,
            Err(errno) => {
                context.set_errno(errno);
                context.ret_ptr(0);
                return;
            }
        };
        if !fd_table::contains(fd) {
            context.set_errno(EBADF);
            context.ret_ptr(0);
            return;
        }
        let (access, readable, writable) = match stream::mode_of(mode) {
            Ok(parts) => parts,
            Err(errno) => {
                context.set_errno(errno);
                context.ret_ptr(0);
                return;
            }
        };
        let current = super::fs_fcntl::control(fd, super::linux_abi::fcntl::GETFL, 0);
        if current < 0 {
            finish_long(context, current);
            context.ret_ptr(0);
            return;
        }
        let current_access = (current as i32) & super::linux_abi::open::ACCMODE;
        let wanted = access & super::linux_abi::open::ACCMODE;
        if current_access != super::linux_abi::open::RDWR && current_access != wanted {
            context.set_errno(EINVAL);
            context.ret_ptr(0);
            return;
        }
        if wanted == super::linux_abi::open::RDWR
            && current_access == super::linux_abi::open::RDONLY
        {
            context.set_errno(EINVAL);
            context.ret_ptr(0);
            return;
        }
        let buffering = stream::buffering_for(fd);
        let file_flags = (if readable {
            super::linux_abi::stdio::SRD
        } else {
            0
        }) | (if writable {
            super::linux_abi::stdio::SWR
        } else {
            0
        });
        let stream = stream::create(fd, readable, writable, buffering, false, file_flags);
        let file = stream::allocate(&stream);
        if file == 0 {
            context.set_errno(ENOMEM);
            context.ret_ptr(0);
            return;
        }
        context.ret_ptr(file);
    });
    for name in ["freopen", "freopen64"] {
        in_libc(name, move |context: &mut CallContext| {
            let file = context.arg_ptr(2);
            if context.arg_ptr(0) == 0 {
                missing(
                    "freopen",
                    "a null path means changing the mode of the current descriptor",
                );
                context.set_errno(EINVAL);
                context.ret_ptr(0);
                return;
            }
            if stream::of(file).is_err() {
                context.set_errno(EINVAL);
                context.ret_ptr(0);
                return;
            }
            let (path, mode) = match (
                path_argument(context.arg_ptr(0)),
                path_argument(context.arg_ptr(1)),
            ) {
                (Ok(path), Ok(mode)) => (path, mode),
                (Err(errno), _) | (_, Err(errno)) => {
                    context.set_errno(errno);
                    context.ret_ptr(0);
                    return;
                }
            };
            stream::flush(&stream_at(file).expect("checked stream")).ok();
            stream::detach(file);
            match open_stream(path, mode) {
                Ok(replacement) => {
                    let stream = stream_at(replacement).expect("new stream");
                    stream::detach(replacement);
                    unsafe { libc::free(replacement as *mut libc::c_void) };
                    stream::attach(file, &stream);
                    context.ret_ptr(file);
                }
                Err(errno) => {
                    context.set_errno(errno);
                    context.ret_ptr(0);
                }
            }
        });
    }
    in_libc("fclose", |context| {
        finish_int(context, stream::close_file(context.arg_ptr(0)))
    });
    for name in ["fread", "fread_unlocked"] {
        in_libc(name, move |context: &mut CallContext| {
            let stream = match file_of(context, 3) {
                Ok(stream) => stream,
                Err(failure) => {
                    finish_long(context, failure);
                    return;
                }
            };
            let (size, count) = (context.arg_u64(1), context.arg_u64(2));
            let Some(total) = size.checked_mul(count) else {
                context.set_errno(EINVAL);
                context.ret_i64(0);
                return;
            };
            if total == 0 {
                context.ret_i64(0);
                return;
            }
            let target = context.arg_ptr(0);
            if target == 0 {
                context.set_errno(EFAULT);
                context.ret_i64(0);
                return;
            }
            match stream::read_into(&stream, target, total as usize) {
                Ok(written) => context.ret_i64((written as u64 / size) as i64),
                Err(errno) => {
                    context.set_errno(errno);
                    context.ret_i64(0);
                }
            }
        });
    }
    for name in ["fwrite", "fwrite_unlocked"] {
        in_libc(name, move |context: &mut CallContext| {
            let stream = match file_of(context, 3) {
                Ok(stream) => stream,
                Err(failure) => {
                    finish_long(context, failure);
                    return;
                }
            };
            let (size, count) = (context.arg_u64(1), context.arg_u64(2));
            let Some(total) = size.checked_mul(count) else {
                context.set_errno(EINVAL);
                context.ret_i64(0);
                return;
            };
            if total == 0 {
                context.ret_i64(0);
                return;
            }
            let source = context.arg_ptr(0);
            if source == 0 {
                context.set_errno(EFAULT);
                context.ret_i64(0);
                return;
            }
            match stream::write_bytes(&stream, source, total as usize) {
                Ok(written) => context.ret_i64((written as u64 / size) as i64),
                Err(errno) => {
                    context.set_errno(errno);
                    context.ret_i64(0);
                }
            }
        });
    }
    in_libc("__fread_chk", |context| {
        // cite: bionic fortify.cpp#__fread_chk
        let stream = match file_of(context, 4) {
            Ok(stream) => stream,
            Err(failure) => {
                finish_long(context, failure);
                return;
            }
        };
        let (size, count) = (context.arg_u64(2), context.arg_u64(3));
        let Some(total) = size.checked_mul(count) else {
            context.set_errno(EINVAL);
            context.ret_i64(0);
            return;
        };
        check_access("fread", "write into", total, context.arg_u64(1));
        match stream::read_into(&stream, context.arg_ptr(0), total as usize) {
            Ok(written) => context.ret_i64((written as u64 / size) as i64),
            Err(errno) => {
                context.set_errno(errno);
                context.ret_i64(0);
            }
        }
    });
    in_libc("__fwrite_chk", |context| {
        let stream = match file_of(context, 4) {
            Ok(stream) => stream,
            Err(failure) => {
                finish_long(context, failure);
                return;
            }
        };
        let (size, count) = (context.arg_u64(2), context.arg_u64(3));
        let Some(total) = size.checked_mul(count) else {
            context.set_errno(EINVAL);
            context.ret_i64(0);
            return;
        };
        check_access("fwrite", "write into", total, context.arg_u64(1));
        match stream::write_bytes(&stream, context.arg_ptr(0), total as usize) {
            Ok(written) => context.ret_i64((written as u64 / size) as i64),
            Err(errno) => {
                context.set_errno(errno);
                context.ret_i64(0);
            }
        }
    });
    for name in ["fgetc", "getc", "getc_unlocked", "fgetc_unlocked"] {
        in_libc(name, move |context: &mut CallContext| {
            let stream = match file_of(context, 0) {
                Ok(stream) => stream,
                Err(failure) => {
                    finish_int(context, failure);
                    return;
                }
            };
            match stream::read_byte(&stream) {
                Ok(Some(byte)) => context.ret_i32(i32::from(byte)),
                Ok(None) => context.ret_i32(-1),
                Err(errno) => {
                    context.set_errno(errno);
                    context.ret_i32(-1);
                }
            }
        });
    }
    for name in ["fputc", "putc", "putc_unlocked", "fputc_unlocked"] {
        in_libc(name, move |context: &mut CallContext| {
            let stream = match file_of(context, 1) {
                Ok(stream) => stream,
                Err(failure) => {
                    finish_int(context, failure);
                    return;
                }
            };
            let byte = [context.arg_i32(0) as u8];
            match stream::write_bytes(&stream, byte.as_ptr() as usize, 1) {
                Ok(1) => context.ret_i32(i32::from(byte[0])),
                Ok(_) => {
                    context.set_errno(EIO);
                    context.ret_i32(-1);
                }
                Err(errno) => {
                    context.set_errno(errno);
                    context.ret_i32(-1);
                }
            }
        });
    }
    for name in ["fgets", "fgets_unlocked"] {
        in_libc(name, move |context: &mut CallContext| {
            let stream = match file_of(context, 2) {
                Ok(stream) => stream,
                Err(failure) => {
                    finish_long(context, failure);
                    return;
                }
            };
            let limit = i64::from(context.arg_i32(1));
            let target = context.arg_ptr(0);
            if limit <= 0 || target == 0 {
                context.set_errno(if target == 0 { EFAULT } else { EINVAL });
                context.ret_ptr(0);
                return;
            }
            if limit == 1 {
                write_value::<u8>(target, 0);
                context.ret_ptr(target);
                return;
            }
            match read_line(&stream, target, limit as usize, None) {
                value if value < 0 => context.ret_ptr(0),
                0 => context.ret_ptr(0),
                _ => context.ret_ptr(target),
            }
        });
    }
    in_libc("__fgets_chk", |context| {
        let stream = match file_of(context, 3) {
            Ok(stream) => stream,
            Err(failure) => {
                finish_long(context, failure);
                return;
            }
        };
        let limit = i64::from(context.arg_i32(2));
        let target = context.arg_ptr(0);
        if limit <= 0 || target == 0 {
            context.ret_ptr(0);
            return;
        }
        check_access("fgets", "write into", limit as u64, context.arg_u64(1));
        match read_line(&stream, target, limit as usize, None) {
            value if value < 0 => context.ret_ptr(0),
            0 => context.ret_ptr(0),
            _ => context.ret_ptr(target),
        }
    });
    for name in ["fputs", "fputs_unlocked"] {
        in_libc(name, move |context: &mut CallContext| {
            let stream = match file_of(context, 1) {
                Ok(stream) => stream,
                Err(failure) => {
                    finish_int(context, failure);
                    return;
                }
            };
            let text = match path_argument(context.arg_ptr(0)) {
                Ok(text) => text,
                Err(errno) => return finish_int(context, error(errno)),
            };
            match stream::write_bytes(&stream, text.as_ptr() as usize, text.len()) {
                Ok(written) if written == text.len() => context.ret_i32(0),
                Ok(_) => {
                    context.set_errno(EIO);
                    context.ret_i32(-1);
                }
                Err(errno) => {
                    context.set_errno(errno);
                    context.ret_i32(-1);
                }
            }
        });
    }
    in_libc("puts", |context| {
        let text = match path_argument(context.arg_ptr(0)) {
            Ok(text) => text,
            Err(errno) => return finish_int(context, error(errno)),
        };
        let stream = match stream_at(standard(1)) {
            Ok(stream) => stream,
            Err(failure) => return finish_int(context, failure),
        };
        let mut bytes = text.to_vec();
        bytes.push(b'\n');
        match stream::write_bytes(&stream, bytes.as_ptr() as usize, bytes.len()) {
            Ok(written) if written == bytes.len() => context.ret_i32(0),
            Ok(_) => {
                context.set_errno(EIO);
                context.ret_i32(-1);
            }
            Err(errno) => {
                context.set_errno(errno);
                context.ret_i32(-1);
            }
        }
    });
    in_libc("getline", |context| {
        let result = get_delimiter(
            context.arg_ptr(0),
            context.arg_ptr(1),
            i32::from(b'\n'),
            context.arg_ptr(2),
        );
        finish_long(context, result);
    });
    in_libc("getdelim", |context| {
        let result = get_delimiter(
            context.arg_ptr(0),
            context.arg_ptr(1),
            context.arg_i32(2),
            context.arg_ptr(3),
        );
        finish_long(context, result);
    });
    in_libc("ungetc", |context| {
        let stream = match file_of(context, 1) {
            Ok(stream) => stream,
            Err(failure) => return finish_int(context, failure),
        };
        let byte = context.arg_i32(0) as u8;
        match stream::unread_byte(&stream, byte) {
            Ok(()) => context.ret_i32(i32::from(byte)),
            Err(errno) => {
                context.set_errno(errno);
                context.ret_i32(-1);
            }
        }
    });
    for name in ["fseek", "fseeko", "fseeko64"] {
        in_libc(name, move |context: &mut CallContext| {
            let stream = match file_of(context, 0) {
                Ok(stream) => stream,
                Err(failure) => return finish_int(context, failure),
            };
            match stream::seek(&stream, context.arg_u64(1) as i64, context.arg_i32(2)) {
                Ok(_) => context.ret_i32(0),
                Err(errno) => {
                    context.set_errno(errno);
                    context.ret_i32(-1);
                }
            }
        });
    }
    for name in ["ftell", "ftello", "ftello64"] {
        in_libc(name, move |context: &mut CallContext| {
            let stream = match file_of(context, 0) {
                Ok(stream) => stream,
                Err(failure) => return finish_long(context, failure),
            };
            match stream::tell(&stream) {
                Ok(position) => context.ret_i64(position),
                Err(errno) => {
                    context.set_errno(errno);
                    context.ret_i64(-1);
                }
            }
        });
    }
    in_libc("fgetpos", |context| {
        let stream = match file_of(context, 0) {
            Ok(stream) => stream,
            Err(failure) => return finish_int(context, failure),
        };
        match stream::tell(&stream) {
            Ok(position) => {
                write_value::<i64>(context.arg_ptr(1), position);
                context.ret_i32(0)
            }
            Err(errno) => {
                context.set_errno(errno);
                context.ret_i32(-1)
            }
        }
    });
    in_libc("fsetpos", |context| {
        let stream = match file_of(context, 0) {
            Ok(stream) => stream,
            Err(failure) => return finish_int(context, failure),
        };
        let position = read_value::<i64>(context.arg_ptr(1));
        match stream::seek(&stream, position, super::linux_abi::seek::SET) {
            Ok(_) => context.ret_i32(0),
            Err(errno) => {
                context.set_errno(errno);
                context.ret_i32(-1)
            }
        }
    });
    in_libc("rewind", |context| {
        let stream = match file_of(context, 0) {
            Ok(stream) => stream,
            Err(failure) => return finish_int(context, failure),
        };
        let _ = stream::seek(&stream, 0, super::linux_abi::seek::SET);
        stream::clear_state(&stream);
    });
    in_libc("fflush", |context| {
        let file = context.arg_ptr(0);
        if file == 0 {
            stream::flush_all();
            context.ret_i32(0);
            return;
        }
        let stream = match stream_at(file) {
            Ok(stream) => stream,
            Err(failure) => return finish_int(context, failure),
        };
        match stream::flush(&stream) {
            Ok(()) => context.ret_i32(0),
            Err(errno) => {
                context.set_errno(errno);
                context.ret_i32(-1)
            }
        }
    });
    in_libc("fpurge", |context| {
        finish_int(context, stream::purge(context.arg_ptr(0)))
    });
    in_libc("__fpurge", |context| {
        finish_int(context, stream::purge(context.arg_ptr(0)))
    });
    for name in ["feof", "feof_unlocked"] {
        in_libc(name, move |context: &mut CallContext| {
            match file_of(context, 0) {
                Ok(stream) => context.ret_i32(i32::from(stream::is_eof(&stream))),
                Err(failure) => finish_int(context, failure),
            }
        });
    }
    for name in ["ferror", "ferror_unlocked"] {
        in_libc(name, move |context: &mut CallContext| {
            match file_of(context, 0) {
                Ok(stream) => context.ret_i32(i32::from(stream::has_error(&stream))),
                Err(failure) => finish_int(context, failure),
            }
        });
    }
    in_libc("clearerr", |context| match file_of(context, 0) {
        Ok(stream) => stream::clear_state(&stream),
        Err(failure) => return finish_int(context, failure),
    });
    in_libc("clearerr_unlocked", |context| match file_of(context, 0) {
        Ok(stream) => stream::clear_state(&stream),
        Err(failure) => return finish_int(context, failure),
    });
    for name in ["fileno", "fileno_unlocked"] {
        in_libc(name, move |context: &mut CallContext| {
            match file_of(context, 0) {
                Ok(stream) => context.ret_i32(stream.fd),
                Err(failure) => finish_int(context, failure),
            }
        });
    }
    in_libc("setvbuf", |context| {
        let result = stream::set_buffer(
            context.arg_ptr(0),
            context.arg_ptr(1),
            context.arg_i32(2),
            context.arg_u64(3) as usize,
        );
        finish_int(context, if result == 0 { 0 } else { -1 });
    });
    in_libc("setbuf", |context| {
        let result = stream::set_buffer(
            context.arg_ptr(0),
            context.arg_ptr(1),
            if context.arg_ptr(1) == 0 {
                stream::IONBF
            } else {
                stream::IOFBF
            },
            stream::BUFSIZ,
        );
        finish_int(context, if result == 0 { 0 } else { -1 });
    });
    in_libc("setbuffer", |context| {
        let result = stream::set_buffer(
            context.arg_ptr(0),
            context.arg_ptr(1),
            if context.arg_ptr(1) == 0 {
                stream::IONBF
            } else {
                stream::IOFBF
            },
            context.arg_u64(2) as usize,
        );
        finish_int(context, if result == 0 { 0 } else { -1 });
    });
    in_libc("setlinebuf", |context| {
        let result = stream::set_buffer(context.arg_ptr(0), 0, stream::IOLBF, stream::BUFSIZ);
        finish_int(context, if result == 0 { 0 } else { -1 });
    });
    in_libc("perror", |context| {
        let prefix = if context.arg_ptr(0) == 0 {
            Vec::new()
        } else {
            cstr_bytes(context.arg_ptr(0)).to_vec()
        };
        let code = super::errno::guest_errno();
        let message = super::errno::message(code)
            .map_or_else(|| format!("Unknown error {code}"), str::to_string);
        let mut line = prefix;
        if !line.is_empty() {
            line.extend_from_slice(b": ");
        }
        line.extend_from_slice(message.as_bytes());
        line.push(b'\n');
        if let Ok(stream) = stream_at(standard(2)) {
            let _ = stream::write_bytes(&stream, line.as_ptr() as usize, line.len());
        }
    });
    in_libc("remove", |context| {
        match path_argument(context.arg_ptr(0)) {
            Ok(path) => {
                let result = super::fs_ops::unlink_at(at::FDCWD, path, 0);
                if result == error(21) {
                    finish_int(
                        context,
                        super::fs_ops::unlink_at(at::FDCWD, path, at::REMOVEDIR),
                    );
                } else {
                    finish_int(context, result);
                }
            }
            Err(errno) => finish_int(context, error(errno)),
        }
    });
    for name in ["tmpfile", "tmpfile64"] {
        in_libc(name, move |context: &mut CallContext| {
            let fd = super::temp_file::anonymous_file();
            if fd < 0 {
                context.set_errno((-fd) as i32);
                context.ret_ptr(0);
                return;
            }
            let stream = stream::create(
                fd as i32,
                true,
                true,
                stream::IOFBF,
                false,
                super::linux_abi::stdio::SRD | super::linux_abi::stdio::SWR,
            );
            let file = stream::allocate(&stream);
            if file == 0 {
                super::fs_close_dup::close_fd(fd as i32);
                context.set_errno(ENOMEM);
                context.ret_ptr(0);
                return;
            }
            context.ret_ptr(file);
        });
    }
    for name in ["popen", "pclose"] {
        in_libc(name, move |context: &mut CallContext| {
            missing(name, "this runtime has no host subprocess layer");
            context.set_errno(ENOENT);
            context.ret_i32(-1);
        });
    }
    in_libc("fwide", |context| {
        // cite: bionic fwide.c#fwide
        let _ = context.arg_i32(1);
        context.ret_i32(-1);
    });
    for name in ["flockfile", "ftrylockfile"] {
        in_libc(name, move |context: &mut CallContext| {
            if stream::of(context.arg_ptr(0)).is_err() {
                context.set_errno(EBADF);
                context.ret_i32(-1);
            } else {
                context.ret_i32(0);
            }
        });
    }
    in_libc("funlockfile", |context| {
        if stream::of(context.arg_ptr(0)).is_err() {
            context.set_errno(EBADF);
        }
    });
}
