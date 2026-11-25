use std::collections::BTreeMap;
use std::path::PathBuf;

use super::bionic::errno_map::{darwin_from_linux, is_linux_errno, linux_from_darwin, DARWIN_ALIASES, LINUX_ERRNO_MAX, ROWS};
use super::bridge_test_support::Bridged;
use super::call_assign::CallBuilder;
use super::bridge::handler_address;

fn definitions(paths: &[PathBuf]) -> BTreeMap<String, i32> {
    let mut raw: BTreeMap<String, String> = BTreeMap::new();
    for path in paths {
        let text = std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        for line in text.lines() {
            let mut words = line.split_whitespace();
            let (Some(define), Some(name), Some(value)) = (words.next(), words.next(), words.next()) else { continue };
            let hash_define = define == "#define" || (define == "#" && name == "define");
            if !hash_define {
                continue;
            }
            let (name, value) = if define == "#" { (value, words.next().unwrap_or("")) } else { (name, value) };
            if name.starts_with('E') && name[1..].chars().all(|letter| letter.is_ascii_uppercase() || letter.is_ascii_digit()) {
                raw.insert(name.to_string(), value.to_string());
            }
        }
    }
    let resolve = |start: &str| -> Option<i32> {
        let mut current = start.to_string();
        for _ in 0..4 {
            let value = raw.get(&current)?.clone();
            if let Ok(number) = value.parse::<i32>() {
                return Some(number);
            }
            current = value;
        }
        None
    };
    raw.keys().filter_map(|name| resolve(name).map(|value| (name.clone(), value))).collect()
}

fn darwin_headers() -> PathBuf {
    let sdk = std::process::Command::new("xcrun").arg("--show-sdk-path").output().expect("xcrun runs on macOS");
    PathBuf::from(String::from_utf8(sdk.stdout).unwrap().trim()).join("usr/include/sys/errno.h")
}

fn linux_headers() -> Vec<PathBuf> {
    let root = std::env::var_os("ANDROID_REF_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../android-ref"));
    ["errno-base.h", "errno.h"].iter().map(|name| root.join("bionic/libc/kernel/uapi/asm-generic").join(name)).collect()
}

fn header_tables() -> Option<(BTreeMap<String, i32>, BTreeMap<String, i32>)> {
    let linux = linux_headers();
    if !linux.iter().all(|path| path.exists()) {
        eprintln!("skipped: android-ref errno headers not found, set ANDROID_REF_ROOT");
        return None;
    }
    Some((definitions(&linux), definitions(&[darwin_headers()])))
}

#[test]
fn every_errno_name_in_the_bionic_kernel_uapi_and_macos_sdk_headers_has_a_row_with_both_numbers() {
    let Some((linux, darwin)) = header_tables() else { return };
    let both: Vec<&String> = linux.keys().filter(|name| darwin.contains_key(*name) && name.as_str() != "EWOULDBLOCK").collect();

    assert!(both.len() >= 80, "{}", both.len());
    for name in both {
        let row = ROWS.iter().find(|row| row.name == name).unwrap_or_else(|| panic!("{name} has no row"));
        assert_eq!((row.linux, row.darwin), (linux[name], darwin[name]), "{name}");
    }
    for row in ROWS {
        assert_eq!((linux.get(row.name), darwin.get(row.name)), (Some(&row.linux), Some(&row.darwin)), "{} is not in both headers", row.name);
    }
}

#[test]
fn every_row_translates_both_ways_and_darwin_only_or_linux_only_names_never_get_a_number_by_accident() {
    let Some((linux, darwin)) = header_tables() else { return };

    for row in ROWS {
        assert_eq!(linux_from_darwin(row.darwin), Some(row.linux), "{} to Linux", row.name);
        assert_eq!(darwin_from_linux(row.linux), Some(row.darwin), "{} to Darwin", row.name);
    }
    for (name, value) in darwin.iter().filter(|(name, _)| !linux.contains_key(*name)) {
        let alias = DARWIN_ALIASES.iter().find(|alias| alias.1 == *value).map(|alias| alias.2);
        assert_eq!(linux_from_darwin(*value), alias, "Darwin only {name}");
    }
    for (name, value) in linux.iter().filter(|(name, _)| !darwin.contains_key(*name) && name.as_str() != "EDEADLOCK") {
        assert_eq!(darwin_from_linux(*value), None, "Linux only {name}");
    }
}

#[test]
fn the_numbers_that_differ_between_linux_and_darwin_swap_for_eagain_and_edeadlk() {
    assert_eq!((linux_from_darwin(35), linux_from_darwin(11)), (Some(11), Some(35)));
    assert_eq!((darwin_from_linux(11), darwin_from_linux(35)), (Some(35), Some(11)));
    assert_eq!((darwin_from_linux(110), darwin_from_linux(111)), (Some(60), Some(61)));
    assert_eq!((linux_from_darwin(45), linux_from_darwin(93)), (Some(95), Some(61)));
    assert_eq!((linux_from_darwin(0), darwin_from_linux(0)), (Some(0), Some(0)));
    assert_eq!(linux_from_darwin(67), None);
    assert_eq!(darwin_from_linux(52), None);
}

#[test]
fn rows_have_unique_linux_and_darwin_numbers_inside_the_linux_range() {
    let mut linux: Vec<i32> = ROWS.iter().map(|row| row.linux).collect();
    let mut darwin: Vec<i32> = ROWS.iter().map(|row| row.darwin).collect();
    linux.sort_unstable();
    darwin.sort_unstable();
    linux.dedup();
    darwin.dedup();

    assert_eq!((linux.len(), darwin.len()), (ROWS.len(), ROWS.len()));
    assert!(ROWS.iter().all(|row| is_linux_errno(row.linux) && row.linux > 0));
    assert!(is_linux_errno(0) && is_linux_errno(LINUX_ERRNO_MAX) && !is_linux_errno(LINUX_ERRNO_MAX + 1) && !is_linux_errno(-1));
}

#[test]
fn a_handler_translates_a_host_eagain_to_the_linux_number_the_guest_reads_from_errno() {
    let bridged = Bridged::load("16k", "libfx_bridge.so");

    assert_eq!(bridged.call("fx_errno_after").i32(2).invoke().unwrap().i32(), 11, "Darwin EAGAIN 35 is Linux EAGAIN 11");
    assert_eq!(bridged.call("fx_errno_after").i32(1).invoke().unwrap().i32(), 34, "set_errno stores the Linux number as given");
    assert_eq!(bridged.call("fx_errno_after").i32(0).invoke().unwrap().i32(), 0, "a handler that only clobbers the host errno leaves the guest value");
}

#[test]
fn a_handler_errno_without_a_linux_number_is_a_failure_not_a_substitute() {
    let bridged = Bridged::load("16k", "libfx_bridge.so");

    let unmappable = bridged.call("fx_errno_after").i32(3).invoke().unwrap_err().to_string();
    let invalid = bridged.call("fx_errno_after").i32(4).invoke().unwrap_err().to_string();

    assert!(unmappable.contains("host errno 67") && unmappable.contains("no Linux errno"), "{unmappable}");
    assert!(invalid.contains("set errno 1000") && invalid.contains("not a Linux errno"), "{invalid}");
    assert!(handler_address("fx_touch_errno").is_some());
    assert!(CallBuilder::new(handler_address("fx_touch_errno").unwrap()).i32(1).invoke().is_ok());
}
