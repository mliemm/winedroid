use std::cell::RefCell;
use std::ffi::{c_char, CStr};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use super::dl_api::DlHandle;
use super::link_resolution::{ImportKind, SystemLibraryProvider};
use super::loader::Loader;

thread_local! {
    static CURRENT: RefCell<Option<Arc<Loader>>> = const { RefCell::new(None) };
}

pub static MUTEX_CALLS: AtomicUsize = AtomicUsize::new(0);
pub static ATEXIT: Mutex<Vec<(usize, usize, usize)>> = Mutex::new(Vec::new());

pub fn fixture_dir(variant: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/native_fixtures/bin").join(variant)
}

pub fn corpus_dir() -> PathBuf {
    PathBuf::from("/private/tmp/claude-501/-Users-mliem-dev-GitHub/72d22855-6f45-48eb-8f6a-a82fc02a6831/scratchpad/tiktok-libs/lib/arm64-v8a")
}

extern "C" fn mutex_stub(_mutex: *mut u8) -> i32 {
    MUTEX_CALLS.fetch_add(1, Ordering::SeqCst);
    0
}

extern "C" fn atexit_stub(function: usize, argument: usize, dso: usize) -> i32 {
    ATEXIT.lock().unwrap().push((function, argument, dso));
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
