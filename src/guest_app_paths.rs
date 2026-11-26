// cite: frameworks-base ApplicationInfo.java#sourceDir, nativeLibraryDir
pub fn package_root(package: &str) -> String {
    format!("/data/app/~~winedroid==/{package}-winedroid==")
}

pub fn base_apk(package: &str) -> String {
    format!("{}/base.apk", package_root(package))
}

pub fn native_library_dir(package: &str) -> String {
    format!("{}/lib/arm64", package_root(package))
}

pub(crate) fn native_library_name<'a>(package: &str, path: &'a str) -> Option<&'a str> {
    let root = native_library_dir(package);
    let file = path.strip_prefix(root.as_str())?.strip_prefix('/')?;
    (!file.is_empty() && !file.contains(['/', '\0']) && !matches!(file, "." | "..")).then_some(file)
}
