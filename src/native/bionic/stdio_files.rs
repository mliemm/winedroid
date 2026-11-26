use std::sync::OnceLock;

use super::export::data_in_libc;
use super::guest_mem::write_value;
use super::linux_abi::stdio as flags;
use super::stream;

static STANDARD: OnceLock<[usize; 3]> = OnceLock::new();

pub fn standard(index: usize) -> usize {
    STANDARD.get().copied().unwrap_or([0; 3])[index]
}

pub fn register() {
    // cite: bionic stdio.cpp#MAKE_STD_STREAM, __sF
    let files = data_in_libc("__sF", 3 * stream::FILE_SIZE);
    let stdout_buffering = if unsafe { libc::isatty(1) } == 1 {
        stream::IOLBF
    } else {
        stream::IOFBF
    };
    let specs = [
        (true, false, stream::IOFBF, flags::SRD),
        (false, true, stdout_buffering, flags::SWR),
        (false, true, stream::IONBF, flags::SWR | flags::SNBF),
    ];
    let mut addresses = [0usize; 3];
    for (index, (readable, writable, buffering, file_flags)) in specs.into_iter().enumerate() {
        let file = files + index * stream::FILE_SIZE;
        let stream = stream::create(
            index as i32,
            readable,
            writable,
            buffering,
            true,
            file_flags,
        );
        stream::attach(file, &stream);
        addresses[index] = file;
    }
    STANDARD.set(addresses).ok();
    for (index, name) in ["stdin", "stdout", "stderr"].into_iter().enumerate() {
        let cell = data_in_libc(name, 8);
        write_value::<usize>(cell, addresses[index]);
    }
}
