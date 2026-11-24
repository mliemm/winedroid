use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

use super::fixture_header::manifest_path;

const VARIANTS: [(&str, u32); 2] = [("16k", 16384), ("4k", 4096)];
const LIBRARIES: [&str; 8] = ["fx_add", "fx_aps2", "fx_dep", "fx_jni", "fx_order_child", "fx_order_parent", "fx_reloc", "fx_relr"];
const SOURCES: [&str; 6] = ["fx_add", "fx_dep", "fx_jni", "fx_order_child", "fx_order_parent", "fx_reloc"];
const FAKE_CLANG: &str = r#"#!/bin/sh
printf 'clang %s\n' "$*" >>"$FAKE_LOG"
case "$1" in
--version) echo "clang version 1.2.3"; exit 0 ;;
-print-resource-dir) echo /fake-resource; exit 0 ;;
esac
out=""
prev=""
for argument in "$@"; do
    [ "$prev" = "-o" ] && out="$argument"
    prev="$argument"
done
[ -n "$out" ] && echo object >"$out"
exit 0
"#;
const FAKE_LLD: &str = r#"#!/bin/sh
printf 'lld %s\n' "$*" >>"$FAKE_LOG"
case "$1" in
--version) echo "LLD 4.5.6"; exit 0 ;;
esac
out=""
prev=""
for argument in "$@"; do
    [ "$prev" = "-o" ] && out="$argument"
    prev="$argument"
done
[ -n "$out" ] && echo "shared object $out" >"$out"
exit 0
"#;

fn write_tool(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn run_script() -> (String, Vec<String>) {
    let root = std::env::temp_dir().join(format!("winedroid-fixture-script-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    write_tool(&root.join("clang"), FAKE_CLANG);
    write_tool(&root.join("ld.lld"), FAKE_LLD);
    let log = root.join("commands.log");
    fs::write(&log, "").unwrap();
    let output = root.join("out");
    let status = Command::new("bash")
        .arg(manifest_path("scripts/build_native_fixtures.sh"))
        .env("CLANG", root.join("clang"))
        .env("LD_LLD", root.join("ld.lld"))
        .env("ANDROID_REF_ROOT", root.join("ref"))
        .env("FIXTURE_OUT_ROOT", &output)
        .env("FAKE_LOG", &log)
        .status()
        .expect("build script should start");
    assert!(status.success(), "build_native_fixtures.sh failed against the recording tools");
    let commands = fs::read_to_string(&log).unwrap().lines().map(str::to_owned).collect();
    let manifest = fs::read_to_string(output.join("manifest.txt")).unwrap();
    fs::remove_dir_all(&root).unwrap();
    (manifest, commands)
}

fn recorded_commands() -> (&'static str, &'static [String]) {
    static RUN: OnceLock<(String, Vec<String>)> = OnceLock::new();
    let (manifest, commands) = RUN.get_or_init(run_script);
    (manifest, commands)
}

fn links_of<'a>(commands: &'a [String], variant: &str) -> Vec<&'a String> {
    let marker = format!("/{variant}/lib");
    commands.iter().filter(|line| line.starts_with("lld ") && line.contains(" -o ") && line.contains(&marker)).collect()
}

fn has_pair(line: &str, first: &str, second: &str) -> bool {
    line.contains(&format!("{first} {second}"))
}

#[test]
fn fixture_build_script_links_each_variant_with_its_own_page_size_and_hardening_flags() {
    let (_, commands) = recorded_commands();

    for (variant, page) in VARIANTS {
        let links = links_of(commands, variant);
        assert_eq!(links.len(), LIBRARIES.len(), "{variant} link count");
        for library in LIBRARIES {
            assert!(links.iter().any(|line| line.contains(&format!("/{variant}/lib{library}.so"))), "{variant} {library}");
        }
        for line in links {
            assert!(has_pair(line, "-z", &format!("max-page-size={page}")), "{variant} page size in {line}");
            assert_eq!(line.matches("max-page-size").count(), 1, "{variant} page size flag count in {line}");
            for flag in ["relro", "now", "noexecstack"] {
                assert!(has_pair(line, "-z", flag), "{variant} -z {flag} in {line}");
            }
            for flag in ["--hash-style=gnu", "--eh-frame-hdr", "-shared"] {
                assert!(line.split(' ').any(|word| word == flag), "{variant} {flag} in {line}");
            }
        }
    }
}

#[test]
fn fixture_build_script_packs_dynamic_relocations_only_for_the_aps2_and_relr_variants() {
    let (_, commands) = recorded_commands();

    for (variant, _) in VARIANTS {
        for line in links_of(commands, variant) {
            let packed = line.split(' ').find(|word| word.starts_with("--pack-dyn-relocs="));
            let expected = if line.contains("/libfx_aps2.so") {
                Some("--pack-dyn-relocs=android")
            } else if line.contains("/libfx_relr.so") {
                Some("--pack-dyn-relocs=relr")
            } else {
                None
            };
            assert_eq!(packed, expected, "{variant} packing in {line}");
        }
    }
}

#[test]
fn fixture_build_script_compiles_every_source_per_variant_with_stack_protector_and_bionic_headers() {
    let (_, commands) = recorded_commands();
    let compiles: Vec<&String> = commands.iter().filter(|line| line.starts_with("clang ") && line.contains("/src/fx_")).collect();

    assert_eq!(compiles.len(), SOURCES.len() * VARIANTS.len());
    for source in SOURCES {
        assert_eq!(compiles.iter().filter(|line| line.contains(&format!("/src/{source}.c"))).count(), VARIANTS.len(), "{source}");
    }
    for line in compiles {
        for flag in ["--target=aarch64-linux-android29", "-fstack-protector-strong", "-fno-emulated-tls", "-fPIC", "-ffreestanding"] {
            assert!(line.split(' ').any(|word| word == flag), "{flag} in {line}");
        }
        assert!(line.contains("/ref/bionic/libc/include"), "bionic include in {line}");
        assert!(line.contains("tests/native_fixtures/include"), "fixture include in {line}");
    }
}

#[test]
fn fixture_build_script_links_the_dependency_fixtures_against_their_imports() {
    let (_, commands) = recorded_commands();

    for (variant, _) in VARIANTS {
        let links = links_of(commands, variant);
        let find = |library: &str| links.iter().find(|line| line.contains(&format!("/lib{library}.so"))).unwrap();
        assert!(find("fx_dep").split(' ').any(|word| word == "-lfx_add"));
        assert!(find("fx_dep").split(' ').any(|word| word == "-ldl"));
        assert!(find("fx_reloc").split(' ').any(|word| word == "-llog"));
        assert!(find("fx_add").split(' ').all(|word| word != "-lc"));
        assert!(find("fx_order_parent").split(' ').any(|word| word == "-lfx_order_child"));
        assert!(find("fx_order_parent").contains("-rpath $ORIGIN/dep"));
        assert!(find("fx_order_child").split(' ').all(|word| word != "-rpath"));
    }
}

#[test]
fn fixture_build_script_manifest_lists_the_tools_sources_and_every_binary_it_built() {
    let (manifest, _) = recorded_commands();
    let lines: Vec<&str> = manifest.lines().collect();

    assert_eq!(lines[0], "clang clang version 1.2.3");
    assert_eq!(lines[1], "ld.lld LLD 4.5.6");
    assert_eq!(lines.iter().filter(|line| line.starts_with("source ")).count(), SOURCES.len() + 1);
    for (variant, _) in VARIANTS {
        for library in LIBRARIES {
            assert!(lines.iter().any(|line| line.starts_with(&format!("binary {variant}/lib{library}.so "))), "{variant} {library}");
        }
    }
}
