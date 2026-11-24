use crate::error::{Result, WinedroidError};

use super::elf_image::{ElfImage, Phdr, PF_W, PF_X};

pub const ELF_PAGE: u64 = 4096;
pub const HOST_PAGE: u64 = 16384;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageLayout {
    pub lo: u64,
    pub hi: u64,
    pub shift: u64,
    pub region_len: u64,
    pub host_page: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageRange {
    pub start: u64,
    pub len: u64,
    pub prot: i32,
}

fn round_up(value: u64, unit: u64) -> u64 {
    value.div_ceil(unit) * unit
}

impl ImageLayout {
    pub fn plan(image: &ElfImage, host_page: u64) -> Result<Self> {
        // cite: bionic linker_phdr.cpp#phdr_table_get_load_size
        let (lo, hi) = image.image_bounds();
        if hi <= lo {
            return Err(WinedroidError::new("image has no loadable bytes"));
        }
        let aligned = image.loads().all(|load| load.align >= host_page) && lo % host_page == 0;
        let lowest_writable = image.loads().filter(|load| load.flags & PF_W != 0).map(|load| load.vaddr & !(ELF_PAGE - 1)).min();
        let shift = if aligned { 0 } else { shift_for(lo, lowest_writable, host_page) };
        Ok(Self {
            lo,
            hi,
            shift,
            region_len: round_up(shift + hi - lo, host_page),
            host_page,
        })
    }

    pub fn region_offset(&self, vaddr: u64) -> u64 {
        vaddr + self.shift - self.lo
    }

    pub fn page_protections(&self, image: &ElfImage) -> Result<Vec<PageRange>> {
        // cite: bionic linker_phdr.cpp#phdr_table_protect_segments
        let pages = (self.region_len / self.host_page) as usize;
        let mut flags = vec![None::<u32>; pages];
        for load in image.loads().filter(|load| load.memsz > 0) {
            let start = self.region_offset(load.vaddr);
            let end = start + load.memsz;
            for page in (start / self.host_page)..=((end - 1) / self.host_page) {
                let slot = &mut flags[page as usize];
                *slot = Some(slot.unwrap_or(0) | load.flags);
            }
        }
        let mut ranges: Vec<PageRange> = Vec::new();
        for (index, union) in flags.iter().enumerate() {
            let page_start = index as u64 * self.host_page;
            let prot = match union {
                None => libc::PROT_NONE,
                Some(union) if union & PF_X != 0 && union & PF_W != 0 => {
                    return Err(WinedroidError::new(format!(
                        "host page at image offset {page_start:#x} needs both write and execute"
                    )));
                }
                Some(union) if union & PF_X != 0 => libc::PROT_READ | libc::PROT_EXEC,
                Some(union) if union & PF_W != 0 => libc::PROT_READ | libc::PROT_WRITE,
                Some(_) => libc::PROT_READ,
            };
            match ranges.last_mut() {
                Some(last) if last.prot == prot => last.len += self.host_page,
                _ => ranges.push(PageRange { start: page_start, len: self.host_page, prot }),
            }
        }
        Ok(ranges)
    }

    pub fn relro_pages(&self, relro: &Phdr) -> Option<PageRange> {
        // cite: bionic linker_phdr.cpp#phdr_table_protect_gnu_relro
        let start = round_up(self.region_offset(relro.vaddr), self.host_page);
        let end = (self.region_offset(relro.vaddr) + relro.memsz) / self.host_page * self.host_page;
        (end > start).then(|| PageRange { start, len: end - start, prot: libc::PROT_READ })
    }
}

fn shift_for(lo: u64, lowest_writable: Option<u64>, host_page: u64) -> u64 {
    let Some(lowest) = lowest_writable else {
        return 0;
    };
    let offset_of_writable = lowest - lo;
    (host_page - offset_of_writable % host_page) % host_page
}
