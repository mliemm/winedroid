use crate::error::{Result, WinedroidError};

const GROUPED_BY_INFO: u64 = 1;
const GROUPED_BY_OFFSET_DELTA: u64 = 2;
const GROUPED_BY_ADDEND: u64 = 4;
const GROUP_HAS_ADDEND: u64 = 8;
const RELA_SIZE: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reloc {
    pub offset: u64,
    pub info: u64,
    pub addend: i64,
}

impl Reloc {
    pub fn kind(&self) -> u32 {
        self.info as u32
    }

    pub fn symbol(&self) -> u32 {
        (self.info >> 32) as u32
    }
}

pub fn decode_rela(bytes: &[u8]) -> Vec<Reloc> {
    bytes
        .chunks_exact(RELA_SIZE)
        .map(|entry| Reloc {
            offset: u64::from_le_bytes(entry[0..8].try_into().expect("8 bytes")),
            info: u64::from_le_bytes(entry[8..16].try_into().expect("8 bytes")),
            addend: i64::from_le_bytes(entry[16..24].try_into().expect("8 bytes")),
        })
        .collect()
}

struct Sleb128<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl Sleb128<'_> {
    fn next(&mut self) -> Result<u64> {
        // cite: bionic linker_sleb128.h#sleb128_decoder
        let mut value = 0u64;
        let mut shift = 0u32;
        loop {
            let byte = *self
                .bytes
                .get(self.position)
                .ok_or_else(|| WinedroidError::new("android relocation stream ends inside a value"))?;
            self.position += 1;
            if shift < 64 {
                value |= u64::from(byte & 0x7f) << shift;
            }
            shift += 7;
            if byte & 0x80 == 0 {
                if shift < 64 && byte & 0x40 != 0 {
                    value |= u64::MAX << shift;
                }
                return Ok(value);
            }
        }
    }
}

pub fn decode_aps2(bytes: &[u8]) -> Result<Vec<Reloc>> {
    // cite: bionic linker_reloc_iterators.h#for_all_packed_relocs, RELOCATION_GROUPED_BY_INFO_FLAG, RELOCATION_GROUPED_BY_OFFSET_DELTA_FLAG, RELOCATION_GROUPED_BY_ADDEND_FLAG, RELOCATION_GROUP_HAS_ADDEND_FLAG
    if bytes.len() < 4 || bytes[..4] != *b"APS2" {
        return Err(WinedroidError::new("bad android relocation header."));
    }
    let mut decoder = Sleb128 { bytes, position: 4 };
    let total = decoder.next()?;
    let mut reloc = Reloc { offset: decoder.next()?, info: 0, addend: 0 };
    let mut relocs = Vec::new();
    let mut index = 0u64;
    while index < total {
        let group_size = decoder.next()?;
        let group_flags = decoder.next()?;
        if group_size > total - index {
            return Err(WinedroidError::new("android relocation group exceeds the declared count"));
        }
        let offset_delta = if group_flags & GROUPED_BY_OFFSET_DELTA != 0 { decoder.next()? } else { 0 };
        if group_flags & GROUPED_BY_INFO != 0 {
            reloc.info = decoder.next()?;
        }
        let addend_flags = group_flags & (GROUP_HAS_ADDEND | GROUPED_BY_ADDEND);
        if addend_flags == GROUP_HAS_ADDEND | GROUPED_BY_ADDEND {
            reloc.addend = reloc.addend.wrapping_add(decoder.next()? as i64);
        } else if addend_flags != GROUP_HAS_ADDEND {
            reloc.addend = 0;
        }
        for _ in 0..group_size {
            reloc.offset = reloc.offset.wrapping_add(if group_flags & GROUPED_BY_OFFSET_DELTA != 0 { offset_delta } else { decoder.next()? });
            if group_flags & GROUPED_BY_INFO == 0 {
                reloc.info = decoder.next()?;
            }
            if addend_flags == GROUP_HAS_ADDEND {
                reloc.addend = reloc.addend.wrapping_add(decoder.next()? as i64);
            }
            relocs.push(reloc);
        }
        index += group_size;
    }
    if decoder.position != bytes.len() {
        return Err(WinedroidError::new("android relocation stream has trailing bytes"));
    }
    Ok(relocs)
}

pub fn decode_relr(bytes: &[u8]) -> Vec<u64> {
    // cite: bionic linker.cpp#relocate_relr
    let mut offsets = Vec::new();
    let mut base = 0u64;
    for entry in bytes.chunks_exact(8).map(|chunk| u64::from_le_bytes(chunk.try_into().expect("8 bytes"))) {
        if entry & 1 == 0 {
            offsets.push(entry);
            base = entry.wrapping_add(8);
            continue;
        }
        let mut bits = entry;
        let mut offset = base;
        while bits != 0 {
            bits >>= 1;
            if bits & 1 != 0 {
                offsets.push(offset);
            }
            offset = offset.wrapping_add(8);
        }
        base = base.wrapping_add(63 * 8);
    }
    offsets
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sleb(mut value: i64) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let byte = (value & 0x7f) as u8;
            value >>= 7;
            let done = (value == 0 && byte & 0x40 == 0) || (value == -1 && byte & 0x40 != 0);
            out.push(if done { byte } else { byte | 0x80 });
            if done {
                return out;
            }
        }
    }

    #[test]
    fn bionic_sleb128_decoder_round_trips_signed_and_64_bit_values() {
        for value in [0i64, 1, -1, 63, 64, -64, -65, 0x3192d, i64::MAX, i64::MIN, -123456789012] {
            let bytes = sleb(value);
            let mut decoder = Sleb128 { bytes: &bytes, position: 0 };
            assert_eq!(decoder.next().unwrap() as i64, value);
            assert_eq!(decoder.position, bytes.len());
        }
    }

    #[test]
    fn bionic_for_all_packed_relocs_group_flags_expand_offsets_infos_and_addends() {
        let mut stream = b"APS2".to_vec();
        for value in [5i64, 0x100] {
            stream.extend(sleb(value));
        }
        for value in [2i64, 1 | 8, 1027, 0x10, 5, 0x20, 7, 3, 2 | 1 | 4 | 8, 8, 1025, 0x100] {
            stream.extend(sleb(value));
        }
        let relocs = decode_aps2(&stream).unwrap();
        let expected = [(0x110, 1027, 5), (0x130, 1027, 12), (0x138, 1025, 0x10c), (0x140, 1025, 0x10c), (0x148, 1025, 0x10c)];
        let actual: Vec<_> = relocs.iter().map(|reloc| (reloc.offset, reloc.info, reloc.addend)).collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn bionic_android_relocation_header_and_truncation_are_errors() {
        assert!(decode_aps2(b"APS3\x00\x00").is_err());
        assert!(decode_aps2(b"APS2").is_err());
        let mut truncated = b"APS2".to_vec();
        truncated.extend(sleb(1));
        truncated.extend(sleb(0));
        truncated.extend(sleb(1));
        assert!(decode_aps2(&truncated).is_err());
    }

    #[test]
    fn bionic_relocate_relr_bitmap_advances_base_by_63_words() {
        let entries: [u64; 4] = [0x1000, 0b11, 0b11, 0x2000];
        let bytes: Vec<u8> = entries.iter().flat_map(|entry| entry.to_le_bytes()).collect();
        assert_eq!(decode_relr(&bytes), vec![0x1000, 0x1008, 0x1200, 0x2000]);
    }
}
