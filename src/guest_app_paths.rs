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
