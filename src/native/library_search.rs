use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

pub struct OpenedLibrary {
    pub path: PathBuf,
    pub identity: (u64, u64),
}

pub fn expand_runpath(runpath: &str, library_path: &str) -> Vec<PathBuf> {
    // cite: bionic linker_soinfo.cpp#set_dt_runpath
    let origin = Path::new(library_path).parent().map(|parent| parent.to_string_lossy().into_owned()).unwrap_or_default();
    runpath
        .split(':')
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            entry
                .replace("${ORIGIN}", &origin)
                .replace("$ORIGIN", &origin)
                .replace("${LIB}", "lib64")
                .replace("$LIB", "lib64")
        })
        .filter_map(|entry| fs::canonicalize(entry).ok())
        .collect()
}

pub fn open_library(name: &str, library_dirs: &[PathBuf], needed_by_runpath: &[PathBuf]) -> Option<OpenedLibrary> {
    // cite: bionic linker.cpp#open_library
    let candidates: Vec<PathBuf> = if name.contains('/') {
        vec![PathBuf::from(name)]
    } else {
        library_dirs.iter().chain(needed_by_runpath).map(|dir| dir.join(name)).collect()
    };
    candidates.into_iter().find_map(|candidate| {
        let metadata = fs::metadata(&candidate).ok().filter(|metadata| metadata.is_file())?;
        let path = fs::canonicalize(&candidate).ok()?;
        Some(OpenedLibrary { path, identity: (metadata.dev(), metadata.ino()) })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("winedroid-search-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::canonicalize(dir).unwrap()
    }

    #[test]
    fn bionic_set_dt_runpath_expands_origin_and_drops_missing_directories() {
        let root = scratch("expand");
        fs::create_dir_all(root.join("plugins/deps")).unwrap();
        let library = root.join("plugins/libhost.so");

        let expanded = expand_runpath("$ORIGIN/deps:${ORIGIN}:$ORIGIN/absent::", library.to_str().unwrap());

        assert_eq!(expanded, vec![root.join("plugins/deps"), root.join("plugins")]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bionic_open_library_searches_the_runpath_after_the_library_directories() {
        let root = scratch("open");
        let (primary, runpath) = (root.join("primary"), root.join("runpath"));
        fs::create_dir_all(&primary).unwrap();
        fs::create_dir_all(&runpath).unwrap();
        fs::write(runpath.join("libonly_runpath.so"), b"x").unwrap();
        fs::write(runpath.join("libboth.so"), b"x").unwrap();
        fs::write(primary.join("libboth.so"), b"y").unwrap();

        let only = open_library("libonly_runpath.so", std::slice::from_ref(&primary), std::slice::from_ref(&runpath)).unwrap();
        let both = open_library("libboth.so", std::slice::from_ref(&primary), std::slice::from_ref(&runpath)).unwrap();

        assert_eq!(only.path, runpath.join("libonly_runpath.so"));
        assert_eq!(both.path, primary.join("libboth.so"));
        assert!(open_library("libonly_runpath.so", std::slice::from_ref(&primary), &[]).is_none());
        fs::remove_dir_all(root).unwrap();
    }
}
