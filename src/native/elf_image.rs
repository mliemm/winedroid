use crate::error::{Result, WinedroidError};

pub const PT_LOAD: u32 = 1;
pub const PT_DYNAMIC: u32 = 2;
pub const PT_INTERP: u32 = 3;
pub const PT_PHDR: u32 = 6;
pub const PT_TLS: u32 = 7;
pub const PT_GNU_EH_FRAME: u32 = 0x6474_e550;
pub const PT_GNU_RELRO: u32 = 0x6474_e552;
pub const PF_X: u32 = 1;
pub const PF_W: u32 = 2;

const EM_AARCH64: u16 = 183;
const PHDR_SIZE: usize = 56;
const SHDR_SIZE: usize = 64;
const SHF_ALLOC: u64 = 0x2;
const SHF_EXECINSTR: u64 = 0x4;
const MAX_PHNUM: usize = 65536 / PHDR_SIZE;
const MAX_IMAGE_SIZE: u64 = 1 << 30;
const PAGE: u64 = 4096;

pub(crate) fn le_u16(data: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        data.get(offset..offset.checked_add(2)?)?.try_into().ok()?,
    ))
}

pub(crate) fn le_u32(data: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        data.get(offset..offset.checked_add(4)?)?.try_into().ok()?,
    ))
}

pub(crate) fn le_u64(data: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        data.get(offset..offset.checked_add(8)?)?.try_into().ok()?,
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Phdr {
    pub kind: u32,
    pub flags: u32,
    pub offset: u64,
    pub vaddr: u64,
    pub filesz: u64,
    pub memsz: u64,
    pub align: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsSegment {
    pub size: u64,
    pub alignment: u64,
    pub init_vaddr: u64,
    pub init_size: u64,
}

#[derive(Debug, Clone)]
pub struct ElfImage {
    pub phdrs: Vec<Phdr>,
    pub phoff: u64,
    pub code_sections: Vec<(u64, u64)>,
}

pub fn is_arm64_elf(data: &[u8]) -> bool {
    data.len() >= 64
        && data[..4] == *b"\x7fELF"
        && data[4] == 2
        && le_u16(data, 18) == Some(EM_AARCH64)
}

fn code_sections(data: &[u8]) -> Vec<(u64, u64)> {
    // cite: bionic linker_phdr.cpp#ReadSectionHeaders
    let table = le_u64(data, 40).unwrap_or(0);
    let count = usize::from(le_u16(data, 60).unwrap_or(0));
    if table == 0 || usize::from(le_u16(data, 58).unwrap_or(0)) != SHDR_SIZE {
        return Vec::new();
    }
    (0..count)
        .filter_map(|index| {
            let at = usize::try_from(table)
                .ok()?
                .checked_add(index * SHDR_SIZE)?;
            let flags = le_u64(data, at.checked_add(8)?)?;
            if flags & (SHF_ALLOC | SHF_EXECINSTR) != SHF_ALLOC | SHF_EXECINSTR {
                return None;
            }
            let start = le_u64(data, at.checked_add(16)?)?;
            Some((
                start,
                start.checked_add(le_u64(data, at.checked_add(32)?)?)?,
            ))
        })
        .collect()
}

fn floor_page(value: u64) -> u64 {
    value & !(PAGE - 1)
}

fn ceil_page(value: u64) -> Option<u64> {
    value.checked_add(PAGE - 1).map(floor_page)
}

impl ElfImage {
    pub fn parse(name: &str, data: &[u8]) -> Result<Self> {
        // cite: bionic linker_phdr.cpp#VerifyElfHeader, ReadProgramHeaders
        if data.len() < 64 {
            return Err(WinedroidError::new(format!(
                "\"{name}\" is too small to be an ELF executable: only found {} bytes",
                data.len()
            )));
        }
        if data[..4] != *b"\x7fELF" {
            return Err(WinedroidError::new(format!(
                "\"{name}\" has bad ELF magic: {:02x}{:02x}{:02x}{:02x}",
                data[0], data[1], data[2], data[3]
            )));
        }
        match data[4] {
            2 => {}
            1 => {
                return Err(WinedroidError::new(format!(
                    "\"{name}\" is 32-bit instead of 64-bit"
                )))
            }
            other => {
                return Err(WinedroidError::new(format!(
                    "\"{name}\" has unknown ELF class: {other}"
                )))
            }
        }
        if data[5] != 1 {
            return Err(WinedroidError::new(format!(
                "\"{name}\" not little-endian: {}",
                data[5]
            )));
        }
        let e_type = le_u16(data, 16).unwrap_or(0);
        if e_type != 3 {
            return Err(WinedroidError::new(format!(
                "\"{name}\" has unexpected e_type: {e_type}"
            )));
        }
        let version = le_u32(data, 20).unwrap_or(0);
        if version != 1 {
            return Err(WinedroidError::new(format!(
                "\"{name}\" has unexpected e_version: {version}"
            )));
        }
        let machine = le_u16(data, 18).unwrap_or(0);
        if machine != EM_AARCH64 {
            return Err(WinedroidError::new(format!(
                "\"{name}\" is for EM_??? ({machine}) instead of EM_AARCH64 ({EM_AARCH64})"
            )));
        }
        let phoff = le_u64(data, 32).unwrap_or(0);
        let entry_size = le_u16(data, 54).unwrap_or(0);
        let phnum = usize::from(le_u16(data, 56).unwrap_or(0));
        if usize::from(entry_size) != PHDR_SIZE || phnum == 0 || phnum > MAX_PHNUM {
            return Err(WinedroidError::new(format!(
                "\"{name}\" has invalid e_phnum: {phnum}"
            )));
        }
        let table_end = phoff.checked_add((phnum * PHDR_SIZE) as u64);
        if phoff == 0 || phoff % 8 != 0 || table_end.is_none_or(|end| end > data.len() as u64) {
            return Err(WinedroidError::new(format!(
                "\"{name}\" has invalid phdr offset/size: {phoff}/{}",
                phnum * PHDR_SIZE
            )));
        }
        let mut phdrs = Vec::with_capacity(phnum);
        for index in 0..phnum {
            let at = phoff as usize + index * PHDR_SIZE;
            let field64 = |delta: usize| le_u64(data, at + delta).unwrap_or(0);
            phdrs.push(Phdr {
                kind: le_u32(data, at).unwrap_or(0),
                flags: le_u32(data, at + 4).unwrap_or(0),
                offset: field64(8),
                vaddr: field64(16),
                filesz: field64(32),
                memsz: field64(40),
                align: field64(48),
            });
        }
        let image = Self {
            phdrs,
            phoff,
            code_sections: code_sections(data),
        };
        image.validate(name, data.len() as u64)?;
        Ok(image)
    }

    fn validate(&self, name: &str, file_len: u64) -> Result<()> {
        // cite: bionic linker_phdr.cpp#LoadSegments, CheckFileRange, FindPhdr, CheckPhdr
        if self.phdrs.iter().any(|phdr| phdr.kind == PT_INTERP) {
            return Err(WinedroidError::new(format!(
                "\"{name}\" is a PIE executable, not a library"
            )));
        }
        let loads: Vec<&Phdr> = self.loads().collect();
        if loads.is_empty() {
            return Err(WinedroidError::new(format!(
                "\"{name}\" has no loadable segments"
            )));
        }
        let mut previous_end = 0u64;
        for (index, load) in loads.iter().enumerate() {
            let file_end = load.offset.checked_add(load.filesz);
            if load.memsz < load.filesz || file_end.is_none_or(|end| end > file_len) {
                return Err(WinedroidError::new(format!(
                    "invalid ELF file \"{name}\" load segment[{index}]: p_offset ({}) + p_filesz ({}) past end of file ({file_len})",
                    load.offset, load.filesz
                )));
            }
            if load.flags & PF_W != 0 && load.flags & PF_X != 0 {
                return Err(WinedroidError::new(format!(
                    "\"{name}\": W+E load segments are not allowed"
                )));
            }
            if load.vaddr % PAGE != load.offset % PAGE {
                return Err(WinedroidError::new(format!(
                    "\"{name}\" load segment[{index}] has p_vaddr and p_offset in different pages"
                )));
            }
            let end = load
                .vaddr
                .checked_add(load.memsz)
                .filter(|end| *end <= MAX_IMAGE_SIZE);
            let Some(end) = end else {
                return Err(WinedroidError::new(format!(
                    "\"{name}\" load segment[{index}] is outside the supported image size"
                )));
            };
            if index > 0 && load.vaddr < previous_end {
                return Err(WinedroidError::new(format!(
                    "\"{name}\" load segment[{index}] overlaps the previous segment"
                )));
            }
            previous_end = end;
        }
        let dynamics: Vec<&Phdr> = self
            .phdrs
            .iter()
            .filter(|phdr| phdr.kind == PT_DYNAMIC)
            .collect();
        let [dynamic] = dynamics.as_slice() else {
            return Err(WinedroidError::new(format!(
                "missing PT_DYNAMIC in \"{name}\""
            )));
        };
        if dynamic.filesz % 16 != 0 || !self.in_file_backed_load(dynamic.vaddr, dynamic.filesz) {
            return Err(WinedroidError::new(format!(
                "\"{name}\" has an invalid PT_DYNAMIC range"
            )));
        }
        if self.phdrs.iter().filter(|phdr| phdr.kind == PT_TLS).count() > 1 {
            return Err(WinedroidError::new(format!(
                "\"{name}\" has more than one PT_TLS"
            )));
        }
        if let Some(tls) = self.tls() {
            // cite: bionic bionic_elf_tls.cpp#__bionic_check_tls_alignment
            if tls.init_size > tls.size || tls.alignment == 0 || !tls.alignment.is_power_of_two() {
                return Err(WinedroidError::new(format!(
                    "TLS segment alignment in \"{name}\" is not a power of 2: {}",
                    tls.alignment
                )));
            }
        }
        let phdr_len = (self.phdrs.len() * PHDR_SIZE) as u64;
        let phdr_vaddr = self
            .phdr_vaddr()
            .ok_or_else(|| WinedroidError::new(format!("can't find loaded phdr for \"{name}\"")))?;
        if !self.in_file_backed_load(phdr_vaddr, phdr_len) {
            return Err(WinedroidError::new(format!(
                "\"{name}\" loaded phdr {phdr_vaddr:#x} not in loadable segment"
            )));
        }
        Ok(())
    }

    fn in_file_backed_load(&self, vaddr: u64, len: u64) -> bool {
        let Some(end) = vaddr.checked_add(len) else {
            return false;
        };
        self.loads()
            .any(|load| vaddr >= load.vaddr && end <= load.vaddr + load.filesz)
    }

    pub fn loads(&self) -> impl Iterator<Item = &Phdr> {
        self.phdrs.iter().filter(|phdr| phdr.kind == PT_LOAD)
    }

    pub fn dynamic(&self) -> &Phdr {
        self.phdrs
            .iter()
            .find(|phdr| phdr.kind == PT_DYNAMIC)
            .expect("validated PT_DYNAMIC")
    }

    pub fn phdr_vaddr(&self) -> Option<u64> {
        // cite: bionic linker_phdr.cpp#FindPhdr
        if let Some(phdr) = self.phdrs.iter().find(|phdr| phdr.kind == PT_PHDR) {
            return Some(phdr.vaddr);
        }
        let first = self.loads().next()?;
        (first.offset == 0).then(|| first.vaddr + self.phoff)
    }

    pub fn tls(&self) -> Option<TlsSegment> {
        // cite: bionic bionic_elf_tls.cpp#__bionic_get_tls_segment, __bionic_check_tls_alignment
        let phdr = self.phdrs.iter().find(|phdr| phdr.kind == PT_TLS)?;
        Some(TlsSegment {
            size: phdr.memsz,
            alignment: phdr.align.min(PAGE),
            init_vaddr: phdr.vaddr,
            init_size: phdr.filesz,
        })
    }

    pub fn relro(&self) -> Option<&Phdr> {
        self.phdrs.iter().find(|phdr| phdr.kind == PT_GNU_RELRO)
    }

    pub fn eh_frame_hdr(&self) -> Option<&Phdr> {
        self.phdrs
            .iter()
            .find(|phdr| phdr.kind == PT_GNU_EH_FRAME && phdr.memsz != 0)
    }

    pub fn image_bounds(&self) -> (u64, u64) {
        let lo = self
            .loads()
            .map(|load| floor_page(load.vaddr))
            .min()
            .unwrap_or(0);
        let hi = self
            .loads()
            .map(|load| ceil_page(load.vaddr + load.memsz).unwrap_or(u64::MAX))
            .max()
            .unwrap_or(0);
        (lo, hi)
    }
}
