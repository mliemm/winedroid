use crate::error::{Result, WinedroidError};

use super::elf_image::{ElfImage, PF_X};
use super::image_map::MappedImage;
use super::loader::ImagePatcher;
use super::thread_block::block;
use super::tp_veneer::redirect_site;

const MRS_TPIDR_EL0: u32 = 0xD53B_D040;
const MSR_TPIDR_EL0: u32 = 0xD51B_D040;
const REGISTER_MASK: u32 = 0xFFFF_FFE0;
const SHADOW_STACK_PUSH: u32 = 0xF800_865E;
const SHADOW_STACK_POP: u32 = 0xF85F_8E5E;
const NOP: u32 = 0xD503_201F;
const MOV_X18_REGISTER: u32 = 0x2A00_03F2;
const MOV_X18_REGISTER_MASK: u32 = 0x7FE0_FFFF;
const MOV_X18_IMMEDIATE_WIDE: u32 = 0x1280_0012;
const MOV_X18_IMMEDIATE_WIDE_MASK: u32 = 0x1F80_001F;
const ORR_X18_IMMEDIATE: u32 = 0x3200_03F2;
const ORR_X18_IMMEDIATE_MASK: u32 = 0x7F80_03FF;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PatchCounts {
    pub thread_pointer_reads: usize,
    pub shadow_stack_ops: usize,
    pub x18_writes: usize,
}

pub struct TextPatcher;

fn writes_x18(word: u32) -> bool {
    word & MOV_X18_REGISTER_MASK == MOV_X18_REGISTER
        || word & MOV_X18_IMMEDIATE_WIDE_MASK == MOV_X18_IMMEDIATE_WIDE
        || word & ORR_X18_IMMEDIATE_MASK == ORR_X18_IMMEDIATE
}

pub fn count_thread_pointer_reads(code: &[u8]) -> usize {
    code.chunks_exact(4)
        .filter(|chunk| {
            u32::from_le_bytes((*chunk).try_into().expect("4 bytes")) & REGISTER_MASK
                == MRS_TPIDR_EL0
        })
        .count()
}

pub type Redirect<'a> = &'a mut dyn FnMut(usize, u32) -> Result<u32>;

pub fn patch_words(code: &mut [u8], base: usize, redirect: Redirect) -> Result<PatchCounts> {
    let mut counts = PatchCounts::default();
    for (index, chunk) in code.chunks_exact_mut(4).enumerate() {
        let word = u32::from_le_bytes(chunk.try_into().expect("4 bytes"));
        let replacement = if word & REGISTER_MASK == MRS_TPIDR_EL0 {
            counts.thread_pointer_reads += 1;
            redirect(base + index * 4, word & 0x1f)?
        } else if word & REGISTER_MASK == MSR_TPIDR_EL0 {
            return Err(WinedroidError::new(
                "guest code writes tpidr_el0; next missing primitive: msr tpidr_el0",
            ));
        } else if word == SHADOW_STACK_PUSH || word == SHADOW_STACK_POP {
            counts.shadow_stack_ops += 1;
            NOP
        } else {
            if writes_x18(word) {
                counts.x18_writes += 1;
            }
            continue;
        };
        chunk.copy_from_slice(&replacement.to_le_bytes());
    }
    Ok(counts)
}

fn read_encoded(image: &MappedImage, vaddr: u64, encoding: u8) -> Result<(u64, u64)> {
    // cite: art dwarf_constants.h#DW_EH_PE_pcrel, DW_EH_PE_datarel
    let size_of = |bytes: u64| -> Result<&[u8]> { image.bytes(vaddr, bytes) };
    let (value, length) = match encoding & 0x0f {
        0x03 => (u64::from(u32::from_le_bytes(size_of(4)?.try_into().expect("4 bytes"))), 4),
        0x0b => (i64::from(i32::from_le_bytes(size_of(4)?.try_into().expect("4 bytes"))) as u64, 4),
        0x04 | 0x0c => (u64::from_le_bytes(size_of(8)?.try_into().expect("8 bytes")), 8),
        other => {
            return Err(WinedroidError::new(format!(
                "unsupported eh_frame pointer encoding {other:#x}; next missing primitive: eh_frame pointer encoding {other:#x}"
            )))
        }
    };
    let relative = match encoding & 0x70 {
        0x10 => vaddr,
        _ => 0,
    };
    Ok((value.wrapping_add(relative), length))
}

fn uleb(image: &MappedImage, vaddr: &mut u64) -> Result<u64> {
    let mut value = 0u64;
    let mut shift = 0;
    loop {
        let byte = image.bytes(*vaddr, 1)?[0];
        *vaddr += 1;
        value |= u64::from(byte & 0x7f) << shift;
        shift += 7;
        if byte & 0x80 == 0 || shift > 63 {
            return Ok(value);
        }
    }
}

fn fde_encoding(image: &MappedImage, cie: u64) -> Result<u8> {
    let mut at = cie + 8;
    let version = image.bytes(at, 1)?[0];
    at += 1;
    let augmentation = image.cstr(at, 16)?.to_vec();
    at += augmentation.len() as u64 + 1;
    uleb(image, &mut at)?;
    uleb(image, &mut at)?;
    if version == 1 {
        at += 1;
    } else {
        uleb(image, &mut at)?;
    }
    if augmentation.first() != Some(&b'z') {
        return Ok(0);
    }
    uleb(image, &mut at)?;
    for letter in &augmentation[1..] {
        match letter {
            b'R' => return Ok(image.bytes(at, 1)?[0]),
            b'L' => at += 1,
            b'P' => {
                let encoding = image.bytes(at, 1)?[0];
                at += 1;
                at += read_encoded(image, at, encoding & 0x0f)?.1;
            }
            _ => {}
        }
    }
    Ok(0)
}

fn function_ranges(image: &MappedImage, elf: &ElfImage) -> Result<Option<Vec<(u64, u64)>>> {
    let Some(header) = elf.eh_frame_hdr() else {
        return Ok(None);
    };
    let base = header.vaddr;
    let encodings = image.bytes(base, 4)?;
    let (version, pointer_encoding, count_encoding, table_encoding) =
        (encodings[0], encodings[1], encodings[2], encodings[3]);
    if version != 1 || table_encoding != 0x3b {
        return Err(WinedroidError::new(
            "unsupported eh_frame_hdr layout; next missing primitive: eh_frame_hdr layout",
        ));
    }
    let (_, pointer_length) = read_encoded(image, base + 4, pointer_encoding)?;
    let (count, count_length) = read_encoded(image, base + 4 + pointer_length, count_encoding)?;
    let table = base + 4 + pointer_length + count_length;
    let mut ranges = Vec::with_capacity(count as usize);
    for index in 0..count {
        let entry = image.bytes(table + index * 8, 8)?;
        let fde = base.wrapping_add(i64::from(i32::from_le_bytes(
            entry[4..8].try_into().expect("4 bytes"),
        )) as u64);
        let cie_offset = u64::from(u32::from_le_bytes(
            image.bytes(fde + 4, 4)?.try_into().expect("4 bytes"),
        ));
        let encoding = fde_encoding(image, fde + 4 - cie_offset)?;
        let (begin, begin_length) = read_encoded(image, fde + 8, encoding)?;
        let (range, _) = read_encoded(image, fde + 8 + begin_length, encoding & 0x0f)?;
        ranges.push((begin, begin + range));
    }
    Ok(Some(ranges))
}

impl ImagePatcher for TextPatcher {
    fn patch(&self, image: &MappedImage, elf: &ElfImage) -> Result<PatchCounts> {
        let functions = function_ranges(image, elf)?;
        let bias = block().bias();
        let mut total = PatchCounts::default();
        for load in elf.loads().filter(|load| load.flags & PF_X != 0) {
            let segment = (load.vaddr, load.vaddr + load.filesz);
            let spans: Vec<(u64, u64)> = match &functions {
                Some(ranges) => ranges
                    .iter()
                    .map(|(start, end)| ((*start).max(segment.0), (*end).min(segment.1)))
                    .filter(|(start, end)| start < end)
                    .collect(),
                None => vec![segment],
            };
            for (start, end) in spans {
                let mut redirect =
                    |site: usize, register: u32| redirect_site(image, site, register, bias);
                let counts = patch_words(
                    image.bytes_mut(start, (end - start) & !3)?,
                    image.address(start),
                    &mut redirect,
                )?;
                total.thread_pointer_reads += counts.thread_pointer_reads;
                total.shadow_stack_ops += counts.shadow_stack_ops;
                total.x18_writes += counts.x18_writes;
            }
        }
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(values: &[u32]) -> Vec<u8> {
        values
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect()
    }

    fn keep(_: usize, register: u32) -> Result<u32> {
        Ok(0xD53B_D060 | register)
    }

    #[test]
    fn aarch64_mrs_tpidr_el0_is_handed_to_the_redirect_with_its_site_and_destination_register() {
        let mut code = words(&[0xD53BD048, 0xD53BD040, 0xD503201F, 0xD53BD05E]);
        let mut seen = Vec::new();
        let mut record = |site: usize, register: u32| {
            seen.push((site, register));
            keep(site, register)
        };
        let counts = patch_words(&mut code, 0x1000, &mut record).unwrap();

        assert_eq!(counts.thread_pointer_reads, 3);
        assert_eq!(seen, vec![(0x1000, 8), (0x1004, 0), (0x100c, 30)]);
        assert_eq!(
            code,
            words(&[0xD53BD068, 0xD53BD060, 0xD503201F, 0xD53BD07E])
        );
    }

    #[test]
    fn aarch64_thread_pointer_reads_are_counted_in_the_file_bytes_before_mapping() {
        let code = words(&[0xD53BD048, 0xD53BD060, 0xD53BD05F, 0xD503201F]);

        assert_eq!(count_thread_pointer_reads(&code), 2);
    }

    #[test]
    fn aarch64_shadow_call_stack_push_and_pop_become_nops() {
        let mut code = words(&[0xF800865E, 0xF85F8E5E, 0xF800865F]);
        let counts = patch_words(&mut code, 0, &mut keep).unwrap();

        assert_eq!(counts.shadow_stack_ops, 2);
        assert_eq!(code, words(&[0xD503201F, 0xD503201F, 0xF800865F]));
    }

    #[test]
    fn aarch64_msr_tpidr_el0_is_an_explicit_error() {
        let error = patch_words(&mut words(&[0xD51BD041]), 0, &mut keep)
            .unwrap_err()
            .to_string();

        assert!(error.contains("next missing primitive: msr tpidr_el0"));
    }

    #[test]
    fn aarch64_mov_into_x18_is_counted_and_left_in_place() {
        let mut code = words(&[0xAA0003F2, 0xAA0103F2, 0xD2800012, 0xAA0003F3, 0xD2800013]);
        let before = code.clone();
        let counts = patch_words(&mut code, 0, &mut keep).unwrap();

        assert_eq!(counts.x18_writes, 3);
        assert_eq!(code, before);
    }

    #[test]
    fn aarch64_wide_immediate_and_logical_immediate_writes_to_x18_are_counted_in_both_widths() {
        let writes = [
            0x9280_0012,
            0xF280_0012,
            0x5280_0012,
            0x2A00_03F2,
            0xB240_03F2,
            0x3240_03F2,
        ];
        let others = [0x9280_0013, 0xB240_03F3, 0xAA12_03E0];
        let mut code = words(&[writes.as_slice(), others.as_slice()].concat());
        let before = code.clone();
        let counts = patch_words(&mut code, 0, &mut keep).unwrap();

        assert_eq!(counts.x18_writes, writes.len());
        assert_eq!(code, before);
    }

    #[test]
    fn bionic_eh_frame_hdr_function_ranges_exclude_the_literal_pool_of_the_order_fixture() {
        use crate::native::fixture_elf::FixtureElf;
        use crate::native::loader_test_support::fixture_dir;

        for variant in ["16k", "4k"] {
            let path = fixture_dir(variant).join("libfx_order_child.so");
            let data = std::fs::read(&path).unwrap();
            let elf = ElfImage::parse("libfx_order_child.so", &data).unwrap();
            let image = MappedImage::map("libfx_order_child.so", &data, &elf).unwrap();
            let ranges = function_ranges(&image, &elf)
                .unwrap()
                .expect("fixture has PT_GNU_EH_FRAME");
            let exports = FixtureElf::parse(&data).export_addresses;
            let covered = |name: &str| {
                ranges
                    .iter()
                    .any(|(start, end)| exports[name] >= *start && exports[name] < *end)
            };

            assert!(ranges.iter().all(|(start, end)| start < end), "{variant}");
            assert!(covered("fx_pool_word"), "{variant}");
            assert!(covered("fx_write_x18"), "{variant}");
            assert!(
                !covered("fx_pool_words"),
                "{variant} literal pool must not be inside an FDE"
            );
        }
    }
}
