use std::fs;

use super::elf_image::{is_arm64_elf, ElfImage, Phdr, PT_LOAD};
use super::image_layout::{ImageLayout, HOST_PAGE};
use super::loader_test_support::{corpus_dir, fixture_dir};

fn fixture(variant: &str, name: &str) -> Vec<u8> {
    fs::read(fixture_dir(variant).join(name)).unwrap()
}

fn parse_error(data: &[u8]) -> String {
    ElfImage::parse("libx.so", data).unwrap_err().to_string()
}

fn mutated(offset: usize, bytes: &[u8]) -> Vec<u8> {
    let mut data = fixture("16k", "libfx_reloc.so");
    data[offset..offset + bytes.len()].copy_from_slice(bytes);
    data
}

fn load(flags: u32, vaddr: u64, memsz: u64, align: u64) -> Phdr {
    Phdr {
        kind: PT_LOAD,
        flags,
        offset: vaddr,
        vaddr,
        filesz: memsz,
        memsz,
        align,
    }
}

fn synthetic(loads: Vec<Phdr>) -> ElfImage {
    ElfImage {
        phdrs: loads,
        phoff: 0,
        code_sections: Vec::new(),
    }
}

#[test]
fn bionic_verify_elf_header_messages_for_malformed_inputs() {
    assert!(parse_error(&[0u8; 10])
        .contains("is too small to be an ELF executable: only found 10 bytes"));
    assert!(parse_error(&mutated(0, b"MEDI")).contains("has bad ELF magic: 4d454449"));
    assert!(parse_error(&mutated(4, &[1])).contains("is 32-bit instead of 64-bit"));
    assert!(parse_error(&mutated(4, &[3])).contains("has unknown ELF class: 3"));
    assert!(parse_error(&mutated(5, &[2])).contains("not little-endian: 2"));
    assert!(parse_error(&mutated(16, &[2, 0])).contains("has unexpected e_type: 2"));
    assert!(parse_error(&mutated(18, &[62, 0])).contains("instead of EM_AARCH64 (183)"));
    assert!(parse_error(&mutated(56, &[0, 0])).contains("has invalid e_phnum: 0"));
    assert!(parse_error(&mutated(56, &1171u16.to_le_bytes())).contains("has invalid e_phnum: 1171"));
    assert!(parse_error(&mutated(32, &0x10_0000u64.to_le_bytes()))
        .contains("has invalid phdr offset/size"));
}

#[test]
fn bionic_load_segments_checks_reject_bad_program_headers() {
    let at = |index: usize| 64 + index * 56;
    assert!(
        parse_error(&mutated(at(1) + 32, &0x100_0000u64.to_le_bytes()))
            .contains("past end of file")
    );
    assert!(parse_error(&mutated(at(2) + 4, &7u32.to_le_bytes()))
        .contains("W+E load segments are not allowed"));
    assert!(parse_error(&mutated(at(2) + 16, &0x100u64.to_le_bytes())).contains("different pages"));
    assert!(
        parse_error(&mutated(at(3) + 16, &0x4b5cu64.to_le_bytes()).as_slice())
            .contains("different pages")
    );
    assert!(parse_error(&mutated(at(1), &3u32.to_le_bytes()))
        .contains("is a PIE executable, not a library"));
}

#[test]
fn bionic_load_segments_overlapping_loads_are_rejected() {
    let mut data = fixture("16k", "libfx_reloc.so");
    let (first, second) = (64 + 56, 64 + 2 * 56);
    let vaddr = data[first + 16..first + 24].to_vec();
    let offset = data[first + 8..first + 16].to_vec();
    data[second + 16..second + 24].copy_from_slice(&vaddr);
    data[second + 8..second + 16].copy_from_slice(&offset);

    assert!(parse_error(&data).contains("overlaps the previous segment"));
}

#[test]
fn bionic_phdr_checks_require_a_dynamic_segment_inside_a_load() {
    let phdr_count = 10;
    let mut data = fixture("16k", "libfx_reloc.so");
    for index in 0..phdr_count {
        let at = 64 + index * 56;
        if data[at..at + 4] == 2u32.to_le_bytes() {
            data[at..at + 4].copy_from_slice(&0x6474e551u32.to_le_bytes());
        }
    }

    assert!(parse_error(&data).contains("missing PT_DYNAMIC in \"libx.so\""));
}

#[test]
fn bionic_elf_loader_ignores_payloads_through_is_arm64_elf() {
    assert!(is_arm64_elf(&fixture("16k", "libfx_add.so")));
    assert!(!is_arm64_elf(b"MEDIAMEDIAMEDIA"));
    assert!(!is_arm64_elf(&mutated(18, &[62, 0])));
}

#[test]
fn spike_image_layout_shift_is_zero_for_16k_and_0x2000_for_4k_fixtures() {
    for (variant, shift, region_len) in [("16k", 0, 0x10000), ("4k", 0x2000, 0x8000)] {
        let image = ElfImage::parse("libfx_reloc.so", &fixture(variant, "libfx_reloc.so")).unwrap();
        let layout = ImageLayout::plan(&image, HOST_PAGE).unwrap();

        assert_eq!(layout.shift, shift, "{variant}");
        assert_eq!(layout.region_len, region_len, "{variant}");
        assert_eq!(
            layout.region_offset(0x2e60) % HOST_PAGE,
            if variant == "4k" { 0xe60 } else { 0x2e60 },
            "{variant}"
        );
    }
}

#[test]
fn spike_page_protections_split_read_exec_and_write_pages_per_host_page() {
    let image = ElfImage::parse("libfx_reloc.so", &fixture("16k", "libfx_reloc.so")).unwrap();
    let ranges = ImageLayout::plan(&image, HOST_PAGE)
        .unwrap()
        .page_protections(&image)
        .unwrap();
    let summary: Vec<_> = ranges
        .iter()
        .map(|range| (range.start, range.len, range.prot))
        .collect();

    assert_eq!(
        summary,
        vec![
            (0, 0x4000, libc::PROT_READ),
            (0x4000, 0x4000, libc::PROT_READ | libc::PROT_EXEC),
            (0x8000, 0x8000, libc::PROT_READ | libc::PROT_WRITE),
        ]
    );
    let image = ElfImage::parse("libfx_reloc.so", &fixture("4k", "libfx_reloc.so")).unwrap();
    let ranges = ImageLayout::plan(&image, HOST_PAGE)
        .unwrap()
        .page_protections(&image)
        .unwrap();
    let protections: Vec<_> = ranges.iter().map(|range| range.prot).collect();
    assert_eq!(
        protections,
        vec![
            libc::PROT_READ | libc::PROT_EXEC,
            libc::PROT_READ | libc::PROT_WRITE
        ]
    );
}

#[test]
fn spike_host_page_needing_write_and_execute_is_a_hard_error() {
    let image = synthetic(vec![
        load(5, 0, 0x1b5c, 0x1000),
        load(6, 0x1e60, 0x400, 0x1000),
    ]);
    let layout = ImageLayout::plan(&image, HOST_PAGE).unwrap();
    let error = layout.page_protections(&image).unwrap_err().to_string();

    assert!(error.contains("needs both write and execute"), "{error}");
}

#[test]
fn spike_shift_rw_aligns_the_lowest_writable_4k_page_when_the_image_starts_above_zero() {
    let image = synthetic(vec![
        load(4, 0x1000, 0x1000, 0x1000),
        load(6, 0x5000, 0x1000, 0x1000),
    ]);
    let layout = ImageLayout::plan(&image, HOST_PAGE).unwrap();

    assert_eq!((layout.lo, layout.hi), (0x1000, 0x6000));
    assert_eq!(layout.region_offset(0x5000) % HOST_PAGE, 0);
    assert_eq!(layout.region_len % HOST_PAGE, 0);
}

#[test]
fn spike_relro_rounds_inward_to_whole_host_pages() {
    let image = synthetic(vec![load(6, 0, 0x20000, 0x4000)]);
    let layout = ImageLayout::plan(&image, HOST_PAGE).unwrap();
    let small = Phdr {
        kind: 0x6474e552,
        flags: 4,
        offset: 0,
        vaddr: 0x8e60,
        filesz: 0x11a0,
        memsz: 0x11a0,
        align: 1,
    };
    let large = Phdr {
        vaddr: 0x4010,
        memsz: 0x8000,
        ..small.clone()
    };

    assert!(layout.relro_pages(&small).is_none());
    let pages = layout.relro_pages(&large).unwrap();
    assert_eq!((pages.start, pages.len), (0x8000, 0x4000));
}

#[test]
fn measured_tiktok_npth_helpers_are_refused_as_executables() {
    let path = corpus_dir().join("libnpth_dumper.so");
    let Ok(data) = fs::read(path) else {
        eprintln!("skipped: corpus absent");
        return;
    };

    assert!(parse_error(&data).contains("PIE executable"));
}
