use std::collections::BTreeSet;
use std::fs;

use regex::Regex;

use super::fixture_header::{header_slots, manifest_path, read_header};

const VARIANT_DIRECTORIES: [&str; 2] = ["16k", "4k"];
const LDR_X_UNSIGNED_OFFSET: u32 = 0xF940_0000;
const LDR_X_MASK: u32 = 0xFFC0_0000;
const BLR_BR_MASK: u32 = 0xFFDF_FC1F;
const BLR_BR_OPCODE: u32 = 0xD61F_0000;
const ENV_CALLS: [&str; 11] = [
    "CallIntMethod",
    "CallStaticVoidMethod",
    "FindClass",
    "GetArrayLength",
    "GetMethodID",
    "GetStaticMethodID",
    "GetStringUTFChars",
    "NewStringUTF",
    "RegisterNatives",
    "ReleaseStringUTFChars",
    "ThrowNew",
];
const VM_CALLS: [&str; 1] = ["GetEnv"];
const SLOT_SIZE: u64 = 8;
const PLT_BASE_REGISTER: u32 = 16;
const LOOKBACK_WORDS: usize = 6;

fn source_calls(source: &str, receiver: &str) -> BTreeSet<String> {
    let pattern = Regex::new(&format!(r"\(\*{receiver}\)->(\w+)\(")).unwrap();
    pattern.captures_iter(source).map(|capture| capture[1].to_owned()).collect()
}

fn slot_offsets(header: &str, struct_name: &str, names: &BTreeSet<String>) -> BTreeSet<u64> {
    header_slots(header, struct_name)
        .iter()
        .enumerate()
        .filter(|(_, slot)| names.contains(&slot.name))
        .map(|(index, _)| index as u64 * SLOT_SIZE)
        .collect()
}

fn called_table_offsets(binary: &[u8]) -> BTreeSet<u64> {
    let words: Vec<u32> = binary
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap()))
        .collect();
    let mut offsets = BTreeSet::new();
    for (index, &word) in words.iter().enumerate() {
        if word & BLR_BR_MASK != BLR_BR_OPCODE {
            continue;
        }
        let register = (word >> 5) & 31;
        let definer = words[index.saturating_sub(LOOKBACK_WORDS)..index]
            .iter()
            .rev()
            .find(|&&earlier| earlier & 31 == register && earlier & LDR_X_MASK == LDR_X_UNSIGNED_OFFSET);
        let definer = definer.filter(|&&load| (load >> 5) & 31 != PLT_BASE_REGISTER);
        if let Some(&load) = definer {
            offsets.insert(u64::from((load >> 10) & 0xFFF) * SLOT_SIZE);
        }
    }
    offsets
}

#[test]
fn fixture_source_fx_jni_calls_exactly_the_jni_slots_its_source_names() {
    let source = fs::read_to_string(manifest_path("tests/native_fixtures/src/fx_jni.c")).unwrap();
    let header = read_header();
    let env_calls = source_calls(&source, "env");
    let vm_calls = source_calls(&source, "vm");
    assert_eq!(env_calls, ENV_CALLS.iter().map(|name| (*name).to_owned()).collect::<BTreeSet<_>>());
    assert_eq!(vm_calls, VM_CALLS.iter().map(|name| (*name).to_owned()).collect::<BTreeSet<_>>());
    let mut expected = slot_offsets(&header, "JNINativeInterface", &env_calls);
    expected.extend(slot_offsets(&header, "JNIInvokeInterface", &vm_calls));

    assert_eq!(expected.len(), 11, "eleven distinct slot offsets are named in the source, FindClass and GetEnv share one");
    for directory in VARIANT_DIRECTORIES {
        let binary = fs::read(manifest_path("tests/native_fixtures/bin").join(directory).join("libfx_jni.so")).unwrap();

        assert_eq!(called_table_offsets(&binary), expected, "{directory} JNI table slots called");
    }
}

#[test]
fn llvm_mc_aarch64_encodings_scanner_reports_the_slot_of_a_load_followed_by_blr_and_ignores_plt_loads() {
    let words: [u32; 6] = [
        0xF940_0C09,
        0xF940_0008,
        0xF940_3908,
        0xD63F_0100,
        0xF940_A211,
        0xD63F_0220,
    ];
    let binary: Vec<u8> = words.iter().flat_map(|word| word.to_le_bytes()).collect();

    assert_eq!(called_table_offsets(&binary), BTreeSet::from([14 * SLOT_SIZE]));
}
