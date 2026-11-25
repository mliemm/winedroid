use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use super::abi_types::{EBADF, EINVAL};
use super::guest_mem::{read_value, write_value};
use super::linux_abi::{open, stdio as flags};

pub const FILE_SIZE: usize = 152;
pub const EOF: i64 = -1;
pub const BUFSIZ: usize = 1024;
pub const IOFBF: i32 = 0;
pub const IOLBF: i32 = 1;
pub const IONBF: i32 = 2;

pub const FLAGS_OFFSET: usize = 16;
pub const FD_OFFSET: usize = 20;
pub const COOKIE_OFFSET: usize = 48;
const BUFFER_BASE_OFFSET: usize = 24;
const BUFFER_SIZE_OFFSET: usize = 32;
const BLOCKSIZE_OFFSET: usize = 136;
const MAX_PUSHBACK: usize = 8;

pub struct Buffers {
    pub read: Vec<u8>,
    pub read_position: usize,
    pub write: Vec<u8>,
    pub pushback: Vec<u8>,
    pub capacity: usize,
    pub guest_buffer: usize,
}

pub struct Stream {
    pub fd: i32,
    pub readable: bool,
    pub writable: bool,
    pub standard: bool,
    pub eof: AtomicBool,
    pub error: AtomicBool,
    pub buffering: AtomicI32,
    pub file_flags: AtomicI32,
    pub buffers: Mutex<Buffers>,
}

static STREAMS: OnceLock<Mutex<HashMap<u64, Arc<Stream>>>> = OnceLock::new();
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn streams() -> &'static Mutex<HashMap<u64, Arc<Stream>>> {
    STREAMS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn error(code: i32) -> i64 {
    -i64::from(code)
}

pub fn create(fd: i32, readable: bool, writable: bool, buffering: i32, standard: bool, file_flags: i32) -> Arc<Stream> {
    let capacity = match buffering {
        IONBF => 1,
        _ => BUFSIZ,
    };
    let stream = Arc::new(Stream {
        fd,
        readable,
        writable,
        standard,
        eof: AtomicBool::new(false),
        error: AtomicBool::new(false),
        buffering: AtomicI32::new(buffering),
        file_flags: AtomicI32::new(file_flags),
        buffers: Mutex::new(Buffers { read: Vec::new(), read_position: 0, write: Vec::new(), pushback: Vec::new(), capacity, guest_buffer: 0 }),
    });
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    streams().lock().expect("stdio streams").insert(id, stream.clone());
    stream
}

pub fn attach(file: usize, stream: &Arc<Stream>) {
    // cite: bionic local.h#__sFILE
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    streams().lock().expect("stdio streams").insert(id, stream.clone());
    write_value::<i32>(file + FLAGS_OFFSET, stream.file_flags.load(Ordering::SeqCst));
    write_value::<i32>(file + FD_OFFSET, stream.fd);
    write_value::<usize>(file + BUFFER_BASE_OFFSET, 0);
    write_value::<usize>(file + BUFFER_SIZE_OFFSET, stream.buffers.lock().expect("stdio buffer").capacity);
    write_value::<i32>(file + BLOCKSIZE_OFFSET, 4096);
    write_value::<u64>(file + COOKIE_OFFSET, id);
}

pub fn allocate(stream: &Arc<Stream>) -> usize {
    let file = unsafe { libc::calloc(1, FILE_SIZE) } as usize;
    if file != 0 {
        attach(file, stream);
    }
    file
}

pub fn of(file: usize) -> Result<Arc<Stream>, i32> {
    if file == 0 {
        return Err(EINVAL);
    }
    let id = read_value::<u64>(file + COOKIE_OFFSET);
    if id == 0 {
        return Err(EINVAL);
    }
    streams().lock().expect("stdio streams").get(&id).cloned().ok_or(EBADF)
}

pub fn detach(file: usize) {
    let id = read_value::<u64>(file + COOKIE_OFFSET);
    if id != 0 {
        streams().lock().expect("stdio streams").remove(&id);
    }
    write_value::<u32>(file + FLAGS_OFFSET, 0);
    write_value::<i32>(file + FD_OFFSET, -1);
    write_value::<u64>(file + COOKIE_OFFSET, 0);
}

pub fn all() -> Vec<Arc<Stream>> {
    streams().lock().expect("stdio streams").values().cloned().collect()
}

pub fn mode_of(mode: &[u8]) -> Result<(i32, bool, bool), i32> {
    // cite: bionic stdio.cpp#fopen
    let Some(first) = mode.first() else { return Err(EINVAL) };
    let (mut access, mut readable, mut writable) = match first {
        b'r' => (open::RDONLY, true, false),
        b'w' => (open::WRONLY | open::CREAT | open::TRUNC, false, true),
        b'a' => (open::WRONLY | open::CREAT | open::APPEND, false, true),
        _ => return Err(EINVAL),
    };
    let mut seen_plus = false;
    for byte in &mode[1..] {
        match byte {
            b'+' if !seen_plus => {
                seen_plus = true;
                access = (access & !open::ACCMODE) | open::RDWR;
                readable = true;
                writable = true;
            }
            b'e' => access |= open::CLOEXEC,
            b'x' => access |= open::EXCL,
            b'b' | b'm' => {}
            _ => return Err(EINVAL),
        }
    }
    Ok((access, readable, writable))
}

fn fill(stream: &Stream, buffers: &mut Buffers) -> Result<bool, i32> {
    let capacity = buffers.capacity.max(1);
    let mut chunk = vec![0u8; capacity];
    let result = super::fs_io::read_fd(stream.fd, chunk.as_mut_ptr() as usize, capacity);
    if result < 0 {
        stream.error.store(true, Ordering::SeqCst);
        return Err((-result) as i32);
    }
    if result == 0 {
        stream.eof.store(true, Ordering::SeqCst);
        return Ok(false);
    }
    chunk.truncate(result as usize);
    if buffers.guest_buffer != 0 {
        super::guest_mem::write_bytes(buffers.guest_buffer, &chunk);
    }
    buffers.read = chunk;
    buffers.read_position = 0;
    Ok(true)
}

pub fn read_byte(stream: &Stream) -> Result<Option<u8>, i32> {
    if !stream.readable {
        return Err(EBADF);
    }
    let mut buffers = stream.buffers.lock().expect("stdio buffer");
    if let Some(byte) = buffers.pushback.pop() {
        return Ok(Some(byte));
    }
    if buffers.read_position >= buffers.read.len() && !fill(stream, &mut buffers)? {
        return Ok(None);
    }
    let byte = buffers.read[buffers.read_position];
    buffers.read_position += 1;
    Ok(Some(byte))
}

pub fn unread_byte(stream: &Stream, byte: u8) -> Result<(), i32> {
    if !stream.readable {
        return Err(EBADF);
    }
    let mut buffers = stream.buffers.lock().expect("stdio buffer");
    if buffers.pushback.len() >= MAX_PUSHBACK {
        return Err(EINVAL);
    }
    buffers.pushback.push(byte);
    stream.eof.store(false, Ordering::SeqCst);
    Ok(())
}

pub fn read_into(stream: &Stream, target: usize, count: usize) -> Result<usize, i32> {
    if !stream.readable {
        return Err(EBADF);
    }
    let mut buffers = stream.buffers.lock().expect("stdio buffer");
    let mut done = 0usize;
    while done < count {
        if let Some(byte) = buffers.pushback.pop() {
            write_value::<u8>(target + done, byte);
            done += 1;
            continue;
        }
        if buffers.read_position < buffers.read.len() {
            let available = (buffers.read.len() - buffers.read_position).min(count - done);
            super::guest_mem::write_bytes(target + done, &buffers.read[buffers.read_position..buffers.read_position + available]);
            buffers.read_position += available;
            done += available;
            continue;
        }
        if count - done >= buffers.capacity.max(1) {
            let wanted = count - done;
            let result = super::fs_io::read_fd(stream.fd, target + done, wanted);
            if result < 0 {
                stream.error.store(true, Ordering::SeqCst);
                return if done > 0 { Ok(done) } else { Err((-result) as i32) };
            }
            if result == 0 {
                stream.eof.store(true, Ordering::SeqCst);
            }
            done += result as usize;
            if (result as usize) < wanted {
                break;
            }
            continue;
        }
        if !fill(stream, &mut buffers)? {
            break;
        }
    }
    Ok(done)
}

fn raw_write(stream: &Stream, source: usize, count: usize) -> Result<usize, i32> {
    let result = super::fs_io::write_fd(stream.fd, source, count);
    if result < 0 {
        stream.error.store(true, Ordering::SeqCst);
        Err((-result) as i32)
    } else {
        Ok(result as usize)
    }
}

fn flush_buffer(stream: &Stream, buffers: &mut Buffers) -> Result<(), i32> {
    if buffers.write.is_empty() {
        return Ok(());
    }
    let mut done = 0usize;
    while done < buffers.write.len() {
        match raw_write(stream, buffers.write.as_ptr() as usize + done, buffers.write.len() - done) {
            Ok(0) => return Err(super::abi_types::EIO),
            Ok(written) => done += written,
            Err(errno) => return Err(errno),
        }
    }
    buffers.write.clear();
    Ok(())
}

pub fn flush(stream: &Stream) -> Result<(), i32> {
    let mut buffers = stream.buffers.lock().expect("stdio buffer");
    flush_buffer(stream, &mut buffers)
}

pub fn flush_all() {
    for stream in all() {
        let _ = flush(&stream);
    }
}

pub fn write_bytes(stream: &Stream, source: usize, count: usize) -> Result<usize, i32> {
    if !stream.writable {
        return Err(EBADF);
    }
    let buffering = stream.buffering.load(Ordering::SeqCst);
    if buffering == IONBF {
        let mut done = 0usize;
        while done < count {
            match raw_write(stream, source + done, count - done) {
                Ok(0) => return Err(super::abi_types::EIO),
                Ok(written) => done += written,
                Err(errno) => return Err(errno),
            }
        }
        return Ok(count);
    }
    let mut buffers = stream.buffers.lock().expect("stdio buffer");
    let capacity = buffers.capacity.max(1);
    let mut done = 0usize;
    while done < count {
        let space = capacity.saturating_sub(buffers.write.len()).max(1);
        let take = (count - done).min(space);
        let bytes = unsafe { std::slice::from_raw_parts((source + done) as *const u8, take) };
        buffers.write.extend_from_slice(bytes);
        done += take;
        if buffers.write.len() >= capacity {
            flush_buffer(stream, &mut buffers)?;
        }
    }
    if buffering == IOLBF {
        if let Some(position) = buffers.write.iter().rposition(|byte| *byte == b'\n') {
            let remainder = buffers.write.split_off(position + 1);
            flush_buffer(stream, &mut buffers)?;
            buffers.write = remainder;
        }
    }
    Ok(count)
}

pub fn seek(stream: &Stream, offset: i64, whence: i32) -> Result<i64, i32> {
    flush(stream)?;
    let mut buffers = stream.buffers.lock().expect("stdio buffer");
    buffers.read.clear();
    buffers.read_position = 0;
    buffers.pushback.clear();
    let result = super::fs_io::lseek_fd(stream.fd, offset, whence);
    if result < 0 {
        stream.error.store(true, Ordering::SeqCst);
        return Err((-result) as i32);
    }
    stream.eof.store(false, Ordering::SeqCst);
    Ok(result)
}

pub fn tell(stream: &Stream) -> Result<i64, i32> {
    let buffers = stream.buffers.lock().expect("stdio buffer");
    let pending_write = buffers.write.len() as i64;
    let unread = (buffers.read.len() - buffers.read_position) as i64 + buffers.pushback.len() as i64;
    drop(buffers);
    let position = super::fs_io::lseek_fd(stream.fd, 0, super::linux_abi::seek::CUR);
    if position < 0 {
        return Err((-position) as i32);
    }
    Ok(position - unread + pending_write)
}

pub fn close_file(file: usize) -> i64 {
    let stream = match of(file) {
        Ok(stream) => stream,
        Err(errno) => return error(errno),
    };
    let flushed = flush(&stream);
    let closed = super::fs_close_dup::close_fd(stream.fd);
    detach(file);
    if !stream.standard {
        unsafe { libc::free(file as *mut libc::c_void) };
    }
    match flushed {
        Err(errno) => error(errno),
        Ok(()) if closed < 0 => closed,
        Ok(()) => 0,
    }
}

pub fn set_buffer(file: usize, buffer: usize, mode: i32, size: usize) -> i64 {
    let stream = match of(file) {
        Ok(stream) => stream,
        Err(errno) => return error(errno),
    };
    if !matches!(mode, IOFBF | IOLBF | IONBF) || size > i32::MAX as usize {
        return error(EINVAL);
    }
    let mut buffers = stream.buffers.lock().expect("stdio buffer");
    if let Err(errno) = flush_buffer(&stream, &mut buffers) {
        return error(errno);
    }
    buffers.read.clear();
    buffers.read_position = 0;
    buffers.pushback.clear();
    buffers.capacity = match mode {
        IONBF => 1,
        _ if size == 0 => BUFSIZ,
        _ => size,
    };
    buffers.guest_buffer = buffer;
    stream.buffering.store(mode, Ordering::SeqCst);
    let file_flags = match mode {
        IONBF => flags::SNBF,
        IOLBF => flags::SLBF,
        _ => 0,
    };
    let access_flags = (if stream.readable { flags::SRD } else { 0 }) | (if stream.writable { flags::SWR } else { 0 });
    stream.file_flags.store(file_flags | access_flags, Ordering::SeqCst);
    write_value::<i32>(file + FLAGS_OFFSET, stream.file_flags.load(Ordering::SeqCst));
    0
}

pub fn purge(file: usize) -> i64 {
    match of(file) {
        Ok(stream) => {
            let mut buffers = stream.buffers.lock().expect("stdio buffer");
            buffers.read.clear();
            buffers.read_position = 0;
            buffers.write.clear();
            buffers.pushback.clear();
            stream.eof.store(false, Ordering::SeqCst);
            stream.error.store(false, Ordering::SeqCst);
            0
        }
        Err(errno) => error(errno),
    }
}

pub fn set_error(file: usize, value: bool) {
    if let Ok(stream) = of(file) {
        stream.error.store(value, Ordering::SeqCst);
    }
}

pub fn is_eof(stream: &Stream) -> bool {
    stream.eof.load(Ordering::SeqCst)
}

pub fn has_error(stream: &Stream) -> bool {
    stream.error.load(Ordering::SeqCst)
}

pub fn clear_state(stream: &Stream) {
    stream.eof.store(false, Ordering::SeqCst);
    stream.error.store(false, Ordering::SeqCst);
}

pub fn buffering_for(fd: i32) -> i32 {
    if unsafe { libc::isatty(fd) } == 1 {
        IOLBF
    } else {
        IOFBF
    }
}
