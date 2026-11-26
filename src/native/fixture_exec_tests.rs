use std::fs;
use std::path::PathBuf;

use super::fixture_elf::{FixtureElf, PT_LOAD};

const IMAGE_ALIGN: usize = 0x4000;

extern "C" {
    fn sys_icache_invalidate(start: *mut libc::c_void, length: usize);
}

struct Image {
    base: *mut u8,
    length: usize,
    elf: FixtureElf,
}

impl Image {
    fn map(variant: &str, name: &str) -> Self {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/native_fixtures/bin")
            .join(variant)
            .join(name);
        let data = fs::read(&path).unwrap();
        let elf = FixtureElf::parse(&data);
        assert!(
            elf.relocations.is_empty(),
            "{variant}/{name} needs relocations, raw mapping is invalid"
        );
        let end = elf
            .segments
            .iter()
            .filter(|segment| segment.kind == PT_LOAD)
            .map(|segment| (segment.vaddr + segment.memsz) as usize)
            .max()
            .unwrap();
        let length = end.div_ceil(IMAGE_ALIGN) * IMAGE_ALIGN;
        let base = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                length,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANON,
                -1,
                0,
            )
        };
        assert_ne!(base, libc::MAP_FAILED);
        let base = base as *mut u8;
        for segment in elf
            .segments
            .iter()
            .filter(|segment| segment.kind == PT_LOAD)
        {
            let source = &data[segment.offset as usize..(segment.offset + segment.filesz) as usize];
            unsafe {
                std::ptr::copy_nonoverlapping(
                    source.as_ptr(),
                    base.add(segment.vaddr as usize),
                    source.len(),
                )
            };
        }
        let status =
            unsafe { libc::mprotect(base as *mut _, length, libc::PROT_READ | libc::PROT_EXEC) };
        assert_eq!(status, 0);
        unsafe { sys_icache_invalidate(base as *mut _, length) };
        Self { base, length, elf }
    }

    fn address(&self, symbol: &str) -> *const u8 {
        unsafe { self.base.add(self.elf.export_addresses[symbol] as usize) }
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe { libc::munmap(self.base as *mut _, self.length) };
    }
}

#[test]
fn fixture_source_fx_add_binary_adds_its_arguments_and_exports_the_data_value() {
    for variant in ["16k", "4k"] {
        let image = Image::map(variant, "libfx_add.so");
        let add: extern "C" fn(i32, i32) -> i32 =
            unsafe { std::mem::transmute(image.address("add")) };
        let value = unsafe { *(image.address("fx_data_value") as *const i32) };

        assert_eq!(add(2, 3), 5, "{variant}");
        assert_eq!(add(10, 4), 14, "{variant}");
        assert_eq!(add(-7, 3), -4, "{variant}");
        assert_eq!(value, 0x1234, "{variant}");
    }
}
