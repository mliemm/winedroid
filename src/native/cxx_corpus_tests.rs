use super::bionic::guest_mem::read_value;
use super::bridge_test_support::register_all;
use super::call_assign::CallBuilder;
use super::dl_api::DlHandle;
use super::loader_test_support::{corpus_dir, dlopen_without_constructors, fixture_dir};
use super::runtime::NativeRuntime;

fn corpus_library(name: &str) -> Option<std::path::PathBuf> {
    let path = corpus_dir().join(name);
    if path.is_file() {
        return Some(path);
    }
    eprintln!("skipped: {name} is absent at {}", path.display());
    None
}

fn runtime() -> &'static NativeRuntime {
    register_all();
    NativeRuntime::initialize(vec![fixture_dir("16k")])
        .expect("the native runtime is initialized with the 16k fixture directory")
}

#[test]
fn measured_libc_plus_plus_shared_resolves_every_import_this_milestone_owns() {
    let Some(shared) = corpus_library("libc++_shared.so") else {
        return;
    };
    let loader = runtime().loader();
    dlopen_without_constructors(loader, &shared.to_string_lossy())
        .unwrap_or_else(|error| panic!("libc++_shared.so: {error}"));

    let unresolved = loader.report().unresolved_imports;
    for name in [
        "strtold",
        "strtold_l",
        "wcstold",
        "syslog",
        "openlog",
        "closelog",
    ] {
        assert!(
            !unresolved.iter().any(|(_, imported)| imported == name),
            "{name} must resolve against the shim"
        );
    }
}

#[test]
fn measured_libc_plus_plus_shared_throws_and_catches_through_its_own_unwinder_on_the_shim_dl_iterate_phdr(
) {
    let Some(shared) = corpus_library("libc++_shared.so") else {
        return;
    };
    let loader = runtime().loader();
    dlopen_without_constructors(loader, &shared.to_string_lossy())
        .unwrap_or_else(|error| panic!("libc++_shared.so: {error}"));
    let handle = loader
        .dlopen(Some("libfx_cxx.so"), 2, None)
        .unwrap_or_else(|error| panic!("libfx_cxx.so: {error}"));
    let throw_and_catch = loader
        .dlsym(DlHandle::Module(handle), "fx_cxx_throw_and_catch", None)
        .expect("the fixture exports the throwing entry point");
    let plain = loader
        .dlsym(DlHandle::Module(handle), "fx_cxx_plain_add", None)
        .expect("the fixture exports a plain entry point");

    assert_eq!(
        CallBuilder::new(plain)
            .i32(20)
            .i32(22)
            .invoke()
            .unwrap()
            .i32(),
        42
    );
    assert_eq!(
        CallBuilder::new(throw_and_catch)
            .i32(41)
            .invoke()
            .unwrap()
            .i32(),
        42,
        "the guest libunwind walked two fixture frames and matched the fixture type info"
    );
}

fn capture_stdout(work: impl FnOnce()) -> String {
    let saved = unsafe { libc::dup(1) };
    let mut ends = [0i32; 2];
    assert_eq!(unsafe { libc::pipe(ends.as_mut_ptr()) }, 0);
    assert_eq!(unsafe { libc::dup2(ends[1], 1) }, 1);
    work();
    assert_eq!(unsafe { libc::dup2(saved, 1) }, 1);
    unsafe { libc::close(saved) };
    unsafe { libc::close(ends[1]) };
    let mut text = Vec::new();
    let mut chunk = [0u8; 1024];
    loop {
        let count = unsafe { libc::read(ends[0], chunk.as_mut_ptr() as *mut _, chunk.len()) };
        if count <= 0 {
            break;
        }
        text.extend_from_slice(&chunk[..count as usize]);
    }
    unsafe { libc::close(ends[0]) };
    String::from_utf8_lossy(&text).into_owned()
}

#[test]
fn measured_libc_plus_plus_shared_constructors_initialize_cout_and_insert_an_integer() {
    let Some(shared) = corpus_library("libc++_shared.so") else {
        return;
    };
    let loader = runtime().loader();
    let handle = loader
        .dlopen(Some(&shared.to_string_lossy()), 2, None)
        .unwrap_or_else(|error| panic!("libc++_shared.so: {error}"));
    let cout = loader
        .dlsym(DlHandle::Module(handle), "_ZNSt6__ndk14coutE", None)
        .expect("libc++ exports cout");
    let insert = loader
        .dlsym(
            DlHandle::Module(handle),
            "_ZNSt6__ndk113basic_ostreamIcNS_11char_traitsIcEEElsEi",
            None,
        )
        .expect("libc++ exports operator<<");
    let newline = loader
        .dlsym(
            DlHandle::Module(handle),
            "_ZSt4endlRNSt6__ndk113basic_ostreamIcNS_11char_traitsIcEEE",
            None,
        )
        .unwrap_or(0);
    let flush = loader
        .dlsym(
            DlHandle::Module(handle),
            "_ZNSt6__ndk113basic_ostreamIcNS_11char_traitsIcEE5flushEv",
            None,
        )
        .expect("libc++ exports flush");
    let object = read_value::<usize>(cout);

    let printed = capture_stdout(|| {
        let result = CallBuilder::new(insert)
            .ptr(object)
            .i32(42)
            .invoke()
            .unwrap();
        assert_eq!(
            result.u64() as usize,
            object,
            "operator<< returns the stream"
        );
        if newline != 0 {
            let _ = CallBuilder::new(newline).ptr(object).invoke();
        }
        let _ = CallBuilder::new(flush).ptr(object).invoke();
    });

    assert!(
        printed.contains("42"),
        "the guest stdout carried {printed:?}"
    );
}
