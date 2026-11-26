use crate::error::{Result, WinedroidError};

use super::dynamic_table::DynamicInfo;
use super::image_map::MappedImage;

pub const STB_LOCAL: u8 = 0;
pub const STB_GLOBAL: u8 = 1;
pub const STB_WEAK: u8 = 2;
pub const STT_TLS: u8 = 6;
pub const STT_GNU_IFUNC: u8 = 10;
pub const SHN_UNDEF: u16 = 0;
const SYM_SIZE: u64 = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sym {
    pub name: u32,
    pub info: u8,
    pub shndx: u16,
    pub value: u64,
    pub size: u64,
}

impl Sym {
    pub fn bind(&self) -> u8 {
        self.info >> 4
    }

    pub fn kind(&self) -> u8 {
        self.info & 0xf
    }

    pub fn is_global_and_defined(&self) -> bool {
        // cite: bionic linker_relocate.h#is_symbol_global_and_defined
        matches!(self.bind(), STB_GLOBAL | STB_WEAK) && self.shndx != SHN_UNDEF
    }
}

#[derive(Debug, Clone)]
struct Gnu {
    nbuckets: u32,
    symndx: u32,
    maskwords_m1: u32,
    shift2: u32,
    bloom: u64,
    buckets: u64,
    chain: u64,
}

#[derive(Debug, Clone)]
struct Sysv {
    nbucket: u32,
    nchain: u32,
    buckets: u64,
    chain: u64,
}

#[derive(Debug, Clone)]
pub struct SymbolTable {
    symtab: u64,
    strtab: u64,
    strsz: u64,
    count: u64,
    gnu: Option<Gnu>,
    sysv: Option<Sysv>,
}

pub fn gnu_hash(name: &[u8]) -> u32 {
    // cite: bionic linker_gnu_hash.h#calculate_gnu_hash_simple
    name.iter().fold(5381u32, |hash, byte| {
        hash.wrapping_add(hash << 5).wrapping_add(u32::from(*byte))
    })
}

pub fn elf_hash(name: &[u8]) -> u32 {
    // cite: bionic linker_soinfo.cpp#calculate_elf_hash
    let mut hash = 0u32;
    for byte in name {
        hash = (hash << 4).wrapping_add(u32::from(*byte));
        let high = hash & 0xf000_0000;
        hash ^= high;
        hash ^= high >> 24;
    }
    hash
}

fn u32_at(image: &MappedImage, vaddr: u64) -> Result<u32> {
    Ok(u32::from_le_bytes(
        image.bytes(vaddr, 4)?.try_into().expect("4 bytes"),
    ))
}

impl SymbolTable {
    pub fn new(name: &str, image: &MappedImage, dynamic: &DynamicInfo) -> Result<Self> {
        let gnu = if dynamic.gnu_hash != 0 {
            Some(read_gnu(name, image, dynamic.gnu_hash)?)
        } else {
            None
        };
        let sysv = if dynamic.hash != 0 {
            Some(read_sysv(image, dynamic.hash)?)
        } else {
            None
        };
        let count = match (&sysv, &gnu) {
            (Some(sysv), _) => u64::from(sysv.nchain),
            (None, Some(gnu)) => gnu_symbol_count(image, gnu)?,
            (None, None) => 0,
        };
        Ok(Self {
            symtab: dynamic.symtab,
            strtab: dynamic.strtab,
            strsz: dynamic.strsz,
            count,
            gnu,
            sysv,
        })
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn gnu_derived_count(&self, image: &MappedImage) -> Option<Result<u64>> {
        self.gnu.as_ref().map(|gnu| gnu_symbol_count(image, gnu))
    }

    pub fn has_gnu_hash(&self) -> bool {
        self.gnu.is_some()
    }

    pub fn has_sysv_hash(&self) -> bool {
        self.sysv.is_some()
    }

    pub fn symbol(&self, image: &MappedImage, index: u64) -> Result<Sym> {
        if index >= self.count {
            return Err(WinedroidError::new(format!(
                "symbol index {index} is outside the symbol table of {} entries",
                self.count
            )));
        }
        let entry = image.bytes(self.symtab + index * SYM_SIZE, SYM_SIZE)?;
        Ok(Sym {
            name: u32::from_le_bytes(entry[0..4].try_into().expect("4 bytes")),
            info: entry[4],
            shndx: u16::from_le_bytes(entry[6..8].try_into().expect("2 bytes")),
            value: u64::from_le_bytes(entry[8..16].try_into().expect("8 bytes")),
            size: u64::from_le_bytes(entry[16..24].try_into().expect("8 bytes")),
        })
    }

    pub fn name_bytes<'a>(&self, image: &'a MappedImage, sym: &Sym) -> Result<&'a [u8]> {
        let offset = u64::from(sym.name);
        if offset >= self.strsz {
            return Err(WinedroidError::new(format!(
                "strtab out of bounds error; STRSZ={}, name={offset}",
                self.strsz
            )));
        }
        image.cstr(self.strtab + offset, self.strsz - offset)
    }

    pub fn name(&self, image: &MappedImage, sym: &Sym) -> Result<String> {
        Ok(String::from_utf8_lossy(self.name_bytes(image, sym)?).into_owned())
    }

    pub fn lookup(&self, image: &MappedImage, name: &str) -> Result<Option<(u64, Sym)>> {
        match (&self.gnu, &self.sysv) {
            (Some(_), _) => self.lookup_gnu(image, name),
            (None, Some(_)) => self.lookup_sysv(image, name),
            (None, None) => Ok(None),
        }
    }

    pub fn lookup_gnu(&self, image: &MappedImage, name: &str) -> Result<Option<(u64, Sym)>> {
        // cite: bionic linker_soinfo.cpp#gnu_lookup
        let Some(gnu) = &self.gnu else {
            return Ok(None);
        };
        let hash = gnu_hash(name.as_bytes());
        let word_index = u64::from((hash / 64) & gnu.maskwords_m1);
        let word = image.read_u64(gnu.bloom + word_index * 8)?;
        let bit1 = hash % 64;
        let bit2 = (hash >> gnu.shift2) % 64;
        if (word >> bit1) & (word >> bit2) & 1 == 0 {
            return Ok(None);
        }
        let mut index = u64::from(u32_at(
            image,
            gnu.buckets + u64::from(hash % gnu.nbuckets) * 4,
        )?);
        if index == 0 {
            return Ok(None);
        }
        loop {
            if index < u64::from(gnu.symndx) {
                return Ok(None);
            }
            let chain = u32_at(image, gnu.chain + (index - u64::from(gnu.symndx)) * 4)?;
            if (chain ^ hash) >> 1 == 0 {
                let sym = self.symbol(image, index)?;
                if self.name_bytes(image, &sym)? == name.as_bytes() && sym.is_global_and_defined() {
                    return Ok(Some((index, sym)));
                }
            }
            if chain & 1 != 0 {
                return Ok(None);
            }
            index += 1;
        }
    }

    pub fn lookup_sysv(&self, image: &MappedImage, name: &str) -> Result<Option<(u64, Sym)>> {
        // cite: bionic linker_soinfo.cpp#elf_lookup
        let Some(sysv) = &self.sysv else {
            return Ok(None);
        };
        let hash = elf_hash(name.as_bytes());
        let mut index = u64::from(u32_at(
            image,
            sysv.buckets + u64::from(hash % sysv.nbucket) * 4,
        )?);
        let mut steps = 0u32;
        while index != 0 {
            if steps > sysv.nchain || index >= self.count {
                return Err(WinedroidError::new("malformed DT_HASH chain"));
            }
            steps += 1;
            let sym = self.symbol(image, index)?;
            if self.name_bytes(image, &sym)? == name.as_bytes() && sym.is_global_and_defined() {
                return Ok(Some((index, sym)));
            }
            index = u64::from(u32_at(image, sysv.chain + index * 4)?);
        }
        Ok(None)
    }

    pub fn find_by_address(&self, image: &MappedImage, soaddr: u64) -> Result<Option<Sym>> {
        // cite: bionic linker_soinfo.cpp#gnu_addr_lookup, elf_addr_lookup, symbol_matches_soaddr
        let matches = |sym: &Sym| {
            sym.shndx != SHN_UNDEF
                && sym.kind() != STT_TLS
                && sym.value <= soaddr
                && soaddr - sym.value < sym.size
        };
        if let Some(gnu) = &self.gnu {
            for bucket in 0..u64::from(gnu.nbuckets) {
                let mut index = u64::from(u32_at(image, gnu.buckets + bucket * 4)?);
                if index == 0 {
                    continue;
                }
                loop {
                    if index < u64::from(gnu.symndx) {
                        break;
                    }
                    let sym = self.symbol(image, index)?;
                    if matches(&sym) {
                        return Ok(Some(sym));
                    }
                    if u32_at(image, gnu.chain + (index - u64::from(gnu.symndx)) * 4)? & 1 != 0 {
                        break;
                    }
                    index += 1;
                }
            }
            return Ok(None);
        }
        for index in 0..self.count {
            let sym = self.symbol(image, index)?;
            if matches(&sym) {
                return Ok(Some(sym));
            }
        }
        Ok(None)
    }
}

fn read_gnu(name: &str, image: &MappedImage, vaddr: u64) -> Result<Gnu> {
    // cite: bionic linker.cpp#prelink_image
    let nbuckets = u32_at(image, vaddr)?;
    let symndx = u32_at(image, vaddr + 4)?;
    let maskwords = u32_at(image, vaddr + 8)?;
    let shift2 = u32_at(image, vaddr + 12)?;
    if maskwords == 0 || !maskwords.is_power_of_two() {
        return Err(WinedroidError::new(format!(
            "invalid maskwords for gnu_hash = {maskwords:#x}, in \"{name}\" expecting power to two"
        )));
    }
    if nbuckets == 0 {
        return Err(WinedroidError::new(format!(
            "DT_GNU_HASH in \"{name}\" has no buckets"
        )));
    }
    let bloom = vaddr + 16;
    let buckets = bloom + u64::from(maskwords) * 8;
    let chain = buckets + u64::from(nbuckets) * 4;
    image.bytes(vaddr, chain - vaddr)?;
    Ok(Gnu {
        nbuckets,
        symndx,
        maskwords_m1: maskwords - 1,
        shift2,
        bloom,
        buckets,
        chain,
    })
}

fn read_sysv(image: &MappedImage, vaddr: u64) -> Result<Sysv> {
    let nbucket = u32_at(image, vaddr)?;
    let nchain = u32_at(image, vaddr + 4)?;
    if nbucket == 0 {
        return Err(WinedroidError::new("DT_HASH has no buckets"));
    }
    let buckets = vaddr + 8;
    let chain = buckets + u64::from(nbucket) * 4;
    image.bytes(vaddr, chain + u64::from(nchain) * 4 - vaddr)?;
    Ok(Sysv {
        nbucket,
        nchain,
        buckets,
        chain,
    })
}

fn gnu_symbol_count(image: &MappedImage, gnu: &Gnu) -> Result<u64> {
    let mut highest = 0u32;
    for bucket in 0..u64::from(gnu.nbuckets) {
        highest = highest.max(u32_at(image, gnu.buckets + bucket * 4)?);
    }
    if highest < gnu.symndx {
        return Ok(u64::from(gnu.symndx));
    }
    let mut index = u64::from(highest);
    while u32_at(image, gnu.chain + (index - u64::from(gnu.symndx)) * 4)? & 1 == 0 {
        index += 1;
    }
    Ok(index + 1)
}
