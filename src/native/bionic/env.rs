use std::ffi::{c_char, CString};
use std::sync::{Mutex, OnceLock};

use super::abi_types::EINVAL;
use super::device_profile::PROGRAM_NAME;
use super::export::{data_in_libc, in_libc};
use super::guest_mem::{cstr_bytes, read_value, write_value};

const INITIAL: &[&str] = &[
    "ANDROID_ROOT=/system",
    "ANDROID_DATA=/data",
    "ANDROID_STORAGE=/storage",
    "ANDROID_ART_ROOT=/apex/com.android.art",
    "ANDROID_I18N_ROOT=/apex/com.android.i18n",
    "ANDROID_TZDATA_ROOT=/apex/com.android.tzdata",
    "ANDROID_RUNTIME_ROOT=/apex/com.android.runtime",
    "EXTERNAL_STORAGE=/sdcard",
    "ASEC_MOUNTPOINT=/mnt/asec",
    "DOWNLOAD_CACHE=/data/cache",
    "PATH=/product/bin:/apex/com.android.runtime/bin:/apex/com.android.art/bin:/system_ext/bin:/system/bin:/system/xbin:/odm/bin:/vendor/bin:/vendor/xbin",
];

struct Environment {
    cell: usize,
    owned_array: usize,
}

static ENVIRONMENT: OnceLock<Mutex<Environment>> = OnceLock::new();
static PROGRAM_CELL: OnceLock<usize> = OnceLock::new();

fn environment() -> &'static Mutex<Environment> {
    ENVIRONMENT
        .get()
        .expect("environment is registered before first use")
}

fn entries(cell: usize) -> Vec<usize> {
    let mut array = read_value::<usize>(cell);
    let mut found = Vec::new();
    if array == 0 {
        return found;
    }
    while read_value::<usize>(array) != 0 {
        found.push(read_value::<usize>(array));
        array += 8;
    }
    found
}

fn publish(environment: &mut Environment, list: &[usize]) {
    let array = unsafe { libc::malloc((list.len() + 1) * 8) } as usize;
    assert_ne!(array, 0, "environment array allocation failed");
    for (index, entry) in list.iter().enumerate() {
        write_value::<usize>(array + index * 8, *entry);
    }
    write_value::<usize>(array + list.len() * 8, 0);
    write_value::<usize>(environment.cell, array);
    if environment.owned_array != 0 {
        unsafe { libc::free(environment.owned_array as *mut libc::c_void) };
    }
    environment.owned_array = array;
}

fn entry_matches(entry: usize, name: &[u8]) -> bool {
    let text = cstr_bytes(entry);
    text.len() > name.len() && &text[..name.len()] == name && text[name.len()] == b'='
}

fn valid_name(name: &[u8]) -> bool {
    !name.is_empty() && !name.contains(&b'=')
}

fn replace_or_append(environment: &mut Environment, name: &[u8], entry: usize) {
    // cite: bionic setenv.c#putenv, setenv
    let mut list = entries(environment.cell);
    let mut placed = false;
    list.retain_mut(|existing| {
        if !entry_matches(*existing, name) {
            return true;
        }
        if placed {
            return false;
        }
        *existing = entry;
        placed = true;
        true
    });
    if !placed {
        list.push(entry);
    }
    publish(environment, &list);
}

fn getenv(name: &[u8]) -> usize {
    // cite: bionic getenv.c#getenv
    let environment = environment().lock().expect("environment");
    entries(environment.cell)
        .into_iter()
        .find(|entry| entry_matches(*entry, name))
        .map_or(0, |entry| entry + name.len() + 1)
}

pub fn lookup(name: &[u8]) -> Option<Vec<u8>> {
    let pointer = getenv(name);
    (pointer != 0).then(|| cstr_bytes(pointer).to_vec())
}

fn setenv(name: &[u8], value: &[u8], overwrite: bool) -> i32 {
    // cite: bionic setenv.c#setenv
    if !valid_name(name) {
        return -EINVAL;
    }
    let mut environment = environment().lock().expect("environment");
    if !overwrite
        && entries(environment.cell)
            .iter()
            .any(|entry| entry_matches(*entry, name))
    {
        return 0;
    }
    let mut text = name.to_vec();
    text.push(b'=');
    text.extend_from_slice(value);
    text.push(0);
    let entry = unsafe { libc::malloc(text.len()) } as usize;
    assert_ne!(entry, 0, "environment entry allocation failed");
    super::guest_mem::write_bytes(entry, &text);
    replace_or_append(&mut environment, name, entry);
    0
}

fn unsetenv(name: &[u8]) -> i32 {
    // cite: bionic setenv.c#unsetenv
    if !valid_name(name) {
        return -EINVAL;
    }
    let mut environment = environment().lock().expect("environment");
    let list: Vec<usize> = entries(environment.cell)
        .into_iter()
        .filter(|entry| !entry_matches(*entry, name))
        .collect();
    publish(&mut environment, &list);
    0
}

fn putenv(text: usize) -> i32 {
    // cite: bionic setenv.c#putenv
    let bytes = cstr_bytes(text);
    let Some(equals) = bytes
        .iter()
        .position(|byte| *byte == b'=')
        .filter(|position| *position > 0)
    else {
        return -EINVAL;
    };
    let mut environment = environment().lock().expect("environment");
    replace_or_append(&mut environment, &bytes[..equals], text);
    0
}

fn clearenv() {
    // cite: bionic clearenv.cpp#clearenv
    let mut environment = environment().lock().expect("environment");
    write_value::<usize>(environment.cell, 0);
    if environment.owned_array != 0 {
        unsafe { libc::free(environment.owned_array as *mut libc::c_void) };
        environment.owned_array = 0;
    }
}

fn finish_status(context: &mut super::super::bridge::CallContext, status: i32) {
    if status < 0 {
        context.set_errno(-status);
        context.ret_i32(-1);
    } else {
        context.ret_i32(status);
    }
}

fn set_program_name(name: &[u8]) {
    let base = name.rsplit(|byte| *byte == b'/').next().unwrap_or(name);
    let text = CString::new(base).unwrap_or_default();
    let leaked: &'static CString = Box::leak(Box::new(text));
    write_value::<usize>(
        *PROGRAM_CELL.get().expect("program cell"),
        leaked.as_ptr() as *const c_char as usize,
    );
}

pub fn program_name() -> String {
    String::from_utf8_lossy(cstr_bytes(read_value::<usize>(
        *PROGRAM_CELL.get().expect("program cell"),
    )))
    .into_owned()
}

pub fn register() {
    let cell = data_in_libc("environ", 8);
    let program = data_in_libc("__progname", 8);
    let _ = PROGRAM_CELL.set(program);
    let _ = ENVIRONMENT.set(Mutex::new(Environment {
        cell,
        owned_array: 0,
    }));
    set_program_name(PROGRAM_NAME.as_bytes());
    {
        let mut guard = environment().lock().expect("environment");
        let list: Vec<usize> = INITIAL
            .iter()
            .map(|text| {
                let entry = CString::new(*text).expect("no interior nul");
                Box::leak(Box::new(entry)).as_ptr() as usize
            })
            .collect();
        publish(&mut guard, &list);
    }
    in_libc("getenv", |context| {
        context.ret_ptr(super::guest_mem::optional_cstr(context.arg_ptr(0)).map_or(0, getenv))
    });
    in_libc("setenv", |context| {
        let (Some(name), Some(value)) = (
            super::guest_mem::optional_cstr(context.arg_ptr(0)),
            super::guest_mem::optional_cstr(context.arg_ptr(1)),
        ) else {
            finish_status(context, -EINVAL);
            return;
        };
        let status = setenv(name, value, context.arg_i32(2) != 0);
        finish_status(context, status);
    });
    in_libc("unsetenv", |context| {
        let status = super::guest_mem::optional_cstr(context.arg_ptr(0)).map_or(-EINVAL, unsetenv);
        finish_status(context, status);
    });
    in_libc("putenv", |context| {
        let status = if context.arg_ptr(0) == 0 {
            -EINVAL
        } else {
            putenv(context.arg_ptr(0))
        };
        finish_status(context, status);
    });
    in_libc("clearenv", |context| {
        clearenv();
        context.ret_i32(0);
    });
    in_libc("getprogname", |context| {
        context.ret_ptr(read_value::<usize>(
            *PROGRAM_CELL.get().expect("program cell"),
        ))
    });
    in_libc("setprogname", |context| {
        if let Some(name) = super::guest_mem::optional_cstr(context.arg_ptr(0)) {
            set_program_name(name);
        }
    });
}
