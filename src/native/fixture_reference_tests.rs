use std::fs;
use std::path::PathBuf;

use super::fixture_header::{art_slots, header_slots, read_header, Slot};

pub fn android_ref_root() -> Option<PathBuf> {
    let root = match std::env::var_os("ANDROID_REF_ROOT") {
        Some(value) => PathBuf::from(value),
        None => PathBuf::from(std::env::var_os("HOME")?).join("Dev/android-ref"),
    };
    root.is_dir().then_some(root)
}

#[test]
fn art_jni_internal_cc_gjninativeinterface_slot_count_and_ends_of_the_header_table() {
    let slots = header_slots(&read_header(), "JNINativeInterface");

    assert_eq!(slots.len(), 233);
    assert_eq!(slots[0].name, "reserved0");
    assert_eq!(slots[4].name, "GetVersion");
    assert_eq!(slots[232].name, "GetObjectRefType");
}

#[test]
fn art_java_vm_ext_cc_gjniinvokeinterface_slot_order_in_the_header() {
    let slots = header_slots(&read_header(), "JNIInvokeInterface");

    let names: Vec<&str> = slots.iter().map(|slot| slot.name.as_str()).collect();
    assert_eq!(
        names,
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

fn assert_slots_equal(header: &[Slot], art: &[Slot]) {
    assert_eq!(header.len(), art.len(), "slot count");
    let differing: Vec<String> = header
        .iter()
        .zip(art)
        .filter(|(left, right)| left != right)
        .map(|(left, right)| format!("header {left:?} art {right:?}"))
        .collect();
    assert!(
        differing.is_empty(),
        "slots differ from ART:\n{}",
        differing.join("\n")
    );
}

#[test]
fn art_jni_internal_cc_gjninativeinterface_matches_the_header_slot_for_slot() {
    let Some(root) = android_ref_root() else {
        eprintln!("skipped: ANDROID_REF_ROOT is not set and ~/Dev/android-ref is absent");
        return;
    };
    let header = read_header();
    let internal =
        fs::read_to_string(root.join("art/runtime/jni/jni_internal.cc")).expect("jni_internal.cc");
    let vm_ext =
        fs::read_to_string(root.join("art/runtime/jni/java_vm_ext.cc")).expect("java_vm_ext.cc");

    assert_slots_equal(
        &header_slots(&header, "JNINativeInterface"),
        &art_slots(&internal, "gJniNativeInterface = {", "JNIImpl::"),
    );
    assert_slots_equal(
        &header_slots(&header, "JNIInvokeInterface"),
        &art_slots(&vm_ext, "gJniInvokeInterface = {", "JII::"),
    );
}

#[test]
fn art_slot_parser_reports_a_swapped_pair() {
    let reference = "x gJniNativeInterface = {\n nullptr,\n JNIImpl::A,\n JNIImpl::B,\n};\nstatic jint A(JNIEnv*) {}\nstatic jint B(JNIEnv*) {}";
    let header = "struct JNINativeInterface {\n void* reserved0;\n jint (*B)(JNIEnv*);\n jint (*A)(JNIEnv*);\n};";

    assert_ne!(
        header_slots(header, "JNINativeInterface"),
        art_slots(reference, "gJniNativeInterface = {", "JNIImpl::")
    );
}

#[test]
fn art_slot_parser_reports_a_changed_parameter_type_and_ignores_parameter_names() {
    let reference = "x gJniNativeInterface = {\n nullptr,\n JNIImpl::A,\n};\nstatic jint A(JNIEnv* env, jclass, const char* name) {}";
    let same = "struct JNINativeInterface {\n void* reserved0;\n jint (*A)(JNIEnv *, jclass c, const char *);\n};";
    let changed =
        "struct JNINativeInterface {\n void* reserved0;\n jint (*A)(JNIEnv *, jclass, int);\n};";
    let wanted = art_slots(reference, "gJniNativeInterface = {", "JNIImpl::");

    assert_eq!(header_slots(same, "JNINativeInterface"), wanted);
    assert_ne!(header_slots(changed, "JNINativeInterface"), wanted);
    assert_eq!(
        wanted[1],
        Slot {
            name: "A".to_owned(),
            signature: "jint(JNIEnv*,jclass,constchar*)".to_owned()
        }
    );
}
