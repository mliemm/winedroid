use std::collections::{BTreeMap, BTreeSet};

// cite: bionic elf.h#PT_LOAD, PT_DYNAMIC, PT_TLS, PT_GNU_EH_FRAME, PF_X, PF_W
pub const PT_LOAD: u32 = 1;
pub const PT_DYNAMIC: u32 = 2;
pub const PT_TLS: u32 = 7;
pub const PT_GNU_EH_FRAME: u32 = 0x6474_e550;
pub const PF_X: u32 = 1;
pub const PF_W: u32 = 2;

// cite: bionic elf_common.h#R_AARCH64_ABS64, R_AARCH64_GLOB_DAT, R_AARCH64_JUMP_SLOT, R_AARCH64_RELATIVE, R_AARCH64_TLSDESC
pub const R_AARCH64_ABS64: u32 = 257;
pub const R_AARCH64_GLOB_DAT: u32 = 1025;
pub const R_AARCH64_JUMP_SLOT: u32 = 1026;
pub const R_AARCH64_RELATIVE: u32 = 1027;
pub const R_AARCH64_TLSDESC: u32 = 1031;

// cite: bionic elf_common.h#DT_HASH, DT_GNU_HASH, DT_RELR, DT_RELRSZ
pub const DT_HASH: u64 = 4;
pub const DT_GNU_HASH: u64 = 0x6fff_fef5;
pub const DT_RELRSZ: u64 = 35;
pub const DT_RELR: u64 = 36;
// cite: bionic elf.h#DT_ANDROID_RELA, DT_ANDROID_RELASZ
pub const DT_ANDROID_RELA: u64 = 0x6000_0011;
pub const DT_ANDROID_RELASZ: u64 = 0x6000_0012;

const DT_NULL: u64 = 0;
const DT_PLTRELSZ: u64 = 2;
const DT_RELA: u64 = 7;
const DT_RELASZ: u64 = 8;
const DT_INIT_ARRAYSZ: u64 = 27;
const DT_FINI_ARRAYSZ: u64 = 28;
const DT_JMPREL: u64 = 23;
const DT_INIT_ARRAY: u64 = 25;
const DT_FINI_ARRAY: u64 = 26;

const SHT_DYNSYM: u32 = 11;
const SHN_UNDEF: u16 = 0;

pub struct Segment {
    pub kind: u32,
    pub flags: u32,
    pub offset: u64,
    pub vaddr: u64,
    pub filesz: u64,
    pub memsz: u64,
}

pub struct Relocation {
    pub kind: u32,
    pub vaddr: u64,
    pub file_offset: usize,
    pub symbol: Option<String>,
}

pub struct FixtureElf {
    pub segments: Vec<Segment>,
    pub dynamic: Vec<(u64, u64)>,
    pub relocations: Vec<Relocation>,
    pub init_array_len: usize,
    pub fini_array_len: usize,
    pub exports: BTreeSet<String>,
    pub export_addresses: BTreeMap<String, u64>,
    pub imports: BTreeSet<String>,
}

fn u16_at(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap())
}

fn u32_at(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

fn u64_at(data: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap())
}

fn c_string(data: &[u8], offset: usize) -> String {
    let end = data[offset..].iter().position(|&byte| byte == 0).unwrap();
    String::from_utf8(data[offset..offset + end].to_vec()).unwrap()
}

fn read_segments(data: &[u8]) -> Vec<Segment> {
    let table = u64_at(data, 32) as usize;
    let entry_size = usize::from(u16_at(data, 54));
    let count = usize::from(u16_at(data, 56));
    (0..count)
        .map(|index| table + index * entry_size)
        .map(|header| Segment {
            kind: u32_at(data, header),
            flags: u32_at(data, header + 4),
            offset: u64_at(data, header + 8),
            vaddr: u64_at(data, header + 16),
            filesz: u64_at(data, header + 32),
            memsz: u64_at(data, header + 40),
        })
        .collect()
}

fn vaddr_to_offset(segments: &[Segment], vaddr: u64) -> usize {
    let load = segments
        .iter()
        .find(|segment| segment.kind == PT_LOAD && vaddr >= segment.vaddr && vaddr < segment.vaddr + segment.filesz)
        .unwrap_or_else(|| panic!("vaddr {vaddr:#x} is not file backed"));
    (vaddr - load.vaddr + load.offset) as usize
}

struct Symbol {
    name: String,
    defined: bool,
    value: u64,
}

fn read_dynamic_symbols(data: &[u8]) -> Vec<Symbol> {
    let section_table = u64_at(data, 40) as usize;
    let entry_size = usize::from(u16_at(data, 58));
    let count = usize::from(u16_at(data, 60));
    let sections: Vec<usize> = (0..count).map(|index| section_table + index * entry_size).collect();
    let dynsym = sections
        .iter()
        .copied()
        .find(|&header| u32_at(data, header + 4) == SHT_DYNSYM)
        .expect("fixture has no .dynsym section");
    let symbol_offset = u64_at(data, dynsym + 24) as usize;
    let symbol_size = u64_at(data, dynsym + 32) as usize;
    let string_header = sections[u32_at(data, dynsym + 40) as usize];
    let string_offset = u64_at(data, string_header + 24) as usize;
    (0..symbol_size / 24)
        .map(|index| symbol_offset + index * 24)
        .map(|entry| Symbol {
            name: c_string(data, string_offset + u32_at(data, entry) as usize),
            defined: u16_at(data, entry + 6) != SHN_UNDEF,
            value: u64_at(data, entry + 8),
        })
        .collect()
}

fn read_relocations(data: &[u8], segments: &[Segment], symbols: &[Symbol], dynamic: &[(u64, u64)]) -> Vec<Relocation> {
    let value = |tag: u64| dynamic.iter().find(|entry| entry.0 == tag).map(|entry| entry.1);
    let ranges = [(DT_RELA, DT_RELASZ), (DT_JMPREL, DT_PLTRELSZ)];
    let mut relocations = Vec::new();
    for (start_tag, size_tag) in ranges {
        let (Some(start), Some(size)) = (value(start_tag), value(size_tag)) else {
            continue;
        };
        let base = vaddr_to_offset(segments, start);
        for index in 0..(size as usize) / 24 {
            let entry = base + index * 24;
            let info = u64_at(data, entry + 8);
            let symbol_index = (info >> 32) as usize;
            relocations.push(Relocation {
                kind: info as u32,
                vaddr: u64_at(data, entry),
                file_offset: entry,
                symbol: (symbol_index != 0).then(|| symbols[symbol_index].name.clone()),
            });
        }
    }
    relocations
}

impl FixtureElf {
    pub fn parse(data: &[u8]) -> Self {
        let segments = read_segments(data);
        let symbols = read_dynamic_symbols(data);

        let dynamic_segment = segments
            .iter()
            .find(|segment| segment.kind == PT_DYNAMIC)
            .expect("fixture has no PT_DYNAMIC");
        let base = dynamic_segment.offset as usize;
        let mut dynamic = Vec::new();
        for index in 0..(dynamic_segment.filesz as usize) / 16 {
            let tag = u64_at(data, base + index * 16);
            if tag == DT_NULL {
                break;
            }
            dynamic.push((tag, u64_at(data, base + index * 16 + 8)));
        }
        let value = |tag: u64| dynamic.iter().find(|entry| entry.0 == tag).map(|entry| entry.1);
        let array_len = |array_tag: u64, size_tag: u64| match (value(array_tag), value(size_tag)) {
            (Some(_), Some(size)) => (size / 8) as usize,
            _ => 0,
        };

        let relocations = read_relocations(data, &segments, &symbols, &dynamic);
        let named = |defined: bool| {
            symbols
                .iter()
                .filter(|symbol| !symbol.name.is_empty() && symbol.defined == defined)
                .map(|symbol| symbol.name.clone())
                .collect()
        };
        let init_array_len = array_len(DT_INIT_ARRAY, DT_INIT_ARRAYSZ);
        let fini_array_len = array_len(DT_FINI_ARRAY, DT_FINI_ARRAYSZ);
        Self {
            relocations,
            dynamic,
            init_array_len,
            fini_array_len,
            exports: named(true),
            export_addresses: symbols
                .iter()
                .filter(|symbol| !symbol.name.is_empty() && symbol.defined)
                .map(|symbol| (symbol.name.clone(), symbol.value))
                .collect(),
            imports: named(false),
            segments,
        }
    }

    pub fn file_offset(&self, vaddr: u64) -> usize {
        vaddr_to_offset(&self.segments, vaddr)
    }

    pub fn tag(&self, tag: u64) -> Option<u64> {
        self.dynamic.iter().find(|entry| entry.0 == tag).map(|entry| entry.1)
    }

    pub fn has_segment(&self, kind: u32) -> bool {
        self.segments.iter().any(|segment| segment.kind == kind)
    }

    pub fn count(&self, kind: u32) -> usize {
        self.relocations.iter().filter(|relocation| relocation.kind == kind).count()
    }

    pub fn symbols_of(&self, kind: u32) -> BTreeSet<String> {
        self.relocations
            .iter()
            .filter(|relocation| relocation.kind == kind)
            .filter_map(|relocation| relocation.symbol.clone())
            .collect()
    }
}
