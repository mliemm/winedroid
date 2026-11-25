use super::bionic::guest_mem::cstr_lossy;
use super::call_assign::CallBuilder;
use super::dl_api::DlHandle;
use super::elf_image::ElfImage;
use super::loader_test_support::{dlopen_without_constructors, fixture_dir};
use super::runtime::NativeRuntime;
use super::bridge_test_support::register_all;

const UNTOUCHED: usize = 0x5a5a_5a5a_5a5a_5a5a;
const SYSTEM_MODULES: usize = 13;

#[repr(C)]
#[derive(Clone, Copy)]
struct Entry {
    addr: usize,
    name: usize,
    phnum: usize,
    first_load_vaddr: usize,
    first_load_memsz: usize,
    gnu_eh_frame_vaddr: usize,
    adds: u64,
    subs: u64,
    tls_modid: usize,
    tls_data: usize,
}

impl Entry {
    fn sentinel() -> Self {
        Self {
            addr: UNTOUCHED,
            name: UNTOUCHED,
            phnum: UNTOUCHED,
            first_load_vaddr: UNTOUCHED,
            first_load_memsz: UNTOUCHED,
            gnu_eh_frame_vaddr: UNTOUCHED,
            adds: UNTOUCHED as u64,
            subs: UNTOUCHED as u64,
            tls_modid: UNTOUCHED,
            tls_data: UNTOUCHED,
        }
    }
}

fn runtime() -> &'static NativeRuntime {
    register_all();
    NativeRuntime::initialize(vec![fixture_dir("16k")]).expect("the native runtime is initialized with the 16k fixture directory")
}

fn load(name: &str) -> u64 {
    dlopen_without_constructors(runtime().loader(), name).unwrap_or_else(|error| panic!("{name}: {error}"))
}

fn fixture_symbol(handle: u64, name: &str) -> usize {
    runtime().loader().dlsym(DlHandle::Module(handle), name, None).unwrap_or_else(|error| panic!("{name}: {error}"))
}

fn record(capacity: usize) -> (i32, Vec<Entry>) {
    let handle = load("libfx_phdr.so");
    let mut entries = vec![Entry::sentinel(); capacity.max(2)];
    let result = CallBuilder::new(fixture_symbol(handle, "fx_phdr_record"))
        .ptr(entries.as_mut_ptr() as usize)
        .i32(capacity as i32)
        .invoke()
        .unwrap()
        .i32();
    (result, entries)
}

fn entry_named<'a>(entries: &'a [Entry], suffix: &str) -> &'a Entry {
    entries
        .iter()
        .find(|entry| entry.name != UNTOUCHED && cstr_lossy(entry.name).ends_with(suffix))
        .unwrap_or_else(|| panic!("no dl_iterate_phdr entry for {suffix}"))
}

#[test]
fn bionic_dl_iterate_phdr_reports_the_load_bias_phdr_count_and_gnu_eh_frame_segment_of_a_mapped_library() {
    let handle = load("libfx_phdr.so");
    let info = runtime().loader().module_info(handle).expect("the fixture has a guest image");
    let (result, entries) = record(64);
    assert!(result as usize > SYSTEM_MODULES, "the walk must reach past the system modules, got {result}");
    let entry = entry_named(&entries, "libfx_phdr.so");
    let data = std::fs::read(fixture_dir("16k").join("libfx_phdr.so")).unwrap();
    let elf = ElfImage::parse("libfx_phdr.so", &data).unwrap();
    let first_load = elf.loads().next().expect("the fixture has a PT_LOAD");

    assert_eq!(entry.addr, info.load_bias, "dlpi_addr is the load bias");
    assert_eq!(entry.phnum, info.phdr_count as usize, "dlpi_phnum is the mapped phdr count");
    assert_eq!(entry.first_load_vaddr as u64, first_load.vaddr, "the phdr array holds the real program headers");
    assert_eq!(entry.first_load_memsz as u64, first_load.memsz);
    assert_eq!(entry.gnu_eh_frame_vaddr as u64, info.eh_frame_hdr_vaddr, "PT_GNU_EH_FRAME p_vaddr is reachable");
    assert_ne!(entry.gnu_eh_frame_vaddr, 0);
    let magic = unsafe { std::ptr::read_unaligned((entry.addr + entry.first_load_vaddr) as *const u32) };
    assert_eq!(magic, 0x464c_457f, "dlpi_addr plus the first PT_LOAD p_vaddr is the mapped ELF header");
    assert!(entry.adds >= 1, "a module that finished link_image counts one load");
}

#[test]
fn bionic_dl_iterate_phdr_returns_the_first_nonzero_callback_result_after_the_entries_it_wrote() {
    let (result, entries) = record(2);

    assert_eq!(result, 7, "the callback stops the walk with its own nonzero result");
    assert_ne!(entries[0].name, UNTOUCHED, "the first entry was written");
    assert_ne!(entries[1].name, UNTOUCHED, "the second entry was written");
    assert!(entries[2..].iter().all(|entry| entry.name == UNTOUCHED), "the walk wrote no third entry");
}

#[test]
fn bionic_dl_iterate_phdr_lists_the_system_modules_without_a_mapped_phdr_table() {
    let (_, entries) = record(64);
    let without_phdr: Vec<&Entry> = entries.iter().filter(|entry| entry.name != UNTOUCHED && entry.phnum == 0).collect();

    assert_eq!(without_phdr.len(), SYSTEM_MODULES);
    for entry in without_phdr {
        let name = cstr_lossy(entry.name);
        assert!(name.starts_with("/apex/") || name.starts_with("/system/"), "{name}");
        assert_eq!(entry.addr, 0, "{name} has no load bias");
        assert_eq!(entry.tls_modid, 0, "{name} has no TLS segment");
        assert_eq!(entry.tls_data, 0, "{name} has no TLS block");
    }
}

#[test]
fn bionic_dl_iterate_phdr_reports_a_tls_block_only_on_a_thread_that_materialized_it() {
    let handle = load("libfx_phdr.so");
    load("libfx_add.so");
    let touched = CallBuilder::new(fixture_symbol(handle, "fx_phdr_touch_tls")).invoke().unwrap().i32();
    assert_eq!(touched, 8, "the fixture thread local starts at 7 and the touch increments it");
    let (_, entries) = record(64);
    let entry = entry_named(&entries, "libfx_phdr.so");
    assert_ne!(entry.tls_modid, 0, "the fixture has a PT_TLS segment");
    assert_ne!(entry.tls_data, 0, "the calling thread materialized the block");
    let plain = entry_named(&entries, "libfx_add.so");
    assert_eq!(plain.tls_modid, 0, "libfx_add has no PT_TLS segment");
    assert_eq!(plain.tls_data, 0);

    let fresh = std::thread::spawn(|| {
        let (_, entries) = record(64);
        let entry = entry_named(&entries, "libfx_phdr.so");
        (entry.tls_modid, entry.tls_data)
    })
    .join()
    .unwrap();
    assert_ne!(fresh.0, 0, "the module id is thread independent");
    assert_eq!(fresh.1, 0, "a thread that never ran guest code has no allocated TLS block");
}

#[test]
fn bionic_phdr_counters_advance_on_a_new_load_and_on_an_unload() {
    let (_, before) = record(64);
    let (adds_before, subs_before) = {
        let entry = entry_named(&before, "libfx_phdr.so");
        (entry.adds, entry.subs)
    };
    let handle = runtime().loader().dlopen(Some("libfx_relro.so"), 2, None).expect("libfx_relro loads");
    let (_, after_load) = record(64);
    assert!(entry_named(&after_load, "libfx_phdr.so").adds > adds_before, "link_image increments the load counter");
    runtime().loader().dlclose(handle).expect("libfx_relro unloads");
    let (_, after_unload) = record(64);
    assert!(entry_named(&after_unload, "libfx_phdr.so").subs > subs_before, "soinfo_unload_impl increments the unload counter");
}
