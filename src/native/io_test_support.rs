use std::ffi::CString;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use super::call_assign::CallBuilder;
use super::guest_context::{use_on_this_thread, GuestContext};
use super::thread_test_support::shim;
use crate::external_storage::{ExternalStoragePolicy, ExternalStorageRuntimeAccess};

pub const PACKAGE: &str = "com.example.io";

pub struct IoGuest {
    pub prefix: PathBuf,
}

impl IoGuest {
    pub fn new(name: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let prefix = std::env::temp_dir().join(format!("winedroid-io-{name}-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        for part in ["data", "data_de"] {
            std::fs::create_dir_all(prefix.join("apps").join(PACKAGE).join(part)).unwrap();
        }
        std::fs::create_dir_all(prefix.join("storage")).unwrap();
        std::fs::create_dir_all(prefix.join("apps").join(PACKAGE).join("native/arm64-v8a")).unwrap();
        std::fs::write(prefix.join("apps").join(PACKAGE).join("source.apk"), b"apk bytes").unwrap();
        let access = ExternalStorageRuntimeAccess::with_legacy_file_access(Vec::new(), true, true, true);
        let policy = Arc::new(RwLock::new(ExternalStoragePolicy::new(28, 28, Vec::new(), access)));
        let context = GuestContext::open(&prefix, PACKAGE, policy).unwrap();
        use_on_this_thread(Arc::new(context));
        super::bridge_test_support::register_all();
        super::bionic::guest_process::set_cwd("/");
        Self { prefix }
    }

    pub fn guest(&self, relative: &str) -> String {
        format!("/data/user/0/{PACKAGE}/{relative}")
    }

    pub fn host(&self, relative: &str) -> PathBuf {
        self.prefix.join("apps").join(PACKAGE).join("data").join(relative)
    }
}

impl Drop for IoGuest {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.prefix).ok();
    }
}

pub struct Text(CString);

impl Text {
    pub fn new(text: &str) -> Self {
        Self(CString::new(text).unwrap())
    }

    pub fn ptr(&self) -> usize {
        self.0.as_ptr() as usize
    }
}

pub fn call(name: &str) -> CallBuilder {
    shim(name)
}

pub fn open(path: &str, flags: i32, mode: u32) -> i32 {
    let text = Text::new(path);
    call("open").ptr(text.ptr()).i32(flags).u32(mode).invoke().unwrap().i32()
}

pub fn guest_errno() -> i32 {
    unsafe { *libc::__error() }
}

pub fn write_all(fd: i32, bytes: &[u8]) -> i64 {
    call("write").i32(fd).ptr(bytes.as_ptr() as usize).int(bytes.len() as u64).invoke().unwrap().u64() as i64
}

pub fn read_some(fd: i32, length: usize) -> Vec<u8> {
    let mut buffer = vec![0u8; length];
    let count = call("read").i32(fd).ptr(buffer.as_mut_ptr() as usize).int(length as u64).invoke().unwrap().u64() as i64;
    assert!(count >= 0, "read failed with errno {}", guest_errno());
    buffer.truncate(count as usize);
    buffer
}

pub fn close(fd: i32) -> i32 {
    call("close").i32(fd).invoke().unwrap().i32()
}
