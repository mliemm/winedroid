use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::dl_api::DlHandle;
use super::fixture_elf::{
    FixtureElf, R_AARCH64_ABS64, R_AARCH64_GLOB_DAT, R_AARCH64_JUMP_SLOT, R_AARCH64_TLSDESC,
};
use super::loader::Loader;
use super::loader_test_support::{
    dlopen_without_constructors, fixture_dir, loader_for, take_recorded, unresolved_import_address,
};
use super::tls_modules;
use super::tp_veneer_tests::veneer_for;

const ADDEND: u64 = 0x10;
const VARIANTS: [&str; 2] = ["16k", "4k"];
const R_AARCH64_RELATIVE_KIND: u32 = super::fixture_elf::R_AARCH64_RELATIVE;

fn scratch_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("winedroid-link-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn slot(load_bias: usize, vaddr: u64) -> u64 {
    unsafe { *((load_bias + vaddr as usize) as *const u64) }
}

fn within_deadline<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    let handle = std::thread::spawn(work);
    let started = Instant::now();
    while !handle.is_finished() {
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "guest call did not return in 20 s, the dl lock deadlocked"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    handle.join().unwrap()
}

fn symbol(loader: &Loader, handle: u64, name: &str) -> usize {
    loader
        .dlsym(DlHandle::Module(handle), name, None)
        .unwrap_or_else(|error| panic!("{name}: {error}"))
}

#[test]
fn bionic_lookup_symbol_failure_writes_the_provider_stub_into_every_unresolved_import_slot() {
    for variant in VARIANTS {
        let data = fs::read(fixture_dir(variant).join("libfx_reloc.so")).unwrap();
        let elf = FixtureElf::parse(&data);
        let loader = loader_for(vec![fixture_dir(variant)]);
        let handle = loader.dlopen(Some("libfx_reloc.so"), 2, None).unwrap();
        let bias = loader.module_info(handle).unwrap().load_bias;

        let unresolved: Vec<_> = elf
            .relocations
            .iter()
            .filter(|relocation| {
                matches!(
                    relocation.symbol.as_deref(),
                    Some("snprintf" | "__android_log_print")
                )
            })
            .collect();
        assert_eq!(unresolved.len(), 2, "{variant}");
        for relocation in unresolved {
            assert_eq!(
                slot(bias, relocation.vaddr) as usize,
                unresolved_import_address(),
                "{variant} {:?}",
                relocation.symbol
            );
        }
    }
}

#[test]
fn bionic_get_addend_norel_adds_the_addend_to_abs64_glob_dat_and_jump_slot_targets() {
    for variant in VARIANTS {
        let mut data = fs::read(fixture_dir(variant).join("libfx_reloc.so")).unwrap();
        let elf = FixtureElf::parse(&data);
        let symbolic: Vec<_> = elf
            .relocations
            .iter()
            .filter(|relocation| relocation.symbol.is_some())
            .filter(|relocation| {
                [R_AARCH64_ABS64, R_AARCH64_GLOB_DAT, R_AARCH64_JUMP_SLOT]
                    .contains(&relocation.kind)
            })
            .map(|relocation| (relocation.kind, relocation.vaddr, relocation.file_offset))
            .collect();
        for kind in [R_AARCH64_ABS64, R_AARCH64_GLOB_DAT, R_AARCH64_JUMP_SLOT] {
            assert!(
                symbolic.iter().any(|entry| entry.0 == kind),
                "{variant} fixture has no symbolic relocation of type {kind}"
            );
        }
        for (_, _, offset) in &symbolic {
            let at = offset + 16;
            let addend = u64::from_le_bytes(data[at..at + 8].try_into().unwrap());
            data[at..at + 8].copy_from_slice(&addend.wrapping_add(ADDEND).to_le_bytes());
        }
        let dir = scratch_dir(&format!("addend-{variant}"));
        fs::write(dir.join("libfx_reloc.so"), &data).unwrap();

        let plain_loader = loader_for(vec![fixture_dir(variant)]);
        let plain_handle = dlopen_without_constructors(&plain_loader, "libfx_reloc.so").unwrap();
        let plain = plain_loader.module_info(plain_handle).unwrap();
        let shifted_loader = loader_for(vec![dir.clone()]);
        let shifted_handle =
            dlopen_without_constructors(&shifted_loader, "libfx_reloc.so").unwrap();
        let shifted = shifted_loader.module_info(shifted_handle).unwrap();

        for (kind, vaddr, _) in symbolic {
            let before = slot(plain.load_bias, vaddr);
            let after = slot(shifted.load_bias, vaddr);
            let inside_image = (before as usize) >= plain.region
                && (before as usize) < plain.region + plain.region_len;
            let expected = if inside_image {
                before - plain.load_bias as u64 + shifted.load_bias as u64 + ADDEND
            } else {
                before + ADDEND
            };
            assert_eq!(
                after, expected,
                "{variant} relocation type {kind} at {vaddr:#x}"
            );
        }
        fs::remove_dir_all(dir).unwrap();
    }
}

#[test]
fn bionic_call_constructors_run_dependencies_first_in_init_array_order_and_destructors_in_reverse()
{
    for variant in VARIANTS {
        let (recorded, nested, closed, destructors) = within_deadline(move || {
            let loader = loader_for(vec![fixture_dir(variant)]);
            take_recorded();
            let count = loader.loaded_module_count();
            let handle = loader
                .dlopen(Some("libfx_order_parent.so"), 2, None)
                .unwrap();
            let recorded = take_recorded();
            let nested = unsafe { *(symbol(&loader, handle, "fx_nested_result") as *const i32) };
            let closed = unsafe { *(symbol(&loader, handle, "fx_nested_close") as *const i32) };
            loader.dlclose(handle).unwrap();
            assert_eq!(
                loader.loaded_module_count(),
                count,
                "{variant} parent and child unloaded"
            );
            (recorded, nested, closed, take_recorded())
        });

        assert_eq!(
            recorded,
            vec![1, 2, 3, 4, 5],
            "{variant} child init_array, then parent init_array, nested dlopen last"
        );
        assert_eq!(
            (nested, closed),
            (5, 0),
            "{variant} dlopen, dlsym and dlclose from a constructor under the dl lock"
        );
        assert_eq!(
            destructors,
            vec![-4, -3, -2, -1],
            "{variant} parent fini_array reversed, then child fini_array reversed"
        );
    }
}

#[test]
fn bionic_open_library_finds_dt_needed_through_dt_runpath_origin_of_the_needing_library() {
    for variant in VARIANTS {
        let root = scratch_dir(&format!("runpath-{variant}"));
        fs::create_dir_all(root.join("dep")).unwrap();
        fs::copy(
            fixture_dir(variant).join("libfx_order_parent.so"),
            root.join("libfx_order_parent.so"),
        )
        .unwrap();
        fs::copy(
            fixture_dir(variant).join("libfx_order_child.so"),
            root.join("dep/libfx_order_child.so"),
        )
        .unwrap();
        let loader = loader_for(vec![root.clone()]);
        take_recorded();

        let handle = loader
            .dlopen(Some("libfx_order_parent.so"), 2, None)
            .unwrap();

        assert!(
            loader.report().missing_dependencies.is_empty(),
            "{variant} {:?}",
            loader.report().missing_dependencies
        );
        assert!(
            loader
                .module_info(handle)
                .unwrap()
                .children
                .contains(&"libfx_order_child.so".to_string()),
            "{variant}"
        );
        assert_eq!(
            take_recorded(),
            vec![1, 2, 3, 4],
            "{variant} constructors ran, nested dlopen found no libfx_add.so"
        );
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn bionic_text_patcher_leaves_words_outside_every_fde_untouched_and_patches_functions() {
    for variant in VARIANTS {
        let loader = loader_for(vec![fixture_dir(variant)]);
        let child = loader
            .dlopen(Some("libfx_order_child.so"), 2, None)
            .unwrap();
        let pool = symbol(&loader, child, "fx_pool_words") as *const u32;
        let words: Vec<u32> = (0..3).map(|index| unsafe { *pool.add(index) }).collect();

        assert_eq!(
            words,
            vec![0xD53BD040, 0xF800865E, 0xF85F8E5E],
            "{variant} literal pool outside any FDE is data"
        );

        let reloc = loader.dlopen(Some("libfx_reloc.so"), 2, None).unwrap();
        let function = symbol(&loader, reloc, "fx_tls_next") as *const u32;
        let code: Vec<u32> = (0..48)
            .map(|index| unsafe { *function.add(index) })
            .collect();
        assert!(
            code.iter().all(|word| word & 0xFFFF_FFE0 != 0xD53B_D040),
            "{variant} no mrs tpidr_el0 left in fx_tls_next"
        );
        let sites: Vec<usize> = (0..code.len())
            .filter(|index| veneer_for(function as usize + 4 * index, code[*index]).is_some())
            .collect();
        assert_eq!(
            sites.len(),
            2,
            "{variant} both mrs in fx_tls_next became a branch to a thread pointer veneer"
        );
    }
}

#[test]
fn bionic_text_patcher_reports_guest_writes_to_x18_per_library() {
    for variant in VARIANTS {
        let loader = loader_for(vec![fixture_dir(variant)]);
        loader.dlopen(Some("libfx_reloc.so"), 2, None).unwrap();
        assert!(loader.report().x18_writes.is_empty(), "{variant}");

        loader
            .dlopen(Some("libfx_order_child.so"), 2, None)
            .unwrap();
        let writes = loader.report().x18_writes;

        assert_eq!(writes.len(), 1, "{variant} {writes:?}");
        assert!(writes[0].0.ends_with("libfx_order_child.so"), "{variant}");
        assert_eq!(writes[0].1, 1, "{variant}");
    }
}

#[test]
fn bionic_dlopen_of_a_loaded_dt_needed_child_raises_the_count_of_its_local_group_root() {
    let loader = loader_for(vec![fixture_dir("16k")]);
    let dep = loader.dlopen(Some("libfx_dep.so"), 2, None).unwrap();
    let count = loader.loaded_module_count();
    assert_eq!(loader.module_info(dep).unwrap().ref_count, 1);

    let add = loader.dlopen(Some("libfx_add.so"), 2, None).unwrap();
    assert_eq!(
        loader.module_info(dep).unwrap().ref_count,
        2,
        "the child's group root is the dependent library"
    );
    assert_eq!(loader.module_info(add).unwrap().ref_count, 0);

    loader.dlclose(add).unwrap();
    assert_eq!(loader.module_info(dep).unwrap().ref_count, 1);
    assert_eq!(loader.loaded_module_count(), count);
    loader.dlclose(dep).unwrap();
    assert_eq!(loader.loaded_module_count(), count - 2);
}

#[test]
fn bionic_soinfo_unload_unregisters_the_tls_module_of_the_closed_library() {
    let loader = loader_for(vec![fixture_dir("16k")]);
    let handle = loader.dlopen(Some("libfx_reloc.so"), 2, None).unwrap();
    let module = loader.module_info(handle).unwrap().tls_module;
    let generation =
        tls_modules::generation_of(module).expect("TLS module registered while loaded");
    let next: extern "C" fn() -> i32 =
        unsafe { std::mem::transmute(symbol(&loader, handle, "fx_tls_next")) };
    assert_eq!(next(), 6);
    assert!(tls_modules::existing_block_address(module).is_some());

    loader.dlclose(handle).unwrap();

    assert_ne!(tls_modules::generation_of(module), Some(generation));
    assert_eq!(
        tls_modules::existing_block_address(module),
        None,
        "this thread's block of the closed module is gone"
    );
}

#[test]
fn bionic_open_library_prefers_the_dt_runpath_copy_over_the_library_directory_copy() {
    let root = scratch_dir("runpath-first");
    fs::create_dir_all(root.join("dep")).unwrap();
    fs::copy(
        fixture_dir("16k").join("libfx_order_parent.so"),
        root.join("libfx_order_parent.so"),
    )
    .unwrap();
    fs::copy(
        fixture_dir("16k").join("libfx_order_child.so"),
        root.join("dep/libfx_order_child.so"),
    )
    .unwrap();
    fs::copy(
        fixture_dir("4k").join("libfx_order_child.so"),
        root.join("libfx_order_child.so"),
    )
    .unwrap();
    let loader = loader_for(vec![fs::canonicalize(&root).unwrap()]);
    take_recorded();

    let parent = dlopen_without_constructors(&loader, "libfx_order_parent.so").unwrap();
    let child = dlopen_without_constructors(&loader, "libfx_order_child.so").unwrap();

    assert!(loader
        .module_info(parent)
        .unwrap()
        .children
        .contains(&"libfx_order_child.so".to_string()));
    let path = loader.module_info(child).unwrap().path;
    assert!(path.ends_with("dep/libfx_order_child.so"), "{path}");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn bionic_process_relocation_relative_stores_the_load_bias_plus_the_addend() {
    for variant in VARIANTS {
        let data = fs::read(fixture_dir(variant).join("libfx_reloc.so")).unwrap();
        let elf = FixtureElf::parse(&data);
        let relative: Vec<_> = elf
            .relocations
            .iter()
            .filter(|relocation| relocation.kind == R_AARCH64_RELATIVE_KIND)
            .collect();
        assert!(
            !relative.is_empty(),
            "{variant} fixture has RELATIVE relocations"
        );
        let loader = loader_for(vec![fixture_dir(variant)]);
        let handle = dlopen_without_constructors(&loader, "libfx_reloc.so").unwrap();
        let bias = loader.module_info(handle).unwrap().load_bias;

        for relocation in relative {
            let at = relocation.file_offset + 16;
            let addend = u64::from_le_bytes(data[at..at + 8].try_into().unwrap());
            assert_eq!(
                slot(bias, relocation.vaddr),
                (bias as u64).wrapping_add(addend),
                "{variant} RELATIVE at {:#x}",
                relocation.vaddr
            );
        }
    }
}

#[test]
fn bionic_dlclose_frees_the_tls_module_id_for_the_next_load() {
    let loader = loader_for(vec![fixture_dir("16k")]);
    let mut reused = false;
    for _ in 0..50 {
        let handle = loader.dlopen(Some("libfx_reloc.so"), 2, None).unwrap();
        let module = loader.module_info(handle).unwrap().tls_module;
        loader.dlclose(handle).unwrap();
        let again = loader.dlopen(Some("libfx_reloc.so"), 2, None).unwrap();
        reused |= loader.module_info(again).unwrap().tls_module == module;
        loader.dlclose(again).unwrap();
        if reused {
            break;
        }
    }
    assert!(
        reused,
        "a reload after dlclose never got the freed module id back"
    );
}

#[test]
fn bionic_text_patcher_patches_whole_segments_when_pt_gnu_eh_frame_is_an_empty_placeholder() {
    let mut data = fs::read(fixture_dir("16k").join("libfx_order_child.so")).unwrap();
    let table = u64::from_le_bytes(data[32..40].try_into().unwrap()) as usize;
    let count = usize::from(u16::from_le_bytes(data[56..58].try_into().unwrap()));
    let header = (0..count)
        .map(|index| table + index * 56)
        .find(|at| data[*at..*at + 4] == 0x6474_e550u32.to_le_bytes())
        .unwrap();
    data[header + 8..header + 48].fill(0);
    let dir = scratch_dir("empty-eh-frame");
    fs::write(dir.join("libfx_order_child.so"), data).unwrap();
    let loader = loader_for(vec![dir.clone()]);

    let handle = loader
        .dlopen(Some("libfx_order_child.so"), 2, None)
        .unwrap();
    let pool = symbol(&loader, handle, "fx_pool_words") as *const u32;

    assert!(
        veneer_for(pool as usize, unsafe { *pool }).is_some(),
        "no FDE ranges, so the whole executable segment is rewritten"
    );
    assert_eq!(unsafe { *pool.add(1) }, 0xD503201F);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn bionic_call_constructors_pass_argc_argv_and_envp_to_every_init_array_function() {
    for variant in VARIANTS {
        let (argc, first_byte, envp_terminated) = within_deadline(move || {
            let loader = loader_for(vec![fixture_dir(variant)]);
            let handle = loader
                .dlopen(Some("libfx_order_child.so"), 2, None)
                .unwrap();
            let read = |name: &str| unsafe { *(symbol(&loader, handle, name) as *const i32) };
            (
                read("fx_ctor_argc"),
                read("fx_ctor_argv0"),
                read("fx_ctor_envp_terminated"),
            )
        });

        assert_eq!(argc, 1, "{variant} argc");
        assert_eq!(
            first_byte,
            i32::from(b'/'),
            "{variant} argv[0] is the program path"
        );
        assert_eq!(
            envp_terminated, 1,
            "{variant} envp is a non null empty array"
        );
    }
}

#[test]
fn bionic_dtprel64_relocation_stores_the_symbol_value_plus_the_addend() {
    const DTPREL64: u32 = 1029;
    for variant in VARIANTS {
        let original = fs::read(fixture_dir(variant).join("libfx_reloc.so")).unwrap();
        let elf = FixtureElf::parse(&original);
        let target = elf
            .relocations
            .iter()
            .find(|relocation| {
                relocation.kind == R_AARCH64_TLSDESC
                    && relocation.symbol.as_deref() == Some("fx_tls_zero")
            })
            .unwrap();
        let symbol_value = elf.export_addresses["fx_tls_zero"];
        assert_ne!(
            symbol_value, 0,
            "{variant} the TLS symbol value is nonzero so a dropped symbol is visible"
        );
        for addend in [0u64, ADDEND] {
            let mut data = original.clone();
            let info = u64::from_le_bytes(
                data[target.file_offset + 8..target.file_offset + 16]
                    .try_into()
                    .unwrap(),
            );
            data[target.file_offset + 8..target.file_offset + 16]
                .copy_from_slice(&((info & !0xFFFF_FFFF) | u64::from(DTPREL64)).to_le_bytes());
            data[target.file_offset + 16..target.file_offset + 24]
                .copy_from_slice(&addend.to_le_bytes());
            let dir = scratch_dir(&format!("dtprel-{variant}-{addend}"));
            fs::write(dir.join("libfx_reloc.so"), &data).unwrap();
            let loader = loader_for(vec![dir.clone()]);
            let handle = dlopen_without_constructors(&loader, "libfx_reloc.so").unwrap();
            let bias = loader.module_info(handle).unwrap().load_bias;

            assert_eq!(
                slot(bias, target.vaddr),
                symbol_value + addend,
                "{variant} addend {addend}"
            );
            fs::remove_dir_all(dir).unwrap();
        }
    }
}

#[test]
fn bionic_relr_and_aps2_packed_relative_relocations_add_the_load_bias() {
    for variant in VARIANTS {
        for library in ["libfx_reloc.so", "libfx_aps2.so", "libfx_relr.so"] {
            let loader = loader_for(vec![fixture_dir(variant)]);
            let handle = dlopen_without_constructors(&loader, library).unwrap();
            let info = loader.module_info(handle).unwrap();
            let pointer = unsafe { *(symbol(&loader, handle, "fx_local_pointer") as *const usize) };

            assert!(
                pointer >= info.region && pointer < info.region + info.region_len,
                "{variant} {library}: {pointer:#x} is outside the image"
            );
            assert_eq!(
                unsafe { *(pointer as *const i32) },
                11,
                "{variant} {library}"
            );
        }
    }
}
