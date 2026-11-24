use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use regex::Regex;

use super::fixture_header::manifest_path;
use super::fixture_reference_tests::android_ref_root;

const TOOL_MISSING_STATUS: i32 = 127;
const VARIANT_DIRECTORIES: [&str; 2] = ["16k", "4k"];
const FIXTURE_LIBRARIES: [&str; 6] = [
    "libfx_add.so",
    "libfx_aps2.so",
    "libfx_dep.so",
    "libfx_jni.so",
    "libfx_reloc.so",
    "libfx_relr.so",
];

fn crc_feed(crc: u32, byte: u8) -> u32 {
    let mut crc = crc ^ (u32::from(byte) << 24);
    for _ in 0..8 {
        crc = if crc & 0x8000_0000 != 0 { (crc << 1) ^ 0x04C1_1DB7 } else { crc << 1 };
    }
    crc
}

fn cksum(data: &[u8]) -> String {
    let mut crc = data.iter().fold(0, |crc, &byte| crc_feed(crc, byte));
    let mut length = data.len();
    while length > 0 {
        crc = crc_feed(crc, (length & 0xFF) as u8);
        length >>= 8;
    }
    format!("{} {}", !crc, data.len())
}

fn read_manifest(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("{} unreadable: {error}", path.display()))
        .lines()
        .map(str::to_owned)
        .collect()
}

fn checksummed(lines: &[String], kind: &str) -> BTreeMap<String, String> {
    lines
        .iter()
        .filter_map(|line| line.strip_prefix(kind)?.strip_prefix(' '))
        .map(|rest| {
            let (name, sum) = rest.split_once(' ').expect("manifest line has a checksum");
            (name.to_owned(), sum.to_owned())
        })
        .collect()
}

fn files_in(directory: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("{} unreadable: {error}", directory.display()))
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn checksums_on_disk(root: &Path, relative: &[String]) -> BTreeMap<String, String> {
    relative
        .iter()
        .map(|name| (name.clone(), cksum(&fs::read(root.join(name)).unwrap())))
        .collect()
}

#[test]
fn posix_cksum_matches_the_cksum_utility_for_a_known_string() {
    assert_eq!(cksum(b"123456789"), "930766865 9");
}

#[test]
fn fixture_manifest_sources_and_binaries_match_the_files_on_disk() {
    let fixtures = manifest_path("tests/native_fixtures");
    let manifest = read_manifest(&fixtures.join("bin/manifest.txt"));

    let sources = checksummed(&manifest, "source");
    let mut source_names: Vec<String> = files_in(&fixtures.join("src")).iter().map(|name| format!("src/{name}")).collect();
    source_names.push("include/jni.h".to_owned());
    assert_eq!(sources, checksums_on_disk(&fixtures, &source_names), "a source changed without rebuilding the fixtures");

    let binaries = checksummed(&manifest, "binary");
    let binary_names: Vec<String> = VARIANT_DIRECTORIES
        .iter()
        .flat_map(|directory| FIXTURE_LIBRARIES.iter().map(move |library| format!("{directory}/{library}")))
        .collect();
    assert_eq!(binaries, checksums_on_disk(&fixtures.join("bin"), &binary_names), "a binary differs from the manifest");
    for directory in VARIANT_DIRECTORIES {
        assert_eq!(files_in(&fixtures.join("bin").join(directory)), FIXTURE_LIBRARIES, "{directory} file set");
    }
}

#[test]
fn clang_and_lld_version_output_formats_manifest_records_the_toolchain_that_built_the_binaries() {
    let manifest = read_manifest(&manifest_path("tests/native_fixtures/bin/manifest.txt"));
    let clang = Regex::new(r"^clang .*clang version \d+\.\d+\.\d+").unwrap();
    let lld = Regex::new(r"^ld\.lld LLD \d+\.\d+\.\d+").unwrap();

    assert!(clang.is_match(&manifest[0]), "clang line {:?}", manifest[0]);
    assert!(lld.is_match(&manifest[1]), "ld.lld line {:?}", manifest[1]);
}

#[test]
fn fixture_build_command_reproduces_the_checked_in_binaries_byte_for_byte() {
    let Some(root) = android_ref_root() else {
        eprintln!("skipped: ANDROID_REF_ROOT is not set and ~/Dev/android-ref is absent");
        return;
    };
    let output = std::env::temp_dir().join(format!("winedroid-fixtures-{}", std::process::id()));
    let _ = fs::remove_dir_all(&output);
    let status = Command::new("bash")
        .arg(manifest_path("scripts/build_native_fixtures.sh"))
        .env("ANDROID_REF_ROOT", &root)
        .env("FIXTURE_OUT_ROOT", &output)
        .status()
        .expect("build script should start");
    if status.code() == Some(TOOL_MISSING_STATUS) {
        eprintln!("skipped: the build script found no clang or ld.lld");
        let _ = fs::remove_dir_all(&output);
        return;
    }
    assert!(status.success(), "build_native_fixtures.sh failed");

    let checked_in = manifest_path("tests/native_fixtures/bin");
    let expected = read_manifest(&checked_in.join("manifest.txt"));
    let rebuilt = read_manifest(&output.join("manifest.txt"));
    if expected[..2] != rebuilt[..2] {
        eprintln!("skipped: toolchain {:?} differs from the recorded {:?}", &rebuilt[..2], &expected[..2]);
        fs::remove_dir_all(&output).unwrap();
        return;
    }
    assert_eq!(rebuilt, expected, "manifest");
    for directory in VARIANT_DIRECTORIES {
        for library in FIXTURE_LIBRARIES {
            let path = format!("{directory}/{library}");
            assert_eq!(
                fs::read(output.join(&path)).unwrap(),
                fs::read(checked_in.join(&path)).unwrap(),
                "{path} differs from the rebuilt binary"
            );
        }
    }
    fs::remove_dir_all(&output).unwrap();
}
