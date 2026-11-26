use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use super::super::*;
use super::helpers::*;

const ZIP_FILE: &str = "Ljava/util/zip/ZipFile;";
const ZIP_ENTRY: &str = "Ljava/util/zip/ZipEntry;";
const ZIP_EXCEPTION: &str = "Ljava/util/zip/ZipException;";
const CRC32: &str = "Ljava/util/zip/CRC32;";
const CHECKED: &str = "Ljava/util/zip/CheckedInputStream;";
const INPUT: &str = "Ljava/io/InputStream;";
const IO_EXCEPTION: &str = "Ljava/io/IOException;";
const CHECK_INPUT: &[u8] = b"123456789";
// cite: libcore CRC32.c#Java_java_util_zip_CRC32_updateBytes0
const CHECK_VALUE: i64 = 0xCBF4_3926;

struct Entry<'a> {
    name: &'a str,
    deflated: bool,
    contents: &'a [u8],
    crc: u32,
    extra: &'a [u8],
    comment: &'a str,
}

fn entry<'a>(name: &'a str, deflated: bool, contents: &'a [u8]) -> Entry<'a> {
    Entry {
        name,
        deflated,
        contents,
        crc: if contents == CHECK_INPUT {
            CHECK_VALUE as u32
        } else {
            0
        },
        extra: &[],
        comment: "",
    }
}

fn push16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

// cite: libcore zip_util.c#readCEN, newEntry, ZIP_GetEntryDataOffset
fn archive(entries: &[Entry<'_>]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut central = Vec::new();
    for entry in entries {
        let data = if entry.deflated {
            let mut encoder =
                flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
            encoder.write_all(entry.contents).expect("deflate");
            encoder.finish().expect("deflate")
        } else {
            entry.contents.to_vec()
        };
        let mut common = Vec::new();
        push16(&mut common, 20);
        push16(&mut common, 0);
        push16(&mut common, if entry.deflated { 8 } else { 0 });
        push16(&mut common, 0x6000);
        push16(&mut common, 0x5021);
        push32(&mut common, entry.crc);
        push32(&mut common, data.len() as u32);
        push32(&mut common, entry.contents.len() as u32);
        push16(&mut common, entry.name.len() as u16);

        central.extend_from_slice(b"PK\x01\x02");
        push16(&mut central, 20);
        central.extend_from_slice(&common);
        push16(&mut central, entry.extra.len() as u16);
        push16(&mut central, entry.comment.len() as u16);
        central.extend_from_slice(&[0; 8]);
        push32(&mut central, bytes.len() as u32);
        central.extend_from_slice(entry.name.as_bytes());
        central.extend_from_slice(entry.extra);
        central.extend_from_slice(entry.comment.as_bytes());

        bytes.extend_from_slice(b"PK\x03\x04");
        bytes.extend_from_slice(&common);
        push16(&mut bytes, 0);
        bytes.extend_from_slice(entry.name.as_bytes());
        bytes.extend_from_slice(&data);
    }
    let central_start = bytes.len() as u32;
    bytes.extend_from_slice(&central);
    bytes.extend_from_slice(b"PK\x05\x06");
    bytes.extend_from_slice(&[0; 4]);
    push16(&mut bytes, entries.len() as u16);
    push16(&mut bytes, entries.len() as u16);
    push32(&mut bytes, central.len() as u32);
    push32(&mut bytes, central_start);
    push16(&mut bytes, 0);

    bytes
}

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        let root = std::env::temp_dir().join(format!(
            "winedroid-java-zip-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        for directory in [
            "apps/com.example/data",
            "apps/com.example/data_de",
            "storage",
        ] {
            fs::create_dir_all(root.join(directory)).expect("fixture directory");
        }

        Self { root }
    }

    fn program(&self) -> DexProgram {
        DexProgram::with_guest_filesystem(
            Vec::new(),
            "com.example",
            self.root.join("apps/com.example/data"),
            self.root.join("storage"),
        )
        .expect("guest filesystem program")
    }

    fn write(&self, name: &str, bytes: &[u8]) -> String {
        fs::write(self.root.join("apps/com.example/data").join(name), bytes).expect("fixture file");

        format!("/data/user/0/com.example/{name}")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).ok();
    }
}

fn text(value: &str) -> DexValue {
    DexValue::String(std::rc::Rc::from(value))
}

fn call(
    program: &DexProgram,
    class: &str,
    name: &str,
    signature: (&[&str], &str),
    arguments: Vec<DexValue>,
) -> Option<DexExecutionResult> {
    let method = builtin_method(class, name, signature.1, signature.0);

    program.invoke_builtin_method(&method, Some(&test_invocation(arguments)))
}

fn value(result: Option<DexExecutionResult>) -> DexValue {
    match result {
        Some(DexExecutionResult::Value(value)) => value,
        Some(DexExecutionResult::Int(value)) => DexValue::Int(value),
        result => panic!("expected a value, got {result:?}"),
    }
}

fn int(result: Option<DexExecutionResult>) -> i32 {
    match value(result) {
        DexValue::Int(value) => value,
        value => panic!("expected an int, got {value}"),
    }
}

fn assert_throws(
    program: &DexProgram,
    result: Option<DexExecutionResult>,
    class: &str,
    message: Option<&str>,
) {
    let exception = thrown_exception(result);
    assert_eq!(value_class_descriptor(&exception), Some(class));
    assert_eq!(
        thrown_detail_message(program, &exception).as_deref(),
        message
    );
}

fn open(program: &DexProgram, path: &str) -> (DexValue, Option<DexExecutionResult>) {
    let zip = program.allocate_object(ZIP_FILE.to_owned());
    let result = call(
        program,
        ZIP_FILE,
        "<init>",
        (&["Ljava/lang/String;"], "V"),
        vec![zip.clone(), text(path)],
    );

    (zip, result)
}

fn opened(program: &DexProgram, path: &str) -> DexValue {
    let (zip, result) = open(program, path);
    assert_eq!(result, Some(DexExecutionResult::Void));

    zip
}

fn get_entry(program: &DexProgram, zip: &DexValue, name: DexValue) -> Option<DexExecutionResult> {
    call(
        program,
        ZIP_FILE,
        "getEntry",
        (&["Ljava/lang/String;"], ZIP_ENTRY),
        vec![zip.clone(), name],
    )
}

fn accessor(program: &DexProgram, entry: &DexValue, name: &str, return_type: &str) -> DexValue {
    value(call(
        program,
        ZIP_ENTRY,
        name,
        (&[], return_type),
        vec![entry.clone()],
    ))
}

fn stream(program: &DexProgram, zip: &DexValue, name: &str) -> DexValue {
    let entry = value(get_entry(program, zip, text(name)));

    value(call(
        program,
        ZIP_FILE,
        "getInputStream",
        (&[ZIP_ENTRY], INPUT),
        vec![zip.clone(), entry],
    ))
}

fn buffer(program: &DexProgram, length: i32) -> DexValue {
    program
        .allocate_array("[B".to_owned(), length)
        .expect("byte array")
}

fn read(program: &DexProgram, class: &str, stream: &DexValue, array: &DexValue) -> i32 {
    int(call(
        program,
        class,
        "read",
        (&["[B"], "I"),
        vec![stream.clone(), array.clone()],
    ))
}

fn read_result(program: &DexProgram, stream: &DexValue) -> Option<DexExecutionResult> {
    call(program, INPUT, "read", (&[], "I"), vec![stream.clone()])
}

fn drain(program: &DexProgram, class: &str, stream: &DexValue) -> Vec<u8> {
    let array = buffer(program, 4);
    let mut bytes = Vec::new();
    loop {
        let count = read(program, class, stream, &array);
        if count == -1 {
            return bytes;
        }
        for index in 0..count {
            let Some(DexValue::Int(byte)) = program.read_array_value(&array, index) else {
                panic!("byte array element");
            };
            bytes.push(byte as u8);
        }
    }
}

#[test]
fn libcore_zip_file_reads_stored_and_deflated_entries() {
    // cite: libcore ZipFile.java#getEntry, getZipEntry, getInputStream, size, getName
    // cite: libcore ZipFile.c#ZipFile_getEntryMethod, ZipFile_getEntryCSize
    let fixture = Fixture::new();
    let repeated = vec![b'a'; 70_000];
    let mut stored = entry("lib/stored.bin", false, CHECK_INPUT);
    stored.comment = "note";
    let path = fixture.write(
        "a.zip",
        &archive(&[stored, entry("deflated.txt", true, &repeated)]),
    );
    let program = fixture.program();
    let zip = opened(&program, &path);

    assert_eq!(
        value(call(
            &program,
            ZIP_FILE,
            "getName",
            (&[], "Ljava/lang/String;"),
            vec![zip.clone()]
        )),
        text(&path)
    );
    assert_eq!(
        int(call(
            &program,
            ZIP_FILE,
            "size",
            (&[], "I"),
            vec![zip.clone()]
        )),
        2
    );
    let first = value(get_entry(&program, &zip, text("lib/stored.bin")));
    assert_eq!(
        accessor(&program, &first, "getName", "Ljava/lang/String;"),
        text("lib/stored.bin")
    );
    assert_eq!(
        accessor(&program, &first, "getCrc", "J"),
        DexValue::Long(CHECK_VALUE)
    );
    assert_eq!(
        accessor(&program, &first, "getSize", "J"),
        DexValue::Long(9)
    );
    assert_eq!(
        accessor(&program, &first, "getCompressedSize", "J"),
        DexValue::Long(9)
    );
    assert_eq!(
        accessor(&program, &first, "getMethod", "I"),
        DexValue::Int(0)
    );
    assert_eq!(
        accessor(&program, &first, "getComment", "Ljava/lang/String;"),
        text("note")
    );
    assert_eq!(accessor(&program, &first, "getExtra", "[B"), DexValue::Null);
    let second = value(get_entry(&program, &zip, text("deflated.txt")));
    assert_eq!(
        accessor(&program, &second, "getMethod", "I"),
        DexValue::Int(8)
    );
    assert_eq!(
        accessor(&program, &second, "getSize", "J"),
        DexValue::Long(70_000)
    );

    let first = stream(&program, &zip, "lib/stored.bin");
    assert_eq!(drain(&program, INPUT, &first), CHECK_INPUT);
    let second = stream(&program, &zip, "deflated.txt");
    assert_eq!(
        int(call(
            &program,
            INPUT,
            "available",
            (&[], "I"),
            vec![second.clone()]
        )),
        70_000
    );
    let array = buffer(&program, 70_000);
    assert_eq!(read(&program, INPUT, &second, &array), 70_000);
    assert_eq!(
        program.read_array_value(&array, 69_999),
        Some(DexValue::Int(i32::from(b'a')))
    );
    assert_eq!(read(&program, INPUT, &second, &array), -1);
}

#[test]
fn libcore_checked_input_stream_checksums_a_file_input_stream_until_end_of_file() {
    // cite: libcore CheckedInputStream.java#read, getChecksum
    // cite: libcore FilterInputStream.java#read
    // cite: libcore FileInputStream.java#read
    let fixture = Fixture::new();
    let path = fixture.write("digits", CHECK_INPUT);
    let program = fixture.program();
    let source = program.allocate_object("Ljava/io/FileInputStream;".to_owned());
    assert_eq!(
        call(
            &program,
            "Ljava/io/FileInputStream;",
            "<init>",
            (&["Ljava/lang/String;"], "V"),
            vec![source.clone(), text(&path)],
        ),
        Some(DexExecutionResult::Void)
    );
    let checksum = program.allocate_object(CRC32.to_owned());
    let checked = program.allocate_object(CHECKED.to_owned());
    assert_eq!(
        call(
            &program,
            CHECKED,
            "<init>",
            (&[INPUT, "Ljava/util/zip/Checksum;"], "V"),
            vec![checked.clone(), source.clone(), checksum.clone()],
        ),
        Some(DexExecutionResult::Void)
    );

    assert_eq!(drain(&program, CHECKED, &checked), CHECK_INPUT);
    let returned = value(call(
        &program,
        CHECKED,
        "getChecksum",
        (&[], "Ljava/util/zip/Checksum;"),
        vec![checked.clone()],
    ));
    assert_eq!(returned, checksum);
    assert_eq!(
        value(call(
            &program,
            "Ljava/util/zip/Checksum;",
            "getValue",
            (&[], "J"),
            vec![returned]
        )),
        DexValue::Long(CHECK_VALUE)
    );

    assert_eq!(
        call(
            &program,
            "Ljava/io/Closeable;",
            "close",
            (&[], "V"),
            vec![checked.clone()]
        ),
        Some(DexExecutionResult::Void)
    );
    assert_throws(
        &program,
        read_result(&program, &source),
        IO_EXCEPTION,
        Some("Stream Closed"),
    );
}

#[test]
fn libcore_crc32_updates_by_byte_and_range() {
    // cite: libcore CRC32.java#update, reset, getValue
    let program = DexProgram::new(Vec::new());
    let checksum = program.allocate_object(CRC32.to_owned());
    let bytes = program.allocate_filled_array(
        "[B".to_owned(),
        CHECK_INPUT
            .iter()
            .map(|&byte| DexValue::Int(i32::from(byte)))
            .collect(),
    );
    let update = |signature: &[&str], arguments: Vec<DexValue>| {
        let mut with_receiver = vec![checksum.clone()];
        with_receiver.extend(arguments);
        call(&program, CRC32, "update", (signature, "V"), with_receiver)
    };
    let current = || {
        value(call(
            &program,
            CRC32,
            "getValue",
            (&[], "J"),
            vec![checksum.clone()],
        ))
    };

    assert_eq!(current(), DexValue::Long(0));
    for &byte in &CHECK_INPUT[..4] {
        update(&["I"], vec![DexValue::Int(i32::from(byte))]);
    }
    update(
        &["[B", "I", "I"],
        vec![bytes.clone(), DexValue::Int(4), DexValue::Int(5)],
    );
    assert_eq!(current(), DexValue::Long(CHECK_VALUE));
    call(&program, CRC32, "reset", (&[], "V"), vec![checksum.clone()]);
    assert_eq!(current(), DexValue::Long(0));
    update(&["[B"], vec![bytes.clone()]);
    assert_eq!(current(), DexValue::Long(CHECK_VALUE));
    assert_throws(
        &program,
        update(
            &["[B", "I", "I"],
            vec![bytes, DexValue::Int(5), DexValue::Int(5)],
        ),
        "Ljava/lang/ArrayIndexOutOfBoundsException;",
        None,
    );
}

#[test]
fn libcore_zip_file_looks_up_and_enumerates_entries() {
    // cite: libcore zip_util.c#ZIP_GetEntry2
    // cite: libcore ZipFile.java#getEntry, nextElement
    // cite: libcore ZipEntry.java#isDirectory, setExtra0, getTime
    let fixture = Fixture::new();
    let extended_timestamp = [0x55, 0x54, 5, 0, 1, 0x00, 0xf1, 0x53, 0x65];
    let mut directory = entry("dir/", false, b"");
    directory.extra = &extended_timestamp;
    let path = fixture.write("a.zip", &archive(&[directory, entry("b", false, b"x")]));
    let program = fixture.program();
    let zip = opened(&program, &path);

    let found = value(get_entry(&program, &zip, text("dir")));
    assert_eq!(
        accessor(&program, &found, "getName", "Ljava/lang/String;"),
        text("dir")
    );
    assert_eq!(
        accessor(&program, &found, "getTime", "J"),
        DexValue::Long(1_700_000_000_000)
    );
    assert_eq!(
        value(get_entry(&program, &zip, text("missing"))),
        DexValue::Null
    );
    assert_throws(
        &program,
        get_entry(&program, &zip, DexValue::Null),
        "Ljava/lang/NullPointerException;",
        Some("name"),
    );

    let entries = value(call(
        &program,
        ZIP_FILE,
        "entries",
        (&[], "Ljava/util/Enumeration;"),
        vec![zip.clone()],
    ));
    let more = || {
        int(call(
            &program,
            "Ljava/util/Enumeration;",
            "hasMoreElements",
            (&[], "Z"),
            vec![entries.clone()],
        ))
    };
    let next = || {
        call(
            &program,
            "Ljava/util/Enumeration;",
            "nextElement",
            (&[], "Ljava/lang/Object;"),
            vec![entries.clone()],
        )
    };
    for (name, is_directory) in [("dir/", 1), ("b", 0)] {
        assert_eq!(more(), 1);
        let listed = value(next());
        assert_eq!(
            accessor(&program, &listed, "getName", "Ljava/lang/String;"),
            text(name)
        );
        assert_eq!(
            accessor(&program, &listed, "isDirectory", "Z"),
            DexValue::Int(is_directory)
        );
    }
    assert_eq!(more(), 0);
    assert_throws(&program, next(), "Ljava/util/NoSuchElementException;", None);
}

#[test]
fn libcore_zip_file_close_invalidates_the_file_and_its_streams() {
    // cite: libcore ZipFile.java#close, ensureOpen, ensureOpenOrZipException
    // cite: libcore InflaterInputStream.java#ensureOpen
    let fixture = Fixture::new();
    let path = fixture.write(
        "a.zip",
        &archive(&[
            entry("stored", false, CHECK_INPUT),
            entry("deflated", true, CHECK_INPUT),
        ]),
    );
    let program = fixture.program();
    let zip = opened(&program, &path);
    let stored = stream(&program, &zip, "stored");
    let deflated = stream(&program, &zip, "deflated");
    assert_eq!(int(read_result(&program, &stored)), i32::from(b'1'));

    for _ in 0..2 {
        assert_eq!(
            call(&program, ZIP_FILE, "close", (&[], "V"), vec![zip.clone()]),
            Some(DexExecutionResult::Void)
        );
    }
    assert_throws(
        &program,
        read_result(&program, &stored),
        ZIP_EXCEPTION,
        Some("ZipFile closed"),
    );
    assert_throws(
        &program,
        read_result(&program, &deflated),
        IO_EXCEPTION,
        Some("Stream closed"),
    );
    assert_throws(
        &program,
        get_entry(&program, &zip, text("stored")),
        "Ljava/lang/IllegalStateException;",
        Some("zip file closed"),
    );
}

#[test]
fn libcore_zip_file_reports_corrupt_deflate_data_when_it_is_read() {
    // cite: libcore InflaterInputStream.java#read
    let fixture = Fixture::new();
    let mut bytes = archive(&[entry("deflated", true, CHECK_INPUT)]);
    bytes[30 + "deflated".len()] = 0x07;
    let path = fixture.write("a.zip", &bytes);
    let program = fixture.program();
    let zip = opened(&program, &path);
    let stream = stream(&program, &zip, "deflated");

    assert_throws(
        &program,
        read_result(&program, &stream),
        ZIP_EXCEPTION,
        Some("Invalid ZLIB data format"),
    );
}

#[test]
fn libcore_zip_file_open_failures_match_the_native_reader() {
    // cite: libcore ZipFile.c#ZipFile_open, throwFileNotFoundException
    // cite: libcore zip_util.c#readCEN, ZIP_Put_In_Cache0
    // cite: frameworks-base SafeZipPathValidatorCallback.java#onZipEntryAccess
    let fixture = Fixture::new();
    let empty = fixture.write("empty.zip", b"");
    let garbage = fixture.write("garbage.zip", b"not a zip archive at all");
    let duplicate = fixture.write(
        "duplicate.zip",
        &archive(&[entry("a", false, b"1"), entry("a", false, b"2")]),
    );
    let traversal = fixture.write("traversal.zip", &archive(&[entry("x/../y", false, b"")]));
    let host = fixture.root.join("apps/com.example/data/garbage.zip");
    let mut program = fixture.program();

    for path in ["/data/user/0/com.example/missing", host.to_str().unwrap()] {
        let exception = thrown_exception(open(&program, path).1);
        assert_eq!(
            value_class_descriptor(&exception),
            Some("Ljava/io/FileNotFoundException;")
        );
    }
    for (path, message) in [
        (&empty, "zip file is empty"),
        (&garbage, "error in opening zip file"),
        (&duplicate, "invalid CEN header (duplicate entry)"),
        (&traversal, "restricted zip entry name"),
    ] {
        assert_throws(
            &program,
            open(&program, path).1,
            ZIP_EXCEPTION,
            Some(message),
        );
    }
    let zip = program.allocate_object(ZIP_FILE.to_owned());
    assert_throws(
        &program,
        call(
            &program,
            ZIP_FILE,
            "<init>",
            (&["Ljava/io/File;", "I"], "V"),
            vec![zip, DexValue::Null, DexValue::Int(4)],
        ),
        "Ljava/lang/IllegalArgumentException;",
        Some("Illegal mode: 0x4"),
    );

    program.target_sdk_version = 33;
    opened(&program, &traversal);
}

#[test]
fn libcore_zip_file_open_delete_removes_app_data_but_never_the_installed_apk() {
    // cite: libcore ZipFile.java#close, OPEN_DELETE
    // cite: frameworks-base ApplicationInfo.java#sourceDir
    let fixture = Fixture::new();
    let bytes = archive(&[entry("classes.dex", false, CHECK_INPUT)]);
    let apk = fixture.root.join("apps/com.example/source.apk");
    fs::write(&apk, &bytes).expect("installed apk");
    let private = fixture.write("a.zip", &bytes);
    let program = fixture.program();

    for path in [crate::guest_app_paths::base_apk("com.example"), private] {
        let file = DexValue::Reference(std::rc::Rc::from(format!("Ljava/io/File;:{path}")));
        let zip = program.allocate_object(ZIP_FILE.to_owned());
        assert_eq!(
            call(
                &program,
                ZIP_FILE,
                "<init>",
                (&["Ljava/io/File;", "I"], "V"),
                vec![zip.clone(), file, DexValue::Int(5)],
            ),
            Some(DexExecutionResult::Void)
        );
        let stream = stream(&program, &zip, "classes.dex");
        assert_eq!(drain(&program, INPUT, &stream), CHECK_INPUT);
        call(&program, ZIP_FILE, "close", (&[], "V"), vec![zip]);
    }

    assert_eq!(fs::read(&apk).expect("installed apk"), bytes);
    assert!(!fixture.root.join("apps/com.example/data/a.zip").exists());
}

#[test]
fn libcore_checked_input_stream_dispatches_to_guest_stream_and_checksum_overrides() {
    // cite: libcore CheckedInputStream.java#read, getChecksum
    // cite: libcore FilterInputStream.java#read, close
    let fixture = Fixture::new();
    let path = fixture.write("digits", CHECK_INPUT);
    let dex_file = crate::dex::parse_dex_file(
        "zip_stream_contract.dex",
        include_bytes!("fixtures/zip_stream_contract.dex").to_vec(),
    )
    .expect("zip stream contract DEX should parse");
    let program = DexProgram::with_guest_filesystem(
        vec![dex_file],
        "com.example",
        fixture.root.join("apps/com.example/data"),
        fixture.root.join("storage"),
    )
    .expect("guest filesystem program");

    for (name, recorded_jvm_result) in [
        ("checksumThroughOverriddenStream", 3_421_780_262_106),
        ("guestChecksum", 477),
    ] {
        let method = builtin_method(
            "Lwinedroid/tests/ZipStreamContract;",
            name,
            "J",
            &["Ljava/lang/String;"],
        );
        let resolved = program
            .resolve_exact_method(&method)
            .expect("contract method should resolve")
            .expect("contract method should exist");

        assert_eq!(
            program
                .execute_method(None, &resolved, Some(&[text(&path)]), 0)
                .expect("contract method should execute"),
            DexExecutionResult::Value(DexValue::Long(recorded_jvm_result)),
            "{name}"
        );
    }
}
