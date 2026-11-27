use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use super::super::*;
use super::helpers::*;

const CONTRACT: &str = "Lwinedroid/tests/FileObserverContract;";
const WATCHED: &str = "/data/user/0/com.example/files/watched";
const CREATE_CLOSE_WRITE: i32 = 0x108;

struct ObserverFixture {
    root: PathBuf,
    program: DexProgram,
}

impl ObserverFixture {
    fn new(name: &str) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        let root = std::env::temp_dir().join(format!(
            "winedroid-file-observer-{name}-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let private = root.join("apps/com.example/data");
        fs::create_dir_all(private.join("files/watched")).unwrap();
        fs::create_dir_all(root.join("storage")).unwrap();
        let dex_file = crate::dex::parse_dex_file(
            "file_observer_contract.dex",
            include_bytes!("fixtures/file_observer_contract.dex").to_vec(),
        )
        .expect("FileObserver contract DEX should parse");
        let program = DexProgram::with_guest_filesystem(
            vec![dex_file],
            "com.example",
            private,
            root.join("storage"),
        )
        .expect("guest filesystem program should initialize");
        Self { root, program }
    }

    fn host(&self, relative: &str) -> PathBuf {
        self.root
            .join("apps/com.example/data/files/watched")
            .join(relative)
    }

    fn call(
        &self,
        name: &str,
        return_type: &str,
        parameters: &[&str],
        arguments: Vec<DexValue>,
    ) -> DexExecutionResult {
        let method = DexMethodId {
            class_descriptor: CONTRACT.to_owned(),
            name: name.to_owned(),
            proto: crate::dex::DexProto {
                shorty: String::new(),
                return_type: return_type.to_owned(),
                parameters: parameters
                    .iter()
                    .map(|parameter| (*parameter).to_owned())
                    .collect(),
            },
        };
        let resolved = self
            .program
            .resolve_exact_method_in_class_hierarchy(CONTRACT, &method)
            .unwrap()
            .unwrap_or_else(|| panic!("{name} should exist in the contract"));
        self.program
            .execute_method(None, &resolved, Some(&arguments), 0)
            .unwrap_or_else(|error| panic!("{name} should execute: {error}"))
    }

    fn watch(&self, path: &str, mask: i32, throws: i32) -> DexValue {
        match self.call(
            "watch",
            CONTRACT,
            &["Ljava/lang/String;", "I", "I"],
            vec![string(path), DexValue::Int(mask), DexValue::Int(throws)],
        ) {
            DexExecutionResult::Value(observer) => observer,
            other => panic!("watch should return the observer: {other:?}"),
        }
    }

    fn write_stream(&self, path: &str) {
        assert_eq!(
            self.call(
                "writeStream",
                "V",
                &["Ljava/lang/String;"],
                vec![string(path)]
            ),
            DexExecutionResult::Void
        );
    }

    fn events(&self, observer: &DexValue) -> String {
        match self.call("events", "Ljava/lang/String;", &[], vec![observer.clone()]) {
            DexExecutionResult::Value(DexValue::String(text)) => text.to_string(),
            other => panic!("events should return a String: {other:?}"),
        }
    }

    fn stop(&self, observer: &DexValue) {
        let stop = DexMethodId {
            class_descriptor: "Landroid/os/FileObserver;".to_owned(),
            name: "stopWatching".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "V".to_owned(),
                return_type: "V".to_owned(),
                parameters: Vec::new(),
            },
        };
        assert_eq!(
            self.program
                .invoke_builtin_method(&stop, Some(&test_invocation(vec![observer.clone()]))),
            Some(DexExecutionResult::Void)
        );
    }

    fn pump(&self) {
        self.program
            .dispatch_pending_main_loop(320, 480, 0)
            .expect("main loop should dispatch FileObserver events");
    }
}

impl Drop for ObserverFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn string(text: &str) -> DexValue {
    DexValue::String(std::rc::Rc::from(text))
}

#[test]
fn file_observer_thread_receives_create_then_close_write_from_the_main_loop() {
    let fixture = ObserverFixture::new("main_loop");
    let observer = fixture.watch(WATCHED, CREATE_CLOSE_WRITE, 0);
    fixture.write_stream(&format!("{WATCHED}/x"));

    assert_eq!(
        fixture.events(&observer),
        "",
        "FileObserver.java ObserverThread delivers on its own thread, never inside the writer's call"
    );
    assert_eq!(
        fixture.program.runtime.borrow().next_wake_delay_millis(),
        Some(0),
        "queued inotify events wake the loop like any runnable work"
    );
    fixture.pump();
    assert_eq!(
        fixture.events(&observer),
        "256,x,FileObserver;8,x,FileObserver;",
        "android_util_FileObserver.cpp android_os_fileobserver_observe passes the raw mask and name; ObserverThread is super(\"FileObserver\")"
    );
    assert_eq!(
        fixture.program.runtime.borrow().next_wake_delay_millis(),
        None
    );
}

#[test]
fn file_observer_stop_watching_delivers_in_ignored_with_a_null_path() {
    let fixture = ObserverFixture::new("stop");
    let observer = fixture.watch(WATCHED, CREATE_CLOSE_WRITE, 0);
    fixture.stop(&observer);
    fixture.pump();
    assert_eq!(
        fixture.events(&observer),
        "32768,null,FileObserver;",
        "FileObserver.java stopWatching keeps mRealObservers, and inotify.h IN_IGNORED has no name"
    );
    fixture.write_stream(&format!("{WATCHED}/after"));
    fixture.pump();
    assert_eq!(
        fixture.events(&observer),
        "32768,null,FileObserver;",
        "android_os_fileobserver_stopWatching removed the inotify watch"
    );
}

#[test]
fn file_observer_second_observer_on_the_same_directory_replaces_the_first() {
    let fixture = ObserverFixture::new("shared_wd");
    let first = fixture.watch(WATCHED, CREATE_CLOSE_WRITE, 0);
    let second = fixture.watch(WATCHED, CREATE_CLOSE_WRITE, 0);
    fixture.write_stream(&format!("{WATCHED}/s"));
    fixture.pump();
    assert_eq!(
        fixture.events(&first),
        "",
        "FileObserver.java ObserverThread.startWatching mRealObservers.put overwrites the shared wd"
    );
    assert_eq!(
        fixture.events(&second),
        "256,s,FileObserver;8,s,FileObserver;"
    );

    fixture.stop(&first);
    fixture.write_stream(&format!("{WATCHED}/t"));
    fixture.pump();
    assert_eq!(fixture.events(&first), "");
    assert_eq!(
        fixture.events(&second),
        "256,s,FileObserver;8,s,FileObserver;32768,null,FileObserver;",
        "stopWatching on the shared wd ends the watch for both observers"
    );
}

#[test]
fn file_observer_on_event_is_never_reentered_by_a_wait_inside_it() {
    let fixture = ObserverFixture::new("reentrant");
    let observer = match fixture.call(
        "watchReentrant",
        CONTRACT,
        &["Ljava/lang/String;", "I", "Ljava/lang/String;"],
        vec![
            string(WATCHED),
            DexValue::Int(CREATE_CLOSE_WRITE),
            string(&format!("{WATCHED}/b")),
        ],
    ) {
        DexExecutionResult::Value(observer) => observer,
        other => panic!("watchReentrant should return the observer: {other:?}"),
    };
    fixture.write_stream(&format!("{WATCHED}/a"));
    fixture.pump();
    assert_eq!(
        fixture.events(&observer),
        "256,a,FileObserver;8,a,FileObserver;256,b,FileObserver;8,b,FileObserver;",
        "FileObserver.java ObserverThread runs one onEvent at a time on its own thread"
    );
    assert_eq!(
        fixture.call("maxDepth", "I", &[], vec![observer]),
        DexExecutionResult::Int(1)
    );
}

#[test]
fn file_observer_throwable_from_on_event_is_logged_and_delivery_continues() {
    let fixture = ObserverFixture::new("throws");
    let observer = fixture.watch(WATCHED, CREATE_CLOSE_WRITE, 1);
    fixture.write_stream(&format!("{WATCHED}/e"));
    fixture.pump();
    assert_eq!(
        fixture.events(&observer),
        "256,e,FileObserver;8,e,FileObserver;",
        "FileObserver.java ObserverThread.onEvent catches Throwable and calls Log.wtf"
    );
}

#[test]
fn file_observer_events_from_another_host_thread_arrive_at_the_next_main_loop_pass() {
    let fixture = ObserverFixture::new("host_thread");
    let observer = fixture.watch(WATCHED, CREATE_CLOSE_WRITE, 0);
    let directory = crate::capability_fs::Directory::open_ambient(&fixture.host("")).unwrap();
    std::thread::spawn(move || {
        drop(
            directory
                .create_new(&[std::ffi::CString::new("y").unwrap()])
                .unwrap(),
        );
    })
    .join()
    .unwrap();
    fixture.pump();
    assert_eq!(
        fixture.events(&observer),
        "256,y,FileObserver;8,y,FileObserver;",
        "FileObserver.java ObserverThread reads one inotify fd for every producing thread"
    );
}

#[test]
fn file_observer_wakes_a_guest_blocked_in_object_wait() {
    let fixture = ObserverFixture::new("wait");
    let observer = fixture.watch(WATCHED, CREATE_CLOSE_WRITE, 0);
    assert_eq!(
        fixture.call(
            "writeAndAwait",
            "I",
            &[CONTRACT, "Ljava/lang/String;"],
            vec![observer.clone(), string(&format!("{WATCHED}/w"))],
        ),
        DexExecutionResult::Int(2),
        "FileObserver.java ObserverThread runs concurrently with a thread waiting on a monitor"
    );
    assert_eq!(
        fixture.events(&observer),
        "256,w,FileObserver;8,w,FileObserver;"
    );
}

#[test]
fn file_observer_file_writer_reports_close_write_once_at_close() {
    let fixture = ObserverFixture::new("file_writer");
    let observer = fixture.watch(WATCHED, CREATE_CLOSE_WRITE, 0);
    let writer = match fixture.call(
        "openWriter",
        "Ljava/io/FileWriter;",
        &["Ljava/lang/String;"],
        vec![string(&format!("{WATCHED}/fw"))],
    ) {
        DexExecutionResult::Value(writer) => writer,
        other => panic!("openWriter should return a FileWriter: {other:?}"),
    };
    fixture.pump();
    assert_eq!(
        fixture.events(&observer),
        "256,fw,FileObserver;",
        "FileWriter.java opens a FileOutputStream at construction and holds it"
    );
    assert_eq!(
        fixture.call(
            "writeAndClose",
            "V",
            &["Ljava/io/FileWriter;"],
            vec![writer.clone()]
        ),
        DexExecutionResult::Void
    );
    assert_eq!(
        fixture.call(
            "writeAfterClose",
            "I",
            &["Ljava/io/FileWriter;"],
            vec![writer]
        ),
        DexExecutionResult::Int(1),
        "StreamEncoder.java ensureOpen throws IOException Stream closed"
    );
    fixture.pump();
    assert_eq!(
        fixture.events(&observer),
        "256,fw,FileObserver;8,fw,FileObserver;",
        "BufferedWriter.java close closes the FileWriter once, after both writes"
    );
    assert_eq!(fs::read(fixture.host("fw")).unwrap(), b"ab");
}

#[test]
fn file_observer_random_access_file_reports_close_write_at_close() {
    let fixture = ObserverFixture::new("random_access");
    let observer = fixture.watch(WATCHED, CREATE_CLOSE_WRITE, 0);
    let file = match fixture.call(
        "openRandomAccess",
        "Ljava/io/RandomAccessFile;",
        &["Ljava/lang/String;"],
        vec![string(&format!("{WATCHED}/raf"))],
    ) {
        DexExecutionResult::Value(file) => file,
        other => panic!("openRandomAccess should return the file: {other:?}"),
    };
    fixture.pump();
    assert_eq!(
        fixture.events(&observer),
        "256,raf,FileObserver;",
        "RandomAccessFile.java opens the descriptor in the constructor"
    );
    assert_eq!(
        fixture.call(
            "closeRandomAccess",
            "V",
            &["Ljava/io/RandomAccessFile;"],
            vec![file]
        ),
        DexExecutionResult::Void
    );
    fixture.pump();
    assert_eq!(
        fixture.events(&observer),
        "256,raf,FileObserver;8,raf,FileObserver;",
        "RandomAccessFile.java close releases the writable descriptor"
    );
}

#[test]
fn file_observer_watch_through_the_data_data_alias_sees_writes_through_data_user_0() {
    let fixture = ObserverFixture::new("alias");
    let observer = fixture.watch(
        "/data/data/com.example/files/watched",
        CREATE_CLOSE_WRITE,
        0,
    );
    fixture.write_stream(&format!("{WATCHED}/z"));
    fixture.pump();
    assert_eq!(
        fixture.events(&observer),
        "256,z,FileObserver;8,z,FileObserver;",
        "StrictMode.java onCredentialProtectedPathAccess: /data/data is /data/user/0, and inotify watches the inode"
    );
}

#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
#[test]
fn file_observer_receives_a_native_open_write_close_in_the_watched_directory() {
    use crate::native::bionic::linux_abi::{at, open};
    let fixture = ObserverFixture::new("native");
    let access = crate::external_storage::ExternalStorageRuntimeAccess::with_legacy_file_access(
        Vec::new(),
        true,
        true,
        true,
    );
    let policy = std::sync::Arc::new(std::sync::RwLock::new(ExternalStoragePolicy::new(
        28,
        28,
        Vec::new(),
        access,
    )));
    let context =
        crate::native::guest_context::GuestContext::open(&fixture.root, "com.example", policy)
            .unwrap();
    crate::native::guest_context::use_on_this_thread(std::sync::Arc::new(context));
    let observer = fixture.watch(WATCHED, CREATE_CLOSE_WRITE, 0);

    let fd = crate::native::bionic::fs_open::open_at(
        at::FDCWD,
        format!("{WATCHED}/native").as_bytes(),
        open::WRONLY | open::CREAT,
        0o600,
    );
    assert!(fd >= 0, "native open failed: {fd}");
    assert_eq!(
        unsafe { libc::write(fd as i32, b"n".as_ptr().cast(), 1) },
        1
    );
    assert_eq!(crate::native::bionic::fs_close_dup::close_fd(fd as i32), 0);
    fixture.pump();
    assert_eq!(
        fixture.events(&observer),
        "256,native,FileObserver;8,native,FileObserver;",
        "inotify(7) events come from the inode whichever layer wrote it"
    );
}

fn boolean_call(fixture: &ObserverFixture, name: &str, paths: &[&str]) -> DexExecutionResult {
    let parameters = vec!["Ljava/lang/String;"; paths.len()];
    fixture.call(
        name,
        "Z",
        &parameters,
        paths.iter().map(|path| string(path)).collect(),
    )
}

#[test]
fn file_observer_on_a_file_gets_close_write_and_delete_self_with_a_null_path() {
    let fixture = ObserverFixture::new("file_watch");
    let state = format!("{WATCHED}/networkStatef");
    let observer = match fixture.call(
        "createAndWatch",
        CONTRACT,
        &["Ljava/lang/String;", "I"],
        vec![string(&state), DexValue::Int(0x408)],
    ) {
        DexExecutionResult::Value(observer) => observer,
        other => panic!("createAndWatch should return the observer: {other:?}"),
    };
    fixture.write_stream(&state);
    fixture.pump();
    assert_eq!(
        fixture.events(&observer),
        "8,null,FileObserver;",
        "android_util_FileObserver.cpp passes a null path when inotify event len is 0, as for an event on the watched file itself"
    );

    let input = match fixture.call(
        "openInput",
        "Ljava/io/FileInputStream;",
        &["Ljava/lang/String;"],
        vec![string(&state)],
    ) {
        DexExecutionResult::Value(input) => input,
        other => panic!("openInput should return the stream: {other:?}"),
    };
    assert_eq!(
        boolean_call(&fixture, "delete", &[&state]),
        DexExecutionResult::Int(1)
    );
    fixture.pump();
    assert_eq!(
        fixture.events(&observer),
        "8,null,FileObserver;",
        "unlink(2): an open FileInputStream keeps the deleted inode alive"
    );
    assert_eq!(
        fixture.call(
            "closeInput",
            "V",
            &["Ljava/io/FileInputStream;"],
            vec![input]
        ),
        DexExecutionResult::Void
    );
    fixture.pump();
    assert_eq!(
        fixture.events(&observer),
        "8,null,FileObserver;1024,null,FileObserver;32768,null,FileObserver;",
        "inotify.h IN_DELETE_SELF then IN_IGNORED once FileInputStream.close releases the last reference"
    );
}

#[test]
fn file_observer_on_a_file_replaced_by_file_rename_to_gets_delete_self_then_ignored() {
    let fixture = ObserverFixture::new("file_rename_over");
    let state = format!("{WATCHED}/networkStatef");
    let observer = match fixture.call(
        "createAndWatch",
        CONTRACT,
        &["Ljava/lang/String;", "I"],
        vec![string(&state), DexValue::Int(0x408)],
    ) {
        DexExecutionResult::Value(observer) => observer,
        other => panic!("createAndWatch should return the observer: {other:?}"),
    };
    fs::write(fixture.host("replacement"), b"r").unwrap();
    assert_eq!(
        boolean_call(
            &fixture,
            "renameOver",
            &[&format!("{WATCHED}/replacement"), &state]
        ),
        DexExecutionResult::Int(1)
    );
    fixture.pump();
    assert_eq!(
        fixture.events(&observer),
        "1024,null,FileObserver;32768,null,FileObserver;",
        "File.java renameTo is rename(2), which deletes the replaced inode"
    );
    assert_eq!(fs::read(fixture.host("networkStatef")).unwrap(), b"r");
}

#[test]
fn file_observer_directory_watch_with_delete_self_gets_it_before_ignored() {
    let fixture = ObserverFixture::new("dir_delete_self");
    let observer = fixture.watch(WATCHED, 0x500, 0);
    assert_eq!(
        boolean_call(&fixture, "delete", &[WATCHED]),
        DexExecutionResult::Int(1)
    );
    fixture.pump();
    assert_eq!(
        fixture.events(&observer),
        "1073742848,null,FileObserver;32768,null,FileObserver;",
        "inotify.h IN_DELETE_SELF|IN_ISDIR, then IN_IGNORED, for the deleted watched directory"
    );
}

#[test]
fn file_observer_delete_self_waits_for_buffered_reader_and_random_access_file_close() {
    let fixture = ObserverFixture::new("reader_holds");
    let state = format!("{WATCHED}/held");
    let observer = match fixture.call(
        "createAndWatch",
        CONTRACT,
        &["Ljava/lang/String;", "I"],
        vec![string(&state), DexValue::Int(0x408)],
    ) {
        DexExecutionResult::Value(observer) => observer,
        other => panic!("createAndWatch should return the observer: {other:?}"),
    };
    let reader = match fixture.call(
        "openReader",
        "Ljava/io/BufferedReader;",
        &["Ljava/lang/String;"],
        vec![string(&state)],
    ) {
        DexExecutionResult::Value(reader) => reader,
        other => panic!("openReader should return the reader: {other:?}"),
    };
    let file = match fixture.call(
        "openRandomAccessRead",
        "Ljava/io/RandomAccessFile;",
        &["Ljava/lang/String;"],
        vec![string(&state)],
    ) {
        DexExecutionResult::Value(file) => file,
        other => panic!("openRandomAccessRead should return the file: {other:?}"),
    };
    assert_eq!(
        boolean_call(&fixture, "delete", &[&state]),
        DexExecutionResult::Int(1)
    );
    assert_eq!(
        fixture.call(
            "closeReader",
            "V",
            &["Ljava/io/BufferedReader;"],
            vec![reader]
        ),
        DexExecutionResult::Void
    );
    fixture.pump();
    assert_eq!(
        fixture.events(&observer),
        "",
        "RandomAccessFile.java keeps its descriptor open until close, so the deleted inode lives on"
    );
    assert_eq!(
        fixture.call(
            "closeRandomAccess",
            "V",
            &["Ljava/io/RandomAccessFile;"],
            vec![file]
        ),
        DexExecutionResult::Void
    );
    fixture.pump();
    assert_eq!(
        fixture.events(&observer),
        "1024,null,FileObserver;32768,null,FileObserver;",
        "BufferedReader.java close closes the FileReader, and RandomAccessFile.java close releases the last descriptor"
    );
}

#[test]
fn file_observer_delete_self_waits_for_a_buffered_reader_over_a_file_reader() {
    let fixture = ObserverFixture::new("reader_chain");
    let state = format!("{WATCHED}/read");
    let observer = match fixture.call(
        "createAndWatch",
        CONTRACT,
        &["Ljava/lang/String;", "I"],
        vec![string(&state), DexValue::Int(0x408)],
    ) {
        DexExecutionResult::Value(observer) => observer,
        other => panic!("createAndWatch should return the observer: {other:?}"),
    };
    let reader = match fixture.call(
        "openReader",
        "Ljava/io/BufferedReader;",
        &["Ljava/lang/String;"],
        vec![string(&state)],
    ) {
        DexExecutionResult::Value(reader) => reader,
        other => panic!("openReader should return the reader: {other:?}"),
    };
    assert_eq!(
        boolean_call(&fixture, "delete", &[&state]),
        DexExecutionResult::Int(1)
    );
    fixture.pump();
    assert_eq!(
        fixture.events(&observer),
        "",
        "FileReader.java opens a FileInputStream that stays open until close"
    );
    assert_eq!(
        fixture.call(
            "closeReader",
            "V",
            &["Ljava/io/BufferedReader;"],
            vec![reader]
        ),
        DexExecutionResult::Void
    );
    fixture.pump();
    assert_eq!(
        fixture.events(&observer),
        "1024,null,FileObserver;32768,null,FileObserver;",
        "BufferedReader.java close reaches the FileReader and releases the deleted inode"
    );
}
