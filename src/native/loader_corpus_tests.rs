use std::collections::BTreeMap;
use std::fs;
use std::time::Instant;

use super::dl_api::DlHandle;
use super::elf_image::is_arm64_elf;
use super::loader::Loader;
use super::loader_test_support::{corpus_dir, dlopen_without_constructors, TestProvider};
use super::reloc_stream::{decode_aps2, decode_rela};
use super::symbol_table::gnu_hash;

fn corpus_libraries() -> Option<Vec<std::path::PathBuf>> {
    let dir = corpus_dir();
    let Ok(entries) = fs::read_dir(&dir) else {
        eprintln!("skipped: TikTok library extraction not found at {}", dir.display());
        return None;
    };
    let mut paths: Vec<_> = entries.filter_map(|entry| entry.ok().map(|entry| entry.path())).collect();
    paths.sort();
    Some(paths)
}

fn is_elf(path: &std::path::Path) -> bool {
    fs::read(path).map(|data| is_arm64_elf(&data)).unwrap_or(false)
}

fn without_interpreter(mut data: Vec<u8>) -> Vec<u8> {
    let phoff = u64::from_le_bytes(data[32..40].try_into().unwrap()) as usize;
    let phnum = u16::from_le_bytes(data[56..58].try_into().unwrap()) as usize;
    for index in 0..phnum {
        let at = phoff + index * 56;
        if data[at..at + 4] == 3u32.to_le_bytes() {
            data[at..at + 4].copy_from_slice(&0u32.to_le_bytes());
        }
    }
    data
}

fn new_loader() -> Loader {
    Loader::new(vec![corpus_dir()], Box::new(TestProvider::new())).with_strict(false)
}

#[test]
fn measured_tiktok_47_9_3_corpus_has_268_arm64_elf_files_and_29_payloads() {
    let Some(paths) = corpus_libraries() else { return };
    let elf = paths.iter().filter(|path| is_elf(path)).count();

    assert_eq!(elf, 268);
    assert_eq!(paths.len() - elf, 29);
}

#[test]
fn measured_tiktok_relocation_totals_match_the_independent_python_decoder() {
    let Some(paths) = corpus_libraries() else { return };
    let mut totals: BTreeMap<u32, usize> = BTreeMap::new();
    let mut executables = Vec::new();
    for path in paths {
        let data = fs::read(&path).unwrap();
        if !is_arm64_elf(&data) {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let image = match Loader::load_image(&path.to_string_lossy(), &data) {
            Ok(image) => image,
            Err(error) => {
                assert!(error.to_string().contains("PIE executable"), "{name}: {error}");
                executables.push(name.clone());
                Loader::load_image(&name, &without_interpreter(data)).unwrap()
            }
        };
        let dynamic = &image.dynamic;
        let mut relocs = Vec::new();
        for table in [&dynamic.rela, &dynamic.jmprel] {
            if table.size != 0 {
                relocs.extend(decode_rela(image.mapped.bytes(table.vaddr, table.size).unwrap()));
            }
        }
        if dynamic.android_rela.size != 0 {
            relocs.extend(decode_aps2(image.mapped.bytes(dynamic.android_rela.vaddr, dynamic.android_rela.size).unwrap()).unwrap());
        }
        for reloc in relocs {
            *totals.entry(reloc.kind()).or_default() += 1;
        }
        assert_eq!(image.mapped.layout().shift, 0, "{name} ShiftRw");
    }
    executables.sort();

    assert_eq!(executables, ["libnpth_dumper.so", "libnpth_logcat.so"]);
    assert_eq!(totals.get(&1027), Some(&581_219));
    assert_eq!(totals.get(&257), Some(&114_362));
    assert_eq!(totals.get(&1026), Some(&77_436));
    assert_eq!(totals.get(&1025), Some(&17_831));
    assert_eq!(totals.len(), 4);
}

#[test]
fn measured_tiktok_symbol_tables_agree_between_gnu_and_sysv_lookups() {
    let Some(paths) = corpus_libraries() else { return };
    for name in ["libkeva.so", "libc++_shared.so"] {
        let path = paths.iter().find(|path| path.file_name().unwrap() == name).unwrap();
        let data = fs::read(path).unwrap();
        let image = Loader::load_image(&path.to_string_lossy(), &data).unwrap();
        let (mapped, symbols) = (&image.mapped, &image.symbols);
        assert!(symbols.has_gnu_hash() && symbols.has_sysv_hash(), "{name}");
        assert_eq!(symbols.gnu_derived_count(mapped).unwrap().unwrap(), symbols.count(), "{name} GNU derived symbol count");
        let mut checked = 0;
        for index in 1..symbols.count() {
            let sym = symbols.symbol(mapped, index).unwrap();
            if !sym.is_global_and_defined() {
                continue;
            }
            let symbol_name = symbols.name(mapped, &sym).unwrap();
            let gnu = symbols.lookup_gnu(mapped, &symbol_name).unwrap().unwrap_or_else(|| panic!("{name}: GNU misses {symbol_name}"));
            let sysv = symbols.lookup_sysv(mapped, &symbol_name).unwrap().unwrap_or_else(|| panic!("{name}: SysV misses {symbol_name}"));
            assert_eq!(gnu.1.value, sysv.1.value, "{name} {symbol_name}");
            checked += 1;
        }
        assert!(checked > 100, "{name} checked {checked}");
        assert!(symbols.lookup(mapped, "definitely_not_exported_symbol").unwrap().is_none());
    }
    let keva = fs::read(corpus_dir().join("libkeva.so")).unwrap();
    let image = Loader::load_image("libkeva.so", &keva).unwrap();
    assert_eq!(image.symbols.lookup(&image.mapped, "JNI_OnLoad").unwrap().unwrap().1.value, 0x1c788);
}

#[test]
fn measured_tiktok_single_hash_libraries_resolve_exports() {
    let Some(_) = corpus_libraries() else { return };
    let lite = fs::read(corpus_dir().join("libLiteRt.so")).unwrap();
    let lite = Loader::load_image("libLiteRt.so", &lite).unwrap();
    assert!(lite.symbols.has_gnu_hash() && !lite.symbols.has_sysv_hash());
    assert_eq!(lite.symbols.count(), 923);

    let encryptor = fs::read(corpus_dir().join("libEncryptor.so")).unwrap();
    let encryptor = Loader::load_image("libEncryptor.so", &encryptor).unwrap();
    assert!(encryptor.symbols.has_sysv_hash() && !encryptor.symbols.has_gnu_hash());
    assert_eq!(encryptor.symbols.lookup(&encryptor.mapped, "JNI_OnLoad").unwrap().unwrap().1.value, 0x99c);
}

#[test]
fn measured_libkeva_closure_loads_with_libc_plus_plus_shared_and_relocates_every_import() {
    let Some(_) = corpus_libraries() else { return };
    let loader = new_loader();
    let handle = dlopen_without_constructors(&loader, "libkeva.so").unwrap();
    let info = loader.module_info(handle).unwrap();

    assert!(info.children.contains(&"libc++_shared.so".to_string()));
    assert_eq!(info.shift, 0);
    let on_load = loader.dlsym(DlHandle::Module(handle), "JNI_OnLoad", None).unwrap();
    assert_eq!(on_load, info.load_bias + 0x1c788);
    assert!(loader.report().missing_dependencies.is_empty());
    assert_eq!(loader.dladdr(on_load).unwrap().symbol.unwrap().0, "JNI_OnLoad");
}

#[test]
fn measured_tiktok_every_library_maps_and_relocates_and_absent_dependencies_are_named() {
    let Some(paths) = corpus_libraries() else { return };
    let started = Instant::now();
    let mut missing: BTreeMap<String, usize> = BTreeMap::new();
    let mut loaded = 0;
    for path in paths.iter().filter(|path| is_elf(path)) {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let loader = new_loader();
        match dlopen_without_constructors(&loader, &name) {
            Ok(_) => loaded += 1,
            Err(error) => assert!(error.to_string().contains("PIE executable"), "{name}: {error}"),
        }
        for dependency in loader.report().missing_dependencies {
            *missing.entry(dependency.library).or_default() += 1;
        }
    }
    eprintln!("corpus mapped and relocated {loaded} libraries in {:?}", started.elapsed());

    assert_eq!(loaded, 266);
    for absent in ["libkrypton.so", "libskity.so", "libtttext_lite.so", "libanimax.so", "liblynx.so", "libRealXBase.so"] {
        assert!(missing.contains_key(absent), "{absent} must be reported missing");
    }
}

#[test]
fn bionic_find_loaded_library_by_soname_satisfies_dt_needed_from_a_plugin_file() {
    let Some(_) = corpus_libraries() else { return };
    let direct = new_loader();
    dlopen_without_constructors(&direct, "libEffectCreatorJni.so").unwrap();
    assert!(direct.report().missing_dependencies.iter().any(|dependency| dependency.library == "libeffect.so"));

    let preloaded = new_loader();
    dlopen_without_constructors(&preloaded, "libeffect_plugin.so").unwrap();
    dlopen_without_constructors(&preloaded, "libEffectCreatorJni.so").unwrap();
    assert!(!preloaded.report().missing_dependencies.iter().any(|dependency| dependency.library == "libeffect.so"));
}

#[test]
fn glibc_documented_hash_values_match_gnu_and_sysv_hash_functions() {
    assert_eq!(gnu_hash(b""), 5381);
    assert_eq!(gnu_hash(b"printf"), 0x156b_2bb8);
    assert_eq!(super::symbol_table::elf_hash(b"printf"), 0x0779_05a6);
}

#[test]
fn bionic_process_relocation_general_resolves_weak_undefined_imports_to_zero_without_a_stub() {
    let Some(_) = corpus_libraries() else { return };
    let loader = new_loader();
    dlopen_without_constructors(&loader, "libc++_shared.so").unwrap();
    let imports = loader.report().unresolved_imports;

    assert!(imports.iter().any(|(_, name)| name == "__sF"), "non weak imports are reported");
    assert!(!imports.iter().any(|(_, name)| name == "__cxa_thread_atexit_impl"), "weak import must not be reported");
}
