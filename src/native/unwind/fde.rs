use std::sync::{Arc, Mutex};

use super::super::module::{GuestImage, ModuleId};

const VERSION_ONE: u8 = 1;
const TABLE_ENCODING: u8 = 0x3b;
const POINTER_ENCODINGS: [u8; 3] = [0x1b, 0x03, 0x0b];
const RECORD_LIMIT: usize = 4096;
const BLOCK_LIMIT: usize = 1 << 20;
const RECORD_LIMIT_BYTES: usize = 1 << 16;
const AUGMENTATION_LIMIT: usize = 32;
const DW_EH_PE_ABSPTR: u8 = 0x00;
const DW_EH_PE_ULEB128: u8 = 0x01;
const DW_EH_PE_UDATA2: u8 = 0x02;
const DW_EH_PE_UDATA4: u8 = 0x03;
const DW_EH_PE_UDATA8: u8 = 0x04;
const DW_EH_PE_SLEB128: u8 = 0x09;
const DW_EH_PE_SDATA2: u8 = 0x0a;
const DW_EH_PE_SDATA4: u8 = 0x0b;
const DW_EH_PE_SDATA8: u8 = 0x0c;
const DW_EH_PE_PCREL: u8 = 0x10;
const DW_EH_PE_DATAREL: u8 = 0x30;
const DW_EH_PE_INDIRECT: u8 = 0x80;
const DW_EH_PE_OMIT: u8 = 0xff;

#[derive(Clone)]
pub struct Memory {
    image: Option<Arc<GuestImage>>,
    bias: usize,
    start: usize,
    end: usize,
}

impl Memory {
    pub fn module(image: Arc<GuestImage>) -> Self {
        let bias = image.mapped.load_bias();
        Self {
            image: Some(image),
            bias,
            start: 0,
            end: 0,
        }
    }

    pub fn block(start: usize, end: usize) -> Self {
        Self {
            image: None,
            bias: 0,
            start,
            end,
        }
    }

    pub fn image(&self) -> Option<&Arc<GuestImage>> {
        self.image.as_ref()
    }

    pub fn bytes(&self, address: usize, len: usize) -> Result<&[u8], String> {
        match &self.image {
            Some(image) => {
                let vaddr = address.wrapping_sub(self.bias) as u64;
                image.mapped.bytes(vaddr, len as u64).map_err(|error| {
                    format!("{error}; next missing primitive: guest unwind section")
                })
            }
            None => {
                let end = address.checked_add(len);
                if address < self.start || end.is_none_or(|end| end > self.end) {
                    return Err(format!(
                        "unwind record read {address:#x}+{len:#x} is outside the registered frame block {:#x}..{:#x}; next missing primitive: registered unwind frame bounds",
                        self.start, self.end
                    ));
                }
                Ok(unsafe { std::slice::from_raw_parts(address as *const u8, len) })
            }
        }
    }

    fn u8(&self, address: usize) -> Result<u8, String> {
        Ok(self.bytes(address, 1)?[0])
    }

    fn u16(&self, address: usize) -> Result<u16, String> {
        Ok(u16::from_le_bytes(
            self.bytes(address, 2)?.try_into().expect("2 bytes"),
        ))
    }

    fn u32(&self, address: usize) -> Result<u32, String> {
        Ok(u32::from_le_bytes(
            self.bytes(address, 4)?.try_into().expect("4 bytes"),
        ))
    }

    fn u64(&self, address: usize) -> Result<u64, String> {
        Ok(u64::from_le_bytes(
            self.bytes(address, 8)?.try_into().expect("8 bytes"),
        ))
    }

    fn augmentation(&self, address: usize) -> Result<Vec<u8>, String> {
        let mut text = Vec::new();
        for index in 0..AUGMENTATION_LIMIT {
            let byte = self.u8(address + index)?;
            if byte == 0 {
                return Ok(text);
            }
            text.push(byte);
        }
        Err(format!("the CIE augmentation string at {address:#x} is not terminated; next missing primitive: unwind augmentation string"))
    }

    fn uleb(&self, at: &mut usize) -> Result<u64, String> {
        let mut value = 0u64;
        let mut shift = 0u32;
        loop {
            let byte = self.u8(*at)?;
            *at += 1;
            if shift < 64 {
                value |= u64::from(byte & 0x7f) << shift;
            }
            shift += 7;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
            if shift > 70 {
                return Err(format!("unwind LEB128 value at {:#x} is longer than 10 bytes; next missing primitive: unwind LEB128", *at));
            }
        }
    }

    fn sleb(&self, at: &mut usize) -> Result<i64, String> {
        let mut value = 0i64;
        let mut shift = 0u32;
        loop {
            let byte = self.u8(*at)?;
            *at += 1;
            if shift < 64 {
                value |= i64::from(byte & 0x7f) << shift;
            }
            shift += 7;
            if byte & 0x80 == 0 {
                if shift < 64 && byte & 0x40 != 0 {
                    value |= -1i64 << shift;
                }
                return Ok(value);
            }
            if shift > 70 {
                return Err(format!("unwind LEB128 value at {:#x} is longer than 10 bytes; next missing primitive: unwind LEB128", *at));
            }
        }
    }

    pub fn encoded(&self, at: &mut usize, encoding: u8, datarel: u64) -> Result<u64, String> {
        if encoding == DW_EH_PE_OMIT {
            return Ok(0);
        }
        let application = encoding & 0x70;
        if !matches!(application, 0x00 | DW_EH_PE_PCREL | DW_EH_PE_DATAREL) {
            return Err(format!(
                "unsupported eh_frame pointer application {application:#x} at {:#x}; next missing primitive: eh_frame pointer encoding {encoding:#x}",
                *at
            ));
        }
        let field = *at as u64;
        let raw = match encoding & 0x0f {
            DW_EH_PE_ABSPTR => {
                *at += 8;
                self.u64(field as usize)?
            }
            DW_EH_PE_ULEB128 => self.uleb(at)?,
            DW_EH_PE_SLEB128 => self.sleb(at)? as u64,
            DW_EH_PE_UDATA2 => {
                *at += 2;
                u64::from(self.u16(field as usize)?)
            }
            DW_EH_PE_SDATA2 => {
                *at += 2;
                i64::from(self.u16(field as usize)? as i16) as u64
            }
            DW_EH_PE_UDATA4 => {
                *at += 4;
                u64::from(self.u32(field as usize)?)
            }
            DW_EH_PE_SDATA4 => {
                *at += 4;
                i64::from(self.u32(field as usize)? as i32) as u64
            }
            DW_EH_PE_UDATA8 | DW_EH_PE_SDATA8 => {
                *at += 8;
                self.u64(field as usize)?
            }
            other => {
                return Err(format!(
                    "unsupported eh_frame pointer format {other:#x} at {:#x}; next missing primitive: eh_frame pointer encoding {encoding:#x}",
                    *at
                ))
            }
        };
        let base = match application {
            DW_EH_PE_PCREL => field,
            DW_EH_PE_DATAREL => datarel,
            _ => 0,
        };
        let mut value = raw.wrapping_add(base);
        if encoding & DW_EH_PE_INDIRECT != 0 {
            value = self.u64(value as usize)?;
        }
        Ok(value)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Cie {
    pub code_align: u64,
    pub data_align: i64,
    pub return_register: u32,
    pub pointer_encoding: u8,
    pub lsda_encoding: u8,
    pub personality: u64,
    pub has_z: bool,
    pub instructions: (usize, usize),
}

#[derive(Clone, Debug)]
pub struct Fde {
    pub address: usize,
    pub initial_location: u64,
    pub range: u64,
    pub lsda: u64,
    pub cie: Cie,
    pub instructions: (usize, usize),
}

#[derive(Clone)]
pub struct Found {
    pub memory: Memory,
    pub fde: Fde,
    pub name: String,
}

fn record_length(memory: &Memory, address: usize) -> Result<usize, String> {
    let length = memory.u32(address)?;
    if length == u32::MAX {
        return Err(format!("a DWARF64 length at {address:#x} is not supported; next missing primitive: DWARF64 unwind record"));
    }
    if length == 0 || length as usize > RECORD_LIMIT_BYTES {
        return Err(format!("unwind record at {address:#x} has an invalid length {length}; next missing primitive: unwind record length"));
    }
    Ok(length as usize)
}

pub fn parse_cie(memory: &Memory, address: usize, datarel: u64) -> Result<Cie, String> {
    let length = record_length(memory, address)?;
    let end = address + 4 + length;
    if memory.u32(address + 4)? != 0 {
        return Err(format!("unwind CIE at {address:#x} has a nonzero identifier; next missing primitive: unwind CIE record"));
    }
    let mut at = address + 8;
    let version = memory.u8(at)?;
    at += 1;
    if !matches!(version, 1 | 3) {
        return Err(format!("unwind CIE at {address:#x} has version {version}; next missing primitive: unwind CIE version {version}"));
    }
    let augmentation = memory.augmentation(at)?;
    at += augmentation.len() + 1;
    let code_align = memory.uleb(&mut at)?;
    let data_align = memory.sleb(&mut at)?;
    let return_register = if version == 1 {
        let register = u32::from(memory.u8(at)?);
        at += 1;
        register
    } else {
        memory.uleb(&mut at)? as u32
    };
    let mut pointer_encoding = 0u8;
    let mut lsda_encoding = 0u8;
    let mut personality = 0u64;
    let has_z = augmentation.first() == Some(&b'z');
    let mut instructions = (at, end);
    if has_z {
        let augmentation_size = memory.uleb(&mut at)? as usize;
        let data_start = at;
        let mut letter_at = data_start;
        for letter in &augmentation[1..] {
            match letter {
                b'L' => {
                    lsda_encoding = memory.u8(letter_at)?;
                    letter_at += 1;
                }
                b'R' => {
                    pointer_encoding = memory.u8(letter_at)?;
                    letter_at += 1;
                }
                b'P' => {
                    let encoding = memory.u8(letter_at)?;
                    letter_at += 1;
                    personality = memory.encoded(&mut letter_at, encoding, datarel)?;
                }
                _ => {}
            }
        }
        if data_start + augmentation_size > end {
            return Err(format!("unwind CIE at {address:#x} has augmentation data past the record end; next missing primitive: unwind CIE record"));
        }
        instructions.0 = data_start + augmentation_size;
    } else if !augmentation.is_empty() {
        return Err(format!(
            "unsupported unwind CIE augmentation \"{}\" at {address:#x}; next missing primitive: unwind CIE augmentation",
            String::from_utf8_lossy(&augmentation)
        ));
    }
    if instructions.0 > end {
        return Err(format!("unwind CIE at {address:#x} ends before its instructions; next missing primitive: unwind CIE record"));
    }
    Ok(Cie {
        code_align,
        data_align,
        return_register,
        pointer_encoding,
        lsda_encoding,
        personality,
        has_z,
        instructions,
    })
}

pub fn parse_fde(memory: &Memory, address: usize, datarel: u64) -> Result<Fde, String> {
    let length = record_length(memory, address)?;
    let end = address + 4 + length;
    let cie_pointer = memory.u32(address + 4)?;
    if cie_pointer == 0 {
        return Err(format!("unwind FDE at {address:#x} has no CIE pointer; next missing primitive: unwind FDE record"));
    }
    let cie = parse_cie(memory, address + 4 - cie_pointer as usize, datarel)?;
    let mut at = address + 8;
    let initial_location = memory.encoded(&mut at, cie.pointer_encoding, datarel)?;
    let range = memory.encoded(&mut at, cie.pointer_encoding & 0x0f, datarel)?;
    let mut lsda = 0u64;
    if cie.has_z {
        let augmentation_size = memory.uleb(&mut at)? as usize;
        let data_start = at;
        if cie.lsda_encoding != 0 {
            lsda = memory.encoded(&mut at, cie.lsda_encoding, datarel)?;
        }
        if at != data_start + augmentation_size || at > end {
            return Err(format!("unwind FDE at {address:#x} has malformed augmentation data; next missing primitive: unwind FDE record"));
        }
    }
    if at > end {
        return Err(format!("unwind FDE at {address:#x} header is past the record end; next missing primitive: unwind FDE record"));
    }
    Ok(Fde {
        address,
        initial_location,
        range,
        lsda,
        cie,
        instructions: (at, end),
    })
}

#[derive(Clone, Copy)]
struct Header {
    count: usize,
    table: usize,
    datarel: u64,
}

static HEADERS: Mutex<Vec<(ModuleId, usize, Header)>> = Mutex::new(Vec::new());

pub fn forget(ids: &[ModuleId]) {
    HEADERS
        .lock()
        .expect("unwind header cache")
        .retain(|(id, _, _)| !ids.contains(id));
}

fn read_header(memory: &Memory, base: usize) -> Result<Header, String> {
    let version = memory.u8(base)?;
    let pointer_encoding = memory.u8(base + 1)?;
    let count_encoding = memory.u8(base + 2)?;
    let table_encoding = memory.u8(base + 3)?;
    if version != VERSION_ONE
        || table_encoding != TABLE_ENCODING
        || !POINTER_ENCODINGS.contains(&pointer_encoding)
    {
        return Err(format!(
            "unsupported eh_frame_hdr layout at {base:#x} (version {version}, pointer {pointer_encoding:#x}, table {table_encoding:#x}); next missing primitive: eh_frame_hdr layout"
        ));
    }
    let mut at = base + 4;
    let datarel = base as u64;
    let _frame_pointer = memory.encoded(&mut at, pointer_encoding, datarel)?;
    let count = memory.encoded(&mut at, count_encoding, datarel)? as usize;
    if count > RECORD_LIMIT {
        return Err(format!("eh_frame_hdr at {base:#x} claims {count} FDEs; next missing primitive: eh_frame_hdr table size"));
    }
    memory
        .bytes(at, count * 8)
        .map_err(|error| format!("{error}; next missing primitive: eh_frame_hdr table"))?;
    Ok(Header {
        count,
        table: at,
        datarel,
    })
}

fn header_for(id: ModuleId, memory: &Memory, base: usize) -> Result<Header, String> {
    if let Some((_, _, header)) = HEADERS
        .lock()
        .expect("unwind header cache")
        .iter()
        .find(|(cached, cached_base, _)| *cached == id && *cached_base == base)
    {
        return Ok(*header);
    }
    let header = read_header(memory, base)?;
    let mut cache = HEADERS.lock().expect("unwind header cache");
    if !cache
        .iter()
        .any(|(cached, cached_base, _)| *cached == id && *cached_base == base)
    {
        cache.push((id, base, header));
    }
    Ok(header)
}

fn search_header(memory: &Memory, header: &Header, pc: u64) -> Result<Option<usize>, String> {
    let entry = |index: usize| -> Result<(u64, usize), String> {
        let at = header.table + index * 8;
        let start = memory.u32(at)? as i32 as i64;
        let fde = memory.u32(at + 4)? as i32 as i64;
        Ok((
            header.datarel.wrapping_add(start as u64),
            header.datarel.wrapping_add(fde as u64) as usize,
        ))
    };
    let (mut low, mut high) = (0usize, header.count);
    while low < high {
        let middle = (low + high) / 2;
        let (start, _) = entry(middle)?;
        if start <= pc {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    if low == 0 {
        return Ok(None);
    }
    let (start, fde) = entry(low - 1)?;
    Ok((pc >= start).then_some(fde))
}

static FRAMES: Mutex<Vec<usize>> = Mutex::new(Vec::new());

pub fn register_frame(begin: usize) {
    // cite: bionic libc.map.txt#__register_frame
    let mut frames = FRAMES.lock().expect("registered unwind frames");
    if !frames.contains(&begin) {
        frames.push(begin);
    }
}

pub fn deregister_frame(begin: usize) {
    // cite: bionic libc.map.txt#__deregister_frame
    FRAMES
        .lock()
        .expect("registered unwind frames")
        .retain(|frame| *frame != begin);
}

fn raw_u32(address: usize) -> u32 {
    u32::from_le_bytes(unsafe { std::ptr::read_unaligned(address as *const [u8; 4]) })
}

fn search_registered(pc: u64) -> Result<Option<Found>, String> {
    let blocks = FRAMES.lock().expect("registered unwind frames").clone();
    for begin in blocks {
        let mut at = begin;
        let mut scanned = 0usize;
        let mut records = 0usize;
        loop {
            let length = raw_u32(at) as usize;
            if length == 0 {
                break;
            }
            if length == u32::MAX as usize || length > RECORD_LIMIT_BYTES {
                return Err(format!("registered frame record at {at:#x} has an invalid length {length}; next missing primitive: registered unwind frame record"));
            }
            if raw_u32(at + 4) != 0 {
                let memory = Memory::block(begin, at + 4 + length);
                let fde = parse_fde(&memory, at, 0)?;
                if pc >= fde.initial_location && pc - fde.initial_location < fde.range {
                    return Ok(Some(Found {
                        memory,
                        fde,
                        name: format!("registered frame at {begin:#x}"),
                    }));
                }
            }
            at += 4 + length;
            scanned += 4 + length;
            records += 1;
            if scanned > BLOCK_LIMIT || records > RECORD_LIMIT {
                return Err(format!("registered frame block at {begin:#x} has no zero terminator; next missing primitive: registered unwind frame block"));
            }
        }
    }
    Ok(None)
}

pub fn module_snapshot() -> Vec<(ModuleId, Arc<GuestImage>, String)> {
    match super::super::runtime::NativeRuntime::current() {
        Some(runtime) => runtime.loader().module_images(),
        None => Vec::new(),
    }
}

pub fn lookup(pc: u64) -> Result<Option<Found>, String> {
    if let Some(found) = search_registered(pc)? {
        return Ok(Some(found));
    }
    for (id, image, path) in module_snapshot() {
        let Some(header) = image.elf.eh_frame_hdr() else {
            continue;
        };
        let bias = image.mapped.load_bias();
        if !image.mapped.contains_vaddr(pc.wrapping_sub(bias as u64), 1) {
            continue;
        }
        let base = bias.wrapping_add(header.vaddr as usize);
        let memory = Memory::module(image);
        let header = header_for(id, &memory, base)?;
        let Some(address) = search_header(&memory, &header, pc)? else {
            continue;
        };
        let fde = parse_fde(&memory, address, header.datarel)?;
        if pc >= fde.initial_location && pc - fde.initial_location < fde.range {
            return Ok(Some(Found {
                memory,
                fde,
                name: path,
            }));
        }
    }
    Ok(None)
}

pub fn find_enclosing(pc: u64) -> Option<u64> {
    lookup(pc)
        .ok()
        .flatten()
        .map(|found| found.fde.initial_location)
}
