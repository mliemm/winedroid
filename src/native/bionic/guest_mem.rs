use std::ffi::{c_char, CStr};

pub fn cstr_bytes<'a>(pointer: usize) -> &'a [u8] {
    unsafe { CStr::from_ptr(pointer as *const c_char) }.to_bytes()
}

pub fn cstr_bytes_bounded<'a>(pointer: usize, limit: usize) -> &'a [u8] {
    let length = unsafe { libc::strnlen(pointer as *const c_char, limit) };
    unsafe { std::slice::from_raw_parts(pointer as *const u8, length) }
}

pub fn cstr_lossy(pointer: usize) -> String {
    String::from_utf8_lossy(cstr_bytes(pointer)).into_owned()
}

pub fn optional_cstr(pointer: usize) -> Option<&'static [u8]> {
    (pointer != 0).then(|| cstr_bytes(pointer))
}

pub fn write_value<T: Copy>(address: usize, value: T) {
    unsafe { std::ptr::write_unaligned(address as *mut T, value) };
}

pub fn read_value<T: Copy>(address: usize) -> T {
    unsafe { std::ptr::read_unaligned(address as *const T) }
}

pub fn write_bytes(address: usize, bytes: &[u8]) {
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), address as *mut u8, bytes.len()) };
}

pub fn write_cstr_truncated(address: usize, capacity: usize, text: &[u8]) -> usize {
    if capacity == 0 {
        return text.len();
    }
    let copied = text.len().min(capacity - 1);
    write_bytes(address, &text[..copied]);
    write_value::<u8>(address + copied, 0);
    text.len()
}

static INTERNED: std::sync::Mutex<Option<std::collections::HashMap<String, usize>>> = std::sync::Mutex::new(None);

pub fn intern(text: &str) -> usize {
    let mut guard = INTERNED.lock().expect("interned strings");
    let map = guard.get_or_insert_with(std::collections::HashMap::new);
    *map.entry(text.to_string()).or_insert_with(|| Box::leak(Box::new(std::ffi::CString::new(text).unwrap_or_default())).as_ptr() as usize)
}
