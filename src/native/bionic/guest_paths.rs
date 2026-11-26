use std::ffi::CString;
use std::path::PathBuf;

use super::super::guest_context::{self, GuestContext};
use super::super::host_log;
use super::abi_types::{EINVAL, ENAMETOOLONG, ENOENT};
use super::fd_table::{self, FdKind};
use super::fs_result::linux_errno_of;
use super::guest_process;
use super::linux_abi::{at, misc};
use super::procfs;
use super::synthetic_node::Synthetic;
use crate::capability_fs::Directory;
use crate::external_storage::ExternalStorageAccess;
use crate::guest_app_paths;
use crate::guest_filesystem::GuestPathError;

const EACCES: i32 = 13;
const ENOTDIR: i32 = 20;
const ELOOP: i32 = 40;
const MAX_LINK_EXPANSIONS: usize = 40;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Read,
    Write,
}

pub struct HostRef {
    pub mount: &'static str,
    pub protected: bool,
    pub dir: Directory,
    pub leaf: Option<CString>,
    pub read_only: bool,
    pub guest_path: String,
}

pub struct Device {
    pub name: &'static str,
    pub rdev: (u32, u32),
}

pub struct VirtualDir {
    pub guest_path: String,
    pub entries: Vec<(String, u8)>,
    pub mode: u32,
    pub owner: u32,
}

pub enum Resolved {
    Host(HostRef),
    Device(Device),
    Synthetic(Synthetic, String),
    Virtual(VirtualDir),
}

// cite: bionic sysmacros.h#makedev
const DEVICES: &[(&str, (u32, u32))] = &[
    ("null", (1, 3)),
    ("zero", (1, 5)),
    ("random", (1, 8)),
    ("urandom", (1, 9)),
    ("tty", (5, 0)),
];

pub fn device_host_path(name: &str) -> &'static str {
    match name {
        "null" => "/dev/null",
        "zero" => "/dev/zero",
        "tty" => "/dev/tty",
        _ => "/dev/urandom",
    }
}

fn normalize(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for component in path.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    format!("/{}", parts.join("/"))
}

pub fn absolute(dirfd: i32, path: &[u8]) -> Result<String, i32> {
    if path.is_empty() {
        return Err(ENOENT);
    }
    if path.len() >= misc::PATH_MAX {
        return Err(ENAMETOOLONG);
    }
    if path
        .split(|byte| *byte == b'/')
        .any(|component| component.len() > misc::NAME_MAX)
    {
        return Err(ENAMETOOLONG);
    }
    let text = String::from_utf8_lossy(path).into_owned();
    if text.starts_with('/') {
        return Ok(normalize(&text));
    }
    let base = if dirfd == at::FDCWD {
        guest_process::cwd()
    } else {
        let entry = fd_table::get(dirfd).ok_or(9)?;
        if !matches!(entry.kind, FdKind::Dir | FdKind::PathOnly) {
            return Err(ENOTDIR);
        }
        entry.guest_path.clone()
    };
    Ok(normalize(&format!("{base}/{text}")))
}

fn rewrite_alias(path: &str) -> String {
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

fn directory_entries(names: &[&str]) -> Vec<(String, u8)> {
    names
        .iter()
        .map(|name| (name.to_string(), misc::DT_DIR))
        .collect()
}

fn skeleton(path: &str, context: &GuestContext) -> Option<(Vec<(String, u8)>, u32, u32)> {
    let package = context.package.as_str();
    let root_code = guest_app_paths::package_root(package);
    let app_dir = |name: &str| (name.to_string(), misc::DT_DIR);
    let (entries, mode, owner) = match path {
        "/" => (
            directory_entries(&["data", "dev", "mnt", "proc", "storage", "sys", "system"]),
            0o755,
            0,
        ),
        "/data" => (
            directory_entries(&["app", "data", "local", "user", "user_de"]),
            0o771,
            1000,
        ),
        "/data/user" | "/data/user_de" => (directory_entries(&["0"]), 0o711, 1000),
        "/data/user/0" | "/data/user_de/0" => (vec![app_dir(package)], 0o711, 1000),
        "/data/app" => (directory_entries(&["~~winedroid=="]), 0o751, 1000),
        "/data/app/~~winedroid==" => (
            vec![app_dir(&format!("{package}-winedroid=="))],
            0o755,
            1000,
        ),
        "/data/local" => (directory_entries(&["tmp"]), 0o751, 2000),
        "/data/local/tmp" => (Vec::new(), 0o771, 2000),
        "/storage" => (directory_entries(&["emulated", "self"]), 0o755, 0),
        "/storage/emulated" => (directory_entries(&["0"]), 0o755, 0),
        "/storage/self" => (directory_entries(&["primary"]), 0o755, 0),
        "/dev" => (
            DEVICES
                .iter()
                .map(|(name, _)| (name.to_string(), misc::DT_CHR))
                .collect(),
            0o755,
            0,
        ),
        "/mnt" => (Vec::new(), 0o755, 0),
        "/system" => (vec![("build.prop".to_string(), misc::DT_REG)], 0o755, 0),
        "/sys" => (directory_entries(&["devices"]), 0o555, 0),
        "/sys/devices" => (directory_entries(&["system"]), 0o755, 0),
        "/sys/devices/system" => (directory_entries(&["cpu"]), 0o755, 0),
        other if other == root_code => (
            vec![("base.apk".to_string(), misc::DT_REG), app_dir("lib")],
            0o755,
            1000,
        ),
        other if other == format!("{root_code}/lib") => {
            (directory_entries(&["arm64"]), 0o755, 1000)
        }
        _ => return None,
    };
    Some((entries, mode, owner))
}

fn private_mount(path: &str, context: &GuestContext) -> bool {
    let package = &context.package;
    ["/data/user/0", "/data/user_de/0"].iter().any(|root| {
        path == format!("{root}/{package}") || path.starts_with(&format!("{root}/{package}/"))
    }) || path == "/storage/emulated/0"
        || path.starts_with("/storage/emulated/0/")
        || path == "/sdcard"
        || path.starts_with("/sdcard/")
}

fn errno_of_io(error: &std::io::Error) -> i32 {
    match error.raw_os_error() {
        Some(code) => {
            let linux = linux_errno_of(code);
            if linux == ELOOP {
                super::fs_result::missing("guest-symlink-follow", "a guest path component is a symlink and the capability walk never follows symlinks");
            }
            linux
        }
        None if error.kind() == std::io::ErrorKind::InvalidInput => EINVAL,
        None => EACCES,
    }
}

fn through_storage(path: &str, access: Access, context: &GuestContext) -> Result<Resolved, i32> {
    let wanted = if access == Access::Write {
        ExternalStorageAccess::Write
    } else {
        ExternalStorageAccess::Read
    };
    let policy = context.policy.read().expect("external storage policy");
    let resolved = context
        .filesystem
        .resolve_typed(path, wanted, &policy)
        .map_err(|failure| match failure.kind {
            GuestPathError::OutsideMounts
            | GuestPathError::EscapesRoot
            | GuestPathError::HiddenDirectory => ENOENT,
            GuestPathError::NulByte => EINVAL,
            GuestPathError::ExternalScopeDenied => EACCES,
        })?;
    let (dir, leaf) = resolved
        .parent_and_leaf()
        .map_err(|error| errno_of_io(&error))?;
    let guest_mount = resolved.mount();
    let mount = match guest_mount {
        crate::guest_filesystem::GuestMount::CredentialProtected => "credential",
        crate::guest_filesystem::GuestMount::DeviceProtected => "device",
        crate::guest_filesystem::GuestMount::Shared => "shared",
        crate::guest_filesystem::GuestMount::AppCode => "app code",
    };
    Ok(Resolved::Host(HostRef {
        mount,
        protected: resolved.is_mutation_protected(),
        dir,
        leaf,
        read_only: guest_mount == crate::guest_filesystem::GuestMount::AppCode,
        guest_path: path.to_string(),
    }))
}

fn app_code(path: &str, context: &GuestContext) -> Option<Result<Resolved, i32>> {
    for mount in &context.app_code {
        let remainder = if path == mount.guest {
            Some("")
        } else {
            path.strip_prefix(mount.guest.as_str())
                .and_then(|rest| rest.strip_prefix('/'))
        };
        let Some(remainder) = remainder else { continue };
        let mut host: PathBuf = mount.host.clone();
        if !remainder.is_empty() {
            host.push(remainder);
        }
        let parent = host
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/"));
        let leaf = host
            .file_name()
            .map(|name| CString::new(name.as_encoded_bytes()).expect("path has no NUL"));
        let dir = match Directory::open_ambient(&parent) {
            Ok(dir) => dir,
            Err(error) => return Some(Err(errno_of_io(&error))),
        };
        return Some(Ok(Resolved::Host(HostRef {
            mount: "code",
            protected: true,
            dir,
            leaf,
            read_only: true,
            guest_path: path.to_string(),
        })));
    }
    None
}

pub fn resolve_absolute(path: &str, access: Access) -> Result<Resolved, i32> {
    let context = guest_context::current().ok_or_else(|| {
        host_log::line("FIXME: guest filesystem is not installed; next missing primitive: guest context install before native file calls");
        ENOENT
    })?;
    let path = rewrite_alias(path);
    if let Some((entries, mode, owner)) = skeleton(&path, &context) {
        return Ok(Resolved::Virtual(VirtualDir {
            guest_path: path,
            entries,
            mode,
            owner,
        }));
    }
    if private_mount(&path, &context) {
        return through_storage(&path, access, &context);
    }
    for root in ["/data/user/0/", "/data/user_de/0/"] {
        if let Some(rest) = path.strip_prefix(root) {
            let other = rest.split('/').next().unwrap_or("");
            return Err(if context.prefix.join("apps").join(other).is_dir() {
                EACCES
            } else {
                ENOENT
            });
        }
    }
    if let Some(found) = app_code(&path, &context) {
        return found;
    }
    if let Some(name) = path.strip_prefix("/dev/") {
        if let Some((device, rdev)) = DEVICES.iter().find(|(device, _)| *device == name) {
            return Ok(Resolved::Device(Device {
                name: device,
                rdev: *rdev,
            }));
        }
        if let Some(descriptor) = ["stdin", "stdout", "stderr"]
            .iter()
            .position(|stream| *stream == name)
        {
            let target = format!("/proc/self/fd/{descriptor}");
            return Ok(Resolved::Synthetic(
                Synthetic::link(super::synthetic_node::PROC_DEVICE, move || target.clone()),
                path,
            ));
        }
        host_log::line(&format!("FIXME: missing-primitive guest device {path}: no host equivalent; next missing primitive: guest device {path}"));
        return Err(ENOENT);
    }
    if let Some(node) = procfs::lookup(&path, &context) {
        return Ok(Resolved::Synthetic(node, path));
    }
    if path.starts_with("/proc") || path.starts_with("/sys") || path.starts_with("/system") {
        host_log::line(&format!("FIXME: missing-primitive guest path {path}: not synthesized; next missing primitive: guest path {path}"));
    }
    Err(ENOENT)
}

pub fn resolve(dirfd: i32, path: &[u8], access: Access) -> Result<Resolved, i32> {
    if path.contains(&0) {
        return Err(EINVAL);
    }
    resolve_absolute(&absolute(dirfd, path)?, access)
}

fn parent_of(path: &str) -> &str {
    match path.rfind('/') {
        Some(0) | None => "/",
        Some(index) => &path[..index],
    }
}

pub fn resolve_follow_absolute(path: &str, access: Access) -> Result<Resolved, i32> {
    resolve_follow_with_path(path, access).map(|(resolved, _)| resolved)
}

pub fn resolve_follow_with_path(path: &str, access: Access) -> Result<(Resolved, String), i32> {
    let mut current = path.to_string();
    for _ in 0..MAX_LINK_EXPANSIONS {
        let resolved = resolve_absolute(&current, access)?;
        let target = match &resolved {
            Resolved::Host(reference) => reference.link_target(),
            Resolved::Synthetic(Synthetic::Link { target, .. }, _) => Some(target()),
            _ => None,
        };
        let Some(target) = target else {
            return Ok((resolved, current));
        };
        current = if target.starts_with('/') {
            normalize(&target)
        } else {
            normalize(&format!("{}/{target}", parent_of(&current)))
        };
    }
    Err(ELOOP)
}

pub fn resolve_follow(dirfd: i32, path: &[u8], access: Access) -> Result<Resolved, i32> {
    if path.contains(&0) {
        return Err(EINVAL);
    }
    resolve_follow_absolute(&absolute(dirfd, path)?, access)
}

pub fn host_ref(resolved: Resolved, access: Access) -> Result<HostRef, i32> {
    match resolved {
        Resolved::Host(reference) => {
            if access == Access::Write && reference.read_only {
                return Err(30);
            }
            Ok(reference)
        }
        Resolved::Device(_) | Resolved::Virtual(_) | Resolved::Synthetic(..) => {
            Err(if access == Access::Write {
                EACCES
            } else {
                ENOENT
            })
        }
    }
}

pub fn to_guest(host_path: &str) -> Option<String> {
    let context = guest_context::current()?;
    let host = PathBuf::from(host_path);
    for mount in &context.app_code {
        if let Ok(rest) = host.strip_prefix(&mount.host) {
            return Some(if rest.as_os_str().is_empty() {
                mount.guest.clone()
            } else {
                format!("{}/{}", mount.guest, rest.display())
            });
        }
    }
    None
}
