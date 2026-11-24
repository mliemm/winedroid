use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const VARIANT_DIRECTORIES: [&str; 2] = ["16k", "4k"];
const FIXTURE_LIBRARIES: [&str; 4] = ["libfx_add.so", "libfx_dep.so", "libfx_jni.so", "libfx_reloc.so"];

fn manifest_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn android_ref_root() -> Option<PathBuf> {
    let root = match std::env::var_os("ANDROID_REF_ROOT") {
        Some(value) => PathBuf::from(value),
        None => PathBuf::from(std::env::var_os("HOME")?).join("Dev/android-ref"),
    };
    root.is_dir().then_some(root)
}

fn header_slots(header: &str, struct_name: &str) -> Vec<String> {
    let start = header
        .find(&format!("struct {struct_name} {{"))
        .unwrap_or_else(|| panic!("jni.h has no struct {struct_name}"));
    let body = &header[start..];
    let body = &body[..body.find("\n};").expect("struct is not closed")];
    body.lines()
        .filter_map(|line| {
            if let Some(open) = line.find("(*") {
                let rest = &line[open + 2..];
                return Some(rest[..rest.find(')')?].to_owned());
            }
            let name = line.trim().strip_prefix("void*")?.trim().strip_suffix(';')?;
            name.starts_with("reserved").then(|| name.to_owned())
        })
        .collect()
}

fn art_slots(source: &str, table_marker: &str, prefix: &str) -> Vec<String> {
    let start = source
        .find(table_marker)
        .unwrap_or_else(|| panic!("art source has no {table_marker}"));
    let body = &source[start..];
    let body = &body[..body.find("};").expect("table is not closed")];
    let mut reserved = 0;
    body.lines()
        .skip(1)
        .filter_map(|line| {
            let entry = line.trim().split("//").next()?.trim().trim_end_matches(',');
            if entry == "nullptr" {
                reserved += 1;
                return Some(format!("reserved{}", reserved - 1));
            }
            entry.strip_prefix(prefix).map(str::to_owned)
        })
        .collect()
}

fn read_header() -> String {
    fs::read_to_string(manifest_path("tests/native_fixtures/include/jni.h")).expect("jni.h should be readable")
}

#[test]
fn art_jni_internal_cc_gjninativeinterface_slot_count_and_ends_of_the_header_table() {
    let slots = header_slots(&read_header(), "JNINativeInterface");

    assert_eq!(slots.len(), 233);
    assert_eq!(slots[0], "reserved0");
    assert_eq!(slots[4], "GetVersion");
    assert_eq!(slots[232], "GetObjectRefType");
}

#[test]
fn art_java_vm_ext_cc_gjniinvokeinterface_slot_order_in_the_header() {
    let slots = header_slots(&read_header(), "JNIInvokeInterface");

    assert_eq!(
        slots,
        [
            "reserved0",
            "reserved1",
            "reserved2",
            "DestroyJavaVM",
            "AttachCurrentThread",
            "DetachCurrentThread",
            "GetEnv",
            "AttachCurrentThreadAsDaemon",
        ]
    );
}

#[test]
fn art_jni_internal_cc_gjninativeinterface_matches_the_header_slot_for_slot() {
    let Some(root) = android_ref_root() else {
        eprintln!("skipped: ANDROID_REF_ROOT is not set and ~/Dev/android-ref is absent");
        return;
    };
    let header = read_header();
    let internal = fs::read_to_string(root.join("art/runtime/jni/jni_internal.cc")).expect("jni_internal.cc");
    let vm_ext = fs::read_to_string(root.join("art/runtime/jni/java_vm_ext.cc")).expect("java_vm_ext.cc");

    assert_eq!(
        header_slots(&header, "JNINativeInterface"),
        art_slots(&internal, "gJniNativeInterface = {", "JNIImpl::")
    );
    assert_eq!(
        header_slots(&header, "JNIInvokeInterface"),
        art_slots(&vm_ext, "gJniInvokeInterface = {", "JII::")
    );
}

#[test]
fn art_slot_parser_reports_a_swapped_pair() {
    let reference = "x gJniNativeInterface = {\n nullptr, // r\n JNIImpl::A,\n JNIImpl::B,\n};";
    let header = "struct JNINativeInterface {\n void* reserved0;\n jint (*B)(void);\n jint (*A)(void);\n};";

    assert_ne!(header_slots(header, "JNINativeInterface"), art_slots(reference, "gJniNativeInterface = {", "JNIImpl::"));
}

fn files_in(directory: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("{} unreadable: {error}", directory.display()))
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
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
    assert!(status.success(), "build_native_fixtures.sh failed");

    for directory in VARIANT_DIRECTORIES {
        let checked_in = manifest_path("tests/native_fixtures/bin").join(directory);
        let rebuilt = output.join(directory);
        assert_eq!(files_in(&rebuilt), FIXTURE_LIBRARIES, "{directory} file set");
        assert_eq!(files_in(&checked_in), FIXTURE_LIBRARIES, "{directory} checked in file set");
        for library in FIXTURE_LIBRARIES {
            assert_eq!(
                fs::read(rebuilt.join(library)).unwrap(),
                fs::read(checked_in.join(library)).unwrap(),
                "{directory}/{library} differs from the rebuilt binary"
            );
        }
    }
    fs::remove_dir_all(&output).unwrap();
}
