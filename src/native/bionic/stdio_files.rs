use super::export::data_in_libc;
use super::guest_mem::write_value;

const FILE_SIZE: usize = 152;
const FLAGS_OFFSET: usize = 16;
const FILE_OFFSET: usize = 20;
const COOKIE_OFFSET: usize = 48;
const READING: i32 = 0x0004;
const WRITING: i32 = 0x0008;
const UNBUFFERED: i32 = 0x0002;

pub fn register() {
    // cite: bionic stdio.cpp#MAKE_STD_STREAM, __sF
    let files = data_in_libc("__sF", 3 * FILE_SIZE);
    for (index, flags) in [READING, WRITING, WRITING | UNBUFFERED].into_iter().enumerate() {
        let file = files + index * FILE_SIZE;
        write_value::<i32>(file + FLAGS_OFFSET, flags);
        write_value::<i32>(file + FILE_OFFSET, index as i32);
        write_value::<usize>(file + COOKIE_OFFSET, file);
    }
    for (index, name) in ["stdin", "stdout", "stderr"].into_iter().enumerate() {
        let cell = data_in_libc(name, 8);
        write_value::<usize>(cell, files + index * FILE_SIZE);
    }
}
