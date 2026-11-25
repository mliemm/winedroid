pub fn line(message: &str) {
    let text = format!("{message}\n");
    unsafe { libc::write(2, text.as_ptr() as *const libc::c_void, text.len()) };
}
