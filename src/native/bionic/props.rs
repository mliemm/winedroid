use std::collections::BTreeMap;
use std::ffi::CString;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

use super::super::bridge::CallContext;
use super::super::call_assign::CallBuilder;
use super::abi_types::{PROP_INFO_HEADER, PROP_NAME_MAX, PROP_VALUE_MAX};
use super::device_profile as profile;
use super::export::{data_in_libc, in_libc};
use super::guest_mem::{cstr_bytes, optional_cstr, read_value, write_bytes, write_cstr_truncated, write_value};
use super::log_sink::{emit, LogRecord};

const LONG_FLAG: u32 = 1 << 16;
const LONG_LEGACY_MESSAGE: &str = "Must use __system_property_read_callback() to read";
const LONG_OFFSET_FIELD: usize = 60;
const SET_DENIED: u32 = 0x14;
const LOG_WARN: i32 = 5;

struct Property {
    info: usize,
    value: Vec<u8>,
}

struct Store {
    properties: BTreeMap<Vec<u8>, Property>,
}

static STORE: OnceLock<Mutex<Store>> = OnceLock::new();
static AREA_SERIAL: AtomicU32 = AtomicU32::new(0);

fn tokens(name: &[u8]) -> Vec<(usize, Vec<u8>)> {
    name.split(|byte| *byte == b'.').map(|token| (token.len(), token.to_vec())).collect()
}

fn has_empty_token(name: &[u8]) -> bool {
    name.split(|byte| *byte == b'.').any(<[u8]>::is_empty)
}

fn allocate(name: &[u8], value: &[u8]) -> usize {
    // cite: bionic prop_info.h#prop_info
    let long = value.len() >= PROP_VALUE_MAX && name.starts_with(b"ro.");
    let size = PROP_INFO_HEADER + name.len() + 1;
    let info = unsafe { libc::calloc(1, size) } as usize;
    assert_ne!(info, 0, "property allocation failed");
    let serial = if long {
        (LONG_LEGACY_MESSAGE.len() as u32) << 24 | LONG_FLAG
    } else {
        (value.len() as u32) << 24
    };
    write_value::<u32>(info, serial);
    if long {
        write_cstr_truncated(info + 4, 56, LONG_LEGACY_MESSAGE.as_bytes());
        write_value::<u32>(info + LONG_OFFSET_FIELD, 0);
    } else {
        write_cstr_truncated(info + 4, PROP_VALUE_MAX, value);
    }
    write_bytes(info + PROP_INFO_HEADER, name);
    info
}

fn host_zone_id() -> Option<String> {
    let target = std::fs::read_link("/etc/localtime").ok()?;
    let text = target.to_string_lossy().into_owned();
    text.split_once("zoneinfo/").map(|(_, zone)| zone.to_string())
}

fn seed(store: &mut Store) {
    let mut entries: Vec<(&str, String)> = vec![
        ("ro.build.version.sdk", profile::SDK.into()),
        ("ro.build.version.release", profile::RELEASE.into()),
        ("ro.build.version.codename", profile::CODENAME.into()),
        ("ro.build.fingerprint", profile::FINGERPRINT.into()),
        ("ro.build.display.id", profile::DISPLAY_ID.into()),
        ("ro.product.brand", profile::BRAND.into()),
        ("ro.product.manufacturer", profile::MANUFACTURER.into()),
        ("ro.product.model", profile::MODEL.into()),
        ("ro.product.device", profile::DEVICE.into()),
        ("ro.product.name", profile::PRODUCT.into()),
        ("ro.product.board", profile::BOARD.into()),
        ("ro.hardware", profile::HARDWARE.into()),
        ("ro.product.cpu.abi", profile::CPU_ABI.into()),
        ("ro.product.cpu.abilist", profile::CPU_ABI.into()),
        ("dalvik.vm.heapsize", "512m".into()),
        ("dalvik.vm.heapgrowthlimit", "256m".into()),
    ];
    if let Some(zone) = host_zone_id() {
        entries.push(("persist.sys.timezone", zone));
    }
    for (name, value) in entries {
        add(store, name.as_bytes(), value.as_bytes());
    }
}

fn add(store: &mut Store, name: &[u8], value: &[u8]) {
    let info = allocate(name, value);
    store.properties.insert(name.to_vec(), Property { info, value: value.to_vec() });
    AREA_SERIAL.fetch_add(1, Ordering::SeqCst);
}

fn store() -> &'static Mutex<Store> {
    STORE.get_or_init(|| {
        let mut store = Store { properties: BTreeMap::new() };
        seed(&mut store);
        Mutex::new(store)
    })
}

pub fn get(name: &str) -> Option<String> {
    let store = store().lock().expect("property store");
    store.properties.get(name.as_bytes()).map(|property| String::from_utf8_lossy(&property.value).into_owned())
}

fn find(name: &[u8]) -> usize {
    // cite: bionic prop_area.cpp#find_property
    if has_empty_token(name) {
        return 0;
    }
    store().lock().expect("property store").properties.get(name).map_or(0, |property| property.info)
}

fn ordered() -> Vec<(usize, Vec<u8>)> {
    // cite: bionic prop_area.cpp#foreach_property
    let store = store().lock().expect("property store");
    let mut listing: Vec<(&Vec<u8>, &Property)> = store.properties.iter().collect();
    listing.sort_by_key(|(name, _)| tokens(name));
    listing.into_iter().map(|(name, property)| (property.info, name.clone())).collect()
}

pub fn ordered_names() -> Vec<String> {
    ordered().into_iter().map(|(_, name)| String::from_utf8_lossy(&name).into_owned()).collect()
}

fn value_of(info: usize) -> Option<Vec<u8>> {
    let store = store().lock().expect("property store");
    store.properties.values().find(|property| property.info == info).map(|property| property.value.clone())
}

fn read_into(info: usize, name_buffer: usize, value_buffer: usize) -> u32 {
    // cite: bionic system_properties.cpp#Read
    let serial = read_value::<u32>(info);
    let copied = cstr_bytes(info + 4);
    write_cstr_truncated(value_buffer, PROP_VALUE_MAX, copied);
    if name_buffer != 0 {
        write_cstr_truncated(name_buffer, PROP_NAME_MAX, cstr_bytes(info + PROP_INFO_HEADER));
    }
    serial >> 24
}

fn property_get(context: &mut CallContext) {
    // cite: bionic system_properties.cpp#Get
    let (name, value_buffer) = (optional_cstr(context.arg_ptr(0)).unwrap_or(b""), context.arg_ptr(1));
    let info = find(name);
    if info == 0 {
        write_value::<u8>(value_buffer, 0);
        context.ret_i32(0);
        return;
    }
    context.ret_i32(read_into(info, 0, value_buffer) as i32);
}

fn read_callback(context: &mut CallContext) {
    // cite: bionic system_properties.cpp#ReadCallback
    let (info, callback, cookie) = (context.arg_ptr(0), context.arg_ptr(1), context.arg_u64(2));
    let Some(value) = value_of(info) else {
        context.fail("__system_property_read_callback got a prop_info that is not in the property store".to_string());
        return;
    };
    let name = CString::new(cstr_bytes(info + PROP_INFO_HEADER)).unwrap_or_default();
    let value = CString::new(value).unwrap_or_default();
    let serial = read_value::<u32>(info);
    let outcome = CallBuilder::new(callback).int(cookie).ptr(name.as_ptr() as usize).ptr(value.as_ptr() as usize).u32(serial).invoke();
    if let Err(error) = outcome {
        context.fail(error.to_string());
    }
}

fn foreach(context: &mut CallContext) {
    let (callback, cookie) = (context.arg_ptr(0), context.arg_u64(1));
    for (info, _) in ordered() {
        if let Err(error) = CallBuilder::new(callback).ptr(info).int(cookie).invoke() {
            context.fail(error.to_string());
            return;
        }
    }
    context.ret_i32(0);
}

fn property_set(context: &mut CallContext) {
    // cite: bionic system_property_set.cpp#__system_property_set
    let Some(key) = optional_cstr(context.arg_ptr(0)) else {
        context.ret_i32(-1);
        return;
    };
    let value = optional_cstr(context.arg_ptr(1)).unwrap_or(b"");
    let message = format!(
        "Unable to set property \"{}\" to \"{}\": error code: {:#x}",
        String::from_utf8_lossy(key),
        String::from_utf8_lossy(value),
        SET_DENIED
    );
    emit(LogRecord { buffer: 0, priority: LOG_WARN, tag: "libc".to_string(), file: None, line: 0, message });
    context.ret_i32(-1);
}

fn property_wait(context: &mut CallContext) {
    // cite: bionic system_properties.cpp#Wait
    let (info, old_serial, new_serial_out, timeout) = (context.arg_ptr(0), context.arg_u32(1), context.arg_ptr(2), context.arg_ptr(3));
    let current = if info == 0 { AREA_SERIAL.load(Ordering::SeqCst) } else { read_value::<u32>(info) };
    if current != old_serial {
        if new_serial_out != 0 {
            write_value::<u32>(new_serial_out, current);
        }
        context.ret_i32(1);
        return;
    }
    if timeout == 0 {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3600));
        }
    }
    let wanted = std::time::Duration::new(read_value::<i64>(timeout).max(0) as u64, read_value::<i64>(timeout + 8).clamp(0, 999_999_999) as u32);
    std::thread::sleep(wanted);
    context.ret_i32(0);
}

pub fn register() {
    data_in_libc("__system_property_area__", 8);
    in_libc("__system_property_find", |context| context.ret_ptr(optional_cstr(context.arg_ptr(0)).map_or(0, find)));
    in_libc("__system_property_get", property_get);
    in_libc("__system_property_read", |context| {
        let length = read_into(context.arg_ptr(0), context.arg_ptr(1), context.arg_ptr(2));
        context.ret_i32(length as i32);
    });
    in_libc("__system_property_read_callback", read_callback);
    in_libc("__system_property_serial", |context| context.ret_u32(read_value::<u32>(context.arg_ptr(0))));
    in_libc("__system_property_area_serial", |context| context.ret_u32(AREA_SERIAL.load(Ordering::SeqCst)));
    in_libc("__system_property_find_nth", |context| {
        let index = context.arg_u32(0) as usize;
        context.ret_ptr(ordered().get(index).map_or(0, |(info, _)| *info));
    });
    in_libc("__system_property_foreach", foreach);
    in_libc("__system_property_set", property_set);
    in_libc("__system_property_wait", property_wait);
}
