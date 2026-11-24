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
