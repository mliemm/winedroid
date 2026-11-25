use std::cell::RefCell;
use std::ffi::{c_char, CStr};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::error::Result;

use super::dl_api::{set_dl_error, DlHandle};
use super::link_resolution::{ImportKind, SystemLibraryProvider};
use super::loader::Loader;

thread_local! {
    static CURRENT: RefCell<Option<Arc<Loader>>> = const { RefCell::new(None) };
    static RECORDED: RefCell<Vec<i32>> = const { RefCell::new(Vec::new()) };
    static ATEXIT: RefCell<Vec<(usize, usize, usize)>> = const { RefCell::new(Vec::new()) };
}

pub fn take_atexit() -> Vec<(usize, usize, usize)> {
    ATEXIT.with(|registered| std::mem::take(&mut *registered.borrow_mut()))
}

extern "C" fn record_stub(value: i32) {
    RECORDED.with(|recorded| recorded.borrow_mut().push(value));
}

pub fn take_recorded() -> Vec<i32> {
    RECORDED.with(|recorded| std::mem::take(&mut *recorded.borrow_mut()))
}

pub fn dlopen_without_constructors(loader: &Loader, name: &str) -> Result<u64> {
    let guard = loader.lock.lock();
    let result = loader.find_libraries(&guard, name, 2).map(|root| guard.borrow().module(root).handle);
    set_dl_error("dlopen failed: ", result)
}

pub static MUTEX_CALLS: AtomicUsize = AtomicUsize::new(0);

pub fn fixture_dir(variant: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/native_fixtures/bin").join(variant)
}

pub fn corpus_dir() -> PathBuf {
    std::env::var_os("WINEDROID_TIKTOK_LIBS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tiktok-libs/lib/arm64-v8a"))
}

extern "C" fn mutex_stub(_mutex: *mut u8) -> i32 {
    MUTEX_CALLS.fetch_add(1, Ordering::SeqCst);
    0
}

extern "C" fn atexit_stub(function: usize, argument: usize, dso: usize) -> i32 {
    ATEXIT.with(|registered| registered.borrow_mut().push((function, argument, dso)));
    0
}

extern "C" fn stack_chk_fail() {
    std::process::abort();
}

extern "C" fn unresolved_import() {
    std::process::abort();
}

extern "C" fn dlopen_shim(name: *const c_char, flags: i32) -> u64 {
    let name = unsafe { CStr::from_ptr(name) }.to_str().unwrap().to_string();
    CURRENT.with(|current| current.borrow().as_ref().unwrap().dlopen(Some(&name), flags, None).unwrap_or(0))
}

extern "C" fn dlsym_shim(handle: u64, name: *const c_char) -> usize {
    let name = unsafe { CStr::from_ptr(name) }.to_str().unwrap().to_string();
    CURRENT.with(|current| current.borrow().as_ref().unwrap().dlsym(DlHandle::from_raw(handle), &name, None).unwrap_or(0))
}

extern "C" fn dlclose_shim(handle: u64) -> i32 {
    CURRENT.with(|current| if current.borrow().as_ref().unwrap().dlclose(handle).is_ok() { 0 } else { -1 })
}

pub struct TestProvider {
    pub unresolved_calls: Mutex<Vec<(String, String, ImportKind)>>,
}

impl TestProvider {
    pub fn new() -> Self {
        Self { unresolved_calls: Mutex::new(Vec::new()) }
    }
}

impl SystemLibraryProvider for TestProvider {
    fn export(&self, soname: &str, name: &str) -> Option<usize> {
        if soname != "libc.so" && soname != "libdl.so" {
            return None;
        }
        Some(match name {
            "strlen" => libc::strlen as *const () as usize,
            "memcpy" => libc::memcpy as *const () as usize,
            "malloc" => libc::malloc as *const () as usize,
            "free" => libc::free as *const () as usize,
            "pthread_mutex_lock" | "pthread_mutex_unlock" => mutex_stub as *const () as usize,
            "__cxa_atexit" => atexit_stub as *const () as usize,
            "__stack_chk_fail" => stack_chk_fail as *const () as usize,
            "__fx_record" => record_stub as *const () as usize,
            "dlopen" => dlopen_shim as *const () as usize,
            "dlsym" => dlsym_shim as *const () as usize,
            "dlclose" => dlclose_shim as *const () as usize,
            _ => return None,
        })
    }

    fn unresolved(&self, importer: &str, name: &str, kind: ImportKind) -> usize {
        self.unresolved_calls.lock().unwrap().push((importer.to_string(), name.to_string(), kind));
        unresolved_import as *const () as usize
    }
}

pub fn unresolved_import_address() -> usize {
    unresolved_import as *const () as usize
}

pub fn host_function_address(name: &str) -> usize {
    TestProvider::new().export("libc.so", name).unwrap()
}

pub fn loader_for(dirs: Vec<PathBuf>) -> Arc<Loader> {
    let loader = Arc::new(Loader::new(dirs, Box::new(TestProvider::new())).with_strict(false));
    CURRENT.with(|current| *current.borrow_mut() = Some(loader.clone()));
    loader
}

extern "C" {
    fn mach_vm_region(task: u32, address: *mut u64, size: *mut u64, flavor: i32, info: *mut i32, count: *mut u32, object_name: *mut u32) -> i32;
}

pub fn host_protection_of(address: usize) -> i32 {
    const VM_REGION_BASIC_INFO_64: i32 = 9;
    let mut at = address as u64;
    let (mut size, mut info, mut count, mut object) = (0u64, [0i32; 9], 9u32, 0u32);
    let status = unsafe { mach_vm_region(super::mach_task::current(), &mut at, &mut size, VM_REGION_BASIC_INFO_64, info.as_mut_ptr(), &mut count, &mut object) };
    assert_eq!(status, 0, "mach_vm_region({address:#x})");
    assert!(at as usize <= address, "{address:#x} is not mapped");
    info[0]
}

fn run_child_work(work: impl FnOnce()) {
    if let Err(panic) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)) {
        let text = panic.downcast_ref::<String>().cloned().or_else(|| panic.downcast_ref::<&str>().map(|text| text.to_string())).unwrap_or_default();
        super::host_log::line(&format!("child test body panicked: {text}"));
        unsafe { libc::_exit(101) };
    }
}

pub fn terminating_signal_of_child(work: impl FnOnce()) -> Option<i32> {
    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        run_child_work(work);
        unsafe { libc::_exit(0) };
    }
    let mut status = 0;
    assert_eq!(unsafe { libc::waitpid(pid, &mut status, 0) }, pid);
    libc::WIFSIGNALED(status).then(|| libc::WTERMSIG(status))
}

pub fn child_outcome(work: impl FnOnce()) -> (Option<i32>, String) {
    let (signal, _, output) = child_result(work);
    (signal, output)
}

pub fn child_result(work: impl FnOnce()) -> (Option<i32>, Option<i32>, String) {
    let mut ends = [0i32; 2];
    assert_eq!(unsafe { libc::pipe(ends.as_mut_ptr()) }, 0);
    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        unsafe {
            libc::close(ends[0]);
            libc::dup2(ends[1], 2);
        }
        run_child_work(work);
        unsafe { libc::_exit(0) };
    }
    unsafe { libc::close(ends[1]) };
    let mut output = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let count = unsafe { libc::read(ends[0], chunk.as_mut_ptr() as *mut _, chunk.len()) };
        if count <= 0 {
            break;
        }
        output.extend_from_slice(&chunk[..count as usize]);
    }
    unsafe { libc::close(ends[0]) };
    let mut status = 0;
    assert_eq!(unsafe { libc::waitpid(pid, &mut status, 0) }, pid);
    let signal = libc::WIFSIGNALED(status).then(|| libc::WTERMSIG(status));
    let code = libc::WIFEXITED(status).then(|| libc::WEXITSTATUS(status));
    (signal, code, String::from_utf8_lossy(&output).into_owned())
}

pub fn child_exit_code(work: impl FnOnce()) -> Option<i32> {
    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        run_child_work(work);
        unsafe { libc::_exit(0) };
    }
    let mut status = 0;
    assert_eq!(unsafe { libc::waitpid(pid, &mut status, 0) }, pid);
    libc::WIFEXITED(status).then(|| libc::WEXITSTATUS(status))
}

pub fn is_mapped(address: usize) -> bool {
    const VM_REGION_BASIC_INFO_64: i32 = 9;
    let mut at = address as u64;
    let (mut size, mut info, mut count, mut object) = (0u64, [0i32; 9], 9u32, 0u32);
    let status = unsafe { mach_vm_region(super::mach_task::current(), &mut at, &mut size, VM_REGION_BASIC_INFO_64, info.as_mut_ptr(), &mut count, &mut object) };
    status == 0 && at as usize <= address
}
