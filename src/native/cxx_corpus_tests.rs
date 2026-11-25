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
    NativeRuntime::initialize(vec![fixture_dir("16k")]).expect("the native runtime is initialized with the 16k fixture directory")
}

#[test]
fn measured_libc_plus_plus_shared_resolves_every_import_this_milestone_owns() {
    let Some(shared) = corpus_library("libc++_shared.so") else { return };
    let loader = runtime().loader();
    dlopen_without_constructors(loader, &shared.to_string_lossy()).unwrap_or_else(|error| panic!("libc++_shared.so: {error}"));

    let unresolved = loader.report().unresolved_imports;
    for name in ["strtold", "strtold_l", "wcstold", "syslog", "openlog", "closelog"] {
        assert!(!unresolved.iter().any(|(_, imported)| imported == name), "{name} must resolve against the shim");
    }
}

#[test]
fn measured_libc_plus_plus_shared_throws_and_catches_through_its_own_unwinder_on_the_shim_dl_iterate_phdr() {
    let Some(shared) = corpus_library("libc++_shared.so") else { return };
    let loader = runtime().loader();
    dlopen_without_constructors(loader, &shared.to_string_lossy()).unwrap_or_else(|error| panic!("libc++_shared.so: {error}"));
    let handle = loader.dlopen(Some("libfx_cxx.so"), 2, None).unwrap_or_else(|error| panic!("libfx_cxx.so: {error}"));
    let throw_and_catch = loader.dlsym(DlHandle::Module(handle), "fx_cxx_throw_and_catch", None).expect("the fixture exports the throwing entry point");
    let plain = loader.dlsym(DlHandle::Module(handle), "fx_cxx_plain_add", None).expect("the fixture exports a plain entry point");

    assert_eq!(CallBuilder::new(plain).i32(20).i32(22).invoke().unwrap().i32(), 42);
    assert_eq!(
        CallBuilder::new(throw_and_catch).i32(41).invoke().unwrap().i32(),
        42,
        "the guest libunwind walked two fixture frames and matched the fixture type info"
    );
}
