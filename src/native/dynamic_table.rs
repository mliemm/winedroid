use crate::error::{Result, WinedroidError};

use super::image_map::MappedImage;

const DT_NULL: i64 = 0;
const DT_NEEDED: i64 = 1;
const DT_PLTRELSZ: i64 = 2;
const DT_HASH: i64 = 4;
const DT_STRTAB: i64 = 5;
const DT_SYMTAB: i64 = 6;
const DT_RELA: i64 = 7;
const DT_RELASZ: i64 = 8;
const DT_RELAENT: i64 = 9;
const DT_STRSZ: i64 = 10;
const DT_SYMENT: i64 = 11;
const DT_INIT: i64 = 12;
const DT_FINI: i64 = 13;
const DT_SONAME: i64 = 14;
const DT_SYMBOLIC: i64 = 16;
const DT_REL: i64 = 17;
const DT_RELSZ: i64 = 18;
const DT_PLTREL: i64 = 20;
const DT_TEXTREL: i64 = 22;
const DT_JMPREL: i64 = 23;
const DT_INIT_ARRAY: i64 = 25;
const DT_FINI_ARRAY: i64 = 26;
const DT_INIT_ARRAYSZ: i64 = 27;
const DT_FINI_ARRAYSZ: i64 = 28;
const DT_RUNPATH: i64 = 29;
const DT_FLAGS: i64 = 30;
const DT_PREINIT_ARRAY: i64 = 32;
const DT_PREINIT_ARRAYSZ: i64 = 33;
const DT_RELRSZ: i64 = 35;
const DT_RELR: i64 = 36;
const DT_RELRENT: i64 = 37;
const DT_ANDROID_RELR: i64 = 0x6fff_e000;
const DT_ANDROID_RELRSZ: i64 = 0x6fff_e001;
const DT_ANDROID_RELRENT: i64 = 0x6fff_e003;
const DT_ANDROID_REL: i64 = 0x6000_000f;
const DT_ANDROID_RELSZ: i64 = 0x6000_0010;
const DT_ANDROID_RELA: i64 = 0x6000_0011;
const DT_ANDROID_RELASZ: i64 = 0x6000_0012;
const DT_GNU_HASH: i64 = 0x6fff_fef5;
const DT_FLAGS_1: i64 = 0x6fff_fffb;
const DF_TEXTREL: u64 = 0x4;
const DF_SYMBOLIC: u64 = 0x2;
pub const DF_1_GLOBAL: u64 = 0x2;
pub const DF_1_NODELETE: u64 = 0x8;
const DYN_SIZE: u64 = 16;

#[derive(Debug, Clone, Default)]
pub struct Table {
    pub vaddr: u64,
    pub size: u64,
}

#[derive(Debug, Clone, Default)]
pub struct DynamicInfo {
    pub needed: Vec<String>,
    pub soname: Option<String>,
    pub runpath: Option<String>,
    pub symtab: u64,
    pub strtab: u64,
    pub strsz: u64,
    pub hash: u64,
    pub gnu_hash: u64,
    pub rela: Table,
    pub jmprel: Table,
    pub android_rela: Table,
    pub relr: Table,
    pub init: u64,
    pub fini: u64,
    pub init_array: Table,
    pub fini_array: Table,
    pub preinit_array: Table,
    pub has_symbolic: bool,
    pub flags_1: u64,
}

fn require(condition: bool, message: impl FnOnce() -> String) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(WinedroidError::new(message()))
    }
}

impl DynamicInfo {
    pub fn read(name: &str, image: &MappedImage, dyn_vaddr: u64, dyn_size: u64) -> Result<Self> {
        // cite: bionic linker.cpp#prelink_image
        let mut info = Self::default();
        let mut needed_offsets = Vec::new();
        let mut soname_offset = None;
        let mut runpath_offset = None;
        let mut pltrel = None;
        let hi = image.layout().hi;
        let pointer = |value: u64, tag: i64| -> Result<u64> {
            require(value < hi, || {
                format!("\"{name}\" has DT tag {tag:#x} pointing outside the image: {value:#x}")
            })?;
            Ok(value)
        };
        let mut terminated = false;
        for index in 0..dyn_size / DYN_SIZE {
            let entry = image.bytes(dyn_vaddr + index * DYN_SIZE, DYN_SIZE)?;
            let tag = i64::from_le_bytes(entry[0..8].try_into().expect("8 bytes"));
            let value = u64::from_le_bytes(entry[8..16].try_into().expect("8 bytes"));
            match tag {
                DT_NULL => {
                    terminated = true;
                    break;
                }
                DT_NEEDED => needed_offsets.push(value),
                DT_PLTRELSZ => info.jmprel.size = value,
                DT_HASH => info.hash = pointer(value, tag)?,
                DT_STRTAB => info.strtab = pointer(value, tag)?,
                DT_SYMTAB => info.symtab = pointer(value, tag)?,
                DT_RELA => info.rela.vaddr = pointer(value, tag)?,
                DT_RELASZ => info.rela.size = value,
                DT_RELAENT => require(value == 24, || format!("invalid DT_RELAENT: {value}"))?,
                DT_STRSZ => info.strsz = value,
                DT_SYMENT => require(value == 24, || format!("invalid DT_SYMENT: {value}"))?,
                DT_INIT => info.init = value,
                DT_FINI => info.fini = value,
                DT_SONAME => soname_offset = Some(value),
                DT_SYMBOLIC => info.has_symbolic = true,
                DT_REL | DT_RELSZ => {
                    return Err(WinedroidError::new(format!(
                        "unsupported DT_REL/DT_RELSZ in \"{name}\""
                    )))
                }
                DT_ANDROID_REL | DT_ANDROID_RELSZ => {
                    return Err(WinedroidError::new(format!(
                        "unsupported DT_ANDROID_REL in \"{name}\""
                    )))
                }
                DT_PLTREL => pltrel = Some(value),
                DT_TEXTREL => {
                    return Err(WinedroidError::new(format!(
                        "\"{name}\" has text relocations"
                    )))
                }
                DT_JMPREL => info.jmprel.vaddr = pointer(value, tag)?,
                DT_INIT_ARRAY => info.init_array.vaddr = pointer(value, tag)?,
                DT_INIT_ARRAYSZ => info.init_array.size = value,
                DT_FINI_ARRAY => info.fini_array.vaddr = pointer(value, tag)?,
                DT_FINI_ARRAYSZ => info.fini_array.size = value,
                DT_PREINIT_ARRAY => info.preinit_array.vaddr = pointer(value, tag)?,
                DT_PREINIT_ARRAYSZ => info.preinit_array.size = value,
                DT_RUNPATH => runpath_offset = Some(value),
                DT_FLAGS => {
                    require(value & DF_TEXTREL == 0, || {
                        format!("\"{name}\" has text relocations")
                    })?;
                    info.has_symbolic |= value & DF_SYMBOLIC != 0;
                }
                DT_RELR | DT_ANDROID_RELR => info.relr.vaddr = pointer(value, tag)?,
                DT_RELRSZ | DT_ANDROID_RELRSZ => info.relr.size = value,
                DT_RELRENT | DT_ANDROID_RELRENT => {
                    require(value == 8, || format!("invalid DT_RELRENT: {value}"))?
                }
                DT_ANDROID_RELA => info.android_rela.vaddr = pointer(value, tag)?,
                DT_ANDROID_RELASZ => info.android_rela.size = value,
                DT_GNU_HASH => info.gnu_hash = pointer(value, tag)?,
                DT_FLAGS_1 => info.flags_1 = value,
                _ => {}
            }
        }
        require(terminated, || {
            format!("unterminated dynamic section in \"{name}\"")
        })?;
        require(pltrel.is_none_or(|value| value == 7), || {
            format!("unsupported DT_PLTREL in \"{name}\"; expected DT_RELA")
        })?;
        require(info.hash != 0 || info.gnu_hash != 0, || {
            format!(
                "empty/missing DT_HASH/DT_GNU_HASH in \"{name}\" (new hash type from the future?)"
            )
        })?;
        require(info.strtab != 0, || {
            format!("empty/missing DT_STRTAB in \"{name}\"")
        })?;
        require(info.symtab != 0, || {
            format!("empty/missing DT_SYMTAB in \"{name}\"")
        })?;
        require(info.strsz != 0 && info.strtab + info.strsz <= hi, || {
            format!(
                "strtab out of bounds error in \"{name}\"; STRSZ={}",
                info.strsz
            )
        })?;
        let string = |offset: u64| -> Result<String> {
            require(offset < info.strsz, || {
                format!(
                    "strtab out of bounds error; STRSZ={}, name={offset}",
                    info.strsz
                )
            })?;
            let bytes = image.cstr(info.strtab + offset, info.strsz - offset)?;
            Ok(String::from_utf8_lossy(bytes).into_owned())
        };
        info.needed = needed_offsets
            .into_iter()
            .map(string)
            .collect::<Result<_>>()?;
        info.soname = soname_offset.map(string).transpose()?;
        info.runpath = runpath_offset.map(string).transpose()?;
        Ok(info)
    }
}
