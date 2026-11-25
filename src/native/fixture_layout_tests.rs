use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use super::fixture_elf::{FixtureElf, DT_GNU_HASH, DT_HASH, PF_W, PF_X, PT_GNU_EH_FRAME, PT_LOAD};

const HOST_PAGE: u64 = 0x4000;
const VARIANTS: [&str; 2] = ["16k", "4k"];
const PAGE_LIBRARIES: [&str; 9] = [
    "libfx_add.so",
    "libfx_aps2.so",
    "libfx_dep.so",
    "libfx_jni.so",
    "libfx_order_child.so",
    "libfx_order_parent.so",
    "libfx_reloc.so",
    "libfx_relr.so",
    "libfx_relro.so",
];
const LIBRARIES: [&str; 12] = [
    "libfx_add.so",
    "libfx_aps2.so",
    "libfx_cxx.so",
    "libfx_dep.so",
    "libfx_jni.so",
    "libfx_order_child.so",
    "libfx_order_parent.so",
    "libfx_phdr.so",
    "libfx_reloc.so",
    "libfx_relr.so",
    "libfx_relro.so",
    "libfx_unwind.so",
];

fn fixture_bytes(variant: &str, name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/native_fixtures/bin")
        .join(variant)
        .join(name);
    fs::read(&path).unwrap_or_else(|error| panic!("{} unreadable: {error}", path.display()))
}

fn host_pages_with(elf: &FixtureElf, flag: u32) -> BTreeSet<u64> {
    elf.segments
        .iter()
        .filter(|segment| segment.kind == PT_LOAD && segment.flags & flag != 0 && segment.memsz > 0)
        .flat_map(|segment| segment.vaddr / HOST_PAGE..=(segment.vaddr + segment.memsz - 1) / HOST_PAGE)
        .collect()
}

#[test]
fn bionic_elf_h_pt_gnu_eh_frame_is_present_in_every_fixture() {
    for variant in VARIANTS {
        for name in LIBRARIES {
            let elf = FixtureElf::parse(&fixture_bytes(variant, name));

            assert!(elf.has_segment(PT_GNU_EH_FRAME), "{variant}/{name} PT_GNU_EH_FRAME");
        }
    }
}

#[test]
fn bionic_elf_common_h_dt_gnu_hash_is_the_only_symbol_hash_table_in_every_fixture() {
    for variant in VARIANTS {
        for name in LIBRARIES {
            let elf = FixtureElf::parse(&fixture_bytes(variant, name));

            assert!(elf.tag(DT_GNU_HASH).is_some(), "{variant}/{name} DT_GNU_HASH");
            assert_eq!(elf.tag(DT_HASH), None, "{variant}/{name} DT_HASH");
        }
    }
}

#[test]
fn fixture_build_command_4k_variants_put_executable_and_writable_bytes_in_one_16k_host_page() {
    for name in PAGE_LIBRARIES {
        let elf = FixtureElf::parse(&fixture_bytes("4k", name));
        let shared: Vec<u64> = host_pages_with(&elf, PF_X)
            .intersection(&host_pages_with(&elf, PF_W))
            .copied()
            .collect();

        assert!(!shared.is_empty(), "4k/{name} has no host page holding both RX and RW bytes");
    }
}

#[test]
fn fixture_build_command_16k_variants_never_share_a_16k_host_page_between_rx_and_rw() {
    for name in PAGE_LIBRARIES {
        let elf = FixtureElf::parse(&fixture_bytes("16k", name));
        let shared: Vec<u64> = host_pages_with(&elf, PF_X)
            .intersection(&host_pages_with(&elf, PF_W))
            .copied()
            .collect();

        assert!(shared.is_empty(), "16k/{name} shares host pages {shared:?}");
    }
}

#[test]
fn fixture_source_string_literals_are_present_in_the_binaries_that_use_them() {
    let table: [(&str, &[(&str, usize)]); 3] = [
        ("libfx_dep.so", &[("libfx_add.so", 2), ("add", 2)]),
        ("libfx_reloc.so", &[("%d %s %f", 1), ("%s %d", 1), ("fx_reloc", 1)]),
        (
            "libfx_jni.so",
            &[
                ("fx/Native", 1),
                ("registeredAdd", 1),
                ("(II)I", 1),
                ("fx_jni", 1),
                ("java/lang/String", 1),
                ("length", 1),
                ("()I", 1),
                ("ping", 1),
                ("()V", 1),
                ("java/lang/IllegalStateException", 1),
            ],
        ),
    ];
    for variant in VARIANTS {
        for (name, literals) in table {
            let data = fixture_bytes(variant, name);
            for (literal, minimum) in literals {
                let needle = [&[0], literal.as_bytes(), &[0]].concat();
                let found = data.windows(needle.len()).filter(|window| *window == needle).count();

                assert!(found >= *minimum, "{variant}/{name} has {found} of the literal {literal:?}, wanted {minimum}");
            }
        }
    }
}
