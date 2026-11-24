use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use super::fixture_elf::{
    FixtureElf, PT_TLS, R_AARCH64_ABS64, R_AARCH64_GLOB_DAT, R_AARCH64_JUMP_SLOT, R_AARCH64_RELATIVE,
    R_AARCH64_TLSDESC, DT_ANDROID_RELA, DT_ANDROID_RELASZ, DT_RELR, DT_RELRSZ,
};
use crate::elf::{parse_elf, ElfClass, ElfEndian, ElfMachine};

const PT_LOAD: u32 = 1;

struct Variant {
    directory: &'static str,
    page_size: u64,
}

const VARIANTS: [Variant; 2] = [
    Variant {
        directory: "16k",
        page_size: 0x4000,
    },
    Variant {
        directory: "4k",
        page_size: 0x1000,
    },
];

const FIXTURES: [(&str, &[&str]); 7] = [
    ("libfx_add.so", &[]),
    ("libfx_reloc.so", &["libc.so", "liblog.so"]),
    ("libfx_dep.so", &["libfx_add.so", "libdl.so"]),
    ("libfx_jni.so", &["libc.so"]),
    ("libfx_order_child.so", &["libc.so"]),
    ("libfx_order_parent.so", &["libfx_order_child.so", "libc.so", "libdl.so"]),
    ("libfx_relro.so", &[]),
];

fn fixture_path(variant: &Variant, name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/native_fixtures/bin")
        .join(variant.directory)
        .join(name)
}

fn read_fixture(variant: &Variant, name: &str) -> Vec<u8> {
    let path = fixture_path(variant, name);
    fs::read(&path).unwrap_or_else(|error| panic!("fixture {} is unreadable: {error}", path.display()))
}

fn load_alignments(data: &[u8]) -> Vec<(u64, u64, u64)> {
    let u16_at = |offset: usize| u64::from(u16::from_le_bytes([data[offset], data[offset + 1]]));
    let u32_at = |offset: usize| u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
    let u64_at = |offset: usize| u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());

    let table = u64_at(32) as usize;
    let entry_size = u16_at(54) as usize;
    let count = u16_at(56) as usize;
    (0..count)
        .map(|index| table + index * entry_size)
        .filter(|&header| u32_at(header) == PT_LOAD)
        .map(|header| (u64_at(header + 8), u64_at(header + 16), u64_at(header + 48)))
        .collect()
}

#[test]
fn fixture_source_and_bionic_abi_every_variant_parses_as_aarch64_elf64() {
    for variant in &VARIANTS {
        for (name, _) in FIXTURES {
            let summary = parse_elf(&read_fixture(variant, name))
                .unwrap_or_else(|error| panic!("{}/{name} did not parse: {error}", variant.directory));

            assert_eq!(summary.class, ElfClass::Elf64, "{}/{name}", variant.directory);
            assert_eq!(summary.endian, ElfEndian::Little, "{}/{name}", variant.directory);
            assert_eq!(summary.machine, ElfMachine::Aarch64, "{}/{name}", variant.directory);
        }
    }
}

#[test]
fn fixture_source_dt_needed_matches_the_imports_each_source_declares() {
    for variant in &VARIANTS {
        for (name, expected) in FIXTURES {
            let mut summary = parse_elf(&read_fixture(variant, name)).expect("fixture should parse");
            summary.needed_libraries.sort();
            let mut expected: Vec<String> = expected.iter().map(|library| (*library).to_owned()).collect();
            expected.sort();

            assert_eq!(summary.needed_libraries, expected, "{}/{name}", variant.directory);
        }
    }
}

#[test]
fn fixture_build_command_p_align_is_the_requested_max_page_size() {
    for variant in &VARIANTS {
        for (name, _) in FIXTURES {
            let loads = load_alignments(&read_fixture(variant, name));

            assert!(!loads.is_empty(), "{}/{name} has no PT_LOAD", variant.directory);
            for (offset, vaddr, align) in loads {
                assert_eq!(align, variant.page_size, "{}/{name}", variant.directory);
                assert_eq!(
                    offset % align,
                    vaddr % align,
                    "{}/{name} PT_LOAD offset and vaddr disagree modulo p_align",
                    variant.directory
                );
            }
        }
    }
}

fn names(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

#[test]
fn fixture_source_fx_reloc_has_every_relocation_kind_the_loader_must_apply() {
    for variant in &VARIANTS {
        let elf = FixtureElf::parse(&read_fixture(variant, "libfx_reloc.so"));
        let label = variant.directory;

        assert_eq!(elf.count(R_AARCH64_RELATIVE), 4, "{label} RELATIVE");
        assert_eq!(
            elf.symbols_of(R_AARCH64_ABS64),
            names(&["memcpy", "strlen", "fx_exported_value"]),
            "{label} ABS64"
        );
        assert_eq!(
            elf.symbols_of(R_AARCH64_GLOB_DAT),
            names(&["strlen", "fx_exported_value", "fx_fini_marker"]),
            "{label} GLOB_DAT"
        );
        assert_eq!(
            elf.symbols_of(R_AARCH64_JUMP_SLOT),
            names(&[
                "memcpy",
                "strlen",
                "malloc",
                "free",
                "snprintf",
                "pthread_mutex_lock",
                "pthread_mutex_unlock",
                "__cxa_atexit",
                "__stack_chk_fail",
                "__android_log_print",
            ]),
            "{label} JUMP_SLOT"
        );
        assert_eq!(
            elf.symbols_of(R_AARCH64_TLSDESC),
            names(&["fx_tls_counter", "fx_tls_zero"]),
            "{label} TLSDESC"
        );
        assert_eq!(elf.relocations.len(), 4 + 3 + 3 + 10 + 2, "{label} no other relocation kind");
        assert_eq!(elf.init_array_len, 1, "{label} DT_INIT_ARRAY entries");
        assert_eq!(elf.fini_array_len, 1, "{label} DT_FINI_ARRAY entries");
        assert!(elf.has_segment(PT_TLS), "{label} PT_TLS");
    }
}

#[test]
fn fixture_source_exports_and_imports_match_what_each_source_defines_and_calls() {
    let reloc_exports = names(&[
        "fx_atexit_marker",
        "fx_copy_sum",
        "fx_ctor_marker",
        "fx_exported_pointer",
        "fx_exported_value",
        "fx_fini_marker",
        "fx_format",
        "fx_guarded_sum",
        "fx_import_table",
        "fx_local_pointer",
        "fx_lock_roundtrip",
        "fx_log",
        "fx_strlen",
        "fx_strlen_address",
        "fx_tls_counter",
        "fx_tls_next",
        "fx_tls_zero",
    ]);
    let reloc_imports = names(&[
        "__android_log_print",
        "__cxa_atexit",
        "__stack_chk_fail",
        "free",
        "malloc",
        "memcpy",
        "pthread_mutex_lock",
        "pthread_mutex_unlock",
        "snprintf",
        "strlen",
    ]);
    let table = [
        ("libfx_add.so", names(&["add", "fx_data_value"]), names(&[])),
        ("libfx_reloc.so", reloc_exports, reloc_imports),
        (
            "libfx_dep.so",
            names(&["fx_dep_add", "fx_dep_dlsym_add"]),
            names(&["add", "dlclose", "dlopen", "dlsym", "fx_data_value"]),
        ),
        (
            "libfx_jni.so",
            names(&[
                "JNI_OnLoad",
                "Java_fx_Native_arrayLength",
                "Java_fx_Native_callLength",
                "Java_fx_Native_callPing",
                "Java_fx_Native_greeting",
                "Java_fx_Native_stringLength",
                "Java_fx_Native_throwState",
            ]),
            names(&["__stack_chk_fail", "strlen"]),
        ),
    ];
    for variant in &VARIANTS {
        for (name, exports, imports) in &table {
            let elf = FixtureElf::parse(&read_fixture(variant, name));

            assert_eq!(&elf.exports, exports, "{}/{name} exports", variant.directory);
            assert_eq!(&elf.imports, imports, "{}/{name} imports", variant.directory);
            if *name != "libfx_reloc.so" {
                assert!(!elf.has_segment(PT_TLS), "{}/{name} must have no PT_TLS", variant.directory);
                assert_eq!(elf.init_array_len + elf.fini_array_len, 0, "{}/{name}", variant.directory);
            }
        }
    }
}

#[test]
fn fixture_source_fx_reloc_without_packing_has_neither_aps2_nor_relr_tables() {
    for variant in &VARIANTS {
        let elf = FixtureElf::parse(&read_fixture(variant, "libfx_reloc.so"));

        for tag in [DT_ANDROID_RELA, DT_ANDROID_RELASZ, DT_RELR, DT_RELRSZ] {
            assert_eq!(elf.tag(tag), None, "{} tag {tag:#x}", variant.directory);
        }
    }
}

#[test]
fn fixture_build_command_pack_dyn_relocs_android_emits_an_aps2_table() {
    for variant in &VARIANTS {
        let data = read_fixture(variant, "libfx_aps2.so");
        let elf = FixtureElf::parse(&data);
        let start = elf.tag(DT_ANDROID_RELA).expect("DT_ANDROID_RELA");
        let size = elf.tag(DT_ANDROID_RELASZ).expect("DT_ANDROID_RELASZ");
        let offset = elf.file_offset(start);

        assert!(size > 4, "{} APS2 payload size {size}", variant.directory);
        assert_eq!(&data[offset..offset + 4], b"APS2", "{} APS2 magic", variant.directory);
        assert_eq!(elf.tag(DT_RELR), None, "{}", variant.directory);
        assert_eq!(elf.count(R_AARCH64_RELATIVE), 0, "{} RELATIVE moved into the packed table", variant.directory);
    }
}

#[test]
fn fixture_build_command_pack_dyn_relocs_relr_emits_a_relr_table() {
    for variant in &VARIANTS {
        let elf = FixtureElf::parse(&read_fixture(variant, "libfx_relr.so"));

        assert!(elf.tag(DT_RELR).is_some(), "{} DT_RELR", variant.directory);
        assert!(elf.tag(DT_RELRSZ).is_some_and(|size| size >= 8 && size % 8 == 0), "{} DT_RELRSZ", variant.directory);
        assert_eq!(elf.tag(DT_ANDROID_RELA), None, "{}", variant.directory);
        assert!(elf.count(R_AARCH64_RELATIVE) < 4, "{} RELATIVE moved into the RELR table", variant.directory);
    }
}

#[test]
fn fixture_source_packed_variants_export_what_fx_reloc_exports() {
    for variant in &VARIANTS {
        let plain = FixtureElf::parse(&read_fixture(variant, "libfx_reloc.so"));
        for name in ["libfx_aps2.so", "libfx_relr.so"] {
            let packed = FixtureElf::parse(&read_fixture(variant, name));

            assert_eq!(packed.exports, plain.exports, "{}/{name} exports", variant.directory);
            assert_eq!(packed.imports, plain.imports, "{}/{name} imports", variant.directory);
        }
    }
}
