use std::fmt;

use crate::error::{Result, WinedroidError};

const ELF_MAGIC: &[u8; 4] = b"\x7fELF";

pub(crate) fn has_magic(data: &[u8]) -> bool {
    data.starts_with(ELF_MAGIC)
}

const EI_CLASS: usize = 4;
const EI_DATA: usize = 5;
const ELFCLASS32: u8 = 1;
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;
const ELFDATA2MSB: u8 = 2;

const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;

const DT_NULL: i64 = 0;
const DT_NEEDED: i64 = 1;
const DT_STRTAB: i64 = 5;
const DT_STRSZ: i64 = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElfSummary {
    pub class: ElfClass,
    pub endian: ElfEndian,
    pub machine: ElfMachine,
    pub needed_libraries: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfClass {
    Elf32,
    Elf64,
}

impl fmt::Display for ElfClass {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Elf32 => write!(formatter, "ELF32"),
            Self::Elf64 => write!(formatter, "ELF64"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfEndian {
    Little,
    Big,
}

impl fmt::Display for ElfEndian {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Little => write!(formatter, "little-endian"),
            Self::Big => write!(formatter, "big-endian"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElfMachine {
    Arm,
    Aarch64,
    X86,
    X86_64,
    Other(u16),
}

impl fmt::Display for ElfMachine {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Arm => write!(formatter, "ARM"),
            Self::Aarch64 => write!(formatter, "AArch64"),
            Self::X86 => write!(formatter, "x86"),
            Self::X86_64 => write!(formatter, "x86_64"),
            Self::Other(value) => write!(formatter, "machine-{value}"),
        }
    }
}

pub fn parse_elf(data: &[u8]) -> Result<ElfSummary> {
    let header = ElfHeader::parse(data)?;
    let program_headers = read_program_headers(data, &header)?;
    let needed_libraries = read_needed_libraries(data, &header, &program_headers)?;

    Ok(ElfSummary {
        class: header.class,
        endian: header.endian,
        machine: header.machine,
        needed_libraries,
    })
}

#[derive(Debug, Clone)]
struct ElfHeader {
    class: ElfClass,
    endian: ElfEndian,
    machine: ElfMachine,
    program_header_offset: u64,
    program_header_entry_size: u16,
    program_header_count: u16,
}

impl ElfHeader {
    fn parse(data: &[u8]) -> Result<Self> {
        require_range(data, 0, 16, "ELF ident")?;

        if !has_magic(data) {
            return Err(WinedroidError::new(
                "file does not start with an ELF magic header",
            ));
        }

        let class = parse_class(data[EI_CLASS])?;
        let endian = parse_endian(data[EI_DATA])?;

        match class {
            ElfClass::Elf32 => Self::parse_32(data, endian),
            ElfClass::Elf64 => Self::parse_64(data, endian),
        }
    }

    fn parse_32(data: &[u8], endian: ElfEndian) -> Result<Self> {
        require_range(data, 0, 52, "ELF32 header")?;

        Ok(Self {
            class: ElfClass::Elf32,
            endian,
            machine: parse_machine(read_u16(data, 18, endian)?),
            program_header_offset: u64::from(read_u32(data, 28, endian)?),
            program_header_entry_size: read_u16(data, 42, endian)?,
            program_header_count: read_u16(data, 44, endian)?,
        })
    }

    fn parse_64(data: &[u8], endian: ElfEndian) -> Result<Self> {
        require_range(data, 0, 64, "ELF64 header")?;

        Ok(Self {
            class: ElfClass::Elf64,
            endian,
            machine: parse_machine(read_u16(data, 18, endian)?),
            program_header_offset: read_u64(data, 32, endian)?,
            program_header_entry_size: read_u16(data, 54, endian)?,
            program_header_count: read_u16(data, 56, endian)?,
        })
    }
}

fn parse_class(value: u8) -> Result<ElfClass> {
    match value {
        ELFCLASS32 => Ok(ElfClass::Elf32),
        ELFCLASS64 => Ok(ElfClass::Elf64),
        _ => Err(WinedroidError::new(format!(
            "unsupported ELF class value {value}"
        ))),
    }
}

fn parse_endian(value: u8) -> Result<ElfEndian> {
    match value {
        ELFDATA2LSB => Ok(ElfEndian::Little),
        ELFDATA2MSB => Ok(ElfEndian::Big),
        _ => Err(WinedroidError::new(format!(
            "unsupported ELF endian value {value}"
        ))),
    }
}

fn parse_machine(value: u16) -> ElfMachine {
    match value {
        3 => ElfMachine::X86,
        40 => ElfMachine::Arm,
        62 => ElfMachine::X86_64,
        183 => ElfMachine::Aarch64,
        other => ElfMachine::Other(other),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProgramHeader {
    program_type: u32,
    offset: u64,
    virtual_address: u64,
    file_size: u64,
    memory_size: u64,
}

fn read_program_headers(data: &[u8], header: &ElfHeader) -> Result<Vec<ProgramHeader>> {
    if header.program_header_count == 0 {
        return Ok(Vec::new());
    }

    let table_offset = usize_from_u64(header.program_header_offset, "program header offset")?;
    let entry_size = usize::from(header.program_header_entry_size);
    let table_size = entry_size
        .checked_mul(usize::from(header.program_header_count))
        .ok_or_else(|| WinedroidError::new("program header table size overflow"))?;
    require_range(data, table_offset, table_size, "program header table")?;

    let mut headers = Vec::with_capacity(usize::from(header.program_header_count));

    for index in 0..header.program_header_count {
        let offset = table_offset + usize::from(index) * entry_size;
        headers.push(read_program_header(data, header, offset)?);
    }

    Ok(headers)
}

fn read_program_header(data: &[u8], header: &ElfHeader, offset: usize) -> Result<ProgramHeader> {
    match header.class {
        ElfClass::Elf32 => read_program_header_32(data, header.endian, offset),
        ElfClass::Elf64 => read_program_header_64(data, header.endian, offset),
    }
}

fn read_program_header_32(data: &[u8], endian: ElfEndian, offset: usize) -> Result<ProgramHeader> {
    require_range(data, offset, 32, "ELF32 program header")?;

    Ok(ProgramHeader {
        program_type: read_u32(data, offset, endian)?,
        offset: u64::from(read_u32(data, offset + 4, endian)?),
        virtual_address: u64::from(read_u32(data, offset + 8, endian)?),
        file_size: u64::from(read_u32(data, offset + 16, endian)?),
        memory_size: u64::from(read_u32(data, offset + 20, endian)?),
    })
}

fn read_program_header_64(data: &[u8], endian: ElfEndian, offset: usize) -> Result<ProgramHeader> {
    require_range(data, offset, 56, "ELF64 program header")?;

    Ok(ProgramHeader {
        program_type: read_u32(data, offset, endian)?,
        offset: read_u64(data, offset + 8, endian)?,
        virtual_address: read_u64(data, offset + 16, endian)?,
        file_size: read_u64(data, offset + 32, endian)?,
        memory_size: read_u64(data, offset + 40, endian)?,
    })
}

fn read_needed_libraries(
    data: &[u8],
    header: &ElfHeader,
    program_headers: &[ProgramHeader],
) -> Result<Vec<String>> {
    let Some(dynamic_header) = program_headers
        .iter()
        .find(|program_header| program_header.program_type == PT_DYNAMIC)
    else {
        return Ok(Vec::new());
    };

    let dynamic_entries = read_dynamic_entries(data, header, dynamic_header)?;
    let Some(strtab_address) = dynamic_value(&dynamic_entries, DT_STRTAB) else {
        return Ok(Vec::new());
    };
    let Some(strtab_size) = dynamic_value(&dynamic_entries, DT_STRSZ) else {
        return Ok(Vec::new());
    };

    let strtab_offset = virtual_address_to_file_offset(program_headers, strtab_address)?;
    let strtab_offset = usize_from_u64(strtab_offset, "dynamic string table offset")?;
    let strtab_size = usize_from_u64(strtab_size, "dynamic string table size")?;
    require_range(data, strtab_offset, strtab_size, "dynamic string table")?;

    let mut libraries = Vec::new();

    for entry in dynamic_entries {
        if entry.tag != DT_NEEDED {
            continue;
        }

        let name_offset = strtab_offset
            .checked_add(usize_from_u64(entry.value, "DT_NEEDED string offset")?)
            .ok_or_else(|| WinedroidError::new("DT_NEEDED string offset overflow"))?;

        if name_offset >= strtab_offset + strtab_size {
            return Err(WinedroidError::new(
                "DT_NEEDED string offset points outside the dynamic string table",
            ));
        }

        libraries.push(read_null_terminated_string(data, name_offset)?);
    }

    libraries.sort();
    libraries.dedup();
    Ok(libraries)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DynamicEntry {
    tag: i64,
    value: u64,
}

fn read_dynamic_entries(
    data: &[u8],
    header: &ElfHeader,
    dynamic_header: &ProgramHeader,
) -> Result<Vec<DynamicEntry>> {
    let entry_size = match header.class {
        ElfClass::Elf32 => 8usize,
        ElfClass::Elf64 => 16usize,
    };
    let dynamic_offset = usize_from_u64(dynamic_header.offset, "dynamic segment offset")?;
    let dynamic_size = usize_from_u64(dynamic_header.file_size, "dynamic segment size")?;
    require_range(data, dynamic_offset, dynamic_size, "dynamic segment")?;

    if dynamic_size % entry_size != 0 {
        return Err(WinedroidError::new(
            "dynamic segment size is not aligned to dynamic entry size",
        ));
    }

    let mut entries = Vec::new();
    let entry_count = dynamic_size / entry_size;

    for index in 0..entry_count {
        let offset = dynamic_offset + index * entry_size;
        let entry = read_dynamic_entry(data, header, offset)?;

        if entry.tag == DT_NULL {
            return Ok(entries);
        }

        entries.push(entry);
    }

    Err(WinedroidError::new(
        "dynamic segment does not contain a DT_NULL terminator",
    ))
}

fn read_dynamic_entry(data: &[u8], header: &ElfHeader, offset: usize) -> Result<DynamicEntry> {
    match header.class {
        ElfClass::Elf32 => Ok(DynamicEntry {
            tag: i64::from(read_i32(data, offset, header.endian)?),
            value: u64::from(read_u32(data, offset + 4, header.endian)?),
        }),
        ElfClass::Elf64 => Ok(DynamicEntry {
            tag: read_i64(data, offset, header.endian)?,
            value: read_u64(data, offset + 8, header.endian)?,
        }),
    }
}

fn dynamic_value(entries: &[DynamicEntry], tag: i64) -> Option<u64> {
    entries
        .iter()
        .find(|entry| entry.tag == tag)
        .map(|entry| entry.value)
}

fn virtual_address_to_file_offset(
    program_headers: &[ProgramHeader],
    virtual_address: u64,
) -> Result<u64> {
    for header in program_headers {
        if header.program_type != PT_LOAD {
            continue;
        }

        let Some(memory_end) = header.virtual_address.checked_add(header.memory_size) else {
            return Err(WinedroidError::new(
                "PT_LOAD virtual address range overflow",
            ));
        };

        if virtual_address < header.virtual_address || virtual_address >= memory_end {
            continue;
        }

        let relative_offset = virtual_address - header.virtual_address;

        if relative_offset >= header.file_size {
            return Err(WinedroidError::new(
                "virtual address maps outside PT_LOAD file contents",
            ));
        }

        return header
            .offset
            .checked_add(relative_offset)
            .ok_or_else(|| WinedroidError::new("mapped file offset overflow"));
    }

    Err(WinedroidError::new(format!(
        "virtual address 0x{virtual_address:x} is not covered by a PT_LOAD segment"
    )))
}

fn read_null_terminated_string(data: &[u8], offset: usize) -> Result<String> {
    for end in offset..data.len() {
        if data[end] == 0 {
            let value = std::str::from_utf8(&data[offset..end]).map_err(|error| {
                WinedroidError::with_source("ELF string is not valid UTF-8", error)
            })?;

            return Ok(value.to_owned());
        }
    }

    Err(WinedroidError::new(format!(
        "ELF string at offset {offset} is not null-terminated"
    )))
}

fn read_u16(data: &[u8], offset: usize, endian: ElfEndian) -> Result<u16> {
    require_range(data, offset, 2, "u16")?;
    let bytes = [data[offset], data[offset + 1]];

    Ok(match endian {
        ElfEndian::Little => u16::from_le_bytes(bytes),
        ElfEndian::Big => u16::from_be_bytes(bytes),
    })
}

fn read_u32(data: &[u8], offset: usize, endian: ElfEndian) -> Result<u32> {
    require_range(data, offset, 4, "u32")?;
    let bytes = [
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ];

    Ok(match endian {
        ElfEndian::Little => u32::from_le_bytes(bytes),
        ElfEndian::Big => u32::from_be_bytes(bytes),
    })
}

fn read_i32(data: &[u8], offset: usize, endian: ElfEndian) -> Result<i32> {
    require_range(data, offset, 4, "i32")?;
    let bytes = [
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ];

    Ok(match endian {
        ElfEndian::Little => i32::from_le_bytes(bytes),
        ElfEndian::Big => i32::from_be_bytes(bytes),
    })
}

fn read_u64(data: &[u8], offset: usize, endian: ElfEndian) -> Result<u64> {
    require_range(data, offset, 8, "u64")?;
    let bytes = [
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
        data[offset + 4],
        data[offset + 5],
        data[offset + 6],
        data[offset + 7],
    ];

    Ok(match endian {
        ElfEndian::Little => u64::from_le_bytes(bytes),
        ElfEndian::Big => u64::from_be_bytes(bytes),
    })
}

fn read_i64(data: &[u8], offset: usize, endian: ElfEndian) -> Result<i64> {
    require_range(data, offset, 8, "i64")?;
    let bytes = [
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
        data[offset + 4],
        data[offset + 5],
        data[offset + 6],
        data[offset + 7],
    ];

    Ok(match endian {
        ElfEndian::Little => i64::from_le_bytes(bytes),
        ElfEndian::Big => i64::from_be_bytes(bytes),
    })
}

fn usize_from_u64(value: u64, label: &str) -> Result<usize> {
    usize::try_from(value)
        .map_err(|_| WinedroidError::new(format!("{label} does not fit in usize")))
}

fn require_range(data: &[u8], offset: usize, length: usize, label: &str) -> Result<()> {
    let end = offset
        .checked_add(length)
        .ok_or_else(|| WinedroidError::new(format!("{label} range overflow")))?;

    if end > data.len() {
        return Err(WinedroidError::new(format!(
            "{label} at offset {offset} extends past the end of the file"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_elf64_with_needed_library() {
        let elf = minimal_elf64();
        let summary = parse_elf(&elf).expect("ELF should parse");

        assert_eq!(summary.class, ElfClass::Elf64);
        assert_eq!(summary.endian, ElfEndian::Little);
        assert_eq!(summary.machine, ElfMachine::Aarch64);
        assert_eq!(summary.needed_libraries, vec!["libc.so".to_owned()]);
    }

    #[test]
    fn rejects_invalid_magic() {
        let mut elf = minimal_elf64();
        elf[0] = 0;

        let error = parse_elf(&elf).expect_err("invalid magic should fail");

        assert!(
            error.to_string().contains("ELF magic"),
            "unexpected error: {error}"
        );
    }

    fn minimal_elf64() -> Vec<u8> {
        let mut data = vec![0u8; 0x280];
        let phoff = 0x40usize;
        let dynamic_offset = 0x180usize;
        let strtab_offset = 0x240usize;
        let strtab_address = 0x400240u64;

        data[0..4].copy_from_slice(ELF_MAGIC);
        data[EI_CLASS] = ELFCLASS64;
        data[EI_DATA] = ELFDATA2LSB;
        data[6] = 1;

        write_u16(&mut data, 16, 3);
        write_u16(&mut data, 18, 183);
        write_u32(&mut data, 20, 1);
        write_u64(&mut data, 32, phoff as u64);
        write_u16(&mut data, 52, 64);
        write_u16(&mut data, 54, 56);
        write_u16(&mut data, 56, 2);

        let data_len = data.len() as u64;
        write_program_header_64(&mut data, phoff, PT_LOAD, 0, 0x400000, data_len);
        write_program_header_64(
            &mut data,
            phoff + 56,
            PT_DYNAMIC,
            dynamic_offset as u64,
            0x400180,
            64,
        );

        write_dynamic_64(&mut data, dynamic_offset, DT_STRTAB, strtab_address);
        write_dynamic_64(&mut data, dynamic_offset + 16, DT_STRSZ, 16);
        write_dynamic_64(&mut data, dynamic_offset + 32, DT_NEEDED, 1);
        write_dynamic_64(&mut data, dynamic_offset + 48, DT_NULL, 0);

        data[strtab_offset] = 0;
        data[strtab_offset + 1..strtab_offset + 8].copy_from_slice(b"libc.so");

        data
    }

    fn write_program_header_64(
        data: &mut [u8],
        offset: usize,
        program_type: u32,
        file_offset: u64,
        virtual_address: u64,
        file_size: u64,
    ) {
        write_u32(data, offset, program_type);
        write_u64(data, offset + 8, file_offset);
        write_u64(data, offset + 16, virtual_address);
        write_u64(data, offset + 32, file_size);
        write_u64(data, offset + 40, file_size);
    }

    fn write_dynamic_64(data: &mut [u8], offset: usize, tag: i64, value: u64) {
        data[offset..offset + 8].copy_from_slice(&tag.to_le_bytes());
        write_u64(data, offset + 8, value);
    }

    fn write_u16(data: &mut [u8], offset: usize, value: u16) {
        data[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u32(data: &mut [u8], offset: usize, value: u32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u64(data: &mut [u8], offset: usize, value: u64) {
        data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
}
