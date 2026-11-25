use std::cell::RefCell;
use std::ffi::{c_char, CString};

use crate::error::{Result, WinedroidError};

use super::call_assign::CallBuilder;
use super::guest_thread::attach_current;
use super::host_log;
use super::trampoline::{spk_call_guest, spk_unwind_to, GuestCallFrame};

#[derive(Debug, Clone, Copy)]
pub struct GuestProcessArgs {
    pub argc: i32,
    argv: usize,
    envp: usize,
}

impl GuestProcessArgs {
    pub fn new() -> Self {
        // cite: bionic linker_soinfo.cpp#call_constructors
        let program = CString::new("/system/bin/app_process64").expect("no interior nul").into_raw();
        let argv: &'static mut [*mut c_char; 2] = Box::leak(Box::new([program, std::ptr::null_mut()]));
        let envp: &'static mut [*mut c_char; 1] = Box::leak(Box::new([std::ptr::null_mut()]));
        Self { argc: 1, argv: argv.as_ptr() as usize, envp: envp.as_ptr() as usize }
    }
}

impl Default for GuestProcessArgs {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GuestResult {
    x: [u64; 2],
    q: [[u64; 2]; 4],
}

impl GuestResult {
    pub fn i32(&self) -> i32 {
        self.x[0] as i32
    }

    pub fn u64(&self) -> u64 {
        self.x[0]
    }

    pub fn pair(&self) -> (u64, u64) {
        (self.x[0], self.x[1])
    }

    pub fn f32(&self) -> f32 {
        f32::from_bits(self.q[0][0] as u32)
    }

    pub fn f64(&self) -> f64 {
        f64::from_bits(self.q[0][0])
    }

    pub fn quad(&self) -> u128 {
        u128::from(self.q[0][0]) | (u128::from(self.q[0][1]) << 64)
    }

    pub fn hfa_f32(&self, members: usize) -> Vec<f32> {
        self.q[..members].iter().map(|register| f32::from_bits(register[0] as u32)).collect()
    }
}

thread_local! {
    static ACTIVE: RefCell<Vec<*mut GuestCallFrame>> = const { RefCell::new(Vec::new()) };
    static ABORT: RefCell<Option<String>> = const { RefCell::new(None) };
}

pub fn invoke_frame(frame: &mut GuestCallFrame) -> Result<GuestResult> {
    attach_current()?;
    frame.aborted = 0;
    ACTIVE.with(|active| active.borrow_mut().push(frame));
    unsafe { spk_call_guest(frame) };
    ACTIVE.with(|active| active.borrow_mut().pop());
    if frame.aborted != 0 {
        let message = ABORT.with(|abort| abort.borrow_mut().take()).unwrap_or_else(|| "guest call aborted".to_string());
        return Err(WinedroidError::new(message));
    }
    Ok(GuestResult { x: frame.ret_x, q: frame.ret_q })
}

#[cfg(test)]
pub(super) fn active_depth() -> usize {
    ACTIVE.with(|active| active.borrow().len())
}

fn current_thread_label() -> String {
    let mut host_id = 0u64;
    unsafe { libc::pthread_threadid_np(0, &mut host_id) };
    let thread = std::thread::current();
    format!("thread {} (id {host_id})", thread.name().unwrap_or("unnamed"))
}

pub fn abort_active_call(message: String) {
    let frame = ACTIVE.with(|active| active.borrow().last().copied());
    let Some(frame) = frame else {
        host_log::line(&format!("{message}; no active guest call on {}", current_thread_label()));
        std::process::abort();
    };
    ABORT.with(|abort| *abort.borrow_mut() = Some(message));
    let saved = unsafe {
        (*frame).aborted = 1;
        (*frame).frame
    };
    unsafe { spk_unwind_to(saved) }
}

pub fn call_guest(address: usize, args: &[u64]) -> Result<GuestResult> {
    let mut call = CallBuilder::new(address);
    for argument in args {
        call = call.int(*argument);
    }
    call.invoke()
}

pub fn call_constructor(address: usize, args: &GuestProcessArgs) -> Result<()> {
    CallBuilder::new(address).i32(args.argc).ptr(args.argv).ptr(args.envp).invoke().map(|_| ())
}

pub fn call_init(address: usize) -> Result<()> {
    // cite: bionic linker_soinfo.cpp#call_function
    CallBuilder::new(address).invoke().map(|_| ())
}

pub fn call_destructor(address: usize) -> Result<()> {
    CallBuilder::new(address).invoke().map(|_| ())
}
