use std::cell::RefCell;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRecord {
    pub buffer: i32,
    pub priority: i32,
    pub tag: String,
    pub file: Option<String>,
    pub line: u32,
    pub message: String,
}

thread_local! {
    static CAPTURE: RefCell<Option<Vec<LogRecord>>> = const { RefCell::new(None) };
}

pub fn start_capture() {
    CAPTURE.with(|capture| *capture.borrow_mut() = Some(Vec::new()));
}

pub fn take_capture() -> Vec<LogRecord> {
    CAPTURE.with(|capture| capture.borrow_mut().take()).unwrap_or_default()
}

pub fn priority_letter(priority: i32) -> char {
    // cite: bionic async_safe_log.cpp#async_safe_write_log
    const LETTERS: &[u8; 9] = b"XXVDIWEFS";
    let index = if priority < 0 { 0 } else { (priority as usize).min(LETTERS.len() - 1) };
    LETTERS[index] as char
}

fn timestamp() -> String {
    let mut now: libc::time_t = 0;
    let mut parts: libc::tm = unsafe { std::mem::zeroed() };
    let mut text = [0u8; 32];
    unsafe {
        libc::time(&mut now);
        libc::localtime_r(&now, &mut parts);
        let written = libc::strftime(text.as_mut_ptr() as *mut libc::c_char, text.len(), c"%m-%d %H:%M:%S".as_ptr(), &parts);
        String::from_utf8_lossy(&text[..written]).into_owned()
    }
}

fn stderr_line(record: &LogRecord) -> String {
    let (pid, mut tid) = (unsafe { libc::getpid() }, 0u64);
    unsafe { libc::pthread_threadid_np(0, &mut tid) };
    let location = record.file.as_ref().map(|file| format!(" {file}:{}", record.line)).unwrap_or_default();
    format!("{} {} {} {:5} {:5}{}] {}\n", record.tag, priority_letter(record.priority), timestamp(), pid, tid, location, record.message)
}

pub fn write_stderr(record: &LogRecord) {
    let line = stderr_line(record);
    unsafe { libc::write(2, line.as_ptr() as *const libc::c_void, line.len()) };
}

pub fn emit(record: LogRecord) {
    let captured = CAPTURE.with(|capture| match capture.borrow_mut().as_mut() {
        Some(records) => {
            records.push(record.clone());
            true
        }
        None => false,
    });
    if !captured {
        write_stderr(&record);
    }
}
