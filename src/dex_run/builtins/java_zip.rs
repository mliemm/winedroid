use std::io::{Read, Seek, SeekFrom};
use std::os::unix::fs::FileExt;

use super::super::*;
use crate::external_storage::ExternalStorageAccess;
use crate::guest_filesystem::ResolvedGuestPath;

const ZIP_FILE: &str = "Ljava/util/zip/ZipFile;";
const ZIP_ENTRY: &str = "Ljava/util/zip/ZipEntry;";
const ZIP_EXCEPTION: &str = "Ljava/util/zip/ZipException;";
const ENTRY_ITERATOR: &str = "Ljava/util/zip/ZipFile$ZipEntryIterator;";
const STORED_STREAM: &str = "Ljava/util/zip/ZipFile$ZipFileInputStream;";
const DEFLATED_STREAM: &str = "Ljava/util/zip/ZipFile$ZipFileInflaterInputStream;";
const CRC32: &str = "Ljava/util/zip/CRC32;";
const CHECKSUM: &str = "Ljava/util/zip/Checksum;";
const CHECKED_STREAM: &str = "Ljava/util/zip/CheckedInputStream;";
const INPUT_STREAM: &str = "Ljava/io/InputStream;";
const FILTER_STREAM: &str = "Ljava/io/FilterInputStream;";
const FILE_STREAM: &str = "Ljava/io/FileInputStream;";
const IO_EXCEPTION: &str = "Ljava/io/IOException;";
const OPEN_READ: i32 = 0x1;
const OPEN_DELETE: i32 = 0x4;

pub(in crate::dex_run) struct ZipFileState {
    file: std::fs::File,
    records: Vec<ZipRecord>,
    index: HashMap<Vec<u8>, usize>,
    streams: Vec<DexValue>,
    remove_on_close: Option<String>,
}

#[derive(Clone)]
struct ZipRecord {
    name: Vec<u8>,
    flag: u16,
    time: u32,
    crc: u32,
    size: u64,
    deflated_size: u64,
    local_header: u64,
    extra: Vec<u8>,
    comment: Vec<u8>,
}

#[derive(Clone)]
enum StreamKind {
    Checked {
        source: DexValue,
        checksum: DexValue,
    },
    File(String),
    StoredEntry,
    DeflatedEntry,
    Buffered,
}

impl ZipFileState {
    // cite: libcore zip_util.c#ZIP_GetEntry2
    fn find(&self, name: &str) -> Option<&ZipRecord> {
        let mut key = name.as_bytes().to_vec();
        if !self.index.contains_key(&key) && !name.ends_with('/') {
            key.push(b'/');
        }
        self.index.get(&key).map(|&index| &self.records[index])
    }

    // cite: libcore zip_util.c#ZIP_GetEntryDataOffset, ZIP_Read
    fn load(&self, record: &ZipRecord) -> (Vec<u8>, Option<(&'static str, &'static str)>) {
        let failed = |message| (Vec::new(), Some((ZIP_EXCEPTION, message)));
        let mut header = [0u8; 30];
        if self
            .file
            .read_exact_at(&mut header, record.local_header)
            .is_err()
        {
            return failed("error reading zip file");
        }
        if !header.starts_with(b"PK\x03\x04") {
            return failed("invalid LOC header (bad signature)");
        }
        let start =
            record.local_header + 30 + u64::from(le16(&header, 26)) + u64::from(le16(&header, 28));
        let length = if record.deflated_size == 0 {
            record.size
        } else {
            record.deflated_size
        };
        let file_length = self.file.metadata().map_or(0, |metadata| metadata.len());
        if start
            .checked_add(length)
            .is_none_or(|end| end > file_length)
        {
            return failed("ZIP_Read: corrupt zip file: invalid entry size");
        }
        let mut bytes = vec![0u8; length as usize];
        if self.file.read_exact_at(&mut bytes, start).is_err() {
            return failed("ZIP_Read: error reading zip file");
        }
        if record.deflated_size == 0 {
            return (bytes, None);
        }
        inflate_entry(&bytes)
    }
}

// cite: libcore InflaterInputStream.java#read
// cite: libcore ZipFile.java#fill
fn inflate_entry(compressed: &[u8]) -> (Vec<u8>, Option<(&'static str, &'static str)>) {
    let mut inflater = flate2::Decompress::new(false);
    let mut output = Vec::new();
    loop {
        let (consumed, produced) = (inflater.total_in(), inflater.total_out());
        output.reserve(32 * 1024);
        let status = inflater.decompress_vec(
            &compressed[consumed as usize..],
            &mut output,
            flate2::FlushDecompress::None,
        );
        let failure = match status {
            Ok(flate2::Status::StreamEnd) => return (output, None),
            Ok(_) if inflater.total_in() == consumed && inflater.total_out() == produced => (
                "Ljava/io/EOFException;",
                "Unexpected end of ZLIB input stream",
            ),
            Ok(_) => continue,
            Err(_) => (ZIP_EXCEPTION, "Invalid ZLIB data format"),
        };
        return (output, Some(failure));
    }
}

fn le16(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn le32(bytes: &[u8], at: usize) -> u32 {
    u32::from(le16(bytes, at)) | u32::from(le16(bytes, at + 2)) << 16
}

fn le64(bytes: &[u8], at: usize) -> u64 {
    u64::from(le32(bytes, at)) | u64::from(le32(bytes, at + 4)) << 32
}

// cite: libcore ZipFile.c#ZipFile_open
// cite: libcore zip_util.c#readCEN, newEntry, isValidEntryName, ZIP_Put_In_Cache0
// cite: frameworks-base SafeZipPathValidatorCallback.java#onZipEntryAccess
fn read_zip_directory(
    file: std::fs::File,
    validate_paths: bool,
) -> std::result::Result<ZipFileState, &'static str> {
    const UNREADABLE: &str = "error in opening zip file";
    const ZIP64: u64 = 0xffff_ffff;
    let length = file.metadata().map_err(|_| UNREADABLE)?.len();
    if length == 0 {
        return Err("zip file is empty");
    }
    let (start, base) = {
        let archive = zip::ZipArchive::new(&file).map_err(|_| UNREADABLE)?;
        (archive.central_directory_start(), archive.offset())
    };
    let mut directory =
        vec![0u8; usize::try_from(length.saturating_sub(start)).map_err(|_| UNREADABLE)?];
    file.read_exact_at(&mut directory, start)
        .map_err(|_| UNREADABLE)?;

    let mut records = Vec::new();
    let mut index = HashMap::new();
    let mut at = 0;
    while directory[at..].starts_with(b"PK\x01\x02") {
        let bad_size = "invalid CEN header (bad header size)";
        let header = directory.get(at..at + 46).ok_or(bad_size)?;
        let flag = le16(header, 8);
        if flag & 1 != 0 {
            return Err("invalid CEN header (encrypted entry)");
        }
        let method = le16(header, 10);
        if method != 0 && method != 8 {
            return Err("invalid CEN header (bad compression method)");
        }
        let name_end = at + 46 + usize::from(le16(header, 28));
        let extra_end = name_end + usize::from(le16(header, 30));
        let end = extra_end + usize::from(le16(header, 32));
        let name = directory.get(at + 46..name_end).ok_or(bad_size)?.to_vec();
        if name.contains(&0) {
            return Err("invalid CEN header (invalid entry name)");
        }
        if validate_paths
            && !std::str::from_utf8(&name).is_ok_and(|name| {
                !name.starts_with('/') && !name.split('/').any(|part| part == "..")
            })
        {
            return Err("restricted zip entry name");
        }
        let extra = directory.get(name_end..extra_end).ok_or(bad_size)?.to_vec();
        let comment = directory.get(extra_end..end).ok_or(bad_size)?.to_vec();

        let mut deflated_size = if method == 0 {
            0
        } else {
            u64::from(le32(header, 20))
        };
        let mut size = u64::from(le32(header, 24));
        let mut offset = u64::from(le32(header, 42));
        if deflated_size == ZIP64 || size == ZIP64 || offset == ZIP64 {
            let mut field = 0;
            while field + 4 < extra.len() {
                let mut remaining = usize::from(le16(&extra, field + 2));
                if le16(&extra, field) != 1 {
                    field += remaining + 4;
                    continue;
                }
                field += 4;
                for value in [&mut size, &mut deflated_size, &mut offset] {
                    if *value != ZIP64 {
                        continue;
                    }
                    if remaining < 8 || field + 8 > extra.len() {
                        break;
                    }
                    *value = le64(&extra, field);
                    remaining -= 8;
                    field += 8;
                }
                break;
            }
        }

        if index.insert(name.clone(), records.len()).is_some() {
            return Err("invalid CEN header (duplicate entry)");
        }
        records.push(ZipRecord {
            name,
            flag,
            time: le32(header, 12),
            crc: le32(header, 16),
            size,
            deflated_size,
            local_header: base.checked_add(offset).ok_or(UNREADABLE)?,
            extra,
            comment,
        });
        at = end;
    }
    if !directory[at..].starts_with(b"PK\x05\x06") && !directory[at..].starts_with(b"PK\x06\x06") {
        return Err("invalid CEN header (bad signature)");
    }

    Ok(ZipFileState {
        file,
        records,
        index,
        streams: Vec::new(),
        remove_on_close: None,
    })
}

// cite: libcore ZipEntry.java#setExtra0
// cite: libcore ZipUtils.java#winTimeToFileTime, unixTimeToFileTime
fn extra_modification_millis(extra: &[u8]) -> Option<i64> {
    let mut modified = None;
    let mut at = 0;
    while at + 4 < extra.len() {
        let tag = le16(extra, at);
        let size = usize::from(le16(extra, at + 2));
        at += 4;
        if at + size > extra.len() {
            break;
        }
        if tag == 0x000a && size >= 32 && le16(extra, at + 4) == 1 && le16(extra, at + 6) == 24 {
            let windows_ticks = le64(extra, at + 8) as i64;
            modified = Some((windows_ticks / 10 - 11_644_473_600_000_000) / 1000);
        }
        if tag == 0x5455 && extra[at] & 1 != 0 && size >= 5 {
            modified = Some(i64::from(le32(extra, at + 1)) * 1000);
        }
        at += size;
    }

    modified
}

// cite: libcore ZipUtils.java#dosToJavaTime, overflowDosToJavaTime
fn dos_local_seconds(time: i64) -> i64 {
    let month = ((time >> 21) & 0x0f) - 1;
    let year = ((time >> 25) & 0x7f) + 1980 + month.div_euclid(12);
    let month = month.rem_euclid(12) as u32 + 1;
    let days = super::java_date::days_from_civil(year, month, 1) + ((time >> 16) & 0x1f) - 1;

    days * 86_400 + ((time >> 11) & 0x1f) * 3600 + ((time >> 5) & 0x3f) * 60 + ((time << 1) & 0x3e)
}

fn zip_field(class_descriptor: &str, type_descriptor: &str, name: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: class_descriptor.to_owned(),
        type_descriptor: type_descriptor.to_owned(),
        name: name.to_owned(),
    }
}

fn entry_field(type_descriptor: &str, name: &str) -> DexFieldId {
    zip_field(ZIP_ENTRY, type_descriptor, name)
}

fn is_null(value: &DexValue) -> bool {
    matches!(value, DexValue::Null | DexValue::Int(0))
}

fn string_result(value: &str) -> DexExecutionResult {
    DexExecutionResult::Value(DexValue::String(std::rc::Rc::from(value)))
}

fn long_result(value: i64) -> DexExecutionResult {
    DexExecutionResult::Value(DexValue::Long(value))
}

fn out_of_bounds(length: i32, offset: i32, count: i32) -> bool {
    offset < 0 || count < 0 || offset > length - count
}

fn zip_proto(return_type: &str, parameters: &[&str]) -> Option<crate::dex::DexProto> {
    let parameters: Vec<String> = parameters.iter().map(|&value| value.to_owned()).collect();

    Some(crate::dex::DexProto {
        shorty: proto_shorty(return_type, &parameters)?,
        return_type: return_type.to_owned(),
        parameters,
    })
}

impl DexProgram {
    pub(in crate::dex_run) fn invoke_java_zip_method(
        &self,
        method: &DexMethodId,
        invocation: &DexInvocation,
        depth: usize,
    ) -> Result<Option<DexExecutionResult>> {
        let class = method.class_descriptor.as_str();
        if !matches!(
            class,
            ZIP_FILE
                | ZIP_ENTRY
                | CRC32
                | CHECKSUM
                | CHECKED_STREAM
                | INPUT_STREAM
                | FILTER_STREAM
                | FILE_STREAM
                | "Ljava/io/Closeable;"
                | "Ljava/lang/AutoCloseable;"
                | "Ljava/util/Enumeration;"
        ) {
            return Ok(None);
        }
        let arguments = invocation.arguments.as_slice();
        let Some(receiver) = arguments.first() else {
            return Ok(None);
        };

        // cite: art interpreter_common.h#DoInvoke, FindMethodToCall
        if matches!(
            invocation.kind,
            DexInvokeKind::Virtual | DexInvokeKind::Interface
        ) && method.name != "<init>"
        {
            if let Some(resolved) = self.zip_guest_override(receiver, method)? {
                let owned = [INPUT_STREAM, ZIP_FILE, ZIP_ENTRY, CRC32]
                    .iter()
                    .any(|owner| self.value_matches_type(receiver, owner));
                return if owned {
                    self.execute_method(None, &resolved, Some(arguments), depth)
                        .map(Some)
                } else {
                    Ok(None)
                };
            }
        }

        let descriptor = method.proto.descriptor();
        Ok(match (class, method.name.as_str(), descriptor.as_str()) {
            (ZIP_FILE, _, _) => self.invoke_zip_file(method.name.as_str(), &descriptor, arguments),
            (ZIP_ENTRY, name, _) => self.zip_entry_accessor(receiver, name, &descriptor),
            ("Ljava/util/Enumeration;", name, _) => self.zip_enumeration(receiver, name),
            (CRC32, "<init>", "()V") => Some(DexExecutionResult::Void),
            (CRC32 | CHECKSUM, name, _) => self.crc32_method(name, &descriptor, arguments),
            (CHECKED_STREAM, "<init>", "(Ljava/io/InputStream;Ljava/util/zip/Checksum;)V") => {
                // cite: libcore CheckedInputStream.java#CheckedInputStream
                // cite: libcore FilterInputStream.java#FilterInputStream
                let [_, source, checksum] = arguments else {
                    return Ok(None);
                };
                let stored = self.write_stored_instance_field(
                    receiver,
                    &zip_field(FILTER_STREAM, INPUT_STREAM, "in"),
                    source.clone(),
                ) && self.write_stored_instance_field(
                    receiver,
                    &zip_field(CHECKED_STREAM, CHECKSUM, "cksum"),
                    checksum.clone(),
                );
                stored.then_some(DexExecutionResult::Void)
            }
            // cite: libcore CheckedInputStream.java#getChecksum
            (CHECKED_STREAM, "getChecksum", "()Ljava/util/zip/Checksum;") => self
                .read_stored_instance_field(receiver, &zip_field(CHECKED_STREAM, CHECKSUM, "cksum"))
                .map(DexExecutionResult::Value),
            (INPUT_STREAM | FILTER_STREAM | FILE_STREAM | CHECKED_STREAM, "read", "()I") => {
                self.builtin_stream_read_byte(receiver, depth)?
            }
            (INPUT_STREAM | FILTER_STREAM | FILE_STREAM | CHECKED_STREAM, "read", "([B)I") => {
                // cite: libcore InputStream.java#read
                // cite: libcore FileInputStream.java#read
                // cite: libcore FilterInputStream.java#read
                let [_, array] = arguments else {
                    return Ok(None);
                };
                if is_null(array) {
                    Some(self.null_pointer_exception_result())
                } else {
                    let Some(length) = self.array_length(array) else {
                        return Ok(None);
                    };
                    self.stream_read_into(receiver, array, 0, length, depth)?
                }
            }
            (INPUT_STREAM | FILTER_STREAM | FILE_STREAM | CHECKED_STREAM, "read", "([BII)I") => {
                let [_, array, DexValue::Int(offset), DexValue::Int(count)] = arguments else {
                    return Ok(None);
                };
                self.builtin_stream_read_into(receiver, array, *offset, *count, depth)?
            }
            (INPUT_STREAM | FILTER_STREAM | FILE_STREAM | CHECKED_STREAM, "available", "()I") => {
                self.builtin_stream_available(receiver, depth)?
            }
            (INPUT_STREAM | FILTER_STREAM | FILE_STREAM | CHECKED_STREAM, "close", "()V") => {
                self.builtin_stream_close(receiver, depth)?
            }
            ("Ljava/io/Closeable;" | "Ljava/lang/AutoCloseable;", "close", "()V") => {
                if self.value_matches_type(receiver, ZIP_FILE) {
                    self.zip_file_close(receiver)
                } else if self.value_matches_type(receiver, INPUT_STREAM) {
                    self.builtin_stream_close(receiver, depth)?
                } else {
                    None
                }
            }
            _ => None,
        })
    }

    fn zip_guest_override(
        &self,
        receiver: &DexValue,
        method: &DexMethodId,
    ) -> Result<Option<std::rc::Rc<ResolvedMethod>>> {
        let Some(class_descriptor) = value_class_descriptor(receiver) else {
            return Ok(None);
        };
        let method = DexMethodId {
            class_descriptor: class_descriptor.to_owned(),
            ..method.clone()
        };

        self.resolve_exact_method_in_class_hierarchy(class_descriptor, &method)
    }

    fn zip_guest_call(
        &self,
        receiver: &DexValue,
        name: &str,
        return_type: &str,
        parameters: &[&str],
        arguments: &[DexValue],
        depth: usize,
    ) -> Result<Option<DexExecutionResult>> {
        let (Some(class_descriptor), Some(proto)) = (
            value_class_descriptor(receiver),
            zip_proto(return_type, parameters),
        ) else {
            return Ok(None);
        };
        let method = DexMethodId {
            class_descriptor: class_descriptor.to_owned(),
            name: name.to_owned(),
            proto,
        };
        let Some(resolved) = self.zip_guest_override(receiver, &method)? else {
            return Ok(None);
        };
        let mut initial_arguments = vec![receiver.clone()];
        initial_arguments.extend_from_slice(arguments);

        Ok(Some(
            match self.execute_method(None, &resolved, Some(&initial_arguments), depth + 1)? {
                DexExecutionResult::Parked(execution) => {
                    self.discard_parked_execution(&execution);
                    return Err(WinedroidError::new(format!(
                        "guest stream callback parked\nnext missing primitive: method {class_descriptor}.{name}"
                    )));
                }
                DexExecutionResult::Value(DexValue::Int(value)) => DexExecutionResult::Int(value),
                result => result,
            },
        ))
    }

    fn zip_throw(&self, class_descriptor: &str, message: Option<&str>) -> DexExecutionResult {
        DexExecutionResult::Throw(self.allocate_generated_exception(DexGeneratedException {
            class_descriptor: class_descriptor.to_owned(),
            detail_message: message.map(str::to_owned),
        }))
    }

    fn zip_flag(&self, object: &DexValue, field: &DexFieldId) -> bool {
        self.read_stored_instance_field(object, field) == Some(DexValue::Int(1))
    }

    fn zip_string_field(&self, object: &DexValue, field: &DexFieldId) -> Option<String> {
        match self.read_stored_instance_field(object, field)? {
            DexValue::String(value) => Some(value.to_string()),
            _ => None,
        }
    }

    fn zip_long_field(&self, object: &DexValue, field: &DexFieldId) -> Option<i64> {
        match self.read_stored_instance_field(object, field)? {
            DexValue::Long(value) => Some(value),
            _ => None,
        }
    }

    fn guest_bytes(&self, array: &DexValue, offset: i32, count: i32) -> Option<Vec<u8>> {
        let start = usize::try_from(offset).ok()?;
        let end = start.checked_add(usize::try_from(count).ok()?)?;
        self.runtime
            .borrow()
            .array_entries
            .get(&map_identity_key(array))?
            .get(start..end)?
            .iter()
            .map(|value| match value {
                DexValue::Int(byte) => Some(*byte as u8),
                _ => None,
            })
            .collect()
    }

    fn new_guest_bytes(&self, bytes: &[u8]) -> DexValue {
        self.allocate_filled_array(
            "[B".to_owned(),
            bytes
                .iter()
                .map(|&byte| DexValue::Int(i32::from(byte as i8)))
                .collect(),
        )
    }

    fn write_guest_bytes(&self, array: &DexValue, offset: i32, bytes: &[u8]) -> Option<()> {
        let start = usize::try_from(offset).ok()?;
        let mut runtime = self.runtime.borrow_mut();
        let slots = runtime
            .array_entries
            .get_mut(&map_identity_key(array))?
            .get_mut(start..start.checked_add(bytes.len())?)?;
        for (slot, &byte) in slots.iter_mut().zip(bytes) {
            *slot = DexValue::Int(i32::from(byte as i8));
        }

        Some(())
    }

    fn invoke_zip_file(
        &self,
        name: &str,
        descriptor: &str,
        arguments: &[DexValue],
    ) -> Option<DexExecutionResult> {
        match (name, descriptor, arguments) {
            // cite: libcore ZipFile.java#ZipFile
            // cite: libcore File.java#File
            ("<init>", "(Ljava/lang/String;)V", [zip, path]) => match path {
                DexValue::String(path) => self.zip_file_open(
                    zip,
                    &crate::guest_filesystem::normalize_guest_file_path(path),
                    OPEN_READ,
                ),
                path => is_null(path).then(|| self.null_pointer_exception_result()),
            },
            ("<init>", "(Ljava/io/File;)V", [zip, file]) => {
                self.zip_file_open_file(zip, file, OPEN_READ)
            }
            ("<init>", "(Ljava/io/File;I)V", [zip, file, DexValue::Int(mode)]) => {
                self.zip_file_open_file(zip, file, *mode)
            }
            ("getEntry", "(Ljava/lang/String;)Ljava/util/zip/ZipEntry;", [zip, name]) => {
                self.zip_file_get_entry(zip, name)
            }
            ("getInputStream", "(Ljava/util/zip/ZipEntry;)Ljava/io/InputStream;", [zip, entry]) => {
                self.zip_file_input_stream(zip, entry)
            }
            // cite: libcore ZipFile.java#entries, ZipEntryIterator
            ("entries", "()Ljava/util/Enumeration;", [zip]) => {
                if let Err(thrown) = self.zip_file_ensure_open(zip) {
                    return Some(thrown);
                }
                let iterator = self.allocate_object(ENTRY_ITERATOR.to_owned());
                self.write_stored_instance_field(
                    &iterator,
                    &zip_field(ENTRY_ITERATOR, ZIP_FILE, "this$0"),
                    zip.clone(),
                );
                self.write_stored_instance_field(
                    &iterator,
                    &zip_field(ENTRY_ITERATOR, "I", "i"),
                    DexValue::Int(0),
                );
                Some(DexExecutionResult::Value(iterator))
            }
            // cite: libcore ZipFile.java#size
            ("size", "()I", [zip]) => {
                if let Err(thrown) = self.zip_file_ensure_open(zip) {
                    return Some(thrown);
                }
                let total = self.zip_file_total(zip)?;
                Some(DexExecutionResult::Int(i32::try_from(total).ok()?))
            }
            // cite: libcore ZipFile.java#getName
            ("getName", "()Ljava/lang/String;", [zip]) => self
                .read_stored_instance_field(zip, &zip_field(ZIP_FILE, "Ljava/lang/String;", "name"))
                .map(DexExecutionResult::Value),
            ("close", "()V", [zip]) => self.zip_file_close(zip),
            _ => None,
        }
    }

    // cite: libcore ZipFile.java#ZipFile
    fn zip_file_open_file(
        &self,
        zip: &DexValue,
        file: &DexValue,
        mode: i32,
    ) -> Option<DexExecutionResult> {
        if mode & OPEN_READ == 0 || mode & !(OPEN_READ | OPEN_DELETE) != 0 {
            return Some(self.zip_throw(
                "Ljava/lang/IllegalArgumentException;",
                Some(&format!("Illegal mode: 0x{mode:x}")),
            ));
        }
        if is_null(file) {
            return Some(self.null_pointer_exception_result());
        }
        let path = super::java_io::file_path_from_value(self, file)?;

        self.zip_file_open(zip, &path, mode)
    }

    // cite: libcore ZipFile.c#ZipFile_open, throwFileNotFoundException
    // cite: frameworks-base ActivityThread.java#initZipPathValidatorCallback
    // cite: frameworks-base SafeZipPathValidatorCallback.java#VALIDATE_ZIP_PATH_FOR_PATH_TRAVERSAL
    fn zip_file_open(&self, zip: &DexValue, path: &str, mode: i32) -> Option<DexExecutionResult> {
        let resolved = match self.open_guest_file_for_read(path) {
            Ok(resolved) => resolved,
            Err(exception) => return Some(DexExecutionResult::Throw(exception)),
        };
        let file = resolved.open_read().ok()?;
        let validate_paths = self.target_sdk_version >= 34;
        let mut state = match read_zip_directory(file, validate_paths) {
            Ok(state) => state,
            Err(message) => return Some(self.zip_throw(ZIP_EXCEPTION, Some(message))),
        };
        state.remove_on_close = (mode & OPEN_DELETE != 0).then(|| path.to_owned());

        let stored = self.write_stored_instance_field(
            zip,
            &zip_field(ZIP_FILE, "Ljava/lang/String;", "name"),
            DexValue::String(std::rc::Rc::from(path)),
        );
        self.runtime
            .borrow_mut()
            .zip_files
            .insert(map_identity_key(zip), state);

        stored.then_some(DexExecutionResult::Void)
    }

    // cite: libcore ZipFile.java#ensureOpen
    fn zip_file_ensure_open(&self, zip: &DexValue) -> std::result::Result<(), DexExecutionResult> {
        let message = if self.zip_flag(zip, &zip_field(ZIP_FILE, "Z", "closeRequested")) {
            "zip file closed"
        } else if !self
            .runtime
            .borrow()
            .zip_files
            .contains_key(&map_identity_key(zip))
        {
            "The object is not initialized."
        } else {
            return Ok(());
        };

        Err(self.zip_throw("Ljava/lang/IllegalStateException;", Some(message)))
    }

    fn zip_file_total(&self, zip: &DexValue) -> Option<usize> {
        self.runtime
            .borrow()
            .zip_files
            .get(&map_identity_key(zip))
            .map(|state| state.records.len())
    }

    // cite: libcore ZipFile.java#getEntry
    fn zip_file_get_entry(&self, zip: &DexValue, name: &DexValue) -> Option<DexExecutionResult> {
        let name = match name {
            DexValue::String(name) => name,
            name => {
                return is_null(name)
                    .then(|| self.zip_throw("Ljava/lang/NullPointerException;", Some("name")))
            }
        };
        if let Err(thrown) = self.zip_file_ensure_open(zip) {
            return Some(thrown);
        }
        let record = self
            .runtime
            .borrow()
            .zip_files
            .get(&map_identity_key(zip))?
            .find(name)
            .cloned();

        match record {
            Some(record) => Some(self.new_zip_entry(&record, name.to_string())),
            None => Some(DexExecutionResult::Value(DexValue::Null)),
        }
    }

    // cite: libcore ZipCoder.java#toString
    fn zip_text(&self, bytes: &[u8]) -> std::result::Result<String, DexExecutionResult> {
        String::from_utf8(bytes.to_vec())
            .map_err(|_| self.zip_throw("Ljava/lang/IllegalArgumentException;", Some("MALFORMED")))
    }

    // cite: libcore ZipFile.java#getZipEntry
    // cite: libcore ZipFile.c#ZipFile_getEntryMethod, ZipFile_getEntryCSize, ZipFile_getEntryBytes
    fn new_zip_entry(&self, record: &ZipRecord, name: String) -> DexExecutionResult {
        let comment = if record.comment.is_empty() {
            DexValue::Null
        } else {
            let length = record.comment.iter().position(|&byte| byte == 0);
            match self.zip_text(&record.comment[..length.unwrap_or(record.comment.len())]) {
                Ok(comment) => DexValue::String(std::rc::Rc::from(comment)),
                Err(thrown) => return thrown,
            }
        };
        let extra = if record.extra.is_empty() {
            DexValue::Null
        } else {
            self.new_guest_bytes(&record.extra)
        };
        let (method, compressed_size) = if record.deflated_size == 0 {
            (0, record.size)
        } else {
            (8, record.deflated_size)
        };

        let entry = self.allocate_object(ZIP_ENTRY.to_owned());
        for (type_descriptor, field, value) in [
            (
                "Ljava/lang/String;",
                "name",
                DexValue::String(std::rc::Rc::from(name)),
            ),
            ("J", "xdostime", DexValue::Long(i64::from(record.time))),
            ("J", "crc", DexValue::Long(i64::from(record.crc))),
            ("J", "size", DexValue::Long(record.size as i64)),
            ("J", "csize", DexValue::Long(compressed_size as i64)),
            ("I", "method", DexValue::Int(method)),
            ("I", "flag", DexValue::Int(i32::from(record.flag))),
            ("[B", "extra", extra),
            ("Ljava/lang/String;", "comment", comment),
        ] {
            self.write_stored_instance_field(&entry, &entry_field(type_descriptor, field), value);
        }

        DexExecutionResult::Value(entry)
    }

    // cite: libcore ZipEntry.java#getName, getTime, getSize, getCompressedSize, getCrc, getMethod, getExtra, getComment, isDirectory, toString
    fn zip_entry_accessor(
        &self,
        entry: &DexValue,
        name: &str,
        descriptor: &str,
    ) -> Option<DexExecutionResult> {
        let entry_name =
            self.zip_string_field(entry, &entry_field("Ljava/lang/String;", "name"))?;
        let long = |field| self.zip_long_field(entry, &entry_field("J", field));
        let value = |type_descriptor, field| {
            self.read_stored_instance_field(entry, &entry_field(type_descriptor, field))
                .map(DexExecutionResult::Value)
        };

        match (name, descriptor) {
            ("getName" | "toString", "()Ljava/lang/String;") => Some(string_result(&entry_name)),
            ("isDirectory", "()Z") => Some(DexExecutionResult::Int(i32::from(
                entry_name.ends_with('/'),
            ))),
            ("getSize", "()J") => long("size").map(long_result),
            ("getCompressedSize", "()J") => long("csize").map(long_result),
            ("getCrc", "()J") => long("crc").map(long_result),
            ("getTime", "()J") => self.zip_entry_time(entry).map(long_result),
            ("getMethod", "()I") => {
                match self.read_stored_instance_field(entry, &entry_field("I", "method"))? {
                    DexValue::Int(method) => Some(DexExecutionResult::Int(method)),
                    _ => None,
                }
            }
            ("getExtra", "()[B") => value("[B", "extra"),
            ("getComment", "()Ljava/lang/String;") => value("Ljava/lang/String;", "comment"),
            _ => None,
        }
    }

    // cite: libcore ZipEntry.java#getTime
    // cite: libcore ZipUtils.java#extendedDosToJavaTime, dosToJavaTime
    fn zip_entry_time(&self, entry: &DexValue) -> Option<i64> {
        let extra = self.read_stored_instance_field(entry, &entry_field("[B", "extra"))?;
        if let Some(length) = self.array_length(&extra) {
            if let Some(millis) = extra_modification_millis(&self.guest_bytes(&extra, 0, length)?) {
                return Some(millis);
            }
        }
        let time = self.zip_long_field(entry, &entry_field("J", "xdostime"))?;
        let local = dos_local_seconds(time);
        let rules =
            crate::tzdata::zone_rules_for_id(&super::java_core_runtime::host_default_zone_id())?;

        Some((local - i64::from(rules.offset_seconds_for_local(local))) * 1000)
    }

    // cite: libcore ZipFile.java#getInputStream
    fn zip_file_input_stream(
        &self,
        zip: &DexValue,
        entry: &DexValue,
    ) -> Option<DexExecutionResult> {
        if is_null(entry) {
            return Some(self.zip_throw("Ljava/lang/NullPointerException;", Some("entry")));
        }
        if let Err(thrown) = self.zip_file_ensure_open(zip) {
            return Some(thrown);
        }
        let name = self.zip_string_field(entry, &entry_field("Ljava/lang/String;", "name"))?;
        let zip_key = map_identity_key(zip);
        let loaded = {
            let runtime = self.runtime.borrow();
            let state = runtime.zip_files.get(&zip_key)?;
            state.find(&name).map(|record| {
                let class = if record.deflated_size == 0 {
                    STORED_STREAM
                } else {
                    DEFLATED_STREAM
                };
                (class, state.load(record))
            })
        };
        let Some((class, (bytes, failure))) = loaded else {
            return Some(DexExecutionResult::Value(DexValue::Null));
        };

        let stream = self.allocate_object(class.to_owned());
        self.write_stored_instance_field(
            &stream,
            &zip_field(INPUT_STREAM, ZIP_FILE, "winedroidZipFile"),
            zip.clone(),
        );
        if let Some((class_descriptor, message)) = failure {
            if let DexExecutionResult::Throw(exception) =
                self.zip_throw(class_descriptor, Some(message))
            {
                self.write_stored_instance_field(
                    &stream,
                    &zip_field(INPUT_STREAM, IO_EXCEPTION, "winedroidFailure"),
                    exception,
                );
            }
        }
        let mut runtime = self.runtime.borrow_mut();
        runtime.input_stream_values.insert(
            map_identity_key(&stream),
            InputStreamState {
                bytes,
                position: 0,
                pending_low_surrogate: None,
            },
        );
        runtime
            .zip_files
            .get_mut(&zip_key)?
            .streams
            .push(stream.clone());

        Some(DexExecutionResult::Value(stream))
    }

    // cite: libcore ZipFile.java#close
    fn zip_file_close(&self, zip: &DexValue) -> Option<DexExecutionResult> {
        let state = self
            .runtime
            .borrow_mut()
            .zip_files
            .remove(&map_identity_key(zip));
        self.write_stored_instance_field(
            zip,
            &zip_field(ZIP_FILE, "Z", "closeRequested"),
            DexValue::Int(1),
        )
        .then_some(())?;
        if let Some(state) = state {
            for stream in &state.streams {
                self.close_entry_stream(stream);
            }
            if let (Some(path), Some(filesystem)) =
                (state.remove_on_close, self.guest_filesystem.as_ref())
            {
                let writable: Option<ResolvedGuestPath> = filesystem
                    .resolve(
                        &path,
                        ExternalStorageAccess::Write,
                        &self.external_storage_policy,
                    )
                    .ok();
                if let Some(writable) = writable {
                    writable.remove().ok();
                }
            }
        }

        Some(DexExecutionResult::Void)
    }

    fn close_entry_stream(&self, stream: &DexValue) {
        self.write_stored_instance_field(
            stream,
            &zip_field(INPUT_STREAM, "Z", "closed"),
            DexValue::Int(1),
        );
        let zip = self.read_stored_instance_field(
            stream,
            &zip_field(INPUT_STREAM, ZIP_FILE, "winedroidZipFile"),
        );
        let mut runtime = self.runtime.borrow_mut();
        runtime
            .input_stream_values
            .remove(&map_identity_key(stream));
        if let Some(state) = zip.and_then(|zip| runtime.zip_files.get_mut(&map_identity_key(&zip)))
        {
            state.streams.retain(|open| open != stream);
        }
    }

    // cite: libcore ZipFile.java#hasMoreElements, hasNext, nextElement, next
    fn zip_enumeration(&self, iterator: &DexValue, name: &str) -> Option<DexExecutionResult> {
        if value_class_descriptor(iterator) != Some(ENTRY_ITERATOR) {
            return None;
        }
        let zip = self
            .read_stored_instance_field(iterator, &zip_field(ENTRY_ITERATOR, ZIP_FILE, "this$0"))?;
        let position_field = zip_field(ENTRY_ITERATOR, "I", "i");
        let DexValue::Int(position) = self.read_stored_instance_field(iterator, &position_field)?
        else {
            return None;
        };
        if let Err(thrown) = self.zip_file_ensure_open(&zip) {
            return Some(thrown);
        }
        let record = self
            .runtime
            .borrow()
            .zip_files
            .get(&map_identity_key(&zip))?
            .records
            .get(usize::try_from(position).ok()?)
            .cloned();

        match (name, record) {
            ("hasMoreElements", record) => {
                Some(DexExecutionResult::Int(i32::from(record.is_some())))
            }
            ("nextElement", None) => {
                Some(self.zip_throw("Ljava/util/NoSuchElementException;", None))
            }
            ("nextElement", Some(record)) => {
                self.write_stored_instance_field(
                    iterator,
                    &position_field,
                    DexValue::Int(position + 1),
                );
                match self.zip_text(&record.name) {
                    Ok(name) => Some(self.new_zip_entry(&record, name)),
                    Err(thrown) => Some(thrown),
                }
            }
            _ => None,
        }
    }

    // cite: libcore CRC32.java#update, updateBytes, updateBytesCheck, reset, getValue
    // cite: libcore CRC32.c#Java_java_util_zip_CRC32_update, Java_java_util_zip_CRC32_updateBytes0
    fn crc32_method(
        &self,
        name: &str,
        descriptor: &str,
        arguments: &[DexValue],
    ) -> Option<DexExecutionResult> {
        let checksum = arguments.first()?;
        if !self.value_matches_type(checksum, CRC32) {
            return None;
        }
        let field = zip_field(CRC32, "I", "crc");
        let crc = match self.read_stored_instance_field(checksum, &field) {
            Some(DexValue::Int(crc)) => crc as u32,
            _ => 0,
        };

        let bytes = match (name, descriptor, &arguments[1..]) {
            ("getValue", "()J", []) => return Some(long_result(i64::from(crc))),
            ("reset", "()V", []) => {
                return self
                    .write_stored_instance_field(checksum, &field, DexValue::Int(0))
                    .then_some(DexExecutionResult::Void)
            }
            ("update", "(I)V", [DexValue::Int(byte)]) => vec![*byte as u8],
            ("update", "([B)V", [array]) => {
                if is_null(array) {
                    return Some(self.null_pointer_exception_result());
                }
                self.guest_bytes(array, 0, self.array_length(array)?)?
            }
            ("update", "([BII)V", [array, DexValue::Int(offset), DexValue::Int(count)]) => {
                if is_null(array) {
                    return Some(self.null_pointer_exception_result());
                }
                if out_of_bounds(self.array_length(array)?, *offset, *count) {
                    return Some(
                        self.zip_throw("Ljava/lang/ArrayIndexOutOfBoundsException;", None),
                    );
                }
                self.guest_bytes(array, *offset, *count)?
            }
            _ => return None,
        };
        let mut hasher = crc32fast::Hasher::new_with_initial(crc);
        hasher.update(&bytes);

        self.write_stored_instance_field(checksum, &field, DexValue::Int(hasher.finalize() as i32))
            .then_some(DexExecutionResult::Void)
    }

    fn checksum_update(
        &self,
        checksum: &DexValue,
        parameters: &[&str],
        arguments: &[DexValue],
        depth: usize,
    ) -> Result<Option<DexExecutionResult>> {
        if is_null(checksum) {
            return Ok(Some(self.null_pointer_exception_result()));
        }
        if let Some(result) =
            self.zip_guest_call(checksum, "update", "V", parameters, arguments, depth)?
        {
            return Ok(Some(result));
        }
        let mut with_receiver = vec![checksum.clone()];
        with_receiver.extend_from_slice(arguments);
        let descriptor = format!("({})V", parameters.concat());

        self.crc32_method("update", &descriptor, &with_receiver)
            .map(Some)
            .ok_or_else(|| {
                let class_descriptor = value_class_descriptor(checksum).unwrap_or(CHECKSUM);
                WinedroidError::new(format!(
                    "Checksum has no implementation\nnext missing primitive: method {class_descriptor}.update{descriptor}"
                ))
            })
    }

    fn stream_kind(&self, stream: &DexValue) -> Option<StreamKind> {
        if let Some(checksum) =
            self.read_stored_instance_field(stream, &zip_field(CHECKED_STREAM, CHECKSUM, "cksum"))
        {
            let source = self.read_stored_instance_field(
                stream,
                &zip_field(FILTER_STREAM, INPUT_STREAM, "in"),
            )?;
            return Some(StreamKind::Checked { source, checksum });
        }
        if let Some(path) = self.zip_string_field(
            stream,
            &zip_field(FILE_STREAM, "Ljava/lang/String;", "path"),
        ) {
            return Some(StreamKind::File(path));
        }
        match value_class_descriptor(stream)? {
            STORED_STREAM => Some(StreamKind::StoredEntry),
            DEFLATED_STREAM => Some(StreamKind::DeflatedEntry),
            _ => self
                .runtime
                .borrow()
                .input_stream_values
                .contains_key(&map_identity_key(stream))
                .then_some(StreamKind::Buffered),
        }
    }

    fn stream_closed(&self, stream: &DexValue, kind: &StreamKind) -> bool {
        let class = match kind {
            StreamKind::File(_) => FILE_STREAM,
            _ => INPUT_STREAM,
        };

        self.zip_flag(stream, &zip_field(class, "Z", "closed"))
    }

    // cite: libcore ZipFile.java#ensureOpenOrZipException, read
    // cite: libcore InflaterInputStream.java#ensureOpen
    // cite: libcore FileInputStream.java#read
    fn stream_closed_failure(
        &self,
        stream: &DexValue,
        kind: &StreamKind,
        count: i32,
    ) -> Option<DexExecutionResult> {
        match kind {
            StreamKind::File(_) if count > 0 && self.stream_closed(stream, kind) => {
                Some(self.zip_throw(IO_EXCEPTION, Some("Stream Closed")))
            }
            StreamKind::StoredEntry => {
                let zip = self.read_stored_instance_field(
                    stream,
                    &zip_field(INPUT_STREAM, ZIP_FILE, "winedroidZipFile"),
                )?;
                self.zip_flag(&zip, &zip_field(ZIP_FILE, "Z", "closeRequested"))
                    .then(|| self.zip_throw(ZIP_EXCEPTION, Some("ZipFile closed")))
            }
            StreamKind::DeflatedEntry if self.stream_closed(stream, kind) => {
                Some(self.zip_throw(IO_EXCEPTION, Some("Stream closed")))
            }
            _ => None,
        }
    }

    // cite: libcore ByteArrayInputStream.java#buf, count, read
    fn refresh_byte_array_input_stream(&self, stream: &DexValue) {
        let Some(array) = self.read_stored_instance_field(stream, &byte_array_input_buffer_field())
        else {
            return;
        };
        let Some(DexValue::Int(count)) =
            self.read_stored_instance_field(stream, &byte_array_input_count_field())
        else {
            return;
        };
        let Some(mut bytes) = self.byte_array_contents(&array) else {
            return;
        };
        bytes.truncate(usize::try_from(count).unwrap_or(0));
        if let Some(state) = self
            .runtime
            .borrow_mut()
            .input_stream_values
            .get_mut(&map_identity_key(stream))
        {
            state.bytes = bytes;
        }
    }

    // cite: libcore ByteArrayInputStream.java#ByteArrayInputStream
    pub(super) fn initialize_byte_array_input_stream(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let (stream, array, offset, length) = match invocation.arguments.as_slice() {
            [stream, array] => (stream, array, 0, None),
            [stream, array, DexValue::Int(offset), DexValue::Int(length)] => {
                (stream, array, *offset, Some(*length))
            }
            _ => return None,
        };
        if !dex_value_is_object_like(stream) {
            return None;
        }
        if is_null(array) {
            return Some(self.null_pointer_exception_result());
        }
        let buffer_length = self.array_length(array)?;
        let count = match length {
            None => buffer_length,
            Some(length) => offset.saturating_add(length).min(buffer_length),
        };
        self.write_stored_instance_field(stream, &byte_array_input_buffer_field(), array.clone());
        self.write_stored_instance_field(
            stream,
            &byte_array_input_count_field(),
            DexValue::Int(count),
        );
        self.runtime.borrow_mut().input_stream_values.insert(
            map_identity_key(stream),
            InputStreamState {
                bytes: Vec::new(),
                position: usize::try_from(offset).unwrap_or(0),
                pending_low_surrogate: None,
            },
        );
        self.refresh_byte_array_input_stream(stream);

        Some(DexExecutionResult::Void)
    }

    fn stream_remaining(&self, stream: &DexValue) -> usize {
        self.refresh_byte_array_input_stream(stream);
        self.runtime
            .borrow()
            .input_stream_values
            .get(&map_identity_key(stream))
            .map_or(0, |state| state.bytes.len().saturating_sub(state.position))
    }

    fn stream_take(
        &self,
        stream: &DexValue,
        kind: &StreamKind,
        count: usize,
    ) -> Option<std::result::Result<Vec<u8>, DexExecutionResult>> {
        if let StreamKind::File(path) = kind {
            let position_field = zip_field(FILE_STREAM, "J", "winedroidPosition");
            let position = self.zip_long_field(stream, &position_field).unwrap_or(0);
            let read = self
                .open_guest_file_for_read(path)
                .map_err(DexExecutionResult::Throw)
                .and_then(|resolved| {
                    let mut bytes = Vec::new();
                    resolved
                        .open_read()
                        .and_then(|mut file| {
                            file.seek(SeekFrom::Start(position as u64))?;
                            file.take(count as u64).read_to_end(&mut bytes)
                        })
                        .map_err(|error| self.zip_throw(IO_EXCEPTION, Some(&error.to_string())))?;
                    Ok(bytes)
                });
            if let Ok(bytes) = &read {
                self.write_stored_instance_field(
                    stream,
                    &position_field,
                    DexValue::Long(position + bytes.len() as i64),
                )
                .then_some(())?;
            }
            return Some(read);
        }

        self.refresh_byte_array_input_stream(stream);
        let bytes = self
            .runtime
            .borrow_mut()
            .input_stream_values
            .get_mut(&map_identity_key(stream))
            .map_or_else(Vec::new, |state| {
                let start = state.position.min(state.bytes.len());
                let end = start.saturating_add(count).min(state.bytes.len());
                state.position = end;
                state.bytes[start..end].to_vec()
            });
        if bytes.is_empty() {
            if let Some(failure) = self.read_stored_instance_field(
                stream,
                &zip_field(INPUT_STREAM, IO_EXCEPTION, "winedroidFailure"),
            ) {
                return Some(Err(DexExecutionResult::Throw(failure)));
            }
        }

        Some(Ok(bytes))
    }

    fn stream_read_into(
        &self,
        stream: &DexValue,
        array: &DexValue,
        offset: i32,
        count: i32,
        depth: usize,
    ) -> Result<Option<DexExecutionResult>> {
        if is_null(stream) {
            return Ok(Some(self.null_pointer_exception_result()));
        }
        let arguments = [array.clone(), DexValue::Int(offset), DexValue::Int(count)];
        match self.zip_guest_call(stream, "read", "I", &["[B", "I", "I"], &arguments, depth)? {
            Some(result) => Ok(Some(result)),
            None => self.builtin_stream_read_into(stream, array, offset, count, depth),
        }
    }

    fn builtin_stream_read_into(
        &self,
        stream: &DexValue,
        array: &DexValue,
        offset: i32,
        count: i32,
        depth: usize,
    ) -> Result<Option<DexExecutionResult>> {
        let Some(kind) = self.stream_kind(stream) else {
            return Ok(None);
        };
        // cite: libcore CheckedInputStream.java#read
        if let StreamKind::Checked { source, checksum } = &kind {
            let result = self.stream_read_into(source, array, offset, count, depth + 1)?;
            let Some(DexExecutionResult::Int(read)) = result else {
                return Ok(result);
            };
            if read != -1 {
                let update = self.checksum_update(
                    checksum,
                    &["[B", "I", "I"],
                    &[array.clone(), DexValue::Int(offset), DexValue::Int(read)],
                    depth,
                )?;
                if update != Some(DexExecutionResult::Void) {
                    return Ok(update);
                }
            }
            return Ok(result);
        }
        if let Some(failure) = self.stream_closed_failure(stream, &kind, count) {
            return Ok(Some(failure));
        }

        // cite: libcore ZipFile.java#read
        // cite: libcore ZipFile.c#ZipFile_read
        // cite: libcore IoBridge.java#read
        // cite: libcore ArrayUtils.java#throwsIfOutOfBounds
        // cite: libcore InflaterInputStream.java#read
        // cite: libcore InputStream.java#read
        let mut count = count;
        if matches!(kind, StreamKind::StoredEntry) {
            let remaining = self.stream_remaining(stream);
            if remaining == 0 {
                return Ok(match self.stream_take(stream, &kind, 1) {
                    Some(Err(failure)) => Some(failure),
                    taken => taken.map(|_| DexExecutionResult::Int(-1)),
                });
            }
            if count <= 0 {
                return Ok(Some(DexExecutionResult::Int(0)));
            }
            count = count.min(i32::try_from(remaining).unwrap_or(i32::MAX));
        }
        if is_null(array) {
            return Ok(Some(self.null_pointer_exception_result()));
        }
        let Some(length) = self.array_length(array) else {
            return Ok(None);
        };
        if out_of_bounds(length, offset, count) {
            let class = match kind {
                StreamKind::File(_) | StreamKind::StoredEntry => {
                    "Ljava/lang/ArrayIndexOutOfBoundsException;"
                }
                _ => "Ljava/lang/IndexOutOfBoundsException;",
            };
            return Ok(Some(self.zip_throw(class, None)));
        }
        if count == 0 {
            return Ok(Some(DexExecutionResult::Int(0)));
        }

        Ok(match self.stream_take(stream, &kind, count as usize) {
            Some(Ok(bytes)) if bytes.is_empty() => Some(DexExecutionResult::Int(-1)),
            Some(Ok(bytes)) => self
                .write_guest_bytes(array, offset, &bytes)
                .map(|()| DexExecutionResult::Int(bytes.len() as i32)),
            Some(Err(failure)) => Some(failure),
            None => None,
        })
    }

    fn stream_read_byte(
        &self,
        stream: &DexValue,
        depth: usize,
    ) -> Result<Option<DexExecutionResult>> {
        if is_null(stream) {
            return Ok(Some(self.null_pointer_exception_result()));
        }
        match self.zip_guest_call(stream, "read", "I", &[], &[], depth)? {
            Some(result) => Ok(Some(result)),
            None => self.builtin_stream_read_byte(stream, depth),
        }
    }

    fn builtin_stream_read_byte(
        &self,
        stream: &DexValue,
        depth: usize,
    ) -> Result<Option<DexExecutionResult>> {
        let Some(kind) = self.stream_kind(stream) else {
            return Ok(None);
        };
        match &kind {
            // cite: libcore CheckedInputStream.java#read
            StreamKind::Checked { source, checksum } => {
                let result = self.stream_read_byte(source, depth + 1)?;
                let Some(DexExecutionResult::Int(byte)) = result else {
                    return Ok(result);
                };
                if byte != -1 {
                    let update =
                        self.checksum_update(checksum, &["I"], &[DexValue::Int(byte)], depth)?;
                    if update != Some(DexExecutionResult::Void) {
                        return Ok(update);
                    }
                }
                return Ok(result);
            }
            // cite: libcore FileInputStream.java#read
            StreamKind::File(_) => {
                let array = self.new_guest_bytes(&[0]);
                let arguments = [array.clone(), DexValue::Int(0), DexValue::Int(1)];
                if let Some(result) =
                    self.zip_guest_call(stream, "read", "I", &["[B", "I", "I"], &arguments, depth)?
                {
                    let DexExecutionResult::Int(read) = result else {
                        return Ok(Some(result));
                    };
                    return Ok(match read {
                        -1 => Some(DexExecutionResult::Int(-1)),
                        _ => self
                            .guest_bytes(&array, 0, 1)
                            .map(|bytes| DexExecutionResult::Int(i32::from(bytes[0]))),
                    });
                }
            }
            _ => {}
        }
        if let Some(failure) = self.stream_closed_failure(stream, &kind, 1) {
            return Ok(Some(failure));
        }

        Ok(self.stream_take(stream, &kind, 1).map(|taken| match taken {
            Ok(bytes) => DexExecutionResult::Int(bytes.first().map_or(-1, |&byte| i32::from(byte))),
            Err(failure) => failure,
        }))
    }

    fn builtin_stream_available(
        &self,
        stream: &DexValue,
        depth: usize,
    ) -> Result<Option<DexExecutionResult>> {
        let Some(kind) = self.stream_kind(stream) else {
            return Ok(None);
        };
        let available = match &kind {
            // cite: libcore FilterInputStream.java#available
            StreamKind::Checked { source, .. } => {
                if is_null(source) {
                    return Ok(Some(self.null_pointer_exception_result()));
                }
                return match self.zip_guest_call(source, "available", "I", &[], &[], depth)? {
                    Some(result) => Ok(Some(result)),
                    None => self.builtin_stream_available(source, depth + 1),
                };
            }
            // cite: libcore FileInputStream.java#available
            // cite: libcore FileInputStream.c#FileInputStream_available0
            StreamKind::File(path) => {
                if self.stream_closed(stream, &kind) {
                    return Ok(Some(self.zip_throw(IO_EXCEPTION, Some("Stream Closed"))));
                }
                let length = match self.open_guest_file_for_read(path) {
                    Ok(resolved) => resolved.metadata().ok(),
                    Err(exception) => return Ok(Some(DexExecutionResult::Throw(exception))),
                };
                let position = self
                    .zip_long_field(stream, &zip_field(FILE_STREAM, "J", "winedroidPosition"))
                    .unwrap_or(0);
                match length {
                    Some(metadata) => (metadata.len as i64 - position).max(0) as u64,
                    None => return Ok(None),
                }
            }
            // cite: libcore ZipFile.java#available
            _ => self.stream_remaining(stream) as u64,
        };

        Ok(Some(DexExecutionResult::Int(
            i32::try_from(available).unwrap_or(i32::MAX),
        )))
    }

    fn builtin_stream_close(
        &self,
        stream: &DexValue,
        depth: usize,
    ) -> Result<Option<DexExecutionResult>> {
        let Some(kind) = self.stream_kind(stream) else {
            return Ok(None);
        };
        match &kind {
            // cite: libcore FilterInputStream.java#close
            StreamKind::Checked { source, .. } => {
                if is_null(source) {
                    return Ok(Some(self.null_pointer_exception_result()));
                }
                return match self.zip_guest_call(source, "close", "V", &[], &[], depth)? {
                    Some(result) => Ok(Some(result)),
                    None => self.builtin_stream_close(source, depth + 1),
                };
            }
            // cite: libcore FileInputStream.java#close, channel
            StreamKind::File(_) => {
                self.write_stored_instance_field(
                    stream,
                    &zip_field(FILE_STREAM, "Z", "closed"),
                    DexValue::Int(1),
                );
                if let Some(channel @ DexValue::Object(_)) = self.read_stored_instance_field(
                    stream,
                    &super::java_io::file_input_stream_channel_field(),
                ) {
                    if self.file_channel_is_open(&channel) == Some(true) {
                        self.close_file_channel(&channel);
                    }
                }
            }
            // cite: libcore ZipFile.java#close
            StreamKind::StoredEntry | StreamKind::DeflatedEntry => self.close_entry_stream(stream),
            StreamKind::Buffered => {
                self.runtime
                    .borrow_mut()
                    .input_stream_values
                    .remove(&map_identity_key(stream));
            }
        }

        Ok(Some(DexExecutionResult::Void))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dos_time_decodes_to_local_seconds_including_overflowing_fields() {
        // cite: libcore ZipUtils.java#dosToJavaTime, overflowDosToJavaTime
        let noon_2020 = (40 << 25) | (1 << 21) | (1 << 16) | (12 << 11);
        assert_eq!(dos_local_seconds(noon_2020), 1_577_880_000);
        let month_zero_day_zero = 40 << 25;
        assert_eq!(dos_local_seconds(month_zero_day_zero), 1_575_072_000);
    }
}

fn byte_array_input_buffer_field() -> DexFieldId {
    zip_field("Ljava/io/ByteArrayInputStream;", "[B", "buf")
}

fn byte_array_input_count_field() -> DexFieldId {
    zip_field("Ljava/io/ByteArrayInputStream;", "I", "count")
}
