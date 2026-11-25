use std::ffi::{c_char, CString};

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

pub fn call_constructor(address: usize, args: &GuestProcessArgs) {
    let function: extern "C" fn(i32, usize, usize) = unsafe { std::mem::transmute(address) };
    function(args.argc, args.argv, args.envp);
}

pub fn call_init(address: usize) {
    // cite: bionic linker_soinfo.cpp#call_function
    let function: extern "C" fn() = unsafe { std::mem::transmute(address) };
    function();
}

pub fn call_destructor(address: usize) {
    let function: extern "C" fn() = unsafe { std::mem::transmute(address) };
    function();
}
