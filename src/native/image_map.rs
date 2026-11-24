use crate::error::{Result, WinedroidError};

use super::elf_image::ElfImage;
use super::image_layout::{ImageLayout, PageRange};

extern "C" {
    fn sys_icache_invalidate(start: *mut libc::c_void, length: usize);
}

pub fn host_page_size() -> u64 {
    // cite: bionic libc.map.txt#getpagesize
    (unsafe { libc::sysconf(libc::_SC_PAGESIZE) }).max(4096) as u64
}

pub struct MappedImage {
    region: *mut u8,
    layout: ImageLayout,
}

unsafe impl Send for MappedImage {}
unsafe impl Sync for MappedImage {}

impl MappedImage {
    pub fn map(name: &str, data: &[u8], image: &ElfImage) -> Result<Self> {
        let layout = ImageLayout::plan(image, host_page_size())?;
        let region = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                layout.region_len as usize,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANON,
                -1,
                0,
            )
        };
        if region == libc::MAP_FAILED {
            return Err(WinedroidError::new(format!(
                "couldn't reserve {} bytes of address space for \"{name}\"",
                layout.region_len
            )));
        }
        let mapped = Self { region: region as *mut u8, layout };
        for load in image.loads() {
            let source = &data[load.offset as usize..(load.offset + load.filesz) as usize];
            let target = mapped.bytes_mut(load.vaddr, load.filesz)?;
            target.copy_from_slice(source);
        }
        Ok(mapped)
    }

    pub fn layout(&self) -> &ImageLayout {
        &self.layout
    }

    pub fn region(&self) -> usize {
        self.region as usize
    }

    pub fn region_len(&self) -> usize {
        self.layout.region_len as usize
    }

    pub fn load_bias(&self) -> usize {
        self.region as usize + self.layout.shift as usize - self.layout.lo as usize
    }

    pub fn address(&self, vaddr: u64) -> usize {
        self.load_bias().wrapping_add(vaddr as usize)
    }

    pub fn contains_vaddr(&self, vaddr: u64, len: u64) -> bool {
        vaddr >= self.layout.lo && vaddr.checked_add(len).is_some_and(|end| end <= self.layout.hi)
    }

    pub fn bytes(&self, vaddr: u64, len: u64) -> Result<&[u8]> {
        self.check(vaddr, len)?;
        Ok(unsafe { std::slice::from_raw_parts(self.address(vaddr) as *const u8, len as usize) })
    }

    pub fn bytes_mut(&self, vaddr: u64, len: u64) -> Result<&mut [u8]> {
        self.check(vaddr, len)?;
        Ok(unsafe { std::slice::from_raw_parts_mut(self.address(vaddr) as *mut u8, len as usize) })
    }

    fn check(&self, vaddr: u64, len: u64) -> Result<()> {
        if self.contains_vaddr(vaddr, len) {
            return Ok(());
        }
        Err(WinedroidError::new(format!(
            "range {vaddr:#x}+{len:#x} is outside the image {:#x}..{:#x}",
            self.layout.lo, self.layout.hi
        )))
    }

    pub fn read_u64(&self, vaddr: u64) -> Result<u64> {
        Ok(u64::from_le_bytes(self.bytes(vaddr, 8)?.try_into().expect("8 bytes")))
    }

    pub fn write_u64(&self, vaddr: u64, value: u64) -> Result<()> {
        self.bytes_mut(vaddr, 8)?.copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    pub fn cstr(&self, vaddr: u64, limit: u64) -> Result<&[u8]> {
        let bytes = self.bytes(vaddr, limit)?;
        let end = bytes.iter().position(|byte| *byte == 0).ok_or_else(|| {
            WinedroidError::new(format!("string at {vaddr:#x} is not terminated inside {limit} bytes"))
        })?;
        Ok(&bytes[..end])
    }

    pub fn protect_final(&self, image: &ElfImage) -> Result<()> {
        // cite: bionic linker_phdr.cpp#phdr_table_protect_segments
        for range in self.layout.page_protections(image)? {
            self.protect(range)?;
            if range.prot & libc::PROT_EXEC != 0 {
                unsafe { sys_icache_invalidate((self.region as usize + range.start as usize) as *mut _, range.len as usize) };
            }
        }
        Ok(())
    }

    pub fn protect_relro(&self, image: &ElfImage) -> Result<()> {
        // cite: bionic linker_phdr.cpp#phdr_table_protect_gnu_relro
        match image.relro().and_then(|relro| self.layout.relro_pages(relro)) {
            Some(range) => self.protect(range),
            None => Ok(()),
        }
    }

    fn protect(&self, range: PageRange) -> Result<()> {
        let status = unsafe { libc::mprotect((self.region as usize + range.start as usize) as *mut _, range.len as usize, range.prot) };
        if status != 0 {
            return Err(WinedroidError::new(format!(
                "mprotect({:#x}, {:#x}, {}) failed: {}",
                range.start,
                range.len,
                range.prot,
                std::io::Error::last_os_error()
            )));
        }
        Ok(())
    }
}

impl Drop for MappedImage {
    fn drop(&mut self) {
        unsafe { libc::munmap(self.region as *mut _, self.layout.region_len as usize) };
    }
}
