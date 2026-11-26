use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CString;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Mutex;

use super::super::bridge::CallContext;
use super::super::call_assign::CallBuilder;
use super::super::dl_api::{dlerror, set_dl_error, DlHandle, PhdrInfo};
use super::super::loader::Loader;
use super::super::runtime::NativeRuntime;
use super::abi_types::{DL_INFO_SIZE, DL_PHDR_INFO_SIZE};
use super::export::in_libdl;
use super::fatal::fatal;
use super::guest_mem::{optional_cstr, read_value, write_bytes, write_cstr_truncated, write_value};
use crate::error::WinedroidError;

const ERROR_BUFFER: usize = 512;
const TARGET_SDK_FUTURE: i32 = 10000;
const RTLD_VALID: i32 = super::super::module::RTLD_FLAGS_MASK | 0x0;
const EXT_VALID_BITS: u64 = 0x1 | 0x2 | 0x4 | 0x8 | 0x10 | 0x20 | 0x40 | 0x200 | 0x400;
const EXT_USE_LIBRARY_FD: u64 = 0x10;
const EXT_USE_LIBRARY_FD_OFFSET: u64 = 0x20;
const EXT_USE_NAMESPACE: u64 = 0x200;

static TARGET_SDK: AtomicI32 = AtomicI32::new(TARGET_SDK_FUTURE);
static STRINGS: Mutex<Option<HashMap<String, usize>>> = Mutex::new(None);

thread_local! {
    static ERROR_TEXT: RefCell<[u8; ERROR_BUFFER]> = const { RefCell::new([0; ERROR_BUFFER]) };
}

pub fn set_application_target_sdk_version(version: i32) {
    // cite: bionic linker_sdk_versions.cpp#set_application_target_sdk_version
    TARGET_SDK.store(
        if version == 0 {
            TARGET_SDK_FUTURE
        } else {
            version
        },
        Ordering::SeqCst,
    );
}

fn runtime(context: &mut CallContext) -> Option<&'static NativeRuntime> {
    let runtime = NativeRuntime::current();
    if runtime.is_none() {
        context.fail(format!("{} native runtime is not initialized; next missing primitive: native runtime initialization before dl calls", context.symbol()));
    }
    runtime
}

fn interned(text: &str) -> usize {
    let mut guard = STRINGS.lock().expect("dl strings");
    let map = guard.get_or_insert_with(HashMap::new);
    *map.entry(text.to_string()).or_insert_with(|| {
        Box::leak(Box::new(CString::new(text).unwrap_or_default())).as_ptr() as usize
    })
}

fn name_argument(address: usize) -> Option<String> {
    optional_cstr(address).map(|bytes| String::from_utf8_lossy(bytes).into_owned())
}

fn open(loader: &Loader, name: Option<&str>, flags: i32, caller: usize) -> u64 {
    loader.dlopen(name, flags, Some(caller)).unwrap_or(0)
}

fn dlopen_handler(context: &mut CallContext) {
    // cite: bionic dlfcn.cpp#__loader_dlopen
    let Some(runtime) = runtime(context) else {
        return;
    };
    let handle = open(
        runtime.loader(),
        name_argument(context.arg_ptr(0)).as_deref(),
        context.arg_i32(1),
        context.return_address() as usize,
    );
    context.ret_u64(handle);
}

fn extended_flags_error(flags: u64, namespace: u64) -> Option<String> {
    // cite: bionic linker.cpp#do_dlopen
    if flags & !EXT_VALID_BITS != 0 {
        return Some(format!(
            "invalid extended flags to android_dlopen_ext: {flags:#x}"
        ));
    }
    if flags & EXT_USE_LIBRARY_FD_OFFSET != 0 && flags & EXT_USE_LIBRARY_FD == 0 {
        return Some(format!(
            "invalid extended flag combination (ANDROID_DLEXT_USE_LIBRARY_FD_OFFSET without ANDROID_DLEXT_USE_LIBRARY_FD): {flags:#x}"
        ));
    }
    if flags & EXT_USE_NAMESPACE != 0 && namespace == 0 {
        return Some(
            "ANDROID_DLEXT_USE_NAMESPACE is set but extinfo->library_namespace is null".to_string(),
        );
    }
    None
}

fn android_dlopen_ext_handler(context: &mut CallContext) {
    // cite: bionic dlfcn.cpp#__loader_android_dlopen_ext
    let Some(runtime) = runtime(context) else {
        return;
    };
    let (flags, extinfo) = (context.arg_i32(1), context.arg_ptr(2));
    let ext_flags = if extinfo != 0 {
        read_value::<u64>(extinfo)
    } else {
        0
    };
    let problem = if flags & !RTLD_VALID != 0 {
        Some(format!("invalid flags to dlopen: {flags:x}"))
    } else if extinfo != 0 {
        extended_flags_error(ext_flags, read_value::<u64>(extinfo + 40)).or_else(|| {
            (ext_flags != 0).then(|| format!("android_dlopen_ext flag {ext_flags:#x} is not implemented; next missing primitive: android_dlopen_ext flag {ext_flags:#x}"))
        })
    } else {
        None
    };
    if let Some(message) = problem {
        let _ = set_dl_error::<()>("dlopen failed: ", Err(WinedroidError::new(message)));
        context.ret_u64(0);
        return;
    }
    let handle = open(
        runtime.loader(),
        name_argument(context.arg_ptr(0)).as_deref(),
        flags,
        context.return_address() as usize,
    );
    context.ret_u64(handle);
}

fn dlsym_handler(context: &mut CallContext) {
    // cite: bionic dlfcn.cpp#__loader_dlsym
    let Some(runtime) = runtime(context) else {
        return;
    };
    let Some(name) = name_argument(context.arg_ptr(1)) else {
        let _ = set_dl_error::<()>(
            "",
            Err(WinedroidError::new("dlsym failed: symbol name is null")),
        );
        context.ret_u64(0);
        return;
    };
    let address = runtime
        .loader()
        .dlsym(
            DlHandle::from_raw(context.arg_u64(0)),
            &name,
            Some(context.return_address() as usize),
        )
        .unwrap_or(0);
    context.ret_ptr(address);
}

fn dlerror_handler(context: &mut CallContext) {
    // cite: bionic dlfcn.cpp#__loader_dlerror
    match dlerror() {
        None => context.ret_ptr(0),
        Some(message) => ERROR_TEXT.with(|buffer| {
            let mut buffer = buffer.borrow_mut();
            let address = buffer.as_mut_ptr() as usize;
            write_cstr_truncated(address, ERROR_BUFFER, message.as_bytes());
            context.ret_ptr(address);
        }),
    }
}

fn dladdr_handler(context: &mut CallContext) {
    // cite: bionic linker.cpp#do_dladdr
    let Some(runtime) = runtime(context) else {
        return;
    };
    let (address, destination) = (context.arg_ptr(0), context.arg_ptr(1));
    let Some(info) = runtime.loader().dladdr(address) else {
        context.ret_i32(0);
        return;
    };
    let mut block = [0u8; DL_INFO_SIZE];
    block[..8].copy_from_slice(&(interned(&info.path) as u64).to_le_bytes());
    block[8..16].copy_from_slice(&(info.base as u64).to_le_bytes());
    if let Some((name, location)) = &info.symbol {
        block[16..24].copy_from_slice(&(interned(name) as u64).to_le_bytes());
        block[24..32].copy_from_slice(&(location.to_owned() as u64).to_le_bytes());
    }
    write_bytes(destination, &block);
    context.ret_i32(1);
}

fn phdr_block(info: &PhdrInfo, loads: u64, unloads: u64) -> [u8; DL_PHDR_INFO_SIZE] {
    // cite: bionic link.h#dl_phdr_info
    let mut block = [0u8; DL_PHDR_INFO_SIZE];
    block[..8].copy_from_slice(&(info.addr as u64).to_le_bytes());
    block[8..16].copy_from_slice(&(interned(&info.name) as u64).to_le_bytes());
    block[16..24].copy_from_slice(&(info.phdr as u64).to_le_bytes());
    block[24..26].copy_from_slice(&info.phnum.to_le_bytes());
    block[32..40].copy_from_slice(&loads.to_le_bytes());
    block[40..48].copy_from_slice(&unloads.to_le_bytes());
    block[48..56].copy_from_slice(&(info.tls_modid as u64).to_le_bytes());
    block[56..64].copy_from_slice(&(info.tls_data as u64).to_le_bytes());
    block
}

fn iterate_phdr_handler(context: &mut CallContext) {
    // cite: bionic dlfcn.cpp#__loader_dl_iterate_phdr
    let Some(runtime) = runtime(context) else {
        return;
    };
    let (callback, data) = (context.arg_ptr(0), context.arg_u64(1));
    let mut failure = None;
    let result = runtime.loader().dl_iterate_phdr(&mut |info| {
        let block = phdr_block(info, info.adds, info.subs);
        match CallBuilder::new(callback)
            .ptr(block.as_ptr() as usize)
            .int(DL_PHDR_INFO_SIZE as u64)
            .int(data)
            .invoke()
        {
            Ok(outcome) => outcome.i32(),
            Err(error) => {
                failure = Some(error.to_string());
                -1
            }
        }
    });
    match failure {
        Some(message) => context.fail(message),
        None => context.ret_i32(result),
    }
}

fn ld_library_path_handler(context: &mut CallContext) {
    // cite: bionic dlfcn.cpp#__loader_android_get_LD_LIBRARY_PATH
    let Some(runtime) = runtime(context) else {
        return;
    };
    let (buffer, length) = (context.arg_ptr(0), context.arg_u64(1) as usize);
    let paths: Vec<String> = runtime
        .library_dirs()
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    let required: usize = paths.iter().map(|path| path.len() + 1).sum();
    if length < required {
        fatal(None, &format!("android_get_LD_LIBRARY_PATH failed, buffer too small: buffer len {length}, required len {required}"));
    }
    if !paths.is_empty() {
        let joined = paths.join(":");
        write_bytes(buffer, joined.as_bytes());
        write_value::<u8>(buffer + joined.len(), 0);
    }
}

pub fn register() {
    in_libdl("dlopen", dlopen_handler);
    in_libdl("android_dlopen_ext", android_dlopen_ext_handler);
    in_libdl("dlsym", dlsym_handler);
    in_libdl("dlvsym", dlsym_handler);
    in_libdl("dlerror", dlerror_handler);
    in_libdl("dladdr", dladdr_handler);
    in_libdl("dl_iterate_phdr", iterate_phdr_handler);
    in_libdl("dlclose", |context| {
        let Some(runtime) = runtime(context) else {
            return;
        };
        context.ret_i32(if runtime.loader().dlclose(context.arg_u64(0)).is_ok() {
            0
        } else {
            -1
        });
    });
    in_libdl("android_get_LD_LIBRARY_PATH", ld_library_path_handler);
    in_libdl("android_get_application_target_sdk_version", |context| {
        // cite: bionic linker_sdk_versions.cpp#get_application_target_sdk_version
        context.ret_i32(TARGET_SDK.load(Ordering::SeqCst));
    });
}
