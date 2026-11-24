use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use super::elf_image::is_arm64_elf;
use super::loader::Loader;
use super::loader_test_support::{corpus_dir, dlopen_without_constructors, TestProvider};

struct Node {
    name: String,
    needed: Vec<String>,
}

fn visit(name: &str, nodes: &BTreeMap<String, Node>, seen: &mut BTreeSet<String>, order: &mut Vec<String>) {
    if !seen.insert(name.to_string()) {
        return;
    }
    for needed in &nodes[name].needed {
        if nodes.contains_key(needed) {
            visit(needed, nodes, seen, order);
        }
    }
    order.push(name.to_string());
}

fn corpus_nodes(paths: &[PathBuf]) -> (BTreeMap<String, Node>, Vec<String>, Vec<String>) {
    let mut nodes = BTreeMap::new();
    let (mut payloads, mut executables) = (Vec::new(), Vec::new());
    for path in paths {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let data = fs::read(path).unwrap();
        if !is_arm64_elf(&data) {
            payloads.push(name);
            continue;
        }
        match Loader::load_image(&path.to_string_lossy(), &data) {
            Ok(image) => {
                nodes.insert(name.clone(), Node { name, needed: image.dynamic.needed.clone() });
            }
            Err(error) => {
                assert!(error.to_string().contains("PIE executable"), "{name}: {error}");
                executables.push(name);
            }
        }
    }
    (nodes, payloads, executables)
}

#[test]
fn measured_tiktok_corpus_loads_in_dependency_order_into_one_loader_and_summarises_unresolved_imports() {
    let dir = corpus_dir();
    let Ok(entries) = fs::read_dir(&dir) else {
        eprintln!("skipped: TikTok library extraction not found at {}", dir.display());
        return;
    };
    let mut paths: Vec<PathBuf> = entries.filter_map(|entry| entry.ok().map(|entry| entry.path())).collect();
    paths.sort();
    let (nodes, payloads, executables) = corpus_nodes(&paths);
    let (mut seen, mut order) = (BTreeSet::new(), Vec::new());
    for name in nodes.keys() {
        visit(name, &nodes, &mut seen, &mut order);
    }

    let loader = Loader::new(vec![dir], Box::new(TestProvider::new())).with_strict(false);
    for name in &order {
        dlopen_without_constructors(&loader, name).unwrap_or_else(|error| panic!("{name}: {error}"));
    }
    let report = loader.report();
    let mut per_library: BTreeMap<String, usize> = order.iter().map(|name| (name.clone(), 0)).collect();
    for (importer, _) in &report.unresolved_imports {
        let name = importer.rsplit('/').next().unwrap();
        *per_library.get_mut(name).unwrap_or_else(|| panic!("unresolved import reported for unknown importer {importer}")) += 1;
    }
    let mut ranked: Vec<_> = per_library.iter().collect();
    ranked.sort_by(|left, right| right.1.cmp(left.1).then(left.0.cmp(right.0)));
    eprintln!("corpus loaded {} libraries in dependency order into one loader", order.len());
    eprintln!("skipped {} non ELF payloads and {} PIE executables: {executables:?}", payloads.len(), executables.len());
    eprintln!("{} missing dependency edges, {} unresolved imports in total", report.missing_dependencies.len(), report.unresolved_imports.len());
    for (name, count) in &ranked {
        eprintln!("unresolved {count:>5} {name}");
    }

    assert_eq!(order.len(), 266);
    assert_eq!(payloads.len(), 29);
    assert_eq!(executables, ["libnpth_dumper.so", "libnpth_logcat.so"]);
    assert_eq!(per_library.values().sum::<usize>(), report.unresolved_imports.len());
    assert!(per_library["libc++_shared.so"] > 0, "libc++_shared imports names the test provider does not serve");
    assert!(per_library.values().any(|count| *count == 0), "some library resolves every import");
    let position: BTreeMap<&str, usize> = order.iter().enumerate().map(|(index, name)| (name.as_str(), index)).collect();
    let forward_edges = nodes
        .values()
        .flat_map(|node| node.needed.iter().filter(|needed| nodes.contains_key(*needed)).map(move |needed| (node.name.as_str(), needed.as_str())))
        .filter(|(importer, needed)| position[needed] > position[importer])
        .count();
    assert!(forward_edges < 10, "{forward_edges} dependencies load after their importer");
}
