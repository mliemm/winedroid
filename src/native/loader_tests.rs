use std::ffi::CString;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::Ordering;

use super::dl_api::{dlerror, DlHandle};
use super::loader_test_support::{fixture_dir, host_function_address, loader_for, ATEXIT, MUTEX_CALLS};

const VARIANTS: [&str; 2] = ["16k", "4k"];
const RELOCATING: [&str; 3] = ["libfx_reloc.so", "libfx_aps2.so", "libfx_relr.so"];

fn symbol(loader: &super::loader::Loader, handle: u64, name: &str) -> usize {
    loader.dlsym(DlHandle::Module(handle), name, None).unwrap_or_else(|error| panic!("{name}: {error}"))
}

#[test]
fn fixture_source_fx_add_loads_and_adds_natively_in_both_page_variants() {
    for variant in VARIANTS {
        let loader = loader_for(vec![fixture_dir(variant)]);
        let handle = loader.dlopen(Some("libfx_add.so"), 2, None).unwrap();
        let add: extern "C" fn(i32, i32) -> i32 = unsafe { std::mem::transmute(symbol(&loader, handle, "add")) };
        let value = unsafe { *(symbol(&loader, handle, "fx_data_value") as *const i32) };

        assert_eq!(add(2, 3), 5, "{variant}");
        assert_eq!(add(-7, 3), -4, "{variant}");
        assert_eq!(value, 0x1234, "{variant}");
        assert_eq!(handle & 1, 1);
    }
}

#[test]
fn fixture_source_fx_reloc_variants_run_constructors_and_resolve_every_relocation_kind() {
    for variant in VARIANTS {
        for library in RELOCATING {
            let loader = loader_for(vec![fixture_dir(variant)]);
            ATEXIT.lock().unwrap().clear();
            let handle = loader.dlopen(Some(library), 2, None).unwrap();
            let label = format!("{variant}/{library}");
            let call_int = |name: &str| -> i32 {
                let function: extern "C" fn() -> i32 = unsafe { std::mem::transmute(symbol(&loader, handle, name)) };
                function()
            };

            assert_eq!(call_int("fx_ctor_marker"), 0xC0DE, "{label} init_array ran");
            let exported_value = symbol(&loader, handle, "fx_exported_value");
            let exported_pointer = unsafe { *(symbol(&loader, handle, "fx_exported_pointer") as *const usize) };
            assert_eq!(exported_pointer, exported_value, "{label} GLOB_DAT or ABS64");
            let local_pointer = unsafe { *(symbol(&loader, handle, "fx_local_pointer") as *const *const i32) };
            assert_eq!(unsafe { *local_pointer }, 11, "{label} RELATIVE");
            let table = symbol(&loader, handle, "fx_import_table") as *const usize;
            let strlen_address = host_function_address("strlen");
            assert_eq!(unsafe { *table }, strlen_address, "{label} import table entry 0");
            assert_eq!(unsafe { *table.add(1) }, host_function_address("memcpy"), "{label} import table entry 1");
            let strlen_through_guest: extern "C" fn() -> usize = unsafe { std::mem::transmute(symbol(&loader, handle, "fx_strlen_address")) };
            assert_eq!(strlen_through_guest(), strlen_address, "{label} function pointer equality");
            let text = CString::new("abcd").unwrap();
            let fx_strlen: extern "C" fn(*const std::ffi::c_char) -> usize = unsafe { std::mem::transmute(symbol(&loader, handle, "fx_strlen")) };
            assert_eq!(fx_strlen(text.as_ptr()), 4, "{label} JUMP_SLOT");
            let copy_sum: extern "C" fn(*const u8, usize) -> i32 = unsafe { std::mem::transmute(symbol(&loader, handle, "fx_copy_sum")) };
            assert_eq!(copy_sum([1u8, 2, 3, 4].as_ptr(), 4), 10, "{label} malloc memcpy free");
            let before = MUTEX_CALLS.load(Ordering::SeqCst);
            assert_eq!(call_int("fx_lock_roundtrip"), 0);
            assert_eq!(MUTEX_CALLS.load(Ordering::SeqCst) - before, 2, "{label} mutex imports");

            let registered = ATEXIT.lock().unwrap().clone();
            assert_eq!(registered.len(), 1, "{label} __cxa_atexit registrations");
            assert_eq!(registered[0].1, exported_value, "{label} atexit argument");
            let handler: extern "C" fn(usize) = unsafe { std::mem::transmute(registered[0].0) };
            handler(registered[0].1);
            assert_eq!(call_int("fx_atexit_marker"), 7, "{label} atexit handler");

            let info = loader.module_info(handle).unwrap();
            let tls = info.tls.expect("PT_TLS recorded");
            assert_eq!((tls.size, tls.init_size, tls.alignment), (8, 4, 4), "{label} TLS template");
            assert_ne!(info.tls_module, 0);
            assert_eq!(info.shift, if variant == "4k" { 0x2000 } else { 0 }, "{label} ShiftRw");
            assert_eq!(info.load_bias % 4096, 0);
            let imports = loader.report().unresolved_imports;
            for name in ["snprintf", "__android_log_print"] {
                assert!(imports.iter().any(|(_, import)| import == name), "{label} unresolved {name}");
            }
        }
    }
}

#[test]
fn bionic_soinfo_unload_runs_fini_array_when_the_last_reference_is_closed() {
    let loader = loader_for(vec![fixture_dir("16k")]);
    let handle = loader.dlopen(Some("libfx_reloc.so"), 2, None).unwrap();
    assert_eq!(loader.dlopen(Some("libfx_reloc.so"), 2, None).unwrap(), handle);
    assert_eq!(loader.module_info(handle).unwrap().ref_count, 2);
    let marker = symbol(&loader, handle, "fx_fini_marker") as *const i32;
    let count = loader.loaded_module_count();

    loader.dlclose(handle).unwrap();
    assert_eq!(unsafe { *marker }, 0, "one reference left");
    assert_eq!(loader.loaded_module_count(), count);
    loader.dlclose(handle).unwrap();

    assert_eq!(unsafe { *marker }, 0xDEAD, "destructor ran");
    assert_eq!(loader.loaded_module_count(), count - 1);
    assert!(loader.dlclose(handle).is_err());
    assert!(dlerror().unwrap().starts_with("dlclose failed: invalid handle: 0x"));
    assert!(dlerror().is_none());
}

#[test]
fn bionic_find_libraries_loads_dt_needed_children_and_guest_dlopen_roundtrips() {
    for variant in VARIANTS {
        let loader = loader_for(vec![fixture_dir(variant)]);
        let handle = loader.dlopen(Some("libfx_dep.so"), 2, None).unwrap();
        let info = loader.module_info(handle).unwrap();
        assert!(info.children.contains(&"libfx_add.so".to_string()), "{variant}");
        let count = loader.loaded_module_count();

        let dep_add: extern "C" fn(i32, i32) -> i32 = unsafe { std::mem::transmute(symbol(&loader, handle, "fx_dep_add")) };
        assert_eq!(dep_add(2, 3), 5 + 0x1234, "{variant} DT_NEEDED resolution");
        let through_dlopen: extern "C" fn(i32, i32) -> i32 = unsafe { std::mem::transmute(symbol(&loader, handle, "fx_dep_dlsym_add")) };
        assert_eq!(through_dlopen(2, 3), 5, "{variant} dlopen dlsym dlclose from guest");
        assert_eq!(loader.loaded_module_count(), count, "{variant}");
        assert!(loader.report().missing_dependencies.is_empty());
    }
}

#[test]
fn bionic_dlsym_default_sees_only_global_modules_and_handle_lookup_walks_children() {
    let loader = loader_for(vec![fixture_dir("16k")]);
    let handle = loader.dlopen(Some("libfx_dep.so"), 2, None).unwrap();

    assert!(loader.dlsym(DlHandle::Default, "fx_dep_add", None).is_err());
    assert_eq!(dlerror().unwrap(), "undefined symbol: fx_dep_add");
    assert_ne!(loader.dlsym(DlHandle::Module(handle), "add", None).unwrap(), 0, "child symbol through parent handle");
    let again = loader.dlopen(Some("libfx_add.so"), 2 | 0x100, None).unwrap();
    assert!(loader.dlsym(DlHandle::Default, "add", None).is_err(), "re-dlopen never promotes rtld flags");
    assert_ne!(again, 0);

    let fresh = loader_for(vec![fixture_dir("16k")]);
    let global = fresh.dlopen(Some("libfx_add.so"), 2 | 0x100, None).unwrap();
    assert_eq!(fresh.dlsym(DlHandle::Default, "add", None).unwrap(), symbol(&fresh, global, "add"));
    assert!(loader.dlsym(DlHandle::Module(0x1235), "add", None).is_err());
    assert_eq!(dlerror().unwrap(), "dlsym failed: invalid handle: 0x1235");
}

#[test]
fn bionic_dladdr_and_dl_iterate_phdr_describe_the_loaded_module() {
    let loader = loader_for(vec![fixture_dir("16k")]);
    let handle = loader.dlopen(Some("libfx_add.so"), 2, None).unwrap();
    let add = symbol(&loader, handle, "add");
    let info = loader.module_info(handle).unwrap();

    let found = loader.dladdr(add | (0x5a << 56)).unwrap();
    assert_eq!(found.path, info.path);
    assert_eq!(found.base, info.region);
    assert_eq!(found.symbol, Some(("add".to_string(), add)));
    assert!(loader.dladdr(host_function_address("strlen")).is_none());

    let mut seen = None;
    loader.dl_iterate_phdr(&mut |entry| {
        if entry.name == info.path {
            seen = Some(entry.clone());
        }
        0
    });
    let entry = seen.unwrap();
    assert_eq!(entry.addr, info.load_bias);
    assert!(entry.phdr >= info.region && entry.phdr < info.region + info.region_len);
    assert_eq!(unsafe { *((entry.phdr) as *const u32) }, 6, "first program header in memory is PT_PHDR");
    assert!(entry.phnum >= 4);
    let mut visited = 0;
    assert_eq!(loader.dl_iterate_phdr(&mut |_| { visited += 1; 42 }), 42);
    assert_eq!(visited, 1);
}

#[test]
fn bionic_dlopen_failures_have_the_linker_error_text_and_clean_up() {
    let loader = loader_for(vec![fixture_dir("16k")]);
    let count = loader.loaded_module_count();

    assert!(loader.dlopen(Some("libnope.so"), 2, None).is_err());
    assert_eq!(dlerror().unwrap(), "dlopen failed: library \"libnope.so\" not found");
    assert!(dlerror().is_none(), "dlerror clears after one read");
    assert!(loader.dlopen(Some("libfx_add.so"), 8, None).is_err());
    assert_eq!(dlerror().unwrap(), "dlopen failed: invalid flags to dlopen: 8");
    assert!(loader.dlopen(Some("libfx_add.so"), 4, None).is_err());
    assert_eq!(dlerror().unwrap(), "dlopen failed: library \"libfx_add.so\" wasn't loaded and RTLD_NOLOAD prevented it");
    assert_eq!(loader.loaded_module_count(), count);
    assert!(loader.dlopen(Some("libc.so"), 2, None).is_ok(), "system pseudo module found by soname");
}

fn scratch_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("winedroid-loader-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn bionic_open_library_rejects_non_elf_payloads_with_bad_magic_text() {
    let dir = scratch_dir("payload");
    fs::write(dir.join("libpayload.so"), [b'M', b'E', b'D', b'I', b'A', 0, 0, 0].repeat(16)).unwrap();
    let loader = loader_for(vec![dir.clone()]);

    assert!(loader.dlopen(Some("libpayload.so"), 2, None).is_err());
    assert!(dlerror().unwrap().contains("has bad ELF magic: 4d454449"));
    assert_eq!(loader.loaded_module_count(), 13);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn bionic_missing_dt_needed_is_reported_in_best_effort_and_fatal_in_strict() {
    let dir = scratch_dir("missing");
    fs::copy(fixture_dir("16k").join("libfx_dep.so"), dir.join("libfx_dep.so")).unwrap();
    let loader = loader_for(vec![dir.clone()]);

    let handle = loader.dlopen(Some("libfx_dep.so"), 2, None).unwrap();
    let report = loader.report();
    assert_eq!(report.missing_dependencies.len(), 1);
    assert_eq!(report.missing_dependencies[0].library, "libfx_add.so");
    assert!(report.unresolved_imports.iter().any(|(_, name)| name == "add"));
    let function = symbol(&loader, handle, "fx_dep_add");
    assert_ne!(function, 0);

    let strict = std::sync::Arc::new(super::loader::Loader::new(vec![dir.clone()], Box::new(super::loader_test_support::TestProvider::new())).with_strict(true));
    assert!(strict.dlopen(Some("libfx_dep.so"), 2, None).is_err());
    assert!(dlerror().unwrap().contains("library \"libfx_add.so\" not found: needed by"));
    assert_eq!(strict.loaded_module_count(), 13, "failed dlopen unmaps what it created");
    fs::remove_dir_all(dir).unwrap();
}

fn relocation_table_file_offset(data: &[u8]) -> usize {
    let image = super::elf_image::ElfImage::parse("fixture", data).unwrap();
    let dynamic = image.dynamic();
    let entries = &data[dynamic.offset as usize..(dynamic.offset + dynamic.filesz) as usize];
    let rela = entries
        .chunks_exact(16)
        .find(|entry| u64::from_le_bytes(entry[0..8].try_into().unwrap()) == 7)
        .map(|entry| u64::from_le_bytes(entry[8..16].try_into().unwrap()))
        .unwrap();
    let load = image.loads().find(|load| rela >= load.vaddr && rela < load.vaddr + load.filesz).unwrap();
    (rela - load.vaddr + load.offset) as usize
}

fn load_with_first_rela_type(kind: u64) -> String {
    load_with_first_rela(&format!("reloc{kind}"), |data, at| {
        let info = u64::from_le_bytes(data[at + 8..at + 16].try_into().unwrap());
        data[at + 8..at + 16].copy_from_slice(&((info & !0xffff_ffff) | kind).to_le_bytes());
    })
}

fn load_with_first_rela(label: &str, edit: impl FnOnce(&mut Vec<u8>, usize)) -> String {
    let dir = scratch_dir(label);
    let mut data = fs::read(fixture_dir("16k").join("libfx_reloc.so")).unwrap();
    let at = relocation_table_file_offset(&data);
    edit(&mut data, at);
    fs::write(dir.join("libfx_reloc.so"), data).unwrap();
    let loader = loader_for(vec![dir.clone()]);
    let error = loader.dlopen(Some("libfx_reloc.so"), 2, None).err().map(|error| error.to_string()).unwrap_or_default();
    assert_eq!(loader.loaded_module_count(), 13);
    fs::remove_dir_all(dir).unwrap();
    error
}

#[test]
fn bionic_process_relocation_general_rejects_ifunc_copy_tpoff_and_unknown_types() {
    let ifunc = load_with_first_rela_type(1032);
    assert!(ifunc.contains("next missing primitive: R_AARCH64_IRELATIVE (ifunc)"), "{ifunc}");
    assert!(load_with_first_rela_type(1024).contains("COPY relocations are not supported"));
    assert!(load_with_first_rela_type(1030).contains("using IE access model"));
    assert!(load_with_first_rela_type(999).contains("unknown reloc type 999"));
}

fn patched_loader(variant: &str) -> std::sync::Arc<super::loader::Loader> {
    let loader = super::loader::Loader::new(vec![fixture_dir(variant)], Box::new(super::loader_test_support::TestProvider::new()))
        .with_patcher(Box::new(super::text_patch::TextPatcher))
        .with_strict(false);
    std::sync::Arc::new(loader)
}

#[test]
fn bionic_tlsdesc_resolver_gives_each_thread_its_own_copy_of_the_tls_template() {
    for variant in VARIANTS {
        for library in RELOCATING {
            let loader = patched_loader(variant);
            let handle = loader.dlopen(Some(library), 2, None).unwrap();
            let next: extern "C" fn() -> i32 = unsafe { std::mem::transmute(symbol(&loader, handle, "fx_tls_next")) };
            let label = format!("{variant}/{library}");

            assert_eq!(next(), 6, "{label} fx_tls_counter++ + ++fx_tls_zero");
            assert_eq!(next(), 8, "{label}");
            let other_thread = std::thread::spawn(move || next()).join().unwrap();
            assert_eq!(other_thread, 6, "{label} fresh thread starts from the .tdata template");
            assert_eq!(next(), 10, "{label} first thread keeps its own block");
        }
    }
}

#[test]
fn bionic_relocation_offsets_outside_the_image_are_a_load_error() {
    let error = load_with_first_rela("offset", |data, at| data[at..at + 8].copy_from_slice(&0x4000_0000u64.to_le_bytes()));

    assert!(error.contains("is outside the image"), "{error}");
}
