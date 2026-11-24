use std::fs;
use std::path::PathBuf;

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

const FIXTURES: [(&str, &[&str]); 4] = [
    ("libfx_add.so", &[]),
    ("libfx_reloc.so", &["libc.so", "liblog.so"]),
    ("libfx_dep.so", &["libfx_add.so", "libdl.so"]),
    ("libfx_jni.so", &["libc.so"]),
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
