use super::dl_api::DlHandle;
use super::fixture_elf::FixtureElf;
use super::loader::Loader;
use super::loader_test_support::{fixture_dir, host_protection_of, is_mapped, loader_for, terminating_signal_of_child, unresolved_import_address};

const VARIANTS: [&str; 2] = ["16k", "4k"];
const HOST_PAGE: usize = 16384;

fn symbol(loader: &Loader, handle: u64, name: &str) -> usize {
    loader.dlsym(DlHandle::Module(handle), name, None).unwrap_or_else(|error| panic!("{name}: {error}"))
}

#[test]
fn bionic_phdr_table_protect_gnu_relro_makes_a_fixture_relro_page_read_only_and_faults_writes() {
    for variant in VARIANTS {
        let path = fixture_dir(variant).join("libfx_relro.so");
        let data = std::fs::read(&path).unwrap();
        let probe = Loader::load_image("libfx_relro.so", &data).unwrap();
        let range = probe.mapped.layout().relro_pages(probe.elf.relro().expect("fixture has PT_GNU_RELRO")).expect("RELRO covers a whole host page");
        assert!(range.len >= HOST_PAGE as u64, "{variant} RELRO spans a whole host page");
        let loader = loader_for(vec![fixture_dir(variant)]);
        let handle = loader.dlopen(Some("libfx_relro.so"), 2, None).unwrap();
        let info = loader.module_info(handle).unwrap();
        let call: extern "C" fn(i32) -> i32 = unsafe { std::mem::transmute(symbol(&loader, handle, "fx_relro_call")) };

        assert_eq!(call(0), 42, "{variant} relocated table entry before the protection matters");
        assert_eq!(call(4095), 42, "{variant}");
        for page in (range.start..range.start + range.len).step_by(HOST_PAGE) {
            assert_eq!(host_protection_of(info.region + page as usize), libc::PROT_READ, "{variant} RELRO page at image offset {page:#x}");
        }
        let plain = symbol(&loader, handle, "fx_relro_plain");
        assert_eq!(host_protection_of(plain), libc::PROT_READ | libc::PROT_WRITE, "{variant} data after RELRO stays writable");
        unsafe { *(plain as *mut i32) = 6 };
        let inside_relro = info.region + range.start as usize;
        let signal = terminating_signal_of_child(|| unsafe { std::ptr::write_volatile(inside_relro as *mut u8, 1) });

        assert!(matches!(signal, Some(libc::SIGSEGV | libc::SIGBUS)), "{variant} write to a RELRO page was not faulted: {signal:?}");
    }
}

#[test]
fn bionic_soinfo_unload_unmaps_the_image_of_the_closed_library() {
    let loader = loader_for(vec![fixture_dir("16k")]);
    let handle = loader.dlopen(Some("libfx_reloc.so"), 2, None).unwrap();
    let info = loader.module_info(handle).unwrap();
    assert!(is_mapped(info.region));
    assert!(is_mapped(info.region + info.region_len - 1));

    loader.dlclose(handle).unwrap();

    assert!(!is_mapped(info.region));
    assert!(!is_mapped(info.region + info.region_len - 1));
}

#[test]
fn bionic_lookup_symbol_failure_stub_terminates_the_process_when_the_guest_calls_it() {
    let variant = "16k";
    let data = std::fs::read(fixture_dir(variant).join("libfx_reloc.so")).unwrap();
    let elf = FixtureElf::parse(&data);
    let loader = loader_for(vec![fixture_dir(variant)]);
    let handle = loader.dlopen(Some("libfx_reloc.so"), 2, None).unwrap();
    let bias = loader.module_info(handle).unwrap().load_bias;
    let slot = elf.relocations.iter().find(|relocation| relocation.symbol.as_deref() == Some("snprintf")).unwrap();
    let stub = unsafe { *((bias + slot.vaddr as usize) as *const usize) };
    assert_eq!(stub, unresolved_import_address());

    let signal = terminating_signal_of_child(|| {
        let function: extern "C" fn() = unsafe { std::mem::transmute(stub) };
        function();
    });

    assert_eq!(signal, Some(libc::SIGABRT));
}
