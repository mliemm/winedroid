use std::ffi::CString;
use std::fs;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::capability_fs::{Directory, Metadata};
use crate::error::{Result, WinedroidError};
use crate::external_storage::{
    ExternalStorageAccess, ExternalStoragePolicy, GUEST_PRIMARY_EXTERNAL_ALIAS,
    GUEST_PRIMARY_EXTERNAL_ROOT,
};

const GUEST_CREDENTIAL_PROTECTED_ROOT_PREFIX: &str = "/data/user/0";
const GUEST_DEVICE_PROTECTED_ROOT_PREFIX: &str = "/data/user_de/0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestPathError {
    OutsideMounts,
    EscapesRoot,
    NulByte,
    HiddenDirectory,
    ExternalScopeDenied,
}

#[derive(Debug)]
pub struct GuestPathFailure {
    pub kind: GuestPathError,
    pub error: WinedroidError,
}

fn failure(kind: GuestPathError, message: impl Into<String>) -> GuestPathFailure {
    GuestPathFailure {
        kind,
        error: WinedroidError::new(message),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivateStorageKind {
    CredentialProtected,
    DeviceProtected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestMount {
    CredentialProtected,
    DeviceProtected,
    Shared,
    AppCode,
}

#[derive(Debug, Clone)]
pub struct ResolvedGuestPath {
    mount: GuestMount,
    root: Directory,
    components: Arc<[CString]>,
    #[cfg(any(test, target_os = "emscripten"))]
    _host_path: PathBuf,
    mutation_protected: bool,
}

impl PartialEq for ResolvedGuestPath {
    fn eq(&self, other: &Self) -> bool {
        self.mount == other.mount
            && self.components == other.components
            && self.mutation_protected == other.mutation_protected
    }
}

impl Eq for ResolvedGuestPath {}

impl ResolvedGuestPath {
    pub fn mount(&self) -> GuestMount {
        self.mount
    }

    #[cfg(any(test, target_os = "emscripten"))]
    pub fn host_path(&self) -> &Path {
        &self._host_path
    }

    pub fn is_mutation_protected(&self) -> bool {
        self.mutation_protected
    }

    pub fn metadata(&self) -> io::Result<Metadata> {
        self.root.metadata(&self.components)
    }

    pub fn volume_stats(&self) -> io::Result<crate::capability_fs::VolumeStats> {
        self.root.volume_stats(&self.components)
    }

    pub fn can_access(&self, write: bool) -> io::Result<bool> {
        if write && self.mount == GuestMount::AppCode {
            return Ok(false);
        }
        self.root.can_access(&self.components, write)
    }

    pub fn open_read(&self) -> io::Result<File> {
        self.root.open_read(&self.components)
    }

    pub fn read(&self) -> io::Result<Vec<u8>> {
        self.root.read(&self.components)
    }

    pub fn open_truncate(&self) -> io::Result<crate::fs_notify::NotifiedFile> {
        self.require_writable_mount()?;
        self.root.open_truncate(&self.components)
    }

    pub fn open_append(&self) -> io::Result<crate::fs_notify::NotifiedFile> {
        self.require_writable_mount()?;
        self.root.open_append(&self.components)
    }

    pub fn create_new(&self) -> io::Result<crate::fs_notify::NotifiedFile> {
        self.require_writable_mount()?;
        self.root.create_new(&self.components)
    }

    pub fn directory_key(&self) -> io::Result<crate::fs_notify::InodeKey> {
        self.root.directory_key(&self.components)
    }

    pub fn mkdir(&self) -> io::Result<()> {
        self.require_writable_mount()?;
        self.root.mkdir(&self.components)
    }

    pub fn mkdir_all(&self) -> io::Result<bool> {
        self.require_writable_mount()?;
        self.root.mkdir_all(&self.components)
    }

    pub fn clear_write_permission(&self) -> io::Result<()> {
        self.require_writable_mount()?;
        self.root.clear_write_permission(&self.components)
    }

    pub fn remove(&self) -> io::Result<()> {
        self.require_writable_mount()?;
        self.root.remove(&self.components)
    }

    pub fn rename_to(&self, destination: &ResolvedGuestPath) -> io::Result<()> {
        self.require_writable_mount()?;
        destination.require_writable_mount()?;
        self.root
            .rename(&self.components, &destination.root, &destination.components)
    }

    pub fn read_dir_names(&self) -> io::Result<Vec<String>> {
        self.root.read_dir_names(&self.components)
    }

    pub fn atomic_replace(&self, bytes: &[u8]) -> io::Result<()> {
        self.require_writable_mount()?;
        self.root.atomic_replace(&self.components, bytes)
    }

    pub fn parent_and_leaf(&self) -> io::Result<(Directory, Option<CString>)> {
        self.root.parent_and_leaf(&self.components)
    }

    pub fn directory_capability_and_leaf(&self) -> io::Result<(Directory, CString)> {
        self.require_writable_mount()?;
        self.root.parent_directory_and_leaf(&self.components)
    }

    fn require_writable_mount(&self) -> io::Result<()> {
        if self.mount == GuestMount::AppCode {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "installed app code is read-only",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct GuestFilesystem {
    package_name: String,
    credential_protected_guest_root: String,
    #[cfg(any(test, target_os = "emscripten"))]
    _credential_protected_host_root: PathBuf,
    credential_protected_root: Directory,
    device_protected_guest_root: String,
    #[cfg(any(test, target_os = "emscripten"))]
    _device_protected_host_root: PathBuf,
    device_protected_root: Directory,
    #[cfg(any(test, target_os = "emscripten"))]
    _shared_host_root: PathBuf,
    shared_root: Directory,
    app_root: Directory,
    #[cfg(any(test, target_os = "emscripten"))]
    app_host_root: PathBuf,
}

impl GuestFilesystem {
    pub fn new(
        package_name: impl Into<String>,
        credential_protected_host_root: impl AsRef<Path>,
        device_protected_host_root: impl AsRef<Path>,
        shared_host_root: impl AsRef<Path>,
    ) -> Result<Self> {
        let package_name = package_name.into();
        let credential_protected_host_path = credential_protected_host_root.as_ref();
        let device_protected_host_path = device_protected_host_root.as_ref();
        let shared_host_path = shared_host_root.as_ref();
        let prefix_path = shared_host_path.parent().ok_or_else(|| {
            WinedroidError::new(format!(
                "shared storage directory `{}` has no prefix parent",
                shared_host_path.display()
            ))
        })?;
        let canonical_prefix = canonical_followed_directory(prefix_path, "prefix")?;
        let apps_path = prefix_path.join("apps");
        canonical_real_directory(&apps_path, "apps")?;
        require_exact_child_name(&apps_path, &package_name)?;
        let app_path = apps_path.join(&package_name);
        canonical_real_directory(&app_path, "app")?;

        let credential_protected_host_root = canonical_directory(
            credential_protected_host_path,
            "credential-protected app data",
        )?;
        let expected_credential_protected_root = canonical_prefix
            .join("apps")
            .join(&package_name)
            .join("data");
        if credential_protected_host_root != expected_credential_protected_root {
            return Err(WinedroidError::new(format!(
                "credential-protected app data directory `{}` escapes its prefix layout",
                credential_protected_host_path.display()
            )));
        }
        let device_protected_host_root =
            canonical_directory(device_protected_host_path, "device-protected app data")?;
        let expected_device_protected_root = canonical_prefix
            .join("apps")
            .join(&package_name)
            .join("data_de");
        if device_protected_host_root != expected_device_protected_root {
            return Err(WinedroidError::new(format!(
                "device-protected app data directory `{}` escapes its prefix layout",
                device_protected_host_path.display()
            )));
        }
        let shared_host_root = canonical_directory(shared_host_path, "shared storage")?;
        if shared_host_root != canonical_prefix.join("storage") {
            return Err(WinedroidError::new(format!(
                "shared storage directory `{}` escapes its prefix layout",
                shared_host_path.display()
            )));
        }

        let prefix_root = Directory::open_ambient(&canonical_prefix).map_err(|error| {
            WinedroidError::with_source(
                format!(
                    "failed to acquire prefix directory capability `{}`",
                    canonical_prefix.display()
                ),
                error,
            )
        })?;
        let apps_root = open_capability_child(&prefix_root, "apps", "apps")?;
        require_exact_child_name_capability(&apps_root, &package_name)?;
        let app_root = open_capability_child(&apps_root, &package_name, "app")?;
        let credential_protected_root =
            open_capability_child(&app_root, "data", "credential-protected app data")?;
        let device_protected_root =
            open_capability_child(&app_root, "data_de", "device-protected app data")?;
        let shared_root = open_capability_child(&prefix_root, "storage", "shared storage")?;

        // cite: frameworks-base Environment.java#getDataUserCePackageDirectory, getDataUserDePackageDirectory, DIR_USER_CE, DIR_USER_DE
        Ok(Self {
            credential_protected_guest_root: format!(
                "{GUEST_CREDENTIAL_PROTECTED_ROOT_PREFIX}/{package_name}"
            ),
            device_protected_guest_root: format!(
                "{GUEST_DEVICE_PROTECTED_ROOT_PREFIX}/{package_name}"
            ),
            #[cfg(any(test, target_os = "emscripten"))]
            app_host_root: canonical_prefix.join("apps").join(&package_name),
            package_name,
            #[cfg(any(test, target_os = "emscripten"))]
            _credential_protected_host_root: credential_protected_host_root,
            credential_protected_root,
            #[cfg(any(test, target_os = "emscripten"))]
            _device_protected_host_root: device_protected_host_root,
            device_protected_root,
            #[cfg(any(test, target_os = "emscripten"))]
            _shared_host_root: shared_host_root,
            shared_root,
            app_root,
        })
    }

    pub fn private_guest_path(&self, storage: PrivateStorageKind, child: &str) -> String {
        let root = match storage {
            PrivateStorageKind::CredentialProtected => &self.credential_protected_guest_root,
            PrivateStorageKind::DeviceProtected => &self.device_protected_guest_root,
        };
        if child.is_empty() {
            return root.clone();
        }

        format!("{root}/{child}")
    }

    pub fn resolve(
        &self,
        guest_path: &str,
        access: ExternalStorageAccess,
        external_storage_policy: &ExternalStoragePolicy,
    ) -> Result<ResolvedGuestPath> {
        self.resolve_typed(guest_path, access, external_storage_policy)
            .map_err(|failure| failure.error)
    }

    pub fn resolve_typed(
        &self,
        guest_path: &str,
        access: ExternalStorageAccess,
        external_storage_policy: &ExternalStoragePolicy,
    ) -> std::result::Result<ResolvedGuestPath, GuestPathFailure> {
        let normalized = rewrite_guest_alias(&mount_match_path(guest_path)?);
        if let Some(relative) = self.app_code_relative_path(&normalized) {
            if access == ExternalStorageAccess::Write {
                return Err(failure(
                    GuestPathError::OutsideMounts,
                    "installed app code is read-only",
                ));
            }
            let mut path = self.resolve_under(
                GuestMount::AppCode,
                &self.app_root,
                &relative,
                guest_path,
                false,
                access,
                external_storage_policy,
            )?;
            path.mutation_protected = true;
            return Ok(path);
        }

        if let Some(relative) =
            strip_path_prefix(&normalized, &self.credential_protected_guest_root)
        {
            return self.resolve_under(
                GuestMount::CredentialProtected,
                &self.credential_protected_root,
                relative,
                guest_path,
                false,
                access,
                external_storage_policy,
            );
        }

        if let Some(relative) = strip_path_prefix(&normalized, &self.device_protected_guest_root) {
            return self.resolve_under(
                GuestMount::DeviceProtected,
                &self.device_protected_root,
                relative,
                guest_path,
                false,
                access,
                external_storage_policy,
            );
        }

        for mount in [GUEST_PRIMARY_EXTERNAL_ROOT, GUEST_PRIMARY_EXTERNAL_ALIAS] {
            if let Some(relative) = strip_path_prefix(&normalized, mount) {
                // cite: frameworks-base Environment.java#UserEnvironment, buildExternalStorageAppFilesDirs, buildExternalStorageAppCacheDirs
                return self.resolve_under(
                    GuestMount::Shared,
                    &self.shared_root,
                    relative,
                    guest_path,
                    true,
                    access,
                    external_storage_policy,
                );
            }
        }

        Err(failure(
            GuestPathError::OutsideMounts,
            format!(
                "guest path `{guest_path}` is outside the app-private and shared-storage mounts"
            ),
        ))
    }

    fn app_code_relative_path(&self, path: &str) -> Option<String> {
        // cite: frameworks-base ApplicationInfo.java#sourceDir, nativeLibraryDir
        if path == crate::guest_app_paths::base_apk(&self.package_name) {
            return Some("source.apk".to_owned());
        }
        if path == crate::guest_app_paths::native_library_dir(&self.package_name) {
            return Some("native/arm64-v8a".to_owned());
        }
        crate::guest_app_paths::native_library_name(&self.package_name, path)
            .map(|name| format!("native/arm64-v8a/{name}"))
    }

    fn resolve_under(
        &self,
        mount: GuestMount,
        root: &Directory,
        relative: &str,
        guest_path: &str,
        enforce_external_app_scope: bool,
        access: ExternalStorageAccess,
        external_storage_policy: &ExternalStoragePolicy,
    ) -> std::result::Result<ResolvedGuestPath, GuestPathFailure> {
        #[cfg(any(test, target_os = "emscripten"))]
        let mut host_path = self.test_host_root(mount).to_path_buf();
        let mut resolved_components = Vec::new();
        let mut operation_components = Vec::new();
        for component in relative.split('/') {
            match component {
                "" | "." => continue,
                ".." => {
                    if resolved_components.pop().is_none() {
                        return Err(failure(
                            GuestPathError::EscapesRoot,
                            format!("guest path `{guest_path}` escapes its mounted root"),
                        ));
                    }
                    #[cfg(any(test, target_os = "emscripten"))]
                    host_path.push("..");
                    operation_components.push(CString::new("..").expect("literal has no NUL"));
                    continue;
                }
                _ => {}
            }

            resolved_components.push(component);
            operation_components
                .push(CString::new(component).expect("guest path was checked for embedded NUL"));
            if enforce_external_app_scope && resolved_components.len() >= 3 {
                self.ensure_shared_app_scope(
                    &resolved_components.join("/"),
                    guest_path,
                    access,
                    external_storage_policy,
                )?;
            }

            #[cfg(any(test, target_os = "emscripten"))]
            host_path.push(component);
        }

        if enforce_external_app_scope {
            self.ensure_shared_app_scope(
                &resolved_components.join("/"),
                guest_path,
                access,
                external_storage_policy,
            )?;
        }

        let protects_android_privacy_root = mount == GuestMount::Shared
            && resolved_components.len() == 1
            && resolved_components[0].eq_ignore_ascii_case("Android");

        Ok(ResolvedGuestPath {
            mount,
            root: root.clone(),
            components: operation_components.into(),
            #[cfg(any(test, target_os = "emscripten"))]
            _host_path: host_path,
            mutation_protected: resolved_components.is_empty() || protects_android_privacy_root,
        })
    }

    #[cfg(any(test, target_os = "emscripten"))]
    fn test_host_root(&self, mount: GuestMount) -> &Path {
        match mount {
            GuestMount::CredentialProtected => &self._credential_protected_host_root,
            GuestMount::DeviceProtected => &self._device_protected_host_root,
            GuestMount::Shared => &self._shared_host_root,
            GuestMount::AppCode => &self.app_host_root,
        }
    }

    fn ensure_shared_app_scope(
        &self,
        relative: &str,
        guest_path: &str,
        access: ExternalStorageAccess,
        external_storage_policy: &ExternalStoragePolicy,
    ) -> std::result::Result<(), GuestPathFailure> {
        let components = relative.split('/').collect::<Vec<_>>();
        let android_root_has_wrong_case = components
            .first()
            .is_some_and(|component| component.eq_ignore_ascii_case("Android"))
            && components.first().copied() != Some("Android");
        if android_root_has_wrong_case {
            return Err(failure(
                GuestPathError::HiddenDirectory,
                format!("guest path `{guest_path}` uses a non-canonical Android storage directory"),
            ));
        }

        // cite: frameworks-base Context.java#getExternalFilesDir, getExternalCacheDir, getExternalMediaDirs, getObbDir
        let app_specific = components.len() >= 3
            && components[0] == "Android"
            && matches!(components[1], "data" | "obb" | "media")
            && components[2] == self.package_name;
        if app_specific {
            return Ok(());
        }

        let app_specific_tree = components.len() >= 2
            && components[0].eq_ignore_ascii_case("Android")
            && (components[1].eq_ignore_ascii_case("data")
                || components[1].eq_ignore_ascii_case("obb")
                || components[1].eq_ignore_ascii_case("sandbox"));
        if app_specific_tree {
            // cite: frameworks-base FileSystemProvider.java#PATTERN_HIDDEN_PATH, shouldHide
            return Err(failure(
                GuestPathError::HiddenDirectory,
                format!("guest path `{guest_path}` addresses an unavailable app-specific external directory"),
            ));
        }

        if external_storage_policy.allows_public_path(access) {
            return Ok(());
        }

        Err(failure(
            GuestPathError::ExternalScopeDenied,
            format!("guest path `{guest_path}` requires effective legacy read/write or all-files access; scoped media access requires indexed ownership and media grants"),
        ))
    }
}

pub fn normalize_guest_file_path(path: &str) -> String {
    // cite: libcore UnixFileSystem.java#normalize
    let mut normalized = String::with_capacity(path.len());
    let mut previous_was_slash = false;
    for character in path.chars() {
        if character == '/' && previous_was_slash {
            continue;
        }
        normalized.push(character);
        previous_was_slash = character == '/';
    }

    if normalized.len() > 1 && normalized.ends_with('/') {
        normalized.pop();
    }

    normalized
}

pub fn absolute_guest_path(path: &str) -> String {
    // cite: libcore UnixFileSystem.java#resolve
    // cite: libcore System.java#getProperties
    let path = normalize_guest_file_path(path);
    if path.starts_with('/') {
        return path;
    }
    if path.is_empty() {
        return "/".to_owned();
    }

    format!("/{path}")
}

pub fn canonical_guest_path(path: &str) -> Result<String> {
    // cite: libcore canonicalize_md.c#canonicalize, collapse
    if path.contains('\0') {
        return Err(WinedroidError::new("guest path contains a NUL character"));
    }

    let absolute = absolute_guest_path(path);
    let mut components = Vec::new();
    for component in absolute.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                components.pop();
            }
            component => components.push(component),
        }
    }

    if components.is_empty() {
        return Ok("/".to_owned());
    }

    Ok(format!("/{}", components.join("/")))
}

pub fn rewrite_guest_alias(path: &str) -> String {
    // cite: frameworks-base StrictMode.java#onCredentialProtectedPathAccess
    for (alias, target) in [
        ("/data/data", "/data/user/0"),
        ("/storage/self/primary", "/storage/emulated/0"),
        ("/mnt/sdcard", "/storage/emulated/0"),
    ] {
        if path == alias {
            return target.to_string();
        }
        if let Some(rest) = path
            .strip_prefix(alias)
            .and_then(|rest| rest.strip_prefix('/'))
        {
            return format!("{target}/{rest}");
        }
    }
    path.to_string()
}

fn mount_match_path(path: &str) -> std::result::Result<String, GuestPathFailure> {
    if path.contains('\0') {
        return Err(failure(
            GuestPathError::NulByte,
            "guest path contains a NUL character",
        ));
    }

    let absolute = absolute_guest_path(path);
    let components = absolute
        .split('/')
        .filter(|component| !matches!(*component, "" | "."))
        .collect::<Vec<_>>();
    if components.is_empty() {
        return Ok("/".to_owned());
    }

    Ok(format!("/{}", components.join("/")))
}

fn strip_path_prefix<'a>(path: &'a str, prefix: &str) -> Option<&'a str> {
    if path == prefix {
        return Some("");
    }

    path.strip_prefix(prefix)?.strip_prefix('/')
}

fn canonical_directory(path: &Path, label: &str) -> Result<PathBuf> {
    canonical_real_directory(path, label)
}

fn canonical_real_directory(path: &Path, label: &str) -> Result<PathBuf> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        WinedroidError::with_source(
            format!("failed to inspect {label} directory `{}`", path.display()),
            error,
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(WinedroidError::new(format!(
            "{label} path `{}` is not a real directory",
            path.display()
        )));
    }

    fs::canonicalize(path).map_err(|error| {
        WinedroidError::with_source(
            format!(
                "failed to canonicalize {label} directory `{}`",
                path.display()
            ),
            error,
        )
    })
}

fn canonical_followed_directory(path: &Path, label: &str) -> Result<PathBuf> {
    let metadata = fs::metadata(path).map_err(|error| {
        WinedroidError::with_source(
            format!("failed to inspect {label} directory `{}`", path.display()),
            error,
        )
    })?;
    if !metadata.is_dir() {
        return Err(WinedroidError::new(format!(
            "{label} path `{}` is not a directory",
            path.display()
        )));
    }

    fs::canonicalize(path).map_err(|error| {
        WinedroidError::with_source(
            format!(
                "failed to canonicalize {label} directory `{}`",
                path.display()
            ),
            error,
        )
    })
}

fn require_exact_child_name(parent: &Path, expected: &str) -> Result<()> {
    let entries = fs::read_dir(parent).map_err(|error| {
        WinedroidError::with_source(
            format!("failed to inspect apps directory `{}`", parent.display()),
            error,
        )
    })?;
    let mut found_exact = false;
    for entry in entries {
        let entry = entry.map_err(|error| {
            WinedroidError::with_source(
                format!("failed to inspect apps directory `{}`", parent.display()),
                error,
            )
        })?;
        let Some(name) = entry.file_name().to_str().map(ToOwned::to_owned) else {
            continue;
        };
        if name == expected {
            found_exact = true;
            continue;
        }
        if name.eq_ignore_ascii_case(expected) {
            return Err(WinedroidError::new(format!(
                "package directory `{name}` conflicts with package `{expected}`"
            )));
        }
    }

    if found_exact {
        return Ok(());
    }

    Err(WinedroidError::new(format!(
        "package directory `{expected}` is missing from `{}`",
        parent.display()
    )))
}

fn open_capability_child(parent: &Directory, name: &str, label: &str) -> Result<Directory> {
    let name = CString::new(name)
        .map_err(|_| WinedroidError::new(format!("{label} directory name contains NUL")))?;
    parent.open_child(&name).map_err(|error| {
        WinedroidError::with_source(
            format!("failed to acquire {label} directory capability"),
            error,
        )
    })
}

fn require_exact_child_name_capability(parent: &Directory, expected: &str) -> Result<()> {
    let names = parent.read_dir_names(&[]).map_err(|error| {
        WinedroidError::with_source("failed to inspect apps directory capability", error)
    })?;
    let mut found_exact = false;
    for name in names {
        if name == expected {
            found_exact = true;
            continue;
        }
        if name.eq_ignore_ascii_case(expected) {
            return Err(WinedroidError::new(format!(
                "package directory `{name}` conflicts with package `{expected}`"
            )));
        }
    }
    if found_exact {
        return Ok(());
    }
    Err(WinedroidError::new(format!(
        "package directory `{expected}` is missing from apps directory capability"
    )))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    #[test]
    fn application_info_code_paths_allow_reads_and_refuse_writes() {
        // cite: frameworks-base ApplicationInfo.java#sourceDir, nativeLibraryDir
        let fixture = Fixture::new("code");
        let app = fixture.private.parent().unwrap();
        fs::write(app.join("source.apk"), b"archive").unwrap();
        fs::create_dir_all(app.join("native/arm64-v8a")).unwrap();
        fs::write(app.join("native/arm64-v8a/libtest.so"), b"library").unwrap();
        let files = fixture.filesystem();
        let policy = restricted_policy();
        let source = crate::guest_app_paths::base_apk("com.example");
        let libraries = crate::guest_app_paths::native_library_dir("com.example");
        for (path, bytes) in [
            (source, b"archive".as_slice()),
            (format!("{libraries}/libtest.so"), b"library".as_slice()),
        ] {
            let resolved = resolve_read(&files, &path, &policy).unwrap();
            assert_eq!(resolved.mount(), GuestMount::AppCode);
            assert_eq!(resolved.read().unwrap(), bytes);
            assert_eq!(
                resolved.open_truncate().unwrap_err().kind(),
                io::ErrorKind::PermissionDenied
            );
            assert!(files
                .resolve(&path, ExternalStorageAccess::Write, &policy)
                .is_err());
        }
        assert!(resolve_read(&files, &format!("{libraries}/../data"), &policy).is_err());
        assert!(resolve_read(
            &files,
            &crate::guest_app_paths::base_apk("com.other"),
            &policy
        )
        .is_err());
    }

    #[test]
    fn context_impl_private_path_maps_only_the_current_package() {
        let fixture = Fixture::new("private");
        let filesystem = fixture.filesystem();

        let resolved = resolve_read(
            &filesystem,
            "/data/user/0/com.example/files/state.txt",
            &restricted_policy(),
        )
        .expect("current app path should resolve");

        assert_eq!(resolved.mount(), GuestMount::CredentialProtected);
        assert_eq!(
            resolved.host_path(),
            fs::canonicalize(&fixture.private)
                .expect("private root should canonicalize")
                .join("files/state.txt")
        );
        assert!(resolve_read(
            &filesystem,
            "/data/user/0/com.other/files/state.txt",
            &restricted_policy(),
        )
        .is_err());
    }

    #[test]
    fn context_impl_device_protected_path_maps_only_the_current_package() {
        // cite: cts ContextTest.java#testCreateDeviceProtectedStorageContext
        // cite: frameworks-base Environment.java#getDataUserDePackageDirectory, DIR_USER_DE
        let fixture = Fixture::new("device_private");
        let filesystem = fixture.filesystem();

        let resolved = resolve_read(
            &filesystem,
            "/data/user_de/0/com.example/files/state.txt",
            &restricted_policy(),
        )
        .expect("current app device-protected path should resolve");

        assert_eq!(resolved.mount(), GuestMount::DeviceProtected);
        assert_eq!(
            resolved.host_path(),
            fs::canonicalize(&fixture.device_private)
                .expect("device-protected root should canonicalize")
                .join("files/state.txt")
        );
        assert!(resolve_read(
            &filesystem,
            "/data/user_de/0/com.other/files/state.txt",
            &restricted_policy(),
        )
        .is_err());
        assert!(resolve_read(
            &filesystem,
            "/data/user_de/0/com.example/../../user/0/com.example/files/state.txt",
            &restricted_policy(),
        )
        .is_err());
    }

    #[test]
    fn environment_primary_and_sdcard_app_paths_share_one_mount() {
        let fixture = Fixture::new("aliases");
        let filesystem = fixture.filesystem();

        let primary = resolve_read(
            &filesystem,
            "/storage/emulated/0/Android/data/com.example/files/file.txt",
            &restricted_policy(),
        )
        .expect("primary external path should resolve");
        let alias = resolve_read(
            &filesystem,
            "/sdcard/Android/data/com.example/files/file.txt",
            &restricted_policy(),
        )
        .expect("sdcard alias should resolve");

        assert_eq!(primary.mount(), GuestMount::Shared);
        assert_eq!(primary, alias);
    }

    #[test]
    fn unix_filesystem_parent_components_work_without_escaping_a_mount() {
        let fixture = Fixture::new("parent_components");
        let filesystem = fixture.filesystem();

        let contained = resolve_read(
            &filesystem,
            "/data/user/0/com.example/files/../cache/value",
            &restricted_policy(),
        )
        .expect("contained parent component should resolve");

        assert_eq!(
            contained.host_path(),
            fs::canonicalize(&fixture.private)
                .expect("private root should canonicalize")
                .join("files/../cache/value")
        );
        assert!(resolve_read(
            &filesystem,
            "/data/user/0/com.example/../../com.other/value",
            &restricted_policy(),
        )
        .is_err());
        assert!(resolve_read(
            &filesystem,
            "/data/user/0/com.example/../com.example/cache/value",
            &restricted_policy(),
        )
        .is_err());
    }

    #[test]
    fn resolver_rejects_other_apps_external_directories() {
        let fixture = Fixture::new("external_app_scope");
        let filesystem = fixture.filesystem();

        // cite: frameworks-base Context.java#getExternalFilesDir, getExternalCacheDir, getExternalMediaDirs, getObbDir
        let policy = restricted_policy();
        assert!(resolve_read(
            &filesystem,
            "/storage/emulated/0/Android/data/com.other/files/value",
            &policy,
        )
        .is_err());
        assert!(resolve_read(
            &filesystem,
            "/storage/emulated/0/android/data/com.other/files/value",
            &policy,
        )
        .is_err());
        assert!(resolve_read(
            &filesystem,
            "/storage/emulated/0/Android/data/com.other/../com.example/files/value",
            &policy,
        )
        .is_err());
        assert!(resolve_read(
            &filesystem,
            "/storage/emulated/0/Android/data/com.example/files/value",
            &policy,
        )
        .is_ok());
        assert!(resolve_read(
            &filesystem,
            "/storage/emulated/0/Android/obb/com.example/value",
            &policy,
        )
        .is_ok());
        assert!(resolve_read(
            &filesystem,
            "/storage/emulated/0/Android/media/com.example/value",
            &policy,
        )
        .is_ok());
        assert!(resolve_read(&filesystem, "/storage/emulated/0/Download/value", &policy,).is_err());
        assert!(resolve_read(
            &filesystem,
            "/storage/emulated/0/Android/sandbox/com.example/value",
            &policy,
        )
        .is_err());
    }

    #[test]
    fn environment_legacy_policy_distinguishes_public_reads_and_writes() {
        // cite: frameworks-base Environment.java#isExternalStorageLegacy
        // cite: frameworks-base AndroidManifest.xml#READ_EXTERNAL_STORAGE, WRITE_EXTERNAL_STORAGE
        let fixture = Fixture::new("legacy_public_access");
        let filesystem = fixture.filesystem();
        let read_policy = policy_with_runtime_access(
            28,
            vec![crate::external_storage::READ_EXTERNAL_STORAGE],
            true,
            false,
        );
        let write_policy = policy_with_runtime_access(
            28,
            vec![crate::external_storage::WRITE_EXTERNAL_STORAGE],
            true,
            false,
        );
        let path = "/storage/emulated/0/Download/value";

        assert!(filesystem
            .resolve(path, ExternalStorageAccess::Read, &read_policy)
            .is_ok());
        assert!(filesystem
            .resolve(path, ExternalStorageAccess::Write, &read_policy)
            .is_err());
        assert!(filesystem
            .resolve(path, ExternalStorageAccess::Read, &write_policy)
            .is_err());
        assert!(filesystem
            .resolve(path, ExternalStorageAccess::Write, &write_policy)
            .is_ok());
    }

    #[test]
    fn environment_all_files_access_still_excludes_other_app_data_and_obb() {
        // cite: frameworks-base Environment.java#isExternalStorageManager
        // cite: frameworks-base FileSystemProvider.java#PATTERN_HIDDEN_PATH, shouldHide
        let fixture = Fixture::new("all_files_privacy");
        let filesystem = fixture.filesystem();
        let policy = policy_with_runtime_access(34, Vec::new(), false, true);

        assert!(filesystem
            .resolve(
                "/storage/emulated/0/Documents/value",
                ExternalStorageAccess::Write,
                &policy,
            )
            .is_ok());
        assert!(filesystem
            .resolve(
                "/storage/emulated/0/Android/data/com.other/files/value",
                ExternalStorageAccess::Read,
                &policy,
            )
            .is_err());
        assert!(filesystem
            .resolve(
                "/storage/emulated/0/Android/obb/com.other/value",
                ExternalStorageAccess::Write,
                &policy,
            )
            .is_err());
    }

    #[cfg(unix)]
    #[test]
    fn capability_operations_reject_every_existing_symbolic_link_component() {
        let fixture = Fixture::new("symlink");
        let outside = fixture.root.join("outside");
        fs::create_dir_all(&outside).expect("outside directory should exist");
        std::os::unix::fs::symlink(&outside, fixture.private.join("escape"))
            .expect("test symlink should be created");
        let filesystem = fixture.filesystem();

        let direct = resolve_read(
            &filesystem,
            "/data/user/0/com.example/escape/secret.txt",
            &restricted_policy(),
        )
        .expect("policy resolution should succeed");
        assert!(direct.open_read().is_err());
        let before_parent = resolve_read(
            &filesystem,
            "/data/user/0/com.example/escape/../files/value.txt",
            &restricted_policy(),
        )
        .expect("policy resolution should succeed");
        assert!(before_parent.open_read().is_err());
    }

    #[cfg(unix)]
    #[test]
    fn mount_capability_survives_visible_root_replacement() {
        let fixture = Fixture::new("replaced_root");
        fs::create_dir(fixture.private.join("files")).expect("files directory should exist");
        let outside = fixture.root.join("outside");
        fs::create_dir_all(outside.join("files")).expect("outside files directory should exist");
        let filesystem = fixture.filesystem();
        let moved = fixture.root.join("original-private");

        fs::rename(&fixture.private, &moved).expect("private root should move");
        std::os::unix::fs::symlink(&outside, &fixture.private)
            .expect("replacement symlink should be created");
        let resolved = filesystem
            .resolve(
                "/data/user/0/com.example/files/value",
                ExternalStorageAccess::Write,
                &restricted_policy(),
            )
            .expect("guest path should resolve");
        resolved
            .open_truncate()
            .expect("held mount should remain writable");

        assert!(moved.join("files/value").is_file());
        assert!(!outside.join("files/value").exists());
    }

    #[cfg(unix)]
    #[test]
    fn resolver_rejects_a_symbolic_link_used_as_a_mount_root() {
        let fixture = Fixture::new("root_symlink");
        let outside = fixture.root.join("outside-private");
        fs::remove_dir_all(&fixture.private).expect("private root should be removable");
        fs::create_dir_all(&outside).expect("outside root should exist");
        std::os::unix::fs::symlink(&outside, &fixture.private)
            .expect("test root symlink should be created");

        assert!(GuestFilesystem::new(
            "com.example",
            &fixture.private,
            &fixture.device_private,
            &fixture.shared
        )
        .is_err());
    }

    #[cfg(unix)]
    #[test]
    fn resolver_rejects_a_symbolic_link_used_as_the_device_mount_root() {
        let fixture = Fixture::new("device_root_symlink");
        let outside = fixture.root.join("outside-device-private");
        fs::remove_dir_all(&fixture.device_private).expect("device root should be removable");
        fs::create_dir_all(&outside).expect("outside root should exist");
        std::os::unix::fs::symlink(&outside, &fixture.device_private)
            .expect("test device root symlink should be created");

        assert!(GuestFilesystem::new(
            "com.example",
            &fixture.private,
            &fixture.device_private,
            &fixture.shared
        )
        .is_err());
    }

    #[cfg(unix)]
    #[test]
    fn resolver_rejects_a_symbolic_link_used_as_the_app_directory() {
        let fixture = Fixture::new("app_symlink");
        let real_app = fixture.root.join("real-app");
        let linked_app = fixture
            .private
            .parent()
            .expect("private root should have an app parent");
        fs::remove_dir_all(linked_app).expect("app directory should be removable");
        fs::create_dir_all(real_app.join("data")).expect("real app data should exist");
        std::os::unix::fs::symlink(&real_app, linked_app)
            .expect("test app symlink should be created");

        assert!(GuestFilesystem::new(
            "com.example",
            &fixture.private,
            &fixture.device_private,
            &fixture.shared
        )
        .is_err());
    }

    #[cfg(unix)]
    #[test]
    fn resolver_rejects_a_symbolic_link_used_as_the_apps_directory() {
        let fixture = Fixture::new("apps_symlink");
        let apps = fixture.root.join("apps");
        let outside_apps = fixture.root.join("outside-apps");
        fs::remove_dir_all(&apps).expect("apps directory should be removable");
        fs::create_dir_all(outside_apps.join("com.example/data"))
            .expect("outside app data should exist");
        std::os::unix::fs::symlink(&outside_apps, &apps)
            .expect("test apps symlink should be created");

        assert!(GuestFilesystem::new(
            "com.example",
            &fixture.private,
            &fixture.device_private,
            &fixture.shared
        )
        .is_err());
    }

    struct Fixture {
        root: PathBuf,
        private: PathBuf,
        device_private: PathBuf,
        shared: PathBuf,
    }

    impl Fixture {
        fn new(name: &str) -> Self {
            static NEXT_ID: AtomicU64 = AtomicU64::new(1);
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "winedroid-guest-filesystem-{name}-{}-{id}",
                std::process::id()
            ));
            let private = root.join("apps/com.example/data");
            let device_private = root.join("apps/com.example/data_de");
            let shared = root.join("storage");
            fs::create_dir_all(&private).expect("private directory should exist");
            fs::create_dir_all(&device_private).expect("device private directory should exist");
            fs::create_dir_all(&shared).expect("shared directory should exist");

            Self {
                root,
                private,
                device_private,
                shared,
            }
        }

        fn filesystem(&self) -> GuestFilesystem {
            GuestFilesystem::new(
                "com.example",
                &self.private,
                &self.device_private,
                &self.shared,
            )
            .expect("guest filesystem should initialize")
        }
    }

    fn restricted_policy() -> ExternalStoragePolicy {
        ExternalStoragePolicy::restricted(35)
    }

    fn resolve_read(
        filesystem: &GuestFilesystem,
        path: &str,
        policy: &ExternalStoragePolicy,
    ) -> Result<ResolvedGuestPath> {
        filesystem.resolve(path, ExternalStorageAccess::Read, policy)
    }

    fn policy_with_runtime_access(
        target_sdk_version: i32,
        permissions: Vec<&str>,
        legacy_storage: bool,
        all_files_access: bool,
    ) -> ExternalStoragePolicy {
        let permissions = permissions
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        let runtime_access = if all_files_access {
            crate::external_storage::ExternalStorageRuntimeAccess::new(
                permissions.clone(),
                legacy_storage,
                true,
            )
        } else {
            let read_access = permissions
                .iter()
                .any(|permission| permission == crate::external_storage::READ_EXTERNAL_STORAGE);
            let write_access = permissions
                .iter()
                .any(|permission| permission == crate::external_storage::WRITE_EXTERNAL_STORAGE);
            crate::external_storage::ExternalStorageRuntimeAccess::with_legacy_file_access(
                permissions.clone(),
                legacy_storage,
                read_access,
                write_access,
            )
        };
        ExternalStoragePolicy::new(
            target_sdk_version,
            target_sdk_version,
            permissions,
            runtime_access,
        )
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.root).ok();
        }
    }
}
