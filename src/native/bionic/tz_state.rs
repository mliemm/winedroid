use std::sync::{Arc, Mutex, OnceLock};

use super::abi_types::{EOVERFLOW, TM_SIZE};
use super::civil::{from_seconds, read_tm, to_seconds, write_tm};
use super::env::lookup;
use super::export::{data_in_libc, in_libc};
use super::guest_mem::{intern, read_value, write_value};
use super::props::get as property;
use super::time_handlers::{asctime_into, leaked_buffer};
use super::tzif::{parse, parse_posix, Zone};

const ZONE_DIRECTORIES: [&str; 2] = ["/usr/share/zoneinfo", "/var/db/timezone/zoneinfo"];

struct State {
    name: Option<Vec<u8>>,
    zone: Arc<Zone>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);
static TZNAME: OnceLock<usize> = OnceLock::new();
static TIMEZONE: OnceLock<usize> = OnceLock::new();
static DAYLIGHT: OnceLock<usize> = OnceLock::new();
static LOCALTIME_BUFFER: OnceLock<usize> = OnceLock::new();
static CTIME_STATIC: OnceLock<usize> = OnceLock::new();

fn selected_name() -> Vec<u8> {
    // cite: bionic bionic.cpp#tzset_unlocked
    if let Some(name) = lookup(b"TZ") {
        return name;
    }
    match property("persist.sys.timezone") {
        Some(mut name) if name.len() > 3 => {
            let sign = name.as_bytes()[3];
            if sign == b'-' || sign == b'+' {
                name.replace_range(3..4, if sign == b'-' { "+" } else { "-" });
            }
            name.into_bytes()
        }
        Some(name) => name.into_bytes(),
        None => b"GMT".to_vec(),
    }
}

fn load_file(name: &str) -> Option<Zone> {
    if name.is_empty() || name.starts_with('/') || name.contains("..") {
        return None;
    }
    ZONE_DIRECTORIES.iter().find_map(|directory| {
        std::fs::read(format!("{directory}/{name}"))
            .ok()
            .and_then(|bytes| parse(&bytes))
    })
}

fn load_zone(name: &[u8]) -> Zone {
    // cite: bionic localtime.c#tzsetlcl
    let text = String::from_utf8_lossy(name).into_owned();
    let (explicit, plain) = match text.strip_prefix(':') {
        Some(rest) => (true, rest.to_string()),
        None => (false, text),
    };
    load_file(&plain)
        .or_else(|| {
            if explicit {
                None
            } else {
                parse_posix(&plain).map(Zone::from_posix)
            }
        })
        .unwrap_or_else(Zone::utc)
}

fn publish_names(zone: &Zone) {
    let (standard, daylight) = zone.standard_and_daylight();
    write_value::<usize>(
        *TZNAME.get().expect("tzname registered"),
        intern(&standard.abbreviation),
    );
    write_value::<usize>(
        *TZNAME.get().expect("tzname registered") + 8,
        intern(&daylight.as_ref().unwrap_or(&standard).abbreviation),
    );
    write_value::<i64>(
        *TIMEZONE.get().expect("timezone registered"),
        -i64::from(standard.offset),
    );
    write_value::<i32>(
        *DAYLIGHT.get().expect("daylight registered"),
        i32::from(daylight.is_some()),
    );
}

pub fn current_zone() -> Arc<Zone> {
    let name = selected_name();
    let mut guard = STATE.lock().expect("time zone state");
    let reload = guard
        .as_ref()
        .is_none_or(|state| state.name.as_deref() != Some(&name));
    if reload {
        let zone = Arc::new(load_zone(&name));
        publish_names(&zone);
        *guard = Some(State {
            name: Some(name),
            zone,
        });
    }
    guard.as_ref().expect("state was just set").zone.clone()
}

fn fill_local(address: usize, seconds: i64, zone: &Zone) -> Result<(), i32> {
    let kind = zone.at(seconds);
    let parts = from_seconds(
        seconds
            .checked_add(i64::from(kind.offset))
            .ok_or(EOVERFLOW)?,
    )?;
    write_tm(
        address,
        &parts,
        i32::from(kind.is_dst),
        i64::from(kind.offset),
        intern(&kind.abbreviation),
    );
    Ok(())
}

fn matching_offsets(zone: &Zone, local: i64, hint: i32) -> i64 {
    // cite: bionic localtime.c#time2sub
    let probes = [
        zone.at(local - 86_400).offset,
        zone.at(local + 86_400).offset,
        zone.at(local).offset,
    ];
    let mut candidates: Vec<(i64, bool)> = Vec::new();
    for offset in probes {
        let seconds = local - i64::from(offset);
        let kind = zone.at(seconds);
        if kind.offset == offset && !candidates.iter().any(|(value, _)| *value == seconds) {
            candidates.push((seconds, kind.is_dst));
        }
    }
    if candidates.is_empty() {
        return local - i64::from(zone.at(local - 86_400).offset);
    }
    candidates.sort_unstable();
    candidates
        .iter()
        .find(|(_, is_dst)| hint >= 0 && *is_dst == (hint > 0))
        .or(candidates.first())
        .map(|(seconds, _)| *seconds)
        .expect("candidates is not empty")
}

pub fn mktime_copy(address: usize) -> Result<i64, i32> {
    let mut scratch = [0u8; TM_SIZE];
    super::guest_mem::write_bytes(scratch.as_mut_ptr() as usize, unsafe {
        std::slice::from_raw_parts(address as *const u8, TM_SIZE)
    });
    mktime(scratch.as_mut_ptr() as usize)
}

fn mktime(address: usize) -> Result<i64, i32> {
    let zone = current_zone();
    let parts = read_tm(address);
    let hint = read_value::<i32>(address + 32);
    let local = to_seconds(&parts)?;
    let seconds = matching_offsets(&zone, local, hint);
    fill_local(address, seconds, &zone)?;
    Ok(seconds)
}

pub fn register() {
    let _ = TZNAME.set(data_in_libc("tzname", 16));
    let _ = TIMEZONE.set(data_in_libc("timezone", 8));
    let _ = DAYLIGHT.set(data_in_libc("daylight", 4));
    in_libc("tzset", |_| {
        current_zone();
    });
    for name in ["localtime_r", "localtime"] {
        in_libc(name, move |context| {
            let destination = if context.symbol() == "localtime" {
                leaked_buffer(&LOCALTIME_BUFFER, TM_SIZE)
            } else {
                context.arg_ptr(1)
            };
            let zone = current_zone();
            match fill_local(destination, read_value::<i64>(context.arg_ptr(0)), &zone) {
                Ok(()) => context.ret_ptr(destination),
                Err(code) => {
                    context.set_errno(code);
                    context.ret_ptr(0);
                }
            }
        });
    }
    for name in ["mktime", "timelocal"] {
        in_libc(name, |context| match mktime(context.arg_ptr(0)) {
            Ok(seconds) => context.ret_i64(seconds),
            Err(code) => {
                context.set_errno(code);
                context.ret_i64(-1);
            }
        });
    }
    for name in ["ctime", "ctime_r"] {
        in_libc(name, move |context| {
            let buffer = if context.symbol() == "ctime" {
                leaked_buffer(&CTIME_STATIC, 80)
            } else {
                context.arg_ptr(1)
            };
            let zone = current_zone();
            let mut scratch = [0u8; TM_SIZE];
            let address = scratch.as_mut_ptr() as usize;
            match fill_local(address, read_value::<i64>(context.arg_ptr(0)), &zone) {
                Ok(()) => {
                    asctime_into(buffer, &read_tm(address), context.symbol() == "ctime");
                    context.ret_ptr(buffer);
                }
                Err(code) => {
                    context.set_errno(code);
                    context.ret_ptr(0);
                }
            }
        });
    }
}
